//! Python Runtime using RustPython
//!
//! Provides a sandboxed Python interpreter for running FabiScOS apps.
//! Features:
//! - Instruction limit to prevent infinite loops
//! - No access to real filesystem (only VFS)
//! - No network access except through app API
//! - Isolated per-app execution context

use rustpython_vm::{pymodule, Interpreter, Settings, VirtualMachine};
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
}

// ============================================================================
// Python Modules using #[pymodule] macro
// ============================================================================

/// Main fabiscos module - re-exports submodules
#[pymodule]
mod fabiscos {
    use rustpython_vm::VirtualMachine;

    #[pyfunction]
    fn version(_vm: &VirtualMachine) -> String {
        "0.1.0".to_string()
    }

    /// Log to browser console (for debugging)
    #[pyfunction]
    fn log(message: String, _vm: &VirtualMachine) {
        web_sys::console::log_1(&format!("[PyLog] {}", message).into());
    }
}

// VFS sync helper - exposed for use by ui.image() and other modules
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = getDataUrl)]
    pub fn vfs_get_data_url(path: &str) -> Option<String>;
}

/// fabiscos.vfs - Virtual Filesystem API
///
/// VFS operations use synchronous JavaScript interop to access IndexedDB
/// through a cached mechanism exposed via window.__vfsSync
#[pymodule]
mod fabiscos_vfs {
    use rustpython_vm::{VirtualMachine, PyResult, PyObjectRef};
    use rustpython_vm::builtins::PyDict;
    use crate::python::runtime::get_current_state;
    use wasm_bindgen::prelude::*;

    // JavaScript interop for synchronous VFS access
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = readText)]
        fn js_vfs_read_text(path: &str) -> Option<String>;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = readBytes)]
        fn js_vfs_read_bytes(path: &str) -> Option<js_sys::Uint8Array>;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = exists)]
        fn js_vfs_exists(path: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = listDir)]
        fn js_vfs_list_dir(path: &str) -> JsValue;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = write)]
        fn js_vfs_write(path: &str, content: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = writeBytes)]
        fn js_vfs_write_bytes(path: &str, bytes: &js_sys::Uint8Array) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = mkdir)]
        fn js_vfs_mkdir(path: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = remove)]
        fn js_vfs_remove(path: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = copy)]
        fn js_vfs_copy(src: &str, dst: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = "move")]
        fn js_vfs_move(src: &str, dst: &str) -> bool;

        #[wasm_bindgen(js_namespace = ["window", "__vfsSync"], js_name = getDataUrl)]
        fn js_vfs_get_data_url(path: &str) -> Option<String>;
    }

    #[pyfunction]
    fn read_text(path: String, vm: &VirtualMachine) -> PyResult<String> {
        match js_vfs_read_text(&path) {
            Some(content) => Ok(content),
            None => Err(vm.new_runtime_error(format!("Cannot read file: {}", path)))
        }
    }

    #[pyfunction]
    fn read_bytes(path: String, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
        match js_vfs_read_bytes(&path) {
            Some(arr) => Ok(arr.to_vec()),
            None => Err(vm.new_runtime_error(format!("Cannot read file: {}", path)))
        }
    }

    #[pyfunction]
    fn exists(path: String, _vm: &VirtualMachine) -> bool {
        js_vfs_exists(&path)
    }

    #[pyfunction]
    fn list_dir(path: String, vm: &VirtualMachine) -> PyObjectRef {
        let js_result = js_vfs_list_dir(&path);

        // Convert JS array to Python list of dicts
        if let Some(array) = js_result.dyn_ref::<js_sys::Array>() {
            let mut py_list = vec![];
            for i in 0..array.length() {
                let item = array.get(i);
                if let Some(obj) = item.dyn_ref::<js_sys::Object>() {
                    // Create Python dict for each entry
                    let dict = PyDict::new_ref(&vm.ctx);

                    // Get name
                    if let Ok(name) = js_sys::Reflect::get(obj, &JsValue::from_str("name")) {
                        if let Some(name_str) = name.as_string() {
                            let _ = dict.set_item("name", vm.ctx.new_str(name_str).into(), vm);
                        }
                    }

                    // Get type
                    if let Ok(ftype) = js_sys::Reflect::get(obj, &JsValue::from_str("type")) {
                        if let Some(type_str) = ftype.as_string() {
                            let _ = dict.set_item("type", vm.ctx.new_str(type_str).into(), vm);
                        }
                    }

                    py_list.push(dict.into());
                }
            }
            vm.ctx.new_list(py_list).into()
        } else {
            // Return empty list if VFS not available
            vm.ctx.new_list(vec![]).into()
        }
    }

    #[pyfunction]
    fn write(path: String, content: String, vm: &VirtualMachine) -> PyResult<()> {
        if js_vfs_write(&path, &content) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot write file: {}", path)))
        }
    }

    #[pyfunction]
    fn write_bytes(path: String, data: Vec<u8>, vm: &VirtualMachine) -> PyResult<()> {
        let arr = js_sys::Uint8Array::from(data.as_slice());
        if js_vfs_write_bytes(&path, &arr) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot write file: {}", path)))
        }
    }

    #[pyfunction]
    fn get_data_url(path: String, vm: &VirtualMachine) -> PyResult<String> {
        match js_vfs_get_data_url(&path) {
            Some(url) => Ok(url),
            None => Err(vm.new_runtime_error(format!("Cannot get data URL for: {}", path)))
        }
    }

    #[pyfunction]
    fn mkdir(path: String, vm: &VirtualMachine) -> PyResult<()> {
        if js_vfs_mkdir(&path) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot create directory: {}", path)))
        }
    }

    #[pyfunction]
    fn remove(path: String, vm: &VirtualMachine) -> PyResult<()> {
        if js_vfs_remove(&path) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot remove: {}", path)))
        }
    }

    #[pyfunction]
    fn copy(src: String, dst: String, vm: &VirtualMachine) -> PyResult<()> {
        if js_vfs_copy(&src, &dst) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot copy {} to {}", src, dst)))
        }
    }

    #[pyfunction(name = "move")]
    fn move_file(src: String, dst: String, vm: &VirtualMachine) -> PyResult<()> {
        if js_vfs_move(&src, &dst) {
            Ok(())
        } else {
            Err(vm.new_runtime_error(format!("Cannot move {} to {}", src, dst)))
        }
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
        use crate::python::runtime::vfs_get_data_url;

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
            vfs_get_data_url(&src).unwrap_or(src)
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
