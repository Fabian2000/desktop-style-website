use crate::database::IndexedDb;
use crate::utils::get_local_time_no_sec;
use gloo_timers::callback::{Interval, Timeout};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TopBarProps {
    pub visible: bool,
    pub on_disconnect: Callback<()>,
}

#[function_component(TopBar)]
pub fn top_bar(props: &TopBarProps) -> Html {
    let current_time = use_state(|| get_local_time_no_sec());
    let popup_open = use_state(|| false);
    let popup_visible = use_state(|| false);
    let show_disconnect_btn = use_state(|| false);
    let volume = use_state(|| 0i32);
    let volume_display = use_state(|| String::from("0 %"));
    let close_timeout = use_state(|| None::<Timeout>);

    // Initialize volume from IndexedDB on mount
    {
        let volume = volume.clone();
        let volume_display = volume_display.clone();
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
                }
            });
            || ()
        });
    }

    // Update time every second
    {
        let current_time = current_time.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                current_time.set(get_local_time_no_sec());
            });
            || drop(interval)
        });
    }

    let toggle_popup = {
        let popup_open = popup_open.clone();
        let popup_visible = popup_visible.clone();
        let close_timeout = close_timeout.clone();
        let show_disconnect_btn = show_disconnect_btn.clone();

        Callback::from(move |_| {
            if *popup_open {
                // Close popup
                popup_open.set(false);
                show_disconnect_btn.set(false);

                let popup_visible = popup_visible.clone();
                let timeout = Timeout::new(250, move || {
                    popup_visible.set(false);
                });
                close_timeout.set(Some(timeout));
            } else {
                // Open popup - cancel any pending close timeout by setting to None
                close_timeout.set(None);

                popup_visible.set(true);

                let popup_open = popup_open.clone();
                // Use requestAnimationFrame to ensure CSS transition works
                if let Some(window) = web_sys::window() {
                    let closure = Closure::once(Box::new(move || {
                        popup_open.set(true);
                    }) as Box<dyn FnOnce()>);
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                    closure.forget();
                }
            }
        })
    };

    let on_mousedown = {
        let popup_open = popup_open.clone();
        let popup_visible = popup_visible.clone();
        let close_timeout = close_timeout.clone();
        let show_disconnect_btn = show_disconnect_btn.clone();

        Callback::from(move |e: MouseEvent| {
            if !*popup_open {
                return;
            }

            // Check if click is inside popup or the toggle button
            if let Some(target) = e.target() {
                if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                    // Check if it's the slider itself
                    if element.id() == "volume-slider" {
                        return;
                    }

                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        // Check if clicked inside popup
                        if let Ok(Some(popup)) = document.query_selector(".wifi-speaker-popup") {
                            if popup.contains(Some(element)) {
                                return;
                            }
                        }
                        // Check if clicked on toggle button
                        if let Ok(Some(btn)) = document.query_selector("#top-bar-wifi-audio-btn") {
                            if btn.contains(Some(element)) {
                                return;
                            }
                        }
                    }
                }
            }

            // Close popup
            popup_open.set(false);
            show_disconnect_btn.set(false);

            let popup_visible = popup_visible.clone();
            let timeout = Timeout::new(250, move || {
                popup_visible.set(false);
            });
            close_timeout.set(Some(timeout));
        })
    };

    // Register global mousedown listener
    {
        let on_mousedown = on_mousedown.clone();
        use_effect_with((*popup_open,), move |_| {
            let document = web_sys::window().and_then(|w| w.document());
            let closure = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                on_mousedown.emit(e);
            }) as Box<dyn FnMut(_)>);

            if let Some(doc) = &document {
                let _ =
                    doc.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
            }

            let document_for_cleanup = document.clone();
            move || {
                if let Some(doc) = document_for_cleanup {
                    let _ = doc
                        .remove_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
                }
            }
        });
    }

    let on_wifi_menu_click = {
        let show_disconnect_btn = show_disconnect_btn.clone();
        Callback::from(move |_| {
            show_disconnect_btn.set(true);
        })
    };

    let on_wifi_disconnect = {
        let on_disconnect = props.on_disconnect.clone();
        Callback::from(move |_| {
            on_disconnect.emit(());
        })
    };

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
                                web_sys::console::log_1(&format!("Saved volume: {}", value).into());
                            }
                        });
                    }
                }
            }
        })
    };

    if !props.visible {
        return html! {};
    }

    let popup_class = {
        let mut classes = vec!["wifi-speaker-popup"];
        if !*popup_visible {
            classes.push("display-none");
        }
        if *popup_open {
            classes.push("open");
        }
        classes.join(" ")
    };

    let wifi_menu_class = if *show_disconnect_btn {
        "right-btn menu display-none"
    } else {
        "right-btn menu"
    };

    let wifi_disconnect_class = if *show_disconnect_btn {
        "right-btn disconnect"
    } else {
        "right-btn disconnect display-none"
    };

    html! {
        <>
            <div class="top-bar">
                <div></div>
                <div></div>
                <div>
                    <button class="top-bar-btn">
                        <i class="fa-solid fa-user"></i>
                    </button>
                    <button class="top-bar-btn">
                        <i class="fa-solid fa-bell"></i>
                    </button>
                    <button class="top-bar-btn" id="top-bar-wifi-audio-btn" onclick={toggle_popup.clone()}>
                        <i class="fa-solid fa-volume-high spacer-right"></i>
                        <i class="fa-solid fa-wifi"></i>
                    </button>
                    <button class="time-btn top-bar-btn" id="top-bar-time">
                        {(*current_time).clone()}
                    </button>
                </div>
            </div>
            <div class="top-bar-popups">
                <div class={popup_class}>
                    <p>{"Wifi connection"}</p>
                    <div class="long-btn-wrapper">
                        <div>
                            <i class="fa-solid fa-wifi powered-on"></i>
                            {" fabi-sc.de"}
                        </div>
                        <button class={wifi_disconnect_class} id="wifi-disconnect" onclick={on_wifi_disconnect}>
                            <i class="fa-solid fa-link-slash"></i>
                            {" Disconnect"}
                        </button>
                        <button class={wifi_menu_class} id="wifi-extended-menu" onclick={on_wifi_menu_click}>
                            <i class="fa-solid fa-ellipsis"></i>
                        </button>
                    </div>
                    <hr />
                    <p>{"Volume "}<span id="volume-display">{(*volume_display).clone()}</span></p>
                    <input
                        class="slider headphone"
                        id="volume-slider"
                        type="range"
                        min="0"
                        max="100"
                        value={(*volume).to_string()}
                        oninput={on_volume_input}
                        onchange={on_volume_change}
                    />
                </div>
            </div>
        </>
    }
}
