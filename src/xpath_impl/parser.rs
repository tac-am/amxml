//
// xpath_impl/parser.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use xmlerror::*;
use xpath_impl::lexer::*;
use xpath_impl::func;
        // func::check_function_spec() を使う。

// =====================================================================
//
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum XNodeType {
    Nil,
    OperatorPath,
    AxisRoot,
    AxisAncestor,
    AxisAncestorOrSelf,
    AxisAttribute,
    AxisChild,
    AxisDescendant,
    AxisDescendantOrSelf,
    AxisFollowing,
    AxisFollowingSibling,
    AxisNamespace,
    AxisParent,
    AxisPreceding,
    AxisPrecedingSibling,
    AxisSelf,
    PredicateTop,
    PredicateRevTop,
    OperatorConcatenate,
    OperatorOr,
    OperatorAnd,
    OperatorGeneralEQ,
    OperatorGeneralNE,
    OperatorGeneralLT,
    OperatorGeneralGT,
    OperatorGeneralLE,
    OperatorGeneralGE,
    OperatorValueEQ,
    OperatorValueNE,
    OperatorValueLT,
    OperatorValueGT,
    OperatorValueLE,
    OperatorValueGE,
    OperatorAdd,
    OperatorSubtract,
    OperatorUnaryPlus,
    OperatorUnaryMinus,
    OperatorMultiply,
    OperatorDiv,
    OperatorIDiv,
    OperatorMod,
    OperatorUnion,
    OperatorIntersect,
    OperatorExcept,
    OperatorTo,
    OperatorIsSameNode,
    OperatorNodeBefore,
    OperatorNodeAfter,
    OperatorInstanceOf,
    OperatorTreatAs,
    OperatorCastableAs,
    OperatorCastAs,
    IfExpr,
    IfThenElse,
    ForExpr,
    SomeExpr,
    EveryExpr,
    ForVarBind,
    SomeVarBind,
    EveryVarBind,
    StringLiteral,
    IntegerLiteral,
    DecimalLiteral,
    DoubleLiteral,
    ContextItem,
    FunctionCall,
    ArgumentTop,
    VariableReference,
    ApplyPredicates,
    KindTest,
    DocumentTest,
    ElementTest,
    AttributeTest,
    SchemaElementTest,
    SchemaAttributeTest,
    PITest,
    CommentTest,
    TextTest,
    AnyKindTest,
    KindTestTypeName,
    EmptySequenceTest,
    ItemTest,
    AtomicType,
    SingleType,
    SequenceType,
}

impl XNodeType {
    pub fn to_string(&self) -> String {
        let xnode_desc: HashMap<XNodeType, &str> = [
            ( XNodeType::Nil,                  "Nil" ),
            ( XNodeType::OperatorPath,         "OperatorPath" ),
            ( XNodeType::AxisRoot,             "AxisRoot" ),
            ( XNodeType::AxisAncestor,         "AxisAncestor" ),
            ( XNodeType::AxisAncestorOrSelf,   "AxisAncestorOrSelf" ),
            ( XNodeType::AxisAttribute,        "AxisAttribute" ),
            ( XNodeType::AxisChild,            "AxisChild" ),
            ( XNodeType::AxisDescendant,       "AxisDescendant" ),
            ( XNodeType::AxisDescendantOrSelf, "AxisDescendantOrSelf" ),
            ( XNodeType::AxisFollowing,        "AxisFollowing" ),
            ( XNodeType::AxisFollowingSibling, "AxisFollowingSibling" ),
            ( XNodeType::AxisNamespace,        "AxisNamespace" ),
            ( XNodeType::AxisParent,           "AxisParent" ),
            ( XNodeType::AxisPreceding,        "AxisPreceding" ),
            ( XNodeType::AxisPrecedingSibling, "AxisPrecedingSibling" ),
            ( XNodeType::AxisSelf,             "AxisSelf" ),
            ( XNodeType::PredicateTop,         "PredicateTop" ),
            ( XNodeType::PredicateRevTop,      "PredicateRevTop" ),
            ( XNodeType::OperatorConcatenate,  "OperatorConcatenate" ),
            ( XNodeType::OperatorOr,           "OperatorOr" ),
            ( XNodeType::OperatorAnd,          "OperatorAnd" ),
            ( XNodeType::OperatorGeneralEQ,    "OperatorGeneralEQ" ),
            ( XNodeType::OperatorGeneralNE,    "OperatorGeneralNE" ),
            ( XNodeType::OperatorGeneralLT,    "OperatorGeneralLT" ),
            ( XNodeType::OperatorGeneralGT,    "OperatorGeneralGT" ),
            ( XNodeType::OperatorGeneralLE,    "OperatorGeneralLE" ),
            ( XNodeType::OperatorGeneralGE,    "OperatorGeneralGE" ),
            ( XNodeType::OperatorValueEQ,      "OperatorValueEQ" ),
            ( XNodeType::OperatorValueNE,      "OperatorValueNE" ),
            ( XNodeType::OperatorValueLT,      "OperatorValueLT" ),
            ( XNodeType::OperatorValueGT,      "OperatorValueGT" ),
            ( XNodeType::OperatorValueLE,      "OperatorValueLE" ),
            ( XNodeType::OperatorValueGE,      "OperatorValueGE" ),
            ( XNodeType::OperatorAdd,          "OperatorAdd" ),
            ( XNodeType::OperatorSubtract,     "OperatorSubtract" ),
            ( XNodeType::OperatorUnaryMinus,   "OperatorUnaryMinus" ),
            ( XNodeType::OperatorMultiply,     "OperatorMultiply" ),
            ( XNodeType::OperatorDiv,          "OperatorDiv" ),
            ( XNodeType::OperatorIDiv,         "OperatorIDiv" ),
            ( XNodeType::OperatorMod,          "OperatorMod" ),
            ( XNodeType::OperatorUnion,        "OperatorUnion" ),
            ( XNodeType::OperatorIntersect,    "OperatorIntersect" ),
            ( XNodeType::OperatorExcept,       "OperatorExcept" ),
            ( XNodeType::OperatorTo,           "OperatorTo" ),
            ( XNodeType::OperatorIsSameNode,   "OperatorIsSameNode" ),
            ( XNodeType::OperatorNodeBefore,   "OperatorNodeBefore" ),
            ( XNodeType::OperatorNodeAfter,    "OperatorNodeAfter" ),
            ( XNodeType::OperatorInstanceOf,   "OperatorInstanceOf" ),
            ( XNodeType::OperatorTreatAs,      "OperatorTreatAs" ),
            ( XNodeType::OperatorCastableAs,   "OperatorCastableAs" ),
            ( XNodeType::OperatorCastAs,       "OperatorCastAs" ),
            ( XNodeType::IfExpr,               "IfExpr" ),
            ( XNodeType::IfThenElse,           "IfThenElse" ),
            ( XNodeType::ForExpr,              "ForExpr" ),
            ( XNodeType::SomeExpr,             "SomeExpr" ),
            ( XNodeType::EveryExpr,            "EveryExpr" ),
            ( XNodeType::ForVarBind,           "ForVarBind" ),
            ( XNodeType::SomeVarBind,          "SomeVarBind" ),
            ( XNodeType::EveryVarBind,         "EveryVarBind" ),
            ( XNodeType::StringLiteral,        "StringLiteral" ),
            ( XNodeType::IntegerLiteral,       "IntegerLiteral" ),
            ( XNodeType::DecimalLiteral,       "DecimalLiteral" ),
            ( XNodeType::DoubleLiteral,        "DoubleLiteral" ),
            ( XNodeType::ContextItem,          "ContextItem" ),
            ( XNodeType::FunctionCall,         "FunctionCall" ),
            ( XNodeType::ArgumentTop,          "ArgumentTop" ),
            ( XNodeType::VariableReference,    "VariableReference" ),
            ( XNodeType::ApplyPredicates,      "ApplyPredicates" ),
            ( XNodeType::KindTest,             "KindTest" ),
            ( XNodeType::DocumentTest,         "DocumentTest" ),
            ( XNodeType::ElementTest,          "ElementTest" ),
            ( XNodeType::AttributeTest,        "AttributeTest" ),
            ( XNodeType::SchemaElementTest,    "SchemaElementTest" ),
            ( XNodeType::SchemaAttributeTest,  "SchemaAttributeTest" ),
            ( XNodeType::PITest,               "PITest" ),
            ( XNodeType::CommentTest,          "CommentTest" ),
            ( XNodeType::TextTest,             "TextTest" ),
            ( XNodeType::AnyKindTest,          "AnyKindTest" ),
            ( XNodeType::KindTestTypeName,     "KindTestTypeName" ),
            ( XNodeType::EmptySequenceTest,    "EmptySequenceTest" ),
            ( XNodeType::ItemTest,             "ItemTest" ),
            ( XNodeType::AtomicType,           "AtomicType" ),
            ( XNodeType::SingleType,           "SingleType" ),
            ( XNodeType::SequenceType,         "SequenceType" ),
        ].iter().cloned().collect();

        return xnode_desc.get(&self).unwrap_or(&"UNKNOWN").to_string();
    }
}

