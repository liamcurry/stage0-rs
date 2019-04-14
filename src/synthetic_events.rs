use super::DOCUMENT;
use js_sys::Reflect;
use lazy_static::lazy_static;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn native_to_synthetic_event(event: web_sys::Event, name: &str) {
    let mut event_key = String::new();
    event_key.push_str("__");
    event_key.push_str(name);
    let event_key = JsValue::from(event_key);

    let mut dom = event
        .target()
        .and_then(|et| et.dyn_into::<web_sys::Node>().ok());
    while let Some(node) = dom.take() {
        let event_handler = Reflect::get(&node, &event_key).unwrap();
        if let Some(event_handler) = event_handler.dyn_ref::<js_sys::Function>() {
            event_handler.call1(&JsValue::NULL, &event).unwrap();
            return;
        }
        dom = node.parent_node();
    }
}

lazy_static! {
    static ref CONFIGURED_SYNTHETIC_EVENTS: HashSet<&'static str> = HashSet::new();
}

pub fn setup_synthetic_event(name: &'static str) {
    if CONFIGURED_SYNTHETIC_EVENTS.contains(name) {
        return;
    }
    let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        native_to_synthetic_event(event, name);
    }) as Box<dyn Fn(web_sys::Event)>);
    DOCUMENT
        .0
        .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())
        .unwrap();
    callback.forget();
}
