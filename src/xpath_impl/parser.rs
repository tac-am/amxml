//
// xpath_impl/parser.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
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
    VarRef,
    ApplyPostfix,
    KindTest,
    DocumentTest,
    ElementTest,
    AttributeTest,
    SchemaElementTest,
    SchemaAttributeTest,
    PITest,
    CommentTest,
    TextTest,
    NamespaceNodeTest,
    AnyKindTest,
    KindTestTypeName,
    EmptySequenceTest,
    ItemTest,
    AtomicType,
    SingleType,
    SequenceType,
    OperatorConcat,
    OperatorMap,
    LetExpr,
    LetVarBind,
    InlineFunction,
    Param,
    ReturnType,
    ArgumentListTop,
    NamedFunctionRef,
}

// =====================================================================
//
impl fmt::Display for XNodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{:?}", self);
    }
}

// =====================================================================
//
#[derive(Eq, PartialEq, Clone)]
pub struct XNodePtr {
    xnode_ptr: Rc<RefCell<XNode>>,
}

#[derive(Eq, PartialEq, Clone)]
struct XNode {
    n_type: XNodeType,
    name: String,
    left: Option<XNodePtr>,
    right: Option<XNodePtr>,
}

// =====================================================================
//
impl fmt::Debug for XNodePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{}", xnode_dump(self));
    }
}

// =====================================================================
//
impl fmt::Display for XNodePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{:?}", self);
    }
}

// ---------------------------------------------------------------------
//
fn xnode_dump(xnode: &XNodePtr) -> String {
    return xnode_dump_sub(xnode, 0, 4, "T");
}

// ---------------------------------------------------------------------
//
fn xnode_dump_sub(xnode: &XNodePtr, indent: usize, step: usize, pref: &str) -> String {
    let mut buf: String = format!("{}{} [{}] {}\n",
            &" ".repeat(indent),
            pref,
            get_xnode_type(xnode),
            &get_xnode_name(&xnode));
    let xl = get_left(xnode);
    if ! is_nil_xnode(&xl) {
        buf += &xnode_dump_sub(&xl, indent + step, step, "L");
    }
    let xr = get_right(xnode);
    if ! is_nil_xnode(&xr) {
        buf += &xnode_dump_sub(&xr, indent + step, step, "R");
    }
    return buf;
}

// =====================================================================
// 構文解析用の補助マクロ。
//

// ---------------------------------------------------------------------
// 次にトークン $ttype が現れることを確認し、そうでなければエラーとする。
//
macro_rules! error_if_not_ttype {
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
//          then else in return satisfies as
//      については、TType::Nameとして返される。
//
macro_rules! error_if_not_name {
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
macro_rules! return_nil_if_not_ttype {
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
    let mut lex = Lexer::new(xpath)?;

    return parse_main(&mut lex);
}

// ---------------------------------------------------------------------
// [  1] XPath ::= Expr
//
fn parse_main(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_expr(lex)?;

    error_if_not_ttype!(lex, TType::EOF, "{}: 余分な字句が継続。");

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 39] AxisStep ::= (ReverseStep | ForwardStep) PredicateList
// [ 40] ForwardStep ::= (ForwardAxis NodeTest) | AbbrevForwardStep
// [ 41] ForwardAxis ::= ("child" "::")
//                     | ("descendant" "::")
//                     | ("attribute" "::")
//                     | ("self" "::")
//                     | ("descendant-or-self" "::")
//                     | ("following-sibling" "::")
//                     | ("following" "::")
//                     | ("namespace" "::")
// [ 42] AbbrevForwardStep ::= "@"? NodeTest
// [ 43] ReverseStep ::= (ReverseAxis NodeTest) | AbbrevReverseStep
// [ 44] ReverseAxis ::= ("parent" "::")
//                     | ("ancestor" "::")
//                     | ("preceding-sibling" "::")
//                     | ("preceding" "::")
//                     | ("ancestor-or-self" "::")
// [ 45] AbbrevReverseStep ::= ".."
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
fn parse_axis_step(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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

            error_if_not_ttype!(lex, TType::ColonColon, "{}: 軸名の次に :: が必要。");
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
            return parse_node_test(lex, axis);
        },
        TType::At => {  // 「@」は「attribute::」の省略形
            lex.get_token();
            return parse_node_test(lex, &XNodeType::AxisAttribute);
        },
        TType::DotDot => {// 「..」は「parent::node()」の省略形
            lex.get_token();
            return Ok(new_xnode(XNodeType::AxisParent, "node()"));
        },
        _ => {  // 「空」は「child::」の省略形
            return parse_node_test(lex, &XNodeType::AxisChild);
        },
    }
}

// ---------------------------------------------------------------------
// [ 46] NodeTest ::= KindTest | NameTest
// [ 47] NameTest ::= EQName | Wildcard
//
fn parse_node_test(lex: &mut Lexer, axis_type: &XNodeType) -> Result<XNodePtr, Box<Error>> {
    let mut name = parse_wildcard(lex)?;
    if name == "" {
        name = parse_eqname(lex)?;
    }

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
// [ 83] KindTest ::= DocumentTest                                     ☆
//                  | ElementTest
//                  | AttributeTest
//                  | SchemaElementTest                                ☆
//                  | SchemaAttributeTest                              ☆
//                  | PITest
//                  | CommentTest
//                  | TextTest
//                  | NamespaceNodeTest                                ☆
//                  | AnyKindTest
//
//
//     KindTest        KindTest      KindTest    etc.
//         |              |             |
//    DocumentTest   ElementTest      PITest
//         |        (element-name)    (arg)
//         |              |
//       .....          .....
//
fn parse_kind_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

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
        xnode = parse_namespace_node_test(lex)?;
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
// [ 85] DocumentTest ::= "document-node" "(" (ElementTest | SchemaElementTest)? ")"
//
fn parse_document_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::DocumentTest);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();

    // s_xnode: (ElementTest | SchemaElementTest)?
    let mut s_xnode = parse_element_test(lex)?;
    if is_nil_xnode(&s_xnode) {
        s_xnode = parse_schema_element_test(lex)?;
    }

    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    let document_test_xnode = new_xnode(XNodeType::DocumentTest, "");
    if ! is_nil_xnode(&s_xnode) {
        assign_as_left(&document_test_xnode, &s_xnode);
    }
    return Ok(document_test_xnode);
}


// ---------------------------------------------------------------------
// [ 94] ElementTest ::= "element" "(" (ElementNameOrWildcard ("," TypeName "?"?)?)? ")"
// [ 95] ElementNameOrWildcard ::= ElementName | "*"
// [ 99] ElementName ::= EQName
//                              ☆ EQNameを取得するようになっていない
// [101] TypeName ::= EQName
//
//     ElementTest
//    (element-name)
//          |
//   KindTestTypeName
//      (type-name)
//
fn parse_element_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::ElementTest);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
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
                    error_if_not_ttype!(lex, TType::Name, "{}: 型名が必要。");
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

    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    let element_test_xnode = new_xnode(XNodeType::ElementTest, arg);

    if arg_type_name != "" {
        let type_name_xnode = new_xnode(XNodeType::KindTestTypeName, &arg_type_name);
        assign_as_left(&element_test_xnode, &type_name_xnode);
    }

    return Ok(element_test_xnode);
}

