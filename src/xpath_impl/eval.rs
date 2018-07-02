//
// xpath_impl/eval.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::collections::HashMap;
use std::cmp::Ordering;
use std::error::Error;
use std::f64;
use std::i64;
use std::rc::Rc;
use std::str::FromStr;
use std::usize;

use dom::*;
use xmlerror::*;
use xpath_impl::parser::*;
use xpath_impl::xitem::*;
use xpath_impl::xsequence::*;
use xpath_impl::func::*;
use xpath_impl::oper::*;

// =====================================================================
//
fn xnode_to_string(xnode: &XNodePtr) -> String {
    return xnode_dump_sub(xnode, 0, 0, "T", false);
}

pub fn xnode_dump(xnode: &XNodePtr) -> String {
    return xnode_dump_sub(xnode, 0, 4, "T", true);
}

// =====================================================================
//
fn xnode_dump_sub(xnode: &XNodePtr, indent: usize, step: usize, pref: &str, recursive: bool) -> String {
    let mut buf: String = format!("{}{} [{}] {}\n",
            &" ".repeat(indent),
            pref,
            get_xnode_type(xnode).to_string(),
            &get_xnode_name(&xnode));
    if recursive {
        let xl = get_left(xnode);
        if ! is_nil_xnode(&xl) {
            buf += &xnode_dump_sub(&xl, indent + step, step, "L", recursive);
        }
        let xr = get_right(xnode);
        if ! is_nil_xnode(&xr) {
            buf += &xnode_dump_sub(&xr, indent + step, step, "R", recursive);
        }
    }
    return buf;
}

// ---------------------------------------------------------------------
// 文字列→数値の変換。
// 空白 (オプション)、負符号 (オプション)、Number、空白 (オプション) が
// この順で連なる文字列を、IEEE 754の数値に変換する。
// それ以外はNaNにする。
// 規格上は、正記号も使えないことになる。
//
fn atof(s: &str) -> f64 {
    return f64::from_str(s.trim()).unwrap_or(f64::NAN);
}

fn atoi(s: &str) -> i64 {
    return i64::from_str(s.trim()).unwrap_or(0);
}

// ---------------------------------------------------------------------
//
fn usize_to_i64(n: usize) -> i64 {
    return n as i64;
}

// =====================================================================
// 評価環境
//
pub struct EvalEnv {
    doc_order_hash: HashMap<i64, i64>,      // node_id -> 順序番号
    position: usize,                        // 組み込み函数 position() の値
    last: usize,                            // 組み込み函数 last() の値
    var_hash: HashMap<String, XItem>,       // 変数表
}

fn new_eval_env() -> EvalEnv {
    return EvalEnv{
        doc_order_hash: HashMap::new(),
        position: 0,
        last: 0,
        var_hash: HashMap::new(),
    }
}

impl EvalEnv {
    // -----------------------------------------------------------------
    // 文書順を調べ、登録しておく。
    //
    fn setup_doc_order(&mut self, node: &NodePtr) {
        self.setup_doc_order_sub(&node.root(), 1);
    }
    fn setup_doc_order_sub(&mut self, node: &NodePtr, order_beg: i64) -> i64 {
        let mut order = order_beg;
        self.doc_order_hash.insert(node.node_id(), order);
        order += 1;
        for at in node.attributes().iter() {
            self.doc_order_hash.insert(at.node_id(), order);
            order += 1;
        }
        for ch in node.children().iter() {
            order = self.setup_doc_order_sub(ch, order + 1);
        }
        return order;
    }

    // -----------------------------------------------------------------
    // 文書順に整列し、重複を除去する。
    //
    pub fn sort_by_doc_order(&self, node_array: &mut Vec<NodePtr>) {
        if node_array.len() <= 1 {
            return;
        }
        node_array.sort_by(|a, b| {
            return self.compare_by_doc_order(a, b);
        });
        let mut i = node_array.len() - 1;
        while 0 < i {
            if node_array[i].node_id() == node_array[i - 1].node_id() {
                node_array.remove(i);
            }
            i -= 1;
        }
    }

    // -----------------------------------------------------------------
    // 文書順を比較し、Ordering::{Less,Equal,Greater} を返す。
    //
    pub fn compare_by_doc_order(&self, a: &NodePtr, b: &NodePtr) -> Ordering {
        let a_order = self.doc_order_hash.get(&a.node_id()).unwrap_or(&0);
        let b_order = self.doc_order_hash.get(&b.node_id()).unwrap_or(&0);
        return a_order.cmp(&b_order);
    }

    // -----------------------------------------------------------------
    //
    fn set_var(&mut self, name: &str, value: &XItem) {
        self.var_hash.insert(String::from(name), value.clone());
    }

    // -----------------------------------------------------------------
    //
    fn get_var(&mut self, name: &str) -> Option<&XItem> {
        return self.var_hash.get(name);
    }

    // -----------------------------------------------------------------
    //
    fn remove_var(&mut self, name: &str) {
        self.var_hash.remove(name);
    }

    // -----------------------------------------------------------------
    //
    fn set_position(&mut self, position: usize) -> usize {
        let old_position = self.position;
        self.position = position;
        return old_position;
    }
    fn set_last(&mut self, last: usize) -> usize{
        let old_last = self.last;
        self.last = last;
        return old_last;
    }

