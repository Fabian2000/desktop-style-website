//! Virtual Filesystem Module
//!
//! Provides a complete virtual filesystem that runs in the browser,
//! persisted in IndexedDB.
//!
//! # Features
//! - Unix-like path structure starting at /home/
//! - File and directory operations (read, write, delete, rename, copy)
//! - Permission system (readonly, system, hidden)
//! - Trash with automatic cleanup (5 days retention)
//! - System file synchronization from server
//! - Symlink support
//!
//! # Usage
//! ```rust,ignore
//! use fabi_sc_code::filesystem::{vfs, init};
//!
//! // Initialize during boot
//! init::initialize().await?;
//!
//! // Read a directory
//! let entries = vfs::read_dir("/home/Documents").await?;
//!
//! // Read a file
//! let content = vfs::read_to_string("/home/Documents/notes.txt").await?;
//!
//! // Write a file
//! vfs::write_file("/home/Documents/new.txt", b"Hello!").await?;
//!
//! // Delete a file (moves to trash)
//! vfs::remove_file("/home/Documents/old.txt").await?;
//! ```

pub mod cache;
pub mod events;
pub mod init;
pub mod path;
pub(crate) mod storage;
pub mod types;
pub mod vfs;

// Re-export commonly used items
pub use cache::{
    copy_sync, exists_sync, get_data_url_sync, list_dir_sync, mkdir_p_sync, mkdir_sync,
    read_bytes_sync, read_text_sync, remove_sync, rename_sync, stat_sync, with_cache,
    with_cache_mut, write_sync, write_text_sync,
};
pub use init::{initialize, InitResult};
pub use types::{FileNode, FileType, Permissions, VfsError};

// Re-export all vfs functions for convenience
pub use vfs::{
    // Read operations
    exists,
    read_dir,
    read_dir_visible,
    read_file,
    read_link,
    read_to_string,
    stat,
    // Write operations
    copy,
    create_dir,
    create_dir_all,
    rename,
    symlink,
    write_file,
    // Delete operations
    remove_dir,
    remove_dir_all,
    remove_file,
    // Trash
    cleanup_trash,
};

// Re-export path utilities
pub use path::{
    extension, file_name, file_stem, is_child_of, is_hidden, is_valid, join, mime_type, normalize,
    parent,
};
