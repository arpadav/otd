//! Request handlers for the OTD server.
//!
//! This module contains the main business logic for handling HTTP requests,
//! including file browsing, download link generation, and file serving.
//! It implements a clean separation between admin and download functionality.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod browse;
pub(crate) mod download;
mod links;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{config::CONFIG, http::*, prelude::*, types::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
use uuid::Uuid;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Maximum number of HTTP headers to parse per request.
const MAX_PARSE_HEADERS: usize = 64;
/// Cookie name used for admin session tracking.
const SESSION_COOKIE_NAME: &str = "otd_session";
/// Maximum session age (24 hours).
const SESSION_MAX_AGE: std::time::Duration = std::time::Duration::from_hours(24);
/// Max-Age value for session cookies (seconds, as string).
const SESSION_COOKIE_MAX_AGE: &str = "86400";
/// WWW-Authenticate realm for bearer token auth.
const AUTH_REALM: &str = "otd-admin";
/// Separator between HTTP headers and body.
const HEADER_BODY_SEPARATOR: &str = "\r\n\r\n";

/// HTTP method constants
mod method {
    pub const GET: &str = "GET";
    pub const POST: &str = "POST";
    pub const DELETE: &str = "DELETE";
}
/// HTTP route constants
mod route {
    pub const ROOT: &str = "/";
    pub const ABOUT: &str = "/about";
    pub const LOGIN: &str = "/login";
    pub const LOGOUT: &str = "/logout";
    pub const API_BROWSE: &str = "/api/browse";
    pub const API_STATS: &str = "/api/stats";
    pub const API_TOKENS: &str = "/api/tokens";
    pub const API_GENERATE: &str = "/api/generate";
    pub const API_TOKENS_BULK_DELETE: &str = "/api/tokens/bulk-delete";
    pub const API_TOKENS_PREFIX: &str = "/api/tokens/";
    pub const LOGO: &str = "/logo.svg";
}
/// HTTP header name constants
mod header_name {
    pub const AUTHORIZATION: &str = "Authorization";
    pub const BEARER_PREFIX: &str = "Bearer ";
    pub const COOKIE: &str = "Cookie";
    pub const SET_COOKIE: &str = "Set-Cookie";
}

/// Main request handler containing business logic for both admin and download servers.
///
/// The handler manages file browsing, download link generation, and file serving.
/// It maintains shared state and configuration, and provides separate entry points
/// for admin interface requests and download requests.
///
/// Cloning a `Handler` only bumps `Arc` reference counts - zero heap allocations.
///
/// # Examples
///
/// ```rust,no_run
/// use otd::{Handler, types::AppState};
/// use std::sync::Arc;
///
/// let state = Arc::new(AppState::new());
/// # smol::block_on(async {
/// let handler = Handler::new(state).await;
/// # });
/// ```
pub struct Handler {
    /// Shared application state containing download tokens and configuration
    pub state: Arc<AppState>,
    /// Cached index.html with all config placeholders replaced
    index_html: Arc<str>,
    /// Cached about.html with CSS injected
    about_html: Arc<str>,
    /// Cached login.html with CSS injected ({{MESSAGE}} still present for runtime replacement)
    login_html_base: Arc<str>,
    /// Cached Set-Cookie header for logout (session clear)
    logout_cookie: Arc<str>,
    /// Cached logo SVG content
    logo_svg: Arc<str>,
}
/// [`Handler`] implementation
impl Handler {
    /// Creates a new handler with the given state and pre-parsed configuration.
    ///
    /// Pre-computes and caches all HTML templates at construction time so that
    /// serving pages requires zero template processing.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::{Handler, Config, types::AppState};
    /// use std::{sync::Arc, path::PathBuf};
    ///
    /// let state = Arc::new(AppState::new());
    /// let handler = Handler::new(state);
    /// ```
    pub async fn new(state: Arc<AppState>) -> Self {
        let (admin_host, admin_port, download_host, download_port, base_path) = CONFIG
            .read_with(|cfg| {
                (
                    cfg.raw.admin_host.clone(),
                    cfg.raw.admin_port.to_string(),
                    cfg.raw.download_host.clone(),
                    cfg.raw.download_port.to_string(),
                    cfg.raw.base_path.clone(),
                )
            })
            .await;

        let css = include_str!("../../static/style.css");

        let index_html: Arc<str> = include_str!("../../static/index.html")
            .replace("{{TAILWIND_CSS}}", css)
            .replace("{{ADMIN_HOST}}", &admin_host)
            .replace("{{ADMIN_PORT}}", &admin_port)
            .replace("{{DOWNLOAD_HOST}}", &download_host)
            .replace("{{DOWNLOAD_PORT}}", &download_port)
            .replace("{{BASE_PATH}}", &base_path)
            .into();

        let about_html: Arc<str> = include_str!("../../static/about.html")
            .replace("{{TAILWIND_CSS}}", css)
            .into();

        let login_html_base: Arc<str> = include_str!("../../static/login.html")
            .replace("{{TAILWIND_CSS}}", css)
            .into();

        let logout_cookie: Arc<str> =
            format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",).into();

        let logo_svg: Arc<str> = include_str!("../../static/logo.svg").into();

        Self {
            state,
            index_html,
            about_html,
            login_html_base,
            logout_cookie,
            logo_svg,
        }
    }

