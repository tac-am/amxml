//
// xpath_impl/lexer.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use std::collections::HashMap;
use std::error::Error;

use xmlerror::*;

// =====================================================================
//
const EOF: char = '\u{0000}';

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum TType {
    EOF,
    InnerName,
    // specialTokenRule() を適用する前で、
    // Name、NodeType、AxisName のどれになるか未確定の状態
    Nop,
    // 2語から成るトークンの2語め
    Name,
    NodeType,
    AxisName,
    SlashSlash,
    Slash,
    DotDot,
    Dot,
    ColonColon,
    Colon,
    ValueEQ,
    ValueNE,
    ValueGT,
    ValueGE,
    ValueLT,
    ValueLE,
    GeneralEQ,
    GeneralNE,
    GeneralGT,
    GeneralGE,
    GeneralLT,
    GeneralLE,
    IsSameNode,
    NodeBefore,
    NodeAfter,
    And,
    Or,
    Union,
    Intersect,
    Except,
    To,
    InstanceOf,
    TreatAs,
    CastableAs,
    CastAs,
    Plus,
    Minus,
    Div,
    IDiv,
    Mod,
    If,
    For,
    Some,
    Every,
//    Then,
//    Else,
//    In,
//    Return,
//    Satisfies,
//              以上5つは、if/for/some/every構文の、特定の箇所にのみ現れる。
//              字句解析器ではトークン種別を確定できないので、
//              TType::Nameとして返し、構文解析器での判定に委ねる。
//              「for $a in ... return ($a, ...)」のようにシーケンスを返す
//              記述の場合、函数名と区別ができない。
//              一方、for構文以外の箇所に「return (...)」とあれば、
//              函数名として扱う必要がある。
//
    Asterisk,
    Dollar,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    At,
    Question,
    Comma,
    IntegerLiteral,
    DecimalLiteral,
    DoubleLiteral,
    StringLiteral,
    EmptySequence,
    Item,
    TypeSwitch,
    DocumentTest,
    ElementTest,
    AttributeTest,
    SchemaElementTest,
    SchemaAttributeTest,
    PITest,
    CommentTest,
    TextTest,
    AnyKindTest,
}

// =====================================================================
//
#[derive(Debug, Clone)]
pub struct Token {
    t_type: TType,
    name: String,
}

impl Token {
    pub fn get_type(&self) -> TType {
        return self.t_type.clone();
    }
    pub fn get_name(&self) -> &str {
        return self.name.as_str();
    }
}

// =====================================================================
//
#[derive(Debug)]
pub struct Lexer2 {
    char_vec: Vec<char>,
    tokens: Vec<Token>,
    index: usize,
}

// =====================================================================
/// Lexer2: 
// 字句解析器
// // 初めに末尾まで読んでトークンに分解し、トークン型を調べるように実装。
impl Lexer2 {