    // -----------------------------------------------------------------
    //
    pub fn get_position(&self) -> usize {
        return self.position;
    }
    pub fn get_last(&self) -> usize {
        return self.last;
    }
}

// =====================================================================
// (EVAL)
//
pub fn match_xpath(start_node: &NodePtr, xnode: &XNodePtr) -> Result<XSequence, Box<Error>> {

    let mut eval_env = new_eval_env();
    eval_env.setup_doc_order(start_node);

    let start_xsequence = new_singleton_node(start_node);
    return evaluate_xnode(&start_xsequence, xnode, &mut eval_env);
}

// ---------------------------------------------------------------------
// あるXMLノードに対して、XPath構文木のあるノードを適用し、評価結果を返す。
//
fn evaluate_xnode(xseq: &XSequence, xnode: &XNodePtr,
                    eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    if is_nil_xnode(xnode) {
        panic!("Can't occur: evaluate_xnode, xnode is nil");
    }

    let xnode_type = get_xnode_type(&xnode);
    match xnode_type {
        XNodeType::OperatorPath => {
            let left_xnode = get_left(xnode);
            let lhs = if ! is_nil_xnode(&left_xnode) {
                evaluate_xnode(xseq, &left_xnode, eval_env)?
            } else {
                new_xsequence()
            };
            let right_xnode = get_right(xnode);
            if ! is_nil_xnode(&right_xnode) {
                return evaluate_xnode(&lhs, &right_xnode, eval_env);
            } else {
                return Ok(lhs);
            }
        },

        XNodeType::AxisAncestor |
        XNodeType::AxisAncestorOrSelf |
        XNodeType::AxisAttribute |
        XNodeType::AxisChild |
        XNodeType::AxisDescendant |
        XNodeType::AxisDescendantOrSelf |
        XNodeType::AxisFollowing |
        XNodeType::AxisFollowingSibling |
        XNodeType::AxisParent |
        XNodeType::AxisPreceding |
        XNodeType::AxisPrecedingSibling |
        XNodeType::AxisRoot |
        XNodeType::AxisSelf => {
            return match_location_path(xseq, xnode, eval_env);
        },

        XNodeType::ContextItem => {
            println!("ContextItem => xseq = {}", xseq.to_string());
            return Ok(xseq.clone());
        }

        XNodeType::ApplyPredicates => {
            // leftを評価して得られるシーケンスに対し、rightの述語を
            // 適用してしぼり込む。
            //
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = get_right(xnode);
            if ! is_nil_xnode(&rhs) {
                let filtered_xseq = filter_by_predicates(&lhs, &rhs, eval_env)?;
                return Ok(filtered_xseq);
            } else {
                return Ok(lhs);
            }
        },

        XNodeType::OperatorConcatenate => {
            // シーケンスを連結する。
            //
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_concatenate(&vec!{lhs, rhs});
        },

        XNodeType::OperatorOr => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let lhs_b = lhs.effective_boolean_value()?;
            if lhs_b == true {
                return Ok(new_singleton_boolean(true));
            } else {
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                let rhs_b = rhs.effective_boolean_value()?;
                return Ok(new_singleton_boolean(rhs_b));
            }
        },
        XNodeType::OperatorAnd => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let lhs_b = lhs.effective_boolean_value()?;
            if lhs_b == false {
                return Ok(new_singleton_boolean(false));
            } else {
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                let rhs_b = rhs.effective_boolean_value()?;
                return Ok(new_singleton_boolean(rhs_b));
            }
        },

        XNodeType::OperatorGeneralEQ => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_eq(&lhs, &rhs);
        },

        XNodeType::OperatorGeneralNE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_ne(&lhs, &rhs);
        },

        XNodeType::OperatorGeneralLT => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_lt(&lhs, &rhs);
        },

        XNodeType::OperatorGeneralLE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_le(&lhs, &rhs);
        },

        XNodeType::OperatorGeneralGT => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_gt(&lhs, &rhs);
        },

        XNodeType::OperatorGeneralGE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return general_compare_ge(&lhs, &rhs);
        },

        XNodeType::OperatorValueEQ => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_eq(&lhs, &rhs);
        },

        XNodeType::OperatorValueNE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_ne(&lhs, &rhs);
        },

        XNodeType::OperatorValueLT => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_lt(&lhs, &rhs);
        },

        XNodeType::OperatorValueLE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_le(&lhs, &rhs);
        },

        XNodeType::OperatorValueGT => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_gt(&lhs, &rhs);
        },

        XNodeType::OperatorValueGE => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return value_compare_ge(&lhs, &rhs);
        },

        XNodeType::OperatorAdd => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_add(&vec!{lhs, rhs});
        },

        XNodeType::OperatorSubtract => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_subtract(&vec!{lhs, rhs});
        },

        XNodeType::OperatorUnaryPlus => {
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_unary_plus(&vec!{rhs});
        },

        XNodeType::OperatorUnaryMinus => {
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_unary_minus(&vec!{rhs});
        },

        XNodeType::OperatorMultiply => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_multiply(&vec!{lhs, rhs});
        },

        XNodeType::OperatorDiv => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_divide(&vec!{lhs, rhs});
        },

        XNodeType::OperatorIDiv => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_integer_divide(&vec!{lhs, rhs});
        },

        XNodeType::OperatorMod => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_numeric_mod(&vec!{lhs, rhs});
        },

        XNodeType::OperatorUnion => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_union(&vec!{lhs, rhs}, eval_env);
        },

        XNodeType::OperatorIntersect => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_intersect(&vec!{lhs, rhs}, eval_env);
        },

        XNodeType::OperatorExcept => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_except(&vec!{lhs, rhs}, eval_env);
        },

        XNodeType::OperatorTo => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_to(&vec!{lhs, rhs});
        },

        XNodeType::OperatorIsSameNode => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_is_same_node(&vec!{lhs, rhs}, eval_env);
        },
        XNodeType::OperatorNodeBefore => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_node_before(&vec!{lhs, rhs}, eval_env);
        },
        XNodeType::OperatorNodeAfter => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return op_node_after(&vec!{lhs, rhs}, eval_env);
        },

        XNodeType::IfExpr => {
            let cond = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let xnode_if_then_else = get_right(xnode);
            if get_xnode_type(&xnode_if_then_else) != XNodeType::IfThenElse {
                return Err(cant_occur!("IfExpr: rightがIfThenElseでない。"));
            }
            if cond.effective_boolean_value()? == true {
                let value = evaluate_xnode(xseq, &get_left(&xnode_if_then_else), eval_env)?;
                return Ok(value);
            } else {
                let value = evaluate_xnode(xseq, &get_right(&xnode_if_then_else), eval_env)?;
                return Ok(value);
            }
        },

        XNodeType::ForExpr => {
            return evaluate_xnode(xseq, &get_right(xnode), eval_env);
        },

        XNodeType::ForVarBind => {
            let var_name = get_xnode_name(&xnode);
            let range = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let mut result = new_xsequence();
            for xitem in range.iter() {
                eval_env.set_var(var_name.as_str(), xitem);
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                result.append(&rhs);
                eval_env.remove_var(var_name.as_str());
            }
            return Ok(result);
        },

        XNodeType::SomeExpr => {
            return evaluate_xnode(xseq, &get_right(xnode), eval_env);
        },

        XNodeType::SomeVarBind => {
            let var_name = get_xnode_name(&xnode);
            let range = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;

            for xitem in range.iter() {
                eval_env.set_var(var_name.as_str(), xitem);
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                if rhs.effective_boolean_value()? == true {
                    return Ok(new_singleton_boolean(true));
                }
                eval_env.remove_var(var_name.as_str());
            }
            return Ok(new_singleton_boolean(false));
        },

        XNodeType::EveryExpr => {
            return evaluate_xnode(xseq, &get_right(xnode), eval_env);
        },

        XNodeType::EveryVarBind => {
            let var_name = get_xnode_name(&xnode);
            let range = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            for xitem in range.iter() {
                eval_env.set_var(var_name.as_str(), xitem);
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                if rhs.effective_boolean_value()? == false {
                    return Ok(new_singleton_boolean(false));
                }
                eval_env.remove_var(var_name.as_str());
            }
            return Ok(new_singleton_boolean(true));
        },

        XNodeType::OperatorCastableAs => {
            let value = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let single_type_xnode = get_right(xnode);
            let atomic_type_xnode = get_left(&single_type_xnode);
            let type_name = get_xnode_name(&atomic_type_xnode);
            return Ok(new_singleton_boolean(value.castable_as(&type_name)));
        }

        XNodeType::OperatorCastAs => {
            let value = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let single_type_xnode = get_right(xnode);
            let atomic_type_xnode = get_left(&single_type_xnode);
            let type_name = get_xnode_name(&atomic_type_xnode);
            return value.cast_as(&type_name);
        }

        XNodeType::FunctionCall => {
            // rightに連なっているXNodeArgumentTopノード群のleft以下にある
            // 式を評価し、argsArray (引数の配列) を得た後、
            // この引数列を渡して函数を評価する。
            //
            let mut args_array: Vec<XSequence> = vec!{};
            let mut curr_xnode = get_right(&xnode);
            while ! is_nil_xnode(&curr_xnode) {
                match get_xnode_type(&curr_xnode) {
                    XNodeType::ArgumentTop => {
                        let arg = evaluate_xnode(xseq,
                                    &get_left(&curr_xnode), eval_env)?;
                        args_array.push(arg);
                    },
                    _ => {
                        return Err(cant_occur!("FunctionCall: rightがArgumentTopでない。"));
                    },
                }
                curr_xnode = get_right(&curr_xnode);
            }
            return evaluate_function(&get_xnode_name(&xnode),
                    &mut args_array, xseq, eval_env);
        },

        XNodeType::StringLiteral => {
            return Ok(new_singleton_string(&get_xnode_name(&xnode)));
        },
        XNodeType::IntegerLiteral => {
            return Ok(new_singleton_integer(atoi(&get_xnode_name(&xnode))));
        },
        XNodeType::DecimalLiteral => {
            return Ok(new_singleton_decimal(atof(&get_xnode_name(&xnode))));
        },
        XNodeType::DoubleLiteral => {
            return Ok(new_singleton_double(atof(&get_xnode_name(&xnode))));
        },
        XNodeType::VariableReference => {
            let var_name = get_xnode_name(&xnode);
            match eval_env.get_var(var_name.as_str()) {
                Some(item) => {
                    return Ok(new_singleton(item));
                },
                None => {
                    return Ok(new_xsequence());
                },
            }
        },
        _ => {
            return Err(cant_occur!(
                "evaluate_xnode: xnode = {}", xnode_to_string(xnode)));
        }
    }
}