// ---------------------------------------------------------------------
// [ 90] AttributeTest ::= "attribute" "(" (AttribNameOrWildcard ("," TypeName)?)? ")"
// [ 91] AttribNameOrWildcard ::= AttributeName | "*"
// [ 98] AttributeName ::= EQName
//                              ☆ EQNameを取得するようになっていない
// [101] TypeName ::= EQName
//
fn parse_attribute_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::AttributeTest);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
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
                    error_if_not_ttype!(lex, TType::Name, "{}: 型名が必要。");
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

    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
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
// [ 96] SchemaElementTest ::= "schema-element" "(" ElementDeclaration ")"
// [ 97] ElementDeclaration ::= ElementName
// [ 99] ElementName ::= EQName
//
fn parse_schema_element_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_one(lex,
                TType::SchemaElementTest, XNodeType::SchemaElementTest);
}

// ---------------------------------------------------------------------
// (当面、構文解析のみ)
// [ 92] SchemaAttributeTest ::= "schema-attribute" "(" AttributeDeclaration ")"
// [ 93] AttributeDeclaration ::= AttributeName
// [ 98] AttributeName ::= EQName
//
fn parse_schema_attribute_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_one(lex,
                TType::SchemaAttributeTest, XNodeType::SchemaAttributeTest);
}

// ---------------------------------------------------------------------
// SchemaElementTest | SchemaAttributeTest に共通:
// テスト名 (ttype) の後に、"(" EQName ")" が続いているとき、
// xnode (XNodeType: xnode_type) を生成して返す。
//                              ☆ EQNameを取得するようになっていない
//
fn parse_kind_test_sub_one(lex: &mut Lexer,
        ttype: TType, xnode_type: XNodeType)
                                    -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, ttype);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();
    error_if_not_ttype!(lex, TType::Name, "{}: 名前が必要。");
    let tok = lex.get_token();
    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(xnode_type, tok.get_name()));
}

// ---------------------------------------------------------------------
// [ 89] PITest ::= "processing-instruction" "(" (NCName | StringLiteral)? ")"
//
fn parse_pi_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::PITest);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
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

    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(XNodeType::PITest, arg));
}

// ---------------------------------------------------------------------
// [ 87] CommentTest ::= "comment" "(" ")"
//
fn parse_comment_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::CommentTest, XNodeType::CommentTest);
}

// ---------------------------------------------------------------------
// [ 86] TextTest ::= "text" "(" ")"
//
fn parse_text_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::TextTest, XNodeType::TextTest);
}

// ---------------------------------------------------------------------
// (当面、構文解析のみ)
// [ 88] NamespaceNodeTest ::= "namespace-node" "(" ")"                ☆
//
fn parse_namespace_node_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::NamespaceNodeTest, XNodeType::NamespaceNodeTest);
}

// ---------------------------------------------------------------------
// [ 84] AnyKindTest ::= "node" "(" ")"
//
fn parse_any_kind_test(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return parse_kind_test_sub_none(lex,
                TType::AnyKindTest, XNodeType::AnyKindTest);
}

// ---------------------------------------------------------------------
// AnyKindTest | TextTest | NamespaceNodeTest | CommentTest に共通。
// また、SequenceTypeの「empty-sequence()」、ItemType の「item()」にも共通。
// テスト名 (ttype) の後に、引数なしで "(" ")" が続いているとき、
// xnode (XNodeType: xnode_type) を生成して返す。
//
fn parse_kind_test_sub_none(lex: &mut Lexer,
        ttype: TType, xnode_type: XNodeType)
                                    -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, ttype);
    lex.get_token();
    error_if_not_ttype!(lex, TType::LeftParen, "{}: 開き括弧が必要。");
    lex.get_token();
    error_if_not_ttype!(lex, TType::RightParen, "{}: 閉じ括弧が必要。");
    lex.get_token();

    return Ok(new_xnode(xnode_type, ""));
}

// ---------------------------------------------------------------------
// [ 51] PredicateList ::= Predicate*
//
// Predicate{Rev}Top --- Predicate{Rev}Top ---...
//        |                     |
//        |                   (Expr)
//        |
//      (Expr)
//
//          Predicateが0個の場合はNilを返す。
//
fn parse_predicate_list(lex: &mut Lexer, reverse_order: bool) -> Result<XNodePtr, Box<Error>> {
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
// [ 52] Predicate ::= "[" Expr "]"
//
fn parse_predicate(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, TType::LeftBracket);
    lex.get_token();

    let xnode = parse_expr(lex)?;

    error_if_not_ttype!(lex, TType::RightBracket, "{}: 述語を閉じる「]」が必要。");
    lex.get_token();

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [  6] Expr ::= ExprSingle ( "," ExprSingle )*
//
//   OperatorConcatenate --- OperatorConcatenate --- nil
//         |                       |
//         |                    IfExpr ...
//         |                      ...
//     OperatorOr ...
//        ...
// 3.3.1 Constructing Sequences
// Comma operator: evaluates each of its operands and concatenates
// the resulting sequences, in order, into a single result sequence.
//
fn parse_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Comma, XNodeType::OperatorConcatenate ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_expr_single, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [  7] ExprSingle ::= ForExpr
//                    | LetExpr
//                    | QuantifiedExpr
//                    | IfExpr
//                    | OrExpr
//
fn parse_expr_single(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_for_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_let_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_quantified_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_if_expr(lex)?;
    return_if_non_nil!(xnode);

    return parse_or_expr(lex);
}

// ---------------------------------------------------------------------
// [  8] ForExpr ::= SimpleForClause "return" ExprSingle
// [  9] SimpleForClause ::= "for" SimpleForBinding ("," SimpleForBinding)*
//
//  ForExpr --- ForVarBind ------ ForVarBind --- ... --- (ExprSingle)
//               (変数名)          (変数名)
//                  |                 |
//                 ... (ExprSingle)  ... (ExprSingle)
// 
fn parse_for_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::For);
    lex.get_token();

    let for_expr_xnode = new_xnode(XNodeType::ForExpr, "for");
    let mut curr_xnode = for_expr_xnode.clone();
    loop {
        let var_bind_xnode = parse_simple_for_binding(lex)?;
        if is_nil_xnode(&var_bind_xnode) {
            break;
        }
        assign_as_right(&curr_xnode, &var_bind_xnode);
        curr_xnode = get_right(&curr_xnode);

        let tok = lex.next_token();
        if tok.get_type() != TType::Comma {
            break;
        }
        lex.get_token();
    }

    error_if_not_name!(lex, "return", "{}: for に対応する return が必要。");
    lex.get_token();

    let expr_single_xnode = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &expr_single_xnode);

    return Ok(for_expr_xnode);
}

// ---------------------------------------------------------------------
// [ 10] SimpleForBinding ::= "$" VarName "in" ExprSingle
// [ 60] VarName ::= EQName
//
//  ForVarBind
//   (変数名)
//      |
//     ... (ExprSingle)
//
fn parse_simple_for_binding(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    return parse_simple_binding(lex, &XNodeType::ForVarBind);
}

