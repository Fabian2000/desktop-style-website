/// Path utilities for the virtual filesystem
/// All paths use forward slashes and start with /home/

/// Maximum length for a single path segment (file/directory name)
/// Reduced from 255 to 100 for better UI display
pub const MAX_NAME_LENGTH: usize = 100;

/// Maximum length for a full path
pub const MAX_PATH_LENGTH: usize = 4096;

/// Reserved names that cannot be used (case-insensitive)
const RESERVED_NAMES: &[&str] = &[
    ".", "..", "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Join two path segments
pub fn join(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        base.to_string()
    } else if base.is_empty() {
        format!("/{}", path)
    } else {
        format!("{}/{}", base, path)
    }
}

/// Get the parent directory of a path
pub fn parent(path: &str) -> Option<String> {
    let path = path.trim_end_matches('/');

    if path.is_empty() || path == "/home" {
        return None;
    }

    match path.rfind('/') {
        Some(0) => Some("/home".to_string()),
        Some(idx) => Some(path[..idx].to_string()),
        None => None,
    }
}

/// Get the file name from a path
pub fn file_name(path: &str) -> Option<String> {
    let path = path.trim_end_matches('/');

    if path.is_empty() {
        return None;
    }

    match path.rfind('/') {
        Some(idx) => {
            let name = &path[idx + 1..];
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        }
        None => Some(path.to_string()),
    }
}

/// Get the file extension from a path
pub fn extension(path: &str) -> Option<String> {
    let name = file_name(path)?;

    // Don't treat hidden files as extensions (e.g., .gitignore)
    if name.starts_with('.') && !name[1..].contains('.') {
        return None;
    }

    match name.rfind('.') {
        Some(idx) if idx > 0 => Some(name[idx + 1..].to_string()),
        _ => None,
    }
}

/// Get the file stem (name without extension)
pub fn file_stem(path: &str) -> Option<String> {
    let name = file_name(path)?;

    // Handle hidden files
    if name.starts_with('.') && !name[1..].contains('.') {
        return Some(name);
    }

    match name.rfind('.') {
        Some(idx) if idx > 0 => Some(name[..idx].to_string()),
        _ => Some(name),
    }
}

/// Normalize a path (resolve . and .., remove duplicate slashes)
/// SECURITY: All paths are contained within /home - cannot escape above it
/// Paths that don't start with /home are returned as-is (after basic normalization)
/// so that is_valid() can properly reject them
pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                // SECURITY: Only pop if we have more than just "home"
                // This ensures we can never go above /home
                // When parts = ["home"], we stay at /home
                // When parts = ["home", "Documents"], we can go back to ["home"]
                if parts.len() > 1 {
                    parts.pop();
                } else if parts.len() == 1 && parts[0] == "home" {
                    // At /home, .. does nothing - stay at /home
                    // This is the key security fix
                } else if !parts.is_empty() {
                    // For non-/home paths (like /etc/..), allow normal behavior
                    // These will be rejected by is_valid() anyway
                    parts.pop();
                }
            }
            _ => parts.push(part),
        }
    }

    // Return normalized path
    if parts.is_empty() {
        // Empty path (like "/" or "") defaults to /home
        "/home".to_string()
    } else {
        format!("/{}", parts.join("/"))
    }
}

/// Check if a path is valid (starts with /home/)
pub fn is_valid(path: &str) -> bool {
    let normalized = normalize(path);
    normalized == "/home" || normalized.starts_with("/home/")
}

/// Sanitize a filename by replacing invalid characters with underscores
/// Returns the sanitized name (may be same as input if already valid)
pub fn sanitize_name(name: &str) -> String {
    // First pass: replace non-Latin characters (emojis, symbols) with underscore
    let latin_safe: String = name
        .chars()
        .map(|c| {
            // Allow: ASCII printable, Latin-1 Supplement, Latin Extended-A/B
            let is_valid_char = c.is_ascii_graphic()
                || c == ' '
                || (c as u32 >= 0x00C0 && c as u32 <= 0x024F); // Latin Extended

            if is_valid_char { c } else { '_' }
        })
        .collect();

    // Second pass: use sanitize-filename for Windows-unsafe characters
    let sanitized = sanitize_filename::sanitize(&latin_safe);

    // Trim trailing spaces and dots (Windows compatibility)
    sanitized.trim_end_matches(|c| c == ' ' || c == '.').to_string()
}

