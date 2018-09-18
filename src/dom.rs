//
// dom.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//
//!
//! XML DOM processor
//!
//! See the Crate document and 'Structs: NodePtr' document for detail.
//!
//! ### Note
//!
//! This processor does not translate namespace prefixes to
//! their corresponding URIs.
//! If needed, you can get the URI via 'namespace_uri' method.
//!
//! This processor does not care Directives &lt;!DOCTYPE ...&gt;,
//! &lt;!ELEMENT ...&gt;, etc.
//!
//! This processor accepts some illegal XML documents, like those
//! that have more than one root elements.
//! Sometimes it is convenient to accept such document temporally
//! in the course of manipurating.
//!

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::{Rc, Weak};
use std::usize;
use sax::{SaxDecoder, XmlToken};
use xmlerror::*;

// =====================================================================
/// A node in the XML document tree.
///
#[derive(Clone)]
pub struct NodePtr {
    rc_node: RcNode,
}

type RcNode = Rc<Node>;

fn wrap_rc_clone(rc_node: &RcNode) -> NodePtr {
    return NodePtr{ rc_node: Rc::clone(rc_node) };
}

// ---------------------------------------------------------------------
//
impl fmt::Debug for NodePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.node_type() {
            NodeType::DocumentRoot => {
                return write!(f, "(DocumentRoot)");
            },
            NodeType::Element => {
                let mut str = String::new();
                str += &"<";
                str += &self.name();
                for at in self.attributes().iter() {
                    str += &format!(r#" {}="{}""#, at.name(), at.value());
                }
                str += &">";
                return write!(f, "{}", str);
            },
            NodeType::Text => {
                return write!(f, "{}", self.value());
            },
            NodeType::Attribute => {
                return write!(f, r#"{}="{}""#, self.name(), self.value());
            },
            _ => {
                return write!(f, "");
            },
        }
    }
}

// ---------------------------------------------------------------------
//
impl fmt::Display for NodePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "{:?}", self);
    }
}

// ---------------------------------------------------------------------
//
impl PartialEq for NodePtr {
    fn eq(&self, other: &NodePtr) -> bool {
        return Rc::ptr_eq(&self.rc_node, &other.rc_node)
    }
}

impl Eq for NodePtr {
}

// =====================================================================
/// Type of node in the XML document tree.
///
#[derive(Debug, PartialEq, Clone)]
pub enum NodeType {
    DocumentRoot,
    Element,
    Text,
    Comment,
    XMLDecl,
    Instruction,
    Attribute,
    Directive,
}

// =====================================================================
//
#[derive(Debug)]
struct Node {
    node_type: NodeType,
    order: Cell<i64>,
    name: String,
    value: String,
    parent: Option<RefCell<Weak<Node>>>,
    children: RefCell<Vec<RcNode>>,
    attributes: RefCell<Vec<RcNode>>,
}

// ---------------------------------------------------------------------
// RcNodeを生成する。親があるとは限らない。
//
fn make_new_rc_node(node_type: NodeType,
                parent: Option<&mut RcNode>,
                name: &str, value: &str) -> RcNode {
    let node = Rc::new(Node {
        node_type,
        order: Cell::new(0),
        name: String::from(name),
        value: String::from(value),
        parent: match parent {
            Some(p) => Some(RefCell::new(Rc::downgrade(p))),
            None => None,
        },
        children: RefCell::new(vec!{}),
        attributes: RefCell::new(vec!{}),
    });
    return node;
}

// ---------------------------------------------------------------------
// RcNodeを、既存の親 (parent) の子として生成する。
// // したがって、DocumentRootやAttributeの生成には使えない。
//
fn make_new_child_rc_node(node_type: NodeType, parent: &mut RcNode,
                name: &str, value: &str, index_hint: usize) -> RcNode {

    let node = make_new_rc_node(node_type, Some(parent), name, value);
    if index_hint <= parent.children.borrow_mut().len() {
        parent.children.borrow_mut().insert(index_hint, Rc::clone(&node));
    } else {
        parent.children.borrow_mut().push(Rc::clone(&node));
    }

    return node;
}