    // -----------------------------------------------------------------
    //
    #[allow(dead_code)]
    pub fn token_dump(&self) -> String {
        let ttype_desc: HashMap<TType, &str> = [
            ( TType::EOF,                 "EOF" ),
            ( TType::InnerName,           "InnerName" ),
            ( TType::Nop,                 "Nop" ),
            ( TType::Name,                "Name" ),
            ( TType::NodeType,            "NodeType" ),
            ( TType::AxisName,            "AxisName" ),
            ( TType::SlashSlash,          "SlashSlash" ),
            ( TType::Slash,               "Slash" ),
            ( TType::DotDot,              "DotDot" ),
            ( TType::Dot,                 "Dot" ),
            ( TType::ColonColon,          "ColonColon" ),
            ( TType::Colon,               "Colon" ),
            ( TType::ValueEQ,             "ValueEQ" ),
            ( TType::ValueNE,             "ValueNE" ),
            ( TType::ValueGT,             "ValueGT" ),
            ( TType::ValueGE,             "ValueGE" ),
            ( TType::ValueLT,             "ValueLT" ),
            ( TType::ValueLE,             "ValueLE" ),
            ( TType::GeneralEQ,           "GeneralEQ" ),
            ( TType::GeneralNE,           "GeneralNE" ),
            ( TType::GeneralGT,           "GeneralGT" ),
            ( TType::GeneralGE,           "GeneralGE" ),
            ( TType::GeneralLT,           "GeneralLT" ),
            ( TType::GeneralLE,           "GeneralLE" ),
            ( TType::IsSameNode,          "IsSameNode" ),
            ( TType::NodeBefore,          "NodeBefore" ),
            ( TType::NodeAfter,           "NodeAfter" ),
            ( TType::And,                 "And" ),
            ( TType::Or,                  "Or" ),
            ( TType::Union,               "Union" ),
            ( TType::Intersect,           "Intersect" ),
            ( TType::Except,              "Except" ),
            ( TType::To,                  "To" ),
            ( TType::InstanceOf,          "InstanceOf" ),
            ( TType::TreatAs,             "TreatAs" ),
            ( TType::CastableAs,          "CastableAs" ),
            ( TType::CastAs,              "CastAs" ),
            ( TType::Plus,                "Plus" ),
            ( TType::Minus,               "Minus" ),
            ( TType::Div,                 "Div" ),
            ( TType::IDiv,                "IDiv" ),
            ( TType::Mod,                 "Mod" ),
            ( TType::If,                  "If" ),
            ( TType::For,                 "For" ),
            ( TType::Some,                "Some" ),
            ( TType::Every,               "Every" ),
//            ( TType::Then,                "Then" ),
//            ( TType::Else,                "Else" ),
//            ( TType::In,                  "In" ),
//            ( TType::Return,              "Return" ),
//            ( TType::Satisfies,           "Satisfies" ),
            ( TType::Asterisk,            "Asterisk" ),
            ( TType::Dollar,              "Dollar" ),
            ( TType::LeftBracket,         "LeftBracket" ),
            ( TType::RightBracket,        "RightBracket" ),
            ( TType::LeftParen,           "LeftParen" ),
            ( TType::RightParen,          "RightParen" ),
            ( TType::At,                  "At" ),
            ( TType::Question,            "Question" ),
            ( TType::Comma,               "Comma" ),
            ( TType::IntegerLiteral,      "IntegerLiteral" ),
            ( TType::DecimalLiteral,      "DecimalLiteral" ),
            ( TType::DoubleLiteral,       "DoubleLiteral" ),
            ( TType::StringLiteral,       "StringLiteral" ),
            ( TType::EmptySequence,       "EmptySequence" ),
            ( TType::Item,                "Item" ),
            ( TType::TypeSwitch,          "TypeSwitch" ),
            ( TType::DocumentTest,        "DocumentTest" ),
            ( TType::ElementTest,         "ElementTest" ),
            ( TType::AttributeTest,       "AttributeTest" ),
            ( TType::SchemaElementTest,   "SchemaElementTest" ),
            ( TType::SchemaAttributeTest, "SchemaAttributeTest" ),
            ( TType::PITest,              "PITest" ),
            ( TType::CommentTest,         "CommentTest" ),
            ( TType::TextTest,            "TextTest" ),
            ( TType::AnyKindTest,         "AnyKindTest" ),
        ].iter().cloned().collect();

        let mut s = String::new();
        for token in self.tokens.iter() {
            s += &format!("[{}] {}\n", 
                ttype_desc.get(&token.t_type).unwrap_or(&"UNKNOWN").to_string(),
                token.name);
        }
        return s;
    }

    // -----------------------------------------------------------------
    //
    pub fn next_token(&self) -> Token {
        return self.tokens[self.index].clone();
    }

    // -----------------------------------------------------------------
    //
    pub fn get_token(&mut self) -> Token {
        let tok = self.tokens[self.index].clone();
        if self.index < self.tokens.len() - 1 {
            self.index += 1;
        }
        return tok;
    }

    // -----------------------------------------------------------------
    //
    pub fn unget_token(&mut self) {
        if 0 < self.index {
            self.index -= 1;
        }
    }

    // -----------------------------------------------------------------
    //
    pub fn around_tokens(&self) -> String {
        let min_index = if self.index <= 3 { 1 } else { self.index - 3 };
        let max_index = (self.tokens.len() - 1).min(self.index + 3);

        let mut s = String::new();
        for i in min_index .. max_index {
            if i == self.index - 1 {
                s += &format!("≪{}≫", self.tokens[i].name);
            } else {
                s += &format!(" {} ", self.tokens[i].name);
            }
        }
        return s;
    }

