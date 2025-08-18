use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::prelude::*;
use web_sys::window;

mod event_handlers;

use event_handlers::lock_screen_handlers;

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let window = window().ok_or("Unable to get the window.")?;
    let document = window.document().ok_or("Unable to get the document.")?;
    
    TimeoutFuture::new(3_000).await;

    let boot_screen = document.query_selector(".boot-screen")?.ok_or("Unable to get the boot-screen.")?;
    _ = boot_screen.class_list().add_1("display-none");
    
    let lock_screen_login_button = document.query_selector("#lock-screen-login")?.ok_or("Unable to get the boot-screen.")?;

    let login_action = Closure::wrap(Box::new(move || {
        lock_screen_handlers::login_click();
    }) as Box<dyn FnMut()>);
    lock_screen_login_button.add_event_listener_with_callback("click", login_action.as_ref().unchecked_ref()).unwrap();
    login_action.forget();

    Ok(())
}