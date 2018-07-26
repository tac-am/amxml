//
// xpath_impl/xitem.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::error::Error;
use std::fmt;
use std::f64;
use std::i64;
use std::ops::Neg;
use std::ops::Rem;
use std::str::FromStr;

use dom::*;
use xmlerror::*;
use xpath_impl::parser::*;
use xpath_impl::xsequence::*;

// ---------------------------------------------------------------------
//
fn atof(s: &str) -> f64 {
    return f64::from_str(s.trim()).unwrap_or(f64::NAN);
}

fn atoi(s: &str) -> i64 {
    return i64::from_str(s.trim()).unwrap_or(0);
}

fn f64_to_i64(f: f64) -> i64 {
    // return i64::from_str(&format!("{}", f)).unwrap_or(i64::MAX);
    return f as i64;
}

fn i64_to_f64(n: i64) -> f64 {
    return n as f64;
}

fn int_to_dec(n: i64) -> f64 {          // 代替
    return atof(&format!("{}.0", n));
}

fn dec_to_dbl(n: f64) -> f64 {          // 昇格
    return n;
}

fn int_to_dbl(n: i64) -> f64 {          // 代替と昇格
    return atof(&format!("{}.0", n));
}

// =====================================================================
// An [item] is either an atomic value or a node.
// An [atomic value] is a value in the value space of an atomic type.
// atomic type: https://www.w3.org/TR/xmlschema-0/ (Table 2)
//
#[derive(Debug, PartialEq, Clone)]
pub enum XItem {
    XItemXNodePtr {
        value: XNodePtr,
            // (内部処理用) インライン函数をXItemとして扱う。
    },
    XIMap {
        value: XSeqMap,
    },
    XIArray {
        value: XSeqArray,
    },
    XINode {
        value: NodePtr,
    },
    XIString {
        value: String,
    },
    // XINormalizedString,
    // XIToken,
    // XIBase64Binary,
    // XIHexBinary,
    XIInteger {
        value: i64,
    },
    // XIPositiveInteger,
    // XINegativeInteger,
    // XINonNegativeInteger,
    // XINonPositiveInteger,
    // XILong,
    // XIUnsignedLong,
    // XIInt,
    // XIUnsignedInt,
    // XIShort,
    // XIUnsignedShort,
    // XIByte,
    // XIUnsignedByte,
    XIDecimal {
        value: f64,
    },
    // XIFloat,
    XIDouble {
        value: f64,
    },
    XIBoolean {
        value: bool,
    },
    // XIDuration,
    // XIDateTime,
    // XIDate,
    // XITime,
    // XIGYear,
    // XIGYearMonth,
    // XIGMonth,
    // XIGMonthDay,
    // XIGDay,
    // XIName,
    // XIQName,
    // XINCName,
    // XIAnyURI,
    // XILanguage,
    // XIID,
    // XIIDREF,
    // XIIDREFS,
    // XIENTITY,
    // XIENTITIES,
    // XINOTATION,
    // XINMTOKEN,
    // XINMTOKENS,
}

// =====================================================================
//
#[derive(Debug, PartialEq, Clone)]
pub struct XSeqMap {
    v: Vec<(XItem, XSequence)>,
}

impl XSeqMap {
    pub fn map_get(&self, key: &XItem) -> Option<XSequence> {
        for entry in self.v.iter() {
            let c = xitem_compare(&entry.0, &key);
            match c {
                Ok(c) => {
                    if c == 0 {
                        return Some(entry.1.clone());
                    }
                },
                _ => {},
            }
        }
        return None;
    }
}

// =====================================================================
//
#[derive(Debug, PartialEq, Clone)]
pub struct XSeqArray {
    v: Vec<XSequence>,
}

impl XSeqArray {
    pub fn array_get(&self, index: &XItem) -> Option<XSequence> {
        let i = index.get_as_raw_integer();
        match i {
            Ok(i) => {
                if 1 <= i && i <= self.v.len() as i64 {
                    return Some(self.v[(i - 1) as usize].clone());
                }
            },
            _ => {},
        }
        return None;
    }
}

