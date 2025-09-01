use gloo_timers::{callback::Interval, future::TimeoutFuture};
use wasm_bindgen::prelude::*;
use web_sys::{console, Event};
use web_sys::{window, Document, Window};

mod event_handlers;
mod database;

use event_handlers::lock_screen_handlers;
use event_handlers::time_update_handler;

use crate::event_handlers::top_bar_handlers::{hide_wifi_audio_popup, init_volume_slider, live_volume_slider, save_volume_slider, show_wifi_disconnect, toggle_wifi_audio_popup, wifi_disconnect};

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    console::log_1(&format!("Build: {} {}", env!("BUILD_DATE"), env!("BUILD_TIME")).into());
    
    let window = window().ok_or("Unable to get the window.")?;
    let document = window.document().ok_or("Unable to get the document.")?;
    
    TimeoutFuture::new(3_000).await;

    let boot_screen = document.query_selector(".boot-screen")?.ok_or("Unable to get the boot-screen.")?;
    _ = boot_screen.class_list().add_1("display-none");

    time_update_handler::set_top_bar_time();
    init_volume_slider();
    set_start_events(&window, &document)?;

    let top_bar_timer_update = Interval::new(1000, || time_update_handler::set_top_bar_time());
    top_bar_timer_update.forget();

    Ok(())
}

fn set_start_events(_window: &Window, document: &Document) -> Result<(), JsValue> {
    let lock_screen_login_button = document.query_selector("#lock-screen-login")?.ok_or("Unable to get the login button.")?;
    let wifi_extended_menu_button = document.query_selector("#wifi-extended-menu")?.ok_or("Unable to get the wifi extended menu button.")?;
    let wifi_disconnect_button = document.query_selector("#wifi-disconnect")?.ok_or("Unable to get the wifi disconnect button.")?;
    let volume_slider = document.query_selector("#volume-slider")?.ok_or("Unable to get the volume slider.")?;

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

    if let Ok(Some(top_bar_wifi_audio_btn)) = document.query_selector("#top-bar-wifi-audio-btn") {
        let toggle_wifi_audio_popup_action = Closure::wrap(Box::new(move || {
            toggle_wifi_audio_popup();
        }) as Box<dyn FnMut()>);
        top_bar_wifi_audio_btn.add_event_listener_with_callback("click", toggle_wifi_audio_popup_action.as_ref().unchecked_ref())?;
        toggle_wifi_audio_popup_action.forget();
        
        let hide_wifi_audio_popup_action = Closure::wrap(Box::new(move |e| {
            hide_wifi_audio_popup(e);
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("mousedown", hide_wifi_audio_popup_action.as_ref().unchecked_ref())?;
        hide_wifi_audio_popup_action.forget();
    }

    let show_wifi_disconnect_action = Closure::wrap(Box::new(move || {
        show_wifi_disconnect();
    }) as Box<dyn FnMut()>);
    wifi_extended_menu_button.add_event_listener_with_callback("click", show_wifi_disconnect_action.as_ref().unchecked_ref())?;
    show_wifi_disconnect_action.forget();

    let wifi_disconnect_action = Closure::wrap(Box::new(move || {
        wifi_disconnect();
    }) as Box<dyn FnMut()>);
    wifi_disconnect_button.add_event_listener_with_callback("click", wifi_disconnect_action.as_ref().unchecked_ref())?;
    wifi_disconnect_action.forget();
    
    let volume_slider_action = Closure::wrap(Box::new(||{
        save_volume_slider();
    }) as Box<dyn FnMut()>);

    let volume_slider_spam_action = Closure::wrap(Box::new(||{
        live_volume_slider();
    }) as Box<dyn FnMut()>);
    volume_slider.add_event_listener_with_callback("pointerup", volume_slider_action.as_ref().unchecked_ref())?;
    volume_slider.add_event_listener_with_callback("keyup", volume_slider_action.as_ref().unchecked_ref())?;
    volume_slider_action.forget();
    volume_slider.add_event_listener_with_callback("input", volume_slider_spam_action.as_ref().unchecked_ref())?;
    volume_slider_spam_action.forget();

    Ok(())
}