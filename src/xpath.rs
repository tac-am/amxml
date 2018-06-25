//
// xpath.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//
//!
//! (Inner Module) XPath 2.0 Processor.
//! Caller does not need to know about this module.
//!
//! Retrieve or apply function to the nodes on XML DOM tree
//! that match the specified xpath.
//!
//! NodePtr methods eval_xpath(), each_node(), get_first_node(), get_nodeset()
//! accept xpath as argument.
//! cf. Module amxml::dom -> Struct NodePtr -> Methods.
//!
//! # Restrictions
//!
//! - Features related to 'Type' is restrictive, since this processor
//!   does not refer xml schema.
//! - Only String, Integer, Decimal, Double, Boolean types can be used.
//!   Integer type is i64, both Decimal and Double types are f64.
//!
//! # Features that are not implemented yet
//!
//! - XPath 1.0 compatible mode
//! - instance of
//! - treat as
//! - KindTest: SchemaElementTest | SchemaAttributeTest | DocumentTest
//! - KindTest: ElementTest | AttributeTest (form with TypeName)
//! - 'namespace' axis
//! - builtin function 'id'
//! - Many built-in functions that is new in XPath 2.0
//! - Collation (in built-in functions: contains, starts-with, etc.).
//!

use std::error::Error;

use dom::*;
use xpath2::parser::*;
use xpath2::eval::*;

// =====================================================================
//
impl NodePtr {

    // =================================================================
    // XML構文木のあるノードを起点としてxpathを評価し、
    // シーケンス (典型的にはノード集合) を取得する。
    /// Evaluates the xpath and returns the sequence as String format.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml = r#"<root><a v="x"/><a v="y"/></root>"#;
    /// let doc = new_document(xml).unwrap();
    /// let result = doc.eval_xpath(r#"some $a in /root/a satisfies $a/@v = "y" "#).unwrap();
    /// assert_eq!(result, "(true)");
    /// ```
    ///
    /// # Errors
    ///
    /// - When syntax error or unimplemented feature in xpath.
    ///
    pub fn eval_xpath(&self, xpath: &str) -> Result<String, Box<Error>> {
        let xnode = compile_xpath(&String::from(xpath))?;
        let result = match_xpath(self, &xnode)?;
        return Ok(result.to_string());
    }

    // =================================================================
    // XML構文木のあるノードを起点として、xpathに合致するノード集合を取得し、
    // その最初のノードを返す。
    /// Retrieves the first node that match with xpath.
    /// Returns None if not found,
    /// or when syntax error or unimplemented feature in xpath.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml = r#"<root img="basic"><a img="a1"/><a img="a2"/></root>"#;
    /// let doc = new_document(xml).unwrap();
    /// let node = doc.get_first_node("//a").unwrap();
    /// assert_eq!(node.attribute_value("img").unwrap(), "a1");
    /// ```
    ///
    /// # Panics
    ///
    /// - When syntax error or unimplemented feature in xpath.
    ///
    pub fn get_first_node(&self, xpath: &str) -> Option<NodePtr> {
        let node_set_array = match self.get_nodeset(xpath) {
            Ok(n) => n,
            Err(_) => return None,
        };
        if node_set_array.len() != 0 {
            return Some(node_set_array[0].rc_clone());
        } else {
            return None;
        }
    }

    // =================================================================
    // XML構文木のあるノードを起点として、xpathに合致する各ノードに対して
    // 函数fnの処理を施す。
    /// Applies func to each nodes that match with xpath.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml = r#"<root img="basic"><a img="a1"/><a img="a2"/></root>"#;
    /// let doc = new_document(xml).unwrap();
    /// let mut img = String::new();
    /// doc.each_node("/root/a", |n| {
    ///     img += n.attribute_value("img").unwrap().as_str();
    /// });
    /// assert_eq!("a1a2", img);
    /// ```
    ///
    /// # Errors
    ///
    /// - When syntax error or unimplemented feature in xpath.
    ///
    pub fn each_node<F>(&self, xpath: &str, mut func: F) -> Result<(), Box<Error>>
        where F: FnMut(NodePtr) -> () {

        let node_set_array = self.get_nodeset(xpath)?;
        for node in node_set_array {
            func(node.rc_clone());
        }
        return Ok(());
    }

