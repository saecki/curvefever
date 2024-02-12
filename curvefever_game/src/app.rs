use std::f32::consts::{PI, TAU};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant};

use async_channel::{Receiver, Sender};
use curvefever_common::{ClientEvent, Direction, GameEvent};
use eframe::CreationContext;
use egui::epaint::{PathShape, RectShape};
use egui::layers::ShapeIdx;
use egui::{
    Align2, CentralPanel, Color32, Context, Event, FontFamily, FontId, Frame, Id, Key, Painter,
    Pos2, Rect, Rounding, Shape, Stroke, Vec2,
};
use qrcode::QrCode;

use crate::world::{
    CrashMessage, GameState, Item, Player, TrailSection, TurnDirection, World, BASE_THICKNESS,
    ITEM_KINDS, ITEM_RADIUS, PLAYER_COLORS, START_DELAY, UPDATE_TIME, WORLD_SIZE,
};

pub const PLAYER_MENU_FIELDS: usize = 3;

pub struct CurvefeverApp {
    bg_thread: Option<std::thread::JoinHandle<()>>,
    game_sender: Sender<GameEvent>,
    world: Arc<RwLock<World>>,
    menu: Arc<RwLock<Menu>>,
    world_to_screen_offset: Vec2,
    world_to_screen_scale: f32,
    local_url: String,
    qrcode: QrCode,
}

impl CurvefeverApp {
    #[inline(always)]
    fn wts_pos(&self, pos: Pos2) -> Pos2 {
        Pos2::new(
            self.world_to_screen_scale * pos.x,
            self.world_to_screen_scale * pos.y,
        ) + self.world_to_screen_offset
    }

    #[inline(always)]
    fn wts_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.wts_pos(rect.min), self.wts_pos(rect.max))
    }

    #[inline(always)]
    fn wts_rounding(&self, rounding: Rounding) -> Rounding {
        Rounding {
            nw: self.world_to_screen_scale * rounding.nw,
            ne: self.world_to_screen_scale * rounding.ne,
            sw: self.world_to_screen_scale * rounding.sw,
            se: self.world_to_screen_scale * rounding.se,
        }
    }

    #[inline(always)]
    fn stw_pos(&self, pos: Pos2) -> Pos2 {
        let pos = pos - self.world_to_screen_offset;
        Pos2::new(
            pos.x / self.world_to_screen_scale,
            pos.y / self.world_to_screen_scale,
        )
    }

    #[inline(always)]
    fn stw_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.stw_pos(rect.min), self.stw_pos(rect.max))
    }
}

