use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};

use crate::{GameEvent, ServerEvent};

#[derive(Clone)]
struct AppState {
    server_sender: crossbeam::channel::Sender<ServerEvent>,
    game_receiver: crossbeam::channel::Receiver<GameEvent>,
}

pub fn start_server(
    server_sender: crossbeam::channel::Sender<ServerEvent>,
    game_receiver: crossbeam::channel::Receiver<GameEvent>,
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
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener
            .serve(app.into_make_service())
            .with_graceful_shutdown(async { kill_signal.await.unwrap() })
            .await
            .unwrap();
    });
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state.server_sender, state.game_receiver))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    socket: WebSocket,
    server_sender: crossbeam::channel::Sender<ServerEvent>,
    game_receiver: crossbeam::channel::Receiver<GameEvent>,
) {
    let (sender, receiver) = socket.split();

    tokio::spawn(receive_messages(receiver, server_sender));
    tokio::spawn(send_messages(sender, game_receiver));
}

async fn receive_messages(
    mut socket: SplitStream<WebSocket>,
    server_sender: crossbeam::channel::Sender<ServerEvent>,
) {
    while let Some(msg) = socket.next().await {
        if let Ok(msg) = msg {
            if let Some(event) = parse_msg(msg) {
                server_sender.send(event).unwrap();
            }
        } else {
            tracing::info!("client abruptly disconnected");
            return;
        }
    }
}

fn parse_msg(msg: Message) -> Option<ServerEvent> {
    const INPUT_EVENT_TYPE: u8 = 0x01;

    let Message::Binary(data) = msg else {
        tracing::warn!("Expected binary message: {:?}", msg);
        return None;
    };

    if let &[INPUT_EVENT_TYPE, player_idx, dir] = data.as_slice() {
        let Ok(dir) = dir.try_into() else {
            tracing::warn!("unknown direction: dir");
            return None;
        };
        return Some(ServerEvent::Input { player_idx, dir });
    } else {
        tracing::warn!("unknown message: {:?}", data.as_slice());
    }

    None
}

async fn send_messages(
    mut socket: SplitSink<WebSocket, Message>,
    game_receiver: crossbeam::channel::Receiver<GameEvent>,
) {
    loop {
        let event = game_receiver.recv().unwrap();
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
    }
}
