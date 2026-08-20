//! Rendering and input only. No game rules live here — every legality
//! question (does this piece fit, did the slot assemble, who wins the fight)
//! goes to the engine.

use std::collections::HashSet;

use gearmaster_engine::combat::{CombatLog, Event, Outcome, Side, LADDER};
use gearmaster_engine::loadout::{ItemProfile, Loadout, SlotReport};
use gearmaster_engine::piece::{
    default_cooldown_ms, PieceDef, PieceId, PieceKind, PieceRegistry, SlotKind,
};
use gearmaster_engine::run::{Phase, Run};
use gearmaster_engine::shop::REROLL_COST;
use gearmaster_engine::shape::Shape;
use gearmaster_engine::slot::{SLOT_H, SLOT_W};
use macroquad::prelude::*;

/// The interface is authored at this fixed size and scaled to fit whatever
/// window it actually gets — a resized desktop window, or a browser canvas of
/// any shape. Everything below works in these coordinates and never asks the
/// window how big it is.
const LOGICAL_W: f32 = 1600.0;
const LOGICAL_H: f32 = 980.0;

const PANEL_W: f32 = 366.0;
const SLOT_CELL: f32 = 26.0;
const SLOT_GAP: f32 = 22.0;
const SLOT_TOP: f32 = 112.0;
const INV_CELL: f32 = 15.0;
const CARD_W: f32 = 124.0;
const CARD_H: f32 = 146.0;
const CARD_GAP: f32 = 10.0;
/// How long an item jitters after it fires, and how far.
const SHAKE_MS: u32 = 260;
const SHAKE_PX: f32 = 3.4;

/// Cell size for the gear boards shown during a fight. Bigger than the
/// loadout screen's: the two boards stack vertically rather than sitting side
/// by side, so width stops being the constraint.
const MINI_CELL: f32 = 32.0;
const MINI_GAP: f32 = 14.0;

/// Playback rate a fight opens at, and the rates the speed button cycles.
const DEFAULT_SPEED: f32 = 1.0;
/// How long a struck fighter's panel stays tinted.
const FLASH_SECS: f64 = 0.22;

/// Where the logical canvas lands on the real screen, letterboxed and centred.
struct Viewport {
    x: f32,
    y: f32,
    scale: f32,
}

impl Viewport {
    fn current() -> Self {
        // The only place in the file that asks the real window how big it is.
        let (sw, sh) = (screen_width(), screen_height());
        let scale = (sw / LOGICAL_W).min(sh / LOGICAL_H);
        Viewport {
            x: (sw - LOGICAL_W * scale) / 2.0,
            y: (sh - LOGICAL_H * scale) / 2.0,
            scale,
        }
    }

    fn camera(&self) -> Camera2D {
        // NOT `Camera2D::from_display_rect`: that sets a negative y-zoom, which
        // double-flips against macroquad's screen convention and renders the
        // whole frame upside down. Positive y-zoom keeps y pointing down.
        let mut cam = Camera2D {
            target: vec2(LOGICAL_W / 2.0, LOGICAL_H / 2.0),
            zoom: vec2(2.0 / LOGICAL_W, 2.0 / LOGICAL_H),
            ..Default::default()
        };
        cam.viewport = Some((
            self.x as i32,
            self.y as i32,
            (LOGICAL_W * self.scale) as i32,
            (LOGICAL_H * self.scale) as i32,
        ));
        cam
    }

    /// Real mouse pixels -> logical coordinates. Every input path uses this,
    /// so hit-testing lines up with drawing at any window size.
    fn mouse(&self) -> (f32, f32) {
        // GEARMASTER_MOUSE=x,y pins the cursor in logical coordinates, so a
        // hover state can be captured in a screenshot.
        if let Ok(v) = std::env::var("GEARMASTER_MOUSE") {
            if let Some((a, b)) = v.split_once(',') {
                if let (Ok(x), Ok(y)) = (a.trim().parse(), b.trim().parse()) {
                    return (x, y);
                }
            }
        }
        let (mx, my) = mouse_position();
        ((mx - self.x) / self.scale, (my - self.y) / self.scale)
    }
}

fn window_conf() -> Conf {
    // GEARMASTER_SIZE=WxH overrides the opening window, for checking that the
    // logical viewport scales properly.
    let (w, h) = std::env::var("GEARMASTER_SIZE")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once('x')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .unwrap_or((LOGICAL_W as i32, LOGICAL_H as i32));
    Conf {
        window_title: "Gear Master".to_string(),
        window_width: w,
        window_height: h,
        window_resizable: true,
        ..Default::default()
    }
}

// ============================================================== layout

/// One 6x8 equipment grid on screen.
#[derive(Clone, Copy)]
struct SlotView {
    kind: SlotKind,
    origin: (f32, f32),
}

impl SlotView {
    fn size(&self) -> (f32, f32) {
        (SLOT_W as f32 * SLOT_CELL, SLOT_H as f32 * SLOT_CELL)
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        let (w, h) = self.size();
        x >= self.origin.0 && x < self.origin.0 + w && y >= self.origin.1 && y < self.origin.1 + h
    }

    /// grid -> pixels (top-left of the cell)
    fn cell_origin(&self, gx: u8, gy: u8) -> (f32, f32) {
        (self.origin.0 + gx as f32 * SLOT_CELL, self.origin.1 + gy as f32 * SLOT_CELL)
    }

    /// pixels -> grid
    fn hit(&self, mx: f32, my: f32) -> Option<(u8, u8)> {
        let gx = ((mx - self.origin.0) / SLOT_CELL).floor() as i32;
        let gy = ((my - self.origin.1) / SLOT_CELL).floor() as i32;
        if !(0..SLOT_W as i32).contains(&gx) || !(0..SLOT_H as i32).contains(&gy) {
            return None;
        }
        Some((gx as u8, gy as u8))
    }
}

#[derive(Clone, Copy)]
struct Card {
    id: PieceId,
    rect: Rect,
}

/// A shelf in the shop. Refers to a catalog entry, not an owned piece.
#[derive(Clone, Copy)]
struct ShopCard {
    slot_index: usize,
    def: &'static PieceDef,
    rect: Rect,
}

/// Recomputed every frame from the engine state and the window size. Owns both
/// coordinate directions so drawing and hit-testing cannot drift apart.
struct Layout {
    slots: Vec<SlotView>,
    cards: Vec<Card>,
    shop_cards: Vec<ShopCard>,
    shop: Rect,
    inv: Rect,
    panel_x: f32,
}

impl Layout {
    fn build(run: &Run) -> Self {
        let panel_x = LOGICAL_W - PANEL_W;
        let (gw, gh) = (SLOT_W as f32 * SLOT_CELL, SLOT_H as f32 * SLOT_CELL);
        let total = 5.0 * gw + 4.0 * SLOT_GAP;
        let x0 = ((panel_x - total) / 2.0).max(20.0);

        let slots = SlotKind::ALL
            .iter()
            .enumerate()
            .map(|(i, &kind)| SlotView {
                kind,
                origin: (x0 + i as f32 * (gw + SLOT_GAP), SLOT_TOP),
            })
            .collect();

        let strip_y = SLOT_TOP + gh + 82.0;
        let width = (panel_x - 48.0).max(100.0);
        let shop_h = CARD_H + 58.0;
        let shop = Rect::new(24.0, strip_y, width, shop_h);
        let inv_y = strip_y + shop_h + 14.0;
        let inv = Rect::new(24.0, inv_y, width, (LOGICAL_H - inv_y - 24.0).max(100.0));

        // Cards flow left to right, wrapping to fill the tray.
        let per_row = (((inv.w + CARD_GAP) / (CARD_W + CARD_GAP)) as usize).max(1);
        let cards = run
            .inventory()
            .into_iter()
            .enumerate()
            .map(|(i, id)| Card {
                id,
                rect: Rect::new(
                    inv.x + (i % per_row) as f32 * (CARD_W + CARD_GAP),
                    inv.y + 36.0 + (i / per_row) as f32 * (CARD_H + CARD_GAP),
                    CARD_W,
                    CARD_H,
                ),
            })
            .collect();

        let shop_cards = run
            .shop
            .stock
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                run.shop.def(i).map(|def| ShopCard {
                    slot_index: i,
                    def,
                    rect: Rect::new(
                        shop.x + 130.0 + i as f32 * (CARD_W + CARD_GAP),
                        shop.y + 34.0,
                        CARD_W,
                        CARD_H,
                    ),
                })
            })
            .collect();

        Layout { slots, cards, shop_cards, shop, inv, panel_x }
    }

    fn view(&self, kind: SlotKind) -> &SlotView {
        &self.slots[kind.index()]
    }

    /// Which slot cell, if any, is under this point.
    fn slot_hit(&self, mx: f32, my: f32) -> Option<(SlotKind, u8, u8)> {
        self.slots
            .iter()
            .find(|v| v.contains(mx, my))
            .and_then(|v| v.hit(mx, my).map(|(x, y)| (v.kind, x, y)))
    }

    fn shop_hit(&self, mx: f32, my: f32) -> Option<usize> {
        self.shop_cards
            .iter()
            .find(|c| c.rect.contains(Vec2::new(mx, my)))
            .map(|c| c.slot_index)
    }

    fn card_hit(&self, mx: f32, my: f32) -> Option<PieceId> {
        self.cards
            .iter()
            .find(|c| c.rect.contains(Vec2::new(mx, my)))
            .map(|c| c.id)
    }
}

// ============================================================= palette

fn col_bg() -> Color {
    Color::from_rgba(16, 16, 24, 255)
}
fn col_panel() -> Color {
    Color::from_rgba(22, 22, 32, 255)
}
fn col_tray() -> Color {
    Color::from_rgba(24, 24, 35, 255)
}
fn col_cell_a() -> Color {
    Color::from_rgba(36, 36, 50, 255)
}
fn col_cell_b() -> Color {
    Color::from_rgba(43, 43, 58, 255)
}
fn col_ok() -> Color {
    Color::from_rgba(90, 200, 130, 255)
}
fn col_bad() -> Color {
    Color::from_rgba(230, 95, 95, 255)
}
fn col_dim() -> Color {
    Color::from_rgba(120, 120, 140, 255)
}
fn col_gold() -> Color {
    Color::from_rgba(240, 200, 90, 255)
}
fn col_effect() -> Color {
    Color::from_rgba(105, 205, 235, 255)
}
fn col_trigger() -> Color {
    Color::from_rgba(225, 130, 225, 255)
}

/// Hue per slot, so a piece's colour says where it belongs at a glance.
fn slot_hue(slot: SlotKind) -> f32 {
    match slot {
        SlotKind::Helmet => 0.58,
        SlotKind::Chest => 0.34,
        SlotKind::Gloves => 0.79,
        SlotKind::Greaves => 0.47,
        SlotKind::Weapon => 0.03,
    }
}

/// Lightness per role, so the primary component of a recipe reads darkest.
fn kind_lightness(kind: PieceKind) -> f32 {
    match kind {
        PieceKind::Handle | PieceKind::Frame | PieceKind::Base | PieceKind::Material => 0.42,
        PieceKind::Damaging | PieceKind::Plating | PieceKind::Layer | PieceKind::Mold => 0.55,
        PieceKind::Accessory | PieceKind::Crest => 0.68,
    }
}

fn piece_color(def: &PieceDef) -> Color {
    macroquad::color::hsl_to_rgb(slot_hue(def.slot), 0.52, kind_lightness(def.kind))
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

// ========================================================= primitives

/// "Balanced Grip" -> "BG". Fits inside a 26px cell where the full name can't.
fn abbrev(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase()
}

fn draw_shape(shape: &Shape, ox: f32, oy: f32, cell: f32, color: Color, alpha: f32) {
    for &(dx, dy) in shape.cells() {
        let x = ox + dx as f32 * cell;
        let y = oy + dy as f32 * cell;
        draw_rectangle(x + 1.0, y + 1.0, cell - 2.0, cell - 2.0, with_alpha(color, alpha));
        draw_rectangle_lines(
            x + 1.0,
            y + 1.0,
            cell - 2.0,
            cell - 2.0,
            1.0,
            with_alpha(Color::from_rgba(0, 0, 0, 110), alpha),
        );
    }
}

/// Everything on screen is drawn through `text`/`text_width`, so this single
/// constant controls how large the whole interface reads.
const TEXT_SCALE: f32 = 1.34;

/// Draw text at the interface's scale.
fn ui_text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(s, x, y, size * TEXT_SCALE, color);
}

