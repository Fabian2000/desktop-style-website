//! In-memory VFS Cache
//!
//! Provides synchronous access to the virtual filesystem by keeping
//! all file metadata and content in memory. Changes are persisted
//! to IndexedDB asynchronously in the background.
//!
//! This replaces the JavaScript __vfsSync bridge and allows Python
//! apps to access the filesystem synchronously.

use crate::filesystem::path;
use crate::filesystem::storage::FsStorage;
use crate::filesystem::types::{FileNode, Permissions};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

/// Cached file entry with metadata and optional content
#[derive(Clone)]
struct CacheEntry {
    node: FileNode,
    /// Text content for files (None for directories)
    content: Option<Vec<u8>>,
}

/// In-memory VFS cache
pub struct VfsCache {
    /// Path -> CacheEntry mapping
    entries: HashMap<String, CacheEntry>,
    /// Reference to storage for async persistence
    storage: Option<Rc<FsStorage>>,
    /// Whether the cache has been initialized
    initialized: bool,
}

impl VfsCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            storage: None,
            initialized: false,
        }
    }

    /// Set the storage backend for async persistence
    pub fn set_storage(&mut self, storage: Rc<FsStorage>) {
        self.storage = Some(storage);
    }

    /// Check if cache is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Mark cache as initialized
    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    /// Load a node into the cache (called during init)
    pub fn load_node(&mut self, node: FileNode) {
        let path = node.path.clone();
        self.entries.insert(
            path,
            CacheEntry {
                node,
                content: None,
            },
        );
    }

    /// Load content into the cache (called during init)
    pub fn load_content(&mut self, path: &str, content: Vec<u8>) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.content = Some(content);
        }
    }

    // ============ Synchronous Read Operations ============

    /// Check if a path exists
    pub fn exists(&self, path: &str) -> bool {
        let normalized = path::normalize(path);
        self.entries.contains_key(&normalized)
    }

    /// Get file metadata
    pub fn stat(&self, path: &str) -> Option<FileNode> {
        let normalized = path::normalize(path);
        self.entries.get(&normalized).map(|e| e.node.clone())
    }

    /// Read file content as bytes
    pub fn read_bytes(&self, path: &str) -> Option<Vec<u8>> {
        let normalized = path::normalize(path);
        self.entries
            .get(&normalized)
            .and_then(|e| {
                if e.node.is_file() {
                    e.content.clone()
                } else {
                    None
                }
            })
    }

    /// Read file content as string
    pub fn read_text(&self, path: &str) -> Option<String> {
        self.read_bytes(path)
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    /// List directory contents
    pub fn list_dir(&self, path: &str) -> Vec<FileNode> {
        let normalized = path::normalize(path);
        let prefix = if normalized.ends_with('/') {
            normalized.clone()
        } else {
            format!("{}/", normalized)
        };

        let mut results = Vec::new();
        for (entry_path, entry) in &self.entries {
            if entry_path.starts_with(&prefix) {
                let remaining = &entry_path[prefix.len()..];
                // Only direct children (no further slashes)
                if !remaining.is_empty() && !remaining.contains('/') {
                    results.push(entry.node.clone());
                }
            }
        }
        results
    }

    // ============ Synchronous Write Operations ============

    /// Write file content (creates or overwrites)
    /// The parent directory must exist - use mkdir_p to create directories recursively
    pub fn write(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        let normalized = path::normalize(path);

        // Validate path
        if let Some(err) = path::validate_path(&normalized) {
            return Err(err);
        }

        // Check if path is writable
        if !path::can_write(&normalized) {
            return Err(format!("Cannot write to protected path: {}", normalized));
        }

        // Check if parent directory exists
        if let Some(parent_path) = path::parent(&normalized) {
            if !self.entries.contains_key(&parent_path) {
                return Err(format!("Parent directory does not exist: {}", parent_path));
            }
            // Also verify parent is actually a directory
            if let Some(parent_entry) = self.entries.get(&parent_path) {
                if !parent_entry.node.is_dir() {
                    return Err(format!("Parent is not a directory: {}", parent_path));
                }
            }
        }

        // Check if trying to overwrite a directory
        if let Some(existing) = self.entries.get(&normalized) {
            if existing.node.is_dir() {
                return Err(format!("Cannot overwrite directory: {}", normalized));
            }
            if existing.node.permissions.readonly {
                return Err(format!("File is read-only: {}", normalized));
            }
        }

        let name = path::file_name(&normalized).unwrap_or_default();
        let mime_type = path::mime_type(&normalized);
        let mut node = FileNode::new_file(&normalized, &name, content.len() as u64, mime_type);
        node.touch();

        // Update cache
        self.entries.insert(
            normalized.clone(),
            CacheEntry {
                node: node.clone(),
                content: Some(content.to_vec()),
            },
        );

        // Async persist to IndexedDB
        self.persist_node_and_content(&normalized, &node, content);

        Ok(())
    }

    /// Create a directory (parent must exist)
    pub fn mkdir(&mut self, path: &str) -> Result<(), String> {
        let normalized = path::normalize(path);

        // Validate path
        if let Some(err) = path::validate_path(&normalized) {
            return Err(err);
        }

        // Check if path is writable
        if !path::can_write(&normalized) {
            return Err(format!("Cannot create directory in protected path: {}", normalized));
        }

        // Check if parent directory exists (except for /home which has no parent in our system)
        if let Some(parent_path) = path::parent(&normalized) {
            if parent_path != "/home" && !self.entries.contains_key(&parent_path) {
                return Err(format!("Parent directory does not exist: {}", parent_path));
            }
        }

        // Check if already exists
        if self.entries.contains_key(&normalized) {
            return Err(format!("Already exists: {}", normalized));
        }

        let name = path::file_name(&normalized).unwrap_or_default();
        let node = FileNode::new_directory(&normalized, &name);

        // Update cache
        self.entries.insert(
            normalized.clone(),
            CacheEntry {
                node: node.clone(),
                content: None,
            },
        );

        // Async persist to IndexedDB
        self.persist_node(&normalized, &node);

        Ok(())
    }

    /// Create a directory and all parent directories if needed (like mkdir -p)
    pub fn mkdir_p(&mut self, path: &str) -> Result<(), String> {
        let normalized = path::normalize(path);

        // Validate path
        if let Some(err) = path::validate_path(&normalized) {
            return Err(err);
        }

        // Check if path is writable
        if !path::can_write(&normalized) {
            return Err(format!("Cannot create directory in protected path: {}", normalized));
        }

        // If already exists and is a directory, that's fine
        if let Some(existing) = self.entries.get(&normalized) {
            if existing.node.is_dir() {
                return Ok(()); // Already exists as directory
            } else {
                return Err(format!("Path exists as a file: {}", normalized));
            }
        }

        // Collect all directories that need to be created
        let mut to_create = vec![normalized.clone()];
        let mut current = normalized.clone();

        while let Some(parent) = path::parent(&current) {
            if parent == "/home" {
                break; // /home always exists
            }
            if self.entries.contains_key(&parent) {
                // Check that parent is a directory
                if let Some(entry) = self.entries.get(&parent) {
                    if !entry.node.is_dir() {
                        return Err(format!("Parent path is not a directory: {}", parent));
                    }
                }
                break; // Parent exists, stop here
            }
            // Check if this parent path is writable
            if !path::can_write(&parent) {
                return Err(format!("Cannot create directory in protected path: {}", parent));
            }
            to_create.push(parent.clone());
            current = parent;
        }

        // Create directories from parent to child
        to_create.reverse();
        for dir_path in to_create {
            let name = path::file_name(&dir_path).unwrap_or_default();
            let node = FileNode::new_directory(&dir_path, &name);

            self.entries.insert(
                dir_path.clone(),
                CacheEntry {
                    node: node.clone(),
                    content: None,
                },
            );

            self.persist_node(&dir_path, &node);
        }

        Ok(())
    }

    /// Remove a file or directory
    pub fn remove(&mut self, path: &str) -> Result<(), String> {
        let normalized = path::normalize(path);

        // Check if path is protected
        if path::is_protected(&normalized) {
            return Err(format!("Cannot delete protected path: {}", normalized));
        }

        // Check if exists
        let entry = self.entries.get(&normalized)
            .ok_or_else(|| format!("Not found: {}", normalized))?;

        if entry.node.permissions.readonly {
            return Err(format!("Read-only: {}", normalized));
        }

        // If directory, check if empty
        if entry.node.is_dir() {
            let children = self.list_dir(&normalized);
            if !children.is_empty() {
                return Err(format!("Directory not empty: {}", normalized));
            }
        }

        // Remove from cache
        self.entries.remove(&normalized);

        // Async delete from IndexedDB
        self.delete_node(&normalized);

        Ok(())
    }

    /// Copy a file
    pub fn copy(&mut self, src: &str, dst: &str) -> Result<(), String> {
        let src_normalized = path::normalize(src);
        let dst_normalized = path::normalize(dst);

        // Get source entry
        let src_entry = self.entries.get(&src_normalized)
            .ok_or_else(|| format!("Source not found: {}", src_normalized))?;

        if !src_entry.node.is_file() {
            return Err(format!("Cannot copy directory: {}", src_normalized));
        }

        // Get content
        let content = src_entry.content.clone()
            .ok_or_else(|| format!("No content: {}", src_normalized))?;

        // Write to destination
        self.write(&dst_normalized, &content)
    }

    /// Move/rename a file or directory
    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), String> {
        let src_normalized = path::normalize(src);
        let dst_normalized = path::normalize(dst);

        // Check if source is protected
        if path::is_protected(&src_normalized) {
            return Err(format!("Cannot move protected path: {}", src_normalized));
        }

        // Check if destination is writable
        if !path::can_write(&dst_normalized) {
            return Err(format!("Cannot move to protected path: {}", dst_normalized));
        }

        // Get source entry
        let src_entry = self.entries.get(&src_normalized)
            .ok_or_else(|| format!("Source not found: {}", src_normalized))?
            .clone();

        if src_entry.node.permissions.readonly {
            return Err(format!("Source is read-only: {}", src_normalized));
        }

        // Check destination doesn't exist
        if self.entries.contains_key(&dst_normalized) {
            return Err(format!("Destination exists: {}", dst_normalized));
        }

        if src_entry.node.is_dir() {
            // Move directory and all children
            let prefix = format!("{}/", src_normalized);
            let to_move: Vec<_> = self.entries.iter()
                .filter(|(p, _)| *p == &src_normalized || p.starts_with(&prefix))
                .map(|(p, e)| (p.clone(), e.clone()))
                .collect();

            for (old_path, entry) in to_move {
                // Calculate new path
                let relative = if old_path == src_normalized {
                    String::new()
                } else {
                    old_path[src_normalized.len()..].to_string()
                };
                let new_path = format!("{}{}", dst_normalized, relative);
                let new_name = path::file_name(&new_path).unwrap_or_default();

                // Create new entry
                let mut new_node = entry.node.clone();
                new_node.path = new_path.clone();
                new_node.name = new_name;

                // Remove old, add new
                self.entries.remove(&old_path);
                self.entries.insert(
                    new_path.clone(),
                    CacheEntry {
                        node: new_node.clone(),
                        content: entry.content.clone(),
                    },
                );

                // Persist changes
                self.delete_node(&old_path);
                if let Some(content) = &entry.content {
                    self.persist_node_and_content(&new_path, &new_node, content);
                } else {
                    self.persist_node(&new_path, &new_node);
                }
            }
        } else {
            // Move single file
            let new_name = path::file_name(&dst_normalized).unwrap_or_default();
            let mut new_node = src_entry.node.clone();
            new_node.path = dst_normalized.clone();
            new_node.name = new_name;

            // Remove old, add new
            self.entries.remove(&src_normalized);
            self.entries.insert(
                dst_normalized.clone(),
                CacheEntry {
                    node: new_node.clone(),
                    content: src_entry.content.clone(),
                },
            );

            // Persist changes
            self.delete_node(&src_normalized);
            if let Some(content) = &src_entry.content {
                self.persist_node_and_content(&dst_normalized, &new_node, content);
            } else {
                self.persist_node(&dst_normalized, &new_node);
            }
        }

        Ok(())
    }

    // ============ Internal Write Operations (bypass protection) ============

    /// Write file with custom permissions (internal use)
    #[allow(dead_code)]
    pub(crate) fn write_force(&mut self, path: &str, content: &[u8], permissions: Permissions) {
        let normalized = path::normalize(path);
        let name = path::file_name(&normalized).unwrap_or_default();
        let mime_type = path::mime_type(&normalized);

        let mut node = FileNode::new_file(&normalized, &name, content.len() as u64, mime_type);
        node.permissions = permissions;

        self.entries.insert(
            normalized.clone(),
            CacheEntry {
                node: node.clone(),
                content: Some(content.to_vec()),
            },
        );

        self.persist_node_and_content(&normalized, &node, content);
    }

    /// Create directory with custom permissions (internal use)
    #[allow(dead_code)]
    pub(crate) fn mkdir_force(&mut self, path: &str, permissions: Permissions) {
        let normalized = path::normalize(path);
        let name = path::file_name(&normalized).unwrap_or_default();

        let node = FileNode::new_directory(&normalized, &name).with_permissions(permissions);

        self.entries.insert(
            normalized.clone(),
            CacheEntry {
                node: node.clone(),
                content: None,
            },
        );

        self.persist_node(&normalized, &node);
    }

    // ============ Async Persistence Helpers ============

    /// Persist a node to IndexedDB (async, fire-and-forget)
    fn persist_node(&self, _path: &str, node: &FileNode) {
        if let Some(storage) = &self.storage {
            let storage = storage.clone();
            let node = node.clone();
            spawn_local(async move {
                if let Err(e) = storage.put_node(&node).await {
                    web_sys::console::error_1(&format!("[VFS Cache] Failed to persist node: {:?}", e).into());
                }
            });
        }
    }

    /// Persist a node and its content to IndexedDB (async, fire-and-forget)
    fn persist_node_and_content(&self, path: &str, node: &FileNode, content: &[u8]) {
        if let Some(storage) = &self.storage {
            let storage = storage.clone();
            let node = node.clone();
            let path = path.to_string();
            let content = content.to_vec();
            spawn_local(async move {
                if let Err(e) = storage.put_node(&node).await {
                    web_sys::console::error_1(&format!("[VFS Cache] Failed to persist node: {:?}", e).into());
                }
                if let Err(e) = storage.put_content(&path, &content).await {
                    web_sys::console::error_1(&format!("[VFS Cache] Failed to persist content: {:?}", e).into());
                }
            });
        }
    }

    /// Delete a node (and content) from IndexedDB (async, fire-and-forget)
    fn delete_node(&self, path: &str) {
        if let Some(storage) = &self.storage {
            let storage = storage.clone();
            let path = path.to_string();
            spawn_local(async move {
                let _ = storage.delete_content(&path).await;
                if let Err(e) = storage.delete_node(&path).await {
                    web_sys::console::error_1(&format!("[VFS Cache] Failed to delete node: {:?}", e).into());
                }
            });
        }
    }
}