// ---------------------------------------------------------------------
// [ 11] LetExpr ::= SimpleLetClause "return" ExprSingle
// [ 12] SimpleLetClause ::= "let" SimpleLetBinding ("," SimpleLetBinding)*
//
//  LetExpr --- LetVarBind ------ LetVarBind --- ... --- (ExprSingle)
//               (変数名)          (変数名)
//                  |                 |
//                 ... (ExprSingle)  ... (ExprSingle)
// 
fn parse_let_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::Let);
    lex.get_token();

    let let_expr_xnode = new_xnode(XNodeType::LetExpr, "let");
    let mut curr_xnode = let_expr_xnode.clone();
    loop {
        let var_bind_xnode = parse_simple_let_binding(lex)?;
        if is_nil_xnode(&var_bind_xnode) {
            break;
        }
        assign_as_right(&curr_xnode, &var_bind_xnode);
        curr_xnode = get_right(&curr_xnode);

        let tok = lex.next_token();
        if tok.get_type() != TType::Comma {
            break;
        }
        lex.get_token();
    }

    error_if_not_name!(lex, "return", "{}: let に対応する return が必要。");
    lex.get_token();

    let expr_single_xnode = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &expr_single_xnode);

    return Ok(let_expr_xnode);
}

// ---------------------------------------------------------------------
// [ 13] SimpleLetBinding ::= "$" VarName ":=" ExprSingle
// [ 60] VarName ::= EQName
//
//  LetVarBind
//   (変数名)
//      |
//     ... (ExprSingle)
//
fn parse_simple_let_binding(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, TType::Dollar);
    lex.get_token();

    let var_name = parse_eqname(lex)?;
    if var_name == "" {
        return Err(xpath_syntax_error!(
                    "{}: $ の後には変数名が必要。",
                    lex.around_tokens().as_str()));
    }

    error_if_not_ttype!(lex, TType::Bind, "{}: 変数名の後に := が必要。");
    lex.get_token();

    let expr_single_xnode = parse_expr_single(lex)?;
    let var_bind_xnode = new_xnode(XNodeType::LetVarBind, &var_name);
    assign_as_left(&var_bind_xnode, &expr_single_xnode);

    return Ok(var_bind_xnode);
}

// ---------------------------------------------------------------------
// [ 14] QuantifiedExpr ::= ("some" | "every")
//                  "$" VarName "in" ExprSingle
//                      ("," "$" VarName "in" ExprSingle)*
//                  "satisfies" ExprSingle
//
// 規格の記述は上のようになっているが、ForExprに準じて次のように考える。
// [ 14a] QuantifiedExpr ::= SimpleQuantifiedClause "satisfies" ExprSingle
// [ 14b] SimpleQuantifiedClause ::= ("some" | "every") 
//                   SimpleQuantifiedBinding ("," SimpleQuantifiedBinding)*
// [ 14c] SimpleQuantifiedBinding ::= "$" VarName "in" ExprSingle
// [ 60] VarName ::= EQName
//
fn parse_quantified_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let quantified_expr_xnode;
    let xnode_ttype;
    let tok = lex.next_token();
    match tok.get_type() {
        TType::Some => {
            lex.get_token();
            quantified_expr_xnode = new_xnode(XNodeType::SomeExpr, "some");
            xnode_ttype = XNodeType::SomeVarBind;
        },
        TType::Every => {
            lex.get_token();
            quantified_expr_xnode = new_xnode(XNodeType::EveryExpr, "every");
            xnode_ttype = XNodeType::EveryVarBind;
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }

    let mut curr_xnode = quantified_expr_xnode.clone();
    loop {
        let xnode_var_bind = parse_simple_binding(lex, &xnode_ttype)?;
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

    error_if_not_name!(lex, "satisfies", "{}: some/every に対応する satisfies が必要。");
    lex.get_token();

    let expr_single_xnode = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &expr_single_xnode);

    return Ok(quantified_expr_xnode);
}

// ---------------------------------------------------------------------
// [ 10] SimpleForBinding ::= "$" VarName "in" ExprSingle
// [ 60] VarName ::= EQName
// および、{Some,Every}Exprについて同様の構文。
//
// {For,Some,Every}VarBind
//         (変数名)
//            |
//           ... (ExprSingle)
//
fn parse_simple_binding(lex: &mut Lexer, xnode_type: &XNodeType) -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, TType::Dollar);
    lex.get_token();

    let var_name = parse_eqname(lex)?;
    if var_name == "" {
        return Err(xpath_syntax_error!(
                    "{}: $ の後には変数名が必要。",
                    lex.around_tokens().as_str()));
    }

    error_if_not_name!(lex, "in", "{}: 変数名の後に in が必要。");
    lex.get_token();

    let expr_single_xnode = parse_expr_single(lex)?;
    let var_bind_xnode = new_xnode(xnode_type.clone(), &var_name);
    assign_as_left(&var_bind_xnode, &expr_single_xnode);

    return Ok(var_bind_xnode);
}

// ---------------------------------------------------------------------
// [ 15] IfExpr ::= "if" "(" Expr ")" "then" ExprSingle "else" ExprSingle
//
//      IfExpr --- IfThenElse --- (xnode_else)
//         |            |
//         |        (xnode_then)
//         |
//    (xnode_cond)
//
fn parse_if_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    return_nil_if_not_ttype!(lex, TType::If);
    lex.get_token();

    error_if_not_ttype!(lex, TType::LeftParen, "{}: if 文には左括弧が必要。");
    lex.get_token();

    let xnode_cond = parse_expr(lex)?;
    error_if_nil!(lex, xnode_cond, "{}: if文の条件式が不正。");

    error_if_not_ttype!(lex, TType::RightParen, "{}: 条件式を閉じる右括弧が必要。");
    lex.get_token();

    error_if_not_name!(lex, "then", "{}: if に対応する then が必要。");
    lex.get_token();

    let xnode_then = parse_expr_single(lex)?;
    if is_nil_xnode(&xnode_then) {
        return Err(xpath_syntax_error!(
                "{}: if文のthen節が不正。", lex.around_tokens().as_str()));
    }

    error_if_not_name!(lex, "else", "{}: if に対応する else が必要。");
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
// [ 16] OrExpr ::= AndExpr ( "or" AndExpr )*
//
fn parse_or_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Or, XNodeType::OperatorOr ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_and_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 17] AndExpr ::= ComparisonExpr ( "and" ComparisonExpr )*
//
fn parse_and_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::And, XNodeType::OperatorAnd ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_comparison_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 18] ComparisonExpr ::= StringConcatExpr ( (ValueComp
//                           | GeneralComp
//                           | NodeComp) StringConcatExpr )?
// [ 33] ValueComp ::= "eq" | "ne" | "lt" | "le" | "gt" | "ge"
// [ 32] GenerapComp ::= "=" | "!=" | "<" | "<=" | ">" | ">="
// [ 34] NodeComp ::= "is" | "<<" | ">>"
//
fn parse_comparison_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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

    return parse_bin_op_sub(lex, parse_string_concat_expr, &token_node_map, true);
}

// ---------------------------------------------------------------------
// [ 19] StringConcatExpr ::= RangeExpr ( "||" RangeExpr )*
//
fn parse_string_concat_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::OperatorConcat, XNodeType::OperatorConcat ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_range_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 20] RangeExpr ::= AdditiveExpr ( "to" AdditiveExpr )?
//
fn parse_range_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::To, XNodeType::OperatorTo ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_additive_expr, &token_node_map, true);
}

