//
// xpath2/parser.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use xmlerror::*;
use xpath2::lexer::*;
use xpath2::func;
        // func::check_function_spec() を使う。

// =====================================================================
//
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum XNodeType {
    Nil,
    Undef,
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
    OperatorComma,
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
}

impl XNodeType {
    pub fn to_string(&self) -> String {
        let xnode_desc: HashMap<XNodeType, &str> = [
            ( XNodeType::Nil,                  "Nil" ),
            ( XNodeType::Undef,                "Undef" ),
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
            ( XNodeType::OperatorComma,        "OperatorComma" ),
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

    let tok = lex.next_token();
    if tok.get_type() != TType::EOF {
        return Err(xpath_syntax_error!(
            "{}: 余分な字句が継続。", lex.around_tokens().as_str()));
    }

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
//   (NodeTest)
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
            let tok2 = lex.get_token();
            if tok2.get_type() != TType::ColonColon {   // just in case
                panic!("parse_axis_step: TType::AxisNameの次がTType::ColonColonでないのは字句解析器の誤り");
            }
            let axis = match axis_tbl.get(tok.get_name()) {
                Some(a) => a,
                None => {
                    return Err(xpath_syntax_error!(
                        "{}: 軸名として不正。",
                        lex.around_tokens().as_str()));
                },
            };
            if *axis == XNodeType::AxisNamespace {
                return Err(xpath_syntax_error!(
                    "{}: namespace 軸は未実装。",
                    lex.around_tokens().as_str()));
            }
            return parse_axis_step_sub(lex, axis);
        },
        TType::At => {  // 「@」は「attribute::」の省略形
            lex.get_token();
            return parse_axis_step_sub(lex, &XNodeType::AxisAttribute);
        },
        TType::Dot => { // 「.」は「self::node()」の省略形
            lex.get_token();
            return Ok(new_xnode(XNodeType::AxisSelf, "node()"));
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

fn parse_axis_step_sub(lex: &mut Lexer2, axis: &XNodeType) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_node_test(lex)?;

    if ! is_nil_xnode(&xnode) {
        assign_xnode_type(&xnode, axis);
        let predicates_node = parse_predicate_list(
                lex, is_xnode_reverse_axis(&axis))?;
        assign_as_right(&xnode, &predicates_node);
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [35] NodeTest ::= KindTest | NameTest
//
fn parse_node_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_name_test(lex)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    } else {
        return parse_kind_test(lex);
    }
}

// ---------------------------------------------------------------------
// [36] NameTest ::= QName | Wildcard
// [37] Wildcard ::= "*"
//                 | (NCName ":" "*")
//                 | ("*" ":" NCName)
//
// これを、
// NameTest ::= (NCName | "*") ( ":" (NCName | "*"))?
// と考えて解析し、"*:*" を除外する。
//
// nTypeは未定 (XNodeType::Undef) の状態で *XNode を返す。
// 呼び出し元 (parse_axis_step_sub) で軸を判断し、適切に設定することになる。
//
fn parse_name_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let mut name = String::new();
    let mut tok = lex.next_token();
    match tok.get_type() {
        TType::Name | TType::Asterisk => {
            lex.get_token();
            name += tok.get_name();
            tok = lex.next_token();
            if tok.get_type() == TType::Colon {
                lex.get_token();
                name += &":";
                tok = lex.next_token();
                match tok.get_type() {
                    TType::Name | TType::Asterisk => {
                        lex.get_token();
                        name += tok.get_name();
                    },
                    _ => {
                        return Err(xpath_syntax_error!(
                            "{}: 「:」の後には名前または「*」が必要。",
                            lex.around_tokens().as_str()));
                    },
                }
                if name == "*:*" {
                    return Err(xpath_syntax_error!(
                        "{}: 「*:*」という形のNameTestは不可。",
                        lex.around_tokens().as_str()));
                }
            }
            return Ok(new_xnode(XNodeType::Undef, name.as_str()));
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }
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
// [56] DocumentTest ::= "document-node" "(" (ElementTest | SchemaElementTest)? ")"
// [64] ElementTest ::= "element" "(" (ElementNameOrWildcard ("," TypeName "?"?)?)? ")"
// [60] AttributeTest ::= "attribute" "(" (AttribNameOrWildcard ("," TypeName)?)? ")"
// [66] SchemaElementTest ::= "schema-element" "(" ElementDeclaration ")"
// [62] SchemaAttributeTest ::= "schema-attribute" "(" AttributeDeclaration ")"
// [59] PITest ::= "processing-instruction" "(" (NCName | StringLiteral)? ")"
// [58] CommentTest ::= "comment" "(" ")"
// [57] TextTest ::= "text" "(" ")"
// [55] AnyKindTest ::= "node" "(" ")"
//
// [65] ElementNameOrWildcard ::= ElementName | "*"
// [61] AttribNameOrWildcard ::= AttributeName | "*"
// [70] TypeName ::= QName
// [67] ElementDeclaration ::= ElementName
// [63] AttributeDeclaration ::= AttributeName
// [69] ElementName ::= QName
// [68] AttributeName ::= QName
//
//
// 厳密ではないが、次の構文であるものとして解析する。
// TypeNameが出てくる構文は未実装とする。
// NodeTypeは、字句解析の段階でTType::NodeTypeと判定済みになっている。
//
// KindTest ::= NodeType "(" ( QName | * )? ")"
// NodeType ::= "document-node"
//            | "element"
//            | "attribute"
//            | "schema-element"
//            | "schema-attribute"
//            | "processing-instruction"
//            | "comment"
//            | "text"
//            | "node"
//
fn parse_kind_test(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    let tok = lex.next_token();
    if tok.get_type() != TType::NodeType {
        return Ok(new_nil_xnode());
    }
    let node_type_name = tok.get_name();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::LeftParen {
        return Err(xpath_syntax_error!(
                "{}: NodeType: () が必要。",
                lex.around_tokens().as_str()));
    }
    lex.get_token();

    let mut kind_test_arg = String::new();

    let tok = lex.next_token();
    match tok.get_type() {
        TType::RightParen => {
            lex.get_token();
        },
        TType::Asterisk | TType::StringLiteral => {
            lex.get_token();
            kind_test_arg += tok.get_name();
            let tok = lex.next_token();
            if tok.get_type() != TType::RightParen {
                return Err(xpath_syntax_error!(
                    "{}: NodeType: 閉じ括弧が必要。",
                    lex.around_tokens().as_str()));
            }
            lex.get_token();
        },
        _ => {
            return Err(xpath_syntax_error!(
                "{}: NodeType: 閉じ括弧または文字列が必要。",
                lex.around_tokens().as_str()));
        },
    }

    return Ok(new_xnode(XNodeType::Undef,
        &format!("{}({})", node_type_name, kind_test_arg.as_str())));
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
    if is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

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
    let tok = lex.next_token();
    if tok.get_type() != TType::LeftBracket {
        return Ok(new_nil_xnode());
    }
    lex.get_token();

    let xnode = parse_expr(lex)?;

    let tok = lex.get_token();
    if tok.get_type() != TType::RightBracket {
        return Err(xpath_syntax_error!(
                "{}: 述語を閉じる「]」が必要。", lex.around_tokens().as_str())
        );
    }

    return Ok(xnode);
}

// ---------------------------------------------------------------------
// [ 2] Expr ::= ExprSingle ( "," ExprSingle )*
//
//   OperatorComma --- OperatorComma --- nil
//         |                |
//         |             IfExpr ...
//         |              ...
//     OperatorOr --- ...
//        ...
// 3.3.1 Constructing Sequences
// Comma operator: evaluates each of its operands and concatenates
// the resulting sequences, in order, into a single result sequence.
//
fn parse_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let token_node_map: HashMap<TType, XNodeType> = [
        ( TType::Comma, XNodeType::OperatorComma ),
    ].iter().cloned().collect();

    return parse_bin_op_sub(lex, parse_expr_single, &token_node_map, false);
}

// ---------------------------------------------------------------------
// [ 3] ExprSingle ::= ForExpr
//                   | QuantifiedExpr                                   // ☆
//                   | IfExpr
//                   | OrExpr
//
fn parse_expr_single(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_for_expr(lex)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

    let xnode = parse_quantified_expr(lex)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

    let xnode = parse_if_expr(lex)?;
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

    return parse_or_expr(lex);
}

// ---------------------------------------------------------------------
// [ 4] ForExpr ::= SimpleForClause "return" ExprSingle
// [ 5] SimpleForClause ::= "for" "$" VarName "in" ExprSingle
//                              ("," "$" VarName "in" ExprSingle)*
// [45] VarName ::= QName
// 
fn parse_for_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    if tok.get_type() != TType::For {
        return Ok(new_nil_xnode());
    }
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

    let tok = lex.next_token();
    if tok.get_type() != TType::Return {
        return Err(xpath_syntax_error!(
            "{}: for文に return が必要。", lex.around_tokens().as_str()));
    }
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

    let tok = lex.next_token();
    if tok.get_type() != TType::Satisfies {
        return Err(xpath_syntax_error!(
            "{}: some/every文に satisfies が必要。",
            lex.around_tokens().as_str()));
    }
    lex.get_token();

    let xnode_expr_single = parse_expr_single(lex)?;
    assign_as_right(&curr_xnode, &xnode_expr_single);
    
    return Ok(xnode_quantified_expr);
}

// ---------------------------------------------------------------------
// "$" VarName "in" ExprSingle
//
fn parse_var_bind(lex: &mut Lexer2, xnode_type: &XNodeType) -> Result<XNodePtr, Box<Error>> {
    let tok = lex.next_token();
    if tok.get_type() != TType::Dollar {
        return Ok(new_nil_xnode());
    }
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::Name {
        return Err(xpath_syntax_error!(
            "{}: for文は $ の後に変数名が必要。", lex.around_tokens().as_str()));
    }
    let var_name = tok.get_name();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::In {
        return Err(xpath_syntax_error!(
            "{}: for文は変数名の後に in が必要。", lex.around_tokens().as_str()));
    }
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
    let tok = lex.next_token();
    if tok.get_type() != TType::If {
        return Ok(new_nil_xnode());
    }
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::LeftParen {
        return Err(xpath_syntax_error!(
                "{}: if文には左括弧が必要。", lex.around_tokens().as_str()));
    }
    lex.get_token();

    let xnode_cond = parse_expr(lex)?;
    if is_nil_xnode(&xnode_cond) {
        return Err(xpath_syntax_error!(
                "{}: if文の条件式が不正。", lex.around_tokens().as_str()));
    }

    let tok = lex.next_token();
    if tok.get_type() != TType::RightParen {
        return Err(xpath_syntax_error!(
                "{}: 条件式を閉じる右括弧が必要。", lex.around_tokens().as_str()));
    }
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::Then {
        return Err(xpath_syntax_error!(
                "{}: if文にはthen節が必要。", lex.around_tokens().as_str()));
    }
    lex.get_token();

    let xnode_then = parse_expr_single(lex)?;
    if is_nil_xnode(&xnode_then) {
        return Err(xpath_syntax_error!(
                "{}: if文のthen節が不正。", lex.around_tokens().as_str()));
    }

    let tok = lex.next_token();
    if tok.get_type() != TType::Else {
        return Err(xpath_syntax_error!(
                "{}: if文にはelse節が必要。", lex.around_tokens().as_str()));
    }
    lex.get_token();

    let xnode_else = parse_expr_single(lex)?;
    if is_nil_xnode(&xnode_else) {
        return Err(xpath_syntax_error!(
                "{}: if文のelse節が不正。", lex.around_tokens().as_str()));
    }

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
        ( TType::ValueEQ, XNodeType::OperatorValueEQ ),
        ( TType::ValueNE, XNodeType::OperatorValueNE ),
        ( TType::ValueLT, XNodeType::OperatorValueLT ),
        ( TType::ValueGT, XNodeType::OperatorValueGT ),
        ( TType::ValueLE, XNodeType::OperatorValueLE ),
        ( TType::ValueGE, XNodeType::OperatorValueGE ),
        ( TType::GeneralEQ, XNodeType::OperatorGeneralEQ ),
        ( TType::GeneralNE, XNodeType::OperatorGeneralNE ),
        ( TType::GeneralLT, XNodeType::OperatorGeneralLT ),
        ( TType::GeneralGT, XNodeType::OperatorGeneralGT ),
        ( TType::GeneralLE, XNodeType::OperatorGeneralLE ),
        ( TType::GeneralGE, XNodeType::OperatorGeneralGE ),
        ( TType::IsSameNode, XNodeType::OperatorIsSameNode ),
        ( TType::NodeBefore, XNodeType::OperatorNodeBefore ),
        ( TType::NodeAfter, XNodeType::OperatorNodeAfter ),
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
        ( TType::Multiply, XNodeType::OperatorMultiply ),
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
// [16] InstanceofExpr ::= TreatExpr
//                         ( "instance" "of" ) SequenceType )?          // ☆
// [50] SequenceType ::= ("empty-sequence" "(" ")")
//                     | (ItemType OccurrenceIndicator?)
// [51] OccurrenceIndicator ::= "?" | "*" | "+"
// [52] ItemType ::= KindTest 
//                 | ( "item" "(" ")" )
//                 | AtomicType
// [53] AtomicType ::= QName
//
fn parse_instanceof_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_treat_expr(lex);
}

// ---------------------------------------------------------------------
// [17] TreatExpr ::= CastableExpr ( "treat" "as" ) SequenceType )?      // ☆
// [50] SequenceType ::= ("empty-sequence" "(" ")")
//                     | (ItemType OccurrenceIndicator?)
// [51] OccurrenceIndicator ::= "?" | "*" | "+"
// [52] ItemType ::= KindTest 
//                 | ( "item" "(" ")" )
//                 | AtomicType
// [53] AtomicType ::= QName
//
fn parse_treat_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    return parse_castable_expr(lex);
}

