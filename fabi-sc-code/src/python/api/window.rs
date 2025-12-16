//! Window control API for Python apps
//!
//! Allows apps to control their window (title, content, close).

/// Window control commands that apps can issue
pub enum WindowCommand {
    /// Set the window title
    SetTitle(String),
    /// Set the window content (HTML to inject into Shadow DOM)
    SetContent(String),
    /// Request to close the window
    Close,
}