// ---------------------------------------------------------------------
// [ 21] AdditiveExpr ::= MultiplicativeExpr
//                         ( ( "+" | "-" ) MultiplicativeExpr )*
//
fn parse_additive_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Plus, XNodeType::OperatorAdd ),
        ( TType::Minus, XNodeType::OperatorSubtract ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_multiplicative_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 22] MultiplicativeExpr ::= UnionExpr
//                         ( ( "*" | "div" | "idiv" | "mod" ) UnionExpr )*
//
fn parse_multiplicative_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Asterisk, XNodeType::OperatorMultiply ),
        ( TType::Div, XNodeType::OperatorDiv ),
        ( TType::IDiv, XNodeType::OperatorIDiv ),
        ( TType::Mod, XNodeType::OperatorMod ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_union_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 23] UnionExpr ::= IntersectExceptExpr
//                         ( ( "union" | "|" ) IntersectExceptExpr )*
//
fn parse_union_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Union, XNodeType::OperatorUnion ),
    ].iter().cloned().collect();

    let xnode = parse_bin_op_sub(lex, parse_intersect_except_expr, &token_node_map, false)?;

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 24] IntersectExceptExpr ::= InstanceofExpr
//                         ( ( "intersect" | "except" ) InstanceofExpr )*
//
fn parse_intersect_except_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Intersect, XNodeType::OperatorIntersect ),
        ( TType::Except, XNodeType::OperatorExcept ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_instanceof_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 25] InstanceofExpr ::= TreatExpr ( ( "instance" "of" ) SequenceType )?
//
fn parse_instanceof_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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
// [ 26] TreatExpr ::= CastableExpr ( ( "treat" "as" ) SequenceType )?
//
fn parse_treat_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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
// [ 79] SequenceType ::= ("empty-sequence" "(" ")")
//                      | (ItemType OccurenceIndicator?)
// [ 80] OccurrenceIndicator ::= "?" | "*" | "+"
//
//   SequenceType            SequenceType         SequenceType
//        |                 (? | * | + | _)      (? | * | + | _)
//        |                       |                    |
// EmptySequenceTest          KindTest             AtomicType
//                              .....                .....
//
fn parse_sequence_type(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

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
// ((52)) ItemType ::= KindTest 
//                 | ( "item" "(" ")" )
//                 | AtomicType
// ((53)) AtomicType ::= QName
//
// [ 81] ItemType ::= KindTest
//                  | ("item" "(" ")")
//                  | FunctionTest                                     ☆
//                  | MapTest                                          ☆
//                  | ArrayTest                                        ☆
//                  | AtomicOrUnionType                               (☆)
//                  | ParenthesizedItemType                            ☆
// [102] FunctionTest ::= AnyFunctionTest
//                      | TypedFunctionTest
// [103] AnyFunctionTest ::= "function" "(" "*" ")"
// [104] TypedFunctionTest ::= "function" "("
//                                 SequenceType ("," SequenceType)*)? ")"
//                                 "as" SequenceType
// [ 82] AtomicOrUnionType ::= EQName
// [111] ParenthesizedItemType ::= "(" ItemType ")"
// [105] MapTest ::= AnyMapTest | TypedMapTest
// [106] AnyMapTest ::= "map" "(" "*" ")"
// [107] TypedMapTest ::= "map" "(" AtomicOrUnionType "," SequenceType ")"
// [108] ArrayTest ::= AnyArrayTest | TypedArrayTest
// [109] AnyArrayTest ::= "array" "(" "*" ")"
// [110] TypedArrayTest ::= "array" "(" SequenceType ")"
//
//   KindTest             KindTest         AtomicType
//      |                    |               (type)
//  DocumentTestなど      ItemTest
//    .....           (これもKindTest扱い)
//
fn parse_item_type(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_kind_test(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_kind_test_sub_none(lex, TType::Item, XNodeType::ItemTest)?;
    return_if_non_nil!(xnode);

    let eqname = parse_eqname(lex)?;
    if eqname != "" {
        let xnode = new_xnode(XNodeType::AtomicType, &eqname);
        return Ok(xnode);
    }

    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [ 27] CastableExpr ::= CastExpr ( "castable" "as" ) SingleType )?
//
// OperatorCastableAs --- SingleType
//       |                   |
//   (CastExpr)          AtomicType
//                         (type)
//
fn parse_castable_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

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
// [ 28] CastExpr ::= ArrowExpr ( ( "cast" "as" ) SingleType )?
//
// OperatorCastAs --- SingleType
//       |               |
//   (UnaryExpr)     AtomicType
//                     (type)
//
fn parse_cast_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_arrow_expr(lex)?;
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
// ((49)) SingleType ::= AtomicType "?"?
// ((53)) AtomicType ::= QName
//
// [ 77] SingleType ::= SimpleTypeName "?"?
// [100] SimpleTypeName ::= TypeName
// [101] TypeName ::= EQName
//
fn parse_single_type(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let mut eqname = parse_eqname(lex)?;
    if eqname != "" {
        let tok = lex.next_token();
        if tok.get_type() == TType::Question {
            lex.get_token();
            eqname += tok.get_name();
        }
        let single_type_xnode = new_xnode(XNodeType::SingleType, "");
        let atomic_type_xnode = new_xnode(XNodeType::AtomicType, &eqname);
        assign_as_left(&single_type_xnode, &atomic_type_xnode);
        return Ok(single_type_xnode);
    }
    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [ 29] ArrowExpr ::= UnaryExpr ( "=>" ArrowFunctionSpecifier ArgumentList)*
// [ 55] ArrowFunctionSpecifier ::= EQName
//                                | VarRef
//                                | ParenthesizedExpr                  ☆
//
// UnaryExprを第1引数とすることを除き、FunctionCallと同じ構文木を生成する。
//
// (ArrowFunctionSpecifier ::= EQName の場合)
//
// FunctionCall --- ArgumentTop --- ArgumentTop --- ...
//   (函数名)           |               |    <ArgumentList相当の構文木>
//                      |              ...
//                      |
//                 (UnaryExpr)
//
// (ArrowFunctionSpecifier ::= VarRef の場合)
//
// ApplyPostfix --- ArgumentListTop
//      |                 |
//    VarRef          ArgumentTop --- ArgumentTop --- ...
//   (変数名)             |               |    <ArgumentList相当の構文木>
//                    (UnaryExpr)        ...
//
fn parse_arrow_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_unary_expr(lex)?;
    let mut curr_xnode = xnode.clone();
    while lex.next_token().get_type() == TType::Arrow {
        lex.get_token();

        let func_name = parse_static_func_name(lex)?;
        if func_name != "" {
            let fcall_xnode = new_xnode(XNodeType::FunctionCall, &func_name);
            let arglist_xnode = parse_argument_list(lex)?;
            let arg_top_xnode = new_xnode(XNodeType::ArgumentTop, "");
            assign_as_left(&arg_top_xnode, &curr_xnode);
            assign_as_right(&arg_top_xnode, &arglist_xnode);
            assign_as_right(&fcall_xnode, &arg_top_xnode);
            curr_xnode = fcall_xnode.clone();
            continue;
        }

        let varref_xnode = parse_varref(lex)?;
        if ! is_nil_xnode(&varref_xnode) {
            let apply_postfix_xnode = new_xnode(XNodeType::ApplyPostfix, "");
            assign_as_left(&apply_postfix_xnode, &varref_xnode);
            let argument_top_xnode = new_xnode(XNodeType::ArgumentListTop, "");
            assign_as_right(&apply_postfix_xnode, &argument_top_xnode);

            let arglist_xnode = parse_argument_list(lex)?;
            let arg_top_xnode = new_xnode(XNodeType::ArgumentTop, "");
            assign_as_left(&arg_top_xnode, &curr_xnode);
            assign_as_right(&arg_top_xnode, &arglist_xnode);
            assign_as_left(&argument_top_xnode, &arg_top_xnode);

            curr_xnode = apply_postfix_xnode.clone();
            continue;
        }

        return Err(xpath_syntax_error!(
                    "{}: アロー演算子: 函数名が必要。",
                    lex.around_tokens().as_str()));
    }

    return Ok(curr_xnode.clone());
}

// ---------------------------------------------------------------------
// [ 23] UnaryExpr ::= ( "-" | "+" )? ValueExpr
//
fn parse_unary_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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
// [ 31] ValueExpr ::= SimpleMapExpr
// [ 35] SimpleMapExpr ::= PathExpr ("!" PathExpr)*
//
fn parse_value_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::OperatorMap, XNodeType::OperatorMap ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_path_expr, &token_node_map, false);
}

