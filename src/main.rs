use arc_swap::ArcSwap;
use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use axum_server::Server;
use mime_guess::mime;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{fs::File, sync::RwLock};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
// use hyper::server::Server;
// use hyper::service::make_service_fn;

// #[derive(Clone)]
struct DownloadItem {
    path: PathBuf,
    is_folder: bool,
    downloaded: ArcSwap<bool>,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    k: String,
}

struct AppState {
    tokens: RwLock<HashMap<String, DownloadItem>>,
    one_time_enabled: AtomicBool,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        tokens: RwLock::new(HashMap::new()),
        one_time_enabled: AtomicBool::new(true),
    });

    let app = Router::new()
        .route("/generate/{type}", get(generate_handler))
        .route("/", get(download_handler))
        .route("/config/one-time/{enabled}", get(config_handler))
        .with_state(Arc::clone(&state));

    let addr: std::net::SocketAddr = "0.0.0.0:15204".parse().map_err(|e| {
        tracing::error!("Failed to parse address: {}", e);
        std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
    })?;
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    Server::from_tcp(listener.into_std()?)
        .serve(app.into_make_service())
        .await
}

async fn generate_handler(
    State(state): State<Arc<AppState>>,
    Path(item_type): Path<String>,
) -> Result<String, StatusCode> {
    let (path, is_folder) = match item_type.as_str() {
        "file" => (
            PathBuf::from("/home/arpadav/repos/otd/dummy/test_file.txt"),
            false,
        ),
        "folder" => (
            PathBuf::from("/home/arpadav/repos/otd/dummy/test_folder"),
            true,
        ),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if !path.exists() {
        tracing::error!("Path does not exist: {:?}", path);
        return Err(StatusCode::NOT_FOUND);
    }

    let token = Uuid::new_v4().to_string();
    let item = DownloadItem {
        path,
        is_folder,
        downloaded: ArcSwap::from_pointee(false),
    };

    state.tokens.write().await.insert(token.clone(), item);
    Ok(token)
}

async fn download_handler(
    State(state): State<Arc<AppState>>,
    query: Query<DownloadQuery>,
) -> Result<Response, StatusCode> {
    let token = query.k.clone();

    let tokens = state.tokens.read().await;
    let item = tokens.get(&token).ok_or(StatusCode::NOT_FOUND)?;
    
    // Check if already downloaded
    if state.one_time_enabled.load(Ordering::Relaxed) {
        if **item.downloaded.load() {
            return Err(StatusCode::GONE);
        }
        // Mark as downloaded
        item.downloaded.store(true.into());
    }

    serve_content(&item.path, item.is_folder).await
}

async fn serve_content(path: &PathBuf, is_folder: bool) -> Result<Response, StatusCode> {
    if is_folder {
        serve_folder_zip(path).await
    } else {
        serve_file(path).await
    }
}

async fn serve_file(path: &PathBuf) -> Result<Response, StatusCode> {
    let file = match File::open(path).await {
        Ok(file) => file,
        Err(e) => {
            tracing::error!("File open error: {:?} - {}", path, e);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime::APPLICATION_OCTET_STREAM.as_ref().parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );

    Ok(response)
}

async fn serve_folder_zip(_path: &PathBuf) -> Result<Response, StatusCode> {
    // For now, return a placeholder response
    let response = Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .body(Body::from("Folder download not implemented"))
        .unwrap();

    Ok(response)
}

async fn config_handler(
    State(state): State<Arc<AppState>>,
    Path(enabled): Path<bool>,
) -> &'static str {
    state.one_time_enabled.store(enabled, Ordering::Relaxed);
    "Configuration updated"
}
