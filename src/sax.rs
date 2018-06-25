//
// sax.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//
//!
//! Subset of simple SAX processor.
//!
//! Module sax implements a subset of simple XML parser.
//!
//! # Examples
//!
//! Basic usage:
//!
//! - Create the SaxDecoder object with the XML string as argument.
//!
//! - Call SaxDecoder#raw_token() in loop. raw_token() returns
//!   the next token, which is one of StartElement, EndElement, CharData, etc.
//!
//! - If XML string has standalone element like &lt;foo/&gt;, raw_token()
//!   returns in 2 phases: first as StartElement &lt;foo&gt;, and second
//!   as EndElement &lt;/foo&gt;.
//!
//! - SaxDecoder decodes the entity/charcter references, e.g., '&amp;gt;'
//!   into '&gt;'.
//!
//! ```
//! use amxml::sax::*;
//! let xml_string = r#"<?xml version="1.0"?><!--COM--><root a="v">-&gt;text&#169;<img/></root>"#;
//! let mut dec = SaxDecoder::new(&xml_string).unwrap();
//! let mut buf = String::from("");
//! loop {
//!     match dec.raw_token() {
//!         Ok(XmlToken::EOF) => {
//!             buf += "EOF";
//!             break;
//!         },
//!         Ok(XmlToken::StartElement{name, attr}) => {
//!             buf += &format!("[S] {}; ", name);
//!             for at in attr.iter() {
//!                 buf += &format!("[A] {} = \"{}\"; ", at.name(), at.value());
//!             }
//!         },
//!         Ok(XmlToken::EndElement{name}) => {
//!             buf += &format!("[E] {}; ", name);
//!         },
//!         Ok(XmlToken::CharData{chardata}) => {
//!             buf += &format!("[T] \"{}\"; ", chardata);
//!         },
//!         Ok(XmlToken::ProcInst{target, inst}) => {
//!             buf += &format!("[P] {}: {}; ", target, inst);
//!         },
//!         Ok(XmlToken::Comment{comment}) => {
//!             buf += &format!("[C] {}; ", comment);
//!         },
//!         _ => {},
//!     }
//! }
//! assert_eq!(buf, r#"[P] xml: version="1.0"; [C] COM; [S] root; [A] a = "v"; [T] "->text©"; [S] img; [E] img; [E] root; EOF"#);
//! ```
//!
//! ### Note
//!
//! SaxDecoder does not verify that StartElement and EndElement match.
//! Also SaxDecoder does not translate namespace prefixes to
//! their corresponding URIs.
//!
//! SaxDecoder does not care Directives &lt;!DOCTYPE ...&gt;,
//! &lt;!ELEMENT ...&gt;, etc.
//!
//! SaxDecoder recognizes the XML declaration as ProcInit.
//! Caller should check if target equals to "xml".
//!
//! SaxDecoder accepts some illegal XML documents, like those
//! that have more than one XML declarations, more than one root elements.
//!

use std::char;
use std::error::Error;
use std::u32;
use std::usize;
use xmlerror::*;

// =====================================================================
//
const EOF: char = '\u{0000}';

// =====================================================================
/// SaxDecoder represents an XML parser reading a particular input stream.
/// See the module document for details.
///
pub struct SaxDecoder {
    char_vec: Vec<char>,
    index: usize,
    to_close: String,
            // <foo/> が現れてStartElementを返し、次にEndElementを返す
            // 必要があるとき、そのタグ名。
}

// =====================================================================
/// XmlToken, return type of SaxDecoder#raw_token()
///
pub enum XmlToken {
    EOF,
    StartElement {
        name: String,
        attr: Vec<Attr>,
    },
    EndElement {
        name: String,
    },
    CharData {
        chardata: String,
    },
    ProcInst {
        target: String,
        inst: String,
    },
    Comment {
        comment: String,
    },
    Directive {
        directive: String,
    },
}

// =====================================================================
/// In XmlToken::StartElement, attribute of element.
///
pub struct Attr {
    name: String,
    value: String,
}

impl Attr {
    pub fn name(&self) -> &str {
        return self.name.as_str();
    }
    pub fn value(&self) -> &str {
        return self.value.as_str();
    }
}

// =====================================================================
//
impl SaxDecoder {

    // -----------------------------------------------------------------
    /// Creates a new XML parser reading from String.
    ///
    pub fn new(xml_string: &str) -> Result<SaxDecoder, Box<Error>> {
        return Ok(SaxDecoder{
            char_vec: xml_string.chars().filter(|x| *x != '\r').collect(),
                // XML 1.0: 行末の処理
                // 解析前に改行すべてを #x0A に標準化する。
            index: 0,
            to_close: String::from(""),
        });
    }

