use js_sys::{Date, Reflect};
use serde::{Deserialize, Serialize};
use stage0::h;
use stage0::reconcile::reconcile;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, HtmlElement, HtmlInputElement, KeyboardEvent, Node};

const TODO_VIEW: &str = r#"
<li>
    <input class="toggle" type="checkbox" #checkbox>
    <label>#label</label>
    <button class="destroy" #destroy></button>
</li>
"#;

#[derive(PartialEq, Serialize, Deserialize, Clone)]
struct Todo {
    id: u64,
    title: String,
    completed: bool,
}

struct Scope {
    render: Box<dyn Fn()>,
    delete: Box<dyn Fn(&Todo)>,
    // todo_template: stage0::Template,
}

fn todo_view(item: Rc<RefCell<Todo>>, scope: &Scope) -> Result<stage0::Template, JsValue> {
    let root = h(TODO_VIEW)?;
    {
        let root_node: &Node = root.as_ref();
        root_node
            .unchecked_ref::<HtmlElement>()
            .set_class_name(if item.borrow().completed {
                "completed"
            } else {
                ""
            });
    }
    let mut refs = root.collect()?;

    let scope = Rc::new(scope);

    let label = refs.remove("label").unwrap();
    label.set_node_value(Some(&item.borrow().title));

    let checkbox = refs
        .remove("checkbox")
        .unwrap()
        .unchecked_into::<HtmlInputElement>();
    checkbox.set_checked(item.borrow().completed);

    {
        let item = item.clone();
        let onchange = Closure::wrap(Box::new(move || {
            item.borrow_mut().completed = checkbox.checked();
            // (scope.render)();
        }) as Box<dyn Fn()>);
        checkbox.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    }

    let destroy = refs.remove("destroy").unwrap();
    Ok(root)
}

const MAIN_VIEW: &str = r##"
<section class="todoapp">
    <header class="header">
        <h1>todos</h1>
        <input class="new-todo" placeholder="What needs to be done?" autofocus #input>
    </header>
    <section style="display:none" class="main" #body>
        <input id="toggle-all" class="toggle-all" type="checkbox" #toggleall>
        <label for="toggle-all">Mark all as complete</label>
        <ul class="todo-list" #list></ul>
        <footer class="footer">
            <span class="todo-count">#count</span>
            <ul class="filters">
                <li>
                <a href="#/" class="selected" #all>All</a>
                </li>
                <li>
                <a href="#/active" #active>Active</a>
                </li>
                <li>
                <a href="#/completed" #completed>Completed</a>
                </li>
            </ul>
            <button class="clear-completed" #clear>Clear completed</button>
        </footer>
    </section>
</section>
"##;

enum Filter {
    All,
    Active,
    Completed,
}

