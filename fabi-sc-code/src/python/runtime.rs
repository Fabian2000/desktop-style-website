//! Python Runtime using RustPython
//!
//! Provides a sandboxed Python interpreter for running FabiScOS apps.
//! Features:
//! - Instruction limit to prevent infinite loops
//! - No access to real filesystem (only VFS)
//! - No network access except through app API
//! - Isolated per-app execution context

use rustpython_vm::{pymodule, Interpreter, Settings};
use std::cell::RefCell;
use std::rc::Rc;

/// Maximum number of Python instructions before execution is stopped
/// This prevents infinite loops from freezing the browser
#[allow(dead_code)]
const MAX_INSTRUCTIONS: u64 = 1_000_000;

/// Result of app execution
#[derive(Debug, Clone)]
pub enum AppExecResult {
    /// App executed successfully
    Success,
    /// App was stopped due to instruction limit
    #[allow(dead_code)]
    InstructionLimit,
    /// App encountered an error
    Error(String),
}

/// State shared between Rust and Python
#[derive(Clone)]
pub struct AppState {
    /// The window ID this app is running in
    pub window_id: String,
    /// Current working directory in VFS
    pub cwd: String,
    /// App's base path (e.g., /home/.system/apps/terminal/)
    pub app_path: String,
    /// App's ID
    pub app_id: String,
    /// Whether the app requested to close
    pub close_requested: bool,
    /// Pending UI update (HTML to inject)
    pub pending_ui: Option<String>,
    /// Current window title
    pub title: String,
    /// Output lines (for terminal-style apps)
    pub output_lines: Vec<String>,
    /// Element name to focus (via data-name attribute)
    pub focus_selector: Option<String>,
    /// Element name to scroll to bottom (via data-name attribute)
    pub scroll_to_bottom: Option<String>,
    /// Request to launch another app (app_id, optional file_path)
    pub launch_app_request: Option<(String, Option<String>)>,
    /// Request to open a file with the system handler (file_path)
    pub open_file_request: Option<String>,
}

impl AppState {
    pub fn new(window_id: String, app_id: String, app_path: String) -> Self {
        Self {
            window_id,
            cwd: "/home".to_string(),
            app_path,
            app_id,
            close_requested: false,
            pending_ui: None,
            title: String::new(),
            output_lines: Vec::new(),
            focus_selector: None,
            scroll_to_bottom: None,
            launch_app_request: None,
            open_file_request: None,
        }
    }
}

// Thread-local storage for app state (WASM is single-threaded)
thread_local! {
    static CURRENT_APP_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = RefCell::new(None);
}

/// Set the current app state for Python API calls
pub fn set_current_state(state: Rc<RefCell<AppState>>) {
    CURRENT_APP_STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });
}

/// Get the current app state from Python API calls
pub fn get_current_state() -> Option<Rc<RefCell<AppState>>> {
    CURRENT_APP_STATE.with(|s| s.borrow().clone())
}

/// Clear the current app state
pub fn clear_current_state() {
    CURRENT_APP_STATE.with(|s| {
        *s.borrow_mut() = None;
    });
}

/// Python Runtime for FabiScOS
pub struct PythonRuntime {
    /// Shared app state
    state: Rc<RefCell<AppState>>,
}

impl PythonRuntime {
    /// Create a new Python runtime for an app
    pub fn new(window_id: String, app_id: String, app_path: String) -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::new(window_id, app_id, app_path))),
        }
    }

    /// Run a Python script
    ///
    /// # Arguments
    /// * `code` - The Python source code to execute
    ///
    /// # Returns
    /// The result of execution
    pub fn run(&self, code: &str) -> AppExecResult {
        // Set the global state for this execution
        set_current_state(self.state.clone());

        // Create interpreter with settings
        let settings = Settings::default();

        let interp = Interpreter::with_init(settings, |vm| {
            // Add the fabiscos modules as separate top-level modules
            // Apps import as: import fabiscos_ui as ui
            vm.add_native_module("fabiscos".to_owned(), Box::new(fabiscos::make_module));
            vm.add_native_module("fabiscos_vfs".to_owned(), Box::new(fabiscos_vfs::make_module));
            vm.add_native_module("fabiscos_ui".to_owned(), Box::new(fabiscos_ui::make_module));
            vm.add_native_module("fabiscos_window".to_owned(), Box::new(fabiscos_window::make_module));
            vm.add_native_module("fabiscos_system".to_owned(), Box::new(fabiscos_system::make_module));
            vm.add_native_module("fabiscos_state".to_owned(), Box::new(fabiscos_state::make_module));
            vm.add_native_module("fabiscos_time".to_owned(), Box::new(fabiscos_time::make_module));
            vm.add_native_module("fabiscos_random".to_owned(), Box::new(fabiscos_random::make_module));
            vm.add_native_module("fabiscos_base64".to_owned(), Box::new(fabiscos_base64::make_module));
            vm.add_native_module("fabiscos_hash".to_owned(), Box::new(fabiscos_hash::make_module));
            vm.add_native_module("fabiscos_crypto".to_owned(), Box::new(fabiscos_crypto::make_module));
            vm.add_native_module("fabiscos_csv".to_owned(), Box::new(fabiscos_csv::make_module));
            vm.add_native_module("fabiscos_json".to_owned(), Box::new(fabiscos_json::make_module));
            vm.add_native_module("fabiscos_math".to_owned(), Box::new(fabiscos_math::make_module));
            vm.add_native_module("fabiscos_re".to_owned(), Box::new(fabiscos_re::make_module));
            vm.add_native_module("fabiscos_archive".to_owned(), Box::new(fabiscos_archive::make_module));
            vm.add_native_module("fabiscos_http".to_owned(), Box::new(fabiscos_http::make_module));
            vm.add_native_module("fabiscos_notify".to_owned(), Box::new(fabiscos_notify::make_module));
        });

        // Execute the code
        let result = interp.enter(|vm| {
            // Create a new scope
            let scope = vm.new_scope_with_builtins();

            // Setup code to redirect print() to browser console
            let setup_code = r#"
import sys
import fabiscos

class ConsoleWriter:
    def write(self, text):
        if text and text.strip():
            fabiscos.log(text.rstrip())
    def flush(self):
        pass

sys.stdout = ConsoleWriter()
sys.stderr = ConsoleWriter()
"#;

            // Compile and run setup code first
            let setup_obj = match vm.compile(
                setup_code,
                rustpython_compiler::Mode::Exec,
                "<setup>".to_owned(),
            ) {
                Ok(code) => code,
                Err(e) => return AppExecResult::Error(format!("Setup compilation error: {}", e)),
            };

            if let Err(exc) = vm.run_code_obj(setup_obj, scope.clone()) {
                let mut msg = String::new();
                if vm.write_exception(&mut msg, &exc).is_err() {
                    msg = "Setup error".to_string();
                }
                return AppExecResult::Error(msg);
            }

            // Compile the app code
            let code_obj = match vm.compile(
                code,
                rustpython_compiler::Mode::Exec,
                "<app>".to_owned(),
            ) {
                Ok(code) => code,
                Err(e) => return AppExecResult::Error(format!("Compilation error: {}", e)),
            };

            // Execute the app code
            match vm.run_code_obj(code_obj, scope) {
                Ok(_) => AppExecResult::Success,
                Err(exc) => {
                    let mut msg = String::new();
                    if vm.write_exception(&mut msg, &exc).is_err() {
                        msg = "Unknown error".to_string();
                    }
                    AppExecResult::Error(msg)
                }
            }
        });

        // Clear the global state
        clear_current_state();

        result
    }

    /// Get the current app state
    pub fn state(&self) -> Rc<RefCell<AppState>> {
        self.state.clone()
    }

    /// Get pending UI HTML and clear it
    pub fn take_pending_ui(&self) -> Option<String> {
        self.state.borrow_mut().pending_ui.take()
    }

    /// Check if close was requested
    pub fn close_requested(&self) -> bool {
        self.state.borrow().close_requested
    }

    /// Take the focus selector (returns Some(selector) if focus was requested, and clears it)
    pub fn take_focus_selector(&self) -> Option<String> {
        self.state.borrow_mut().focus_selector.take()
    }

    /// Take the scroll_to_bottom target (returns Some(name) if scroll was requested, and clears it)
    pub fn take_scroll_to_bottom(&self) -> Option<String> {
        self.state.borrow_mut().scroll_to_bottom.take()
    }

    /// Take the launch_app request (app_id, file_path)
    pub fn take_launch_app_request(&self) -> Option<(String, Option<String>)> {
        self.state.borrow_mut().launch_app_request.take()
    }

    /// Take the open_file request (file_path)
    pub fn take_open_file_request(&self) -> Option<String> {
        self.state.borrow_mut().open_file_request.take()
    }
}

// ============================================================================
// Python Modules using #[pymodule] macro
// ============================================================================

/// Main fabiscos module - re-exports submodules
#[pymodule]
mod fabiscos {
    use rustpython_vm::VirtualMachine;
    use super::get_current_state;

    #[pyfunction]
    fn version(_vm: &VirtualMachine) -> String {
        "0.1.0".to_string()
    }

    /// Log to browser console (for debugging)
    /// Format: [App → name] [Python] message
    #[pyfunction]
    fn log(message: String, _vm: &VirtualMachine) {
        let app_id = get_current_state()
            .map(|s| s.borrow().app_id.clone())
            .unwrap_or_else(|| "unknown".to_string());
        web_sys::console::log_1(&format!("[App → {}] [Python] {}", app_id, message).into());
    }
}

/// fabiscos.vfs - Virtual Filesystem API
///
/// VFS operations use the Rust in-memory cache for synchronous access.
/// Changes are persisted to IndexedDB asynchronously in the background.
#[pymodule]
mod fabiscos_vfs {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;
    use crate::python::runtime::get_current_state;
    use crate::filesystem::{
        exists_sync, read_text_sync, read_bytes_sync, list_dir_sync,
        write_sync, mkdir_sync, mkdir_p_sync, remove_sync, copy_sync, rename_sync,
        get_data_url_sync, path,
    };

