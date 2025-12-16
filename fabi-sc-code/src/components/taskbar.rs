use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MouseEvent, TouchEvent};
use yew::prelude::*;

use crate::database::{PinnedApp, TaskbarDb};

/// App info for display
#[derive(Clone, PartialEq)]
pub struct AppDisplayInfo {
    pub id: String,
    pub icon: String,  // FontAwesome class OR image path (starts with /)
    pub label: String,
}

impl AppDisplayInfo {
    fn new(id: &str, icon: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            icon: icon.to_string(),
            label: label.to_string(),
        }
    }

    /// Check if icon is an image path (starts with /)
    pub fn is_image_icon(&self) -> bool {
        self.icon.starts_with('/')
    }
}

#[derive(Properties, PartialEq)]
pub struct TaskbarProps {
    pub visible: bool,
    #[prop_or_default]
    pub active_app: Option<String>,
    #[prop_or_default]
    pub open_apps: Vec<String>,  // All open app IDs (for running indicator)
    #[prop_or_default]
    pub on_app_click: Callback<String>,
}

/// Get icon class or path for an app ID
fn get_app_icon(app_id: &str) -> &'static str {
    match app_id {
        "terminal" => "/resources/apps/terminal/icon.png",
        "files" => "fa-solid fa-folder",
        "browser" => "fa-solid fa-globe",
        "settings" => "fa-solid fa-gear",
        "gallery" => "fa-solid fa-images",
        "music" => "fa-solid fa-music",
        "contacts" => "fa-solid fa-address-book",
        "info" | "about" => "fa-solid fa-circle-info",
        _ => "fa-solid fa-cube",
    }
}

/// Get label for an app ID
fn get_app_label(app_id: &str) -> &'static str {
    match app_id {
        "terminal" => "Terminal",
        "files" => "Files",
        "browser" => "Browser",
        "settings" => "Settings",
        "gallery" => "Gallery",
        "music" => "Music",
        "contacts" => "Contacts",
        "info" | "about" => "About",
        _ => "App",
    }
}

/// All available apps for the start menu
fn get_all_apps() -> Vec<AppDisplayInfo> {
    vec![
        AppDisplayInfo::new("browser", "fa-solid fa-globe", "Browser"),
        AppDisplayInfo::new("files", "fa-solid fa-folder", "Files"),
        AppDisplayInfo::new("terminal", "/resources/apps/terminal/icon.png", "Terminal"),
        AppDisplayInfo::new("settings", "fa-solid fa-gear", "Settings"),
        AppDisplayInfo::new("info", "fa-solid fa-circle-info", "About"),
        AppDisplayInfo::new("gallery", "fa-solid fa-images", "Gallery"),
        AppDisplayInfo::new("music", "fa-solid fa-music", "Music"),
        AppDisplayInfo::new("contacts", "fa-solid fa-address-book", "Contacts"),
    ]
}

/// Convert PinnedApp to AppDisplayInfo
fn pinned_to_display(pinned: &[PinnedApp]) -> Vec<AppDisplayInfo> {
    pinned
        .iter()
        .map(|p| AppDisplayInfo {
            id: p.id.clone(),
            icon: get_app_icon(&p.id).to_string(),
            label: get_app_label(&p.id).to_string(),
        })
        .collect()
}

/// Mobile dock apps (first 3 pinned apps)
fn get_mobile_dock_apps(pinned: &[AppDisplayInfo]) -> Vec<AppDisplayInfo> {
    pinned.iter().take(3).cloned().collect()
}

/// Context menu state
#[derive(Clone, PartialEq)]
struct ContextMenuState {
    visible: bool,
    x: i32,
    y: i32,
    app_id: String,
    is_pinned: bool,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            app_id: String::new(),
            is_pinned: false,
        }
    }
}

