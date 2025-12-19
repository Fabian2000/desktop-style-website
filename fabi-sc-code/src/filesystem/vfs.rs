//! Virtual Filesystem API
//!
//! Provides fs-like functions for file operations with permission enforcement.

use crate::filesystem::path;
use crate::filesystem::storage::FsStorage;
use crate::filesystem::types::{FileNode, FileType, Permissions, VfsError, MAX_FILE_SIZE, TRASH_RETENTION_MS};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Global storage instance (thread-local for WASM single-threaded environment)
    static STORAGE: RefCell<Option<Rc<FsStorage>>> = const { RefCell::new(None) };
}

/// Initialize the VFS storage (must be called once at startup)
pub async fn init_storage() -> Result<(), VfsError> {
    let storage = FsStorage::open().await?;
    STORAGE.with(|s| {
        *s.borrow_mut() = Some(Rc::new(storage));
    });
    Ok(())
}

/// Get the storage instance
fn get_storage() -> Result<Rc<FsStorage>, VfsError> {
    STORAGE.with(|s| {
        s.borrow()
            .clone()
            .ok_or_else(|| VfsError::StorageError("Storage not initialized".to_string()))
    })
}

/// Check if the filesystem has been initialized
pub async fn is_initialized() -> Result<bool, VfsError> {
    get_storage()?.is_initialized().await
}

// ============ Read Operations ============

/// Get file/directory metadata
pub async fn stat(path: &str) -> Result<FileNode, VfsError> {
    let normalized = path::normalize(path);
    if !path::is_valid(&normalized) {
        return Err(VfsError::InvalidPath(path.to_string()));
    }

    get_storage()?
        .get_node(&normalized)
        .await?
        .ok_or_else(|| VfsError::NotFound(normalized))
}

/// Check if a path exists
pub async fn exists(path: &str) -> Result<bool, VfsError> {
    let normalized = path::normalize(path);
    if !path::is_valid(&normalized) {
        return Ok(false);
    }

    Ok(get_storage()?.get_node(&normalized).await?.is_some())
}

/// List directory contents
pub async fn read_dir(path: &str) -> Result<Vec<FileNode>, VfsError> {
    let normalized = path::normalize(path);
    let node = stat(&normalized).await?;

    if !node.is_dir() {
        return Err(VfsError::NotADirectory(normalized));
    }

    get_storage()?.list_children(&normalized).await
}

/// List directory contents (excluding hidden files)
pub async fn read_dir_visible(path: &str) -> Result<Vec<FileNode>, VfsError> {
    let entries = read_dir(path).await?;
    Ok(entries
        .into_iter()
        .filter(|e| !e.permissions.hidden && !e.name.starts_with('.'))
        .collect())
}

/// Read file contents as bytes
pub async fn read_file(path: &str) -> Result<Vec<u8>, VfsError> {
    read_file_impl(path, 0).await
}

/// Internal implementation with symlink recursion limit
async fn read_file_impl(path: &str, depth: u32) -> Result<Vec<u8>, VfsError> {
    const MAX_SYMLINK_DEPTH: u32 = 10;

    if depth > MAX_SYMLINK_DEPTH {
        return Err(VfsError::InvalidPath("Too many symlink levels".to_string()));
    }

    let normalized = path::normalize(path);
    let node = stat(&normalized).await?;

    // Follow symlinks
    if let FileType::Symlink { target } = &node.file_type {
        return Box::pin(read_file_impl(target, depth + 1)).await;
    }

    if !node.is_file() {
        return Err(VfsError::NotAFile(normalized));
    }

    get_storage()?
        .get_content(&normalized)
        .await?
        .ok_or_else(|| VfsError::NotFound(normalized))
}

/// Read file contents as string (UTF-8)
pub async fn read_to_string(path: &str) -> Result<String, VfsError> {
    let bytes = read_file(path).await?;
    String::from_utf8(bytes).map_err(|e| VfsError::SerializationError(format!("Invalid UTF-8: {}", e)))
}

/// Read symlink target
pub async fn read_link(path: &str) -> Result<String, VfsError> {
    let normalized = path::normalize(path);
    let node = stat(&normalized).await?;

    match node.file_type {
        FileType::Symlink { target } => Ok(target),
        _ => Err(VfsError::NotAFile(format!("{} is not a symlink", normalized))),
    }
}