// =====================================================================
//
pub type XNodePtr = Rc<RefCell<XNode>>;

pub struct XNode {
    n_type: XNodeType,
    name: String,
    left: Option<XNodePtr>,
    right: Option<XNodePtr>,
}

// ---------------------------------------------------------------------
// 軸を表すノード型
//
pub fn is_xnode_axis(n_type: &XNodeType) -> bool {
    return [
        XNodeType::AxisRoot,
        XNodeType::AxisAncestor,
        XNodeType::AxisAncestorOrSelf,
        XNodeType::AxisAttribute,
        XNodeType::AxisChild,
        XNodeType::AxisDescendant,
        XNodeType::AxisDescendantOrSelf,
        XNodeType::AxisFollowing,
        XNodeType::AxisFollowingSibling,
        XNodeType::AxisNamespace,
        XNodeType::AxisParent,
        XNodeType::AxisPreceding,
        XNodeType::AxisPrecedingSibling,
        XNodeType::AxisSelf,
    ].contains(n_type);
}

// ---------------------------------------------------------------------
// 逆順軸を表すノード型
//
fn is_xnode_reverse_axis(n_type: &XNodeType) -> bool {
    return [
        XNodeType::AxisParent,              // XPath 1.0 では正順
        XNodeType::AxisAncestor,
        XNodeType::AxisAncestorOrSelf,
        XNodeType::AxisPreceding,
        XNodeType::AxisPrecedingSibling,
    ].contains(n_type);
}

// ---------------------------------------------------------------------
// 次にトークン $ttype が現れることを確認し、そうでなければエラーとする。
//
macro_rules! must_next_token {
    ( $lex: expr, $ttype: expr, $msg: expr ) => {
        if $lex.next_token().get_type() != $ttype {
            return Err(xpath_syntax_error!($msg,
                $lex.around_tokens().as_str()));
        }
    }
}

// ---------------------------------------------------------------------
// 次にトークン Name が現れ、その名前が $name であることを確認し、
// そうでなければエラーとする。
//      字句解析器ではキーワードか否か判断できないトークン
//          then else in return satisfies
//      については、TType::Nameとして返される。
//
macro_rules! must_next_name {
    ( $lex: expr, $name: expr, $msg: expr ) => {
        if $lex.next_token().get_type() != TType::Name &&
           $lex.next_token().get_name() != $name {
            return Err(xpath_syntax_error!($msg,
                $lex.around_tokens().as_str()));
        }
    }
}

// ---------------------------------------------------------------------
// 次にトークン $ttype が現れるかどうか調べ、そうでなければ nil を返す。
//
macro_rules! check_next_token {
    ( $lex: expr, $ttype: expr ) => {
        if $lex.next_token().get_type() != $ttype {
            return Ok(new_nil_xnode());
        }
    }
}

// ---------------------------------------------------------------------
// nil でない xnode が得られた場合、その xnode を返す。
//      「X ::= A | B」という選択型の構文規則のときに使う。
//
macro_rules! return_if_non_nil {
    ( $xnode: expr ) => {
        if ! is_nil_xnode(&$xnode) {
            return Ok($xnode);
        }
    }
}

// ---------------------------------------------------------------------
// nil でである xnode が得られた場合、そのまま nil を返す。
//      「X ::= A*」という繰り返し型 (0個以上) の構文規則のときに使う。
//
macro_rules! return_if_nil {
    ( $xnode: expr ) => {
        if is_nil_xnode(&$xnode) {
            return Ok($xnode);
        }
    }
}

// ---------------------------------------------------------------------
// nil でである xnode が得られた場合、エラーとする。
//
macro_rules! error_if_nil {
    ( $lex: expr, $xnode: expr, $msg: expr ) => {
        if is_nil_xnode(&$xnode) {
            return Err(xpath_syntax_error!($msg,
                $lex.around_tokens().as_str()));
        }
    }
}

// =====================================================================
// [PARSE]
//
pub fn compile_xpath(xpath: &String) -> Result<XNodePtr, Box<Error>> {
    let mut lex = Lexer2::new(xpath)?;

    return parse_main(&mut lex);
}

