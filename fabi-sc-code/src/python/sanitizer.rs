//! HTML/JS Sanitizer
//!
//! Ensures that user-generated content cannot inject scripts or break out
//! of the Shadow DOM isolation.

/// Sanitize text content for safe HTML display
/// Escapes HTML special characters to prevent XSS
pub fn escape_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            '/' => result.push_str("&#x2F;"),
            '`' => result.push_str("&#x60;"),
            '=' => result.push_str("&#x3D;"),
            _ => result.push(c),
        }
    }
    result
}

/// Check if a string contains potentially dangerous patterns
/// This is a defense-in-depth measure - the primary protection is
/// that we generate HTML from Rust, not from Python strings
pub fn contains_script_patterns(text: &str) -> bool {
    let lower = text.to_lowercase();
    let dangerous_patterns = [
        "<script",
        "javascript:",
        "onerror=",
        "onload=",
        "onclick=",
        "onmouseover=",
        "onfocus=",
        "onblur=",
        "onsubmit=",
        "onchange=",
        "oninput=",
        "onkeydown=",
        "onkeyup=",
        "onkeypress=",
        "eval(",
        "expression(",
        "url(data:",
        "url(javascript:",
        "import(",
        "Function(",
        "setTimeout(",
        "setInterval(",
        "document.write",
        "document.cookie",
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
    ];

    for pattern in &dangerous_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }
    false
}

/// Sanitize a URL for use in href/src attributes
/// Returns None if the URL is potentially dangerous
pub fn sanitize_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();

    // Block javascript: and data: URLs (except safe data: images)
    if lower.starts_with("javascript:") {
        return None;
    }

    if lower.starts_with("data:") {
        // Only allow safe image data URLs
        if !lower.starts_with("data:image/png;")
            && !lower.starts_with("data:image/jpeg;")
            && !lower.starts_with("data:image/gif;")
            && !lower.starts_with("data:image/webp;")
            && !lower.starts_with("data:image/svg+xml;")
        {
            return None;
        }
    }

    // Block vbscript: URLs
    if lower.starts_with("vbscript:") {
        return None;
    }

    // Allow relative URLs, http(s), and VFS paths
    Some(trimmed.to_string())
}

/// Sanitize an attribute value for safe HTML
pub fn sanitize_attribute(value: &str) -> String {
    escape_html(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("hello & world"), "hello &amp; world");
        assert_eq!(escape_html("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn test_contains_script_patterns() {
        assert!(contains_script_patterns("<script>alert(1)</script>"));
        assert!(contains_script_patterns("javascript:alert(1)"));
        assert!(contains_script_patterns("onclick=alert(1)"));
        assert!(!contains_script_patterns("Hello World"));
        assert!(!contains_script_patterns("Normal text with script word"));
    }

    #[test]
    fn test_sanitize_url() {
        assert!(sanitize_url("javascript:alert(1)").is_none());
        assert!(sanitize_url("data:text/html,<script>").is_none());
        assert!(sanitize_url("data:image/png;base64,abc").is_some());
        assert!(sanitize_url("https://example.com").is_some());
        assert!(sanitize_url("/home/Documents/file.txt").is_some());
    }
}