/// Width of `s` once scaled — for centring and right-aligning.
fn text_width(s: &str, size: f32) -> f32 {
    measure_text(s, None, (size * TEXT_SCALE).round().max(1.0) as u16, 1.0).width
}

/// Height one line of `size` occupies, including its leading.
fn line_h(size: f32) -> f32 {
    (size * TEXT_SCALE * 1.32).round()
}

fn centered_text(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    ui_text(s, cx - text_width(s, size) / 2.0, y, size, color);
}

fn button(rect: Rect, label: &str, enabled: bool, mx: f32, my: f32) -> bool {
    let hovered = enabled && rect.contains(Vec2::new(mx, my));
    let (bg, fg) = match (enabled, hovered) {
        (false, _) => (Color::from_rgba(38, 38, 50, 255), Color::from_rgba(95, 95, 110, 255)),
        (true, false) => (Color::from_rgba(52, 56, 82, 255), Color::from_rgba(225, 228, 245, 255)),
        (true, true) => (Color::from_rgba(84, 92, 140, 255), WHITE),
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        2.0,
        if hovered { col_gold() } else { Color::from_rgba(80, 80, 105, 255) },
    );
    centered_text(label, rect.x + rect.w / 2.0, rect.y + rect.h / 2.0 + 6.0, 18.0, fg);
    hovered
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.len() + 1 + word.len() > width {
            out.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur.push(' ');
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// ============================================================ drag state

enum Drag {
    None,
    Held {
        id: PieceId,
        /// Cursor offset from the piece's top-left, so it doesn't snap its
        /// corner to the mouse.
        grab: (f32, f32),
        /// Where it came from, so an invalid drop can put it back.
        restore: Option<(SlotKind, u8, u8)>,
    },
}

impl Drag {
    fn held_id(&self) -> Option<PieceId> {
        match self {
            Drag::Held { id, .. } => Some(*id),
            Drag::None => None,
        }
    }
}

// ============================================================== playback

/// Replays a finished `CombatLog` against wall-clock time. The fight is already
/// decided in the engine — this only decides what is on screen, so it can be
/// sped up or skipped without changing the result.
struct Playback {
    /// Wall-clock reading at the last advance, and simulated time accumulated
    /// so far. Accumulating rather than deriving from a start instant is what
    /// lets the speed change mid-fight without the clock jumping.
    last_wall: f64,
    sim_ms: u32,
    speed: f32,
    cursor: usize,
    player_hp: i32,
    player_max: i32,
    player_armor: i32,
    player_mana: i32,
    enemy_hp: i32,
    enemy_max: i32,
    enemy_armor: i32,
    /// Curses on each side, as (name, when it runs out in sim-ms).
    player_curses: Vec<(&'static str, u32)>,
    enemy_curses: Vec<(&'static str, u32)>,
    lines: Vec<String>,
    flash_player: f64,
    flash_enemy: f64,
    now_ms: u32,
    done: bool,
    /// When each item fired, indexed the same way as the combatant's item
    /// list. Cooldown bars are drawn straight from these, which is why a
    /// frost-slowed item's bar visibly crawls: the gap between two real
    /// activations *is* the slowdown, so nothing here has to know what frost
    /// does.
    player_schedule: Vec<Vec<u32>>,
    enemy_schedule: Vec<Vec<u32>>,
    /// The enemy's gear, laid out once when the fight starts.
    enemy_reg: PieceRegistry,
    enemy_loadout: Loadout,
    enemy_reports: Vec<SlotReport>,
    /// Full profiles, for the hover summaries. A monster's item list puts its
    /// innate attacks first, so its profiles start `enemy_attack_count` in.
    player_profiles: Vec<ItemProfile>,
    enemy_profiles: Vec<ItemProfile>,
    enemy_attack_count: usize,
}

/// How full an item's bar is at `now_ms`, from the times it actually fired.
fn bar_progress(schedule: &[u32], cooldown_ms: u32, now_ms: u32) -> f32 {
    let last = schedule.iter().rev().find(|&&t| t <= now_ms).copied();
    let next = schedule.iter().find(|&&t| t > now_ms).copied();
    let cd = cooldown_ms.max(1) as f32;
    match (last, next) {
        // Between two real firings: fill proportionally across the gap.
        (Some(l), Some(n)) if n > l => (now_ms - l) as f32 / (n - l) as f32,
        // Before the first firing.
        (None, Some(n)) if n > 0 => now_ms as f32 / n as f32,
        // After the last one the fight ended, so fall back to nominal speed.
        (Some(l), None) => ((now_ms - l) as f32 / cd).min(1.0),
        _ => (now_ms as f32 / cd).min(1.0),
    }
}

/// Collect every activation time per item index for one side.
fn schedule_for(log: &CombatLog, want: Side, count: usize) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::new(); count];
    for e in &log.entries {
        if let Event::Activate { side, index, .. } = &e.event {
            if *side == want {
                if let Some(slot) = out.get_mut(*index) {
                    slot.push(e.at_ms);
                }
            }
        }
    }
    out
}

impl Playback {
    fn new(log: &CombatLog, player_profiles: &[ItemProfile]) -> Self {
        let (er, eloadout) = log.spec.loadout();
        let (er2, eloadout2) = (er.clone(), eloadout.clone());
        let eprof = eloadout.combat_items(&er);
        let pprof = player_profiles.to_vec();
        Playback {
            last_wall: get_time(),
            sim_ms: 0,
            speed: DEFAULT_SPEED,
            cursor: 0,
            player_hp: log.player.health,
            player_max: log.player.max_health,
            player_armor: 0,
            player_mana: 0,
            enemy_hp: log.enemy.health,
            enemy_max: log.enemy.max_health,
            enemy_armor: 0,
            player_curses: Vec::new(),
            enemy_curses: Vec::new(),
            lines: Vec::new(),
            flash_player: -10.0,
            flash_enemy: -10.0,
            now_ms: 0,
            done: false,
            player_schedule: schedule_for(log, Side::Player, log.player.items.len()),
            enemy_schedule: schedule_for(log, Side::Enemy, log.enemy.items.len()),
            enemy_reg: er,
            enemy_reports: eloadout.reports(&er2),
            enemy_loadout: eloadout2,
            player_profiles: pprof,
            enemy_profiles: eprof,
            enemy_attack_count: log.spec.attacks.len(),
        }
    }

    fn apply(&mut self, log: &CombatLog, index: usize) {
        let entry = &log.entries[index];
        let now = get_time();
        match &entry.event {
            Event::Activate { .. } => return, // shown as a bar, not a log line
            Event::Hit { by, target_health, target_armor, .. } => match by {
                Side::Player => {
                    self.enemy_hp = (*target_health).max(0);
                    self.enemy_armor = *target_armor;
                    self.flash_enemy = now;
                }
                Side::Enemy => {
                    self.player_hp = (*target_health).max(0);
                    self.player_armor = *target_armor;
                    self.flash_player = now;
                }
            },
            Event::MindHit { by, target_max_health, .. } => match by {
                Side::Player => self.enemy_max = *target_max_health,
                Side::Enemy => {
                    self.player_max = *target_max_health;
                    self.player_hp = self.player_hp.min(self.player_max);
                    self.flash_player = now;
                }
            },
            Event::GainArmor { side, total, .. } => match side {
                Side::Player => self.player_armor = *total,
                Side::Enemy => self.enemy_armor = *total,
            },
            Event::GainMana { side, total, .. } => {
                if *side == Side::Player {
                    self.player_mana = *total;
                }
            }
            Event::ManaCheck { side, remaining, .. } => {
                if *side == Side::Player {
                    self.player_mana = *remaining;
                }
            }
            Event::Cursed { on, kind, duration_ms } => {
                let entry_pair = (kind.name(), self.now_ms + duration_ms);
                let list = match on {
                    Side::Player => &mut self.player_curses,
                    Side::Enemy => &mut self.enemy_curses,
                };
                match list.iter_mut().find(|c| c.0 == entry_pair.0) {
                    Some(existing) => existing.1 = existing.1.max(entry_pair.1),
                    None => list.push(entry_pair),
                }
            }
            Event::Burn { side, health, .. } => match side {
                Side::Player => self.player_hp = (*health).max(0),
                Side::Enemy => self.enemy_hp = (*health).max(0),
            },
            Event::Regen { side, health, .. } => {
                if *side == Side::Player {
                    self.player_hp = *health;
                }
                return; // healing ticks would drown the log
            }
            Event::Hastened { .. } => {}
            Event::Fell { .. } => {}
            Event::End { .. } => self.done = true,
        }
        self.lines.push(log.describe(entry));
    }

    /// Advance to whatever beat playback time has reached.
    fn advance(&mut self, run: &Run) {
        let Some(log) = run.log.as_ref() else { return };
        let now = get_time();
        let dt = (now - self.last_wall).max(0.0);
        self.last_wall = now;
        self.sim_ms += (dt * 1000.0 * self.speed as f64) as u32;

        while self.cursor < log.entries.len() && log.entries[self.cursor].at_ms <= self.sim_ms {
            let i = self.cursor;
            self.apply(log, i);
            self.cursor += 1;
        }
        self.now_ms = self.sim_ms.min(log.duration_ms);
        self.player_curses.retain(|c| c.1 > self.now_ms);
        self.enemy_curses.retain(|c| c.1 > self.now_ms);
    }

    /// Step through the available speeds, slowest last so it is easy to reach.
    fn cycle_speed(&mut self) {
        self.speed = match self.speed {
            s if s >= 2.0 => 1.0,
            s if s >= 1.0 => 0.5,
            s if s >= 0.5 => 0.25,
            _ => 2.0,
        };
    }

    fn skip_to_end(&mut self, run: &Run) {
        let Some(log) = run.log.as_ref() else { return };
        while self.cursor < log.entries.len() {
            let i = self.cursor;
            self.apply(log, i);
            self.cursor += 1;
        }
        self.sim_ms = log.duration_ms;
        self.now_ms = log.duration_ms;
        self.player_curses.clear();
        self.enemy_curses.clear();
    }
}

// =============================================================== render

/// Trace a border around every group in the slot: gold for a finished item,
/// muted red for one still missing something. This is what makes "two separate
/// items in one slot" legible at a glance.
fn render_item_outlines(view: &SlotView, run: &Run, report: &SlotReport) {
    let slot = run.loadout.slot(view.kind);
    for item in &report.items {
        let cells: HashSet<(u8, u8)> = item
            .pieces
            .iter()
            .flat_map(|&p| slot.cells_of(p))
            .collect();
        let color = if item.assembled {
            let p = ((get_time() * 3.0).sin() * 0.5 + 0.5) as f32;
            Color::from_rgba(215 + (40.0 * p) as u8, 180 + (30.0 * p) as u8, 80, 255)
        } else {
            Color::from_rgba(150, 70, 70, 210)
        };
        let t = if item.assembled { 3.0 } else { 2.0 };

        for &(x, y) in &cells {
            let (px, py) = view.cell_origin(x, y);
            let up = y > 0 && cells.contains(&(x, y - 1));
            let down = y + 1 < SLOT_H && cells.contains(&(x, y + 1));
            let left = x > 0 && cells.contains(&(x - 1, y));
            let right = x + 1 < SLOT_W && cells.contains(&(x + 1, y));
            if !up {
                draw_line(px, py, px + SLOT_CELL, py, t, color);
            }
            if !down {
                draw_line(px, py + SLOT_CELL, px + SLOT_CELL, py + SLOT_CELL, t, color);
            }
            if !left {
                draw_line(px, py, px, py + SLOT_CELL, t, color);
            }
            if !right {
                draw_line(px + SLOT_CELL, py, px + SLOT_CELL, py + SLOT_CELL, t, color);
            }
        }
    }
}

fn render_slots(layout: &Layout, run: &Run, reports: &[SlotReport], drag: &Drag) {
    let held = drag.held_id();

    for view in &layout.slots {
        let report = &reports[view.kind.index()];
        let any_assembled = report.assembled_count() > 0;
        let (gw, gh) = view.size();
        let (ox, oy) = view.origin;

        // Header: slot name, then the recipe one item needs.
        ui_text(view.kind.name(), ox, oy - 54.0, 20.0, WHITE);
        for (i, line) in wrap(view.kind.recipe_text(), 19).into_iter().take(3).enumerate() {
            ui_text(&line, ox, oy - 36.0 + i as f32 * 15.0, 11.0, col_dim());
        }

        // Slot border lights up once at least one item has come together.
        let border = if any_assembled {
            let p = ((get_time() * 3.0).sin() * 0.5 + 0.5) as f32;
            Color::from_rgba(200 + (55.0 * p) as u8, 170 + (40.0 * p) as u8, 70, 255)
        } else {
            Color::from_rgba(70, 70, 92, 255)
        };
        draw_rectangle(ox - 3.0, oy - 3.0, gw + 6.0, gh + 6.0, border);

        for gy in 0..SLOT_H {
            for gx in 0..SLOT_W {
                let (cx, cy) = view.cell_origin(gx, gy);
                let c = if (gx + gy) % 2 == 0 { col_cell_a() } else { col_cell_b() };
                draw_rectangle(cx, cy, SLOT_CELL, SLOT_CELL, c);
            }
        }

        // Which item each piece ended up in, so markers can reflect that item
        // rather than the slot as a whole.
        let assembled_piece = |id: PieceId| -> bool {
            report.items.iter().any(|i| i.assembled && i.pieces.contains(&id))
        };

        for id in run.loadout.slot(view.kind).pieces() {
            if Some(id) == held {
                continue; // it's on the cursor instead
            }
            let Some((ax, ay)) = run.loadout.slot(view.kind).anchor_of(id) else { continue };
            let def = run.registry.def(id);
            let shape = run.registry.shape(id);
            let (px, py) = view.cell_origin(ax, ay);
            draw_shape(&shape, px, py, SLOT_CELL, piece_color(def), 1.0);

            if let Some(&(dx, dy)) = shape.cells().first() {
                let tx = px + dx as f32 * SLOT_CELL + SLOT_CELL / 2.0;
                let ty = py + dy as f32 * SLOT_CELL + SLOT_CELL / 2.0 + 4.0;
                centered_text(&abbrev(def.name), tx, ty, 13.0, Color::from_rgba(15, 15, 20, 230));

                let live = assembled_piece(id);
                // Top-right: assembly bonus. Filled once its item is finished.
                if def.adjacency.is_some() {
                    let cx = px + dx as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + 6.0;
                    if live {
                        draw_circle(cx, cy, 4.0, col_gold());
                    } else {
                        draw_circle_lines(cx, cy, 4.0, 1.5, col_dim());
                    }
                }
                // Bottom-right: positional effect. Filled while its condition
                // currently holds.
                if let Some(eff) = def.effect {
                    let cx = px + dx as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    if eff.when.holds(live) {
                        draw_circle(cx, cy, 4.0, col_effect());
                    } else {
                        draw_circle_lines(cx, cy, 4.0, 1.5, col_dim());
                    }
                }
                if !def.triggers.is_empty() {
                    let cx = px + dx as f32 * SLOT_CELL + 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    if live {
                        draw_circle(cx, cy, 4.0, col_trigger());
                    } else {
                        draw_circle_lines(cx, cy, 4.0, 1.5, col_dim());
                    }
                }
            }
        }

        render_item_outlines(view, run, report);

        // Status line under the grid: how many items, and what they add up to.
        let color = if any_assembled {
            col_ok()
        } else if report.is_empty() {
            col_dim()
        } else {
            col_bad()
        };
        ui_text(&report.summary(), ox, oy + gh + 20.0, 14.0, color);
        let contrib = report.stats.summary();
        if !contrib.is_empty() {
            for (i, line) in wrap(&contrib, 24).into_iter().take(2).enumerate() {
                ui_text(&line, ox, oy + gh + 38.0 + i as f32 * 14.0, 12.0, col_dim());
            }
        }
    }
}

/// Where the reroll button sits inside the shop strip.
fn reroll_rect(shop: Rect) -> Rect {
    Rect::new(shop.x + 12.0, shop.y + 80.0, 104.0, 30.0)
}

/// The shelf. Clicking a card buys it if you can afford it.
fn render_shop(layout: &Layout, run: &Run, mx: f32, my: f32) {
    let r = layout.shop;
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(28, 26, 22, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(96, 84, 52, 255));

    ui_text("SHOP", r.x + 14.0, r.y + 26.0, 18.0, col_gold());
    ui_text(&format!("{} gold", run.gold), r.x + 14.0, r.y + 50.0, 20.0, WHITE);
    ui_text("click to buy", r.x + 14.0, r.y + 68.0, 12.0, col_dim());
    button(
        reroll_rect(r),
        &format!("REROLL {}g", REROLL_COST),
        run.gold >= REROLL_COST,
        mx,
        my,
    );

    for card in &layout.shop_cards {
        let def = card.def;
        let afford = run.gold >= def.price;
        let hovered = card.rect.contains(Vec2::new(mx, my));

        draw_rectangle(
            card.rect.x,
            card.rect.y,
            card.rect.w,
            card.rect.h,
            if hovered && afford {
                Color::from_rgba(52, 48, 36, 255)
            } else {
                Color::from_rgba(34, 32, 28, 255)
            },
        );
        draw_rectangle_lines(
            card.rect.x,
            card.rect.y,
            card.rect.w,
            card.rect.h,
            1.5,
            if !afford {
                Color::from_rgba(80, 60, 60, 255)
            } else if hovered {
                col_gold()
            } else {
                Color::from_rgba(88, 78, 54, 255)
            },
        );

        let shape = Shape::new(def.cells);
        let sw = shape.width() as f32 * INV_CELL;
        let sh = shape.height() as f32 * INV_CELL;
        let alpha = if afford { 1.0 } else { 0.4 };
        draw_shape(
            &shape,
            card.rect.x + (card.rect.w - sw) / 2.0,
            card.rect.y + 10.0 + (60.0 - sh) / 2.0,
            INV_CELL,
            piece_color(def),
            alpha,
        );

        let cx = card.rect.x + card.rect.w / 2.0;
        let mut ty = card.rect.y + 84.0;
        for line in wrap(def.name, 14).into_iter().take(2) {
            centered_text(&line, cx, ty, 12.0, if afford { WHITE } else { col_dim() });
            ty += 18.0;
        }
        centered_text(def.kind.name(), cx, card.rect.y + 124.0, 11.0, col_dim());
        centered_text(
            &format!("{} gold", def.price),
            cx,
            card.rect.y + card.rect.h - 8.0,
            13.0,
            if afford { col_gold() } else { col_bad() },
        );

        // Same markers as the inventory, so a triggered piece is obvious
        // before you pay for it.
        if def.adjacency.is_some() {
            draw_circle(card.rect.x + card.rect.w - 11.0, card.rect.y + 11.0, 4.0, col_gold());
        }
        if def.effect.is_some() {
            draw_circle(card.rect.x + 11.0, card.rect.y + 11.0, 4.0, col_effect());
        }
        if !def.triggers.is_empty() {
            draw_circle(card.rect.x + 11.0, card.rect.y + 24.0, 4.0, col_trigger());
        }
    }
}

fn render_inventory(layout: &Layout, run: &Run, drag: &Drag, mx: f32, my: f32) {
    draw_rectangle(layout.inv.x, layout.inv.y, layout.inv.w, layout.inv.h, col_tray());
    draw_rectangle_lines(
        layout.inv.x,
        layout.inv.y,
        layout.inv.w,
        layout.inv.h,
        2.0,
        Color::from_rgba(60, 60, 82, 255),
    );
    ui_text("INVENTORY", layout.inv.x + 14.0, layout.inv.y + 24.0, 18.0, WHITE);
    ui_text(
        "drag a component onto a slot  ·  right-click to rotate  ·  drag back here to remove",
        layout.inv.x + 128.0,
        layout.inv.y + 24.0,
        13.0,
        col_dim(),
    );

    let held = drag.held_id();
    for card in &layout.cards {
        if Some(card.id) == held {
            continue;
        }
        let def = run.registry.def(card.id);
        let shape = run.registry.shape(card.id);
        let hovered = card.rect.contains(Vec2::new(mx, my));

        draw_rectangle(
            card.rect.x,
            card.rect.y,
            card.rect.w,
            card.rect.h,
            if hovered {
                Color::from_rgba(46, 46, 64, 255)
            } else {
                Color::from_rgba(33, 33, 46, 255)
            },
        );
        draw_rectangle_lines(
            card.rect.x,
            card.rect.y,
            card.rect.w,
            card.rect.h,
            1.5,
            if hovered { col_gold() } else { Color::from_rgba(58, 58, 78, 255) },
        );

        // Shape preview, centred in the upper part of the card.
        let sw = shape.width() as f32 * INV_CELL;
        let sh = shape.height() as f32 * INV_CELL;
        draw_shape(
            &shape,
            card.rect.x + (card.rect.w - sw) / 2.0,
            card.rect.y + 12.0 + (72.0 - sh) / 2.0,
            INV_CELL,
            piece_color(def),
            1.0,
        );

        // Name (wrapped) and role.
        let cx = card.rect.x + card.rect.w / 2.0;
        let mut ty = card.rect.y + 100.0;
        for line in wrap(def.name, 14).into_iter().take(2) {
            centered_text(&line, cx, ty, 12.0, Color::from_rgba(215, 218, 235, 255));
            ty += 18.0;
        }
        centered_text(def.kind.name(), cx, card.rect.y + card.rect.h - 8.0, 11.0, col_dim());

        if def.adjacency.is_some() {
            draw_circle(card.rect.x + card.rect.w - 11.0, card.rect.y + 11.0, 4.0, col_gold());
        }
        if def.effect.is_some() {
            draw_circle(card.rect.x + 11.0, card.rect.y + 11.0, 4.0, col_effect());
        }
        if !def.triggers.is_empty() {
            draw_circle(card.rect.x + 11.0, card.rect.y + 24.0, 4.0, col_trigger());
        }
    }
}

/// Detail card for whatever the cursor is over. Drawn last so it sits on top.
///
/// Shows everything a piece does, not just its flat stats: its cooldown, any
/// speed it lends the item, its positional effect, and every trigger it fires
/// on activation. A piece whose whole behaviour lives in triggers — the Cursed
/// Blade, say — reads as blank without this.
fn render_tooltip_titled(run: &Run, id: PieceId, item_name: Option<&str>, mx: f32, my: f32) {
    render_def_tooltip_inner(run.registry.def(id), item_name, mx, my);
}

fn render_def_tooltip(def: &'static PieceDef, mx: f32, my: f32) {
    render_def_tooltip_inner(def, None, mx, my);
}

/// `item_name` is the procedurally generated name of the assembled item this
/// piece belongs to, shown above the component's own details.
fn render_def_tooltip_inner(
    def: &'static PieceDef,
    item_name: Option<&str>,
    mx: f32,
    my: f32,
) {
    let mut lines: Vec<(String, Color)> = Vec::new();
    if let Some(n) = item_name {
        lines.push((n.to_string(), col_gold()));
        lines.push(("part of".to_string(), col_dim()));
    }
    lines.push((def.name.to_string(), WHITE));
    lines.push((format!("{} · {}", def.slot.name(), def.kind.name()), col_dim()));

    let base = def.base.summary();
    if !base.is_empty() {
        lines.push((base, Color::from_rgba(190, 210, 245, 255)));
    }

    // Timing: a core sets the item's cooldown, anything else can lend speed.
    if def.kind.is_core() {
        let cd = if def.cooldown_ms == 0 {
            default_cooldown_ms(def.slot)
        } else {
            def.cooldown_ms
        };
        lines.push((
            format!("fires every {:.2}s as an item's core", cd as f32 / 1000.0),
            Color::from_rgba(200, 190, 150, 255),
        ));
    }
    if def.speed_bonus != 0 {
        lines.push((
            format!("{:+}% speed to its item", def.speed_bonus),
            Color::from_rgba(200, 190, 150, 255),
        ));
    }

    if let Some(adj) = def.adjacency {
        for (i, l) in wrap(adj.label, 46).into_iter().enumerate() {
            lines.push((
                if i == 0 { format!("when assembled: {}", l) } else { format!("  {}", l) },
                col_gold(),
            ));
        }
    }
    if let Some(eff) = def.effect {
        for l in wrap(&eff.describe(), 46) {
            lines.push((l, col_effect()));
        }
    }
    for t in def.triggers {
        for l in wrap(&t.describe(), 46) {
            lines.push((l, col_trigger()));
        }
    }
    lines.push((format!("{} gold", def.price), col_gold()));

    let w = lines
        .iter()
        .map(|(s, _)| text_width(s, 14.0))
        .fold(0.0_f32, f32::max)
        + 20.0;
    let h = lines.len() as f32 * 18.0 + 14.0;
    let x = (mx + 16.0).min(LOGICAL_W - w - 6.0);
    let y = (my + 16.0).min(LOGICAL_H - h - 6.0);

    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 244));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(110, 110, 145, 255));
    for (i, (s, c)) in lines.iter().enumerate() {
        ui_text(s, x + 10.0, y + 20.0 + i as f32 * 18.0, 14.0, *c);
    }
}