    #[pyfunction]
    fn read_text(path: String, vm: &VirtualMachine) -> PyResult<String> {
        match read_text_sync(&path) {
            Some(content) => Ok(content),
            None => Err(vm.new_runtime_error(format!("Cannot read file: {}", path)))
        }
    }

    #[pyfunction]
    fn read_bytes(path: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        match read_bytes_sync(&path) {
            Some(bytes) => Ok(bytes),
            None => Err(vm.new_runtime_error(format!("Cannot read file: {}", path)))
        }
    }

    #[pyfunction]
    fn exists(path: String, _vm: &VirtualMachine) -> bool {
        exists_sync(&path)
    }

    #[pyfunction]
    fn list_dir(path: String, vm: &VirtualMachine) -> PyObjectRef {
        let entries = list_dir_sync(&path);

        // Convert to Python list of dicts
        let mut py_list = vec![];
        for entry in entries {
            let dict = PyDict::new_ref(&vm.ctx);

            // Name
            let _ = dict.set_item("name", vm.ctx.new_str(entry.name.clone()).into(), vm);

            // Type
            let type_str = if entry.is_dir() { "directory" } else { "file" };
            let _ = dict.set_item("type", vm.ctx.new_str(type_str).into(), vm);

            py_list.push(dict.into());
        }

        vm.ctx.new_list(py_list).into()
    }

    #[pyfunction]
    fn write(path: String, content: String, vm: &VirtualMachine) -> PyResult<()> {
        match write_sync(&path, content.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot write file: {} - {}", path, e)))
        }
    }

    #[pyfunction]
    fn write_bytes(path: String, data: Vec<u8>, vm: &VirtualMachine) -> PyResult<()> {
        match write_sync(&path, &data) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot write file: {} - {}", path, e)))
        }
    }

    #[pyfunction]
    fn get_data_url(path: String, vm: &VirtualMachine) -> PyResult<String> {
        match get_data_url_sync(&path) {
            Some(url) => Ok(url),
            None => Err(vm.new_runtime_error(format!("Cannot get data URL for: {}", path)))
        }
    }

    /// Create a directory (parent must exist)
    #[pyfunction]
    fn mkdir(path: String, vm: &VirtualMachine) -> PyResult<()> {
        match mkdir_sync(&path) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot create directory: {} - {}", path, e)))
        }
    }

    /// Create a directory and all parent directories (like mkdir -p)
    /// Example: mkdir_p("/home/appdata/subdir") creates both appdata and subdir
    #[pyfunction]
    fn mkdir_p(path: String, vm: &VirtualMachine) -> PyResult<()> {
        match mkdir_p_sync(&path) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot create directories: {} - {}", path, e)))
        }
    }

    #[pyfunction]
    fn remove(path: String, vm: &VirtualMachine) -> PyResult<()> {
        match remove_sync(&path) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot remove: {} - {}", path, e)))
        }
    }

    #[pyfunction]
    fn copy(src: String, dst: String, vm: &VirtualMachine) -> PyResult<()> {
        match copy_sync(&src, &dst) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot copy {} to {}: {}", src, dst, e)))
        }
    }

    #[pyfunction(name = "move")]
    fn move_file(src: String, dst: String, vm: &VirtualMachine) -> PyResult<()> {
        match rename_sync(&src, &dst) {
            Ok(()) => Ok(()),
            Err(e) => Err(vm.new_runtime_error(format!("Cannot move {} to {}: {}", src, dst, e)))
        }
    }

    /// Validate a filename (not a path, just the name portion)
    /// Returns True if valid, False if invalid
    /// Use this before creating files/directories to check if the name is valid
    #[pyfunction]
    fn valid_filename(name: String, _vm: &VirtualMachine) -> bool {
        path::validate_name(&name).is_none()
    }

    /// Validate a filename and return the error message if invalid
    /// Returns None if valid, or the error message string if invalid
    #[pyfunction]
    fn validate_filename(name: String, _vm: &VirtualMachine) -> Option<String> {
        path::validate_name(&name)
    }

    /// Sanitize a filename by replacing invalid characters with underscores
    /// Use this to automatically fix filenames before saving
    /// Example: "test🎉.txt" -> "test_.txt"
    #[pyfunction]
    fn sanitize_filename(name: String, _vm: &VirtualMachine) -> String {
        path::sanitize_name(&name)
    }

    #[pyfunction]
    fn cwd(_vm: &VirtualMachine) -> String {
        get_current_state()
            .map(|s| s.borrow().cwd.clone())
            .unwrap_or_else(|| "/home".to_string())
    }

    #[pyfunction]
    fn set_cwd(path: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().cwd = path;
        }
    }
}

/// fabiscos.ui - UI Widget API with Style support
#[pymodule]
mod fabiscos_ui {
    use rustpython_vm::VirtualMachine;
    use rustpython_vm::function::KwArgs;
    use rustpython_vm::builtins::PyStrRef;
    use crate::python::api::ui::{self, Style};

    /// Helper: Parse style from string format "key:value;key:value"
    fn parse_style(style_str: &str) -> Style {
        let mut style = Style::new();
        for part in style_str.split(';') {
            let part = part.trim();
            if !part.is_empty() {
                if let Some((k, v)) = part.split_once(':') {
                    style.set(k.trim(), v.trim());
                }
            }
        }
        style
    }

    /// Helper: Convert KwArgs to Style
    fn kwargs_to_style(kwargs: KwArgs<PyStrRef>, vm: &VirtualMachine) -> Style {
        let mut style = Style::new();
        for (key, value) in kwargs.into_iter() {
            let key_str = key.as_str();
            let val_str = value.as_str();
            style.set(key_str, val_str);
        }
        let _ = vm; // suppress warning
        style
    }

    /// Create a reusable style object
    /// Usage: mono = ui.style(font_family="monospace", color="#0f0")
    /// Any CSS property can be passed as kwargs (use snake_case)
    #[pyfunction(name = "style")]
    fn create_style(kwargs: KwArgs<PyStrRef>, vm: &VirtualMachine) -> String {
        let style = kwargs_to_style(kwargs, vm);
        // Serialize to "key:value;key:value" format
        style.properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Merge multiple style strings together
    /// Usage: combined = ui.merge_styles([base_style, extra_style])
    #[pyfunction]
    fn merge_styles(styles: Vec<String>, _vm: &VirtualMachine) -> String {
        let mut combined = Style::new();
        for style_str in styles {
            let parsed = parse_style(&style_str);
            combined.merge(&parsed);
        }
        combined.properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Helper: Extract style from kwargs, merging with base style if present
    fn extract_style_from_kwargs(kwargs: &mut std::collections::HashMap<String, String>, base_style: Option<&str>) -> Style {
        let mut s = base_style.map(|st| parse_style(st)).unwrap_or_default();
        // Apply any additional style kwargs
        for (key, value) in kwargs.iter() {
            if key != "style" {
                s.set(key, value);
            }
        }
        s
    }

    /// Text element (multiline, preserves whitespace)
    /// Usage: ui.text(content, style=my_style, color="#f00", name="output")
    /// The name parameter assigns a data-name attribute for targeting with window.scroll_to_bottom()
    #[pyfunction]
    fn text(content: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let name = kw.remove("name");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_text(&content, name.as_deref(), &s)
    }

    /// Label element (single line)
    #[pyfunction]
    fn label(content: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_label(&content, &s)
    }

    /// Button element
    /// Usage: ui.button("Click Me", style=my_style, on_click="my_handler", name="my-btn")
    /// Usage with icon: ui.button("", icon="fa-solid fa-bars", on_click="my_handler")
    /// The on_click parameter specifies the Python function to call when clicked
    /// The name parameter assigns a data-name attribute for targeting with window.focus()
    #[pyfunction]
    fn button(text: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let on_click = kw.remove("on_click");
        let icon = kw.remove("icon");
        let name = kw.remove("name");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_button(&text, on_click.as_deref(), icon.as_deref(), name.as_deref(), &s)
    }

    /// Input field
    /// Usage: ui.input("placeholder", style=my_style, flex="1", on_submit="execute", name="my-input")
    /// The on_submit parameter specifies the Python function to call when Enter is pressed
    /// The name parameter assigns a data-name attribute for targeting with window.focus()
    #[pyfunction]
    fn input(placeholder: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let on_submit = kw.remove("on_submit");
        let name = kw.remove("name");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_input(&placeholder, on_submit.as_deref(), name.as_deref(), &s)
    }

    /// Textarea (multi-line input)
    /// Usage: ui.textarea(content, style=my_style, on_change="handle_change", name="editor")
    /// The on_change parameter specifies the Python function to call when content changes
    #[pyfunction]
    fn textarea(value: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let on_change = kw.remove("on_change");
        let name = kw.remove("name");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_textarea(&value, on_change.as_deref(), name.as_deref(), &s)
    }

    /// Checkbox element
    #[pyfunction]
    fn checkbox(label_text: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let checked = kw.remove("checked").map(|v| v == "true" || v == "True").unwrap_or(false);
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_checkbox(&label_text, checked, &s)
    }

    /// Radio button element
    #[pyfunction]
    fn radio(label_text: String, name: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let checked = kw.remove("checked").map(|v| v == "true" || v == "True").unwrap_or(false);
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_radio(&label_text, &name, checked, &s)
    }

    /// Select/dropdown element
    #[pyfunction]
    fn select(options: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let selected = kw.remove("selected").and_then(|v| v.parse().ok());
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_select(&options, selected, &s)
    }

    /// Progress bar
    #[pyfunction]
    fn progress(value: f64, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let max = kw.remove("max").and_then(|v| v.parse().ok()).unwrap_or(100.0);
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_progress(value, max, &s)
    }

    /// Divider/separator line
    #[pyfunction]
    fn divider(kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_divider(&s)
    }

    /// Container (generic div)
    /// Usage: ui.container([...], style=my_style, on_click="my_handler")
    #[pyfunction]
    fn container(children: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let on_click = kw.remove("on_click");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_container(&children.join(""), on_click.as_deref(), &s)
    }

    /// Row (horizontal flex container)
    #[pyfunction]
    fn row(children: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_row(&children.join(""), &s)
    }

    /// Column (vertical flex container)
    #[pyfunction]
    fn column(children: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_column(&children.join(""), &s)
    }

    /// Spacer (flexible empty space in flex containers)
    #[pyfunction]
    fn spacer(_vm: &VirtualMachine) -> String {
        ui::render_spacer()
    }

    /// Image element
    /// If src is a VFS path (not http/https), it will be converted to a data URL
    #[pyfunction]
    fn image(src: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        use crate::filesystem::get_data_url_sync;

        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let alt = kw.remove("alt").unwrap_or_default();
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());

        // Convert VFS paths to data URLs (except http/https URLs)
        let actual_src = if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:") {
            src
        } else {
            // Try to get data URL from VFS
            get_data_url_sync(&src).unwrap_or(src)
        };

        ui::render_image(&actual_src, &alt, &s)
    }

