use async_channel::{Receiver, Sender};
use curvefever_common::{ClientEvent, Direction, GameEvent, Player};
use eframe::CreationContext;
use egui::{CentralPanel, Rect, Sense};
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

const SERVER_URL: &str = "ws://127.0.0.1:8910/join";

fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let (game_sender, game_receiver) = async_channel::unbounded();
    let client_sender = start_websocket(game_sender).unwrap();

    let options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let res = eframe::WebRunner::new()
            .start(
                "curvefever_canvas_id",
                options,
                Box::new(|c| Box::new(CurvefeverRemoteApp::new(c, client_sender, game_receiver))),
            )
            .await;
        if let Err(e) = res {
            log::error!("error running app: {e:?}");
        }
    });
}

struct CurvefeverRemoteApp {
    player: Option<Player>,
    client_sender: ClientSender,
    game_receiver: Receiver<GameEvent>,
}

impl CurvefeverRemoteApp {
    fn new(
        _cc: &CreationContext,
        client_sender: ClientSender,
        game_receiver: Receiver<GameEvent>,
    ) -> Self {
        Self {
            player: None,
            client_sender,
            game_receiver,
        }
    }
}

impl eframe::App for CurvefeverRemoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        if let Ok(msg) = self.game_receiver.try_recv() {
            match msg {
                GameEvent::Exit => {
                    // TODO: exit
                }
                GameEvent::PlayerList(players) => {
                    dbg!(players);
                }
            }
        }

        let mut left_down = false;
        let mut right_down = false;
        CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |uis| {
                {
                    let ui = &mut uis[0];
                    let rect = Rect::from_min_size(ui.cursor().min, ui.available_size());
                    let resp = ui.interact(rect, "left_touch".into(), Sense::click());
                    left_down = resp.is_pointer_button_down_on();
                }
                {
                    let ui = &mut uis[1];
                    let rect = Rect::from_min_size(ui.cursor().min, ui.available_size());
                    let resp = ui.interact(rect, "right_touch".into(), Sense::click());
                    right_down = resp.is_pointer_button_down_on();
                }
            })
        });

        let dir = Direction::from_left_right_down(left_down, right_down);
        let input_event = ClientEvent::Input { player_id: 0, dir };
        self.client_sender.send(input_event);
    }
}

struct ClientSender {
    inner: WebSocket,
}

impl ClientSender {
    fn send(&self, msg: ClientEvent) {
        let mut bytes = Vec::new();
        msg.encode(&mut bytes).unwrap();
        let res = self.inner.send_with_u8_array(&bytes);
        if let Err(e) = res {
            log::error!("Error sending message `{msg:?}`:\n{e:?}");
        }
    }
}

fn start_websocket(sender: Sender<GameEvent>) -> Result<ClientSender, wasm_bindgen::JsValue> {
    use wasm_bindgen::prelude::*;

    let ws = WebSocket::new(SERVER_URL)?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(buf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&buf);
            let len = array.byte_length() as usize;
            let bytes = array.to_vec();
            todo!(
                "parse and send message through sender: {sender:?}, len: {len}, bytes: {bytes:?} "
            );
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            log::debug!("message event, received Text: {:?}", txt);
        } else {
            log::debug!("message event, received Unknown: {:?}", e.data());
        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        log::error!("onerror: {:?}", e);
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: CloseEvent| {
        log::debug!("onclose: {:?}", e);
    });
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    let onopen_callback = Closure::<dyn FnMut(_)>::new(move |e: Event| {
        log::debug!("opopen, {:?}", e);
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let client_sender = ClientSender { inner: ws };

    Ok(client_sender)
}
