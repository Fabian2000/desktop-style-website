//! App Window Component
//!
//! A draggable, resizable window that can host app content.
//! Desktop: Traditional windowed mode with drag/resize
//! Mobile: Fullscreen with navigation bar (back button)

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, KeyboardEvent, HtmlInputElement};
use yew::prelude::*;

use crate::python::runtime::{PythonRuntime, AppExecResult};

/// Window state for minimize/maximize
#[derive(Clone, Copy, PartialEq, Debug)]
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
    /// Python code to execute (loaded from VFS)
    #[prop_or_default]
    pub python_code: Option<String>,
    /// App's base path for VFS operations
    #[prop_or_default]
    pub app_path: String,
    /// Whether window is minimized (controlled by parent)
    #[prop_or(false)]
    pub minimized: bool,
    #[prop_or_default]
    pub on_close: Callback<()>,
    #[prop_or_default]
    pub on_focus: Callback<()>,
    #[prop_or_default]
    pub on_minimize: Callback<()>,
    #[prop_or_default]
    pub on_back: Callback<()>,  // Back button pressed (for Python apps)
    #[prop_or_default]
    pub on_show_recents: Callback<()>,  // Show recents/app switcher
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

    // Python Runtime state
    let app_content = use_state(|| None::<String>);
    let app_error = use_state(|| None::<String>);
    let runtime_title = use_state(|| None::<String>);

    // Store the input value for re-execution (when Enter is pressed)
    // Using use_ref to avoid triggering re-renders when clearing
    let pending_input = use_mut_ref(|| None::<String>);

    // Store whether back button was pressed (for Python on_back handler)
    let pending_back = use_mut_ref(|| false);

    // Trigger for re-running the app (incremented on input submit or back press)
    // Using use_state for the actual trigger, plus a Cell to track the count
    // in closures that don't have access to current state
    let run_counter = use_state(|| 0u32);
    let run_counter_cell = use_mut_ref(|| 0u32);

    // Helper function to run Python code with optional input or back event
    // Returns true if Python requested window close
    fn run_python_app(
        code: &str,
        input: Option<&str>,
        back_pressed: bool,
        window_id: &str,
        app_id: &str,
        app_path: &str,
        app_content: &UseStateHandle<Option<String>>,
        app_error: &UseStateHandle<Option<String>>,
        runtime_title: &UseStateHandle<Option<String>>,
    ) -> bool {
        web_sys::console::log_1(&format!("[Python] Running app: {} ({})", app_id, window_id).into());

        // Create Python runtime for this app
        let runtime = PythonRuntime::new(
            window_id.to_string(),
            app_id.to_string(),
            app_path.to_string(),
        );

        // Prepare code with input/back injection if needed
        let full_code = if let Some(input_val) = input {
            // Inject __input__ variable and call on_input if defined
            web_sys::console::log_1(&format!("[Python] Injecting input: {}", input_val).into());
            format!(
                r#"__input__ = "{}"
{}
if '__input__' in dir() and __input__ and 'on_input' in dir():
    print("[Python] Calling on_input with: " + __input__)
    on_input(__input__)
"#,
                input_val.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
                code
            )
        } else if back_pressed {
            // Inject __back_pressed__ and call on_back if defined
            web_sys::console::log_1(&"[Python] Injecting back_pressed event".into());
            format!(
                r#"__back_pressed__ = True
{}
if '__back_pressed__' in dir() and __back_pressed__:
    if 'on_back' in dir():
        print("[Python] on_back() found, calling it now...")
        on_back()
        print("[Python] on_back() returned")
    else:
        print("[Python] WARNING: on_back not defined!")
"#,
                code
            )
        } else {
            code.to_string()
        };

        // Execute the code
        match runtime.run(&full_code) {
            AppExecResult::Success => {
                web_sys::console::log_1(&"[Python] Execution successful".into());
                // Get the UI HTML from the runtime
                if let Some(html) = runtime.take_pending_ui() {
                    web_sys::console::log_1(&format!("[Python] UI HTML: {} chars", html.len()).into());
                    app_content.set(Some(html));
                } else {
                    web_sys::console::warn_1(&"[Python] No UI output from app".into());
                }
                // Get any title change
                let state = runtime.state();
                let title = state.borrow().title.clone();
                if !title.is_empty() {
                    runtime_title.set(Some(title));
                }
                // Clear any previous error
                app_error.set(None);
            }
            AppExecResult::Error(err) => {
                web_sys::console::error_1(&format!("[Python] Error: {}", err).into());
                app_error.set(Some(err));
            }
            AppExecResult::InstructionLimit => {
                web_sys::console::error_1(&"[Python] Instruction limit reached".into());
                app_error.set(Some("App stopped (instruction limit reached)".to_string()));
            }
        }

        // Return whether close was requested
        let close_req = runtime.close_requested();
        web_sys::console::log_1(&format!("[Python] close_requested = {}", close_req).into());
        close_req
    }

    // Run Python code when it's provided or when input is submitted or back is pressed
    // Using use_memo pattern to run synchronously during render (not in effect)
    {
        let python_code = props.python_code.clone();
        let window_id = props.window_id.clone();
        let app_id = props.app_id.clone();
        let app_path = props.app_path.clone();
        let run_counter_val = *run_counter;

        // Track if we've run for this counter value
        let last_run_counter = use_mut_ref(|| None::<u32>);

        // Check if we need to run
        let should_run = {
            let last = *last_run_counter.borrow();
            last != Some(run_counter_val)
        };

        if should_run {
            // Update last run counter
            *last_run_counter.borrow_mut() = Some(run_counter_val);

            if let Some(code) = &python_code {
                // Take the pending input (move it out, leaving None)
                let input = pending_input.borrow_mut().take();
                // Take pending back event
                let back_pressed = std::mem::replace(&mut *pending_back.borrow_mut(), false);

                let close_requested = run_python_app(
                    code,
                    input.as_deref(),
                    back_pressed,
                    &window_id,
                    &app_id,
                    &app_path,
                    &app_content,
                    &app_error,
                    &runtime_title,
                );

                // If Python requested close (via window.close()), emit the close callback
                if close_requested {
                    web_sys::console::log_1(&"[Python] App requested window close".into());
                    props.on_close.emit(());
                }
            } else {
                web_sys::console::log_1(&format!("[Python] No code provided for {}", app_id).into());
            }
        }
    }

    // Window node ref for rendering
    let window_ref = use_node_ref();

    // Keydown handler using Yew's onkeydown callback (re-created each render)
    let on_keydown = {
        let pending_input = pending_input.clone();
        let run_counter = run_counter.clone();
        let run_counter_cell = run_counter_cell.clone();
        Callback::from(move |e: KeyboardEvent| {
            // Check if Enter was pressed
            if e.key() == "Enter" {
                // Check if the target is an input with data-on-submit
                if let Some(target) = e.target() {
                    if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                        if input.get_attribute("data-on-submit").is_some() {
                            e.prevent_default();
                            let value = input.value();
                            web_sys::console::log_1(&format!("[Input] Enter pressed, value: {}", value).into());

                            // Clear the input
                            input.set_value("");

                            // Set the pending input and trigger re-run
                            *pending_input.borrow_mut() = Some(value);
                            // Increment counter via cell (always has current value)
                            // then set state to trigger re-render
                            let new_val = {
                                let mut cell = run_counter_cell.borrow_mut();
                                *cell = cell.wrapping_add(1);
                                *cell
                            };
                            run_counter.set(new_val);
                        }
                    }
                }
            }
        })
    };

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

    // Track if window was maximized before minimizing
    let was_maximized_before_minimize = use_state(|| false);

    // Sync window_state with external minimized prop
    {
        let window_state = window_state.clone();
        let was_maximized = was_maximized_before_minimize.clone();
        let minimized_prop = props.minimized;
        use_effect_with(minimized_prop, move |&is_minimized| {
            if is_minimized {
                // Being minimized from parent - remember current state
                if *window_state == WindowState::Maximized {
                    was_maximized.set(true);
                }
                window_state.set(WindowState::Minimized);
            } else if *window_state == WindowState::Minimized {
                // Being restored - check if we were maximized before
                if *was_maximized {
                    window_state.set(WindowState::Maximized);
                    was_maximized.set(false); // Reset for next time
                } else {
                    window_state.set(WindowState::Normal);
                }
            }
            || ()
        });
    }

    // Minimize button handler - just emit callback, let parent handle state
    // The use_effect_with(minimized_prop) will update window_state when parent changes props.minimized
    let on_minimize_click = {
        let on_minimize = props.on_minimize.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
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

    // Icon and title come from props (loaded from metadata.json)
    let icon_class = "fa-solid fa-cube"; // Default fallback, real icon comes from taskbar
    let display_title = &props.title;

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

    // Mobile navbar callbacks (defined outside html! macro)
    let on_back_click = {
        let pending_back = pending_back.clone();
        let run_counter = run_counter.clone();
        let run_counter_cell = run_counter_cell.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            // Set pending back event and trigger Python re-run
            // Python's on_back() handler will decide what to do (e.g., navigate back or call window.close())
            *pending_back.borrow_mut() = true;
            let new_val = {
                let mut cell = run_counter_cell.borrow_mut();
                *cell = cell.wrapping_add(1);
                *cell
            };
            run_counter.set(new_val);
            web_sys::console::log_1(&"[Back] Back button pressed, triggering Python on_back".into());
        })
    };

    let on_home_click = {
        let on_minimize = props.on_minimize.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_minimize.emit(());
        })
    };

    let on_recents_click = {
        let on_show_recents = props.on_show_recents.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_show_recents.emit(());
        })
    };

    html! {
        <div
            ref={window_ref}
            class={window_class}
            style={window_style}
            onmousedown={on_window_mousedown}
            onkeydown={on_keydown}
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


            // Content area
            <div class="window-content">
                if let Some(error) = (*app_error).clone() {
                    // Show Python error
                    <div class="app-error" style="padding: 20px; text-align: center;">
                        <i class="fa-solid fa-triangle-exclamation fa-2x" style="color: #ff6b6b; margin-bottom: 16px;"></i>
                        <h3 style="color: #ff6b6b; margin-bottom: 12px;">{"App Error"}</h3>
                        <pre style="text-align: left; font-size: 12px; background: rgba(255,107,107,0.1); padding: 12px; border-radius: 8px; overflow: auto; max-height: 300px; white-space: pre-wrap; color: #ccc;">{error}</pre>
                    </div>
                } else if let Some(ref content) = *app_content {
                    // Render Python app UI (inject HTML directly)
                    { Html::from_html_unchecked(AttrValue::from(format!(r#"<div class="app-ui">{}</div>"#, content))) }
                } else if props.python_code.is_none() {
                    // No code yet - show loading
                    <div style="color: #888; text-align: center; padding-top: 50px;">
                        <i class={format!("{} fa-3x", icon_class)} style="margin-bottom: 20px; display: block;"></i>
                        <p>{format!("App: {}", props.app_id)}</p>
                        <p style="font-size: 12px; color: #666;">{"Loading..."}</p>
                    </div>
                } else {
                    // Code provided but no UI output
                    <div style="color: #888; text-align: center; padding-top: 50px;">
                        <i class={format!("{} fa-3x", icon_class)} style="margin-bottom: 20px; display: block;"></i>
                        <p>{format!("App: {}", props.app_id)}</p>
                        <p style="font-size: 12px; color: #666;">{"App running (no UI output)"}</p>
                    </div>
                }
            </div>


            // Resize handle (desktop only, not when maximized)
            if !is_mobile_view && !is_maximized {
                <div class="window-resize-handle" onmousedown={on_resize_mousedown}></div>
            }

            // Mobile: Navigation bar at bottom (Back, Home, Recents)
            if is_mobile_view {
                <div class="mobile-navbar">
                    <button class="nav-btn back" onclick={on_back_click} title="Zurück">
                        <i class="fa-solid fa-chevron-left"></i>
                    </button>
                    <button class="nav-btn home" onclick={on_home_click} title="Home">
                        <i class="fa-solid fa-circle"></i>
                    </button>
                    <button class="nav-btn recent" onclick={on_recents_click} title="Letzte Apps">
                        <i class="fa-solid fa-bars"></i>
                    </button>
                </div>
            }
        </div>
    }
}
