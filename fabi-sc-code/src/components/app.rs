use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use super::lock_screen::LockScreen;
use super::offline_screen::OfflineScreen;
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

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_state(|| AppState::Booting);

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

    let is_booting = *app_state == AppState::Booting;
    let show_lock_screen = *app_state == AppState::LockScreen;
    let show_desktop = *app_state == AppState::Desktop || *app_state == AppState::LockScreen;
    let is_offline = *app_state == AppState::Offline;

    // Pre-render everything during boot (hidden behind boot screen)
    // This way when boot completes, everything is already rendered and ready
    html! {
        <>
            <TopBar visible={show_desktop && !is_offline && !is_booting} on_disconnect={on_disconnect} />
            <Workspace visible={show_desktop && !is_offline && !is_booting} />
            <Taskbar visible={show_desktop && !is_offline && !is_booting} />
            <OfflineScreen visible={is_offline} />
            <LockScreen visible={show_lock_screen || is_booting} boot_complete={!is_booting} on_login={on_login} />
        </>
    }
}
