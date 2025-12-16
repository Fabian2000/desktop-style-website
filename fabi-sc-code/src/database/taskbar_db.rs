//! TaskbarDB - Persistence for pinned taskbar apps
//!
//! Stores the user's pinned apps in IndexedDB.
//! Maximum 10 pinned apps allowed.

use super::IndexedDb;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

const DB_NAME: &str = "taskbar";
const STORE_NAME: &str = "pinned";
const PINNED_KEY: &str = "pinned_apps";
const MAX_PINNED: usize = 10;

/// A pinned app entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinnedApp {
    /// App ID (e.g., "terminal", "files")
    pub id: String,
    /// Display order (0 = first)
    pub order: u32,
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

    /// Pin an app (add to end)
    pub async fn pin_app(&self, app_id: &str) -> Result<(), String> {
        let mut apps = self.get_pinned().await;

        // Check if already pinned
        if apps.iter().any(|a| a.id == app_id) {
            return Ok(()); // Already pinned
        }

        // Check max limit
        if apps.len() >= MAX_PINNED {
            return Err(format!("Maximum {} apps can be pinned", MAX_PINNED));
        }

        // Add at end
        let order = apps.iter().map(|a| a.order).max().unwrap_or(0) + 1;
        apps.push(PinnedApp {
            id: app_id.to_string(),
            order,
        });

        self.set_pinned(apps).await
    }

    /// Unpin an app
    pub async fn unpin_app(&self, app_id: &str) -> Result<(), String> {
        let apps = self.get_pinned().await;
        let apps: Vec<PinnedApp> = apps.into_iter().filter(|a| a.id != app_id).collect();
        self.set_pinned(apps).await
    }

    /// Check if an app is pinned
    pub async fn is_pinned(&self, app_id: &str) -> bool {
        let apps = self.get_pinned().await;
        apps.iter().any(|a| a.id == app_id)
    }

    /// Reorder pinned apps (move app to new position)
    pub async fn reorder(&self, app_id: &str, new_order: u32) -> Result<(), String> {
        let mut apps = self.get_pinned().await;

        // Find and update the app
        if let Some(app) = apps.iter_mut().find(|a| a.id == app_id) {
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
                id: "files".to_string(),
                order: 0,
            },
            PinnedApp {
                id: "browser".to_string(),
                order: 1,
            },
            PinnedApp {
                id: "terminal".to_string(),
                order: 2,
            },
            PinnedApp {
                id: "settings".to_string(),
                order: 3,
            },
        ]
    }
}
