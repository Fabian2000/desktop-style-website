//! VFS API for Python apps
//!
//! Provides async filesystem access to the virtual filesystem.
//! All operations are sandboxed to prevent access outside VFS.

// VFS operations will be implemented here
// For now, we define the API structure

/// VFS operation result
pub enum VfsResult<T> {
    Ok(T),
    NotFound(String),
    PermissionDenied(String),
    Error(String),
}

/// File info returned by list_dir
#[derive(Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

// Note: The actual VFS implementation is in src/filesystem/
// This module will bridge Python calls to the VFS API