// ---------------------------------------------------------------------
// [ 1] XPath ::= Expr
//
fn parse_main(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_expr(lex)?;

    must_next_token!(lex, TType::EOF, "{}: 余分な字句が継続。");

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [28] AxisStep ::= (ReverseStep | ForwardStep) PredicateList
// [29] ForwardStep ::= (ForwardAxis NodeTest) | AbbrevForwardStep
// [30] ForwardAxis ::= ("child" "::")
//                    | ("descendant" "::")
//                    | ("attribute" "::")
//                    | ("self" "::")
//                    | ("descendant-or-self" "::")
//                    | ("following-sibling" "::")
//                    | ("following" "::")
//                    | ("namespace" "::")
// [31] AbbrevForwardStep ::= "@"? NodeTest
// [32] ReverseStep ::= (ReverseAxis NodeTest) | AbbrevReverseStep
// [33] ReverseAxis ::= ("parent" "::")
//                    | ("ancestor" "::")
//                    | ("preceding-sibling" "::")
//                    | ("preceding" "::")
//                    | ("ancestor-or-self" "::")
// [34] AbbrevReverseStep ::= ".."
//
//    AxisXXXX  --- (predicates)...
//   (NameTest)
//        |
//    KindTest
//
// AxisXXXXにNameTestがある場合: XNode.nameに、照合する名前を設定する。
//
// AxisXXXXにKindTestがある場合: leftにXNode (n_type = KindTest) をつなげる。
//
//
fn parse_axis_step(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let axis_tbl: HashMap<&str, XNodeType> = [
        ( "ancestor",           XNodeType::AxisAncestor ),
        ( "ancestor-or-self",   XNodeType::AxisAncestorOrSelf ),
        ( "attribute",          XNodeType::AxisAttribute ),
        ( "child",              XNodeType::AxisChild ),
        ( "descendant",         XNodeType::AxisDescendant ),
        ( "descendant-or-self", XNodeType::AxisDescendantOrSelf ),
        ( "following",          XNodeType::AxisFollowing ),
        ( "following-sibling",  XNodeType::AxisFollowingSibling ),
        ( "namespace",          XNodeType::AxisNamespace ),
        ( "parent",             XNodeType::AxisParent ),
        ( "preceding",          XNodeType::AxisPreceding ),
        ( "preceding-sibling",  XNodeType::AxisPrecedingSibling ),
        ( "self",               XNodeType::AxisSelf ),
    ].iter().cloned().collect();

    let tok = lex.next_token();
    match tok.get_type() {
        TType::AxisName => {
            lex.get_token();

            must_next_token!(lex, TType::ColonColon, "{}: 軸名の次に :: が必要。");
                    // 字句解析器が正しければ、ColonColonしか現れないはず。
            lex.get_token();

            let axis = match axis_tbl.get(tok.get_name()) {
                Some(a) => a,
                None => {
                    return Err(xpath_syntax_error!(
                        "{}: 軸名として不正。",
                        lex.around_tokens().as_str()));
                },
            };
            if *axis == XNodeType::AxisNamespace {
                return Err(uninplemented!(
                    "{}: namespace 軸は未実装。",
                    lex.around_tokens().as_str()));
            }
            return parse_axis_step_sub(lex, axis);
        },
        TType::At => {  // 「@」は「attribute::」の省略形
            lex.get_token();
            return parse_axis_step_sub(lex, &XNodeType::AxisAttribute);
        },
        TType::DotDot => {// 「..」は「parent::node()」の省略形
            lex.get_token();
            return Ok(new_xnode(XNodeType::AxisParent, "node()"));
        },
        _ => {  // 「空」は「child::」の省略形
            return parse_axis_step_sub(lex, &XNodeType::AxisChild);
        },
    }
}

// ---------------------------------------------------------------------
// [35] NodeTest ::= KindTest | NameTest
// [36] NameTest ::= QName | Wildcard
// [37] Wildcard ::= "*"
//                 | (NCName ":" "*")
//                 | ("*" ":" NCName)
//
fn parse_axis_step_sub(lex: &mut Lexer2, axis_type: &XNodeType) -> Result<XNodePtr, Box<Error>> {
    let name = parse_qname_or_wildcard(lex)?;
    if name != "" {                             // NameTestがあった場合
        let axis_xnode = new_xnode(axis_type.clone(), name.as_str());
        let predicates_xnode = parse_predicate_list(
                    lex, is_xnode_reverse_axis(&axis_type))?;
        assign_as_right(&axis_xnode, &predicates_xnode);
        return Ok(axis_xnode);
    }

    let kind_test_xnode = parse_kind_test(lex)?;
    if ! is_nil_xnode(&kind_test_xnode) {       // KindTestがあった場合
        let axis_xnode = new_xnode(axis_type.clone(), "");
        assign_as_left(&axis_xnode, &kind_test_xnode);
        let predicates_xnode = parse_predicate_list(
                    lex, is_xnode_reverse_axis(&axis_type))?;
        assign_as_right(&axis_xnode, &predicates_xnode);
        return Ok(axis_xnode);
    }

    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [54] KindTest ::= DocumentTest
//                 | ElementTest
//                 | AttributeTest
//                 | SchemaElementTest
//                 | SchemaAttributeTest
//                 | PITest
//                 | CommentTest
//                 | TextTest
//                 | AnyKindTest
//
//     KindTest        KindTest      KindTest    etc.
//         |              |             |
//    DocumentTest   ElementTest      PITest
//         |        (element-name)    (arg)
//         |              |
//       .....          .....
//
fn parse_kind_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let mut xnode = parse_document_test(lex)?;
    if is_nil_xnode(&xnode) {
        xnode = parse_element_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_attribute_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_schema_element_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_schema_attribute_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_pi_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_comment_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_text_test(lex)?;
    }
    if is_nil_xnode(&xnode) {
        xnode = parse_any_kind_test(lex)?;
    }

    if ! is_nil_xnode(&xnode) {
        let kind_test_xnode = new_xnode(XNodeType::KindTest, "");
        assign_as_left(&kind_test_xnode, &xnode);
        return Ok(kind_test_xnode);
    } else {
        return Ok(new_nil_xnode());
    }
}

// ---------------------------------------------------------------------
// (当面、構文解析のみ)
// [56] DocumentTest ::= "document-node" "(" (ElementTest | SchemaElementTest)? ")"
//
fn parse_document_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    check_next_token!(lex, TType::DocumentTest);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();

    // s_xnode: (ElementTest | SchemaElementTest)?
    let mut s_xnode = parse_element_test(lex)?;
    if is_nil_xnode(&s_xnode) {
        s_xnode = parse_schema_element_test(lex)?;
    }

    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    let document_test_xnode = new_xnode(XNodeType::DocumentTest, "");
    if ! is_nil_xnode(&s_xnode) {
        assign_as_left(&document_test_xnode, &s_xnode);
    }
    return Ok(document_test_xnode);
}


// ---------------------------------------------------------------------
// [64] ElementTest ::= "element" "(" (ElementNameOrWildcard ("," TypeName "?"?)?)? ")"
// [65] ElementNameOrWildcard ::= ElementName | "*"
// [69] ElementName ::= QName
// [70] TypeName ::= QName
//
//     ElementTest
//    (element-name)
//          |
//   KindTestTypeName
//      (type-name)
//
fn parse_element_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    check_next_token!(lex, TType::ElementTest);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();

    let tok = lex.next_token();
    let mut arg = "";
    let mut arg_type_name = String::new();
    match tok.get_type() {
        TType::Name | TType::Asterisk => {
            lex.get_token();
            arg = tok.get_name();
            let tok2 = lex.next_token();
            match tok2.get_type() {
                TType::Comma => {
                    lex.get_token();
                    must_next_token!(lex, TType::Name, "{}: 型名が必要。");
                    arg_type_name = lex.get_token().get_name().to_string();
                    let tok4 = lex.next_token();
                    if tok4.get_type() == TType::Question {
                        lex.get_token();
                        arg_type_name += &"?";
                    }
                },
                _ => {},
            }
        },
        _ => {},
    }

    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    let element_test_xnode = new_xnode(XNodeType::ElementTest, arg);

    if arg_type_name != "" {
        let type_name_xnode = new_xnode(XNodeType::KindTestTypeName, &arg_type_name);
        assign_as_left(&element_test_xnode, &type_name_xnode);
    }

    return Ok(element_test_xnode);
}

// ---------------------------------------------------------------------
// [60] AttributeTest ::= "attribute" "(" (AttribNameOrWildcard ("," TypeName)?)? ")"
// [61] AttribNameOrWildcard ::= AttributeName | "*"
// [68] AttributeName ::= QName
// [70] TypeName ::= QName
//
fn parse_attribute_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    check_next_token!(lex, TType::AttributeTest);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();

    let tok = lex.next_token();
    let mut arg = "";
    let mut arg_type_name = String::new();
    match tok.get_type() {
        TType::Name | TType::Asterisk => {
            lex.get_token();
            arg = tok.get_name();
            let tok2 = lex.next_token();
            match tok2.get_type() {
                TType::Comma => {
                    lex.get_token();
                    must_next_token!(lex, TType::Name, "{}: 型名が必要。");
                    arg_type_name = lex.get_token().get_name().to_string();
                },
                TType::RightParen => {},
                _ => {
                    return Err(xpath_syntax_error!(
                                "{}: 属性名 (または「*」) の後に指定できるのは型名。",
                                lex.around_tokens().as_str()));
                },
            }
        },
        _ => {},
    }

    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    let attribute_test_xnode = new_xnode(XNodeType::AttributeTest, arg);

    if arg_type_name != "" {
        let type_name_xnode = new_xnode(XNodeType::KindTestTypeName, &arg_type_name);
        assign_as_left(&attribute_test_xnode, &type_name_xnode);
    }

    return Ok(attribute_test_xnode);
}

// ---------------------------------------------------------------------
// (当面、構文解析のみ)
// [66] SchemaElementTest ::= "schema-element" "(" ElementDeclaration ")"
// [67] ElementDeclaration ::= ElementName
// [69] ElementName ::= QName
//
fn parse_schema_element_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_one(lex,
                TType::SchemaElementTest, XNodeType::SchemaElementTest);
}