// ============ Write Operations ============

/// Write file contents (creates or overwrites)
pub async fn write_file(file_path: &str, data: &[u8]) -> Result<(), VfsError> {
    let normalized = path::normalize(file_path);

    // Validate path format and characters
    if let Some(err) = path::validate_path(&normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if path is writable (not in .system)
    if !path::can_write(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot write to protected path: {}",
            normalized
        )));
    }

    // Check file size limit
    if data.len() as u64 > MAX_FILE_SIZE {
        return Err(VfsError::FileTooLarge {
            size: data.len() as u64,
            max: MAX_FILE_SIZE,
        });
    }

    // Check if file exists and is writable
    if let Ok(existing) = stat(&normalized).await {
        if existing.permissions.readonly {
            return Err(VfsError::PermissionDenied(format!(
                "File is read-only: {}",
                normalized
            )));
        }
        // Check if trying to overwrite a directory with a file
        if existing.is_dir() {
            return Err(VfsError::NotAFile(format!(
                "Cannot overwrite directory with file: {}",
                normalized
            )));
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = path::parent(&normalized) {
        if !exists(&parent).await? {
            return Err(VfsError::NotFound(format!("Parent directory: {}", parent)));
        }
    }

    let name = path::file_name(&normalized).unwrap_or_default();
    let mime_type = path::mime_type(&normalized);

    // Create or update node
    let mut node = FileNode::new_file(&normalized, &name, data.len() as u64, mime_type);

    // Preserve permissions if file exists
    if let Ok(existing) = stat(&normalized).await {
        node.permissions = existing.permissions;
        node.created = existing.created;
    }
    node.touch();

    let storage = get_storage()?;
    storage.put_node(&node).await?;
    storage.put_content(&normalized, data).await?;

    Ok(())
}

/// Write file contents (internal - bypasses readonly check for system updates)
pub(crate) async fn write_file_force(file_path: &str, data: &[u8], permissions: Permissions) -> Result<(), VfsError> {
    let normalized = path::normalize(file_path);
    if !path::is_valid(&normalized) {
        return Err(VfsError::InvalidPath(file_path.to_string()));
    }

    // Check file size limit
    if data.len() as u64 > MAX_FILE_SIZE {
        return Err(VfsError::FileTooLarge {
            size: data.len() as u64,
            max: MAX_FILE_SIZE,
        });
    }

    // Ensure parent directory exists
    if let Some(parent) = path::parent(&normalized) {
        if !exists(&parent).await? {
            // Create parent directories
            create_dir_all(&parent).await?;
        }
    }

    let name = path::file_name(&normalized).unwrap_or_default();
    let mime_type = path::mime_type(&normalized);

    let mut node = FileNode::new_file(&normalized, &name, data.len() as u64, mime_type);
    node.permissions = permissions;

    let storage = get_storage()?;
    storage.put_node(&node).await?;
    storage.put_content(&normalized, data).await?;

    Ok(())
}

/// Create a directory
pub async fn create_dir(dir_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(dir_path);

    // Validate path format and characters
    if let Some(err) = path::validate_path(&normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if path is writable (not in .system)
    if !path::can_write(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot create directory in protected path: {}",
            normalized
        )));
    }

    // Check if already exists
    if exists(&normalized).await? {
        return Err(VfsError::AlreadyExists(normalized));
    }

    // Check parent exists
    if let Some(parent) = path::parent(&normalized) {
        let parent_node = stat(&parent).await?;
        if !parent_node.is_dir() {
            return Err(VfsError::NotADirectory(parent));
        }
        if parent_node.permissions.readonly {
            return Err(VfsError::PermissionDenied(format!(
                "Parent directory is read-only: {}",
                parent
            )));
        }
    }

    let name = path::file_name(&normalized).unwrap_or_default();
    let node = FileNode::new_directory(&normalized, &name);

    get_storage()?.put_node(&node).await
}