#[function_component(Taskbar)]
pub fn taskbar(props: &TaskbarProps) -> Html {
    if !props.visible {
        return html! {};
    }

    // Pinned apps state (loaded from DB)
    let pinned_apps = use_state(Vec::<AppDisplayInfo>::new);
    let pinned_ids = use_state(Vec::<String>::new);

    // Load pinned apps on mount
    {
        let pinned_apps = pinned_apps.clone();
        let pinned_ids = pinned_ids.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    let apps = db.get_pinned().await;
                    let ids: Vec<String> = apps.iter().map(|a| a.id.clone()).collect();
                    pinned_ids.set(ids);
                    pinned_apps.set(pinned_to_display(&apps));
                }
            });
            || ()
        });
    }

    let all_apps = get_all_apps();
    let mobile_dock_apps = get_mobile_dock_apps(&pinned_apps);

    // App drawer/start menu state
    let drawer_open = use_state(|| false);
    let drawer_dragging = use_state(|| false);
    let drawer_offset = use_state(|| 0.0f64);
    let touch_start_y = use_state(|| 0.0f64);

    // Context menu state
    let context_menu = use_state(ContextMenuState::default);

    // Toggle start menu (desktop)
    let toggle_start_menu = {
        let drawer_open = drawer_open.clone();
        let context_menu = context_menu.clone();
        Callback::from(move |_| {
            context_menu.set(ContextMenuState::default());
            drawer_open.set(!*drawer_open);
        })
    };

    // Handle right-click on app in start menu
    let on_app_context_menu = {
        let context_menu = context_menu.clone();
        let pinned_ids = pinned_ids.clone();
        Callback::from(move |(e, app_id): (MouseEvent, String)| {
            e.prevent_default();
            let is_pinned = pinned_ids.contains(&app_id);
            context_menu.set(ContextMenuState {
                visible: true,
                x: e.client_x(),
                y: e.client_y(),
                app_id,
                is_pinned,
            });
        })
    };

    // Handle right-click on taskbar item (to unpin)
    let on_taskbar_context_menu = {
        let context_menu = context_menu.clone();
        Callback::from(move |(e, app_id): (MouseEvent, String)| {
            e.prevent_default();
            context_menu.set(ContextMenuState {
                visible: true,
                x: e.client_x(),
                y: e.client_y(),
                app_id,
                is_pinned: true, // Taskbar items are always pinned
            });
        })
    };

    // Pin app handler
    let on_pin_app = {
        let context_menu = context_menu.clone();
        let pinned_apps = pinned_apps.clone();
        let pinned_ids = pinned_ids.clone();
        Callback::from(move |_| {
            let app_id = context_menu.app_id.clone();
            let pinned_apps = pinned_apps.clone();
            let pinned_ids = pinned_ids.clone();
            let context_menu = context_menu.clone();

            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    if db.pin_app(&app_id).await.is_ok() {
                        let apps = db.get_pinned().await;
                        let ids: Vec<String> = apps.iter().map(|a| a.id.clone()).collect();
                        pinned_ids.set(ids);
                        pinned_apps.set(pinned_to_display(&apps));
                    }
                }
                context_menu.set(ContextMenuState::default());
            });
        })
    };

    // Unpin app handler
    let on_unpin_app = {
        let context_menu = context_menu.clone();
        let pinned_apps = pinned_apps.clone();
        let pinned_ids = pinned_ids.clone();
        Callback::from(move |_| {
            let app_id = context_menu.app_id.clone();
            let pinned_apps = pinned_apps.clone();
            let pinned_ids = pinned_ids.clone();
            let context_menu = context_menu.clone();

            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    if db.unpin_app(&app_id).await.is_ok() {
                        let apps = db.get_pinned().await;
                        let ids: Vec<String> = apps.iter().map(|a| a.id.clone()).collect();
                        pinned_ids.set(ids);
                        pinned_apps.set(pinned_to_display(&apps));
                    }
                }
                context_menu.set(ContextMenuState::default());
            });
        })
    };

    // Close context menu
    let close_context_menu = {
        let context_menu = context_menu.clone();
        Callback::from(move |_: MouseEvent| {
            context_menu.set(ContextMenuState::default());
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

    // Swipe down on drawer to close (mobile)
    let on_drawer_touch_start = {
        let touch_start_y = touch_start_y.clone();
        Callback::from(move |e: TouchEvent| {
            e.prevent_default();
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
            e.prevent_default();
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
        let context_menu = context_menu.clone();
        Callback::from(move |_: ()| {
            drawer_open.set(false);
            context_menu.set(ContextMenuState::default());
        })
    };

    // Global click listener to close menus when clicking outside
    {
        let drawer_open = drawer_open.clone();
        let context_menu = context_menu.clone();
        use_effect_with((*drawer_open, context_menu.visible), move |(is_open, ctx_visible)| {
            let document = web_sys::window().and_then(|w| w.document());

            let closure = if *is_open || *ctx_visible {
                let drawer_open = drawer_open.clone();
                let context_menu = context_menu.clone();
                Some(Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                    if let Some(target) = e.target() {
                        if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                // Check if click is inside context menu
                                if let Ok(Some(ctx_menu)) = doc.query_selector(".context-menu") {
                                    if ctx_menu.contains(Some(element)) {
                                        return;
                                    }
                                }
                                // Close context menu on outside click
                                context_menu.set(ContextMenuState::default());

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

    // Context menu style
    let context_menu_style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 10000;",
        context_menu.x, context_menu.y
    );

    html! {
        <>
            // Desktop: Floating dock with start button
            <div class="taskbar">
                // Start menu button with logo
                <button class="taskbar-item start-btn" onclick={toggle_start_menu.clone()}>
                    <img class="start-logo" src="resources/img/logo_inverted_bg.webp" alt="Start" />
                </button>
                <div class="taskbar-separator"></div>
                // Pinned apps from DB
                { for pinned_apps.iter().map(|app| {
                    let app_id = app.id.clone();
                    let app_id_ctx = app_id.clone();
                    let on_click = {
                        let on_app_click = props.on_app_click.clone();
                        let app_id = app_id.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(app_id.clone());
                        })
                    };
                    let on_context = {
                        let on_taskbar_context_menu = on_taskbar_context_menu.clone();
                        Callback::from(move |e: MouseEvent| {
                            on_taskbar_context_menu.emit((e, app_id_ctx.clone()));
                        })
                    };

                    let is_active = props.active_app.as_ref().map(|a| a == &app.id).unwrap_or(false);
                    let is_open = props.open_apps.contains(&app.id);
                    let class = match (is_active, is_open) {
                        (true, _) => "taskbar-item active",
                        (false, true) => "taskbar-item open",
                        (false, false) => "taskbar-item",
                    };

                    html! {
                        <button class={class} onclick={on_click} oncontextmenu={on_context}>
                            <i class={format!("taskbar-icon {}", app.icon)}></i>
                        </button>
                    }
                })}
            </div>

            // Desktop: Custom App Launcher (Start Menu)
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

                // Search bar
                <div class="start-menu-search">
                    <i class="fa-solid fa-magnifying-glass"></i>
                    <input type="text" placeholder="Search apps..." />
                </div>

                // Apps section
                <div class="start-menu-apps">
                    <div class="apps-label">{"Applications"}</div>
                    <div class="start-menu-grid">
                        { for all_apps.iter().map(|app| {
                            let app_id = app.id.clone();
                            let app_id_ctx = app_id.clone();
                            let close_drawer = close_drawer.clone();
                            let on_click = {
                                let on_app_click = props.on_app_click.clone();
                                let app_id = app_id.clone();
                                Callback::from(move |_| {
                                    on_app_click.emit(app_id.clone());
                                    close_drawer.emit(());
                                })
                            };
                            let on_context = {
                                let on_app_context_menu = on_app_context_menu.clone();
                                Callback::from(move |e: MouseEvent| {
                                    on_app_context_menu.emit((e, app_id_ctx.clone()));
                                })
                            };
                            let is_pinned = pinned_ids.contains(&app.id);

                            html! {
                                <button
                                    class={if is_pinned { "start-menu-item pinned" } else { "start-menu-item" }}
                                    onclick={on_click}
                                    oncontextmenu={on_context}
                                >
                                    <div class="app-icon-wrapper">
                                        <i class={app.icon.clone()}></i>
                                        if is_pinned {
                                            <span class="pinned-indicator" title="Angeheftet">
                                                <i class="fa-solid fa-thumbtack"></i>
                                            </span>
                                        }
                                    </div>
                                    <span>{&app.label}</span>
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

            // Context Menu (Pin/Unpin)
            if context_menu.visible {
                <div class="context-menu" style={context_menu_style} onclick={close_context_menu}>
                    if context_menu.is_pinned {
                        <button class="context-menu-item" onclick={on_unpin_app}>
                            <i class="fa-solid fa-thumbtack"></i>
                            <span>{"Von Taskbar lösen"}</span>
                        </button>
                    } else {
                        <button class="context-menu-item" onclick={on_pin_app}>
                            <i class="fa-solid fa-thumbtack"></i>
                            <span>{"An Taskbar anheften"}</span>
                        </button>
                    }
                </div>
            }

            // Mobile: Floating icons at bottom
            <div
                class="mobile-dock"
                ontouchstart={on_dock_touch_start}
                ontouchmove={on_dock_touch_move}
                ontouchend={on_dock_touch_end}
            >
                { for mobile_dock_apps.iter().map(|app| {
                    let app_id = app.id.clone();
                    let on_click = {
                        let on_app_click = props.on_app_click.clone();
                        let app_id = app_id.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(app_id.clone());
                        })
                    };

                    html! {
                        <button class="dock-item" onclick={on_click}>
                            <i class={app.icon.clone()}></i>
                        </button>
                    }
                })}
            </div>

            // Mobile: App Drawer
            <div
                class={drawer_class}
                style={drawer_style}
                ontouchstart={on_drawer_touch_start}
                ontouchmove={on_drawer_touch_move}
                ontouchend={on_drawer_touch_end}
            >
                // Search bar
                <div class="app-drawer-search">
                    <i class="fa-solid fa-magnifying-glass"></i>
                    <input type="text" placeholder="Search apps..." />
                </div>

                <div class="app-drawer-grid">
                    { for all_apps.iter().map(|app| {
                        let app_id = app.id.clone();
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
                                    <i class={app.icon.clone()}></i>
                                </div>
                                <span class="app-icon-label">{&app.label}</span>
                            </button>
                        }
                    })}
                </div>
            </div>
        </>
    }
}
