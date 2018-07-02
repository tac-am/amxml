//
// lib.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//
//!
//! XML processor with some features of XPath 2.0.
//!
//! # Building DOM tree from XML document string
//!
//! Building DOM tree can be done by calling 'new_document' function.
//! The DOM tree can be turned into String.
//!
//! ```
//! use amxml::dom::*;
//! let xml_string = r#"<?xml version="1.0"?><article>foo</article>"#;
//! let doc = new_document(&xml_string).unwrap();
//! let result = doc.to_string();
//! assert_eq!(result, xml_string);
//! ```
//!
//! # Navigating DOM tree
//!
//! Navigating DOM tree, or retrieving the DOM node, can be done by
//! 'root_element', 'parent', 'first_child', 'nth_child',
//! 'attribute_value' methods.
//!
//! See the description and example of corresponding method.
//! 
//! # Retrieving the DOM node by XPath
//!
//! But more convenient way for retrieving the DOM node is, perhaps,
//! using XPath, especially when the search criteria is not trivial.
//! 
//! First XPath example is somewhat straightforward.
//! 'each_node' method visits the DOM nodes that match with the given XPath,
//! and apply the function (closure) to these nodes.
//!
//! ```
//! use amxml::dom::*;
//! let xml = r#"<root><a img="a1"/><a img="a2"/></root>"#;
//! let doc = new_document(xml).unwrap();
//! let mut img = String::new();
//! doc.each_node("/root/a", |n| {
//!     img += n.attribute_value("img").unwrap().as_str();
//! });
//! assert_eq!(img, "a1a2");
//! ```
//!
//! Second XPath example is more complex.
//! This finds the clerk OR engineer (NOT advisor) who has no subordinates.
//! Note that clerks and enginners appear in <em>document order</em>
//! in 'each_node' iteration.
//!
//! ```
//! use amxml::dom::*;
//! let xml = r#"
//! <?xml version='1.0' encoding='UTF-8'?>
//! <root>
//!     <clerk name="Ann">
//!         <advisor name="Betty"/>
//!         <clerk name="Charlie"/>
//!     </clerk>
//!     <engineer name="Dick">
//!         <engineer name="Emily"/>
//!     </engineer>
//!     <clerk name="Fred"/>
//! </root>
//! "#;
//! let doc = new_document(xml).unwrap();
//! let root = doc.root_element();
//! let xpath = "(//clerk | //engineer)[count(./*) = 0]";
//! let mut names = String::new();
//! root.each_node(xpath, |n| {
//!     names += n.attribute_value("name").unwrap().as_str();
//!     names += "; ";
//! });
//! assert_eq!(names, "Charlie; Emily; Fred; ");
//! 
//! ```
//!
//! Also see the description and example of 'each_node', 'get_first_node',
//! 'get_nodeset' methods.
//!
//! # Evaluating XPath
//!
//! XPath can also be used to evaluate for the DOM tree and get boolean,
//! numeric, string values as well as DOM node.
//! The example below lists up the students, and whether or not each student
//! got 80 or more points in <em>every</em> (not <em>some</em>) examination.
//!
//! ```
//! use amxml::dom::*;
//! let xml = r#"
//! <root base="base">
//!     <student>
//!         <name>George</name>
//!         <exam subject="math" point="70"/>
//!         <exam subject="science" point="90"/>
//!     </student>
//!     <student>
//!         <name>Harry</name>
//!         <exam subject="math" point="80"/>
//!         <exam subject="science" point="95"/>
//!     </student>
//!     <student>
//!         <name>Ivonne</name>
//!         <exam subject="math" point="60"/>
//!         <exam subject="science" point="75"/>
//!     </student>
//! </root>
//! "#;
//! let doc = new_document(xml).unwrap();
//! let root = doc.root_element();
//! let xpath= r#"
//! for $student in /root/student return
//!     ($student/name/text(),
//!      every $exam in $student/exam satisfies number($exam/@point) >= 80)
//! "#;
//! let result = root.eval_xpath(xpath).unwrap();
//! assert_eq!(result.to_string(), "(George, false, Harry, true, Ivonne, false)");
//! 
//! ```
//!
//! # Manipurating the DOM node
//!
//! Inserting / replacing / deleting the DOM node can be done by
//! methods like 'append_child', 'insert_as_previous_sibling', 
//! 'insert_as_next_sibling', 'delete_child', 'replace_with', 'set_attribute',
//! 'delete_attribute' methods.
//!
//! See the description and example of corresponding method.
//!

#[macro_use]
pub mod xmlerror;
pub mod sax;
pub mod dom;

pub mod xpath;
mod xpath_impl {
    pub mod lexer;
    pub mod parser;
    pub mod xitem;
    pub mod xsequence;
    pub mod eval;
    pub mod func;
    pub mod oper;
    pub mod helpers;
}

