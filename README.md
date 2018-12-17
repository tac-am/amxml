# amxml

Rust XML processor with some features of XPath 2.0 / 3.0 / 3.1.

# Building DOM tree from XML document string

Building DOM tree can be done by calling <strong>new_document()</strong> function.
The DOM tree can be turned into String.

```rust
use amxml::dom::*;
let xml_string = r#"<?xml version="1.0"?><article>foo</article>"#;
let doc = new_document(&xml_string).unwrap();
let result = doc.to_string();
assert_eq!(result, xml_string);
```

# Navigating DOM tree

Navigating DOM tree, or retrieving the DOM node, can be done by
<strong>root_element()</strong>, <strong>parent()</strong>,
<strong>first_child()</strong>, <strong>nth_child()</strong>,
<strong>attribute_value()</strong> methods.

See the description and example of corresponding method.

# Retrieving the DOM node by XPath

But more convenient way for retrieving the DOM node is, perhaps,
using XPath, especially when the search criteria is not trivial.

First XPath example is somewhat straightforward.
<strong>each_node()</strong> method visits the DOM nodes
that match with the given XPath,
and apply the function (closure) to these nodes.

```rust
use amxml::dom::*;
let xml = r#"<root><a img="a1"/><a img="a2"/></root>"#;
let doc = new_document(xml).unwrap();
let mut img = String::new();
doc.each_node("/root/a", |n| {
    img += n.attribute_value("img").unwrap().as_str();
});
assert_eq!(img, "a1a2");
```

Second XPath example is more complex.
This finds the clerk OR engineer (NOT advisor) who has no subordinates.
Note that clerks and enginners appear in <em>document order</em>
in <strong>each_node()</strong> iteration.

```rust
use amxml::dom::*;
let xml = r#"
<root>
    <clerk name="Ann">
        <advisor name="Betty"/>
        <clerk name="Charlie"/>
    </clerk>
    <engineer name="Dick">
        <engineer name="Emily"/>
    </engineer>
    <clerk name="Fred"/>
</root>
"#;
let doc = new_document(xml).unwrap();
let root = doc.root_element();
let xpath = "(//clerk | //engineer)[count(./*) = 0]";
let mut names = String::new();
root.each_node(xpath, |n| {
    names += n.attribute_value("name").unwrap().as_str();
    names += "; ";
});
assert_eq!(names, "Charlie; Emily; Fred; ");

```

Also see the description and example of <strong>each_node()</strong>,
<strong>get_first_node()</strong>, <strong>get_nodeset()</strong> methods.

# Evaluating XPath

XPath can also be used to evaluate for the DOM tree and get boolean,
numeric, string values as well as DOM node.
The example below lists up the students, and whether or not each student
got 80 or more points in <em>every</em> (not <em>some</em>) examination.

```rust
use amxml::dom::*;
let xml = r#"
<root>
    <student>
        <name>George</name>
        <exam subject="math" point="70"/>
        <exam subject="science" point="90"/>
    </student>
    <student>
        <name>Harry</name>
        <exam subject="math" point="80"/>
        <exam subject="science" point="95"/>
    </student>
    <student>
        <name>Ivonne</name>
        <exam subject="math" point="60"/>
        <exam subject="science" point="75"/>
    </student>
</root>
"#;
let doc = new_document(xml).unwrap();
let root = doc.root_element();
let xpath= r#"
for $student in /root/student return
    ($student/name/text(),
     every $exam in $student/exam satisfies number($exam/@point) >= 80)
"#;
let result = root.eval_xpath(xpath).unwrap();
assert_eq!(result.to_string(), "(George, false, Harry, true, Ivonne, false)");

```

# Manipurating the DOM node

Inserting / replacing / deleting the DOM node can be done by
methods like <strong>append_child()</strong>,
<strong>insert_as_previous_sibling()</strong>, 
<strong>insert_as_next_sibling()</strong>,
<strong>delete_child()</strong>, <strong>replace_with()</strong>,
<strong>set_attribute()</strong>, <strong>delete_attribute()</strong> methods.

See the description and example of corresponding method.