// ---------------------------------------------------------------------
// [18] CastableExpr ::= CastExpr ( "castable" "as" ) SingleType )?
// [49] SingleType ::= AtomicType "?"?
// [53] AtomicType ::= QName
//
fn parse_castable_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    return parse_of_atomic_type_op_sub(lex,
            parse_cast_expr, TType::CastableAs, XNodeType::OperatorCastableAs);
}

// ---------------------------------------------------------------------
// [19] CastExpr ::= UnaryExpr ( "cast" "as" ) SingleType )?
// [49] SingleType ::= AtomicType "?"?
// [53] AtomicType ::= QName
//
fn parse_cast_expr(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {

    return parse_of_atomic_type_op_sub(lex,
            parse_unary_expr, TType::CastAs, XNodeType::OperatorCastAs);
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
// 「cast of」、「castable of」演算子を解析
// // 構文の形がほぼ同じ。
// [18] CastableExpr ::= CastExpr  ( "castable" "as" ) SingleType )?
// [19] CastExpr ::=     UnaryExpr ( "cast"     "as" ) SingleType )?
// [49] SingleType ::= AtomicType "?"?
// [53] AtomicType ::= QName
//
fn parse_of_atomic_type_op_sub(lex: &mut Lexer2,
        sub_parser: fn(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>>,
        tok_type: TType, node_type: XNodeType) -> Result<XNodePtr, Box<Error>> {

    let xnode = sub_parser(lex)?;

    let tok = lex.next_token();
    if tok.get_type() != tok_type {         // castable as | cast as
        return Ok(xnode);
    }
    lex.get_token();

    let mut qname = parse_qname(lex)?;
    if qname != "" {
        let tok = lex.next_token();
        if tok.get_type() == TType::Question {
            lex.get_token();
            qname += tok.get_name();
        }
        let xnode_cast = new_xnode(node_type, qname.as_str());
                                    // OperatorCastableAs | OperatorCastAs
        assign_as_left(&xnode_cast, &xnode);
        return Ok(xnode_cast);
    } else {
        return Err(xpath_syntax_error!(
                    "{}: キャストする型の名前が必要。",
                    lex.around_tokens().as_str()));
    }
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
            let root_xnode = new_xnode(XNodeType::AxisRoot, "/");
            assign_as_left(&op_path_xnode, &root_xnode);

            let rel_node = parse_relative_path_expr(lex)?;
            if ! is_nil_xnode(&rel_node) {
                assign_as_right(&op_path_xnode, &rel_node);
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

            let rel_node = parse_relative_path_expr(lex)?;
            if ! is_nil_xnode(&rel_node) {
                assign_as_right(&op_path_xnode_l, &rel_node);
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
    let top_op_path_xnode = new_xnode(XNodeType::OperatorPath, "op_path");
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
    if ! is_nil_xnode(&xnode) {
        return Ok(xnode);
    }

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
    if is_nil_xnode(&xnode) {
        return Ok(new_nil_xnode());
    }

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
    // [XNodeApplyPredicates] -- XNodePredTop --...
    //           |
    //     (PrimaryExpr) --- (右辺値)...
    //           |
    //       (左辺値)...
    //
    if ! is_xnode_axis(&get_xnode_type(&xnode)) {
        let xnode_preds = parse_predicate_list(lex, false)?;
        if ! is_nil_xnode(&xnode_preds) {
            let nop_node = new_xnode(XNodeType::ApplyPredicates, "node()");
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
            let tok2 = lex.next_token();
            if tok2.get_type() == TType::RightParen {
                lex.get_token();
                if ! is_nil_xnode(&xnode) {
                    return Ok(xnode);
                } else {
                    return Ok(new_xnode(XNodeType::OperatorPath, "op_path"));
                }
            } else {
                return Err(xpath_syntax_error!(
                    "{}: 左括弧に対応する右括弧が必要。",
                    lex.around_tokens().as_str()));
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
            return Ok(new_xnode(XNodeType::AxisSelf, "node()"));
//            return Ok(new_xnode(XNodeType::ContextItem, "."));
                // 実際には、AxisSelfの意味の場合と、
                // 原子値 (as in the expression (1 to 100) [. mod 5 eq 0]) を
                // 表す場合がある。
        },
        TType::FunctionName => {
            return parse_function_call(lex);
        },
        _ => {
            return Ok(new_nil_xnode());
        },
    }
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
    let tok = lex.get_token();
    if tok.get_type() != TType::FunctionName { // just in case
        panic!("parseFunctionCall: 次の字句がTokenFunctionNameでないのは字句解析器の誤り");
    }
    let func_name = tok.get_name();

    let tok = lex.get_token();
    if tok.get_type() != TType::LeftParen { // just in case
        panic!("parseFunctionCall: TokenFunctionNameの次がTokenLeftParenでないのは字句解析器の誤り");
    }

    let arg_node = parse_argument_array(lex)?;
    let tok = lex.get_token();
    if tok.get_type() != TType::RightParen {
        return Err(xpath_syntax_error!(
                "{}: 函数の引数並びを閉じる右括弧が欠落。",
                lex.around_tokens().as_str()));
    }

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
    if func::check_function_spec(func_name, num_args) == false {
        return Err(xpath_syntax_error!(
                "{}: 函数が未実装、または引数の数が不適切。", func_name));
    }

    // -------------------------------------------------------------
    //
    let func_node = new_xnode(XNodeType::FunctionCall, func_name);
    assign_as_right(&func_node, &arg_node);

    return Ok(func_node);
}

// ---------------------------------------------------------------------
// [48] FunctionCall ::= QName "(" (ExprSingle ("," ExprSingle)*)? ")"
//
fn parse_argument_array(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_argument(lex)?;
    let mut curr = Rc::clone(&xnode);
    loop {
        let tok = lex.next_token();
        if tok.get_type() == TType::Comma {
            lex.get_token();
            let next_node = parse_argument(lex)?;
            assign_as_right(&curr, &next_node);
            curr = Rc::clone(&next_node);
        } else {
            break;
        }
    }
    return Ok(xnode);
}

// ---------------------------------------------------------------------
// Argument ::= Expr
//
fn parse_argument(lex: &mut Lexer2) -> Result<XNodePtr, Box<Error>> {
    let xnode = parse_expr_single(lex)?;

    if ! is_nil_xnode(&xnode) {
        let xnode_top = new_xnode(XNodeType::ArgumentTop, "");
        assign_as_left(&xnode_top, &xnode);
        return Ok(xnode_top);
    } else {
        return Ok(new_nil_xnode());
    }
}

// ---------------------------------------------------------------------
// (Lexerの) 現在位置以降にQNameと解析できる字句があれば、その文字列を返す。
// 該当する字句がなければ空文字列を返す。
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
        return Ok(qname);
    }
    qname += tok.get_name();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::Colon {
        return Ok(qname);
    }
    qname += tok.get_name();
    lex.get_token();

    let tok = lex.next_token();
    if tok.get_type() != TType::Name {
        return Err(xpath_syntax_error!("{}: コロンの後には名前が必要", 
                                lex.around_tokens().as_str()));
    }
    qname += tok.get_name();
    lex.get_token();

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
fn assign_xnode_type(xnode: &XNodePtr, n_type: &XNodeType) {
    xnode.borrow_mut().n_type = n_type.clone();
}

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

    use xpath2::lexer::*;
    use xpath2::eval::xnode_dump;
    use xpath2::parser::compile_xpath;

    // -----------------------------------------------------------------
    //
    #[test]
    fn test_parse() {

//        let xpath = r#"for $x in 1 to 2, $y in 3 to 4 return $x + $y"#;
//        let xpath = r#"every $x in 1 to 2, $y in 3 to 4 satisfies $x + $y"#;
        let xpath = "sum(())";

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