    // -----------------------------------------------------------------
    //
    fn push_token(&mut self, ttype: TType, name: &str) {
        self.tokens.push(Token{
            t_type: ttype,
            name: String::from(name),
        });
    }

    // -----------------------------------------------------------------
    //
    pub fn new(xpath_string: &String) -> Result<Lexer2, Box<Error>> {
        let mut lexer = Lexer2 {
            char_vec: xpath_string.chars().collect(),
            tokens: vec!{},
            index: 0,
        };

        lexer.push_token(TType::EOF, "");
        lexer.collect_tokens()?;

        lexer.index = 1;

        let mut i = 1;
        while lexer.tokens[i].t_type != TType::EOF {
            match lexer.tokens[i].t_type {
                TType::InnerName => {
                    lexer.tokens[i].t_type = special_token_rule(
                        &lexer.tokens[i], &lexer.tokens[i-1], &lexer.tokens[i+1]);
                },
                _ => {},
            }
            i += 1;
        }

        let mut i = lexer.tokens.len() - 1;
        while 0 < i {
            if lexer.tokens[i].t_type == TType::Nop {
                lexer.tokens.remove(i as usize);
            }
            i -= 1;
        }
        return Ok(lexer);
    }

    // -----------------------------------------------------------------
    //
    fn collect_tokens(&mut self) -> Result<(), Box<Error>> {
        loop {
            self.skip_spaces();
            let ch1 = self.read_rune();

            if is_eof(ch1) {
                self.push_token(TType::EOF, "");
                return Ok(());

            } else if is_name_start_char(ch1) {
                let mut name = String::new();
                name.push(ch1);
                loop {
                    let ch2 = self.read_rune();
                    if ! is_name_char(ch2) {
                        break;
                    }
                    name.push(ch2);
                }
                self.unread_rune();
                self.push_token(TType::InnerName, &name);

            } else if ch1 == '"' || ch1 == '\'' {
                let literal = self.fetch_string_literal(&ch1)?;
                self.push_token(TType::StringLiteral, &literal);

            } else if is_digit(ch1) {
                self.unread_rune();
                let literal = &self.fetch_numeric_literal()?;
                if literal.contains("e") || literal.contains("E") {
                    self.push_token(TType::DoubleLiteral, literal);
                } else if literal.contains(".") {
                    self.push_token(TType::DecimalLiteral, literal);
                } else {
                    self.push_token(TType::IntegerLiteral, literal);
                }

            } else if ch1 == '.' {
                let ch2 = self.read_rune();
                if is_digit(ch2) {
                    self.unread_rune();
                    self.unread_rune();
                    let literal = &self.fetch_numeric_literal()?;
                    if literal.contains("e") || literal.contains("E") {
                        self.push_token(TType::DoubleLiteral, literal);
                    } else if literal.contains(".") {
                        self.push_token(TType::DecimalLiteral, literal);
                    } else {
                        self.push_token(TType::IntegerLiteral, literal);
                    }
                } else if ch2 == '.' {
                    self.push_token(TType::DotDot, "..");
                } else {
                    self.unread_rune();
                    self.push_token(TType::Dot, ".");
                }

            } else if ch1 == '(' {
                let ch2 = self.read_rune();
                if ch2 == ':' {                 // "(:" - Comment
                    self.skip_comment()?;
                } else {
                    self.unread_rune();
                    self.push_token(TType::LeftParen, "(");
                }

            } else {
                let keywords_spec = [
                    ( "//", TType::SlashSlash ),
                    ( "/",  TType::Slash ),
                    ( "::", TType::ColonColon ),
                    ( ":",  TType::Colon ),
                    ( "=",  TType::GeneralEQ ),
                    ( "!=", TType::GeneralNE ),
                    ( "|",  TType::Union ),
                    ( "<=", TType::GeneralLE ),
                    ( "<<", TType::NodeBefore ),
                    ( "<",  TType::GeneralLT ),
                    ( ">=", TType::GeneralGE ),
                    ( ">>", TType::NodeAfter ),
                    ( ">",  TType::GeneralGT ),
                    ( ",",  TType::Comma ),
                    ( "?",  TType::Question ),
                    ( "+",  TType::Plus ),
                    ( "-",  TType::Minus ),
                    ( "*",  TType::Asterisk ),
                    ( "$",  TType::Dollar ),
                    ( "[",  TType::LeftBracket ),
                    ( "]",  TType::RightBracket ),
                    ( ")",  TType::RightParen ),
                    ( "@",  TType::At ),
                ];
                let mut found = false;
                self.unread_rune();
                for (keyword, ttype) in keywords_spec.iter() {
                    if self.look_ahead_keyword(keyword) == true {
                        self.push_token(ttype.clone(), keyword);
                        found = true;
                        break;
                    }
                }

                if ! found {
                    return Err(xpath_syntax_error!(
                        "XPathを構成する字句として認識できない文字: {}", ch1));
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // 現在位置以降に keyword と一致する文字列が続いている場合は、
    // その末尾位置まで読み進めて true を返す。
    // そうでなければ現在位置に戻り、false を返す。
    //
    fn look_ahead_keyword(&mut self, keyword: &str) -> bool {
        let keyword_vec: Vec<char> = keyword.chars().collect();
        for (i, key_ch) in keyword_vec.iter().enumerate() {
            let ch = self.read_rune();
            if ch != *key_ch {
                for _ in 0 ..= i {
                    self.unread_rune();
                }
                return false;
            }
        }
        return true;
    }

    // -----------------------------------------------------------------
    // 数値リテラルを取得する。
    // [43] NumericLiteral ::= IntegerLiteral | DecimalLiteral | DoubleLiteral
    // [71] IntegerLiteral ::= Digits
    // [72] DecimalLiteral ::= ("." Digits) | (Digits "." [0-9]*)
    // [73] DoubleLiteral  ::= (("." Digits) | (Digits ("." [0-9]*)?)) [eE] [+-]? Digits
    // [81] Digits ::= [0-9]+
    //
    fn fetch_numeric_literal(&mut self) -> Result<String, Box<Error>> {
        let mut numeric_literal = String::new();

        let mut ch1 = self.read_rune();
        if is_digit(ch1) {
            numeric_literal.push(ch1);
            numeric_literal.push_str(&self.fetch_digits());

            ch1 = self.read_rune();
            if ch1 == '.' {
                self.unread_rune();
                numeric_literal.push_str(&self.fetch_numeric_after_period()?);
            } else {
                self.unread_rune();
            }
            numeric_literal.push_str(&self.fetch_numeric_after_e()?);
            return Ok(numeric_literal);
        } else if ch1 == '.' {
            self.unread_rune();
            return self.fetch_numeric_after_period();
        } else {
            self.unread_rune();
            return Ok(numeric_literal);
        }
    }

    // -----------------------------------------------------------------
    // 次の文字が '.' であれば、
    //      "." [0-9]* ([eE] [+-]? [0-9]+)?
    // という部分を取得する。
    // そうでなければ空を返す。
    //
    fn fetch_numeric_after_period(&mut self) -> Result<String, Box<Error>> {
        let mut numeric_after_period = String::new();
        let ch1 = self.read_rune();
        if ch1 == '.' {
            numeric_after_period.push(ch1);
            numeric_after_period.push_str(&self.fetch_digits());
            numeric_after_period.push_str(&self.fetch_numeric_after_e()?);
        }
        return Ok(numeric_after_period);
    }

    // -----------------------------------------------------------------
    // 次の文字が 'e' または 'E' であれば、
    //      [eE] [+-]? [0-9]+
    // という部分を取得する。そうでなければ空を返す。
    //
    fn fetch_numeric_after_e(&mut self) -> Result<String, Box<Error>> {
        let mut numeric_after_e = String::new();
        let mut ch1 = self.read_rune();
        if ch1 == 'e' || ch1 == 'E' {
            numeric_after_e.push(ch1);
            ch1 = self.read_rune();
            if ch1 == '+' || ch1 == '-' {
                numeric_after_e.push(ch1);
            } else if is_digit(ch1) {
                self.unread_rune();
            } else {
                return Err(xpath_syntax_error!(
                        "指数を表す [eE] の後には数字が必要。"));
            }
            numeric_after_e.push_str(&self.fetch_digits());
        } else {
            self.unread_rune();
        }
        return Ok(numeric_after_e);
    }

    // -----------------------------------------------------------------
    // 数字で始まる、
    //      [0-9]*
    // という部分を取得する。
    //
    fn fetch_digits(&mut self) -> String {
        let mut digits = String::new();
        loop {
            let ch1 = self.read_rune();
            if is_digit(ch1) {
                digits.push(ch1);
            } else {
                self.unread_rune();
                return digits;
            }
        }
    }

    // -----------------------------------------------------------------
    // 文字列リテラルを取得する。
    // [74] StringLiteral ::= ('"' (EscapeQuot | [^"])* '"')
    //                      | ("'" (EscapeApos | [^'])* "'")
    // [75] EscapeQuot ::= '""'
    // [76] EscapeApos ::= "''"
    //
    fn fetch_string_literal(&mut self, delim: &char) -> Result<String, Box<Error>> {
        let mut string_literal = String::new();
        loop {
            let ch1 = self.read_rune();
            if is_eof(ch1) {
                return Err(xpath_syntax_error!("Unexpected EOF while scanning string literal."));
            } else if ch1 == *delim {
                let ch2 = self.read_rune();
                if ch2 == *delim {
                    string_literal.push(ch2);
                } else {
                    self.unread_rune();
                    return Ok(string_literal);
                }
            } else {
                string_literal.push(ch1);
            }
        }
    }

    // -----------------------------------------------------------------
    // 註釈を読み飛ばす。
    // [77] Comment ::= "(:" (CommentContents | Comment)* ":)"
    // [82] CommentContents ::= (Char+ - (Char* ('(:' | ':)') Char*))
    //
    fn skip_comment(&mut self) -> Result<(), Box<Error>> {
        let mut nest_level = 1;
        while 0 < nest_level {
            let ch1 = self.read_rune();
            if is_eof(ch1) {
                return Err(xpath_syntax_error!("Unexpected EOF while scanning comment."));
            } else if ch1 == '(' {
                let ch2 = self.read_rune();
                if ch2 == ':' {
                    nest_level += 1;
                } else {
                    self.unread_rune();
                }
            } else if ch1 == ':' {
                let ch2 = self.read_rune();
                if ch2 == ')' {
                    nest_level -= 1;
                } else {
                    self.unread_rune();
                }
            } else {
                // CommentContentsとして読み飛ばす。
            }
        }
        return Ok(());
    }

    // -----------------------------------------------------------------
    //
    fn skip_spaces(&mut self) {
        loop {
            let ch = self.read_rune();
            if is_eof(ch) {
                return;
            } else if ! is_space(ch) {
                self.unread_rune();
                return;
            }
        }
    }

    // -----------------------------------------------------------------
    // 文字を読む。
    //
    fn read_rune(&mut self) -> char {
        self.index += 1;
        if self.char_vec.len() <= self.index - 1 {
            return EOF;
        } else {
            return self.char_vec[self.index - 1];
        }
    }

    // -----------------------------------------------------------------
    // 文字を読み戻す。
    //
    fn unread_rune(&mut self) {
        if 0 < self.index {
            self.index -= 1;
        }
    }
}

// -----------------------------------------------------------------
// 特別なトークン規則: Name、Asteriskは、次の規則により、他の型の
//                     トークンに読み替える。
// (1) 前にトークンがあり、そのトークンが
//             @ :: ( [ , and or div mod * / // | + - = != < <= > >=    (☆)
//     のいずれでもない場合、
//       "*" は乗算演算子とする。
//       "and" "or" "div" "mod" は演算子名とする。
// (1x) 追加:
//       idiv eq ne lt le gt ge is to union intersect except 
//       // then else in return satisfies は、構文解析器での判定に委ねる。
// (1.註) 規格には明示的に書いてない (字句構造規則なので) が、
//        ☆にはコロン (:) も加える必要がある。
//        そうでないと、「@ns:*」のようなXPathで、規則
//           NameTest ::= NCName ':' '*'
//        に現れる「*」が乗算演算子になってしまう。
// (2) Nameについて、その次のトークンが '(' のとき、
//     所定の字句 (例: 「node()」) であればそれに応じた
//     トークン (AnyKindTestなど)、
//     そうでなければNameとする。
// (3) Nameについて、その次のトークンが '::' であれば、軸名とする。
// (4) Name 'for' は、その次のトークンが '$' であればFor、
//     Name 'some' は、その次のトークンが '$' であればSome、
//     Name 'every' は、その次のトークンが '$' であればEveryとする
//
fn special_token_rule(tok: &Token, prev_tok: &Token, next_tok: &Token) -> TType {
    // -------------------------------------------------------------
    // 特別なトークン規則 (1)
    //
    let node_type_prev = [
        TType::EOF,             // 前にトークンがない場合はこの状態
        TType::At,
        TType::ColonColon,
        TType::LeftParen,
        TType::LeftBracket,
        TType::Comma,
        TType::And,
        TType::Or,
        TType::Div,
        TType::IDiv,
        TType::Mod,
        TType::Slash,
        TType::SlashSlash,
        TType::Union,
        TType::Intersect,
        TType::Except,
        TType::InstanceOf,
        TType::TreatAs,
        TType::CastableAs,
        TType::CastAs,
        TType::Plus,
        TType::Minus,
        TType::ValueEQ,
        TType::ValueNE,
        TType::ValueGT,
        TType::ValueGE,
        TType::ValueLT,
        TType::ValueLE,
        TType::GeneralEQ,
        TType::GeneralNE,
        TType::GeneralGT,
        TType::GeneralGE,
        TType::GeneralLT,
        TType::GeneralLE,
        TType::IsSameNode,
        TType::To,
        TType::NodeBefore,
        TType::NodeAfter,
        TType::Asterisk,
        TType::Colon,            // (1.註)
    ];

    let operator_words: HashMap<&str, TType> = [
        ( "and",       TType::And ),
        ( "or",        TType::Or ),
        ( "div",       TType::Div ),
        ( "mod",       TType::Mod ),
        ( "idiv",      TType::IDiv ),
        ( "eq",        TType::ValueEQ ),
        ( "ne",        TType::ValueNE ),
        ( "lt",        TType::ValueLT ),
        ( "le",        TType::ValueLE ),
        ( "gt",        TType::ValueGT ),
        ( "ge",        TType::ValueGE ),
        ( "is",        TType::IsSameNode ),
        ( "to",        TType::To ),
        ( "union",     TType::Union ),
        ( "intersect", TType::Intersect ),
        ( "except",    TType::Except ),
    ].iter().cloned().collect();

    if tok.t_type == TType::InnerName {
        if ! node_type_prev.contains(&prev_tok.t_type) {
            if let Some(ttype) = operator_words.get(&tok.name.as_str()) {
                return ttype.clone();
            }
        }
    }

    // -------------------------------------------------------------
    // 2つのトークンが組になる場合の規則
    //
    let operator_pair_words: [(&str, &str, TType); 4] = [
        ( "instance", "of", TType::InstanceOf ),
        ( "treat",    "as", TType::TreatAs ),
        ( "castable", "as", TType::CastableAs ),
        ( "cast",     "as", TType::CastAs ),
    ];
    for (str1, str2, t_type) in operator_pair_words.iter() {
        if tok.name.as_str() == *str1 &&
           next_tok.name.as_str() == *str2 {
            return t_type.clone();
        }
        if tok.name.as_str() == *str2 {
            return TType::Nop;
        }
    }

    // -------------------------------------------------------------
    // 特別なトークン規則 (2)(3)(4)
    //
    let token_words_p: HashMap<&str, TType> = [
        ( "attribute",              TType::AttributeTest ),
        ( "comment",                TType::CommentTest ),
        ( "document-node",          TType::DocumentTest ),
        ( "element",                TType::ElementTest ),
        ( "empty-sequence",         TType::EmptySequence ),
        ( "if",                     TType::If ),
        ( "item",                   TType::Item ),
        ( "node",                   TType::AnyKindTest ),
        ( "processing-instruction", TType::PITest ),
        ( "schema-attribute",       TType::SchemaAttributeTest ),
        ( "schema-element",         TType::SchemaElementTest ),
        ( "text",                   TType::TextTest ),
        ( "typeswitch",             TType::TypeSwitch ),
                    // 以上は A.3 Reserved Function Names に載っており、
                    // 函数名としては使えないことになっている。
                    // "typeswitch" は、XPathでは使わないが、XQueryとの
                    // 互換性のため、予約語になっている。
    ].iter().cloned().collect();
    let token_words_d: HashMap<&str, TType> = [
        ( "for",       TType::For ),
        ( "some",      TType::Some ),
        ( "every",     TType::Every ),
    ].iter().cloned().collect();

    if tok.t_type == TType::InnerName {
        match next_tok.t_type {
            TType::LeftParen => {       // (2)
                if let Some(ttype) = token_words_p.get(&tok.name.as_str()) {
                    return ttype.clone();
                } else {
                    return TType::Name;
                }
            },
            TType::ColonColon => {      // (3)
                return TType::AxisName;
            },
            TType::Dollar => {       // (4)
                if let Some(ttype) = token_words_d.get(&tok.name.as_str()) {
                    return ttype.clone();
                }
            },
            _ => {
                return TType::Name;
            },
        }
    }

    return TType::Name;
}

// =====================================================================
// 空白
//
fn is_space(ch: char) -> bool {
    return [ ' ', '\t', '\r', '\n' ].contains(&ch);
}

// ---------------------------------------------------------------------
//
fn is_digit(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x0030, 0x0039 ), // [0-9]
    ]);
}

// ---------------------------------------------------------------------
// 「名前」の先頭に使える文字。
//
fn is_name_start_char(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x0041, 0x005A ), // [A-Z]
        ( 0x005F, 0x005F ), // "_"
        ( 0x0061, 0x007A ), // [a-z]
        ( 0x00C0, 0x00D6 ),
        ( 0x00D8, 0x00F6 ),
        ( 0x00F8, 0x00FF ), // ここまで、Hi <= 00FF
        ( 0x0100, 0x02FF ),
        ( 0x0370, 0x037D ),
        ( 0x037F, 0x1FFF ),
        ( 0x200C, 0x200D ),
        ( 0x2070, 0x218F ),
        ( 0x2C00, 0x2FEF ),
        ( 0x3001, 0xD7FF ),
        ( 0xF900, 0xFDCF ),
        ( 0xFDF0, 0xFFFD ),
        ( 0x00010000, 0x000EFFFF ),
    ]);
}

