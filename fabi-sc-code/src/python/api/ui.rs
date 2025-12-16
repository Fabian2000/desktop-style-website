//! UI Widget API for Python apps
//!
//! Provides functions to create UI widgets that are rendered into the
//! app's Shadow DOM. All HTML is generated in Rust to prevent XSS.
//!
//! Style system: widgets accept a `style` parameter (from ui.style()) plus
//! additional kwargs that override/extend the style.

use crate::python::sanitizer;
use std::collections::HashMap;

/// CSS property name mapping (Python snake_case -> CSS kebab-case)
fn to_css_property(name: &str) -> String {
    name.replace('_', "-")
}

/// Sanitize a CSS value (prevent injection)
fn sanitize_css_value(value: &str) -> String {
    // Remove dangerous characters that could break out of CSS
    value
        .chars()
        .filter(|c| !matches!(c, ';' | '{' | '}' | '<' | '>' | '"' | '\''))
        .collect()
}

/// A reusable style object
#[derive(Clone, Default)]
pub struct Style {
    pub properties: HashMap<String, String>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a property
    pub fn set(&mut self, key: &str, value: &str) {
        self.properties.insert(to_css_property(key), sanitize_css_value(value));
    }

    /// Merge another style into this one (other takes precedence)
    pub fn merge(&mut self, other: &Style) {
        for (k, v) in &other.properties {
            self.properties.insert(k.clone(), v.clone());
        }
    }

    /// Convert to inline style string
    pub fn to_inline(&self) -> String {
        if self.properties.is_empty() {
            String::new()
        } else {
            let props: Vec<String> = self.properties
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            format!(" style=\"{}\"", props.join("; "))
        }
    }
}

