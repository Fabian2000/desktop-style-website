use serde::{Deserialize, Serialize};

/// File types in the virtual filesystem
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
    Symlink { target: String },
}

/// Access permissions for files/directories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permissions {
    /// Cannot be modified or deleted by user
    pub readonly: bool,
    /// System file - gets updated from server on boot
    pub system: bool,
    /// Hidden from normal directory listings
    pub hidden: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            readonly: false,
            system: false,
            hidden: false,
        }
    }
}

impl Permissions {
    pub fn readonly() -> Self {
        Self {
            readonly: true,
            system: false,
            hidden: false,
        }
    }

    pub fn system() -> Self {
        Self {
            readonly: true,
            system: true,
            hidden: false,
        }
    }

    pub fn hidden() -> Self {
        Self {
            readonly: false,
            system: false,
            hidden: true,
        }
    }

    pub fn system_hidden() -> Self {
        Self {
            readonly: true,
            system: true,
            hidden: true,
        }
    }
}

/// Metadata for a file or directory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileNode {
    /// File/directory name (without path)
    pub name: String,
    /// Full path from root
    pub path: String,
    /// Type of this node
    pub file_type: FileType,
    /// Access permissions
    pub permissions: Permissions,
    /// Creation timestamp (Unix milliseconds)
    pub created: u64,
    /// Last modification timestamp (Unix milliseconds)
    pub modified: u64,
    /// Size in bytes (0 for directories)
    pub size: u64,
    /// MIME type for files (e.g., "text/plain", "image/png")
    pub mime_type: Option<String>,
}

impl FileNode {
    /// Create a new file node
    pub fn new_file(path: &str, name: &str, size: u64, mime_type: Option<String>) -> Self {
        let now = js_sys::Date::now() as u64;
        Self {
            name: name.to_string(),
            path: path.to_string(),
            file_type: FileType::File,
            permissions: Permissions::default(),
            created: now,
            modified: now,
            size,
            mime_type,
        }
    }

    /// Create a new directory node
    pub fn new_directory(path: &str, name: &str) -> Self {
        let now = js_sys::Date::now() as u64;
        Self {
            name: name.to_string(),
            path: path.to_string(),
            file_type: FileType::Directory,
            permissions: Permissions::default(),
            created: now,
            modified: now,
            size: 0,
            mime_type: None,
        }
    }

    /// Create a new symlink node
    pub fn new_symlink(path: &str, name: &str, target: &str) -> Self {
        let now = js_sys::Date::now() as u64;
        Self {
            name: name.to_string(),
            path: path.to_string(),
            file_type: FileType::Symlink {
                target: target.to_string(),
            },
            permissions: Permissions::default(),
            created: now,
            modified: now,
            size: 0,
            mime_type: None,
        }
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File)
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        matches!(self.file_type, FileType::Symlink { .. })
    }

    /// Update the modified timestamp to now
    pub fn touch(&mut self) {
        self.modified = js_sys::Date::now() as u64;
    }

    /// Set permissions
    pub fn with_permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }
}

/// Errors that can occur during VFS operations
#[derive(Debug, Clone)]
pub enum VfsError {
    /// File or directory not found
    NotFound(String),
    /// File already exists
    AlreadyExists(String),
    /// Permission denied (readonly/system file)
    PermissionDenied(String),
    /// Not a directory (e.g., trying to list contents of a file)
    NotADirectory(String),
    /// Not a file (e.g., trying to read contents of a directory)
    NotAFile(String),
    /// Directory is not empty (can't delete)
    DirectoryNotEmpty(String),
    /// Invalid path format
    InvalidPath(String),
    /// File too large (exceeds 10MB limit)
    FileTooLarge { size: u64, max: u64 },
    /// Storage error (IndexedDB)
    StorageError(String),
    /// Network error (fetching system files)
    NetworkError(String),
    /// Serialization/deserialization error
    SerializationError(String),
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsError::NotFound(path) => write!(f, "Not found: {}", path),
            VfsError::AlreadyExists(path) => write!(f, "Already exists: {}", path),
            VfsError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            VfsError::NotADirectory(path) => write!(f, "Not a directory: {}", path),
            VfsError::NotAFile(path) => write!(f, "Not a file: {}", path),
            VfsError::DirectoryNotEmpty(path) => write!(f, "Directory not empty: {}", path),
            VfsError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            VfsError::FileTooLarge { size, max } => {
                write!(f, "File too large: {} bytes (max: {} bytes)", size, max)
            }
            VfsError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            VfsError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            VfsError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

/// Maximum file size in bytes (10 MB)
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Trash retention period in milliseconds (5 days)
pub const TRASH_RETENTION_MS: u64 = 5 * 24 * 60 * 60 * 1000;
