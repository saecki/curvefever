use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::time::{Duration, SystemTime};

use curvefever_common::Direction;
use egui::{Color32, Key, Pos2, Vec2};
use rand::Rng;

use curvefever_derive::EnumMembersArray;

pub const UPDATE_TIME: Duration = Duration::from_nanos(1_000_000_000 / 240);

pub const WORLD_SIZE: Vec2 = Vec2::new(1280.0, 720.0);
pub const MIN_PLAYER_WALL_DIST: f32 = 150.0;
pub const MIN_PLAYER_DIST: f32 = 200.0;
pub const MIN_ITEM_WALL_DIST: f32 = 40.0;
pub const MIN_ITEM_DIST: f32 = 80.0;
pub const ITEM_SPAWN_RATE: f32 = 0.48;
pub const ITEM_RADIUS: f32 = 7.5;
pub const START_DELAY: Duration = Duration::from_secs(2);
pub const MAX_ITEMS: usize = 8;
pub const GAP_RATE: f32 = 0.4;

pub const PLAYER_EFFECT_DURATION: Duration = Duration::from_secs(5);
pub const PLAYER_EFFECT_DEVIATION_DURATION: Duration = Duration::from_secs(1);
pub const GAP_EFFECT_DURATION: Duration = Duration::from_millis(150);
pub const GAP_EFFECT_DEVIATION_DURATION: Duration = Duration::from_millis(100);
pub const WORLD_EFFECT_DURATION: Duration = Duration::from_secs(10);
pub const WORLD_EFFECT_DEVIATION_DURATION: Duration = Duration::from_secs(3);
pub const BASE_SPEED: f32 = 150.0;
pub const MIN_SPEED: f32 = 50.0;
pub const BASE_THICKNESS: f32 = 4.0;
pub const MIN_THICKNESS: f32 = 1.0;
pub const BASE_TURNING_RADIUS: f32 = 50.0;
pub const MIN_TURNING_RADIUS: f32 = 25.0;

pub const ITEM_KINDS: &[ItemKind] = ItemKind::members();
pub const SUM_OF_ITEM_SPAWN_RATES: u8 = {
    let mut i = 0;
    let mut sum = 0;
    while i < ITEM_KINDS.len() {
        sum += ITEM_KINDS[i].spawn_rate();
        i += 1;
    }
    sum
};
pub const PLAYER_COLORS: &[PlayerColor] = PlayerColor::members();

pub struct World {
    next_id: u16,
    pub is_running: bool,
    pub clock: Clock,
    pub state: GameState,
    pub items: Vec<Item>,
    pub effects: Vec<Effect<WorldEffect>>,
    pub players: Vec<Player>,
    pub crash_feed: Vec<Crash>,
}

impl World {
    pub fn new() -> Self {
        let mut next_id = 0;
        let mut players = Vec::with_capacity(2);
        let player1 = random_player(
            next_id,
            "Player1".to_string(),
            Key::ArrowLeft,
            Key::ArrowRight,
            &players,
        );
        players.push(player1);
        next_id += 1;

        let player2 = random_player(next_id, "Player2".to_string(), Key::A, Key::D, &players);
        players.push(player2);
        next_id += 1;

        let clock = Clock::new();
        let now = clock.now;
        Self {
            next_id,
            is_running: true,
            clock,
            state: GameState::Stopped(now),
            items: Vec::new(),
            effects: Vec::new(),
            crash_feed: Vec::new(),
            players,
        }
    }

    pub fn wall_teleporting(&self) -> bool {
        self.effects
            .iter()
            .any(|e| e.kind == WorldEffect::WallTeleporting)
    }
}

pub struct Clock {
    last_frame: SystemTime,
    pub now: SystemTime,
    pub frame_delta: Duration,
}

impl Clock {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            last_frame: now,
            now,
            frame_delta: Duration::ZERO,
        }
    }

    pub fn update(&mut self, state: &GameState) {
        let now = SystemTime::now();

        match state {
            GameState::Paused(_) | GameState::Stopped(_) => {
                self.frame_delta = Duration::ZERO;
            }
            GameState::Starting(_) | GameState::Running(_) => {
                self.frame_delta = UPDATE_TIME;
                self.now += self.frame_delta;
            }
        }
        self.last_frame = now;
    }
}