fn hp_bar(x: f32, y: f32, w: f32, h: f32, hp: i32, max: i32, color: Color) {
    draw_rectangle(x, y, w, h, Color::from_rgba(30, 30, 42, 255));
    let frac = if max > 0 { (hp as f32 / max as f32).clamp(0.0, 1.0) } else { 0.0 };
    draw_rectangle(x, y, w * frac, h, color);
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(80, 80, 105, 255));
    let label = format!("{} / {}", hp.max(0), max);
    centered_text(&label, x + w / 2.0, y + h / 2.0 + 6.0, 17.0, WHITE);
}

/// One item's cooldown bar: name, a filling track, and the interval it is
/// actually running at. Flashes on the frame it fires.
#[allow(clippy::too_many_arguments)]
fn render_cooldown_row(
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    slot: Option<SlotKind>,
    cooldown_ms: u32,
    schedule: &[u32],
    now_ms: u32,
    tint: Color,
) {
    let icon = 19.0;
    let label_w = 216.0;
    let track_x = x + label_w;
    let track_w = (w - label_w - 62.0).max(20.0);
    let h = 14.0;

    // A firing within the last fifth of a second lights the row up.
    let just_fired = schedule
        .iter()
        .rev()
        .find(|&&t| t <= now_ms)
        .map(|&t| now_ms.saturating_sub(t) < 180)
        .unwrap_or(false);

    let fg = if just_fired { WHITE } else { Color::from_rgba(178, 180, 200, 255) };
    draw_slot_icon(x, y - 1.0, icon, slot, if just_fired { WHITE } else { tint });
    ui_text(name, x + icon + 8.0, y + 12.0, 13.0, fg);

    draw_rectangle(track_x, y, track_w, h, Color::from_rgba(26, 26, 38, 255));
    let p = bar_progress(schedule, cooldown_ms, now_ms).clamp(0.0, 1.0);
    let fill = if just_fired { WHITE } else { tint };
    draw_rectangle(track_x, y, track_w * p, h, fill);
    draw_rectangle_lines(track_x, y, track_w, h, 1.0, Color::from_rgba(74, 74, 98, 255));

    // The gap it is genuinely running at right now, which drifts from the
    // nominal cooldown whenever the owner is slowed.
    let observed = {
        let last = schedule.iter().rev().find(|&&t| t <= now_ms).copied();
        let next = schedule.iter().find(|&&t| t > now_ms).copied();
        match (last, next) {
            (Some(l), Some(n)) if n > l => n - l,
            _ => cooldown_ms,
        }
    };
    let slowed = observed > cooldown_ms + 20;
    ui_text(
        &format!("{:.1}s", observed as f32 / 1000.0),
        track_x + track_w + 8.0,
        y + 12.0,
        12.0,
        if slowed { Color::from_rgba(150, 200, 255, 255) } else { col_dim() },
    );
}

