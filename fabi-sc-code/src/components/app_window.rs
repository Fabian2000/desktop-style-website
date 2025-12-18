//! App Window Component
//!
//! A draggable, resizable window that can host app content.
//! Desktop: Traditional windowed mode with drag/resize
//! Mobile: Fullscreen with navigation bar (back button)

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, KeyboardEvent, InputEvent, HtmlInputElement, HtmlTextAreaElement};
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
    /// Icon class (FontAwesome) or image path for the app
    #[prop_or_default]
    pub icon: String,
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
    /// Command line arguments passed to the app
    #[prop_or_default]
    pub args: Vec<String>,
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
    #[prop_or_default]
    pub on_launch_app: Callback<(String, Option<String>)>,  // Launch another app (app_id, file_path)
    #[prop_or_default]
    pub on_open_file: Callback<String>,  // Open a file with system handler (file_path)
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

    // Store the button click handler for re-execution (when button with on_click is pressed)
    let pending_click = use_mut_ref(|| None::<String>);

    // Store the change handler and value for textareas with on_change
    let pending_change = use_mut_ref(|| None::<(String, String)>);

    // Trigger for re-running the app (incremented on input submit or back press)
    // Using use_state for the actual trigger, plus a Cell to track the count
    // in closures that don't have access to current state
    let run_counter = use_state(|| 0u32);
    let run_counter_cell = use_mut_ref(|| 0u32);

    // Helper function to run app code with optional input, back event, click handler, or change event
    // Returns (close_requested, focus_selector, scroll_to_bottom, launch_app_request, open_file_request)
    fn run_app(
        code: &str,
        input: Option<&str>,
        back_pressed: bool,
        click_handler: Option<&str>,
        change_handler: Option<(&str, &str)>,  // (handler_name, value)
        window_id: &str,
        app_id: &str,
        app_path: &str,
        args: &[String],
        app_content: &UseStateHandle<Option<String>>,
        app_error: &UseStateHandle<Option<String>>,
        runtime_title: &UseStateHandle<Option<String>>,
    ) -> (bool, Option<String>, Option<String>, Option<(String, Option<String>)>, Option<String>) {
        web_sys::console::log_1(&format!("[Python] Running app: {} ({})", app_id, window_id).into());

        // Create Python runtime for this app
        let runtime = PythonRuntime::new(
            window_id.to_string(),
            app_id.to_string(),
            app_path.to_string(),
        );

        // Build args list as Python code: __args__ = ["arg1", "arg2", ...]
        let args_python = if args.is_empty() {
            "__args__ = []\n".to_string()
        } else {
            let escaped: Vec<String> = args.iter()
                .map(|a| format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"")))
                .collect();
            format!("__args__ = [{}]\n", escaped.join(", "))
        };

        // Prepare code with input/back/click injection if needed
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
        } else if let Some(handler) = click_handler {
            // Inject __click_handler__ and call the specified function
            web_sys::console::log_1(&format!("[Python] Injecting click handler: {}", handler).into());
            format!(
                r#"__click_handler__ = "{}"
{}
if '__click_handler__' in dir() and __click_handler__:
    if __click_handler__ in dir():
        print("[Python] Calling " + __click_handler__ + "()")
        eval(__click_handler__ + "()")
    else:
        print("[Python] WARNING: " + __click_handler__ + " not defined!")
"#,
                handler.replace('\\', "\\\\").replace('"', "\\\""),
                code
            )
        } else if let Some((handler, value)) = change_handler {
            // Inject __change_handler__ and __change_value__ and call the specified function
            web_sys::console::log_1(&format!("[Python] Injecting change handler: {} with value length: {}", handler, value.len()).into());
            format!(
                r#"__change_handler__ = "{}"
__change_value__ = """{}"""
{}
if '__change_handler__' in dir() and __change_handler__:
    if __change_handler__ in dir():
        eval(__change_handler__ + "(__change_value__)")
    else:
        print("[Python] WARNING: " + __change_handler__ + " not defined!")
"#,
                handler.replace('\\', "\\\\").replace('"', "\\\""),
                value.replace('\\', "\\\\").replace("\"\"\"", "\\\"\\\"\\\""),
                code
            )
        } else {
            code.to_string()
        };

        // Prepend args to the code
        let full_code = format!("{}{}", args_python, full_code);

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

        // Return close_requested, focus_selector, scroll_to_bottom, launch_app_request, open_file_request
        let close_req = runtime.close_requested();
        let focus_sel = runtime.take_focus_selector();
        let scroll_bottom = runtime.take_scroll_to_bottom();
        let launch_req = runtime.take_launch_app_request();
        let open_file_req = runtime.take_open_file_request();
        web_sys::console::log_1(&format!("[Python] close_requested = {}, focus_selector = {:?}, scroll_to_bottom = {:?}, launch_app = {:?}, open_file = {:?}", close_req, focus_sel, scroll_bottom, launch_req, open_file_req).into());
        (close_req, focus_sel, scroll_bottom, launch_req, open_file_req)
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
                // Take pending click handler
                let click_handler = pending_click.borrow_mut().take();
                // Take pending change handler (for textareas)
                let change_handler = pending_change.borrow_mut().take();

                let (close_requested, focus_selector, scroll_to_bottom, launch_app_request, open_file_request) = run_app(
                    code,
                    input.as_deref(),
                    back_pressed,
                    click_handler.as_deref(),
                    change_handler.as_ref().map(|(h, v)| (h.as_str(), v.as_str())),
                    &window_id,
                    &app_id,
                    &app_path,
                    &props.args,
                    &app_content,
                    &app_error,
                    &runtime_title,
                );

                // If Python requested close (via window.close()), emit the close callback
                if close_requested {
                    web_sys::console::log_1(&"[Python] App requested window close".into());
                    props.on_close.emit(());
                }

                // If Python requested to launch another app, emit the callback
                if let Some((target_app_id, file_path)) = launch_app_request {
                    web_sys::console::log_1(&format!("[Python] App requested launch: {} with file: {:?}", target_app_id, file_path).into());
                    props.on_launch_app.emit((target_app_id, file_path));
                }

                // If Python requested to open a file, emit the callback
                if let Some(file_path) = open_file_request {
                    web_sys::console::log_1(&format!("[Python] App requested open file: {}", file_path).into());
                    props.on_open_file.emit(file_path);
                }

                // If Python requested focus, do it after DOM update
                if let Some(name) = focus_selector {
                    // Build selector scoped to this window's .app-ui container
                    // The name parameter maps to data-name attribute
                    let window_id_clone = window_id.clone();
                    let name_clone = name.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        // Small delay to ensure DOM update
                        gloo_timers::future::TimeoutFuture::new(10).await;
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                // Find element by data-name inside this specific window
                                let scoped_selector = format!(
                                    "[data-window-id=\"{}\"] .app-ui [data-name=\"{}\"]",
                                    window_id_clone, name_clone
                                );
                                if let Ok(Some(element)) = document.query_selector(&scoped_selector) {
                                    if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                                        let _ = html_element.focus();
                                    }
                                }
                            }
                        }
                    });
                }

                // If Python requested scroll to bottom, do it after DOM update
                if let Some(name) = scroll_to_bottom {
                    let window_id_clone = window_id.clone();
                    let name_clone = name.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        // Small delay to ensure DOM update
                        gloo_timers::future::TimeoutFuture::new(10).await;
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                // Find element by data-name inside this specific window
                                let scoped_selector = format!(
                                    "[data-window-id=\"{}\"] .app-ui [data-name=\"{}\"]",
                                    window_id_clone, name_clone
                                );
                                if let Ok(Some(element)) = document.query_selector(&scoped_selector) {
                                    // Scroll to bottom by setting scrollTop to scrollHeight
                                    let scroll_height = element.scroll_height();
                                    element.set_scroll_top(scroll_height);
                                }
                            }
                        }
                    });
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

    // Input handler for textareas with data-on-change attribute
    let on_input = {
        let pending_change = pending_change.clone();
        let run_counter = run_counter.clone();
        let run_counter_cell = run_counter_cell.clone();
        Callback::from(move |e: InputEvent| {
            // Check if the target is a textarea with data-on-change
            if let Some(target) = e.target() {
                if let Ok(textarea) = target.dyn_into::<HtmlTextAreaElement>() {
                    if let Some(handler) = textarea.get_attribute("data-on-change") {
                        let value = textarea.value();
                        web_sys::console::log_1(&format!("[Textarea] Input changed, handler: {}, length: {}", handler, value.len()).into());

                        // Set the pending change (handler name + value) and trigger re-run
                        *pending_change.borrow_mut() = Some((handler, value));
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
        })
    };

    // Click handler for buttons with data-on-click attribute
    let on_click = {
        let pending_click = pending_click.clone();
        let run_counter = run_counter.clone();
        let run_counter_cell = run_counter_cell.clone();
        Callback::from(move |e: MouseEvent| {
            // Don't trigger click handler if user is selecting text
            // Use JS interop to check window.getSelection().toString()
            if let Some(window) = web_sys::window() {
                if let Ok(selection) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("getSelection")) {
                    if let Some(get_selection_fn) = selection.dyn_ref::<js_sys::Function>() {
                        if let Ok(sel_obj) = get_selection_fn.call0(&window) {
                            if let Ok(to_string_fn) = js_sys::Reflect::get(&sel_obj, &wasm_bindgen::JsValue::from_str("toString")) {
                                if let Some(to_string) = to_string_fn.dyn_ref::<js_sys::Function>() {
                                    if let Ok(text) = to_string.call0(&sel_obj) {
                                        if let Some(selected) = text.as_string() {
                                            if !selected.is_empty() {
                                                // User has selected text, don't trigger click handler
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check if the target or any ancestor has data-on-click
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                    // Use closest() to find button with data-on-click (handles clicks on icon children)
                    if let Ok(Some(button)) = element.closest("[data-on-click]") {
                        if let Some(handler) = button.get_attribute("data-on-click") {
                            e.prevent_default();
                            e.stop_propagation();
                            web_sys::console::log_1(&format!("[Button] Clicked, handler: {}", handler).into());

                            // Set the pending click handler and trigger re-run
                            *pending_click.borrow_mut() = Some(handler);
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
                                let (win_width, win_height) = *size_clone;

                                // Get viewport dimensions
                                let viewport_width = web_sys::window()
                                    .map(|w| w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1920.0) as i32)
                                    .unwrap_or(1920);
                                let viewport_height = web_sys::window()
                                    .map(|w| w.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1080.0) as i32)
                                    .unwrap_or(1080);

                                // Workspace boundaries
                                let top_bar = 30;
                                let taskbar_height = 90;

                                // Window must stay fully inside workspace (no overflow)
                                // Note: -3 accounts for window border and shadows
                                let min_x = 0;
                                let max_x = (viewport_width - (win_width as i32) - 3).max(0);
                                let min_y = top_bar;
                                let max_y = (viewport_height - taskbar_height - (win_height as i32)).max(min_y);

                                let new_x = (e.client_x() - offset_x).max(min_x).min(max_x);
                                let new_y = (e.client_y() - offset_y).max(min_y).min(max_y);
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

    // Prevent drag when clicking window control buttons
    let stop_drag_propagation = Callback::from(move |e: MouseEvent| {
        e.stop_propagation();
    });

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
    // Use icon from props, fallback to default cube
    // Determine if icon is FontAwesome class or image path
    let icon_value = if props.icon.is_empty() {
        "fa-solid fa-cube".to_string()
    } else {
        props.icon.clone()
    };
    let is_fa_icon = icon_value.starts_with("fa-") || icon_value.starts_with("fas ") || icon_value.starts_with("far ");
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
            data-window-id={props.window_id.clone()}
            onmousedown={on_window_mousedown}
            onkeydown={on_keydown}
            oninput={on_input}
            onclick={on_click}
        >
            // Desktop: Title bar with window controls
            if !is_mobile_view {
                <div
                    class="window-titlebar"
                    onmousedown={on_titlebar_mousedown}
                    ondblclick={on_titlebar_dblclick}
                >
                    <div class="window-title">
                        if is_fa_icon {
                            <i class={icon_value.clone()}></i>
                        } else {
                            <img src={icon_value.clone()} alt="App Icon" class="window-icon-img" />
                        }
                        <span>{display_title}</span>
                    </div>
                    <div class="window-controls">
                        <button
                            class="window-btn minimize"
                            onclick={on_minimize_click}
                            onmousedown={stop_drag_propagation.clone()}
                            title="Minimieren"
                        >
                            <i class="fa-solid fa-minus"></i>
                        </button>
                        <button
                            class="window-btn maximize"
                            onclick={on_maximize_click}
                            onmousedown={stop_drag_propagation.clone()}
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
                            onmousedown={stop_drag_propagation.clone()}
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
                        if is_fa_icon {
                            <i class={format!("{} fa-3x", icon_value)} style="margin-bottom: 20px; display: block;"></i>
                        } else {
                            <img src={icon_value.clone()} alt="App Icon" style="width: 64px; height: 64px; margin-bottom: 20px; display: block; margin-left: auto; margin-right: auto;" />
                        }
                        <p>{format!("App: {}", props.app_id)}</p>
                        <p style="font-size: 12px; color: #666;">{"Loading..."}</p>
                    </div>
                } else {
                    // Code provided but no UI output
                    <div style="color: #888; text-align: center; padding-top: 50px;">
                        if is_fa_icon {
                            <i class={format!("{} fa-3x", icon_value)} style="margin-bottom: 20px; display: block;"></i>
                        } else {
                            <img src={icon_value.clone()} alt="App Icon" style="width: 64px; height: 64px; margin-bottom: 20px; display: block; margin-left: auto; margin-right: auto;" />
                        }
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