    /// Link element
    #[pyfunction]
    fn link(text: String, href: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_link(&text, &href, &s)
    }

    /// FontAwesome icon
    /// Usage: ui.icon("fa-solid fa-bars", style=s)
    #[pyfunction]
    fn icon(icon_class: String, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_icon(&icon_class, &s)
    }

    /// Desktop-only container (hidden on mobile)
    /// Usage: ui.desktop_only([...], style=s)
    #[pyfunction]
    fn desktop_only(children: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_desktop_only(&children.join(""), &s)
    }

    /// Mobile-only container (hidden on desktop)
    /// Usage: ui.mobile_only([...], style=s)
    #[pyfunction]
    fn mobile_only(children: Vec<String>, kwargs: KwArgs<PyStrRef>, _vm: &VirtualMachine) -> String {
        let mut kw: std::collections::HashMap<String, String> = kwargs
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v.as_str().to_string()))
            .collect();
        let base_style = kw.remove("style");
        let s = extract_style_from_kwargs(&mut kw, base_style.as_deref());
        ui::render_mobile_only(&children.join(""), &s)
    }
}

/// fabiscos.window - Window Control API
#[pymodule]
mod fabiscos_window {
    use rustpython_vm::{VirtualMachine, PyObjectRef};
    use crate::python::runtime::get_current_state;

    #[pyfunction]
    fn set_title(title: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().title = title;
        }
    }

    #[pyfunction]
    fn set_content(html: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().pending_ui = Some(html);
        }
    }

    #[pyfunction]
    fn close(_vm: &VirtualMachine) {
        web_sys::console::log_1(&"[Python] window.close() called".into());
        if let Some(state) = get_current_state() {
            state.borrow_mut().close_requested = true;
            web_sys::console::log_1(&"[Python] close_requested set to true".into());
        } else {
            web_sys::console::error_1(&"[Python] window.close(): No current state!".into());
        }
    }

    #[pyfunction]
    fn print_line(text: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().output_lines.push(text);
        }
    }

    #[pyfunction]
    fn get_output(vm: &VirtualMachine) -> PyObjectRef {
        let lines = get_current_state()
            .map(|s| s.borrow().output_lines.clone())
            .unwrap_or_default();
        let py_lines: Vec<PyObjectRef> = lines
            .into_iter()
            .map(|s| vm.ctx.new_str(s).into())
            .collect();
        vm.ctx.new_list(py_lines).into()
    }

    /// Focus an element by its name (data-name attribute)
    /// Usage: window.focus("my-input") where ui.input(..., name="my-input")
    #[pyfunction]
    fn focus(name: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().focus_selector = Some(name);
        }
    }

    /// Scroll an element to the bottom (for terminal output, chat logs, etc.)
    /// Usage: window.scroll_to_bottom("output") where ui.text(..., name="output")
    #[pyfunction]
    fn scroll_to_bottom(name: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().scroll_to_bottom = Some(name);
        }
    }

    #[pyfunction]
    fn clear_output(_vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().output_lines.clear();
        }
    }
}

/// fabiscos.system - System Information API
#[pymodule]
mod fabiscos_system {
    use rustpython_vm::VirtualMachine;
    use crate::python::runtime::get_current_state;
    use crate::python::api::system;

    #[pyfunction]
    fn time(_vm: &VirtualMachine) -> String {
        system::time()
    }

    #[pyfunction]
    fn date(_vm: &VirtualMachine) -> String {
        system::date()
    }

    #[pyfunction]
    fn app_id(_vm: &VirtualMachine) -> String {
        get_current_state()
            .map(|s| s.borrow().app_id.clone())
            .unwrap_or_default()
    }

    #[pyfunction]
    fn app_path(_vm: &VirtualMachine) -> String {
        get_current_state()
            .map(|s| s.borrow().app_path.clone())
            .unwrap_or_default()
    }

    #[pyfunction]
    fn window_id(_vm: &VirtualMachine) -> String {
        get_current_state()
            .map(|s| s.borrow().window_id.clone())
            .unwrap_or_default()
    }

    /// Launch an app with an optional file path
    /// The app will be opened and receive the file_path as an argument
    #[pyfunction]
    fn launch_app(app_id: String, file_path: Option<String>, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().launch_app_request = Some((app_id, file_path));
        }
    }

    /// Open a file with the system file handler
    /// The system will find apps that can handle this file type and show a picker if needed
    #[pyfunction]
    fn open_file(file_path: String, _vm: &VirtualMachine) {
        if let Some(state) = get_current_state() {
            state.borrow_mut().open_file_request = Some(file_path);
        }
    }
}

/// fabiscos_state - Persistent App State API
/// Allows Python apps to persist state between re-runs (e.g., when handling input)
#[pymodule]
mod fabiscos_state {
    use rustpython_vm::{VirtualMachine, PyObjectRef};
    use crate::python::runtime::get_current_state;
    use wasm_bindgen::prelude::*;

    // JavaScript interop for app state
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__appState"], js_name = get)]
        fn js_state_get(window_id: &str, key: &str) -> JsValue;

        #[wasm_bindgen(js_namespace = ["window", "__appState"], js_name = set)]
        fn js_state_set(window_id: &str, key: &str, value: &str);

        #[wasm_bindgen(js_namespace = ["window", "__appState"], js_name = clear)]
        fn js_state_clear(window_id: &str);
    }

    fn get_window_id() -> String {
        get_current_state()
            .map(|s| s.borrow().window_id.clone())
            .unwrap_or_default()
    }

    /// Get a string value from persistent state
    #[pyfunction]
    fn get(key: String, _vm: &VirtualMachine) -> Option<String> {
        let wid = get_window_id();
        let val = js_state_get(&wid, &key);
        val.as_string()
    }

    /// Set a string value in persistent state
    #[pyfunction]
    fn set(key: String, value: String, _vm: &VirtualMachine) {
        let wid = get_window_id();
        js_state_set(&wid, &key, &value);
    }

    /// Get a list from persistent state (stored as JSON)
    #[pyfunction]
    fn get_list(key: String, vm: &VirtualMachine) -> PyObjectRef {
        let wid = get_window_id();
        let val = js_state_get(&wid, &key);
        if let Some(json_str) = val.as_string() {
            // Parse JSON array
            if let Ok(parsed) = js_sys::JSON::parse(&json_str) {
                if let Some(array) = parsed.dyn_ref::<js_sys::Array>() {
                    let mut items = vec![];
                    for i in 0..array.length() {
                        let item = array.get(i);
                        if let Some(s) = item.as_string() {
                            items.push(vm.ctx.new_str(s).into());
                        }
                    }
                    return vm.ctx.new_list(items).into();
                }
            }
        }
        vm.ctx.new_list(vec![]).into()
    }

    /// Set a list in persistent state (stored as JSON)
    #[pyfunction]
    fn set_list(key: String, items: Vec<String>, _vm: &VirtualMachine) {
        let wid = get_window_id();
        // Convert to JSON array
        let array = js_sys::Array::new();
        for item in items {
            array.push(&JsValue::from_str(&item));
        }
        if let Ok(json) = js_sys::JSON::stringify(&array) {
            if let Some(json_str) = json.as_string() {
                js_state_set(&wid, &key, &json_str);
            }
        }
    }

    /// Clear all state for this window
    #[pyfunction]
    fn clear(_vm: &VirtualMachine) {
        let wid = get_window_id();
        js_state_clear(&wid);
    }
}

/// fabiscos_time - Time and timing utilities
/// Note: sleep/set_timeout/set_interval are NOT truly async in synchronous Python.
/// They record the time and can be checked later.
#[pymodule]
mod fabiscos_time {
    use rustpython_vm::VirtualMachine;

    /// Get current timestamp in milliseconds since Unix epoch
    #[pyfunction]
    fn now(_vm: &VirtualMachine) -> f64 {
        js_sys::Date::now()
    }

    /// Get monotonic time in milliseconds (high resolution)
    /// Better for measuring elapsed time than now()
    #[pyfunction]
    fn monotonic(_vm: &VirtualMachine) -> f64 {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or_else(|| js_sys::Date::now())
    }

    /// Get current time as ISO 8601 string
    #[pyfunction]
    fn iso_now(_vm: &VirtualMachine) -> String {
        let date = js_sys::Date::new_0();
        date.to_iso_string().into()
    }

