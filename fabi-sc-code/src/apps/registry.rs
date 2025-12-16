//! App Registry - Discovery, Installation, and Management
//!
//! Handles:
//! - Discovering apps from /home/.system/apps/ (System) and /home/apps/ (User)
//! - Installing new user apps
//! - Uninstalling user apps (system apps are protected)
//! - Loading app metadata

use super::types::{AppError, AppInfo, AppMetadata};
use crate::filesystem::{self as vfs};

/// System apps directory
pub const SYSTEM_APPS_PATH: &str = "/home/.system/apps";
/// User apps directory
pub const USER_APPS_PATH: &str = "/home/apps";

/// Discover all installed apps
///
/// Scans both system and user app directories for valid apps.
/// Returns a list of all found apps with their metadata.
pub async fn discover_apps() -> Result<Vec<AppInfo>, AppError> {
    let mut apps = Vec::new();

    // Discover system apps
    if let Ok(entries) = vfs::read_dir(SYSTEM_APPS_PATH).await {
        for entry in entries {
            if entry.is_dir() {
                let app_path = format!("{}/{}", SYSTEM_APPS_PATH, entry.name);
                if let Ok(app) = load_app(&app_path, true).await {
                    apps.push(app);
                }
            }
        }
    }

    // Discover user apps
    if let Ok(entries) = vfs::read_dir(USER_APPS_PATH).await {
        for entry in entries {
            if entry.is_dir() {
                let app_path = format!("{}/{}", USER_APPS_PATH, entry.name);
                if let Ok(app) = load_app(&app_path, false).await {
                    apps.push(app);
                }
            }
        }
    }

    Ok(apps)
}

/// Load app metadata from a directory
pub async fn load_app(path: &str, is_system: bool) -> Result<AppInfo, AppError> {
    let metadata_path = format!("{}/metadata.json", path);

    // Read metadata.json
    let metadata_content = vfs::read_to_string(&metadata_path)
        .await
        .map_err(|e| AppError::NotFound(format!("{}: {}", path, e)))?;

    // Parse metadata
    let metadata: AppMetadata = serde_json::from_str(&metadata_content)
        .map_err(|e| AppError::InvalidMetadata(e.to_string()))?;

    Ok(AppInfo::new(metadata, path.to_string(), is_system))
}

/// Get a specific app by ID
pub async fn get_app(app_id: &str) -> Result<AppInfo, AppError> {
    // Check system apps first
    let system_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
    if let Ok(app) = load_app(&system_path, true).await {
        return Ok(app);
    }

    // Check user apps
    let user_path = format!("{}/{}", USER_APPS_PATH, app_id);
    if let Ok(app) = load_app(&user_path, false).await {
        return Ok(app);
    }

    Err(AppError::NotFound(app_id.to_string()))
}

/// Install a new user app from a directory
///
/// The source directory must contain a valid metadata.json.
/// The app will be copied to /home/apps/{id}/
pub async fn install_app(source_path: &str) -> Result<AppInfo, AppError> {
    // Load metadata from source
    let metadata_path = format!("{}/metadata.json", source_path);
    let metadata_content = vfs::read_to_string(&metadata_path)
        .await
        .map_err(|e| AppError::VfsError(e.to_string()))?;

    let metadata: AppMetadata = serde_json::from_str(&metadata_content)
        .map_err(|e| AppError::InvalidMetadata(e.to_string()))?;

    // Target path
    let target_path = format!("{}/{}", USER_APPS_PATH, metadata.id);

    // Create app directory
    vfs::create_dir_all(&target_path)
        .await
        .map_err(|e| AppError::VfsError(e.to_string()))?;

    // Copy all files from source to target
    if let Ok(entries) = vfs::read_dir(source_path).await {
        for entry in entries {
            let src = format!("{}/{}", source_path, entry.name);
            let dst = format!("{}/{}", target_path, entry.name);

            if entry.is_file() {
                // Copy file
                if let Ok(content) = vfs::read_file(&src).await {
                    let _ = vfs::write_file(&dst, &content).await;
                }
            }
            // Note: Recursive directory copy not implemented yet
        }
    }

    Ok(AppInfo::new(metadata, target_path, false))
}

/// Uninstall a user app
///
/// System apps cannot be uninstalled.
pub async fn uninstall_app(app_id: &str) -> Result<(), AppError> {
    // Check if it's a system app
    let system_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
    if vfs::exists(&system_path).await.unwrap_or(false) {
        return Err(AppError::SystemAppProtected(app_id.to_string()));
    }

    // Remove user app
    let user_path = format!("{}/{}", USER_APPS_PATH, app_id);
    if !vfs::exists(&user_path).await.unwrap_or(false) {
        return Err(AppError::NotFound(app_id.to_string()));
    }

    vfs::remove_dir_all(&user_path)
        .await
        .map_err(|e| AppError::VfsError(e.to_string()))?;

    Ok(())
}

/// Check if an app is installed
pub async fn is_installed(app_id: &str) -> bool {
    let system_path = format!("{}/{}", SYSTEM_APPS_PATH, app_id);
    let user_path = format!("{}/{}", USER_APPS_PATH, app_id);

    vfs::exists(&system_path).await.unwrap_or(false)
        || vfs::exists(&user_path).await.unwrap_or(false)
}
