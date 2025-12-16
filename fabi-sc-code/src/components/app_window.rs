//! App Window Component
//!
//! A draggable, resizable window that can host app content.
//! Desktop: Traditional windowed mode with drag/resize
//! Mobile: Fullscreen with navigation bar (back button)

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;
use yew::prelude::*;

/// Window state for minimize/maximize
#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

#[derive(Properties, PartialEq, Clone)]
pub struct AppWindowProps {
    pub window_id: String,
    pub app_id: String,
    pub title: String,
    #[prop_or(100)]
    pub x: i32,
    #[prop_or(100)]
    pub y: i32,
    #[prop_or(600)]
    pub width: u32,
    #[prop_or(400)]
    pub height: u32,
    #[prop_or(100)]
    pub z_index: u32,
    #[prop_or_default]
    pub on_close: Callback<()>,
    #[prop_or_default]
    pub on_focus: Callback<()>,
    #[prop_or_default]
    pub on_minimize: Callback<()>,
}

/// Check if we're on a mobile device (portrait orientation)
fn is_mobile() -> bool {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|w| w.as_f64())
        .map(|width| {
            let height = web_sys::window()
                .and_then(|w| w.inner_height().ok())
                .and_then(|h| h.as_f64())
                .unwrap_or(0.0);
            height > width // Portrait = mobile
        })
        .unwrap_or(false)
}

