use js_sys::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{HtmlElement, HtmlTemplateElement, Node};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn collector() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let el = document.create_element("div").unwrap();
    let node: &Node = &el;

    {
        el.set_inner_html("<span></span>");
        let test_node = node.first_child().unwrap();
        let test_el = test_node.unchecked_ref::<HtmlElement>();
        assert_eq!(stage0::collector(test_el), Ok(None));
    }

    {
        el.set_inner_html("<span #test-attr></span>");
        let test_node = node.first_child().unwrap();
        let test_el = test_node.unchecked_ref::<HtmlElement>();
        assert_eq!(stage0::collector(test_el), Ok(Some("test-attr".to_owned())));
    }

    {
        let text = document.create_text_node("#test-text");
        assert_eq!(stage0::collector(&text), Ok(Some("test-text".to_owned())));
    }
}

#[wasm_bindgen_test]
fn compile_str() {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let compiler_template = document
        .create_element("template")
        .unwrap()
        .unchecked_into::<HtmlTemplateElement>();
    let tree_walker = document.create_tree_walker(&document).unwrap();

    {
        let result = stage0::compile_str(&compiler_template, &tree_walker, "<div></div>");
        assert!(result.is_ok());
        let template = result.unwrap();
        let refs = template.collect(&tree_walker).unwrap();
        assert!(refs.is_empty());
    }

    {
        let result = stage0::compile_str(&compiler_template, &tree_walker, "<div #test></div>");
        assert!(result.is_ok());
        let template = result.unwrap();
        let refs = template.collect(&tree_walker).unwrap();
        assert!(refs.len() == 1);
        assert!(refs.contains_key("test"));
    }

    {
        let result = stage0::compile_str(&compiler_template, &tree_walker, "<div #foo>#bar</div>");
        assert!(result.is_ok());
        let template = result.unwrap();
        let refs = template.collect(&tree_walker).unwrap();
        assert!(refs.len() == 2);
        assert!(refs.contains_key("foo"));
        assert!(refs.contains_key("bar"));

        let foo = refs.get("foo").unwrap();
        assert_eq!(foo.node_type(), Node::ELEMENT_NODE);
        assert_eq!(foo.node_name(), "DIV");

        let bar = refs.get("bar").unwrap();
        assert_eq!(bar.node_type(), Node::TEXT_NODE);
        assert_eq!(bar.node_name(), "#text");
        assert_eq!(bar.node_value(), Some("".to_owned()));
    }
}
