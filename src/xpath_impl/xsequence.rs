//
// xpath_impl/xsequence.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::error::Error;
use std::fmt;
use std::slice::Iter;

use dom::*;
use xmlerror::*;
use xpath_impl::xitem::*;
use xpath_impl::func::*;
use xpath_impl::oper::*;
use xpath_impl::parser::*;

// =====================================================================
// A [sequence] is an ordered collection of zero or more items.
// A sequence containing exactly one item is called a [singleton].
// An item is identical to a singleton sequence containing that item.
// A sequence containing zero items is called an [empty sequence].
//
#[derive(Debug, PartialEq, Clone)]
pub struct XSequence {
    value: Vec<XItem>,
}

pub fn new_xsequence() -> XSequence {
    return XSequence{
        value: vec!{},
    };
}

pub fn new_singleton(item: &XItem) -> XSequence {
    return XSequence{
        value: vec!{item.clone()},
    };
}

pub fn new_xsequence_from_node_array(node_array: &Vec<NodePtr>) -> XSequence {
    let mut xsequence = new_xsequence();
    for node in node_array.iter() {
        xsequence.push(&XItem::XINode{value: node.rc_clone()});
    }
    return xsequence;
}

pub fn new_singleton_xnodeptr(xnode: &XNodePtr) -> XSequence {
    return new_singleton(&new_xitem_xnodeptr(xnode));
}

pub fn new_singleton_node(node: &NodePtr) -> XSequence {
    return new_singleton(&new_xitem_node(node));
}

pub fn new_singleton_string(value: &str) -> XSequence {
    return new_singleton(&new_xitem_string(value));
}

pub fn new_singleton_integer(value: i64) -> XSequence {
    return new_singleton(&new_xitem_integer(value));
}

pub fn new_singleton_decimal(value: f64) -> XSequence {
    return new_singleton(&new_xitem_decimal(value));
}

pub fn new_singleton_double(value: f64) -> XSequence {
    return new_singleton(&new_xitem_double(value));
}

pub fn new_singleton_boolean(value: bool) -> XSequence {
    return new_singleton(&new_xitem_boolean(value));
}

// =====================================================================
// Trait std::fmt::Display
//
impl fmt::Display for XSequence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s += &"(";
        for (i, v) in self.value.iter().enumerate() {
            if i != 0 {
                s += &", ";
            }
            s += &v.to_string();        // XSequenceを構成する各XItem
        }
        s += &")";
        return write!(f, "{}", s);
    }
}

// =====================================================================
//
impl XSequence {

    // -----------------------------------------------------------------
    // シングルトンである場合に、これと同一視するXItemを返す。
    //
    pub fn get_singleton_item(&self) -> Result<XItem, Box<Error>> {
        if self.is_singleton() {
            return Ok(self.value[0].clone());
        } else {
            return Err(type_error!("This sequence must be singleton."));
        }
    }