    // =================================================================
    // XML構文木のあるノードを起点として、xpathに合致するノード集合を
    // 文書順で取得する。
    /// Retrieves all nodes that match with xpath in document order.
    ///
    /// # Examples
    ///
    /// ```
    /// use amxml::dom::*;
    /// let xml = r#"<root img="basic"><a img="a1"/><a img="a2"/></root>"#;
    /// let doc = new_document(xml).unwrap();
    /// let nodeset = doc.get_nodeset("//a").unwrap();
    /// assert_eq!(nodeset[0].attribute_value("img").unwrap(), "a1");
    /// assert_eq!(nodeset[1].attribute_value("img").unwrap(), "a2");
    /// ```
    ///
    /// # Errors
    ///
    /// - When syntax error or unimplemented feature in xpath.
    ///
    pub fn get_nodeset(&self, xpath: &str) -> Result<Vec<NodePtr>, Box<Error>> {
        let xnode = compile_xpath(&String::from(xpath))?;

        let result = match_xpath(self, &xnode)?;

        let nodeset = result.to_nodeset();
        return Ok(nodeset);
    }
}

// =====================================================================
//
#[cfg(test)]
mod test {
    use super::*;

    use xpath2::helpers::compress_spaces;
    use xpath2::helpers::subtest_xpath;
    use xpath2::helpers::subtest_eval_xpath;

    // -----------------------------------------------------------------
    // - child::para は文脈ノードの子の para 要素すべてを選択する。
    // - para は文脈ノードの para 子要素すべてを選択する。
    // - child::* は文脈ノードの子要素すべてを選択する。
    // - * は文脈ノードの子要素すべてを選択する。
    //
    #[test]
    fn test_sample_01() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base">
        <para img="春"/>
        <div img="夏"/>
        <para img="秋"/>
        <div img="冬"/>
    </chap>
</root>
        "#);

