//! TaskbarDB - Persistence for pinned taskbar apps
//!
//! Stores the user's pinned apps in IndexedDB.
//! Maximum 10 pinned apps allowed.
//!
//! Apps are stored by VFS path (e.g., "/home/.system/apps/terminal/")
//! and metadata is loaded from {path}metadata.json via VFS

use super::IndexedDb;
use crate::filesystem;
use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_true")]
    pub pinned: bool,
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub async fn is_pinned(&self, app_path: &str) -> bool {
        let apps = self.get_pinned().await;
        apps.iter().any(|a| a.path == app_path)
    }

    /// Reorder pinned apps (move app to new position)
    #[allow(dead_code)]
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

    /// Default pinned apps for new users - empty, apps are discovered from VFS
    fn default_pinned() -> Vec<PinnedApp> {
        vec![]
    }

    /// Default available apps - empty, apps are discovered from VFS
    fn default_apps() -> Vec<AvailableApp> {
        vec![]
    }

    /// Discover and register all apps from VFS
    /// Scans both /home/.system/apps/ (system) and /home/apps/ (user) directories
    /// Returns the number of apps found, or an error if VFS is not ready
    pub async fn discover_apps(&self) -> Result<usize, String> {
        let system_apps_dir = "/home/.system/apps/";
        let user_apps_dir = "/home/apps/";

        let mut available_apps = Vec::new();
        let mut pinned_apps = self.get_pinned().await;
        let mut order = pinned_apps.iter().map(|a| a.order).max().unwrap_or(0);

        // Helper to scan a directory for apps
        async fn scan_apps_dir(
            apps_dir: &str,
            available_apps: &mut Vec<AvailableApp>,
            pinned_apps: &mut Vec<PinnedApp>,
            order: &mut u32,
        ) -> Result<(), String> {
            if let Ok(entries) = filesystem::vfs::read_dir(apps_dir).await {
                for entry in entries {
                    if entry.is_dir() {
                        let app_path = format!("{}{}/", apps_dir, entry.name);
                        let metadata_path = format!("{}metadata.json", app_path);

                        // Check if metadata.json exists and parse it
                        if let Ok(json) = filesystem::vfs::read_to_string(&metadata_path).await {
                            // Add to available apps (all apps are launchable)
                            available_apps.push(AvailableApp {
                                path: app_path.clone(),
                            });

                            // Check if app should be pinned
                            // For system apps: respect pinned field, default to true
                            // For user apps: NEVER auto-pin (user must pin manually)
                            let is_system_app = apps_dir.contains(".system");
                            let should_pin = if is_system_app {
                                // System apps: use pinned field from metadata, default true
                                serde_json::from_str::<AppMetadata>(&json)
                                    .map(|meta| meta.pinned)
                                    .unwrap_or(true)
                            } else {
                                // User apps: never auto-pin, must be pinned manually
                                false
                            };

                            if should_pin {
                                // Auto-pin if pinned=true and not already pinned
                                if !pinned_apps.iter().any(|p| p.path == app_path) {
                                    *order += 1;
                                    pinned_apps.push(PinnedApp {
                                        path: app_path,
                                        order: *order,
                                    });
                                }
                            } else if is_system_app {
                                // Only system apps can force-unpin via metadata
                                // User apps keep their manual pin status
                                pinned_apps.retain(|p| p.path != app_path);
                            }
                            // User apps: do nothing here - respect user's manual pin choice
                        }
                    }
                }
            }
            Ok(())
        }

        // Scan system apps first
        match filesystem::vfs::read_dir(system_apps_dir).await {
            Ok(_) => {
                scan_apps_dir(system_apps_dir, &mut available_apps, &mut pinned_apps, &mut order).await?;
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("[Taskbar] Could not read system apps directory: {:?}", e).into());
                return Err(format!("VFS not ready: {:?}", e));
            }
        }

        // Scan user apps (ignore errors - directory may not exist)
        let _ = scan_apps_dir(user_apps_dir, &mut available_apps, &mut pinned_apps, &mut order).await;

        // Clean up pins for apps that no longer exist
        let available_paths: std::collections::HashSet<_> = available_apps.iter().map(|a| a.path.as_str()).collect();
        pinned_apps.retain(|p| available_paths.contains(p.path.as_str()));

        let app_count = available_apps.len();

        // Save discovered apps
        self.set_all_apps(available_apps).await?;
        self.set_pinned(pinned_apps).await?;

        web_sys::console::log_1(&format!("[Taskbar] Discovered {} apps from VFS", app_count).into());
        Ok(app_count)
    }
}

/// Fetch app metadata from VFS path
pub async fn fetch_app_metadata(app_path: &str) -> Result<AppMetadata, String> {
    let metadata_path = format!("{}metadata.json", app_path);

    let json = filesystem::vfs::read_to_string(&metadata_path)
        .await
        .map_err(|e| format!("Failed to read {}: {:?}", metadata_path, e))?;

    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse metadata: {:?}", e))
}
