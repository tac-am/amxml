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
    // 特別なトークン規則を適用する前で、最終的なトークン種が未確定の状態
    Nop,
    // 無効なトークン
    Name,
//    NodeType,
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
    Switch,
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
    BracedURILiteral,
    OperatorConcat,
    Sharp,
    Bind,
    Arrow,
    LeftCurly,
    RightCurly,
    ColonAsterisk,
    AsteriskColon,
    OperatorMap,
    Let,
    Array,
    Map,
    Function,
}

// =====================================================================
//
#[derive(Debug, Clone)]
pub struct Token {
    t_type: TType,
    name: String,
}

fn new_token(t_type: TType, name: &str) -> Token {
    return Token {
        t_type: t_type,
        name: String::from(name),
    };
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
pub struct Lexer {
    char_vec: Vec<char>,
    ch_index: usize,
    tokens: Vec<Token>,
    index: usize,
    mark_index: usize,
}

// =====================================================================
/// Lexer: 
// 字句解析器
// // 初めに末尾まで読んでトークンに分解し、トークン型を調べるように実装。
impl Lexer {

    // -----------------------------------------------------------------
    //
    #[allow(dead_code)]
    pub fn token_dump(&self) -> String {
        let mut s = String::new();
        for token in self.tokens.iter() {
            s += &format!("[{:?}] {}\n", token.t_type, token.name);
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
    pub fn mark_token_index(&mut self) {
        self.mark_index = self.index;
    }

    // -----------------------------------------------------------------
    //
    pub fn restore_marked_index(&mut self) {
        self.index = self.mark_index;
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
    pub fn new(xpath_string: &String) -> Result<Lexer, Box<Error>> {
        let mut lexer = Lexer {
            char_vec: xpath_string.chars().collect(),
            ch_index: 0,
            tokens: vec!{},
            index: 1,
            mark_index: 1,
        };

        // -------------------------------------------------------------
        // 字句を切り出して順に登録する。
        // この時点では、名前の分類が未確定 (InnerNameのまま)。
        // 先頭と末尾に番兵としてEOFを入れておく。
        //
        lexer.push_token(TType::EOF, "");
        loop {
            lexer.skip_spaces();
            let tok = lexer.get_tok()?;
            if tok.t_type == TType::EOF {
                break;
            }
            if tok.t_type == TType::Nop {
                continue;
            }
            lexer.tokens.push(tok);
        }
        lexer.push_token(TType::EOF, "");
        lexer.index = 1;

        // -------------------------------------------------------------
        // InnerName ":" InnerName -> InnerName と縮約
        // BracedURILiteral InnerName -> URIQualifiedName と縮約
        //

        // -------------------------------------------------------------
        // 特別なトークン規則 (1)
        // 所定の条件のとき、名前を演算子に書き替え。
        //
        lexer.rewrite_operator_type();

        // -------------------------------------------------------------
        // 特別なトークン規則 (2)
        // 2語から成るトークンを縮約。縮約によって生じたNopを削除。
        //
        lexer.rewrite_pair_words();
        lexer.eliminate_nops();

        // -------------------------------------------------------------
        // 特別なトークン規則 (3)
        // 所定の条件のとき、名前を所定のトークン種に書き替え。
        //
        lexer.rewrite_name_and_symbol();

        return Ok(lexer);
    }

    // -----------------------------------------------------------------
    //
    fn get_tok(&mut self) -> Result<Token, Box<Error>> {

        if self.look_ahead_keyword("(:") == true {
            self.skip_comment()?;
            return Ok(new_token(TType::Nop, ""));
        }

        if self.look_ahead_keyword("Q{") == true {
            self.unread_rune();
            self.unread_rune();
            let literal = self.fetch_until('}')?;
            return Ok(new_token(TType::BracedURILiteral, &literal));
        }

        let keywords_spec = [
            ( "//", TType::SlashSlash ),
            ( "/",  TType::Slash ),
            ( "::", TType::ColonColon ),
            ( ":=", TType::Bind ),
            ( ":*", TType::ColonAsterisk ),
            ( ":",  TType::Colon ),
            ( "=>", TType::Arrow ),
            ( "=",  TType::GeneralEQ ),
            ( "!=", TType::GeneralNE ),
            ( "!",  TType::OperatorMap ),
            ( "||", TType::OperatorConcat ),
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
            ( "*:", TType::AsteriskColon ),
            ( "*",  TType::Asterisk ),
            ( "$",  TType::Dollar ),
            ( "[",  TType::LeftBracket ),
            ( "]",  TType::RightBracket ),
            ( "(",  TType::LeftParen ),
            ( ")",  TType::RightParen ),
            ( "@",  TType::At ),
            ( "#",  TType::Sharp ),
            ( "{",  TType::LeftCurly ),
            ( "}",  TType::RightCurly ),
            ( "..", TType::DotDot ),
        ];

        for (keyword, ttype) in keywords_spec.iter() {
            if self.look_ahead_keyword(keyword) == true {
                return Ok(new_token(ttype.clone(), keyword));
            }
        }

        // -------------------------------------------------------------
        //
        let ch1 = self.read_rune();
        if is_eof(ch1) {
            return Ok(new_token(TType::EOF, ""));

        } else if is_name_start_char(ch1) {
            let mut name = String::new();
            name.push(ch1);
            loop {
                let ch2 = self.read_rune();
                if ! is_name_char(ch2) {
                    self.unread_rune();
                    break;
                }
                name.push(ch2);
            }
            return Ok(new_token(TType::InnerName, &name));

        } else if ch1 == '"' || ch1 == '\'' {
            let literal = self.fetch_string_literal(ch1)?;
            return Ok(new_token(TType::StringLiteral, &literal));

        } else if is_digit(ch1) {
            self.unread_rune();
            return self.fetch_numerics();

        } else if ch1 == '.' {
            let ch2 = self.read_rune();
            if is_digit(ch2) {
                self.unread_rune();
                self.unread_rune();
                return self.fetch_numerics();
            } else {
                self.unread_rune();
                return Ok(new_token(TType::Dot, "."));
            }

        } else {
            return Err(xpath_syntax_error!(
                    "XPathを構成する字句として認識できない文字: {}", ch1));
        }
    }

    // -----------------------------------------------------------------
    // 特別なトークン規則 (1)
    // 前にトークンがあり、そのトークンが
    //      prev_t_types
    // のいずれでもない場合、
    //      "and" "or" "div" "mod" その他の名前を演算子名とする。
    // (註1) XPath 1.0 の規格には明示的に書いてない (字句構造規則なので) が、
    //       prev_t_typesにはコロン (:) も加える必要がある。
    // (註2) XPath 2.0 でさらにトークン種を追加した。
    //
    fn rewrite_operator_type(&mut self) {
        let prev_t_types = [
            TType::EOF,             // 前にトークンがない場合はこの状態
            TType::At,
            TType::ColonColon,
            TType::LeftParen,
            TType::LeftBracket,
            TType::Comma,
            TType::And,
            TType::Or,
            TType::Div,
            TType::IDiv,            // (註2)
            TType::Mod,
            TType::Slash,
            TType::SlashSlash,
            TType::Union,
            TType::Intersect,       // (註2)
            TType::Except,          // (註2)
            TType::InstanceOf,      // (註2)
            TType::TreatAs,         // (註2)
            TType::CastableAs,      // (註2)
            TType::CastAs,          // (註2)
            TType::Plus,
            TType::Minus,
            TType::ValueEQ,         // (註2)
            TType::ValueNE,         // (註2)
            TType::ValueGT,         // (註2)
            TType::ValueGE,         // (註2)
            TType::ValueLT,         // (註2)
            TType::ValueLE,         // (註2)
            TType::GeneralEQ,
            TType::GeneralNE,
            TType::GeneralGT,
            TType::GeneralGE,
            TType::GeneralLT,
            TType::GeneralLE,
            TType::IsSameNode,      // (註2)
            TType::To,              // (註2)
            TType::NodeBefore,      // (註2)
            TType::NodeAfter,       // (註2)
            TType::Asterisk,
            TType::Colon,           // (註1)
        ];

        let operator_words: HashMap<&str, TType> = [
            ( "and",       TType::And ),
            ( "or",        TType::Or ),
            ( "div",       TType::Div ),
            ( "mod",       TType::Mod ),
            ( "idiv",      TType::IDiv ),           // (註2)
            ( "eq",        TType::ValueEQ ),        // (註2)
            ( "ne",        TType::ValueNE ),        // (註2)
            ( "lt",        TType::ValueLT ),        // (註2)
            ( "le",        TType::ValueLE ),        // (註2)
            ( "gt",        TType::ValueGT ),        // (註2)
            ( "ge",        TType::ValueGE ),        // (註2)
            ( "is",        TType::IsSameNode ),     // (註2)
            ( "to",        TType::To ),             // (註2)
            ( "union",     TType::Union ),          // (註2)
            ( "intersect", TType::Intersect ),      // (註2)
            ( "except",    TType::Except ),         // (註2)
        ].iter().cloned().collect();

        let mut i = 1;
        while self.tokens[i].t_type != TType::EOF {
            if ! prev_t_types.contains(&self.tokens[i-1].t_type) &&
               self.tokens[i].t_type == TType::InnerName {
                if let Some(op_type) = operator_words.get(self.tokens[i].name.as_str()) {
                    self.tokens[i].t_type = op_type.clone();
                }
            }
            i += 1;
        }
    }

    // -----------------------------------------------------------------
    // 特別なトークン規則 (2)
    // 2語から成るトークンを縮約する。
    //
    fn rewrite_pair_words(&mut self) {
        let operator_pair_words: [(&str, &str, TType); 4] = [
            ( "instance", "of", TType::InstanceOf ),
            ( "treat",    "as", TType::TreatAs ),
            ( "castable", "as", TType::CastableAs ),
            ( "cast",     "as", TType::CastAs ),
        ];
        let mut i = 1;
        while self.tokens[i+1].t_type != TType::EOF {
            if self.tokens[i].t_type == TType::InnerName &&
               self.tokens[i+1].t_type == TType::InnerName {
                for (str1, str2, t_type) in operator_pair_words.iter() {
                    if self.tokens[i].name.as_str() == *str1 &&
                       self.tokens[i+1].name.as_str() == *str2 {
                        self.tokens[i].t_type = t_type.clone();
                        self.tokens[i+1].t_type = TType::Nop;
                    }
                }
            }
            i += 1;
        }
    }

    // -----------------------------------------------------------------
    // 特別なトークン規則 (3)
    // 所定の字句 (Name) について、その次のトークンが '(' などの時、
    // 所定のトークン種に書き替える。
    //
    fn rewrite_name_and_symbol(&mut self) {
        let name_and_symbol_tbl: [(&str, TType, TType); 35] = [
            ( "array",              TType::LeftParen, TType::Array ),
            ( "attribute",          TType::LeftParen, TType::AttributeTest ),
            ( "comment",            TType::LeftParen, TType::CommentTest ),
            ( "document-node",      TType::LeftParen, TType::DocumentTest ),
            ( "element",            TType::LeftParen, TType::ElementTest ),
            ( "empty-sequence",     TType::LeftParen, TType::EmptySequence ),
            ( "function",           TType::LeftParen, TType::Function ),
            ( "if",                 TType::LeftParen, TType::If ),
            ( "item",               TType::LeftParen, TType::Item ),
            ( "map",                TType::LeftParen, TType::Map ),
            ( "namespace-node",     TType::LeftParen, TType::NamespaceNodeTest ),
            ( "node",               TType::LeftParen, TType::AnyKindTest ),
            ( "processing-instruction", TType::LeftParen, TType::PITest ),
            ( "schema-attribute",   TType::LeftParen, TType::SchemaAttributeTest ),
            ( "schema-element",     TType::LeftParen, TType::SchemaElementTest ),
            ( "switch",             TType::LeftParen, TType::Switch ),
            ( "text",               TType::LeftParen, TType::TextTest ),
            ( "typeswitch",         TType::LeftParen, TType::TypeSwitch ),
            ( "for",                TType::Dollar,     TType::For ),
            ( "some",               TType::Dollar,     TType::Some ),
            ( "every",              TType::Dollar,     TType::Every ),
            ( "let",                TType::Dollar,     TType::Let ),
            ( "ancestor",           TType::ColonColon, TType::AxisName ),
            ( "ancestor-or-self",   TType::ColonColon, TType::AxisName ),
            ( "attribute",          TType::ColonColon, TType::AxisName ),
            ( "child",              TType::ColonColon, TType::AxisName ),
            ( "descendant",         TType::ColonColon, TType::AxisName ),
            ( "descendant-or-self", TType::ColonColon, TType::AxisName ),
            ( "following",          TType::ColonColon, TType::AxisName ),
            ( "following-sibling",  TType::ColonColon, TType::AxisName ),
            ( "namespace",          TType::ColonColon, TType::AxisName ),
            ( "parent",             TType::ColonColon, TType::AxisName ),
            ( "preceding",          TType::ColonColon, TType::AxisName ),
            ( "preceding-sibling",  TType::ColonColon, TType::AxisName ),
            ( "self",               TType::ColonColon, TType::AxisName ),
        ];
//                        // "map "{"

        let mut i = 1;
        while self.tokens[i].t_type != TType::EOF {
            if self.tokens[i].t_type == TType::InnerName {
                for (name, next_t_type, new_t_type) in name_and_symbol_tbl.iter() {
                    if self.tokens[i].name.as_str() == *name &&
                       self.tokens[i+1].t_type == *next_t_type {
                        self.tokens[i].t_type = new_t_type.clone();
                    }
                }

                // 書き替えが起こらなかった場合はTType::Nameに書き替え
                if self.tokens[i].t_type == TType::InnerName {
                    self.tokens[i].t_type = TType::Name;
                            // 次がLeftParenならばFunctionNameに書き替え?
                }
            }
            i += 1;
        }
    }

    // -----------------------------------------------------------------
    // 縮約によって生じたNopを削除。
    //
    fn eliminate_nops(&mut self) {
        let mut i = self.tokens.len() - 1;
        while 0 < i {
            if self.tokens[i].t_type == TType::Nop {
                self.tokens.remove(i as usize);
            }
            i -= 1;
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
    // 数値リテラルを取得し、種類に応じたトークン種を返す。
    // [ 58] NumericLiteral ::= IntegerLiteral | DecimalLiteral | DoubleLiteral
    // [113] IntegerLiteral ::= Digits
    // [114] DecimalLiteral ::= ("." Digits) | (Digits "." [0-9]*)
    // [115] DoubleLiteral  ::= (("." Digits) | (Digits ("." [0-9]*)?)) [eE] [+-]? Digits
    // [125] Digits ::= [0-9]+
    //
    fn fetch_numerics(&mut self) -> Result<Token, Box<Error>> {
        let literal = &self.fetch_numeric_literal()?;
        if literal.contains("e") || literal.contains("E") {
            return Ok(new_token(TType::DoubleLiteral, literal));
        } else if literal.contains(".") {
            return Ok(new_token(TType::DecimalLiteral, literal));
        } else {
            return Ok(new_token(TType::IntegerLiteral, literal));
        }
    }

    // -----------------------------------------------------------------
    // 数値リテラルを取得する。
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
    // [116] StringLiteral ::= ('"' (EscapeQuot | [^"])* '"')
    //                       | ("'" (EscapeApos | [^'])* "'")
    // [119] EscapeQuot ::= '""'
    // [120] EscapeApos ::= "''"
    //
    fn fetch_string_literal(&mut self, delim: char) -> Result<String, Box<Error>> {
        let mut string_literal = String::new();
        loop {
            let ch1 = self.read_rune();
            if is_eof(ch1) {
                return Err(xpath_syntax_error!("Unexpected EOF while scanning string literal."));
            } else if ch1 == delim {
                let ch2 = self.read_rune();
                if ch2 == delim {
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
    // delimまでのリテラルを取得する。
    // [118] BracedURILiteral ::= "Q" "{" [^{}]* "}"
    //
    fn fetch_until(&mut self, delim: char) -> Result<String, Box<Error>> {
        let mut literal = String::new();
        loop {
            let ch1 = self.read_rune();
            if is_eof(ch1) {
                return Err(xpath_syntax_error!("Unexpected EOF while scanning."));
            } else if ch1 == delim {
                literal.push(ch1);
                return Ok(literal);
            } else {
                literal.push(ch1);
            }
        }
    }

    // -----------------------------------------------------------------
    // 註釈を読み飛ばす。
    // [121] Comment ::= "(:" (CommentContents | Comment)* ":)"
    // [126] CommentContents ::= (Char+ - (Char* ('(:' | ':)') Char*))
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
        self.ch_index += 1;
        if self.char_vec.len() <= self.ch_index - 1 {
            return EOF;
        } else {
            return self.char_vec[self.ch_index - 1];
        }
    }

    // -----------------------------------------------------------------
    // 文字を読み戻す。
    //
    fn unread_rune(&mut self) {
        if 0 < self.ch_index {
            self.ch_index -= 1;
        }
    }
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