// ---------------------------------------------------------------------
// XSequence中の各ノードに対し、xnodeで示されるLocStepを適用して
// 合致するノード集合を取得し、その合併をXSequenceとして返す。
//
fn match_location_path(xseq: &XSequence, xnode: &XNodePtr,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {
    let mut new_node_array: Vec<NodePtr> = vec!{};
    for node in xseq.to_nodeset().iter() {
        let mut matched_xseq = match_loc_step(node, xnode, eval_env)?;
        new_node_array.append(&mut matched_xseq.to_nodeset());
    }

    // -------------------------------------------------------------
    // 得られたノード集合を文書順に整列し、重複を除去する。
    // // 「!」演算子 (XPath-3.0) の場合は整列しない。
    //
    eval_env.sort_by_doc_order(&mut new_node_array);
    let result = new_xsequence_from_node_array(&new_node_array);
    return Ok(result);
}

// ---------------------------------------------------------------------
// XML木のあるノードを起点として、
// xNodeで示されるLocStep (例: 「child::foo[@attr='at']」) に、
// 軸、ノード・テスト、述語が合致するノード集合をXSequenceの形で返す。
//
fn match_loc_step(node: &NodePtr, xnode: &XNodePtr,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut node_array: Vec<NodePtr> = vec!{};

    match get_xnode_type(&xnode) {
        XNodeType::AxisAncestor => {
            node_array = match_along_axis(node, xnode, array_ancestor);
        },

        XNodeType::AxisAncestorOrSelf => {
            node_array = match_along_axis(node, xnode, array_ancestor_or_self);
        },

        XNodeType::AxisAttribute => {
            node_array = match_along_axis(node, xnode, NodePtr::attributes);
        },

        XNodeType::AxisChild => {
            node_array = match_along_axis(node, xnode, NodePtr::children);
        },

        XNodeType::AxisDescendant => {
            node_array = match_along_axis(node, xnode, array_descendant);
        },

        XNodeType::AxisDescendantOrSelf => {
            node_array = match_along_axis(node, xnode, array_descendant_or_self);
        },

        XNodeType::AxisFollowing => {
            node_array = match_along_axis(node, xnode, array_following);
        },

        XNodeType::AxisFollowingSibling => {
            node_array = match_along_axis(node, xnode, array_following_sibling);
        },

        XNodeType::AxisParent => {
            if let Some(parent) = node.parent() {
                if match_node_test(&parent, xnode) {
                    node_array.push(parent.rc_clone());
                }
            }
        },

        XNodeType::AxisPreceding => {
            node_array = match_along_axis(node, xnode, array_preceding);
        },

        XNodeType::AxisPrecedingSibling => {
            node_array = match_along_axis(node, xnode, array_preceding_sibling);
        },

        XNodeType::AxisRoot => {
            node_array.push(node.root().rc_clone());
        },

        XNodeType::AxisSelf => {
            if match_node_test(&node, xnode) {
                node_array.push(node.rc_clone());
            }
        },
        _ => {
            return Err(cant_occur!("match_loc_step: xnode: {}",
                    xnode_to_string(&xnode)));
        },
    }

    // 述語によって絞り込む。
    let rhs = get_right(&xnode);
    if ! is_nil_xnode(&rhs) {
        let result = filter_by_predicates(
                &new_xsequence_from_node_array(&node_array), &rhs, eval_env)?;
        return Ok(result);
    } else {
        return Ok(new_xsequence_from_node_array(&node_array));
    }
}

// ---------------------------------------------------------------------
// 函数 along_axis_func を適用して得たノード配列から、match_node_test() に
// 合格したノードのみ集めて返す。
//
fn match_along_axis<F>(node: &NodePtr, xnode: &XNodePtr,
                        mut along_axis_func: F) -> Vec<NodePtr>
    where F: FnMut(&NodePtr) -> Vec<NodePtr> {

    let mut node_array: Vec<NodePtr> = vec!{};
    for n in along_axis_func(&node).iter() {
        if match_node_test(&n, xnode) {
            node_array.push(n.rc_clone());
        }
    }
    return node_array;
}

// ---------------------------------------------------------------------
// ancestor軸で合致する候補ノード。
//
fn array_ancestor(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    if let Some(parent) = node.parent() {
        node_array.append(&mut array_ancestor(&parent));
        node_array.push(parent.rc_clone());
    }
    return node_array;
}

// ---------------------------------------------------------------------
// ancestor-or-self軸で合致する候補ノード。
//
pub fn array_ancestor_or_self(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    node_array.append(&mut array_ancestor(node));
    node_array.push(node.rc_clone());
    return node_array;
}

// ---------------------------------------------------------------------
// descendant軸で合致する候補ノード。
//
fn array_descendant(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    for ch in node.children().iter() {
        node_array.push(ch.rc_clone());
        node_array.append(&mut array_descendant(ch));
    }
    return node_array;
}

// ---------------------------------------------------------------------
// descendant-or-self軸で合致する候補ノード。
//
fn array_descendant_or_self(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    node_array.push(node.rc_clone());
    node_array.append(&mut array_descendant(node));
    return node_array;
}

// ---------------------------------------------------------------------
// following軸で合致する候補ノード。
//
fn array_following(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    if node.node_type() != NodeType::Attribute {
        let all_nodes = array_descendant_or_self(&node.root());
        let descendant_or_self_nodes = array_descendant_or_self(&node);
        let mut self_occured = false;
        for ch in all_nodes.iter() {
            if self_occured && ! descendant_or_self_nodes.contains(ch) {
                node_array.push(ch.rc_clone());
            }
            if ch == node {
                self_occured = true;
            }
        }
    }
    return node_array;
}

// ---------------------------------------------------------------------
// following-sibling軸で合致する候補ノード。
//
fn array_following_sibling(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    if node.node_type() != NodeType::Attribute {
        if let Some(parent) = node.parent() {
            let mut occured = false;
            for ch in parent.children().iter() {
                if occured {
                    node_array.push(ch.rc_clone());
                }
                if ch == node {
                    occured = true;
                }
            }
        }
    }
    return node_array;
}

// ---------------------------------------------------------------------
// preceding軸で合致する候補ノード。
//
fn array_preceding(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    if node.node_type() != NodeType::Attribute {
        let all_nodes = array_descendant_or_self(&node.root());
        let ancestor_nodes = array_ancestor(&node);
        let mut self_occured = false;
        for ch in all_nodes.iter() {
            if ch == node {
                self_occured = true;
            }
            if ! self_occured && ! ancestor_nodes.contains(ch) {
                node_array.push(ch.rc_clone());
            }
        }
    }
    return node_array;
}

// ---------------------------------------------------------------------
// preceding-sibling軸で合致する候補ノード。
//
fn array_preceding_sibling(node: &NodePtr) -> Vec<NodePtr> {
    let mut node_array: Vec<NodePtr> = vec!{};
    if node.node_type() != NodeType::Attribute {
        if let Some(parent) = node.parent() {
            let mut occured = false;
            for ch in parent.children().iter() {
                if ch == node {
                    occured = true;
                }
                if ! occured {
                    node_array.push(ch.rc_clone());
                }
            }
        }
    }
    return node_array;
}

// ---------------------------------------------------------------------
// ノード・テスト (名前テストまたは種類テスト)。
//
fn match_node_test(node: &NodePtr, xnode: &XNodePtr) -> bool {

    // xnode: AxisNNNN;
    // get_left(&xnode) がXNodeType::KindTestのとき:
    //     そのget_xnode_name(&xnode): KindTestで照合する規則
    // get_left(&xnode) がNilのとき:
    //     get_xnode_name(&xnode): NameTestで照合する名前
    //
    let kind_test_xnode = get_left(&xnode);
    if is_nil_xnode(&kind_test_xnode) {
        return match_name_test(node, xnode);
    } else {
        return match_kind_test(node, &kind_test_xnode);
    }
}

// ---------------------------------------------------------------------
// ノードの名前テスト。
//
fn match_name_test(node: &NodePtr, xnode: &XNodePtr) -> bool {

    let name_test_pattern = get_xnode_name(&xnode);
        // ノード名と照合するパターン。
        // 例えば「child::para」というステップの「para」。

    // -------------------------------------------------------------
    // 省略記法「//」は「/descendant-or-self::node()/」、
    // 「..」は「parent::node()」の意味であるが、
    // 便宜上、NameTestの形式とし、「node()」を設定してある。
    // (XNodeType::KindTestのノードを作るよりも処理が簡単)
    //
    if name_test_pattern.as_str() == "node()" {
        return true;
    }

    // -------------------------------------------------------------
    // 軸によって決まる主ノード型と実際のノード型が一致して
    // いなければfalseとする。
    //   attribute軸 => attribute
    //   //namespace軸 => namespace
    //   それ以外 => element
    //
    let main_node_type =
        if get_xnode_type(&xnode) == XNodeType::AxisAttribute {
            NodeType::Attribute
        } else {
            NodeType::Element
        };

    if main_node_type != node.node_type() {
        return false;
    }

    // -------------------------------------------------------------
    // 名前の照合にもとづく判定 (「*」とも照合)
    //
    if name_test_pattern == node.name() {
        return true;
    }
    if name_test_pattern.as_str() == "*" {
        return true;
    }

    // -------------------------------------------------------------
    // 「na:*」との照合にもとづく判定
    //
    let v: Vec<&str> = name_test_pattern.splitn(2, ":").collect();
    if v.len() == 2 && v[1] == "*" {
        if node.space_name() == v[0] {
            return true;
        }
    }

    return false;
}

// ---------------------------------------------------------------------
// [54] KindTest ::= DocumentTest                                   // ☆
//                 | ElementTest                                    // *
//                 | AttributeTest                                  // *
//                 | SchemaElementTest                              // ☆
//                 | SchemaAttributeTest                            // ☆
//                 | PITest
//                 | CommentTest
//                 | TextTest
//                 | AnyKindTest
// ☆ 未実装 (構文解析のみ)
// *  XNodeType::KindTestTypeName (引数にTypeNameが入っている場合) に
//    ついては未実装
//
fn match_kind_test(node: &NodePtr, xnode: &XNodePtr) -> bool {
    // assert:: get_xnode_type(&kind_test_xnode) == XNodeType::KindTest

    let node_type = node.node_type();

    let test_xnode = get_left(xnode);
    match get_xnode_type(&test_xnode) {
        XNodeType::DocumentTest => {
            // DocumentTestは未実装。
        },
        XNodeType::ElementTest => {
            let arg = get_xnode_name(&test_xnode);
            return node_type == NodeType::Element &&
                   (arg == "" || arg == "*" || arg == node.name());
        },              // 当面、TypeName (get_left(&test_xnode)) は無視
        XNodeType::AttributeTest => {
            let arg = get_xnode_name(&test_xnode);
            return node_type == NodeType::Attribute &&
                   (arg == "" || arg == "*" || arg == node.name());
        },              // 当面、TypeName (get_left(&test_xnode)) は無視
        XNodeType::SchemaElementTest => {
            // SchemaElementTestは未実装。
        },
        XNodeType::SchemaAttributeTest => {
            // SchemaAttributeTestは未実装。
        },
        XNodeType::PITest => {
            let arg = get_xnode_name(&test_xnode);
            return node_type == NodeType::Instruction &&
                   (arg == "" || arg == node.name());
        },
        XNodeType::CommentTest => {
            return node_type == NodeType::Comment;
        },
        XNodeType::TextTest => {
            return node_type == NodeType::Text;
        },
        XNodeType::AnyKindTest => {
            return true;
        },
        _ => {},
    }

    return false;
}

// ---------------------------------------------------------------------
// シーケンスに対して、述語を順次適用してしぼり込み、
// 最終的に得られるシーケンスを返す。
// xNode: nTypeがXNodeAxis*であるノードの右。
//    rightをたどったノードはすべてXNodePredicate{Rev}Topであり、
//    そのleft以下に述語式の構文木がある。
//
fn filter_by_predicates(xseq: &XSequence, xnode: &XNodePtr,
            eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut curr_xseq = xseq.clone();
    let mut curr_xnode = Rc::clone(xnode);

    while ! is_nil_xnode(&curr_xnode) {
        match get_xnode_type(&curr_xnode) {
            XNodeType::PredicateTop => {
                curr_xseq = filter_by_predicate(&curr_xseq,
                            &get_left(&curr_xnode), false, eval_env)?;
            },
            XNodeType::PredicateRevTop => {
                curr_xseq = filter_by_predicate(&curr_xseq,
                            &get_left(&curr_xnode), true, eval_env)?;
            },
            _ => {
                return Err(cant_occur!(
                    "filter_by_predicates: 述語以外のノード: {}",
                    get_xnode_type(&curr_xnode).to_string()));
            }
        }

        curr_xnode = get_right(&curr_xnode);
    }
    return Ok(curr_xseq);
}

// ---------------------------------------------------------------------
// シーケンスに属する個々のアイテムに対して、ある (ひとつの) 述語を
// 適用してしぼり込み、新しいシーケンスを返す。
//
fn filter_by_predicate(xseq: &XSequence, xnode: &XNodePtr,
        reverse_order: bool, eval_env: &mut EvalEnv)
                                    -> Result<XSequence, Box<Error>> {

    if is_nil_xnode(xnode) {
        return Err(cant_occur!("filter_by_predicate: xnode is nil"));
    }

    let mut result = new_xsequence();
    for (i, xitem) in xseq.iter().enumerate() {

        // 評価環境に文脈位置を設定する。
        let last = xseq.len();
        let position = if ! reverse_order { i + 1 } else { last - i };
                                            // 文脈位置の番号は1が起点
        let old_position = eval_env.set_position(position);
        let old_last = eval_env.set_last(last);

        // シーケンス中、i番目のアイテムに対してxnodeを適用して評価する。
        let val = evaluate_xnode(&new_singleton(xitem), xnode, eval_env)?;

        // 評価環境を元に戻しておく。
        eval_env.set_last(old_last);
        eval_env.set_position(old_position);

        // 評価結果をもとに、このアイテムを残すかどうか判定する。
        let mut do_push = false;
        match val.get_singleton_item() {
            Ok(XItem::XIInteger{value}) => {
                do_push = value == usize_to_i64(position);
            },
            Ok(XItem::XINode{value: _}) => {
                do_push = true;
            },
            Ok(XItem::XIBoolean{value}) => {
                do_push = value;
            },
            _ => {},
        }
        if do_push {
            result.push(&xitem.clone());
        }
    }
    return Ok(result);

}

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath_impl::helpers::compress_spaces;
    use xpath_impl::helpers::subtest_eval_xpath;
    use xpath_impl::helpers::subtest_xpath;


    // -----------------------------------------------------------------
    // Comma
    //
    #[test]
    fn test_comma() {
        let xml = compress_spaces(r#"
<root img="basic" base="base">
    <a img="a" />
    <b img="b1" />
    <b img="b2" />
    <c img="c" />
    <d img="d" />
</root>
        "#);

        subtest_eval_xpath("comma", &xml, &[
            ( "//a, //c", r#"(<a img="a">, <c img="c">)"# ),
            ( "2, 3", r#"(2, 3)"# ),
            ( "(2, 3)", r#"(2, 3)"# ),
            ( "2, 1 + 3", r#"(2, 4)"# ),
            ( "(2, (3, 4))", r#"(2, 3, 4)"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // if ( Expr ) then ExprSingle else ExprSingle
    //
    #[test]
    fn test_if_expr() {
        let xml = compress_spaces(r#"
<root base="base">
    <prod discount="discount">
        <wholesale id="wa">wholesaled apple</wholesale>
        <wholesale id="wb">wholesaled banana</wholesale>
        <retail id="ra">retailed apple</retail>
        <retail id="rb">retailed banana</retail>
    </prod>
    <item>
        <wholesale id="wa">wholesaled apple</wholesale>
        <wholesale id="wb">wholesaled banana</wholesale>
        <retail id="ra">retailed apple</retail>
        <retail id="rb">retailed banana</retail>
    </item>
</root>
        "#);

        subtest_eval_xpath("if_expr", &xml, &[
//            ( "if (1 = 1) then 3 else 5", "(3)" ),
//            ( "if (1 = 9) then 3 else 5", "(5)" ),
            ( "if (prod/@discount) then prod/wholesale else prod/retail",
              r#"(<wholesale id="wa">, <wholesale id="wb">)"# ),
//            ( "if (item/@discount) then item/wholesale else item/retail",
//              r#"(<retail id="ra">, <retail id="rb">)"# ),
                ]);
    }

    // -----------------------------------------------------------------
    // for $VarName in ExprSingle return ExprSingle
    //
    #[test]
    fn test_for_expr() {
        let xml = compress_spaces(r#"
<root base="base">
    <a v="x"/>
    <a v="y"/>
    <a v="z"/>
</root>
        "#);

        subtest_eval_xpath("for_expr", &xml, &[
            ( "for $x in 3 to 5 return $x * 2", "(6, 8, 10)" ),
            ( "for $x in 3 to 5, $y in 2 to 3 return $x * $y", "(6, 9, 8, 12, 10, 15)" ),
            ( "/root/a/@v", r#"(v="x", v="y", v="z")"# ),
            ( "for $aa in /root/a return $aa", r#"(<a v="x">, <a v="y">, <a v="z">)"# ),
            ( "for $aa in /root/a return $aa/@v", r#"(v="x", v="y", v="z")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // some $VarName in ExprSingle satisfies ExprSingle
    //
    #[test]
    fn test_some_expr() {
        let xml = compress_spaces(r#"
<root base="base">
    <a v="x"/>
    <a v="y"/>
    <a v="z"/>
</root>
        "#);

        subtest_eval_xpath("some_expr", &xml, &[
            ( "some $x in 3 to 5 satisfies $x mod 2 = 0", "(true)" ),
            ( "some $x in 3 to 5 satisfies $x mod 6 = 0", "(false)" ),
            ( "some $x in 1 to 2, $y in 2 to 3 satisfies $x + $y = 5", "(true)" ),
            ( "some $x in 1 to 2, $y in 2 to 3 satisfies $x + $y = 7", "(false)" ),
            ( r#"some $a in /root/a satisfies $a/@v = "y""#, "(true)" ),
            ( r#"some $a in /root/a satisfies $a/@v = "w""#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // every $VarName in ExprSingle satisfies ExprSingle
    //
    #[test]
    fn test_every_expr() {
        let xml = compress_spaces(r#"
<root base="base">
    <a v="x"/>
    <a v="y"/>
    <a v="z"/>
</root>
        "#);

        subtest_eval_xpath("every_expr", &xml, &[
            ( "every $x in 3 to 5 satisfies $x > 2", "(true)" ),
            ( "every $x in 3 to 5 satisfies $x > 3", "(false)" ),
            ( "every $x in 1 to 2, $y in 2 to 3 satisfies $x + $y > 2", "(true)" ),
            ( "every $x in 1 to 2, $y in 2 to 3 satisfies $x + $y > 4", "(false)" ),
            ( r#"every $a in /root/a satisfies $a/@v != "w""#, "(true)" ),
            ( r#"every $a in /root/a satisfies $a/@v = "y""#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // castable as
    //
    #[test]
    fn test_castable_as() {
        let xml = compress_spaces(r#"
<root base="base">
    <a v="x"/>
    <a v="y"/>
    <a v="z"/>
</root>
        "#);

        subtest_eval_xpath("castable_as", &xml, &[
            ( "100 castable as string", "(true)" ),
            ( "100 castable as string?", "(true)" ),
            ( r#"/root/empty castable as string"#, "(false)" ),
            ( r#"/root/empty castable as string?"#, "(true)" ),
            ( r#"/root/a[@v="x"] castable as string"#, "(true)" ),
            ( r#"/root/a[@v="x"] castable as string?"#, "(true)" ),
            ( r#"/root/a castable as string"#, "(false)" ),
            ( r#"/root/a castable as string?"#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // cast as
    //
    #[test]
    fn test_cast_as() {
        let xml = compress_spaces(r#"
<root base="base">
    <a v="x"/>
    <a v="y"/>
    <a v="z"/>
</root>
        "#);

        subtest_eval_xpath("cast_as", &xml, &[
            ( r#"/root/empty cast as string?"#, "()" ),
            ( r#"/root/a[@v="x"] castable as string"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 軸: following
    //
    #[test]
    fn test_axis_following() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <foo img="上">
        <foo img="甲"/>
        <baa img="乙"/>
        <foo img="上上" base="base">
            <foo img="丙"/>
            <baa img="丁"/>
        </foo>
        <foo img="戊"/>
    </foo>
    <foo img="下">
        <baa img="己"/>
        <foo img="庚"/>
        <foo img="下下">
            <baa img="辛"/>
            <foo img="壬"/>
        </foo>
        <baa img="癸"/>
    </foo>
</root>
        "#);

        subtest_xpath("axis_following", &xml, false, &[
            ( "following::*", "戊下己庚下下辛壬癸" ),
            ( "following::foo", "戊下庚下下壬" ),
            ( "following::foo[1]", "戊" ),
            ( "following::baa", "己辛癸" ),
            ( "following::baa[1]", "己" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 軸: preceding
    //
    #[test]
    fn test_axis_preceding() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <foo img="上">
        <foo img="甲"/>
        <baa img="乙"/>
        <foo img="上上">
            <foo img="丙"/>
            <baa img="丁"/>
        </foo>
        <foo img="戊"/>
    </foo>
    <foo img="下">
        <baa img="己"/>
        <foo img="庚" base="base"/>
        <baa img="辛"/>
        <foo img="壬"/>
        <baa img="癸"/>
    </foo>
</root>
        "#);

        subtest_xpath("axis_preceding", &xml, false, &[
            ( "preceding::*", "上甲乙上上丙丁戊己" ),
            ( "preceding::foo", "上甲上上丙戊" ),
            ( "preceding::foo[1]", "戊" ),
            ( "preceding::baa", "乙丁己" ),
            ( "preceding::baa[1]", "己" ),
        ]);
    }

    // -----------------------------------------------------------------
    // element() | element(*) | element(sel)
    // element(sel, type_anno) | element(sel, type_anno?) // 構文解析のみ
    //
    #[test]
    fn test_kind_test_element() {
        let xml = compress_spaces(r#"
<root>
    <a base="base">
        <sel img="z0"/>
        <sel img="z1"/>
        <sel img="z2"/>
        <alt img="a0"/>
        <alt img="a1"/>
    </a>
</root>
        "#);

        subtest_eval_xpath("kind_test_element", &xml, &[
            ( "count(child::element())", "(5)" ),
            ( "count(child::element(*))", "(5)" ),
            ( "count(child::element(sel))", "(3)" ),
            ( "count(child::element(sel, typ))", "(3)" ),
            ( "count(child::element(sel, typ?))", "(3)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // attribute() | attribute(*) | attribute(a)
    // attribute(sel, type_anno)                          // 構文解析のみ
    //
    #[test]
    fn test_kind_test_attribute() {
        let xml = compress_spaces(r#"
<root>
    <a base="base">
        <sel a="1" b="2"/>
    </a>
</root>
        "#);

        subtest_eval_xpath("kind_test_attribute", &xml, &[
            ( "sel/attribute::attribute()", r#"(a="1", b="2")"# ),
            ( "sel/attribute::attribute(*)", r#"(a="1", b="2")"# ),
            ( "sel/attribute::attribute(a)", r#"(a="1")"# ),
            ( "sel/attribute::attribute(a, typ)", r#"(a="1")"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // processing-instruction()
    //
    #[test]
    fn test_kind_test_processing_instruction() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<?style-sheet alt="1" src="sample.css"?>
<?style-sheet alt="2" src="default.css"?>
<?pseudo-style-sheet src="sample.css"?>
<xroot>
    <a base="base">
        <sel img="z0" ans="0" />
        <sel img="z1" ans="1" />
        <sel img="z2" ans="2" />
        <sel img="z3" ans="3" />
        <sel img="z4" ans="4" />
    </a>
</xroot>
        "#);

        subtest_eval_xpath("kind_test_processing_instruction", &xml, &[
            ( "count(/child::processing-instruction())", "(3)" ),
            ( "count(/child::processing-instruction('style-sheet'))", "(2)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // ContextItemExpr
    //
    #[test]
    fn test_context_item() {
        let xml = compress_spaces(r#"
<root>
    <a base="base">
        <b id="b"/>
    </a>
</root>
        "#);

        subtest_eval_xpath("context_item", &xml, &[
            ( ".", r#"(<a base="base">)"# ),
            ( "./b", r#"(<b id="b">)"# ),
            ( "self::a", r#"(<a base="base">)"# ),
            ( r#"self::a[@base="base"]"#, r#"(<a base="base">)"# ),
            ( "self::b", "()" ),
                    // 「self」と明記した場合はAxisSelfであり、
                    // NodeTestを記述できる。
            ( ".::a", "Syntax Error in XPath" ),
            ( ".a", "Syntax Error in XPath" ),
                    // 「.」と書き、さらにNodeTestを記述する構文はない。
            ( r#".[name()="a"]"#, r#"(<a base="base">)"# ),
            ( r#".[@base="base"]"#, r#"(<a base="base">)"# ),
                    // しかし述語は記述できる。

            ( "(1 to 20)[. mod 5 eq 0]", "(5, 10, 15, 20)" ),
        ]);
    }

}

