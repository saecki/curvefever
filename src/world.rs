use std::f32::consts::{TAU, FRAC_PI_2, PI};
use std::time::{Duration, SystemTime};

use egui::{Color32, Key, Pos2, Vec2};
use rand::seq::SliceRandom;
use rand::Rng;

use curvefever_derive::EnumMembersArray;

pub const WORLD_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);
pub const MIN_WALL_DIST: f32 = 150.0;
pub const MIN_PLAYER_DIST: f32 = 200.0;
pub const MIN_ITEM_DIST: f32 = 120.0;
pub const ITEM_SPAWN_RATE: f32 = 0.002;
pub const START_DELAY: Duration = Duration::from_secs(2);
pub const MAX_ITEMS: usize = 5;

pub const BASE_SPEED: f32 = 50.0;
pub const MIN_SPEED: f32 = 25.0;
pub const BASE_THICKNESS: f32 = 4.0;
pub const MIN_THICKNESS: f32 = 1.0;
pub const BASE_TURNING_RADIUS: f32 = 50.0;
pub const MIN_TURNING_RADIUS: f32 = 25.0;

struct World {
    clock: Clock,
    state: GameState,
    items: Vec<Item>,
    effects: Vec<Effect<WorldEffect>>,
    players: Vec<Player>,
}

impl World {
    pub fn new() -> Self {
        let mut players = Vec::with_capacity(2);
        let mut player1 = random_player(
            "Player1".to_string(),
            Key::ArrowLeft,
            Key::ArrowRight,
            &players,
        );
        players.push(player1);
        let mut player2 = random_player("Player1".to_string(), Key::A, Key::D, &players);
        players.push(player2);

        Self {
            clock: Clock::new(),
            state: GameState::Stopped,
            items: Vec::new(),
            effects: Vec::new(),
            players,
        }
    }