    // -----------------------------------------------------------------
    /// Returns the next XML token in the input stream.
    /// At end of the input stream, raw_token() returns XmlToken::EOF.
    ///
    pub fn raw_token(&mut self) -> Result<XmlToken, Box<Error>> {
        if self.to_close != "" {
            let name = self.to_close.clone();
            self.to_close = String::from("");
            return Ok(XmlToken::EndElement{name});
        }

        let mut ch = self.getchar();
        if ch == EOF {
            return Ok(XmlToken::EOF);

        } else if ch == '<' {
            ch = self.getchar();

            if ch == EOF {
                return Err(xml_syntax_error!("Unexpected EOF after <"));

            // ---------------------------------------------------------
            // 終了タグ
            // [42] ETag ::= '</' Name S? '>'
            //
            } else if ch == '/' {              // End Element: </foo>
                let name = self.get_name();
                self.get_until_ch('>')?;
                return Ok(XmlToken::EndElement{name});

            // ---------------------------------------------------------
            // 処理命令; ここではxml宣言も処理命令扱いする。
            // [16] PI ::= '<?' PITarget (S (Char* - (Char* '?>' Char*)))? '?>'
            // [17] PITarget ::= Name - (('X' | 'x') ('M' | 'm') ('L' | 'l'))
            //
            // [23] XMLDecl ::= '<?xml' VersionInfo EncodingDecl? SDDecl? S? '?>'
            //
            } else if ch == '?' {   // Processing instruction
                let target = self.get_name();
                self.skip_spaces();
                let inst = self.get_until("?>")?;
                return Ok(XmlToken::ProcInst{target, inst});

            // ---------------------------------------------------------
            // comment: <!-- --> / <![CDATA[ ... ]]> / <!DOCTYPE ...>
            //
            } else if ch == '!' {
                ch = self.getchar();
                if ch == '-' {                      // <!-
                    ch = self.getchar();
                    if ch == '-' {                  // <!--
                        let comment = self.get_until("-->")?;
                        return Ok(XmlToken::Comment{comment});
                    } else {
                        self.ungetchar();
                        return Err(xml_syntax_error!("Invalid sequence '<!-', not part of '<!--'"));
                    }
                } else if ch == '[' {               // <![
                    if self.look_ahead_keyword("CDATA[") == true {
                        let chardata = self.get_until("]]>")?;
                        return Ok(XmlToken::CharData{chardata: chardata});
                    } else {
                        return Err(xml_syntax_error!("Invalid sequence '<![', not part of '<![CDATA[' "));
                    }
                } else {                            // <!DOCTYPE ...>, etc.
                    let directive = format!("<!{}{}",
                        ch, &self.get_until_matching_bracket()?);
                    return Ok(XmlToken::Directive{directive: directive});
                }

            // ---------------------------------------------------------
            // 開始タグ
            // [40] STag ::= '<' Name (S Attribute)* S? '>'
            // [41] Attribute ::= Name Eq AttValue
            // [25] Eq ::= S? '=' S?
            // [10] AttValue ::= '"' ([^<&'] | Reference)* '"'
            //                 | "'" ([^<&'] | Reference)* "'"
            // [67] Reference :: EntityRef | CharRef
            // [68] EntityRef ::= '&' Name ';'      // 実体参照
            // [66] CharRef ::= '&#' [0-9]+ ';'
            //                | '&#x' [0-9a-fA-F]+ ';'      // キャラクター参照
            // 空要素用のタグ
            // [44] EmptyElemTag ::= '<' Name (S Attribute)* S? '/>'
            //
            } else {                    // Start Element
                self.ungetchar();
                let name = self.get_name();
                let mut attr = vec!{};
                loop {
                    self.skip_spaces();
                    let attr_name = self.get_name();
                    if attr_name != "" {
                        self.skip_spaces();
                        ch = self.getchar();
                        if ch == '=' {
                            self.skip_spaces();
                            ch = self.getchar();
                            if ch == '"' || ch == '\'' {
                                let attr_value = self.get_until_ch(ch)?;
                                attr.push(Attr{
                                    name: attr_name,
                                    value: decode_entity(&attr_value),
                                });
                            } else {
                                self.get_until_ch('>')?;
                                return Err(xml_syntax_error!("attr_value: no Quote"));
                            }
                        } else {
                            self.get_until_ch('>')?;
                            return Err(xml_syntax_error!("attr_name: no Eq"));
                        }
                    }
                    ch = self.getchar();
                    if ch == '>' {
                        break;
                    } else if ch == '/' {       // Standalone Element
                        ch = self.getchar();
                        if ch == '>' {
                            self.to_close = name.clone();
                            break;
                        } else {
                            return Err(xml_syntax_error!("illegal char after /"));
                        }
                    }
                }
                return Ok(XmlToken::StartElement{name, attr});
            }

        // ---------------------------------------------------------
        // char data
        //
        } else {
            self.ungetchar();
            let chardata = self.get_chardata();
            return Ok(XmlToken::CharData{chardata: decode_entity(&chardata)});
        }
    }

