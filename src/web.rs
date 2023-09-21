use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{WebSocket, Message};
use axum::extract::{ConnectInfo, WebSocketUpgrade, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::runtime::Runtime;

use crate::{GameEvent, ServerEvent};

pub fn start_server(
    runtime: &Runtime,
    server_send: crossbeam::channel::Sender<ServerEvent>,
    game_recv: crossbeam::channel::Receiver<GameEvent>,
) {
    runtime.block_on(async {
        let app = Router::new()
            .with_state(Arc::new(Mutex::new(server_send)))
            .route("/join", get(ws_handler));

        axum::Server::bind(&"0.0.0.0:8910".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(server_send): State<Arc<Mutex<crossbeam::channel::Sender<ServerEvent>>>>,
) -> impl IntoResponse {
    ws.on_upgrade::<_, _>(move |socket| handle_socket(socket, addr, server_send))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    server_send: Arc<Mutex<crossbeam::channel::Sender<ServerEvent>>>,
) {
    //send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    }

    // receive single message from a client (we can either receive or send with socket).
    // this will likely be the Pong for our Ping or a hello message from client.
    // waiting for message from a client will block this task, but will not block other client's
    // connections.
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            let data = msg.into_data();
            if let Some(e) = parse_msg(&data) {
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }
}

fn parse_msg(msg: &[u8]) -> Option<ServerEvent> {
    const INPUT_EVENT_TYPE: u8 = 0x01;
    if let &[INPUT_EVENT_TYPE, player_idx, left_down, right_down] = msg {
        return Some(ServerEvent::Input { player_idx, left_down: left_down != 0, right_down: left_down != 0 });
    }

    None
}