    /// Get timestamp from date components (year, month 1-12, day, hour, minute, second)
    #[pyfunction]
    fn timestamp(year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, _vm: &VirtualMachine) -> f64 {
        let date = js_sys::Date::new_0();
        date.set_full_year(year);
        date.set_month(month - 1); // JS months are 0-indexed
        date.set_date(day);
        date.set_hours(hour);
        date.set_minutes(minute);
        date.set_seconds(second);
        date.set_milliseconds(0);
        date.get_time()
    }

    /// Format timestamp (ms since epoch) to localized time string
    #[pyfunction]
    fn format_time(timestamp_ms: f64, _vm: &VirtualMachine) -> String {
        let date = js_sys::Date::new(&timestamp_ms.into());
        date.to_locale_time_string("de-DE").into()
    }

    /// Format timestamp to localized date string
    #[pyfunction]
    fn format_date(timestamp_ms: f64, _vm: &VirtualMachine) -> String {
        let date = js_sys::Date::new(&timestamp_ms.into());
        date.to_locale_date_string("de-DE", &wasm_bindgen::JsValue::UNDEFINED).into()
    }

    /// Format timestamp to ISO 8601 string
    #[pyfunction]
    fn format_iso(timestamp_ms: f64, _vm: &VirtualMachine) -> String {
        let date = js_sys::Date::new(&timestamp_ms.into());
        date.to_iso_string().into()
    }
}

/// fabiscos_random - Random number generation
#[pymodule]
mod fabiscos_random {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rand::Rng;

    /// Generate random integer in range [min, max] (inclusive)
    #[pyfunction]
    fn randint(min: i64, max: i64, _vm: &VirtualMachine) -> i64 {
        let mut rng = rand::rng();
        rng.random_range(min..=max)
    }

    /// Generate random float in range [0.0, 1.0)
    #[pyfunction]
    fn random(_vm: &VirtualMachine) -> f64 {
        let mut rng = rand::rng();
        rng.random::<f64>()
    }

    /// Generate random float in range [min, max)
    #[pyfunction]
    fn uniform(min: f64, max: f64, _vm: &VirtualMachine) -> f64 {
        let mut rng = rand::rng();
        rng.random_range(min..max)
    }

    /// Generate random bytes (cryptographically secure)
    #[pyfunction]
    fn random_bytes(count: usize, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        if count > 1024 * 1024 {
            return Err(vm.new_value_error("Cannot generate more than 1MB of random bytes".to_string()));
        }
        let mut bytes = vec![0u8; count];
        getrandom::fill(&mut bytes).map_err(|e| vm.new_runtime_error(format!("Random generation failed: {}", e)))?;
        Ok(bytes)
    }

    /// Generate a random UUID v4
    #[pyfunction]
    fn uuid4(_vm: &VirtualMachine) -> String {
        let mut bytes = [0u8; 16];
        let _ = getrandom::fill(&mut bytes);
        // Set version (4) and variant (RFC 4122)
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            u16::from_be_bytes([bytes[4], bytes[5]]),
            u16::from_be_bytes([bytes[6], bytes[7]]),
            u16::from_be_bytes([bytes[8], bytes[9]]),
            u64::from_be_bytes([0, 0, bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]) & 0xffffffffffff
        )
    }

    /// Choose a random element from a list
    #[pyfunction]
    fn choice(items: Vec<String>, vm: &VirtualMachine) -> PyResult<String> {
        if items.is_empty() {
            return Err(vm.new_value_error("Cannot choose from empty list".to_string()));
        }
        let mut rng = rand::rng();
        let idx = rng.random_range(0..items.len());
        Ok(items[idx].clone())
    }

    /// Shuffle a list and return it
    #[pyfunction]
    fn shuffle(items: Vec<String>, vm: &VirtualMachine) -> PyObjectRef {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        let mut items = items;
        items.shuffle(&mut rng);
        let py_items: Vec<PyObjectRef> = items.into_iter().map(|s| vm.ctx.new_str(s).into()).collect();
        vm.ctx.new_list(py_items).into()
    }
}

/// fabiscos_base64 - Base64 encoding/decoding
#[pymodule]
mod fabiscos_base64 {
    use rustpython_vm::{VirtualMachine, PyResult};
    use base64::{Engine as _, engine::general_purpose};

    /// Encode bytes to base64 string
    #[pyfunction]
    fn encode(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        general_purpose::STANDARD.encode(&data)
    }

    /// Encode string to base64
    #[pyfunction]
    fn encode_str(text: String, _vm: &VirtualMachine) -> String {
        general_purpose::STANDARD.encode(text.as_bytes())
    }

    /// Decode base64 string to bytes
    #[pyfunction]
    fn decode(encoded: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        general_purpose::STANDARD
            .decode(&encoded)
            .map_err(|e| vm.new_value_error(format!("Invalid base64: {}", e)))
    }

    /// Decode base64 string to UTF-8 string
    #[pyfunction]
    fn decode_str(encoded: String, vm: &VirtualMachine) -> PyResult<String> {
        let bytes = general_purpose::STANDARD
            .decode(&encoded)
            .map_err(|e| vm.new_value_error(format!("Invalid base64: {}", e)))?;
        String::from_utf8(bytes)
            .map_err(|e| vm.new_value_error(format!("Invalid UTF-8: {}", e)))
    }

    /// Encode to URL-safe base64
    #[pyfunction]
    fn encode_url(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        general_purpose::URL_SAFE_NO_PAD.encode(&data)
    }

    /// Decode URL-safe base64
    #[pyfunction]
    fn decode_url(encoded: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        general_purpose::URL_SAFE_NO_PAD
            .decode(&encoded)
            .map_err(|e| vm.new_value_error(format!("Invalid base64url: {}", e)))
    }
}

/// fabiscos_hash - Cryptographic hash functions
#[pymodule]
mod fabiscos_hash {
    use rustpython_vm::VirtualMachine;
    use sha2::{Sha256, Sha512, Digest};
    use sha1::Sha1;
    use md5::Md5;
    use hmac::{Hmac, Mac};

    fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Compute MD5 hash (not secure, for compatibility only)
    #[pyfunction]
    fn md5(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        let mut hasher = Md5::new();
        hasher.update(&data);
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute MD5 hash of string
    #[pyfunction]
    fn md5_str(text: String, _vm: &VirtualMachine) -> String {
        let mut hasher = Md5::new();
        hasher.update(text.as_bytes());
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-1 hash (not secure, for compatibility only)
    #[pyfunction]
    fn sha1(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha1::new();
        hasher.update(&data);
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-1 hash of string
    #[pyfunction]
    fn sha1_str(text: String, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha1::new();
        hasher.update(text.as_bytes());
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-256 hash
    #[pyfunction]
    fn sha256(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-256 hash of string
    #[pyfunction]
    fn sha256_str(text: String, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-512 hash
    #[pyfunction]
    fn sha512(data: Vec<u8>, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha512::new();
        hasher.update(&data);
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute SHA-512 hash of string
    #[pyfunction]
    fn sha512_str(text: String, _vm: &VirtualMachine) -> String {
        let mut hasher = Sha512::new();
        hasher.update(text.as_bytes());
        bytes_to_hex(&hasher.finalize())
    }

    /// Compute HMAC-SHA256
    #[pyfunction]
    fn hmac_sha256(key: Vec<u8>, data: Vec<u8>, _vm: &VirtualMachine) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC can take key of any size");
        mac.update(&data);
        bytes_to_hex(&mac.finalize().into_bytes())
    }

    /// Compute HMAC-SHA256 from strings
    #[pyfunction]
    fn hmac_sha256_str(key: String, data: String, _vm: &VirtualMachine) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        bytes_to_hex(&mac.finalize().into_bytes())
    }
}

/// fabiscos_crypto - Symmetric encryption (AES-256-CBC)
#[pymodule]
mod fabiscos_crypto {
    use rustpython_vm::{VirtualMachine, PyResult};
    use aes::Aes256;
    use aes::cipher::{BlockEncrypt, BlockDecrypt, KeyInit, generic_array::GenericArray};
    use base64::{Engine as _, engine::general_purpose};
    use sha2::{Sha256, Digest};

    /// Derive a 32-byte key from password using SHA-256
    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    /// PKCS7 padding
    fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
        let block_size = 16;
        let padding_len = block_size - (data.len() % block_size);
        let mut padded = data.to_vec();
        padded.extend(std::iter::repeat(padding_len as u8).take(padding_len));
        padded
    }

    /// PKCS7 unpadding
    fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, &'static str> {
        if data.is_empty() {
            return Err("Empty data");
        }
        let padding_len = *data.last().unwrap() as usize;
        if padding_len == 0 || padding_len > 16 || padding_len > data.len() {
            return Err("Invalid padding");
        }
        for &byte in &data[data.len() - padding_len..] {
            if byte as usize != padding_len {
                return Err("Invalid padding bytes");
            }
        }
        Ok(data[..data.len() - padding_len].to_vec())
    }

    /// XOR two byte slices
    fn xor_blocks(a: &mut [u8], b: &[u8]) {
        for (x, y) in a.iter_mut().zip(b.iter()) {
            *x ^= *y;
        }
    }

    /// Encrypt data with password (AES-256-CBC)
    /// Returns base64-encoded ciphertext with IV prepended
    #[pyfunction]
    fn encrypt(data: Vec<u8>, password: String, vm: &VirtualMachine) -> PyResult<String> {
        let key = derive_key(&password);
        let cipher = Aes256::new(GenericArray::from_slice(&key));

        // Generate random IV
        let mut iv = [0u8; 16];
        getrandom::fill(&mut iv).map_err(|e| vm.new_runtime_error(format!("IV generation failed: {}", e)))?;

        // Pad data
        let padded = pkcs7_pad(&data);

        // CBC encrypt
        let mut ciphertext = Vec::with_capacity(padded.len());
        let mut prev_block = iv;

        for chunk in padded.chunks(16) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            xor_blocks(&mut block, &prev_block);

            let mut block_arr = GenericArray::clone_from_slice(&block);
            cipher.encrypt_block(&mut block_arr);

            prev_block.copy_from_slice(&block_arr);
            ciphertext.extend_from_slice(&block_arr);
        }

        // Prepend IV and encode
        let mut result = iv.to_vec();
        result.extend(ciphertext);
        Ok(general_purpose::STANDARD.encode(&result))
    }

    /// Encrypt string with password
    #[pyfunction]
    fn encrypt_str(text: String, password: String, vm: &VirtualMachine) -> PyResult<String> {
        encrypt(text.into_bytes(), password, vm)
    }

    /// Decrypt data with password (AES-256-CBC)
    /// Input is base64-encoded ciphertext with IV prepended
    #[pyfunction]
    fn decrypt(encoded: String, password: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        let key = derive_key(&password);
        let cipher = Aes256::new(GenericArray::from_slice(&key));

        // Decode base64
        let data = general_purpose::STANDARD
            .decode(&encoded)
            .map_err(|e| vm.new_value_error(format!("Invalid base64: {}", e)))?;

        if data.len() < 32 || data.len() % 16 != 0 {
            return Err(vm.new_value_error("Invalid ciphertext length".to_string()));
        }

        // Extract IV and ciphertext
        let iv: [u8; 16] = data[..16].try_into().unwrap();
        let ciphertext = &data[16..];

        // CBC decrypt
        let mut plaintext = Vec::with_capacity(ciphertext.len());
        let mut prev_block = iv;

        for chunk in ciphertext.chunks(16) {
            let mut block_arr = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block_arr);

            let mut decrypted = [0u8; 16];
            decrypted.copy_from_slice(&block_arr);
            xor_blocks(&mut decrypted, &prev_block);

            plaintext.extend_from_slice(&decrypted);
            prev_block.copy_from_slice(chunk);
        }

        // Remove padding
        pkcs7_unpad(&plaintext)
            .map_err(|e| vm.new_value_error(format!("Invalid padding: {}", e)))
    }

    /// Decrypt to string with password
    #[pyfunction]
    fn decrypt_str(encoded: String, password: String, vm: &VirtualMachine) -> PyResult<String> {
        let bytes = decrypt(encoded, password, vm)?;
        String::from_utf8(bytes)
            .map_err(|e| vm.new_value_error(format!("Invalid UTF-8: {}", e)))
    }
}

/// fabiscos_csv - CSV parsing and writing
#[pymodule]
mod fabiscos_csv {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;

