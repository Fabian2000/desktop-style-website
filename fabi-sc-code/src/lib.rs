use wasm_bindgen::prelude::*;
use web_sys::{console, window};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let title = window().ok_or("No Window? Noooo :(")?.document().ok_or("No document? Noooo :(")?.query_selector("h1")?.ok_or("No h1? Noooo :(")?;
    let content = title.text_content().ok_or("No content? Noooo :(")?;
    title.set_text_content(Some(&content.replace("World", "Fabi-sc")));
    console::log_1(&JsValue::from_str("Hello World!"));
    Ok(())
}