/// Geometry of the battle screen. Rendering and hit-testing both go through
/// this, so the buttons are always exactly where they are drawn.
///
/// The two boards stack — yours above, theirs below — because five 6x8 grids
/// side by side capped the cells at about 22px, and the whole point of this
/// screen is being able to read both loadouts. Cooldowns move to a column
/// beside each board.
struct BattleGeom {
    board_x: f32,
    board_w: f32,
    player_board_y: f32,
    enemy_board_y: f32,
    player_bar_y: f32,
    enemy_bar_y: f32,
    cd_x: f32,
    cd_w: f32,
    log: Rect,
    buttons: [Rect; 5],
}

fn battle_geom() -> BattleGeom {
    let board_x = 24.0;
    let board_w = mini_board_width();
    let gh = SLOT_H as f32 * MINI_CELL;

    let player_board_y = 46.0;
    let player_bar_y = player_board_y + gh + 42.0;
    let enemy_board_y = player_bar_y + 100.0;
    let enemy_bar_y = enemy_board_y + gh + 42.0;

    let cd_x = board_x + board_w + 26.0;
    let cd_w = LOGICAL_W - 24.0 - cd_x;

    let log_top = enemy_bar_y + 92.0;
    let log = Rect::new(board_x, log_top, LOGICAL_W - 2.0 * board_x, 88.0);

    let w = 190.0;
    let gap = 12.0;
    let x0 = (LOGICAL_W - (5.0 * w + 4.0 * gap)) / 2.0;
    let btn_y = log_top + log.h + 16.0;
    BattleGeom {
        board_x,
        board_w,
        player_board_y,
        enemy_board_y,
        player_bar_y,
        enemy_bar_y,
        cd_x,
        cd_w,
        log,
        buttons: [
            Rect::new(x0, btn_y, w, 38.0),
            Rect::new(x0 + (w + gap), btn_y, w, 38.0),
            Rect::new(x0 + 2.0 * (w + gap), btn_y, w, 38.0),
            Rect::new(x0 + 3.0 * (w + gap), btn_y, w, 38.0),
            Rect::new(x0 + 4.0 * (w + gap), btn_y, w, 38.0),
        ],
    }
}

/// A small glyph for each slot, drawn from primitives — the default font has
/// no symbols to borrow. `None` means an innate attack: a creature's own teeth
/// rather than equipment.
fn draw_slot_icon(x: f32, y: f32, s: f32, slot: Option<SlotKind>, c: Color) {
    let t = 1.8;
    match slot {
        // A sword: blade, crossguard, grip.
        Some(SlotKind::Weapon) => {
            draw_line(x + s * 0.5, y + s * 0.04, x + s * 0.5, y + s * 0.70, t, c);
            draw_line(x + s * 0.20, y + s * 0.60, x + s * 0.80, y + s * 0.60, t, c);
            draw_line(x + s * 0.5, y + s * 0.70, x + s * 0.5, y + s * 0.94, t, c);
        }
        // A helm: domed top, brow line.
        Some(SlotKind::Helmet) => {
            draw_line(x + s * 0.16, y + s * 0.76, x + s * 0.16, y + s * 0.38, t, c);
            draw_line(x + s * 0.16, y + s * 0.38, x + s * 0.5, y + s * 0.12, t, c);
            draw_line(x + s * 0.5, y + s * 0.12, x + s * 0.84, y + s * 0.38, t, c);
            draw_line(x + s * 0.84, y + s * 0.38, x + s * 0.84, y + s * 0.76, t, c);
            draw_line(x + s * 0.28, y + s * 0.56, x + s * 0.72, y + s * 0.56, t, c);
        }
        // A breastplate: torso with shoulder pieces.
        Some(SlotKind::Chest) => {
            draw_rectangle_lines(x + s * 0.26, y + s * 0.26, s * 0.48, s * 0.62, t, c);
            draw_rectangle_lines(x + s * 0.06, y + s * 0.26, s * 0.16, s * 0.26, t, c);
            draw_rectangle_lines(x + s * 0.78, y + s * 0.26, s * 0.16, s * 0.26, t, c);
        }
        // A gauntlet: palm, three fingers, a thumb.
        Some(SlotKind::Gloves) => {
            draw_rectangle_lines(x + s * 0.30, y + s * 0.44, s * 0.44, s * 0.46, t, c);
            for i in 0..3 {
                let fx = x + s * (0.36 + i as f32 * 0.14);
                draw_line(fx, y + s * 0.44, fx, y + s * 0.16, t, c);
            }
            draw_line(x + s * 0.30, y + s * 0.56, x + s * 0.10, y + s * 0.40, t, c);
        }
        // A boot: shin above, foot below.
        Some(SlotKind::Greaves) => {
            draw_rectangle_lines(x + s * 0.32, y + s * 0.08, s * 0.30, s * 0.56, t, c);
            draw_rectangle_lines(x + s * 0.32, y + s * 0.64, s * 0.54, s * 0.26, t, c);
        }
        // A fang.
        None => {
            draw_triangle(
                Vec2::new(x + s * 0.24, y + s * 0.16),
                Vec2::new(x + s * 0.76, y + s * 0.16),
                Vec2::new(x + s * 0.5, y + s * 0.88),
                c,
            );
        }
    }
}

/// Slot order for the cooldown list: weapon first, then head to toe. Innate
/// attacks sort with weapons, being the creature's own armament.
fn slot_rank(slot: Option<SlotKind>) -> u8 {
    match slot {
        None => 0,
        Some(SlotKind::Weapon) => 1,
        Some(SlotKind::Helmet) => 2,
        Some(SlotKind::Chest) => 3,
        Some(SlotKind::Gloves) => 4,
        Some(SlotKind::Greaves) => 5,
    }
}

/// Item indices ordered for display, keeping the original index so schedules
/// and hover summaries still line up.
fn cooldown_order(items: &[gearmaster_engine::combat::RunningItem]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (slot_rank(items[i].slot), i));
    order
}

/// The clickable/hoverable band of one cooldown row.
fn cooldown_row_rect(g: &BattleGeom, top: f32, i: usize) -> Rect {
    Rect::new(g.cd_x, top + 30.0 + i as f32 * 28.0 - 5.0, g.cd_w, 26.0)
}

/// Total width of five grids side by side./// Total width of five shrunk grids side by side.
fn mini_board_width() -> f32 {
    5.0 * (SLOT_W as f32 * MINI_CELL) + 4.0 * MINI_GAP
}

/// Draw one combatant's whole gear board at reduced scale: five grids, the
/// pieces in them, and a gold outline round each finished item. Takes a
/// registry and loadout rather than a `Run`, so it can draw either side.
/// Per-piece pixel offsets, so an item can be jolted when it activates.
type Shakes = std::collections::HashMap<PieceId, (f32, f32)>;

/// Offsets for every piece whose item fired in the last [`SHAKE_MS`].
///
/// Decays to nothing over that window, so the jolt reads as an impact rather
/// than a wobble. Driven off the activation schedule, which means it stays in
/// step with the cooldown bars for free.
fn shake_offsets(profiles: &[ItemProfile], schedules: &[Vec<u32>], offset: usize, now_ms: u32) -> Shakes {
    let mut out = Shakes::new();
    for (i, profile) in profiles.iter().enumerate() {
        let Some(times) = schedules.get(i + offset) else { continue };
        let Some(&fired) = times.iter().rev().find(|&&t| t <= now_ms) else { continue };
        let since = now_ms.saturating_sub(fired);
        if since >= SHAKE_MS {
            continue;
        }
        let decay = 1.0 - since as f32 / SHAKE_MS as f32;
        let phase = since as f32 * 0.055;
        let dx = (phase).sin() * SHAKE_PX * decay;
        let dy = (phase * 1.7).cos() * SHAKE_PX * 0.55 * decay;
        for &p in &profile.pieces {
            out.insert(p, (dx, dy));
        }
    }
    out
}