    // -----------------------------------------------------------------
    // [3] S ::= (#x20 | #x9 | #xD | #xA)+
    //
    fn skip_spaces(&mut self) {
        loop {
            let ch = self.getchar();
            if ch == EOF {
                return;
            } else if ! is_space(ch) {
                self.ungetchar();
                return;
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
            let ch = self.getchar();
            if ch != *key_ch {
                for _ in 0 ..= i {
                    self.ungetchar();
                }
                return false;
            }
        }
        return true;
    }

    // -----------------------------------------------------------------
    // 現在位置以降に区切り文字列 (delim) が現れるまでの部分文字列を返す。
    // 終了時点で、区切り文字列の直後が現在位置になっている。
    // - 区切り文字列まで読み飛ばす目的でも使える。
    //
    fn get_until(&mut self, delim: &str) -> Result<String, Box<XmlError>> {
        let mut s = String::new();
        loop {
            if self.look_ahead_keyword(delim) == true {
                return Ok(s);
            } else {
                let ch = self.getchar();
                if ch == EOF {
                    return Err(xml_syntax_error!("Unexpected EOF while searching {}", delim));
                }
                s.push(ch);
            }
        }
    }

    // -----------------------------------------------------------------
    // 現在位置以降に区切り文字 (delim) が現れるまでの部分文字列を返す。
    // 終了時点で、区切り文字の直後が現在位置になっている。
    // - 区切り文字まで読み飛ばす目的でも使える。
    // - get_until() で、delim が1字だけの場合を効率よく処理する。
    //
    fn get_until_ch(&mut self, delim: char) -> Result<String, Box<XmlError>> {
        let mut s = String::new();
        loop {
            let ch = self.getchar();
            if ch == EOF {
                return Err(xml_syntax_error!("Unexpected EOF while searching {}", delim));
            } else if ch == delim {
                return Ok(s);
            } else {
                s.push(ch);
            }
        }
    }

    // -----------------------------------------------------------------
    // [5] Name ::= (Letter | '_' | ':') (NameChar)*
    // - 名前空間を表す「:」が含まれていても、分解せずそのまま返す。
    //
    fn get_name(&mut self) -> String {
        let mut s = String::new();

        let ch = self.getchar();
        if ! is_name_first_char(ch) {
            self.ungetchar();
            return s;
        }

        s.push(ch);
        loop {
            let ch = self.getchar();
            if is_name_char(ch) {
                s.push(ch);
            } else {
                self.ungetchar();
                return s;
            }
        }
    }

    // -----------------------------------------------------------------
    // '<' の直前、または EOF までを取得する。
    //
    fn get_chardata(&mut self) -> String {
        let mut s = String::new();
        loop {
            let ch = self.getchar();
            if ch == EOF {
                return s;
            } else if ch == '<' {
                self.ungetchar();
                return s;
            } else {
                s.push(ch);
            }
        }
    }

    // -----------------------------------------------------------------
    //
    fn get_until_matching_bracket(&mut self) -> Result<String, Box<XmlError>> {
        let mut s = String::new();
        let mut nest_level = 1;
        loop {
            let ch = self.getchar();
            if ch == EOF {
                return Err(xml_syntax_error!("Unexpected EOF while searching matching bracket"));
            } else if ch == '<' {
                s.push(ch);
                nest_level += 1;
            } else if ch == '>' {
                s.push(ch);
                nest_level -= 1;
                if nest_level == 0 {
                    return Ok(s);
                }
            } else {
                s.push(ch);
            }
        }
    }

    // -----------------------------------------------------------------
    //
    fn getchar(&mut self) -> char {
        self.index += 1;
        if self.char_vec.len() <= self.index - 1 {
            return EOF;
        } else {
            return self.char_vec[self.index - 1];
        }
    }

    // -----------------------------------------------------------------
    //
    fn ungetchar(&mut self) {
        if 0 < self.index {
            self.index -= 1;
        }
    }
}

// ---------------------------------------------------------------------
// 空白
// [3] S ::= (#x20 | #x9 | #xD | #xA)+
//
fn is_space(ch: char) -> bool {
    return [ ' ', '\t', '\r', '\n' ].contains(&ch);
}

// ---------------------------------------------------------------------
// [4] NameChar ::= Letter | Digit | '.' | '-' | '_' | ':' | CombiningChar | Extender
//
fn is_name_char(ch: char) -> bool {
    if is_letter(ch) || is_digit(ch) ||
       ch == '.' || ch == '-' || ch == '_' || ch == ':' ||
       is_combining_char(ch) || is_extender(ch) {
        return true;
    } else {
        return false;
    }
}

// ---------------------------------------------------------------------
// [5] Name ::= (Letter | '_' | ':') (NameChar)*
//
fn is_name_first_char(ch: char) -> bool {
    if is_letter(ch) || ch == '_' || ch == ':' {
        return true;
    } else {
        return false;
    }
}

// ---------------------------------------------------------------------
// [84] Letter ::= BaseChar | Ideographic
// [85] BaseChar ::= ...
// [86] Ideographic ::= ...
//
fn is_letter(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x0041, 0x005A ),         // [A-Z]
        ( 0x0061, 0x007A ),         // [a-z]
        ( 0x00C0, 0x00D6 ),         // [À-Ö]
        ( 0x00D8, 0x00F6 ),         // [Ø-ö]
        ( 0x00F8, 0x00FF ),         // [ø-ÿ]
        ( 0x0100, 0x0131 ),         // [Ā-ı]
        ( 0x0134, 0x013E ),         // [Ĵ-ľ]
        ( 0x0141, 0x0148 ),         // [Ł-ň]
        ( 0x014A, 0x017E ),         // [Ŋ-ž]
        ( 0x0180, 0x01C3 ),         // ラテン文字拡張B
        ( 0x01CD, 0x01F0 ),
        ( 0x01F4, 0x01F5 ),
        ( 0x01FA, 0x0217 ),
        ( 0x0250, 0x02A8 ),         // IPA拡張
        ( 0x02BB, 0x02C1 ),
        ( 0x0386, 0x0386 ),
        ( 0x0388, 0x038A ),
        ( 0x038C, 0x038C ),
        ( 0x038E, 0x03A1 ),
        ( 0x03A3, 0x03CE ),
        ( 0x03D0, 0x03D6 ),
        ( 0x03DA, 0x03DA ),
        ( 0x03DC, 0x03DC ),
        ( 0x03DE, 0x03DE ),
        ( 0x03E0, 0x03E0 ),
        ( 0x03E2, 0x03F3 ),
        ( 0x0401, 0x040C ),
        ( 0x040E, 0x044F ),
        ( 0x0451, 0x045C ),
        ( 0x045E, 0x0481 ),
        ( 0x0490, 0x04C4 ),
        ( 0x04C7, 0x04C8 ),
        ( 0x04CB, 0x04CC ),
        ( 0x04D0, 0x04EB ),
        ( 0x04EE, 0x04F5 ),
        ( 0x04F8, 0x04F9 ),
        ( 0x0531, 0x0556 ),
        ( 0x0559, 0x0559 ),
        ( 0x0561, 0x0586 ),
        ( 0x05D0, 0x05EA ),
        ( 0x05F0, 0x05F2 ),
        ( 0x0621, 0x063A ),
        ( 0x0641, 0x064A ),
        ( 0x0671, 0x06B7 ),
        ( 0x06BA, 0x06BE ),
        ( 0x06C0, 0x06CE ),
        ( 0x06D0, 0x06D3 ),
        ( 0x06D5, 0x06D5 ),
        ( 0x06E5, 0x06E6 ),
        ( 0x0905, 0x0939 ),
        ( 0x093D, 0x093D ),
        ( 0x0958, 0x0961 ),
        ( 0x0985, 0x098C ),
        ( 0x098F, 0x0990 ),
        ( 0x0993, 0x09A8 ),
        ( 0x09AA, 0x09B0 ),
        ( 0x09B2, 0x09B2 ),
        ( 0x09B6, 0x09B9 ),
        ( 0x09DC, 0x09DD ),
        ( 0x09DF, 0x09E1 ),
        ( 0x09F0, 0x09F1 ),
        ( 0x0A05, 0x0A0A ),
        ( 0x0A0F, 0x0A10 ),
        ( 0x0A13, 0x0A28 ),
        ( 0x0A2A, 0x0A30 ),
        ( 0x0A32, 0x0A33 ),
        ( 0x0A35, 0x0A36 ),
        ( 0x0A38, 0x0A39 ),
        ( 0x0A59, 0x0A5C ),
        ( 0x0A5E, 0x0A5E ),
        ( 0x0A72, 0x0A74 ),
        ( 0x0A85, 0x0A8B ),
        ( 0x0A8D, 0x0A8D ),
        ( 0x0A8F, 0x0A91 ),
        ( 0x0A93, 0x0AA8 ),
        ( 0x0AAA, 0x0AB0 ),
        ( 0x0AB2, 0x0AB3 ),
        ( 0x0AB5, 0x0AB9 ),
        ( 0x0ABD, 0x0ABD ),
        ( 0x0AE0, 0x0AE0 ),
        ( 0x0B05, 0x0B0C ),
        ( 0x0B0F, 0x0B10 ),
        ( 0x0B13, 0x0B28 ),
        ( 0x0B2A, 0x0B30 ),
        ( 0x0B32, 0x0B33 ),
        ( 0x0B36, 0x0B39 ),
        ( 0x0B3D, 0x0B3D ),
        ( 0x0B5C, 0x0B5D ),
        ( 0x0B5F, 0x0B61 ),
        ( 0x0B85, 0x0B8A ),
        ( 0x0B8E, 0x0B90 ),
        ( 0x0B92, 0x0B95 ),
        ( 0x0B99, 0x0B9A ),
        ( 0x0B9C, 0x0B9C ),
        ( 0x0B9E, 0x0B9F ),
        ( 0x0BA3, 0x0BA4 ),
        ( 0x0BA8, 0x0BAA ),
        ( 0x0BAE, 0x0BB5 ),
        ( 0x0BB7, 0x0BB9 ),
        ( 0x0C05, 0x0C0C ),
        ( 0x0C0E, 0x0C10 ),
        ( 0x0C12, 0x0C28 ),
        ( 0x0C2A, 0x0C33 ),
        ( 0x0C35, 0x0C39 ),
        ( 0x0C60, 0x0C61 ),
        ( 0x0C85, 0x0C8C ),
        ( 0x0C8E, 0x0C90 ),
        ( 0x0C92, 0x0CA8 ),
        ( 0x0CAA, 0x0CB3 ),
        ( 0x0CB5, 0x0CB9 ),
        ( 0x0CDE, 0x0CDE ),
        ( 0x0CE0, 0x0CE1 ),
        ( 0x0D05, 0x0D0C ),
        ( 0x0D0E, 0x0D10 ),
        ( 0x0D12, 0x0D28 ),
        ( 0x0D2A, 0x0D39 ),
        ( 0x0D60, 0x0D61 ),
        ( 0x0E01, 0x0E2E ),
        ( 0x0E30, 0x0E30 ),
        ( 0x0E32, 0x0E33 ),
        ( 0x0E40, 0x0E45 ),
        ( 0x0E81, 0x0E82 ),
        ( 0x0E84, 0x0E84 ),
        ( 0x0E87, 0x0E88 ),
        ( 0x0E8A, 0x0E8A ),
        ( 0x0E8D, 0x0E8D ),
        ( 0x0E94, 0x0E97 ),
        ( 0x0E99, 0x0E9F ),
        ( 0x0EA1, 0x0EA3 ),
        ( 0x0EA5, 0x0EA5 ),
        ( 0x0EA7, 0x0EA7 ),
        ( 0x0EAA, 0x0EAB ),
        ( 0x0EAD, 0x0EAE ),
        ( 0x0EB0, 0x0EB0 ),
        ( 0x0EB2, 0x0EB3 ),
        ( 0x0EBD, 0x0EBD ),
        ( 0x0EC0, 0x0EC4 ),
        ( 0x0F40, 0x0F47 ),
        ( 0x0F49, 0x0F69 ),
        ( 0x10A0, 0x10C5 ),
        ( 0x10D0, 0x10F6 ),
        ( 0x1100, 0x1100 ),
        ( 0x1102, 0x1103 ),
        ( 0x1105, 0x1107 ),
        ( 0x1109, 0x1109 ),
        ( 0x110B, 0x110C ),
        ( 0x110E, 0x1112 ),
        ( 0x113C, 0x113C ),
        ( 0x113E, 0x113E ),
        ( 0x1140, 0x1140 ),
        ( 0x114C, 0x114C ),
        ( 0x114E, 0x114E ),
        ( 0x1150, 0x1150 ),
        ( 0x1154, 0x1155 ),
        ( 0x1159, 0x1159 ),
        ( 0x115F, 0x1161 ),
        ( 0x1163, 0x1163 ),
        ( 0x1165, 0x1165 ),
        ( 0x1167, 0x1167 ),
        ( 0x1169, 0x1169 ),
        ( 0x116D, 0x116E ),
        ( 0x1172, 0x1173 ),
        ( 0x1175, 0x1175 ),
        ( 0x119E, 0x119E ),
        ( 0x11A8, 0x11A8 ),
        ( 0x11AB, 0x11AB ),
        ( 0x11AE, 0x11AF ),
        ( 0x11B7, 0x11B8 ),
        ( 0x11BA, 0x11BA ),
        ( 0x11BC, 0x11C2 ),
        ( 0x11EB, 0x11EB ),
        ( 0x11F0, 0x11F0 ),
        ( 0x11F9, 0x11F9 ),
        ( 0x1E00, 0x1E9B ),
        ( 0x1EA0, 0x1EF9 ),
        ( 0x1F00, 0x1F15 ),
        ( 0x1F18, 0x1F1D ),
        ( 0x1F20, 0x1F45 ),
        ( 0x1F48, 0x1F4D ),
        ( 0x1F50, 0x1F57 ),
        ( 0x1F59, 0x1F59 ),
        ( 0x1F5B, 0x1F5B ),
        ( 0x1F5D, 0x1F5D ),
        ( 0x1F5F, 0x1F7D ),
        ( 0x1F80, 0x1FB4 ),
        ( 0x1FB6, 0x1FBC ),
        ( 0x1FBE, 0x1FBE ),
        ( 0x1FC2, 0x1FC4 ),
        ( 0x1FC6, 0x1FCC ),
        ( 0x1FD0, 0x1FD3 ),
        ( 0x1FD6, 0x1FDB ),
        ( 0x1FE0, 0x1FEC ),
        ( 0x1FF2, 0x1FF4 ),
        ( 0x1FF6, 0x1FFC ),
        ( 0x2126, 0x2126 ),
        ( 0x212A, 0x212B ),
        ( 0x212E, 0x212E ),
        ( 0x2180, 0x2182 ),
        ( 0x3041, 0x3094 ),         // [ぁ-ゔ]
        ( 0x30A1, 0x30FA ),         // [ァ-ヺ]
        ( 0x3105, 0x312C ),
        ( 0xAC00, 0xD7A3 ),
            // ここまで、BaseCharの定義
        ( 0x4E00, 0x9FA5 ),         // CJK統合漢字
        ( 0x3007, 0x3007 ),         // 〇
        ( 0x3021, 0x3029 ),
            // ここまで、Ideographicの定義
    ]);
}