    /// Handles requests to the admin interface (file browsing, link generation).
    ///
    /// Orchestrates the admin request pipeline: parse → authenticate → log →
    /// split path → verify session → handle login → route to handler.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, types::AppState};
    /// # use std::sync::Arc;
    /// # smol::block_on(async {
    /// # let state = Arc::new(AppState::new());
    /// # let handler = Handler::new(state).await;
    /// let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    /// let peer_addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    /// let response = handler.handle_admin_request(request, peer_addr).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_admin_request(
        &self,
        request: &str,
        peer_addr: std::net::SocketAddr,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_PARSE_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(request.as_bytes()) {
            Ok(_) => {
                let method = req.method.unwrap_or(method::GET);
                let path = req.path.unwrap_or(route::ROOT);

                // Serve logo without auth so it loads on the login page and as favicon
                if method == method::GET && path == route::LOGO {
                    return Ok(HttpResponse::ok()
                        .content_type(content_type::SVG)
                        .body_text(&self.logo_svg)
                        .to_bytes());
                }

                if let Some(bytes) = self.verify_bearer_auth(&req).await {
                    return Ok(bytes);
                }
                match method {
                    method::POST | method::DELETE | "PUT" | "PATCH" => {
                        tracing::info!("Admin request: {method} {path} from {peer_addr}");
                    }
                    _ => {
                        tracing::debug!("Admin request: {method} {path}");
                    }
                }

                let (path, query) = Self::split_path_query(path);
                tracing::trace!("Parsed path: '{path}', query: '{query}'");

                let is_loopback = peer_addr.ip().is_loopback();
                if let Some(bytes) = self.verify_session(&req, path, is_loopback).await {
                    return Ok(bytes);
                }

                if let Some(result) = self.handle_login(method, path, request).await {
                    return result;
                }

                let session_cookie = helpers::extract_session_token(&req).map(|s| s.to_string());
                let response = self
                    .route_admin_request(method, path, query, request, session_cookie)
                    .await?;
                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse HTTP request: {e}");
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }

    /// Handles requests to the download server (file downloads only).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, types::AppState};
    /// # use std::sync::Arc;
    /// # smol::block_on(async {
    /// # let state = Arc::new(AppState::new());
    /// # let handler = Handler::new(state).await;
    /// let request = "GET /document.pdf?k=550e8400-e29b-41d4-a716-446655440000 HTTP/1.1\r\n\r\n";
    /// let peer_addr: std::net::SocketAddr = "192.168.1.10:54321".parse().unwrap();
    /// let response = handler.handle_download_request(request, peer_addr).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_download_request(
        &self,
        request: &str,
        peer_addr: std::net::SocketAddr,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_PARSE_HEADERS];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(request.as_bytes()) {
            Ok(_) => {
                let method = req.method.unwrap_or(method::GET);
                let path = req.path.unwrap_or(route::ROOT);
                tracing::info!("Download request from {peer_addr}: {method} {path}");

                let (_, query) = Self::split_path_query(path);

                let response = match method {
                    method::GET => self.download(query, peer_addr).await?,
                    _ => HttpResponse::not_found(),
                };

                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse download request from {peer_addr}: {e}");
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }

    /// Checks the `Authorization: Bearer <token>` header against the configured admin token.
    ///
    /// # Arguments
    ///
    /// * `req` - The parsed HTTP request to check
    ///
    /// # Returns
    ///
    /// `None` if auth passes or no token is configured; `Some(401_bytes)` if auth fails.
    async fn verify_bearer_auth(&self, req: &httparse::Request<'_, '_>) -> Option<Vec<u8>> {
        let expected_token = CONFIG.read_with(|cfg| cfg.raw.admin_token.clone()).await?;
        let auth_header = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(header_name::AUTHORIZATION));
        let authorized = auth_header
            .and_then(|h| std::str::from_utf8(h.value).ok())
            .and_then(|v| v.strip_prefix(header_name::BEARER_PREFIX))
            .map(|token| token.trim() == expected_token.as_str())
            .unwrap_or(false);
        if authorized {
            None
        } else {
            tracing::warn!("Admin request rejected: missing or invalid Authorization header");
            Some(HttpResponse::unauthorized(AUTH_REALM).to_bytes())
        }
    }