    /// Parse CSV string into list of lists
    #[pyfunction]
    fn parse(content: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(content.as_bytes());

        let mut rows = vec![];
        for result in reader.records() {
            let record = result.map_err(|e| vm.new_value_error(format!("CSV parse error: {}", e)))?;
            let row: Vec<PyObjectRef> = record
                .iter()
                .map(|field| vm.ctx.new_str(field.to_string()).into())
                .collect();
            rows.push(vm.ctx.new_list(row).into());
        }
        Ok(vm.ctx.new_list(rows).into())
    }

    /// Parse CSV with headers into list of dicts
    #[pyfunction]
    fn parse_dict(content: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());

        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| vm.new_value_error(format!("CSV header error: {}", e)))?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows = vec![];
        for result in reader.records() {
            let record = result.map_err(|e| vm.new_value_error(format!("CSV parse error: {}", e)))?;
            let dict = PyDict::new_ref(&vm.ctx);
            for (i, field) in record.iter().enumerate() {
                if i < headers.len() {
                    let _ = dict.set_item(&headers[i], vm.ctx.new_str(field.to_string()).into(), vm);
                }
            }
            rows.push(dict.into());
        }
        Ok(vm.ctx.new_list(rows).into())
    }

    /// Convert list of lists to CSV string
    #[pyfunction]
    fn stringify(rows: Vec<Vec<String>>, _vm: &VirtualMachine) -> String {
        let mut writer = csv::Writer::from_writer(vec![]);
        for row in rows {
            let _ = writer.write_record(&row);
        }
        let data = writer.into_inner().unwrap_or_default();
        String::from_utf8(data).unwrap_or_default()
    }

    /// Convert list of lists to CSV with custom headers
    #[pyfunction]
    fn stringify_with_headers(headers: Vec<String>, rows: Vec<Vec<String>>, _vm: &VirtualMachine) -> String {
        let mut writer = csv::Writer::from_writer(vec![]);
        let _ = writer.write_record(&headers);
        for row in rows {
            let _ = writer.write_record(&row);
        }
        let data = writer.into_inner().unwrap_or_default();
        String::from_utf8(data).unwrap_or_default()
    }
}

/// fabiscos_json - JSON encoding/decoding
#[pymodule]
mod fabiscos_json {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::{PyDict, PyList};

    /// Parse JSON string to Python object (dict, list, str, int, float, bool, None)
    #[pyfunction]
    fn loads(json_str: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| vm.new_value_error(format!("Invalid JSON: {}", e)))?;
        json_to_python(value, vm)
    }

    /// Convert Python object to JSON string
    #[pyfunction]
    fn dumps(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
        let value = python_to_json(obj, vm)?;
        serde_json::to_string(&value)
            .map_err(|e| vm.new_value_error(format!("JSON serialization error: {}", e)))
    }

    /// Convert Python object to pretty-printed JSON string
    #[pyfunction]
    fn dumps_pretty(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
        let value = python_to_json(obj, vm)?;
        serde_json::to_string_pretty(&value)
            .map_err(|e| vm.new_value_error(format!("JSON serialization error: {}", e)))
    }

    fn json_to_python(value: serde_json::Value, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        use serde_json::Value;
        match value {
            Value::Null => Ok(vm.ctx.none()),
            Value::Bool(b) => Ok(vm.ctx.new_bool(b).into()),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(vm.ctx.new_int(i).into())
                } else if let Some(f) = n.as_f64() {
                    Ok(vm.ctx.new_float(f).into())
                } else {
                    Ok(vm.ctx.none())
                }
            }
            Value::String(s) => Ok(vm.ctx.new_str(s).into()),
            Value::Array(arr) => {
                let mut items = vec![];
                for item in arr {
                    items.push(json_to_python(item, vm)?);
                }
                Ok(vm.ctx.new_list(items).into())
            }
            Value::Object(obj) => {
                let dict = PyDict::new_ref(&vm.ctx);
                for (k, v) in obj {
                    let _ = dict.set_item(&k, json_to_python(v, vm)?, vm);
                }
                Ok(dict.into())
            }
        }
    }

    fn python_to_json(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<serde_json::Value> {
        use serde_json::Value;
        use rustpython_vm::builtins::{PyInt, PyFloat, PyStr};
        use rustpython_vm::AsObject;

        // Check for None
        if vm.is_none(&obj) {
            return Ok(Value::Null);
        }

        // Get type name for checking
        let type_name = obj.class().name().to_string();

        // Check for bool (must be before int, as bool is subclass of int in Python)
        if type_name == "bool" {
            // Get bool value by checking if it's the True singleton
            let is_true = obj.is(&vm.ctx.true_value);
            return Ok(Value::Bool(is_true));
        }

        // Check for int - convert via string representation to avoid BigInt trait issues
        if let Some(i) = obj.payload::<PyInt>() {
            let int_str = i.as_bigint().to_string();
            if let Ok(val) = int_str.parse::<i64>() {
                return Ok(Value::Number(val.into()));
            }
            return Err(vm.new_value_error("Integer too large for JSON".to_string()));
        }

        // Check for float
        if let Some(f) = obj.payload::<PyFloat>() {
            let val = f.to_f64();
            if let Some(n) = serde_json::Number::from_f64(val) {
                return Ok(Value::Number(n));
            }
            return Err(vm.new_value_error("Cannot serialize NaN or Infinity".to_string()));
        }

        // Check for string
        if let Some(s) = obj.payload::<PyStr>() {
            return Ok(Value::String(s.as_str().to_string()));
        }

        // Check for list
        if let Some(list) = obj.payload::<PyList>() {
            let mut arr = vec![];
            for item in list.borrow_vec().iter() {
                arr.push(python_to_json(item.clone(), vm)?);
            }
            return Ok(Value::Array(arr));
        }

        // Check for dict
        if let Some(dict) = obj.payload::<PyDict>() {
            let mut map = serde_json::Map::new();
            for (k, v) in dict.into_iter() {
                let key_str = k.str(vm)
                    .map_err(|_| vm.new_value_error("Dict key must be string".to_string()))?
                    .as_str()
                    .to_string();
                map.insert(key_str, python_to_json(v, vm)?);
            }
            return Ok(Value::Object(map));
        }

        Err(vm.new_type_error(format!("Object of type {} is not JSON serializable", type_name)))
    }
}

/// fabiscos_math - Mathematical functions
#[pymodule]
mod fabiscos_math {
    use rustpython_vm::{VirtualMachine, PyResult};

    // ===== Constants =====

    /// Mathematical constant pi (π ≈ 3.14159...)
    #[pyfunction]
    fn pi(_vm: &VirtualMachine) -> f64 {
        std::f64::consts::PI
    }

    /// Mathematical constant e (Euler's number ≈ 2.71828...)
    #[pyfunction]
    fn e(_vm: &VirtualMachine) -> f64 {
        std::f64::consts::E
    }

    /// Positive infinity
    #[pyfunction]
    fn inf(_vm: &VirtualMachine) -> f64 {
        f64::INFINITY
    }

    /// Not a Number (NaN)
    #[pyfunction]
    fn nan(_vm: &VirtualMachine) -> f64 {
        f64::NAN
    }

    /// Tau (τ = 2π ≈ 6.28318...)
    #[pyfunction]
    fn tau(_vm: &VirtualMachine) -> f64 {
        std::f64::consts::TAU
    }

    // ===== Basic arithmetic =====

    /// Absolute value
    #[pyfunction]
    fn abs(x: f64, _vm: &VirtualMachine) -> f64 {
        x.abs()
    }

