use async_channel::{Receiver, Sender};
use curvefever_common::{ClientEvent, Direction, GameEvent, Player};
use eframe::CreationContext;
use egui::{
    Align, Button, CentralPanel, Color32, FontFamily, FontId, Frame, Key, Margin, Rect, RichText,
    Rounding, ScrollArea, Sense, TextEdit, Vec2, WidgetText,
};
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

const TEXT_SIZE: f32 = 20.0;

fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let base_url = document.url().unwrap();
    let base_url = base_url.strip_prefix("http://").unwrap_or(&base_url);
    let base_url = base_url.strip_prefix("https://").unwrap_or(&base_url);
    let url = format!("ws://{base_url}join");

    let (game_sender, game_receiver) = async_channel::unbounded();
    let client_sender = start_websocket(&url, game_sender).unwrap();

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
    players: Vec<Player>,
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
            players: Vec::new(),
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
                    self.player = None;
                }
                GameEvent::PlayerList(players) => {
                    if let Some(current) = &self.player {
                        // remove or update player
                        self.player = players
                            .iter()
                            .find(|p| p.id == current.id)
                            .map(Clone::clone);
                    }
                    self.players = players;
                }
            }
        }

        if let Some(player) = &self.player {
            let left = self.draw_controls(ctx, player);
            if left {
                self.player = None;
            }
        } else {
            self.draw_home(ctx);
        }
    }
}

impl CurvefeverRemoteApp {
    fn draw_controls(&self, ctx: &egui::Context, player: &Player) -> bool {
        let mut back = false;
        let mut left_down = false;
        let mut right_down = false;
        let mut restart = false;
        ctx.input(|i| {
            left_down |= i.key_down(Key::ArrowLeft);
            right_down |= i.key_down(Key::ArrowRight);
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |uis| {
                {
                    let ui = &mut uis[0];
                    Frame::none()
                        .rounding(Rounding::same(8.0))
                        .fill(Color32::from_gray(0x18))
                        .show(ui, |ui| {
                            let rect = Rect::from_min_size(ui.cursor().min, ui.available_size());
                            ui.allocate_ui_at_rect(rect, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new("right").size(32.0));
                                });
                            });
                            let resp = ui.allocate_rect(rect, Sense::click_and_drag());
                            left_down |= resp.is_pointer_button_down_on() | resp.dragged();
                        });
                }
                {
                    let ui = &mut uis[1];
                    ui.vertical_centered(|ui| {
                        Frame::none()
                            .outer_margin(Margin::symmetric(0.0, 24.0))
                            .show(ui, |ui| {
                                let mut buf = String::from(&player.name);
                                TextEdit::singleline(&mut buf)
                                    .frame(false)
                                    .horizontal_align(Align::Center)
                                    .font(FontId::new(1.5 * TEXT_SIZE, FontFamily::Proportional))
                                    .text_color(player_color(player))
                                    .show(ui);

                                if buf != player.name {
                                    self.client_sender.send(ClientEvent::Rename {
                                        player_id: player.id,
                                        name: buf,
                                    })
                                }

                                ui.add_space(8.0);

                                if button(ui, RichText::new("back").size(TEXT_SIZE)) {
                                    back = true;
                                }
                                if button(ui, RichText::new("restart").size(TEXT_SIZE)) {
                                    restart = true;
                                }

                                ui.columns(2, |uis| {
                                    let ui = &mut uis[0];
                                    if button(ui, RichText::new("prev color").size(TEXT_SIZE)) {
                                        self.client_sender.send(ClientEvent::PrevColor {
                                            player_id: player.id,
                                        });
                                    }
                                    let ui = &mut uis[1];
                                    if button(ui, RichText::new("next color").size(TEXT_SIZE)) {
                                        self.client_sender.send(ClientEvent::NextColor {
                                            player_id: player.id,
                                        });
                                    }
                                });
                            })
                    });
                }
                {
                    let ui = &mut uis[2];
                    Frame::none()
                        .rounding(Rounding::same(8.0))
                        .fill(Color32::from_gray(0x18))
                        .show(ui, |ui| {
                            let rect = Rect::from_min_size(ui.cursor().min, ui.available_size());
                            ui.allocate_ui_at_rect(rect, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new("right").size(32.0));
                                });
                            });
                            let resp = ui.allocate_rect(rect, Sense::click_and_drag());
                            right_down |= resp.is_pointer_button_down_on() | resp.dragged();
                        });
                }
            })
        });

        let dir = Direction::from_left_right_down(left_down, right_down);
        let input_event = ClientEvent::Input {
            player_id: player.id,
            dir,
        };
        self.client_sender.send(input_event);

        if restart {
            self.client_sender.send(ClientEvent::Restart);
        }

        back
    }

    fn draw_home(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |uis| {
                let ui = &mut uis[1];
                ui.vertical_centered(|ui| {
                    Frame::none()
                        .outer_margin(Margin::symmetric(0.0, 24.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Players").size(1.5 * TEXT_SIZE));
                            ui.add_space(8.0);

                            ScrollArea::vertical().show(ui, |ui| {
                                for p in self.players.iter() {
                                    if button(ui, player_text(p).size(TEXT_SIZE)) {
                                        self.player = Some(p.clone());
                                    }
                                }
                            })
                        });
                });
            });
        });
    }
}

fn button(ui: &mut egui::Ui, text: impl Into<WidgetText>) -> bool {
    ui.add_space(8.0);

    let button_size = Vec2::new(ui.available_size().x, 2.0 * TEXT_SIZE);
    let resp = ui.add_sized(button_size, Button::new(text).rounding(Rounding::same(8.0)));
    resp.clicked()
}

fn player_text(player: &Player) -> RichText {
    RichText::new(&player.name).color(player_color(player))
}

fn player_color(player: &Player) -> Color32 {
    unsafe { std::mem::transmute(player.color) }
}

#[derive(Clone)]
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

fn start_websocket(
    url: &str,
    sender: Sender<GameEvent>,
) -> Result<ClientSender, wasm_bindgen::JsValue> {
    use wasm_bindgen::prelude::*;

    let ws = WebSocket::new(url)?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(buf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&buf);
            let bytes = array.to_vec();
            let mut cursor = std::io::Cursor::new(&bytes);
            match GameEvent::decode(&mut cursor) {
                Ok(msg) => {
                    log::debug!("received message: {:?}", msg);
                    sender.try_send(msg).unwrap();
                }
                Err(e) => {
                    log::error!("Error decoding message:\n{}", e);
                }
            }
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

    let cloned_client = ClientSender { inner: ws.clone() };
    let onopen_callback = Closure::<dyn FnMut(_)>::new(move |e: Event| {
        cloned_client.send(ClientEvent::SyncPlayers);
        log::debug!("onopen, {:?}", e);
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let client_sender = ClientSender { inner: ws };

    Ok(client_sender)
}