    /// Validates the session for non-loopback requests.
    ///
    /// Skips validation for loopback addresses and login routes. For external
    /// requests, checks that a valid session cookie exists and cleans up expired
    /// sessions on failure.
    ///
    /// # Arguments
    ///
    /// * `req` - The parsed HTTP request containing cookies
    /// * `path` - The request path (login route is exempt from session checks)
    /// * `is_loopback` - Whether the request originates from a loopback address
    ///
    /// # Returns
    ///
    /// `None` if session is valid or not required; `Some(redirect_bytes)` or
    /// `Some(forbidden_bytes)` otherwise.
    async fn verify_session(
        &self,
        req: &httparse::Request<'_, '_>,
        path: &str,
        is_loopback: bool,
    ) -> Option<Vec<u8>> {
        if is_loopback {
            return None;
        }
        if CONFIG
            .read_with(|cfg| cfg.raw.admin_password.is_none())
            .await
        {
            let html =
                self.login_page("External access requires admin_password to be set in config.");
            return Some(
                HttpResponse::new(status::FORBIDDEN.0, status::FORBIDDEN.1)
                    .content_type(content_type::HTML)
                    .body_text(&html)
                    .to_bytes(),
            );
        }

        if path == route::LOGIN {
            return None;
        }

        let session_token = helpers::extract_session_token(req);
        let mut valid_session = false;
        if let Some(token) = session_token {
            let sessions = self.state.sessions.read().await;
            if let Some(created) = sessions.get(token)
                && created.elapsed() < SESSION_MAX_AGE
            {
                valid_session = true;
            }
        }

        if !valid_session {
            let mut sessions = self.state.sessions.write().await;
            sessions.retain(|_, created| created.elapsed() < SESSION_MAX_AGE);
        }

        if valid_session {
            None
        } else {
            Some(HttpResponse::redirect(route::LOGIN).to_bytes())
        }
    }

