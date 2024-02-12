use std::sync::Arc;

use async_channel::{Receiver, Sender};
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{header, HeaderValue, Response};
use axum::response::IntoResponse;
use axum::routing::{get, MethodRouter};
use axum::Router;
use curvefever_common::{ClientEvent, GameEvent};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[rustfmt::skip]
mod files {
    pub const INDEX_HTML: &'static [u8] = include_bytes!("../../curvefever_remote/dist/index.html");
    pub const APP_JS: &'static [u8] = include_bytes!("../../curvefever_remote/dist/curvefever_remote.js");
    pub const APP_WASM: &'static [u8] = include_bytes!("../../curvefever_remote/dist/curvefever_remote_bg.wasm");
    pub const MANIFEST_JSON: &'static [u8] = include_bytes!("../../curvefever_remote/dist/manifest.json");
    pub const SW_JS: &'static [u8] = include_bytes!("../../curvefever_remote/dist/sw.js");
}

struct AppState {
    next_session_id: u64,
    server_sender: Sender<ClientEvent>,
    sessions: Vec<Session>,
}

impl AppState {
    fn next_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }
}

struct Session {
    id: u64,
    sender: Sender<GameEvent>,
}

pub fn start_server(
    server_sender: Sender<ClientEvent>,
    game_receiver: Receiver<GameEvent>,
    kill_signal: tokio::sync::oneshot::Receiver<()>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        let state = Arc::new(RwLock::new(AppState {
            next_session_id: 0,
            server_sender,
            sessions: Vec::new(),
        }));

        let state_ref = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                let Ok(event) = game_receiver.recv().await else {
                    tracing::debug!("Exiting game message loop");
                    break;
                };

                let state = state_ref.read().await;
                for c in state.sessions.iter() {
                    let res = c.sender.send(event.clone()).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending game event to client session:\n{e}");
                    }
                }
            }
        });

        let app = Router::new()
            .route(
                "/",
                get_embedded_file("text/html; charset=utf-8", files::INDEX_HTML),
            )
            .route(
                "/index.html",
                get_embedded_file("text/html; charset=utf-8", files::INDEX_HTML),
            )
            .route(
                "/curvefever_remote.js",
                get_embedded_file("text/javascript; charset=utf-8", files::APP_JS),
            )
            .route(
                "/curvefever_remote_bg.wasm",
                get_embedded_file("application/wasm", files::APP_WASM),
            )
            .route(
                "/manifest.json",
                get_embedded_file("application/json", files::MANIFEST_JSON),
            )
            .route("/sw.js", get_embedded_file("text/javascript", files::SW_JS))
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = TcpListener::bind(&"0.0.0.0:8910").await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async { kill_signal.await.unwrap() })
            .await
            .unwrap();
    });
}

fn get_embedded_file<T>(
    content_type: &'static str,
    bytes: &'static [u8],
) -> MethodRouter<T, std::convert::Infallible>
where
    T: Clone + Send + Sync + 'static,
{
    get(move || serve_embedded_file(content_type, bytes))
}

async fn serve_embedded_file(
    content_type: &'static str,
    bytes: &'static [u8],
) -> impl IntoResponse {
    let body = Body::from(bytes);
    let mut resp = Response::new(body);
    let headers = resp.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
    resp
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    let (sender, receiver) = async_channel::unbounded();
    let server_sender;
    let id;
    {
        let mut state = state.write().await;
        id = state.next_session_id();
        state.sessions.push(Session { id, sender });
        server_sender = state.server_sender.clone();
    }
    tracing::debug!("Session with id {} connected", id);

    ws.on_upgrade(move |socket| handle_socket(id, state, socket, server_sender, receiver))
}

async fn handle_socket(
    id: u64,
    state: Arc<RwLock<AppState>>,
    socket: WebSocket,
    server_sender: Sender<ClientEvent>,
    game_receiver: Receiver<GameEvent>,
) {
    let (sender, receiver) = socket.split();

    tokio::spawn(receiver_task(id, state, receiver, server_sender));
    tokio::spawn(sender_task(sender, game_receiver));
}

async fn receiver_task(
    id: u64,
    state: Arc<RwLock<AppState>>,
    mut socket: SplitStream<WebSocket>,
    server_sender: Sender<ClientEvent>,
) {
    while let Some(Ok(msg)) = socket.next().await {
        let Message::Binary(data) = msg else {
            tracing::warn!("Expected binary message: {:?}", msg);
            continue;
        };

        let mut cursor = std::io::Cursor::new(&data);
        let event = match ClientEvent::decode(&mut cursor) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Error decoding message `{:?}`:\n{e}", data.as_slice());
                continue;
            }
        };

        let res = server_sender.send(event).await;
        if let Err(e) = res {
            tracing::error!("Error sending client event to server:\n{e}");
        }
    }

    let mut state = state.write().await;
    if let Some(i) = state.sessions.iter().position(|s| s.id == id) {
        tracing::debug!("Session with id {} disconnected", id);
        let session = state.sessions.remove(i);
        session.sender.close();
    }
}

async fn sender_task(
    mut socket: SplitSink<WebSocket, Message>,
    game_receiver: Receiver<GameEvent>,
) {
    loop {
        let Ok(event) = game_receiver.recv().await else {
            break;
        };

        let mut buf = Vec::new();
        event.encode(&mut buf).expect("should always succeed");
        let msg = Message::Binary(buf);

        let res = socket.send(msg).await;
        if let Err(e) = res {
            tracing::warn!("Error sending game event to client socket: {e}");
        }
    }
}
