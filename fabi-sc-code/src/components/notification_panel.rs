use crate::database::IndexedDb;
use gloo_timers::callback::Interval;
use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NotificationPanelProps {
    pub visible: bool,
    pub open: bool,
    pub drag_offset: f32,
    pub is_dragging: bool,
    pub on_disconnect: Callback<()>,
}

#[function_component(NotificationPanel)]
pub fn notification_panel(props: &NotificationPanelProps) -> Html {
    let current_time = use_state(|| get_current_time());
    let current_date = use_state(|| get_current_date());
    let volume = use_state(|| 0i32);
    let volume_display = use_state(|| String::from("0 %"));
    let brightness = use_state(|| 100i32);
    let brightness_display = use_state(|| String::from("100 %"));

    // Initialize volume and brightness from IndexedDB
    {
        let volume = volume.clone();
        let volume_display = volume_display.clone();
        let brightness = brightness.clone();
        let brightness_display = brightness_display.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(db) = IndexedDb::open("settings", "system_settings").await {
                    if db.has_item("volume").await {
                        if let Ok(value) = db.get_item("volume").await {
                            if let Some(v) = value.as_f64() {
                                let v = v as i32;
                                volume.set(v);
                                volume_display.set(format!("{} %", v));
                            }
                        }
                    }
                    if db.has_item("brightness").await {
                        if let Ok(value) = db.get_item("brightness").await {
                            if let Some(v) = value.as_f64() {
                                let v = v as i32;
                                brightness.set(v);
                                brightness_display.set(format!("{} %", v));
                                // Apply brightness overlay
                                apply_brightness_overlay(v);
                            }
                        }
                    }
                }
            });
            || ()
        });
    }

    // Update time every second
    {
        let current_time = current_time.clone();
        let current_date = current_date.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                current_time.set(get_current_time());
                current_date.set(get_current_date());
            });
            || drop(interval)
        });
    }

    let on_volume_input = {
        let volume = volume.clone();
        let volume_display = volume_display.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target() {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    if let Ok(value) = input.value().parse::<i32>() {
                        volume.set(value);
                        volume_display.set(format!("{} %", value));
                    }
                }
            }
        })
    };

    let on_volume_change = {
        Callback::from(move |e: Event| {
            if let Some(target) = e.target() {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    if let Ok(value) = input.value().parse::<i32>() {
                        spawn_local(async move {
                            if let Ok(db) = IndexedDb::open("settings", "system_settings").await {
                                let _ = db.set_item("volume", &JsValue::from(value)).await;
                            }
                        });
                    }
                }
            }
        })
    };

    let on_brightness_input = {
        let brightness = brightness.clone();
        let brightness_display = brightness_display.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target() {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    if let Ok(value) = input.value().parse::<i32>() {
                        brightness.set(value);
                        brightness_display.set(format!("{} %", value));
                        apply_brightness_overlay(value);
                    }
                }
            }
        })
    };

    let on_brightness_change = {
        Callback::from(move |e: Event| {
            if let Some(target) = e.target() {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    if let Ok(value) = input.value().parse::<i32>() {
                        spawn_local(async move {
                            if let Ok(db) = IndexedDb::open("settings", "system_settings").await {
                                let _ = db.set_item("brightness", &JsValue::from(value)).await;
                            }
                        });
                    }
                }
            }
        })
    };

    let on_wifi_disconnect = {
        let on_disconnect = props.on_disconnect.clone();
        Callback::from(move |_| {
            on_disconnect.emit(());
        })
    };

    if !props.visible {
        return html! {};
    }

    // Calculate transform based on state
    let transform_style = if props.is_dragging {
        if props.open {
            // Panel is open, dragging up to close (offset is negative)
            format!("transform: translateY({}px)", props.drag_offset.min(0.0))
        } else {
            // Panel is closed, dragging down to open (offset is positive)
            format!("transform: translateY(calc(-100% + {}px))", props.drag_offset.max(0.0))
        }
    } else if props.open {
        "transform: translateY(0)".to_string()
    } else {
        "transform: translateY(-100%)".to_string()
    };

    let panel_class = {
        let mut classes = vec!["notification-panel"];
        if props.open && !props.is_dragging {
            classes.push("open");
        }
        if props.is_dragging {
            classes.push("dragging");
        }
        classes.join(" ")
    };

    html! {
        <div class={panel_class} style={transform_style}>
            <div class="notification-panel-content">
                <div class="notification-time-section">
                    <div class="notification-large-time">{(*current_time).clone()}</div>
                    <div class="notification-date">{(*current_date).clone()}</div>
                </div>

                <div class="notification-quick-settings">
                    // Android-style WiFi toggle button
                    <button class="wifi-toggle-btn active" onclick={on_wifi_disconnect.clone()}>
                        <div class="wifi-toggle-icon">
                            <i class="fa-solid fa-wifi"></i>
                        </div>
                        <div class="wifi-toggle-info">
                            <span class="wifi-toggle-label">{"WLAN"}</span>
                            <span class="wifi-toggle-network">{"fabi-sc.de"}</span>
                        </div>
                    </button>
                    // Airplane mode button (locked/disabled)
                    <button class="wifi-toggle-btn disabled" disabled={true}>
                        <div class="wifi-toggle-icon">
                            <i class="fa-solid fa-plane-up"></i>
                        </div>
                        <div class="wifi-toggle-info">
                            <span class="wifi-toggle-label">{"Airplane Mode"}</span>
                            <span class="wifi-toggle-network">{"Locked"}</span>
                        </div>
                    </button>
                </div>

                <div class="notification-section">
                    <div class="slider-container">
                        <div class="slider-fill" style={format!("width: {}%", *volume)}></div>
                        <div class="slider-content">
                            <i class="fa-solid fa-volume-high"></i>
                            <span>{(*volume_display).clone()}</span>
                        </div>
                        <input
                            class="slider"
                            type="range"
                            min="0"
                            max="100"
                            value={(*volume).to_string()}
                            oninput={on_volume_input}
                            onchange={on_volume_change}
                        />
                    </div>
                    <div class="slider-container brightness-slider">
                        <div class="slider-fill" style={format!("width: {}%", *brightness)}></div>
                        <div class="slider-content">
                            <i class="fa-solid fa-sun"></i>
                            <span>{(*brightness_display).clone()}</span>
                        </div>
                        <input
                            class="slider"
                            type="range"
                            min="0"
                            max="100"
                            value={(*brightness).to_string()}
                            oninput={on_brightness_input}
                            onchange={on_brightness_change}
                        />
                    </div>
                </div>

            </div>
        </div>
    }
}