#[derive(Clone, Copy, Default)]
struct Menu {
    state: MenuState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MenuState {
    #[default]
    Home,
    Help,
    Share,
    Player(PlayerMenu),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PlayerMenu {
    player_index: usize,
    field_index: usize,
    selection_active: bool,
}

impl PlayerMenu {
    fn selection_left(&mut self) {
        if self.field_index == 0 {
            self.field_index = PLAYER_MENU_FIELDS - 1;
        } else {
            self.field_index -= 1;
        }
    }

    fn selection_right(&mut self) {
        self.field_index += 1;
        self.field_index %= PLAYER_MENU_FIELDS;
    }

    fn selection_up(&mut self, num_players: usize) {
        if self.player_index == 0 {
            self.player_index = num_players - 1;
        } else {
            self.player_index -= 1;
        }
    }

    fn selection_down(&mut self, num_players: usize) {
        self.player_index += 1;
        self.player_index %= num_players;
    }
}

impl CurvefeverApp {
    pub fn new(
        cc: &CreationContext,
        server_receiver: Receiver<ClientEvent>,
        game_sender: Sender<GameEvent>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let world = Arc::new(RwLock::new(World::new()));
        let menu = Arc::new(RwLock::new(Menu::default()));

        let bg_world = Arc::clone(&world);
        let bg_menu = Arc::clone(&menu);
        let bg_game_sender = game_sender.clone();
        let bg_thread = std::thread::spawn(move || {
            let mut start = Instant::now();
            loop {
                {
                    let mut world = bg_world.write().unwrap();
                    if !world.is_running {
                        break;
                    }

                    while let Ok(e) = server_receiver.try_recv() {
                        match e {
                            ClientEvent::SyncPlayers => {
                                sync_players(&bg_game_sender, &world.players);
                            }
                            ClientEvent::Restart => {
                                world.restart();
                            }
                            ClientEvent::Input { player_id, dir } => {
                                if let Some(p) = find_player(&mut world.players, player_id) {
                                    p.remote_direction = dir;
                                }
                            }
                            ClientEvent::PrevColor { player_id } => {
                                if matches!(&world.state, GameState::Stopped(_)) {
                                    if let Some(p) = find_player(&mut world.players, player_id) {
                                        p.color.prev();
                                        sync_players(&bg_game_sender, &world.players);
                                    }
                                }
                            }
                            ClientEvent::NextColor { player_id } => {
                                if matches!(&world.state, GameState::Stopped(_)) {
                                    if let Some(p) = find_player(&mut world.players, player_id) {
                                        p.color.next();
                                        sync_players(&bg_game_sender, &world.players);
                                    }
                                }
                            }
                            ClientEvent::Rename { player_id, name } => {
                                if let Some(p) = find_player(&mut world.players, player_id) {
                                    p.name = name;
                                    sync_players(&bg_game_sender, &world.players);
                                }
                            }
                            ClientEvent::Share => {
                                if matches!(&world.state, GameState::Stopped(_)) {
                                    let mut menu = bg_menu.write().unwrap();
                                    if menu.state == MenuState::Share {
                                        menu.state = MenuState::Home;
                                    } else {
                                        menu.state = MenuState::Share;
                                    }
                                }
                            }
                            ClientEvent::AddPlayer { request_id } => {
                                if matches!(&world.state, GameState::Stopped(_)) {
                                    if let Some(_) = world.add_player() {
                                        sync_players(&bg_game_sender, &world.players);
                                        let player = player_dto(world.players.last().unwrap());
                                        let event = GameEvent::PlayerAdded { request_id, player };
                                        bg_game_sender.send_blocking(event).unwrap();
                                    }
                                }
                            }
                        }
                    }

                    world.update();
                }

                ctx.request_repaint();
                let update_time = start.elapsed();
                if update_time < UPDATE_TIME {
                    let remaining = UPDATE_TIME - update_time;
                    tracing::trace!("fast {}µs", update_time.as_micros());
                    std::thread::sleep(remaining);
                } else {
                    tracing::warn!("slow {}µs", update_time.as_micros());
                }
                start = Instant::now();
            }
        });

        let local_ip = local_ip_address::local_ip().unwrap();
        let local_url = format!("http://{local_ip}:8910");
        let qrcode = QrCode::new(&local_url).unwrap();
        Self {
            bg_thread: Some(bg_thread),
            game_sender,
            world,
            menu,
            world_to_screen_offset: Vec2::ZERO,
            world_to_screen_scale: 1.0,
            local_url,
            qrcode,
        }
    }
}

impl eframe::App for CurvefeverApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        ctx.input(|input| {
            let mut world = self.world.write().unwrap();
            let mut menu = self.menu.write().unwrap();
            match &mut menu.state {
                MenuState::Home => {
                    for p in world.players.iter_mut() {
                        let left_down = input.key_down(p.left_key);
                        let right_down = input.key_down(p.right_key);
                        p.local_direction = Direction::from_left_right_down(left_down, right_down);
                    }

                    if input.key_pressed(Key::Escape) {
                        world.toggle_pause();
                    } else if input.key_pressed(Key::Space) {
                        world.restart();
                    } else if input.key_pressed(Key::H) {
                        menu.state = MenuState::Help;
                    } else if input.key_pressed(Key::S) {
                        menu.state = MenuState::Share;

                    } else if input.key_pressed(Key::P) {
                        menu.state = MenuState::Player(PlayerMenu::default());
                    }
                }
                MenuState::Help => {
                    if input.key_pressed(Key::Escape) {
                        menu.state = MenuState::Home;
                    }
                }
                MenuState::Share => {
                    if input.key_pressed(Key::Escape) {
                        menu.state = MenuState::Home;
                    }
                }
                MenuState::Player(player_menu) => {
                    let mut invalidated = false;

                    if input.key_pressed(Key::Escape) {
                        if player_menu.selection_active {
                            player_menu.selection_active = false;
                        } else {
                            menu.state = MenuState::Home;
                        }
                    } else if input.key_pressed(Key::Space) || input.key_pressed(Key::Enter) {
                        player_menu.selection_active = !player_menu.selection_active;
                    } else if player_menu.selection_active {
                        match player_menu.field_index {
                            0 => {
                                for e in input.events.iter() {
                                    if let Event::Key {
                                        key,
                                        pressed: true,
                                        modifiers,
                                        ..
                                    } = e
                                    {
                                        match key {
                                            Key::ArrowLeft | Key::ArrowUp => {
                                                let idx = player_menu.player_index;
                                                world.players[idx].color.prev();
                                                invalidated = true;
                                            }
                                            Key::ArrowRight | Key::ArrowDown => {
                                                let idx = player_menu.player_index;
                                                world.players[idx].color.next();
                                                invalidated = true;
                                            }
                                            Key::Enter => {
                                                player_menu.selection_active =
                                                    !player_menu.selection_active;
                                            }
                                            Key::Backspace => {
                                                world.players[player_menu.player_index].name.pop();
                                                invalidated = true;
                                            }
                                            &k if (Key::A as u32..=Key::Z as u32)
                                                .contains(&(k as u32)) =>
                                            {
                                                let char_offset = k as u32 - Key::A as u32;
                                                let char = if modifiers.shift {
                                                    'A' as u32 + char_offset
                                                } else {
                                                    'a' as u32 + char_offset
                                                };
                                                let char = char::from_u32(char).unwrap();
                                                world.players[player_menu.player_index]
                                                    .name
                                                    .push(char);
                                                invalidated = true;
                                            }
                                            &k if (Key::Num0 as u32..=Key::Num9 as u32)
                                                .contains(&(k as u32)) =>
                                            {
                                                let char_offset = k as u32 - Key::Num0 as u32;
                                                let char = '0' as u32 + char_offset;
                                                let char = char::from_u32(char).unwrap();
                                                world.players[player_menu.player_index]
                                                    .name
                                                    .push(char);
                                                invalidated = true;
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                            }
                            1 => {
                                for e in input.events.iter() {
                                    if let Event::Key {
                                        key, pressed: true, ..
                                    } = e
                                    {
                                        world.players[player_menu.player_index].left_key = *key;
                                    }
                                }
                            }
                            2 => {
                                for e in input.events.iter() {
                                    if let Event::Key {
                                        key, pressed: true, ..
                                    } = e
                                    {
                                        world.players[player_menu.player_index].right_key = *key;
                                    }
                                }
                            }
                            _ => (),
                        }
                    } else {
                        if input.key_pressed(Key::Equals) {
                            world.add_player();
                            invalidated = true;
                        } else if input.key_pressed(Key::Minus) {
                            world.remove_player(player_menu.player_index);
                            if player_menu.player_index >= world.players.len() {
                                player_menu.player_index -= 1;
                            }
                            invalidated = true;
                        }

                        if input.key_pressed(Key::ArrowLeft) {
                            player_menu.selection_left();
                        } else if input.key_pressed(Key::ArrowRight) {
                            player_menu.selection_right();
                        } else if input.key_pressed(Key::ArrowUp) {
                            player_menu.selection_up(world.players.len());
                        } else if input.key_pressed(Key::ArrowDown) {
                            player_menu.selection_down(world.players.len());
                        }

                        if input.key_pressed(Key::H) {
                            player_menu.selection_left();
                        } else if input.key_pressed(Key::L) {
                            player_menu.selection_right();
                        } else if input.key_pressed(Key::K) {
                            player_menu.selection_up(world.players.len());
                        } else if input.key_pressed(Key::J) {
                            player_menu.selection_down(world.players.len());
                        }
                    }

                    if invalidated {
                        sync_players(&self.game_sender, &world.players);
                    }
                }
            }
        });

        CentralPanel::default()
            .frame(Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let painter = ui.painter();

                {
                    let screen_size = ui.available_size();
                    self.world_to_screen_scale = {
                        let scale_factors = screen_size / WORLD_SIZE;
                        scale_factors.min_elem()
                    };
                    self.world_to_screen_offset = {
                        let scaled_size = self.world_to_screen_scale * WORLD_SIZE;
                        0.5 * (screen_size - scaled_size)
                    };
                }

                self.rect_filled(
                    painter,
                    Rect::from_min_size(Pos2::ZERO, WORLD_SIZE),
                    Rounding::ZERO,
                    Color32::from_gray(24),
                );

                let world = self.world.read().unwrap();
                for i in world.items.iter() {
                    self.draw_item(painter, i);
                }
                for p in world.players.iter() {
                    self.draw_player(painter, p, &world);
                }
                if world.wall_teleporting() {
                    let rect = Rect::from_min_size(Pos2::ZERO, WORLD_SIZE);
                    let stroke = Stroke::new(2.0, Color32::from_rgb(0, 200, 0));
                    self.rect_stroke(painter, rect, Rounding::ZERO, stroke);
                }

                if matches!(world.state, GameState::Paused(_) | GameState::Stopped(_)) {
                    // menu background
                    let rect = Rect::from_min_size(Pos2::ZERO, WORLD_SIZE);
                    self.rect_filled(painter, rect, Rounding::ZERO, Color32::from_black_alpha(80));

                    let menu = self.menu.read().unwrap();
                    match &menu.state {
                        MenuState::Home => {
                            self.draw_normal_menu(painter, &world);
                        }
                        MenuState::Help => {
                            self.draw_help_menu(painter);
                        }
                        MenuState::Share => {
                            self.draw_join_menu(painter);
                        }
                        MenuState::Player(player_menu) => {
                            self.draw_player_menu(painter, &player_menu, &world);
                        }
                    }
                }

                self.draw_hud(painter, &world);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // tell the bg thread to stop
        {
            let mut world = self.world.write().unwrap();
            world.is_running = false;
        }

        // wait for it to stop
        let bg_thread = self.bg_thread.take();
        if let Some(t) = bg_thread {
            if let Err(e) = t.join() {
                tracing::error!("Error joining background thread: {e:?}");
            }
        }
    }
}

impl CurvefeverApp {
    fn draw_player(&self, painter: &Painter, player: &Player, world: &RwLockReadGuard<World>) {
        // draw trail
        let mut trail_points = Vec::new();
        let mut last_pos = player.trail.first().map_or(Pos2::ZERO, |s| s.start_pos());
        let mut thickness = player.trail.first().map_or(0.0, |s| s.thickness());
        let mut push_start = true;
        for s in player.trail.iter() {
            if s.gap() {
                let color = player.color.color32();
                self.draw_trail(painter, trail_points.clone(), thickness, color);
                trail_points.clear();

                push_start = true;
                last_pos = s.end_pos();
                continue;
            }

            if s.thickness() != thickness || s.start_pos() != last_pos {
                let color = player.color.color32();
                self.draw_trail(painter, trail_points.clone(), thickness, color);
                trail_points.clear();

                push_start = true;
            }

            match s {
                TrailSection::Straight(s) => {
                    if push_start {
                        trail_points.push(s.start);
                    }
                    trail_points.push(s.end);
                }
                TrailSection::Arc(s) => {
                    let angle_delta = match s.dir {
                        TurnDirection::Right => {
                            let angle_delta = if s.player_end_angle < s.player_start_angle {
                                s.player_end_angle.rem_euclid(TAU) - s.player_start_angle
                            } else {
                                s.player_end_angle - s.player_start_angle
                            };
                            angle_delta
                        }
                        TurnDirection::Left => {
                            let angle_delta = if s.player_start_angle < s.player_end_angle {
                                s.player_start_angle.rem_euclid(TAU) - s.player_end_angle
                            } else {
                                s.player_start_angle - s.player_end_angle
                            };
                            -angle_delta
                        }
                    };

                    let num_points = (angle_delta / (0.01 * TAU)).abs().round().max(1.0);
                    let angle_step = angle_delta / num_points;

                    trail_points.reserve(num_points as usize);
                    let center_pos = s.center_pos();
                    let arc_start_angle = s.arc_start_angle();
                    let iter_start = 1 - push_start as u8;
                    for i in iter_start..(num_points as u8) {
                        let arc_angle = arc_start_angle + i as f32 * angle_step;
                        let pos =
                            center_pos + s.radius * Vec2::new(arc_angle.cos(), arc_angle.sin());
                        trail_points.push(pos);
                    }
                    trail_points.push(s.end_pos());
                }
            }

            thickness = s.thickness();
            last_pos = s.end_pos();
        }
        if trail_points.len() > 1 {
            let color = player.color.color32();
            self.draw_trail(painter, trail_points, thickness, color);
        }

        // draw player dot
        if !player.crashed && (player.gap() || player.trail.is_empty()) {
            let a = if player.gap() { 80 } else { 255 };
            let color = player.color.color32().with_alpha(a);
            self.circle_filled(painter, player.pos, 0.5 * player.thickness(), color);
        }

        // draw arrow
        if let GameState::Starting(_) = world.state {
            let stroke = Stroke::new(0.3 * BASE_THICKNESS, Color32::from_gray(230));

            let start_distance = 10.0;
            let end_distance = 30.0;
            let arrow_distance = 5.0;
            let left_tip_angle = player.angle - 0.25 * PI;
            let right_tip_angle = player.angle + 0.25 * PI;

            let base_start = player.pos
                + Vec2::new(
                    player.angle.cos() * start_distance,
                    player.angle.sin() * start_distance,
                );
            let base_end = player.pos
                + Vec2::new(
                    player.angle.cos() * end_distance,
                    player.angle.sin() * end_distance,
                );

            let tip_left = base_end
                - Vec2::new(
                    left_tip_angle.cos() * arrow_distance,
                    left_tip_angle.sin() * arrow_distance,
                );
            let tip_right = base_end
                - Vec2::new(
                    right_tip_angle.cos() * arrow_distance,
                    right_tip_angle.sin() * arrow_distance,
                );

            self.line_segment(painter, [tip_left, base_end], stroke);
            self.line_segment(painter, [tip_right, base_end], stroke);
            self.line_segment(painter, [base_start, base_end], stroke);
        }
    }

    fn draw_trail(
        &self,
        painter: &Painter,
        trail_points: Vec<Pos2>,
        thickness: f32,
        color: Color32,
    ) {
        if trail_points.len() < 2 {
            return;
        }

        let stroke = Stroke::new(thickness, color);

        let first = *trail_points.first().unwrap();
        self.circle_filled(painter, first, 0.5 * thickness - 0.1, color);

        let last = *trail_points.last().unwrap();
        self.circle_filled(painter, last, 0.5 * thickness - 0.1, color);

        let path = PathShape::line(trail_points, stroke);
        self.add_path(painter, path);
    }

    fn draw_item(&self, painter: &Painter, item: &Item) {
        self.circle_filled(painter, item.pos, ITEM_RADIUS, item.kind.color32());
    }

    fn draw_normal_menu(&self, painter: &Painter, world: &RwLockReadGuard<World>) {
        if let GameState::Stopped(_) = world.state {
            const FONT: FontId = FontId::new(20.0, FontFamily::Proportional);
            const BG_RECT_EXPAND: Vec2 = Vec2::new(6.0, 4.0);
            let bg_rounding = Rounding::same(6.0);
            const V_OFFSET: f32 = 40.0;
            const H_OFFSET: f32 = 15.0;
            let text_color = Color32::from_gray(200);
            let key_bg_color = Color32::from_gray(48).with_alpha(160);
            let center_pos = (0.5 * WORLD_SIZE).to_pos2();

            let outline_rect_idx = painter.add(Shape::Noop);
            let text_rect = self.text(
                painter,
                center_pos + Vec2::new(-H_OFFSET, -V_OFFSET),
                Align2::RIGHT_CENTER,
                "SPACE",
                FONT,
                text_color,
            );
            self.set_rect(
                painter,
                outline_rect_idx,
                text_rect.expand2(BG_RECT_EXPAND),
                bg_rounding,
                key_bg_color,
                Stroke::NONE,
            );
            self.text(
                painter,
                center_pos + Vec2::new(H_OFFSET, -V_OFFSET),
                Align2::LEFT_CENTER,
                "to restart",
                FONT,
                text_color,
            );

            let hints = [
                ("H", "for help"),
                ("S", "to share"),
                ("P", "to manage players"),
            ];

            for (i, (key, desc)) in hints.iter().enumerate() {
                let outline_rect_idx = painter.add(Shape::Noop);
                let text_rect = self.text(
                    painter,
                    center_pos + Vec2::new(-H_OFFSET, i as f32 * V_OFFSET),
                    Align2::RIGHT_CENTER,
                    key,
                    FONT,
                    text_color,
                );
                self.set_rect(
                    painter,
                    outline_rect_idx,
                    text_rect.expand2(BG_RECT_EXPAND),
                    bg_rounding,
                    key_bg_color,
                    Stroke::NONE,
                );
                self.text(
                    painter,
                    center_pos + Vec2::new(H_OFFSET, i as f32 * V_OFFSET),
                    Align2::LEFT_CENTER,
                    desc,
                    FONT,
                    text_color,
                );
            }
        }
    }

    fn draw_help_menu(&self, painter: &Painter) {
        const FIELD_HEIGHT: f32 = WORLD_SIZE.y / (ITEM_KINDS.len() + 1) as f32;
        for (i, item) in ITEM_KINDS.iter().enumerate() {
            let pos = Pos2::new(0.5 * WORLD_SIZE.x, (i + 1) as f32 * FIELD_HEIGHT);
            self.circle_filled(
                painter,
                pos - Vec2::new(40.0, 0.0),
                ITEM_RADIUS,
                item.color32(),
            );
            let font = FontId::new(20.0, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::LEFT_CENTER,
                item.name(),
                font,
                Color32::from_gray(200),
            );
        }
    }

    fn draw_join_menu(&self, painter: &Painter) {
        let center: Pos2 = (WORLD_SIZE / 2.0).to_pos2();
        let qrcode_size: Vec2 = Vec2::splat(WORLD_SIZE.min_elem() / 2.0);
        let qrcode_pos: Pos2 = center - qrcode_size / 2.0;
        let text_size: f32 = qrcode_size.y / 12.0;
        let url_pos: Pos2 = center + Vec2::new(0.0, -qrcode_size.y / 2.0 - text_size);
        let font = FontId::new(text_size, FontFamily::Proportional);

        // url
        {
            const BG_RECT_EXPAND: Vec2 = Vec2::new(12.0, 8.0);
            let bg_rounding = Rounding::same(6.0);
            let bg_color = Color32::from_rgb(0x30, 0x40, 0x80).with_alpha(160);
            let outline_rect_idx = painter.add(Shape::Noop);
            let text_rect = self.text(
                painter,
                url_pos,
                Align2::CENTER_BOTTOM,
                &self.local_url,
                font,
                Color32::from_rgb(0x50, 0x80, 0xff).with_alpha(160),
            );
            self.set_rect(
                painter,
                outline_rect_idx,
                text_rect.expand2(BG_RECT_EXPAND),
                bg_rounding,
                bg_color,
                Stroke::NONE,
            );
        }

        // qr code
        const CODE_PADDING: usize = 3;
        let num_cells = self.qrcode.width();
        let cell_size = qrcode_size / (num_cells + 2 * CODE_PADDING) as f32;

        let rect = Rect::from_min_size(qrcode_pos, qrcode_size);
        self.rect_filled(
            painter,
            rect,
            Rounding::same(2.0 * cell_size.y),
            Color32::WHITE,
        );

        for y in 0..num_cells {
            for x in 0..num_cells {
                let color = self.qrcode[(x, y)];
                if color != qrcode::Color::Dark {
                    continue;
                }
                let offset = Vec2::new((CODE_PADDING + x) as f32, (CODE_PADDING + y) as f32);
                let min = qrcode_pos + cell_size * offset;
                let rect = Rect::from_min_size(min, cell_size);
                self.pixel_perfect_rect_filled(painter, rect, Rounding::ZERO, Color32::BLACK);
            }
        }
    }

    fn draw_player_menu(
        &self,
        painter: &Painter,
        player_menu: &PlayerMenu,
        world: &RwLockReadGuard<World>,
    ) {
        const FIELD_SIZE: Vec2 = Vec2::new(
            WORLD_SIZE.x / 6.0,
            WORLD_SIZE.y / (PLAYER_COLORS.len() + 1) as f32,
        );

        for (index, player) in world.players.iter().enumerate() {
            //name
            let pos = Pos2::new(
                0.5 * WORLD_SIZE.x - FIELD_SIZE.x,
                (index as f32 + 1.0) * FIELD_SIZE.y,
            );
            let font = FontId::new(0.5 * FIELD_SIZE.y, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::CENTER_CENTER,
                &player.name,
                font,
                player.color.color32(),
            );

            //left key
            let pos = Pos2::new(
                0.5 * WORLD_SIZE.x + 0.5 * FIELD_SIZE.x,
                (index as f32 + 1.0) * FIELD_SIZE.y,
            );
            let font = FontId::new(0.5 * FIELD_SIZE.y, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::CENTER_CENTER,
                player.left_key.name(),
                font,
                Color32::from_gray(200),
            );

            //right key
            let pos = Pos2::new(
                0.5 * WORLD_SIZE.x + 1.5 * FIELD_SIZE.x,
                (index as f32 + 1.0) * FIELD_SIZE.y,
            );
            let font = FontId::new(0.5 * FIELD_SIZE.y, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::CENTER_CENTER,
                player.right_key.name(),
                font,
                Color32::from_gray(200),
            );
        }

        //selection
        let color = if player_menu.selection_active {
            Color32::from_gray(200)
        } else {
            Color32::from_gray(100)
        };

        let mut selection_size = FIELD_SIZE;
        if player_menu.field_index == 0 {
            selection_size.x *= 2.0;
        }

        let x = if player_menu.field_index == 0 {
            0.5 * WORLD_SIZE.x - 2.0 * FIELD_SIZE.x
        } else {
            0.5 * WORLD_SIZE.x + (player_menu.field_index as f32 - 1.0) * FIELD_SIZE.x
        };
        let y = (player_menu.player_index as f32 + 0.5) * FIELD_SIZE.y;
        let rect = Rect::from_min_size(Pos2::new(x, y), selection_size);
        let stroke = Stroke::new(4.0, color);
        self.rect_stroke(painter, rect, Rounding::same(0.1 * FIELD_SIZE.y), stroke);
    }

    fn draw_hud(&self, painter: &Painter, world: &RwLockReadGuard<World>) {
        const HUD_FONT: FontId = FontId::new(14.0, FontFamily::Proportional);
        const HUD_ALPHA: u8 = 160;
        let hud_rounding = Rounding::same(8.0);
        let hud_text_color = Color32::from_gray(160).with_alpha(HUD_ALPHA);
        let hud_effect_bar_color = Color32::from_gray(100).with_alpha(HUD_ALPHA);
        let hud_bg_color = Color32::from_gray(48).with_alpha(HUD_ALPHA);
        let text_offset = Vec2::new(5.0, 0.0);

        for (index, p) in world.players.iter().enumerate() {
            // player name and score
            let outline_rect_idx = painter.add(Shape::Noop);

            let text_pos = Pos2::new(20.0, 20.0 + index as f32 * 30.0);
            let text_rect = self.text(
                painter,
                text_pos,
                Align2::LEFT_TOP,
                &p.name,
                HUD_FONT,
                p.color.color32(),
            );
            let min = text_pos;

            let text_pos = text_rect.right_top() + text_offset;
            let text_rect = self.text(
                painter,
                text_pos,
                Align2::LEFT_TOP,
                p.score,
                HUD_FONT,
                hud_text_color,
            );
            let mut max = text_rect.right_bottom();

            // player effects
            let mut effect_pos = text_rect.right_center() + Vec2::new(20.0, 0.0);
            for e in p.effects.iter() {
                let Some(item_kind) = e.kind.item_kind() else {
                    continue;
                };

                let now = world.clock.now;
                let passed_duration = now.duration_since(e.start).unwrap();
                let ratio = passed_duration.as_secs_f32() / e.duration.as_secs_f32();

                // effect arc
                {
                    let ratio = 1.0 - ratio;
                    let num_points = ((20.0 * ratio).round() as u8).max(2);
                    let angle_step = (ratio * TAU) / (num_points - 1) as f32;
                    let mut points = Vec::new();
                    let mut angle: f32 = 0.0;
                    for _ in 0..num_points {
                        let pos = effect_pos + 6.0 * Vec2::new(angle.cos(), angle.sin());
                        points.push(pos);
                        angle += angle_step;
                    }
                    let color = item_kind.color32();
                    let stroke = Stroke::new(3.0, color.with_alpha(HUD_ALPHA));
                    let path = PathShape::line(points, stroke);
                    self.add_path(painter, path);
                }

                // grey arc
                {
                    let num_points = ((20.0 * ratio).round() as u8).max(2);
                    let angle_step = (ratio * TAU) / (num_points - 1) as f32;
                    let mut points = Vec::new();
                    let mut angle: f32 = 0.0;
                    for _ in 0..num_points {
                        let pos = effect_pos + 6.0 * Vec2::new(angle.cos(), angle.sin());
                        points.push(pos);
                        angle -= angle_step;
                    }
                    let stroke = Stroke::new(3.0, hud_effect_bar_color);
                    let path = PathShape::line(points, stroke);
                    self.add_path(painter, path);
                }

                // update next pos
                max.x = effect_pos.x + 10.0;
                effect_pos.x += 20.0;
            }

            let outline_rect = Rect::from_min_max(min, max);
            self.set_rect(
                painter,
                outline_rect_idx,
                outline_rect.expand(4.0),
                hud_rounding,
                hud_bg_color,
                Stroke::NONE,
            );
        }

        // crash feed
        let mut text_pos = Pos2::new(WORLD_SIZE.x - 20.0, 20.0);
        for c in world.crash_feed.iter() {
            match world.state {
                GameState::Starting(_) | GameState::Running(_) => {
                    const CRASH_DISPLAY_DURATION: Duration = Duration::from_secs(5);
                    let passed_duration = world.clock.now.duration_since(c.time).unwrap();
                    if passed_duration > CRASH_DISPLAY_DURATION {
                        continue;
                    }
                }
                GameState::Paused(_) | GameState::Stopped(_) => (),
            }

            let outline_rect_idx = painter.add(Shape::Noop);
            let outline_rect = match &c.message {
                CrashMessage::Own { name, color } => {
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        "crashed into themself",
                        HUD_FONT,
                        hud_text_color,
                    );
                    let max = text_rect.right_bottom();

                    let text_pos = text_rect.left_top() - text_offset;
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        name,
                        HUD_FONT,
                        color.with_alpha(HUD_ALPHA),
                    );
                    let min = text_rect.left_top();
                    Rect::from_min_max(min, max)
                }
                CrashMessage::Wall { name, color } => {
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        "crashed into the wall",
                        HUD_FONT,
                        hud_text_color,
                    );
                    let max = text_rect.right_bottom();

                    let text_pos = text_rect.left_top() - text_offset;
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        name,
                        HUD_FONT,
                        color.with_alpha(HUD_ALPHA),
                    );
                    let min = text_rect.left_top();
                    Rect::from_min_max(min, max)
                }
                CrashMessage::Other {
                    crashed_name,
                    crashed_color,
                    other_name,
                    other_color,
                } => {
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        other_name,
                        HUD_FONT,
                        other_color.with_alpha(HUD_ALPHA),
                    );
                    let max = text_rect.right_bottom();

                    let text_pos = text_rect.left_top() - text_offset;
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        "crashed into",
                        HUD_FONT,
                        hud_text_color,
                    );

                    let text_pos = text_rect.left_top() - text_offset;
                    let text_rect = self.text(
                        painter,
                        text_pos,
                        Align2::RIGHT_TOP,
                        crashed_name,
                        HUD_FONT,
                        crashed_color.with_alpha(HUD_ALPHA),
                    );
                    let min = text_rect.left_top();
                    Rect::from_min_max(min, max)
                }
            };

            self.set_rect(
                painter,
                outline_rect_idx,
                outline_rect.expand(4.0),
                hud_rounding,
                hud_bg_color,
                Stroke::NONE,
            );

            text_pos.y += 30.0;
        }