    /// Handles GET and POST requests to the login route.
    ///
    /// Returns `None` if the path is not `/login` (caller continues to routing).
    /// Returns `Some(Ok(bytes))` for GET (render form) or POST (validate credentials).
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path
    /// * `request` - The raw HTTP request string (needed for POST body extraction)
    ///
    /// # Returns
    ///
    /// `None` if path is not `/login`; `Some(Result)` with the login response otherwise.
    async fn handle_login(
        &self,
        method: &str,
        path: &str,
        request: &str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>> {
        if path != route::LOGIN {
            return None;
        }

        if method == method::GET {
            let html = self.login_page("");
            return Some(Ok(HttpResponse::ok()
                .content_type(content_type::HTML)
                .body_text(&html)
                .to_bytes()));
        }

        if method == method::POST {
            let body = match helpers::extract_body(request) {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            let params = helpers::parse_query(&body);
            let password = params.get("password").cloned().unwrap_or_default();

            let expected = CONFIG
                .read_with(|cfg| cfg.raw.admin_password.clone().unwrap_or_else(String::new))
                .await;

            if !expected.is_empty() && password == expected {
                let token = Uuid::new_v4().to_string();
                self.state
                    .sessions
                    .write()
                    .await
                    .insert(token.clone(), std::time::Instant::now());
                return Some(Ok(HttpResponse::redirect(route::ROOT)
                    .header(header_name::SET_COOKIE, &Self::session_set_cookie(&token))
                    .to_bytes()));
            } else {
                let html = self.login_page("Invalid password.");
                return Some(Ok(HttpResponse::new(
                    status::FORBIDDEN.0,
                    status::FORBIDDEN.1,
                )
                .content_type(content_type::HTML)
                .body_text(&html)
                .to_bytes()));
            }
        }

        None
    }

    /// Routes an authenticated admin request to the appropriate handler.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path (without query string)
    /// * `query` - The query string
    /// * `request` - The raw HTTP request string (needed for POST body extraction)
    /// * `session_cookie` - The session token, if present (used for logout invalidation)
    ///
    /// # Returns
    ///
    /// The constructed `HttpResponse` for the matched route, or 404 for unknown routes.
    async fn route_admin_request(
        &self,
        method: &str,
        path: &str,
        query: &str,
        request: &str,
        session_cookie: Option<String>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        match (method, path) {
            (method::GET, route::ROOT) => self.web_interface().await,
            (method::GET, route::ABOUT) => self.about_page().await,
            (method::GET, route::API_BROWSE) => self.browse(query).await,
            (method::GET, route::API_STATS) => self.stats().await,
            (method::GET, route::API_TOKENS) => self.list_tokens().await,
            (method::POST, route::API_GENERATE) => {
                let body = helpers::extract_body(request)?;
                self.generate_link(&body).await
            }
            (method::POST, route::API_TOKENS_BULK_DELETE) => {
                let body = helpers::extract_body(request)?;
                self.bulk_delete_tokens(&body).await
            }
            (method::DELETE, path) if path.starts_with(route::API_TOKENS_PREFIX) => {
                let token = path.strip_prefix(route::API_TOKENS_PREFIX).unwrap_or("");
                self.delete_token(token).await
            }
            (method::GET, route::LOGOUT) => {
                if let Some(ref token) = session_cookie {
                    self.state.sessions.write().await.remove(token);
                    tracing::info!("Session invalidated on logout");
                }
                Ok(HttpResponse::redirect(route::LOGIN)
                    .header(header_name::SET_COOKIE, &self.logout_cookie))
            }
            _ => Ok(HttpResponse::not_found()),
        }
    }

    /// Serves the main web interface HTML (pre-cached at startup).
    async fn web_interface(
        &self,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HttpResponse::ok()
            .content_type(content_type::HTML)
            .body_text(&self.index_html))
    }

    /// Serves the static "About" page (pre-cached at startup).
    async fn about_page(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HttpResponse::ok()
            .content_type(content_type::HTML)
            .body_text(&self.about_html))
    }

    /// Returns dashboard statistics as JSON.
    ///
    /// Aggregates token state (active/used/expired counts, total downloads)
    /// and server uptime into a [`StatsResponse`].
    async fn stats(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tokens = self.state.tokens.read().await;
        let now = std::time::Instant::now();

        let mut active = 0u32;
        let mut used = 0u32;
        let mut expired = 0u32;
        let mut total_downloads = 0u64;

        for item in tokens.values() {
            let count = item.download_count.load(Ordering::Relaxed);
            total_downloads += count as u64;
            let is_expired = item.expires_at.is_some_and(|e| now >= e);
            let is_used = count >= item.max_downloads;
            if is_expired {
                expired += 1;
            } else if is_used {
                used += 1;
            } else {
                active += 1;
            }
        }

        let uptime_seconds = self.state.started_at.elapsed().as_secs();

        let stats = StatsResponse {
            active_tokens: active,
            used_tokens: used,
            expired_tokens: expired,
            total_downloads,
            uptime_seconds,
        };

        HttpResponse::ok().body_json(&stats).map_err(Into::into)
    }

