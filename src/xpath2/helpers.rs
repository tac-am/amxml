//
// xpath2/helpers.rs
//
// amxml: XML processor with XPath.
// Copyright (C) 2018 KOYAMA Hiro <tac@amris.co.jp>
//

use dom::new_document;
use xpath2::eval::xnode_dump;
use xpath2::parser::compile_xpath;

// -----------------------------------------------------------------
// 行頭の空白および改行を除去する。
//
#[allow(dead_code)]
pub fn compress_spaces(s: &str) -> String {
    return s.to_string().split("\n").map(|s| s.trim_left()).collect();
}

// -----------------------------------------------------------------
// xpath評価のテスト: each_node。
//
#[allow(dead_code)]
pub fn subtest_xpath(id: &str, xml: &str, check_val: bool, test_specs: &[(&str, &str)]) {

    let doc = new_document(xml).unwrap();
    let base_node = match doc.get_first_node(r#"//*[@base="base"]"#) {
        Some(n) => n,
        None => doc,
    };
    for test_spec in test_specs.iter() {
        let xpath = test_spec.0;
        let guess = test_spec.1;

        if let Ok(xnode) = compile_xpath(&String::from(xpath)) {
            print!("\n{}", xnode_dump(&xnode));
        }

        let mut actual = String::new();
        let r = base_node.each_node(xpath, |n| {
            if check_val {
                actual += n.value().as_str();
            } else {
                if let Some(val) = n.attribute_value("img") {
                    actual += val.as_str();
                }
            }
        });
        match r {
            Ok(()) => {
            assert_eq!(guess, &actual,
                "[id = {}]: xpath = {}: guess = {}, actual = {}",
                id, xpath, guess, actual);
            },
            Err(e) => {
                println!("Err: {}", e);
                let v: Vec<&str> = e.description().split(':').collect();
                let actual = v[0];
                assert_eq!(guess, actual,
                    "[id = {}] (Err): xpath = {}: guess = {}, actual = {}",
                    id, xpath, guess, actual);
            },
        }
    }
}

// -----------------------------------------------------------------
// xpath評価のテスト: eval_xpath。
//
#[allow(dead_code)]
pub fn subtest_eval_xpath(id: &str, xml: &str, test_specs: &[(&str, &str)]) {
    let doc = new_document(&xml).unwrap();
    let base_node = match doc.get_first_node(r#"//*[@base="base"]"#) {
        Some(n) => n,
        None => doc,
    };
    for test_spec in test_specs.iter() {
        let xpath = test_spec.0;
        let guess = test_spec.1;

        if let Ok(xnode) = compile_xpath(&String::from(xpath)) {
            print!("\n{}", xnode_dump(&xnode));
        }

        match base_node.eval_xpath(xpath) {
            Ok(actual) => {
                println!("Seq: {}", actual);
                assert_eq!(guess, actual,
                    "[id = {}]: xpath = {}: guess = {}, actual = {}",
                    id, xpath, guess, actual);
            },
            Err(e) => {
                println!("Err: {}", e);
                let v: Vec<&str> = e.description().split(':').collect();
                let actual = v[0];
                assert_eq!(guess, actual,
                    "[id = {}] (Err): xpath = {}: guess = {}, actual = {}",
                    id, xpath, guess, actual);
            },
        }
    }
}
