//! Based on tokio-tungstenite example websocket client, but with multiple
//! concurrent websocket clients in one package
//!
//! This will connect to a server specified in the SERVER with N_CLIENTS
//! concurrent connections, and then flood some test messages over websocket.
//! This will also print whatever it gets into stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! stdout mutex management. Rather it's intended to show an example of working with axum's
//! websocket server and how the client-side and server-side code can be quite similar.
//!

use futures_util::{SinkExt, StreamExt};

// we will use tungstenite for websocket client impl (same library as what axum is using)
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

const SERVER: &str = "ws://127.0.0.1:8910/join";

#[tokio::main]
async fn main() {
    spawn_client(0).await;
}

//creates a client. quietly exits on failure.
async fn spawn_client(who: usize) {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, response)) => {
            println!("Handshake for client {who} has been completed");
            // This will be the HTTP response, same as with server this is the last moment we
            // can still access HTTP stuff.
            println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake for client {who} failed with {e}!");
            return;
        }
    };

    let (mut sender, _receiver) = ws_stream.split();

    //we can ping the server for start
    sender
        .send(Message::Binary(vec![0x01, 1, false as u8, true as u8]))
        .await
        .expect("Can not send!");
}