        subtest_xpath("01", &xml, false, &[
            ( "child::para", "春秋" ),
            ( "para", "春秋" ),
            ( "child::*", "春夏秋冬" ),
            ( "*", "春夏秋冬" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::text() は文脈ノードの、すべての子テキストノードを選択する。
    // - text() は文脈ノードの子テキストノードすべてを選択する。
    //
    #[test]
    fn test_sample_02() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base">
        <para img="春">はる</para>
        なつ
        <para img="秋">あき</para>
        ふゆ
    </chap>
</root>
        "#);

        subtest_xpath("02", &xml, true, &[
            ( "child::text()", "なつふゆ" ),
            ( "text()", "なつふゆ" ),
            // -------------------------------------
            ( "text()[preceding-sibling::*[1]/@img = '春']", "なつ" ),
            ( "text()[following-sibling::*[1]/@img = '秋']", "なつ" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::node() はノード型に関係なく、文脈ノードのすべての子ノードを選択する。
    //
    #[test]
    fn test_sample_03() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base">
        <para img="春">はる</para>
        なつ
        <sub img="秋">あき</sub>
        ふゆ
        <!--季節-->
    </chap>
</root>
        "#);

        subtest_xpath("03", &xml, true, &[
            ( "child::node()", "なつふゆ季節" ),
            ( "node()", "なつふゆ季節" ),
            // -------------------------------------
            ( "text()", "なつふゆ" ),
            ( "comment()", "季節" ),
        ]);
        subtest_xpath("03", &xml, false, &[
            ( "child::node()", "春秋" ),
            ( "node()", "春秋" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - attribute::name は文脈ノードの name 属性を選択する。
    // - @name は文脈ノードの name 属性を選択する。
    // - attribute::* は文脈ノードのすべての属性を選択する。
    // - @* は文脈ノードの属性すべてを選択する。
    //
    #[test]
    fn test_sample_04() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap base="base" name="四季" ruby="しき" ns:e="seasons">
        <para img="春">はる</para>
    </chap>
</root>
        "#);

        subtest_xpath("04", &xml, true, &[
            ( "attribute::name", "四季" ),
            ( "@name", "四季" ),
            ( "attribute::*", "base四季しきseasons" ),
            ( "@*", "base四季しきseasons" ),
            // ----------------------------------------------
            ( "attribute::ns:e", "seasons" ),
            ( "@ns:e", "seasons" ),
            ( "attribute::ns:*", "seasons" ),
            ( "@ns:*", "seasons" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - descendant::para は文脈ノードの子孫の para 要素すべてを選択する。
    // - .//para は文脈ノードの para 子孫要素すべてを選択する。
    //
    #[test]
    fn test_sample_05() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季">
        <para img="春">はる</para>
        <div img="夏">
            <para img="夏" />
            <span>
                <para img="秋" />
            </span>
        </div>
    </para>
</root>
        "#);

        subtest_xpath("05", &xml, false, &[
            ( "descendant::para", "春夏秋" ),
            ( ".//para", "春夏秋" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - ancestor::div は文脈ノードの先祖の div 要素すべてを選択する。
    // - ancestor-or-self::div は文脈ノードの先祖の div 要素すべてに加え、文脈ノード自身が div 要素ならば自身も選択する。
    //
    #[test]
    fn test_sample_06() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <div img="四季">
        <div img="春">はる</div>
        <section img="夏">
            <div base="base" img="秋" />
            <span>
                <div img="冬" />
            </span>
        </section>
    </div>
</root>
        "#);

        subtest_xpath("06", &xml, false, &[
            ( "ancestor::div", "四季" ),
            ( "ancestor-or-self::div", "四季秋" ),
        ]);
    }


    // -----------------------------------------------------------------
    // - descendant-or-self::para は文脈ノードの子孫の para 要素すべてに加え、文脈ノード自身が para 要素ならば自身も選択する。
    //
    #[test]
    fn test_sample_07() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季">
        <para img="春">はる</para>
        <section img="夏">
            <para img="秋" />
            <span>
                <para img="冬" />
            </span>
        </section>
    </para>
</root>
        "#);

        subtest_xpath("07", &xml, false, &[
            ( "descendant-or-self::para", "四季春秋冬" ),
            // -----------------------------------------------------
            ( ".//para", "春秋冬" ),
            ( "..//para", "四季春秋冬" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - self::para は文脈ノード自身が para 要素ならば自身を選択し、そうでなければ何も選択しない。
    //
    #[test]
    fn test_sample_08() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季">
        <para img="春">はる</para>
    </para>
</root>
        "#);

        subtest_xpath("08a", &xml, false, &[
//            ( "self::para", "四季" ),
        ]);

        let xml2 = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <section base="base" img="四季">
        <para img="春">はる</para>
    </section>
</root>
        "#);

        subtest_xpath("08b", &xml2, false, &[
            ( "self::para", "" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::chapter/descendant::para は文脈ノードの子の chapter 要素すべての子孫の para 要素すべてを選択する。
    // - chapter//para は文脈ノードの chapter 子要素の para 子孫要素すべてを選択する。
    // - child::*/child::para は文脈ノードの孫（子の子）の para 要素すべてを選択する。
    // - */para は文脈ノードの para 孫要素すべてを選択する。
    //
    #[test]
    fn test_sample_09() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <division base="base" img="四季">
        <chapter img="chap-1">
            <para img="春">はる</para>
            <note img="梅雨">つゆ</note>
            <span>
                <para img="夏">なつ</para>
            </span>
        </chapter>
        <chapter img="chap-2">
            <para img="秋">あき</para>
            <span>
                <para img="冬">ふゆ</para>
            </span>
        </chapter>
    </division>
</root>
        "#);

        subtest_xpath("09", &xml, false, &[
            ( "child::chapter/descendant::para", "春夏秋冬" ),
            ( "chapter//para", "春夏秋冬" ),
            ( "child::*/child::para", "春秋" ),
            ( "*/para", "春秋" ),
        ]);

    }

    // -----------------------------------------------------------------
    // - / は文書ルートを選択する（文書ルートは常に文書要素の親となる）。
    //
    #[test]
    fn test_sample_10() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<?style-sheet alt="1" src="sample.css"?>
<root>
    <div base="base" img="四季">
        <chapter img="chap-1" />
    </div>
</root>
        "#);

        let doc = new_document(&xml).unwrap();
        let base_node = doc.get_first_node(r#"//*[@base="base"]"#).unwrap();

        let document_root = base_node.get_first_node("/").unwrap();

        let pi = document_root.get_first_node("processing-instruction()").unwrap();
        assert_eq!(pi.name(), "style-sheet");
    }

    // -----------------------------------------------------------------
    // - /descendant::para は文脈ノードと同じ文書内の para 要素すべてを選択する。
    // - //para は文書ルートの para 子孫要素すべてを選択する。
    //
    #[test]
    fn test_sample_11() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季">
        <para img="春">はる</para>
        <div img="夏">
            <para img="夏" />
            <span>
                <para img="秋" />
            </span>
        </div>
    </para>
</root>
        "#);

        subtest_xpath("11", &xml, false, &[
            ( "/descendant::para", "四季春夏秋" ),
            ( "//para", "四季春夏秋" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - /descendant::olist/child::item は文脈ノードと同じ文書内にある item 要素のうち、 olist 要素を親に持つものすべてを選択する。
    // - //olist/item は、文脈ノードと同じ文書内にある item 要素のうち、olist 要素を親に持つものすべてを選択する。
    //
    #[test]
    fn test_sample_12() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季" />
    <div>
        <item img="x1">XX</item>
    </div>
    <olist>
        <item img="春" />
        <item img="夏" />
        <ulist>
            <item img="秋" />
        </ulist>
    </olist>
</root>
        "#);

        subtest_xpath("12", &xml, false, &[
            ( "/descendant::olist/child::item", "春夏" ),
            ( "//olist/item", "春夏" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::para[position()=1] は文脈ノードの子の para 要素のうち、最初のものを選択する。
    // - para[1] は文脈ノードの最初の para 子要素を選択する。
    // - child::para[position()=last()] は文脈ノードの子の para 要素のうち、最後のものを選択する。
    // - para[last()] は文脈ノードの最後の para 子要素を選択する。
    // - child::para[position()=last()-1] は文脈ノードの最後から 2 番目の para 子要素を選択する。
    // - child::para[position()>1] は文脈ノードの para 子要素のうち、最初のものを除くすべてを選択する。
    //
    #[test]
    fn test_sample_13() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para base="base" img="四季">
        <note img="きせつ"/>
        <para img="春">はる</para>
        <para img="夏">なつ</para>
        <para img="秋">あき</para>
        <note img="季節"/>
    </para>
</root>
        "#);

        subtest_xpath("13", &xml, false, &[
            ( "child::para[position()=1]", "春" ),
            ( "para[1]", "春" ),
            ( "child::para[position()=last()]", "秋" ),
            ( "para[last()]", "秋" ),
            ( "child::para[position()=last()-1]", "夏" ),
            ( "child::para[position()>1]", "夏秋" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - following-sibling::chapter[position()=1] は文脈ノードの次の chapter 同胞要素を選択する。
    // - preceding-sibling::chapter[position()=1] は文脈ノードの前の chapter 同胞要素を選択する。
    //
    #[test]
    fn test_sample_14() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para img="四季">
        <chapter img="甲"/>
        <chapter img="乙"/>
        <chapter img="丙"/>
        <chapter img="丁" base="base"/>
        <chapter img="戊"/>
        <chapter img="己"/>
        <chapter img="庚"/>
        <chapter img="辛"/>
        <chapter img="壬"/>
        <chapter img="癸"/>
    </para>
</root>
        "#);

        subtest_xpath("14", &xml, false, &[
            ( "following-sibling::chapter[position()=1]", "戊" ),
            ( "preceding-sibling::chapter[position()=1]", "丙" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - /descendant::figure[position()=42] は文書内の 42 番目の figure 要素を選択する。
    //
    #[test]
    fn test_sample_15() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <para img="p1">
        <figure img="甲"/>
        <span>
            <figure img="乙"/>
        </span>
    </para>
    <para base="base" img="p2">
        <figure img="丙"/>
        <span>
            <figure img="丁"/>
            <figure img="戊"/>
        </span>
    </para>
    <para img="p3">
        <figure img="己"/>
        <figure img="庚"/>
        <figure img="辛"/>
        <figure img="壬"/>
        <figure img="癸"/>
    </para>
</root>
        "#);

        subtest_xpath("15", &xml, false, &[
            ( "/descendant::figure[position()=4]", "丁" ),
            ( "/descendant::figure[position()=6]", "己" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - /child::doc/child::chapter[position()=5]/child::section[position()=2] は文書要素 doc の 5 番目の chapter 子要素の、2 番目の section 子要素を選択する。
    // - /doc/chapter[5]/section[2] はルートノードの doc 子要素（文書要素）の 5 番目の chapter 子要素の 2 番目の section 子要素を選択する。
    //
    #[test]
    fn test_sample_16() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<doc>
    <chapter img="c1">
    </chapter>
    <chapter img="c2">
        <section img="S21" base="base"/>
    </chapter>
    <chapter img="c3">
    </chapter>
    <chapter img="c4">
    </chapter>
    <chapter img="c5">
        <section img="S51"/>
        <section img="S52"/>
        <section img="S53"/>
    </chapter>
    <chapter img="c6">
    </chapter>
</doc>
        "#);

        subtest_xpath("20", &xml, false, &[
            ( "/child::doc/child::chapter[position()=5]/child::section[position()=2]", "S52" ),
            ( "/doc/chapter[5]/section[2]", "S52" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::para[attribute::type="warning"] は文脈ノードの para 子要素のうち、 type 属性の値が warning のものすべてを選択する。
    // - para[@type="warning"] は文脈ノードの para 子要素のうち、 type 属性の値が warning になるものすべてを選択する。
    // - child::para[attribute::type='warning'][position()=5] は文脈ノードの para 子要素で type 属性の値が warning のものから、 5 番目の要素を選択する。
    // - para[@type="warning"][5] は文脈ノードの para 子要素のうち、 type 属性の値が warning になるものの中から 5 番目のものを選択する。
    // - child::para[position()=5][attribute::type="warning"] は、文脈ノードの 5 番目の para 子要素の type 属性の値が warning であれば、その para 子要素を選択する。
    // - para[5][@type="warning"] は、文脈ノードの 5 番目の para 子要素の type 属性の値が warning ならば、その子要素を選択する。
    //
    #[test]
    fn test_sample_17() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<doc>
    <chapter img="c1" base="base">
        <para img="甲" type="warning"/>
        <para img="乙"/>
        <para img="丙"/>
        <para img="丁" type="warning"/>
        <para img="戊" type="warning"/>
        <para img="己"/>
        <para img="庚" type="warning"/>
        <para img="辛" type="warning"/>
        <para img="壬"/>
        <para img="癸" type="warning"/>
    </chapter>
</doc>
        "#);

        subtest_xpath("17", &xml, false, &[
            ( "child::para[attribute::type='warning']", "甲丁戊庚辛癸" ),
            ( "para[@type='warning']", "甲丁戊庚辛癸" ),
            ( "child::para[attribute::type='warning'][position()=5]", "辛" ),
            ( "para[@type='warning'][5]", "辛" ),
            ( "child::para[position()=5][attribute::type='warning']", "戊" ),
            ( "para[5][@type='warning']", "戊" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::chapter[child::title='Introduction'] は、文脈ノードの chapter 子要素のうち、 文字列値 が Introduction になる title 要素を子要素に１個以上持つものすべてを選択する。
    // - chapter[title="Introduction"] は、文脈ノードの chapter 子要素のうち、 文字列値 が Introduction になる title 子要素を１個以上持つものすべてを選択する。
    // - child::chapter[child::title] は、文脈ノードの chapter 子要素のうち、 title 子要素を１個以上持つものすべてを選択する。
    // - chapter[title] は文脈ノードの chapter 子要素のうち、 title 子要素を１個以上持つものすべてを選択する。
    //
    #[test]
    fn test_sample_18() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <doc base="base">
        <chapter img="春">
        </chapter>
        <chapter img="夏">
            <title><bold>I</bold>ntroduction</title>
        </chapter>
        <chapter img="秋">
            <title>NextStep</title>
        </chapter>
        <chapter img="冬">
        </chapter>
    </doc>
</root>
        "#);

        subtest_xpath("18", &xml, false, &[
            ( "child::chapter[child::title='Introduction']", "夏" ),
            ( "chapter[title='Introduction']", "夏" ),
            ( "child::chapter[child::title]", "夏秋" ),
            ( "chapter[title]", "夏秋" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - child::*[self::chapter or self::appendix] は、文脈ノードの chapter 子要素すべてと appendix 子要素すべてを選択する。
    // - child::*[self::chapter or self::appendix][position()=last()] は、文脈ノードの chapter 子要素すべてと appendix 子要素すべてを併せた中から最後のものを選択する。
    //
    #[test]
    fn test_sample_19() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <doc base="base">
        <chapter img="春">
        </chapter>
        <note img="梅雨">ばいう</note>
        <chapter img="夏">
            なつ
        </chapter>
        <note img="重陽" />
        <chapter img="秋">
            あき
        </chapter>
        <chapter img="冬">
        </chapter>
        <appendix img="正月">
        </appendix>
        <appendix img="四季">
        </appendix>
    </doc>
</root>
        "#);

        subtest_xpath("19", &xml, false, &[
            ( "child::*[self::chapter or self::appendix]", "春夏秋冬正月四季" ),
            ( "child::*[self::chapter or self::appendix][position()=last()]", "四季" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - . は文脈ノードを選択する。
    // - .. は文脈ノードの親ノードを選択する。
    // - ../@lang は文脈ノードの親ノードの lang 属性を選択する。
    //
    #[test]
    fn test_sample_20() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root lang="en" img="root">
    <doc lang="ja" img="四季">
        <chapter base="base" lang="ja-JP" img="春">
        </chapter>
    </doc>
</root>
        "#);

        subtest_xpath("20", &xml, false, &[
            ( ".", "春" ),
            ( "..", "四季" ),
        ]);
        subtest_xpath("20", &xml, true, &[
            ( "../@lang", "ja" ),
        ]);
    }

    // -----------------------------------------------------------------
    // - employee[@secretary and @assistant] は文脈ノードの employee 子要素のうち、 secretary 属性と assistant 属性の両方を持つものすべてを選択する。
    //
    #[test]
    fn test_sample_21() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <hr base="base">
        <employee img="John" secretary="t">John</employee>
        <employee img="Jack" assistant="t">Jack</employee>
        <employee img="Betty" secretary="t" assistant="t">Betty</employee>
        <employee img="Tom" president="t">Tom</employee>
    </hr>
</root>
        "#);

        subtest_xpath("21", &xml, false, &[
            ( "employee[@secretary and @assistant]", "Betty" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 注記： ロケーションパス //para[1] はロケーションパス /descendant::para[1] と同じではない。 後者は、最初の para 子孫要素を選択する一方、前者は、その要素の親にとって最初の para 子要素になるような para 子孫要素すべてを選択する。
    //
    #[test]
    fn test_sample_51a() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <chap img="上" base="base">
        <para img="春"/>
        <para img="夏"/>
    </chap>
    <chap img="下">
        <para img="秋"/>
        <para img="冬"/>
    </chap>
</root>
        "#);

        subtest_xpath("51a", &xml, false, &[
            ( "//para[1]", "春秋" ),
            ( "/descendant::para[1]", "春" ),
        ]);
    }

    // -----------------------------------------------------------------
    // 「//」と文書順の関係。
    // 注記： ロケーションパス //para[1] はロケーションパス /descendant::para[1] と同じではない。 後者は、最初の para 子孫要素を選択する一方、前者は、その要素の親にとって最初の para 子要素になるような para 子孫要素すべてを選択する。
    //
    #[test]
    fn test_sample_51b() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<file base="base">
    <body>
        <unit img="A"/>
        <group>
            <unit img="B"/>
            <unit img="C"/>
        </group>
        <unit img="D"/>
        <unit img="E"/>
    </body>
</file>
        "#);

        subtest_xpath("51b", &xml, false, &[
            ( "body//unit", "ABCDE" ),
            ( "body//unit[2]", "CD" ),                      // [X]
            ( "body/descendant-or-self::*/unit[2]", "CD" ), // [X] と同じ
            ( "body/descendant-or-self::unit[2]", "B" ),    // [Y]: [X] とは違う
            ( "body/descendant::unit[2]", "B" ),            // [Z]: [Y] と実質的に同じ
            ( "body//unit[3]", "E" ),
            ( "(body//unit)[1]", "A" ),
        ]);
        // xml に現れる <unit> すべてのうち、文書順で2番目 (B) を得たければ、
        // [X] ではなく [Y] のように書かなければならない。
    }

    // -----------------------------------------------------------------
    // 注記： 述語 の意味は、適用する軸に大きく依存する。 例えば preceding::foo[1] は、述語 [1] を適用する軸が preceding 軸になるので、逆文書順で最初の foo 要素を返す。 一方、 (preceding::foo)[1] では、述語 [1] を適用する軸が child 軸になるので、文書順で最初の foo 要素を返す。
    //
    #[test]
    fn test_sample_52() {
        let xml = compress_spaces(r#"
<?xml version='1.0' encoding='UTF-8'?>
<root>
    <foo img="上">
        <foo img="春"/>
        <baa img="夏"/>
    </foo>
    <foo img="下">
        <foo img="秋" base="base"/>
        <baa img="冬"/>
    </foo>
</root>
        "#);

        subtest_xpath("52", &xml, false, &[
            ( "preceding::foo[1]", "春" ),
            // preceding 軸の場合、先祖ノードは除外されるので、
            // 「夏、春、上、xml」の順にたどる。
            // そのうち foo は「春、上」であり、その1番なので
            // 「春」になる。
            // -----------------------------------------------------
            ( "(preceding::foo)[1]", "上" ),
            // 同様であるが、この場合は正順にたどって1番なので
            // 「上」になる。
            // =====================================================

            ( "/descendant::foo[preceding::foo[1]/@img = '下']", "" ),
            ( "/descendant::foo[preceding::foo[1]/@img = '春']", "下秋" ),
            // [1] は逆順で1番
            // 「/descendant::foo」で見つかる、上春下秋のうち、
            // 上: 「preceding::foo」が空なのでその [1] も空
            // 春: 「preceding::foo」が空なのでその [1] も空
            //     - 「上」はancestorなのでprecedingに入らない
            // 下: 「preceding::foo」は「春上」、その [1] は「春」
            // 秋: 「preceding::foo」は「春上」、その [1] は「春」
            ( "/descendant::foo[preceding::foo[2]/@img = '上']", "下秋" ),
            // 同様にして、
            // 下: 「preceding::foo」は「春上」、その [2] は「上」
            // 秋: 「preceding::foo」は「春上」、その [2] は「上」

            ( "/descendant::foo[(preceding::foo)[1]/@img = '上']", "下秋" ),
            // [1] は正順で1番
            // 「/descendant::foo」で見つかる、上春下秋のうち、
            // 上: 「preceding::foo」が空なのでその [1] も空
            // 春: 「preceding::foo」が空なのでその [1] も空
            //     - 「上」はancestorなのでprecedingに入らない
            // 下: 「preceding::foo」は「上春」、その [1] は「上」
            // 秋: 「preceding::foo」は「上春」、その [1] は「上」
            ( "/descendant::foo[(preceding::foo)[2]/@img = '春']", "下秋" ),
            // 同様にして、
            // 下: 「preceding::foo」は「上春」、その [2] は「春」
            // 秋: 「preceding::foo」は「上春」、その [2] は「春」

            ( "/descendant::foo[(preceding::foo)[1]/@img = '春']", "" ),
        ]);
    }

    // -----------------------------------------------------------------
    //
    #[test]
    fn test_basic_xpath() {
        let xml = compress_spaces(r#"
<root base="base">
    <student>
        <name>George</name>
        <exam subject="math" point="70"/>
        <exam subject="science" point="90"/>
    </student>
    <student>
        <name>Harry</name>
        <exam subject="math" point="85"/>
        <exam subject="science" point="95"/>
    </student>
    <student>
        <name>Ivonne</name>
        <exam subject="math" point="60"/>
        <exam subject="science" point="75"/>
    </student>
</root>
        "#);

        subtest_eval_xpath("basic_xpath", &xml, &[
            ( r#"
for $student in /root/student return
    ($student/name/text(),
     every $exam in $student/exam satisfies number($exam/@point) > 80)
              "#,
              r#"(George, false, Harry, true, Ivonne, false)"# ),
        ]);








//        let xml = compress_spaces(r#"
//<root img="basic" base="base">
//    <a img="a1" id="i1" />
//    <a img="a2" id="i2" />
//    <b img="b1" id="i1" />
//    <b img="b2" id="i2" />
//</root>
//        "#);
//
//        subtest_xpath("basic_xpath", &xml, false, &[
//            ( "/root", "basic" ),
//            ( "/root/a", "a1a2" ),
//            ( "/root/a[2]", "a2" ),
//            ( "/root/*[name() = 'a']", "a1a2" ),
//            ( "/root/a[9 idiv 4]", "a2" ),
//            ( "/root/a[8 mod 3]", "a2" ),
//            ( "/root/a[@id='i2']", "a2" ),
//            ( "/root/*[@id='i2']", "a2b2" ),
//            ( "//a", "a1a2" ),
//            ( "/root/a[@img='a2']/following-sibling::node()", "b1b2" ),
//            ( "/root/b[@img='b1']/preceding-sibling::node()", "a1a2" ),
//        ]);
    }

}

