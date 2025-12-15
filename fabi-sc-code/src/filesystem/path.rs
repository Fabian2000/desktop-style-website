/// Path utilities for the virtual filesystem
/// All paths use forward slashes and start with /home/

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
pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                // Don't go above /home
                if parts.len() > 1 || (parts.len() == 1 && parts[0] != "home") {
                    parts.pop();
                }
            }
            _ => parts.push(part),
        }
    }

    if parts.is_empty() {
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
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid("/home"));
        assert!(is_valid("/home/Documents"));
        assert!(!is_valid("/etc/passwd"));
        assert!(!is_valid("/"));
    }

    #[test]
    fn test_is_child_of() {
        assert!(is_child_of("/home/Documents/file.txt", "/home/Documents"));
        assert!(is_child_of("/home/Documents", "/home"));
        assert!(!is_child_of("/home/Documents", "/home/Documents"));
        assert!(!is_child_of("/home/Pictures", "/home/Documents"));
    }
}
