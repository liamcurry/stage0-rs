use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const VIEW: &str = "
<div>
    <h1>#count</h1>
    <button #down>-</button>
    <button #up>+</button>
</div>
";

#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");
    let body = document.body().expect("no body");
    let compiler_template = document
        .create_element("template")
        .unwrap()
        .unchecked_into::<web_sys::HtmlTemplateElement>();
    let tree_walker = document.create_tree_walker(&document).unwrap();

    let root = stage0::compile_str(&compiler_template, &tree_walker, VIEW)?;
    let refs = root.collect(&tree_walker)?;

    let mut state = 0;

    // let count = refs.get("count").unwrap();

    // let down = refs.get("down").unwrap();
    // let up = refs.get("up").unwrap();

    web_sys::console::log_1(&root.node);
    body.append_child(&root.node)?;
    Ok(())
}