/// Create a directory and all parent directories
pub async fn create_dir_all(dir_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(dir_path);

    // Validate path format and characters
    if let Some(err) = path::validate_path(&normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if path is writable (not in .system)
    if !path::can_write(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot create directory in protected path: {}",
            normalized
        )));
    }

    // Build list of directories to create
    let mut to_create = Vec::new();
    let mut current = normalized.clone();

    while !exists(&current).await? {
        to_create.push(current.clone());
        match path::parent(&current) {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // Create directories from root to leaf
    let storage = get_storage()?;
    for dir in to_create.into_iter().rev() {
        let name = path::file_name(&dir).unwrap_or_default();
        let node = FileNode::new_directory(&dir, &name);
        storage.put_node(&node).await?;
    }

    Ok(())
}

/// Create a directory (internal - with custom permissions)
pub(crate) async fn create_dir_with_permissions(dir_path: &str, permissions: Permissions) -> Result<(), VfsError> {
    let normalized = path::normalize(dir_path);
    if !path::is_valid(&normalized) {
        return Err(VfsError::InvalidPath(dir_path.to_string()));
    }

    let name = path::file_name(&normalized).unwrap_or_default();
    let node = FileNode::new_directory(&normalized, &name).with_permissions(permissions);

    get_storage()?.put_node(&node).await
}

/// Create a symlink
pub async fn symlink(target: &str, link_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(link_path);

    // Validate path format and characters
    if let Some(err) = path::validate_path(&normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if path is writable (not in .system)
    if !path::can_write(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot create symlink in protected path: {}",
            normalized
        )));
    }

    // Check if link already exists
    if exists(&normalized).await? {
        return Err(VfsError::AlreadyExists(normalized));
    }

    // Check parent directory permissions
    if let Some(parent) = path::parent(&normalized) {
        let parent_node = stat(&parent).await?;
        if parent_node.permissions.readonly {
            return Err(VfsError::PermissionDenied(format!(
                "Parent directory is read-only: {}",
                parent
            )));
        }
    }

    let name = path::file_name(&normalized).unwrap_or_default();
    let node = FileNode::new_symlink(&normalized, &name, target);

    get_storage()?.put_node(&node).await
}

// ============ Delete Operations ============

/// Remove a file (moves to trash unless force=true)
pub async fn remove_file(file_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(file_path);

    // Check if path is protected
    if path::is_protected(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot delete protected path: {}",
            normalized
        )));
    }

    let node = stat(&normalized).await?;

    if !node.is_file() && !node.is_symlink() {
        return Err(VfsError::NotAFile(normalized));
    }

    // Check permissions
    if node.permissions.readonly {
        return Err(VfsError::PermissionDenied(format!(
            "File is read-only: {}",
            normalized
        )));
    }

    // Move to trash instead of permanent delete
    let timestamp = js_sys::Date::now() as u64;
    let trash_name = format!("{}_{}", timestamp, node.name);
    let trash_path = path::join("/home/.Trash", &trash_name);

    // Ensure trash directory exists
    if !exists("/home/.Trash").await? {
        create_dir_with_permissions("/home/.Trash", Permissions::hidden()).await?;
    }

    rename(&normalized, &trash_path).await
}

/// Remove a file permanently (bypasses trash)
/// Note: For public use, this checks permissions. Internal trash cleanup uses remove_permanently_force.
pub async fn remove_permanently(file_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(file_path);
    let node = stat(&normalized).await?;

    if !node.is_file() && !node.is_symlink() {
        return Err(VfsError::NotAFile(normalized));
    }

    // Check permissions - only allow if not readonly
    if node.permissions.readonly {
        return Err(VfsError::PermissionDenied(format!(
            "File is read-only: {}",
            normalized
        )));
    }

    let storage = get_storage()?;
    storage.delete_content(&normalized).await?;
    storage.delete_node(&normalized).await
}

/// Remove a file permanently (internal - bypasses permission check for trash cleanup)
async fn remove_permanently_force(file_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(file_path);
    let node = stat(&normalized).await?;

    if !node.is_file() && !node.is_symlink() {
        return Err(VfsError::NotAFile(normalized));
    }

    let storage = get_storage()?;
    storage.delete_content(&normalized).await?;
    storage.delete_node(&normalized).await
}