fn render_mini_board(
    x0: f32,
    y0: f32,
    reg: &PieceRegistry,
    loadout: &Loadout,
    reports: &[SlotReport],
    accent: Color,
    shakes: &Shakes,
) {
    let gw = SLOT_W as f32 * MINI_CELL;
    let gh = SLOT_H as f32 * MINI_CELL;

    for (i, &kind) in SlotKind::ALL.iter().enumerate() {
        let gx = x0 + i as f32 * (gw + MINI_GAP);
        let slot = loadout.slot(kind);
        let report = &reports[kind.index()];
        let live = report.assembled_count() > 0;

        draw_rectangle(
            gx - 2.0,
            y0 - 2.0,
            gw + 4.0,
            gh + 4.0,
            if live { accent } else { Color::from_rgba(58, 58, 76, 255) },
        );
        for cy in 0..SLOT_H {
            for cx in 0..SLOT_W {
                let (px, py) = (gx + cx as f32 * MINI_CELL, y0 + cy as f32 * MINI_CELL);
                let c = if (cx + cy) % 2 == 0 { col_cell_a() } else { col_cell_b() };
                draw_rectangle(px, py, MINI_CELL, MINI_CELL, c);
            }
        }

        for id in slot.pieces() {
            let Some((ax, ay)) = slot.anchor_of(id) else { continue };
            let def = reg.def(id);
            let shape = reg.shape(id);
            let (dx, dy) = shakes.get(&id).copied().unwrap_or((0.0, 0.0));
            draw_shape(
                &shape,
                gx + ax as f32 * MINI_CELL + dx,
                y0 + ay as f32 * MINI_CELL + dy,
                MINI_CELL,
                piece_color(def),
                1.0,
            );
        }

        // Outline each finished item so the two boards read as gear, not
        // confetti.
        for item in &report.items {
            if !item.assembled {
                continue;
            }
            let cells: HashSet<(u8, u8)> =
                item.pieces.iter().flat_map(|&p| slot.cells_of(p)).collect();
            let (odx, ody) = item
                .pieces
                .first()
                .and_then(|p| shakes.get(p).copied())
                .unwrap_or((0.0, 0.0));
            for &(cx, cy) in &cells {
                let (px, py) = (
                    gx + cx as f32 * MINI_CELL + odx,
                    y0 + cy as f32 * MINI_CELL + ody,
                );
                if cy == 0 || !cells.contains(&(cx, cy - 1)) {
                    draw_line(px, py, px + MINI_CELL, py, 2.0, col_gold());
                }
                if cy + 1 >= SLOT_H || !cells.contains(&(cx, cy + 1)) {
                    draw_line(px, py + MINI_CELL, px + MINI_CELL, py + MINI_CELL, 2.0, col_gold());
                }
                if cx == 0 || !cells.contains(&(cx - 1, cy)) {
                    draw_line(px, py, px, py + MINI_CELL, 2.0, col_gold());
                }
                if cx + 1 >= SLOT_W || !cells.contains(&(cx + 1, cy)) {
                    draw_line(px + MINI_CELL, py, px + MINI_CELL, py + MINI_CELL, 2.0, col_gold());
                }
            }
        }

        let label = kind.name();
        let d_w = text_width(label, 12.0);
        ui_text(
            label,
            gx + (gw - d_w) / 2.0,
            y0 + gh + 19.0,
            12.0,
            if live { col_dim() } else { Color::from_rgba(80, 80, 96, 255) },
        );
    }
}

/// The whole battle screen, filling the window: your board above, theirs
/// below, cooldowns down the right, a quiet log strip and the controls at the
/// foot. Pressing SHOW FULL LOG overlays the complete transcript.
fn render_battle(run: &Run, pb: &Playback, log_expanded: bool, mx: f32, my: f32) {
    let Some(log) = run.log.as_ref() else { return };
    let g = battle_geom();
    let gh = SLOT_H as f32 * MINI_CELL;
    let reports = run.reports();

    // ---- your half ----
    ui_text(
        "YOUR GEAR",
        g.board_x,
        g.player_board_y - 12.0,
        18.0,
        Color::from_rgba(120, 220, 150, 255),
    );
    let player_shakes =
        shake_offsets(&pb.player_profiles, &pb.player_schedule, 0, pb.now_ms);
    render_mini_board(
        g.board_x,
        g.player_board_y,
        &run.registry,
        &run.loadout,
        &reports,
        Color::from_rgba(90, 150, 110, 255),
        &player_shakes,
    );
    render_battle_side(
        g.board_x,
        g.player_bar_y,
        g.board_w,
        &log.player.name,
        pb.player_hp,
        pb.player_max,
        pb.player_armor,
        Some(pb.player_mana),
        &pb.player_curses,
        pb.flash_player,
        Color::from_rgba(90, 190, 120, 255),
    );

    // ---- their half ----
    let enemy_label = format!("{}'s GEAR", log.enemy.name.to_uppercase());
    ui_text(
        &enemy_label,
        g.board_x,
        g.enemy_board_y - 12.0,
        18.0,
        Color::from_rgba(230, 140, 120, 255),
    );
    if pb.enemy_loadout.slots.iter().all(|s| s.is_empty()) {
        draw_rectangle_lines(
            g.board_x,
            g.enemy_board_y,
            g.board_w,
            gh,
            2.0,
            Color::from_rgba(70, 54, 54, 255),
        );
        centered_text(
            "no gear - it just has teeth",
            g.board_x + g.board_w / 2.0,
            g.enemy_board_y + gh / 2.0,
            18.0,
            col_dim(),
        );
    } else {
        let enemy_shakes = shake_offsets(
            &pb.enemy_profiles,
            &pb.enemy_schedule,
            pb.enemy_attack_count,
            pb.now_ms,
        );
        render_mini_board(
            g.board_x,
            g.enemy_board_y,
            &pb.enemy_reg,
            &pb.enemy_loadout,
            &pb.enemy_reports,
            Color::from_rgba(150, 90, 80, 255),
            &enemy_shakes,
        );
    }
    render_battle_side(
        g.board_x,
        g.enemy_bar_y,
        g.board_w,
        &log.enemy.name,
        pb.enemy_hp,
        pb.enemy_max,
        pb.enemy_armor,
        None,
        &pb.enemy_curses,
        pb.flash_enemy,
        Color::from_rgba(210, 110, 90, 255),
    );

    // ---- right column: clock, then each side's cooldowns beside its board ----
    let cx = g.cd_x + g.cd_w / 2.0;
    centered_text(
        &format!("{:.1}s", pb.now_ms as f32 / 1000.0),
        cx,
        g.player_board_y - 12.0,
        22.0,
        col_gold(),
    );
    let rung = format!("rung {} of {}", run.rung.min(LADDER.len() - 1) + 1, LADDER.len());
    let d_w = text_width(&rung, 14.0);
    ui_text(&rung, g.cd_x + g.cd_w - d_w, g.player_board_y - 12.0, 14.0, col_dim());

    for (label, items, sched, top, tint) in [
        (
            "YOUR COOLDOWNS",
            &log.player.items,
            &pb.player_schedule,
            g.player_board_y,
            Color::from_rgba(90, 190, 120, 255),
        ),
        (
            "THEIR COOLDOWNS",
            &log.enemy.items,
            &pb.enemy_schedule,
            g.enemy_board_y,
            Color::from_rgba(210, 110, 90, 255),
        ),
    ] {
        ui_text(label, g.cd_x, top + 14.0, 13.0, col_dim());
        for (row, &i) in cooldown_order(items).iter().enumerate() {
            let it = &items[i];
            if cooldown_row_rect(&g, top, row).contains(Vec2::new(mx, my)) {
                draw_rectangle(
                    g.cd_x - 6.0,
                    top + 25.0 + row as f32 * 28.0,
                    g.cd_w + 12.0,
                    26.0,
                    Color::from_rgba(255, 255, 255, 14),
                );
            }
            render_cooldown_row(
                g.cd_x,
                top + 30.0 + row as f32 * 28.0,
                g.cd_w,
                &it.name,
                it.slot,
                it.cooldown_ms,
                sched.get(i).map(|v| v.as_slice()).unwrap_or(&[]),
                pb.now_ms,
                tint,
            );
        }
    }

    // ---- the quiet log strip ----
    let r = g.log;
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(20, 20, 30, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, Color::from_rgba(56, 56, 76, 255));
    let lh = line_h(14.0);
    let visible = (((r.h - 12.0) / lh) as usize).max(1);
    let start = pb.lines.len().saturating_sub(visible);
    for (i, line) in pb.lines[start..].iter().enumerate() {
        let is_last = start + i == pb.lines.len() - 1;
        ui_text(
            line,
            r.x + 14.0,
            r.y + lh + i as f32 * lh,
            14.0,
            if is_last { WHITE } else { Color::from_rgba(128, 130, 150, 255) },
        );
    }

    if pb.done {
        let (label, color) = match log.outcome {
            Outcome::Victory => ("VICTORY", col_ok()),
            Outcome::Defeat => ("DEFEAT", col_bad()),
            Outcome::Stalemate => ("STALEMATE", col_gold()),
        };
        let (bw, bh) = (320.0, 78.0);
        let bx = g.cd_x + (g.cd_w - bw) / 2.0;
        let by = g.enemy_bar_y + 34.0;
        draw_rectangle(bx, by, bw, bh, Color::from_rgba(18, 18, 28, 250));
        draw_rectangle_lines(bx, by, bw, bh, 3.0, color);
        centered_text(label, bx + bw / 2.0, by + 52.0, 36.0, color);
    }

    let btn = g.buttons;
    button(btn[0], "BACK TO GEAR", true, mx, my);
    button(btn[1], "SKIP", !pb.done, mx, my);
    button(btn[2], "REMATCH", true, mx, my);
    button(btn[3], if log_expanded { "HIDE LOG" } else { "FULL LOG" }, true, mx, my);
    button(btn[4], &format!("SPEED {}x", speed_label(pb.speed)), true, mx, my);

    // The full transcript is an overlay, so it never pushes the boards around.
    if log_expanded {
        render_log_overlay(pb);
    } else {
        // Hovering a cooldown row explains what that item is worth.
        for (items, top, profiles, offset) in [
            (&log.player.items, g.player_board_y, &pb.player_profiles, 0usize),
            (&log.enemy.items, g.enemy_board_y, &pb.enemy_profiles, pb.enemy_attack_count),
        ] {
            for (row, &i) in cooldown_order(items).iter().enumerate() {
                if !cooldown_row_rect(&g, top, row).contains(Vec2::new(mx, my)) {
                    continue;
                }
                match i.checked_sub(offset).and_then(|j| profiles.get(j)) {
                    Some(profile) => render_item_summary(profile, run, mx, my),
                    // An innate attack has no gear behind it.
                    None => render_innate_summary(&items[i], mx, my),
                }
                return;
            }
        }
    }
}

