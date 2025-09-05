use chrono::Utc;

// Constants for floating point precision
pub const BTC_PRECISION: u32 = 8;
pub const USD_PRECISION: u32 = 2;

// Convert USD price to cents (integer)
pub fn usd_to_cents(usd: f64) -> i64 {
    (usd * 100.0).round() as i64
}

// Convert cents back to USD
pub fn cents_to_usd(cents: i64) -> f64 {
    cents as f64 / 100.0
}

// Convert float to string with specified precision for database storage
pub fn float_to_db_string(value: f64, precision: u32) -> String {
    format!("{:.prec$}", value, prec = precision as usize)
}

// Convert database string back to float
pub fn db_string_to_float(value: &str) -> Result<f64, std::num::ParseFloatError> {
    value.parse()
}

// Format BTC with proper precision (8 decimals)
pub fn format_btc(btc: f64) -> String {
    format!("{:.8}", btc)
}

// Round BTC to proper precision
pub fn round_btc(btc: f64) -> f64 {
    let factor = 10f64.powi(BTC_PRECISION as i32);
    (btc * factor).round() / factor
}

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
    fn test_usd_to_cents() {
        assert_eq!(usd_to_cents(100.0), 10000);
        assert_eq!(usd_to_cents(100.50), 10050);
        assert_eq!(usd_to_cents(100.999), 10100); // rounds up
        assert_eq!(usd_to_cents(100.001), 10000); // rounds down
    }

    #[test]
    fn test_cents_to_usd() {
        assert_eq!(cents_to_usd(10000), 100.0);
        assert_eq!(cents_to_usd(10050), 100.5);
        assert_eq!(cents_to_usd(12345), 123.45);
    }

    #[test]
    fn test_float_to_db_string() {
        assert_eq!(float_to_db_string(0.12345678, 8), "0.12345678");
        assert_eq!(float_to_db_string(0.123456789, 8), "0.12345679"); // rounds
        assert_eq!(float_to_db_string(100.50, 2), "100.50");
    }

    #[test]
    fn test_db_string_to_float() {
        assert_eq!(db_string_to_float("0.12345678").unwrap(), 0.12345678);
        assert_eq!(db_string_to_float("100.50").unwrap(), 100.50);
        assert!(db_string_to_float("invalid").is_err());
    }

    #[test]
    fn test_format_btc() {
        assert_eq!(format_btc(0.12345678), "0.12345678");
        assert_eq!(format_btc(1.0), "1.00000000");
        assert_eq!(format_btc(0.00000001), "0.00000001");
    }

    #[test]
    fn test_round_btc() {
        assert_eq!(round_btc(0.123456789), 0.12345679);
        assert_eq!(round_btc(0.123456784), 0.12345678);
        assert_eq!(round_btc(1.0), 1.0);
        assert_eq!(round_btc(0.00000001), 0.00000001);
    }

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