// =====================================================================
/// Parses the XML string and creates the DOM tree and
/// returns the topmost DocumentRoot node.
///
/// # Examples
///
/// See the module document.
///
/// # Errors
///
/// - When there is syntax error, e.g. "&lt;foo&gt;xxx&lt;/bar&gt;".
///
pub fn new_document(xml_string: &str) -> Result<NodePtr, Box<Error>> {

    let mut dec = SaxDecoder::new(&String::from(xml_string))?;

    let doc_root = make_new_rc_node(NodeType::DocumentRoot, None, "", "");
    let mut curr_node = Rc::clone(&doc_root);
    loop {
        match dec.raw_token() {
            Ok(XmlToken::EOF) => {
                break;
            },
            Ok(XmlToken::StartElement{name, attr}) => {
                let e = make_new_child_rc_node(NodeType::Element,
                            &mut curr_node,
                            name.as_str(), "", usize::MAX);
                curr_node = Rc::clone(&e);
                for at in attr.iter() {
                    let attr_node = make_new_rc_node(NodeType::Attribute,
                            Some(&mut curr_node), at.name(), at.value());
                    curr_node.attributes.borrow_mut().push(
                            Rc::clone(&attr_node));
                }
            },
            Ok(XmlToken::EndElement{name}) => {
                if curr_node.name.as_str() != name {
                    return Err(xml_syntax_error!(
                        "Element name mismatch: {} and {}",
                        curr_node.name.as_str(), name));
                }
                curr_node = match curr_node.parent {
                    Some(ref p) => p.borrow().upgrade().unwrap(),
                    None => Rc::clone(&curr_node),
                };
            },
            Ok(XmlToken::CharData{chardata}) => {
                make_new_child_rc_node(NodeType::Text,
                            &mut curr_node,
                            "", chardata.as_str(), usize::MAX);
            },
            Ok(XmlToken::ProcInst{target, inst}) => {
                if target == "xml" {
                    make_new_child_rc_node(NodeType::XMLDecl,
                                &mut curr_node,
                                "xml", inst.as_str(), usize::MAX);
                } else {
                    make_new_child_rc_node(NodeType::Instruction,
                                &mut curr_node,
                                target.as_str(), inst.as_str(), usize::MAX);
                    
                }
            },
            Ok(XmlToken::Comment{comment}) => {
                make_new_child_rc_node(NodeType::Comment,
                            &mut curr_node,
                            "", comment.as_str(), usize::MAX);
            },
            Ok(XmlToken::Directive{directive: _directive}) => {},
            Err(e) => {
                return Err(xml_syntax_error!("XML syntax error: {}", e));
            },
        }
    }
    return Ok(NodePtr{rc_node: doc_root});
}

// ---------------------------------------------------------------------
//
fn shallow_copy_rc_rels(target: &mut RcNode, source: &RcNode) {
    for ch in source.children.borrow().iter() {
        target.children.borrow_mut().push(Rc::clone(ch));
    }
    for at in source.attributes.borrow().iter() {
        target.attributes.borrow_mut().push(Rc::clone(at));
    }
}

// =====================================================================
//
impl NodePtr {

    // =================================================================
    /// Turns XML DOM tree into XML string. cf. inner_xml()
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<?xml version="1.0"?><article>About <em>XML</em> string</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// let result = root_elem.to_string();
    /// assert_eq!(result, "<article>About <em>XML</em> string</article>");
    /// ```
    ///
    pub fn to_string(&self) -> String {
        return to_string_with_indent(&self.unwrap_rc(), 0, 0);
    }

