use std::sync::Arc;

use async_channel::{Receiver, Sender};
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::handler::Handler;
use axum::http::{header, HeaderValue, Response};
use axum::response::IntoResponse;
use axum::routing::{get, MethodRouter};
use axum::Router;
use curvefever_common::{ClientEvent, GameEvent};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;

#[rustfmt::skip]
mod files {
    pub const INDEX_HTML: &'static [u8] = include_bytes!("../../curvefever_remote/dist/index.html");
    pub const APP_JS: &'static [u8] = include_bytes!("../../curvefever_remote/dist/curvefever_remote.js");
    pub const APP_WASM: &'static [u8] = include_bytes!("../../curvefever_remote/dist/curvefever_remote_bg.wasm");
    pub const MANIFEST_JSON: &'static [u8] = include_bytes!("../../curvefever_remote/dist/manifest.json");
    pub const SW_JS: &'static [u8] = include_bytes!("../../curvefever_remote/dist/sw.js");
}

struct AppState {
    server_sender: Sender<ClientEvent>,
    clients: tokio::sync::RwLock<Vec<Sender<GameEvent>>>,
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
        let state = Arc::new(AppState {
            server_sender,
            clients: tokio::sync::RwLock::new(Vec::new()),
        });

        let state_ref = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                let msg = game_receiver.recv().await.unwrap();

                {
                    let clients = state_ref.clients.read().await;
                    for c in clients.iter() {
                        c.send(msg.clone()).await;
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
    get(move || async move { serve_embedded_file(content_type, bytes) })
}

fn serve_embedded_file(content_type: &'static str, bytes: &'static [u8]) -> impl IntoResponse {
    let body = Body::from(bytes);
    let mut resp = Response::new(body);
    let headers = resp.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
    resp
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (sender, receiver) = async_channel::unbounded();
    {
        let mut lock = state.clients.write().await;
        lock.push(sender);
    }

    let server_sender = state.server_sender.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, server_sender, receiver))
}

async fn handle_socket(
    socket: WebSocket,
    server_sender: Sender<ClientEvent>,
    game_receiver: Receiver<GameEvent>,
) {
    let (sender, receiver) = socket.split();

    tokio::spawn(receive_messages(receiver, server_sender));
    tokio::spawn(send_messages(sender, game_receiver));
}

async fn receive_messages(mut socket: SplitStream<WebSocket>, server_sender: Sender<ClientEvent>) {
    while let Some(msg) = socket.next().await {
        if let Ok(msg) = msg {
            if let Some(event) = parse_msg(msg) {
                server_sender.send(event).await.unwrap();
            }
        } else {
            tracing::info!("client abruptly disconnected");
            return;
        }
    }
}

fn parse_msg(msg: Message) -> Option<ClientEvent> {
    let Message::Binary(data) = msg else {
        tracing::warn!("Expected binary message: {:?}", msg);
        return None;
    };

    let mut cursor = std::io::Cursor::new(&data);
    match ClientEvent::decode(&mut cursor) {
        Ok(e) => return Some(e),
        Err(e) => {
            tracing::warn!("Error decoding message `{:?}`:\n{e}", data.as_slice());
        }
    }

    None
}

async fn send_messages(
    mut socket: SplitSink<WebSocket, Message>,
    game_receiver: Receiver<GameEvent>,
) {
    loop {
        let Ok(event) = game_receiver.recv().await else {
            break;
        };
        let msg = to_msg(event);
        let res = socket.send(msg).await;
        if let Err(e) = res {
            tracing::warn!("Error sending game event: {e}");
        }
    }
}

fn to_msg(event: GameEvent) -> Message {
    let mut buf = Vec::new();
    event.encode(&mut buf).unwrap();
    Message::Binary(buf)
}
