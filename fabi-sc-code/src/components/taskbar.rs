use base64::Engine;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MouseEvent, TouchEvent};
use yew::prelude::*;

use crate::database::{fetch_app_metadata, AppMetadata, PinnedApp, TaskbarDb};
use crate::filesystem;

/// App info for display - loaded from metadata.json
#[derive(Clone, PartialEq)]
pub struct AppDisplayInfo {
    pub path: String,       // VFS app path (e.g., "/home/.system/apps/terminal/")
    pub id: String,         // App ID from metadata
    pub icon: String,       // Data URL or FontAwesome class
    pub label: String,      // App name from metadata
}

impl AppDisplayInfo {
    /// Create from app path and loaded metadata
    /// If icon is a file path (not FA class), load from VFS and convert to data URL
    async fn from_metadata(path: &str, metadata: &AppMetadata) -> Self {
        let icon = if metadata.icon.starts_with("fa-") || metadata.icon.starts_with("fas ") || metadata.icon.starts_with("far ") {
            // FontAwesome icon - use as-is
            metadata.icon.clone()
        } else {
            // File icon - load from VFS and convert to data URL
            let icon_path = filesystem::path::join(path, &metadata.icon);
            match filesystem::vfs::read_file(&icon_path).await {
                Ok(bytes) => {
                    let mime = filesystem::path::mime_type(&icon_path)
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let base64_data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    format!("data:{};base64,{}", mime, base64_data)
                }
                Err(e) => {
                    web_sys::console::warn_1(&format!("[Taskbar] Could not load icon {}: {}", icon_path, e).into());
                    "fa-solid fa-cube".to_string()
                }
            }
        };
        Self {
            path: path.to_string(),
            id: metadata.id.clone(),
            icon,
            label: metadata.name.clone(),
        }
    }

    /// Check if icon is an image (data URL or path with extension)
    pub fn is_image_icon(&self) -> bool {
        // Data URLs or paths with file extensions
        self.icon.starts_with("data:") || self.icon.contains('.')
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PowerAction {
    Shutdown,
    Restart,
}

#[derive(Properties, PartialEq)]
pub struct TaskbarProps {
    pub visible: bool,
    #[prop_or_default]
    pub active_app: Option<String>,
    #[prop_or_default]
    pub open_apps: Vec<String>,  // All open app IDs (for running indicator)
    #[prop_or_default]
    pub on_app_click: Callback<(String, Vec<String>)>,  // Emits (app_id, args)
    #[prop_or_default]
    pub on_power_action: Callback<PowerAction>,  // Emits shutdown/restart actions
}

/// Mobile dock apps (first 3 pinned apps)
fn get_mobile_dock_apps(pinned: &[AppDisplayInfo]) -> Vec<AppDisplayInfo> {
    pinned.iter().take(3).cloned().collect()
}

/// Render an app icon - handles both image paths/data URLs and FontAwesome classes
fn render_icon(icon: &str, class: &str) -> Html {
    if icon.starts_with("data:") || icon.starts_with('/') || (icon.contains('.') && !icon.starts_with("fa")) {
        // Image icon (data URL, absolute path, or file with extension) - use fixed size with object-fit
        html! {
            <img
                class={class.to_string()}
                src={icon.to_string()}
                alt=""
                style="width: 24px; height: 24px; object-fit: contain;"
            />
        }
    } else {
        // FontAwesome icon
        html! {
            <i class={format!("{} {}", class, icon)}></i>
        }
    }
}

/// Render an app icon for start menu (larger)
fn render_menu_icon(icon: &str) -> Html {
    if icon.starts_with("data:") || icon.starts_with('/') || (icon.contains('.') && !icon.starts_with("fa")) {
        html! {
            <img
                src={icon.to_string()}
                alt=""
                style="width: 22px; height: 22px; object-fit: contain;"
            />
        }
    } else {
        html! {
            <i class={icon.to_string()}></i>
        }
    }
}

/// Context menu state
#[derive(Clone, PartialEq)]
struct ContextMenuState {
    visible: bool,
    x: i32,
    y: i32,
    app_path: String,  // App path for pin/unpin
    is_pinned: bool,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            app_path: String::new(),
            is_pinned: false,
        }
    }
}