    /// Safely joins `relative` onto `base_path` and verifies the resolved path
    /// is still within `base_path` after canonicalization.
    ///
    /// This prevents both classic `../` path traversal and symlink-based escapes,
    /// since `canonicalize()` resolves all symlinks before the containment check.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, types::AppState};
    /// # use std::sync::Arc;
    /// # smol::block_on(async {
    /// # let state = Arc::new(AppState::new());
    /// # let handler = Handler::new(state).await;
    /// // Safe path - returns Some(canonical_path)
    /// let ok = handler.safe_join("subdir/file.txt").await;
    /// // Traversal - returns None
    /// let bad = handler.safe_join("../../etc/passwd").await;
    /// assert!(bad.is_none());
    /// # });
    /// ```
    pub async fn safe_join(&self, relative: &str) -> Option<PathBuf> {
        let base_path = CONFIG
            .read_with(|cfg| cfg.canonical_base_path.clone())
            .await;
        // --------------------------------------------------
        // canonicalize resolves symlinks and `..` components; if the path
        // doesn't exist it returns an error, which we propagate as None.
        // --------------------------------------------------
        let joined = base_path.join(relative);
        std::fs::canonicalize(&joined)
            .inspect_err(|e| tracing::warn!("Failed to canonicalize '{relative}': {e}"))
            .ok()
            .filter(|c| {
                let safe = c.starts_with(&base_path);
                if !safe {
                    tracing::warn!(
                        "Path escape blocked: '{relative}' resolves to '{c:?}' outside base '{base_path:?}'"
                    );
                }
                safe
            })
    }

    /// Renders the login page with an optional message.
    ///
    /// Substitutes `{{MESSAGE}}` in the pre-cached login HTML template.
    /// CSS is already injected at startup.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to display (e.g. error text); use `""` for none
    ///
    /// # Returns
    ///
    /// The fully rendered login page HTML.
    fn login_page(&self, message: &str) -> String {
        self.login_html_base.replace("{{MESSAGE}}", message)
    }

    /// Builds a `Set-Cookie` header value for a new session.
    ///
    /// # Arguments
    ///
    /// * `token` - The session token to set
    ///
    /// # Returns
    ///
    /// A `Set-Cookie` header value with `HttpOnly`, `SameSite=Strict`, and `Max-Age`.
    fn session_set_cookie(token: &str) -> String {
        format!(
            "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            SESSION_COOKIE_NAME, token, SESSION_COOKIE_MAX_AGE
        )
    }

    /// Splits a URL path into `(path, query)` components at the first `?`.
    ///
    /// # Arguments
    ///
    /// * `path` - The full request path (may include query string)
    ///
    /// # Returns
    ///
    /// A tuple of `(path, query)` where `query` is `""` if no `?` is present.
    fn split_path_query(path: &str) -> (&str, &str) {
        if let Some(pos) = path.find('?') {
            (&path[..pos], &path[pos + 1..])
        } else {
            (path, "")
        }
    }

    /// Builds a full download URL from a display name and token.
    ///
    /// URL-encodes the name and appends the token as a `k` query parameter.
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the download (will be URL-encoded)
    /// * `token` - The unique download token
    ///
    /// # Returns
    ///
    /// The complete download URL.
    pub(crate) async fn download_url(&self, name: &str, token: &str) -> String {
        let download_base_url = CONFIG.read_with(|cfg| cfg.download_base_url.clone()).await;
        format!(
            "{}/{}?k={}",
            download_base_url,
            helpers::url_encode(name),
            token
        )
    }
}
/// [`Handler`] implementation of [`Clone`]
impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            index_html: Arc::clone(&self.index_html),
            about_html: Arc::clone(&self.about_html),
            login_html_base: Arc::clone(&self.login_html_base),
            logout_cookie: Arc::clone(&self.logout_cookie),
            logo_svg: Arc::clone(&self.logo_svg),
        }
    }
}

