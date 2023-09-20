use std::f32::consts::{PI, TAU};
use std::time::Duration;

use eframe::CreationContext;
use egui::epaint::{PathShape, RectShape};
use egui::layers::ShapeIdx;
use egui::{
    Align2, CentralPanel, Color32, Context, Event, FontFamily, FontId, Frame, Key, Painter, Pos2,
    Rect, Rounding, Shape, Stroke, Vec2,
};

use crate::world::{
    CrashMessage, GameState, Item, Player, TrailSection, TurnDirection, World, BASE_THICKNESS,
    ITEM_RADIUS, START_DELAY, WORLD_SIZE,
};

pub const PLAYER_MENU_FIELDS: usize = 3;

pub struct CurvefeverApp {
    world: World,
    menu: Menu,
    world_to_screen_offset: Vec2,
    world_to_screen_scale: f32,
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
    fn stw_pos(&self, pos: Pos2) -> Pos2 {
        let pos = pos - self.world_to_screen_offset;
        Pos2::new(
            pos.x / self.world_to_screen_scale,
            pos.y / self.world_to_screen_scale,
        )
    }

    #[inline(always)]
    fn wts_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.wts_pos(rect.min), self.wts_pos(rect.max))
    }

    #[inline(always)]
    fn stw_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.stw_pos(rect.min), self.stw_pos(rect.max))
    }
}

struct Menu {
    state: MenuState,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            state: MenuState::Normal,
        }
    }
}

enum MenuState {
    Normal,
    Player(PlayerMenu),
}

#[derive(Debug, Default)]
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
    pub fn new(_: &CreationContext) -> Self {
        Self {
            world: World::new(),
            menu: Menu::new(),
            world_to_screen_offset: Vec2::ZERO,
            world_to_screen_scale: 1.0,
        }
    }
}

impl eframe::App for CurvefeverApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        ctx.request_repaint();

        self.world.update();

        ctx.input(|input| match &mut self.menu.state {
            MenuState::Player(player_menu) if self.world.state == GameState::Stopped => {
                if input.key_pressed(Key::Escape) {
                    self.menu.state = MenuState::Normal;
                } else if input.key_pressed(Key::Space) {
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
                                            self.world.players[player_menu.player_index]
                                                .color
                                                .prev();
                                        }
                                        Key::ArrowRight | Key::ArrowDown => {
                                            self.world.players[player_menu.player_index]
                                                .color
                                                .next();
                                        }
                                        Key::Enter => {
                                            player_menu.selection_active =
                                                !player_menu.selection_active;
                                        }
                                        Key::Backspace => {
                                            self.world.players[player_menu.player_index].name.pop();
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
                                            self.world.players[player_menu.player_index]
                                                .name
                                                .push(char);
                                        }
                                        &k if (Key::Num0 as u32..=Key::Num9 as u32)
                                            .contains(&(k as u32)) =>
                                        {
                                            let char_offset = k as u32 - Key::Num0 as u32;
                                            let char = '0' as u32 + char_offset;
                                            let char = char::from_u32(char).unwrap();
                                            self.world.players[player_menu.player_index]
                                                .name
                                                .push(char);
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
                                    self.world.players[player_menu.player_index].left_key = *key;
                                }
                            }
                        }
                        2 => {
                            for e in input.events.iter() {
                                if let Event::Key {
                                    key, pressed: true, ..
                                } = e
                                {
                                    self.world.players[player_menu.player_index].right_key = *key;
                                }
                            }
                        }
                        _ => (),
                    }
                } else {
                    if input.key_pressed(Key::PlusEquals) {
                        self.world.add_player();
                    } else if input.key_pressed(Key::Minus) {
                        self.world.remove_player(player_menu.player_index as usize);
                        if player_menu.player_index as usize >= self.world.players.len() {
                            player_menu.player_index -= 1;
                        }
                    }

                    if input.key_pressed(Key::ArrowLeft) {
                        player_menu.selection_left();
                    } else if input.key_pressed(Key::ArrowRight) {
                        player_menu.selection_right();
                    } else if input.key_pressed(Key::ArrowUp) {
                        player_menu.selection_up(self.world.players.len());
                    } else if input.key_pressed(Key::ArrowDown) {
                        player_menu.selection_down(self.world.players.len());
                    }
                }
            }
            _ => {
                for p in self.world.players.iter_mut() {
                    p.left_down = input.key_down(p.left_key);
                    p.right_down = input.key_down(p.right_key);
                }

                if input.key_pressed(Key::Escape) {
                    self.world.toggle_pause();
                } else if input.key_pressed(Key::Space) {
                    self.world.restart();
                } else if input.key_pressed(Key::P) {
                    self.menu.state = MenuState::Player(PlayerMenu::default());
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
                    Rounding::none(),
                    Color32::from_gray(50),
                );

                for i in self.world.items.iter() {
                    self.draw_item(painter, i);
                }
                for p in self.world.players.iter() {
                    self.draw_player(painter, p);
                }
                if self.world.wall_teleporting() {
                    let rect = Rect::from_min_size(Pos2::ZERO, WORLD_SIZE);
                    let stroke = Stroke::new(2.0, Color32::from_rgb(0, 200, 0));
                    self.rect_stroke(painter, rect, Rounding::none(), stroke);
                }

                if matches!(self.world.state, GameState::Paused | GameState::Stopped) {
                    match &self.menu.state {
                        MenuState::Player(player_menu)
                            if self.world.state == GameState::Stopped =>
                        {
                            self.draw_player_menu(painter, player_menu);
                        }
                        MenuState::Player(_) => {
                            self.menu.state = MenuState::Normal;
                        }
                        MenuState::Normal => {
                            self.draw_normal_menu(painter);
                        }
                    }
                }

                self.draw_hud(painter);
            });
    }
}

