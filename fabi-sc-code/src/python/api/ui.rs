//! UI Widget API for Python apps
//!
//! Provides functions to create UI widgets that are rendered into the
//! app's Shadow DOM. All HTML is generated in Rust to prevent XSS.

use crate::python::sanitizer;

/// A UI widget that can be rendered to HTML
#[derive(Clone)]
pub enum Widget {
    Label { text: String },
    Button { text: String, id: String },
    Input { placeholder: String, id: String, input_type: String },
    Checkbox { label: String, id: String, checked: bool },
    Image { src: String, alt: String },
    Row { children: Vec<Widget> },
    Column { children: Vec<Widget> },
    Spacer,
}

impl Widget {
    /// Render this widget to safe HTML
    pub fn to_html(&self) -> String {
        match self {
            Widget::Label { text } => {
                let escaped = sanitizer::escape_html(text);
                format!(r#"<div class="ui-label">{}</div>"#, escaped)
            }

            Widget::Button { text, id } => {
                let escaped_text = sanitizer::escape_html(text);
                let escaped_id = sanitizer::sanitize_attribute(id);
                format!(
                    r#"<button class="ui-button" data-widget-id="{}">{}</button>"#,
                    escaped_id, escaped_text
                )
            }

            Widget::Input { placeholder, id, input_type } => {
                let escaped_placeholder = sanitizer::sanitize_attribute(placeholder);
                let escaped_id = sanitizer::sanitize_attribute(id);
                let escaped_type = sanitizer::sanitize_attribute(input_type);
                format!(
                    r#"<input class="ui-input" type="{}" placeholder="{}" data-widget-id="{}">"#,
                    escaped_type, escaped_placeholder, escaped_id
                )
            }

            Widget::Checkbox { label, id, checked } => {
                let escaped_label = sanitizer::escape_html(label);
                let escaped_id = sanitizer::sanitize_attribute(id);
                let checked_attr = if *checked { " checked" } else { "" };
                format!(
                    r#"<label class="ui-checkbox"><input type="checkbox" data-widget-id="{}"{}/>{}</label>"#,
                    escaped_id, checked_attr, escaped_label
                )
            }

            Widget::Image { src, alt } => {
                let safe_src = sanitizer::sanitize_url(src)
                    .unwrap_or_else(|| "about:blank".to_string());
                let escaped_alt = sanitizer::sanitize_attribute(alt);
                format!(
                    r#"<img class="ui-image" src="{}" alt="{}">"#,
                    safe_src, escaped_alt
                )
            }

            Widget::Row { children } => {
                let inner: String = children.iter().map(|c| c.to_html()).collect();
                format!(r#"<div class="ui-row">{}</div>"#, inner)
            }

            Widget::Column { children } => {
                let inner: String = children.iter().map(|c| c.to_html()).collect();
                format!(r#"<div class="ui-column">{}</div>"#, inner)
            }

            Widget::Spacer => r#"<div class="ui-spacer"></div>"#.to_string(),
        }
    }
}

/// Generate a unique widget ID
pub fn generate_widget_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("widget-{}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_escapes_html() {
        let widget = Widget::Label {
            text: "<script>alert(1)</script>".to_string(),
        };
        let html = widget.to_html();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_button_renders() {
        let widget = Widget::Button {
            text: "Click me".to_string(),
            id: "btn-1".to_string(),
        };
        let html = widget.to_html();
        assert!(html.contains("ui-button"));
        assert!(html.contains("Click me"));
    }

    #[test]
    fn test_image_sanitizes_url() {
        let widget = Widget::Image {
            src: "javascript:alert(1)".to_string(),
            alt: "test".to_string(),
        };
        let html = widget.to_html();
        assert!(!html.contains("javascript:"));
        assert!(html.contains("about:blank"));
    }
}