/// Everything one assembled item is worth: what it adds to you all the time,
/// and what it does each time its cooldown comes round.
fn render_item_summary(p: &ItemProfile, run: &Run, mx: f32, my: f32) {
    let total = run.player_stats();
    let st = p.stats;
    let mut lines: Vec<(String, Color)> = vec![
        (p.full_name.clone(), col_gold()),
        (
            format!("{} · built on a {}", p.slot.name(), p.core),
            col_dim(),
        ),
    ];

    // Passive half: what it contributes whether or not a fight is happening.
    let mut passive = Vec::new();
    if st.health != 0 {
        passive.push(format!("{:+} max health", st.health));
    }
    if st.strength != 0 {
        passive.push(format!("{:+} strength", st.strength));
    }
    if st.power != 0 {
        passive.push(format!("{:+}.{:02}x weapon power", st.power / 100, st.power.abs() % 100));
    }
    if st.regen != 0 {
        passive.push(format!("{:+} regen a second", st.regen));
    }
    if st.mind_resist != 0 {
        passive.push(format!("{:+}% mind resist", st.mind_resist));
    }
    if st.curse_resist != 0 {
        passive.push(format!("{:+}% curse resist", st.curse_resist));
    }
    lines.push(("OUT OF COMBAT".to_string(), Color::from_rgba(150, 200, 240, 255)));
    if passive.is_empty() {
        lines.push(("  nothing - it only acts in a fight".to_string(), col_dim()));
    } else {
        for l in passive {
            lines.push((format!("  {}", l), Color::from_rgba(200, 216, 240, 255)));
        }
    }

    // Active half: one activation.
    lines.push((
        format!("IN COMBAT - every {:.2}s", p.cooldown_ms as f32 / 1000.0),
        Color::from_rgba(240, 190, 140, 255),
    ));
    let hit = p.hit_for(total.strength, total.power);
    if hit > 0 {
        let dps = p.dps_milli(total.strength, total.power);
        lines.push((
            format!("  hits for {}  ({}.{} a second)", hit, dps / 1000, (dps % 1000) / 100),
            Color::from_rgba(240, 210, 190, 255),
        ));
    }
    if st.mind > 0 {
        lines.push((format!("  {} mind damage", st.mind), Color::from_rgba(240, 210, 190, 255)));
    }
    if st.armor > 0 {
        lines.push((format!("  {} armor", st.armor), Color::from_rgba(240, 210, 190, 255)));
    }
    if st.mana > 0 {
        lines.push((format!("  {} mana", st.mana), Color::from_rgba(240, 210, 190, 255)));
    }
    for t in &p.triggers {
        for l in wrap(&t.describe(), 52) {
            lines.push((format!("  {}", l), col_trigger()));
        }
    }
    if hit == 0 && st.mind == 0 && st.armor == 0 && st.mana == 0 && p.triggers.is_empty() {
        lines.push(("  ticks over doing nothing".to_string(), col_dim()));
    }

    draw_tooltip(&lines, mx, my);
}

/// A monster's innate attack, which has no components behind it.
fn render_innate_summary(it: &gearmaster_engine::combat::RunningItem, mx: f32, my: f32) {
    let mut lines: Vec<(String, Color)> = vec![
        (it.name.clone(), col_gold()),
        ("innate - not gear".to_string(), col_dim()),
        (
            format!("IN COMBAT - every {:.2}s", it.cooldown_ms as f32 / 1000.0),
            Color::from_rgba(240, 190, 140, 255),
        ),
    ];
    if it.damage > 0 {
        lines.push((format!("  {} damage", it.damage), Color::from_rgba(240, 210, 190, 255)));
    }
    if it.mind > 0 {
        lines.push((format!("  {} mind damage", it.mind), Color::from_rgba(240, 210, 190, 255)));
    }
    if it.armor > 0 {
        lines.push((format!("  {} armor", it.armor), Color::from_rgba(240, 210, 190, 255)));
    }
    if let Some(c) = it.curse {
        lines.push((format!("  applies curse of {}", c.name()), col_trigger()));
    }
    draw_tooltip(&lines, mx, my);
}

/// Shared tooltip frame: sized to its content and kept on screen.
fn draw_tooltip(lines: &[(String, Color)], mx: f32, my: f32) {
    let w = lines
        .iter()
        .map(|(s, _)| text_width(s, 14.0))
        .fold(0.0_f32, f32::max)
        + 26.0;
    let lh = line_h(14.0);
    let h = lines.len() as f32 * lh + 18.0;
    let x = (mx + 18.0).min(LOGICAL_W - w - 6.0).max(4.0);
    let y = (my + 18.0).min(LOGICAL_H - h - 6.0).max(4.0);
    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 248));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(120, 120, 155, 255));
    for (i, (s, c)) in lines.iter().enumerate() {
        ui_text(s, x + 13.0, y + lh + i as f32 * lh, 14.0, *c);
    }
}

/// "0.25", "0.5", "1", "2" — trimmed so the button reads cleanly.
fn speed_label(speed: f32) -> String {
    if (speed - speed.round()).abs() < 0.01 {
        format!("{}", speed.round() as i32)
    } else {
        format!("{}", speed).trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// The complete combat transcript, over the top of everything else.
fn render_log_overlay(pb: &Playback) {
    let pad = 90.0;
    let r = Rect::new(pad, pad, LOGICAL_W - 2.0 * pad, LOGICAL_H - 2.0 * pad - 60.0);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 215));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(110, 110, 145, 255));
    ui_text("COMBAT LOG", r.x + 16.0, r.y + 28.0, 18.0, col_gold());

    let lh = line_h(14.0);
    let visible = (((r.h - 62.0) / lh) as usize).max(1);
    let start = pb.lines.len().saturating_sub(visible);
    for (i, line) in pb.lines[start..].iter().enumerate() {
        ui_text(
            line,
            r.x + 16.0,
            r.y + 62.0 + i as f32 * lh,
            14.0,
            Color::from_rgba(196, 198, 216, 255),
        );
    }
}

