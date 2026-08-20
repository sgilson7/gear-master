//! Rendering and input only. No game rules live here — every legality
//! question (does this piece fit, did the slot assemble, who wins the fight)
//! goes to the engine.

use std::collections::HashSet;

use gearmaster_engine::combat::{CombatLog, Event, Outcome, Side, LADDER};
use gearmaster_engine::loadout::SlotReport;
use gearmaster_engine::piece::{default_cooldown_ms, PieceDef, PieceId, PieceKind, SlotKind};
use gearmaster_engine::run::{Phase, Run};
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
const CARD_H: f32 = 124.0;
const CARD_GAP: f32 = 10.0;
/// How much faster than real time the fight replays.
const PLAYBACK_SPEED: f32 = 2.0;
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
        let shop_h = CARD_H + 52.0;
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

fn centered_text(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    let d = measure_text(s, None, size as u16, 1.0);
    draw_text(s, cx - d.width / 2.0, y, size, color);
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
    start: f64,
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
}

impl Playback {
    fn new(log: &CombatLog) -> Self {
        Playback {
            start: get_time(),
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
        }
    }

    fn apply(&mut self, log: &CombatLog, index: usize) {
        let entry = &log.entries[index];
        let now = get_time();
        self.now_ms = entry.at_ms;
        match &entry.event {
            Event::Activate { .. } => return, // too chatty for the on-screen log
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
            Event::Fell { .. } => {}
            Event::End { .. } => self.done = true,
        }
        self.lines.push(log.describe(entry));
    }

    /// Advance to whatever beat wall-clock time has reached.
    fn advance(&mut self, run: &Run) {
        let Some(log) = run.log.as_ref() else { return };
        let elapsed_ms = ((get_time() - self.start) * 1000.0 * PLAYBACK_SPEED as f64) as u32;
        while self.cursor < log.entries.len() && log.entries[self.cursor].at_ms <= elapsed_ms {
            let i = self.cursor;
            self.apply(log, i);
            self.cursor += 1;
        }
        self.now_ms = self.now_ms.max(elapsed_ms.min(log.duration_ms));
        self.player_curses.retain(|c| c.1 > self.now_ms);
        self.enemy_curses.retain(|c| c.1 > self.now_ms);
    }