/// Generate a unique widget ID
pub fn generate_widget_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("widget-{}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Render a text/label element
pub fn render_text(content: &str, style: &Style) -> String {
    let escaped = sanitizer::escape_html(content);
    // Use <pre> to preserve whitespace/newlines for terminal-like output
    format!(r#"<pre class="ui-text"{}>{}</pre>"#, style.to_inline(), escaped)
}

/// Render a label (single line, no pre)
pub fn render_label(content: &str, style: &Style) -> String {
    let escaped = sanitizer::escape_html(content);
    format!(r#"<span class="ui-label"{}>{}</span>"#, style.to_inline(), escaped)
}

/// Render a button
pub fn render_button(text: &str, style: &Style) -> String {
    let escaped = sanitizer::escape_html(text);
    let id = generate_widget_id();
    format!(
        r#"<button class="ui-button" data-widget-id="{}"{} >{}</button>"#,
        sanitizer::sanitize_attribute(&id),
        style.to_inline(),
        escaped
    )
}

/// Render an input field
/// If on_submit is provided, the input will dispatch a custom event when Enter is pressed
pub fn render_input(placeholder: &str, on_submit: Option<&str>, style: &Style) -> String {
    let id = generate_widget_id();
    let on_submit_attr = on_submit
        .map(|name| format!(r#" data-on-submit="{}""#, sanitizer::sanitize_attribute(name)))
        .unwrap_or_default();
    format!(
        r#"<input class="ui-input" type="text" placeholder="{}" data-widget-id="{}"{}{} />"#,
        sanitizer::sanitize_attribute(placeholder),
        sanitizer::sanitize_attribute(&id),
        on_submit_attr,
        style.to_inline()
    )
}

/// Render a container (div)
pub fn render_container(children: &str, style: &Style) -> String {
    format!(r#"<div class="ui-container"{}>{}</div>"#, style.to_inline(), children)
}

/// Render a row (flex horizontal)
pub fn render_row(children: &str, style: &Style) -> String {
    // Row always has display: flex, but style can override
    let mut full_style = Style::new();
    full_style.set("display", "flex");
    full_style.set("flex_direction", "row");
    full_style.set("align_items", "center");
    full_style.set("gap", "8px");
    full_style.merge(style);
    format!(r#"<div class="ui-row"{}>{}</div>"#, full_style.to_inline(), children)
}

/// Render a column (flex vertical)
pub fn render_column(children: &str, style: &Style) -> String {
    let mut full_style = Style::new();
    full_style.set("display", "flex");
    full_style.set("flex_direction", "column");
    full_style.set("gap", "4px");
    full_style.merge(style);
    format!(r#"<div class="ui-column"{}>{}</div>"#, full_style.to_inline(), children)
}

/// Render a spacer (flexible empty space)
pub fn render_spacer() -> String {
    r#"<div class="ui-spacer" style="flex: 1;"></div>"#.to_string()
}

/// Render an image
pub fn render_image(src: &str, alt: &str, style: &Style) -> String {
    let safe_src = sanitizer::sanitize_url(src)
        .unwrap_or_else(|| "about:blank".to_string());
    let escaped_alt = sanitizer::sanitize_attribute(alt);
    format!(
        r#"<img class="ui-image" src="{}" alt="{}"{} />"#,
        safe_src, escaped_alt, style.to_inline()
    )
}

/// Render a checkbox
pub fn render_checkbox(label: &str, checked: bool, style: &Style) -> String {
    let id = generate_widget_id();
    let escaped_label = sanitizer::escape_html(label);
    let checked_attr = if checked { " checked" } else { "" };
    format!(
        r#"<label class="ui-checkbox"{}><input type="checkbox" data-widget-id="{}"{} /><span>{}</span></label>"#,
        style.to_inline(),
        sanitizer::sanitize_attribute(&id),
        checked_attr,
        escaped_label
    )
}

/// Render a radio button
pub fn render_radio(label: &str, name: &str, checked: bool, style: &Style) -> String {
    let id = generate_widget_id();
    let escaped_label = sanitizer::escape_html(label);
    let escaped_name = sanitizer::sanitize_attribute(name);
    let checked_attr = if checked { " checked" } else { "" };
    format!(
        r#"<label class="ui-radio"{}><input type="radio" name="{}" data-widget-id="{}"{} /><span>{}</span></label>"#,
        style.to_inline(),
        escaped_name,
        sanitizer::sanitize_attribute(&id),
        checked_attr,
        escaped_label
    )
}

/// Render a select/dropdown
pub fn render_select(options: &[String], selected: Option<usize>, style: &Style) -> String {
    let id = generate_widget_id();
    let options_html: String = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let escaped = sanitizer::escape_html(opt);
            let selected_attr = if Some(i) == selected { " selected" } else { "" };
            format!(r#"<option value="{}"{}>{}</option>"#, i, selected_attr, escaped)
        })
        .collect();
    format!(
        r#"<select class="ui-select" data-widget-id="{}"{}>{}</select>"#,
        sanitizer::sanitize_attribute(&id),
        style.to_inline(),
        options_html
    )
}

/// Render a progress bar
pub fn render_progress(value: f64, max: f64, style: &Style) -> String {
    let percentage = if max > 0.0 { (value / max * 100.0).min(100.0).max(0.0) } else { 0.0 };
    let mut full_style = Style::new();
    full_style.set("background", "#333");
    full_style.set("border_radius", "4px");
    full_style.set("overflow", "hidden");
    full_style.set("height", "8px");
    full_style.merge(style);
    format!(
        r#"<div class="ui-progress"{}><div style="width: {:.1}%; height: 100%; background: #4caf50;"></div></div>"#,
        full_style.to_inline(),
        percentage
    )
}

/// Render a divider/separator line
pub fn render_divider(style: &Style) -> String {
    let mut full_style = Style::new();
    full_style.set("border", "none");
    full_style.set("border_top", "1px solid #444");
    full_style.set("margin", "8px 0");
    full_style.merge(style);
    format!(r#"<hr class="ui-divider"{} />"#, full_style.to_inline())
}

/// Render a link
pub fn render_link(text: &str, href: &str, style: &Style) -> String {
    let escaped_text = sanitizer::escape_html(text);
    let safe_href = sanitizer::sanitize_url(href)
        .unwrap_or_else(|| "#".to_string());
    format!(
        r#"<a class="ui-link" href="{}" target="_blank" rel="noopener"{}>{}</a>"#,
        safe_href,
        style.to_inline(),
        escaped_text
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_to_inline() {
        let mut style = Style::new();
        style.set("color", "#0f0");
        style.set("font_size", "14px");
        let inline = style.to_inline();
        assert!(inline.contains("color: #0f0"));
        assert!(inline.contains("font-size: 14px"));
    }

    #[test]
    fn test_style_merge() {
        let mut base = Style::new();
        base.set("color", "red");
        base.set("font_size", "12px");

        let mut override_style = Style::new();
        override_style.set("color", "blue");

        base.merge(&override_style);
        assert_eq!(base.properties.get("color"), Some(&"blue".to_string()));
        assert_eq!(base.properties.get("font-size"), Some(&"12px".to_string()));
    }

    #[test]
    fn test_css_value_sanitization() {
        let dangerous = "red; background: url(javascript:alert(1))";
        let safe = sanitize_css_value(dangerous);
        assert!(!safe.contains(';'));
    }

    #[test]
    fn test_text_escapes_html() {
        let style = Style::new();
        let html = render_text("<script>alert(1)</script>", &style);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }
}
