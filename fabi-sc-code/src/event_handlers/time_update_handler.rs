use js_sys::{Date, Object, Reflect};
use wasm_bindgen::JsValue;
use web_sys::{console, window};

pub fn set_top_bar_time() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };

    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };

    let Ok(time_label) = document.query_selector("#top-bar-time") else {
        console::error_1(&JsValue::from_str("An error happened while trying to get the time label."));
        return;
    };

    let Some(time_label) = time_label else {
        console::error_1(&JsValue::from_str("Unable to get the time label."));
        return;
    };

    time_label.set_text_content(Some(&get_local_date_time(DateOrTime::TimeNoSec)));
}

fn get_local_date_time(date_or_time: DateOrTime) -> String {
    let date = Date::new_0();
    let options = Object::new();
    _ = Reflect::set(&options, &JsValue::from_str("hour"), &JsValue::from_str("2-digit"));
    _ = Reflect::set(&options, &JsValue::from_str("minute"), &JsValue::from_str("2-digit"));

    let date_str = date
        .to_locale_date_string("default", &JsValue::UNDEFINED)
        .as_string()
        .unwrap_or(String::from("Unable to load"));

    let time_str = date
        .to_locale_time_string("default")
        .as_string()
        .unwrap_or(String::from("Unable to load"));

    let time_no_sec_str = date
        .to_locale_time_string_with_options("default", &options)
        .as_string()
        .unwrap_or(String::from("Unable to load"));

    match date_or_time {
        DateOrTime::DateTime => format!("{date_str} {time_str}"),
        DateOrTime::DateTimeNoSec => format!("{date_str} {time_no_sec_str}"),
        DateOrTime::Date => format!("{date_str}"),
        DateOrTime::Time => format!("{time_str}"),
        DateOrTime::TimeNoSec => format!("{time_no_sec_str}"),
    }
}

#[allow(dead_code)]
enum DateOrTime {
    DateTime,
    DateTimeNoSec,
    Date,
    Time,
    TimeNoSec
}