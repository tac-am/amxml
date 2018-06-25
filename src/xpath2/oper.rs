//
// xpath2/oper.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::cmp::Ordering;
use std::error::Error;

use dom::*;
use xpath2::eval::*;
use xpath2::xitem::*;
use xpath2::xsequence::*;

// ---------------------------------------------------------------------
// 6.2 Operators on Numeric Values
//       op_numeric_add
//       op_numeric_subtract
//       op_numeric_multiply
//       op_numeric_divide
//       op_numeric_integer_divide
//       op_numeric_mod
//       op_numeric_unary_plus
//       op_numeric_unary_minus
// ---------------------------------------------------------------------
//
pub fn op_numeric_add(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_add);
}

pub fn op_numeric_subtract(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_subtract);
}

pub fn op_numeric_multiply(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_multiply);
}

pub fn op_numeric_divide(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_divide);
}

pub fn op_numeric_integer_divide(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_integer_divide);
}

pub fn op_numeric_mod(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_operation(args, xitem_numeric_mod);
}

pub fn op_numeric_unary_plus(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    let arg = args[0].get_singleton_item()?;
    let result = xitem_numeric_unary_plus(&arg)?;
    return Ok(new_singleton(&result));
}

pub fn op_numeric_unary_minus(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    let arg = args[0].get_singleton_item()?;
    let result = xitem_numeric_unary_minus(&arg)?;
    return Ok(new_singleton(&result));
}

// ---------------------------------------------------------------------
//
fn op_numeric_operation<F>(args: &Vec<XSequence>, mut func_op: F) -> Result<XSequence, Box<Error>>
    where F: FnMut(&XItem, &XItem) -> Result<XItem, Box<Error>> {

    let lhs = args[0].get_singleton_item()?;
    let rhs = args[1].get_singleton_item()?;
    let result = func_op(&lhs, &rhs)?;
    return Ok(new_singleton(&result));
}

// ---------------------------------------------------------------------
// 6.3 Comparison Operators on Numeric Values
//
// ---------------------------------------------------------------------
//
pub fn op_numeric_equal(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_comparison(args, xitem_numeric_equal);
}

pub fn op_numeric_less_than(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_comparison(args, xitem_numeric_less_than);
}

pub fn op_numeric_greater_than(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    return op_numeric_comparison(args, xitem_numeric_greater_than);
}

// ---------------------------------------------------------------------
//
fn op_numeric_comparison<F>(args: &Vec<&XSequence>, mut func_op: F) -> Result<XSequence, Box<Error>>
    where F: FnMut(&XItem, &XItem) -> Result<bool, Box<Error>> {

    let lhs = args[0].get_singleton_item()?;
    let rhs = args[1].get_singleton_item()?;
    let result = func_op(&lhs, &rhs)?;
    return Ok(new_singleton_boolean(result));
}

// ---------------------------------------------------------------------
// 7.3.2 fn:compare
//   (文字列の比較はopでなくfnとして実装)
// fn fn_compare(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
//
// ---------------------------------------------------------------------
// 9.2 Operators on Boolean Values
//
// ---------------------------------------------------------------------
//
pub fn op_boolean_equal(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let lhs = args[0].get_singleton_boolean()?;
    let rhs = args[1].get_singleton_boolean()?;
    return Ok(new_singleton_boolean(lhs == rhs));
}

// ---------------------------------------------------------------------
//
pub fn op_boolean_less_than(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let lhs = args[0].get_singleton_boolean()?;
    let rhs = args[1].get_singleton_boolean()?;
    return Ok(new_singleton_boolean(lhs < rhs));
}

// ---------------------------------------------------------------------
//
pub fn op_boolean_greater_than(args: &Vec<&XSequence>) -> Result<XSequence, Box<Error>> {
    let lhs = args[0].get_singleton_boolean()?;
    let rhs = args[1].get_singleton_boolean()?;
    return Ok(new_singleton_boolean(lhs > rhs));
}

// ---------------------------------------------------------------------
// 14 Functions and Operators on Nodes
//          is_same_node
//          node_before
//          node_after
//
pub fn op_is_same_node(args: &Vec<XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    return op_node_compare(args, eval_env, Ordering::Equal);
}

pub fn op_node_before(args: &Vec<XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    return op_node_compare(args, eval_env, Ordering::Less);
}

pub fn op_node_after(args: &Vec<XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    return op_node_compare(args, eval_env, Ordering::Greater);
}

fn op_node_compare(args: &Vec<XSequence>, eval_env: &EvalEnv,
                    ordering: Ordering) -> Result<XSequence, Box<Error>> {
    let node1 = args[0].get_singleton_node()?;
    let node2 = args[1].get_singleton_node()?;
    let result = eval_env.compare_by_doc_order(&node1, &node2);
    return Ok(new_singleton_boolean(result == ordering));
}

