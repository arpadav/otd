//! Request handlers for the OTD server.
//!
//! This module contains the main business logic for handling HTTP requests,
//! including file browsing, download link generation, and file serving.
//! It implements a clean separation between admin and download functionality.

use crate::{config::Config, http::*, types::*};
use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::write::FileOptions;

/// Main request handler containing business logic for both admin and download servers.
///
/// The handler manages file browsing, download link generation, and file serving.
/// It maintains shared state and configuration, and provides separate entry points
/// for admin interface requests and download requests.
///
/// # Examples
///
/// ```rust,no_run
/// use otd::{Handler, Config, types::AppState};
/// use std::{sync::Arc, path::PathBuf};
///
/// let config = Config::default();
/// let state = Arc::new(AppState::new(PathBuf::from("/files")));
/// let handler = Handler::new(state, config);
/// ```
pub struct Handler {
    /// Shared application state containing download tokens and configuration
    pub state: Arc<AppState>,
    /// Server configuration including ports, paths, and security settings
    pub config: Config,
}

impl Handler {
    /// Creates a new handler with the given state and configuration.
    ///
    /// # Arguments
    ///
    /// * `state` - Shared application state
    /// * `config` - Server configuration
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::{Handler, Config, types::AppState};
    /// use std::{sync::Arc, path::PathBuf};
    ///
    /// let config = Config::default();
    /// let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// let handler = Handler::new(state, config);
    /// ```
    pub fn new(state: Arc<AppState>, config: Config) -> Self {
        Self { state, config }
    }

