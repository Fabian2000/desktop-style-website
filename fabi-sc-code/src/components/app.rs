use base64::Engine;
use gloo_timers::future::TimeoutFuture;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use super::app_window::AppWindow;
use super::lock_screen::LockScreen;
use super::offline_screen::OfflineScreen;
use super::recents_view::{RecentsAppInfo, RecentsView};
use super::shutdown_screen::ShutdownScreen;
use super::taskbar::{PowerAction, Taskbar};
use super::top_bar::TopBar;
use super::workspace::Workspace;
use crate::filesystem;
use crate::session;

fn is_boot_complete() -> bool {
    js_sys::Reflect::get(&web_sys::window().unwrap(), &JsValue::from_str("__bootComplete"))
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Info about an app that can handle a file type
#[derive(Clone, PartialEq)]
struct FileHandlerApp {
    app_id: String,
    app_name: String,
    app_icon: String,  // FontAwesome class or data URL
    app_path: String,
}

/// State for the "Open with" dialog
#[derive(Clone, PartialEq)]
struct OpenFileDialogState {
    file_path: String,
    extension: String,
    available_apps: Vec<FileHandlerApp>,
    loading: bool,
}

#[derive(Clone, PartialEq)]
enum AppState {
    Booting,
    LockScreen,
    Desktop,
    Offline,
    ShuttingDown,
    Restarting,
}

/// Open window state
#[derive(Clone, PartialEq)]
pub struct OpenWindow {
    pub id: String,
    pub app_id: String,
    pub title: String,
    /// Icon class (FontAwesome) or image path
    pub icon: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub z_index: u32,
    pub minimized: bool,
    /// Python code to execute (loaded from VFS)
    pub python_code: Option<String>,
    /// App's base path in VFS
    pub app_path: String,
    /// Command line arguments passed to the app
    pub args: Vec<String>,
}

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_state(|| AppState::Booting);

    // Window ID counter - use Rc<RefCell> for interior mutability
    let window_counter = use_memo((), |_| Rc::new(RefCell::new(0u32)));

    // Open windows state
    let open_windows = use_state(HashMap::<String, OpenWindow>::new);
    let next_z_index = use_state(|| 100u32);
    let active_window = use_state(|| Option::<String>::None);
    let show_recents = use_state(|| false);

    // Initialize filesystem, session management, and poll for boot complete signal
    {
        let app_state = app_state.clone();
        let open_windows = open_windows.clone();
        use_effect_with((), move |_| {
            // Initialize session management (broadcasts takeover to other tabs)
            session::init_session();

            spawn_local(async move {
                // Initialize the virtual filesystem during boot
                match filesystem::initialize().await {
                    Ok(result) => {
                        if result.created_structure {
                            web_sys::console::log_1(&"VFS: Created initial directory structure".into());
                        }
                        if result.files_updated > 0 || result.files_added > 0 {
                            web_sys::console::log_1(
                                &format!(
                                    "VFS: Synced {} files ({} updated, {} added)",
                                    result.files_updated + result.files_added,
                                    result.files_updated,
                                    result.files_added
                                )
                                .into(),
                            );
                        }
                        if result.trash_cleaned > 0 {
                            web_sys::console::log_1(
                                &format!("VFS: Cleaned {} old trash files", result.trash_cleaned).into(),
                            );
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("VFS initialization failed: {}", e).into());
                    }
                }

                // Poll every 100ms until boot animation is complete
                // Also check for session takeover
                loop {
                    // Check if another tab has taken over this session
                    if session::check_and_clear_takeover() {
                        // Kill all processes (clear windows)
                        open_windows.set(HashMap::new());
                        // Show disconnect screen
                        app_state.set(AppState::Offline);
                        break;
                    }

                    if is_boot_complete() {
                        app_state.set(AppState::LockScreen);
                        break;
                    }
                    TimeoutFuture::new(100).await;
                }
            });
            || ()
        });
    }

    // Poll for session takeover after boot (when desktop is running)
    {
        let app_state = app_state.clone();
        let open_windows = open_windows.clone();
        let current_state = (*app_state).clone();
        use_effect_with(current_state, move |state| {
            // Only poll when in Desktop or LockScreen state
            if *state == AppState::Desktop || *state == AppState::LockScreen {
                let app_state = app_state.clone();
                let open_windows = open_windows.clone();
                spawn_local(async move {
                    loop {
                        // Check if another tab has taken over this session
                        if session::check_and_clear_takeover() {
                            web_sys::console::log_1(&"[App] Session takeover detected - disconnecting".into());
                            // Kill all processes (clear windows)
                            open_windows.set(HashMap::new());
                            // Show disconnect screen
                            app_state.set(AppState::Offline);
                            break;
                        }
                        TimeoutFuture::new(500).await;
                    }
                });
            }
            || ()
        });
    }

    // Prevent context menu globally
    {
        use_effect_with((), |_| {
            let document = web_sys::window().and_then(|w| w.document());
            let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
                e.prevent_default();
            }) as Box<dyn FnMut(_)>);

            if let Some(doc) = &document {
                let _ = doc
                    .add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref());
            }

            move || {
                if let Some(doc) = document {
                    let _ = doc.remove_event_listener_with_callback(
                        "contextmenu",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    let on_login = {
        let app_state = app_state.clone();
        Callback::from(move |_| {
            app_state.set(AppState::Desktop);
        })
    };

    let on_disconnect = {
        let app_state = app_state.clone();
        Callback::from(move |_| {
            app_state.set(AppState::Offline);
        })
    };

    // Power action callback (shutdown/restart)
    let on_power_action = {
        let app_state = app_state.clone();
        let open_windows = open_windows.clone();
        Callback::from(move |action: PowerAction| {
            // Clear all windows (kill all Python processes effectively)
            open_windows.set(HashMap::new());

            match action {
                PowerAction::Shutdown => {
                    app_state.set(AppState::ShuttingDown);
                }
                PowerAction::Restart => {
                    app_state.set(AppState::Restarting);
                }
            }
        })
    };

    // Open app callback - focus existing window or create new one
    let on_app_click = {
        let open_windows = open_windows.clone();
        let next_z_index = next_z_index.clone();
        let active_window = active_window.clone();
        let window_counter = window_counter.clone();
        Callback::from(move |(app_id, args): (String, Vec<String>)| {
            // If args are provided (e.g., opening a file), always create a new window
            // Otherwise, check if a window with this app_id already exists
            let existing_window = if args.is_empty() {
                open_windows
                    .iter()
                    .find(|(_, w)| w.app_id == app_id)
                    .map(|(id, _)| id.clone())
            } else {
                None // Always create new window when opening a file
            };

            if let Some(window_id) = existing_window {
                // Focus and restore existing window
                let z = *next_z_index;
                next_z_index.set(z + 1);

                let mut windows = (*open_windows).clone();
                if let Some(window) = windows.get_mut(&window_id) {
                    window.z_index = z;
                    window.minimized = false; // Restore if minimized
                }
                open_windows.set(windows);
                active_window.set(Some(window_id));
            } else {
                // Create new window
                let count = {
                    let mut c = window_counter.borrow_mut();
                    *c += 1;
                    *c
                };
                let window_id = format!("window-{}", count);
                let z = *next_z_index;
                next_z_index.set(z + 1);

                // Try system apps first, then user apps
                // System apps: /home/.system/apps/{app_id}/
                // User apps: /home/apps/{app_id}/
                let app_path = format!("/home/.system/apps/{}/", app_id);

                // Load Python code asynchronously, then create window
                let open_windows_async = open_windows.clone();
                let active_window_async = active_window.clone();
                let window_id_async = window_id.clone();
                let app_id_async = app_id.clone();
                let app_path_async = app_path.clone();
                let z_async = z;
                let args_async = args.clone();

                spawn_local(async move {
                    // Try to read metadata.json from app directory
                    let metadata_path = format!("{}metadata.json", app_path_async);
                    web_sys::console::log_1(&format!("[App] Loading metadata from: {}", metadata_path).into());
                    let (app_title, app_icon, app_width, app_height) = match filesystem::vfs::read_to_string(&metadata_path).await {
                        Ok(json) => {
                            web_sys::console::log_1(&format!("[App] Metadata loaded: {}", json).into());
                            match serde_json::from_str::<serde_json::Value>(&json) {
                                Ok(meta) => {
                                    let title = meta.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&app_id_async)
                                        .to_string();
                                    let icon_raw = meta.get("icon")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("fa-solid fa-cube");
                                    // If icon is a file path (not FA class), load from VFS and convert to data URL
                                    let icon = if icon_raw.starts_with("fa-") || icon_raw.starts_with("fas ") || icon_raw.starts_with("far ") {
                                        icon_raw.to_string()
                                    } else {
                                        // Build VFS path to icon (relative to app directory)
                                        let icon_path = filesystem::path::join(&app_path_async, icon_raw);
                                        // Try to load icon from VFS and convert to data URL
                                        match filesystem::vfs::read_file(&icon_path).await {
                                            Ok(bytes) => {
                                                let mime = filesystem::path::mime_type(&icon_path)
                                                    .unwrap_or_else(|| "application/octet-stream".to_string());
                                                let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                                format!("data:{};base64,{}", mime, base64)
                                            }
                                            Err(e) => {
                                                web_sys::console::warn_1(&format!("[App] Could not load icon {}: {}", icon_path, e).into());
                                                "fa-solid fa-cube".to_string()
                                            }
                                        }
                                    };
                                    let width = meta.get("window")
                                        .and_then(|w| w.get("width"))
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(600) as u32;
                                    let height = meta.get("window")
                                        .and_then(|w| w.get("height"))
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(400) as u32;
                                    web_sys::console::log_1(&format!("[App] Parsed: title={}, icon={}, {}x{}", title, icon, width, height).into());
                                    (title, icon, width, height)
                                }
                                Err(e) => {
                                    web_sys::console::error_1(&format!("[App] JSON parse error: {}", e).into());
                                    (app_id_async.clone(), "fa-solid fa-cube".to_string(), 600, 400)
                                }
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("[App] Could not load metadata {}: {}", metadata_path, e).into());
                            (app_id_async.clone(), "fa-solid fa-cube".to_string(), 600, 400)
                        }
                    };

                    // Try to read main.py from app directory
                    let main_py_path = format!("{}main.py", app_path_async);
                    web_sys::console::log_1(&format!("[App] Loading Python code from: {}", main_py_path).into());

                    let python_code = match filesystem::vfs::read_to_string(&main_py_path).await {
                        Ok(code) => {
                            web_sys::console::log_1(&format!("[App] Loaded {} bytes of Python code", code.len()).into());
                            Some(code)
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("[App] Could not load {}: {}", main_py_path, e).into()
                            );
                            None
                        }
                    };

                    // Create window with code and metadata loaded
                    let window = OpenWindow {
                        id: window_id_async.clone(),
                        app_id: app_id_async.clone(),
                        title: app_title,
                        icon: app_icon,
                        x: 100 + (z_async as i32 % 10) * 30,
                        y: 50 + (z_async as i32 % 10) * 30,
                        width: app_width,
                        height: app_height,
                        z_index: z_async,
                        minimized: false,
                        python_code,
                        app_path: app_path_async,
                        args: args_async,
                    };

                    let mut windows = (*open_windows_async).clone();
                    windows.insert(window_id_async.clone(), window);
                    open_windows_async.set(windows);
                    active_window_async.set(Some(window_id_async));
                });
            }
        })
    };

    // Close window callback
    let on_window_close = {
        let open_windows = open_windows.clone();
        let active_window = active_window.clone();
        Callback::from(move |window_id: String| {
            let mut windows = (*open_windows).clone();
            windows.remove(&window_id);
            open_windows.set(windows);
            if active_window.as_ref() == Some(&window_id) {
                active_window.set(None);
            }
        })
    };

    // Focus window callback
    let on_window_focus = {
        let open_windows = open_windows.clone();
        let next_z_index = next_z_index.clone();
        let active_window = active_window.clone();
        Callback::from(move |window_id: String| {
            let z = *next_z_index;
            next_z_index.set(z + 1);

            let mut windows = (*open_windows).clone();
            if let Some(window) = windows.get_mut(&window_id) {
                window.z_index = z;
                window.minimized = false; // Restore if minimized
            }
            open_windows.set(windows);
            active_window.set(Some(window_id));
        })
    };

    // Minimize window callback
    let on_window_minimize = {
        let open_windows = open_windows.clone();
        let active_window = active_window.clone();
        Callback::from(move |window_id: String| {
            let mut windows = (*open_windows).clone();
            if let Some(window) = windows.get_mut(&window_id) {
                window.minimized = true;
            }
            open_windows.set(windows);
            // Clear active window when minimized
            if active_window.as_ref() == Some(&window_id) {
                active_window.set(None);
            }
        })
    };

    // Show recents callback
    let on_show_recents = {
        let show_recents = show_recents.clone();
        Callback::from(move |_| {
            show_recents.set(true);
        })
    };

    // Recents: select app (restore/focus)
    let on_recents_select = {
        let on_window_focus = on_window_focus.clone();
        let show_recents = show_recents.clone();
        Callback::from(move |window_id: String| {
            on_window_focus.emit(window_id);
            show_recents.set(false);
        })
    };

    // Recents: close app
    let on_recents_close = {
        let on_window_close = on_window_close.clone();
        Callback::from(move |window_id: String| {
            on_window_close.emit(window_id);
        })
    };

    // Recents: dismiss (close recents view)
    let on_recents_dismiss = {
        let show_recents = show_recents.clone();
        Callback::from(move |_| {
            show_recents.set(false);
        })
    };

    // Launch app callback - called when Python app requests to open another app
    // (app_id, optional file_path)
    let on_launch_app: Callback<(String, Option<String>)> = {
        let on_app_click = on_app_click.clone();
        Callback::from(move |(app_id, file_path): (String, Option<String>)| {
            web_sys::console::log_1(&format!("[App] Launch request: {} with file: {:?}", app_id, file_path).into());
            // Pass file_path as first argument if present
            let args = match file_path {
                Some(path) => vec![path],
                None => vec![],
            };
            on_app_click.emit((app_id, args));
        })
    };

    // State for file open dialog
    let open_file_dialog = use_state(|| None::<OpenFileDialogState>);

    // Open file callback - called when Python app requests to open a file with system handler
    let on_open_file: Callback<String> = {
        let open_file_dialog = open_file_dialog.clone();
        Callback::from(move |file_path: String| {
            web_sys::console::log_1(&format!("[System] Open file request: {}", file_path).into());
            // Get file extension
            let ext = file_path.rsplit('.').next().unwrap_or("").to_lowercase();
            // Store the request and trigger app scanning
            open_file_dialog.set(Some(OpenFileDialogState {
                file_path,
                extension: ext,
                available_apps: Vec::new(),
                loading: true,
            }));
        })
    };

    // Effect to scan apps when open_file_dialog is set with loading=true
    {
        let open_file_dialog = open_file_dialog.clone();
        let dialog_state = (*open_file_dialog).clone();
        use_effect_with(dialog_state.clone(), move |state| {
            if let Some(state) = state {
                if state.loading {
                    let ext = state.extension.clone();
                    let file_path = state.file_path.clone();
                    let dialog = open_file_dialog.clone();
                    spawn_local(async move {
                        // Scan all apps in /home/.system/apps/ for file_handlers
                        let apps_dir = "/home/.system/apps/";
                        let mut handlers: Vec<FileHandlerApp> = Vec::new();

                        if let Ok(entries) = filesystem::vfs::read_dir(apps_dir).await {
                            for entry in entries {
                                if entry.is_dir() {
                                    let app_path = format!("{}{}/", apps_dir, entry.name);
                                    let metadata_path = format!("{}metadata.json", app_path);

                                    if let Ok(json) = filesystem::vfs::read_to_string(&metadata_path).await {
                                        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&json) {
                                            // Check if this app handles the file extension
                                            if let Some(file_handlers) = meta.get("file_handlers").and_then(|v| v.as_array()) {
                                                for handler in file_handlers {
                                                    if let Some(extensions) = handler.get("extensions").and_then(|v| v.as_array()) {
                                                        let handles_ext = extensions.iter().any(|e| {
                                                            e.as_str().map(|s| s.to_lowercase() == ext).unwrap_or(false)
                                                        });
                                                        if handles_ext {
                                                            let app_id = meta.get("id").and_then(|v| v.as_str()).unwrap_or(&entry.name).to_string();
                                                            let app_name = meta.get("name").and_then(|v| v.as_str()).unwrap_or(&entry.name).to_string();
                                                            let icon_raw = meta.get("icon").and_then(|v| v.as_str()).unwrap_or("fa-solid fa-cube");

                                                            // Load icon if it's a file path
                                                            let app_icon = if icon_raw.starts_with("fa-") || icon_raw.starts_with("fas ") || icon_raw.starts_with("far ") {
                                                                icon_raw.to_string()
                                                            } else {
                                                                let icon_path = filesystem::path::join(&app_path, icon_raw);
                                                                match filesystem::vfs::read_file(&icon_path).await {
                                                                    Ok(bytes) => {
                                                                        let mime = filesystem::path::mime_type(&icon_path).unwrap_or_else(|| "application/octet-stream".to_string());
                                                                        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                                                                        format!("data:{};base64,{}", mime, b64)
                                                                    }
                                                                    Err(_) => "fa-solid fa-cube".to_string(),
                                                                }
                                                            };

                                                            handlers.push(FileHandlerApp {
                                                                app_id,
                                                                app_name,
                                                                app_icon,
                                                                app_path: app_path.clone(),
                                                            });
                                                            break; // Only add app once even if multiple handlers match
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        web_sys::console::log_1(&format!("[System] Found {} apps for extension '{}'", handlers.len(), ext).into());

                        // Update state with found handlers
                        dialog.set(Some(OpenFileDialogState {
                            file_path,
                            extension: ext,
                            available_apps: handlers,
                            loading: false,
                        }));
                    });
                }
            }
            || ()
        });
    }

    // Get active app_id for taskbar highlighting
    let active_app_id = active_window.as_ref().and_then(|wid| {
        open_windows.get(wid).map(|w| w.app_id.clone())
    });

    let is_booting = *app_state == AppState::Booting;
    let show_lock_screen = *app_state == AppState::LockScreen;
    let show_desktop = *app_state == AppState::Desktop || *app_state == AppState::LockScreen;
    let is_offline = *app_state == AppState::Offline;
    let is_shutting_down = *app_state == AppState::ShuttingDown;
    let is_restarting = *app_state == AppState::Restarting;

    // Collect windows for rendering
    let windows_list: Vec<OpenWindow> = open_windows.values().cloned().collect();

    // Pre-render everything during boot (hidden behind boot screen)
    // This way when boot completes, everything is already rendered and ready
    html! {
        <>
            <TopBar visible={show_desktop && !is_offline && !is_booting} on_disconnect={on_disconnect.clone()} />
            <Workspace visible={show_desktop && !is_offline && !is_booting} />
            // Windows rendered OUTSIDE workspace so they have their own stacking context
            // This allows mobile windows to appear above the mobile-dock
            // Note: We render ALL windows (including minimized) to preserve their state
            // Minimized windows use CSS display:none in app_window.rs
            { for windows_list.iter().map(|window| {
                let on_close = {
                    let on_window_close = on_window_close.clone();
                    let window_id = window.id.clone();
                    Callback::from(move |_| on_window_close.emit(window_id.clone()))
                };
                let on_focus = {
                    let on_window_focus = on_window_focus.clone();
                    let window_id = window.id.clone();
                    Callback::from(move |_| on_window_focus.emit(window_id.clone()))
                };
                let on_minimize = {
                    let on_window_minimize = on_window_minimize.clone();
                    let window_id = window.id.clone();
                    Callback::from(move |_| on_window_minimize.emit(window_id.clone()))
                };
                // Back button: for apps without internal navigation, close the app
                // In future, Python apps can handle this for internal navigation
                let on_back = {
                    let on_window_close = on_window_close.clone();
                    let window_id = window.id.clone();
                    Callback::from(move |_| on_window_close.emit(window_id.clone()))
                };
                let on_recents = on_show_recents.clone();
                let on_launch = on_launch_app.clone();
                let on_file_open = on_open_file.clone();
                html! {
                    <AppWindow
                        key={window.id.clone()}
                        window_id={window.id.clone()}
                        app_id={window.app_id.clone()}
                        title={window.title.clone()}
                        icon={window.icon.clone()}
                        x={window.x}
                        y={window.y}
                        width={window.width}
                        height={window.height}
                        z_index={window.z_index}
                        python_code={window.python_code.clone()}
                        app_path={window.app_path.clone()}
                        args={window.args.clone()}
                        minimized={window.minimized}
                        on_close={on_close}
                        on_focus={on_focus}
                        on_minimize={on_minimize}
                        on_back={on_back}
                        on_show_recents={on_recents}
                        on_launch_app={on_launch}
                        on_open_file={on_file_open}
                    />
                }
            })}
            <Taskbar
                visible={show_desktop && !is_offline && !is_booting && !is_shutting_down && !is_restarting}
                active_app={active_app_id}
                open_apps={windows_list.iter().map(|w| w.app_id.clone()).collect::<Vec<_>>()}
                on_app_click={on_app_click}
                on_power_action={on_power_action}
            />
            // Recents/App Switcher View
            <RecentsView
                visible={*show_recents}
                apps={windows_list.iter().map(|w| RecentsAppInfo {
                    window_id: w.id.clone(),
                    app_id: w.app_id.clone(),
                    title: w.title.clone(),
                }).collect::<Vec<_>>()}
                on_select={on_recents_select}
                on_close={on_recents_close}
                on_dismiss={on_recents_dismiss}
            />
            <OfflineScreen visible={is_offline} />
            <ShutdownScreen
                visible={is_shutting_down || is_restarting}
                is_restart={is_restarting}
                on_shutdown_complete={on_disconnect.clone()}
            />
            <LockScreen visible={show_lock_screen || is_booting} boot_complete={!is_booting} on_login={on_login} />

            // "Open with" system dialog (very high z-index)
            { render_open_with_dialog(&open_file_dialog, &on_launch_app) }
        </>
    }
}

/// Render the "Open with" dialog
fn render_open_with_dialog(
    open_file_dialog: &UseStateHandle<Option<OpenFileDialogState>>,
    on_launch_app: &Callback<(String, Option<String>)>,
) -> Html {
    let Some(dialog) = (**open_file_dialog).clone() else {
        return html! {};
    };

    let file_name = dialog.file_path.rsplit('/').next().unwrap_or(&dialog.file_path).to_string();

    let on_dismiss = {
        let open_file_dialog = open_file_dialog.clone();
        Callback::from(move |_: web_sys::MouseEvent| {
            open_file_dialog.set(None);
        })
    };

    let on_backdrop_click = {
        let open_file_dialog = open_file_dialog.clone();
        Callback::from(move |e: web_sys::MouseEvent| {
            // Only close if clicking directly on backdrop
            if let Some(target) = e.target() {
                if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                    if element.class_list().contains("system-dialog-backdrop") {
                        open_file_dialog.set(None);
                    }
                }
            }
        })
    };

    let content = if dialog.loading {
        html! {
            <div style="text-align: center; padding: 20px; color: #888;">
                <i class="fa-solid fa-spinner fa-spin" style="font-size: 24px;"></i>
                <div style="margin-top: 8px;">{"Searching apps..."}</div>
            </div>
        }
    } else if dialog.available_apps.is_empty() {
        html! {
            <div style="text-align: center; padding: 20px; color: #888;">
                <i class="fa-solid fa-circle-question" style="font-size: 24px; margin-bottom: 8px;"></i>
                <div>{format!("No app found for .{} files", dialog.extension)}</div>
            </div>
        }
    } else {
        let apps_html: Vec<Html> = dialog.available_apps.iter().map(|app| {
            let app_id = app.app_id.clone();
            let file_path = dialog.file_path.clone();
            let on_launch_app = on_launch_app.clone();
            let open_file_dialog = open_file_dialog.clone();
            let on_click = Callback::from(move |_: web_sys::MouseEvent| {
                on_launch_app.emit((app_id.clone(), Some(file_path.clone())));
                open_file_dialog.set(None);
            });
            let is_fa_icon = app.app_icon.starts_with("fa-") || app.app_icon.starts_with("fas ") || app.app_icon.starts_with("far ");
            let icon_html = if is_fa_icon {
                html! { <i class={app.app_icon.clone()} style="font-size: 24px; color: #4a9eff; width: 32px; text-align: center;"></i> }
            } else {
                html! { <img src={app.app_icon.clone()} alt="" style="width: 32px; height: 32px; border-radius: 6px;" /> }
            };
            html! {
                <button
                    class="open-with-app-btn"
                    style="display: flex; align-items: center; gap: 12px; padding: 12px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; cursor: pointer; text-align: left; width: 100%; transition: background 0.2s; color: #fff; font-size: 14px;"
                    onclick={on_click}
                >
                    {icon_html}
                    <span>{&app.app_name}</span>
                </button>
            }
        }).collect();

        html! {
            <div style="display: flex; flex-direction: column; gap: 8px;">
                { for apps_html }
            </div>
        }
    };

    html! {
        <div class="system-dialog-backdrop" style="position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 10000; display: flex; align-items: center; justify-content: center;" onclick={on_backdrop_click}>
            <div class="system-dialog open-with-dialog" style="background: #1a1a2e; border-radius: 12px; padding: 20px; min-width: 320px; max-width: 400px; box-shadow: 0 8px 32px rgba(0,0,0,0.4); border: 1px solid rgba(255,255,255,0.1);">
                <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px;">
                    <i class="fa-solid fa-folder-open" style="font-size: 24px; color: #4a9eff;"></i>
                    <div>
                        <div style="font-size: 16px; font-weight: 600; color: #fff;">{"Open with"}</div>
                        <div style="font-size: 12px; color: #888; margin-top: 2px;">{file_name}</div>
                    </div>
                </div>

                {content}

                <div style="margin-top: 16px; display: flex; justify-content: flex-end;">
                    <button
                        style="padding: 8px 16px; background: rgba(255,255,255,0.1); border: none; border-radius: 6px; color: #fff; cursor: pointer;"
                        onclick={on_dismiss}
                    >
                        {"Cancel"}
                    </button>
                </div>
            </div>
        </div>
    }
}