fn main_view(todos: Vec<Todo>) -> Result<stage0::Template, JsValue> {
    let root = h(MAIN_VIEW)?;
    let mut refs = root.collect()?;

    let filter = Rc::new(RefCell::new(Filter::All));

    let body = refs.remove("body").unwrap().unchecked_into::<HtmlElement>();

    let clear = refs
        .remove("clear")
        .unwrap()
        .unchecked_into::<HtmlElement>();

    let input = Rc::new(
        refs.remove("input")
            .unwrap()
            .unchecked_into::<HtmlInputElement>(),
    );

    let todos = Rc::new(RefCell::new(todos));
    let rendered_ids: Rc<RefCell<Vec<u64>>> = Rc::new(RefCell::new(Vec::new()));

    let scope: Rc<RefCell<Option<Scope>>> = Rc::new(RefCell::new(None));

    let update = {
        let list = refs.remove("list").unwrap();
        let count = refs.remove("count").unwrap();
        let todos = todos.clone();
        let rendered_ids = rendered_ids.clone();
        let filter = filter.clone();
        let scope = scope.clone();
        let body_style = body.style();
        let clear_style = clear.style();
        let func = move || {
            console::log_1(&JsValue::from(111));
            let mut todos = todos.borrow_mut();
            let todos_count = todos.len();
            let completed_todos = todos.iter().filter(|t| t.completed).count();
            let uncompleted_todos = todos_count - completed_todos;
            let mut visible_todos: Vec<_> = match *filter.borrow_mut() {
                Filter::All => todos.iter_mut().collect(),
                Filter::Active => todos.iter_mut().filter(|t| !t.completed).collect(),
                Filter::Completed => todos.iter_mut().filter(|t| t.completed).collect(),
            };

            body_style
                .set_property("display", if todos_count > 0 { "block" } else { "none" })
                .unwrap();
            clear_style
                .set_property(
                    "display",
                    if completed_todos > 0 { "block" } else { "none" },
                )
                .unwrap();
            if todos_count > 0 {
                let mut s = String::new();
                s.push_str(uncompleted_todos.to_string().as_str());
                s.push_str(" item");
                if uncompleted_todos != 1 {
                    s.push('s');
                }
                s.push_str(" left");
                count.set_node_value(Some(&s));
            } else {
                count.set_node_value(None);
            }

            if let Some(scope) = scope.borrow().as_ref() {
                console::log_1(&JsValue::from(222));
                reconcile(
                    &list,
                    &rendered_ids.borrow(),
                    &mut visible_todos,
                    |t| t.id,
                    move |t| todo_view(*t, scope).unwrap().into(),
                    |node, item| {
                        let update_node = Reflect::get(&node, &JsValue::from("update")).unwrap();
                        if let Some(update_node) = update_node.dyn_ref::<js_sys::Function>() {
                            update_node
                                .call1(&JsValue::NULL, &JsValue::from_serde(item).unwrap())
                                .unwrap();
                            return;
                        }
                    },
                    None,
                    None,
                );
                *rendered_ids.borrow_mut() = visible_todos.iter().map(|t| t.id).collect();
            } else {
                console::log_1(&JsValue::from(333));
            }
        };
        Rc::new(func)
    };

    *scope.borrow_mut() = Some(Scope {
        render: {
            let update = update.clone();
            let func = move || update();
            Box::new(func)
        },
        delete: {
            let update = update.clone();
            let todos = todos.clone();
            let func = move |item: &Todo| {
                todos.borrow_mut().retain(|t| t != item);
                update()
            };
            Box::new(func)
        },
    });

    let create_todo = {
        let input = input.clone();
        let todos = todos.clone();
        let update = update.clone();
        let func = move || {
            let value = input.value();
            if value.is_empty() {
                return;
            }
            todos.borrow_mut().insert(
                0,
                Todo {
                    id: Date::now() as u64,
                    title: value,
                    completed: false,
                },
            );
            input.set_value("");
            update();
        };
        Rc::new(func)
    };

    {
        let create_todo = create_todo.clone();
        let onkeyup = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            if e.key_code() == 13 {
                create_todo();
            }
        }) as Box<dyn Fn(KeyboardEvent)>);
        input.set_onkeyup(Some(onkeyup.as_ref().unchecked_ref()));
        onkeyup.forget();
    }

    {
        let create_todo = create_todo.clone();
        let onblur = Closure::wrap(Box::new(move || {
            create_todo();
        }) as Box<dyn Fn()>);
        input.set_onblur(Some(onblur.as_ref().unchecked_ref()));
        onblur.forget();
    }

    {
        let update = update.clone();
        let todos = todos.clone();
        let toggleall = Rc::new(
            refs.remove("toggleall")
                .unwrap()
                .unchecked_into::<HtmlInputElement>(),
        );
        let onchange = {
            let toggleall = toggleall.clone();
            Closure::wrap(Box::new(move || {
                let value = toggleall.checked();
                todos
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|t| t.completed = value);
                update();
            }) as Box<dyn Fn()>)
        };
        toggleall.set_onchange(Some(onchange.as_ref().unchecked_ref()));
        onchange.forget();
    }

    {
        let all = refs.remove("all").unwrap().unchecked_into::<HtmlElement>();
        let filter = filter.clone();
        let update = update.clone();
        let onclick = Closure::wrap(Box::new(move || {
            *filter.borrow_mut() = Filter::All;
            update();
        }) as Box<dyn Fn()>);
        all.set_onclick(Some(onclick.as_ref().unchecked_ref()));
        onclick.forget();
    }

    {
        let active = refs
            .remove("active")
            .unwrap()
            .unchecked_into::<HtmlElement>();
        let filter = filter.clone();
        let update = update.clone();
        let onclick = Closure::wrap(Box::new(move || {
            *filter.borrow_mut() = Filter::Active;
            update();
        }) as Box<dyn Fn()>);
        active.set_onclick(Some(onclick.as_ref().unchecked_ref()));
        onclick.forget();
    }

    {
        let completed = refs
            .remove("completed")
            .unwrap()
            .unchecked_into::<HtmlElement>();
        let filter = filter.clone();
        let update = update.clone();
        let onclick = Closure::wrap(Box::new(move || {
            *filter.borrow_mut() = Filter::Completed;
            update();
        }) as Box<dyn Fn()>);
        completed.set_onclick(Some(onclick.as_ref().unchecked_ref()));
        onclick.forget();
    }

    {
        let todos = todos.clone();
        let update = update.clone();
        let onclick = Closure::wrap(Box::new(move || {
            todos.borrow_mut().retain(|t| !t.completed);
            update();
        }) as Box<dyn Fn()>);
        clear.set_onclick(Some(onclick.as_ref().unchecked_ref()));
        onclick.forget();
    }

    Ok(root)
}

#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let root = main_view(vec![])?;

    web_sys::window()
        .expect("no window")
        .document()
        .expect("no document")
        .body()
        .expect("no body")
        .append_child(root.as_ref())
        .map(|_| ())
}
