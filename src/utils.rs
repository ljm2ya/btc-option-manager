use chrono::Utc;

// Helper function to format expires timestamp to a readable string.
pub fn format_expires_timestamp(expires: i64) -> String {
    let now = Utc::now().timestamp();
    let duration_seconds = expires - now;
    
    if duration_seconds <= 0 {
        "EXPIRED".to_string()
    } else if duration_seconds < 3600 {
        let minutes = duration_seconds / 60;
        format!("{}m", minutes)
    } else if duration_seconds < 86400 {
        let hours = duration_seconds / 3600;
        format!("{}h", hours)
    } else {
        let days = duration_seconds / 86400;
        format!("{}d", days)
    }
}

// Helper function to parse duration strings (e.g., "30m", "1d") into a year fraction.
pub fn parse_duration(duration: &str) -> f64 {
    let d = duration.trim();
    let (num_str, unit) = d.split_at(d.len() - 1);
    let num: f64 = num_str.parse().unwrap_or(0.0);
    match unit {
        "m" => num / (365.0 * 24.0 * 60.0),
        "h" => num / (365.0 * 24.0),
        "d" => num / 365.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_expires_timestamp_expired() {
        let past = Utc::now().timestamp() - 100;
        assert_eq!(format_expires_timestamp(past), "EXPIRED");
    }

    #[test]
    fn test_format_expires_timestamp_minutes() {
        let future = Utc::now().timestamp() + 1800; // 30 minutes
        assert_eq!(format_expires_timestamp(future), "30m");
    }

    #[test]
    fn test_format_expires_timestamp_hours() {
        let future = Utc::now().timestamp() + 7200; // 2 hours
        assert_eq!(format_expires_timestamp(future), "2h");
    }

    #[test]
    fn test_format_expires_timestamp_days() {
        let future = Utc::now().timestamp() + 172800; // 2 days
        assert_eq!(format_expires_timestamp(future), "2d");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30m"), 30.0 / (365.0 * 24.0 * 60.0));
        assert_eq!(parse_duration("2h"), 2.0 / (365.0 * 24.0));
        assert_eq!(parse_duration("7d"), 7.0 / 365.0);
        assert_eq!(parse_duration("invalid"), 0.0);
    }
}