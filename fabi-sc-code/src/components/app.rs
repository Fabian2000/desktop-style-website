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
use super::taskbar::Taskbar;
use super::top_bar::TopBar;
use super::workspace::Workspace;
use crate::filesystem;

fn is_boot_complete() -> bool {
    js_sys::Reflect::get(&web_sys::window().unwrap(), &JsValue::from_str("__bootComplete"))
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

#[derive(Clone, PartialEq)]
enum AppState {
    Booting,
    LockScreen,
    Desktop,
    Offline,
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

    // Initialize filesystem and poll for boot complete signal
    {
        let app_state = app_state.clone();
        use_effect_with((), move |_| {
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

                        // Refresh the JavaScript VFS cache after Rust VFS init
                        // This ensures the JS bridge has all the directories that were just created
                        if let Some(window) = web_sys::window() {
                            if let Ok(vfs_sync) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__vfsSync")) {
                                if let Ok(refresh_fn) = js_sys::Reflect::get(&vfs_sync, &wasm_bindgen::JsValue::from_str("refresh")) {
                                    if let Some(func) = refresh_fn.dyn_ref::<js_sys::Function>() {
                                        // refresh() returns a Promise, we need to await it
                                        if let Ok(promise) = func.call0(&vfs_sync) {
                                            let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await;
                                            web_sys::console::log_1(&"[VFS] JavaScript cache refreshed".into());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("VFS initialization failed: {}", e).into());
                    }
                }

                // Poll every 100ms until boot animation is complete
                loop {
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

    // Open app callback - focus existing window or create new one
    let on_app_click = {
        let open_windows = open_windows.clone();
        let next_z_index = next_z_index.clone();
        let active_window = active_window.clone();
        let window_counter = window_counter.clone();
        Callback::from(move |app_id: String| {
            // Check if a window with this app_id already exists
            let existing_window = open_windows
                .iter()
                .find(|(_, w)| w.app_id == app_id)
                .map(|(id, _)| id.clone());

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

                // Determine app path based on app type
                // System apps: /home/.system/apps/{app_id}/
                // User apps: /home/apps/{app_id}/
                let app_path = if matches!(app_id.as_str(), "terminal" | "files" | "settings" | "help") {
                    format!("/home/.system/apps/{}/", app_id)
                } else {
                    format!("/home/apps/{}/", app_id)
                };

                // Load Python code asynchronously, then create window
                let open_windows_async = open_windows.clone();
                let active_window_async = active_window.clone();
                let window_id_async = window_id.clone();
                let app_id_async = app_id.clone();
                let app_path_async = app_path.clone();
                let z_async = z;

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
                                    // If icon is a file path (not FA class), make it relative to server
                                    let icon = if icon_raw.starts_with("fa-") || icon_raw.starts_with("fas ") || icon_raw.starts_with("far ") {
                                        icon_raw.to_string()
                                    } else {
                                        // Convert VFS path to server path for images
                                        // app_path_async is like "/home/.system/apps/terminal/"
                                        // We need "/resources/apps/terminal/icon.png"
                                        let app_name = app_id_async.clone();
                                        format!("/resources/apps/{}/{}", app_name, icon_raw)
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

    // Get active app_id for taskbar highlighting
    let active_app_id = active_window.as_ref().and_then(|wid| {
        open_windows.get(wid).map(|w| w.app_id.clone())
    });

    let is_booting = *app_state == AppState::Booting;
    let show_lock_screen = *app_state == AppState::LockScreen;
    let show_desktop = *app_state == AppState::Desktop || *app_state == AppState::LockScreen;
    let is_offline = *app_state == AppState::Offline;

    // Collect windows for rendering
    let windows_list: Vec<OpenWindow> = open_windows.values().cloned().collect();

    // Pre-render everything during boot (hidden behind boot screen)
    // This way when boot completes, everything is already rendered and ready
    html! {
        <>
            <TopBar visible={show_desktop && !is_offline && !is_booting} on_disconnect={on_disconnect} />
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
                        minimized={window.minimized}
                        on_close={on_close}
                        on_focus={on_focus}
                        on_minimize={on_minimize}
                        on_back={on_back}
                        on_show_recents={on_recents}
                    />
                }
            })}
            <Taskbar
                visible={show_desktop && !is_offline && !is_booting}
                active_app={active_app_id}
                open_apps={windows_list.iter().map(|w| w.app_id.clone()).collect::<Vec<_>>()}
                on_app_click={on_app_click}
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
            <LockScreen visible={show_lock_screen || is_booting} boot_complete={!is_booting} on_login={on_login} />
        </>
    }
}