impl Default for VfsCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Global Cache Instance ============

thread_local! {
    /// Global VFS cache instance
    static CACHE: RefCell<VfsCache> = RefCell::new(VfsCache::new());
}

/// Get access to the global cache for reading
pub fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&VfsCache) -> R,
{
    CACHE.with(|c| f(&c.borrow()))
}

/// Get access to the global cache for writing
pub fn with_cache_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut VfsCache) -> R,
{
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

// ============ Public Synchronous API ============

/// Check if a path exists (synchronous)
pub fn exists_sync(path: &str) -> bool {
    with_cache(|c| c.exists(path))
}

/// Get file metadata (synchronous)
pub fn stat_sync(path: &str) -> Option<FileNode> {
    with_cache(|c| c.stat(path))
}

/// Read file as bytes (synchronous)
pub fn read_bytes_sync(path: &str) -> Option<Vec<u8>> {
    with_cache(|c| c.read_bytes(path))
}

/// Read file as string (synchronous)
pub fn read_text_sync(path: &str) -> Option<String> {
    with_cache(|c| c.read_text(path))
}

/// List directory contents (synchronous)
pub fn list_dir_sync(path: &str) -> Vec<FileNode> {
    with_cache(|c| c.list_dir(path))
}

/// Write file content (synchronous, persists async)
pub fn write_sync(path: &str, content: &[u8]) -> Result<(), String> {
    with_cache_mut(|c| c.write(path, content))
}

/// Write text file (synchronous, persists async)
pub fn write_text_sync(path: &str, content: &str) -> Result<(), String> {
    write_sync(path, content.as_bytes())
}

/// Create directory (synchronous, persists async)
/// Parent directory must exist - use mkdir_p_sync to create recursively
pub fn mkdir_sync(path: &str) -> Result<(), String> {
    with_cache_mut(|c| c.mkdir(path))
}

/// Create directory and all parent directories (synchronous, persists async)
/// Like mkdir -p: creates parent directories as needed
pub fn mkdir_p_sync(path: &str) -> Result<(), String> {
    with_cache_mut(|c| c.mkdir_p(path))
}

/// Remove file or directory (synchronous, persists async)
pub fn remove_sync(path: &str) -> Result<(), String> {
    with_cache_mut(|c| c.remove(path))
}

/// Copy file (synchronous, persists async)
pub fn copy_sync(src: &str, dst: &str) -> Result<(), String> {
    with_cache_mut(|c| c.copy(src, dst))
}

/// Move/rename file or directory (synchronous, persists async)
pub fn rename_sync(src: &str, dst: &str) -> Result<(), String> {
    with_cache_mut(|c| c.rename(src, dst))
}

/// Get data URL for a file (for images, icons, etc.)
pub fn get_data_url_sync(path: &str) -> Option<String> {
    let bytes = read_bytes_sync(path)?;

    // Determine MIME type from extension
    let ext = path.split('.').last()?.to_lowercase();
    let mime_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    };

    // Convert to base64
    use base64::{engine::general_purpose::STANDARD, Engine};
    let base64 = STANDARD.encode(&bytes);

    Some(format!("data:{};base64,{}", mime_type, base64))
}