#[derive(PartialEq, Eq)]
pub enum GameState {
    Starting(SystemTime),
    Running(SystemTime),
    Paused(SystemTime),
    Stopped(SystemTime),
}

pub struct Item {
    pub pos: Pos2,
    pub kind: ItemKind,
}

#[derive(Clone, Copy, EnumMembersArray)]
pub enum ItemKind {
    Speedup,
    Slowdown,
    FastTurning,
    SlowTurning,
    Expand,
    Shrink,
    Ghost,
    NoGap,
    WallTeleporting,
    Clear,
}

impl ItemKind {
    pub const fn color32(&self) -> Color32 {
        match self {
            Self::Speedup => Color32::from_rgb(50, 60, 200),
            Self::Slowdown => Color32::from_rgb(220, 20, 50),
            Self::FastTurning => Color32::from_rgb(150, 40, 240),
            Self::SlowTurning => Color32::from_rgb(4, 174, 247),
            Self::Expand => Color32::from_rgb(245, 210, 22),
            Self::Shrink => Color32::from_rgb(230, 120, 40),
            Self::Ghost => Color32::from_gray(240),
            Self::NoGap => Color32::from_rgb(4, 247, 136),
            Self::WallTeleporting => Color32::from_rgb(162, 230, 50),
            Self::Clear => Color32::from_rgb(230, 40, 220),
        }
    }

    pub const fn spawn_rate(&self) -> u8 {
        match self {
            ItemKind::Speedup => 4,
            ItemKind::Slowdown => 4,
            ItemKind::FastTurning => 4,
            ItemKind::SlowTurning => 4,
            ItemKind::Expand => 4,
            ItemKind::Shrink => 4,
            ItemKind::Ghost => 1,
            ItemKind::NoGap => 3,
            ItemKind::WallTeleporting => 4,
            ItemKind::Clear => 2,
        }
    }