    fn wall_teleporting(&self) -> bool {
        self.effects.iter().any(|e| e.kind == WorldEffect::WallTeleporting)
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

    pub fn update(&mut self, paused: bool) {
        let now = SystemTime::now();

        if paused {
            self.frame_delta = Duration::ZERO;
        } else {
            self.frame_delta = now.duration_since(self.last_frame).unwrap();
            self.now += self.frame_delta;
        }
        self.last_frame = now;
    }
}

#[derive(PartialEq, Eq)]
pub enum GameState {
    Starting(SystemTime),
    Running,
    Paused,
    Stopped,
}

pub struct Item {
    pos: Pos2,
    kind: ItemKind,
}

#[derive(EnumMembersArray)]
pub enum ItemKind {
    Gap,
    Speedup,
    Slowdown,
    FastTurning,
    SlowTurning,
    Expand,
    Shrink,
    WallTeleporting,
    Clear,
}

pub struct Effect<T> {
    start: SystemTime,
    duration: Duration,
    kind: T,
}

#[derive(PartialEq)]
pub enum PlayerEffect {
    Size(f32),
    Speed(f32),
    Turning(f32),
    Gap,
}

#[derive(PartialEq, Eq)]
pub enum WorldEffect {
    WallTeleporting,
}

pub struct Player {
    name: String,
    trail: Vec<TrailSection>,
    pos: Pos2,
    angle: f32,
    color: Color,
    effects: Vec<Effect<PlayerEffect>>,
    left_key: Key,
    right_key: Key,
    left_down: bool,
    right_down: bool,
    direction: Direction,
    just_crashed: bool,
    crashed: bool,
    score: u16,
}

impl Player {
    pub fn new(
        name: String,
        pos: Pos2,
        angle: f32,
        color: Color,
        left_key: Key,
        right_key: Key,
    ) -> Self {
        Self {
            name,
            trail: Vec::new(),
            pos,
            angle,
            color,
            left_key,
            right_key,
            left_down: false,
            right_down: false,
            effects: Vec::new(),
            direction: Direction::Straight,
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
        self.direction = Direction::Straight;
        self.crashed = false;

        self.left_down = false;
        self.right_down = false;
    }

    fn last_trail_mut(&mut self) -> &mut TrailSection {
        self.trail
            .last_mut()
            .expect("There should be at least on trail section")
    }

    fn gap(&self) -> bool {
        self.effects.iter().any(|e| e.kind == PlayerEffect::Gap)
    }

    fn speed(&self) -> f32 {
        let speed = BASE_SPEED
            + self
                .effects
                .iter()
                .filter_map(|e| match e.kind {
                    PlayerEffect::Size(s) => Some(s),
                    _ => None,
                })
                .sum::<f32>();
        speed.max(MIN_SPEED)
    }

    fn thickness(&self) -> f32 {
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

#[derive(EnumMembersArray)]
pub enum Color {
    Color0 = 0,
    Color1 = 1,
    Color2 = 2,
    Color3 = 3,
    Color4 = 4,
    Color5 = 5,
    Color6 = 6,
    Color7 = 7,
}

impl Color {
    fn color32(&self) -> Color32 {
        match self {
            Self::Color0 => Color32::from_rgb(230, 100, 20),
            Self::Color1 => Color32::from_rgb(50, 230, 20),
            Self::Color2 => Color32::from_rgb(130, 100, 200),
            Self::Color3 => Color32::from_rgb(30, 200, 200),
            Self::Color4 => Color32::from_rgb(230, 40, 200),
            Self::Color5 => Color32::from_rgb(230, 20, 10),
            Self::Color6 => Color32::from_rgb(230, 230, 30),
            Self::Color7 => Color32::from_rgb(50, 40, 230),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    Straight,
    Right,
    Left,
}

impl Direction {
    fn turning_direction(&self) -> Option<TurnDirection> {
        match self {
            Direction::Straight => None,
            Direction::Right => Some(TurnDirection::Right),
            Direction::Left => Some(TurnDirection::Left),
        }
    }
}

#[derive(PartialEq, Eq)]
enum TurnDirection {
    Right,
    Left,
}

impl TurnDirection {
    fn angle_signum(&self) -> f32 {
        match self {
            TurnDirection::Right => 1.0,
            TurnDirection::Left => -1.0,
        }
    }
}

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

    fn gap(&self) -> bool {
        match self {
            TrailSection::Straight(s) => s.gap,
            TrailSection::Arc(s) => s.gap,
        }
    }

    fn thickness(&self) -> f32 {
        match self {
            TrailSection::Straight(s) => s.thickness,
            TrailSection::Arc(s) => s.thickness,
        }
    }
}

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

pub struct ArcTrailSection {
    pub start: Pos2,
    pub gap: bool,
    pub thickness: f32,
    pub dir: TurnDirection,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
}

impl ArcTrailSection {
    pub fn new(
        start: Pos2,
        gap: bool,
        thickness: f32,
        dir: TurnDirection,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
    ) -> Self {
        Self {
            start,
            gap,
            thickness,
            dir,
            radius,
            start_angle,
            end_angle,
        }
    }

    pub fn end_pos(&self) -> Pos2 {
        Pos2 {
            x: self.start.x + (self.end_angle.cos() - self.start_angle.cos()) * self.radius,
            y: self.start.y + (self.end_angle.sin() - self.start_angle.sin()) * self.radius,
        }
    }

    pub fn center_pos(&self) -> Pos2 {
        Pos2 {
            x: self.start.x - self.start_angle.cos() * self.radius,
            y: self.start.y - self.start_angle.sin() * self.radius,
        }
    }
}

impl World {
    pub fn update(&mut self) {
        let mut rng = rand::thread_rng();
        self.clock.update(self.state == GameState::Paused);

        match self.state {
            GameState::Starting(start_time) => {
                let now = self.clock.now;
                if now > start_time + START_DELAY {
                    self.state = GameState::Running;
                }
            }
            GameState::Running => {
                // remove effects
                self.effects
                    .retain(|e| e.start + e.duration > self.clock.now);

                // spawn items
                if self.items.len() < MAX_ITEMS {
                    if rng.gen::<f32>() >= ITEM_SPAWN_RATE {
                        let item = Item {
                            pos: gen_item_position(&self.players, &self.items),
                            kind: *ItemKind::members().choose(&mut rng).unwrap(),
                        };
                        self.items.push(item);
                    }
                }

                for p in self.players.iter_mut() {
                    if p.crashed { continue; }

                    // remove effects
                    p.effects.retain(|e| e.start + e.duration > self.clock.now);

                    move_player(&self.clock, p);
                }
                for p in self.players.iter_mut() {
                    if p.crashed { continue; }

                    // check for crash
                    if self.wall_teleporting() {
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
                        let thickness = p.thickness();
                        if p.pos.x < 0.5 * thickness || p.pos.x > WORLD_SIZE.x - 0.5 * thickness
                            || p.pos.y < 0.5 * thickness || p.pos.y > WORLD_SIZE.y - 0.5 * thickness
                         {
                             p.just_crashed = true;
                            // TODO: world.crashed(this, WALL_CRASH_MESSAGE)
                        }
                    }

                    if !p.gap() {
                        if p.intersects_with_own_trail() {
                            world.crashed(this, SELF_CRASH_MESSAGE)
                        }

                        val others = world.players.filter { it != this }
                        for (p in others) {
                            if (p.intersectsWith(this.x, this.y, this.thickness / 2)) {
                                world.crashed(this, PLAYER_CRASH_MESSAGE + p.name)
                                break
                            }
                        }
                    }


                    // collect items
                }
                for p in self.players.iter() {
                    if p.just_crashed {
                        p.crashed = true;
                    }
                }
            }
            GameState::Paused => (),
            GameState::Stopped => (),
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.state == GameState::Running {
            self.state = GameState::Paused;
        } else if self.state == GameState::Paused {
            self.state = GameState::Running;
        }
    }

    pub fn restart(&mut self) {
        if matches!(self.state, GameState::Paused | GameState::Stopped) {
            self.state = GameState::Starting(self.clock.now);
            self.items.clear();
            self.effects.clear();

            let mut new_players = Vec::with_capacity(self.players.len());
            for mut p in self.players.into_iter() {
                let pos = gen_player_position(&new_players);
                p.reset(pos);
                new_players.push(p);
            }
        }
    }
}

pub fn move_player(clock: &Clock, player: &mut Player) {
    if player.trail.is_empty() {
        add_trail_section(player);
    } else {
        update_trail_section(clock, player);
    }

    let last_trail = player
        .trail
        .last()
        .expect("There should be at least on trail section");

    if player.direction != last_trail.dir() {
        add_trail_section(player);
    } else if player.gap() != last_trail.gap() {
        add_trail_section(player);
    } else if player.thickness() != last_trail.thickness() {
        add_trail_section(player);
    } else if let TrailSection::Arc(s) = last_trail {
        if player.turning_radius() != s.radius {
            add_trail_section(player);
        }
    }
}

fn update_trail_section(clock: &Clock, player: &mut Player) {
    let delta_millis = clock.frame_delta.as_millis() as f32;
    match player
        .trail
        .last_mut()
        .expect("There should be at least on trail section")
    {
        TrailSection::Straight(s) => {
            s.end.x += delta_millis * player.speed() * player.angle.cos();
            s.end.y += delta_millis * player.speed() * player.angle.sin();
            player.pos = s.end;
        }
        TrailSection::Arc(s) => {
            s.end_angle += delta_millis * player.speed() / s.radius * s.dir.angle_signum();
            player.pos = s.end_pos();
            player.angle = s.end_angle;
        }
    }
}

fn add_trail_section(player: &mut Player) {
    match player.direction.turning_direction() {
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

fn random_player(name: String, left_key: Key, right_key: Key, others: &[Player]) -> Player {
    let mut rng = rand::thread_rng();
    let pos = gen_player_position(&others);
    let angle = rng.gen_range(0.0..TAU);
    let color = *Color::members().choose(&mut rng).unwrap();
    Player::new(name, pos, angle, color, left_key, right_key)
}

fn gen_player_position(others: &[Player]) -> Pos2 {
    let mut rng = rand::thread_rng();
    let mut pos = Pos2::ZERO;

    'outer: for _ in 0..1000 {
        pos = Pos2 {
            x: rng.gen_range(MIN_WALL_DIST..(WORLD_SIZE.x - 2.0 * MIN_WALL_DIST)),
            y: rng.gen_range(MIN_WALL_DIST..(WORLD_SIZE.y - 2.0 * MIN_WALL_DIST)),
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

fn gen_item_position(players: &[Player], items: &[Item]) -> Pos2 {
    let mut rng = rand::thread_rng();
    let mut pos = Pos2::ZERO;

    'outer: for _ in 0..1000 {
        pos = Pos2 {
            x: rng.gen_range(MIN_WALL_DIST..(WORLD_SIZE.x - 2.0 * MIN_WALL_DIST)),
            y: rng.gen_range(MIN_WALL_DIST..(WORLD_SIZE.y - 2.0 * MIN_WALL_DIST)),
        };

        for p in players.iter() {
            if intersects(pos, p.pos, MIN_ITEM_DIST) {
                continue 'outer;
            }
        }

        for i in items.iter() {
            if intersects(pos, i.pos, MIN_ITEM_DIST) {
                continue 'outer;
            }
        }

        break;
    }

    pos
}

fn intersects_with_trail(pos: Pos2, dist: f32, trail: &[TrailSection]) -> bool {
    for s in trail.iter() {
        if s.gap() {
            continue
        }

        match s {
            TrailSection::Straight(s) => if intersects_straight_trailsection(s, pos, dist) {
                return true;
            }
            TrailSection::Arc(s) => if intersects_arc_trailsection(s, pos, dist) {
                return true;
            }
        }
    }

    return false
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
        x: s.start.x + (center_line_angle - FRAC_PI_2).cos() * 0.5 * s.thickness,
        y: s.start.y + (center_line_angle - FRAC_PI_2).sin() * 0.5 * s.thickness,
    };
    let outer_line_pos_2 = Pos2 { 
        x: s.start.x - (center_line_angle - FRAC_PI_2).cos() * 0.5 * s.thickness,
        y: s.start.y - (center_line_angle - FRAC_PI_2).sin() * 0.5 * s.thickness,
    };

    let max_dist = PApplet.dist(x2, y2, x_l1, y_l1);
    if p1_dist > max_dist || p2_dist > max_dist {
        return false;
    }

    let angleL1 = angle(x_l1, y_l1, x, y).rem_euclid(PApplet.TAU);
    let angleL2 = angle(x_l2, y_l2, x, y).rem_euclid(PApplet.TAU);

    if (center_line_angle < inverse_center_line_angle) {
        if ((angleL1 > center_line_angle && angleL1 < inverse_center_line_angle)
            != (angleL2 > center_line_angle && angleL2 < inverse_center_line_angle)
        ) {
            printD("linear\n")
            return true
        }
    } else {
        if ((angleL1 > center_line_angle || angleL1 < inverse_center_line_angle)
            != (angleL2 > center_line_angle || angleL2 < inverse_center_line_angle)
        ) {
            printD("linear\n")
            return true
        }
    }

    return false
}

fn intersects_arc_trailsection(s: &ArcTrailSection, pos: Pos2, dist: f32) -> bool {
    let p1_dist = s.start.distance(pos);
    let p2_dist = s.end_pos().distance(pos);
    let max_dist = 0.5 * s.thickness + dist;
    if p1_dist < max_dist || p2_dist < max_dist {
        return true
    }

    let min_dist = s.radius - 0.5 * s.thickness - dist;
    let max_dist = s.radius + 0.5 * s.thickness + dist;
    let center_pos = s.center_pos();
    let arc_center_dist = center_pos.distance(pos);
    if arc_center_dist < min_dist || arc_center_dist > max_dist {
        return false
    }

    let arc_start_angle =
        (if s.dir == TurnDirection::Right { s.start_angle } else { s.end_angle }).rem_euclid(TAU);
    let arc_end_angle =
        (if s.dir == TurnDirection::Right { s.end_angle } else { s.start_angle }).rem_euclid(TAU);
    
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
