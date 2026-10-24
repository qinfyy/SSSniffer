use crate::{
    capture::{self, Packet},
    protocol::Protocol,
};
use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{HeaderMap, HeaderName, Method, StatusCode, Uri, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
#[exclude = "**/*.map"]
struct Assets;

pub struct AppState {
    pub directory: PathBuf,
    pub protocol: Protocol,
    capture: Mutex<CaptureState>,
    hook_error: Mutex<Option<String>>,
}

#[derive(Default)]
struct CaptureState {
    clients: HashMap<String, Arc<Client>>,
    active: Option<String>,
    history: Vec<Arc<Packet>>,
}

#[derive(Default)]
struct Client {
    log: Mutex<ClientLog>,
    changed: Notify,
}

#[derive(Default)]
struct ClientLog {
    packets: Vec<Arc<Packet>>,
    replayed: bool,
    closed: bool,
}

impl Client {
    fn publish(&self, packet: Arc<Packet>) {
        self.log
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .packets
            .push(packet);
        self.changed.notify_waiters();
    }

    fn replay(&self, history: &[Arc<Packet>]) {
        let mut log = self.log.lock().unwrap_or_else(|error| error.into_inner());
        if log.replayed {
            return;
        }
        log.packets.extend_from_slice(history);
        log.replayed = true;
        drop(log);
        self.changed.notify_waiters();
    }

    fn close(&self) {
        self.log
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .closed = true;
        self.changed.notify_waiters();
    }
}

impl AppState {
    pub fn new(directory: PathBuf, protocol: Protocol) -> Self {
        Self {
            directory,
            protocol,
            capture: Mutex::new(CaptureState::default()),
            hook_error: Mutex::new(None),
        }
    }

    pub fn set_hook_error(&self, error: String) {
        *self
            .hook_error
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(error);
    }

    pub fn record(&self, packet: Arc<Packet>) {
        let mut capture = self
            .capture
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // Serialize history replay and live delivery under one lock to avoid gaps or duplicates.
        capture.history.push(packet.clone());
        if let Some(client) = capture
            .active
            .as_ref()
            .and_then(|id| capture.clients.get(id))
        {
            client.publish(packet);
        }
    }

    fn publish(&self, client: &str, packet: Arc<Packet>) {
        if let Some(client) = self
            .capture
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clients
            .get(client)
        {
            client.publish(packet);
        }
    }

    fn client(&self, query: &ClientQuery) -> ApiResult<Arc<Client>> {
        self.capture
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clients
            .get(&query.client_id)
            .cloned()
            .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Client is not registered".into()))
    }
}

struct ApiError(StatusCode, String);
type ApiResult<T> = std::result::Result<T, ApiError>;
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

#[derive(Deserialize)]
struct ClientQuery {
    #[serde(rename = "clientID")]
    client_id: String,
}

pub async fn serve(listener: tokio::net::TcpListener, state: Arc<AppState>) -> Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .to_str()
                .ok()
                .and_then(|origin| origin.parse::<Uri>().ok())
                .is_some_and(|origin| {
                    matches!(origin.scheme_str(), Some("http" | "https"))
                        && origin.host().is_some_and(|host| {
                            matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1")
                        })
                })
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("last-event-id"),
        ]);
    let app = Router::new()
        .route("/api/registration", get(register))
        .route("/api/unregistration", post(unregister))
        .route("/api/start", get(start_capture))
        .route("/api/stop", get(stop_capture))
        .route("/api/stream", get(stream))
        .route("/api/upload", post(upload))
        .route("/api/GetConfig", get(ok))
        .route("/api/SetConfig", get(ok).post(ok))
        .fallback(static_asset)
        .layer(DefaultBodyLimit::max(128 * 1024 * 1024))
        .layer(cors)
        .with_state(state);
    axum::serve(listener, app)
        .await
        .context("HTTP server failed")
}

async fn register(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
) -> ApiResult<&'static str> {
    if query.client_id.is_empty() || query.client_id.len() > 128 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "Invalid clientID".into()));
    }
    let mut capture = state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let clients = &mut capture.clients;
    if clients.len() >= 64 && !clients.contains_key(&query.client_id) {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many clients".into(),
        ));
    }
    clients
        .entry(query.client_id)
        .or_insert_with(|| Arc::new(Client::default()));
    Ok("OK")
}

async fn unregister(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
) -> &'static str {
    let mut capture = state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(client) = capture.clients.remove(&query.client_id) {
        client.close();
    }
    if capture.active.as_deref() == Some(&query.client_id) {
        capture.active = None;
    }
    "OK"
}

async fn start_capture(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
) -> ApiResult<&'static str> {
    if let Some(error) = state
        .hook_error
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
    {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, error.clone()));
    }
    let mut capture = state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let client = capture
        .clients
        .get(&query.client_id)
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Client is not registered".into()))?;
    client.replay(&capture.history);
    capture.active = Some(query.client_id);
    Ok("OK")
}

async fn stop_capture(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
) -> ApiResult<&'static str> {
    let mut capture = state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !capture.clients.contains_key(&query.client_id) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Client is not registered".into(),
        ));
    }
    if capture.active.as_deref() == Some(&query.client_id) {
        capture.active = None;
    }
    Ok("OK")
}

async fn stream(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let client = state.client(&query)?;
    let mut cursor = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(
            client
                .log
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .packets
                .len(),
        );
    let stream = async_stream::stream! {
        loop {
            let changed = client.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let (packets, closed) = {
                let log = client.log.lock().unwrap_or_else(|error| error.into_inner());
                let end = cursor.saturating_add(128).min(log.packets.len());
                (log.packets[cursor..end].to_vec(), log.closed)
            };
            if packets.is_empty() {
                if closed {
                    break;
                }
                changed.await;
                continue;
            }
            for packet in packets {
                cursor += 1;
                if let Ok(event) = Event::default().event("packetNotify").id(cursor.to_string()).json_data(&*packet) {
                    yield Ok::<Event, Infallible>(event);
                }
            }
            tokio::task::yield_now().await;
        }
    };
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

async fn ok() -> &'static str {
    "OK"
}

async fn upload(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ClientQuery>,
    mut multipart: Multipart,
) -> ApiResult<&'static str> {
    state.client(&query)?;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.to_string()))?;
        let packets = tokio::task::spawn_blocking(move || capture::import(&bytes))
            .await
            .map_err(|error| ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, format!("{error:#}")))?;
        for (index, packet) in packets.into_iter().enumerate() {
            state.publish(&query.client_id, Arc::new(packet));
            if index % 64 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
        return Ok("OK");
    }
    Err(ApiError(
        StatusCode::BAD_REQUEST,
        "Missing file field".into(),
    ))
}

async fn static_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(asset) = Assets::get(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        ([(header::CONTENT_TYPE, mime)], asset.data.into_owned()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
