//! App System for FabiScOS
//!
//! All apps (System + User) are written in Python and run in the RustPython runtime.
//! - System-Apps: Located in `/home/.system/apps/`, protected from uninstallation
//! - User-Apps: Located in `/home/apps/`, can be uninstalled

pub mod registry;
pub mod types;

pub use registry::*;
pub use types::*;