// ---------------------------------------------------------------------
// (当面、構文解析のみ)
// [62] SchemaAttributeTest ::= "schema-attribute" "(" AttributeDeclaration ")"
// [63] AttributeDeclaration ::= AttributeName
// [68] AttributeName ::= QName
//
fn parse_schema_attribute_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_one(lex,
                TType::SchemaAttributeTest, XNodeType::SchemaAttributeTest);
}

// ---------------------------------------------------------------------
// SchemaElementTest | SchemaAttributeTest に共通:
// テスト名 (ttype) の後に、"(" QName ")" が続いているとき、
// xnode (XNodeType: xnode_type) を生成して返す。
//
fn parse_kind_test_sub_one(lex: &mut Lexer2,
        ttype: TType, xnode_type: XNodeType)
                                    -> Result<XNodePtr, Box<Error>> {

    check_next_token!(lex, ttype);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();
    must_next_token!(lex, TType::Name, "{}: 名前が必要。");
    let tok = lex.get_token();
    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(xnode_type, tok.get_name()));
}

// ---------------------------------------------------------------------
// [59] PITest ::= "processing-instruction" "(" (NCName | StringLiteral)? ")"
//
fn parse_pi_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    check_next_token!(lex, TType::PITest);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();

    let tok = lex.next_token();
    let mut arg = "";
    match tok.get_type() {
        TType::Name | TType::StringLiteral => {
            lex.get_token();
            arg = tok.get_name();
        },
        TType::RightParen => {},
        _ => {
            return Err(xpath_syntax_error!(
                        "{}: 名前または文字列が必要。",
                        lex.around_tokens().as_str()));
        },
    }

    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(XNodeType::PITest, arg));
}

// ---------------------------------------------------------------------
// [58] CommentTest ::= "comment" "(" ")"
//
fn parse_comment_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::CommentTest, XNodeType::CommentTest);
}

// ---------------------------------------------------------------------
// [57] TextTest ::= "text" "(" ")"
//
fn parse_text_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::TextTest, XNodeType::TextTest);
}

// ---------------------------------------------------------------------
// [55] AnyKindTest ::= "node" "(" ")"
//
fn parse_any_kind_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::AnyKindTest, XNodeType::AnyKindTest);
}

// ---------------------------------------------------------------------
// AnyKindTest | TextTest | CommentTest に共通。
// また、SequenceTypeの「empty-sequence()」、ItemType の「item()」にも共通。
// テスト名 (ttype) の後に、引数なしで "(" ")" が続いているとき、
// xnode (XNodeType: xnode_type) を生成して返す。
//
fn parse_kind_test_sub_none(lex: &mut Lexer2,
        ttype: TType, xnode_type: XNodeType)
                                    -> Result<XNodePtr, Box<Error>> {

    check_next_token!(lex, ttype);
    lex.get_token();
    must_next_token!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();
    must_next_token!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(xnode_type, ""));
}

// ---------------------------------------------------------------------
// [39] PredicateList ::= Predicate*
//  ->  PredicateList ::= (empty)
//                      | Predicate PredicateList
//
// Predicate{Rev}Top --- Predicate{Rev}Top ---...
//        |                     |
//        |                   (Expr)
//        |
//      (Expr)
//
fn parse_predicate_list(lex: &mut Lexer2, reverse_order: bool) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_predicate(lex)?;
    return_if_nil!(xnode);

    let next_node = parse_predicate_list(lex, reverse_order)?;

    let xnode_type = if ! reverse_order {
        XNodeType::PredicateTop
    } else {
        XNodeType::PredicateRevTop
    };

    let xnode_pred = new_xnode(xnode_type, "Predicate");
    assign_as_left(&xnode_pred, &xnode);
    assign_as_right(&xnode_pred, &next_node);
    return Ok(xnode_pred);
}

// ---------------------------------------------------------------------
// [40] Predicate ::= "[" Expr "]"
//
fn parse_predicate(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    check_next_token!(lex, TType::LeftBracket);
    lex.get_token();

    let xnode = parse_expr(lex)?;

    must_next_token!(lex, TType::RightBracket, "{}: 述語を閉じる「]」が必要。");
    lex.get_token();

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 2] Expr ::= ExprSingle ( "," ExprSingle )*
//
//   OperatorConcatenate --- OperatorConcatenate --- nil
//         |                       |
//         |                    IfExpr ...
//         |              ...
//     OperatorOr --- ...
//        ...
// 3.3.1 Constructing Sequences
// Comma operator: evaluates each of its operands and concatenates
// the resulting sequences, in order, into a single result sequence.
//
fn parse_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Comma, XNodeType::OperatorConcatenate ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_expr_single, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 3] ExprSingle ::= ForExpr
//                   | QuantifiedExpr
//                   | IfExpr
//                   | OrExpr
//
fn parse_expr_single(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_for_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_quantified_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_if_expr(lex)?;
    return_if_non_nil!(xnode);

    return parse_or_expr(lex);
}

// ---------------------------------------------------------------------
// [ 4] ForExpr ::= SimpleForClause "return" ExprSingle
// [ 5] SimpleForClause ::= "for" "$" VarName "in" ExprSingle
//                              ("," "$" VarName "in" ExprSingle)*
// [45] VarName ::= QName
// 
fn parse_for_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    check_next_token!(lex, TType::For);
    lex.get_token();

    let xnode_for_expr = new_xnode(XNodeType::ForExpr, &"for");
    let mut curr_xnode = Rc::clone(&xnode_for_expr);
    loop {
        let xnode_var_bind = parse_var_bind(lex, &XNodeType::ForVarBind)?;
        if is_nil_xnode(&xnode_var_bind) {
            break;
        }
        assign_as_right(&curr_xnode, &xnode_var_bind);
        curr_xnode = get_right(&curr_xnode);

        let tok = lex.next_token();
        if tok.get_type() != TType::Comma {
            break;
        }
        lex.get_token();
    }

    must_next_name!(lex, "return", "{}: for に対応する return が必要。");
    lex.get_token();

    let xnode_expr_single = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &xnode_expr_single);
    
    return Ok(xnode_for_expr);
}

// ---------------------------------------------------------------------
// [ 6] QuantifiedExpr ::= ("some" | "every")
//                  "$" VarName "in" ExprSingle
//                      ("," "$" VarName "in" ExprSingle)*
//                  "satisfies" ExprSingle
// [45] VarName ::= QName
//
fn parse_quantified_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode_quantified_expr;
    let xnode_type_bind;
    let tok = lex.next_token();
    match tok.get_type() {
        TType::Some => {
            lex.get_token();
            xnode_quantified_expr = new_xnode(XNodeType::SomeExpr, &"some");
            xnode_type_bind = XNodeType::SomeVarBind;
        },
        TType::Every => {
            lex.get_token();
            xnode_quantified_expr = new_xnode(XNodeType::EveryExpr, &"every");
            xnode_type_bind = XNodeType::EveryVarBind;
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }

    let mut curr_xnode = Rc::clone(&xnode_quantified_expr);
    loop {
        let xnode_var_bind = parse_var_bind(lex, &xnode_type_bind)?;
        if is_nil_xnode(&xnode_var_bind) {
            break;
        }
        assign_as_right(&curr_xnode, &xnode_var_bind);
        curr_xnode = get_right(&curr_xnode);

        let tok = lex.next_token();
        if tok.get_type() != TType::Comma {
            break;
        }
        lex.get_token();
    }

    must_next_name!(lex, "satisfies", "{}: some/every に対応する satisfies が必要。");
    lex.get_token();

    let xnode_expr_single = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &xnode_expr_single);
    
    return Ok(xnode_quantified_expr);
}