// ---------------------------------------------------------------------
// 15 Functions and Operators on Sequences
//
// ---------------------------------------------------------------------
// 15.3 Equals, Union, Intersection and Except
//
pub fn op_union(args: &Vec<XSequence>, eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    let mut node_array: Vec<NodePtr> = vec!{};
    for n in args[0].to_nodeset().iter() {          // lhs
        node_array.push(n.rc_clone());
    }
    for n in args[1].to_nodeset().iter() {          // rhs
        node_array.push(n.rc_clone());
    }
    eval_env.sort_by_doc_order(&mut node_array);
    return Ok(new_xsequence_from_node_array(&node_array));
}

pub fn op_intersect(args: &Vec<XSequence>, _eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    let mut node_array: Vec<NodePtr> = vec!{};
    let rhs = args[1].to_nodeset();
    for n in args[0].to_nodeset().iter() {          // lhs
        if rhs.contains(&n) {
            node_array.push(n.rc_clone());
        }
    }
    // eval_env.sort_by_doc_order(&mut node_array);
    return Ok(new_xsequence_from_node_array(&node_array));
}

pub fn op_except(args: &Vec<XSequence>, _eval_env: &EvalEnv) -> Result<XSequence, Box<Error>> {
    let mut node_array: Vec<NodePtr> = vec!{};
    let rhs = args[1].to_nodeset();
    for n in args[0].to_nodeset().iter() {          // lhs
        if ! rhs.contains(&n) {
            node_array.push(n.rc_clone());
        }
    }
    // eval_env.sort_by_doc_order(&mut node_array);
    return Ok(new_xsequence_from_node_array(&node_array));
}

// ---------------------------------------------------------------------
// 15.5 Functions and Operators that Generate Sequences
//          to
//
pub fn op_to(args: &Vec<XSequence>) -> Result<XSequence, Box<Error>> {
    let firstval = args[0].get_singleton_integer()?;
    let lastval = args[1].get_singleton_integer()?;
    let mut seq = new_xsequence();
    for n in firstval ..= lastval {
        seq.push(&new_xitem_integer(n));
    }
    return Ok(seq);
}

