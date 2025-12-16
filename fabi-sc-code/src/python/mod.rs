//! Python Runtime for FabiScOS Apps
//!
//! This module provides a sandboxed Python runtime using RustPython.
//! All apps (System + User) run in this runtime with access to the
//! `fabiscos` API module (vfs, ui, window, system).

pub mod runtime;
pub mod api;
pub mod sanitizer;

pub use runtime::PythonRuntime;