// ---------------------------------------------------------------------
// "$" VarName "in" ExprSingle
// [45] VarName ::= QName
//
fn parse_var_bind(lex: &mut Lexer2, xnode_type: &XNodeType) -> Result<XNodePtr, Box<Error>> {

    check_next_token!(lex, TType::Dollar);
    lex.get_token();

    let var_name = parse_qname(lex)?;
    if var_name == "" {
        return Err(xpath_syntax_error!(
                    "{}: $ の後には変数名が必要。",
                    lex.around_tokens().as_str()));
    }

    must_next_name!(lex, "in", "{}: 変数名の後に in が必要。");
    lex.get_token();

    let xnode_expr_single = parse_expr_single(lex)?;
    let xnode_for_expr = new_xnode(xnode_type.clone(), &var_name);
    assign_as_left(&xnode_for_expr, &xnode_expr_single);

    return Ok(xnode_for_expr);
}

// ---------------------------------------------------------------------
// [ 7] IfExpr ::= "if" "(" Expr ")" "then" ExprSingle "else" ExprSingle
//
//      IfExpr --- IfThenElse --- (xnode_else)
//         |            |
//         |        (xnode_then)
//         |
//    (xnode_cond)
//
fn parse_if_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    check_next_token!(lex, TType::If);
    lex.get_token();

    must_next_token!(lex, TType::LeftParen, "{}: if 文には左括弧が必要。");
    lex.get_token();

    let xnode_cond = parse_expr(lex)?;
    error_if_nil!(lex, xnode_cond, "{}: if文の条件式が不正。");

    must_next_token!(lex, TType::RightParen, "{}: 条件式を閉じる右括弧が必要。");
    lex.get_token();

    must_next_name!(lex, "then", "{}: if に対応する then が必要。");
    lex.get_token();

    let xnode_then = parse_expr_single(lex)?;
    if is_nil_xnode(&xnode_then) {
        return Err(xpath_syntax_error!(
                "{}: if文のthen節が不正。", lex.around_tokens().as_str()));
    }

    must_next_name!(lex, "else", "{}: if に対応する else が必要。");
    lex.get_token();

    let xnode_else = parse_expr_single(lex)?;
    error_if_nil!(lex, xnode_else, "{}: if文のelse節が不正。");

    let xnode_if_expr = new_xnode(XNodeType::IfExpr, "if_expr");
    let xnode_if_then_else = new_xnode(XNodeType::IfThenElse, "if_then_else");

    assign_as_left(&xnode_if_expr, &xnode_cond);
    assign_as_right(&xnode_if_expr, &xnode_if_then_else);
    assign_as_left(&xnode_if_then_else, &xnode_then);
    assign_as_right(&xnode_if_then_else, &xnode_else);

    return Ok(xnode_if_expr);
}

// ---------------------------------------------------------------------
// [ 8] OrExpr ::= AndExpr ( "or" AndExpr )*
//
fn parse_or_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Or, XNodeType::OperatorOr ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_and_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 9] AndExpr ::= ComparisonExpr ( "and" ComparisonExpr )*
//
fn parse_and_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::And, XNodeType::OperatorAnd ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_comparison_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [10] ComparisonExpr ::= RangeExpr ( (ValueComp
//                           | GeneralComp
//                           | NodeComp) RangeExpr )?
// [22] GenerapComp ::= "=" | "!=" | "<" | "<=" | ">" | ">="
// [23] ValueComp ::= "eq" | "ne" | "lt" | "le" | "gt" | "ge"
// [24] NodeComp ::= "is" | "<<" | ">>"
//
fn parse_comparison_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::ValueEQ,    XNodeType::OperatorValueEQ ),
        ( TType::ValueNE,    XNodeType::OperatorValueNE ),
        ( TType::ValueLT,    XNodeType::OperatorValueLT ),
        ( TType::ValueGT,    XNodeType::OperatorValueGT ),
        ( TType::ValueLE,    XNodeType::OperatorValueLE ),
        ( TType::ValueGE,    XNodeType::OperatorValueGE ),
        ( TType::GeneralEQ,  XNodeType::OperatorGeneralEQ ),
        ( TType::GeneralNE,  XNodeType::OperatorGeneralNE ),
        ( TType::GeneralLT,  XNodeType::OperatorGeneralLT ),
        ( TType::GeneralGT,  XNodeType::OperatorGeneralGT ),
        ( TType::GeneralLE,  XNodeType::OperatorGeneralLE ),
        ( TType::GeneralGE,  XNodeType::OperatorGeneralGE ),
        ( TType::IsSameNode, XNodeType::OperatorIsSameNode ),
        ( TType::NodeBefore, XNodeType::OperatorNodeBefore ),
        ( TType::NodeAfter,  XNodeType::OperatorNodeAfter ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_range_expr, &token_node_map, true);
}

// ---------------------------------------------------------------------
// [11] RangeExpr ::= AdditiveExpr ( "to" AdditiveExpr )?
//
fn parse_range_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::To, XNodeType::OperatorTo ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_additive_expr, &token_node_map, true);
}