// ---------------------------------------------------------------------
// 17 Casting
// ---------------------------------------------------------------------
//

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath2::helpers::compress_spaces;
    use xpath2::helpers::subtest_xpath;
    use xpath2::helpers::subtest_eval_xpath;

    // -----------------------------------------------------------------
    // 6.2 Operators on Numeric Values
    // 加減乗 (優先度、左結合、型の昇格)
    //
    #[test]
    fn test_numeric_operators() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);

        subtest_eval_xpath("numeric_operators", &xml, &[
            ( "10 - 3 - 4", "(3)" ),
            ( "10.5 - 3", "(7.5)" ),
            ( "10.5 - 3 - 1.5", "(6.0)" ),
            ( "1.05e1 - 3 - 1.5", "(6e0)" ),
            ( "10 - (3 - 4)", "(11)" ),
            ( "16 - 6 - 3 - 4", "(3)" ),
            ( "18 - 2 - 6 - 3 - 4", "(3)" ),
            ( "1 + 2 * 4 + 2", "(11)" ),
            ( "1 + 2 * 4 - 2", "(7)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // div: op:numeric-divide
    //
    #[test]
    fn test_numeric_divide() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);

        subtest_eval_xpath("numeric_divide", &xml, &[
            ( "6 div 2", "(3.0)" ),         // Integer div Integer => Decimal
            ( "5 div 2", "(2.5)" ),         // Integer div Integer => Decimal
            ( "5.0 div 2", "(2.5)" ),
            ( "9.6 div 2.4", "(4.0)" ),

            ( "7 div 0", "Dynamic Error" ),
            ( "-7 div 0", "Dynamic Error" ),
            ( "7.0 div 0", "Dynamic Error" ),
            ( "-7.0 div 0", "Dynamic Error" ),
            ( "7 div 0.0e0", "(+Infinity)" ),
            ( "-7 div 0.0e0", "(-Infinity)" ),
            ( "7.0e0 div 0.0e0", "(+Infinity)" ),
            ( "-7.0e0 div 0.0e0", "(-Infinity)" ),

            ( "0.0e0 div 0.0e0", "(NaN)" ),
            ( "0 div 0", "Dynamic Error" ),
        ]);
    }

    // -----------------------------------------------------------------
    // idiv: op:numeric-integer-divide
    //
    #[test]
    fn test_numeric_integer_divide() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);

        subtest_eval_xpath("numeric_integer_divide", &xml, &[
            ( "10 idiv 3", "(3)" ),
            ( "3 idiv -2", "(-1)" ),
            ( "-3 idiv 2", "(-1)" ),
            ( "-3 idiv -2", "(1)" ),
            ( "9.0 idiv 3", "(3)" ),
            ( "-3.5 idiv 3", "(-1)" ),
            ( "3.0 idiv 4", "(0)" ),
            ( "3.1e1 idiv 6", "(5)" ),
            ( "3.1e1 idiv 7", "(4)" ),

            ( "7 idiv 0", "Dynamic Error" ),
            ( "7.0 idiv 0", "Dynamic Error" ),

            // XIDoubleの扱い: JavaやC++の実装とは違っている。
            ( "0.0e0 div 0.0e0", "(NaN)" ),
            ( "(0.0e0 div 0.0e0) idiv 5", "Dynamic Error" ), // NaN idiv any = Error
            ( "5 idiv (0.0e0 div 0.0e0)", "Dynamic Error" ), // any idiv NaN = Error
            ( "7.0e0 div 0.0e0", "(+Infinity)" ),
            ( "(7.0e0 div 0.0e0) idiv 5", "Dynamic Error" ),    // +∞ idiv N = Error
            ( "5 idiv (7.0e0 div 0.0e0)", "(0)" ),   // N idiv +∞ = 0
            ( "5 idiv (-7.0e0 div 0.0e0)", "(0)" ),  // N idiv -∞ = 0
            ( "0.0e0 idiv 5", "(0)" ),              // 0 idiv N = 0

        ]);
    }

    // -----------------------------------------------------------------
    // mod: op:numeric-mod
    //
    #[test]
    fn test_numeric_mod() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);

        subtest_eval_xpath("numeric_mod", &xml, &[
            ( "10 mod 3", "(1)" ),
            ( "6 mod -2", "(0)" ),
            ( "5 mod 2", "(1)" ),
            ( "5 mod -2", "(1)" ),
            ( "-5 mod 2", "(-1)" ),
            ( "-5 mod -2", "(-1)" ),
            ( "7 mod 0", "Dynamic Error" ),
            ( "7.0 mod 0", "Dynamic Error" ),

            ( "3.5 mod 1.5", "(0.5)" ),
            ( "3.5 mod -1.5", "(0.5)" ),
            ( "-3.5 mod 1.5", "(-0.5)" ),
            ( "-3.5 mod -1.5", "(-0.5)" ),
            ( "4.5 mod 1.2", "(0.9000000000000001)" ),  // 0.9
// Decimalの精度???
            ( "1.23e2 mod 0.6e1", "(3e0)" ),            // 123 mod 6 = 3

            // XIDoubleの扱い:
            ( "0.0e0 div 0.0e0", "(NaN)" ),
            ( "(0.0e0 div 0.0e0) mod 5", "(NaN)" ), // NaN mod any = NaN
            ( "5 mod (0.0e0 div 0.0e0)", "(NaN)" ), // any mod NaN = NaN
            ( "7.0e0 div 0.0e0", "(+Infinity)" ),
            ( "5 mod (7.0e0 div 0.0e0)", "(5e0)" ),   // N mod +∞ = N
            ( "5 mod (-7.0e0 div 0.0e0)", "(5e0)" ),  // N mod -∞ = N
            ( "0.0e0 mod 5", "(0e0)" ),              // 0 mod N = 0
        ]);
    }

    // -----------------------------------------------------------------
    // (負のゼロ)
    //
    #[test]
    fn test_minus_zero() {
        let xml = compress_spaces(r#"
<a base="base">
</a>
        "#);

        subtest_eval_xpath("minus_zero", &xml, &[
            ( "round(-0.2)", "(0.0)" ),
            ( "round(0.2)", "(0.0)" ),
            ( "1.0 div round(-0.2)", "Dynamic Error" ),     // 負のゼロで除算
            ( "1.0 div round(0.2)", "Dynamic Error" ),      // 正のゼロで除算
            ( "1.0 div ceiling(-0.2e0)", "(-Infinity)" ),

            ( "0 = -0", "(true)" ),
            ( "0 != -0", "(false)" ),
            ( "0 > -0", "(false)" ),
            ( "0 = -0.0", "(true)" ),
            
            ( "1.0 div (-0e0)", "(-Infinity)" ),
            ( "1.0 div (- (4e0 - 4e0))", "(-Infinity)" ),
            ( "1.0 div -(-0e0)", "(+Infinity)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.6 op:is-same-node
    //
    #[test]
    fn test_op_is_same_node() {
        let xml = compress_spaces(r#"
<a base="base">
    <p id="A" img="A"/>
    <p id="B" img="B"/>
</a>
        "#);
        subtest_eval_xpath("op_is_same_node", &xml, &[
            ( r#"/a/p[@id="A"] is /a/p[@img="A"]"#, "(true)" ),
            ( r#"/a/p[@id="A"] is /a/p[@img="B"]"#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.7 op:node-before
    //
    #[test]
    fn test_op_node_before() {
        let xml = compress_spaces(r#"
<a base="base">
    <p id="A"/>
    <p id="B"/>
</a>
        "#);
        subtest_eval_xpath("op_node_before", &xml, &[
            ( r#"/a/p[@id="A"] << /a/p[@id="B"]"#, "(true)" ),
            ( r#"/a/p[@id="B"] << /a/p[@id="A"]"#, "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 14.8 op:node-after
    //
    #[test]
    fn test_op_node_after() {
        let xml = compress_spaces(r#"
<a base="base">
    <p id="A"/>
    <p id="B"/>
</a>
        "#);
        subtest_eval_xpath("op_node_after", &xml, &[
            ( r#"/a/p[@id="A"] >> /a/p[@id="B"]"#, "(false)" ),
            ( r#"/a/p[@id="B"] >> /a/p[@id="A"]"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.3.2 op:union
    //
    #[test]
    fn test_op_union() {
        let xml = compress_spaces(r#"
<a base="base">
    <left>
        <p img="LA">LA</p>
        <p img="LB">LB</p>
    </left>
    <right>
        <q img="RA">RA</q>
        <q img="RB">RB</q>
        <q img="RC">RC</q>
        <p img="RX">RX</p>
    </right>
    <sel img="T" ans="true" />
    <sel img="F" ans="false" />
</a>
        "#);
        subtest_xpath("op_union", &xml, false, &[
            ( "/a/sel[@ans = string(count(/a/left/p | /a/right/q) = 5)]", "T" ),
            ( "/a/left/p | /a/right/q", "LALBRARBRC" ),
            ( "/a/right/q | /a/left/p", "LALBRARBRC" ), // 文書順に整列
            ( "/a//q | /a/right/q", "RARBRC" ),         // 重複を除いて整列
            // ---------------------------------------------
            ( "(/a/left/p | /a/right/q)[3]", "RA" ),
            ( "(/a/right/q | /a/left/p)[3]", "RA" ), // 「RC」ではない
            // ---------------------------------------------
            ( "(/a/left | /a/right)/p", "LALBRX" ),
            ( "(/a/right | /a/left)/p", "LALBRX" ),
            ( "(/a/left | /a/right)//p", "LALBRX" ),
            ( "(/a/right | /a/left)//p", "LALBRX" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.3.3 op:intersect
    //
    #[test]
    fn test_op_intersect() {
        let xml = compress_spaces(r#"
<a base="base">
    <p img="x11" a="1" b="1"/>
    <p img="x12" a="1" b="2"/>
    <p img="x13" a="1" b="3"/>
    <p img="x21" a="2" b="1"/>
    <p img="x22" a="2" b="2"/>
    <p img="x23" a="2" b="3"/>
    <p img="x31" a="3" b="1"/>
    <p img="x32" a="3" b="2"/>
    <p img="x33" a="3" b="3"/>
</a>
        "#);
        subtest_xpath("op_intersect", &xml, false, &[
            ( r#"/a/p[@a="1"] intersect /a/p[@b="1"]"#, "x11" ),
            ( r#"/a/p[@a>="2"] intersect /a/p[@b>="2"]"#, "x22x23x32x33" ),
            ( r#"/a/p[@b>="2"] intersect /a/p[@a>="2"]"#, "x22x23x32x33" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.3.4 op:except
    //
    #[test]
    fn test_op_except() {
        let xml = compress_spaces(r#"
<a base="base">
    <p img="x11" a="1" b="1"/>
    <p img="x12" a="1" b="2"/>
    <p img="x13" a="1" b="3"/>
    <p img="x21" a="2" b="1"/>
    <p img="x22" a="2" b="2"/>
    <p img="x23" a="2" b="3"/>
    <p img="x31" a="3" b="1"/>
    <p img="x32" a="3" b="2"/>
    <p img="x33" a="3" b="3"/>
</a>
        "#);
        subtest_xpath("op_except", &xml, false, &[
            ( r#"/a/p[@a="1"] except /a/p[@b="1"]"#, "x12x13" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 15.5.1 op:to
    //
    #[test]
    fn test_op_to() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("op_to", &xml, &[
            ( "1 to 3", "(1, 2, 3)" ),
            ( "3 to 1", "()" ),
            ( "5 to 5", "(5)" ),
        ]);
    }

}