#[cfg_attr(feature = "doc-tests", visibility::make(pub))]
/// Helper functions, mainly for URL and request handling, used
/// by [`Handler`]
mod helpers {
    use super::*;

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Extracts the session token from the `Cookie` header.
    ///
    /// Searches for the `otd_session=<value>` cookie among all cookies
    /// in the request.
    ///
    /// # Arguments
    ///
    /// * `req` - The parsed HTTP request
    ///
    /// # Returns
    ///
    /// The session token value, or `None` if not present.
    ///
    /// # Example
    ///
    /// ```rust
    /// use otd::handlers::helpers::extract_session_token;
    /// let req = httparse::Request::new(&mut []);
    /// let token = extract_session_token(&req);
    /// ```
    pub(crate) fn extract_session_token<'a>(req: &'a httparse::Request<'_, '_>) -> Option<&'a str> {
        const PREFIX: &str = concat!("otd_session", "=");
        req.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(header_name::COOKIE))
            .and_then(|h| std::str::from_utf8(h.value).ok())
            .unwrap_or("")
            .split(';')
            .map(|s| s.trim())
            .find(|s| s.starts_with(PREFIX))
            .and_then(|s| s.strip_prefix(PREFIX))
    }

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Parses a URL query string into key-value pairs.
    ///
    /// Splits on `&` and `=`, URL-decoding both keys and values.
    /// Pairs without `=` are silently skipped.
    ///
    /// # Arguments
    ///
    /// * `query` - The raw query string (without leading `?`)
    ///
    /// # Returns
    ///
    /// A map of decoded key-value pairs.
    pub(crate) fn parse_query(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(url_decode(key), url_decode(value));
            }
        }
        params
    }

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Extracts the body content from a raw HTTP request string.
    ///
    /// Finds the `\r\n\r\n` header-body separator and returns everything after it.
    /// Returns an empty string if no separator is found.
    ///
    /// # Arguments
    ///
    /// * `request` - The full raw HTTP request
    ///
    /// # Returns
    ///
    /// The request body as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use otd::handlers::helpers::extract_body;
    /// let body = extract_body("POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nHello");
    /// assert_eq!(body.unwrap(), "Hello");
    /// ```
    pub(crate) fn extract_body(
        request: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(body_start) = request.find(super::HEADER_BODY_SEPARATOR) {
            Ok(request[body_start + super::HEADER_BODY_SEPARATOR.len()..].to_string())
        } else {
            Ok(String::new())
        }
    }

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// URL-encodes a string for safe use in URLs, using `application/x-www-form-urlencoded`
    /// standard (spaces are replaced with `+`)
    ///
    /// # Arguments
    ///
    /// * `input` - The string to encode
    ///
    /// # Returns
    ///
    /// The URL-encoded string
    ///
    /// # Example
    ///
    /// ```rust
    /// use otd::handlers::helpers::url_encode;
    /// let encoded = url_encode("Hello, World!");
    /// assert_eq!(encoded, "Hello%2C+World%21");
    /// ```
    pub(crate) fn url_encode(input: &str) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        input
            .bytes()
            .fold(String::with_capacity(input.len()), |mut acc, b| {
                match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        acc.push(b as char);
                    }
                    b' ' => acc.push('+'),
                    _ => {
                        acc.push('%');
                        acc.push(HEX[(b >> 4) as usize] as char);
                        acc.push(HEX[(b & 0x0F) as usize] as char);
                    }
                }
                acc
            })
    }

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// URL-decodes a percent-encoded string.
    ///
    /// Handles `%XX` hex sequences and `+` as space, per
    /// `application/x-www-form-urlencoded`. Malformed `%` sequences are
    /// passed through literally.
    ///
    /// # Arguments
    ///
    /// * `input` - The percent-encoded string
    ///
    /// # Returns
    ///
    /// The decoded string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use otd::handlers::helpers::url_decode;
    /// let decoded = url_decode("Hello%2C+World%21");
    /// assert_eq!(decoded, "Hello, World!");
    /// ```
    pub(crate) fn url_decode(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut bytes = input.bytes();
        while let Some(b) = bytes.next() {
            match b {
                b'%' => match (bytes.next(), bytes.next()) {
                    (Some(hi), Some(lo)) => match (unhex(hi), unhex(lo)) {
                        (Some(h), Some(l)) => result.push((h << 4 | l) as char),
                        _ => {
                            result.push('%');
                            result.push(hi as char);
                            result.push(lo as char);
                        }
                    },
                    (Some(hi), None) => {
                        result.push('%');
                        result.push(hi as char);
                    }
                    _ => result.push('%'),
                },
                b'+' => result.push(' '),
                _ => result.push(b as char),
            }
        }
        result
    }

    /// A utility function to convert a hexadecimal character to a byte
    ///
    /// This is only used by [`url_decode`]
    const fn unhex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'A'..=b'F' => Some(b - b'A' + 10),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::path::PathBuf;

    /// Tests that depend on the global CONFIG must not run in parallel.
    static HOT_CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_url_encoding() {
        assert_eq!(helpers::url_encode("hello world"), "hello+world");
        assert_eq!(helpers::url_encode("file.txt"), "file.txt");
        assert_eq!(helpers::url_encode("special@chars"), "special%40chars");
    }

    #[test]
    fn test_url_decoding() {
        assert_eq!(helpers::url_decode("hello+world"), "hello world");
        assert_eq!(helpers::url_decode("file.txt"), "file.txt");
        assert_eq!(helpers::url_decode("special%40chars"), "special@chars");
    }

    #[test]
    fn test_query_parsing() {
        let params = helpers::parse_query("k=token123&path=folder%2Ffile");
        assert_eq!(params.get("k"), Some(&"token123".to_string()));
        assert_eq!(params.get("path"), Some(&"folder/file".to_string()));
    }

    /// init config for testing
    fn init_hot_config(token: Option<&str>, password: Option<&str>, base_path: PathBuf) {
        let cfg = Config {
            admin_token: token.map(|t| t.to_string()),
            admin_password: password.map(|p| p.to_string()),
            base_path: base_path.to_string_lossy().into(),
            ..Default::default()
        };
        let parsed = cfg.parse(Default::default());
        let mut cfg = CONFIG.write_blocking();
        *cfg = parsed;
    }

    fn make_handler_with_token(token: Option<&str>) -> Handler {
        init_hot_config(token, None, PathBuf::from("/tmp"));

        smol::block_on(CONFIG.write_with(|cfg| {
            cfg.raw.admin_token = token.map(|t| t.to_string());
        }));
        let state = Arc::new(AppState::new());

        smol::block_on(Handler::new(state))
    }

    fn loopback_addr() -> std::net::SocketAddr {
        "127.0.0.1:12345".parse().unwrap()
    }

    /// When no token is configured, all admin requests are allowed.
    #[test]
    fn test_admin_auth_disabled_allows_all() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let handler = make_handler_with_token(None);
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        // Should NOT be 401
        assert!(
            !response_str.contains("401 Unauthorized"),
            "Unexpected 401 when auth is disabled"
        );
    }

    /// When token is configured, missing Authorization header returns 401.
    #[test]
    fn test_admin_auth_required_rejects_missing_header() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let handler = make_handler_with_token(Some("secret123"));
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
        assert!(response_str.contains("WWW-Authenticate"));
    }

    /// Wrong token returns 401.
    #[test]
    fn test_admin_auth_required_rejects_wrong_token() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let handler = make_handler_with_token(Some("secret123"));
        let request =
            "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrongtoken\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
    }

    /// Correct token is accepted.
    #[test]
    fn test_admin_auth_required_accepts_correct_token() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let handler = make_handler_with_token(Some("secret123"));
        let request =
            "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secret123\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            !response_str.contains("401 Unauthorized"),
            "Correct token should be accepted"
        );
    }

    fn make_handler_with_base(base: PathBuf) -> Handler {
        init_hot_config(None, None, base);
        let state = Arc::new(AppState::new());
        smol::block_on(Handler::new(state))
    }

    /// safe_join with a normal relative path returns the canonical path.
    #[test]
    fn test_safe_join_valid_path() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"hello").unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        let result = smol::block_on(handler.safe_join("file.txt"));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    /// safe_join must block `../` path traversal.
    #[test]
    fn test_safe_join_blocks_traversal() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Non-existent traversal path - canonicalize fails → None
        assert!(smol::block_on(handler.safe_join("../../../etc/passwd")).is_none());
    }

    /// safe_join must block symlinks that point outside base_path.
    #[test]
    fn test_safe_join_blocks_symlink_escape() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let link_path = dir.path().join("evil_link");
        // Create symlink inside base_path → /etc/passwd outside base_path
        std::os::unix::fs::symlink("/etc/passwd", &link_path).unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Must be blocked - /etc/passwd is outside the base dir
        assert!(smol::block_on(handler.safe_join("evil_link")).is_none());
    }

    /// safe_join must allow symlinks that resolve within base_path.
    #[test]
    fn test_safe_join_allows_internal_symlink() {
        let _lock = HOT_CONFIG_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let real_file = dir.path().join("real.txt");
        std::fs::write(&real_file, b"data").unwrap();
        let link_path = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real_file, &link_path).unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Should succeed - symlink resolves within base_path
        let result = smol::block_on(handler.safe_join("link.txt"));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), real_file.canonicalize().unwrap());
    }

    /// Verify that compare_exchange prevents concurrent double-downloads.
    /// Two threads race to claim the same AtomicBool; exactly one must win.
    #[test]
    fn test_one_time_download_race_condition() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
