//! System information API for Python apps
//!
//! Provides read-only access to system information.

/// Get the current time as a formatted string (HH:MM)
pub fn time() -> String {
    let date = js_sys::Date::new_0();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    format!("{:02}:{:02}", hours, minutes)
}

/// Get the current date as a formatted string (e.g., "15. Dez 2025")
pub fn date() -> String {
    let date = js_sys::Date::new_0();
    let day = date.get_date();
    let month = date.get_month();
    let year = date.get_full_year();

    let month_names = [
        "Jan", "Feb", "Mär", "Apr", "Mai", "Jun",
        "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"
    ];

    let month_name = month_names.get(month as usize).unwrap_or(&"???");
    format!("{}. {} {}", day, month_name, year)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a JS environment
    // They will be tested in the browser
}
