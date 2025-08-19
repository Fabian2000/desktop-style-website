use gloo_timers::{callback::Interval, future::TimeoutFuture};
use wasm_bindgen::prelude::*;
use web_sys::Event;
use web_sys::{window, Document, Window};

mod event_handlers;

use event_handlers::lock_screen_handlers;
use event_handlers::time_update_handler;

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let window = window().ok_or("Unable to get the window.")?;
    let document = window.document().ok_or("Unable to get the document.")?;
    
    TimeoutFuture::new(3_000).await;

    let boot_screen = document.query_selector(".boot-screen")?.ok_or("Unable to get the boot-screen.")?;
    _ = boot_screen.class_list().add_1("display-none");
    
    set_start_events(&window, &document)?;

    let top_bar_timer_update = Interval::new(1000, || time_update_handler::set_top_bar_time());
    top_bar_timer_update.forget();

    Ok(())
}

fn set_start_events(_window: &Window, document: &Document) -> Result<(), JsValue> {
    let lock_screen_login_button = document.query_selector("#lock-screen-login")?.ok_or("Unable to get the boot-screen.")?;

    let login_action = Closure::wrap(Box::new(move || {
        lock_screen_handlers::login_click();
    }) as Box<dyn FnMut()>);
    lock_screen_login_button.add_event_listener_with_callback("click", login_action.as_ref().unchecked_ref())?;
    login_action.forget();

    let contextmenu_action = Closure::wrap(Box::new(move |event: Event| {
        event.prevent_default();
    }) as Box<dyn FnMut(_)>);
    document.add_event_listener_with_callback("contextmenu", contextmenu_action.as_ref().unchecked_ref())?;
    contextmenu_action.forget();

    Ok(())
}