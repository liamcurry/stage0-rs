use js_sys::{Array, Object, Reflect};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    console, Document, Element, Event, HtmlElement, HtmlTemplateElement, Node, TreeWalker,
};

pub fn collector(node: &Node) -> Result<Option<String>, JsValue> {
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

pub fn roll(tree_walker: &TreeWalker, mut idx: usize) -> Result<Node, JsValue> {
    while idx > 1 {
        tree_walker.next_node()?;
        idx -= 1;
    }
    Ok(tree_walker.current_node())
}

pub struct Ref {
    idx: usize,
    ref_: String,
}

pub fn gen_path(tree_walker: &TreeWalker, node: &Node) -> Result<Vec<Ref>, JsValue> {
    tree_walker.set_current_node(node);

    let mut indices = Vec::new();
    let mut idx = 0;

    match collector(node)? {
        Some(ref_) => {
            indices.push(Ref { idx: idx + 1, ref_ });
            idx = 1;
        }
        None => idx += 1,
    }

    while let Some(current) = tree_walker.next_node()? {
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
    pub node: Node,
    ref_paths: Vec<Ref>,
}

impl Template {
    pub fn collect(&self, tree_walker: &TreeWalker) -> Result<HashMap<String, Node>, JsValue> {
        let mut refs = HashMap::new();
        tree_walker.set_current_node(&self.node);

        for ref_path in self.ref_paths.iter() {
            let ref_node = roll(tree_walker, ref_path.idx)?;
            refs.insert(ref_path.ref_.clone(), ref_node);
        }

        Ok(refs)
    }
}

pub fn compile_str(
    compiler_template: &HtmlTemplateElement,
    tree_walker: &TreeWalker,
    value: &str,
) -> Result<Template, JsValue> {
    compiler_template.set_inner_html(value);
    let content = compiler_template
        .content()
        .first_child()
        .expect("first child");
    compile_node(tree_walker, content)
}

pub fn compile_node(tree_walker: &TreeWalker, node: Node) -> Result<Template, JsValue> {
    let ref_paths = gen_path(tree_walker, &node)?;
    Ok(Template { node, ref_paths })
}