impl CurvefeverApp {
    fn draw_player(&self, painter: &Painter, player: &Player) {
        // draw trail
        let mut trail_iter = player.trail.iter().peekable();
        let mut trail_points = Vec::new();
        let mut last_pos = trail_iter.peek().map_or(Pos2::ZERO, |s| s.start_pos());
        let mut thickness = trail_iter.peek().map_or(0.0, |s| s.thickness());
        let mut push_start = true;
        while let Some(s) = trail_iter.next() {
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
            let a = if player.gap() { 120 } else { 255 };
            let color = player.color.color32().with_alpha(a);
            self.circle_filled(painter, player.pos, 0.5 * player.thickness(), color);
        }

        // draw arrow
        if let GameState::Starting(_) = &self.world.state {
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

            self.line_segment(painter, [base_start, base_end], stroke);
            self.line_segment(painter, [tip_left, base_end], stroke);
            self.line_segment(painter, [tip_right, base_end], stroke);
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

    fn draw_normal_menu(&self, painter: &Painter) {
        let rect = Rect::from_min_size(Pos2::ZERO, WORLD_SIZE);
        self.rect_filled(
            painter,
            rect,
            Rounding::none(),
            Color32::from_black_alpha(100),
        );

        if self.world.state == GameState::Stopped {
            let text = "SPACE : restart\nP : manage players\n";
            let pos = (0.5 * WORLD_SIZE).to_pos2();
            let font = FontId::new(20.0, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::CENTER_CENTER,
                text,
                font,
                Color32::from_gray(200),
            );
        }
    }

    fn draw_player_menu(&self, painter: &Painter, player_menu: &PlayerMenu) {
        const FIELD_SIZE: Vec2 = Vec2::new(WORLD_SIZE.x / 6.0, WORLD_SIZE.y / 9.0);

        for (index, player) in self.world.players.iter().enumerate() {
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
        self.rect_stroke(painter, rect, Rounding::same(0.2 * FIELD_SIZE.y), stroke);
    }

    fn draw_hud(&self, painter: &Painter) {
        for (index, p) in self.world.players.iter().enumerate() {
            // player name and score
            let text = format!("{} : {}", p.name, p.score);
            let pos = Pos2::new(10.0, 10.0 + index as f32 * 20.0);
            let font = FontId::new(14.0, FontFamily::Proportional);
            let text_rect = self.text(
                painter,
                pos,
                Align2::LEFT_TOP,
                text,
                font,
                p.color.color32(),
            );

            // player effects
            let mut effect_pos = text_rect.right_center() + Vec2::new(20.0, 0.0);
            for e in p.effects.iter() {
                let Some(item_kind) = e.kind.item_kind() else {
                    continue;
                };

                let now = self.world.clock.now;
                let passed_duration = now.duration_since(e.start).unwrap();
                let ratio = 1.0 - passed_duration.as_secs_f32() / e.duration.as_secs_f32();
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
                let stroke = Stroke::new(3.0, color);
                let path = PathShape::line(points, stroke);
                self.add_path(painter, path);

                effect_pos.x += 20.0;
            }
        }

        // crash feed
        const FEED_ALPHA: u8 = 80;
        const FEED_TEXT_COLOR: Color32 =
            Color32::from_rgba_premultiplied(160, 160, 160, FEED_ALPHA);
        const FEED_OUTLINE_COLOR: Color32 =
            Color32::from_rgba_premultiplied(100, 100, 100, FEED_ALPHA);
        const FEED_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 4);
        const MESSAGE_OFFSET: Vec2 = Vec2::new(5.0, 0.0);
        let mut message_pos = Pos2::new(WORLD_SIZE.x - 10.0, 10.0);
        for c in self.world.crash_feed.iter() {
            match self.world.state {
                GameState::Starting(_) | GameState::Running => {
                    const CRASH_DISPLAY_DURATION: Duration = Duration::from_secs(5);
                    let passed_duration = self.world.clock.now.duration_since(c.time).unwrap();
                    if passed_duration > CRASH_DISPLAY_DURATION {
                        continue;
                    }
                }
                GameState::Paused | GameState::Stopped => (),
            }

            let font = FontId::new(14.0, FontFamily::Proportional);
            let outline_rect_idx = painter.add(Shape::Noop);
            let outline_rect = match &c.message {
                CrashMessage::Own { name, color } => {
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        "crashed into themselves",
                        font.clone(),
                        FEED_TEXT_COLOR,
                    );
                    let max = text_rect.right_bottom();

                    let message_pos = text_rect.left_top() - MESSAGE_OFFSET;
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        name,
                        font,
                        color.with_alpha(FEED_ALPHA),
                    );
                    let min = text_rect.left_top();
                    Rect::from_min_max(min, max)
                }
                CrashMessage::Wall { name, color } => {
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        "crashed into the wall",
                        font.clone(),
                        FEED_TEXT_COLOR,
                    );
                    let max = text_rect.right_bottom();

                    let message_pos = text_rect.left_top() - MESSAGE_OFFSET;
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        name,
                        font,
                        color.with_alpha(FEED_ALPHA),
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
                        message_pos,
                        Align2::RIGHT_TOP,
                        other_name,
                        font.clone(),
                        other_color.with_alpha(FEED_ALPHA),
                    );
                    let max = text_rect.right_bottom();

                    let message_pos = text_rect.left_top() - MESSAGE_OFFSET;
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        "crashed into",
                        font.clone(),
                        FEED_TEXT_COLOR,
                    );

                    let message_pos = text_rect.left_top() - MESSAGE_OFFSET;
                    let text_rect = self.text(
                        painter,
                        message_pos,
                        Align2::RIGHT_TOP,
                        crashed_name,
                        font,
                        crashed_color.with_alpha(FEED_ALPHA),
                    );
                    let min = text_rect.left_top();
                    Rect::from_min_max(min, max)
                }
            };

            let stroke = Stroke::new(2.0, FEED_OUTLINE_COLOR);
            self.set_rect(
                painter,
                outline_rect_idx,
                outline_rect.expand(4.0),
                Rounding::same(4.0),
                FEED_BG_COLOR,
                stroke,
            );

            message_pos.y += 30.0;
        }

        // countdown
        if let GameState::Starting(start) = self.world.state {
            let time = self
                .world
                .clock
                .now
                .duration_since(start)
                .unwrap()
                .as_secs();
            let text = START_DELAY.as_secs() - time;
            let pos = (0.5 * WORLD_SIZE).to_pos2();
            let font = FontId::new(30.0, FontFamily::Proportional);
            self.text(
                painter,
                pos,
                Align2::CENTER_CENTER,
                text,
                font,
                Color32::from_gray(230),
            );
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

    fn rect_stroke(
        &self,
        painter: &Painter,
        rect: Rect,
        mut rounding: Rounding,
        mut stroke: Stroke,
    ) {
        rounding.nw *= self.world_to_screen_scale;
        rounding.ne *= self.world_to_screen_scale;
        rounding.sw *= self.world_to_screen_scale;
        rounding.se *= self.world_to_screen_scale;
        stroke.width *= self.world_to_screen_scale;
        painter.rect_stroke(self.wts_rect(rect), rounding, stroke);
    }

    fn rect_filled(
        &self,
        painter: &Painter,
        rect: Rect,
        mut rounding: Rounding,
        fill_color: Color32,
    ) {
        rounding.nw *= self.world_to_screen_scale;
        rounding.ne *= self.world_to_screen_scale;
        rounding.sw *= self.world_to_screen_scale;
        rounding.se *= self.world_to_screen_scale;
        painter.rect_filled(self.wts_rect(rect), rounding, fill_color);
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
        stroke: Stroke,
    ) {
        let shape = RectShape {
            rect: self.wts_rect(rect),
            rounding,
            fill: fill_color,
            stroke,
        };
        painter.set(idx, Shape::Rect(shape));
    }
}

trait ColorExt {
    fn with_alpha(&self, a: u8) -> Color32;
}

impl ColorExt for Color32 {
    fn with_alpha(&self, a: u8) -> Color32 {
        let (r, g, b, _) = self.to_tuple();
        Color32::from_rgba_premultiplied(r, g, b, a)
    }
}
