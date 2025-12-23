//! VFS Events - Notification system for filesystem changes
//!
//! Uses a simple "dirty flag" pattern for /home/apps/ changes.
//! When apps directory is modified, a flag is set.
//! The Start Menu checks this flag when opening and refreshes if needed.

use std::cell::RefCell;

thread_local! {
    /// Flag indicating /home/apps/ has changed since last check
    static APPS_DIRTY: RefCell<bool> = RefCell::new(false);
}

/// Check if apps directory has changed and reset the flag
/// Returns true if apps were modified since last check
pub fn check_and_clear_apps_dirty() -> bool {
    APPS_DIRTY.with(|dirty| {
        let was_dirty = *dirty.borrow();
        *dirty.borrow_mut() = false;
        was_dirty
    })
}

/// Mark apps directory as changed
/// Called internally by VFS write/delete operations
pub(crate) fn notify_apps_changed() {
    APPS_DIRTY.with(|dirty| {
        *dirty.borrow_mut() = true;
    });
    web_sys::console::log_1(&"[VFS] Apps directory changed".into());
}

/// Check if a path is within /home/apps/ directory
pub(crate) fn is_apps_path(path: &str) -> bool {
    path.starts_with("/home/apps/") || path == "/home/apps"
}