// =====================================================================
//
pub fn new_xitem_xnodeptr(xnode: &XNodePtr) -> XItem {
    return XItem::XItemXNodePtr {
        value: xnode.clone(),
    }
}

pub fn new_xitem_node(node: &NodePtr) -> XItem {
    return XItem::XINode {
        value: node.rc_clone(),
    };
}

pub fn new_xitem_map(value: &Vec<(XItem, XSequence)>) -> XItem {
    return XItem::XIMap {
        value: XSeqMap {
            v: value.clone(),
        },
    };
}

pub fn new_xitem_array(value: &Vec<XSequence>) -> XItem {
    return XItem::XIArray{
        value: XSeqArray {
            v: value.clone(),
        },
    };
}

pub fn new_xitem_string(value: &str) -> XItem {
    return XItem::XIString{value: value.to_string()};
}

pub fn new_xitem_integer(value: i64) -> XItem {
    return XItem::XIInteger{value};
}

pub fn new_xitem_decimal(value: f64) -> XItem {
    return XItem::XIDecimal{value};
}

pub fn new_xitem_double(value: f64) -> XItem {
    return XItem::XIDouble{value};
}

pub fn new_xitem_boolean(value: bool) -> XItem {
    return XItem::XIBoolean{value};
}

// =====================================================================
//
impl NodePtr {
    // -----------------------------------------------------------------
    //
    fn node_dump(&self) -> String {
        let mut dump_str = String::new();
        match self.node_type() {
            NodeType::DocumentRoot => {
                dump_str += &"(DocumentRoot)";
            },
            NodeType::Element => {
                dump_str += &"<";
                dump_str += &self.name();
                for at in self.attributes().iter() {
                    dump_str += &format!(r#" {}="{}""#,
                        at.name(), at.value());
                }
                dump_str += &">";
            },
            NodeType::Text => {
                dump_str += &self.value();
            },
            NodeType::Attribute => {
                dump_str += &format!(r#"{}="{}""#, self.name(), self.value());
            },
            _ => {},
        }
        return dump_str;
    }

    // =================================================================
    // Returns the string value of DOM node.
    //
    fn string_value(&self) -> String {
        match self.node_type() {
            NodeType::DocumentRoot | NodeType::Element => {
                let mut s = String::new();
                for ch in self.children().iter() {
                    s += &ch.string_value();
                }
                return s;
            },
            NodeType::Text | NodeType::Attribute | NodeType::Comment => {
                return format!("{}", self.value());
            },
            NodeType::XMLDecl | NodeType::Instruction => {
                return format!("{} {}", self.name(), self.value());
            },
            _ => return String::new(),
        }
    }

    // =================================================================
    // Returns the typed value of DOM node.
    // 型註釈がないとすれば、string_valueと同じ結果になる。
    // 型註釈を考慮するならば、戻り値型はStringでなくXItemであるべきかも
    // 知れない。
    //
    // (XML Path Language (XPath) 2.0 (Second Edition).htm)
    // 1. For text and document nodes, the typed value of the node is
    //    the same as its string value, as an instance of the type
    //    xs:untypedAtomic.
    // 2. The typed value of a comment, namespace, or processing instruction
    //    node is the same as its string value. It is an instance of the type
    //    xs:string.
    // 3. The typed value of an attribute node with the type annotation
    //    xs:anySimpleType or xs:untypedAtomic is the same as its string
    //    value, as an instance of xs:untypedAtomic.
    //    (他のtype annotationについては未実装)
    // 4. For an element node:
    //   a. If the type annotation is xs:untyped or xs:anySimpleType or
    //      denotes a complex type with mixed content (including xs:anyType),
    //      then the typed value of the node is equal to its string value,
    //      as an instance of xs:untypedAtomic.
    //      (nilledプロパティー、他のtype annotationについては未実装)
    //
    fn typed_value(&self) -> String {
        match self.node_type() {
            NodeType::Text => {                     // xs:untypedAtomic
                return format!("{}", self.value());
            },
            NodeType::DocumentRoot => {             // xs:untypedAtomic
                let mut s = String::new();
                for ch in self.children().iter() {
                    s += &ch.typed_value();
                }
                return s;
            },
            NodeType::Comment => {                  // xs:string
                return format!("{}", self.value());
            },
            NodeType::Instruction => {              // xs:string
                return format!("{} {}", self.name(), self.value());
            },
            NodeType::Attribute => {                // xs:untypedAtomic
                return format!("{}", self.value());
            },
            NodeType::Element => {                  // xs:untypedAtomic
                let mut s = String::new();
                for ch in self.children().iter() {
                    s += &ch.typed_value();
                }
                return s;
            },
            _ => return String::new(),
        }
    }

}

// =====================================================================
// Trait std::fmt::Display
//
impl fmt::Display for XItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            XItem::XINode{value} => {
                return write!(f, "{}", value.node_dump());
            },
            XItem::XIString{value} => {
                return write!(f, r#""{}""#, value);
            },
            XItem::XIInteger{value} => {
                return write!(f, "{}", value);
            },
            XItem::XIDecimal{value} => {
                let mut s = String::from(format!("{}", value));
                if ! s.contains(".") {
                    s += &".0";
                }
                return write!(f, "{}", s);
            },
            XItem::XIDouble{value} => {
                if value.is_nan() {
                    return write!(f, "NaN");
                } else if value.is_infinite() {
                    if value.signum() == 1.0 {
                        return write!(f, "+Infinity");
                    } else {
                        return write!(f, "-Infinity");
                    }
                } else if *value == 0.0 && value.signum() == -1.0 {
                    return write!(f, "-0e0");
                } else {
                    return write!(f, "{:e}", value);
                }
            },
            XItem::XIBoolean{value} => {
                if *value == true {
                    return write!(f, "true");
                } else {
                    return write!(f, "false");
                }
            },
            XItem::XItemXNodePtr{value} => {
                return write!(f, "{}", value);
            },
            XItem::XIMap{value} => {
                let mut s = String::from("{");
                for (i, v) in value.v.iter().enumerate() {
                    if i != 0 {
                        s += &", ";
                    }
                    s += &format!("{} => {}", v.0, v.1);
                }
                s += &"}";
                return write!(f, "{}", s);
            },
            XItem::XIArray{value} => {
                let mut s = String::from("[");
                for (i, v) in value.v.iter().enumerate() {
                    if i != 0 {
                        s += &", ";
                    }
                    s += &format!("{}", v);
                }
                s += &"]";
                return write!(f, "{}", s);
            },
        }
    }
}