#[function_component(Taskbar)]
pub fn taskbar(props: &TaskbarProps) -> Html {
    if !props.visible {
        return html! {};
    }

    // Pinned apps state (loaded from DB + metadata)
    let pinned_apps = use_state(Vec::<AppDisplayInfo>::new);
    let pinned_paths = use_state(Vec::<String>::new);

    // All available apps state (loaded from DB + metadata)
    let all_apps = use_state(Vec::<AppDisplayInfo>::new);

    // Load pinned apps on mount
    {
        let pinned_apps = pinned_apps.clone();
        let pinned_paths = pinned_paths.clone();
        let all_apps = all_apps.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    // Discover apps from VFS
                    let _ = db.discover_apps().await;

                    // Load pinned apps
                    let pinned = db.get_pinned().await;
                    let paths: Vec<String> = pinned.iter().map(|a| a.path.clone()).collect();
                    pinned_paths.set(paths.clone());

                    // Load metadata for each pinned app
                    let mut display_apps = Vec::new();
                    for app in &pinned {
                        match fetch_app_metadata(&app.path).await {
                            Ok(metadata) => {
                                display_apps.push(AppDisplayInfo::from_metadata(&app.path, &metadata).await);
                            }
                            Err(e) => {
                                web_sys::console::error_1(
                                    &format!("[Taskbar] Failed to load metadata for {}: {}", app.path, e).into()
                                );
                            }
                        }
                    }
                    pinned_apps.set(display_apps);

                    // Load all available apps
                    let available = db.get_all_apps().await;
                    let mut all_display = Vec::new();
                    for app in &available {
                        match fetch_app_metadata(&app.path).await {
                            Ok(metadata) => {
                                all_display.push(AppDisplayInfo::from_metadata(&app.path, &metadata).await);
                            }
                            Err(e) => {
                                web_sys::console::warn_1(
                                    &format!("[Taskbar] Skipping app {}: {}", app.path, e).into()
                                );
                            }
                        }
                    }
                    all_apps.set(all_display);
                }
            });
            || ()
        });
    }

    let mobile_dock_apps = get_mobile_dock_apps(&pinned_apps);

    // App drawer/start menu state
    let drawer_open = use_state(|| false);
    let drawer_dragging = use_state(|| false);
    let drawer_offset = use_state(|| 0.0f64);
    let touch_start_y = use_state(|| 0.0f64);

    // Search state
    let search_query = use_state(String::new);

    // Filter apps based on search query
    let filtered_apps: Vec<AppDisplayInfo> = {
        let query = search_query.to_lowercase();
        if query.is_empty() {
            (*all_apps).clone()
        } else {
            all_apps
                .iter()
                .filter(|app| {
                    app.label.to_lowercase().contains(&query)
                        || app.id.to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        }
    };

    // Context menu state
    let context_menu = use_state(ContextMenuState::default);

    // Power menu state (for shutdown/restart/sleep options)
    let power_menu_open = use_state(|| false);

    // Toggle start menu (desktop)
    let toggle_start_menu = {
        let drawer_open = drawer_open.clone();
        let context_menu = context_menu.clone();
        let search_query = search_query.clone();
        let power_menu_open = power_menu_open.clone();
        Callback::from(move |_| {
            context_menu.set(ContextMenuState::default());
            power_menu_open.set(false);
            let is_open = !*drawer_open;
            drawer_open.set(is_open);
            // Clear search when closing
            if !is_open {
                search_query.set(String::new());
            }
        })
    };

    // Toggle power menu
    let toggle_power_menu = {
        let power_menu_open = power_menu_open.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            power_menu_open.set(!*power_menu_open);
        })
    };

    // Power action handlers
    let on_shutdown = {
        let on_power_action = props.on_power_action.clone();
        let drawer_open = drawer_open.clone();
        let power_menu_open = power_menu_open.clone();
        Callback::from(move |_: MouseEvent| {
            drawer_open.set(false);
            power_menu_open.set(false);
            on_power_action.emit(PowerAction::Shutdown);
        })
    };

    let on_restart = {
        let on_power_action = props.on_power_action.clone();
        let drawer_open = drawer_open.clone();
        let power_menu_open = power_menu_open.clone();
        Callback::from(move |_: MouseEvent| {
            drawer_open.set(false);
            power_menu_open.set(false);
            on_power_action.emit(PowerAction::Restart);
        })
    };

    // Search input handler
    let on_search_input = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                search_query.set(input.value());
            }
        })
    };

    // Handle right-click on app in start menu
    let on_app_context_menu = {
        let context_menu = context_menu.clone();
        let pinned_paths = pinned_paths.clone();
        Callback::from(move |(e, app_path): (MouseEvent, String)| {
            e.prevent_default();
            let is_pinned = pinned_paths.contains(&app_path);
            context_menu.set(ContextMenuState {
                visible: true,
                x: e.client_x(),
                y: e.client_y(),
                app_path,
                is_pinned,
            });
        })
    };

    // Handle right-click on taskbar item (to unpin)
    let on_taskbar_context_menu = {
        let context_menu = context_menu.clone();
        Callback::from(move |(e, app_path): (MouseEvent, String)| {
            e.prevent_default();
            context_menu.set(ContextMenuState {
                visible: true,
                x: e.client_x(),
                y: e.client_y(),
                app_path,
                is_pinned: true, // Taskbar items are always pinned
            });
        })
    };

    // Pin app handler
    let on_pin_app = {
        let context_menu = context_menu.clone();
        let pinned_apps = pinned_apps.clone();
        let pinned_paths = pinned_paths.clone();
        Callback::from(move |_| {
            let app_path = context_menu.app_path.clone();
            let pinned_apps = pinned_apps.clone();
            let pinned_paths = pinned_paths.clone();
            let context_menu = context_menu.clone();

            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    if db.pin_app(&app_path).await.is_ok() {
                        // Reload pinned apps with metadata
                        let pinned = db.get_pinned().await;
                        let paths: Vec<String> = pinned.iter().map(|a| a.path.clone()).collect();
                        pinned_paths.set(paths);

                        let mut display_apps = Vec::new();
                        for app in &pinned {
                            if let Ok(metadata) = fetch_app_metadata(&app.path).await {
                                display_apps.push(AppDisplayInfo::from_metadata(&app.path, &metadata).await);
                            }
                        }
                        pinned_apps.set(display_apps);
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
        let pinned_paths = pinned_paths.clone();
        Callback::from(move |_| {
            let app_path = context_menu.app_path.clone();
            let pinned_apps = pinned_apps.clone();
            let pinned_paths = pinned_paths.clone();
            let context_menu = context_menu.clone();

            spawn_local(async move {
                if let Ok(db) = TaskbarDb::open().await {
                    if db.unpin_app(&app_path).await.is_ok() {
                        // Reload pinned apps with metadata
                        let pinned = db.get_pinned().await;
                        let paths: Vec<String> = pinned.iter().map(|a| a.path.clone()).collect();
                        pinned_paths.set(paths);

                        let mut display_apps = Vec::new();
                        for app in &pinned {
                            if let Ok(metadata) = fetch_app_metadata(&app.path).await {
                                display_apps.push(AppDisplayInfo::from_metadata(&app.path, &metadata).await);
                            }
                        }
                        pinned_apps.set(display_apps);
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
            // Don't prevent default - allow clicks to pass through
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
            if let Some(touch) = e.touches().get(0) {
                let current_y = touch.client_y() as f64;
                let delta = current_y - *touch_start_y;
                if delta > 20.0 {
                    // Only prevent default when actually dragging
                    e.prevent_default();
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
            // Only prevent default if we were dragging
            if *drawer_dragging {
                e.prevent_default();
            }
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
        let search_query = search_query.clone();
        Callback::from(move |_: ()| {
            drawer_open.set(false);
            context_menu.set(ContextMenuState::default());
            search_query.set(String::new());
        })
    };

    // Global click listener to close DESKTOP start-menu and context menu
    // Note: Mobile drawer is ONLY closed via swipe-down or app click, never by clicking
    {
        let drawer_open = drawer_open.clone();
        let context_menu = context_menu.clone();
        let search_query = search_query.clone();
        use_effect_with((*drawer_open, context_menu.visible), move |(is_open, ctx_visible)| {
            let document = web_sys::window().and_then(|w| w.document());

            let closure = if *is_open || *ctx_visible {
                let drawer_open = drawer_open.clone();
                let context_menu = context_menu.clone();
                let search_query = search_query.clone();
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

                                // Mobile drawer: NEVER close on click, only swipe-down
                                if let Ok(Some(app_drawer)) = doc.query_selector(".app-drawer") {
                                    if app_drawer.contains(Some(element)) {
                                        return;
                                    }
                                }
                                // Also ignore clicks on mobile dock
                                if let Ok(Some(mobile_dock)) = doc.query_selector(".mobile-dock") {
                                    if mobile_dock.contains(Some(element)) {
                                        return;
                                    }
                                }

                                // Desktop start-menu: close on outside click
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
                            // Click was outside desktop start-menu - close it
                            drawer_open.set(false);
                            search_query.set(String::new());
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
                    let app_path = app.path.clone();
                    let on_click = {
                        let on_app_click = props.on_app_click.clone();
                        let app_id = app_id.clone();
                        Callback::from(move |_| {
                            on_app_click.emit((app_id.clone(), vec![]));
                        })
                    };
                    let on_context = {
                        let on_taskbar_context_menu = on_taskbar_context_menu.clone();
                        let app_path = app_path.clone();
                        Callback::from(move |e: MouseEvent| {
                            on_taskbar_context_menu.emit((e, app_path.clone()));
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
                            { render_icon(&app.icon, "taskbar-icon") }
                        </button>
                    }
                })}
            </div>

            // Desktop: Custom App Launcher (Start Menu)
            <div
                class={if *drawer_open { "start-menu open" } else { "start-menu" }}
                onclick={{
                    let power_menu_open = power_menu_open.clone();
                    Callback::from(move |_| {
                        // Close power menu when clicking anywhere in start menu
                        if *power_menu_open {
                            power_menu_open.set(false);
                        }
                    })
                }}
            >
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
                    <input
                        type="text"
                        placeholder="Search apps..."
                        value={(*search_query).clone()}
                        oninput={on_search_input.clone()}
                    />
                </div>

                // Apps section
                <div class="start-menu-apps">
                    <div class="apps-label">{"Applications"}</div>
                    <div class="start-menu-grid">
                        { for filtered_apps.iter().map(|app| {
                            let app_id = app.id.clone();
                            let app_path = app.path.clone();
                            let close_drawer = close_drawer.clone();
                            let on_click = {
                                let on_app_click = props.on_app_click.clone();
                                let app_id = app_id.clone();
                                Callback::from(move |_| {
                                    on_app_click.emit((app_id.clone(), vec![]));
                                    close_drawer.emit(());
                                })
                            };
                            let on_context = {
                                let on_app_context_menu = on_app_context_menu.clone();
                                let app_path = app_path.clone();
                                Callback::from(move |e: MouseEvent| {
                                    on_app_context_menu.emit((e, app_path.clone()));
                                })
                            };
                            let is_pinned = pinned_paths.contains(&app.path);

                            html! {
                                <button
                                    class={if is_pinned { "start-menu-item pinned" } else { "start-menu-item" }}
                                    onclick={on_click}
                                    oncontextmenu={on_context}
                                >
                                    <div class="app-icon-wrapper">
                                        { render_menu_icon(&app.icon) }
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
                    <button class="quick-action" onclick={{
                        let on_app_click = props.on_app_click.clone();
                        let close_drawer = close_drawer.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(("files".to_string(), vec![]));
                            close_drawer.emit(());
                        })
                    }}>
                        <i class="fa-solid fa-folder"></i>
                        <span>{"Files"}</span>
                    </button>
                    <button class="quick-action" onclick={{
                        let on_app_click = props.on_app_click.clone();
                        let close_drawer = close_drawer.clone();
                        Callback::from(move |_| {
                            on_app_click.emit(("terminal".to_string(), vec![]));
                            close_drawer.emit(());
                        })
                    }}>
                        <i class="fa-solid fa-terminal"></i>
                        <span>{"Terminal"}</span>
                    </button>
                    <div class="power-menu-container">
                        <button class="quick-action power" onclick={toggle_power_menu.clone()}>
                            <i class="fa-solid fa-power-off"></i>
                        </button>
                        if *power_menu_open {
                            <div class="power-menu">
                                <button class="power-menu-item" onclick={on_restart.clone()}>
                                    <i class="fa-solid fa-rotate-right"></i>
                                    <span>{"Restart"}</span>
                                </button>
                                <button class="power-menu-item" onclick={on_shutdown.clone()}>
                                    <i class="fa-solid fa-power-off"></i>
                                    <span>{"Shut down"}</span>
                                </button>
                                <button class="power-menu-item disabled" disabled={true}>
                                    <i class="fa-solid fa-moon"></i>
                                    <span>{"Sleep"}</span>
                                </button>
                            </div>
                        }
                    </div>
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
                            on_app_click.emit((app_id.clone(), vec![]));
                        })
                    };

                    html! {
                        <button class="dock-item" onclick={on_click}>
                            { render_icon(&app.icon, "") }
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
                    <input
                        type="text"
                        placeholder="Search apps..."
                        value={(*search_query).clone()}
                        oninput={on_search_input.clone()}
                    />
                </div>

                <div class="app-drawer-grid">
                    { for filtered_apps.iter().map(|app| {
                        let app_id = app.id.clone();
                        let close_drawer = close_drawer.clone();
                        let on_click = {
                            let on_app_click = props.on_app_click.clone();
                            let app_id = app_id.clone();
                            Callback::from(move |_| {
                                on_app_click.emit((app_id.clone(), vec![]));
                                close_drawer.emit(());
                            })
                        };

                        html! {
                            <button class="app-icon" onclick={on_click}>
                                <div class="app-icon-bg">
                                    { render_menu_icon(&app.icon) }
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
