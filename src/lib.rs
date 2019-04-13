use lazy_static::lazy_static;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Node};

fn collector(node: &Node) -> Result<Option<String>, JsValue> {
    if node.node_type() != Node::TEXT_NODE {
        if let Some(el) = node.dyn_ref::<Element>() {
            if el.has_attributes() {
                let attrs = el.attributes();
                for i in 0..attrs.length() {
                    if let Some(attr) = attrs.item(i) {
                        let name = attr.name();
                        if name.as_str().starts_with('#') {
                            el.remove_attribute(&name)?;
                            return Ok(Some(name.as_str().trim_start_matches('#').to_owned()));
                        }
                    }
                }
            }
        }
        Ok(None)
    } else {
        if let Some(node_value) = node.node_value() {
            if node_value.as_str().starts_with('#') {
                node.set_node_value(None);
                return Ok(Some(node_value.as_str().trim_start_matches('#').to_owned()));
            }
        }
        Ok(None)
    }
}

struct Document(web_sys::Document);

unsafe impl Sync for Document {}

lazy_static! {
    static ref DOCUMENT: Document = {
        let document = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");
        Document(document)
    };
}

struct TreeWalker(web_sys::TreeWalker);

unsafe impl Sync for TreeWalker {}

lazy_static! {
    static ref TREE_WALKER: TreeWalker = {
        let tree_walker = DOCUMENT.0.create_tree_walker(&DOCUMENT.0).unwrap();
        TreeWalker(tree_walker)
    };
}

fn roll(mut idx: usize) -> Result<Node, JsValue> {
    while idx > 1 {
        TREE_WALKER.0.next_node()?;
        idx -= 1;
    }
    Ok(TREE_WALKER.0.current_node())
}

pub struct Ref {
    idx: usize,
    ref_: String,
}

fn gen_path(node: &Node) -> Result<Vec<Ref>, JsValue> {
    TREE_WALKER.0.set_current_node(node);

    let mut indices = Vec::new();
    let mut idx = 0;

    match collector(node)? {
        Some(ref_) => {
            indices.push(Ref { idx: idx + 1, ref_ });
            idx = 1;
        }
        None => idx += 1,
    }

    while let Some(current) = TREE_WALKER.0.next_node()? {
        match collector(&current)? {
            Some(ref_) => {
                indices.push(Ref { idx: idx + 1, ref_ });
                idx = 1;
            }
            None => idx += 1,
        }
    }

    Ok(indices)
}

pub struct Template {
    node: Node,
    ref_paths: Vec<Ref>,
}

impl Template {
    pub fn collect(&self) -> Result<HashMap<String, Node>, JsValue> {
        let mut refs = HashMap::new();
        TREE_WALKER.0.set_current_node(&self.node);

        for ref_path in self.ref_paths.iter() {
            let ref_node = roll(ref_path.idx)?;
            refs.insert(ref_path.ref_.clone(), ref_node);
        }

        Ok(refs)
    }
}

impl AsRef<web_sys::Node> for Template {
    fn as_ref(&self) -> &web_sys::Node {
        &self.node
    }
}

impl AsRef<JsValue> for Template {
    fn as_ref(&self) -> &JsValue {
        self.node.as_ref()
    }
}

struct CompilerTemplate(web_sys::HtmlTemplateElement);

unsafe impl Sync for CompilerTemplate {}

lazy_static! {
    static ref COMPILER_TEMPLATE: CompilerTemplate = {
        let compiler_template = DOCUMENT
            .0
            .create_element("template")
            .unwrap()
            .unchecked_into::<web_sys::HtmlTemplateElement>();
        CompilerTemplate(compiler_template)
    };
}

pub fn h(value: &str) -> Result<Template, JsValue> {
    COMPILER_TEMPLATE.0.set_inner_html(value.trim());
    let content = COMPILER_TEMPLATE
        .0
        .content()
        .first_child()
        .expect("first child");
    compile(content)
}

pub fn compile(node: Node) -> Result<Template, JsValue> {
    let ref_paths = gen_path(&node)?;
    Ok(Template { node, ref_paths })
}

#[cfg(test)]
mod tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{HtmlElement, Node};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn collector_tests() {
        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");
        let el = document.create_element("div").unwrap();
        let node: &Node = &el;

        {
            el.set_inner_html("<span></span>");
            let test_node = node.first_child().unwrap();
            let test_el = test_node.unchecked_ref::<HtmlElement>();
            assert_eq!(super::collector(test_el), Ok(None));
        }

        {
            el.set_inner_html("<span #test-attr></span>");
            let test_node = node.first_child().unwrap();
            let test_el = test_node.unchecked_ref::<HtmlElement>();
            assert_eq!(super::collector(test_el), Ok(Some("test-attr".to_owned())));
        }

        {
            let text = document.create_text_node("#test-text");
            assert_eq!(super::collector(&text), Ok(Some("test-text".to_owned())));
        }
    }

    #[wasm_bindgen_test]
    fn h_tests() {
        {
            let result = super::h("<div></div>");
            assert!(result.is_ok());
            let template = result.unwrap();
            let refs = template.collect().unwrap();
            assert!(refs.is_empty());
        }

        {
            let result = super::h("<div #test></div>");
            assert!(result.is_ok());
            let template = result.unwrap();
            let refs = template.collect().unwrap();
            assert!(refs.len() == 1);
            assert!(refs.contains_key("test"));
        }

        {
            let result = super::h("<div #foo>#bar</div>");
            assert!(result.is_ok());
            let template = result.unwrap();
            let refs = template.collect().unwrap();
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

}
