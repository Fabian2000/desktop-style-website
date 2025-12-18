use gloo_timers::callback::Interval;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;
use yew::prelude::*;

use crate::database::IndexedDb;
use crate::utils::{get_local_date, get_local_time_no_sec};

const STYLES: [&str; 6] = ["minimal", "glass", "analog", "bold", "corner", "hidden"];

#[function_component(DesktopWidgets)]
pub fn desktop_widgets() -> Html {
    let time = use_state(|| get_local_time_no_sec());
    let date = use_state(|| get_local_date());
    let style = use_state(|| "minimal".to_string());
    let editing = use_state(|| false);

    // Load style from IndexedDB on mount
    {
        let style = style.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(db) = IndexedDb::open("settings", "system_settings").await {
                    if let Ok(value) = db.get_item("clock_style").await {
                        if let Some(s) = value.as_string() {
                            style.set(s);
                        }
                    }
                }
            });
            || ()
        });
    }

    // Update time every second
    {
        let time = time.clone();
        let date = date.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                time.set(get_local_time_no_sec());
                date.set(get_local_date());
            });
            || drop(interval)
        });
    }

    // Click handler to close editing when clicking outside clock/picker
    let on_backdrop_click = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| {
            editing.set(false);
        })
    };

    // Click handler for clock
    let on_clock_click = {
        let editing = editing.clone();
        let style = style.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            // If hidden, clicking anywhere should open picker
            if *style == "hidden" || !*editing {
                editing.set(true);
            }
        })
    };

    // Style button click handler
    let on_style_click = {
        let style = style.clone();
        let editing = editing.clone();
        Callback::from(move |new_style: String| {
            let new_style_clone = new_style.clone();
            style.set(new_style);
            editing.set(false);

            // Save to IndexedDB
            spawn_local(async move {
                if let Ok(db) = IndexedDb::open("settings", "system_settings").await {
                    let _ = db.set_item("clock_style", &JsValue::from_str(&new_style_clone)).await;
                }
            });
        })
    };

    // Build clock class
    let clock_class = classes!(
        "clock-widget",
        format!("style-{}", *style),
        (*editing).then_some("editing")
    );

    // For hidden style, show a small indicator when editing
    let show_hidden_indicator = *style == "hidden" && !*editing;

    html! {
        <div
            class={classes!("desktop-widgets", (*editing).then_some("editing-mode"))}
            onclick={if *editing { Some(on_backdrop_click) } else { None }}
        >
            if show_hidden_indicator {
                // Clickable area for hidden clock
                <div class="hidden-clock-area" onclick={on_clock_click.clone()}>
                    <i class="fa-regular fa-clock"></i>
                </div>
            } else {
                <div class={clock_class} onclick={on_clock_click}>
                    if *style == "analog" {
                        <AnalogClock />
                    } else if *style != "hidden" {
                        <div class="clock-time">{&*time}</div>
                        <div class="clock-date">{&*date}</div>
                    }
                </div>
            }

            if *editing {
                <div class="style-picker">
                    { for STYLES.iter().map(|s| {
                        let is_active = *style == *s;
                        let style_name = (*s).to_string();
                        let on_click = {
                            let on_style_click = on_style_click.clone();
                            let style_name = style_name.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.stop_propagation();
                                on_style_click.emit(style_name.clone());
                            })
                        };
                        html! {
                            <button
                                class={classes!("style-btn", is_active.then_some("active"))}
                                onclick={on_click}
                            >
                                <StylePreview style={style_name.clone()} />
                                <span class="style-label">{style_name}</span>
                            </button>
                        }
                    })}
                </div>
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct StylePreviewProps {
    style: String,
}

#[function_component(StylePreview)]
fn style_preview(props: &StylePreviewProps) -> Html {
    let icon = match props.style.as_str() {
        "minimal" => "fa-solid fa-font",
        "glass" => "fa-solid fa-square",
        "analog" => "fa-regular fa-clock",
        "bold" => "fa-solid fa-bold",
        "corner" => "fa-solid fa-arrow-down-left",
        "hidden" => "fa-solid fa-eye-slash",
        _ => "fa-solid fa-clock",
    };

    html! {
        <i class={icon}></i>
    }
}

#[function_component(AnalogClock)]
fn analog_clock() -> Html {
    let time = use_state(|| js_sys::Date::new_0());

    // Update every second
    {
        let time = time.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                time.set(js_sys::Date::new_0());
            });
            || drop(interval)
        });
    }

    let hours = time.get_hours() as f64;
    let minutes = time.get_minutes() as f64;
    let seconds = time.get_seconds() as f64;

    // Calculate rotation angles
    let second_deg = seconds * 6.0; // 360/60 = 6 degrees per second
    let minute_deg = minutes * 6.0 + seconds * 0.1; // 6 degrees per minute + slight movement for seconds
    let hour_deg = (hours % 12.0) * 30.0 + minutes * 0.5; // 30 degrees per hour + movement for minutes

    html! {
        <div class="analog-clock">
            <div class="clock-face">
                // Hour markers
                { for (0..12).map(|i| {
                    let rotation = i * 30;
                    html! {
                        <div class="hour-marker" style={format!("transform: rotate({}deg)", rotation)}></div>
                    }
                })}
                // Hands
                <div class="hand hour-hand" style={format!("transform: rotate({}deg)", hour_deg)}></div>
                <div class="hand minute-hand" style={format!("transform: rotate({}deg)", minute_deg)}></div>
                <div class="hand second-hand" style={format!("transform: rotate({}deg)", second_deg)}></div>
                <div class="center-dot"></div>
            </div>
        </div>
    }
}
