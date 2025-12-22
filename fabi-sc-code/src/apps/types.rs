//! App-related type definitions

use serde::{Deserialize, Serialize};

/// Window configuration from metadata.json
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default)]
    pub resizable: bool,
    #[serde(default = "default_min_width")]
    pub min_width: u32,
    #[serde(default = "default_min_height")]
    pub min_height: u32,
    /// Maximum width (0 = unlimited)
    #[serde(default)]
    pub max_width: u32,
    /// Maximum height (0 = unlimited)
    #[serde(default)]
    pub max_height: u32,
}

fn default_width() -> u32 { 400 }
fn default_height() -> u32 { 300 }
fn default_min_width() -> u32 { 200 }
fn default_min_height() -> u32 { 150 }

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            resizable: true,
            min_width: default_min_width(),
            min_height: default_min_height(),
            max_width: 0,
            max_height: 0,
        }
    }
}

/// App metadata from metadata.json
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub window: WindowConfig,
}

fn default_icon() -> String { "icon.png".to_string() }
fn default_entry() -> String { "main.py".to_string() }

/// Full app information including runtime data
#[derive(Debug, Clone, PartialEq)]
pub struct AppInfo {
    /// App metadata from metadata.json
    pub metadata: AppMetadata,
    /// Full path to app directory
    pub path: String,
    /// Whether this is a system app (protected)
    pub is_system: bool,
    /// FontAwesome icon class (derived from metadata or default)
    pub icon_class: String,
}

impl AppInfo {
    /// Create app info from metadata
    pub fn new(metadata: AppMetadata, path: String, is_system: bool) -> Self {
        // Derive icon class from app ID
        let icon_class = get_icon_class(&metadata.id);
        Self {
            metadata,
            path,
            is_system,
            icon_class,
        }
    }

    /// Get the entry point path
    pub fn entry_path(&self) -> String {
        format!("{}/{}", self.path, self.metadata.entry)
    }

    /// Get the icon path
    pub fn icon_path(&self) -> String {
        format!("{}/{}", self.path, self.metadata.icon)
    }
}

/// Get a FontAwesome icon class for an app ID
/// Falls back to a generic icon if no specific mapping exists
fn get_icon_class(app_id: &str) -> String {
    match app_id {
        "terminal" => "fa-solid fa-terminal".to_string(),
        "files" => "fa-solid fa-folder".to_string(),
        "browser" => "fa-solid fa-globe".to_string(),
        "settings" => "fa-solid fa-gear".to_string(),
        "gallery" => "fa-solid fa-images".to_string(),
        "music" => "fa-solid fa-music".to_string(),
        "contacts" => "fa-solid fa-address-book".to_string(),
        "info" | "about" => "fa-solid fa-circle-info".to_string(),
        _ => "fa-solid fa-cube".to_string(),
    }
}

/// Error types for app operations
#[derive(Debug, Clone)]
pub enum AppError {
    /// App not found at the given path
    NotFound(String),
    /// Invalid metadata.json
    InvalidMetadata(String),
    /// Cannot uninstall system app
    SystemAppProtected(String),
    /// VFS error during operation
    VfsError(String),
    /// Python execution error
    PythonError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(path) => write!(f, "App not found: {}", path),
            AppError::InvalidMetadata(msg) => write!(f, "Invalid metadata: {}", msg),
            AppError::SystemAppProtected(id) => write!(f, "Cannot uninstall system app: {}", id),
            AppError::VfsError(msg) => write!(f, "Filesystem error: {}", msg),
            AppError::PythonError(msg) => write!(f, "Python error: {}", msg),
        }
    }
}
