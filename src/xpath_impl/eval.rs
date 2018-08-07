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
use std::str::FromStr;
use std::usize;

use dom::*;
use xmlerror::*;
use xpath_impl::lexer::*;
use xpath_impl::parser::*;
use xpath_impl::xitem::*;
use xpath_impl::xsequence::*;
use xpath_impl::func::*;
use xpath_impl::oper::*;

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
#[derive(Debug, PartialEq, Clone)]
struct VarNameValue {
    name: String,
    value: XSequence,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EvalEnv {
    doc_order_hash: HashMap<i64, i64>,      // node_id -> 順序番号
    position: usize,                        // 組み込み函数 position() の値
    last: usize,                            // 組み込み函数 last() の値
    var_vec: Vec<VarNameValue>,             // 変数表
                                            // 同名の変数にはスコープ規則を適用
}

fn new_eval_env() -> EvalEnv {
    return EvalEnv{
        doc_order_hash: HashMap::new(),
        position: 0,
        last: 0,
        var_vec: vec!{},
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
    fn set_var(&mut self, name: &str, value: &XSequence) {
        self.var_vec.insert(0, VarNameValue{
            name: String::from(name),
            value: value.clone(),
        });
    }

    // -----------------------------------------------------------------
    //
    fn set_var_item(&mut self, name: &str, value: &XItem) {
        self.var_vec.insert(0, VarNameValue{
            name: String::from(name),
            value: new_singleton(value),
        });
    }

    // -----------------------------------------------------------------
    //
    fn remove_var(&mut self, name: &str) {
        let mut index = usize::MAX;
        for (i, entry) in self.var_vec.iter().enumerate() {
            if entry.name == name {
                index = i;
                break;
            }
        }
        if index != usize::MAX {
            self.var_vec.remove(index as usize);
        }
    }

    // -----------------------------------------------------------------
    //
    fn get_var(&self, name: &str) -> Option<XSequence> {
        for entry in self.var_vec.iter() {
            if entry.name == name {
                return Some(entry.value.clone());
            }
        }
        return None;
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
            // ---------------------------------------------------------
            // (1) 左辺値を評価する。
            //     ノードのみのシーケンスでなければエラー (空シーケンスは可)。
            //
            let left_xnode = get_left(xnode);
            let lhs = if ! is_nil_xnode(&left_xnode) {
                    evaluate_xnode(xseq, &left_xnode, eval_env)?
                } else {
                    new_xsequence()
                };

            // ---------------------------------------------------------
            // (1a) 右辺値の評価が必要ない場合は、そのまま左辺値を返す。
            //
            let right_xnode = get_right(xnode);
            if is_nil_xnode(&right_xnode) {
                return Ok(lhs);
            }

            // ---------------------------------------------------------
            // (1b) 左辺値がノードのみのシーケンスでなければ
            //      エラー (空シーケンスは可)。
            //
            if ! lhs.is_no_atom() {
                return Err(type_error!("Path演算子: ノード以外のアイテムがある。"));
            }

            // ---------------------------------------------------------
            // (2) lhsの各ノードについて、右辺値を評価する。
            //
            let mut node_exists = false;
            let mut atom_exists = false;
            let mut result_seq = new_xsequence();

            for item in lhs.iter() {
                let xseq = new_singleton(&item);
                let val_seq = evaluate_xnode(&xseq, &right_xnode, eval_env)?;

                if val_seq.is_empty() {
                    continue;
                }

                // -----------------------------------------------------
                // (2-1) ノードごとの評価値を合併していく。
                //       評価値がすべてノードのみのシーケンスか否かを
                //       調べておく。
                //
                if val_seq.is_no_atom() {
                    node_exists = true;
                } else {
                    atom_exists = true;
                }
                result_seq.append(&val_seq);

                // -----------------------------------------------------
                // (2-3) ノードと非ノードが混在していればエラー。
                //
                if node_exists && atom_exists {
                    return Err(type_error!("Path演算子: ノードと非ノードが混在している。"));
                }
            }

            // ---------------------------------------------------------
            // (3) 最後に、ノードのみのシーケンスであれば、整列、重複排除する。
            //
            if node_exists {
                let mut nodeset = result_seq.to_nodeset();
                eval_env.sort_by_doc_order(&mut nodeset);
                let sorted_seq = new_xsequence_from_node_array(&nodeset);
                return Ok(sorted_seq);
            } else {
                return Ok(result_seq);
            }
        },

        XNodeType::OperatorMap => {
            // ---------------------------------------------------------
            // (1) 左辺値を評価する。
            //
            let left_xnode = get_left(xnode);
            let lhs = if ! is_nil_xnode(&left_xnode) {
                    evaluate_xnode(xseq, &left_xnode, eval_env)?
                } else {
                    new_xsequence()
                };

            // ---------------------------------------------------------
            // (1a) 右辺値の評価が必要ない場合は、そのまま左辺値を返す。
            //
            let right_xnode = get_right(xnode);
            if is_nil_xnode(&right_xnode) {
                return Ok(lhs);
            }

            // ---------------------------------------------------------
            // (2) lhsの各ノードについて右辺値を評価し、順に合併していく。
            //     整列や重複排除はしない。
            //
            let mut result_seq = new_xsequence();
            for item in lhs.iter() {
                let xseq = new_singleton(&item);
                let val_seq = evaluate_xnode(&xseq, &right_xnode, eval_env)?;
                result_seq.append(&val_seq);
            }

            return Ok(result_seq);
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
            return Ok(xseq.clone());
        }

        XNodeType::ApplyPredicate => {
            // 左辺値 (PrimaryExpr) に対して、右辺値の述語を適用して絞り込む。
            //
            let primary_xnode = &get_left(xnode);
            let postfix_xnode = &get_right(xnode);
            let primary_expr = evaluate_xnode(xseq, primary_xnode, eval_env)?;
            return filter_by_predicate(&primary_expr,
                            &get_left(&postfix_xnode), false, eval_env);
        },

        XNodeType::ApplyArgument => {
            // 左辺値 (PrimaryExpr) を函数と見て、右辺値の引数並びを適用する。
            //
            let primary_xnode = &get_left(xnode);
            let postfix_xnode = &get_right(xnode);
            let primary_expr = evaluate_xnode(xseq, primary_xnode, eval_env)?;
            return apply_argument(xseq, &primary_expr, &postfix_xnode, eval_env);
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

        XNodeType::OperatorConcat => {
            let lhs = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            return fn_concat(&vec!{&lhs, &rhs});
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
                eval_env.set_var_item(var_name.as_str(), xitem);
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                result.append(&rhs);
                eval_env.remove_var(var_name.as_str());
            }
            return Ok(result);
        },

        XNodeType::LetExpr => {
            return evaluate_xnode(xseq, &get_right(xnode), eval_env);
        },

        XNodeType::LetVarBind => {
            // -----------------------------------------------------
            // 左辺値を評価し、変数値として登録した上で、右辺値を評価する。
            //
            let var_value = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let var_name = get_xnode_name(&xnode);

            eval_env.set_var(var_name.as_str(), &var_value);
            let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
            eval_env.remove_var(var_name.as_str());

            return Ok(rhs);
        },

        XNodeType::SomeExpr => {
            return evaluate_xnode(xseq, &get_right(xnode), eval_env);
        },

        XNodeType::SomeVarBind => {
            let var_name = get_xnode_name(&xnode);
            let range = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;

            for xitem in range.iter() {
                eval_env.set_var_item(var_name.as_str(), xitem);
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
                eval_env.set_var_item(var_name.as_str(), xitem);
                let rhs = evaluate_xnode(xseq, &get_right(xnode), eval_env)?;
                if rhs.effective_boolean_value()? == false {
                    return Ok(new_singleton_boolean(false));
                }
                eval_env.remove_var(var_name.as_str());
            }
            return Ok(new_singleton_boolean(true));
        },

        XNodeType::OperatorInstanceOf => {
            let expr_xseq = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let sequence_type_xnode = get_right(xnode);
            let b = match_sequence_type(&expr_xseq, &sequence_type_xnode)?;
            return Ok(new_singleton_boolean(b));
        },

        XNodeType::OperatorCastableAs => {
            let value = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let single_type_xnode = get_right(xnode);
            let type_name_xnode = get_left(&single_type_xnode);
            let type_name = get_xnode_name(&type_name_xnode);
            return Ok(new_singleton_boolean(value.castable_as(&type_name)));
        }

        XNodeType::OperatorCastAs => {
            let value = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;
            let single_type_xnode = get_right(xnode);
            let type_name_xnode = get_left(&single_type_xnode);
            let type_name = get_xnode_name(&type_name_xnode);
            return value.cast_as(&type_name);
        }

        XNodeType::FunctionCall => {
            // rightに連なっているArgumentTopノード群のleft以下にある
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

        XNodeType::InlineFunction |
        XNodeType::NamedFunctionRef |
        XNodeType::PartialFunctionCall => {
            // インライン函数 | 名前付き函数参照:
            // この時点では評価せず、シングルトンとして包んで返す。
            return Ok(new_singleton_xnodeptr(&xnode));
        },

        XNodeType::Map |
        XNodeType::SquareArray |
        XNodeType::CurlyArray => {
            // マップ | 配列 (これも函数の一種として扱う)
            let xitem = convert_xnode_to_map_array(&xnode, &xseq, eval_env)?;
            return Ok(new_singleton(&xitem));
        },

        XNodeType::UnaryLookupByWildcard => {
            if let Ok(xitem_map) = xseq.get_singleton_map() {
                let mut result = new_xsequence();
                for key in xitem_map.map_keys().iter() {
                    result.append(&xitem_map.map_get(key).unwrap());
                }
                return Ok(result);

            } else if let Ok(xitem_array) = xseq.get_singleton_array() {
                let size = xitem_array.array_size();
                let mut result = new_xsequence();
                for i in 1 ..= size {
                    let index = new_xitem_integer(i as i64);
                    result.append(&xitem_array.array_get(&index).unwrap());
                }
                return Ok(result);

            } else {
                return Err(type_error!("lookup: マップ/配列でない。"));
            }
        },

        XNodeType::UnaryLookupByExpr => {
            let expr = evaluate_xnode(xseq, &get_left(xnode), eval_env)?;

            if let Ok(xitem_map) = xseq.get_singleton_map() {
                let mut result = new_xsequence();
                for key in expr.iter() {
                    if let Some(v) = xitem_map.map_get(key) {
                        result.append(&v);
                    } else {
                    }
                }
                return Ok(result);

            } else if let Ok(xitem_array) = xseq.get_singleton_array() {
                let mut result = new_xsequence();
                for index in expr.iter() {
                    if let Some(v) = xitem_array.array_get(index) {
                        result.append(&v);
                    } else {
                    }
                }
                return Ok(result);

            } else {
                return Err(type_error!("lookup: マップ/配列でない。"));
            }
        },

        XNodeType::VarRef => {
            let var_name = get_xnode_name(&xnode);
            if let Some(xseq) = eval_env.get_var(var_name.as_str()) {
                return Ok(xseq);
            } else {
                return Ok(new_xsequence());
            }
        },

        XNodeType::ParenthesizedExpr => {
            let lhs_xnode = get_left(xnode);
            if ! is_nil_xnode(&lhs_xnode) {
                return evaluate_xnode(xseq, &lhs_xnode, eval_env);
            } else {
                return Ok(new_xsequence());
            }
        },

        _ => {
            return Err(cant_occur!(
                "evaluate_xnode: xnode_type = {:?}", xnode_type));
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
            return Err(cant_occur!("match_loc_step: xnode_type: {:?}",
                    get_xnode_type(&xnode)));
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
    // get_left(&xnode) がNilのとき:
    //     get_xnode_name(&xnode): NameTestで照合する名前
    // get_left(&xnode) がXNodeType::KindTestのとき:
    //     そのget_left(&xnode): KindTestで照合する規則
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
// ノードの種類テスト
//
// DocumentTest                                  // ☆
// SchemaElementTest                             // ☆
// SchemaAttributeTest                           // ☆
// NamespaceNodeTest                             // ☆
//                                                  ☆ 未実装 (構文解析のみ)
//
// 2.5.1 Predefined Schema Types
// - 未検証の要素ノードについては、型註釈が「xs:untyped」になる。
//
// 2.5.5.3 Element Test
// - element()、element(*): 任意の要素ノードに合致。
// - element(ElementName): 要素名が一致。
// - element(ElementName, TypeName):
//       要素名が一致し、derives-from(xs:untyped, TypeName) が true、
//       かつ、nilledプロパティーがfalse (i.e. 属性 "xsi:nil" の値が "true" でない)。
// - element(ElementName, TypeName?):
//       要素名が一致し、derives-from(xs:untyped, TypeName) が true。
//       nilledプロパティーはtrueでもfalseでもよい。
// - element(*, TypeName):
//       derives-from(xs:untyped, TypeName) が true、
//       かつ、nilledプロパティーがfalse (i.e. 属性 "xsi:nil" の値が "true" でない)。
// - element(*, TypeName?):
//       derives-from(xs:untyped, TypeName) が true。
//       nilledプロパティーはtrueでもfalseでもよい。
//
// 2.5.5.5 Attribute Test
// - attribute()、attribute(*): 任意の属性ノードに合致。
// - attribute(AttributeName): 属性名が一致。
// - attribute(AttributeName, TypeName):
//       属性名が一致し、derives-from(xs:untyped, TypeName) が true。
// - attribute(*, TypeName):
//       derives-from(xs:untyped, TypeName) が true。
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
            if node_type != NodeType::Element {
                return false;
            }

            let element_name = get_xnode_name(&test_xnode);
                    // 明示的に指定がない場合の既定値は *
            if element_name != "*" && element_name != node.name() {
                return false;
            }

            let type_name_xnode = get_left(&test_xnode);
            let type_name_ex = get_xnode_name(&type_name_xnode);
                    // 明示的に指定がない場合の既定値は xs:anyType?
            let type_name = type_name_ex.trim_right_matches('?');
            let with_q = type_name_ex.ends_with("?");
            if ! derives_from("xs:untyped", &type_name) {
                return false;
            }
            if ! with_q {
                if let Some(p) = node.attribute_value("xsi:nil") {
                    if p == "true" {
                        return false;
                    }
                }
            }

            return true;
        },

        XNodeType::AttributeTest => {
            if node_type != NodeType::Attribute {
                return false;
            }

            let attribute_name = get_xnode_name(&test_xnode);
                    // 明示的に指定がない場合の既定値は *
            if attribute_name != "*" && attribute_name != node.name() {
                return false;
            }

            let type_name_xnode = get_left(&test_xnode);
            let type_name = get_xnode_name(&type_name_xnode);
                    // 明示的に指定がない場合の既定値は xs:anyType
            if ! derives_from("xs:untyped", &type_name) {
                return false;
            }

            return true;
        },

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

        XNodeType::NamespaceNodeTest => {
            // NamespaceNodeTestは未実装。
        },

        XNodeType::AnyKindTest => {
            return true;
        },

        _ => {},
    }

    return false;
}

// ---------------------------------------------------------------------
// Location Stepで得たシーケンスに対して、述語を順次適用してしぼり込み、
// 最終的に得られるシーケンスを返す。
//
// apply_postfix() の処理の一部といった恰好だが、構文上、述語のみ
// 並んでいる状況に対応する。
//
// xseq: AxisNNNNNNNN (Location Step) を評価して得たシーケンス (ノード集合)。
// xnode: AxisNNNNNNNNの右。
//    rightをたどったノードはすべてPredicate{Rev}Topであり、
//    そのleft以下に述語式の構文木がある。
//    各ノードに対して述語を適用し、trueであったもののみにしぼり込む。
//
// AxisNNNNNNNN --- Predicate{Rev}Top --- Predicate{Rev}Top ---...
//  (NameTest)              |                     |
//                        (Expr)                (Expr)
//
fn filter_by_predicates(xseq: &XSequence, xnode: &XNodePtr,
            eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut curr_xseq = xseq.clone();
    let mut curr_xnode = xnode.clone();

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

// ---------------------------------------------------------------------
// インライン函数/マップ/配列に、引数を適用する。
// xseq: 引数の値を評価する際、対象とするシーケンス (文脈ノード)。
// curr_xseq: シングルトン。
//            XItem: 典型的にはInlineFunction型のXNodePtr。
//            また、MapやSquareArrayも函数であって、キーや指標を
//            引数として渡し、値を取り出すことができる。
//            このXNodePtrを取り出し、インライン函数として実行する。
// InlineFunction --- ReturnType ------- Param ------- Param ---...
//       |                |            (varname)     (varname)
//       |                |                |             |
//       |          (SequenceType)   (SequenceType)(SequenceType)
//       |
//     Expr (FunctionBody) ---...
//       |
//      ...
//                      ☆戻り値型も照合すること。
//
// curr_xnode: ArgumentListTop型のXNodePtr。インライン函数に渡す引数並び。
//      ArgumentListTop
//             |
//        ArgumentTop --- ArgumentTop ---...
//             |               | 第2引数
//             |           OpLiteral
//             | 第1引数
//            OpEQ --- (rhs)
//             |
//           (lhs)
//
// eval_env: 評価環境。
//
fn apply_argument(xseq: &XSequence, curr_xseq: &XSequence,
                curr_xnode: &XNodePtr,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let arg_xnode = get_left(&curr_xnode);

    let mut argument_xseq: Vec<XSequence> = vec!{};
    let mut curr_arg_top = arg_xnode.clone();
    while ! is_nil_xnode(&curr_arg_top) {
        let arg_expr = get_left(&curr_arg_top);
        let val = evaluate_xnode(xseq, &arg_expr, eval_env)?;
        argument_xseq.push(val);
        curr_arg_top = get_right(&curr_arg_top);
    }

    // -----------------------------------------------------------------
    // インライン函数
    //
    if let Ok(inline_func_xnode) = curr_xseq.get_singleton_xnodeptr() {
        match get_xnode_type(&inline_func_xnode) {
            XNodeType::InlineFunction => {
                return call_inline_func(&inline_func_xnode,
                            argument_xseq, xseq, eval_env);
            },
            _ => {}
        }
    }

    // -----------------------------------------------------------------
    // マップ
    //
    if let Ok(map_item) = curr_xseq.get_singleton_map() {
        let key = argument_xseq[0].get_singleton_item()?;
        if let Some(v) = map_item.map_get(&key) {
            return Ok(v);
        } else {
            return Err(dynamic_error!(
                    "map_lookup: key = {}: 値が見つからない。", key));
        }
    }

    // -----------------------------------------------------------------
    // 配列
    //
    if let Ok(array_item) = curr_xseq.get_singleton_array() {
        let index_item = argument_xseq[0].get_singleton_item()?;
        if let Some(v) = array_item.array_get(&index_item) {
            return Ok(v);
        } else {
            return Err(dynamic_error!(
                "Array index ({}) out of bounds.", index_item));
        }
    }

    return Err(cant_occur!(
                "apply_argument: インライン函数/マップ/配列でない。"));

}

// ---------------------------------------------------------------------
//
fn call_inline_func(inline_func_xnode: &XNodePtr,
                argument_xseq: Vec<XSequence>,
                context_xseq: &XSequence,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    // -----------------------------------------------------------------
    // 函数定義の実体を取り出す。
    //
    let func_body_xnode = get_left(&inline_func_xnode);

    // -----------------------------------------------------------------
    // 仮引数名を調べる。
    // // 個数、引数型も照合すること。
    //
    let mut param_names: Vec<String> = vec!{};
    let mut sequence_types: Vec<XNodePtr> = vec!{};
    let return_type = get_right(&inline_func_xnode);
    let mut param_xnode = get_right(&return_type);
    while ! is_nil_xnode(&param_xnode) {
        param_names.push(get_xnode_name(&param_xnode));
        sequence_types.push(get_left(&param_xnode));
        param_xnode = get_right(&param_xnode);
    }

    // -----------------------------------------------------------------
    // 実引数の値を変数 (仮引数) に束縛する。
    //
    for (i, val) in argument_xseq.iter().enumerate() {
        if match_sequence_type(&val, &sequence_types[i])? == false {
            return Err(type_error!(
                    "インライン函数: 引数の型が合致していない: {}。",
                    val.to_string()));
        }
        eval_env.set_var(&param_names[i], &val);
    }

    // -----------------------------------------------------------------
    // インライン函数を実行する。
    //
    let value = evaluate_xnode(context_xseq, &func_body_xnode, eval_env)?;

    // -----------------------------------------------------------------
    // 変数 (仮引数) を削除する。
    //
    let mut i: i64 = (param_names.len() as i64) - 1;
    while 0 <= i {
        eval_env.remove_var(&param_names[i as usize]);
        i -= 1;
    }

    return Ok(value);
}

// ---------------------------------------------------------------------
// 名前付き函数参照を、引数を渡して呼び出す。
//
fn call_named_func(func_xnode: &XNodePtr,
                argument_xseq: Vec<XSequence>,
                context_xseq: &XSequence,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    // -----------------------------------------------------------------
    // 函数定義の実体を取り出す。
    //
    let func_name = get_xnode_name(&func_xnode);
    let v: Vec<&str> = func_name.split("#").collect();
    let arity = atoi(v[1]) as usize;
    if arity != argument_xseq.len() {
        return Err(type_error!(
                "名前付き函数参照 ({}): 引数の個数が合致しない: {}。",
                func_name, argument_xseq.len()));
    }

    return evaluate_function(&v[0], &argument_xseq, context_xseq, eval_env);
}

// ---------------------------------------------------------------------
// 静的函数の部分函数呼び出し。
//
// func_xnode: 部分函数呼び出しの構文木
//             引数のうちArgumentTopは、普通の静的函数と同様、
//             文脈ノードに対してleft以下を評価し、引数として函数に渡す。
//             ArgumentPlaceholderである場合はargument_xseq[i]を
//             引数として函数に渡す。
//
// PartialFunctionCall --- ArgumentTop --- ArgumentPlaceholder ---...
//    (func_name)               |                  (第2引数)
//                             ... (第1引数)
//
// argument_xseq: 長さはArgumentPlaceholderの個数と同じ。
//                argument_xseq[i]をArgumentPlaceholder部分の引数として
//                函数に渡す。
//
// context_xseq: 文脈シーケンス。
//               ArgumentTopである引数は、文脈シーケンスに対して評価する。
//               関数の評価も文脈シーケンスに対しておこなう。
//
// eval_env: 評価環境 (変数の値など)。
//
fn call_partial_func(func_xnode: &XNodePtr,
                argument_xseq: Vec<XSequence>,
                context_xseq: &XSequence,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    let mut args_array: Vec<XSequence> = vec!{};
    let mut curr_xnode = get_right(&func_xnode);
    let mut i = 0;
    while ! is_nil_xnode(&curr_xnode) {
        match get_xnode_type(&curr_xnode) {
            XNodeType::ArgumentTop => {
                let arg = evaluate_xnode(context_xseq, &get_left(&curr_xnode), eval_env)?;
                args_array.push(arg);
            },
            XNodeType::ArgumentPlaceholder => {
                if i >= argument_xseq.len() {
                    return Err(dynamic_error!("函数呼び出しの「?」に渡すべき引数が足りない。"));
                }
                args_array.push(argument_xseq[i].clone());
                i += 1;
            },
            _ => {
                return Err(cant_occur!("call_partial_func: ArgumentTopでもArgumentPlaceholderでもでない。"));
            }
        }
        curr_xnode = get_right(&curr_xnode);
    }

    return evaluate_function(&get_xnode_name(&func_xnode),
            &mut args_array, context_xseq, eval_env);

}

// ---------------------------------------------------------------------
// 函数呼び出し。
// インライン函数、名前付き函数参照、部分函数。
//
pub fn call_function(func_xnode: &XNodePtr,
                argument_xseq: Vec<XSequence>,
                context_xseq: &XSequence,
                eval_env: &mut EvalEnv) -> Result<XSequence, Box<Error>> {

    match get_xnode_type(&func_xnode) {
        XNodeType::InlineFunction => {
            return call_inline_func(func_xnode, argument_xseq, context_xseq, eval_env);
        },
        XNodeType::NamedFunctionRef => {
            return call_named_func(func_xnode, argument_xseq, context_xseq, eval_env);
        },
        XNodeType::PartialFunctionCall => {
            return call_partial_func(func_xnode, argument_xseq, context_xseq, eval_env);
        },
        _ => {
            return Err(cant_occur!("call_h_function: XNodeType = {:?}",
                    get_xnode_type(&func_xnode)));
        },
    }
}

// ---------------------------------------------------------------------
// シーケンスの型を、
// シーケンス型の定義 (XNodeType::SequenceTypeであるxnodeで表現) と照合する。
//
// [ 79] SequenceType ::= ("empty-sequence" "(" ")")
//                      | (ItemType OccurenceIndicator?)
// [ 80] OccurrenceIndicator ::= "?" | "*" | "+"
// [ 81] ItemType ::= KindTest
//                  | ("item" "(" ")")
//                  | FunctionTest
//                  | MapTest
//                  | ArrayTest
//                  | AtomicOrUnionType
//                  | ParenthesizedItemType                            ☆
// [ 82] AtomicOrUnionType ::= EQName
// [102] FunctionTest ::= AnyFunctionTest
//                      | TypedFunctionTest
//
fn match_sequence_type(xseq: &XSequence, xnode: &XNodePtr) -> Result<bool, Box<Error>> {
    if get_xnode_type(xnode) != XNodeType::SequenceType {
        return Err(cant_occur!(
                "match_sequence_type: xnodeがSequenceTypeでない: {:?}。",
                get_xnode_type(xnode)));
    }

    let type_xnode = get_left(xnode);

    // -----------------------------------------------------------------
    // empty-sequence()
    // xseqが空シーケンスか否かのみ判定する。
    //
    if get_xnode_type(&type_xnode) == XNodeType::EmptySequenceTest {
        return Ok(xseq.is_empty());
    }

    // -----------------------------------------------------------------
    // それ以外の場合、まず出現数の指定 (?、*、+、空) を照合する。
    // シーケンスに含まれるアイテムの個数が指定を満たしていなければ、
    // この時点でfalseとする。
    //
    if match_occurence(xseq, &get_xnode_name(xnode))? == false {
        return Ok(false);
    }

    // -----------------------------------------------------------------
    // ItemTypeに応じて、シーケンスの各アイテムを判定する。
    //
    match get_xnode_type(&type_xnode) {
        XNodeType::KindTest => {            // element(name) など
            return Ok(match_sequence_kind_test(xseq, &type_xnode));
        },
        XNodeType::ItemTest => {            // item()
            return Ok(match_sequence_item_test(xseq));
        },

        XNodeType::AnyFunctionTest => {     // function(*)
            return Ok(match_sequence_any_function_test(xseq));
        },

        XNodeType::TypedFunctionTest => {
            return match_sequence_typed_function_test(xseq, &type_xnode);
        },

        XNodeType::ArrayTest => {
            return match_sequence_array_test(xseq, &type_xnode);
        },

        XNodeType::MapTest => {
            return match_sequence_map_test(xseq, &type_xnode);
        },

        XNodeType::AtomicOrUnionType => {
            return Ok(match_sequence_atomic_or_union_type(xseq, &type_xnode));
        },

        XNodeType::ParenthesizedItemType => {
            // =====================================================
            return Ok(false);
        },

        _ => {
            return Err(cant_occur!(
                "match_sequence_type: xnodeの左辺値のxnode_typeが想定外: {:?}",
                type_xnode));
        },
    }
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::KindTest である場合。
//
// xseq のアイテムすべてについて、NodePtrであり、かつ、そのノード型が
// 指定どおりであることを確かめる。
//
fn match_sequence_kind_test(xseq: &XSequence, type_xnode: &XNodePtr) -> bool {
    for xitem in xseq.iter() {
        if let Some(node) = xitem.as_nodeptr() {
            if match_kind_test(&node, &type_xnode) == false {
                return false;
            }
        } else {
            return false;
        }
    }
    return true;
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::ItemTest である場合。
//
// xseq の要素すべてについて、アイテムであることを確かめる。
//
fn match_sequence_item_test(xseq: &XSequence) -> bool {
    for xitem in xseq.iter() {
        if ! xitem.is_item() {
            return false;
        }
    }
    return true;
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::AnyFunctionTest である場合。
//
// xseq の要素すべてについて、函数 (マップ/配列を含む) であることを確かめる。
//
fn match_sequence_any_function_test(xseq: &XSequence) -> bool {
    for xitem in xseq.iter() {
        let mut is_function = false;
        if let Ok(xnode) = xitem.get_as_raw_xnodeptr() {
            match get_xnode_type(&xnode) {
                XNodeType::InlineFunction |
                XNodeType::NamedFunctionRef |
                XNodeType::PartialFunctionCall => {
                    is_function = true;
                },
                _ => {},
            }
        }
        if let Ok(_) = xitem.get_as_raw_array() {
            is_function = true;
        }
        if let Ok(_) = xitem.get_as_raw_map() {
            is_function = true;
        }
        if ! is_function {
            return false;
        }
    }
    return true;
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::TypedFunctionTest である場合。
//
fn match_sequence_typed_function_test(xseq: &XSequence, type_xnode: &XNodePtr) -> Result<bool, Box<Error>> {

    // -----------------------------------------------------------------
    // 戻り値型および引数型の定義をsignature_xnodeに取り出す。
    // signature_xnode[0]: 戻り値型の定義。
    // signature_xnode[n]: 第n引数の型の定義。
    // signature_xnode.len() - 1: 引数の個数。
    //
    let mut signature_xnode: Vec<XNodePtr> = vec!{};
    let mut curr = get_right(&type_xnode);
    while ! is_nil_xnode(&curr) {
        signature_xnode.push(get_left(&curr));
        curr = get_right(&curr);
    }

    // -----------------------------------------------------------------
    //
    for xitem in xseq.iter() {
        let mut is_function = false;

        // -------------------------------------------------------------
        //
        if let Ok(xnode) = xitem.get_as_raw_xnodeptr() {

            let sig_xnode: XNodePtr;

            match get_xnode_type(&xnode) {

                // -----------------------------------------------------
                // (名前付き函数参照)
                // シグニチャー表を引き、テキスト形式のシグニチャーを
                // 構文木に変換する。
                //
                XNodeType::NamedFunctionRef => {
                    let func_name = get_xnode_name(&xnode);
                    let signature = get_function_signature(func_name.as_str());
                    if signature == "" {
                        return Ok(false);
                    }
                    let mut lex = Lexer::new(&signature)?;
                    sig_xnode = parse_function_test(&mut lex)?;
                },

                // -----------------------------------------------------
                // (インライン函数)
                //
                XNodeType::InlineFunction => {
                    sig_xnode = xnode.clone();
                },

                XNodeType::PartialFunctionCall => {
                    sig_xnode = xnode.clone();
                },
                _ => {
                    return Ok(false);
                },
            }

            let mut i = 0;
            let mut curr = get_right(&sig_xnode);
            while ! is_nil_xnode(&curr) {
                if signature_xnode.len() <= i {     // 引数が多い。
                    return Ok(false);
                }
                let type_xnode = get_left(&curr);
                if i == 0 {                 // 戻り値型
                    if ! subtype(&type_xnode, &signature_xnode[i]) {
                        return Ok(false);
                    }
                } else {                    // 引数型
                    if ! subtype(&signature_xnode[i], &type_xnode) {
                        return Ok(false);
                    }
                }
                curr = get_right(&curr);
                i += 1;
            }
            if signature_xnode.len() != i {         // 引数が少ない。
                return Ok(false);
            }
            // -------------------------------------------------
            //

            is_function = true;
        }

        // -------------------------------------------------------------
        // (マップの場合)
        // キーと値の型を照合する。
        //
        if let Ok(xseq_map) = xitem.get_as_raw_map() {
            if signature_xnode.len() != 2 {
                return Ok(false);
            }
            let key_type = get_xnode_name(&get_left(&signature_xnode[1]));
            if ! match_map_sequence_type(&xseq_map, &key_type, &signature_xnode[0])? {
                return Ok(false);
            }
            is_function = true;
        }

        // -------------------------------------------------------------
        // (配列の場合)
        // 第1引数がintegerであることを確かめ、値の型を照合する。
        //
        if let Ok(xseq_array) = xitem.get_as_raw_array() {
            if signature_xnode.len() != 2 {
                return Ok(false);
            }
            let xseq_int_1 = new_singleton_integer(1);
            if ! match_sequence_type(&xseq_int_1, &signature_xnode[1])? {
                return Ok(false);
            }

            if ! match_array_sequence_type(&xseq_array, &signature_xnode[0])? {
                return Ok(false);
            }
            is_function = true;
        }

        // -------------------------------------------------------------
        // 上記のどの種類にも当てはまらなければ、
        // (型付き) 函数ではないことになる。
        //
        if ! is_function {
            return Ok(false);
        }
    }
    return Ok(true);
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::ArrayTest である場合。
//
// xseq の要素すべてについて、配列であって、その要素のシーケンス型も
// 定義に合致していることを確かめる。
//
fn match_sequence_array_test(xseq: &XSequence, type_xnode: &XNodePtr) -> Result<bool, Box<Error>> {
    let element_type = get_left(&type_xnode);
    for xitem in xseq.iter() {
        if let Ok(xseq_array) = xitem.get_as_raw_array() {
            if ! match_array_sequence_type(&xseq_array, &element_type)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    return Ok(true);
}

// ---------------------------------------------------------------------
// 配列 (XSeqArray) の各要素の型が、element_type に合致するか否かを判定する。
//
fn match_array_sequence_type(xseq_array: &XSeqArray, element_type: &XNodePtr) -> Result<bool, Box<Error>> {
    for i in 1 ..= xseq_array.array_size() {
        let index = new_xitem_integer(i as i64);
        if let Some(item) = xseq_array.array_get(&index) {
            if ! match_sequence_type(&item, &element_type)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    return Ok(true);
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::MapTest である場合。
//
// xseq の要素すべてについて、マップであって、そのキーおよび値のシーケンス型も
// 定義に合致していることを確かめる。
//
fn match_sequence_map_test(xseq: &XSequence, type_xnode: &XNodePtr) -> Result<bool, Box<Error>> {
    let key_type = get_xnode_name(&get_left(&type_xnode));
    let value_type = get_right(&type_xnode);
    for xitem in xseq.iter() {
        if let Ok(xseq_map) = xitem.get_as_raw_map() {
            if ! match_map_sequence_type(&xseq_map, &key_type, &value_type)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    return Ok(true);
}

// ---------------------------------------------------------------------
// マップ (XSeqMap) のキーと値の型が、key_type、value_type に
// 合致するか否かを判定する。
//
fn match_map_sequence_type(xseq_map: &XSeqMap,
                key_type: &str,
                value_type: &XNodePtr) -> Result<bool, Box<Error>> {
    for key in xseq_map.map_keys().iter() {
        if ! derives_from(key.xs_type().as_str(), &key_type) {
            return Ok(false);
        }
        if let Some(val) = xseq_map.map_get(&key) {
            if ! match_sequence_type(&val, &value_type)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    return Ok(true);
}

// ---------------------------------------------------------------------
// シーケンス xseq の型を、シーケンス型の定義 type_xnode と照合する。
// type_xnode が XNodeType::AtomicOrUnionType である場合。
//
fn match_sequence_atomic_or_union_type(xseq: &XSequence, type_xnode: &XNodePtr) -> bool {
    let type_name = get_xnode_name(type_xnode);
    for xitem in xseq.iter() {
        if ! derives_from(xitem.xs_type().as_str(), &type_name) {
            return false;
        }
    }
    return true;
}

// ---------------------------------------------------------------------
// シーケンスに含まれるアイテムの個数 (OccurenceIndicator) を照合する。
// indicator: ? | * | + | ""
//
fn match_occurence(xseq: &XSequence, indicator: &str) -> Result<bool, Box<Error>> {
    let len = xseq.len();
    match indicator {
        "?" => return Ok(len <= 1),
        "*" => return Ok(true),
        "+" => return Ok(1 <= len),
        ""  => return Ok(len == 1),
        _   => return Err(cant_occur!(
                        "match_occurence: bad indicator \"{}\".",
                        indicator)),
    }
}

// ---------------------------------------------------------------------
// 2.5.6.1 The judgement subtype(A, B)
//
// SequenceType A が、SequenceType B のサブタイプであるか否か判定する。
//
const S_EMPTY_SEQ: usize = 0;
const S_OCCUR_Q: usize = 1;
const S_OCCUR_A: usize = 2;
const S_OCCUR_1: usize = 3;
const S_OCCUR_P: usize = 4;
const S_XS_ERROR: usize = 5;
const SUBTYPE_TBL: [ [i64; 6]; 6 ] = [
    [ 1, 1, 1, 0, 0, 0 ],
    [ 0, 9, 9, 0, 0, 0 ],
    [ 0, 0, 9, 0, 0, 0 ],
    [ 0, 9, 9, 9, 9, 0 ],
    [ 0, 0, 9, 0, 9, 0 ],
    [ 1, 1, 1, 1, 1, 1 ],
];

fn subtype(a: &XNodePtr, b: &XNodePtr) -> bool {
    match SUBTYPE_TBL[subtype_entry(a)][subtype_entry(b)] {
        0 => return false,
        1 => return true,
        _ => return subtype_itemtype(&get_left(a), &get_left(b)),
    }
}

fn subtype_entry(xnode: &XNodePtr) -> usize {
    // assert: get_xnode_type(xnode) == XNodeType::SequenceType

    let type_xnode = get_left(xnode);
    if get_xnode_type(&type_xnode) == XNodeType::EmptySequenceTest {
        return S_EMPTY_SEQ;
    }
    if get_xnode_type(&type_xnode) == XNodeType::AtomicOrUnionType &&
       get_xnode_name(&type_xnode) == "xs:error" {
        match get_xnode_name(xnode).as_str() {
            "?" => return S_EMPTY_SEQ,
            "*" => return S_EMPTY_SEQ,
            ""  => return S_XS_ERROR,
            "+" => return S_XS_ERROR,
            _   => return S_XS_ERROR,
        }
    } else {
        match get_xnode_name(xnode).as_str() {
            "?" => return S_OCCUR_Q,
            "*" => return S_OCCUR_A,
            ""  => return S_OCCUR_1,
            "+" => return S_OCCUR_P,
            _   => return S_OCCUR_1,
        }
    }
}

// ---------------------------------------------------------------------
// 2.5.6.2 The judgement subtype-itemtype(Ai, Bi)
//
// ItemType Ai が、ItemType Bi のサブタイプであるか否か判定する。
// 
// ai, bi: xnode (XNodeType::SequenceType) の左辺値であるXNodePtr。
//
fn subtype_itemtype(ai: &XNodePtr, bi: &XNodePtr) -> bool {

    // -----------------------------------------------------------------
    // 1. Ai and Bi are AtomicOrUnionTypes, and derives-from(Ai, Bi) returns true.
    // 2. Ai is a pure union type, and every type t in the transitive membership of Ai satisfies subtype-itemType(t, Bi).
    // 3. Ai is xs:error and Bi is a generalized atomic type.
    //
    if get_xnode_type(ai) == XNodeType::AtomicOrUnionType &&
       get_xnode_type(bi) == XNodeType::AtomicOrUnionType {
        if derives_from(&get_xnode_name(ai), &get_xnode_name(bi)) { // 1. 2.
            return true;
        }
        if get_xnode_name(ai) == "xs:error" {               // 3.
            return true;
        }
    }

    // -----------------------------------------------------------------
    // 4. Bi is item().
    //
    if get_xnode_type(bi) == XNodeType::ItemTest {
        return true;
    }

    // -----------------------------------------------------------------
    // BiがKindTestのとき:
    // 5. Bi is node() and Ai is a KindTest.
    // 6. Bi is text() and Ai is also text().
    // 7. Bi is comment() and Ai is also comment().
    // 8. Bi is namespace-node() and Ai is also namespace-node().
    // 9. Bi is processing-instruction() and
    //    Ai is either processing-instruction() or
    //                 processing-instruction(N) for any name N.
    // 10. Bi is processing-instruction(Bn), and
    //     Ai is also processing-instruction(Bn).
    //
//  --------------------------------------------------------------------- ☆
    // 11. Bi is document-node() and Ai is either document-node() or document-node(E) for any ElementTest E.
    // 12. Bi is document-node(Be) and Ai is document-node(Ae), and subtype-itemtype(Ae, Be).

    // 13. Bi is either element() or element(*), and Ai is an ElementTest.
    // 14. Bi is either element(Bn) or element(Bn, xs:anyType?),
    //     the expanded QName of An equals the expanded QName of Bn,
    //     and Ai is either element(An) or element(An, T) or element(An, T?) for any type T.
    // 15. Bi is element(Bn, Bt), the expanded QName of An equals the expanded QName of Bn, Ai is element(An, At), and derives-from(At, Bt) returns true.
    // 16. Bi is element(Bn, Bt?), the expanded QName of An equals the expanded QName of Bn, Ai is either element(An, At) or element(An, At?), and derives-from(At, Bt) returns true.
    // 17. Bi is element(*, Bt), Ai is either element(*, At) or element(N, At) for any name N, and derives-from(At, Bt) returns true.
    // 18. Bi is element(*, Bt?), Ai is either element(*, At), element(*, At?), element(N, At), or element(N, At?) for any name N, and derives-from(At, Bt) returns true.

    // 19. Bi is schema-element(Bn), Ai is schema-element(An), and every element declaration that is an actual member of the substitution group of An is also an actual member of the substitution group of Bn.
    //    Note:
    //    The fact that P is a member of the substitution group of Q does not mean that every element declaration in the substitution group of P is also in the substitution group of Q. For example, Q might block substitution of elements whose type is derived by extension, while P does not.

    // 20. Bi is either attribute() or attribute(*), and Ai is an AttributeTest.
    // 21. Bi is either attribute(Bn) or attribute(Bn, xs:anyType), the expanded QName of An equals the expanded QName of Bn, and Ai is either attribute(An), or attribute(An, T) for any type T.
    // 22. Bi is attribute(Bn, Bt), the expanded QName of An equals the expanded QName of Bn, Ai is attribute(An, At), and derives-from(At, Bt) returns true.
    // 23. Bi is attribute(*, Bt), Ai is either attribute(*, At), or attribute(N, At) for any name N, and derives-from(At, Bt) returns true.

    // 24. Bi is schema-attribute(Bn), the expanded QName of An equals the expanded QName of Bn, and Ai is schema-attribute(An).
    //
    if get_xnode_type(bi) == XNodeType::KindTest {
        match get_xnode_type(&get_left(bi)) {
            XNodeType::AnyKindTest => {                     // 5.
                if get_xnode_type(ai) == XNodeType::KindTest {
                    return true;
                }
            },
            XNodeType::TextTest => {                        // 6.
                if get_xnode_type(ai) == XNodeType::KindTest &&
                   get_xnode_type(&get_left(ai)) == XNodeType::TextTest {
                    return true;
                }
            },
            XNodeType::CommentTest => {                     // 7.
                if get_xnode_type(ai) == XNodeType::KindTest &&
                   get_xnode_type(&get_left(ai)) == XNodeType::CommentTest {
                    return true;
                }
            },
            XNodeType::NamespaceNodeTest => {               // 8.
                if get_xnode_type(ai) == XNodeType::KindTest &&
                   get_xnode_type(&get_left(ai)) == XNodeType::NamespaceNodeTest {
                    return true;
                }
            },
            XNodeType::PITest => {                          // 9. 10.
                let pi_name = get_xnode_name(bi);
                if pi_name == "" {
                    if get_xnode_type(ai) == XNodeType::KindTest &&
                       get_xnode_type(&get_left(ai)) == XNodeType::PITest {
                        return true;
                    }
                } else {
                    if get_xnode_type(ai) == XNodeType::KindTest &&
                       get_xnode_type(&get_left(ai)) == XNodeType::PITest &&
                       get_xnode_name(&get_left(ai)) == pi_name {
                        return true;
                    }
                }
            },
            _ => {},
        }
    }

    // -----------------------------------------------------------------
    // 25. Bi is function(*), Ai is a FunctionTest.
    //
    if get_xnode_type(bi) == XNodeType::AnyFunctionTest {
        match get_xnode_type(ai) {
            XNodeType::AnyFunctionTest |
            XNodeType::TypedFunctionTest => {
                return true;
            },
            _ => {},
        }
    }

    // -----------------------------------------------------------------
    // 26. Bi is function(Ba_1, Ba_2, ... Ba_N) as Br,
    //     Ai is function(Aa_1, Aa_2, ... Aa_M) as Ar,
    //   where
    //     N (arity of Bi) equals M (arity of Ai);
    //     subtype(Ar, Br);
    //     and for values of I between 1 and N, subtype(Ba_I, Aa_I). 
    //
    if get_xnode_type(bi) == XNodeType::TypedFunctionTest &&
       get_xnode_type(ai) == XNodeType::TypedFunctionTest {

        let mut bi_signature_xnode: Vec<XNodePtr> = vec!{};
        let mut curr = get_right(&bi);
        while ! is_nil_xnode(&curr) {
            bi_signature_xnode.push(get_left(&curr));
            curr = get_right(&curr);
        }

        let mut ai_signature_xnode: Vec<XNodePtr> = vec!{};
        let mut curr = get_right(&ai);
        while ! is_nil_xnode(&curr) {
            ai_signature_xnode.push(get_left(&curr));
            curr = get_right(&curr);
        }

        if bi_signature_xnode.len() == ai_signature_xnode.len() {
            let mut is_fail = false;
            if ! subtype(&ai_signature_xnode[0], &bi_signature_xnode[0]) {
                is_fail = true;
            }
            for i in 1 .. bi_signature_xnode.len() {
                if ! subtype(&bi_signature_xnode[i], &ai_signature_xnode[i]) {
                    is_fail = true;
                }
            }
            if ! is_fail {
                return true;
            }
        }
    }

    // -----------------------------------------------------------------
    // 27. Ai is map(K, V), for any K and V and Bi is map(*).
    // 28. Ai is map(Ka, Va) and Bi is map(Kb, Vb),
    //     where subtype-itemtype(Ka, Kb) and subtype(Va, Vb).
    //
    // 27のBiは、map(xs:anyAtomicType, item()*) と同等として
    // 構文木が作られているので、28.の場合のみ考えればよい。
    // AiとBiそれぞれについて、キーと値のxnodeを取り出し、subtype関係を調べる。
    //
    //     MapTest ------- SequenceType ({ai,bi}_val)
    //        |                ...
    //        | ({ai,bi}_key)
    // AtomicOrUnionType
    //      (...)
    //
    if get_xnode_type(ai) == XNodeType::MapTest &&
       get_xnode_type(bi) == XNodeType::MapTest {
        let ai_key = get_left(&ai);
        let ai_val = get_right(&ai);
        let bi_key = get_left(&bi);
        let bi_val = get_right(&bi);
        if subtype_itemtype(&ai_key, &bi_key) &&
           subtype(&ai_val, &bi_val) {
            return true;
        }
    }

    // -----------------------------------------------------------------
    // 29. Ai is map(*)
    //       (or, because of the transitivity rules, any other map type),
    //     and Bi is function(*).
    //
    if get_xnode_type(ai) == XNodeType::MapTest &&
       get_xnode_type(bi) == XNodeType::AnyFunctionTest {
        return true;
    }

    // -----------------------------------------------------------------
    // 30. Ai is map(*)
    //       (or, because of the transitivity rules, any other map type),
    //     and Bi is function(xs:anyAtomicType) as item()*.
    // 35. Ai is map(K, V), and Bi is function(xs:anyAtomicType) as V?.
    //
    // 30のAiがmap(*)の場合、map(xs:anyAtomicType, item()*) と同等として
    // 構文木が作られているので、特別扱いする必要はない。
    // AiとBiそれぞれについて、キーと値のxnodeを取り出し、subtype関係を調べる。
    //
    //     MapTest ------- SequenceType (ai_val)
    //        |                ...
    //        | (ai_key)
    // AtomicOrUnionType
    //      (...)
    //
    // TypedFunctionTest --- ReturnType ---------------- Param
    //                           |                         |
    //                      SequenceType (bi_val)     SequenceType
    //                                                     |
    //                                                     |   (bi_key)
    //                                              AtomicOrUnionType
    //                                                   (...)
    //
    if get_xnode_type(ai) == XNodeType::MapTest &&
       get_xnode_type(bi) == XNodeType::TypedFunctionTest {
        let ai_key = get_left(&ai);                 // AtomicOrUnionType
        let ai_val = get_right(&ai);                // SequenceType
        let ret_xnode = get_right(&bi);           // ReturnType
        let bi_val = get_left(&ret_xnode);          // SequenceType
        let param_xnode = get_right(&ret_xnode);  // Param
        let seq_xnode = get_left(&param_xnode);   // SequenceType
        let bi_key = get_left(&seq_xnode);          // AtomicOrUnionType
        if subtype_itemtype(&ai_key, &bi_key) &&
           subtype(&ai_val, &bi_val) {
            return true;
        }
    }

    // -----------------------------------------------------------------
    // 31. Ai is array(X) and Bi is array(*).
    // 32. Ai is array(X) and Bi is array(Y), and subtype(X, Y) is true.
    //
    // 31.のarray(*)はarray(item()*) と同等として
    // 構文木が作られているので、32の場合のみ考えればよい。
    // AiとBiそれぞれについて、要素のxnodeを取り出し、subtype関係を調べる。
    //
    //   ArrayTest
    //       |
    //       | ({ai,bi}_elem)
    //  SequenceType
    //      ...
    //
    if get_xnode_type(ai) == XNodeType::ArrayTest &&
       get_xnode_type(bi) == XNodeType::ArrayTest {
        let ai_elem = get_left(&ai);
        let bi_elem = get_left(&bi);
        if subtype(&ai_elem, &bi_elem) {
            return true;
        }
    }

    // -----------------------------------------------------------------
    // 33. Ai is array(*)
    //       (or, because of the transitivity rules, any other array type)
    //     and Bi is function(*).
    //
    if get_xnode_type(ai) == XNodeType::ArrayTest &&
       get_xnode_type(bi) == XNodeType::AnyFunctionTest {
        return true;
    }

    // -----------------------------------------------------------------
    // 34. Ai is array(*)
    //       (or, because of the transitivity rules, any other array type)
    //     and Bi is function(xs:integer) as item()*.
    // 36. Ai is array(X) and Bi is function(xs:integer) as X.
    //
    // 34のAiがarray(*)の場合、array(item()*) と同等として
    // 構文木が作られているので、特別扱いする必要はない。
    // AiとBiそれぞれについて、要素のxnodeを取り出し、subtype関係を調べる。
    // 
    if get_xnode_type(ai) == XNodeType::ArrayTest &&
       get_xnode_type(bi) == XNodeType::TypedFunctionTest {
        let ai_elem = get_left(&ai);                // SequenceType
        let ret_xnode = get_right(&bi);           // ReturnType
        let bi_elem = get_left(&ret_xnode);         // SequenceType
        let param_xnode = get_right(&ret_xnode);  // Param
        let seq_xnode = get_left(&param_xnode);   // SequenceType
        let bi_arg = get_left(&seq_xnode);          // AtomicOrUnitonType
        if derives_from("xs:integer", &get_xnode_name(&bi_arg)) &&
           subtype(&ai_elem, &bi_elem) {
            return true;
        }
    }

    // -----------------------------------------------------------------
    // 以上のどの条件も満たさない場合: サブタイプではない。
    //
    return false;
}

// ---------------------------------------------------------------------
//
fn derives_from(ai: &str, bi: &str) -> bool {
    let derives_from_map: HashMap<&str, &str> = [
        ( "xs:integer",       "xs:decimal" ),
        ( "xs:decimal",       "xs:numeric" ),
        ( "xs:double",        "xs:numeric" ),
                // numericは、実際には union {decimal, float, double}
        ( "xs:numeric",       "xs:anyAtomicType" ),
        ( "xs:string",        "xs:anyAtomicType" ),
        ( "xs:anyURI",        "xs:string" ),
                // anyURIは常にstringに昇格可能
        ( "xs:boolean",       "xs:anyAtomicType" ),
        ( "xs:anyAtomicType", "xs:anySimpleType" ),
        ( "xs:anySimpleType", "xs:anyType" ),
        ( "xs:untyped",       "xs:anyType" ),
    ].iter().cloned().collect();

    let mut t_type = String::from(ai);
    loop {
        if t_type.as_str() == bi {
            return true;
        }
        match derives_from_map.get(t_type.as_str()) {
            Some(s) => t_type = String::from(*s),
            None => return false,
        }
    }
}

// ---------------------------------------------------------------------
// XNodeType::{Map,SquareArray,CurlyArray} が指す内容を
// XItem::{XIMap,XIArray} に変換する。
//
fn convert_xnode_to_map_array(xnode: &XNodePtr,
                context_xseq: &XSequence,
                eval_env: &mut EvalEnv) -> Result<XItem, Box<Error>> {
    match get_xnode_type(&xnode) {
        XNodeType::Map => {
            // ---------------------------------------------------------
            // マップの実体を取り出す。
            //    Map
            //     |
            // MapConsruct -------------- MapConstruct ---...
            //     |                          |
            //  MapEntry --- (value)       MapEntry --- (value)
            //     |                          |
            //   (key)                      (key)
            //
            let map_construct_xnode = get_left(&xnode);
            let mut curr = map_construct_xnode.clone();
            let mut vec_item: Vec<(XItem, XSequence)> = vec!{};
            while ! is_nil_xnode(&curr) {
                if get_xnode_type(&curr) != XNodeType::MapConstruct {
                    return Err(cant_occur!(
                    "convert_xnode_to_map_array[Map]: xnode = {}, not MapConstruct",
                        get_xnode_type(&curr)));
                }

                let map_entry_xnode = get_left(&curr);
                if get_xnode_type(&map_entry_xnode) != XNodeType::MapEntry {
                    return Err(cant_occur!(
                    "convert_xnode_to_map_array[Map]: xnode = {}, not MapEntry",
                        get_xnode_type(&map_entry_xnode)));
                }

                let map_key_xnode = get_left(&map_entry_xnode);
                let key = evaluate_xnode(context_xseq, &map_key_xnode, eval_env)?.get_singleton_item()?;

                let map_value_xnode = get_right(&map_entry_xnode);
                let val = evaluate_xnode(context_xseq, &map_value_xnode, eval_env)?;
                vec_item.push((key, val));

                curr = get_right(&curr);
            }
            return Ok(new_xitem_map(&vec_item));

        },

        XNodeType::SquareArray => {
            // ---------------------------------------------------------
            // 配列の実体を取り出す。
            // SquareArray
            //     |
            //  ArrayEntry --- ArrayEntry ---...
            //     |              |
            //   (expr)         (expr)
            //
            let array_entry_xnode = get_left(&xnode);
            let mut curr = array_entry_xnode.clone();
            let mut vec_item: Vec<XSequence> = vec!{};
            while ! is_nil_xnode(&curr) {
                if get_xnode_type(&curr) != XNodeType::ArrayEntry {
                    return Err(cant_occur!(
                    "convert_xnode_to_map_array[SquareArray]: xnode = {}, not ArrayEntry",
                        get_xnode_type(&curr)));
                }

                let value_xnode = get_left(&curr);
                let val = evaluate_xnode(context_xseq, &value_xnode, eval_env)?;
                vec_item.push(val);
                curr = get_right(&curr);
            }
            return Ok(new_xitem_array(&vec_item));
        },

        XNodeType::CurlyArray => {
            // ---------------------------------------------------------
            // 配列の実体を取り出す。
            //
            let array_entry_xnode = get_left(&xnode);
            let val_xseq = evaluate_xnode(context_xseq, &array_entry_xnode, eval_env)?;
            let mut vec_item: Vec<XSequence> = vec!{};
            for item in val_xseq.iter() {
                vec_item.push(new_singleton(item));
            }
            return Ok(new_xitem_array(&vec_item));
        },

        _ => {
            return Err(cant_occur!(
                "convert_xnode_to_map_array: xnode = {}",
                get_xnode_type(&xnode)));
        },
    }
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
            ( "if (1 = 1) then 3 else 5", "3" ),
            ( "if (1 = 9) then 3 else 5", "5" ),
            ( "if (prod/@discount) then prod/wholesale else prod/retail",
              r#"(<wholesale id="wa">, <wholesale id="wb">)"# ),
            ( "if (item/@discount) then item/wholesale else item/retail",
              r#"(<retail id="ra">, <retail id="rb">)"# ),
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
            ( "some $x in 3 to 5 satisfies $x mod 2 = 0", "true" ),
            ( "some $x in 3 to 5 satisfies $x mod 6 = 0", "false" ),
            ( "some $x in 1 to 2, $y in 2 to 3 satisfies $x + $y = 5", "true" ),
            ( "some $x in 1 to 2, $y in 2 to 3 satisfies $x + $y = 7", "false" ),
            ( r#"some $a in /root/a satisfies $a/@v = "y""#, "true" ),
            ( r#"some $a in /root/a satisfies $a/@v = "w""#, "false" ),
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
            ( "every $x in 3 to 5 satisfies $x > 2", "true" ),
            ( "every $x in 3 to 5 satisfies $x > 3", "false" ),
            ( "every $x in 1 to 2, $y in 2 to 3 satisfies $x + $y > 2", "true" ),
            ( "every $x in 1 to 2, $y in 2 to 3 satisfies $x + $y > 4", "false" ),
            ( r#"every $a in /root/a satisfies $a/@v != "w""#, "true" ),
            ( r#"every $a in /root/a satisfies $a/@v = "y""#, "false" ),
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
            ( "100 castable as string", "true" ),
            ( "100 castable as string?", "true" ),
            ( r#"/root/empty castable as string"#, "false" ),
            ( r#"/root/empty castable as string?"#, "true" ),
            ( r#"/root/a[@v="x"] castable as string"#, "true" ),
            ( r#"/root/a[@v="x"] castable as string?"#, "true" ),
            ( r#"/root/a castable as string"#, "false" ),
            ( r#"/root/a castable as string?"#, "false" ),
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
            ( r#"/root/a[@v="x"] castable as string"#, "true" ),
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
    // element(sel, type_anno) | element(sel, type_anno?)
    //
    #[test]
    fn test_kind_test_element() {
        let xml = compress_spaces(r#"
<root>
    <a base="base">
        <sel img="z0"/>
        <sel img="z1"/>
        <sel img="z2" xsi:nil="true" />
        <alt img="a0"/>
        <alt img="a1"/>
    </a>
</root>
        "#);

        subtest_eval_xpath("kind_test_element", &xml, &[
            ( "count(child::element())", "5" ),
            ( "count(child::element(*))", "5" ),
            ( "count(child::element(sel))", "3" ),
            ( "count(child::element(sel, anyType))", "2" ),
            ( "count(child::element(sel, anyType?))", "3" ),
            ( "count(child::element(sel, bad))", "0" ),
            ( "count(child::element(sel, bad?))", "0" ),
        ]);
    }

    // -----------------------------------------------------------------
    // attribute() | attribute(*) | attribute(a)
    // attribute(sel, type_anno)
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
            ( "sel/attribute::attribute(a)", r#"a="1""# ),
            ( "sel/attribute::attribute(a, anyType)", r#"a="1""# ),
            ( "sel/attribute::attribute(a, BAD)", r#"()"# ),
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
            ( "count(/child::processing-instruction())", "3" ),
            ( "count(/child::processing-instruction('style-sheet'))", "2" ),
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
            ( ".", r#"<a base="base">"# ),
            ( "./b", r#"<b id="b">"# ),
            ( "self::a", r#"<a base="base">"# ),
            ( r#"self::a[@base="base"]"#, r#"<a base="base">"# ),
            ( "self::b", "()" ),
                    // 「self」と明記した場合はAxisSelfであり、
                    // NodeTestを記述できる。
            ( ".::a", "Syntax Error in XPath" ),
            ( ".a", "Syntax Error in XPath" ),
                    // 「.」と書き、さらにNodeTestを記述する構文はない。
            ( r#".[name()="a"]"#, r#"<a base="base">"# ),
            ( r#".[@base="base"]"#, r#"<a base="base">"# ),
                    // しかし述語は記述できる。

            ( "(1 to 20)[. mod 5 eq 0]", "(5, 10, 15, 20)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // OperatorConcat
    //
    #[test]
    fn test_operator_concat() {
        let xml = compress_spaces(r#"
<root>
    <a base="base">
        <b id="b"/>
    </a>
</root>
        "#);

        subtest_eval_xpath("operator_concat", &xml, &[
            ( r#" "あい" || "うえ" "#, r#""あいうえ""# ),
            ( r#" 123 || 456 || 789 "#, r#""123456789""# ),
        ]);
    }

    // -----------------------------------------------------------------
    // OperatorMap
    //
    #[test]
    fn test_operator_map() {
        let xml = compress_spaces(r#"
<root>
    <z>
        <a base="base">
            <b>b1</b>
            <b>b2</b>
        </a>
    </z>
</root>
        "#);

        subtest_eval_xpath("operator_map", &xml, &[
            ( r#"sum((1, 3, 5)!(.*.)) "#, r#"35"# ),
            ( r#"string-join((1 to 4) ! "*") "#, r#""****""# ),
            ( r#"string-join((1 to 4) ! "*", ".") "#, r#""*.*.*.*""# ),
            ( r#"child::b/string()!concat("id-", .)"#, r#"("id-b1", "id-b2")"# ),
            ( r#"string-join(ancestor::*!name(), '/')"#, r#""root/z""# ),
        ]);
    }

    // -----------------------------------------------------------------
    // ArrowExpr
    //
    #[test]
    fn test_arrow_expr() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("arrow_expr", &xml, &[
            ( r#" 'aBcDe' => upper-case() => substring(2, 3)"#, r#""BCD""# ),
            ( "let $f := function($a) { $a * $a } return 5 => $f() ", "25" ),

        ]);
    }

    // -----------------------------------------------------------------
    // LetExpr
    //
    #[test]
    fn test_let_expr() {
        let xml = compress_spaces(r#"
<root>
    <z>
        <a base="base">
            <b>b1</b>
            <b>b2</b>
        </a>
    </z>
</root>
        "#);

        subtest_eval_xpath("let_expr", &xml, &[
            ( r#"let $x := 4, $y := 3 return $x + $y"#, r#"7"# ),
            ( r#"let $x := 4, $y := $x * 2 return $x + $y"#, r#"12"# ),
        ]);
    }

    // -----------------------------------------------------------------
    // InlineFunction
    //
    #[test]
    fn test_inline_function() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("inline_function", &xml, &[
            ( "let $f := function() { 4 } return $f() ", "4" ),
            ( "let $f := function($n as xs:integer) { $n * 3 } return $f(5) ", "15" ),
            ( r#"let $x := function ($m as integer, $n as integer) { ($m + $n) * 3 } return $x(2, 4) "#, r#"18"# ),
            ( "for-each(1 to 4, function($x as xs:integer) { $x * $x })", "(1, 4, 9, 16)" ),
            ( "for-each(1 to 4, function($x as node()) { $x })", "Type Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // NamedFunctionRef
    //
    #[test]
    fn test_named_function_ref() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("named_function_ref", &xml, &[
            ( r#"for-each(("john", "jane"), fn:string-to-codepoints#1)"#,
                        "(106, 111, 104, 110, 106, 97, 110, 101)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // PartialFunctionCall / ArgumentPlaceholder
    //
    #[test]
    fn test_partial_function_call() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("partial_function_call", &xml, &[
            ( r#"for-each(("a", "b"), fn:starts-with(?, "a")) "#,
                        "(true, false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // Map
    //
    #[test]
    fn test_map_lookup() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("map_lookup", &xml, &[
            ( r#"
                let $week := map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } return $week("Su")
              "#, r#""Sunday""# ),
            ( r#"
                let $bk := map {
                    "a" : map {
                        "a1" : "A1",
                        "a2" : "A2"
                    },
                    "b" : map {
                        "b1" : "B1",
                        "b2" : "B2"
                    }
                } return $bk("a")("a2")
              "#, r#""A2""# ),
        ]);
    }

    // -----------------------------------------------------------------
    // Array
    //
    #[test]
    fn test_array_lookup() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("array_lookup", &xml, &[
            ( r#"[ 1, 3, 5, 7 ](4)"#, "7" ),
            ( r#"[ [1, 2, 3], [4, 5, 6]](2)"#, "[4, 5, 6]" ),
            ( r#"[ [1, 2, 3], [4, 5, 6]](2)(2)"#, "5" ),
            ( r#"array{ (1), (2, 3), (4, 5) }(4)"#, "4" ),
        ]);
    }

    // -----------------------------------------------------------------
    // UnaryLookup
    //
    #[test]
    fn test_unary_lookup() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("unary_lookup", &xml, &[
            // NCName
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                }[.("Su") = "Sunday"]
              "#, r#"{"Su" => "Sunday", "Mo" => "Monday"}"# ),
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                }[?Su = "Sunday"]
              "#, r#"{"Su" => "Sunday", "Mo" => "Monday"}"# ),

            // NCName
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } ! ?Su = "Sunday"
              "#, r#"true"# ),
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } ! ?Su = "Monday"
              "#, r#"false"# ),

            // Wildcard
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } ! (for $k in map:keys(.) return .($k))
              "#, r#"("Sunday", "Monday")"# ),
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } ! ?*
              "#, r#"("Sunday", "Monday")"# ),
                                    // map {...} ! ?*

            // ParenthesizedExpr
            ( r#"
                map {
                    "Su" : "Sunday",
                    "Mo" : "Monday"
                } ! ?("Mo", "Su")
              "#, r#"("Monday", "Sunday")"# ),

            // IntegerLiteral
            ( r#"[ 1, 3, 5, 7 ][?3 = 5]"#, "[1, 3, 5, 7]" ),
            ( r#"[ 1, 3, 5, 7 ][?3 = 10]"#, "()" ),

            // Wildcard
            ( r#"[ 1, 3, 5, 7 ] ! ?*"#, "(1, 3, 5, 7)" ),

            // ParenthesizedExpr
            ( r#"[ 1, 3, 5, 7 ] ! ?(2 to 4)"#, "(3, 5, 7)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // Postfix Lookup
    //
    #[test]
    fn test_postfix_lookup() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("postfix_lookup", &xml, &[
            ( r#"map { "Su" : "Sunday", "Mo" : "Monday" }?Su"#, r#""Sunday""# ),
            ( r#"map { 0: "F", 1: "T" }?1 "#, r#""T""# ),

            ( r#"[4, 5, 6]?2"#, r#"5"# ),
            ( r#"[4, 5, 6]?*"#, r#"(4, 5, 6)"# ),
            ( r#" ([1, 2, 3], [4, 5, 6])?2 "#, "(2, 5)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // instance of
    //
    #[test]
    fn test_instance_of() {
        let xml = compress_spaces(r#"
<root>
    <elem base="base"/>
</root>
        "#);

        subtest_eval_xpath("instance_of", &xml, &[
            ( r#"() instance of empty-sequence() "#, "true" ),
            ( r#"7 instance of empty-sequence() "#, "false" ),

            // AtomicOrUnionType
            ( r#"5 instance of xs:integer "#, "true" ),
            ( r#"(5, 7) instance of xs:integer+ "#, "true" ),
            ( r#"(5, 7.3) instance of xs:decimal+ "#, "true" ),
            ( r#"(5, 7) instance of xs:numeric+ "#, "true" ),
            ( r#"(5, "ss") instance of xs:anyAtomicType+ "#, "true" ),
            ( r#" . instance of element() + "#, "true" ),

            // ArrayTest
            ( r#" [1, 2] instance of array(*) "#, "true" ),
            ( r#" [1, 2] instance of array(xs:integer) "#, "true" ),
            ( r#" [1, 2] instance of array(xs:string) "#, "false" ),
            ( r#" [(1, 2), 2] instance of array(xs:integer) "#, "false" ),
            ( r#" [(1, 2), 2] instance of array(xs:integer+) "#, "true" ),
            ( r#" [[1, 2], [2]] instance of array(array(xs:integer)) "#, "true" ),

            // MapTest
            ( r#" map{"a": 1, "b": "x"} instance of map(*) "#, "true" ),
            ( r#" map{"a": 1, "b": 2} instance of map(string, integer) "#, "true" ),
            ( r#" map{"a": 1, "b": 2} instance of map(string, string) "#, "false" ),
            ( r#" map{"a": [1], "b": [2]} instance of map(string, array(integer)) "#, "true" ),
        ]);
    }

    // -----------------------------------------------------------------
    // instance of function()
    //
    #[test]
    fn test_instance_of_function() {
        let xml = compress_spaces(r#"
<root>
    <elem base="base"/>
</root>
        "#);

        subtest_eval_xpath("instance_of_function", &xml, &[
            // AnyFunctionTest
            ( "7 instance of function(*) ", "false" ),
            ( "[1, 2] instance of function(*) ", "true" ),
            ( r#"map{"a": 1} instance of function(*) "#, "true" ),
            ( "fn:string-to-codepoints#1 instance of function(*)", "true" ),
            ( "function($n as xs:integer) { $n * 3 } instance of function(*) ", "true" ),
            ( "let $f := function($n as xs:integer) { $n * 3 } return $f instance of function(*) ", "true" ),
            ( r#"fn:starts-with(?, "a") instance of function(*) "#, "true" ),

            // TypedFunctionTest
            ( "7 instance of function(integer) as integer", "false" ),
            ( "[1, 2] instance of function(integer) as integer", "true" ),
            ( "[1, 2] instance of function(integer) as string", "false" ),
            ( r#"map{"a": 1} instance of function(string) as integer"#, "true" ),
            ( r#"map{"a": 1} instance of function(string) as string"#, "false" ),
            ( r#"map{"a": 1} instance of function(integer) as integer"#, "false" ),

            // TypedFunctionTest (InlineFunction)
            ( "function($n as xs:integer) as xs:integer { $n * 3 } instance of function(xs:integer) as xs:integer", "true" ),
            ( "function($n as xs:integer) as xs:integer { $n * 3 } instance of function(xs:integer) as xs:anyAtomicType", "true" ),

            // TypedFunctionTest (InlineFunction): 引数の不一致
            ( "function($n as xs:numeric) as xs:numeric { $n * 3 } instance of function(xs:anyAtomicType) as xs:integer", "false" ),

            // TypedFunctionTest (InlineFunction): 引数の個数の不一致
            ( "function($n as xs:numeric) as xs:numeric { $n * 3 } instance of function(xs:integer, xs:integer) as xs:integer", "false" ),
            ( "function($n as xs:numeric) as xs:numeric { $n * 3 } instance of function() as xs:integer", "false" ),

            // TypedFunctionTest (NamedFunctionRef):
            ( "fn:abs#1 instance of function(numeric) as numeric?", "true" ),
            ( "fn:abs#1 instance of function(integer) as numeric?", "true" ),
            ( "fn:abs#1 instance of function(numeric) as integer?", "false" ),
            // TypedFunctionTest (NamedFunctionRef): FunctionTest
            ( "fn:filter#2 instance of function(item()*, function(item()) as boolean) as item()*", "true" ),
            ( "fn:filter#2 instance of function(integer*, function(item()) as boolean) as item()*", "true" ),
            ( "fn:filter#2 instance of function(item()*, function(integer) as integer) as item()*", "false" ),

            // TypedFunctionTest (NamedFunctionRef): MapTest
            ( "map:size#1 instance of function(map(*)) as integer", "true" ),
            ( "map:size#1 instance of function(map(string, integer)) as integer", "true" ),

            // TypedFunctionTest (NamedFunctionRef): ArrayTest
            ( "array:size#1 instance of function(array(*)) as integer", "true" ),
        ]);
    }


    // -----------------------------------------------------------------
    // subtype_itemtype (map)
    //
    #[test]
    fn test_subtype_itemtype_map() {
        let xml = compress_spaces(r#"
<root>
    <elem base="base"/>
</root>
        "#);

        subtest_eval_xpath("subtype_itemtype_map", &xml, &[

            // 27. Ai is map(K, V), for any K and V and Bi is map(*).
            ( r#"function() as map(*) { "a" }
                     instance of
                 function() as map(*)"#, "true" ),
            ( r#"function() as map(string, string) { "a" }
                     instance of
                 function() as map(*)"#, "true" ),

            // 28. Ai is map(Ka, Va) and Bi is map(Kb, Vb),
            //     where subtype-itemtype(Ka, Kb) and subtype(Va, Vb).
            ( r#"function() as map(string, integer) { "a" }
                     instance of
                 function() as map(anyAtomicType, decimal)"#, "true" ),
            ( r#"function() as map(anyAtomicType, decimal) { "a" }
                     instance of
                 function() as map(string, integer)"#, "false" ),

            // 29. Ai is map(*) (or any other map type),
            //     and Bi is function(*).
            ( r#"function() as map(*) { "a" }
                     instance of
                 function() as function(*)"#, "true" ),
            ( r#"function() as map(string, integer) { "a" }
                     instance of
                 function() as function(*)"#, "true" ),

            // 30. Ai is map(*) (or any other map type),
            //     and Bi is function(xs:anyAtomicType) as item()*.
            ( r#"function() as map(*) { "a" }
                     instance of
                 function() as function(xs:anyAtomicType) as item()*"#, "true" ),
            ( r#"function() as map(string, string) { "a" }
                     instance of
                 function() as function(xs:anyAtomicType) as item()*"#, "true" ),
            ( r#"function() as map(string, string) { "a" }
                     instance of
                 function() as function(string) as string"#, "true" ),

            // 35. Ai is map(K, V), and Bi is function(xs:anyAtomicType) as V?.
            ( r#"function() as map(integer, string) { "a" }
                     instance of
                 function() as function(xs:anyAtomicType) as string?"#, "true" ),

        ]);
    }

    // -----------------------------------------------------------------
    // subtype_itemtype (array)
    //
    #[test]
    fn test_subtype_itemtype_array() {
        let xml = compress_spaces(r#"
<root>
    <elem base="base"/>
</root>
        "#);

        subtest_eval_xpath("subtype_itemtype_array", &xml, &[

            // 31. Ai is array(X) and Bi is array(*).
            ( r#"function() as array(integer) { "a" }
                     instance of
                 function() as array(*)"#, "true" ),

            // 32. Ai is array(X) and Bi is array(Y), and subtype(X, Y) is true.
            ( r#"function() as array(integer) { "a" }
                     instance of
                 function() as array(decimal)"#, "true" ),

            // 33. Ai is array(*) (or any other array type),
            //     and Bi is function(*).
            ( r#"function() as array(*) { "a" }
                     instance of
                 function() as function(*)"#, "true" ),

            // 34. Ai is array(*) (or any other array type)
            //     and Bi is function(xs:integer) as item()*.
            // 36. Ai is array(X) and Bi is function(xs:integer) as X.
            ( r#"function() as array(*) { "a" }
                     instance of
                 function() as function(integer) as item()*"#, "true" ),
            ( r#"function() as array(string) { "a" }
                     instance of
                 function() as function(integer) as string"#, "true" ),
        ]);
    }

    // -----------------------------------------------------------------
    // instance of ( ParenthesizedItemType )
    //
    #[test]
    fn test_instance_of_parenthesized_item_type() {
        let xml = compress_spaces(r#"
<root>
    <elem base="base"/>
</root>
        "#);

        subtest_eval_xpath("instance_of_parenthesized_item_type", &xml, &[
//            ( " (1) instance of (xs:integer) ", "true" ),
        ]);
    }
}