    /// Floor (round down)
    #[pyfunction]
    fn floor(x: f64, _vm: &VirtualMachine) -> f64 {
        x.floor()
    }

    /// Ceiling (round up)
    #[pyfunction]
    fn ceil(x: f64, _vm: &VirtualMachine) -> f64 {
        x.ceil()
    }

    /// Round to nearest integer
    #[pyfunction]
    fn round(x: f64, _vm: &VirtualMachine) -> f64 {
        x.round()
    }

    /// Truncate (remove fractional part)
    #[pyfunction]
    fn trunc(x: f64, _vm: &VirtualMachine) -> f64 {
        x.trunc()
    }

    /// Sign of number: -1, 0, or 1
    #[pyfunction]
    fn sign(x: f64, _vm: &VirtualMachine) -> f64 {
        if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }
    }

    /// Copy sign from y to x
    #[pyfunction]
    fn copysign(x: f64, y: f64, _vm: &VirtualMachine) -> f64 {
        x.copysign(y)
    }

    /// Modulo (remainder)
    #[pyfunction]
    fn fmod(x: f64, y: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if y == 0.0 {
            Err(vm.new_value_error("math domain error: fmod by zero".to_string()))
        } else {
            Ok(x % y)
        }
    }

    /// Return integer and fractional parts
    #[pyfunction]
    fn modf(x: f64, vm: &VirtualMachine) -> (f64, f64) {
        let _ = vm;
        (x.fract(), x.trunc())
    }

    // ===== Powers and roots =====

    /// Square root
    #[pyfunction]
    fn sqrt(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x < 0.0 {
            Err(vm.new_value_error("math domain error: sqrt of negative number".to_string()))
        } else {
            Ok(x.sqrt())
        }
    }

    /// Cube root
    #[pyfunction]
    fn cbrt(x: f64, _vm: &VirtualMachine) -> f64 {
        x.cbrt()
    }

    /// Power (x^y)
    #[pyfunction]
    fn pow(x: f64, y: f64, _vm: &VirtualMachine) -> f64 {
        x.powf(y)
    }

    /// Euclidean distance sqrt(x² + y²)
    #[pyfunction]
    fn hypot(x: f64, y: f64, _vm: &VirtualMachine) -> f64 {
        x.hypot(y)
    }

    // ===== Exponential and logarithmic =====

    /// e^x
    #[pyfunction]
    fn exp(x: f64, _vm: &VirtualMachine) -> f64 {
        x.exp()
    }

    /// 2^x
    #[pyfunction]
    fn exp2(x: f64, _vm: &VirtualMachine) -> f64 {
        x.exp2()
    }

    /// e^x - 1 (accurate for small x)
    #[pyfunction]
    fn expm1(x: f64, _vm: &VirtualMachine) -> f64 {
        x.exp_m1()
    }

    /// Natural logarithm (base e)
    #[pyfunction]
    fn log(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= 0.0 {
            Err(vm.new_value_error("math domain error: log of non-positive number".to_string()))
        } else {
            Ok(x.ln())
        }
    }

    /// Base-10 logarithm
    #[pyfunction]
    fn log10(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= 0.0 {
            Err(vm.new_value_error("math domain error: log10 of non-positive number".to_string()))
        } else {
            Ok(x.log10())
        }
    }

    /// Base-2 logarithm
    #[pyfunction]
    fn log2(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= 0.0 {
            Err(vm.new_value_error("math domain error: log2 of non-positive number".to_string()))
        } else {
            Ok(x.log2())
        }
    }

    /// log(1 + x) (accurate for small x)
    #[pyfunction]
    fn log1p(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= -1.0 {
            Err(vm.new_value_error("math domain error: log1p domain error".to_string()))
        } else {
            Ok(x.ln_1p())
        }
    }

    /// Logarithm with arbitrary base
    #[pyfunction]
    fn logn(x: f64, base: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= 0.0 || base <= 0.0 || base == 1.0 {
            Err(vm.new_value_error("math domain error: invalid logarithm arguments".to_string()))
        } else {
            Ok(x.log(base))
        }
    }

    // ===== Trigonometric functions (radians) =====

    /// Sine
    #[pyfunction]
    fn sin(x: f64, _vm: &VirtualMachine) -> f64 {
        x.sin()
    }

    /// Cosine
    #[pyfunction]
    fn cos(x: f64, _vm: &VirtualMachine) -> f64 {
        x.cos()
    }

    /// Tangent
    #[pyfunction]
    fn tan(x: f64, _vm: &VirtualMachine) -> f64 {
        x.tan()
    }

    /// Arc sine (inverse sine)
    #[pyfunction]
    fn asin(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x < -1.0 || x > 1.0 {
            Err(vm.new_value_error("math domain error: asin out of range [-1, 1]".to_string()))
        } else {
            Ok(x.asin())
        }
    }

    /// Arc cosine (inverse cosine)
    #[pyfunction]
    fn acos(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x < -1.0 || x > 1.0 {
            Err(vm.new_value_error("math domain error: acos out of range [-1, 1]".to_string()))
        } else {
            Ok(x.acos())
        }
    }

    /// Arc tangent (inverse tangent)
    #[pyfunction]
    fn atan(x: f64, _vm: &VirtualMachine) -> f64 {
        x.atan()
    }

    /// Arc tangent of y/x (quadrant-aware)
    #[pyfunction]
    fn atan2(y: f64, x: f64, _vm: &VirtualMachine) -> f64 {
        y.atan2(x)
    }

    // ===== Hyperbolic functions =====

    /// Hyperbolic sine
    #[pyfunction]
    fn sinh(x: f64, _vm: &VirtualMachine) -> f64 {
        x.sinh()
    }

    /// Hyperbolic cosine
    #[pyfunction]
    fn cosh(x: f64, _vm: &VirtualMachine) -> f64 {
        x.cosh()
    }

    /// Hyperbolic tangent
    #[pyfunction]
    fn tanh(x: f64, _vm: &VirtualMachine) -> f64 {
        x.tanh()
    }

    /// Inverse hyperbolic sine
    #[pyfunction]
    fn asinh(x: f64, _vm: &VirtualMachine) -> f64 {
        x.asinh()
    }

    /// Inverse hyperbolic cosine
    #[pyfunction]
    fn acosh(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x < 1.0 {
            Err(vm.new_value_error("math domain error: acosh requires x >= 1".to_string()))
        } else {
            Ok(x.acosh())
        }
    }

    /// Inverse hyperbolic tangent
    #[pyfunction]
    fn atanh(x: f64, vm: &VirtualMachine) -> PyResult<f64> {
        if x <= -1.0 || x >= 1.0 {
            Err(vm.new_value_error("math domain error: atanh requires -1 < x < 1".to_string()))
        } else {
            Ok(x.atanh())
        }
    }

    // ===== Angular conversion =====

    /// Convert radians to degrees
    #[pyfunction]
    fn degrees(x: f64, _vm: &VirtualMachine) -> f64 {
        x.to_degrees()
    }

    /// Convert degrees to radians
    #[pyfunction]
    fn radians(x: f64, _vm: &VirtualMachine) -> f64 {
        x.to_radians()
    }

    // ===== Special functions =====

    /// Factorial (n!)
    #[pyfunction]
    fn factorial(n: i64, vm: &VirtualMachine) -> PyResult<i64> {
        if n < 0 {
            return Err(vm.new_value_error("factorial not defined for negative numbers".to_string()));
        }
        if n > 20 {
            return Err(vm.new_value_error("factorial too large (max 20)".to_string()));
        }
        let mut result: i64 = 1;
        for i in 2..=n {
            result *= i;
        }
        Ok(result)
    }

    /// Greatest common divisor
    #[pyfunction]
    fn gcd(a: i64, b: i64, _vm: &VirtualMachine) -> i64 {
        let mut a = a.abs();
        let mut b = b.abs();
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }

    /// Least common multiple
    #[pyfunction]
    fn lcm(a: i64, b: i64, vm: &VirtualMachine) -> i64 {
        let _ = vm;
        if a == 0 || b == 0 {
            0
        } else {
            let a_abs = a.abs();
            let b_abs = b.abs();
            let g = {
                let mut x = a_abs;
                let mut y = b_abs;
                while y != 0 {
                    let temp = y;
                    y = x % y;
                    x = temp;
                }
                x
            };
            a_abs / g * b_abs
        }
    }

    // ===== Check functions =====

    /// Check if value is finite
    #[pyfunction]
    fn isfinite(x: f64, _vm: &VirtualMachine) -> bool {
        x.is_finite()
    }

    /// Check if value is infinite
    #[pyfunction]
    fn isinf(x: f64, _vm: &VirtualMachine) -> bool {
        x.is_infinite()
    }

    /// Check if value is NaN
    #[pyfunction]
    fn isnan(x: f64, _vm: &VirtualMachine) -> bool {
        x.is_nan()
    }

    /// Check if two floats are close
    #[pyfunction]
    fn isclose(a: f64, b: f64, rel_tol: Option<f64>, abs_tol: Option<f64>, _vm: &VirtualMachine) -> bool {
        let rel = rel_tol.unwrap_or(1e-9);
        let abs = abs_tol.unwrap_or(0.0);
        (a - b).abs() <= f64::max(rel * f64::max(a.abs(), b.abs()), abs)
    }

    // ===== Aggregation functions =====

    /// Sum of a list
    #[pyfunction]
    fn sum(values: Vec<f64>, _vm: &VirtualMachine) -> f64 {
        values.iter().sum()
    }

    /// Product of a list
    #[pyfunction]
    fn prod(values: Vec<f64>, _vm: &VirtualMachine) -> f64 {
        values.iter().product()
    }

    /// Minimum of a list
    #[pyfunction]
    fn min(values: Vec<f64>, vm: &VirtualMachine) -> PyResult<f64> {
        values.iter().cloned().reduce(f64::min)
            .ok_or_else(|| vm.new_value_error("min of empty list".to_string()))
    }

    /// Maximum of a list
    #[pyfunction]
    fn max(values: Vec<f64>, vm: &VirtualMachine) -> PyResult<f64> {
        values.iter().cloned().reduce(f64::max)
            .ok_or_else(|| vm.new_value_error("max of empty list".to_string()))
    }

    /// Clamp value between min and max
    #[pyfunction]
    fn clamp(x: f64, min_val: f64, max_val: f64, _vm: &VirtualMachine) -> f64 {
        x.max(min_val).min(max_val)
    }

    /// Linear interpolation between a and b
    #[pyfunction]
    fn lerp(a: f64, b: f64, t: f64, _vm: &VirtualMachine) -> f64 {
        a + (b - a) * t
    }

    /// Map value from one range to another
    #[pyfunction]
    fn map_range(x: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64, _vm: &VirtualMachine) -> f64 {
        (x - in_min) / (in_max - in_min) * (out_max - out_min) + out_min
    }
}