// =====================================================================
//
impl XItem {
    // -----------------------------------------------------------------
    //
    pub fn as_nodeptr(&self) -> Option<NodePtr> {
        match self {
            XItem::XINode{value} => return Some(value.rc_clone()),
            _ => return None,
        }
    }

    // -----------------------------------------------------------------
    // 原子化
    // 型註釈がないとすれば、XINodeを原子化した結果は常にXIStringである。
    //
    // (XML Path Language (XPath) 2.0 (Second Edition).htm)
    // 2.4.2 Atomization
    // - If the item is an atomic value, it is returned.
    // - If the item is a node, its typed value is returned.
    //
    pub fn atomize(&self) -> XItem {
        match self {
            XItem::XINode{value} => {
                return XItem::XIString {
                    value: value.typed_value(),
                }
            },
            _ => return self.clone(),
        }
    }

    // -----------------------------------------------------------------
    //
    pub fn is_item(&self) -> bool {
        match self {
            XItem::XItemXNodePtr{value: _} => return false,
            _ => return true,
        }
    }

    // -----------------------------------------------------------------
    //
    pub fn is_numeric(&self) -> bool {
        match self {
            XItem::XIInteger{value: _} => return true,
            XItem::XIDecimal{value: _} => return true,
            XItem::XIDouble{value: _} => return true,
            _ => return false,
        }
    }