    // -----------------------------------------------------------------
    // シングルトンかつXNodePtrであれば、そのノードを返す。
    //
    pub fn get_singleton_xnodeptr(&self) -> Result<XNodePtr, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XItemXNodePtr{value} => return Ok(value.clone()),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton xnodeptr."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつノードであれば、そのノードを返す。
    //
    pub fn get_singleton_node(&self) -> Result<NodePtr, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XINode{value} => return Ok(value.rc_clone()),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton node."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつマップであれば、そのマップを返す。
    //
    pub fn get_singleton_map(&self) -> Result<XSeqMap, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XIMap{value} => return Ok(value.clone()),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton map."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつ配列であれば、その配列を返す。
    //
    pub fn get_singleton_array(&self) -> Result<XSeqArray, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XIArray{value} => return Ok(value.clone()),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton array."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつ文字列であれば、その文字列を返す。
    //
    pub fn get_singleton_string(&self) -> Result<String, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XIString{value} => return Ok(value),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton string."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつ整数であれば、その整数を返す。
    //
    pub fn get_singleton_integer(&self) -> Result<i64, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XIInteger{value} => return Ok(value),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton integer."));
    }

    // -----------------------------------------------------------------
    // シングルトンかつブーリアンであれば、そのブール値を返す。
    //
    pub fn get_singleton_boolean(&self) -> Result<bool, Box<Error>> {
        let item = self.get_singleton_item()?;
        match item {
            XItem::XIBoolean{value} => return Ok(value),
            _ => {},
        }

        return Err(type_error!("This sequence must be singleton boolean."));
    }

    // -----------------------------------------------------------------
    // 2.4.3 Effective Boolean Value
    //
    pub fn effective_boolean_value(&self) -> Result<bool, Box<Error>> {
        if self.is_empty() {
            return Ok(false);
        }
        match self.value[0] {
            XItem::XINode{value: _} => return Ok(true),
            _ => {},
        }
        if self.is_singleton() {
            match self.value[0] {
                XItem::XIBoolean{value} => return Ok(value.clone()),
                XItem::XIString{ref value} => return Ok(value != ""),
                XItem::XIDouble{value} => {
                    return Ok(value != 0.0 && ! value.is_nan());
                },
                XItem::XIDecimal{value} => {
                    return Ok(value != 0.0 && ! value.is_nan());
                },
                XItem::XIInteger{value} => {
                    return Ok(value != 0);
                },
                _ => {},
            }
        }
        return Err(type_error!(
            "effective_boolean_value: Can't determin effective boolean value: {}",
            self.to_string()));

    }

    // -----------------------------------------------------------------
    // 原子化
    //
    pub fn atomize(&self) -> XSequence {
        let mut seq = new_xsequence();
        for v in self.value.iter() {
            seq.push(&v.atomize());
        }
        return seq;
    }

    // -----------------------------------------------------------------
    //
    pub fn is_empty(&self) -> bool {
        return self.value.len() == 0;
    }

    // -----------------------------------------------------------------
    //
    pub fn is_singleton(&self) -> bool {
        return self.value.len() == 1;
    }

    // -----------------------------------------------------------------
    // シーケンスに原子型がない、すなわちノードのみであることを判定する。
    // 空である場合もtrueを返す。
    //
    pub fn is_no_atom(&self) -> bool {
        for item in self.value.iter() {
            match item {
                XItem::XINode{value: _} => {},
                _ => {
                    return false;
                },
            }
        }
        return true;
    }

    // -----------------------------------------------------------------
    //
    pub fn len(&self) -> usize {
        return self.value.len();
    }

    // -----------------------------------------------------------------
    //
    pub fn iter(&self) -> Iter<XItem> {
        return self.value.iter();
    }

    // -----------------------------------------------------------------
    //
    pub fn get_item(&self, pos: usize) -> &XItem {
        return &self.value[pos];
    }

    // -----------------------------------------------------------------
    //
    pub fn push(&mut self, item: &XItem) {
        self.value.push(item.clone());
    }

    // -----------------------------------------------------------------
    //
    pub fn append(&mut self, other: &XSequence) {
        for item in other.value.iter() {
            self.value.push(item.clone());
        }
    }

    // -----------------------------------------------------------------
    //
    pub fn reverse(&mut self) {
        self.value.reverse();
    }

    // -----------------------------------------------------------------
    // キャスト可能か否か。
    //     キャスト指定の末尾が "?" であれば、空シーケンスもキャスト可能。
    //
    pub fn castable_as(&self, type_name: &str) -> bool {
        if self.is_empty() {
            return type_name.ends_with("?");
        }

        if let Ok(xitem) = self.get_singleton_item() {
            return xitem.castable_as(type_name.trim_right_matches('?'));
        }

        return false;
    }

    // -----------------------------------------------------------------
    // キャスト。
    // 1. 原子化を施す。
    // 2. 空シーケンスでもシングルトンでもなければエラー。
    // 3. 空シーケンスのとき:
    //      キャスト指定の末尾が "?" であれば、空シーケンスを返す。
    //      キャスト指定の末尾が "?" でなければエラー。
    //
    pub fn cast_as(&self, type_name: &str) -> Result<XSequence, Box<Error>> {
        if self.is_empty() {
            if type_name.ends_with("?") {
                return Ok(new_xsequence());
            }
        }

        if let Ok(xitem) = self.get_singleton_item() {
            if let Ok(result) = xitem.atomize().cast_as(type_name.trim_right_matches('?')) {
                return Ok(new_singleton(&result));
            }
        }

        return Err(type_error!("{}: can't cast as {}",
                    self.to_string(), type_name));
    }

    // -----------------------------------------------------------------
    // シーケンス中のノードのみを取り出す。
    //
    pub fn to_nodeset(&self) -> Vec<NodePtr> {
        let mut nodeset: Vec<NodePtr> = vec!{};
        for item in self.value.iter() {
            if let XItem::XINode{value} = item {
                nodeset.push(value.clone());
            }
        }
        return nodeset;
    }
}

