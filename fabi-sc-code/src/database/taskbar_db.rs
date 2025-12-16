//! TaskbarDB - Persistence for pinned taskbar apps
//!
//! Stores the user's pinned apps in IndexedDB.
//! Maximum 10 pinned apps allowed.
//!
//! Apps are stored by path (e.g., "/resources/apps/terminal/")
//! and metadata is loaded from {path}metadata.json

use super::IndexedDb;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

const DB_NAME: &str = "taskbar";
const STORE_NAME: &str = "pinned";
const PINNED_KEY: &str = "pinned_apps";
const ALL_APPS_KEY: &str = "all_apps";
const MAX_PINNED: usize = 10;

/// App metadata loaded from metadata.json
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
    pub icon: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub window: WindowConfig,
}

fn default_entry() -> String {
    "main.py".to_string()
}

/// Window configuration from metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_true")]
    pub resizable: bool,
    #[serde(default = "default_min_width")]
    pub min_width: u32,
    #[serde(default = "default_min_height")]
    pub min_height: u32,
}

fn default_width() -> u32 { 600 }
fn default_height() -> u32 { 400 }
fn default_true() -> bool { true }
fn default_min_width() -> u32 { 200 }
fn default_min_height() -> u32 { 150 }

/// A pinned app entry - stores path to app directory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinnedApp {
    /// App path (e.g., "/resources/apps/terminal/")
    pub path: String,
    /// Display order (0 = first)
    pub order: u32,
}

/// An available app entry - stores path to app directory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvailableApp {
    /// App path (e.g., "/resources/apps/terminal/")
    pub path: String,
}

/// TaskbarDB for managing pinned apps
pub struct TaskbarDb {
    db: IndexedDb,
}

impl TaskbarDb {
    /// Open the taskbar database
    pub async fn open() -> Result<Self, String> {
        let db = IndexedDb::open(DB_NAME, STORE_NAME)
            .await
            .map_err(|e| format!("Failed to open taskbar DB: {:?}", e))?;
        Ok(Self { db })
    }

    /// Get all pinned apps, sorted by order
    pub async fn get_pinned(&self) -> Vec<PinnedApp> {
        match self.db.get_item(PINNED_KEY).await {
            Ok(value) => {
                if let Ok(apps) = serde_wasm_bindgen::from_value::<Vec<PinnedApp>>(value) {
                    let mut apps = apps;
                    apps.sort_by_key(|a| a.order);
                    apps
                } else {
                    Self::default_pinned()
                }
            }
            Err(_) => Self::default_pinned(),
        }
    }

    /// Get all available apps (registered app paths)
    pub async fn get_all_apps(&self) -> Vec<AvailableApp> {
        match self.db.get_item(ALL_APPS_KEY).await {
            Ok(value) => {
                if let Ok(apps) = serde_wasm_bindgen::from_value::<Vec<AvailableApp>>(value) {
                    apps
                } else {
                    Self::default_apps()
                }
            }
            Err(_) => Self::default_apps(),
        }
    }

    /// Set all available apps
    pub async fn set_all_apps(&self, apps: Vec<AvailableApp>) -> Result<(), String> {
        let value = serde_wasm_bindgen::to_value(&apps)
            .map_err(|e| format!("Serialization error: {:?}", e))?;

        self.db
            .set_item(ALL_APPS_KEY, &value)
            .await
            .map_err(|e| format!("Failed to save all apps: {:?}", e))
    }

    /// Register an app path
    pub async fn register_app(&self, path: &str) -> Result<(), String> {
        let mut apps = self.get_all_apps().await;

        // Check if already registered
        if apps.iter().any(|a| a.path == path) {
            return Ok(());
        }

        apps.push(AvailableApp {
            path: path.to_string(),
        });

        self.set_all_apps(apps).await
    }

