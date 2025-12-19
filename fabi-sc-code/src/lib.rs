use wasm_bindgen::prelude::*;

pub mod apps;
mod components;
mod database;
pub mod filesystem;
pub mod python;
pub mod session;
mod utils;

use components::App;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    web_sys::console::log_1(
        &format!("Build: {} {}", env!("BUILD_DATE"), env!("BUILD_TIME")).into(),
    );

    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");
    let app_element = document
        .get_element_by_id("app")
        .expect("no #app element");
    yew::Renderer::<App>::with_root(app_element).render();

    Ok(())
}