        // countdown and time
        let time_pos = Pos2::new(0.5 * WORLD_SIZE.x, 20.0);
        let countdown_pos = (0.5 * WORLD_SIZE).to_pos2();
        let pos_anim_frac = painter.ctx().animate_bool_with_time(
            Id::new("countdown_time"),
            !matches!(world.state, GameState::Starting(_)),
            0.2,
        );
        let pos = countdown_pos.lerp(time_pos, pos_anim_frac);
        let bg_alpha = (pos_anim_frac * HUD_ALPHA as f32).round() as u8;

        match world.state {
            GameState::Starting(start) => {
                let time = world.clock.now.duration_since(start).unwrap().as_secs();
                let text = START_DELAY.as_secs() - time;
                let font = FontId::new(80.0, FontFamily::Monospace);
                self.text(
                    painter,
                    pos,
                    Align2::CENTER_CENTER,
                    text,
                    font,
                    hud_text_color,
                );
            }
            GameState::Running(start) | GameState::Paused(start) | GameState::Stopped(start) => {
                let outline_rect_idx = painter.add(Shape::Noop);

                let font = FontId::new(20.0, FontFamily::Monospace);
                let duration = world.clock.now.duration_since(start).unwrap();
                let total_secs = duration.as_secs();
                let minutes = total_secs / 60;
                let secs = total_secs % 60;
                let text = format!("{minutes:02}:{secs:02}");
                let text_rect =
                    self.text(painter, pos, Align2::CENTER_TOP, text, font, hud_text_color);

                self.set_rect(
                    painter,
                    outline_rect_idx,
                    text_rect.expand2(Vec2::new(6.0, 4.0)),
                    hud_rounding,
                    Color32::from_gray(40).with_alpha(bg_alpha),
                    Stroke::NONE,
                );
            }
        }
    }

    fn text(
        &self,
        painter: &Painter,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        mut font_id: FontId,
        text_color: Color32,
    ) -> Rect {
        font_id.size *= self.world_to_screen_scale;
        let rect = painter.text(self.wts_pos(pos), anchor, text, font_id, text_color);
        self.stw_rect(rect)
    }

    fn circle_filled(&self, painter: &Painter, pos: Pos2, mut radius: f32, fill_color: Color32) {
        radius *= self.world_to_screen_scale;
        painter.circle_filled(self.wts_pos(pos), radius, fill_color);
    }

    fn line_segment(&self, painter: &Painter, points: [Pos2; 2], mut stroke: Stroke) {
        let points = [self.wts_pos(points[0]), self.wts_pos(points[1])];
        stroke.width *= self.world_to_screen_scale;
        painter.line_segment(points, stroke);
    }

    fn rect_stroke(&self, painter: &Painter, rect: Rect, rounding: Rounding, mut stroke: Stroke) {
        stroke.width *= self.world_to_screen_scale;
        painter.rect_stroke(self.wts_rect(rect), self.wts_rounding(rounding), stroke);
    }

    fn rect_filled(&self, painter: &Painter, rect: Rect, rounding: Rounding, fill_color: Color32) {
        painter.rect_filled(self.wts_rect(rect), self.wts_rounding(rounding), fill_color);
    }

    fn pixel_perfect_rect_filled(
        &self,
        painter: &Painter,
        rect: Rect,
        rounding: Rounding,
        fill_color: Color32,
    ) {
        let rect = self.wts_rect(rect);
        let pixel_perfect_rect = Rect {
            min: painter.round_pos_to_pixels(rect.min),
            max: painter.round_pos_to_pixels(rect.max),
        };
        painter.rect_filled(pixel_perfect_rect, self.wts_rounding(rounding), fill_color);
    }

    fn add_path(&self, painter: &Painter, mut path: PathShape) {
        for p in path.points.iter_mut() {
            *p = self.wts_pos(*p);
        }
        path.stroke.width *= self.world_to_screen_scale;
        painter.add(Shape::Path(path));
    }

    fn set_rect(
        &self,
        painter: &Painter,
        idx: ShapeIdx,
        rect: Rect,
        rounding: Rounding,
        fill_color: Color32,
        mut stroke: Stroke,
    ) {
        stroke.width *= self.world_to_screen_scale;
        let shape = RectShape::new(
            self.wts_rect(rect),
            self.wts_rounding(rounding),
            fill_color,
            stroke,
        );
        painter.set(idx, Shape::Rect(shape));
    }
}

trait ColorExt {
    fn with_alpha(&self, a: u8) -> Color32;
}

impl ColorExt for Color32 {
    fn with_alpha(&self, a: u8) -> Color32 {
        let (r, g, b, _) = self.to_tuple();
        Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}

fn find_player(players: &mut [Player], player_id: u16) -> Option<&mut Player> {
    for p in players.iter_mut() {
        if p.id == player_id {
            return Some(p);
        }
    }

    None
}

fn sync_players(game_sender: &Sender<GameEvent>, players: &[Player]) {
    let players = players.iter().map(player_dto).collect();
    let event = GameEvent::PlayerSync { players };
    game_sender.send_blocking(event).unwrap();
}

fn player_dto(player: &Player) -> curvefever_common::Player {
    curvefever_common::Player {
        id: player.id,
        color: player.color.color32().to_array(),
        name: player.name.clone(),
    }
}