/// Validate a file/directory name (single path segment)
/// Returns None if valid, Some(error_message) if invalid
pub fn validate_name(name: &str) -> Option<String> {
    // Empty name
    if name.is_empty() {
        return Some("Name cannot be empty".to_string());
    }

    // Length check
    if name.len() > MAX_NAME_LENGTH {
        return Some(format!("Name too long: {} chars (max {})", name.len(), MAX_NAME_LENGTH));
    }

    // Sanitize and compare - if different, the original was invalid
    let sanitized = sanitize_name(name);
    if sanitized != name {
        return Some("Invalid filename: contains forbidden characters or patterns".to_string());
    }

    // Check if sanitized name is empty (e.g., name was just dots/spaces)
    if sanitized.is_empty() {
        return Some("Name cannot be empty after sanitization".to_string());
    }

    // Reserved names (case-insensitive) - additional check for Windows compatibility
    let name_upper = name.to_uppercase();
    for reserved in RESERVED_NAMES {
        if name_upper == *reserved || name_upper.starts_with(&format!("{}.", reserved)) {
            return Some(format!("Reserved name not allowed: {}", name));
        }
    }

    None
}

/// Validate a full path (all segments)
/// Returns None if valid, Some(error_message) if invalid
pub fn validate_path(path: &str) -> Option<String> {
    let normalized = normalize(path);

    // Path length check
    if normalized.len() > MAX_PATH_LENGTH {
        return Some(format!("Path too long: {} chars (max {})", normalized.len(), MAX_PATH_LENGTH));
    }

    // Must be within /home
    if !is_valid(&normalized) {
        return Some(format!("Path must be within /home: {}", path));
    }

    // Validate each segment (skip "home" which is the root)
    for segment in normalized.split('/').skip(2) {
        if segment.is_empty() {
            continue; // Skip empty segments (handled by normalize)
        }
        if let Some(err) = validate_name(segment) {
            return Some(err);
        }
    }

    None
}

/// Check if a path is the root /home directory
pub fn is_root(path: &str) -> bool {
    normalize(path) == "/home"
}

/// Check if a path is within the protected .system directory
pub fn is_in_system(path: &str) -> bool {
    let normalized = normalize(path);
    normalized == "/home/.system" || normalized.starts_with("/home/.system/")
}

/// Check if a path is a protected system path that cannot be deleted
/// Protected paths:
/// - /home itself (root cannot be deleted)
/// - /home/.system and everything inside (system files)
/// - /home/.Trash itself (but contents can be deleted)
pub fn is_protected(path: &str) -> bool {
    let normalized = normalize(path);

    // /home cannot be deleted
    if normalized == "/home" {
        return true;
    }

    // .system directory and ALL its contents are protected against deletion
    if normalized == "/home/.system" || normalized.starts_with("/home/.system/") {
        return true;
    }

    // .Trash directory itself is protected (but contents are not)
    if normalized == "/home/.Trash" {
        return true;
    }

    false
}

/// Check if writing/creating at a path is allowed
/// System paths cannot be written to by normal operations
/// Note: This blocks creating/modifying files, not reading them
pub fn can_write(path: &str) -> bool {
    let normalized = normalize(path);

    // Cannot write directly to /home (it's the root, not a file)
    // But can create files/dirs inside /home
    if normalized == "/home" {
        return false;
    }

    // Cannot write to .system or anything inside it (except via force functions)
    // This protects all system files at any depth
    if normalized == "/home/.system" || normalized.starts_with("/home/.system/") {
        return false;
    }

    true
}