/// Remove an empty directory
pub async fn remove_dir(dir_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(dir_path);

    // Check if path is protected (includes /home, .system, .Trash)
    if path::is_protected(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot delete protected directory: {}",
            normalized
        )));
    }

    let node = stat(&normalized).await?;

    if !node.is_dir() {
        return Err(VfsError::NotADirectory(normalized));
    }

    // Check permissions
    if node.permissions.readonly {
        return Err(VfsError::PermissionDenied(format!(
            "Directory is read-only: {}",
            normalized
        )));
    }

    // Check if empty
    let storage = get_storage()?;
    let children = storage.list_children(&normalized).await?;
    if !children.is_empty() {
        return Err(VfsError::DirectoryNotEmpty(normalized));
    }

    storage.delete_node(&normalized).await
}

/// Remove a directory and all its contents
pub async fn remove_dir_all(dir_path: &str) -> Result<(), VfsError> {
    let normalized = path::normalize(dir_path);

    // Check if path is protected (includes /home, .system, .Trash)
    if path::is_protected(&normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot delete protected directory: {}",
            normalized
        )));
    }

    let node = stat(&normalized).await?;

    if !node.is_dir() {
        return Err(VfsError::NotADirectory(normalized));
    }

    // Check permissions
    if node.permissions.readonly {
        return Err(VfsError::PermissionDenied(format!(
            "Directory is read-only: {}",
            normalized
        )));
    }

    // Get all nodes under this directory
    let storage = get_storage()?;
    let all_nodes = storage.get_nodes_with_prefix(&normalized).await?;

    // Check if any child is readonly
    for child in &all_nodes {
        if child.permissions.readonly && child.path != normalized {
            return Err(VfsError::PermissionDenied(format!(
                "Contains read-only file: {}",
                child.path
            )));
        }
    }

    // Delete all content and nodes (deepest first)
    let mut sorted_nodes = all_nodes;
    sorted_nodes.sort_by(|a, b| b.path.len().cmp(&a.path.len()));

    for node in sorted_nodes {
        if node.is_file() {
            storage.delete_content(&node.path).await?;
        }
        storage.delete_node(&node.path).await?;
    }

    Ok(())
}

// ============ Move/Copy Operations ============

/// Rename/move a file or directory
pub async fn rename(from: &str, to: &str) -> Result<(), VfsError> {
    let from_normalized = path::normalize(from);
    let to_normalized = path::normalize(to);

    // Validate target path format and characters
    if let Some(err) = path::validate_path(&to_normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if source is protected
    if path::is_protected(&from_normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot move protected path: {}",
            from_normalized
        )));
    }

    // Check if target is in protected area
    if !path::can_write(&to_normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot move to protected path: {}",
            to_normalized
        )));
    }

    let node = stat(&from_normalized).await?;

    // Check source permissions
    if node.permissions.readonly {
        return Err(VfsError::PermissionDenied(format!(
            "Source is read-only: {}",
            from_normalized
        )));
    }

    // Check target doesn't exist
    if exists(&to_normalized).await? {
        return Err(VfsError::AlreadyExists(to_normalized));
    }

    // Check target parent permissions
    if let Some(parent) = path::parent(&to_normalized) {
        if let Ok(parent_node) = stat(&parent).await {
            if parent_node.permissions.readonly {
                return Err(VfsError::PermissionDenied(format!(
                    "Target directory is read-only: {}",
                    parent
                )));
            }
        } else {
            return Err(VfsError::NotFound(format!("Target parent: {}", parent)));
        }
    }

    let storage = get_storage()?;

    if node.is_dir() {
        // Move directory and all contents
        let all_nodes = storage.get_nodes_with_prefix(&from_normalized).await?;

        // Check if any child is readonly (can't move protected files)
        for child in &all_nodes {
            if child.permissions.readonly && child.path != from_normalized {
                return Err(VfsError::PermissionDenied(format!(
                    "Contains read-only file: {}",
                    child.path
                )));
            }
        }

        for old_node in all_nodes {
            let relative = &old_node.path[from_normalized.len()..];
            let new_path = format!("{}{}", to_normalized, relative);
            let new_name = path::file_name(&new_path).unwrap_or_default();

            let mut new_node = old_node.clone();
            new_node.path = new_path.clone();
            new_node.name = new_name;

            // Move content if it's a file
            if old_node.is_file() {
                if let Some(content) = storage.get_content(&old_node.path).await? {
                    storage.put_content(&new_path, &content).await?;
                    storage.delete_content(&old_node.path).await?;
                }
            }

            storage.put_node(&new_node).await?;
            storage.delete_node(&old_node.path).await?;
        }
    } else {
        // Move single file
        let new_name = path::file_name(&to_normalized).unwrap_or_default();
        let mut new_node = node.clone();
        new_node.path = to_normalized.clone();
        new_node.name = new_name;

        // Move content
        if let Some(content) = storage.get_content(&from_normalized).await? {
            storage.put_content(&to_normalized, &content).await?;
            storage.delete_content(&from_normalized).await?;
        }

        storage.put_node(&new_node).await?;
        storage.delete_node(&from_normalized).await?;
    }

    Ok(())
}