// ---------------------------------------------------------------------
// [87] CombiningChar ::= ...
//
fn is_combining_char(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x0300, 0x0345 ),
        ( 0x0360, 0x0361 ),
        ( 0x0483, 0x0486 ),
        ( 0x0591, 0x05A1 ),
        ( 0x05A3, 0x05B9 ),
        ( 0x05BB, 0x05BD ),
        ( 0x05BF, 0x05BF ),
        ( 0x05C1, 0x05C2 ),
        ( 0x05C4, 0x05C4 ),
        ( 0x064B, 0x0652 ),
        ( 0x0670, 0x0670 ),
        ( 0x06D6, 0x06DC ),
        ( 0x06DD, 0x06DF ),
        ( 0x06E0, 0x06E4 ),
        ( 0x06E7, 0x06E8 ),
        ( 0x06EA, 0x06ED ),
        ( 0x0901, 0x0903 ),
        ( 0x093C, 0x093C ),
        ( 0x093E, 0x094C ),
        ( 0x094D, 0x094D ),
        ( 0x0951, 0x0954 ),
        ( 0x0962, 0x0963 ),
        ( 0x0981, 0x0983 ),
        ( 0x09BC, 0x09BC ),
        ( 0x09BE, 0x09BE ),
        ( 0x09BF, 0x09BF ),
        ( 0x09C0, 0x09C4 ),
        ( 0x09C7, 0x09C8 ),
        ( 0x09CB, 0x09CD ),
        ( 0x09D7, 0x09D7 ),
        ( 0x09E2, 0x09E3 ),
        ( 0x0A02, 0x0A02 ),
        ( 0x0A3C, 0x0A3C ),
        ( 0x0A3E, 0x0A3E ),
        ( 0x0A3F, 0x0A3F ),
        ( 0x0A40, 0x0A42 ),
        ( 0x0A47, 0x0A48 ),
        ( 0x0A4B, 0x0A4D ),
        ( 0x0A70, 0x0A71 ),
        ( 0x0A81, 0x0A83 ),
        ( 0x0ABC, 0x0ABC ),
        ( 0x0ABE, 0x0AC5 ),
        ( 0x0AC7, 0x0AC9 ),
        ( 0x0ACB, 0x0ACD ),
        ( 0x0B01, 0x0B03 ),
        ( 0x0B3C, 0x0B3C ),
        ( 0x0B3E, 0x0B43 ),
        ( 0x0B47, 0x0B48 ),
        ( 0x0B4B, 0x0B4D ),
        ( 0x0B56, 0x0B57 ),
        ( 0x0B82, 0x0B83 ),
        ( 0x0BBE, 0x0BC2 ),
        ( 0x0BC6, 0x0BC8 ),
        ( 0x0BCA, 0x0BCD ),
        ( 0x0BD7, 0x0BD7 ),
        ( 0x0C01, 0x0C03 ),
        ( 0x0C3E, 0x0C44 ),
        ( 0x0C46, 0x0C48 ),
        ( 0x0C4A, 0x0C4D ),
        ( 0x0C55, 0x0C56 ),
        ( 0x0C82, 0x0C83 ),
        ( 0x0CBE, 0x0CC4 ),
        ( 0x0CC6, 0x0CC8 ),
        ( 0x0CCA, 0x0CCD ),
        ( 0x0CD5, 0x0CD6 ),
        ( 0x0D02, 0x0D03 ),
        ( 0x0D3E, 0x0D43 ),
        ( 0x0D46, 0x0D48 ),
        ( 0x0D4A, 0x0D4D ),
        ( 0x0D57, 0x0D57 ),
        ( 0x0E31, 0x0E31 ),
        ( 0x0E34, 0x0E3A ),
        ( 0x0E47, 0x0E4E ),
        ( 0x0EB1, 0x0EB1 ),
        ( 0x0EB4, 0x0EB9 ),
        ( 0x0EBB, 0x0EBC ),
        ( 0x0EC8, 0x0ECD ),
        ( 0x0F18, 0x0F19 ),
        ( 0x0F35, 0x0F35 ),
        ( 0x0F37, 0x0F37 ),
        ( 0x0F39, 0x0F39 ),
        ( 0x0F3E, 0x0F3E ),
        ( 0x0F3F, 0x0F3F ),
        ( 0x0F71, 0x0F84 ),
        ( 0x0F86, 0x0F8B ),
        ( 0x0F90, 0x0F95 ),
        ( 0x0F97, 0x0F97 ),
        ( 0x0F99, 0x0FAD ),
        ( 0x0FB1, 0x0FB7 ),
        ( 0x0FB9, 0x0FB9 ),
        ( 0x20D0, 0x20DC ),
        ( 0x20E1, 0x20E1 ),
        ( 0x302A, 0x302F ),
        ( 0x3099, 0x3099 ),
        ( 0x309A, 0x309A ),
    ]);
}