    // =================================================================
    /// Turns XML DOM tree into 'pretty' XML string with four spaces indent.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<?xml version="1.0"?><article>About <em>XML</em> string</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// let result = root_elem.to_pretty_string();
    /// let guess = r#"<article>
    ///     About 
    ///     <em>
    ///         XML
    ///     </em>
    ///      string
    /// </article>
    /// "#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn to_pretty_string(&self) -> String {
        return to_string_with_indent(&self.unwrap_rc(), 0, 4);
    }

    // =================================================================
    /// Turns XML DOM tree under self into XML string. cf. to_string()
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<?xml version="1.0"?><article>About <em>XML</em> string</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// let result = root_elem.inner_xml();
    /// assert_eq!(result, "About <em>XML</em> string");
    /// ```
    ///
    pub fn inner_xml(&self) -> String {
        let mut s = String::new();
        for ch in self.children().iter() {
            s += &ch.to_string();
        }
        return s;
    }

    // =================================================================
    /// Returns type of the node (NodeType::Element, etc.).
    //
    pub fn node_type(&self) -> NodeType {
        return self.unwrap_rc().node_type.clone();
    }

    // =================================================================
    /// Returns the name of the Element/Attribute node,
    /// or the target of the Instruction node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<ns:article>XML</ns:article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// assert_eq!(root_elem.name(), "ns:article");
    /// ```
    ///
    pub fn name(&self) -> String {
        return self.unwrap_rc().name.clone();
    }

    // =================================================================
    /// Returns the value of the Attribute node,
    /// text of the Text/Comment node,
    /// inst of XMLDecl/Instruction node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<ns:article>XML</ns:article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let text_node = doc.get_first_node("//text()").unwrap();
    /// assert_eq!(text_node.value(), "XML");
    /// ```
    ///
    pub fn value(&self) -> String {
        return self.unwrap_rc().value.clone();
    }

    // =================================================================
    /// Returns the local name of Element.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<ns:article>foo</ns:article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// assert_eq!(root_elem.local_name(), "article");
    /// ```
    ///
    pub fn local_name(&self) -> String {
        let name = &self.unwrap_rc().name;
        let v: Vec<&str> = name.splitn(2, ":").collect();
        if v.len() == 2 {
            return String::from(v[1]);
        } else {
            return name.clone();
        }
    }

    // =================================================================
    /// Returns the space name of Element, or "" if not with namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<ns:article>foo</ns:article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// assert_eq!(root_elem.space_name(), "ns");
    /// ```
    ///
    pub fn space_name(&self) -> String {
        let name = &self.unwrap_rc().name;
        let v: Vec<&str> = name.splitn(2, ":").collect();
        if v.len() == 2 {
            return String::from(v[0]);
        } else {
            return String::new();
        }
    }

    // =================================================================
    /// Returns the URI for namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<root xmlns="http://def" xmlns:ns="http://ns"><ns:a/><b/></root>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root = doc.root_element();
    /// let elem_ns_a = doc.get_first_node("//ns:a").unwrap();
    /// let elem_b = doc.get_first_node("//b").unwrap();
    /// assert_eq!(elem_ns_a.namespace_uri(), "http://ns");
    /// assert_eq!(elem_b.namespace_uri(), "http://def");
    /// ```
    ///
    pub fn namespace_uri(&self) -> String {
        let mut xmlns_attr = String::from("xmlns");
        let space = self.space_name();
        if space.as_str() != "" {
            xmlns_attr += &":";
            xmlns_attr += &space;
        }

        let mut curr = self.unwrap_rc();
        while (*curr).node_type != NodeType::DocumentRoot {
            let val = wrap_rc_clone(&curr).attribute_value(xmlns_attr.as_str());
            if let Some(value) = val {
                return value.clone();
            }
            curr = match (*curr).parent {
                Some(ref p) => p.borrow().upgrade().unwrap(),
                None => return String::new(),
            };
        }
        return String::new();
    }

    // =================================================================
    /// Returns the root (topmost) node of DOM tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article><p>DOM</p></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_p = doc.get_first_node("/article/p").unwrap();
    /// assert_eq!(elem_p.name(), "p");
    /// let root = elem_p.root();
    /// assert_eq!(root.node_type(), NodeType::DocumentRoot);
    /// ```
    ///
    pub fn root(&self) -> NodePtr {
        let mut curr = self.unwrap_rc();
        loop {
            curr = match (*curr).parent {
                Some(ref p) => p.borrow().upgrade().unwrap(),
                None => return wrap_rc_clone(&curr),
            };
        }
    }

    // =================================================================
    /// Returns the topmost Element node,
    /// or root node when there is no Element node (illegal case),
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article><p>DOM</p></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_p = doc.get_first_node("/article/p").unwrap();
    /// assert_eq!(elem_p.name(), "p");
    /// let root_elem = elem_p.root_element();
    /// assert_eq!(root_elem.name(), "article");
    /// ```
    ///
    pub fn root_element(&self) -> NodePtr {
        let doc_root = self.root();
        for ch in doc_root.children().iter() {
            if ch.node_type() == NodeType::Element {
                return ch.clone();
            }
        }
        return doc_root;
    }

    // =================================================================
    /// Returns the parent of the 'node', or None
    /// if 'node' has no parent (i.e. is DocumentRoot).
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article><chapter>foo</chapter></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_chapter = doc.get_first_node("//chapter").unwrap();
    /// let p = elem_chapter.parent().unwrap();
    /// assert_eq!(p.name(), "article");
    /// ```
    ///
    pub fn parent(&self) -> Option<NodePtr> {
        match self.unwrap_rc().parent {
            Some(ref p) => {
                let parent_node = p.borrow().upgrade().unwrap();
                return Some(wrap_rc_clone(&parent_node));
            },
            None => return None,
        }
    }

    // =================================================================
    // Returns the vector of child nodes.
    //
    pub fn children(&self) -> Vec<NodePtr> {
        let mut node_array: Vec<NodePtr> = vec!{};
        let rc_node = self.unwrap_rc();
        for ch in (*rc_node).children.borrow().iter() {
            node_array.push(wrap_rc_clone(ch));
        }
        return node_array;
    }

    // =================================================================
    // Returns the vector of attribute nodes.
    //
    pub fn attributes(&self) -> Vec<NodePtr> {
        let mut node_array: Vec<NodePtr> = vec!{};
        let rc_node = self.unwrap_rc();
        for at in (*rc_node).attributes.borrow().iter() {
            node_array.push(wrap_rc_clone(at));
        }
        return node_array;
    }

    // =================================================================
    /// Returns the first child of the node.
    /// This is equivalent to: nth_child(0)
    ///
    pub fn first_child(&self) -> Option<NodePtr> {
        return self.nth_child(0);
    }

    // =================================================================
    /// Returns the n'th child of the node, or None if there is
    /// no such child.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article><a/>foo<b/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// assert_eq!(root_elem.name(), "article");
    /// let ch0 = root_elem.first_child().unwrap();
    /// assert_eq!(ch0.name(), "a");
    /// let ch1 = root_elem.nth_child(1).unwrap();
    /// assert_eq!(ch1.value(), "foo");
    /// let ch2 = root_elem.nth_child(2).unwrap();
    /// assert_eq!(ch2.name(), "b");
    /// let ch3 = root_elem.nth_child(3);
    /// assert!(ch3.is_none());
    /// ```
    ///
    pub fn nth_child(&self, n: usize) -> Option<NodePtr> {
        let rc_node = self.unwrap_rc();
        if n < rc_node.children.borrow().len() {
            return Some(wrap_rc_clone(&(*rc_node).children.borrow()[n]));
        } else {
            return None
        }
    }

    // =================================================================
    /// Appends the node tree 'new_child' as the last child of
    /// the element node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    ///
    /// let xml_string = r#"<article><a/>foo<b/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_article = doc.get_first_node("//article").unwrap();
    ///
    /// let new_xml_string = r#"<c>baa</c>"#;
    /// let new_doc = new_document(&new_xml_string).unwrap();
    /// let elem_c = new_doc.root_element();
    ///
    /// // Appends 'c' as the last child of 'article'
    /// elem_article.append_child(&elem_c);   // NB. don't pass &new_doc
    /// let result = doc.to_string();
    /// let guess = r#"<article><a/>foo<b/><c>baa</c></article>"#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn append_child(&self, new_child: &NodePtr) {
        let rc_self = self.unwrap_rc();
        let rc_new_child = new_child.unwrap_rc();
        rc_self.children.borrow_mut().push(Rc::clone(&rc_new_child));
        self.clear_document_order();
    }

    // =================================================================
    /// Inserts the child node tree as the previous sibling of 'self' node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    ///
    /// let xml_string = r#"<article><a/><b/><c/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_b = doc.get_first_node("//b").unwrap();
    ///
    /// let new_xml_string = r#"<x>yyy</x>"#;
    /// let new_doc = new_document(&new_xml_string).unwrap();
    /// let elem_x = new_doc.root_element();
    ///
    /// // Inserts 'x' as the previous sibling of 'b'
    /// elem_b.insert_as_previous_sibling(&elem_x);
    /// let result = doc.to_string();
    /// let guess = r#"<article><a/><x>yyy</x><b/><c/></article>"#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn insert_as_previous_sibling(&self, new_node: &NodePtr) {
        let parent = match self.parent() {
            Some(p) => p,
            None => return,
        };
        let n = parent.find_child_index(self);
        if n != usize::MAX {
            let mut rc_parent = parent.unwrap_rc();
            let rc_new_node = new_node.unwrap_rc();

            let mut rc_new_node_dup = make_new_child_rc_node(
                rc_new_node.node_type.clone(),
                &mut rc_parent,
                &rc_new_node.name,
                &rc_new_node.value,
                n);
            shallow_copy_rc_rels(&mut rc_new_node_dup, &rc_new_node);
        }
        self.clear_document_order();
    }

    // =================================================================
    /// Inserts the child node tree as the next sibling of 'self' node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    ///
    /// let xml_string = r#"<article><a/><b/><c/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_b = doc.get_first_node("//b").unwrap();
    ///
    /// let new_xml_string = r#"<x>yyy</x>"#;
    /// let new_doc = new_document(&new_xml_string).unwrap();
    /// let elem_x = new_doc.root_element();
    ///
    /// // Inserts 'x' as the next sibling of 'b'
    /// elem_b.insert_as_next_sibling(&elem_x);
    /// let result = doc.to_string();
    /// let guess = r#"<article><a/><b/><x>yyy</x><c/></article>"#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn insert_as_next_sibling(&self, new_node: &NodePtr) {
        let parent = match self.parent() {
            Some(p) => p,
            None => return,
        };
        let n = parent.find_child_index(self);
        if n != usize::MAX {
            let mut rc_parent = parent.unwrap_rc();
            let rc_new_node = new_node.unwrap_rc();

            let mut rc_new_node_dup = make_new_child_rc_node(
                rc_new_node.node_type.clone(),
                &mut rc_parent,
                &rc_new_node.name,
                &rc_new_node.value,
                n + 1);
            shallow_copy_rc_rels(&mut rc_new_node_dup, &rc_new_node);
        }
        self.clear_document_order();
    }

    // =================================================================
    /// Deletes the child node tree from 'self' node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    ///
    /// let xml_string = r#"<article><a/><b/><c/><d/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_article = doc.get_first_node("//article").unwrap();
    ///
    /// let elem_b = elem_article.get_first_node("b").unwrap();
    ///
    /// // Deletes the child 'b' from 'article'
    /// elem_article.delete_child(&elem_b);
    /// let result = doc.to_string();
    /// let guess = r#"<article><a/><c/><d/></article>"#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn delete_child(&self, target: &NodePtr) {
        let n = self.find_child_index(target);
        if n != usize::MAX {
            let rc_node = self.unwrap_rc();
            (*rc_node).children.borrow_mut().remove(n);
        }
        self.clear_document_order();
    }

    // =================================================================
    /// Replaces the child node tree with 'self' node.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article><a/><b/><c/></article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let elem_b = doc.get_first_node("//b").unwrap();
    ///
    /// let new_xml_string = r#"<x>yyy</x>"#;
    /// let new_doc = new_document(&new_xml_string).unwrap();
    /// let elem_x = new_doc.root_element();
    ///
    /// // Replace 'b' with 'x'
    /// elem_b.replace_with(&elem_x);
    /// let result = doc.to_string();
    /// let guess = r#"<article><a/><x>yyy</x><c/></article>"#;
    /// assert_eq!(result, guess);
    /// ```
    ///
    pub fn replace_with(&self, new_node: &NodePtr) {
        let parent = match self.parent() {
            Some(p) => p,
            None => return,
        };
        self.insert_as_previous_sibling(new_node);
        parent.delete_child(self);
        self.clear_document_order();
    }

    // -----------------------------------------------------------------
    // find_child_index
    //
    fn find_child_index(&self, target: &NodePtr) -> usize {
        let rc_node = self.unwrap_rc();
        let target_node = target.unwrap_rc();
        for (i, ch) in (*rc_node).children.borrow().iter().enumerate() {
            if Rc::ptr_eq(ch, &target_node) {
                return i;
            }
        }
        return usize::MAX;
    }

    // =================================================================
    /// Returns the attribute value of element,
    /// or None if there is no such attribute.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article id="a1">foo</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let root_elem = doc.root_element();
    /// assert_eq!(root_elem.attribute_value("id").unwrap(), "a1");
    /// assert!(root_elem.attribute_value("none").is_none());
    /// ```
    ///
    pub fn attribute_value(&self, name: &str) -> Option<String> {
        let r_index = self.find_attribute_index(name);
        if r_index != usize::MAX {
            let rc_node = self.unwrap_rc();
            return Some((*rc_node).attributes.borrow()[r_index].value.clone());
        } else {
            return None;
        }
    }

    // =================================================================
    /// Updates the attribute value (if already exists) of element,
    /// or adds the attribute (if not exist).
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article id="a1">foo</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let mut root_elem = doc.root_element();
    /// assert_eq!(root_elem.attribute_value("id").unwrap(), "a1");
    /// root_elem.set_attribute("id", "b1");
    /// assert_eq!(root_elem.attribute_value("id").unwrap(), "b1");
    /// root_elem.set_attribute("title", "about xml");
    /// assert_eq!(root_elem.attribute_value("title").unwrap(), "about xml");
    /// assert_eq!(doc.to_string(), r#"<article id="b1" title="about xml">foo</article>"#);
    /// ```
    ///
    pub fn set_attribute(&mut self, name: &str, value: &str) {

        let mut rc_node = self.unwrap_rc();
        let attr_node = make_new_rc_node(NodeType::Attribute,
                            Some(&mut rc_node), name, value);

        let r_index = self.find_attribute_index(name);
        if r_index != usize::MAX {
            (*rc_node).attributes.borrow_mut().remove(r_index);
            (*rc_node).attributes.borrow_mut().insert(r_index, Rc::clone(&attr_node));
        } else {
            (*rc_node).attributes.borrow_mut().push(Rc::clone(&attr_node));
        }
        self.clear_document_order();
    }

    // =================================================================
    /// Deletes the attribute (if already exists) of element.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml_string = r#"<article id="a1">foo</article>"#;
    /// let doc = new_document(&xml_string).unwrap();
    /// let mut root_elem = doc.root_element();
    /// root_elem.delete_attribute("id");
    /// assert!(root_elem.attribute_value("id").is_none());
    /// assert_eq!(doc.to_string(), r#"<article>foo</article>"#);
    /// ```
    ///
    pub fn delete_attribute(&mut self, name: &str) {
        let r_index = self.find_attribute_index(name);
        if r_index != usize::MAX {
            let rc_node = self.unwrap_rc();
            (*rc_node).attributes.borrow_mut().remove(r_index);
        }
        self.clear_document_order();
    }

    // -----------------------------------------------------------------
    //
    fn find_attribute_index(&self, name: &str) -> usize {
        let rc_node = self.unwrap_rc();
        for (i, at) in (*rc_node).attributes.borrow().iter().enumerate() {
            if at.name == name {
                return i;
            }
        }
        return usize::MAX;
    }

    // -----------------------------------------------------------------
    //
    fn clear_document_order(&self) {
        let root = self.root();
        root.unwrap_rc().order.set(0);
    }

    // =================================================================
    /// (Inner Use)
    ///
    pub fn document_order(&self) -> i64 {
        let root = self.root();
        if root.unwrap_rc().order.get() == 0 {
            root.setup_document_order();
        }
        return self.unwrap_rc().order.get();
    }

    // -----------------------------------------------------------------
    //
    fn setup_document_order(&self) {
        self.setup_document_order_sub(1);
    }

    // -----------------------------------------------------------------
    //
    fn setup_document_order_sub(&self, order_beg: i64) -> i64 {
        let mut order = order_beg;
        self.unwrap_rc().order.set(order);
        order += 1;
        for at in self.attributes().iter() {
            at.unwrap_rc().order.set(order);
            order += 1;
        }
        for ch in self.children().iter() {
            order = ch.setup_document_order_sub(order + 1);
        }
        return order;
    }

    // =================================================================
    /// (Inner Use)
    ///
    pub fn rc_clone(&self) -> NodePtr {
        return NodePtr {
            rc_node: Rc::clone(&self.rc_node),
        };
    }

    // -----------------------------------------------------------------
    //
    fn unwrap_rc(&self) -> RcNode {
        return Rc::clone(&self.rc_node);
    }
}