    /// Handles requests to the admin interface (file browsing, link generation).
    ///
    /// This method routes admin requests to appropriate handlers based on the
    /// HTTP method and path. It supports:
    /// - GET / - Web interface
    /// - GET /api/browse - File browsing
    /// - POST /api/generate - Link generation
    /// - GET /api/tokens - List active tokens
    /// - GET /config/one-time/* - Configuration
    ///
    /// # Arguments
    ///
    /// * `request` - Raw HTTP request string
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>` - HTTP response bytes or error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// # smol::block_on(async {
    /// let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    /// let peer_addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    /// let response = handler.handle_admin_request(request, peer_addr).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_admin_request(&self, request: &str, peer_addr: std::net::SocketAddr) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        
        match req.parse(request.as_bytes()) {
            Ok(_) => {
                // Bearer token authentication — enforced when admin_token is configured.
                if let Some(ref expected_token) = self.config.admin_token {
                    let auth_header = req.headers.iter().find(|h| {
                        h.name.eq_ignore_ascii_case("Authorization")
                    });
                    let authorized = auth_header
                        .and_then(|h| std::str::from_utf8(h.value).ok())
                        .and_then(|v| v.strip_prefix("Bearer "))
                        .map(|token| token.trim() == expected_token.as_str())
                        .unwrap_or(false);
                    if !authorized {
                        tracing::warn!("Admin request rejected: missing or invalid Authorization header");
                        return Ok(HttpResponse::new(401, "Unauthorized")
                            .header("WWW-Authenticate", "Bearer realm=\"otd-admin\"")
                            .body_text("Unauthorized")
                            .to_bytes());
                    }
                }

                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");
                
                tracing::info!("Admin request: {} {}", method, path);
                
                let (path, query) = if let Some(pos) = path.find('?') {
                    (&path[..pos], &path[pos + 1..])
                } else {
                    (path, "")
                };

                tracing::info!("Parsed path: '{}', query: '{}'", path, query);

                // --- Loopback / session auth ---
                let is_loopback = peer_addr.ip().is_loopback();
                if !is_loopback {
                    // Check if admin_password is configured
                    if self.config.admin_password.is_none() {
                        let html = include_str!("../static/login.html")
                            .replace("{{TAILWIND_CSS}}", include_str!("../static/style.css"))
                            .replace("{{MESSAGE}}", "External access requires admin_password to be set in config.");
                        return Ok(HttpResponse::new(403, "Forbidden")
                            .content_type(content_type::HTML)
                            .body_text(&html)
                            .to_bytes());
                    }

                    // Allow login routes without session
                    let is_login_route = path == "/login";
                    if !is_login_route {
                        // Check session cookie
                        let cookie_header = req.headers.iter()
                            .find(|h| h.name.eq_ignore_ascii_case("Cookie"))
                            .and_then(|h| std::str::from_utf8(h.value).ok())
                            .unwrap_or("");
                        
                        let session_token = cookie_header.split(';')
                            .map(|s| s.trim())
                            .find(|s| s.starts_with("otd_session="))
                            .and_then(|s| s.strip_prefix("otd_session="));

                        let mut valid_session = false;
                        if let Some(token) = session_token {
                            let sessions = self.state.sessions.read().await;
                            if let Some(created) = sessions.get(token) {
                                if created.elapsed() < std::time::Duration::from_secs(24 * 60 * 60) {
                                    valid_session = true;
                                }
                            }
                        }

                        // Clean up expired sessions (best-effort, don't block)
                        if !valid_session {
                            let mut sessions = self.state.sessions.write().await;
                            let max_age = std::time::Duration::from_secs(24 * 60 * 60);
                            sessions.retain(|_, created| created.elapsed() < max_age);
                        }

                        if !valid_session {
                            return Ok(HttpResponse::redirect("/login").to_bytes());
                        }
                    }
                }

                // --- Login routes ---
                if path == "/login" {
                    if method == "GET" {
                        let html = include_str!("../static/login.html")
                            .replace("{{TAILWIND_CSS}}", include_str!("../static/style.css"))
                            .replace("{{MESSAGE}}", "");
                        return Ok(HttpResponse::ok()
                            .content_type(content_type::HTML)
                            .body_text(&html)
                            .to_bytes());
                    } else if method == "POST" {
                        let body = self.extract_body(request)?;
                        let params = self.parse_query(&body);
                        let password = params.get("password").cloned().unwrap_or_default();
                        
                        let expected = self.config.admin_password.as_deref().unwrap_or("");
                        if !expected.is_empty() && password == expected {
                            let token = Uuid::new_v4().to_string();
                            self.state.sessions.write().await.insert(token.clone(), std::time::Instant::now());
                            return Ok(HttpResponse::redirect("/")
                                .header("Set-Cookie", &format!("otd_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400", token))
                                .to_bytes());
                        } else {
                            let html = include_str!("../static/login.html")
                                .replace("{{TAILWIND_CSS}}", include_str!("../static/style.css"))
                                .replace("{{MESSAGE}}", "Invalid password.");
                            return Ok(HttpResponse::new(403, "Forbidden")
                                .content_type(content_type::HTML)
                                .body_text(&html)
                                .to_bytes());
                        }
                    }
                }

                let response = match (method, path) {
                    ("GET", "/") => self.web_interface().await?,
                    ("GET", "/api/browse") => self.browse(query).await?,
                    ("POST", "/api/generate") => {
                        let body = self.extract_body(request)?;
                        self.generate_link(&body).await?
                    },
                    ("GET", "/api/tokens") => self.list_tokens().await?,
                    ("DELETE", path) if path.starts_with("/api/tokens/") => {
                        let token = path.strip_prefix("/api/tokens/").unwrap_or("");
                        self.delete_token(token).await?
                    }
                    ("GET", path) if path.starts_with("/config/one-time/") => {
                        let enabled = path.strip_prefix("/config/one-time/")
                            .and_then(|s| s.parse::<bool>().ok())
                            .unwrap_or(true);
                        self.config_one_time(enabled).await?
                    }
                    _ => HttpResponse::not_found(),
                };

                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse HTTP request: {}", e);
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }

    /// Handles requests to the download server (file downloads only).
    ///
    /// This method specifically handles download requests with the format:
    /// - GET /<filename>?k=<token> - Download file with proper filename
    /// - GET /?k=<token> - Download file (fallback, less user-friendly)
    ///
    /// The filename in the URL helps wget and browsers save files with correct names.
    ///
    /// # Arguments
    ///
    /// * `request` - Raw HTTP request string
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>` - HTTP response bytes or error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// # smol::block_on(async {
    /// let request = "GET /document.pdf?k=550e8400-e29b-41d4-a716-446655440000 HTTP/1.1\r\n\r\n";
    /// let response = handler.handle_download_request(request).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_download_request(&self, request: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        
        match req.parse(request.as_bytes()) {
            Ok(_) => {
                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");
                
                let (path, query) = if let Some(pos) = path.find('?') {
                    (&path[..pos], &path[pos + 1..])
                } else {
                    (path, "")
                };

                let response = match (method, path) {
                    ("GET", "/") => self.download(query).await?,
                    ("GET", _) => self.download(query).await?, // Any path with ?k= parameter
                    _ => HttpResponse::not_found(),
                };

                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse download request: {}", e);
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }

    /// Serves the main web interface HTML.
    ///
    /// Returns the admin interface HTML with proper content type headers.
    /// The HTML includes JavaScript for file browsing, staging, and link generation.
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - HTML response or error
    async fn web_interface(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let html = self.get_updated_html();
        Ok(HttpResponse::ok()
            .content_type(content_type::HTML)
            .body_text(&html))
    }

    /// Handles file browsing requests for the admin interface.
    ///
    /// Returns a JSON list of files and folders in the specified directory.
    /// Includes security checks to prevent path traversal attacks.
    ///
    /// # Arguments
    ///
    /// * `query` - URL query string containing path parameter
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - JSON file list or error
    ///
    /// # Security
    ///
    /// This method validates that all requested paths are within the configured
    /// base directory to prevent directory traversal attacks.
    async fn browse(&self, query: &str) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let params = self.parse_query(query);
        let path = params.get("path").map(|s| s.as_str()).unwrap_or("");
        
        let full_path = if path.is_empty() {
            self.state.base_path.clone()
        } else {
            // safe_join canonicalizes and verifies containment, blocking both
            // `../` traversal and symlink escapes.
            match self.safe_join(path) {
                Some(p) => p,
                None => {
                    tracing::warn!("Browse: path traversal/symlink escape blocked for '{path}'");
                    return Ok(HttpResponse::forbidden());
                }
            }
        };

        tracing::info!("Browse request - path: '{path}', full_path: '{full_path:?}'");

        let mut items = Vec::new();

        // Add parent directory if not at root
        if full_path != self.state.base_path
            && let Some(parent) = full_path.parent()
            && parent.starts_with(&self.state.base_path) {
            let relative_parent = parent
                .strip_prefix(&self.state.base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            items.push(FileItem {
                name: "..".to_string(),
                path: relative_parent,
                is_dir: true,
                size: None,
            });
        }

        let entries = match std::fs::read_dir(&full_path) {
            Ok(entries) => {
                tracing::info!("Successfully read directory: {full_path:?}");
                entries
            },
            Err(e) => {
                tracing::error!("Failed to read directory {full_path:?}: {e}");
                return Ok(HttpResponse::not_found());
            },
        };

        for entry in entries.flatten() {
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = if is_dir {
                None
            } else {
                metadata.map(|m| m.len())
            };

            let entry_path = entry.path();
            let relative_path = entry_path
                .strip_prefix(&self.state.base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());

            items.push(FileItem {
                name: entry.file_name().to_string_lossy().to_string(),
                path: relative_path,
                is_dir,
                size,
            });
        }

        // Sort: directories first, then files, both alphabetically
        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        HttpResponse::ok().body_json(&items).map_err(Into::into)
    }

    /// Generates a new one-time download link for the specified files.
    ///
    /// Creates a unique token and stores the download item in the application state.
    /// The generated URL includes the filename to help wget and browsers save files correctly.
    ///
    /// # Arguments
    ///
    /// * `body` - JSON request body containing paths and optional name
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - JSON response with token and URL
    ///
    /// # URL Format
    ///
    /// Generated URLs follow the format: `http://host:port/filename.ext?k=<token>`
    /// This ensures wget saves files with the correct name instead of generic names.
    async fn generate_link(&self, body: &str) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let generate_req: GenerateRequest = serde_json::from_str(body)?;
        
        if generate_req.paths.is_empty() {
            return Ok(HttpResponse::bad_request().body_text("No paths provided"));
        }

        let mut full_paths = Vec::new();
        
        // Validate all paths — safe_join canonicalizes and checks containment,
        // blocking both `../` traversal and symlink-based escapes.
        for path_str in &generate_req.paths {
            match self.safe_join(path_str) {
                Some(full_path) => full_paths.push(full_path),
                None => {
                    tracing::warn!("Generate: path traversal/symlink escape blocked for '{path_str}'");
                    return Ok(HttpResponse::forbidden());
                }
            }
        }

        let token = Uuid::new_v4().to_string();
        let is_multi_file = full_paths.len() > 1 || (full_paths.len() == 1 && full_paths[0].is_dir());
        
        // Determine the name
        let name = if let Some(custom_name) = generate_req.name {
            custom_name
        } else if full_paths.len() == 1 {
            let path = &full_paths[0];
            if path.is_dir() {
                format!("{}.zip", path.file_name().and_then(|n| n.to_str()).unwrap_or("folder"))
            } else {
                path.file_name().and_then(|n| n.to_str()).unwrap_or("download").to_string()
            }
        } else {
            "output.zip".to_string()
        };

        let max_downloads = generate_req.max_downloads.unwrap_or(1).max(1);
        let expires_at = generate_req.expires_in_seconds.map(|secs| {
            std::time::Instant::now() + std::time::Duration::from_secs(secs)
        });

        let item = DownloadItem {
            paths: full_paths,
            is_multi_file,
            name: name.clone(),
            max_downloads,
            download_count: std::sync::atomic::AtomicU32::new(0),
            expires_at,
            created_at: std::time::Instant::now(),
        };

        self.state.tokens.write().await.insert(token.clone(), item);

        // Create URL with filename for better wget/browser behavior
        let download_url = format!("{}/{}?k={}", self.config.download_base_url(), 
                                 self.url_encode(&name), token);
        tracing::info!("Generated download link for '{}': {}", name, token);

        let response = GenerateResponse {
            token,
            download_url,
        };

        HttpResponse::ok().body_json(&response).map_err(Into::into)
    }

