use chrono::{DateTime, Local};

/// Format a timestamp as relative time (e.g., "2 hr ago", "3 days ago")
pub fn format_relative_time(time: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(time);

    let seconds = duration.num_seconds();
    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        return format!("{} min ago", minutes);
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return format!("{} hr ago", hours);
    }

    let days = duration.num_days();
    if days < 7 {
        return format!("{} days ago", days);
    }

    let weeks = days / 7;
    if weeks < 4 {
        return format!("{} wk ago", weeks);
    }

    let months = days / 30;
    if months < 12 {
        return format!("{} mo ago", months);
    }

    let years = days / 365;
    format!("{} yr ago", years)
}