// ---------------------------------------------------------------------
//
fn to_string_with_indent(rc_node: &RcNode, indent: usize, step: usize) -> String {
    match rc_node.node_type {
        NodeType::DocumentRoot => {
            let mut s = String::new();
            for ch in rc_node.children.borrow().iter() {
                s += &to_string_with_indent(ch, indent, step);
            }
            return s;
        },
        NodeType::Element => {
            let mut s = String::new();
            s += &format!("{}<{}", " ".repeat(indent), rc_node.name);
            for at in rc_node.attributes.borrow().iter() {
                s += &format!(r#" {}="{}""#,
                    at.name, encode_entity(&at.value));
            }
            if rc_node.children.borrow().len() == 0 {
                s += &"/>";
            } else {
                s += &">";
                s += &nl_if_positive(step);
                for ch in rc_node.children.borrow().iter() {
                    s += &to_string_with_indent(ch, indent + step, step);
                }
                s += &format!("{}</{}>", " ".repeat(indent), rc_node.name);
            }
            s += &nl_if_positive(step);
            return s;
        },
        NodeType::Text => {
            return format!("{}{}{}",
                &" ".repeat(indent),
                &encode_entity(&(rc_node.value)),
                &nl_if_positive(step));
        },
        NodeType::Comment => {
            return format!("{}<!--{}-->{}",
                &" ".repeat(indent),
                &rc_node.value,
                &nl_if_positive(step));
        },
        NodeType::XMLDecl => {
            return format!("{}<?xml {}?>{}",
                &" ".repeat(indent),
                &rc_node.value,
                &nl_if_positive(step));
        },
        NodeType::Instruction => {
            return format!("{}<?{} {}?>{}",
                &" ".repeat(indent),
                &rc_node.name,
                &rc_node.value,
                &nl_if_positive(step));
        },
        _ => return String::new(),
    }
}

// ---------------------------------------------------------------------
//
fn encode_entity(s: &String) -> String {
    let specs = [
        [ "&", "&amp;" ],
        [ ">", "&gt;" ],
        [ "<", "&lt;" ],
        [ "\"", "&quot;" ],
        [ "'", "&apos;" ],
    ];
    let mut str = s.clone();
    for spec in specs.iter() {
        str = str.replace(spec[0], spec[1]);
    }
    return str
}

// ---------------------------------------------------------------------
//
fn nl_if_positive<'a>(n: usize) -> &'a str {
    return if 0 < n { "\n" } else { "" };
}

