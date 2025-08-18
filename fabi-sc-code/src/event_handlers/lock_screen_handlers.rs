use gloo_timers::callback::Timeout;
use wasm_bindgen::JsValue;
use web_sys::{console, window};

pub fn login_click() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };

    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };

    let Ok(login_screen) = document.query_selector(".lock-screen") else {
        console::error_1(&JsValue::from_str("An error happened while trying to get the login screen."));
        return;
    };

    let Some(login_screen) = login_screen else {
        console::error_1(&JsValue::from_str("Unable to get the login screen."));
        return;
    };

    _ = login_screen.class_list().add_1("fade-out");
    let timeout = Timeout::new(1000, move || {
        _ = login_screen.class_list().add_1("display-none");
    });
    timeout.forget();
}