// ---------------------------------------------------------------------
// 「名前」の2文字め以降を構成する文字。
//
fn is_name_char(ch: char) -> bool {
    return is_name_start_char(ch) ||
        char_is_in_ranges(ch, &[
            ( 0x002D, 0x002E ), // "-", "."
            ( 0x0030, 0x0039 ), // [0-9]
            ( 0x00B7, 0x00B7 ), // "·"
            ( 0x0300, 0x036F ), //
            ( 0x203F, 0x2040 ), //
        ]);
}

// =====================================================================
//
fn char_is_in_ranges(ch: char, ch_ranges: &[(u32, u32)]) -> bool {
    let w = ch as u32;
    for ch_ran in ch_ranges.iter() {
        if ch_ran.0 <= w && w <= ch_ran.1 {
            return true;
        }
    }
    return false;
}

// =====================================================================
//
fn is_eof(ch: char) -> bool {
    return ch == EOF;
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
    // Comment 構文
    //
    #[test]
    fn test_comment() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base" img="base"/>
</root>
        "#);

        subtest_xpath("comment", &xml, false, &[
            ( ".", "base" ),
            ( "(: aa (: あ :) aa :) . ", "base" ),
            ( "(: aa (: : :) aa :). ", "base" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 文字列リテラル (エスケープ表現)
    //
    #[test]
    fn test_string_literal() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base" img="base"/>
    <chap id='Spring"' img="春"/>
    <chap id="Summer'" img="夏"/>
</root>
        "#);

        subtest_xpath("string_literal", &xml, false, &[
            ( r#"//chap[@id = "Spring"""]"#, "春" ),
            ( r#"//chap[@id = 'Summer''']"#, "夏" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 数字リテラル
    //
    #[test]
    fn test_numeric_literal() {
        let xml = compress_spaces(r#"
<root>
</root>
        "#);

        subtest_eval_xpath("numeric_literal", &xml, &[
            ( "107", "(107)" ),
            ( "107.03", "(107.03)" ),
            ( "-107.03", "(-107.03)" ),
            ( ".5", "(0.5)" ),
            ( "-.5", "(-0.5)" ),
            ( "1.07e2", "(1.07e2)" ),
            ( "-1.07e2", "(-1.07e2)" ),
            ( "10.7e1", "(1.07e2)" ),
            ( "10.7E1", "(1.07e2)" ),
        ]);
    }

}

