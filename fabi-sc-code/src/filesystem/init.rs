//! Filesystem initialization and system file synchronization

use crate::filesystem::cache::with_cache_mut;
use crate::filesystem::path;
use crate::filesystem::storage::FsStorage;
use crate::filesystem::types::{Permissions, VfsError};
use crate::filesystem::vfs;
use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// System manifest structure (loaded from server)
#[derive(Debug, Deserialize)]
pub struct SystemManifest {
    pub version: String,
    pub files: Vec<SystemFile>,
}

/// A system file entry in the manifest
#[derive(Debug, Deserialize)]
pub struct SystemFile {
    /// Virtual filesystem path (e.g., "/home/Documents/Portfolio.pdf")
    pub path: String,
    /// Server path to fetch from (e.g., "/resources/files/portfolio.pdf")
    pub server_path: String,
    /// Hash of the file content for change detection
    pub hash: String,
    /// File permissions
    #[serde(default)]
    pub permissions: SystemPermissions,
}

#[derive(Debug, Default, Deserialize)]
pub struct SystemPermissions {
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub system: bool,
    #[serde(default)]
    pub hidden: bool,
}

impl From<SystemPermissions> for Permissions {
    fn from(sp: SystemPermissions) -> Self {
        Permissions {
            readonly: sp.readonly,
            system: sp.system,
            hidden: sp.hidden,
        }
    }
}

/// Initialize the virtual filesystem
/// Called during boot sequence
pub async fn initialize() -> Result<InitResult, VfsError> {
    let mut result = InitResult::default();

    // Initialize storage
    vfs::init_storage().await?;

    // Check if filesystem is already initialized
    let needs_setup = !vfs::is_initialized().await?;

    if needs_setup {
        // First time setup - create directory structure
        create_initial_structure().await?;
        result.created_structure = true;
    }

    // Sync system files from server
    let sync_result = sync_system_files().await?;
    result.files_updated = sync_result.updated;
    result.files_added = sync_result.added;

    // Clean up old trash files
    result.trash_cleaned = vfs::cleanup_trash().await?;

    // Populate the in-memory cache from IndexedDB
    populate_cache().await?;

    Ok(result)
}

/// Populate the in-memory VFS cache from IndexedDB
/// This loads all nodes and file contents into memory for synchronous access
async fn populate_cache() -> Result<(), VfsError> {
    web_sys::console::log_1(&"[VFS] Populating in-memory cache...".into());

    // Open storage directly for loading
    let storage = FsStorage::open().await?;
    let storage = Rc::new(storage);

    // Get all nodes with prefix /home (everything)
    let all_nodes = storage.get_nodes_with_prefix("/home").await?;
    let node_count = all_nodes.len();

    // Load all nodes into cache
    with_cache_mut(|cache| {
        cache.set_storage(storage.clone());

        for node in &all_nodes {
            cache.load_node(node.clone());
        }
    });

    // Load file contents
    let mut content_count = 0;
    for node in &all_nodes {
        if node.is_file() {
            if let Ok(Some(content)) = storage.get_content(&node.path).await {
                with_cache_mut(|cache| {
                    cache.load_content(&node.path, content);
                });
                content_count += 1;
            }
        }
    }

    // Mark cache as initialized
    with_cache_mut(|cache| {
        cache.set_initialized();
    });

    web_sys::console::log_1(
        &format!(
            "[VFS] Cache populated: {} nodes, {} file contents",
            node_count, content_count
        )
        .into(),
    );

    Ok(())
}

/// Result of filesystem initialization
#[derive(Debug, Default)]
pub struct InitResult {
    /// Whether the initial directory structure was created
    pub created_structure: bool,
    /// Number of system files updated
    pub files_updated: u32,
    /// Number of new system files added
    pub files_added: u32,
    /// Number of trash files cleaned up
    pub trash_cleaned: u32,
}

/// Create the initial directory structure
async fn create_initial_structure() -> Result<(), VfsError> {
    let directories = [
        // Root home (with special permissions - can't be deleted)
        ("/home", Permissions::readonly()),
        // User directories
        ("/home/Desktop", Permissions::default()),
        ("/home/Documents", Permissions::default()),
        ("/home/Pictures", Permissions::default()),
        ("/home/Music", Permissions::default()),
        ("/home/Videos", Permissions::default()),
        ("/home/Downloads", Permissions::default()),
        // Apps directory for Python apps
        ("/home/apps", Permissions::default()),
        // Hidden directories
        ("/home/.config", Permissions::hidden()),
        ("/home/.Trash", Permissions::hidden()),
        (
            "/home/.system",
            Permissions {
                readonly: true,
                system: true,
                hidden: true,
            },
        ),
        ("/home/.system/wallpapers", Permissions::system_hidden()),
        ("/home/.system/default_apps", Permissions::system_hidden()),
        ("/home/.system/templates", Permissions::system_hidden()),
        ("/home/apps", Permissions::default()), // User apps directory
    ];

    for (dir_path, permissions) in directories {
        if !vfs::exists(dir_path).await? {
            vfs::create_dir_with_permissions(dir_path, permissions).await?;
        }
    }

    Ok(())
}

