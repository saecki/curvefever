use std::time::Duration;

use futures_util::{SinkExt, StreamExt};

use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

const SERVER: &str = "ws://127.0.0.1:8910/join";

#[tokio::main]
async fn main() {
    spawn_client().await;
}

async fn spawn_client() {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, response)) => {
            println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake failed with {e}!");
            return;
        }
    };

    let (mut sender, _receiver) = ws_stream.split();

    for i in 0_u32..1000_u32 {
        let dir = (i % 3) as u8;

        //we can ping the server for start
        sender
            .send(Message::Binary(vec![0x01, 1, dir]))
            .await
            .expect("Can not send!");

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
