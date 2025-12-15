use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct AppInfo {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

#[derive(Properties, PartialEq)]
pub struct TaskbarProps {
    pub visible: bool,
    #[prop_or_default]
    pub active_app: Option<String>,
    #[prop_or_default]
    pub on_app_click: Callback<String>,
}

// Desktop dock apps (pinned favorites)
fn get_dock_apps() -> Vec<AppInfo> {
    vec![
        AppInfo {
            id: "files",
            icon: "fa-solid fa-folder",
            label: "Files",
        },
        AppInfo {
            id: "browser",
            icon: "fa-solid fa-globe",
            label: "Browser",
        },
        AppInfo {
            id: "terminal",
            icon: "fa-solid fa-terminal",
            label: "Terminal",
        },
        AppInfo {
            id: "settings",
            icon: "fa-solid fa-gear",
            label: "Settings",
        },
    ]
}

// All apps for the app drawer/start menu
fn get_all_apps() -> Vec<AppInfo> {
    vec![
        AppInfo {
            id: "browser",
            icon: "fa-solid fa-globe",
            label: "Browser",
        },
        AppInfo {
            id: "files",
            icon: "fa-solid fa-folder",
            label: "Files",
        },
        AppInfo {
            id: "terminal",
            icon: "fa-solid fa-terminal",
            label: "Terminal",
        },
        AppInfo {
            id: "settings",
            icon: "fa-solid fa-gear",
            label: "Settings",
        },
        AppInfo {
            id: "info",
            icon: "fa-solid fa-circle-info",
            label: "About",
        },
        AppInfo {
            id: "gallery",
            icon: "fa-solid fa-images",
            label: "Gallery",
        },
        AppInfo {
            id: "music",
            icon: "fa-solid fa-music",
            label: "Music",
        },
        AppInfo {
            id: "contacts",
            icon: "fa-solid fa-address-book",
            label: "Contacts",
        },
    ]
}

// Mobile: Dock favorites (bottom bar)
fn get_mobile_dock_apps() -> Vec<AppInfo> {
    vec![
        AppInfo {
            id: "browser",
            icon: "fa-solid fa-globe",
            label: "Browser",
        },
        AppInfo {
            id: "files",
            icon: "fa-solid fa-folder",
            label: "Files",
        },
        AppInfo {
            id: "settings",
            icon: "fa-solid fa-gear",
            label: "Settings",
        },
    ]
}

#[function_component(Taskbar)]
pub fn taskbar(props: &TaskbarProps) -> Html {
    if !props.visible {
        return html! {};
    }

    let dock_apps = get_dock_apps();
    let all_apps = get_all_apps();
    let mobile_dock_apps = get_mobile_dock_apps();

    // App drawer/start menu state
    let drawer_open = use_state(|| false);
    let drawer_dragging = use_state(|| false);
    let drawer_offset = use_state(|| 0.0f64);
    let touch_start_y = use_state(|| 0.0f64);

    // Toggle start menu (desktop)
    let toggle_start_menu = {
        let drawer_open = drawer_open.clone();
        Callback::from(move |_| {
            drawer_open.set(!*drawer_open);
        })
    };

    // Swipe up on dock area to open drawer (mobile)
    let on_dock_touch_start = {
        let touch_start_y = touch_start_y.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                touch_start_y.set(touch.client_y() as f64);
            }
        })
    };

    let on_dock_touch_move = {
        let touch_start_y = touch_start_y.clone();
        let drawer_offset = drawer_offset.clone();
        let drawer_dragging = drawer_dragging.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                let current_y = touch.client_y() as f64;
                let delta = *touch_start_y - current_y;
                if delta > 20.0 {
                    drawer_dragging.set(true);
                    drawer_offset.set(delta.min(500.0));
                }
            }
        })
    };

    let on_dock_touch_end = {
        let drawer_open = drawer_open.clone();
        let drawer_dragging = drawer_dragging.clone();
        let drawer_offset = drawer_offset.clone();
        Callback::from(move |_: TouchEvent| {
            if *drawer_offset > 100.0 {
                drawer_open.set(true);
            }
            drawer_dragging.set(false);
            drawer_offset.set(0.0);
        })
    };

    // Swipe down on drawer to close (mobile) - works anywhere in drawer
    let on_drawer_touch_start = {
        let touch_start_y = touch_start_y.clone();
        Callback::from(move |e: TouchEvent| {
            e.prevent_default(); // Prevent browser pull-to-refresh
            if let Some(touch) = e.touches().get(0) {
                touch_start_y.set(touch.client_y() as f64);
            }
        })
    };

    let on_drawer_touch_move = {
        let touch_start_y = touch_start_y.clone();
        let drawer_offset = drawer_offset.clone();
        let drawer_dragging = drawer_dragging.clone();
        Callback::from(move |e: TouchEvent| {
            e.prevent_default(); // Prevent browser pull-to-refresh
            if let Some(touch) = e.touches().get(0) {
                let current_y = touch.client_y() as f64;
                let delta = current_y - *touch_start_y;
                if delta > 20.0 {
                    drawer_dragging.set(true);
                    drawer_offset.set(delta.min(500.0));
                }
            }
        })
    };

    let on_drawer_touch_end = {
        let drawer_open = drawer_open.clone();
        let drawer_dragging = drawer_dragging.clone();
        let drawer_offset = drawer_offset.clone();
        Callback::from(move |e: TouchEvent| {
            e.prevent_default();
            if *drawer_offset > 100.0 {
                drawer_open.set(false);
            }
            drawer_dragging.set(false);
            drawer_offset.set(0.0);
        })
    };

    // Close drawer when app is clicked
    let close_drawer = {
        let drawer_open = drawer_open.clone();
        Callback::from(move |_: ()| {
            drawer_open.set(false);
        })
    };

    // Global click listener to close start menu when clicking outside (desktop)
    {
        let drawer_open = drawer_open.clone();
        use_effect_with(*drawer_open, move |is_open| {
            let document = web_sys::window().and_then(|w| w.document());

            let closure = if *is_open {
                let drawer_open = drawer_open.clone();
                Some(Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    if let Some(target) = e.target() {
                        if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                // Check if click is inside start-menu or start-btn
                                if let Ok(Some(start_menu)) = doc.query_selector(".start-menu") {
                                    if start_menu.contains(Some(element)) {
                                        return;
                                    }
                                }
                                if let Ok(Some(start_btn)) = doc.query_selector(".start-btn") {
                                    if start_btn.contains(Some(element)) {
                                        return;
                                    }
                                }
                            }
                            // Click was outside - close menu
                            drawer_open.set(false);
                        }
                    }
                }) as Box<dyn FnMut(_)>))
            } else {
                None
            };

            if let (Some(doc), Some(closure)) = (&document, &closure) {
                let _ = doc.add_event_listener_with_callback(
                    "mousedown",
                    closure.as_ref().unchecked_ref(),
                );
            }

            let document_for_cleanup = document.clone();
            move || {
                if let (Some(doc), Some(closure)) = (document_for_cleanup, closure) {
                    let _ = doc.remove_event_listener_with_callback(
                        "mousedown",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    // Drawer classes and style
    let drawer_class = {
        let mut classes = vec!["app-drawer"];
        if *drawer_open && !*drawer_dragging {
            classes.push("open");
        }
        if *drawer_dragging {
            classes.push("dragging");
        }
        classes.join(" ")
    };

    let drawer_style = if *drawer_dragging {
        if *drawer_open {
            format!("transform: translateY({}px)", *drawer_offset)
        } else {
            format!("transform: translateY(calc(100% - {}px))", *drawer_offset)
        }
    } else {
        String::new()
    };

    html! {
        <>
            // Desktop: Floating dock with start button
            <div class="taskbar">
                // Start menu button with logo
                <button class="taskbar-item start-btn" onclick={toggle_start_menu.clone()}>
                    <img class="start-logo" src="resources/img/logo_inverted_bg.webp" alt="Start" />
                </button>
                <div class="taskbar-separator"></div>
                // Pinned apps
                { for dock_apps.iter().map(|app| {
                    let app_id = app.id.to_string();
                    let on_click = {
                        let on_app_click = props.on_app_click.clone();
                        let app_id = app_id.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(app_id.clone());
                        })
                    };

                    let is_active = props.active_app.as_ref().map(|a| a == app.id).unwrap_or(false);
                    let class = if is_active {
                        "taskbar-item active"
                    } else {
                        "taskbar-item"
                    };

                    html! {
                        <button class={class} onclick={on_click}>
                            <i class={format!("taskbar-icon {}", app.icon)}></i>
                        </button>
                    }
                })}
            </div>

            // Desktop: Custom App Launcher
            <div class={if *drawer_open { "start-menu open" } else { "start-menu" }}>
                // Header with user info
                <div class="start-menu-header">
                    <img class="user-avatar" src="resources/img/logo_inverted_bg.webp" alt="User" />
                    <div class="user-details">
                        <div class="user-name">{"fabi-sc"}</div>
                        <div class="user-status">{"Developer"}</div>
                    </div>
                    <div class="header-actions">
                        <button>
                            <i class="fa-solid fa-gear"></i>
                        </button>
                    </div>
                </div>

                // Apps section
                <div class="start-menu-apps">
                    <div class="apps-label">{"Applications"}</div>
                    <div class="start-menu-grid">
                        { for all_apps.iter().map(|app| {
                            let app_id = app.id.to_string();
                            let close_drawer = close_drawer.clone();
                            let on_click = {
                                let on_app_click = props.on_app_click.clone();
                                let app_id = app_id.clone();
                                Callback::from(move |_| {
                                    on_app_click.emit(app_id.clone());
                                    close_drawer.emit(());
                                })
                            };

                            html! {
                                <button class="start-menu-item" onclick={on_click}>
                                    <div class="app-icon-wrapper">
                                        <i class={app.icon}></i>
                                    </div>
                                    <span>{app.label}</span>
                                </button>
                            }
                        })}
                    </div>
                </div>

                // Quick actions footer
                <div class="start-menu-footer">
                    <button class="quick-action">
                        <i class="fa-solid fa-folder"></i>
                        <span>{"Files"}</span>
                    </button>
                    <button class="quick-action">
                        <i class="fa-solid fa-terminal"></i>
                        <span>{"Terminal"}</span>
                    </button>
                    <button class="quick-action power">
                        <i class="fa-solid fa-power-off"></i>
                    </button>
                </div>
            </div>

            // Mobile: Floating icons at bottom
            <div
                class="mobile-dock"
                ontouchstart={on_dock_touch_start}
                ontouchmove={on_dock_touch_move}
                ontouchend={on_dock_touch_end}
            >
                { for mobile_dock_apps.iter().map(|app| {
                    let app_id = app.id.to_string();
                    let on_click = {
                        let on_app_click = props.on_app_click.clone();
                        let app_id = app_id.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(app_id.clone());
                        })
                    };

                    html! {
                        <button class="dock-item" onclick={on_click}>
                            <i class={app.icon}></i>
                        </button>
                    }
                })}
            </div>

            // Mobile: App Drawer (swipe up to open, swipe down anywhere to close)
            <div
                class={drawer_class}
                style={drawer_style}
                ontouchstart={on_drawer_touch_start}
                ontouchmove={on_drawer_touch_move}
                ontouchend={on_drawer_touch_end}
            >
                <div class="app-drawer-grid">
                    { for all_apps.iter().map(|app| {
                        let app_id = app.id.to_string();
                        let close_drawer = close_drawer.clone();
                        let on_click = {
                            let on_app_click = props.on_app_click.clone();
                            let app_id = app_id.clone();
                            Callback::from(move |_| {
                                on_app_click.emit(app_id.clone());
                                close_drawer.emit(());
                            })
                        };

                        html! {
                            <button class="app-icon" onclick={on_click}>
                                <div class="app-icon-bg">
                                    <i class={app.icon}></i>
                                </div>
                                <span class="app-icon-label">{app.label}</span>
                            </button>
                        }
                    })}
                </div>
            </div>
        </>
    }
}
