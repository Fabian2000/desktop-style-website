use js_sys::{Date, Object, Reflect};
use wasm_bindgen::JsValue;

#[allow(dead_code)]
pub enum DateOrTime {
    DateTime,
    DateTimeNoSec,
    Date,
    Time,
    TimeNoSec,
}

#[allow(dead_code)]
pub fn get_local_time_no_sec() -> String {
    get_local_date_time(DateOrTime::TimeNoSec)
}

fn get_local_date_time(date_or_time: DateOrTime) -> String {
    let date = Date::new_0();
    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("hour"),
        &JsValue::from_str("2-digit"),
    );
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("minute"),
        &JsValue::from_str("2-digit"),
    );

    let date_str = date
        .to_locale_date_string("default", &JsValue::UNDEFINED)
        .as_string()
        .unwrap_or_else(|| String::from("Unable to load"));

    let time_str = date
        .to_locale_time_string("default")
        .as_string()
        .unwrap_or_else(|| String::from("Unable to load"));

    let time_no_sec_str = date
        .to_locale_time_string_with_options("default", &options)
        .as_string()
        .unwrap_or_else(|| String::from("Unable to load"));

    match date_or_time {
        DateOrTime::DateTime => format!("{date_str} {time_str}"),
        DateOrTime::DateTimeNoSec => format!("{date_str} {time_no_sec_str}"),
        DateOrTime::Date => date_str,
        DateOrTime::Time => time_str,
        DateOrTime::TimeNoSec => time_no_sec_str,
    }
}
