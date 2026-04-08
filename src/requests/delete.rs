//! Response type for bulk-delete and single-delete API operations
//!
//! Used by the [`delete_link`][crate::core::links::delete_link] and
//! [`bulk_delete_links`][crate::core::links::bulk_delete_links] server functions
//!
//! Author: aav

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// Response payload for bulk-delete and single-delete operations
///
/// # Examples
///
/// ```
/// use otd::requests::BulkDeleteResponse;
///
/// let json = r#"{"removed":3}"#;
/// let resp: BulkDeleteResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(resp.removed, 3);
/// ```
pub struct BulkDeleteResponse {
    /// Number of links removed
    pub removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_delete_response_serde_roundtrip() {
        let resp = BulkDeleteResponse { removed: 7 };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: BulkDeleteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.removed, resp.removed);
    }

    #[test]
    fn bulk_delete_response_zero_removed() {
        let json = r#"{"removed":0}"#;
        let resp: BulkDeleteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.removed, 0);
    }
}