// ---------------------------------------------------------------------
// [88] Digit ::= ...
//
fn is_digit(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x0030, 0x0039 ),         // [0-9]
        ( 0x0660, 0x0669 ),
        ( 0x06F0, 0x06F9 ),
        ( 0x0966, 0x096F ),
        ( 0x09E6, 0x09EF ),
        ( 0x0A66, 0x0A6F ),
        ( 0x0AE6, 0x0AEF ),
        ( 0x0B66, 0x0B6F ),
        ( 0x0BE7, 0x0BEF ),
        ( 0x0C66, 0x0C6F ),
        ( 0x0CE6, 0x0CEF ),
        ( 0x0D66, 0x0D6F ),
        ( 0x0E50, 0x0E59 ),
        ( 0x0ED0, 0x0ED9 ),
        ( 0x0F20, 0x0F29 ),
    ]);
}

// ---------------------------------------------------------------------
// [89] Extender ::= ...
//
fn is_extender(ch: char) -> bool {
    return char_is_in_ranges(ch, &[
        ( 0x00B7, 0x00B7 ),
        ( 0x02D0, 0x02D0 ),
        ( 0x02D1, 0x02D1 ),
        ( 0x0387, 0x0387 ),
        ( 0x0640, 0x0640 ),
        ( 0x0E46, 0x0E46 ),
        ( 0x0EC6, 0x0EC6 ),
        ( 0x3005, 0x3005 ),     // 々
        ( 0x3031, 0x3035 ),     // 〱〲〳〴〵
        ( 0x309D, 0x309E ),     // ゝゞ
        ( 0x30FC, 0x30FE ),     // ーヽヾ
    ]);
}