/// fabiscos_re - Regular expression module
#[pymodule]
mod fabiscos_re {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;
    use regex::{Regex, RegexBuilder};

    /// Build regex from pattern with optional flags
    fn build_regex(pattern: &str, flags: Option<String>, vm: &VirtualMachine) -> PyResult<Regex> {
        let mut builder = RegexBuilder::new(pattern);

        if let Some(f) = flags {
            for c in f.chars() {
                match c {
                    'i' => { builder.case_insensitive(true); }
                    'm' => { builder.multi_line(true); }
                    's' => { builder.dot_matches_new_line(true); }
                    'x' => { builder.ignore_whitespace(true); }
                    _ => {}
                }
            }
        }

        builder.build()
            .map_err(|e| vm.new_value_error(format!("Invalid regex: {}", e)))
    }

    /// Check if pattern matches anywhere in text
    /// Returns True if match found, False otherwise
    #[pyfunction]
    fn test(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<bool> {
        let re = build_regex(&pattern, flags, vm)?;
        Ok(re.is_match(&text))
    }

    /// Find first match in text
    /// Returns dict with {match, start, end, groups} or None
    #[pyfunction]
    fn search(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        match re.captures(&text) {
            Some(caps) => {
                let dict = PyDict::new_ref(&vm.ctx);

                // Full match
                if let Some(m) = caps.get(0) {
                    let _ = dict.set_item("match", vm.ctx.new_str(m.as_str().to_string()).into(), vm);
                    let _ = dict.set_item("start", vm.ctx.new_int(m.start() as i64).into(), vm);
                    let _ = dict.set_item("end", vm.ctx.new_int(m.end() as i64).into(), vm);
                }

                // Capture groups (excluding full match)
                let groups: Vec<PyObjectRef> = caps.iter()
                    .skip(1)
                    .map(|g| {
                        match g {
                            Some(m) => vm.ctx.new_str(m.as_str().to_string()).into(),
                            None => vm.ctx.none(),
                        }
                    })
                    .collect();
                let _ = dict.set_item("groups", vm.ctx.new_list(groups).into(), vm);

                Ok(dict.into())
            }
            None => Ok(vm.ctx.none()),
        }
    }

    /// Find first match at the beginning of text only
    /// Returns dict with {match, start, end, groups} or None
    #[pyfunction]
    fn match_start(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        // Anchor pattern at start
        let anchored = format!("^(?:{})", pattern);
        search(anchored, text, flags, vm)
    }

    /// Find all non-overlapping matches
    /// Returns list of matched strings
    #[pyfunction]
    fn findall(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let matches: Vec<PyObjectRef> = re.find_iter(&text)
            .map(|m| vm.ctx.new_str(m.as_str().to_string()).into())
            .collect();

        Ok(vm.ctx.new_list(matches).into())
    }

    /// Find all matches with capture groups
    /// Returns list of dicts with {match, start, end, groups}
    #[pyfunction]
    fn findall_groups(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let mut results = vec![];
        for caps in re.captures_iter(&text) {
            let dict = PyDict::new_ref(&vm.ctx);

            if let Some(m) = caps.get(0) {
                let _ = dict.set_item("match", vm.ctx.new_str(m.as_str().to_string()).into(), vm);
                let _ = dict.set_item("start", vm.ctx.new_int(m.start() as i64).into(), vm);
                let _ = dict.set_item("end", vm.ctx.new_int(m.end() as i64).into(), vm);
            }

            let groups: Vec<PyObjectRef> = caps.iter()
                .skip(1)
                .map(|g| {
                    match g {
                        Some(m) => vm.ctx.new_str(m.as_str().to_string()).into(),
                        None => vm.ctx.none(),
                    }
                })
                .collect();
            let _ = dict.set_item("groups", vm.ctx.new_list(groups).into(), vm);

            results.push(dict.into());
        }

        Ok(vm.ctx.new_list(results).into())
    }

    /// Replace all matches with replacement string
    /// Supports backreferences: $1, $2, etc. or ${name}
    #[pyfunction]
    fn sub(pattern: String, replacement: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<String> {
        let re = build_regex(&pattern, flags, vm)?;
        Ok(re.replace_all(&text, replacement.as_str()).to_string())
    }

    /// Replace first n matches (0 = all)
    #[pyfunction]
    fn subn(pattern: String, replacement: String, text: String, count: i64, flags: Option<String>, vm: &VirtualMachine) -> PyResult<(String, i64)> {
        let re = build_regex(&pattern, flags, vm)?;

        if count <= 0 {
            let result = re.replace_all(&text, replacement.as_str()).to_string();
            let n = re.find_iter(&text).count() as i64;
            Ok((result, n))
        } else {
            let mut result = text.clone();
            let mut replaced = 0i64;

            for _ in 0..count {
                if let Some(m) = re.find(&result) {
                    let before = &result[..m.start()];
                    let after = &result[m.end()..];
                    result = format!("{}{}{}", before, replacement, after);
                    replaced += 1;
                } else {
                    break;
                }
            }

            Ok((result, replaced))
        }
    }

    /// Split text by pattern
    #[pyfunction]
    fn split(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let parts: Vec<PyObjectRef> = re.split(&text)
            .map(|s| vm.ctx.new_str(s.to_string()).into())
            .collect();

        Ok(vm.ctx.new_list(parts).into())
    }

    /// Split text by pattern, max n splits (0 = unlimited)
    #[pyfunction]
    fn splitn(pattern: String, text: String, max_splits: usize, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let parts: Vec<PyObjectRef> = if max_splits == 0 {
            re.split(&text)
                .map(|s| vm.ctx.new_str(s.to_string()).into())
                .collect()
        } else {
            re.splitn(&text, max_splits + 1)
                .map(|s| vm.ctx.new_str(s.to_string()).into())
                .collect()
        };

        Ok(vm.ctx.new_list(parts).into())
    }

    /// Escape special regex characters in text
    #[pyfunction]
    fn escape(text: String, _vm: &VirtualMachine) -> String {
        regex::escape(&text)
    }

    /// Count number of matches in text
    #[pyfunction]
    fn count(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<i64> {
        let re = build_regex(&pattern, flags, vm)?;
        Ok(re.find_iter(&text).count() as i64)
    }

    /// Get positions of all matches
    /// Returns list of (start, end) tuples
    #[pyfunction]
    fn finditer(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let positions: Vec<PyObjectRef> = re.find_iter(&text)
            .map(|m| {
                let tuple = vec![
                    vm.ctx.new_int(m.start() as i64).into(),
                    vm.ctx.new_int(m.end() as i64).into(),
                ];
                vm.ctx.new_tuple(tuple).into()
            })
            .collect();

        Ok(vm.ctx.new_list(positions).into())
    }

    /// Check if pattern matches the entire text
    #[pyfunction]
    fn fullmatch(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<bool> {
        // Anchor pattern at both ends
        let anchored = format!("^(?:{})$", pattern);
        let re = build_regex(&anchored, flags, vm)?;
        Ok(re.is_match(&text))
    }

    /// Extract named groups from match
    /// Pattern must use (?P<name>...) syntax
    /// Returns dict with group names as keys
    #[pyfunction]
    fn search_named(pattern: String, text: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        match re.captures(&text) {
            Some(caps) => {
                let dict = PyDict::new_ref(&vm.ctx);

                // Add full match info
                if let Some(m) = caps.get(0) {
                    let _ = dict.set_item("_match", vm.ctx.new_str(m.as_str().to_string()).into(), vm);
                    let _ = dict.set_item("_start", vm.ctx.new_int(m.start() as i64).into(), vm);
                    let _ = dict.set_item("_end", vm.ctx.new_int(m.end() as i64).into(), vm);
                }

                // Add named groups
                for name in re.capture_names().flatten() {
                    if let Some(m) = caps.name(name) {
                        let _ = dict.set_item(name, vm.ctx.new_str(m.as_str().to_string()).into(), vm);
                    } else {
                        let _ = dict.set_item(name, vm.ctx.none(), vm);
                    }
                }

                Ok(dict.into())
            }
            None => Ok(vm.ctx.none()),
        }
    }

    /// Get list of capture group names in pattern
    #[pyfunction]
    fn group_names(pattern: String, flags: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let re = build_regex(&pattern, flags, vm)?;

        let names: Vec<PyObjectRef> = re.capture_names()
            .filter_map(|n| n.map(|s| vm.ctx.new_str(s.to_string()).into()))
            .collect();

        Ok(vm.ctx.new_list(names).into())
    }

    /// Validate regex pattern without executing
    /// Returns None if valid, error message if invalid
    #[pyfunction]
    fn validate(pattern: String, flags: Option<String>, _vm: &VirtualMachine) -> Option<String> {
        let mut builder = RegexBuilder::new(&pattern);

        if let Some(f) = flags {
            for c in f.chars() {
                match c {
                    'i' => { builder.case_insensitive(true); }
                    'm' => { builder.multi_line(true); }
                    's' => { builder.dot_matches_new_line(true); }
                    'x' => { builder.ignore_whitespace(true); }
                    _ => {}
                }
            }
        }

        match builder.build() {
            Ok(_) => None,
            Err(e) => Some(format!("{}", e)),
        }
    }
}

/// fabiscos_archive - ZIP archive handling (using VFS)
#[pymodule]
mod fabiscos_archive {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;
    use crate::filesystem::{read_bytes_sync, write_sync, list_dir_sync, stat_sync};
    use std::io::{Read, Write, Cursor};
    use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

    /// List contents of a ZIP file in VFS
    #[pyfunction]
    fn list_zip(path: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let data = read_bytes_sync(&path)
            .ok_or_else(|| vm.new_runtime_error(format!("Cannot read ZIP file: {}", path)))?;

        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| vm.new_value_error(format!("Invalid ZIP file: {}", e)))?;

        let mut entries = vec![];
        for i in 0..archive.len() {
            let file = archive.by_index_raw(i)
                .map_err(|e| vm.new_value_error(format!("ZIP read error: {}", e)))?;

            let dict = PyDict::new_ref(&vm.ctx);
            let _ = dict.set_item("name", vm.ctx.new_str(file.name().to_string()).into(), vm);
            let _ = dict.set_item("size", vm.ctx.new_int(file.size() as i64).into(), vm);
            let _ = dict.set_item("compressed_size", vm.ctx.new_int(file.compressed_size() as i64).into(), vm);
            let _ = dict.set_item("is_dir", vm.ctx.new_bool(file.is_dir()).into(), vm);
            entries.push(dict.into());
        }
        Ok(vm.ctx.new_list(entries).into())
    }

    /// Extract a specific file from ZIP to bytes
    #[pyfunction]
    fn read_from_zip(zip_path: String, file_name: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        let data = read_bytes_sync(&zip_path)
            .ok_or_else(|| vm.new_runtime_error(format!("Cannot read ZIP file: {}", zip_path)))?;

        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| vm.new_value_error(format!("Invalid ZIP file: {}", e)))?;

        let mut file = archive.by_name(&file_name)
            .map_err(|e| vm.new_value_error(format!("File not found in ZIP: {} - {}", file_name, e)))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| vm.new_runtime_error(format!("Failed to read from ZIP: {}", e)))?;
        Ok(contents)
    }

    /// Extract all files from ZIP to a VFS directory
    #[pyfunction]
    fn unzip(zip_path: String, dest_dir: String, vm: &VirtualMachine) -> PyResult<i32> {
        let data = read_bytes_sync(&zip_path)
            .ok_or_else(|| vm.new_runtime_error(format!("Cannot read ZIP file: {}", zip_path)))?;

        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| vm.new_value_error(format!("Invalid ZIP file: {}", e)))?;

        let mut count = 0;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| vm.new_value_error(format!("ZIP read error: {}", e)))?;