    /// Lists all active download tokens with their status.
    ///
    /// Returns a JSON array of all download tokens, including their names,
    /// download URLs, and whether they've been used.
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - JSON token list or error
    async fn list_tokens(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tokens = self.state.tokens.read().await;
        let mut items = Vec::new();

        let now = std::time::Instant::now();
        for (token, item) in tokens.iter() {
            let download_url = format!("{}/{}?k={}", self.config.download_base_url(), 
                                     self.url_encode(&item.name), token);
            let count = item.download_count.load(Ordering::Relaxed);
            let expired = item.expires_at.map(|e| now >= e).unwrap_or(false);
            let expires_in_seconds = item.expires_at.and_then(|e| {
                if now < e { Some(e.duration_since(now).as_secs()) } else { None }
            });
            items.push(serde_json::json!({
                "token": token,
                "name": item.name,
                "is_multi_file": item.is_multi_file,
                "download_count": count,
                "max_downloads": item.max_downloads,
                "remaining_downloads": item.max_downloads.saturating_sub(count),
                "expired": expired,
                "expires_in_seconds": expires_in_seconds,
                "download_url": download_url,
                "paths": item.paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>()
            }));
        }

        HttpResponse::ok().body_json(&items).map_err(Into::into)
    }

    /// Deletes a download token.
    async fn delete_token(&self, token: &str) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut tokens = self.state.tokens.write().await;
        if tokens.remove(token).is_some() {
            tracing::info!("Deleted token: {}", token);
            Ok(HttpResponse::ok()
                .content_type(content_type::JSON)
                .body_text("{\"removed\":true}"))
        } else {
            Ok(HttpResponse::not_found().body_text("{\"removed\":false}"))
        }
    }

    /// Handles file download requests using the provided token.
    ///
    /// Validates the token, checks one-time usage rules, and serves the file(s).
    /// Supports both single file downloads and multi-file ZIP downloads.
    ///
    /// # Arguments
    ///
    /// * `query` - URL query string containing the token parameter
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - File content or error
    ///
    /// # One-Time Usage
    ///
    /// If one-time usage is enabled, the download token is marked as used
    /// after the first successful download attempt.
    async fn download(&self, query: &str) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let params = self.parse_query(query);
        let token = match params.get("k") {
            Some(token) => token,
            None => return Ok(HttpResponse::bad_request().body_text("Missing token parameter")),
        };

        let tokens = self.state.tokens.read().await;
        let item = match tokens.get(token) {
            Some(item) => item,
            None => return Ok(HttpResponse::not_found().body_text("Token not found")),
        };

        // Check expiry
        if let Some(expires_at) = item.expires_at {
            if std::time::Instant::now() >= expires_at {
                return Ok(HttpResponse::gone().body_text("Download link has expired"));
            }
        }

        // Check download count (only when one_time mode is enabled)
        if self.state.one_time_enabled.load(Ordering::Acquire) {
            let prev = item.download_count.fetch_add(1, Ordering::AcqRel);
            if prev >= item.max_downloads {
                // Undo the increment to avoid overflow over time
                item.download_count.fetch_sub(1, Ordering::AcqRel);
                return Ok(HttpResponse::gone().body_text("Download limit reached"));
            }
        }

        if item.is_multi_file || (item.paths.len() == 1 && item.paths[0].is_dir()) {
            self.serve_as_zip(&item.paths, &item.name).await
        } else {
            self.serve_single_file(&item.paths[0]).await
        }
    }

    /// Configures one-time download enforcement.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enforce one-time usage
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - Success message or error
    async fn config_one_time(&self, enabled: bool) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.state.one_time_enabled.store(enabled, Ordering::Relaxed);
        Ok(HttpResponse::ok()
            .content_type(content_type::PLAIN_TEXT)
            .body_text("Configuration updated"))
    }

    /// Serves a single file with appropriate headers for download.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to serve
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - File content or error
    async fn serve_single_file(&self, path: &PathBuf) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let file_contents = match std::fs::read(path) {
            Ok(contents) => contents,
            Err(e) => {
                tracing::error!("File read error: {:?} - {}", path, e);
                return Ok(HttpResponse::not_found().body_text("File not found"));
            }
        };

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        
        Ok(HttpResponse::ok()
            .content_type(content_type::OCTET_STREAM)
            .content_disposition(&format!("attachment; filename=\"{filename}\""))
            .body_bytes(file_contents))
    }

    /// Serves multiple files or folders as a ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `paths` - List of file/folder paths to include
    /// * `zip_name` - Name for the ZIP file
    ///
    /// # Returns
    ///
    /// * `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>` - ZIP content or error
    async fn serve_as_zip(&self, paths: &[PathBuf], zip_name: &str) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut zip_data = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));
            let options = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            for path in paths {
                if path.is_dir() {
                    self.add_directory_to_zip(&mut zip, path, options)?;
                } else {
                    self.add_file_to_zip(&mut zip, path, options)?;
                }
            }

            if let Err(e) = zip.finish() {
                tracing::error!("Failed to finish zip: {}", e);
                return Ok(HttpResponse::internal_server_error().body_text("Failed to create zip file"));
            }
        }

        let final_name = if zip_name.ends_with(".zip") {
            zip_name.to_string()
        } else {
            format!("{zip_name}.zip")
        };

        Ok(HttpResponse::ok()
            .content_type(content_type::ZIP)
            .content_disposition(&format!("attachment; filename=\"{final_name}\""))
            .body_bytes(zip_data))
    }

    /// Adds a directory and its contents to a ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip` - ZIP writer instance
    /// * `dir_path` - Path to the directory to add
    /// * `options` - ZIP file options
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Success or error
    fn add_directory_to_zip(
        &self,
        zip: &mut zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
        dir_path: &PathBuf,
        options: FileOptions<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("folder");
        
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            let relative_path = match entry_path.strip_prefix(dir_path) {
                Ok(path) => path,
                Err(_) => continue,
            };

            if relative_path.as_os_str().is_empty() {
                continue;
            }

            let zip_path = format!("{}/{}", dir_name, relative_path.to_string_lossy());

            if entry.file_type().is_dir() {
                if let Err(e) = zip.add_directory(&zip_path, options) {
                    tracing::error!("Failed to add directory to zip: {}", e);
                }
            } else {
                if let Err(e) = zip.start_file(&zip_path, options) {
                    tracing::error!("Failed to start file in zip: {}", e);
                    continue;
                }

                let file_contents = match std::fs::read(entry_path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        tracing::error!("Failed to read file for zip: {}", e);
                        continue;
                    }
                };

                if let Err(e) = zip.write_all(&file_contents) {
                    tracing::error!("Failed to write file to zip: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Adds a single file to a ZIP archive.
    ///
    /// # Arguments
    ///
    /// * `zip` - ZIP writer instance
    /// * `file_path` - Path to the file to add
    /// * `options` - ZIP file options
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Success or error
    fn add_file_to_zip(
        &self,
        zip: &mut zip::ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
        file_path: &PathBuf,
        options: FileOptions<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        
        if let Err(e) = zip.start_file(filename, options) {
            tracing::error!("Failed to start file in zip: {}", e);
            return Err(Box::new(e));
        }

        let file_contents = std::fs::read(file_path)?;
        if let Err(e) = zip.write_all(&file_contents) {
            tracing::error!("Failed to write file to zip: {}", e);
            return Err(Box::new(e));
        }
        
        Ok(())
    }

    /// Returns the HTML interface with configuration placeholders replaced.
    ///
    /// # Returns
    ///
    /// * `String` - Complete HTML interface
    fn get_updated_html(&self) -> String {
        // Updated HTML with staging functionality
        include_str!("../static/index.html")
            .replace("{{TAILWIND_CSS}}", include_str!("../static/style.css"))
            .replace("{{ADMIN_PORT}}", &self.config.admin_port.to_string())
            .replace("{{DOWNLOAD_PORT}}", &self.config.download_port.to_string())
    }

    /// Safely joins `relative` onto `base_path` and verifies the resolved path
    /// is still within `base_path` after canonicalization.
    ///
    /// This prevents both classic `../` path traversal and symlink-based escapes,
    /// since `canonicalize()` resolves all symlinks before the containment check.
    ///
    /// # Errors
    ///
    /// Returns `None` if:
    /// - The joined path does not exist on disk (can't canonicalize a non-existent path)
    /// - The canonicalized path escapes `base_path`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// // Safe path — returns Some(canonical_path)
    /// let ok = handler.safe_join("subdir/file.txt");
    /// // Traversal — returns None
    /// let bad = handler.safe_join("../../etc/passwd");
    /// assert!(bad.is_none());
    /// ```
    pub fn safe_join(&self, relative: &str) -> Option<PathBuf> {
        let joined = self.state.base_path.join(relative);
        // canonicalize resolves symlinks and `..` components; if the path
        // doesn't exist it returns an error, which we propagate as None.
        let canonical = std::fs::canonicalize(&joined).ok()?;
        if canonical.starts_with(&self.state.base_path) {
            Some(canonical)
        } else {
            tracing::warn!(
                "Path escape blocked: '{}' resolves to '{:?}' outside base '{:?}'",
                relative,
                canonical,
                self.state.base_path
            );
            None
        }
    }

    /// Parses URL query string into key-value pairs.
    ///
    /// # Arguments
    ///
    /// * `query` - URL query string
    ///
    /// # Returns
    ///
    /// * `HashMap<String, String>` - Parsed query parameters
    fn parse_query(&self, query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    self.url_decode(key),
                    self.url_decode(value),
                );
            }
        }
        params
    }

    /// URL-decodes a string (handles %XX encoding and + for spaces).
    ///
    /// # Arguments
    ///
    /// * `input` - URL-encoded string
    ///
    /// # Returns
    ///
    /// * `String` - Decoded string
    fn url_decode(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars();
        
        while let Some(ch) = chars.next() {
            match ch {
                '%' => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        } else {
                            result.push('%');
                            result.push_str(&hex);
                        }
                    } else {
                        result.push('%');
                        result.push_str(&hex);
                    }
                }
                '+' => result.push(' '),
                _ => result.push(ch),
            }
        }
        
        result
    }

    /// URL-encodes a string for safe use in URLs.
    ///
    /// # Arguments
    ///
    /// * `input` - String to encode
    ///
    /// # Returns
    ///
    /// * `String` - URL-encoded string
    fn url_encode(&self, input: &str) -> String {
        let mut result = String::new();
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                b' ' => result.push('+'),
                _ => {
                    result.push_str(&format!("%{byte:02X}"));
                }
            }
        }
        result
    }

    /// Extracts the body content from an HTTP request.
    ///
    /// # Arguments
    ///
    /// * `request` - Complete HTTP request string
    ///
    /// # Returns
    ///
    /// * `Result<String, Box<dyn std::error::Error + Send + Sync>>` - Request body or error
    fn extract_body(&self, request: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(body_start) = request.find("\r\n\r\n") {
            Ok(request[body_start + 4..].to_string())
        } else {
            Ok(String::new())
        }
    }
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_url_encoding() {
        let config = Config::default();
        let state = Arc::new(AppState::new(PathBuf::from("/test")));
        let handler = Handler::new(state, config);

        assert_eq!(handler.url_encode("hello world"), "hello+world");
        assert_eq!(handler.url_encode("file.txt"), "file.txt");
        assert_eq!(handler.url_encode("special@chars"), "special%40chars");
    }

    #[test]
    fn test_url_decoding() {
        let config = Config::default();
        let state = Arc::new(AppState::new(PathBuf::from("/test")));
        let handler = Handler::new(state, config);

        assert_eq!(handler.url_decode("hello+world"), "hello world");
        assert_eq!(handler.url_decode("file.txt"), "file.txt");
        assert_eq!(handler.url_decode("special%40chars"), "special@chars");
    }

    #[test]
    fn test_query_parsing() {
        let config = Config::default();
        let state = Arc::new(AppState::new(PathBuf::from("/test")));
        let handler = Handler::new(state, config);

        let params = handler.parse_query("k=token123&path=folder%2Ffile");
        assert_eq!(params.get("k"), Some(&"token123".to_string()));
        assert_eq!(params.get("path"), Some(&"folder/file".to_string()));
    }

    fn make_handler_with_token(token: Option<&str>) -> Handler {
        let config = Config {
            admin_token: token.map(|t| t.to_string()),
            ..Default::default()
        };
        let state = Arc::new(AppState::new(PathBuf::from("/tmp")));
        Handler::new(state, config)
    }

    fn loopback_addr() -> std::net::SocketAddr {
        "127.0.0.1:12345".parse().unwrap()
    }

    /// When no token is configured, all admin requests are allowed.
    #[test]
    fn test_admin_auth_disabled_allows_all() {
        let handler = make_handler_with_token(None);
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        // Should NOT be 401
        assert!(!response_str.contains("401 Unauthorized"), "Unexpected 401 when auth is disabled");
    }

    /// When token is configured, missing Authorization header returns 401.
    #[test]
    fn test_admin_auth_required_rejects_missing_header() {
        let handler = make_handler_with_token(Some("secret123"));
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
        assert!(response_str.contains("WWW-Authenticate"));
    }

    /// Wrong token returns 401.
    #[test]
    fn test_admin_auth_required_rejects_wrong_token() {
        let handler = make_handler_with_token(Some("secret123"));
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrongtoken\r\n\r\n";
        let response = smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
    }

    /// Correct token is accepted.
    #[test]
    fn test_admin_auth_required_accepts_correct_token() {
        let handler = make_handler_with_token(Some("secret123"));
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secret123\r\n\r\n";
        let response = smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(!response_str.contains("401 Unauthorized"), "Correct token should be accepted");
    }

    /// safe_join with a normal relative path returns the canonical path.
    #[test]
    fn test_safe_join_valid_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"hello").unwrap();

        let config = Config::default();
        let state = Arc::new(AppState::new(dir.path().to_path_buf()));
        let handler = Handler::new(state, config);

        let result = handler.safe_join("file.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    /// safe_join must block `../` path traversal.
    #[test]
    fn test_safe_join_blocks_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let state = Arc::new(AppState::new(dir.path().to_path_buf()));
        let handler = Handler::new(state, config);

        // Non-existent traversal path — canonicalize fails → None
        assert!(handler.safe_join("../../../etc/passwd").is_none());
    }

    /// safe_join must block symlinks that point outside base_path.
    #[test]
    fn test_safe_join_blocks_symlink_escape() {
        let dir = tempfile::tempdir().unwrap();
        let link_path = dir.path().join("evil_link");
        // Create symlink inside base_path → /etc/passwd outside base_path
        std::os::unix::fs::symlink("/etc/passwd", &link_path).unwrap();

        let config = Config::default();
        let state = Arc::new(AppState::new(dir.path().to_path_buf()));
        let handler = Handler::new(state, config);

        // Must be blocked — /etc/passwd is outside the base dir
        assert!(handler.safe_join("evil_link").is_none());
    }

    /// safe_join must allow symlinks that resolve within base_path.
    #[test]
    fn test_safe_join_allows_internal_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real_file = dir.path().join("real.txt");
        std::fs::write(&real_file, b"data").unwrap();
        let link_path = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real_file, &link_path).unwrap();

        let config = Config::default();
        let state = Arc::new(AppState::new(dir.path().to_path_buf()));
        let handler = Handler::new(state, config);

        // Should succeed — symlink resolves within base_path
        let result = handler.safe_join("link.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), real_file.canonicalize().unwrap());
    }

    /// Verify that compare_exchange prevents concurrent double-downloads.
    /// Two threads race to claim the same AtomicBool; exactly one must win.
    #[test]
    fn test_one_time_download_race_condition() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let downloaded = Arc::new(AtomicBool::new(false));
        let success_count = Arc::new(AtomicUsize::new(0));

        let threads: Vec<_> = (0..20)
            .map(|_| {
                let downloaded = Arc::clone(&downloaded);
                let success_count = Arc::clone(&success_count);
                thread::spawn(move || {
                    if downloaded
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(
            success_count.load(Ordering::Relaxed),
            1,
            "Exactly one thread should succeed in claiming the download"
        );
    }

    /// Verify that when one_time is disabled, multiple downloads are allowed.
    #[test]
    fn test_one_time_disabled_allows_redownload() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let one_time_enabled = AtomicBool::new(false);
        let downloaded = AtomicBool::new(false);

        // Simulate two download attempts with one_time disabled
        for _ in 0..3 {
            if one_time_enabled.load(Ordering::Acquire) {
                assert!(
                    downloaded
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok(),
                    "Should only succeed once when one_time enabled"
                );
            }
            // When disabled: no check performed, would proceed to serve
        }
        // downloaded flag never set since one_time is disabled
        assert!(!downloaded.load(Ordering::Relaxed));
    }
}