    pub const fn name(&self) -> &str {
        match self {
            ItemKind::Speedup => "Speedup",
            ItemKind::Slowdown => "Slowdown",
            ItemKind::FastTurning => "Fast turning",
            ItemKind::SlowTurning => "Slow turning",
            ItemKind::Expand => "Expand",
            ItemKind::Shrink => "Shrink",
            ItemKind::Ghost => "Ghost",
            ItemKind::NoGap => "No gap",
            ItemKind::WallTeleporting => "Wall teleporting",
            ItemKind::Clear => "Clear trails",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Effect<T> {
    pub start: SystemTime,
    pub duration: Duration,
    pub kind: T,
}

#[derive(Debug, PartialEq)]
pub enum PlayerEffect {
    Size(f32),
    Speed(f32),
    Turning(f32),
    Ghost,
    NoGap,
    Gap,
}

impl PlayerEffect {
    pub fn item_kind(&self) -> Option<ItemKind> {
        let kind = match self {
            PlayerEffect::Size(s) if *s < 0.0 => ItemKind::Shrink,
            PlayerEffect::Size(_) => ItemKind::Expand,
            PlayerEffect::Speed(s) if *s < 0.0 => ItemKind::Slowdown,
            PlayerEffect::Speed(_) => ItemKind::Speedup,
            PlayerEffect::Turning(r) if *r < 0.0 => ItemKind::FastTurning,
            PlayerEffect::Turning(_) => ItemKind::SlowTurning,
            PlayerEffect::Ghost => ItemKind::Ghost,
            PlayerEffect::NoGap => ItemKind::NoGap,
            PlayerEffect::Gap => return None,
        };
        Some(kind)
    }
}

#[derive(PartialEq, Eq)]
pub enum WorldEffect {
    WallTeleporting,
}

#[derive(Debug, PartialEq)]
pub struct Player {
    pub id: u16,
    pub name: String,
    pub trail: Vec<TrailSection>,
    pub pos: Pos2,
    pub angle: f32,
    pub color: PlayerColor,
    pub effects: Vec<Effect<PlayerEffect>>,
    pub left_key: Key,
    pub right_key: Key,
    pub local_direction: Direction,
    pub remote_direction: Direction,
    pub just_crashed: bool,
    pub crashed: bool,
    pub score: u16,
}

impl Player {
    pub fn new(
        id: u16,
        name: String,
        pos: Pos2,
        angle: f32,
        color: PlayerColor,
        left_key: Key,
        right_key: Key,
    ) -> Self {
        Self {
            id,
            name,
            trail: Vec::new(),
            pos,
            angle,
            color,
            left_key,
            right_key,
            effects: Vec::new(),
            local_direction: Direction::Straight,
            remote_direction: Direction::Straight,
            just_crashed: false,
            crashed: false,
            score: 0,
        }
    }

    pub fn reset(&mut self, pos: Pos2) {
        let mut rng = rand::thread_rng();
        self.pos = pos;
        self.angle = rng.gen_range(0.0..TAU);
        self.effects.clear();
        self.trail.clear();
        self.local_direction = Direction::Straight;
        self.remote_direction = Direction::Straight;
        self.just_crashed = false;
        self.crashed = false;
    }

    pub fn gap(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e.kind, PlayerEffect::Gap | PlayerEffect::Ghost))
    }

    pub fn no_gap(&self) -> bool {
        self.effects.iter().any(|e| e.kind == PlayerEffect::NoGap)
    }

    fn direction(&self) -> Direction {
        match self.local_direction {
            Direction::Straight => self.remote_direction,
            _ => self.local_direction,
        }
    }

    fn speed(&self) -> f32 {
        let speed = BASE_SPEED
            + self
                .effects
                .iter()
                .filter_map(|e| match e.kind {
                    PlayerEffect::Speed(s) => Some(s),
                    _ => None,
                })
                .sum::<f32>();
        speed.max(MIN_SPEED)
    }

    pub fn thickness(&self) -> f32 {
        let thickness = BASE_THICKNESS
            + self
                .effects
                .iter()
                .filter_map(|e| match e.kind {
                    PlayerEffect::Size(s) => Some(s),
                    _ => None,
                })
                .sum::<f32>();
        thickness.max(MIN_THICKNESS)
    }

    fn turning_radius(&self) -> f32 {
        let radius = BASE_TURNING_RADIUS
            + self
                .effects
                .iter()
                .filter_map(|e| match e.kind {
                    PlayerEffect::Turning(r) => Some(r),
                    _ => None,
                })
                .sum::<f32>();
        radius.max(MIN_TURNING_RADIUS)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crash {
    pub time: SystemTime,
    pub message: CrashMessage,
}

impl Crash {
    pub fn new(time: SystemTime, message: CrashMessage) -> Self {
        Self { time, message }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrashMessage {
    Own {
        name: String,
        color: Color32,
    },
    Wall {
        name: String,
        color: Color32,
    },
    Other {
        crashed_name: String,
        crashed_color: Color32,
        other_name: String,
        other_color: Color32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumMembersArray)]
pub enum PlayerColor {
    Orange = 0,
    Green = 1,
    Purple = 2,
    Cyan = 3,
    Magenta = 4,
    Red = 5,
    Yellow = 6,
    Blue = 7,
}

impl PlayerColor {
    pub fn color32(&self) -> Color32 {
        match self {
            Self::Orange => Color32::from_rgb(230, 100, 20),
            Self::Green => Color32::from_rgb(50, 230, 20),
            Self::Purple => Color32::from_rgb(160, 100, 230),
            Self::Cyan => Color32::from_rgb(30, 200, 200),
            Self::Magenta => Color32::from_rgb(230, 40, 180),
            Self::Red => Color32::from_rgb(230, 40, 10),
            Self::Yellow => Color32::from_rgb(230, 230, 30),
            Self::Blue => Color32::from_rgb(70, 90, 230),
        }
    }

    pub fn prev(&mut self) {
        let idx = (*self as isize - 1).rem_euclid(PLAYER_COLORS.len() as isize);
        *self = PLAYER_COLORS[idx as usize];
    }

    pub fn next(&mut self) {
        let idx = (*self as usize + 1) % PLAYER_COLORS.len();
        *self = PLAYER_COLORS[idx];
    }
}

pub trait DirectionExt {
    fn turning_direction(&self) -> Option<TurnDirection>;
}

impl DirectionExt for Direction {
    fn turning_direction(&self) -> Option<TurnDirection> {
        match self {
            Direction::Straight => None,
            Direction::Right => Some(TurnDirection::Right),
            Direction::Left => Some(TurnDirection::Left),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TurnDirection {
    Right,
    Left,
}

impl TurnDirection {
    pub fn angle_sign(&self) -> f32 {
        match self {
            TurnDirection::Right => 1.0,
            TurnDirection::Left => -1.0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TrailSection {
    Straight(StraightTrailSection),
    Arc(ArcTrailSection),
}

impl TrailSection {
    fn dir(&self) -> Direction {
        match self {
            TrailSection::Straight(_) => Direction::Straight,
            TrailSection::Arc(s) => match s.dir {
                TurnDirection::Right => Direction::Right,
                TurnDirection::Left => Direction::Left,
            },
        }
    }

    pub fn gap(&self) -> bool {
        match self {
            TrailSection::Straight(s) => s.gap,
            TrailSection::Arc(s) => s.gap,
        }
    }

    pub fn thickness(&self) -> f32 {
        match self {
            TrailSection::Straight(s) => s.thickness,
            TrailSection::Arc(s) => s.thickness,
        }
    }

    pub fn start_pos(&self) -> Pos2 {
        match self {
            TrailSection::Straight(s) => s.start,
            TrailSection::Arc(s) => s.start_pos,
        }
    }

    pub fn end_pos(&self) -> Pos2 {
        match self {
            TrailSection::Straight(s) => s.end,
            TrailSection::Arc(s) => s.end_pos(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct StraightTrailSection {
    pub start: Pos2,
    pub gap: bool,
    pub thickness: f32,
    pub end: Pos2,
}

impl StraightTrailSection {
    pub fn new(start: Pos2, gap: bool, thickness: f32, end: Pos2) -> Self {
        Self {
            start,
            gap,
            thickness,
            end,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ArcTrailSection {
    /// The position of the player at the start of this arc section.
    pub start_pos: Pos2,
    pub gap: bool,
    pub thickness: f32,
    pub dir: TurnDirection,
    pub radius: f32,
    /// The angle the player was moving towards, at the start of this arc section.
    pub player_start_angle: f32,
    /// The angle the player was moving towards, at the end of this arc section.
    pub player_end_angle: f32,
}

impl ArcTrailSection {
    pub fn new(
        start_pos: Pos2,
        gap: bool,
        thickness: f32,
        dir: TurnDirection,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
    ) -> Self {
        Self {
            start_pos,
            gap,
            thickness,
            dir,
            radius,
            player_start_angle: start_angle,
            player_end_angle: end_angle,
        }
    }

    /// The position of the player at the end of this arc section.
    pub fn end_pos(&self) -> Pos2 {
        let start = self.arc_start_angle();
        let end = self.arc_end_angle();
        Pos2 {
            x: self.start_pos.x + (end.cos() - start.cos()) * self.radius,
            y: self.start_pos.y + (end.sin() - start.sin()) * self.radius,
        }
    }

    /// The center position of this arc section
    pub fn center_pos(&self) -> Pos2 {
        Pos2 {
            x: self.start_pos.x - self.arc_start_angle().cos() * self.radius,
            y: self.start_pos.y - self.arc_start_angle().sin() * self.radius,
        }
    }

    /// The angle from the `Self::center_pos()` of this arc section
    pub fn arc_start_angle(&self) -> f32 {
        self.player_start_angle - FRAC_PI_2 * self.dir.angle_sign()
    }

    /// The angle from the `Self::center_pos()` of this arc section
    pub fn arc_end_angle(&self) -> f32 {
        self.player_end_angle - FRAC_PI_2 * self.dir.angle_sign()
    }
}

impl World {
    pub fn update(&mut self) {
        let mut rng = rand::thread_rng();
        self.clock.update(&self.state);

        match self.state {
            GameState::Starting(start_time) => {
                for player in self.players.iter_mut() {
                    let Some(dir) = player.direction().turning_direction() else {
                        continue;
                    };
                    let delta_time = self.clock.frame_delta.as_secs_f32();
                    player.angle +=
                        delta_time * BASE_SPEED / BASE_TURNING_RADIUS * dir.angle_sign();
                }

                let now = self.clock.now;
                if now > start_time + START_DELAY {
                    self.state = GameState::Running(self.clock.now);
                }
            }
            GameState::Running(start_time) => {
                // remove effects
                self.effects
                    .retain(|e| e.start + e.duration > self.clock.now);

                // spawn items
                if self.items.len() < MAX_ITEMS {
                    let weighted_rate = self.clock.frame_delta.as_secs_f32() * ITEM_SPAWN_RATE;
                    if rng.gen_range(0.0..=1.0) < weighted_rate {
                        let item_kind_idx = rng.gen_range(0..SUM_OF_ITEM_SPAWN_RATES);
                        let mut idx = 0;
                        let mut item_kind = None;
                        for k in ITEM_KINDS.iter() {
                            idx += k.spawn_rate();
                            if idx > item_kind_idx {
                                item_kind = Some(*k);
                                break;
                            }
                        }
                        if let Some(pos) = gen_item_position(&self.players, &self.items) {
                            let item = Item {
                                pos,
                                kind: item_kind.expect("item kind should match one item"),
                            };
                            self.items.push(item);
                        }
                    }
                }

                // remove effects
                for p in self.players.iter_mut() {
                    p.effects.retain(|e| e.start + e.duration > self.clock.now);

                    if p.crashed {
                        continue;
                    }

                    let weighted_range = self.clock.frame_delta.as_secs_f32() * GAP_RATE;
                    if !p.gap() && !p.no_gap() && rng.gen_range(0.0..=1.0) < weighted_range {
                        p.effects.push(gap_effect(&self.clock));
                    }

                    move_player(&self.clock, p);
                }

                let wall_teleporting = self.wall_teleporting();
                for pi in 0..self.players.len() {
                    if self.players[pi].crashed {
                        continue;
                    }

                    // check for crash
                    if wall_teleporting {
                        let p = &mut self.players[pi];
                        if p.pos.x < 0.0 {
                            p.pos.x = WORLD_SIZE.x;
                            add_trail_section(p);
                        } else if p.pos.x > WORLD_SIZE.x {
                            p.pos.x = 0.0;
                            add_trail_section(p);
                        }

                        if p.pos.y < 0.0 {
                            p.pos.y = WORLD_SIZE.y;
                            add_trail_section(p);
                        } else if p.pos.y > WORLD_SIZE.y {
                            p.pos.y = 0.0;
                            add_trail_section(p);
                        }
                    } else {
                        let p = &mut self.players[pi];
                        let thickness = p.thickness();
                        if p.pos.x < 0.5 * thickness
                            || p.pos.x > WORLD_SIZE.x - 0.5 * thickness
                            || p.pos.y < 0.5 * thickness
                            || p.pos.y > WORLD_SIZE.y - 0.5 * thickness
                        {
                            self.crash_feed.push(Crash::new(
                                self.clock.now,
                                CrashMessage::Wall {
                                    name: p.name.clone(),
                                    color: p.color.color32(),
                                },
                            ));
                            p.just_crashed = true;
                        }
                    }

                    if !self.players[pi].gap() {
                        let p = &mut self.players[pi];
                        if intersects_own_trail(p) {
                            self.crash_feed.push(Crash::new(
                                self.clock.now,
                                CrashMessage::Own {
                                    name: p.name.clone(),
                                    color: p.color.color32(),
                                },
                            ));
                            p.just_crashed = true;
                        }

                        let others = self.players.iter().enumerate().filter(|(i, _)| pi != *i);
                        for (_, o) in others {
                            let p = &self.players[pi];
                            if intersects_trail(p.pos, 0.5 * p.thickness(), &o.trail) {
                                let other_name = o.name.clone();
                                let other_color = o.color.color32();
                                let p = &mut self.players[pi];
                                self.crash_feed.push(Crash::new(
                                    self.clock.now,
                                    CrashMessage::Other {
                                        crashed_name: p.name.clone(),
                                        crashed_color: p.color.color32(),
                                        other_name,
                                        other_color,
                                    },
                                ));
                                p.just_crashed = true;
                                break;
                            }
                        }
                    }

                    // collect items
                    let mut i = 0;
                    let mut clear_trails = false;
                    while i < self.items.len() {
                        let item = &self.items[i];
                        let dist = 0.5 * self.players[pi].thickness() + ITEM_RADIUS;
                        let p = &mut self.players[pi];
                        if intersects(p.pos, item.pos, dist) {
                            match item.kind {
                                ItemKind::Speedup => {
                                    p.effects.push(player_effect(
                                        &self.clock,
                                        PlayerEffect::Speed(50.0),
                                    ));
                                }
                                ItemKind::Slowdown => {
                                    p.effects.push(player_effect(
                                        &self.clock,
                                        PlayerEffect::Speed(-50.0),
                                    ));
                                }
                                ItemKind::FastTurning => {
                                    p.effects.push(player_effect(
                                        &self.clock,
                                        PlayerEffect::Turning(-20.0),
                                    ));
                                }
                                ItemKind::SlowTurning => {
                                    p.effects.push(player_effect(
                                        &self.clock,
                                        PlayerEffect::Turning(20.0),
                                    ));
                                }
                                ItemKind::Expand => {
                                    p.effects
                                        .push(player_effect(&self.clock, PlayerEffect::Size(4.0)));
                                }
                                ItemKind::Shrink => {
                                    p.effects
                                        .push(player_effect(&self.clock, PlayerEffect::Size(-2.0)));
                                }
                                ItemKind::Ghost => {
                                    p.effects.retain(|e| e.kind != PlayerEffect::NoGap);
                                    p.effects
                                        .push(player_effect(&self.clock, PlayerEffect::Ghost));
                                }
                                ItemKind::NoGap => {
                                    p.effects.retain(|e| e.kind != PlayerEffect::Gap);
                                    p.effects
                                        .push(player_effect(&self.clock, PlayerEffect::NoGap));
                                }
                                ItemKind::WallTeleporting => {
                                    self.effects.push(world_effect(
                                        &self.clock,
                                        WorldEffect::WallTeleporting,
                                    ));
                                }
                                ItemKind::Clear => clear_trails = true,
                            }

                            self.items.remove(i);
                            continue;
                        }
                        i += 1;
                    }

                    if clear_trails {
                        for p in self.players.iter_mut() {
                            p.trail.clear();
                        }
                    }
                }

                let mut num_alive_players = 0;
                for p in self.players.iter_mut() {
                    if p.just_crashed {
                        p.crashed = true;
                    } else {
                        num_alive_players += 1;
                    }
                }
                if num_alive_players < 2 {
                    for p in self.players.iter_mut() {
                        if !p.crashed {
                            p.score += 1;
                        }
                    }
                    self.state = GameState::Stopped(start_time);
                }
            }
            GameState::Paused(_) => (),
            GameState::Stopped(_) => (),
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            GameState::Running(s) => self.state = GameState::Paused(s),
            GameState::Paused(s) => self.state = GameState::Running(s),
            _ => (),
        }
    }

    pub fn restart(&mut self) {
        if matches!(self.state, GameState::Stopped(_)) {
            self.state = GameState::Starting(self.clock.now);
            self.items.clear();
            self.effects.clear();
            self.crash_feed.clear();

            for i in 0..self.players.len() {
                let pos = gen_player_position(&self.players[0..i]);
                self.players[i].reset(pos);
            }
        }
    }

    pub fn next_id(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_player(&mut self) {
        if self.players.len() >= PLAYER_COLORS.len() {
            return;
        }
        let id = self.next_id();
        let name = format!("Player{}", self.players.len() + 1);
        let player = random_player(id, name, Key::ArrowLeft, Key::ArrowRight, &self.players);
        self.players.push(player)
    }

    pub fn remove_player(&mut self, idx: usize) {
        if self.players.len() > 2 {
            self.players.remove(idx);
        }
    }
}

pub fn move_player(clock: &Clock, player: &mut Player) {
    if player.trail.is_empty() {
        add_trail_section(player);
        return;
    }

    let last_trail = player
        .trail
        .last()
        .expect("There should be at least on trail section");
    if player.direction() != last_trail.dir()
        || player.gap() != last_trail.gap()
        || player.thickness() != last_trail.thickness()
    {
        add_trail_section(player);
    } else if let TrailSection::Arc(s) = last_trail {
        if player.turning_radius() != s.radius {
            add_trail_section(player);
        }
    }

    update_trail_section(clock, player);
}

fn update_trail_section(clock: &Clock, player: &mut Player) {
    let delta_time = clock.frame_delta.as_secs_f32();
    let speed = player.speed();
    let last_trail = player
        .trail
        .last_mut()
        .expect("There should be at least on trail section");
    match last_trail {
        TrailSection::Straight(s) => {
            s.end.x += delta_time * speed * player.angle.cos();
            s.end.y += delta_time * speed * player.angle.sin();
            player.pos = s.end;
        }
        TrailSection::Arc(s) => {
            s.player_end_angle += delta_time * speed / s.radius * s.dir.angle_sign();
            player.pos = s.end_pos();
            player.angle = s.player_end_angle;
        }
    }
}

fn add_trail_section(player: &mut Player) {
    match player.direction().turning_direction() {
        None => {
            let section =
                StraightTrailSection::new(player.pos, player.gap(), player.thickness(), player.pos);
            player.pos = section.end;
            player.trail.push(TrailSection::Straight(section));
        }
        Some(dir) => {
            let section = ArcTrailSection::new(
                player.pos,
                player.gap(),
                player.thickness(),
                dir,
                player.turning_radius(),
                player.angle,
                player.angle,
            );
            player.pos = section.end_pos();
            player.trail.push(TrailSection::Arc(section));
        }
    }
}

fn random_player(
    id: u16,
    name: String,
    left_key: Key,
    right_key: Key,
    others: &[Player],
) -> Player {
    let mut rng = rand::thread_rng();
    let pos = gen_player_position(others);
    let angle = rng.gen_range(0.0..TAU);
    let color_idx = rng.gen_range(0..PLAYER_COLORS.len() - others.len());
    let color = PLAYER_COLORS
        .iter()
        .filter(|c| others.iter().all(|p| **c != p.color))
        .nth(color_idx)
        .unwrap();

    Player::new(id, name, pos, angle, *color, left_key, right_key)
}

fn gen_player_position(others: &[Player]) -> Pos2 {
    let mut rng = rand::thread_rng();
    let mut pos = Pos2::ZERO;

    'outer: for _ in 0..1_000_000 {
        pos = Pos2 {
            x: rng.gen_range(MIN_PLAYER_WALL_DIST..(WORLD_SIZE.x - MIN_PLAYER_WALL_DIST)),
            y: rng.gen_range(MIN_PLAYER_WALL_DIST..(WORLD_SIZE.y - MIN_PLAYER_WALL_DIST)),
        };

        for o in others.iter() {
            if intersects(pos, o.pos, MIN_PLAYER_DIST) {
                continue 'outer;
            }
        }

        break;
    }

    pos
}

fn gen_item_position(players: &[Player], items: &[Item]) -> Option<Pos2> {
    let mut rng = rand::thread_rng();

    'outer: for _ in 0..10_000 {
        let pos = Pos2 {
            x: rng.gen_range(MIN_ITEM_WALL_DIST..(WORLD_SIZE.x - MIN_ITEM_WALL_DIST)),
            y: rng.gen_range(MIN_ITEM_WALL_DIST..(WORLD_SIZE.y - MIN_ITEM_WALL_DIST)),
        };

        for p in players.iter() {
            if intersects_trail(pos, MIN_ITEM_DIST, &p.trail) {
                continue 'outer;
            }
        }

        for i in items.iter() {
            if intersects(pos, i.pos, MIN_ITEM_DIST) {
                continue 'outer;
            }
        }

        return Some(pos);
    }

    None
}

fn intersects_own_trail(player: &Player) -> bool {
    for s in player.trail.iter() {
        let min_dist = 0.5 * player.thickness() + 0.5 * s.thickness();
        let end_dist = player.pos.distance(s.end_pos());

        if end_dist < min_dist {
            if let TrailSection::Arc(s) = s {
                let angle_diff = (s.player_start_angle - s.player_end_angle).abs();
                if angle_diff > PI {
                    let start_dist = s.start_pos.distance(player.pos);
                    if start_dist < min_dist {
                        return true;
                    }
                }
            }
        }

        if end_dist > min_dist {
            let intersects = intersects_trail(
                player.pos,
                0.5 * player.thickness(),
                std::slice::from_ref(s),
            );
            if intersects {
                return true;
            }
        }
    }

    false
}

fn intersects_trail(pos: Pos2, dist: f32, trail: &[TrailSection]) -> bool {
    for s in trail.iter() {
        if s.gap() {
            continue;
        }

        match s {
            TrailSection::Straight(s) => {
                if intersects_straight_trailsection(s, pos, dist) {
                    return true;
                }
            }
            TrailSection::Arc(s) => {
                if intersects_arc_trailsection(s, pos, dist) {
                    return true;
                }
            }
        }
    }

    false
}

fn intersects_straight_trailsection(s: &StraightTrailSection, pos: Pos2, dist: f32) -> bool {
    let p1_dist = s.start.distance(pos);
    let p2_dist = s.end.distance(pos);
    let max_dist = 0.5 * s.thickness + dist;
    if p1_dist < max_dist || p2_dist < max_dist {
        return true;
    }

    let center_line_angle = angle(s.start, s.end).rem_euclid(TAU);
    let inverse_center_line_angle = (center_line_angle + PI).rem_euclid(TAU);

    let outer_line_pos_1 = Pos2 {
        x: s.start.x + (center_line_angle - FRAC_PI_2).cos() * (0.5 * s.thickness + dist),
        y: s.start.y + (center_line_angle - FRAC_PI_2).sin() * (0.5 * s.thickness + dist),
    };
    let outer_line_pos_2 = Pos2 {
        x: s.start.x - (center_line_angle - FRAC_PI_2).cos() * (0.5 * s.thickness + dist),
        y: s.start.y - (center_line_angle - FRAC_PI_2).sin() * (0.5 * s.thickness + dist),
    };

    let max_dist = s.end.distance(outer_line_pos_1);
    if p1_dist > max_dist || p2_dist > max_dist {
        return false;
    }

    let angle_l1 = angle(outer_line_pos_1, pos).rem_euclid(TAU);
    let angle_l2 = angle(outer_line_pos_2, pos).rem_euclid(TAU);
    if center_line_angle < inverse_center_line_angle {
        if (angle_l1 > center_line_angle && angle_l1 < inverse_center_line_angle)
            != (angle_l2 > center_line_angle && angle_l2 < inverse_center_line_angle)
        {
            return true;
        }
    } else {
        if (angle_l1 > center_line_angle || angle_l1 < inverse_center_line_angle)
            != (angle_l2 > center_line_angle || angle_l2 < inverse_center_line_angle)
        {
            return true;
        }
    }

    false
}

fn intersects_arc_trailsection(s: &ArcTrailSection, pos: Pos2, dist: f32) -> bool {
    let p1_dist = s.start_pos.distance(pos);
    let p2_dist = s.end_pos().distance(pos);
    let max_dist = 0.5 * s.thickness + dist;
    if p1_dist < max_dist || p2_dist < max_dist {
        return true;
    }

    let min_dist = s.radius - 0.5 * s.thickness - dist;
    let max_dist = s.radius + 0.5 * s.thickness + dist;
    let center_pos = s.center_pos();
    let arc_center_dist = center_pos.distance(pos);
    if arc_center_dist < min_dist || arc_center_dist > max_dist {
        return false;
    }

    let arc_start_angle = if s.dir == TurnDirection::Right {
        s.arc_start_angle().rem_euclid(TAU)
    } else {
        s.arc_end_angle().rem_euclid(TAU)
    };
    let arc_end_angle = if s.dir == TurnDirection::Right {
        s.arc_end_angle().rem_euclid(TAU)
    } else {
        s.arc_start_angle().rem_euclid(TAU)
    };

    let arc_angle = angle(center_pos, pos).rem_euclid(TAU);
    if arc_start_angle <= arc_end_angle {
        if arc_angle > arc_start_angle && arc_angle < arc_end_angle {
            return true;
        }
    } else {
        if arc_angle > arc_start_angle || arc_angle < arc_end_angle {
            return true;
        }
    }

    false
}

fn intersects(a: Pos2, b: Pos2, dist: f32) -> bool {
    a.distance(b) < dist
}

fn angle(a: Pos2, b: Pos2) -> f32 {
    let diff = b - a;
    f32::atan2(diff.y, diff.x)
}

fn gap_effect(clock: &Clock) -> Effect<PlayerEffect> {
    let mut rng = rand::thread_rng();
    Effect {
        start: clock.now,
        duration: GAP_EFFECT_DURATION + rng.gen_range(0..=1) * GAP_EFFECT_DEVIATION_DURATION,
        kind: PlayerEffect::Gap,
    }
}

fn player_effect(clock: &Clock, kind: PlayerEffect) -> Effect<PlayerEffect> {
    let mut rng = rand::thread_rng();
    Effect {
        start: clock.now,
        duration: PLAYER_EFFECT_DURATION + rng.gen_range(0..=1) * PLAYER_EFFECT_DEVIATION_DURATION,
        kind,
    }
}

fn world_effect(clock: &Clock, kind: WorldEffect) -> Effect<WorldEffect> {
    let mut rng = rand::thread_rng();
    Effect {
        start: clock.now,
        duration: WORLD_EFFECT_DURATION + rng.gen_range(0..=1) * WORLD_EFFECT_DEVIATION_DURATION,
        kind,
    }
}
