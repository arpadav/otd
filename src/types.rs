//! Core data types and structures for the OTD server.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::atomic::{AtomicBool, AtomicU32}};
use smol::lock::RwLock;

/// Represents a downloadable item with one or more files/folders.
#[derive(Debug)]
pub struct DownloadItem {
    pub paths: Vec<PathBuf>,
    pub is_multi_file: bool,
    pub name: String,
    pub max_downloads: u32,
    pub download_count: AtomicU32,
    pub expires_at: Option<std::time::Instant>,
    pub created_at: std::time::Instant,
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub k: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub paths: Vec<String>,
    pub name: Option<String>,
    pub max_downloads: Option<u32>,
    pub expires_in_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub token: String,
    pub download_url: String,
}

#[derive(Debug, Serialize)]
pub struct StagedFile {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

pub struct AppState {
    pub tokens: RwLock<HashMap<String, DownloadItem>>,
    pub one_time_enabled: AtomicBool,
    pub base_path: PathBuf,
}

impl AppState {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
            one_time_enabled: AtomicBool::new(true),
            base_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_app_state_creation() {
        let base_path = PathBuf::from("/test/path");
        let state = AppState::new(base_path.clone());
        assert_eq!(state.base_path, base_path);
        assert!(state.one_time_enabled.load(Ordering::Relaxed));
    }

    #[test]
    fn test_download_item() {
        let item = DownloadItem {
            paths: vec![PathBuf::from("test.txt")],
            is_multi_file: false,
            name: "test.txt".to_string(),
            max_downloads: 3,
            download_count: AtomicU32::new(0),
            expires_at: None,
            created_at: std::time::Instant::now(),
        };
        assert_eq!(item.paths.len(), 1);
        assert!(!item.is_multi_file);
        assert_eq!(item.download_count.load(Ordering::Relaxed), 0);
        assert_eq!(item.name, "test.txt");
        assert_eq!(item.max_downloads, 3);
    }

    #[test]
    fn test_file_item_serialization() {
        let file = FileItem {
            name: "test.txt".to_string(),
            path: "folder/test.txt".to_string(),
            is_dir: false,
            size: Some(1024),
        };
        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("1024"));
        assert!(json.contains("false"));
    }
}