// =====================================================================
// 3.5.1 Value Comparisons
//      Result: (false) | (true) | XmlError::TypeError
//      オペランドがどちらもシングルトンの場合に、そのXItemを比較する。
//
pub fn value_compare_eq(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return value_comparison(lhs, rhs,
            |arg| { op_numeric_equal(arg) },
            |arg| { arg == 0 },
            |arg| { op_boolean_equal(arg) });
}

pub fn value_compare_ne(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    let result = value_compare_eq(lhs, rhs)?;
    return fn_not(&vec!{&result});
}

pub fn value_compare_lt(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return value_comparison(lhs, rhs,
            |arg| { op_numeric_less_than(arg) },
            |arg| { arg < 0 },
            |arg| { op_boolean_less_than(arg) });
}

pub fn value_compare_le(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    let result = value_compare_gt(lhs, rhs)?;
    return fn_not(&vec!{&result});
}

pub fn value_compare_gt(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return value_comparison(lhs, rhs,
            |arg| { op_numeric_greater_than(arg) },
            |arg| { arg > 0 },
            |arg| { op_boolean_greater_than(arg) });
}

pub fn value_compare_ge(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    let result = value_compare_lt(lhs, rhs)?;
    return fn_not(&vec!{&result});
}

// ---------------------------------------------------------------------
//
fn value_comparison<FNUM, FSTR, FBOOL>(lhs: &XSequence, rhs: &XSequence,
            mut num_op: FNUM, mut str_cmp: FSTR, mut bool_op: FBOOL)
                                            -> Result<XSequence, Box<Error>>
    where FNUM: FnMut(&Vec<&XSequence>) -> Result<XSequence, Box<Error>>,
          FSTR: FnMut(i64) -> bool,
          FBOOL: FnMut(&Vec<&XSequence>) -> Result<XSequence, Box<Error>> {

    if lhs.is_empty() || rhs.is_empty() {
        return Ok(new_xsequence());
    }
    if ! lhs.is_singleton() || ! rhs.is_singleton() {
        return Err(type_error!(
                    "value_compare: operand is not singleton: {} : {}",
                    lhs.to_string(), rhs.to_string()));
    }
    let lhs = lhs.atomize();
    let rhs = rhs.atomize();
    if let Ok(result) = num_op(&vec!{&lhs, &rhs}) {
        return Ok(result);
    }
    if let Ok(result) = fn_compare(&vec!{&lhs, &rhs}) {
        let result = str_cmp(result.get_singleton_integer()?);
        return Ok(new_singleton_boolean(result));
    }
    if let Ok(result) = bool_op(&vec!{&lhs, &rhs}) {
        return Ok(result);
    }
    return Err(type_error!(
                "value_compare: operand can't compare: {} : {}",
                lhs.to_string(), rhs.to_string()));
}

// =====================================================================
// 3.5.2 General Comparisons
//      Result: (false) | (true) | XmlError::TypeError
//
// 左辺および右辺からひとつずつ取ったXItemの組の中に、
// 演算子の関係を満たすものが1組でもあればtrueとする。
//
// XPath 1.0 非互換モードの場合、一方のオペランドが数値型であっても、
// もう一方を数値型に変換することはない。
//
pub fn general_compare_eq(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { xitem_numeric_equal(s, t) },
        |arg| { arg == 0 },
        |s, t| { xitem_boolean_equal(s, t) });
}

pub fn general_compare_ne(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { let b = xitem_numeric_equal(s, t)?; return Ok(! b); },
        |arg| { arg != 0 },
        |s, t| { let b = xitem_boolean_equal(s, t)?; return Ok(! b); });
}

pub fn general_compare_lt(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { xitem_numeric_less_than(s, t) },
        |arg| { arg < 0 },
        |s, t| { xitem_boolean_less_than(s, t) });
}

pub fn general_compare_le(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { let b = xitem_numeric_greater_than(s, t)?; return Ok(! b); },
        |arg| { arg <= 0 },
        |s, t| { let b = xitem_boolean_greater_than(s, t)?; return Ok(! b); });
}

pub fn general_compare_gt(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { xitem_numeric_greater_than(s, t) },
        |arg| { arg > 0 },
        |s, t| { xitem_boolean_greater_than(s, t) });
}

pub fn general_compare_ge(lhs: &XSequence, rhs: &XSequence) -> Result<XSequence, Box<Error>> {
    return general_comparison(lhs, rhs,
        |s, t| { let b = xitem_numeric_less_than(s, t)?; return Ok(! b); },
        |arg| { arg >= 0 },
        |s, t| { let b = xitem_boolean_less_than(s, t)?; return Ok(! b); });
}

