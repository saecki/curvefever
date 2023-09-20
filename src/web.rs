use std::net::SocketAddr;
use std::sync::mpsc;

use axum::extract::ws::{WebSocket, Message};
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::runtime::Runtime;

use crate::{GameEvent, ServerEvent};

pub fn start_server(
    runtime: &Runtime,
    server_send: mpsc::Sender<ServerEvent>,
    game_recv: mpsc::Receiver<GameEvent>,
) {
    runtime.block_on(async {
        let app = Router::new()
            .route("/join", get(ws_handler));

        axum::Server::bind(&"0.0.0.0:8910".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}

async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let r = ws.on_upgrade::<_, _>(move |socket| handle_socket(socket, addr));
    r
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
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
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }
}
