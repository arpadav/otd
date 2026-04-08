//! Statistics response type for the dashboard API
//!
//! Used by the [`stats`][crate::core::links::stats] server function
//!
//! Author: aav

#[derive(Debug, serde::Deserialize, serde::Serialize)]
/// Dashboard statistics response payload
///
/// # Examples
///
/// ```
/// use otd::requests::StatsResponse;
///
/// let json = r#"{
///   "active_links": 3,
///   "used_links": 1,
///   "expired_links": 2,
///   "total_downloads": 42,
///   "uptime_seconds": 86400
/// }"#;
/// let stats: StatsResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(stats.active_links, 3);
/// assert_eq!(stats.total_downloads, 42);
/// ```
pub struct StatsResponse {
    /// Number of links that are still valid and have remaining downloads
    pub active_links: u32,
    /// Number of links that have reached their download limit
    pub used_links: u32,
    /// Number of links that have passed their expiration time
    pub expired_links: u32,
    /// Total downloads across all links
    pub total_downloads: u64,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_response_serde_roundtrip() {
        let stats = StatsResponse {
            active_links: 5,
            used_links: 2,
            expired_links: 1,
            total_downloads: 100,
            uptime_seconds: 3600,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let decoded: StatsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.active_links, stats.active_links);
        assert_eq!(decoded.used_links, stats.used_links);
        assert_eq!(decoded.expired_links, stats.expired_links);
        assert_eq!(decoded.total_downloads, stats.total_downloads);
        assert_eq!(decoded.uptime_seconds, stats.uptime_seconds);
    }
}