            let name = file.name().to_string();
            let out_path = format!("{}/{}", dest_dir.trim_end_matches('/'), name);

            if file.is_dir() {
                // Create directory in VFS
                let _ = crate::filesystem::mkdir_p_sync(&out_path);
            } else {
                // Read file contents
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)
                    .map_err(|e| vm.new_runtime_error(format!("Failed to read from ZIP: {}", e)))?;

                // Ensure parent directory exists
                if let Some(parent) = crate::filesystem::parent(&out_path) {
                    let _ = crate::filesystem::mkdir_p_sync(&parent);
                }

                // Write to VFS
                write_sync(&out_path, &contents)
                    .map_err(|e| vm.new_runtime_error(format!("Failed to write file: {} - {}", out_path, e)))?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Create a ZIP file from VFS files/directories
    /// files: list of VFS paths to include
    #[pyfunction]
    fn zip(files: Vec<String>, zip_path: String, vm: &VirtualMachine) -> PyResult<()> {
        let mut buffer = Vec::new();
        {
            let cursor = Cursor::new(&mut buffer);
            let mut zip_writer = ZipWriter::new(cursor);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            fn add_to_zip(
                zip_writer: &mut ZipWriter<Cursor<&mut Vec<u8>>>,
                path: &str,
                base: &str,
                options: SimpleFileOptions,
            ) -> Result<(), String> {
                let stat = stat_sync(path).ok_or_else(|| format!("File not found: {}", path))?;

                let archive_name = if base.is_empty() {
                    crate::filesystem::file_name(path).unwrap_or_else(|| path.to_string())
                } else {
                    format!("{}/{}", base, crate::filesystem::file_name(path).unwrap_or_else(|| path.to_string()))
                };

                if stat.is_dir() {
                    // Add directory entry
                    zip_writer.add_directory(&format!("{}/", archive_name), options)
                        .map_err(|e| format!("Failed to add directory: {}", e))?;

                    // Add children
                    for entry in list_dir_sync(path) {
                        let child_path = format!("{}/{}", path.trim_end_matches('/'), entry.name);
                        add_to_zip(zip_writer, &child_path, &archive_name, options)?;
                    }
                } else {
                    // Add file
                    let data = read_bytes_sync(path).ok_or_else(|| format!("Cannot read: {}", path))?;
                    zip_writer.start_file(&archive_name, options)
                        .map_err(|e| format!("Failed to start file: {}", e))?;
                    zip_writer.write_all(&data)
                        .map_err(|e| format!("Failed to write: {}", e))?;
                }
                Ok(())
            }

            for file_path in &files {
                add_to_zip(&mut zip_writer, file_path, "", options)
                    .map_err(|e| vm.new_runtime_error(e))?;
            }

            zip_writer.finish()
                .map_err(|e| vm.new_runtime_error(format!("Failed to finalize ZIP: {}", e)))?;
        }

        // Write to VFS
        write_sync(&zip_path, &buffer)
            .map_err(|e| vm.new_runtime_error(format!("Failed to write ZIP file: {}", e)))?;
        Ok(())
    }
}

/// fabiscos_http - Simple HTTP client (fetch-based)
/// Note: This uses JavaScript fetch internally and is async in nature,
/// but exposed synchronously by blocking until the response is ready.
/// In WASM single-threaded environment, complex requests may need special handling.
#[pymodule]
mod fabiscos_http {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;
    use wasm_bindgen::prelude::*;

    // JavaScript interop for fetch (simplified sync wrapper)
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__http"], js_name = fetchSync)]
        fn js_fetch_sync(url: &str, method: &str, body: Option<String>, headers_json: &str) -> JsValue;
    }

    /// Perform GET request
    /// Returns dict with {status: int, ok: bool, text: str, headers: dict}
    #[pyfunction]
    fn get(url: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        fetch_internal(url, "GET".to_string(), None, None, vm)
    }

    /// Perform POST request with body
    #[pyfunction]
    fn post(url: String, body: String, content_type: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let headers = content_type.map(|ct| {
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), ct);
            h
        });
        fetch_internal(url, "POST".to_string(), Some(body), headers, vm)
    }

    /// Perform generic fetch request
    /// headers_json: JSON string of headers object, e.g. '{"Content-Type": "application/json"}'
    #[pyfunction]
    fn fetch(url: String, method: String, body: Option<String>, headers_json: Option<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let headers = headers_json.and_then(|json| {
            serde_json::from_str::<std::collections::HashMap<String, String>>(&json).ok()
        });
        fetch_internal(url, method, body, headers, vm)
    }

    fn fetch_internal(
        url: String,
        method: String,
        body: Option<String>,
        headers: Option<std::collections::HashMap<String, String>>,
        vm: &VirtualMachine,
    ) -> PyResult<PyObjectRef> {
        let headers_json = headers
            .map(|h| serde_json::to_string(&h).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());

        let result = js_fetch_sync(&url, &method, body, &headers_json);

        // Check if the JS function exists and returned something
        if result.is_undefined() || result.is_null() {
            return Err(vm.new_runtime_error("HTTP fetch not available. The __http.fetchSync function is not implemented.".to_string()));
        }

        // Parse result as JSON-like object
        let dict = PyDict::new_ref(&vm.ctx);

        // Try to extract fields from JS object
        if let Ok(obj) = result.dyn_into::<js_sys::Object>() {
            if let Ok(status) = js_sys::Reflect::get(&obj, &"status".into()) {
                if let Some(n) = status.as_f64() {
                    let _ = dict.set_item("status", vm.ctx.new_int(n as i64).into(), vm);
                }
            }
            if let Ok(ok) = js_sys::Reflect::get(&obj, &"ok".into()) {
                let _ = dict.set_item("ok", vm.ctx.new_bool(ok.is_truthy()).into(), vm);
            }
            if let Ok(text) = js_sys::Reflect::get(&obj, &"text".into()) {
                if let Some(s) = text.as_string() {
                    let _ = dict.set_item("text", vm.ctx.new_str(s).into(), vm);
                }
            }
            if let Ok(error) = js_sys::Reflect::get(&obj, &"error".into()) {
                if let Some(s) = error.as_string() {
                    let _ = dict.set_item("error", vm.ctx.new_str(s).into(), vm);
                }
            }
        }

        Ok(dict.into())
    }
}

/// fabiscos_notify - Notification API
/// Initially uses alert() as fallback, later TopBar integration
#[pymodule]
mod fabiscos_notify {
    use rustpython_vm::VirtualMachine;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window"], js_name = alert)]
        fn js_alert(message: &str);
    }

    /// Send a notification to the user (currently uses alert, later TopBar integration)
    #[pyfunction]
    fn notify(title: String, message: Option<String>, _vm: &VirtualMachine) {
        let full_message = match message {
            Some(msg) => format!("{}\n\n{}", title, msg),
            None => title,
        };
        js_alert(&full_message);
    }
}
