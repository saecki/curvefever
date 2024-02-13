use async_channel::{Receiver, Sender};
use curvefever_common::{ClientEvent, Direction, GameEvent, Player};
use eframe::CreationContext;
use egui::{
    Align, Align2, Button, CentralPanel, Color32, FontFamily, FontId, Frame, Key, Margin, Rect,
    RichText, Rounding, ScrollArea, Sense, TextEdit, Vec2, WidgetText,
};
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

const TEXT_SIZE: f32 = 20.0;
const BUTTON_SPACE: f32 = 8.0;

fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let base_url = &document.url().unwrap();
    let base_url = base_url.strip_prefix("http://").unwrap_or(base_url);
    let base_url = base_url.strip_prefix("https://").unwrap_or(base_url);
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
    add_request_id: Option<u64>,
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
            add_request_id: None,
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
                GameEvent::PlayerSync { players } => {
                    if let Some(current) = &self.player {
                        // remove or update current player
                        self.player = players.iter().find(|p| p.id == current.id).cloned();
                    }
                    self.players = players;
                }
                GameEvent::PlayerAdded { request_id, player } => {
                    if self.add_request_id == Some(request_id) {
                        self.player = Some(player);
                        request_fullscreen();
                    }
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
        let mut input_event = None;

        if ctx.memory(|m| m.focus().is_none()) {
            ctx.input(|i| {
                left_down |= i.key_down(Key::ArrowLeft);
                right_down |= i.key_down(Key::ArrowRight);

                if i.key_pressed(Key::Space) {
                    input_event = Some(ClientEvent::Restart);
                } else if i.key_pressed(Key::Escape) {
                    input_event = Some(ClientEvent::Pause);
                } else if i.key_pressed(Key::S) {
                    input_event = Some(ClientEvent::Share);
                } else if i.key_pressed(Key::H) {
                    input_event = Some(ClientEvent::Help);
                }
            });
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |uis| {
                left_down |= touch_pad(&mut uis[0], "left");

                {
                    let ui = &mut uis[1];
                    ui.vertical_centered(|ui| {
                        Frame::none()
                            .outer_margin(Margin::symmetric(0.0, 16.0))
                            .show(ui, |ui| {
                                let mut buf = String::from(&player.name);
                                let resp = TextEdit::singleline(&mut buf)
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

                                if resp.response.has_focus() {
                                    ui.input(|i| {
                                        if i.key_pressed(Key::Enter) {
                                            resp.response.surrender_focus();
                                        }
                                    });
                                }

                                ui.add_space(2.0 * BUTTON_SPACE);

                                if button(ui, RichText::new("back").size(TEXT_SIZE)) {
                                    back = true;
                                }
                                ui.add_space(BUTTON_SPACE);
                                if button(ui, RichText::new("restart").size(TEXT_SIZE)) {
                                    input_event = Some(ClientEvent::Restart);
                                }
                                ui.add_space(BUTTON_SPACE);
                                if button(ui, RichText::new("pause").size(TEXT_SIZE)) {
                                    input_event = Some(ClientEvent::Pause);
                                }
                                ui.add_space(BUTTON_SPACE);
                                if button(ui, RichText::new("share").size(TEXT_SIZE)) {
                                    input_event = Some(ClientEvent::Share);
                                }
                                ui.add_space(BUTTON_SPACE);
                                if button(ui, RichText::new("help").size(TEXT_SIZE)) {
                                    input_event = Some(ClientEvent::Help);
                                }
                                ui.add_space(BUTTON_SPACE);
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

                right_down |= touch_pad(&mut uis[2], "right");
            })
        });

        let dir = Direction::from_left_right_down(left_down, right_down);
        self.client_sender.send(ClientEvent::Input {
            player_id: player.id,
            dir,
        });

        if let Some(event) = input_event {
            self.client_sender.send(event);
        }

        back
    }

    fn draw_home(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.columns(3, |uis| {
                let ui = &mut uis[1];
                ui.vertical_centered(|ui| {
                    Frame::none()
                        .outer_margin(Margin::symmetric(0.0, 16.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Players").size(1.5 * TEXT_SIZE));

                            ui.add_space(2.0 * BUTTON_SPACE);

                            if button(ui, RichText::new("add player").size(TEXT_SIZE)) {
                                let request_id = rand::random();
                                self.add_request_id = Some(request_id);
                                self.client_sender
                                    .send(ClientEvent::AddPlayer { request_id });
                            }

                            ui.add_space(BUTTON_SPACE);

                            ScrollArea::vertical().show(ui, |ui| {
                                for p in self.players.iter() {
                                    if button(ui, player_text(p).size(TEXT_SIZE)) {
                                        self.player = Some(p.clone());
                                        request_fullscreen();
                                    }
                                    ui.add_space(BUTTON_SPACE);
                                }
                            })
                        });
                });
            });
        });
    }
}

fn request_fullscreen() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(element) = document.document_element() else {
        return;
    };
    if let Err(e) = element.request_fullscreen() {
        log::error!("Error requresting fullscreen: {e:?}");
    }
}

fn touch_pad(ui: &mut egui::Ui, name: &str) -> bool {
    let mut down = false;
    Frame::none().show(ui, |ui| {
        let rect = Rect::from_min_size(ui.cursor().min, ui.available_size());
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click());
        down |= resp.contains_pointer() && ui.input(|i| i.pointer.primary_down());

        let bg_fill = if down {
            Color32::from_rgba_unmultiplied(0x30, 0x50, 0xc0, 0x10)
        } else {
            Color32::from_gray(0x20)
        };
        painter.rect_filled(rect, Rounding::same(8.0), bg_fill);

        let text_color = if down {
            Color32::from_rgb(0x30, 0x60, 0xff)
        } else {
            Color32::from_gray(0xa0)
        };
        let font = FontId::new(1.5 * TEXT_SIZE, FontFamily::Monospace);
        painter.text(rect.center(), Align2::CENTER_CENTER, name, font, text_color);
    });

    down
}

fn button(ui: &mut egui::Ui, text: impl Into<WidgetText>) -> bool {
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
    fn send(&self, event: ClientEvent) {
        let mut buf = Vec::new();
        event.encode(&mut buf).expect("should always succeed");
        let res = self.inner.send_with_u8_array(&buf);
        if let Err(e) = res {
            log::error!("Error sending message `{event:?}`:\n{e:?}");
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
                Ok(event) => {
                    log::debug!("Received game event: {event:?}");
                    sender.try_send(event).unwrap();
                }
                Err(e) => {
                    log::error!("Error decoding message:\n{e}");
                }
            }
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            log::debug!("Received Text message: {:?}", txt);
        } else {
            log::debug!("Received Unknown message: {:?}", e.data());
        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        log::error!("onerror: {e:?}");
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: CloseEvent| {
        log::debug!("onclose: {e:?}");
    });
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    let cloned_client = ClientSender { inner: ws.clone() };
    let onopen_callback = Closure::<dyn FnMut(_)>::new(move |e: Event| {
        cloned_client.send(ClientEvent::SyncPlayers);
        log::debug!("onopen, {e:?}");
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let client_sender = ClientSender { inner: ws };

    Ok(client_sender)
}