// ---------------------------------------------------------------------
// [12] AdditiveExpr ::= MultiplicativeExpr
//                         ( ( "+" | "-" ) MultiplicativeExpr )*
//
fn parse_additive_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Plus, XNodeType::OperatorAdd ),
        ( TType::Minus, XNodeType::OperatorSubtract ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_multiplicative_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [13] MultiplicativeExpr ::= UnionExpr
//                         ( ( "*" | "div" | "idiv" | "mod" ) UnionExpr )*
//
fn parse_multiplicative_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Asterisk, XNodeType::OperatorMultiply ),
        ( TType::Div, XNodeType::OperatorDiv ),
        ( TType::IDiv, XNodeType::OperatorIDiv ),
        ( TType::Mod, XNodeType::OperatorMod ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_union_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [14] UnionExpr ::= IntersectExceptExpr
//                         ( ( "union" | "|" ) IntersectExceptExpr )*
//
fn parse_union_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Union, XNodeType::OperatorUnion ),
    ].iter().cloned().collect();

    let xnode = parse_bin_op_sub(lex, parse_intersect_except_expr, &token_node_map, false)?;

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [15] IntersectExceptExpr ::= InstanceofExpr
//                         ( ( "intersect" | "except" ) InstanceofExpr )*
//
fn parse_intersect_except_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Intersect, XNodeType::OperatorIntersect ),
        ( TType::Except, XNodeType::OperatorExcept ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_instanceof_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [16] InstanceofExpr ::= TreatExpr ( ( "instance" "of" ) SequenceType )?
//
fn parse_instanceof_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_treat_expr(lex)?;
    let tok = lex.next_token();
    if tok.get_type() == TType::InstanceOf {
        lex.get_token();
        let seq_type_xnode = parse_sequence_type(lex)?;
        if is_nil_xnode(&seq_type_xnode) {
            return Err(xpath_syntax_error!(
                "{}: 「instance of」の後にはSequenceTypeが必要。",
                lex.around_tokens().as_str()));
        }
        let instance_of_xnode = new_xnode(XNodeType::OperatorInstanceOf, "");
        assign_as_left(&instance_of_xnode, &xnode);
        assign_as_right(&instance_of_xnode, &seq_type_xnode);
        return Ok(instance_of_xnode);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [17] TreatExpr ::= CastableExpr ( ( "treat" "as" ) SequenceType )?
//
fn parse_treat_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_castable_expr(lex)?;
    let tok = lex.next_token();
    if tok.get_type() == TType::TreatAs {
        lex.get_token();
        let seq_type_xnode = parse_sequence_type(lex)?;
        if is_nil_xnode(&seq_type_xnode) {
            return Err(xpath_syntax_error!(
                "{}: 「treat of」の後にはSequenceTypeが必要。",
                lex.around_tokens().as_str()));
        }
        let treat_as_xnode = new_xnode(XNodeType::OperatorTreatAs, "");
        assign_as_left(&treat_as_xnode, &xnode);
        assign_as_right(&treat_as_xnode, &seq_type_xnode);
        return Ok(treat_as_xnode);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [50] SequenceType ::= ("empty-sequence" "(" ")")
//                     | (ItemType OccurrenceIndicator?)
// [51] OccurrenceIndicator ::= "?" | "*" | "+"
//
//   SequenceType            SequenceType         SequenceType
//        |                 (? | * | + | _)      (? | * | + | _)
//        |                       |                    |
// EmptySequenceTest          KindTest             AtomicType
//                              .....                .....
//
fn parse_sequence_type(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_kind_test_sub_none(lex,
                TType::EmptySequence, XNodeType::EmptySequenceTest)?;
    if ! is_nil_xnode(&xnode) {
        let seq_type_xnode = new_xnode(XNodeType::SequenceType, "");
        assign_as_left(&seq_type_xnode, &xnode);
        return Ok(seq_type_xnode);
    }

    let xnode = parse_item_type(lex)?;
    if ! is_nil_xnode(&xnode) {
        let tok = lex.next_token();
        let mut occurence_indicator = "";
        match tok.get_type() {
            TType::Question | TType::Asterisk | TType::Plus => {
                lex.get_token();
                occurence_indicator = tok.get_name();
            },
            _ => {},
        }
        let seq_type_xnode = new_xnode(
                        XNodeType::SequenceType, occurence_indicator);
        assign_as_left(&seq_type_xnode, &xnode);
        return Ok(seq_type_xnode);
    }

    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [52] ItemType ::= KindTest 
//                 | ( "item" "(" ")" )
//                 | AtomicType
// [53] AtomicType ::= QName
//
//   KindTest             KindTest         AtomicType
//      |                    |               (type)
//  DocumentTestなど      ItemTest
//    .....           (これもKindTest扱い)
//
fn parse_item_type(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_kind_test(lex)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

    let xnode = parse_kind_test_sub_none(lex, TType::Item, XNodeType::ItemTest)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

    let qname = parse_qname(lex)?;
    if qname != "" {
        let xnode = new_xnode(XNodeType::AtomicType, &qname);
        return Ok(xnode);
    }

    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [18] CastableExpr ::= CastExpr ( "castable" "as" ) SingleType )?
//
// OperatorCastableAs --- SingleType
//       |                   |
//   (CastExpr)          AtomicType
//                         (type)
//
fn parse_castable_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_cast_expr(lex)?;
    let tok = lex.next_token();
    if tok.get_type() == TType::CastableAs {
        lex.get_token();
        let single_type_xnode = parse_single_type(lex)?;
        if is_nil_xnode(&single_type_xnode) {
            return Err(xpath_syntax_error!(
                    "{}: キャストする型の名前が必要。",
                    lex.around_tokens().as_str()));
        }
        let castable_xnode = new_xnode(XNodeType::OperatorCastableAs, "");
        assign_as_left(&castable_xnode, &xnode);
        assign_as_right(&castable_xnode, &single_type_xnode);
        return Ok(castable_xnode);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [19] CastExpr ::= UnaryExpr ( ( "cast" "as" ) SingleType )?
//
// OperatorCastAs --- SingleType
//       |               |
//   (UnaryExpr)     AtomicType
//                     (type)
//
fn parse_cast_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_unary_expr(lex)?;
    let tok = lex.next_token();
    if tok.get_type() == TType::CastAs {
        lex.get_token();
        let single_type_xnode = parse_single_type(lex)?;
        if is_nil_xnode(&single_type_xnode) {
            return Err(xpath_syntax_error!(
                    "{}: キャストする型の名前が必要。",
                    lex.around_tokens().as_str()));
        }
        let cast_xnode = new_xnode(XNodeType::OperatorCastAs, "");
        assign_as_left(&cast_xnode, &xnode);
        assign_as_right(&cast_xnode, &single_type_xnode);
        return Ok(cast_xnode);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [49] SingleType ::= AtomicType "?"?
// [53] AtomicType ::= QName
//
fn parse_single_type(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let mut qname = parse_qname(lex)?;
    if qname != "" {
        let tok = lex.next_token();
        if tok.get_type() == TType::Question {
            lex.get_token();
            qname += tok.get_name();
        }
        let single_type_xnode = new_xnode(XNodeType::SingleType, "");
        let atomic_type_xnode = new_xnode(XNodeType::AtomicType, &qname);
        assign_as_left(&single_type_xnode, &atomic_type_xnode);
        return Ok(single_type_xnode);
    }
    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [20] UnaryExpr ::= ( "-" | "+" )? ValueExpr
//
fn parse_unary_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    match tok.get_type() {
        TType::Minus => {
            lex.get_token();
            let next_node = parse_value_expr(lex)?;
            let xnode_op = new_xnode(XNodeType::OperatorUnaryMinus, "-");
            assign_as_right(&xnode_op, &next_node);
            return Ok(xnode_op);
        },
        TType::Plus => {
            lex.get_token();
            let next_node = parse_value_expr(lex)?;
            let xnode_op = new_xnode(XNodeType::OperatorUnaryPlus, "+");
            assign_as_right(&xnode_op, &next_node);
            return Ok(xnode_op);
        },
        _ => {
            return parse_value_expr(lex);
        }
    }
}

// ---------------------------------------------------------------------
// [21] ValueExpr ::= PathExpr
//
fn parse_value_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_path_expr(lex);
}

// ---------------------------------------------------------------------
// 二項演算子を解析
//    expr ::= subexpr (op subexpr)+ と考え、左結合になるように実装する。
//    op_once: trueならば「subexpr (op subexpr)?」として扱う (nonassoc)。
//
fn parse_bin_op_sub(lex: &mut Lexer2,
        sub_parser: fn(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>>,
        token_node_map: &HashMap<TType, XNodeType>,
        op_once: bool) -> Result<XNodePtr, Box<Error>> {

    let mut xnode = sub_parser(lex)?;
    loop {
        let tok = lex.next_token();
        let n_type = match token_node_map.get(&tok.get_type()) {
            Some(t) => t,
            None => break,
        };
        lex.get_token();
        let next_node = sub_parser(lex)?;

        let xnode_op = new_xnode(n_type.clone(), tok.get_name());
        assign_as_left(&xnode_op, &xnode);
        assign_as_right(&xnode_op, &next_node);
        xnode = xnode_op;
        if op_once {        // 1回だけでループから脱出する
            break;
        }
    }
    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [25] PathExpr ::= ("/" RelativePathExpr?)
//                 | ("//" RelativePathExpr)
//                 | RelativePathExpr
//
//  OpPath  --- ((RelativePathExpr))
//    |
// AxisRoot
//
//  OpPath --- OpPath --- ((RelativePathExpr))
//    |           |
//    |        AxisDescendantOrSelf
//    |
//  AxisRoot
//
fn parse_path_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let tok = lex.next_token();
    match tok.get_type() {
        TType::Slash => {
            lex.get_token();

            let op_path_xnode = new_xnode(XNodeType::OperatorPath, "op_path");
            let root_xnode = new_xnode(XNodeType::AxisRoot, "node()");
            assign_as_left(&op_path_xnode, &root_xnode);

            let rel_xnode = parse_relative_path_expr(lex)?;
            if ! is_nil_xnode(&rel_xnode) {
                assign_as_right(&op_path_xnode, &rel_xnode);
            }
            return Ok(op_path_xnode);
        },

        TType::SlashSlash => {
            lex.get_token();

            let op_path_xnode_u = new_xnode(XNodeType::OperatorPath, "op_path");
            let root_xnode = new_xnode(XNodeType::AxisRoot, "/");
            assign_as_left(&op_path_xnode_u, &root_xnode);

            let op_path_xnode_l = new_xnode(XNodeType::OperatorPath, "op_path");
            let ds_xnode = new_xnode(XNodeType::AxisDescendantOrSelf, "node()");
            assign_as_right(&op_path_xnode_u, &op_path_xnode_l);
            assign_as_left(&op_path_xnode_l, &ds_xnode);

            let rel_xnode = parse_relative_path_expr(lex)?;
            if ! is_nil_xnode(&rel_xnode) {
                assign_as_right(&op_path_xnode_l, &rel_xnode);
            }
            return Ok(op_path_xnode_u);
        },
        _ => {
            return parse_relative_path_expr(lex);
        },
    }
}

// ---------------------------------------------------------------------
// [26] RelativePathExpr ::= StepExpr (("/" | "//") StepExpr)*
//
//  OpPath --- OpPath --- OpPath --- OpPath --- x
//    |          |          |          |
//    |          |          |       AxisXXX --- (predicate)
//    |          |          |
//    |          |       AxisXXX --- (predicate)
//    |          |
//    |     AxisDescendantOrSelf
//    |
// AxisXXX --- (predicate)
//
fn parse_relative_path_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let step_expr_xnode = parse_step_expr(lex)?;
    if is_nil_xnode(&step_expr_xnode) {
        return Ok(new_nil_xnode());
    }
    let top_op_path_xnode = new_xnode(XNodeType::OperatorPath, "op_path aa");
    assign_as_left(&top_op_path_xnode, &step_expr_xnode);
    let mut curr_xnode = Rc::clone(&top_op_path_xnode);

    loop {
        let tok = lex.next_token();
        match tok.get_type() {
            TType::Slash => {
                lex.get_token();
                let step_expr_xnode = parse_step_expr(lex)?;
                let op_path_xnode = new_xnode(XNodeType::OperatorPath, "op_path");
                assign_as_left(&op_path_xnode, &step_expr_xnode);
                assign_as_right(&curr_xnode, &op_path_xnode);
                curr_xnode = Rc::clone(&op_path_xnode);
            },
            TType::SlashSlash => {
                lex.get_token();
                let step_expr_xnode = parse_step_expr(lex)?;

                let op_path_xnode_u = new_xnode(XNodeType::OperatorPath, "op_path");
                let ds_xnode = new_xnode(XNodeType::AxisDescendantOrSelf, "node()");
                assign_as_left(&op_path_xnode_u, &ds_xnode);

                let op_path_xnode_l = new_xnode(XNodeType::OperatorPath, "op_path");
                assign_as_left(&op_path_xnode_l, &step_expr_xnode);

                assign_as_right(&op_path_xnode_u, &op_path_xnode_l);
                assign_as_right(&curr_xnode, &op_path_xnode_u);
                curr_xnode = Rc::clone(&op_path_xnode_l);
            },
            _ => {
                break;
            },
        }
    }
    return Ok(top_op_path_xnode);
}

// ---------------------------------------------------------------------
// [27] StepExpr ::= FilterExpr | AxisStep
//
fn parse_step_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_filter_expr(lex)?;
    return_if_non_nil!(xnode);

    return parse_axis_step(lex);
}

// ---------------------------------------------------------------------
// [38] FilterExpr ::= PrimaryExpr PredicateList
//
// AxisXXXX --- (predicate_list)...
//
//
fn parse_filter_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let mut xnode = parse_primary_expr(lex)?;
    return_if_nil!(xnode);

    // -----------------------------------------------------------------
    // (PrimaryExprがAxis*である場合)
    //
    // AxisXXXX --- (predicate_list)...
    //
    if is_xnode_axis(&get_xnode_type(&xnode)) {
        let xnode_preds = parse_predicate_list(lex, false)?;
        assign_as_right(&xnode, &xnode_preds);
    }

    // -----------------------------------------------------------------
    // (PrimaryExprがAxis*以外である場合)
    //
    //   [ApplyPredicates] -- (predicate_list)...
    //           |
    //     (PrimaryExpr) --- (右辺値)...
    //           |
    //       (左辺値)...
    //
    if ! is_xnode_axis(&get_xnode_type(&xnode)) {
        let xnode_preds = parse_predicate_list(lex, false)?;
        if ! is_nil_xnode(&xnode_preds) {
            let nop_node = new_xnode(XNodeType::ApplyPredicates, "");
            assign_as_left(&nop_node, &xnode);
            assign_as_right(&nop_node, &xnode_preds);
            xnode = nop_node;
        }
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [41] PrimaryExpr ::= Literal
//                    | VarRef
//                    | ParenthesizedExpr
//                    | ContextItemExpr
//                    | FunctionCall
// [42] Literal ::= NumericLiteral (43) | StringLiteral (74)
// [44] VarRef ::= "$" VarName
// [45] VarName ::= QName
// [46] ParenthesizedExpr ::= "(" Expr? ")"
// [47] ContextItemExpr ::= "."
//
fn parse_primary_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    match tok.get_type() {
        TType::Dollar => {              // [44] VarRef
            lex.get_token();

            let qname = parse_qname(lex)?;
            if qname != "" {
                return Ok(new_xnode(XNodeType::VariableReference, qname.as_str()));
            } else {
                return Err(xpath_syntax_error!(
                        "{}: 変数参照の $ に続いて名前が必要。",
                        lex.around_tokens().as_str()));
            }
        },
        TType::LeftParen => {           // [46] ParenthesizedExpr
            lex.get_token();
            let xnode = parse_expr(lex)?;

            must_next_token!(lex, TType::RightParen,
                    "{}: 左括弧に対応する右括弧が必要。");
            lex.get_token();

            if ! is_nil_xnode(&xnode) {
                return Ok(xnode);
            } else {
                return Ok(new_xnode(XNodeType::OperatorPath, "(Empty parenthesized expr)"));
                        // 空の括弧式があることを示す。
            }
        },
        TType::StringLiteral => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::StringLiteral, tok.get_name()));
        },
        TType::IntegerLiteral => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::IntegerLiteral, tok.get_name()));
        },
        TType::DecimalLiteral => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::DecimalLiteral, tok.get_name()));
        },
        TType::DoubleLiteral => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::DoubleLiteral, tok.get_name()));
        },
        TType::Dot => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::ContextItem, "."));
                // XPath 1.0ではAxisSelfの意味であった。
                // 「(1 to 100) [. mod 5 eq 0]」のような文脈では原子値を表す。
                //
        },
        _ => {
        },
    }

    let fcall_xnode = parse_function_call(lex)?;
    if ! is_nil_xnode(&fcall_xnode) {
        return Ok(fcall_xnode);
    }

    return Ok(new_nil_xnode());

}

