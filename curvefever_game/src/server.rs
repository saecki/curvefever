use async_channel::{Receiver, Sender};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use curvefever_common::{ClientEvent, GameEvent};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    server_sender: Sender<ClientEvent>,
    game_receiver: Receiver<GameEvent>,
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
        let state = AppState {
            server_sender,
            game_receiver,
        };
        let app = Router::new()
            .route("/", get(root))
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = TcpListener::bind(&"0.0.0.0:8910").await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async { kill_signal.await.unwrap() })
            .await
            .unwrap();
    });
}

async fn root(State(state): State<AppState>) -> impl IntoResponse {
    // TODO: serve app
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state.server_sender, state.game_receiver))
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
                // TODO: respond to ListPlayers
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
    match event {
        GameEvent::Exit => todo!(),
        GameEvent::PlayerList(_) => todo!(),
    }
}
