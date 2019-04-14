use js_sys::Reflect;
use stage0::h;
use stage0::synthetic_events::setup_synthetic_event;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const VIEW: &str = "
<div>
    <h1>#count</h1>
    <button #down>-</button>
    <button #up>+</button>
</div>
";

struct State {
    count: i32,
}

#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    let root = h(VIEW)?;
    let mut refs = root.collect()?;

    setup_synthetic_event("click");

    let state = Rc::new(RefCell::new(State { count: 0 }));

    let count = Rc::new(refs.remove("count").unwrap());

    let update = {
        let state = state.clone();
        let count = count.clone();
        let func = move || count.set_node_value(Some(&state.borrow().count.to_string()));
        Rc::new(func)
    };
    update();

    let down = refs.remove("down").unwrap();
    let down_onclick = {
        let state = state.clone();
        let update = update.clone();
        Closure::wrap(Box::new(move || {
            state.borrow_mut().count -= 1;
            update();
        }) as Box<dyn FnMut()>)
    };
    Reflect::set(
        &down,
        &JsValue::from("__click"),
        down_onclick.as_ref().unchecked_ref::<js_sys::Function>(),
    )
    .unwrap();
    down_onclick.forget();

    let up = refs.remove("up").unwrap();
    let up_onclick = {
        let state = state.clone();
        let update = update.clone();
        Closure::wrap(Box::new(move || {
            state.borrow_mut().count += 1;
            update();
        }) as Box<dyn FnMut()>)
    };
    Reflect::set(
        &up,
        &JsValue::from("__click"),
        up_onclick.as_ref().unchecked_ref::<js_sys::Function>(),
    )
    .unwrap();
    up_onclick.forget();

    web_sys::window()
        .expect("no window")
        .document()
        .expect("no document")
        .body()
        .expect("no body")
        .append_child(root.as_ref())
        .map(|_| ())
}