// ---------------------------------------------------------------------
// [48] FunctionCall ::= QName "(" (ExprSingle ("," ExprSingle)*)? ")"
//
// FuncCall -- ArgTop -- ArgTop --...
//               |         |
//               |      OpLiteral
//               |
//              OpEQ  -- (rhs)
//               |
//              (lhs)
//
// 引数並びの順に、XNodeArgumentTopを右に連結。
// XNodeArgumentTopの左に、引数を表すExprを連結。
//
fn parse_function_call(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    // -------------------------------------------------------------
    // 左括弧まで先読みして、函数名か否か判定する。
    //
    let func_name = parse_qname_left_paren(lex)?;
    if func_name == "" {
        return Ok(new_nil_xnode());
    }

    let arg_node = parse_argument_array(lex)?;

    must_next_token!(lex, TType::RightParen, "{}: 函数の引数並びを閉じる右括弧が必要。");
    lex.get_token();

    // -------------------------------------------------------------
    // 引数の数を調べる。
    //
    let mut num_args: usize = 0;
    let mut curr = Rc::clone(&arg_node);
    while ! is_nil_xnode(&curr) {
        num_args += 1;
        curr = get_right(&curr);
    }

    // -------------------------------------------------------------
    // この時点で函数表と照合して、函数の存在や引数の数を検査する。
    //
    if func::check_function_spec(&func_name, num_args) == false {
        return Err(xpath_syntax_error!(
            "{}: 函数が未実装、または引数の数 ({}) が不適切。", func_name, num_args));
    }

    // -------------------------------------------------------------
    //
    let func_node = new_xnode(XNodeType::FunctionCall, &func_name);
    assign_as_right(&func_node, &arg_node);

    return Ok(func_node);
}