/// Sync system files from server
async fn sync_system_files() -> Result<SyncResult, VfsError> {
    let mut result = SyncResult::default();

    // Try to fetch manifest from server
    let manifest = match fetch_manifest().await {
        Ok(m) => m,
        Err(e) => {
            // If we can't reach the server, continue without updating
            web_sys::console::warn_1(&format!("Could not fetch system manifest: {:?}", e).into());
            return Ok(result);
        }
    };

    web_sys::console::log_1(&format!("[VFS] Syncing {} system files", manifest.files.len()).into());

    for file in manifest.files {
        let normalized_path = path::normalize(&file.path);

        // Check if file needs update
        let current_hash = vfs::get_system_version(&normalized_path).await?.unwrap_or_default();
        let file_exists = vfs::exists(&normalized_path).await?;

        if current_hash != file.hash || !file_exists {
            web_sys::console::log_1(&format!("[VFS] Fetching: {} -> {}", file.server_path, normalized_path).into());
            // Fetch file from server
            match fetch_file(&file.server_path).await {
                Ok(content) => {
                    // Ensure parent directory exists
                    if let Some(parent) = path::parent(&normalized_path) {
                        if !vfs::exists(&parent).await? {
                            vfs::create_dir_all(&parent).await?;
                        }
                    }

                    // Write file with system permissions
                    vfs::write_file_force(&normalized_path, &content, file.permissions.into())
                        .await?;

                    // Update version hash
                    vfs::set_system_version(&normalized_path, &file.hash).await?;

                    web_sys::console::log_1(&format!("[VFS] Written: {} ({} bytes)", normalized_path, content.len()).into());

                    if file_exists {
                        result.updated += 1;
                    } else {
                        result.added += 1;
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[VFS] Could not fetch system file {}: {:?}", file.server_path, e).into(),
                    );
                }
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Default)]
struct SyncResult {
    updated: u32,
    added: u32,
}

/// Fetch the system manifest from server
async fn fetch_manifest() -> Result<SystemManifest, VfsError> {
    let response = fetch_url("/resources/system_manifest.json").await?;
    let text = response_text(response).await?;

    serde_json::from_str(&text).map_err(|e| VfsError::SerializationError(format!("{}", e)))
}

/// Fetch a file from the server
async fn fetch_file(url: &str) -> Result<Vec<u8>, VfsError> {
    let response = fetch_url(url).await?;
    response_bytes(response).await
}

/// Fetch a URL using the Fetch API
async fn fetch_url(url: &str) -> Result<web_sys::Response, VfsError> {
    let window = web_sys::window()
        .ok_or_else(|| VfsError::NetworkError("No window object".to_string()))?;

    let promise = window.fetch_with_str(url);
    let response = JsFuture::from(promise)
        .await
        .map_err(|e| VfsError::NetworkError(format!("Fetch failed: {:?}", e)))?;

    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| VfsError::NetworkError("Invalid response".to_string()))?;

    if !response.ok() {
        return Err(VfsError::NetworkError(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    Ok(response)
}

/// Get response body as text
async fn response_text(response: web_sys::Response) -> Result<String, VfsError> {
    let promise = response
        .text()
        .map_err(|e| VfsError::NetworkError(format!("Failed to get text: {:?}", e)))?;

    let text = JsFuture::from(promise)
        .await
        .map_err(|e| VfsError::NetworkError(format!("Failed to read text: {:?}", e)))?;

    text.as_string()
        .ok_or_else(|| VfsError::NetworkError("Response is not text".to_string()))
}

/// Get response body as bytes
async fn response_bytes(response: web_sys::Response) -> Result<Vec<u8>, VfsError> {
    let promise = response
        .array_buffer()
        .map_err(|e| VfsError::NetworkError(format!("Failed to get array buffer: {:?}", e)))?;

    let array_buffer = JsFuture::from(promise)
        .await
        .map_err(|e| VfsError::NetworkError(format!("Failed to read array buffer: {:?}", e)))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

/// Get initialization progress for boot screen
/// Returns (step_name, progress_percent)
pub fn get_init_step(step: u32) -> (&'static str, u32) {
    match step {
        0 => ("Initializing storage...", 10),
        1 => ("Creating directories...", 30),
        2 => ("Syncing system files...", 50),
        3 => ("Cleaning up trash...", 80),
        4 => ("Filesystem ready", 100),
        _ => ("Ready", 100),
    }
}