// ---------------------------------------------------------------------
// 二項演算子を解析
//    expr ::= subexpr (op subexpr)+ と考え、左結合になるように実装する。
//    op_once: trueならば「subexpr (op subexpr)?」として扱う (nonassoc)。
//
fn parse_bin_op_sub(lex: &mut Lexer,
        sub_parser: fn(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>>,
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
// [ 36] PathExpr ::= ("/" RelativePathExpr?)
//                  | ("//" RelativePathExpr)
//                  | RelativePathExpr
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
fn parse_path_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let tok = lex.next_token();
    match tok.get_type() {
        TType::Slash => {
            lex.get_token();

            let op_path_xnode = new_xnode(XNodeType::OperatorPath, "parse_path_expr Slash");
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

            let op_path_xnode_u = new_xnode(XNodeType::OperatorPath, "parse_path_expr SlashSlash 1");
            let root_xnode = new_xnode(XNodeType::AxisRoot, "/");
            assign_as_left(&op_path_xnode_u, &root_xnode);

            let op_path_xnode_l = new_xnode(XNodeType::OperatorPath, "parse_path_expr SlashSlash 2");
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
// [ 37] RelativePathExpr ::= StepExpr (("/" | "//") StepExpr)*
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
fn parse_relative_path_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let step_expr_xnode = parse_step_expr(lex)?;
    if is_nil_xnode(&step_expr_xnode) {
        return Ok(new_nil_xnode());
    }
    let top_op_path_xnode = new_xnode(XNodeType::OperatorPath, "parse_relative_path_expr TOP");
    assign_as_left(&top_op_path_xnode, &step_expr_xnode);
    let mut curr_xnode = top_op_path_xnode.clone();

    loop {
        let tok = lex.next_token();
        match tok.get_type() {
            TType::Slash => {
                lex.get_token();
                let step_expr_xnode = parse_step_expr(lex)?;
                let op_path_xnode = new_xnode(XNodeType::OperatorPath, "parse_relative_path_expr Slash");
                assign_as_left(&op_path_xnode, &step_expr_xnode);
                assign_as_right(&curr_xnode, &op_path_xnode);
                curr_xnode = op_path_xnode.clone();
            },
            TType::SlashSlash => {
                lex.get_token();
                let step_expr_xnode = parse_step_expr(lex)?;

                let op_path_xnode_u = new_xnode(XNodeType::OperatorPath, "parse_relative_path_expr SlashSlash 1");
                let ds_xnode = new_xnode(XNodeType::AxisDescendantOrSelf, "node()");
                assign_as_left(&op_path_xnode_u, &ds_xnode);

                let op_path_xnode_l = new_xnode(XNodeType::OperatorPath, "parse_relative_path_expr SlashSlash 2");
                assign_as_left(&op_path_xnode_l, &step_expr_xnode);

                assign_as_right(&op_path_xnode_u, &op_path_xnode_l);
                assign_as_right(&curr_xnode, &op_path_xnode_u);
                curr_xnode = op_path_xnode_l.clone();
            },
            _ => {
                break;
            },
        }
    }

    // -----------------------------------------------------------------
    // 最後にtop_op_path_xnode (最上位のxnode) を返す。
    // ただし、「(("/" | "//") StepExpr)*」部分が0個だった (rightがNil) 場合は
    // 冗長なので、top_op_path_xnodeの左辺ノードを返す。
    //
    let right_of_top = get_right(&top_op_path_xnode);
    if is_nil_xnode(&right_of_top) {
        let left_of_top = get_left(&top_op_path_xnode);
        return Ok(left_of_top);
    } else {
        return Ok(top_op_path_xnode);
    }
}

// ---------------------------------------------------------------------
// [ 38] StepExpr ::= PostfixExpr | AxisStep
//
fn parse_step_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_postfix_expr(lex)?;
    return_if_non_nil!(xnode);

    return parse_axis_step(lex);
}

// ---------------------------------------------------------------------
// [ 49] PostfixExpr ::= PrimaryExpr (Predicate | ArgumentList | Lookup)*
// これを次のように分解する。
// [ 49a] PostfixExpr ::= PrimaryExpr PostfixList
// [ 49b] PostfixList ::= Postfix*
// [ 49c] Postfix ::= Predicate | ArgumentList | Lookup
//
//   [ApplyPostfix] -- (predicate) --- (argument_list) --- ...
//           |
//     (PrimaryExpr) --- (右辺値)...
//           |
//       (左辺値)...
//
// ただしPostfixListが空の場合はPrimaryExprをそのまま返す。
//
fn parse_postfix_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let primary_xnode = parse_primary_expr(lex)?;
    return_if_nil!(primary_xnode);

    let postfix_xnode = parse_postfix_list(lex)?;
    if ! is_nil_xnode(&postfix_xnode) {
        let apply_postfix_xnode = new_xnode(XNodeType::ApplyPostfix, "");
        assign_as_left(&apply_postfix_xnode, &primary_xnode);
        assign_as_right(&apply_postfix_xnode, &postfix_xnode);
        return Ok(apply_postfix_xnode);
    } else {
        return Ok(primary_xnode);
    }
}

// ---------------------------------------------------------------------
// [ 49b] PostfixList ::= Postfix*
//
fn parse_postfix_list(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let top_xnode = parse_postfix(lex)?;
    return_if_nil!(top_xnode);

    let mut curr_xnode = top_xnode.clone();
    loop {
        let xnode = parse_postfix(lex)?;
        if is_nil_xnode(&xnode) {
            break;
        }
        assign_as_right(&curr_xnode, &xnode);
        curr_xnode = xnode.clone();
    }
    return Ok(top_xnode);
}

// ---------------------------------------------------------------------
// [ 49c] Postfix ::= Predicate | ArgumentList | Lookup
//
fn parse_postfix(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    match lex.next_token().get_type() {
        TType::LeftBracket => {
            let xnode = parse_predicate(lex)?;
            let predicate_top_xnode = new_xnode(XNodeType::PredicateTop, "");
            assign_as_left(&predicate_top_xnode, &xnode);
            return Ok(predicate_top_xnode);
        },
        TType::LeftParen => {
            let xnode = parse_argument_list(lex)?;
            let argument_top_xnode = new_xnode(XNodeType::ArgumentListTop, "");
            assign_as_left(&argument_top_xnode, &xnode);
            return Ok(argument_top_xnode);
        },
//        TType::Question => {
//        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }
}

// ---------------------------------------------------------------------
// [ 56] PrimaryExpr ::= Literal
//                     | VarRef
//                     | ParenthesizedExpr
//                     | ContextItemExpr
//                     | FunctionCall
//                     | FunctionItemExpr
//                     | MapConstructor                               ☆
//                     | ArrayConstructor                             ☆
//                     | UnaryLookup                                  ☆
//
fn parse_primary_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let xnode = parse_literal(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_varref(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_parenthesized_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_context_item_expr(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_function_call(lex)?;
    return_if_non_nil!(xnode);

    let xnode = parse_function_item_expr(lex)?;
    return_if_non_nil!(xnode);

    return Ok(new_nil_xnode());
}

// ---------------------------------------------------------------------
// [ 57] Literal ::= NumericLiteral                -- [ 58] Lexer
//                 | StringLiteral                 -- [116] Lexer
//
// {String,Integer,Decimal,Double}Literal
//        (リテラル値の文字列)
//
fn parse_literal(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    match tok.get_type() {
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
        _ => {
            return Ok(new_nil_xnode());
        }
    }
}

// ---------------------------------------------------------------------
// [ 59] VarRef ::= "$" VarName
// [ 60] VarName ::= EQName
//
//      VarRef
// (変数名: EQName)
//
fn parse_varref(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    match tok.get_type() {
        TType::Dollar => {
            lex.get_token();

            let eqname = parse_eqname(lex)?;
            if eqname != "" {
                return Ok(new_xnode(XNodeType::VarRef, eqname.as_str()));
            } else {
                return Err(xpath_syntax_error!(
                        "{}: 変数参照の $ に続いて名前が必要。",
                        lex.around_tokens().as_str()));
            }
        },
        _ => {
            return Ok(new_nil_xnode());
        }
    }
}

// ---------------------------------------------------------------------
// [ 61] ParenthesizedExpr ::= "(" Expr? ")"
//
// Exprに相当するxnode
// ただしExprが空の場合、その旨を表すOperatorPath
//
fn parse_parenthesized_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let tok = lex.next_token();
    match tok.get_type() {
        TType::LeftParen => {
            lex.get_token();
            let xnode = parse_expr(lex)?;

            error_if_not_ttype!(lex, TType::RightParen,
                        "{}: 左括弧に対応する右括弧が必要。");
            lex.get_token();

            if ! is_nil_xnode(&xnode) {
                return Ok(xnode);
            } else {
                return Ok(new_xnode(XNodeType::OperatorPath, "parse_parenthesized_expr (Empty)"));
                        // 空の括弧式があることを示す。
            }
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }
}

// ---------------------------------------------------------------------
// [ 62] ContextItemExpr ::= "."
//
//   ContextItem
//       (.)
//
fn parse_context_item_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    let tok = lex.next_token();
    match tok.get_type() {
        TType::Dot => {
            lex.get_token();
            return Ok(new_xnode(XNodeType::ContextItem, "."));
                // XPath 1.0ではAxisSelfの意味であった。
                // 「(1 to 100) [. mod 5 eq 0]」のような文脈では原子値を表す。
                //
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }
}

// ---------------------------------------------------------------------
// [ 66] FunctionItemExpr ::= NamedFunctionRef
//                          | InlineFunctionExpr
//
fn parse_function_item_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_named_function_ref(lex)?;
    return_if_non_nil!(xnode);

    return parse_inline_function_expr(lex);
}

// ---------------------------------------------------------------------
// [ 67] NamedFunctionRef ::= EQName "#" IntegerLiteral
//
fn parse_named_function_ref(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    lex.mark_token_index();
    let func_name = parse_static_func_name(lex)?;
    if func_name == "" {                            // 非該当
        lex.restore_marked_index();
        return Ok(new_nil_xnode());
    }

    let tok = lex.next_token();
    if tok.get_type() != TType::Sharp {         // 非該当
        lex.restore_marked_index();
        return Ok(new_nil_xnode());
    }
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::IntegerLiteral {         // 非該当
        lex.restore_marked_index();
        return Ok(new_nil_xnode());
    }
    let arity = tok.get_name();
    lex.get_token();

    let xnode = new_xnode(XNodeType::NamedFunctionRef,
                    &(func_name + &"#" + &arity));
    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 68] InlineFunctionExpr ::= "function" "(" ParamList? ")"
//                                  ("as" SequenceType)? FunctionBody
//
// 「"as" SequenceType」省略時は「item()*」
//
// InlineFunction --- ReturnType ------- Param ------- Param ---...
//       |                |            (varname)     (varname)
//       |                |                |             |
//       |          (SequenceType)   (SequenceType)(SequenceType)
//       |
//     Expr (FunctionBody) ---...
//       |
//      ...
//
fn parse_inline_function_expr(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::Function);
    lex.get_token();

    error_if_not_ttype!(lex, TType::LeftParen, "{}: function 文には左括弧が必要。");
    lex.get_token();

    let param_list_xnode = parse_param_list(lex)?;

    error_if_not_ttype!(lex, TType::RightParen, "{}: 引数リストを閉じる右括弧が必要。");
    lex.get_token();

    let return_type_xnode = new_xnode(XNodeType::ReturnType, "");
    let tok = lex.next_token();
    if tok.get_type() == TType::Name && tok.get_name() == "as" {
        lex.get_token();
        let xnode = parse_sequence_type(lex)?;
        assign_as_left(&return_type_xnode, &xnode);
    } else {        // 型の省略時は「item()*」
        assign_as_left(&return_type_xnode, &default_sequence_type());
    }

    let function_body_xnode = parse_function_body(lex)?;

    let inline_function_xnode = new_xnode(XNodeType::InlineFunction, "(inline function)");
    assign_as_left(&inline_function_xnode, &function_body_xnode);
    assign_as_right(&inline_function_xnode, &return_type_xnode);
    assign_as_right(&return_type_xnode, &param_list_xnode);

    return Ok(inline_function_xnode);
}

// ---------------------------------------------------------------------
// [  2] ParamList ::= Param ("," Param)*
//
fn parse_param_list(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let top_param_xnode = parse_param(lex)?;
    return_if_nil!(top_param_xnode);

    let mut curr = top_param_xnode.clone();
    while lex.next_token().get_type() == TType::Comma {
        lex.get_token();
        let param_xnode = parse_param(lex)?;
        assign_as_right(&curr, &param_xnode);
        curr = param_xnode.clone();
    }

    return Ok(top_param_xnode);
}

// ---------------------------------------------------------------------
// [  3] Param ::= "$" EQName TypeDeclaration?
// [ 78] TypeDeclaration ::= "as" SequenceType
//
fn parse_param(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    return_nil_if_not_ttype!(lex, TType::Dollar);
    lex.get_token();

    let param_name = parse_eqname(lex)?;
    if param_name == "" {
        return Err(xpath_syntax_error!(
                    "{}: 引数名が必要。", lex.around_tokens().as_str()));
    }

    let tok = lex.next_token();
    if tok.get_type() == TType::Name && tok.get_name() == "as" {
        lex.get_token();
        let seq_type_xnode = parse_sequence_type(lex)?;
        let param_xnode = new_xnode(XNodeType::Param, &param_name);
        assign_as_left(&param_xnode, &seq_type_xnode);
        return Ok(param_xnode);
    } else {        // 型の省略時は「item()*」
        let param_xnode = new_xnode(XNodeType::Param, &param_name);
        assign_as_left(&param_xnode, &default_sequence_type());
        return Ok(param_xnode);
    }
}

// ---------------------------------------------------------------------
// 「item()*」に相当するSequenceType。
//
fn default_sequence_type() -> XNodePtr {
    let xnode = new_xnode(XNodeType::SequenceType, "*");
    let item_test_xnode = new_xnode(XNodeType::ItemTest, "");
    assign_as_left(&xnode, &item_test_xnode);
    return xnode;
}

// ---------------------------------------------------------------------
// [  4] FunctionBody ::= EnclosedExpr
// [  5] EnclosedExpr ::= "{" Expr? "}"
//
fn parse_function_body(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    error_if_not_ttype!(lex, TType::LeftCurly, "{}: 函数本体を始める左波括弧が必要。");
    lex.get_token();

    let xnode = parse_expr(lex)?;

    error_if_not_ttype!(lex, TType::RightCurly, "{}: 函数本体を閉じる右波括弧が必要。");
    lex.get_token();

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 69] MapConstructor ::= "map" "{" (MapConstructorEntry ("," MapConstructorEntry)*)? "}"
// [ 70] MapConstructorEntry ::= MapKeyExpr ":" MapValueExpr
// [ 71] MapKeyExpr ::= ExprSingle
// [ 72] MapValueExpr ::= ExprSingle
//
// ---------------------------------------------------------------------
// [ 73] ArrayConstructor ::= SquareArrayConstructor | CurlyArrayConstructor
// [ 74] SquareArrayConstructor ::= "[" (ExprSingle ("," ExprSingle)*)? "]"
// [ 75] CurlyArrayConstructor ::= "array" EnclosedExpr
// [  5] EnclosedExpr ::= "{" Expr? "}"
//
// ---------------------------------------------------------------------
// [ 53] Lookup ::= "?" KeySpecifier
//
// ---------------------------------------------------------------------
// [ 76] UnaryLookup ::= "?" KeySpecifier
// [ 54] KeySpecifier ::= NCName
//                      | IntegerLiteral
//                      | ParenthesizedExpr
//                      | "*"
//

// ---------------------------------------------------------------------
// [ 63] FunctionCall ::= EQName ArgumentList
//
// FuncCall -- ArgTop -- ArgTop -- ... -- Nil
//               |         |
//               |      OpLiteral
//               |
//              OpEQ  -- (rhs)
//               |
//              (lhs)
//
// 引数並びの順に、ArgumentTopを右に連結。
// ArgumentTopの左に、引数を表すExprを連結。
//
fn parse_function_call(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    // -------------------------------------------------------------
    // 左括弧まで先読みして、函数名か否か判定する。
    // func_nameは、必要ならば "fn:" を補った形になっている。
    //
    lex.mark_token_index();
    let func_name = parse_static_func_name(lex)?;
    if func_name == "" {                            // 非該当
        lex.restore_marked_index();
        return Ok(new_nil_xnode());
    }
    let tok = lex.next_token();
    if tok.get_type() != TType::LeftParen {         // 非該当
        lex.restore_marked_index();
        return Ok(new_nil_xnode());
    }

    let arg_node = parse_argument_list(lex)?;       // 引数が0個ならばNil

    // -------------------------------------------------------------
    // 引数の数を調べる。
    //
    let mut arity: usize = 0;
    let mut curr = arg_node.clone();
    while ! is_nil_xnode(&curr) {
        arity += 1;
        curr = get_right(&curr);
    }

    // -------------------------------------------------------------
    // この時点で函数表と照合して、函数の存在や引数の数を検査する。
    //
    if func::check_function_spec(&func_name, arity) == false {
        return Err(xpath_syntax_error!(
            "{}: 函数が未実装、または引数の数 ({}) が不適切。",
            func_name, arity));
    }

    // -------------------------------------------------------------
    //
    let func_node = new_xnode(XNodeType::FunctionCall, &func_name);
    assign_as_right(&func_node, &arg_node);

    return Ok(func_node);
}

// ---------------------------------------------------------------------
// [ 50] ArgumentList ::= "(" (Argument ("," Argument)*)? ")"
//
//  ArgTop -- ArgTop --...
//    |         |
//    |      OpLiteral
//    |
//   OpEQ  -- (rhs)
//    |
//  (lhs)
//                  ただし引数が0個の場合はNilを返す。
//
fn parse_argument_list(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {

    error_if_not_ttype!(lex, TType::LeftParen, "{}: 引数並びの左括弧が必要。");
    lex.get_token();

    let xnode = parse_argument_list_sub(lex)?;

    error_if_not_ttype!(lex, TType::RightParen, "{}: 引数並びの右括弧が必要。");
    lex.get_token();

    return Ok(xnode);
}

fn parse_argument_list_sub(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_argument(lex)?;
    return_if_nil!(xnode);

    let mut curr = xnode.clone();
    while lex.next_token().get_type() == TType::Comma {
        lex.get_token();
        let next_arg_xnode = parse_argument(lex)?;
        assign_as_right(&curr, &next_arg_xnode);
        curr = next_arg_xnode.clone();
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 64] Argument ::= ExprSingle
//                  | ArgumentPlaceholder                               ☆
// [ 65] ArgumentPlaceholder ::= "?"                                    ☆
//
fn parse_argument(lex: &mut Lexer) -> Result<XNodePtr, Box<Error>> {
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
// 函数名 (EQName) と解析できる字句があれば、その文字列を返す。
// ただし、UnprefixedNameである場合、"fn:" を補う。
// 該当する字句がなければ空文字列を返す。
//
fn parse_static_func_name(lex: &mut Lexer) -> Result<String, Box<Error>> {
    let func_name = parse_eqname(lex)?;
    if func_name == "" {
        return Ok(String::new());
    }

    let prefixed_func_name = 
        if ! func_name.starts_with("Q{") && ! func_name.contains(":") {
            String::from("fn:") + &func_name
        } else {
            func_name
        };
    return Ok(prefixed_func_name);
}

// ---------------------------------------------------------------------
// EQNameと解析できる字句があれば、その文字列を返す。
// 該当する字句がなければ空文字列を返す。
// [112] EQName ::= QName
//                | URIQualifiedName
// [117] URIQualifiedName ::= BracedURILiteral NCName
// [118] BracedURILiteral ::= "Q" "{" [^{}]* "}"
//
fn parse_eqname(lex: &mut Lexer) -> Result<String, Box<Error>> {
    let qname = parse_qname(lex)?;
    if qname != "" {
        return Ok(qname);
    }

    let mut uri_qualified_name = String::new();
    match lex.next_token().get_type() {
        TType::BracedURILiteral => {
            uri_qualified_name += lex.get_token().get_name();
            match lex.next_token().get_type() {
                TType::Name => {
                    uri_qualified_name += lex.get_token().get_name();
                    return Ok(uri_qualified_name);
                },
                _ => {
                    lex.unget_token();
                },
            }
        },
        _ => {},
    }

    return Ok(String::new());
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
fn parse_qname(lex: &mut Lexer) -> Result<String, Box<Error>> {
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

    error_if_not_ttype!(lex, TType::Name, "{}: QName: コロンの後には名前が必要。");
    let tok = lex.get_token();
    qname += tok.get_name();

    return Ok(qname);
}

// ---------------------------------------------------------------------
// Wildcardと解析できる字句があれば、その文字列を返す。
// 該当する字句がなければ空文字列を返す。
// [ 48] Wildcard ::= "*"
//                  | (NCName ":*")
//                  | ("*:" NCName)
//                  | (BracedURILiteral "*")
// [118] BracedURILiteral ::= "Q" "{" [^{}]* "}"
//
fn parse_wildcard(lex: &mut Lexer) -> Result<String, Box<Error>> {

    let mut qname = String::new();

    match lex.next_token().get_type() {
        TType::Asterisk => {
            qname += lex.get_token().get_name();
            return Ok(qname);
        },
        TType::Name => {
            qname += lex.get_token().get_name();
            match lex.next_token().get_type() {
                TType::ColonAsterisk => {
                    qname += lex.get_token().get_name();
                    return Ok(qname);
                },
                _ => {
                    lex.unget_token();
                },
            }
        },
        TType::AsteriskColon => {
            qname += lex.get_token().get_name();
            match lex.next_token().get_type() {
                TType::Name => {
                    qname += lex.get_token().get_name();
                    return Ok(qname);
                },
                _ => {
                    lex.unget_token();
                },
            }
        },
        TType::BracedURILiteral => {
            qname += lex.get_token().get_name();
            match lex.next_token().get_type() {
                TType::Asterisk => {
                    qname += lex.get_token().get_name();
                    return Ok(qname);
                },
                _ => {
                    lex.unget_token();
                },
            }
        },
        _ => {},
    }
    return Ok(String::new());
}

// =====================================================================
// xnode関係の補助函数 (書き替えを伴うもの; 非公開)
//

// ---------------------------------------------------------------------
//
fn new_xnode(n_type: XNodeType, name: &str) -> XNodePtr {
    return XNodePtr{
        xnode_ptr: Rc::new(RefCell::new(XNode{
            n_type: n_type,
            name: String::from(name),
            left: None,
            right: None,
        })),
    };
}

// ---------------------------------------------------------------------
//
fn new_nil_xnode() -> XNodePtr {
    return new_xnode(XNodeType::Nil, "");
}

// ---------------------------------------------------------------------
//
fn assign_as_left(parent: &XNodePtr, left: &XNodePtr) {
    if ! is_nil_xnode(left) {
        parent.xnode_ptr.borrow_mut().left =
                Some(XNodePtr{xnode_ptr: Rc::clone(&left.xnode_ptr)});
    }
}

// ---------------------------------------------------------------------
//
fn assign_as_right(parent: &XNodePtr, right: &XNodePtr) {
    if ! is_nil_xnode(right) {
        parent.xnode_ptr.borrow_mut().right =
                Some(XNodePtr{xnode_ptr: Rc::clone(&right.xnode_ptr)});
    }
}

// =====================================================================
// xnode関係の補助函数 (参照のみおこなうもの; 公開)
//

// ---------------------------------------------------------------------
//
pub fn get_xnode_name(xnode: &XNodePtr) -> String {
    return xnode.xnode_ptr.borrow().name.clone();
}

// ---------------------------------------------------------------------
//
pub fn get_xnode_type(xnode: &XNodePtr) -> XNodeType {
    return xnode.xnode_ptr.borrow().n_type.clone();
}

// ---------------------------------------------------------------------
//
pub fn is_nil_xnode(xnode: &XNodePtr) -> bool {
    return xnode.xnode_ptr.borrow().n_type == XNodeType::Nil;
}

// ---------------------------------------------------------------------
//
pub fn get_left(parent: &XNodePtr) -> XNodePtr {
    match parent.xnode_ptr.borrow().left {
        Some(ref left) => {
            return XNodePtr{
                xnode_ptr: Rc::clone(&left.xnode_ptr),
            };
        },
        None => {
            return new_nil_xnode();
        },
    }
}

// ---------------------------------------------------------------------
//
pub fn get_right(parent: &XNodePtr) -> XNodePtr {
    match parent.xnode_ptr.borrow().right {
        Some(ref right) => {
            return XNodePtr{
                xnode_ptr: Rc::clone(&right.xnode_ptr),
            };
        },
        None => {
            return new_nil_xnode();
        },
    }
}

// =====================================================================
//
#[cfg(test)]
mod test {
//    use super::*;

    use xpath_impl::lexer::*;
    use xpath_impl::parser::compile_xpath;

    // -----------------------------------------------------------------
    //
    #[test]
    fn test_parse() {

//        let xpath = r#" 'aBcDe' => upper-case() => substring(2, 3)"#;
                // substring(upper-case('aBcDe'), 2, 3)

//        let xpath = "let $f := function($a) { $a * $a } return $f(5)";
        let xpath = "let $f := function($a) { $a * $a } return 5 => $f()";




        match Lexer::new(&String::from(xpath)) {
            Ok(lex) => {
                println!("Tokens:\n{}", lex.token_dump());
            },
            Err(e) => {
                println!("Lexer Err: {}", e);
            },
        }

        match compile_xpath(&String::from(xpath)) {
            Ok(xnode) => {
                println!("\n{}", xnode);
            },
            Err(e) => {
                println!("Err: {}", e);
            }
        }
//        assert_eq!("A", "Z");
    }
}