// ---------------------------------------------------------------------
// [48] FunctionCall ::= QName "(" (ExprSingle ("," ExprSingle)*)? ")"
//
fn parse_argument_array(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_argument(lex)?;

    let mut curr = Rc::clone(&xnode);
    while lex.next_token().get_type() == TType::Comma {
        lex.get_token();
        let next_arg_xnode = parse_argument(lex)?;
        assign_as_right(&curr, &next_arg_xnode);
        curr = Rc::clone(&next_arg_xnode);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// Argument ::= Expr
//
fn parse_argument(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_expr_single(lex)?;
    return_if_nil!(xnode);

    let xnode_top = new_xnode(XNodeType::ArgumentTop, "");
    assign_as_left(&xnode_top, &xnode);
    return Ok(xnode_top);
}

// =====================================================================
// 構文解析器の補助
//

// ---------------------------------------------------------------------
// (Lexerの) 現在位置以降に、QNameまたはWildcardと解析できる字句があれば、
// その文字列を返す。
// 該当する字句がなければ空文字列を返す。
//
// Wildcard ::= "*"
//            | (NCName ":" "*")
//            | ("*" ":" NCName)
//
fn parse_qname_or_wildcard(lex: &mut Lexer2) -> Result<String, Box<Error>> {
    let mut qname = String::new();

    match lex.next_token().get_type() {
        TType::Name => {
            qname += lex.get_token().get_name();

            if lex.next_token().get_type() != TType::Colon {
                return Ok(qname);
            }
            lex.get_token();
            qname += &":";

            match lex.next_token().get_type() {
                TType::Name | TType::Asterisk => {
                    qname += lex.get_token().get_name();
                    return Ok(qname);
                },
                _ => {
                    return Err(xpath_syntax_error!(
                                "{}: 「:」の後には名前または * が必要。",
                                lex.around_tokens().as_str()));
                },
            }
        },

        TType::Asterisk => {
            lex.get_token();
            qname += &"*";

            if lex.next_token().get_type() != TType::Colon {
                return Ok(qname);
            }
            lex.get_token();
            qname += &":";

            if lex.next_token().get_type() != TType::Name {
                return Err(xpath_syntax_error!("{}: 「:」の後には名前が必要。",
                                    lex.around_tokens().as_str()));
            }
            qname += lex.get_token().get_name();
            return Ok(qname);
        },

        _ => {
            return Ok(qname);
        },
    }
}

// ---------------------------------------------------------------------
// (Lexerの) 現在位置以降に、QNameと解析できる字句があり、
// 続いてLeftParenが現れれば、LeftParenの位置まで進め、その文字列を返す。
// 該当する字句がなければ、当初の位置に戻した上で、空文字列を返す。
//
fn parse_qname_left_paren(lex: &mut Lexer2) -> Result<String, Box<Error>> {

    let tok = lex.next_token();
    if tok.get_type() != TType::Name {
        return Ok(String::new());           // 非該当
    }
    let mut qname = tok.get_name().to_string();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() == TType::LeftParen {
        lex.get_token();
        return Ok(qname);                   // localpart (
    }
    if tok.get_type() != TType::Colon {
        lex.unget_token();
        return Ok(String::new());           // 非該当
    }
    lex.get_token();
    qname += tok.get_name();                // prefix:

    must_next_token!(lex, TType::Name, "{}: QName: コロンの後には名前が必要。");
    let tok = lex.get_token();
    qname += tok.get_name();                // prefix:localpart

    let tok = lex.next_token();
    if tok.get_type() == TType::LeftParen {
        lex.get_token();
        return Ok(qname);                   // prefix:localpart (
    } else {
        lex.unget_token();
        lex.unget_token();
        lex.unget_token();
        return Ok(String::new());           // 非該当
    }
}

// ---------------------------------------------------------------------
// (Lexerの) 現在位置以降にQNameと解析できる字句があれば、その文字列を返す。
// 該当する字句がなければ、当初の位置に戻した上で、空文字列を返す。
//
// QName ::= PrefixedName | UnprefixedName
// PrefixedName ::= Prefix ':' LocalPart
// UnprefixedName ::= LocalPart
// Prefix ::= NCName
// LocalPart ::= NCName
//
fn parse_qname(lex: &mut Lexer2) -> Result<String, Box<Error>> {
    let mut qname = String::new();

    let tok = lex.next_token();
    if tok.get_type() != TType::Name {
        return Ok(qname);               // ""
    }
    qname += tok.get_name();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::Colon {
        return Ok(qname);
    }
    qname += tok.get_name();
    lex.get_token();

    must_next_token!(lex, TType::Name, "{}: QName: コロンの後には名前が必要。");
    let tok = lex.get_token();
    qname += tok.get_name();

    return Ok(qname);
}

// =====================================================================
// xnode関係のヘルパー函数
//

// ---------------------------------------------------------------------
//
fn new_xnode(n_type: XNodeType, name: &str) -> XNodePtr {
    return Rc::new(RefCell::new(XNode{
        n_type: n_type,
        name: String::from(name),
        left: None,
        right: None,
    }));
}

// ---------------------------------------------------------------------
//
fn new_nil_xnode() -> XNodePtr {
    return new_xnode(XNodeType::Nil, "");
}

// ---------------------------------------------------------------------
//
//fn assign_xnode_type(xnode: &XNodePtr, n_type: &XNodeType) {
//    xnode.borrow_mut().n_type = n_type.clone();
//}

// ---------------------------------------------------------------------
//
fn assign_as_left(parent: &XNodePtr, left: &XNodePtr) {
    if ! is_nil_xnode(left) {
        parent.borrow_mut().left = Some(Rc::clone(left));
    }
}

// ---------------------------------------------------------------------
//
fn assign_as_right(parent: &XNodePtr, right: &XNodePtr) {
    if ! is_nil_xnode(right) {
        parent.borrow_mut().right = Some(Rc::clone(right));
    }
}

// =====================================================================
// xnode関係のヘルパー函数
//

// ---------------------------------------------------------------------
//
pub fn get_xnode_name(xnode: &XNodePtr) -> String {
    return xnode.borrow().name.clone();
}

// ---------------------------------------------------------------------
//
pub fn get_xnode_type(xnode: &XNodePtr) -> XNodeType {
    return xnode.borrow().n_type.clone();
}

// ---------------------------------------------------------------------
//
pub fn is_nil_xnode(node: &XNodePtr) -> bool {
    return node.borrow().n_type == XNodeType::Nil;
}

// ---------------------------------------------------------------------
//
pub fn get_left(parent: &XNodePtr) -> XNodePtr {
    match parent.borrow().left {
        Some(ref left) => return Rc::clone(&left),
        None => return new_nil_xnode(),
    }
}

// ---------------------------------------------------------------------
//
pub fn get_right(parent: &XNodePtr) -> XNodePtr {
    match parent.borrow().right {
        Some(ref right) => return Rc::clone(&right),
        None => return new_nil_xnode(),
    }
}

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath_impl::lexer::*;
    use xpath_impl::eval::xnode_dump;
    use xpath_impl::parser::compile_xpath;

    // -----------------------------------------------------------------
    //
    #[test]
    fn test_parse() {

        let xpath = "3 treat as integer+";

        match Lexer2::new(&String::from(xpath)) {
            Ok(lex) => {
                println!("Tokens:\n{}", lex.token_dump());
            },
            Err(e) => {
                println!("Lexer2 Err: {}", e);
            },
        }

        match compile_xpath(&String::from(xpath)) {
            Ok(xnode) => {
                println!("\n{}", xnode_dump(&xnode));
            },
            Err(e) => {
                println!("Err: {}", e);
            }
        }
//        assert_eq!("A", "Z");
    }
}