/// Check if a path is a child of another path
pub fn is_child_of(child: &str, parent: &str) -> bool {
    let child = normalize(child);
    let parent = normalize(parent);

    if child == parent {
        return false;
    }

    let parent_with_slash = if parent.ends_with('/') {
        parent
    } else {
        format!("{}/", parent)
    };

    child.starts_with(&parent_with_slash)
}

/// Check if a file/directory name is hidden (starts with .)
pub fn is_hidden(path: &str) -> bool {
    file_name(path)
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

/// Get MIME type from file extension
pub fn mime_type(path: &str) -> Option<String> {
    let ext = extension(path)?.to_lowercase();

    let mime = match ext.as_str() {
        // Text
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "md" => "text/markdown",
        "csv" => "text/csv",

        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",

        // Audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",

        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",

        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",

        // Code
        "py" => "text/x-python",
        "rs" => "text/x-rust",
        "java" => "text/x-java",
        "c" => "text/x-c",
        "cpp" | "cc" => "text/x-c++",
        "h" => "text/x-c-header",
        "sh" => "application/x-sh",

        // Other
        "wasm" => "application/wasm",

        _ => return None,
    };

    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join() {
        assert_eq!(join("/home", "Documents"), "/home/Documents");
        assert_eq!(join("/home/", "Documents"), "/home/Documents");
        assert_eq!(join("/home", "/Documents"), "/home/Documents");
        assert_eq!(join("/home/Documents", "file.txt"), "/home/Documents/file.txt");
    }

    #[test]
    fn test_parent() {
        assert_eq!(parent("/home/Documents/file.txt"), Some("/home/Documents".to_string()));
        assert_eq!(parent("/home/Documents"), Some("/home".to_string()));
        assert_eq!(parent("/home"), None);
    }

    #[test]
    fn test_file_name() {
        assert_eq!(file_name("/home/Documents/file.txt"), Some("file.txt".to_string()));
        assert_eq!(file_name("/home/Documents/"), Some("Documents".to_string()));
        assert_eq!(file_name("/home/.hidden"), Some(".hidden".to_string()));
    }

    #[test]
    fn test_extension() {
        assert_eq!(extension("/home/file.txt"), Some("txt".to_string()));
        assert_eq!(extension("/home/file.tar.gz"), Some("gz".to_string()));
        assert_eq!(extension("/home/.hidden"), None);
        assert_eq!(extension("/home/no_extension"), None);
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("/home/Documents/../Pictures"), "/home/Pictures");
        assert_eq!(normalize("/home/./Documents"), "/home/Documents");
        assert_eq!(normalize("/home//Documents///file.txt"), "/home/Documents/file.txt");
        assert_eq!(normalize("/home/../../../"), "/home");
        // SECURITY: Paths with .. that try to escape /home must stay within /home
        assert_eq!(normalize("/home/.system/../../.."), "/home");
        assert_eq!(normalize("/home/a/b/c/../../../.."), "/home");
        assert_eq!(normalize("/home/Documents/.."), "/home");
        assert_eq!(normalize("/home/.."), "/home");
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid("/home"));
        assert!(is_valid("/home/Documents"));
        assert!(!is_valid("/etc/passwd"));
        // Note: "/" normalizes to "/home" due to .. protection, so it's technically valid
        // This is intentional - we normalize before checking validity
        assert!(is_valid("/")); // normalizes to /home
    }

    #[test]
    fn test_is_child_of() {
        assert!(is_child_of("/home/Documents/file.txt", "/home/Documents"));
        assert!(is_child_of("/home/Documents", "/home"));
        assert!(!is_child_of("/home/Documents", "/home/Documents"));
        assert!(!is_child_of("/home/Pictures", "/home/Documents"));
    }

    #[test]
    fn test_sanitize_name() {
        // Already valid names stay the same
        assert_eq!(sanitize_name("file.txt"), "file.txt");
        assert_eq!(sanitize_name("my-document"), "my-document");
        assert_eq!(sanitize_name(".hidden"), ".hidden");

        // Emojis get replaced with underscore
        assert_eq!(sanitize_name("📁folder"), "_folder");
        assert_eq!(sanitize_name("test🎉"), "test_");
        assert_eq!(sanitize_name("hello🌍world"), "hello_world");

        // Bad characters get removed
        assert_eq!(sanitize_name("file<name"), "filename");
        assert_eq!(sanitize_name("file:name"), "filename");

        // Trailing dots/spaces get trimmed
        assert_eq!(sanitize_name("file."), "file");
        assert_eq!(sanitize_name("file "), "file");
    }

    #[test]
    fn test_validate_name() {
        // Valid names
        assert!(validate_name("file.txt").is_none());
        assert!(validate_name("my-document").is_none());
        assert!(validate_name(".hidden").is_none());
        assert!(validate_name("file with spaces").is_none());

        // Invalid: empty
        assert!(validate_name("").is_some());

        // Invalid: too long
        let long_name = "a".repeat(300);
        assert!(validate_name(&long_name).is_some());

        // Invalid: bad characters (would be sanitized differently)
        assert!(validate_name("file\0name").is_some());
        assert!(validate_name("file<name").is_some());
        assert!(validate_name("file>name").is_some());
        assert!(validate_name("file:name").is_some());
        assert!(validate_name("file|name").is_some());
        assert!(validate_name("file?name").is_some());
        assert!(validate_name("file*name").is_some());

        // Invalid: reserved names
        assert!(validate_name("CON").is_some());
        assert!(validate_name("con").is_some());
        assert!(validate_name("NUL.txt").is_some());

        // Invalid: trailing space/dot
        assert!(validate_name("file ").is_some());
        assert!(validate_name("file.").is_some());

        // Invalid: emojis (would be sanitized to underscore)
        assert!(validate_name("📁folder").is_some());
        assert!(validate_name("test🎉").is_some());
    }

    #[test]
    fn test_validate_path() {
        // Valid paths
        assert!(validate_path("/home/Documents/file.txt").is_none());
        assert!(validate_path("/home/.hidden/file").is_none());

        // Invalid: outside /home
        assert!(validate_path("/etc/passwd").is_some());
        // Note: "/" normalizes to "/home" which is valid
        assert!(validate_path("/").is_none()); // normalizes to /home

        // Invalid: bad segment
        assert!(validate_path("/home/file<name").is_some());
    }

    #[test]
    fn test_is_protected() {
        // Protected paths (cannot be deleted)
        assert!(is_protected("/home"));
        assert!(is_protected("/home/.system"));
        assert!(is_protected("/home/.system/apps"));
        assert!(is_protected("/home/.system/apps/terminal/main.py")); // Deep nesting protected
        assert!(is_protected("/home/.Trash"));

        // Not protected (can be deleted)
        assert!(!is_protected("/home/Documents"));
        assert!(!is_protected("/home/.Trash/old_file")); // Trash contents can be deleted
        assert!(!is_protected("/home/user_file.txt"));
    }

    #[test]
    fn test_can_write() {
        // Can write (create/modify files)
        assert!(can_write("/home/Documents"));
        assert!(can_write("/home/Documents/file.txt"));
        assert!(can_write("/home/.Trash/file"));
        assert!(can_write("/home/my_folder/deep/nested/file.txt"));

        // Cannot write (protected paths)
        assert!(!can_write("/home")); // Root itself
        assert!(!can_write("/home/.system"));
        assert!(!can_write("/home/.system/apps"));
        assert!(!can_write("/home/.system/apps/terminal"));
        assert!(!can_write("/home/.system/apps/terminal/main.py")); // Deep nesting protected
    }

    #[test]
    fn test_is_in_system() {
        assert!(is_in_system("/home/.system"));
        assert!(is_in_system("/home/.system/apps"));
        assert!(is_in_system("/home/.system/apps/terminal/main.py"));

        assert!(!is_in_system("/home"));
        assert!(!is_in_system("/home/Documents"));
        assert!(!is_in_system("/home/.systemconfig")); // Not .system/
    }
}