// ---------------------------------------------------------------------
//
fn general_comparison<FNUM, FSTR, FBOOL>(lhs: &XSequence, rhs: &XSequence,
            mut num_op: FNUM, mut str_cmp: FSTR, mut bool_op: FBOOL)
                                            -> Result<XSequence, Box<Error>>
    where FNUM: FnMut(&XItem, &XItem) -> Result<bool, Box<Error>>,
          FSTR: FnMut(i64) -> bool,
          FBOOL: FnMut(&XItem, &XItem) -> Result<bool, Box<Error>> {

    for xitem_lhs in lhs.atomize().iter() {
        for xitem_rhs in rhs.atomize().iter() {
            if let Ok(b) = num_op(&xitem_lhs, &xitem_rhs) {
                if b == true {
                    return Ok(new_singleton_boolean(true));
                }
            }
            if let Ok(n) = xitem_compare(&xitem_lhs, &xitem_rhs) {
                let b = str_cmp(n);
                if b == true {
                    return Ok(new_singleton_boolean(true));
                }
            }
            if let Ok(b) = bool_op(&xitem_lhs, &xitem_rhs) {
                if b == true {
                    return Ok(new_singleton_boolean(true));
                }
            }
        }
    }
    return Ok(new_singleton_boolean(false));

}

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath_impl::helpers::compress_spaces;
    use xpath_impl::helpers::subtest_xpath;
    use xpath_impl::helpers::subtest_eval_xpath;

    // -----------------------------------------------------------------
    // 6.3 Comparison Operators on Numeric Values
    // 7.3 Equality and Comparison of Strings
    // 9.2 Operators on Boolean Values
    //     比較演算子 (Value Compare / General Compare)
    //
    // -----------------------------------------------------------------
    //
    #[test]
    fn test_compare_general() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("compare_general", &xml, &[
            ( "3 = 3", "(true)" ),
            ( "3 = 5", "(false)" ),
            ( "true() = true()", "(true)" ),
            ( "true() = false()", "(false)" ),
            ( "'ABC' = 'DEF'", "(false)" ),
            ( "'ABC' = 'ABC'", "(true)" ),

            ( "3 < 5", "(true)" ),
            ( "3 > 5", "(false)" ),
            ( "3 <= 5", "(true)" ),
            ( "5 <= 3", "(false)" ),
            ( "3 >= 5", "(false)" ),
            ( "5 >= 3", "(true)" ),

            ( "(3 = 3) = true()", "(true)" ),
            ( "(3 = 10) = true()", "(false)" ),
            ( "(3 = 3) < true()", "(false)" ),
            ( "(3 = 10) < true()", "(true)" ),
            ( "(3 = 3) < false()", "(false)" ),
            ( "(3 = 10) < false()", "(false)" ),

            // 異なる型どうしの比較
            ( "'ABC' = true()", "(false)" ),
            ( "'' = true()", "(false)" ),
            ( "5 = true()", "(false)" ),
            ( "0 = true()", "(false)" ),
            ( "10 = '10'", "(false)" ),
            ( "10 != '10'", "(false)" ),
            ( "5 <= '10'", "(false)" ),
            ( "10 <= '5'", "(false)" ),

            // Division by zero
            ( "3 div 0", "Dynamic Error" ),
            ( "3.0 div 0.0", "Dynamic Error" ),
            ( "'5' <= 3 div 0", "Dynamic Error" ),
            ( "'5' <= 0 div 0", "Dynamic Error" ),
            ( "0 div 0 = 0 div 0", "Dynamic Error" ),
            ( "0 div 0 != 0 div 0", "Dynamic Error" ),

        ]);
    }

    // -----------------------------------------------------------------
    // Infinity
    //
    #[test]
    fn test_compare_infinity() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("compare_infinity", &xml, &[
            ( "999 < 3e0 div 0e0 ", "(true)" ),
            ( "-3e0 div 0e0 < -999", "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // NaN
    //
    #[test]
    fn test_compare_nan() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("compare_nan", &xml, &[
            ( "3.0e1 = 0e0 div 0e0", "(false)" ),
            ( "0e0 div 0e0 = 0e0 div 0e0", "(false)" ),

            ( "'NaN' = 'NaN'", "(true)" ),
            ( "'NaN' != 'NaN'", "(false)" ),
            ( "'NaN' <= 'NaN'", "(true)" ),
            ( "'NaN' < 'NaN'", "(false)" ),
                // 文字列のままで比較。

            ( "number('NaN') = number('NaN')", "(false)" ),
            ( "number('NaN') != number('NaN')", "(true)" ),
                // 明示的に number() で変換した場合。
        ]);
    }

    // -----------------------------------------------------------------
    // Value Compare
    //
    #[test]
    fn test_compare_value() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);
        subtest_eval_xpath("compare_value", &xml, &[
            ( "false() eq true()", "(false)" ),
            ( "false() ne true()", "(true)" ),
            ( "false() lt true()", "(true)" ),
            ( "(1, 2) eq (2, 3)", "Type Error" ),

            ( "3 lt 5", "(true)" ),
            ( "3 gt 5", "(false)" ),
            ( "3 le 5", "(true)" ),
            ( "5 le 3", "(false)" ),
            ( "3 ge 5", "(false)" ),
            ( "5 ge 3", "(true)" ),

            ( r#"'abc' eq 'abc'"#, "(true)" ),
            ( r#"'abc' le 'abc'"#, "(true)" ),
            ( r#"'100' le '99'"#, "(true)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 比較演算子 (シングルトンでないシーケンスの比較)
    //
    #[test]
    fn test_compare_sequence() {
        let xml = compress_spaces(r#"
<root base="base">
</root>
        "#);

        subtest_eval_xpath("test_eval_xpath", &xml, &[
            ( "(1, 2) = (1, 3)", "(true)" ),
            ( "(1, 2) != (1, 3)", "(true)" ),
            ( "(1, 2) = (3, 4)", "(false)" ),
            ( "(1, 2) < (2, 4)", "(true)" ),
            ( "(5, 5) < (2, 4)", "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 比較演算子 (ノード集合と原子値)
    //
    #[test]
    fn test_compare_nodeset_and_atomic() {
        let xml = compress_spaces(r#"
<a base="base">
    <b>red</b>
    <c>green</c>
    <c>blue</c>
    <d>94</d>
</a>
        "#);

        subtest_eval_xpath("compare_nodeset_and_atomic", &xml, &[
            ( "/a/b = 'red'", "(true)" ),
            ( "/a/b eq 'red'", "(true)" ),
            ( "/a/c = 'green'", "(true)" ),
            ( "/a/c eq 'green'", "Type Error" ),
            ( "/a/c[1] eq 'green'", "(true)" ),
            ( "/a/d = '94' ", "(true)" ),
            ( "/a/d cast as integer = 94 ", "(true)" ),
            ( "/a/d cast as decimal = 94 ", "(true)" ),
            ( "/a/d cast as decimal = 94.0 ", "(true)" ),
        ]);

    }

    // -----------------------------------------------------------------
    // 比較演算子 (シングルトンでないシーケンスの比較)
    //
    #[test]
    fn test_compare_nodeset() {
        let xml = compress_spaces(r#"
<a base="base">
    <lhs>
        <p>ABC</p>
        <p>DEF</p>
        <p>100</p>
    </lhs>
    <rhs>
        <p>D<b>E</b>F</p>
        <p>GHI</p>
    </rhs>
    <empty/>
</a>
        "#);

        subtest_eval_xpath("compare_nodeset", &xml, &[
            // [ノード集合を含む場合]
            // 両方ともノード集合: 双方からそれぞれ選んだノードで、
            // 文字列値の比較結果が真になるものがあれば、真とする。
            ( "/a/lhs/p = /a/rhs/p", "(true)" ),
            ( "/a/lhs/p = /a/empty/p", "(false)" ),
            ( "/a/lhs/p < /a/rhs/p", "(true)" ),
            ( "/a/lhs/p > /a/rhs/p", "(false)" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 比較演算子: 属性
    //
    #[test]
    fn test_compare_attr() {
        let xml = compress_spaces(r#"
<a base="base">
    <p attr='a' img='A' />
    <p attr='x' img='X' />
    <p attr=''  img='E' />
    <p          img='V' />
</a>
        "#);

        subtest_xpath("compare_attr", &xml, false, &[
            ( "/a/p[@attr = 'a']", "A" ),
            ( "/a/p[@attr != 'a']", "XE" ),
            ( "/a/p[not(@attr = 'a')]", "XEV" ),
            ( "/a/p[not(@attr != 'a')]", "AV" ),
        ]);
    }

}

