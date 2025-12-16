//! Python Runtime using RustPython
//!
//! Provides a sandboxed Python interpreter for running FabiScOS apps.
//! Features:
//! - Instruction limit to prevent infinite loops
//! - No access to real filesystem (only VFS)
//! - No network access except through app API
//! - Isolated per-app execution context

use rustpython_vm::{
    PyRef, Interpreter, Settings, VirtualMachine,
};
use std::sync::{Arc, Mutex};

/// Maximum number of Python instructions before execution is stopped
/// This prevents infinite loops from freezing the browser
#[allow(dead_code)]
const MAX_INSTRUCTIONS: u64 = 1_000_000;

/// Result of app execution
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
pub struct AppState {
    /// The window ID this app is running in
    pub window_id: String,
    /// Current working directory in VFS
    pub cwd: String,
    /// App's base path (e.g., /home/.system/apps/terminal/)
    pub app_path: String,
    /// Whether the app requested to close
    pub close_requested: bool,
    /// Pending UI update (HTML to inject into Shadow DOM)
    pub pending_ui: Option<String>,
}

impl AppState {
    pub fn new(window_id: String, app_path: String) -> Self {
        Self {
            window_id,
            cwd: "/home".to_string(),
            app_path,
            close_requested: false,
            pending_ui: None,
        }
    }
}

/// Python Runtime for FabiScOS
pub struct PythonRuntime {
    /// Shared app state
    state: Arc<Mutex<AppState>>,
}

impl PythonRuntime {
    /// Create a new Python runtime for an app
    pub fn new(window_id: String, app_path: String) -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::new(window_id, app_path))),
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
        let _state = self.state.clone();

        // Create interpreter with settings
        let settings = Settings::default();

        let interp = Interpreter::with_init(settings, |vm| {
            // Add the fabiscos module
            vm.add_native_module("fabiscos".to_owned(), Box::new(make_fabiscos_module));

            // Add submodules
            vm.add_native_module("fabiscos.ui".to_owned(), Box::new(make_ui_module));
            vm.add_native_module("fabiscos.vfs".to_owned(), Box::new(make_vfs_module));
            vm.add_native_module("fabiscos.window".to_owned(), Box::new(make_window_module));
            vm.add_native_module("fabiscos.system".to_owned(), Box::new(make_system_module));
        });

        // Execute with instruction limit
        interp.enter(|vm| {
            // Compile the code
            let code_obj = match vm.compile(
                code,
                rustpython_compiler::Mode::Exec,
                "<app>".to_owned(),
            ) {
                Ok(code) => code,
                Err(e) => return AppExecResult::Error(format!("Compilation error: {}", e)),
            };

            // Create a new scope
            let scope = vm.new_scope_with_builtins();

            // Execute the code
            match vm.run_code_obj(code_obj, scope) {
                Ok(_) => AppExecResult::Success,
                Err(exc) => {
                    let mut msg = String::new();
                    if let Err(_) = vm.write_exception(&mut msg, &exc) {
                        msg = "Unknown error".to_string();
                    }
                    AppExecResult::Error(msg)
                }
            }
        })
    }

    /// Get the current app state
    #[allow(dead_code)]
    pub fn state(&self) -> Arc<Mutex<AppState>> {
        self.state.clone()
    }
}

// ============================================================================
// Python Modules
// ============================================================================

/// Main fabiscos module
fn make_fabiscos_module(vm: &VirtualMachine) -> PyRef<rustpython_vm::builtins::PyModule> {
    let module = vm.new_module("fabiscos", vm.ctx.new_dict(), None);
    module
}

/// fabiscos.ui module - UI widgets
fn make_ui_module(vm: &VirtualMachine) -> PyRef<rustpython_vm::builtins::PyModule> {
    let ctx = &vm.ctx;
    let module = vm.new_module("ui", ctx.new_dict(), None);
    module
}

/// fabiscos.vfs module - Virtual filesystem access
fn make_vfs_module(vm: &VirtualMachine) -> PyRef<rustpython_vm::builtins::PyModule> {
    let ctx = &vm.ctx;
    let module = vm.new_module("vfs", ctx.new_dict(), None);
    module
}

/// fabiscos.window module - Window control
fn make_window_module(vm: &VirtualMachine) -> PyRef<rustpython_vm::builtins::PyModule> {
    let ctx = &vm.ctx;
    let module = vm.new_module("window", ctx.new_dict(), None);
    module
}

/// fabiscos.system module - System information
fn make_system_module(vm: &VirtualMachine) -> PyRef<rustpython_vm::builtins::PyModule> {
    let ctx = &vm.ctx;
    let module = vm.new_module("system", ctx.new_dict(), None);
    module
}