#[function_component(AppWindow)]
pub fn app_window(props: &AppWindowProps) -> Html {
    let position = use_state(|| (props.x, props.y));
    let size = use_state(|| (props.width, props.height));
    let window_state = use_state(|| WindowState::Normal);
    let dragging = use_state(|| false);
    let resizing = use_state(|| false);
    let drag_offset = use_state(|| (0i32, 0i32));
    let closing = use_state(|| false);

    // Store pre-maximize position/size for restore
    let pre_maximize = use_state(|| None::<(i32, i32, u32, u32)>);

    // Handle drag start (desktop only, not when maximized)
    let on_titlebar_mousedown = {
        let dragging = dragging.clone();
        let drag_offset = drag_offset.clone();
        let position = position.clone();
        let window_state = window_state.clone();
        let on_focus = props.on_focus.clone();
        Callback::from(move |e: MouseEvent| {
            if is_mobile() || *window_state == WindowState::Maximized {
                return;
            }
            e.prevent_default();
            on_focus.emit(());
            dragging.set(true);
            let (x, y) = *position;
            drag_offset.set((e.client_x() - x, e.client_y() - y));
        })
    };

    // Handle resize start (desktop only, not when maximized)
    let on_resize_mousedown = {
        let resizing = resizing.clone();
        let window_state = window_state.clone();
        let on_focus = props.on_focus.clone();
        Callback::from(move |e: MouseEvent| {
            if is_mobile() || *window_state == WindowState::Maximized {
                return;
            }
            e.prevent_default();
            e.stop_propagation();
            on_focus.emit(());
            resizing.set(true);
        })
    };

    // Global mouse move/up handlers for drag/resize
    {
        let dragging = dragging.clone();
        let resizing = resizing.clone();
        let position = position.clone();
        let size = size.clone();
        let drag_offset = drag_offset.clone();

        use_effect_with(
            (*dragging, *resizing),
            move |(is_dragging, is_resizing)| {
                let document = web_sys::window().and_then(|w| w.document());

                let cleanup: Option<(Closure<dyn FnMut(MouseEvent)>, Closure<dyn FnMut(MouseEvent)>)> =
                    if *is_dragging || *is_resizing {
                        let dragging_clone = dragging.clone();
                        let resizing_clone = resizing.clone();
                        let position_clone = position.clone();
                        let size_clone = size.clone();
                        let drag_offset_clone = drag_offset.clone();
                        let is_dragging = *is_dragging;

                        let mousemove = Closure::wrap(Box::new(move |e: MouseEvent| {
                            if is_dragging {
                                let (offset_x, offset_y) = *drag_offset_clone;
                                let new_x = (e.client_x() - offset_x).max(0);
                                let new_y = (e.client_y() - offset_y).max(30);
                                position_clone.set((new_x, new_y));
                            } else {
                                // Resizing
                                let (current_x, current_y) = *position_clone;
                                let new_width = ((e.client_x() - current_x) as u32).max(300);
                                let new_height = ((e.client_y() - current_y) as u32).max(200);
                                size_clone.set((new_width, new_height));
                            }
                        }) as Box<dyn FnMut(_)>);

                        let dragging_up = dragging_clone.clone();
                        let resizing_up = resizing_clone.clone();
                        let mouseup = Closure::wrap(Box::new(move |_: MouseEvent| {
                            dragging_up.set(false);
                            resizing_up.set(false);
                        }) as Box<dyn FnMut(_)>);

                        if let Some(doc) = &document {
                            let _ = doc.add_event_listener_with_callback(
                                "mousemove",
                                mousemove.as_ref().unchecked_ref(),
                            );
                            let _ = doc.add_event_listener_with_callback(
                                "mouseup",
                                mouseup.as_ref().unchecked_ref(),
                            );
                        }

                        Some((mousemove, mouseup))
                    } else {
                        None
                    };

                let document_cleanup = document.clone();
                move || {
                    if let (Some(doc), Some((mousemove, mouseup))) = (document_cleanup, cleanup) {
                        let _ = doc.remove_event_listener_with_callback(
                            "mousemove",
                            mousemove.as_ref().unchecked_ref(),
                        );
                        let _ = doc.remove_event_listener_with_callback(
                            "mouseup",
                            mouseup.as_ref().unchecked_ref(),
                        );
                    }
                }
            },
        );
    }

    // Minimize button handler
    let on_minimize_click = {
        let window_state = window_state.clone();
        let on_minimize = props.on_minimize.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            window_state.set(WindowState::Minimized);
            on_minimize.emit(());
        })
    };

    // Maximize/Restore button handler
    let on_maximize_click = {
        let window_state = window_state.clone();
        let position = position.clone();
        let size = size.clone();
        let pre_maximize = pre_maximize.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            if *window_state == WindowState::Maximized {
                // Restore to previous size/position
                if let Some((x, y, w, h)) = *pre_maximize {
                    position.set((x, y));
                    size.set((w, h));
                }
                window_state.set(WindowState::Normal);
            } else {
                // Save current position/size and maximize
                let (x, y) = *position;
                let (w, h) = *size;
                pre_maximize.set(Some((x, y, w, h)));
                window_state.set(WindowState::Maximized);
            }
        })
    };

    // Close button handler (used for both desktop close and mobile back)
    let on_close_click = {
        let closing = closing.clone();
        let on_close = props.on_close.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            closing.set(true);
            // Delay actual close for animation
            // TODO: Trigger Python on_exit event here before closing
            let on_close = on_close.clone();
            gloo_timers::callback::Timeout::new(200, move || {
                on_close.emit(());
            })
            .forget();
        })
    };

    // Focus on window click
    let on_window_mousedown = {
        let on_focus = props.on_focus.clone();
        Callback::from(move |_: MouseEvent| {
            on_focus.emit(());
        })
    };

    // Double-click on titlebar to maximize/restore
    let on_titlebar_dblclick = {
        let window_state = window_state.clone();
        let position = position.clone();
        let size = size.clone();
        let pre_maximize = pre_maximize.clone();
        Callback::from(move |e: MouseEvent| {
            if is_mobile() {
                return;
            }
            e.prevent_default();
            if *window_state == WindowState::Maximized {
                if let Some((x, y, w, h)) = *pre_maximize {
                    position.set((x, y));
                    size.set((w, h));
                }
                window_state.set(WindowState::Normal);
            } else {
                let (x, y) = *position;
                let (w, h) = *size;
                pre_maximize.set(Some((x, y, w, h)));
                window_state.set(WindowState::Maximized);
            }
        })
    };

    // Get icon for app
    let icon_class = match props.app_id.as_str() {
        "terminal" => "fa-solid fa-terminal",
        "files" => "fa-solid fa-folder",
        "browser" => "fa-solid fa-globe",
        "settings" => "fa-solid fa-gear",
        "gallery" => "fa-solid fa-images",
        "music" => "fa-solid fa-music",
        "contacts" => "fa-solid fa-address-book",
        "info" | "about" => "fa-solid fa-circle-info",
        _ => "fa-solid fa-cube",
    };

    // Get title
    let display_title = if props.title == props.app_id {
        match props.app_id.as_str() {
            "terminal" => "Terminal",
            "files" => "Files",
            "browser" => "Browser",
            "settings" => "Settings",
            "gallery" => "Gallery",
            "music" => "Music",
            "contacts" => "Contacts",
            "info" | "about" => "About",
            _ => &props.title,
        }
    } else {
        &props.title
    };

    // Build classes and styles based on state
    let is_mobile_view = is_mobile();
    let is_maximized = *window_state == WindowState::Maximized;
    let is_minimized = *window_state == WindowState::Minimized;

    // Window class
    let mut window_classes = vec!["app-window"];
    if *closing {
        window_classes.push("closing");
    }
    if is_maximized || is_mobile_view {
        window_classes.push("maximized");
    }
    if is_minimized {
        window_classes.push("minimized");
    }
    if is_mobile_view {
        window_classes.push("mobile");
    }
    let window_class = window_classes.join(" ");

    // Window style - only apply position/size on desktop in normal mode
    let window_style = if is_mobile_view || is_maximized {
        format!("z-index: {};", props.z_index)
    } else {
        let (x, y) = *position;
        let (width, height) = *size;
        format!(
            "left: {}px; top: {}px; width: {}px; height: {}px; z-index: {};",
            x, y, width, height, props.z_index
        )
    };

    html! {
        <div
            class={window_class}
            style={window_style}
            onmousedown={on_window_mousedown}
        >
            // Desktop: Title bar with window controls
            if !is_mobile_view {
                <div
                    class="window-titlebar"
                    onmousedown={on_titlebar_mousedown}
                    ondblclick={on_titlebar_dblclick}
                >
                    <div class="window-title">
                        <i class={icon_class.to_string()}></i>
                        <span>{display_title}</span>
                    </div>
                    <div class="window-controls">
                        <button
                            class="window-btn minimize"
                            onclick={on_minimize_click}
                            title="Minimieren"
                        >
                            <i class="fa-solid fa-minus"></i>
                        </button>
                        <button
                            class="window-btn maximize"
                            onclick={on_maximize_click}
                            title={if is_maximized { "Wiederherstellen" } else { "Maximieren" }}
                        >
                            if is_maximized {
                                <i class="fa-regular fa-window-restore"></i>
                            } else {
                                <i class="fa-regular fa-square"></i>
                            }
                        </button>
                        <button
                            class="window-btn close"
                            onclick={on_close_click.clone()}
                            title="Schließen"
                        >
                            <i class="fa-solid fa-xmark"></i>
                        </button>
                    </div>
                </div>
            }

            // Mobile: Header with back button and title
            if is_mobile_view {
                <div class="mobile-header">
                    <button class="mobile-back-btn" onclick={on_close_click.clone()}>
                        <i class="fa-solid fa-chevron-left"></i>
                    </button>
                    <div class="mobile-title">
                        <i class={icon_class.to_string()}></i>
                        <span>{display_title}</span>
                    </div>
                    <div class="mobile-header-spacer"></div>
                </div>
            }

            // Content area
            <div class="window-content">
                // Placeholder content - will be replaced with Python app output
                <div style="color: #888; text-align: center; padding-top: 50px;">
                    <i class={format!("{} fa-3x", icon_class)} style="margin-bottom: 20px; display: block;"></i>
                    <p>{format!("App: {}", props.app_id)}</p>
                    <p style="font-size: 12px; color: #666;">{"Python Runtime wird geladen..."}</p>
                </div>
            </div>


            // Resize handle (desktop only, not when maximized)
            if !is_mobile_view && !is_maximized {
                <div class="window-resize-handle" onmousedown={on_resize_mousedown}></div>
            }
        </div>
    }
}