/// Name, health, armour, mana and curses for one side of the battle screen.
#[allow(clippy::too_many_arguments)]
fn render_battle_side(
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    hp: i32,
    max: i32,
    armor: i32,
    mana: Option<i32>,
    curses: &[(&'static str, u32)],
    flash: f64,
    tint: Color,
) {
    let flashing = get_time() - flash < FLASH_SECS;
    ui_text(
        name,
        x,
        y - 6.0,
        18.0,
        if hp <= 0 {
            col_bad()
        } else if flashing {
            Color::from_rgba(255, 190, 190, 255)
        } else {
            WHITE
        },
    );
    hp_bar(x, y, w, 30.0, hp, max, tint);

    draw_rectangle(x, y + 34.0, w, 14.0, Color::from_rgba(30, 30, 42, 255));
    if armor > 0 {
        let frac = ((armor as f32) / (max.max(1) as f32)).clamp(0.0, 1.0);
        draw_rectangle(x, y + 34.0, w * frac, 14.0, Color::from_rgba(150, 170, 210, 255));
    }
    draw_rectangle_lines(x, y + 34.0, w, 14.0, 1.0, Color::from_rgba(80, 80, 105, 255));

    let mut label = format!("armor {}", armor);
    if let Some(m) = mana {
        label.push_str(&format!("   mana {}", m));
    }
    ui_text(&label, x, y + 68.0, 13.0, Color::from_rgba(160, 190, 225, 255));

    let mut cx = x + 210.0;
    for (kind, _) in curses {
        let text = format!("curse of {}", kind);
        let d_w = text_width(&text, 12.0);
        draw_rectangle(cx - 5.0, y + 52.0, d_w + 12.0, 22.0, Color::from_rgba(90, 40, 90, 230));
        ui_text(&text, cx + 1.0, y + 68.0, 12.0, Color::from_rgba(240, 190, 240, 255));
        cx += d_w + 18.0;
    }
    if hp <= 0 {
        ui_text("DOWN", x + w - 52.0, y - 6.0, 18.0, col_bad());
    }
}

fn button_rects(panel_x: f32) -> [Rect; 4] {
    let w = PANEL_W - 40.0;
    let x = panel_x + 20.0;
    let y = LOGICAL_H - 232.0;
    [
        Rect::new(x, y, w, 46.0),
        Rect::new(x, y + 56.0, w / 2.0 - 5.0, 40.0),
        Rect::new(x + w / 2.0 + 5.0, y + 56.0, w / 2.0 - 5.0, 40.0),
        Rect::new(x, y + 106.0, w, 40.0),
    ]
}

fn render_panel(
    layout: &Layout,
    run: &Run,
    reports: &[SlotReport],
    message: &str,
    pb: &Option<Playback>,
    mx: f32,
    my: f32,
) {
    let x = layout.panel_x;
    draw_rectangle(x, 0.0, PANEL_W, LOGICAL_H, col_panel());
    draw_line(x, 0.0, x, LOGICAL_H, 2.0, Color::from_rgba(60, 60, 85, 255));

    let mut y = 38.0;
    ui_text("GEAR MASTER", x + 20.0, y, 26.0, WHITE);
    y += 30.0;

    let stats = run.player_stats();
    ui_text("YOUR CHARACTER", x + 20.0, y, 14.0, col_dim());
    y += 22.0;
    for (label, value, color) in [
        ("Health", format!("{}", stats.health), Color::from_rgba(120, 220, 150, 255)),
        ("Strength", format!("{}", stats.strength), Color::from_rgba(240, 170, 120, 255)),
        ("Regen", format!("{}/turn", stats.regen), Color::from_rgba(140, 200, 240, 255)),
        (
            "Weapon power",
            format!("{}.{:02}x", stats.power / 100, stats.power % 100),
            col_gold(),
        ),
    ] {
        ui_text(label, x + 20.0, y, 16.0, LIGHTGRAY);
        let d_w = text_width(&value, 16.0);
        ui_text(&value, x + PANEL_W - 20.0 - d_w, y, 16.0, color);
        y += 21.0;
    }
    y += 4.0;
    // Damage is per item now, so a single "damage per attack" figure would
    // lie. Total damage a second across every weapon is the honest summary.
    let items = run.combat_items();
    let dps_milli: i64 = items
        .iter()
        .map(|i| i.dps_milli(stats.strength, stats.power))
        .sum();
    ui_text("Damage / second", x + 20.0, y, 17.0, WHITE);
    let label = format!("{}.{}", dps_milli / 1000, (dps_milli % 1000) / 100);
    let d_w = text_width(&label, 19.0);
    ui_text(&label, x + PANEL_W - 20.0 - d_w, y, 19.0, col_gold());
    y += 16.0;
    for it in items.iter().filter(|i| i.hit_for(stats.strength, stats.power) > 0).take(3) {
        ui_text(
            &format!(
                "  {} hits {} every {:.2}s",
                it.name,
                it.hit_for(stats.strength, stats.power),
                it.cooldown_ms as f32 / 1000.0
            ),
            x + 20.0,
            y,
            12.0,
            col_dim(),
        );
        y += 14.0;
    }
    y += 14.0;

    // Per-slot assembly readout.
    ui_text("GEAR", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    for r in reports {
        let done = r.assembled_count();
        let (mark, color) = if done > 0 { ("+", col_ok()) } else { ("-", col_dim()) };
        ui_text(mark, x + 20.0, y, 16.0, color);
        ui_text(r.slot.name(), x + 36.0, y, 16.0, if done > 0 { WHITE } else { col_dim() });
        let status = r.summary();
        let d_w = text_width(&status, 13.0);
        ui_text(
            &status,
            x + PANEL_W - 20.0 - d_w,
            y,
            13.0,
            if done > 0 { col_ok() } else if r.is_empty() { col_dim() } else { col_bad() },
        );
        y += 18.0;
        for note in r.notes() {
            // Assembly bonuses read gold, positional effects read blue.
            let c = if note.contains(':') && !note.contains(" from ") && !note.contains("doubled")
            {
                col_gold()
            } else {
                col_effect()
            };
            for line in wrap(&note, 44).into_iter().take(2) {
                ui_text(&format!("  {}", line), x + 36.0, y, 12.0, c);
                y += 14.0;
            }
        }
    }
    y += 12.0;

    ui_text("RUN", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    for (label, value, color) in [
        ("Gold", format!("{}", run.gold), col_gold()),
        ("Won", format!("{}", run.wins), col_ok()),
        ("Lost", format!("{}", run.losses), col_bad()),
    ] {
        ui_text(label, x + 20.0, y, 15.0, LIGHTGRAY);
        let d_w = text_width(&value, 15.0);
        ui_text(&value, x + PANEL_W - 20.0 - d_w, y, 15.0, color);
        y += 18.0;
    }
    y += 10.0;

    let m = run.monster();
    ui_text("NEXT OPPONENT", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    ui_text(m.name, x + 20.0, y, 17.0, Color::from_rgba(230, 140, 120, 255));
    let bounty = format!("{}g", m.bounty);
    let d_w = text_width(&bounty, 15.0);
    ui_text(&bounty, x + PANEL_W - 20.0 - d_w, y, 15.0, col_gold());
    y += 18.0;
    ui_text(
        &format!("rung {} of {}  ·  {} hp", run.rung + 1, LADDER.len(), m.health),
        x + 20.0,
        y,
        13.0,
        col_dim(),
    );
    y += 16.0;
    for a in m.attacks {
        let mut what = String::new();
        if a.damage > 0 {
            what.push_str(&format!("{} dmg ", a.damage));
        }
        if a.mind > 0 {
            what.push_str(&format!("{} mind ", a.mind));
        }
        if a.armor > 0 {
            what.push_str(&format!("{} armor ", a.armor));
        }
        if let Some(c) = a.curse {
            what.push_str(&format!("+{} ", c.name()));
        }
        ui_text(
            &format!("  {} / {:.1}s  {}", a.name, a.cooldown_ms as f32 / 1000.0, what),
            x + 20.0,
            y,
            12.0,
            col_dim(),
        );
        y += 14.0;
    }
    if m.mind_resist > 0 || m.curse_resist > 0 {
        ui_text(
            &format!("  resists: {}% mind, {}% curse", m.mind_resist, m.curse_resist),
            x + 20.0,
            y,
            12.0,
            Color::from_rgba(190, 160, 200, 255),
        );
        y += 14.0;
    }
    y += 12.0;

    for line in wrap(message, 40).into_iter().take(3) {
        ui_text(&line, x + 20.0, y, 14.0, Color::from_rgba(225, 225, 240, 255));
        y += 17.0;
    }

    // Buttons
    let r = button_rects(layout.panel_x);
    let fighting = run.phase == Phase::Fighting;
    if fighting {
        let done = pb.as_ref().map(|p| p.done).unwrap_or(false);
        button(r[0], "BACK TO GEAR", true, mx, my);
        button(r[1], "SKIP", !done, mx, my);
        button(r[2], "REMATCH", true, mx, my);
    } else {
        button(r[0], "BEGIN FIGHT", true, mx, my);
        button(r[1], "AUTO-BUILD", true, mx, my);
        button(r[2], "CLEAR ALL", true, mx, my);
    }
    button(r[3], "F12  save screenshot", true, mx, my);
}

// ================================================================= main

#[macroquad::main(window_conf)]
async fn main() {
    let mut run = Run::new();
    let mut drag = Drag::None;
    let mut pb: Option<Playback> = None;
    // Whether the current fight's reward has been banked yet.
    let mut settled = false;
    // The combat log is a quiet strip unless you ask to see all of it.
    let mut log_expanded = std::env::var("GEARMASTER_LOG").is_ok();
    let mut message =
        String::from("Drag components into a slot. Pieces must touch to become gear.");

    // Debug hooks so this window can be inspected without a human at the
    // keyboard: GEARMASTER_PRESET=1 starts geared up, GEARMASTER_FIGHT=1 opens
    // mid-bout, and GEARMASTER_SHOT=<path> captures a frame and exits.
    if std::env::var("GEARMASTER_PRESET").is_ok() {
        run.apply_preset();
        message = "Auto-built a complete loadout - every bonus is lit.".to_string();
    }
    if let Ok(r) = std::env::var("GEARMASTER_RUNG") {
        if let Ok(n) = r.parse::<usize>() {
            run.rung = n;
        }
    }
    if std::env::var("GEARMASTER_FIGHT").is_ok() {
        pb = Some({
                    let profiles = run.combat_items();
                    Playback::new(run.fight_next(), &profiles)
                });
        message = "Fight in progress.".to_string();
    }
    // Screenshot capture is a desktop-only debugging aid: the browser build has
    // no filesystem to write to.
    #[cfg(not(target_arch = "wasm32"))]
    let shot_path = std::env::var("GEARMASTER_SHOT").ok();
    #[cfg(not(target_arch = "wasm32"))]
    let shot_after: u32 = std::env::var("GEARMASTER_SHOT_FRAME")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);
    #[cfg(not(target_arch = "wasm32"))]
    let mut frame: u32 = 0;

    loop {
        // Clear the whole window (letterbox bars included), then switch into
        // logical space for everything that follows.
        set_default_camera();
        clear_background(Color::from_rgba(8, 8, 12, 255));
        let viewport = Viewport::current();
        set_camera(&viewport.camera());
        clear_background(col_bg());

        let (mx, my) = viewport.mouse();
        let layout = Layout::build(&run);
        let reports = run.reports();

        if let Some(p) = pb.as_mut() {
            p.advance(&run);
            if p.done && !settled {
                settled = true;
                message = match run.settle() {
                    Some(g) => format!("+{} gold. Next up: {}.", g, run.monster().name),
                    None => format!("No reward. {} still stands.", run.monster().name),
                };
            }
        }

        // ---------------------------------------------------- render
        if run.phase == Phase::Fighting {
            if let Some(p) = pb.as_ref() {
                render_battle(&run, p, log_expanded, mx, my);
            }
        } else {
            render_slots(&layout, &run, &reports, &drag);
            render_shop(&layout, &run, mx, my);
            render_inventory(&layout, &run, &drag, mx, my);
        }

        // Drag ghost + placement preview.
        if let Drag::Held { id, grab, .. } = &drag {
            let def = run.registry.def(*id);
            let shape = run.registry.shape(*id);
            let gx = mx - grab.0;
            let gy = my - grab.1;

            if let Some((kind, ax, ay)) =
                layout.slot_hit(gx + SLOT_CELL * 0.5, gy + SLOT_CELL * 0.5)
            {
                let ok = run.can_equip(*id, kind, ax, ay).is_ok();
                let view = layout.view(kind);
                let tint = if ok { col_ok() } else { col_bad() };
                // Show the footprint the drop would claim, clipped to the grid.
                for &(dx, dy) in shape.cells() {
                    let (cx, cy) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
                    if (0..SLOT_W as i32).contains(&cx) && (0..SLOT_H as i32).contains(&cy) {
                        let (px, py) = view.cell_origin(cx as u8, cy as u8);
                        draw_rectangle(px, py, SLOT_CELL, SLOT_CELL, with_alpha(tint, 0.38));
                    }
                }
            }
            draw_shape(&shape, gx, gy, SLOT_CELL, piece_color(def), 0.92);
        }

        if run.phase != Phase::Fighting {
            render_panel(&layout, &run, &reports, &message, &pb, mx, my);
        }

        // Tooltip for whatever is under the cursor (never while dragging).
        if matches!(drag, Drag::None) {
            let hovered_item_name = layout.slot_hit(mx, my).and_then(|(k, x, y)| {
                let id = run.loadout.slot(k).get(x, y)?;
                run.report(k)
                    .items
                    .into_iter()
                    .find(|i| i.assembled && i.pieces.contains(&id))
                    .map(|i| i.name.full)
            });
            let hovered = layout
                .slot_hit(mx, my)
                .and_then(|(k, x, y)| run.loadout.slot(k).get(x, y))
                .or_else(|| {
                    if run.phase == Phase::Loadout {
                        layout.card_hit(mx, my)
                    } else {
                        None
                    }
                });
            if let Some(id) = hovered {
                render_tooltip_titled(&run, id, hovered_item_name.as_deref(), mx, my);
            } else if run.phase == Phase::Loadout {
                if let Some(i) = layout.shop_hit(mx, my) {
                    if let Some(def) = run.shop.def(i) {
                        render_def_tooltip(def, mx, my);
                    }
                }
            }
        }

        // ----------------------------------------------------- input
        let rects = button_rects(layout.panel_x);
        let clicked_button = |i: usize| {
            is_mouse_button_pressed(MouseButton::Left) && rects[i].contains(Vec2::new(mx, my))
        };

        if run.phase == Phase::Fighting {
            let br = battle_geom().buttons;
            let hit = |i: usize| {
                is_mouse_button_pressed(MouseButton::Left)
                    && br[i].contains(Vec2::new(mx, my))
            };
            if hit(0) {
                run.back_to_loadout();
                pb = None;
                log_expanded = false;
                message = "Rearrange your gear and fight again.".to_string();
            } else if hit(1) {
                if let Some(p) = pb.as_mut() {
                    p.skip_to_end(&run);
                }
            } else if hit(2) {
                pb = Some({
                    let profiles = run.combat_items();
                    Playback::new(run.fight_next(), &profiles)
                });
                settled = false;
            } else if hit(3) {
                log_expanded = !log_expanded;
            } else if hit(4) {
                if let Some(p) = pb.as_mut() {
                    p.cycle_speed();
                    message = format!("Playback at {}x.", speed_label(p.speed));
                }
            }
        } else {
            if clicked_button(0) {
                pb = Some({
                    let profiles = run.combat_items();
                    Playback::new(run.fight_next(), &profiles)
                });
                settled = false;
                message = "Fight in progress.".to_string();
            } else if clicked_button(1) {
                run.apply_preset();
                drag = Drag::None;
                message = "Auto-built a complete loadout - every bonus is lit.".to_string();
            } else if clicked_button(2) {
                run.clear_all();
                drag = Drag::None;
                message = "Cleared. Every slot is empty again.".to_string();
            }
        }

        // Buying, checked before the drag handler so clicking a shelf never
        // picks a piece up.
        let mut bought_this_frame = false;
        if run.phase == Phase::Loadout && is_mouse_button_pressed(MouseButton::Left) {
            if reroll_rect(layout.shop).contains(Vec2::new(mx, my)) {
                bought_this_frame = true;
                match run.reroll() {
                    Ok(()) => message = format!("Rerolled. {} gold left.", run.gold),
                    Err(e) => message = format!("{}", e),
                }
            } else if let Some(i) = layout.shop_hit(mx, my) {
                bought_this_frame = true;
                let name = run.shop.def(i).map(|d| d.name).unwrap_or("?");
                match run.buy(i) {
                    Ok(_) => message = format!("Bought {}. {} gold left.", name, run.gold),
                    Err(e) => message = format!("{}", e),
                }
            }
        }

        // Drag and drop is only live while arranging gear.
        if run.phase == Phase::Loadout {
            let over_button =
                bought_this_frame || rects.iter().any(|r| r.contains(Vec2::new(mx, my)));

            // --- pick up ---
            if is_mouse_button_pressed(MouseButton::Left)
                && matches!(drag, Drag::None)
                && !over_button
            {
                if let Some((kind, gx, gy)) = layout.slot_hit(mx, my) {
                    if let Some(id) = run.loadout.slot(kind).get(gx, gy) {
                        let anchor = run
                            .loadout
                            .slot(kind)
                            .anchor_of(id)
                            .expect("a placed piece has an anchor");
                        let (ox, oy) = layout.view(kind).cell_origin(anchor.0, anchor.1);
                        // Lift it out now, so the piece can't collide with its
                        // own old footprint and a rotation mid-drag is free.
                        let _ = run.unequip(id);
                        drag = Drag::Held {
                            id,
                            grab: (mx - ox, my - oy),
                            restore: Some((kind, anchor.0, anchor.1)),
                        };
                    }
                } else if let Some(id) = layout.card_hit(mx, my) {
                    let shape = run.registry.shape(id);
                    drag = Drag::Held {
                        id,
                        // Centre an inventory piece on the cursor: it was drawn
                        // at a smaller scale, so there is no grab point to keep.
                        grab: (
                            shape.width() as f32 * SLOT_CELL / 2.0,
                            shape.height() as f32 * SLOT_CELL / 2.0,
                        ),
                        restore: None,
                    };
                }
            }

            // --- rotate (right-click, held or in place) ---
            if is_mouse_button_pressed(MouseButton::Right) {
                let target = drag.held_id().or_else(|| {
                    layout
                        .slot_hit(mx, my)
                        .and_then(|(k, x, y)| run.loadout.slot(k).get(x, y))
                        .or_else(|| layout.card_hit(mx, my))
                });
                if let Some(id) = target {
                    match run.rotate(id) {
                        Ok(()) => {}
                        Err(e) => message = format!("Can't rotate there - {}", e),
                    }
                }
            }

            // --- drop ---
            if is_mouse_button_released(MouseButton::Left) {
                if let Drag::Held { id, grab, restore } = drag {
                    let gx = mx - grab.0 + SLOT_CELL * 0.5;
                    let gy = my - grab.1 + SLOT_CELL * 0.5;

                    let placed = match layout.slot_hit(gx, gy) {
                        Some((kind, ax, ay)) => match run.equip(id, kind, ax, ay) {
                            Ok(()) => {
                                let r = run.report(kind);
                                message = format!(
                                    "{}: {}  {}",
                                    kind.name(),
                                    r.summary(),
                                    r.stats.summary()
                                );
                                true
                            }
                            Err(e) => {
                                message = format!("{}", e);
                                false
                            }
                        },
                        None => false,
                    };

                    if !placed {
                        // Dropped on the tray? Then leaving it unequipped IS
                        // the intent. Anywhere else, put it back where it was.
                        let on_tray = layout.inv.contains(Vec2::new(mx, my));
                        if on_tray {
                            message = format!("{} returned to inventory.", run.registry.def(id).name);
                        } else if let Some((kind, ax, ay)) = restore {
                            let _ = run.equip(id, kind, ax, ay);
                        }
                    }
                    drag = Drag::None;
                }
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            if let Drag::Held { id, restore, .. } = drag {
                if let Some((kind, ax, ay)) = restore {
                    let _ = run.equip(id, kind, ax, ay);
                }
                drag = Drag::None;
                message = "Cancelled.".to_string();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if is_key_pressed(KeyCode::F12) {
            let path = format!("/tmp/gearmaster-{}.png", (get_time() * 1000.0) as u64);
            get_screen_data().export_png(&path);
            println!("screenshot: {}", path);
            message = format!("Saved {}", path);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            frame += 1;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &shot_path {
            if frame >= shot_after {
                get_screen_data().export_png(path);
                println!("screenshot: {}", path);
                return;
            }
        }

        next_frame().await;
    }
}

// ================================================================ tests
//
// The coordinate maths below is the load-bearing part of drag and drop: if
// `hit` and `cell_origin` ever disagree, pieces land one cell away from where
// they were dropped. None of these touch the GPU, so they run headlessly.

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> SlotView {
        SlotView { kind: SlotKind::Weapon, origin: (137.5, 112.0) }
    }

    #[test]
    fn every_cell_round_trips_from_grid_to_pixels_and_back() {
        let v = view();
        for gy in 0..SLOT_H {
            for gx in 0..SLOT_W {
                let (px, py) = v.cell_origin(gx, gy);
                // Anywhere inside the cell must resolve back to that cell.
                for (ox, oy) in [(0.5, 0.5), (1.0, 1.0), (SLOT_CELL - 1.0, SLOT_CELL - 1.0)] {
                    assert_eq!(
                        v.hit(px + ox, py + oy),
                        Some((gx, gy)),
                        "cell ({}, {}) offset ({}, {})",
                        gx,
                        gy,
                        ox,
                        oy
                    );
                }
            }
        }
    }

    #[test]
    fn points_outside_the_grid_do_not_hit_a_cell() {
        let v = view();
        let (ox, oy) = v.origin;
        assert_eq!(v.hit(ox - 1.0, oy + 5.0), None, "left of the grid");
        assert_eq!(v.hit(ox + 5.0, oy - 1.0), None, "above the grid");
        assert_eq!(v.hit(ox + SLOT_W as f32 * SLOT_CELL + 1.0, oy + 5.0), None, "right");
        assert_eq!(v.hit(ox + 5.0, oy + SLOT_H as f32 * SLOT_CELL + 1.0), None, "below");
    }

    #[test]
    fn contains_agrees_with_hit() {
        let v = view();
        let (ox, oy) = v.origin;
        let (w, h) = v.size();
        for &(x, y) in &[
            (ox - 0.5, oy),
            (ox, oy),
            (ox + w / 2.0, oy + h / 2.0),
            (ox + w - 0.5, oy + h - 0.5),
            (ox + w + 0.5, oy + h),
        ] {
            assert_eq!(
                v.contains(x, y),
                v.hit(x, y).is_some(),
                "disagreement at ({}, {})",
                x,
                y
            );
        }
    }

    /// The drop path offsets the cursor by the grab point then probes half a
    /// cell in, to land on the piece's anchor. Check that round-trips too.
    #[test]
    fn the_drop_probe_recovers_the_anchor_a_piece_was_grabbed_from() {
        let v = view();
        for gy in 0..SLOT_H {
            for gx in 0..SLOT_W {
                let (px, py) = v.cell_origin(gx, gy);
                // Grabbed exactly at the anchor's top-left, so grab == (0, 0).
                let probe = (px + SLOT_CELL * 0.5, py + SLOT_CELL * 0.5);
                assert_eq!(v.hit(probe.0, probe.1), Some((gx, gy)));
            }
        }
    }

    /// The letterbox transform and the mouse transform must be exact inverses,
    /// or clicks land somewhere other than where the cursor is drawn.
    #[test]
    fn the_viewport_maps_screen_pixels_back_to_logical_coordinates() {
        for (sw, sh) in [(1600.0, 980.0), (1100.0, 700.0), (2560.0, 1080.0), (900.0, 1400.0)] {
            let scale = (sw / LOGICAL_W).min(sh / LOGICAL_H);
            let vp = Viewport {
                x: (sw - LOGICAL_W * scale) / 2.0,
                y: (sh - LOGICAL_H * scale) / 2.0,
                scale,
            };
            // Forward-project a logical point the way the camera does, then ask
            // the mouse transform to bring it back.
            for (lx, ly) in [(0.0, 0.0), (800.0, 490.0), (1599.0, 979.0), (183.0, 112.0)] {
                let (sx, sy) = (vp.x + lx * vp.scale, vp.y + ly * vp.scale);
                let (bx, by) = ((sx - vp.x) / vp.scale, (sy - vp.y) / vp.scale);
                assert!(
                    (bx - lx).abs() < 0.01 && (by - ly).abs() < 0.01,
                    "at {}x{}: ({}, {}) round-tripped to ({}, {})",
                    sw, sh, lx, ly, bx, by
                );
            }
            // The logical canvas must sit inside the real window, centred.
            assert!(vp.x >= -0.01 && vp.y >= -0.01, "letterbox offsets go inward");
            assert!(LOGICAL_W * scale <= sw + 0.01 && LOGICAL_H * scale <= sh + 0.01);
        }
    }

    /// The bars are drawn from when items actually fired, so a slowed item has
    /// to visibly crawl without the renderer knowing anything about frost.
    #[test]
    fn a_cooldown_bar_fills_across_the_real_gap_between_firings() {
        let schedule = [1000u32, 2000, 3000];
        assert!((bar_progress(&schedule, 1000, 1000) - 0.0).abs() < 0.001, "just fired");
        assert!((bar_progress(&schedule, 1000, 1500) - 0.5).abs() < 0.001, "halfway");
        assert!((bar_progress(&schedule, 1000, 1999) - 0.999).abs() < 0.01, "about to fire");
    }

    #[test]
    fn a_slowed_item_fills_more_slowly_than_its_nominal_cooldown() {
        // Nominal 1s, but frost stretched the second gap to 1.5s.
        let slowed = [1000u32, 2500];
        let normal = [1000u32, 2000];
        // Half a second after firing, the slowed bar is behind the normal one.
        let s = bar_progress(&slowed, 1000, 1500);
        let n = bar_progress(&normal, 1000, 1500);
        assert!(s < n, "slowed {} should trail normal {}", s, n);
        assert!((s - 1.0 / 3.0).abs() < 0.01, "500ms into a 1500ms gap");
    }

    #[test]
    fn the_bar_fills_toward_the_first_firing_from_empty() {
        let schedule = [2000u32];
        assert!((bar_progress(&schedule, 2000, 0) - 0.0).abs() < 0.001);
        assert!((bar_progress(&schedule, 2000, 1000) - 0.5).abs() < 0.001);
    }

    #[test]
    fn a_bar_with_no_firings_left_falls_back_to_the_nominal_rate_and_clamps() {
        let schedule = [1000u32];
        assert!((bar_progress(&schedule, 2000, 2000) - 0.5).abs() < 0.001);
        assert!(bar_progress(&schedule, 2000, 9000) <= 1.0, "never overfills");
        // An item that never fired at all still shows sensible progress.
        assert!(bar_progress(&[], 1000, 500) > 0.0);
        assert!(bar_progress(&[], 1000, 99_999) <= 1.0);
    }

    fn profile_with(pieces: Vec<PieceId>) -> ItemProfile {
        ItemProfile {
            pieces,
            adjacent_items: Vec::new(),
            aligned_items: Vec::new(),
            name: "T".into(),
            full_name: "T".into(),
            core: "T".into(),
            slot: SlotKind::Weapon,
            cooldown_ms: 1000,
            stats: gearmaster_engine::stats::Stats::ZERO,
            triggers: Vec::new(),
            adjacent_assembled_same_slot: 0,
        }
    }

    #[test]
    fn an_item_shakes_when_it_fires_and_settles_again() {
        let p = PieceId(7);
        let profiles = vec![profile_with(vec![p])];
        let schedules = vec![vec![1000u32, 3000]];

        // Right after firing there is a visible offset...
        let just = shake_offsets(&profiles, &schedules, 0, 1040);
        let (dx, dy) = just[&p];
        assert!(dx.abs() + dy.abs() > 0.3, "expected a jolt, got ({}, {})", dx, dy);

        // ...and it has settled before the shake window closes.
        let settled = shake_offsets(&profiles, &schedules, 0, 1000 + SHAKE_MS - 1);
        let (lx, ly) = settled[&p];
        assert!(lx.abs() + ly.abs() < dx.abs() + dy.abs(), "the shake must decay");

        // Past the window there is no entry at all.
        assert!(shake_offsets(&profiles, &schedules, 0, 1000 + SHAKE_MS).is_empty());
        // And the second firing starts it again.
        assert!(!shake_offsets(&profiles, &schedules, 0, 3020).is_empty());
    }

    #[test]
    fn an_item_that_has_not_fired_yet_never_shakes() {
        let p = PieceId(3);
        let profiles = vec![profile_with(vec![p])];
        let schedules = vec![vec![5000u32]];
        assert!(shake_offsets(&profiles, &schedules, 0, 2000).is_empty());
    }

    #[test]
    fn a_monsters_gear_shakes_past_its_innate_attacks() {
        // A monster's item list puts innate attacks first, so its profiles are
        // read at an offset. Getting this wrong shakes the wrong gear.
        let p = PieceId(1);
        let profiles = vec![profile_with(vec![p])];
        // Index 0 is an innate bite; index 1 is the profile above.
        let schedules = vec![vec![500u32], vec![1000u32]];
        assert!(
            shake_offsets(&profiles, &schedules, 1, 1030).contains_key(&p),
            "the profile should follow schedule index 1"
        );
        assert!(
            shake_offsets(&profiles, &schedules, 1, 530).is_empty(),
            "the bite firing must not shake the gear"
        );
    }

    #[test]
    fn cooldown_rows_are_ordered_weapon_first() {
        use gearmaster_engine::piece::SlotKind::*;
        let ranks: Vec<u8> = [Weapon, Helmet, Chest, Gloves, Greaves]
            .iter()
            .map(|&s| slot_rank(Some(s)))
            .collect();
        assert_eq!(ranks, vec![1, 2, 3, 4, 5], "weapon leads, then head to toe");
        assert!(slot_rank(None) < slot_rank(Some(Weapon)), "innate attacks lead");
        assert!(ranks.windows(2).all(|w| w[0] < w[1]), "and the order is strict");
    }

    #[test]
    fn abbreviations_are_the_initials() {
        assert_eq!(abbrev("Balanced Grip"), "BG");
        assert_eq!(abbrev("Iron Blade"), "IB");
        assert_eq!(abbrev("Woven Underlayer"), "WU");
        assert_eq!(abbrev("Runner's Mold"), "RM");
    }

    #[test]
    fn wrapping_never_exceeds_the_width_unless_a_word_does() {
        let lines = wrap("1 handle + 1-2 damaging + up to 2 accessories", 26);
        assert!(lines.len() > 1);
        for l in &lines {
            assert!(l.len() <= 26, "line too long: {:?}", l);
        }
        assert_eq!(lines.join(" "), "1 handle + 1-2 damaging + up to 2 accessories");
    }
}