    // -----------------------------------------------------------------
    // キャスト可能か否か。
    //
    pub fn castable_as(&self, type_name: &str) -> bool {
        match self.cast_as(type_name) {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }

    // -----------------------------------------------------------------
    // キャスト。
    //     原子化を施してからキャストするので、XItem::XINodeの場合については
    //     考えなくてよい。
    //
    pub fn cast_as(&self, type_name: &str) -> Result<XItem, Box<Error>> {
        match type_name {
            "string" => {
                if let Ok(s) = self.get_as_raw_string() {
                    return Ok(new_xitem_string(&s));
                }
            },
            "double" => {
                if let Ok(d) = self.get_as_raw_double() {
                    return Ok(new_xitem_double(d));
                }
            },
            "decimal" => {
                if let Ok(d) = self.get_as_raw_decimal() {
                    return Ok(new_xitem_decimal(d));
                }
            },
            "integer" => {
                if let Ok(i) = self.get_as_raw_integer() {
                    return Ok(new_xitem_integer(i));
                }
            },
            "boolean" => {
                if let Ok(b) = self.get_as_raw_boolean() {
                    return Ok(new_xitem_boolean(b));
                }
            },
            _ => {},
        }
        return Err(type_error!("Item {}: can't cast to {}",
                                self.to_string(), type_name));
    }

    // -----------------------------------------------------------------
    //
    pub fn get_as_raw_string(&self) -> Result<String, Box<Error>> {
        match self {
            XItem::XINode{value} => {
                return Ok(value.string_value());
            },
            XItem::XIString{value} => {
                return Ok(value.clone());
            },
            XItem::XIInteger{value} => {
                return Ok(String::from(format!("{}", value)));
            },
            XItem::XIDecimal{value} => {
                return Ok(String::from(format!("{}", value)));
            },
            XItem::XIDouble{value} => {
                if value.is_nan() {
                    return Ok(String::from("NaN"));
                } else if value.is_infinite() {
                    if value.signum() == 1.0 {
                        return Ok(String::from("+Infinity"));
                    } else {
                        return Ok(String::from("-Infinity"));
                    }
                } else {
                    return Ok(String::from(format!("{}", value)));
                }
            },
            XItem::XIBoolean{value} => {
                if *value == true {
                    return Ok(String::from("true"));
                } else {
                    return Ok(String::from("false"));
                }
            },
            _ => {},
        }
        return Err(type_error!(
                "Item {}: can't cast to string", self.to_string()));
    }

    // -----------------------------------------------------------------
    //
    pub fn get_as_raw_double(&self) -> Result<f64, Box<Error>> {
        match self {
            XItem::XINode{value} => {
                return Ok(atof(&value.string_value()));
            },
            XItem::XIString{ref value} => {
                return Ok(atof(value.as_str()));
            },
            XItem::XIInteger{ref value} => {
                return Ok(i64_to_f64(*value));
            },
            XItem::XIDecimal{ref value} => {
                return Ok(*value);
            },
            XItem::XIDouble{ref value} => {
                return Ok(*value);
            },
            XItem::XIBoolean{value} => {
                return Ok(if *value == true { 1.0 } else { 0.0 });
            },
            _ => {},
        }
        return Err(type_error!(
                "Item {}: can't cast to double", self.to_string()));
    }

    // -----------------------------------------------------------------
    //
    pub fn get_as_raw_decimal(&self) -> Result<f64, Box<Error>> {
        match self {
            XItem::XINode{value} => {
                return Ok(atof(&value.string_value()));
            },
            XItem::XIString{ref value} => {
                return Ok(atof(value.as_str()));
            },
            XItem::XIInteger{ref value} => {
                return Ok(i64_to_f64(*value));
            },
            XItem::XIDecimal{ref value} => {
                return Ok(*value);
            },
            XItem::XIDouble{ref value} => {
                return Ok(*value);
            },
            XItem::XIBoolean{value} => {
                return Ok(if *value == true { 1.0 } else { 0.0 });
            },
            _ => {},
        }
        return Err(type_error!(
                "Item {}: can't cast to decimal", self.to_string()));
    }

    // -----------------------------------------------------------------
    //
    pub fn get_as_raw_integer(&self) -> Result<i64, Box<Error>> {
        match self {
            XItem::XINode{value} => {
                return Ok(atoi(&value.string_value()));
            },
            XItem::XIInteger{value} => return Ok(*value),
            XItem::XIString{value} => {
                return Ok(atoi(value));
            },
            XItem::XIDecimal{value} => return Ok(f64_to_i64(*value)),
            XItem::XIDouble{value} => return Ok(f64_to_i64(*value)),
                        // dbl->intはキャストできない場合がある。
                        // NaNなど?
            XItem::XIBoolean{value} => {
                if *value == true {
                    return Ok(1);
                } else {
                    return Ok(0);
                }
            },
            _ => {},
        }
        return Err(type_error!(
                "Item {}: can't cast to integer", self.to_string()));
    }

    // -----------------------------------------------------------------
    //
    pub fn get_as_raw_boolean(&self) -> Result<bool, Box<Error>> {
        match self {
            XItem::XINode{value} => {
                match value.string_value().as_str() {
                    "true" | "1" => return Ok(true),
                    "false" | "0" => return Ok(false),
                    _ => {},
                }
            },
            XItem::XIInteger{value} => return Ok(*value != 0),
            XItem::XIString{value} => {
                match value.as_str() {
                    "true" | "1" => return Ok(true),
                    "false" | "0" => return Ok(false),
                    _ => {},
                }
            },
            XItem::XIDecimal{value} => {
                if *value == 0.0 || value.is_nan() {
                    return Ok(false);
                } else {
                    return Ok(true);
                }
            },
            XItem::XIDouble{value} => {
                if *value == 0.0 || value.is_nan() {
                    return Ok(false);
                } else {
                    return Ok(true);
                }
            },
            XItem::XIBoolean{value} => {
                return Ok(*value);
            },
            _ => {},
        }
        return Err(type_error!(
                "Item {}: can't cast to boolean", self.to_string()));
    }
}

// ---------------------------------------------------------------------
// 文字列としての比較。
//
pub fn xitem_compare(lhs: &XItem, rhs: &XItem) -> Result<i64, Box<Error>> {
    match lhs {
        XItem::XIString{value: lhs} => {
            match rhs {
                XItem::XIString{value: rhs} => {
                    if lhs < rhs {
                        return Ok(-1);
                    } else if lhs == rhs {
                        return Ok(0);
                    } else {
                        return Ok(1);
                    }
                },
                _ => {},
            }
        },
        _ => {},
    }
    return Err(type_error!("xitem_compare: Not string"));
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_add(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    return xitem_numeric_operation(lhs, rhs,
                |a, b| { a + b },
                |a, b| { a + b },
                |a, b| { a + b });
}

pub fn xitem_numeric_subtract(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    return xitem_numeric_operation(lhs, rhs,
                |a, b| { a - b },
                |a, b| { a - b },
                |a, b| { a - b });
}

pub fn xitem_numeric_multiply(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    return xitem_numeric_operation(lhs, rhs,
                |a, b| { a * b },
                |a, b| { a * b },
                |a, b| { a * b });
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_divide(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    let rhs_a = match rhs {
        XItem::XIInteger{value: rhs} => {
            if *rhs == 0 {
                return Err(dynamic_error!("Division by zero"));
            }
            new_xitem_decimal(i64_to_f64(*rhs))
                            // 例外: Integer div Integer => Decimal
        },
        XItem::XIDecimal{value: rhs} => {
            if *rhs == 0.0 {
                return Err(dynamic_error!("Division by zero"));
            }
            new_xitem_decimal(*rhs)
        },
        XItem::XIDouble{value: rhs} => new_xitem_double(*rhs),
        _ => return Err(cant_occur!("xitem_numeric_divide: rhs_a")),
    };
    return xitem_numeric_operation(lhs, &rhs_a,
                |a, b| { a / b },
                |a, b| { a / b },
                |a, b| { a / b });
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_integer_divide(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    match lhs {
        XItem::XIDouble{value} => {
            if value.is_nan() {
                return Err(dynamic_error!("Numeric operation overflow/underflow."));
            }
            if ! value.is_finite() {
                return Err(dynamic_error!("Numeric operation overflow/underflow."));
            }
        },
        _ => {},
    }
    match rhs {
        XItem::XIDouble{value} => {
            if value.is_nan() {
                return Err(dynamic_error!("Numeric operation overflow/underflow."));
            }
        },
        _ => {},
    }

    let lhs = match lhs {
        XItem::XIInteger{value} => *value,
        XItem::XIDecimal{value} => f64_to_i64(*value),
        XItem::XIDouble{value} => f64_to_i64(*value),
        _ => 0,
    };
    let rhs = match rhs {
        XItem::XIInteger{value} => *value,
        XItem::XIDecimal{value} => f64_to_i64(*value),
        XItem::XIDouble{value} => f64_to_i64(*value),
        _ => 0,
    };
    if rhs != 0 {
        return Ok(new_xitem_integer(lhs / rhs));
    } else {
        return Err(dynamic_error!("Division by zero"));
    }
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_mod(lhs: &XItem, rhs: &XItem) -> Result<XItem, Box<Error>> {
    match rhs {
        XItem::XIInteger{value: rhs} => {
            if *rhs == 0 {
                return Err(dynamic_error!("Division by zero"));
            }
        },
        XItem::XIDecimal{value: rhs} => {
            if *rhs == 0.0 {
                return Err(dynamic_error!("Division by zero"));
            }
        },
        _ => {},
    }

    return xitem_numeric_operation(lhs, rhs,
                |a, b| { a.rem(b) },
                |a, b| { a.rem(b) },
                |a, b| { a.rem(b) });
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_unary_plus(arg: &XItem) -> Result<XItem, Box<Error>> {
    match arg {
        XItem::XIInteger{value} => return Ok(new_xitem_integer(*value)),
        XItem::XIDecimal{value} => return Ok(new_xitem_decimal(*value)),
        XItem::XIDouble{value} => return Ok(new_xitem_double(*value)),
        _ => return Err(type_error!("xitem_numeric_operation: Not numeric")),
    }
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_unary_minus(arg: &XItem) -> Result<XItem, Box<Error>> {
    match arg {
        XItem::XIInteger{value} => return Ok(new_xitem_integer(value.neg())),
        XItem::XIDecimal{value} => return Ok(new_xitem_decimal(value.neg())),
        XItem::XIDouble{value} => return Ok(new_xitem_double(value.neg())),
        _ => return Err(type_error!("xitem_numeric_operation: Not numeric")),
    }
                        // 「0 - arg」という形の実装は、argがゼロの時、
                        // 負のゼロにならないことに註意。
}

// ---------------------------------------------------------------------
//
fn xitem_numeric_operation<FINT, FDEC, FDBL>(lhs: &XItem, rhs: &XItem,
        mut int_op: FINT, mut dec_op: FDEC, mut dbl_op: FDBL) -> Result<XItem, Box<Error>>
        where FINT: FnMut(i64, i64) -> i64,
              FDEC: FnMut(f64, f64) -> f64,
              FDBL: FnMut(f64, f64) -> f64 {

    match lhs {
        XItem::XIInteger{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(new_xitem_integer(int_op(*lhs, *rhs)));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(new_xitem_decimal(dec_op(int_to_dec(*lhs), *rhs)));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(new_xitem_double(dbl_op(int_to_dbl(*lhs), *rhs)));
                },
                _ => {},
            }
        },
        XItem::XIDecimal{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(new_xitem_decimal(dec_op(*lhs, int_to_dec(*rhs))));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(new_xitem_decimal(dec_op(*lhs, *rhs)));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(new_xitem_double(dec_op(dec_to_dbl(*lhs), *rhs)));
                },
                _ => {},
            }
        },
        XItem::XIDouble{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(new_xitem_double(dbl_op(*lhs, int_to_dbl(*rhs))));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(new_xitem_double(dbl_op(*lhs, dec_to_dbl(*rhs))));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(new_xitem_double(dbl_op(*lhs, *rhs)));
                },
                _ => {},
            }
        },
        _ => {},
    }
    return Err(type_error!("xitem_numeric_operation: Not numeric"));
}

// ---------------------------------------------------------------------
//
pub fn xitem_numeric_equal(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    return xitem_numeric_comparison(lhs, rhs,
            |a, b| { a == b },
            |a, b| { a == b },
            |a, b| { a == b });
}

pub fn xitem_numeric_less_than(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    return xitem_numeric_comparison(lhs, rhs,
            |a, b| { a < b },
            |a, b| { a < b },
            |a, b| { a < b });
}

pub fn xitem_numeric_greater_than(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    return xitem_numeric_comparison(lhs, rhs,
            |a, b| { a > b },
            |a, b| { a > b },
            |a, b| { a > b });
}

// ---------------------------------------------------------------------
// 数値と数値の比較: 必要に応じ、型を昇格する。
//
fn xitem_numeric_comparison<FINT, FDEC, FDBL>(lhs: &XItem, rhs: &XItem,
        mut int_op: FINT, mut dec_op: FDEC, mut dbl_op: FDBL) -> Result<bool, Box<Error>>
        where FINT: FnMut(i64, i64) -> bool,
              FDEC: FnMut(f64, f64) -> bool,
              FDBL: FnMut(f64, f64) -> bool {

    match lhs {
        XItem::XIInteger{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(int_op(*lhs, *rhs));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(dec_op(int_to_dec(*lhs), *rhs));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(dbl_op(int_to_dbl(*lhs), *rhs));
                },
                _ => {},
            }
        },
        XItem::XIDecimal{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(dec_op(*lhs, int_to_dec(*rhs)));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(dec_op(*lhs, *rhs));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(dbl_op(dec_to_dbl(*lhs), *rhs));
                },
                _ => {},
            }
        },
        XItem::XIDouble{value: lhs} => {
            match rhs {
                XItem::XIInteger{value: rhs} => {
                    return Ok(dbl_op(*lhs, int_to_dbl(*rhs)));
                },
                XItem::XIDecimal{value: rhs} => {
                    return Ok(dbl_op(*lhs, dec_to_dbl(*rhs)));
                },
                XItem::XIDouble{value: rhs} => {
                    return Ok(dbl_op(*lhs, *rhs));
                },
                _ => {},
            }
        },
        _ => {},
    }
    return Err(type_error!("xitem_numeric_comparison: Not numeric"));
}

// ---------------------------------------------------------------------
//
pub fn xitem_boolean_equal(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    if let XItem::XIBoolean{value: lhs} = lhs {
        if let XItem::XIBoolean{value: rhs} = rhs {
            return Ok(*lhs == *rhs);
        }
    }
    return Err(type_error!("xitem_boolean_equal: Not boolean"));
}

pub fn xitem_boolean_less_than(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    if let XItem::XIBoolean{value: lhs} = lhs {
        if let XItem::XIBoolean{value: rhs} = rhs {
            return Ok(*lhs == false && *rhs == true);
        }
    }
    return Err(type_error!("xitem_boolean_less_than: Not boolean"));
}

pub fn xitem_boolean_greater_than(lhs: &XItem, rhs: &XItem) -> Result<bool, Box<Error>> {
    if let XItem::XIBoolean{value: lhs} = lhs {
        if let XItem::XIBoolean{value: rhs} = rhs {
            return Ok(*lhs == true && *rhs == false);
        }
    }
    return Err(type_error!("xitem_boolean_greater_than: Not boolean"));
}

