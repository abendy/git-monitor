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
        return format!("{minutes} min ago");
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return format!("{hours} hr ago");
    }

    let days = duration.num_days();
    if days < 7 {
        return format!("{days} days ago");
    }

    let weeks = days / 7;
    if weeks < 4 {
        return format!("{weeks} wk ago");
    }

    let months = days / 30;
    if months < 12 {
        return format!("{months} mo ago");
    }

    let years = days / 365;
    format!("{years} yr ago")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn just_now_for_recent() {
        let now = Local::now();
        let time = now - Duration::seconds(30);

        let result = format_relative_time(time);

        assert_eq!(result, "just now");
    }

    #[test]
    fn minutes_ago() {
        let now = Local::now();
        let time = now - Duration::minutes(15);

        let result = format_relative_time(time);

        assert_eq!(result, "15 min ago");
    }

    #[test]
    fn hours_ago() {
        let now = Local::now();
        let time = now - Duration::hours(5);

        let result = format_relative_time(time);

        assert_eq!(result, "5 hr ago");
    }

    #[test]
    fn days_ago() {
        let now = Local::now();
        let time = now - Duration::days(3);

        let result = format_relative_time(time);

        assert_eq!(result, "3 days ago");
    }

    #[test]
    fn weeks_ago() {
        let now = Local::now();
        let time = now - Duration::weeks(2);

        let result = format_relative_time(time);

        assert_eq!(result, "2 wk ago");
    }

    #[test]
    fn months_ago() {
        let now = Local::now();
        let time = now - Duration::days(60);

        let result = format_relative_time(time);

        assert_eq!(result, "2 mo ago");
    }

    #[test]
    fn years_ago() {
        let now = Local::now();
        let time = now - Duration::days(400);

        let result = format_relative_time(time);

        assert_eq!(result, "1 yr ago");
    }

    #[test]
    fn boundary_59_seconds_is_just_now() {
        let now = Local::now();
        let time = now - Duration::seconds(59);

        let result = format_relative_time(time);

        assert_eq!(result, "just now");
    }

    #[test]
    fn boundary_60_seconds_is_1_min() {
        let now = Local::now();
        let time = now - Duration::seconds(60);

        let result = format_relative_time(time);

        assert_eq!(result, "1 min ago");
    }

    #[test]
    fn boundary_59_minutes_is_minutes() {
        let now = Local::now();
        let time = now - Duration::minutes(59);

        let result = format_relative_time(time);

        assert_eq!(result, "59 min ago");
    }

    #[test]
    fn boundary_60_minutes_is_1_hr() {
        let now = Local::now();
        let time = now - Duration::minutes(60);

        let result = format_relative_time(time);

        assert_eq!(result, "1 hr ago");
    }
}
