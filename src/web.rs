use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::runtime::Runtime;

use crate::{GameEvent, ServerEvent};

pub fn start_server(
    runtime: &Runtime,
    server_sender: crossbeam::channel::Sender<ServerEvent>,
    game_receiver: crossbeam::channel::Receiver<GameEvent>,
    kill_signal: tokio::sync::oneshot::Receiver<()>,
) {
    runtime.block_on(async {
        let app = Router::new()
            .route("/join", get(ws_handler))
            .with_state(Arc::new(Mutex::new(server_sender)));

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener
            .serve(app.into_make_service())
            .with_graceful_shutdown(async { kill_signal.await.unwrap() })
            .await
            .unwrap();
    });
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(server_sender): State<Arc<Mutex<crossbeam::channel::Sender<ServerEvent>>>>,
) -> impl IntoResponse {
    println!("+on_upgrade");
    let r = ws.on_upgrade::<_, _>(move |socket| handle_socket(socket, addr, server_sender));
    println!("-on_upgrade");
    r
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    server_sender: Arc<Mutex<crossbeam::channel::Sender<ServerEvent>>>,
) {
    //send a ping (unsupported by some browsers) just to kick things off and get a response
    // if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
    //     println!("Pinged {}...", who);
    // } else {
    //     println!("Could not send ping {}!", who);
    //     // no Error here since the only thing we can do is to close the connection.
    //     // If we can not send messages, there is no way to salvage the statemachine anyway.
    //     return;
    // }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            let data = msg.into_data();
            if let Some(event) = parse_msg(&data) {
                server_sender.lock().unwrap().send(event).unwrap();
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
        return Some(ServerEvent::Input {
            player_idx,
            left_down: left_down != 0,
            right_down: right_down != 0,
        });
    }

    None
}
