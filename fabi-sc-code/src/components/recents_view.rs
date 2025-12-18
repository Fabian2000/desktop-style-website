//! Recents/App Switcher View
//!
//! Shows all open apps as cards that can be swiped away to close.

use web_sys::{MouseEvent, TouchEvent};
use yew::prelude::*;

/// Info about an open app for display in recents
#[derive(Clone, PartialEq)]
pub struct RecentsAppInfo {
    pub window_id: String,
    pub app_id: String,
    pub title: String,
}

#[derive(Properties, PartialEq)]
pub struct RecentsViewProps {
    pub visible: bool,
    pub apps: Vec<RecentsAppInfo>,
    pub on_select: Callback<String>,   // window_id
    pub on_close: Callback<String>,    // window_id
    pub on_dismiss: Callback<()>,      // Close recents view
}

#[function_component(RecentsView)]
pub fn recents_view(props: &RecentsViewProps) -> Html {
    let swiping_card = use_state(|| Option::<String>::None);
    let swipe_offset = use_state(|| 0i32);
    let swipe_start_x = use_state(|| 0i32);

    if !props.visible {
        return html! {};
    }

    // Get icon for app
    let get_icon_class = |app_id: &str| -> &'static str {
        match app_id {
            "terminal" => "fa-solid fa-terminal",
            "files" => "fa-solid fa-folder",
            "browser" => "fa-solid fa-globe",
            "settings" => "fa-solid fa-gear",
            "gallery" => "fa-solid fa-images",
            "music" => "fa-solid fa-music",
            "contacts" => "fa-solid fa-address-book",
            "info" | "about" => "fa-solid fa-circle-info",
            _ => "fa-solid fa-cube",
        }
    };

    // Get display title
    let get_title = |app_id: &str, title: &str| -> String {
        if title == app_id {
            match app_id {
                "terminal" => "Terminal".to_string(),
                "files" => "Files".to_string(),
                "browser" => "Browser".to_string(),
                "settings" => "Settings".to_string(),
                "gallery" => "Gallery".to_string(),
                "music" => "Music".to_string(),
                "contacts" => "Contacts".to_string(),
                "info" | "about" => "About".to_string(),
                _ => title.to_string(),
            }
        } else {
            title.to_string()
        }
    };

    // Click on backdrop dismisses
    let on_backdrop_click = {
        let on_dismiss = props.on_dismiss.clone();
        Callback::from(move |_: MouseEvent| {
            on_dismiss.emit(());
        })
    };

    // Clone for empty state click
    let on_empty_click = {
        let on_dismiss = props.on_dismiss.clone();
        Callback::from(move |_: MouseEvent| {
            on_dismiss.emit(());
        })
    };

    html! {
        <div class="recents-view" onclick={on_backdrop_click.clone()}>
            <div class="recents-container">
                if props.apps.is_empty() {
                    // Empty state - clicking anywhere dismisses
                    <div class="recents-empty" onclick={on_empty_click}>
                        <i class="fa-solid fa-layer-group"></i>
                        <p>{"Keine offenen Apps"}</p>
                    </div>
                } else {
                    <div class="recents-list">
                        { for props.apps.iter().map(|app| {
                            let window_id = app.window_id.clone();
                            let window_id_close = app.window_id.clone();
                            let icon_class = get_icon_class(&app.app_id);
                            let title = get_title(&app.app_id, &app.title);

                            // Select app on click
                            let on_select = props.on_select.clone();
                            let on_card_click = {
                                let window_id = window_id.clone();
                                Callback::from(move |e: MouseEvent| {
                                    e.stop_propagation();
                                    on_select.emit(window_id.clone());
                                })
                            };

                            // Close button
                            let on_close = props.on_close.clone();
                            let on_close_click = {
                                let window_id = window_id_close.clone();
                                Callback::from(move |e: MouseEvent| {
                                    e.stop_propagation();
                                    on_close.emit(window_id.clone());
                                })
                            };

                            // Touch handlers for swipe-to-close (horizontal swipe)
                            let swiping = swiping_card.clone();
                            let offset = swipe_offset.clone();
                            let start_x = swipe_start_x.clone();
                            let on_close_swipe = props.on_close.clone();

                            let on_touch_start = {
                                let window_id = window_id.clone();
                                let swiping = swiping.clone();
                                let start_x = start_x.clone();
                                let offset = offset.clone();
                                Callback::from(move |e: TouchEvent| {
                                    if let Some(touch) = e.touches().get(0) {
                                        swiping.set(Some(window_id.clone()));
                                        start_x.set(touch.client_x());
                                        offset.set(0);
                                    }
                                })
                            };

                            let on_touch_move = {
                                let window_id = window_id.clone();
                                let swiping = swiping.clone();
                                let start_x = start_x.clone();
                                let offset = offset.clone();
                                Callback::from(move |e: TouchEvent| {
                                    if *swiping == Some(window_id.clone()) {
                                        if let Some(touch) = e.touches().get(0) {
                                            let delta = touch.client_x() - *start_x;
                                            // Allow both left and right swipe
                                            offset.set(delta);
                                        }
                                    }
                                })
                            };

                            let on_touch_end = {
                                let window_id = window_id.clone();
                                let swiping = swiping.clone();
                                let offset = offset.clone();
                                let on_close = on_close_swipe.clone();
                                Callback::from(move |_: TouchEvent| {
                                    if *swiping == Some(window_id.clone()) {
                                        // If swiped more than 100px left or right, close
                                        if (*offset).abs() > 100 {
                                            on_close.emit(window_id.clone());
                                        }
                                        swiping.set(None);
                                        offset.set(0);
                                    }
                                })
                            };

                            // Calculate card style for swipe animation (horizontal)
                            let is_swiping = *swiping_card == Some(window_id.clone());
                            let card_style = if is_swiping && *swipe_offset != 0 {
                                format!(
                                    "transform: translateX({}px); opacity: {};",
                                    *swipe_offset,
                                    1.0 - ((*swipe_offset).abs() as f32 / 200.0).min(0.8)
                                )
                            } else {
                                String::new()
                            };

                            // Trash icon position based on swipe - flies in/out smoothly
                            let trash_side = if *swipe_offset > 0 { "left" } else { "right" };
                            // Calculate how far the trash icon should be visible (0 = hidden, 1 = fully visible)
                            let trash_progress = if is_swiping {
                                (((*swipe_offset).abs() as f32 - 30.0) / 50.0).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let trash_style = if trash_progress > 0.0 {
                                // Animate from outside (-60px) to final position (0px)
                                let offset = (1.0 - trash_progress) * 60.0;
                                let direction = if *swipe_offset > 0 { -1.0 } else { 1.0 };
                                format!(
                                    "transform: translateY(-50%) translateX({}px); opacity: {};",
                                    offset * direction,
                                    trash_progress
                                )
                            } else {
                                "display: none;".to_string()
                            };

                            html! {
                                <div class="recents-card-wrapper">
                                    // Trash indicator - position animated based on swipe
                                    <div class={format!("recents-trash-indicator {}", trash_side)} style={trash_style}>
                                        <i class="fa-solid fa-trash-can"></i>
                                    </div>
                                    <div
                                        class="recents-card"
                                        style={card_style}
                                        onclick={on_card_click}
                                        ontouchstart={on_touch_start}
                                        ontouchmove={on_touch_move}
                                        ontouchend={on_touch_end}
                                    >
                                        <div class="recents-card-header">
                                            <div class="recents-card-title">
                                                <i class={icon_class}></i>
                                                <span>{title}</span>
                                            </div>
                                            <button class="recents-card-close" onclick={on_close_click}>
                                                <i class="fa-solid fa-xmark"></i>
                                            </button>
                                        </div>
                                        <div class="recents-card-preview">
                                            <i class={format!("{} fa-4x", icon_class)}></i>
                                        </div>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }
            </div>
        </div>
    }
}