// ---------------------------------------------------------------------
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

// ---------------------------------------------------------------------
// 定義済み実体、キャラクター参照のデコード。
// [66] CharRef ::= '&#' [0-9]+ ';'
//                | '&#x' [0-9a-fA-F]+ ';'      // キャラクター参照
// [68] EntityRef ::= '&' Name ';'              // 実体参照
//
fn decode_entity(s: &String) -> String {

    // -----------------------------------------------------------------
    //
    let mut buf = s.clone();
    for (pattern, radix) in [("&#x", 16), ("&#", 10)].iter() {
        loop {
            let ss = buf.clone();
            let v: Vec<&str> = ss.splitn(2, pattern).collect();
            if v.len() != 2 {
                break;
            }

            let w: Vec<&str> = v[1].splitn(2, ";").collect();
            if w.len() != 2 {
                break;
            }

            let head = v[0];
            let entity = w[0];
            let tail = w[1];

            let u = u32::from_str_radix(&entity, *radix as u32).unwrap_or(0x3013);
                                                            // \u3013 = '〓'
            let ch = char::from_u32(u).unwrap_or('〓');
            buf = format!("{}{}{}", &head, ch, &tail);
        }
    }

    // -----------------------------------------------------------------
    //
    let entity_specs = [
        [ "&gt;", ">" ],
        [ "&lt;", "<" ],
        [ "&quot;", "\"" ],
        [ "&apos;", "'" ],
        [ "&amp;", "&" ],
    ];
    for spec in entity_specs.iter() {
        buf = buf.replace(spec[0], spec[1]);
    }

    return buf;
}


