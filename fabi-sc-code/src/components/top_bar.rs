use super::calendar_popup::CalendarPopup;
use super::notification_panel::NotificationPanel;
use crate::database::IndexedDb;
use crate::utils::get_local_time_no_sec;
use gloo_timers::callback::{Interval, Timeout};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

fn is_portrait() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(orientation: portrait)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

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
    let calendar_popup_open = use_state(|| false);
    let calendar_popup_visible = use_state(|| false);
    let show_disconnect_btn = use_state(|| false);
    let volume = use_state(|| 0i32);
    let volume_display = use_state(|| String::from("0 %"));
    let close_timeout = use_state(|| None::<Timeout>);
    let calendar_close_timeout = use_state(|| None::<Timeout>);

    // Mobile notification panel state
    let panel_open = use_state(|| false);
    let panel_visible = use_state(|| false);
    let panel_dragging = use_state(|| false);
    let panel_drag_offset = use_state(|| 0.0f32);
    let panel_close_timeout = use_state(|| None::<Timeout>);

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

    let toggle_calendar_popup = {
        let calendar_popup_open = calendar_popup_open.clone();
        let calendar_popup_visible = calendar_popup_visible.clone();
        let calendar_close_timeout = calendar_close_timeout.clone();

        Callback::from(move |_| {
            if *calendar_popup_open {
                // Close popup
                calendar_popup_open.set(false);

                let calendar_popup_visible = calendar_popup_visible.clone();
                let timeout = Timeout::new(250, move || {
                    calendar_popup_visible.set(false);
                });
                calendar_close_timeout.set(Some(timeout));
            } else {
                // Open popup - cancel any pending close timeout
                calendar_close_timeout.set(None);

                calendar_popup_visible.set(true);

                let calendar_popup_open = calendar_popup_open.clone();
                if let Some(window) = web_sys::window() {
                    let closure = Closure::once(Box::new(move || {
                        calendar_popup_open.set(true);
                    }) as Box<dyn FnOnce()>);
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                    closure.forget();
                }
            }
        })
    };

    // Global mousedown listener for calendar popup
    {
        let calendar_popup_open = calendar_popup_open.clone();
        let calendar_popup_visible = calendar_popup_visible.clone();
        let calendar_close_timeout = calendar_close_timeout.clone();

        use_effect_with((*calendar_popup_open,), move |_| {
            let calendar_popup_open = calendar_popup_open.clone();
            let calendar_popup_visible = calendar_popup_visible.clone();
            let calendar_close_timeout = calendar_close_timeout.clone();

            let document = web_sys::window().and_then(|w| w.document());
            let closure = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                if !*calendar_popup_open {
                    return;
                }

                if let Some(target) = e.target() {
                    if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                            // Check if clicked inside calendar popup
                            if let Ok(Some(popup)) = document.query_selector(".calendar-popup") {
                                if popup.contains(Some(element)) {
                                    return;
                                }
                            }
                            // Check if clicked on time button
                            if let Ok(Some(btn)) = document.query_selector("#top-bar-time") {
                                if btn.contains(Some(element)) {
                                    return;
                                }
                            }
                        }
                    }
                }

                // Close popup
                calendar_popup_open.set(false);

                let calendar_popup_visible = calendar_popup_visible.clone();
                let timeout = Timeout::new(250, move || {
                    calendar_popup_visible.set(false);
                });
                calendar_close_timeout.set(Some(timeout));
            }) as Box<dyn FnMut(_)>);

            if let Some(doc) = &document {
                let _ = doc.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
            }

            let document_for_cleanup = document.clone();
            move || {
                if let Some(doc) = document_for_cleanup {
                    let _ = doc.remove_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
                }
            }
        });
    }

    // Touch event handlers for mobile notification panel
    // Use a single shared state struct for all touch handlers
    {
        let panel_open = panel_open.clone();
        let panel_visible = panel_visible.clone();
        let panel_dragging = panel_dragging.clone();
        let panel_drag_offset = panel_drag_offset.clone();
        let panel_close_timeout = panel_close_timeout.clone();

        use_effect_with((), move |_| {
            let panel_open = panel_open.clone();
            let panel_visible = panel_visible.clone();
            let panel_dragging = panel_dragging.clone();
            let panel_drag_offset = panel_drag_offset.clone();
            let panel_close_timeout = panel_close_timeout.clone();

            // Single shared state for all handlers
            #[derive(Default)]
            struct TouchState {
                start_y: f32,
                is_active: bool,
                is_dragging: bool,
                was_panel_open: bool,
                offset: f32,
                on_interactive: bool,
            }
            let state = Rc::new(RefCell::new(TouchState::default()));

            let document = web_sys::window().and_then(|w| w.document());

            // Touchstart handler
            let state_start = state.clone();
            let panel_visible_start = panel_visible.clone();
            let panel_dragging_start = panel_dragging.clone();
            let panel_open_start = panel_open.clone();

            let touchstart_closure = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
                if !is_portrait() {
                    return;
                }

                // Check if panel is open by looking at the DOM
                // Panel has .open class when open and not dragging, or check if it's visible at all
                let panel_is_open = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| {
                        // Check for .open class first
                        if d.query_selector(".notification-panel.open").ok().flatten().is_some() {
                            return Some(true);
                        }
                        // Also check if panel exists and has transform: translateY(0) (meaning it's open)
                        if let Some(panel) = d.query_selector(".notification-panel").ok().flatten() {
                            if let Some(style) = panel.get_attribute("style") {
                                if style.contains("translateY(0") || style.contains("translateY(-") && !style.contains("translateY(-100%)") {
                                    return Some(true);
                                }
                            }
                        }
                        Some(false)
                    })
                    .unwrap_or(false);

                let mut is_on_interactive = false;

                // Check if touch is on interactive elements (sliders, buttons) - don't intercept those
                if let Some(target) = e.target() {
                    if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                        // Check for slider or button elements directly
                        let tag = element.tag_name().to_lowercase();
                        if tag == "input" || tag == "button" {
                            is_on_interactive = true;
                        }
                        // Check if inside a button (for child elements like <i>, <span>, <div>)
                        if element.closest("button").ok().flatten().is_some() {
                            is_on_interactive = true;
                        }
                        // Check if inside input
                        if element.closest("input").ok().flatten().is_some() {
                            is_on_interactive = true;
                        }
                        // Check if inside slider container
                        if element.closest(".slider-container").ok().flatten().is_some() {
                            is_on_interactive = true;
                        }
                    }
                }

                if let Some(touch) = e.touches().get(0) {
                    let y = touch.client_y() as f32;

                    let mut s = state_start.borrow_mut();
                    s.start_y = y;
                    s.is_active = true;
                    s.was_panel_open = panel_is_open;
                    s.offset = 0.0;
                    s.on_interactive = is_on_interactive;

                    // If touching interactive element, let the element handle the touch
                    if is_on_interactive {
                        return;
                    }

                    // Start drag if: touching near top OR panel is already open
                    if y < 60.0 || panel_is_open {
                        e.prevent_default();
                        s.is_dragging = true;
                        drop(s); // Release borrow before setting state
                        panel_visible_start.set(true);
                        panel_dragging_start.set(true);
                    }
                }
            }) as Box<dyn FnMut(_)>);

            // Touchmove handler
            let state_move = state.clone();
            let panel_drag_offset_move = panel_drag_offset.clone();
            let panel_visible_move = panel_visible.clone();
            let panel_dragging_move = panel_dragging.clone();

            let touchmove_closure = Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
                if !is_portrait() {
                    return;
                }

                let mut s = state_move.borrow_mut();

                if !s.is_active {
                    return;
                }

                // Skip if touch started on interactive element
                if s.on_interactive {
                    return;
                }

                if let Some(touch) = e.touches().get(0) {
                    let y = touch.client_y() as f32;
                    let delta = y - s.start_y;

                    // If not dragging yet, check if this is a downward swipe to start drag
                    if !s.is_dragging && delta > 20.0 && !s.was_panel_open {
                        // Started dragging down from desktop area - open panel
                        s.is_dragging = true;
                        drop(s); // Release borrow
                        panel_visible_move.set(true);
                        panel_dragging_move.set(true);
                        return;
                    }

                    if !s.is_dragging {
                        return;
                    }

                    // Calculate offset based on current state
                    let offset = if s.was_panel_open {
                        // Panel is open, allow dragging up to close
                        delta.min(0.0)
                    } else {
                        // Panel is closed, allow dragging down to open
                        delta.max(0.0)
                    };

                    s.offset = offset;
                    drop(s); // Release borrow before setting state
                    panel_drag_offset_move.set(offset);

                    // Prevent default scrolling when dragging panel
                    e.prevent_default();
                }
            }) as Box<dyn FnMut(_)>);

            // Touchend handler
            let state_end = state.clone();
            let panel_drag_offset_end = panel_drag_offset.clone();
            let panel_open_end = panel_open.clone();
            let panel_visible_end = panel_visible.clone();
            let panel_dragging_end = panel_dragging.clone();
            let panel_close_timeout_end = panel_close_timeout.clone();

            let touchend_closure = Closure::wrap(Box::new(move |_e: web_sys::TouchEvent| {
                if !is_portrait() {
                    return;
                }

                let mut s = state_end.borrow_mut();
                let offset = s.offset;
                let was_dragging = s.is_dragging;
                let was_panel_open = s.was_panel_open;
                let was_on_interactive = s.on_interactive;

                // Reset state
                s.is_active = false;
                s.is_dragging = false;
                s.offset = 0.0;
                s.on_interactive = false;
                drop(s); // Release borrow

                panel_dragging_end.set(false);
                panel_drag_offset_end.set(0.0);

                // Skip if touch was on interactive element
                if was_on_interactive {
                    return;
                }

                if !was_dragging {
                    return;
                }

                let threshold = 100.0;

                if was_panel_open {
                    // Panel was open
                    if offset < -threshold {
                        // Dragged up enough to close
                        panel_open_end.set(false);
                        let panel_visible_timeout = panel_visible_end.clone();
                        let timeout = Timeout::new(300, move || {
                            panel_visible_timeout.set(false);
                        });
                        panel_close_timeout_end.set(Some(timeout));
                    }
                    // Otherwise keep it open
                } else {
                    // Panel was closed
                    if offset > threshold {
                        // Dragged down enough to open
                        panel_open_end.set(true);
                    } else {
                        // Not enough drag, hide panel
                        let panel_visible_timeout = panel_visible_end.clone();
                        let timeout = Timeout::new(300, move || {
                            panel_visible_timeout.set(false);
                        });
                        panel_close_timeout_end.set(Some(timeout));
                    }
                }
            }) as Box<dyn FnMut(_)>);

            if let Some(doc) = &document {
                let mut opts = web_sys::AddEventListenerOptions::new();
                opts.set_passive(false);

                let _ = doc.add_event_listener_with_callback_and_add_event_listener_options(
                    "touchstart",
                    touchstart_closure.as_ref().unchecked_ref(),
                    &opts,
                );
                let _ = doc.add_event_listener_with_callback_and_add_event_listener_options(
                    "touchmove",
                    touchmove_closure.as_ref().unchecked_ref(),
                    &opts,
                );
                let _ = doc.add_event_listener_with_callback("touchend", touchend_closure.as_ref().unchecked_ref());
            }

            let document_for_cleanup = document.clone();
            move || {
                if let Some(doc) = document_for_cleanup {
                    let _ = doc.remove_event_listener_with_callback("touchstart", touchstart_closure.as_ref().unchecked_ref());
                    let _ = doc.remove_event_listener_with_callback("touchmove", touchmove_closure.as_ref().unchecked_ref());
                    let _ = doc.remove_event_listener_with_callback("touchend", touchend_closure.as_ref().unchecked_ref());
                }
            }
        });
    }

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
                    <button class="time-btn top-bar-btn" id="top-bar-time" onclick={toggle_calendar_popup}>
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
                <CalendarPopup visible={*calendar_popup_visible} open={*calendar_popup_open} />
            </div>
            <NotificationPanel
                visible={*panel_visible}
                open={*panel_open}
                drag_offset={*panel_drag_offset}
                is_dragging={*panel_dragging}
                on_disconnect={props.on_disconnect.clone()}
            />
        </>
    }
}