    /// Set pinned apps (replaces all)
    pub async fn set_pinned(&self, apps: Vec<PinnedApp>) -> Result<(), String> {
        // Enforce max limit
        let apps: Vec<PinnedApp> = apps.into_iter().take(MAX_PINNED).collect();

        let value = serde_wasm_bindgen::to_value(&apps)
            .map_err(|e| format!("Serialization error: {:?}", e))?;

        self.db
            .set_item(PINNED_KEY, &value)
            .await
            .map_err(|e| format!("Failed to save pinned apps: {:?}", e))
    }

    /// Pin an app by path (add to end)
    pub async fn pin_app(&self, app_path: &str) -> Result<(), String> {
        let mut apps = self.get_pinned().await;

        // Check if already pinned
        if apps.iter().any(|a| a.path == app_path) {
            return Ok(()); // Already pinned
        }

        // Check max limit
        if apps.len() >= MAX_PINNED {
            return Err(format!("Maximum {} apps can be pinned", MAX_PINNED));
        }

        // Add at end
        let order = apps.iter().map(|a| a.order).max().unwrap_or(0) + 1;
        apps.push(PinnedApp {
            path: app_path.to_string(),
            order,
        });

        self.set_pinned(apps).await
    }

    /// Unpin an app by path
    pub async fn unpin_app(&self, app_path: &str) -> Result<(), String> {
        let apps = self.get_pinned().await;
        let apps: Vec<PinnedApp> = apps.into_iter().filter(|a| a.path != app_path).collect();
        self.set_pinned(apps).await
    }

    /// Check if an app path is pinned
    pub async fn is_pinned(&self, app_path: &str) -> bool {
        let apps = self.get_pinned().await;
        apps.iter().any(|a| a.path == app_path)
    }

    /// Reorder pinned apps (move app to new position)
    pub async fn reorder(&self, app_path: &str, new_order: u32) -> Result<(), String> {
        let mut apps = self.get_pinned().await;

        // Find and update the app
        if let Some(app) = apps.iter_mut().find(|a| a.path == app_path) {
            app.order = new_order;
        }

        // Normalize orders
        apps.sort_by_key(|a| a.order);
        for (i, app) in apps.iter_mut().enumerate() {
            app.order = i as u32;
        }

        self.set_pinned(apps).await
    }

    /// Default pinned apps for new users
    fn default_pinned() -> Vec<PinnedApp> {
        vec![
            PinnedApp {
                path: "/resources/apps/terminal/".to_string(),
                order: 0,
            },
            PinnedApp {
                path: "/resources/apps/help/".to_string(),
                order: 1,
            },
        ]
    }

    /// Default available apps
    fn default_apps() -> Vec<AvailableApp> {
        // Only include apps that actually exist with metadata.json
        vec![
            AvailableApp { path: "/resources/apps/terminal/".to_string() },
            AvailableApp { path: "/resources/apps/help/".to_string() },
        ]
    }

    /// Ensure essential app (Help) is pinned for new/existing users
    pub async fn ensure_help_pinned(&self) -> Result<(), String> {
        let help_path = "/resources/apps/help/";
        let mut pinned = self.get_pinned().await;

        if !pinned.iter().any(|p| p.path == help_path) {
            let order = pinned.iter().map(|a| a.order).max().unwrap_or(0) + 1;
            pinned.push(PinnedApp {
                path: help_path.to_string(),
                order,
            });
            self.set_pinned(pinned).await?;
            web_sys::console::log_1(&"[Taskbar] Auto-pinned Help app".into());
        }

        Ok(())
    }
}

/// Fetch app metadata from a path
pub async fn fetch_app_metadata(app_path: &str) -> Result<AppMetadata, String> {
    let url = format!("{}metadata.json", app_path);

    let window = web_sys::window().ok_or("No window")?;
    let promise = window.fetch_with_str(&url);
    let response = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| "Not a Response")?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let json_promise = response.json().map_err(|e| format!("JSON error: {:?}", e))?;
    let json = JsFuture::from(json_promise)
        .await
        .map_err(|e| format!("JSON parse failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Deserialization failed: {:?}", e))
}