// =====================================================================
//
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decoder() {
        let test_specs = [
            [ r#"<article>About <em>XML</em> string</article>"#,
              r#"[S]: article; [T]: "About "; [S]: em; [T]: "XML"; [E]: em; [T]: " string"; [E]: article; EOF"# ],
            [ r#"<ns:a>R&amp;D</ns:a>"#,
              r#"[S]: ns:a; [T]: "R&D"; [E]: ns:a; EOF"# ],
            [ r#"<a>R&amp;D&#169; &#x30BD;&#x30BF;</a>"#,
              r#"[S]: a; [T]: "R&D© ソタ"; [E]: a; EOF"# ],
            [ r#"<a>&#xXXXX; - &#x110000;</a>"#,
              r#"[S]: a; [T]: "〓 - 〓"; [E]: a; EOF"# ],
            [ r#"<a><![CDATA[<R&D>]]></a>"#,
              r#"[S]: a; [T]: "<R&D>"; [E]: a; EOF"# ],
            [ r#"<a b="c&gt;d"/>"#,
              r#"[S]: a; b = "c>d"; [E]: a; EOF"# ],
            [ r#"<?xml version="1.0" ?>"#,
              r#"[P]: xml; version="1.0" ; EOF"# ],
            [ r#"<?xml version="3.0" ??>"#,
              r#"[P]: xml; version="3.0" ?; EOF"# ],
            [ r#"<段落 属性="値">文章</段落>"#,
              r#"[S]: 段落; 属性 = "値"; [T]: "文章"; [E]: 段落; EOF"# ],
            [ r#"<!--COMMENT-->"#,
              r#"[C]: COMMENT; EOF"# ],
            [ r#"<!--A-B-->"#,
              r#"[C]: A-B; EOF"# ],
            [ r#"<!--A--B-->"#,
              r#"[C]: A--B; EOF"# ],
            [ r#"<!-COMMENT-->"#,
              r#"Err: Syntax Error in XML: Invalid sequence '<!-', not part of '<!--'; [T]: "COMMENT-->"; EOF"# ],
            [ r#"<?xml version="1.0" "#,
              r#"Err: Syntax Error in XML: Unexpected EOF while searching ?>; EOF"# ],
            [ r#"<!DOCTYPE a [ <!ENTITY a b> ]>"#,
              r#"[D]: <!DOCTYPE a [ <!ENTITY a b> ]>; EOF"# ],
        ];

        for spec in test_specs.iter() {
            let src = String::from(spec[0]);
            let guess = spec[1];
            let dec = SaxDecoder::new(&src);
            let mut dec = match dec {
                Ok(decoder) => decoder,
                Err(e) => {
                    eprintln!("Err: {}", e);
                    return ();
                },
            };
            let mut result = String::from("");
            loop {
                let token = dec.raw_token();
                match token {
                    Ok(XmlToken::EOF) => {
                        result += "EOF";
                        break;
                    },
                    Ok(XmlToken::StartElement{name, attr}) => {
                        result += &format!("[S]: {}; ", name);
                        for at in attr.iter() {
                            result += &format!("{} = \"{}\"; ", at.name, at.value);
                        }
                    },
                    Ok(XmlToken::EndElement{name}) => {
                        result += &format!("[E]: {}; ", name);
                    },
                    Ok(XmlToken::CharData{chardata}) => {
                        result += &format!("[T]: \"{}\"; ", chardata);
                    },
                    Ok(XmlToken::ProcInst{target, inst}) => {
                        result += &format!("[P]: {}; {}; ", target, inst);
                    },
                    Ok(XmlToken::Comment{comment}) => {
                        result += &format!("[C]: {}; ", comment);
                    },
                    Ok(XmlToken::Directive{directive}) => {
                        result += &format!("[D]: {}; ", directive);
                    },
                    Err(e) => {
                        result += &format!("Err: {}; ", e);
                    },
                }
            }
            assert_eq!(result, guess);
        }
    }
}