fn apply_brightness_overlay(brightness: i32) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Calculate opacity: 100% brightness = 0 opacity, 0% brightness = 0.7 opacity (not fully black)
            let opacity = (100 - brightness) as f32 / 100.0 * 0.7;

            // Get or create brightness overlay (outside #app, so it doesn't affect layout)
            let overlay = match document.get_element_by_id("brightness-overlay") {
                Some(el) => el,
                None => {
                    // Create overlay div
                    if let Ok(el) = document.create_element("div") {
                        el.set_id("brightness-overlay");
                        // z-index above everything including lock screen (999998)
                        // This simulates real screen brightness
                        let base_style = "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
                            background-color: black; pointer-events: none; z-index: 2147483647; \
                            transition: opacity 0.1s ease-out;";
                        let _ = el.set_attribute("style", &format!("{} opacity: {};", base_style, opacity));
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&el);
                        }
                        return;
                    } else {
                        return;
                    }
                }
            };

            // Update opacity on existing overlay
            let base_style = "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
                background-color: black; pointer-events: none; z-index: 2147483647; \
                transition: opacity 0.1s ease-out;";
            let _ = overlay.set_attribute("style", &format!("{} opacity: {};", base_style, opacity));
        }
    }
}

fn get_current_time() -> String {
    let date = Date::new_0();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    format!("{:02}:{:02}", hours, minutes)
}

fn get_current_date() -> String {
    let date = Date::new_0();
    let day = date.get_date();
    let month = get_month_name(date.get_month());
    let weekday = get_weekday_name(date.get_day());
    format!("{}, {} {}", weekday, month, day)
}

fn get_month_name(month: u32) -> &'static str {
    match month {
        0 => "January",
        1 => "February",
        2 => "March",
        3 => "April",
        4 => "May",
        5 => "June",
        6 => "July",
        7 => "August",
        8 => "September",
        9 => "October",
        10 => "November",
        11 => "December",
        _ => "Unknown",
    }
}

fn get_weekday_name(day: u32) -> &'static str {
    match day {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}