/// Copy a file
pub async fn copy(from: &str, to: &str) -> Result<(), VfsError> {
    let from_normalized = path::normalize(from);
    let to_normalized = path::normalize(to);

    // Validate target path format and characters
    if let Some(err) = path::validate_path(&to_normalized) {
        return Err(VfsError::InvalidPath(err));
    }

    // Check if target is in protected area
    if !path::can_write(&to_normalized) {
        return Err(VfsError::PermissionDenied(format!(
            "Cannot copy to protected path: {}",
            to_normalized
        )));
    }

    let node = stat(&from_normalized).await?;

    if !node.is_file() {
        return Err(VfsError::NotAFile(format!(
            "Cannot copy directory: {}",
            from_normalized
        )));
    }

    // Check target doesn't exist
    if exists(&to_normalized).await? {
        return Err(VfsError::AlreadyExists(to_normalized));
    }

    // Check target parent permissions
    if let Some(parent) = path::parent(&to_normalized) {
        if let Ok(parent_node) = stat(&parent).await {
            if parent_node.permissions.readonly {
                return Err(VfsError::PermissionDenied(format!(
                    "Target directory is read-only: {}",
                    parent
                )));
            }
        } else {
            return Err(VfsError::NotFound(format!("Target parent: {}", parent)));
        }
    }

    // Read content and write to new location
    // Note: write_file has its own protection checks, but we already checked above
    let content = read_file(&from_normalized).await?;

    // Create node directly without going through write_file to bypass the can_write check
    // (we already checked it above, and write_file would reject copying to protected paths)
    let name = path::file_name(&to_normalized).unwrap_or_default();
    let mime_type = path::mime_type(&to_normalized);
    let new_node = FileNode::new_file(&to_normalized, &name, content.len() as u64, mime_type);

    let storage = get_storage()?;
    storage.put_node(&new_node).await?;
    storage.put_content(&to_normalized, &content).await?;

    Ok(())
}

// ============ Trash Cleanup ============

/// Clean up old files from trash (called during boot)
pub async fn cleanup_trash() -> Result<u32, VfsError> {
    let trash_path = "/home/.Trash";

    if !exists(trash_path).await? {
        return Ok(0);
    }

    let now = js_sys::Date::now() as u64;
    let entries = read_dir(trash_path).await?;
    let mut deleted = 0;

    for entry in entries {
        // Parse timestamp from filename: "1234567890_filename.txt"
        if let Some(underscore_pos) = entry.name.find('_') {
            if let Ok(timestamp) = entry.name[..underscore_pos].parse::<u64>() {
                if now - timestamp > TRASH_RETENTION_MS {
                    // File is older than retention period - delete permanently
                    if entry.is_file() {
                        remove_permanently_force(&entry.path).await?;
                    } else {
                        // For directories in trash, force delete
                        let storage = get_storage()?;
                        let all_nodes = storage.get_nodes_with_prefix(&entry.path).await?;
                        for node in all_nodes {
                            if node.is_file() {
                                storage.delete_content(&node.path).await?;
                            }
                            storage.delete_node(&node.path).await?;
                        }
                    }
                    deleted += 1;
                }
            }
        }
    }

    Ok(deleted)
}

// ============ System Version Operations ============

/// Get the version hash of a system file
pub async fn get_system_version(path: &str) -> Result<Option<String>, VfsError> {
    get_storage()?.get_system_version(path).await
}

/// Set the version hash of a system file
pub async fn set_system_version(path: &str, hash: &str) -> Result<(), VfsError> {
    get_storage()?.set_system_version(path, hash).await
}
