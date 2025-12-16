//! Python API modules for FabiScOS
//!
//! These modules are exposed to Python apps as the `fabiscos` package:
//! - `fabiscos.vfs` - Virtual filesystem access
//! - `fabiscos.ui` - UI widget creation
//! - `fabiscos.window` - Window control
//! - `fabiscos.system` - System information

pub mod ui;
pub mod vfs;
pub mod window;
pub mod system;