    fn skip_to_end(&mut self, run: &Run) {
        let Some(log) = run.log.as_ref() else { return };
        while self.cursor < log.entries.len() {
            let i = self.cursor;
            self.apply(log, i);
            self.cursor += 1;
        }
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
        draw_text(view.kind.name(), ox, oy - 42.0, 21.0, WHITE);
        for (i, line) in wrap(view.kind.recipe_text(), 26).into_iter().take(2).enumerate() {
            draw_text(&line, ox, oy - 26.0 + i as f32 * 13.0, 12.0, col_dim());
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
        draw_text(&report.summary(), ox, oy + gh + 20.0, 14.0, color);
        let contrib = report.stats.summary();
        if !contrib.is_empty() {
            for (i, line) in wrap(&contrib, 24).into_iter().take(2).enumerate() {
                draw_text(&line, ox, oy + gh + 38.0 + i as f32 * 14.0, 12.0, col_dim());
            }
        }
    }
}

/// The shelf. Clicking a card buys it if you can afford it.
fn render_shop(layout: &Layout, run: &Run, mx: f32, my: f32) {
    let r = layout.shop;
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(28, 26, 22, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(96, 84, 52, 255));

    draw_text("SHOP", r.x + 14.0, r.y + 26.0, 18.0, col_gold());
    draw_text(&format!("{} gold", run.gold), r.x + 14.0, r.y + 50.0, 20.0, WHITE);
    draw_text("click to buy", r.x + 14.0, r.y + 72.0, 12.0, col_dim());
    draw_text(
        "new stock after",
        r.x + 14.0,
        r.y + 92.0,
        12.0,
        col_dim(),
    );
    draw_text("every battle", r.x + 14.0, r.y + 106.0, 12.0, col_dim());

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
        let mut ty = card.rect.y + 80.0;
        for line in wrap(def.name, 15).into_iter().take(2) {
            centered_text(&line, cx, ty, 12.0, if afford { WHITE } else { col_dim() });
            ty += 12.0;
        }
        centered_text(def.kind.name(), cx, card.rect.y + 106.0, 11.0, col_dim());
        centered_text(
            &format!("{} gold", def.price),
            cx,
            card.rect.y + card.rect.h - 6.0,
            14.0,
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
    draw_text("INVENTORY", layout.inv.x + 14.0, layout.inv.y + 24.0, 18.0, WHITE);
    draw_text(
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
        let mut ty = card.rect.y + 98.0;
        for line in wrap(def.name, 15).into_iter().take(2) {
            centered_text(&line, cx, ty, 12.0, Color::from_rgba(215, 218, 235, 255));
            ty += 12.0;
        }
        centered_text(def.kind.name(), cx, card.rect.y + card.rect.h - 6.0, 11.0, col_dim());

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
fn render_tooltip(run: &Run, id: PieceId, mx: f32, my: f32) {
    render_def_tooltip(run.registry.def(id), mx, my);
}

fn render_def_tooltip(def: &'static PieceDef, mx: f32, my: f32) {
    let mut lines: Vec<(String, Color)> = vec![
        (def.name.to_string(), WHITE),
        (format!("{} · {}", def.slot.name(), def.kind.name()), col_dim()),
    ];

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
        .map(|(s, _)| measure_text(s, None, 14, 1.0).width)
        .fold(0.0_f32, f32::max)
        + 20.0;
    let h = lines.len() as f32 * 18.0 + 14.0;
    let x = (mx + 16.0).min(LOGICAL_W - w - 6.0);
    let y = (my + 16.0).min(LOGICAL_H - h - 6.0);

    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 244));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(110, 110, 145, 255));
    for (i, (s, c)) in lines.iter().enumerate() {
        draw_text(s, x + 10.0, y + 20.0 + i as f32 * 18.0, 14.0, *c);
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

fn render_fight(layout: &Layout, run: &Run, pb: &Playback) {
    let Some(log) = run.log.as_ref() else { return };
    let area = layout.inv;
    draw_rectangle(area.x, area.y, area.w, area.h, col_tray());
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, Color::from_rgba(60, 60, 82, 255));

    let now = get_time();
    let half = area.w / 2.0 - 30.0;

    let fighter = |x: f32,
                       name: &str,
                       hp: i32,
                       max: i32,
                       armor: i32,
                       mana: Option<i32>,
                       curses: &[(&'static str, u32)],
                       flash: f64,
                       tint: Color| {
        let flashing = now - flash < FLASH_SECS;
        let bg = if flashing {
            Color::from_rgba(80, 34, 34, 255)
        } else {
            Color::from_rgba(32, 32, 46, 255)
        };
        draw_rectangle(x, area.y + 18.0, half, 150.0, bg);
        draw_rectangle_lines(
            x,
            area.y + 18.0,
            half,
            150.0,
            2.0,
            if hp <= 0 { col_bad() } else { Color::from_rgba(70, 70, 95, 255) },
        );
        draw_text(name, x + 16.0, area.y + 44.0, 22.0, if hp <= 0 { col_bad() } else { WHITE });
        hp_bar(x + 16.0, area.y + 56.0, half - 32.0, 26.0, hp, max, tint);

        // Armour rides above the health bar as its own strip, because it is
        // spent first and refills during the fight.
        let ax = x + 16.0;
        let aw = half - 32.0;
        draw_rectangle(ax, area.y + 88.0, aw, 14.0, Color::from_rgba(30, 30, 42, 255));
        if armor > 0 {
            let frac = ((armor as f32) / (max.max(1) as f32)).clamp(0.0, 1.0);
            draw_rectangle(ax, area.y + 88.0, aw * frac, 14.0, Color::from_rgba(150, 170, 210, 255));
        }
        draw_rectangle_lines(ax, area.y + 88.0, aw, 14.0, 1.5, Color::from_rgba(80, 80, 105, 255));
        draw_text(&format!("armor {}", armor), ax + 6.0, area.y + 100.0, 12.0, WHITE);

        let mut label = String::new();
        if let Some(m) = mana {
            label.push_str(&format!("mana {}", m));
        }
        if !label.is_empty() {
            draw_text(&label, ax, area.y + 122.0, 15.0, Color::from_rgba(150, 200, 240, 255));
        }

        // Active curses.
        let mut cx = ax + if label.is_empty() { 0.0 } else { 110.0 };
        for (kind, _) in curses {
            let text = format!("curse of {}", kind);
            let d = measure_text(&text, None, 13, 1.0);
            draw_rectangle(cx - 4.0, area.y + 110.0, d.width + 10.0, 18.0, Color::from_rgba(90, 40, 90, 230));
            draw_text(&text, cx + 1.0, area.y + 123.0, 13.0, Color::from_rgba(240, 190, 240, 255));
            cx += d.width + 16.0;
        }

        if hp <= 0 {
            draw_text("DOWN", x + half - 70.0, area.y + 44.0, 22.0, col_bad());
        }
    };

    fighter(
        area.x + 20.0,
        &log.player.name,
        pb.player_hp,
        pb.player_max,
        pb.player_armor,
        Some(pb.player_mana),
        &pb.player_curses,
        pb.flash_player,
        Color::from_rgba(90, 190, 120, 255),
    );
    fighter(
        area.x + area.w / 2.0 + 10.0,
        &log.enemy.name,
        pb.enemy_hp,
        pb.enemy_max,
        pb.enemy_armor,
        None,
        &pb.enemy_curses,
        pb.flash_enemy,
        Color::from_rgba(210, 110, 90, 255),
    );

    // Clock plus the rolling log.
    let log_top = area.y + 190.0;
    draw_text("COMBAT LOG", area.x + 20.0, log_top, 16.0, WHITE);
    let clock = format!("{:.1}s", pb.now_ms as f32 / 1000.0);
    draw_text(&clock, area.x + 140.0, log_top, 16.0, col_gold());

    let visible = (((area.h - 220.0) / 19.0) as usize).max(1);
    let start = pb.lines.len().saturating_sub(visible);
    for (i, line) in pb.lines[start..].iter().enumerate() {
        let is_last = start + i == pb.lines.len() - 1;
        draw_text(
            line,
            area.x + 20.0,
            log_top + 24.0 + i as f32 * 19.0,
            15.0,
            if is_last { WHITE } else { Color::from_rgba(150, 152, 172, 255) },
        );
    }

    if pb.done {
        let (label, color) = match log.outcome {
            Outcome::Victory => ("VICTORY", col_ok()),
            Outcome::Defeat => ("DEFEAT", col_bad()),
            Outcome::Stalemate => ("STALEMATE", col_gold()),
        };
        let bw = 300.0;
        let bx = area.x + area.w - bw - 24.0;
        let by = area.y + 196.0;
        draw_rectangle(bx, by, bw, 76.0, Color::from_rgba(18, 18, 28, 240));
        draw_rectangle_lines(bx, by, bw, 76.0, 3.0, color);
        centered_text(label, bx + bw / 2.0, by + 48.0, 38.0, color);
    }
}

/// Buttons live in the right-hand panel; the same rects are used to draw and
/// to hit-test.
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
    draw_text("GEAR MASTER", x + 20.0, y, 26.0, WHITE);
    y += 30.0;

    let stats = run.player_stats();
    draw_text("YOUR CHARACTER", x + 20.0, y, 14.0, col_dim());
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
        draw_text(label, x + 20.0, y, 16.0, LIGHTGRAY);
        let d = measure_text(&value, None, 16, 1.0);
        draw_text(&value, x + PANEL_W - 20.0 - d.width, y, 16.0, color);
        y += 21.0;
    }
    y += 4.0;
    // The formula the whole build is chasing, spelled out.
    let dmg = stats.damage_per_attack();
    draw_text("Damage / attack", x + 20.0, y, 17.0, WHITE);
    let d = measure_text(&format!("{}", dmg), None, 19, 1.0);
    draw_text(&format!("{}", dmg), x + PANEL_W - 20.0 - d.width, y, 19.0, col_gold());
    y += 17.0;
    draw_text(
        &format!(
            "{} strength x {}.{:02} power",
            stats.strength,
            stats.power / 100,
            stats.power % 100
        ),
        x + 20.0,
        y,
        12.0,
        col_dim(),
    );
    y += 26.0;

    // Per-slot assembly readout.
    draw_text("GEAR", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    for r in reports {
        let done = r.assembled_count();
        let (mark, color) = if done > 0 { ("+", col_ok()) } else { ("-", col_dim()) };
        draw_text(mark, x + 20.0, y, 16.0, color);
        draw_text(r.slot.name(), x + 36.0, y, 16.0, if done > 0 { WHITE } else { col_dim() });
        let status = r.summary();
        let d = measure_text(&status, None, 13, 1.0);
        draw_text(
            &status,
            x + PANEL_W - 20.0 - d.width,
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
                draw_text(&format!("  {}", line), x + 36.0, y, 12.0, c);
                y += 14.0;
            }
        }
    }
    y += 12.0;

    draw_text("RUN", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    for (label, value, color) in [
        ("Gold", format!("{}", run.gold), col_gold()),
        ("Won", format!("{}", run.wins), col_ok()),
        ("Lost", format!("{}", run.losses), col_bad()),
    ] {
        draw_text(label, x + 20.0, y, 15.0, LIGHTGRAY);
        let d = measure_text(&value, None, 15, 1.0);
        draw_text(&value, x + PANEL_W - 20.0 - d.width, y, 15.0, color);
        y += 18.0;
    }
    y += 10.0;

    let m = run.monster();
    draw_text("NEXT OPPONENT", x + 20.0, y, 14.0, col_dim());
    y += 20.0;
    draw_text(m.name, x + 20.0, y, 17.0, Color::from_rgba(230, 140, 120, 255));
    let bounty = format!("{}g", m.bounty);
    let d = measure_text(&bounty, None, 15, 1.0);
    draw_text(&bounty, x + PANEL_W - 20.0 - d.width, y, 15.0, col_gold());
    y += 18.0;
    draw_text(
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
        draw_text(
            &format!("  {} / {:.1}s  {}", a.name, a.cooldown_ms as f32 / 1000.0, what),
            x + 20.0,
            y,
            12.0,
            col_dim(),
        );
        y += 14.0;
    }
    if m.mind_resist > 0 || m.curse_resist > 0 {
        draw_text(
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
        draw_text(&line, x + 20.0, y, 14.0, Color::from_rgba(225, 225, 240, 255));
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
    let mut message =
        String::from("Drag components into a slot. Pieces must touch to become gear.");

    // Debug hooks so this window can be inspected without a human at the
    // keyboard: GEARMASTER_PRESET=1 starts geared up, GEARMASTER_FIGHT=1 opens
    // mid-bout, and GEARMASTER_SHOT=<path> captures a frame and exits.
    if std::env::var("GEARMASTER_PRESET").is_ok() {
        run.apply_preset();
        message = "Auto-built a complete loadout - every bonus is lit.".to_string();
    }
    if std::env::var("GEARMASTER_FIGHT").is_ok() {
        pb = Some(Playback::new(run.fight_next()));
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
        render_slots(&layout, &run, &reports, &drag);
        if run.phase == Phase::Fighting {
            if let Some(p) = pb.as_ref() {
                render_fight(&layout, &run, p);
            }
        } else {
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

        render_panel(&layout, &run, &reports, &message, &pb, mx, my);

        // Tooltip for whatever is under the cursor (never while dragging).
        if matches!(drag, Drag::None) {
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
                render_tooltip(&run, id, mx, my);
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
            if clicked_button(0) {
                run.back_to_loadout();
                pb = None;
                message = "Rearrange your gear and fight again.".to_string();
            } else if clicked_button(1) {
                if let Some(p) = pb.as_mut() {
                    p.skip_to_end(&run);
                }
            } else if clicked_button(2) {
                pb = Some(Playback::new(run.fight_next()));
                settled = false;
            }
        } else {
            if clicked_button(0) {
                pb = Some(Playback::new(run.fight_next()));
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
            if let Some(i) = layout.shop_hit(mx, my) {
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
