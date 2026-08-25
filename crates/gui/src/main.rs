//! Rendering and input only. No game rules live here — every legality
//! question (does this piece fit, did the slot assemble, who wins the fight)
//! goes to the engine.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};

use gearmaster_engine::class::Axis;
use gearmaster_engine::combat::{
    CombatLog, Event, MonsterSpec, MonsterSprite, Outcome, RunningItem, Side, LADDER,
};
use gearmaster_engine::loadout::{ItemProfile, Loadout, SlotReport};
use gearmaster_engine::piece::{
    default_cooldown_ms, Action, PieceDef, PieceId, PieceKind, PieceRegistry, Resource, SlotKind,
    Trigger,
};
use gearmaster_engine::rating::{resale_price, shop_price, Rarity};
use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::{Mode, ROGUE_LIVES};
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
/// Sized so the five boards together span exactly the width of the shop
/// beneath them: the H of HELMET starts on the shop's left edge and the weapon
/// board ends on its right. Derived rather than chosen - the panel width less
/// its margins and the four gaps, over thirty columns - and pinned by a test,
/// so changing the side panel cannot quietly break the alignment.
const SLOT_CELL: f32 = 36.6;
const SLOT_GAP: f32 = 22.0;
/// Finished items are listed under the board they were built in, one line
/// each. `STRIP_TOP` is where the first line sits below the grid, `STRIP_ROWS`
/// how many are drawn before the rest are counted instead.
const STRIP_TOP: f32 = 18.0;
const STRIP_ROW_H: f32 = 19.0;
const STRIP_ROWS: usize = 4;

/// Tighter than it was: the taller boards need the room and there is nothing
/// above them but a slot name.
const SLOT_TOP: f32 = 68.0;
const INV_CELL: f32 = 15.0;
const CARD_W: f32 = 126.0;
/// Short enough that a full tray - twelve loose pieces, two rows of eight -
/// fits under the shop without running off the screen. Both card layouts flow
/// their text rather than pinning it to the bottom edge, so the height can
/// come down without the name landing on the role.
const CARD_H: f32 = 114.0;
const CARD_GAP: f32 = 10.0;
/// How long an item jitters after it fires, and how far.
const SHAKE_MS: u32 = 260;
const SHAKE_PX: f32 = 3.4;

/// Cell size for the gear boards shown during a fight. Bigger than the
/// loadout screen's: the two boards stack vertically rather than sitting side
/// by side, so width stops being the constraint.
const MINI_CELL: f32 = 32.0;
const MINI_GAP: f32 = 14.0;

/// Playback rate a fight opens at.
const DEFAULT_SPEED: f32 = 1.0;

/// Step through the playback rates. Settable before a fight as well as during
/// one, so you can line up a slow replay in advance.
fn next_speed(speed: f32) -> f32 {
    match speed {
        s if s >= 2.0 => 1.0,
        s if s >= 1.0 => 0.5,
        s if s >= 0.5 => 0.25,
        _ => 2.0,
    }
}
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
        // hover state can be captured in a screenshot. GEARMASTER_MOUSE2 is
        // where it goes once GEARMASTER_CLICK's frame has passed - the only
        // way to photograph a thing you have to click and then point at.
        if FRAME.load(Ordering::Relaxed) > synthetic_click_frame() {
            if let Ok(v) = std::env::var("GEARMASTER_MOUSE2") {
                if let Some((a, b)) = v.split_once(',') {
                    if let (Ok(x), Ok(y)) = (a.trim().parse(), b.trim().parse()) {
                        return (x, y);
                    }
                }
            }
        }
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

/// Frames drawn, for the scripted-input hooks below. Nothing in the game reads
/// it.
static FRAME: AtomicU32 = AtomicU32::new(0);

/// The frame GEARMASTER_CLICK fires a left press on; `u32::MAX` when unset.
fn synthetic_click_frame() -> u32 {
    std::env::var("GEARMASTER_CLICK").ok().and_then(|v| v.parse().ok()).unwrap_or(u32::MAX)
}

/// The frame GEARMASTER_CLICK2 fires a second left press on. Some flows are
/// two clicks long - pick the shelf, then pick what pays for it.
fn second_click_frame() -> u32 {
    std::env::var("GEARMASTER_CLICK2").ok().and_then(|v| v.parse().ok()).unwrap_or(u32::MAX)
}

/// A left press this frame, real or scripted.
fn left_pressed() -> bool {
    let f = FRAME.load(Ordering::Relaxed);
    is_mouse_button_pressed(MouseButton::Left)
        || f == synthetic_click_frame()
        || f == second_click_frame()
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
    /// How many rows this grid has. A run can be given more, so this is
    /// carried rather than read off a constant.
    rows: u8,
}

impl SlotView {
    fn size(&self) -> (f32, f32) {
        (SLOT_W as f32 * SLOT_CELL, self.rows as f32 * SLOT_CELL)
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
        if !(0..SLOT_W as i32).contains(&gx) || !(0..self.rows as i32).contains(&gy) {
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

/// The bands the loadout screen is built from, in pixels, with no reference to
/// macroquad. Split out so the alignment between the boards and the shop can
/// be tested - `Layout::build` measures text, which needs a graphics context a
/// unit test does not have.
struct Bands {
    /// Left edge of the first board, and of the shop.
    x0: f32,
    /// Combined width of the five boards, and of the shop.
    total: f32,
    /// Top of the shop.
    strip_y: f32,
    shop_h: f32,
    inv_y: f32,
    inv_h: f32,
}

fn bands(worn: usize, rows: u8) -> Bands {
    let panel_x = LOGICAL_W - PANEL_W;
    let gh = rows as f32 * SLOT_CELL;
    let total = (panel_x - 48.0).max(100.0);
    // Each slot lists its own items, so the band is as tall as the fullest one
    // rather than as tall as the whole loadout. One more row for "unfinished".
    let rows = (worn.clamp(1, STRIP_ROWS) + 1) as f32;
    let strip_y = SLOT_TOP + gh + STRIP_TOP + rows * STRIP_ROW_H + 10.0;
    let shop_h = CARD_H + 58.0;
    let inv_y = strip_y + shop_h + 14.0;
    Bands {
        x0: 24.0,
        total,
        strip_y,
        shop_h,
        inv_y,
        inv_h: (LOGICAL_H - inv_y - 24.0).max(100.0),
    }
}

impl Layout {
    /// `worn` is how many finished items the strip below the boards has to
    /// show. The band grows a row at a time rather than always reserving room
    /// for a full loadout, which would be dead space for most of a run.
    fn build(run: &Run, worn: usize) -> Self {
        let panel_x = LOGICAL_W - PANEL_W;
        let gw = SLOT_W as f32 * SLOT_CELL;
        let rows = run.loadout.rows();
        let b = bands(worn, rows);
        let x0 = b.x0;

        let slots = SlotKind::ALL
            .iter()
            .enumerate()
            .map(|(i, &kind)| SlotView {
                kind,
                origin: (x0 + i as f32 * (gw + SLOT_GAP), SLOT_TOP),
                rows,
            })
            .collect();

        let shop = Rect::new(b.x0, b.strip_y, b.total, b.shop_h);
        let inv = Rect::new(b.x0, b.inv_y, b.total, b.inv_h);

        // Cards flow left to right, wrapping to fill the tray.
        let per_row = (((inv.w + CARD_GAP) / (CARD_W + CARD_GAP)) as usize).max(1);
        // Groups, not pieces: a locked item is carried around as one thing, so
        // it gets one card. Its first piece stands for it.
        let cards = run
            .inventory_groups()
            .into_iter()
            .filter_map(|g| g.first().copied())
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
                        shop_cards_x(shop) + i as f32 * (CARD_W + CARD_GAP),
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

// ================================================================= theme

/// The words this frame is being drawn in.
///
/// Set once at the top of each frame from `run.theme`. It lives here rather
/// than being threaded through forty drawing functions because it is a display
/// concern and nothing else - no drawing code decides anything with it, and
/// the engine never sees a themed string at all.
mod words {
    use gearmaster_engine::theme::{Theme, THEMES};
    use std::cell::Cell;

    thread_local! {
        static CURRENT: Cell<&'static Theme> = const { Cell::new(THEMES[0]) };
    }

    pub fn set(t: &'static Theme) {
        CURRENT.with(|c| c.set(t));
    }

    pub fn current() -> &'static Theme {
        CURRENT.with(|c| c.get())
    }

    /// A component's name, in this theme.
    pub fn piece(canonical: &'static str) -> &'static str {
        current().piece(canonical)
    }

    /// A creature's name, in this theme.
    pub fn monster(canonical: &'static str) -> &'static str {
        current().monster(canonical)
    }

    /// A class's title, in this theme. Comparisons keep using the canonical
    /// name - only what the player reads goes through here.
    pub fn class(canonical: &'static str) -> &'static str {
        current().class(canonical)
    }

    /// Any other interface string, by slug, with the plain wording as the
    /// fallback so an unfinished theme reads as English rather than as slugs.
    pub fn word(slug: &str, plain: &'static str) -> &'static str {
        current().word(slug, plain)
    }

    /// Re-tell prose the engine wrote. Whole words only; the plain theme has
    /// no vocabulary and hands the string straight back.
    pub fn retell(prose: &str) -> String {
        current().retell(prose)
    }

    /// The same, and then any class named in it swapped for its title in this
    /// theme.
    ///
    /// `retell` cannot do this: it works a whole word at a time, so it could
    /// never reach "Ticket to Ride", and putting "tired" in the vocabulary
    /// would rename every tired road and tired arm in the game. Class names
    /// are matched longest first so "Ticket to Ride" is not eaten as "Ticket".
    pub fn retell_naming(prose: &str) -> String {
        let mut out = retell(prose);
        let mut names: Vec<&'static str> =
            gearmaster_engine::class::CLASSES.iter().map(|c| c.name).collect();
        names.sort_by_key(|n| std::cmp::Reverse(n.len()));
        for n in names {
            let themed = class(n);
            if themed != n && out.contains(n) {
                out = out.replace(n, themed);
            }
        }
        out
    }
}

// ============================================================= creatures

/// A simple silhouette for each monster, drawn from primitives in a box of
/// side `sz`. Deliberately crude: the point is that a Toad reads differently
/// from a Wisp at a glance, not that either is a portrait.
fn draw_monster(x: f32, y: f32, sz: f32, sprite: MonsterSprite, c: Color, dark: Color) {
    let t = (sz * 0.05).max(1.5);
    // Handy fractions of the box.
    let fx = |f: f32| x + sz * f;
    let fy = |f: f32| y + sz * f;

    match sprite {
        MonsterSprite::Rat => {
            draw_ellipse(fx(0.46), fy(0.62), sz * 0.26, sz * 0.18, 0.0, c);
            draw_circle(fx(0.72), fy(0.52), sz * 0.12, c);
            draw_triangle(
                Vec2::new(fx(0.80), fy(0.46)),
                Vec2::new(fx(0.96), fy(0.54)),
                Vec2::new(fx(0.80), fy(0.58)),
                c,
            ); // snout
            draw_circle(fx(0.68), fy(0.40), sz * 0.06, c); // ear
            draw_circle(fx(0.76), fy(0.50), sz * 0.02, dark); // eye
            // Tail.
            draw_line(fx(0.20), fy(0.62), fx(0.06), fy(0.42), t, c);
            for i in 0..3 {
                let lx = fx(0.34 + i as f32 * 0.16);
                draw_line(lx, fy(0.76), lx, fy(0.90), t, c);
            }
        }
        MonsterSprite::Toad => {
            draw_ellipse(fx(0.5), fy(0.66), sz * 0.36, sz * 0.22, 0.0, c);
            draw_circle(fx(0.34), fy(0.42), sz * 0.11, c);
            draw_circle(fx(0.66), fy(0.42), sz * 0.11, c);
            draw_circle(fx(0.34), fy(0.42), sz * 0.045, dark);
            draw_circle(fx(0.66), fy(0.42), sz * 0.045, dark);
            draw_line(fx(0.28), fy(0.70), fx(0.72), fy(0.70), t, dark); // mouth
            draw_ellipse(fx(0.16), fy(0.82), sz * 0.10, sz * 0.06, 0.0, c);
            draw_ellipse(fx(0.84), fy(0.82), sz * 0.10, sz * 0.06, 0.0, c);
        }
        MonsterSprite::Archer => {
            draw_circle(fx(0.42), fy(0.28), sz * 0.13, c); // skull
            draw_circle(fx(0.38), fy(0.28), sz * 0.035, dark);
            draw_circle(fx(0.47), fy(0.28), sz * 0.035, dark);
            draw_line(fx(0.42), fy(0.41), fx(0.42), fy(0.74), t, c); // spine
            for i in 0..3 {
                let ry = fy(0.48 + i as f32 * 0.10);
                draw_line(fx(0.30), ry, fx(0.54), ry, t, c); // ribs
            }
            draw_line(fx(0.42), fy(0.74), fx(0.32), fy(0.92), t, c);
            draw_line(fx(0.42), fy(0.74), fx(0.52), fy(0.92), t, c);
            // Bow.
            draw_poly_lines(fx(0.76), fy(0.52), 16, sz * 0.24, 0.0, t, c);
            draw_line(fx(0.76), fy(0.28), fx(0.76), fy(0.76), t * 0.7, c);
        }
        MonsterSprite::Golem => {
            draw_rectangle(fx(0.28), fy(0.30), sz * 0.44, sz * 0.44, c); // torso
            draw_rectangle(fx(0.38), fy(0.12), sz * 0.24, sz * 0.18, c); // head
            draw_rectangle(fx(0.44), fy(0.18), sz * 0.12, sz * 0.05, dark); // slit
            draw_rectangle(fx(0.08), fy(0.34), sz * 0.18, sz * 0.36, c); // arms
            draw_rectangle(fx(0.74), fy(0.34), sz * 0.18, sz * 0.36, c);
            draw_rectangle(fx(0.30), fy(0.76), sz * 0.16, sz * 0.18, c); // legs
            draw_rectangle(fx(0.54), fy(0.76), sz * 0.16, sz * 0.18, c);
        }
        MonsterSprite::Wisp => {
            // A cold mote with radiating spines.
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                draw_line(
                    fx(0.5),
                    fy(0.5),
                    fx(0.5) + a.cos() * sz * 0.42,
                    fy(0.5) + a.sin() * sz * 0.42,
                    t,
                    c,
                );
            }
            draw_poly(fx(0.5), fy(0.5), 4, sz * 0.18, 45.0, c);
            draw_poly(fx(0.5), fy(0.5), 4, sz * 0.08, 45.0, dark);
        }
        // Lean, head down, jaws open. Deliberately unlike the rat: raised
        // hackles instead of a smooth back, a brush tail instead of a whip,
        // and a head slung level with the shoulders rather than perched above.
        MonsterSprite::Hound => {
            draw_ellipse(fx(0.44), fy(0.52), sz * 0.28, sz * 0.13, 0.0, c);
            // Hackles along the spine.
            for i in 0..4 {
                let hx = fx(0.26 + i as f32 * 0.13);
                draw_triangle(
                    Vec2::new(hx - sz * 0.05, fy(0.42)),
                    Vec2::new(hx, fy(0.26)),
                    Vec2::new(hx + sz * 0.05, fy(0.42)),
                    c,
                );
            }
            // Head, slung forward and low.
            draw_circle(fx(0.72), fy(0.58), sz * 0.12, c);
            draw_triangle(
                Vec2::new(fx(0.68), fy(0.48)),
                Vec2::new(fx(0.74), fy(0.34)),
                Vec2::new(fx(0.79), fy(0.50)),
                c,
            ); // ear
            // Open jaws: an upper and a lower snout with a bite between them.
            draw_triangle(
                Vec2::new(fx(0.78), fy(0.52)),
                Vec2::new(fx(0.99), fy(0.54)),
                Vec2::new(fx(0.78), fy(0.60)),
                c,
            );
            draw_triangle(
                Vec2::new(fx(0.78), fy(0.64)),
                Vec2::new(fx(0.95), fy(0.72)),
                Vec2::new(fx(0.78), fy(0.70)),
                c,
            );
            draw_circle(fx(0.72), fy(0.53), sz * 0.025, dark);
            // Brush tail.
            draw_triangle(
                Vec2::new(fx(0.20), fy(0.50)),
                Vec2::new(fx(0.02), fy(0.28)),
                Vec2::new(fx(0.16), fy(0.58)),
                c,
            );
            for i in 0..4 {
                let lx = fx(0.28 + i as f32 * 0.12);
                draw_line(lx, fy(0.62), lx, fy(0.92), t, c);
            }
        }
        // A slab of a figure, all chestplate: bands across a wide torso, tiny
        // head, planted feet. Its whole point is armour.
        MonsterSprite::Warden => {
            draw_rectangle(fx(0.16), fy(0.24), sz * 0.68, sz * 0.50, c);
            for i in 0..3 {
                let by = fy(0.34 + i as f32 * 0.14);
                draw_line(fx(0.20), by, fx(0.80), by, t * 1.2, dark);
            }
            draw_rectangle(fx(0.38), fy(0.06), sz * 0.24, sz * 0.18, c);
            draw_rectangle(fx(0.43), fy(0.13), sz * 0.14, sz * 0.04, dark);
            // Pauldrons and boots.
            draw_rectangle(fx(0.04), fy(0.26), sz * 0.12, sz * 0.22, c);
            draw_rectangle(fx(0.84), fy(0.26), sz * 0.12, sz * 0.22, c);
            draw_rectangle(fx(0.22), fy(0.76), sz * 0.20, sz * 0.18, c);
            draw_rectangle(fx(0.58), fy(0.76), sz * 0.20, sz * 0.18, c);
        }
        // Everything at once: a crowned figure ringed by cogs.
        MonsterSprite::Gearwright => {
            // Cog halo.
            for i in 0..10 {
                let a = i as f32 * std::f32::consts::TAU / 10.0;
                let (sx, sy) = (fx(0.5) + a.cos() * sz * 0.42, fy(0.46) + a.sin() * sz * 0.42);
                draw_rectangle(sx - sz * 0.055, sy - sz * 0.055, sz * 0.11, sz * 0.11, c);
            }
            draw_circle_lines(fx(0.5), fy(0.46), sz * 0.34, t * 1.6, c);
            // Body and head.
            draw_triangle(
                Vec2::new(fx(0.26), fy(0.92)),
                Vec2::new(fx(0.74), fy(0.92)),
                Vec2::new(fx(0.5), fy(0.42)),
                c,
            );
            draw_circle(fx(0.5), fy(0.38), sz * 0.15, c);
            draw_circle(fx(0.44), fy(0.36), sz * 0.03, dark);
            draw_circle(fx(0.56), fy(0.36), sz * 0.03, dark);
            // Crown.
            for i in 0..3 {
                let px = fx(0.38 + i as f32 * 0.12);
                draw_triangle(
                    Vec2::new(px - sz * 0.045, fy(0.26)),
                    Vec2::new(px + sz * 0.045, fy(0.26)),
                    Vec2::new(px, fy(0.13)),
                    c,
                );
            }
        }
        MonsterSprite::Sentinel => {
            draw_rectangle(fx(0.32), fy(0.22), sz * 0.36, sz * 0.56, c);
            draw_rectangle(fx(0.38), fy(0.06), sz * 0.24, sz * 0.18, c);
            draw_rectangle(fx(0.42), fy(0.13), sz * 0.16, sz * 0.04, dark);
            // Tower shield.
            draw_rectangle(fx(0.06), fy(0.26), sz * 0.22, sz * 0.48, c);
            draw_rectangle_lines(fx(0.06), fy(0.26), sz * 0.22, sz * 0.48, t, dark);
            draw_line(fx(0.17), fy(0.30), fx(0.17), fy(0.70), t, dark);
            draw_rectangle(fx(0.74), fy(0.30), sz * 0.10, sz * 0.44, c); // haft
            draw_rectangle(fx(0.34), fy(0.80), sz * 0.32, sz * 0.14, c);
        }
        MonsterSprite::Wraith => {
            // Hood: a teardrop over a ragged hem.
            draw_circle(fx(0.5), fy(0.34), sz * 0.20, c);
            draw_triangle(
                Vec2::new(fx(0.30), fy(0.36)),
                Vec2::new(fx(0.70), fy(0.36)),
                Vec2::new(fx(0.5), fy(0.80)),
                c,
            );
            for i in 0..4 {
                let hx = fx(0.32 + i as f32 * 0.12);
                draw_triangle(
                    Vec2::new(hx, fy(0.74)),
                    Vec2::new(hx + sz * 0.06, fy(0.74)),
                    Vec2::new(hx + sz * 0.03, fy(0.94)),
                    c,
                );
            }
            draw_circle(fx(0.44), fy(0.32), sz * 0.035, dark);
            draw_circle(fx(0.56), fy(0.32), sz * 0.035, dark);
        }
        MonsterSprite::Idiot => {
            // Armoured the way a seed is armoured: a closed husk with
            // something coiled inside it, and the shut eye that gives it the
            // name. It is asleep. It is not asleep about you.
            draw_poly(fx(0.5), fy(0.54), 8, sz * 0.34, 22.5, c);
            draw_poly_lines(fx(0.5), fy(0.54), 8, sz * 0.42, 22.5, t, c);
            // the coil
            for k in 0..3 {
                let rr = sz * (0.10 + k as f32 * 0.07);
                draw_circle_lines(fx(0.5), fy(0.54), rr, t * 0.7, dark);
            }
            // the shut eye, lying across it
            draw_line(fx(0.34), fy(0.50), fx(0.66), fy(0.50), t * 1.4, dark);
            draw_triangle(
                Vec2::new(fx(0.40), fy(0.50)),
                Vec2::new(fx(0.60), fy(0.50)),
                Vec2::new(fx(0.50), fy(0.44)),
                dark,
            );
            // roots, which is how it gets it back
            for k in 0..4 {
                let ox = 0.26 + k as f32 * 0.16;
                draw_line(fx(ox), fy(0.86), fx(0.5), fy(0.78), t * 0.8, c);
            }
        }
        MonsterSprite::Curator => {
            // A display case with something still in it, and the small figure
            // beside it holding the watch. He collects planes; the exhibits
            // are people who came to look at them.
            draw_rectangle_lines(fx(0.10), fy(0.16), sz * 0.44, sz * 0.62, t, c);
            draw_line(fx(0.32), fy(0.16), fx(0.32), fy(0.78), t * 0.6, dark);
            // the exhibit: a small shape standing inside the glass
            draw_circle(fx(0.21), fy(0.40), sz * 0.05, dark);
            draw_rectangle(fx(0.17), fy(0.47), sz * 0.08, sz * 0.22, dark);
            // the curator, outside it
            draw_circle(fx(0.72), fy(0.28), sz * 0.09, c);
            draw_rectangle(fx(0.65), fy(0.39), sz * 0.14, sz * 0.34, c);
            draw_line(fx(0.79), fy(0.46), fx(0.88), fy(0.52), t, c); // arm
            draw_circle_lines(fx(0.90), fy(0.56), sz * 0.07, t, c);  // the watch
            draw_line(fx(0.90), fy(0.56), fx(0.90), fy(0.51), t * 0.6, c);
        }
        MonsterSprite::Idol => {
            // Stacked stone with a carved face.
            draw_rectangle(fx(0.30), fy(0.14), sz * 0.40, sz * 0.30, c);
            draw_rectangle(fx(0.24), fy(0.46), sz * 0.52, sz * 0.24, c);
            draw_rectangle(fx(0.18), fy(0.72), sz * 0.64, sz * 0.20, c);
            draw_triangle(
                Vec2::new(fx(0.38), fy(0.24)),
                Vec2::new(fx(0.46), fy(0.24)),
                Vec2::new(fx(0.42), fy(0.34)),
                dark,
            );
            draw_triangle(
                Vec2::new(fx(0.54), fy(0.24)),
                Vec2::new(fx(0.62), fy(0.24)),
                Vec2::new(fx(0.58), fy(0.34)),
                dark,
            );
            draw_poly_lines(fx(0.5), fy(0.58), 12, sz * 0.09, 0.0, t, dark); // ward
        }
        MonsterSprite::Fiend => {
            // A figure and its reflection: solid on the left of the glass,
            // drawn in outline on the right.
            draw_triangle(
                Vec2::new(fx(0.5), fy(0.34)),
                Vec2::new(fx(0.16), fy(0.92)),
                Vec2::new(fx(0.5), fy(0.92)),
                c,
            );
            draw_circle(fx(0.40), fy(0.26), sz * 0.11, c);
            draw_circle(fx(0.43), fy(0.25), sz * 0.03, dark);

            draw_line(fx(0.5), fy(0.04), fx(0.5), fy(0.96), t, c);

            draw_line(fx(0.5), fy(0.34), fx(0.84), fy(0.92), t, c);
            draw_line(fx(0.84), fy(0.92), fx(0.5), fy(0.92), t, c);
            draw_circle_lines(fx(0.60), fy(0.26), sz * 0.11, t, c);
        }
        MonsterSprite::King => {
            // Crown over a skull over a heavy mantle.
            draw_triangle(
                Vec2::new(fx(0.28), fy(0.24)),
                Vec2::new(fx(0.72), fy(0.24)),
                Vec2::new(fx(0.5), fy(0.34)),
                c,
            );
            for i in 0..4 {
                let px = fx(0.28 + i as f32 * 0.147);
                draw_triangle(
                    Vec2::new(px, fy(0.24)),
                    Vec2::new(px + sz * 0.10, fy(0.24)),
                    Vec2::new(px + sz * 0.05, fy(0.06)),
                    c,
                );
            }
            draw_circle(fx(0.5), fy(0.46), sz * 0.16, c);
            draw_circle(fx(0.44), fy(0.44), sz * 0.04, dark);
            draw_circle(fx(0.56), fy(0.44), sz * 0.04, dark);
            draw_triangle(
                Vec2::new(fx(0.20), fy(0.94)),
                Vec2::new(fx(0.80), fy(0.94)),
                Vec2::new(fx(0.5), fy(0.58)),
                c,
            );
        }

        // ---- added when thirteen silhouettes were serving forty-eight
        // creatures. Each of these has to read differently in a 60px box at a
        // glance, so they are built around distinct outlines rather than
        // distinct details: a tall thing, a wide thing, a many-thing.

        // Francis. A crowned figure so laden with coin it has no edges left.
        MonsterSprite::Francis => {
            draw_ellipse(fx(0.5), fy(0.66), sz * 0.34, sz * 0.26, 0.0, c);
            draw_circle(fx(0.5), fy(0.34), sz * 0.15, c);
            // A heavy crown, five points.
            for i in 0..5 {
                let f = 0.30 + i as f32 * 0.10;
                draw_triangle(
                    Vec2::new(fx(f - 0.04), fy(0.22)),
                    Vec2::new(fx(f), fy(0.08)),
                    Vec2::new(fx(f + 0.04), fy(0.22)),
                    c,
                );
            }
            draw_rectangle(fx(0.26), fy(0.20), sz * 0.48, sz * 0.05, c);
            // Coins spilling down the front.
            for (a, b) in [(0.36, 0.56), (0.5, 0.62), (0.64, 0.56), (0.43, 0.72), (0.57, 0.72)] {
                draw_circle(fx(a), fy(b), sz * 0.055, dark);
            }
            draw_circle(fx(0.44), fy(0.33), sz * 0.025, dark);
            draw_circle(fx(0.56), fy(0.33), sz * 0.025, dark);
        }
        // A standard-bearer: one long pole, one hanging banner.
        MonsterSprite::Marshal => {
            draw_line(fx(0.34), fy(0.10), fx(0.34), fy(0.92), t * 1.4, c);
            draw_triangle(
                Vec2::new(fx(0.34), fy(0.12)),
                Vec2::new(fx(0.84), fy(0.24)),
                Vec2::new(fx(0.34), fy(0.46)),
                c,
            );
            draw_line(fx(0.42), fy(0.22), fx(0.68), fy(0.28), t, dark);
            draw_circle(fx(0.34), fy(0.60), sz * 0.11, c);
            draw_ellipse(fx(0.34), fy(0.80), sz * 0.16, sz * 0.12, 0.0, c);
        }
        // Nothing at all, with an edge around it.
        MonsterSprite::Null => {
            draw_circle_lines(fx(0.5), fy(0.5), sz * 0.36, t * 1.6, c);
            draw_circle_lines(fx(0.5), fy(0.5), sz * 0.22, t, dark);
            for i in 0..4 {
                let a = i as f32 * std::f32::consts::TAU / 4.0 + 0.4;
                draw_line(
                    fx(0.5) + a.cos() * sz * 0.36,
                    fy(0.5) + a.sin() * sz * 0.36,
                    fx(0.5) + a.cos() * sz * 0.48,
                    fy(0.5) + a.sin() * sz * 0.48,
                    t,
                    c,
                );
            }
        }
        // A lamp on a hook, with the last of the light in it.
        MonsterSprite::Lantern => {
            draw_line(fx(0.5), fy(0.06), fx(0.5), fy(0.22), t, c);
            draw_rectangle(fx(0.32), fy(0.22), sz * 0.36, sz * 0.06, c);
            draw_rectangle_lines(fx(0.30), fy(0.28), sz * 0.40, sz * 0.44, t * 1.4, c);
            draw_circle(fx(0.5), fy(0.50), sz * 0.11, c);
            draw_circle(fx(0.5), fy(0.50), sz * 0.05, dark);
            draw_rectangle(fx(0.32), fy(0.72), sz * 0.36, sz * 0.06, c);
            for f in [0.38f32, 0.62] {
                draw_line(fx(f), fy(0.78), fx(f), fy(0.92), t, c);
            }
        }
        // Three throats, one note.
        MonsterSprite::Choir => {
            for (i, f) in [0.22f32, 0.5, 0.78].iter().enumerate() {
                let top = 0.30 + (i as f32 - 1.0).abs() * 0.08;
                draw_circle(fx(*f), fy(top), sz * 0.11, c);
                draw_ellipse(fx(*f), fy(top + 0.30), sz * 0.13, sz * 0.22, 0.0, c);
                // Open mouths.
                draw_ellipse(fx(*f), fy(top + 0.05), sz * 0.03, sz * 0.05, 0.0, dark);
            }
        }
        // A mouth held shut, and nothing coming out of it.
        MonsterSprite::Silence => {
            draw_ellipse(fx(0.5), fy(0.46), sz * 0.28, sz * 0.36, 0.0, c);
            draw_line(fx(0.30), fy(0.52), fx(0.70), fy(0.52), t * 2.2, dark);
            draw_line(fx(0.36), fy(0.38), fx(0.46), fy(0.38), t, dark);
            draw_line(fx(0.54), fy(0.38), fx(0.64), fy(0.38), t, dark);
            // A hand across it.
            draw_line(fx(0.18), fy(0.62), fx(0.82), fy(0.58), t * 1.6, c);
            draw_ellipse(fx(0.5), fy(0.86), sz * 0.22, sz * 0.10, 0.0, c);
        }
        // Time, running out.
        MonsterSprite::Hourglass => {
            draw_rectangle(fx(0.24), fy(0.10), sz * 0.52, sz * 0.06, c);
            draw_rectangle(fx(0.24), fy(0.84), sz * 0.52, sz * 0.06, c);
            draw_triangle(
                Vec2::new(fx(0.28), fy(0.16)),
                Vec2::new(fx(0.72), fy(0.16)),
                Vec2::new(fx(0.5), fy(0.50)),
                c,
            );
            draw_triangle(
                Vec2::new(fx(0.28), fy(0.84)),
                Vec2::new(fx(0.72), fy(0.84)),
                Vec2::new(fx(0.5), fy(0.50)),
                c,
            );
            draw_triangle(
                Vec2::new(fx(0.36), fy(0.78)),
                Vec2::new(fx(0.64), fy(0.78)),
                Vec2::new(fx(0.5), fy(0.60)),
                dark,
            );
        }
        // A candle burnt most of the way down.
        MonsterSprite::Tallow => {
            draw_ellipse(fx(0.5), fy(0.24), sz * 0.07, sz * 0.14, 0.0, c);
            draw_circle(fx(0.5), fy(0.30), sz * 0.04, dark);
            draw_rectangle(fx(0.38), fy(0.40), sz * 0.24, sz * 0.42, c);
            // Wax running down one side.
            for (a, b) in [(0.36, 0.52), (0.36, 0.64), (0.64, 0.58)] {
                draw_circle(fx(a), fy(b), sz * 0.045, c);
            }
            draw_ellipse(fx(0.5), fy(0.86), sz * 0.30, sz * 0.08, 0.0, c);
        }
        // A face that has been crying long enough to wear a channel.
        MonsterSprite::Weeping => {
            draw_rectangle(fx(0.30), fy(0.18), sz * 0.40, sz * 0.56, c);
            draw_triangle(
                Vec2::new(fx(0.30), fy(0.18)),
                Vec2::new(fx(0.70), fy(0.18)),
                Vec2::new(fx(0.5), fy(0.04)),
                c,
            );
            draw_circle(fx(0.41), fy(0.36), sz * 0.05, dark);
            draw_circle(fx(0.59), fy(0.36), sz * 0.05, dark);
            for f in [0.41f32, 0.59] {
                for k in 0..3 {
                    draw_circle(fx(f), fy(0.46 + k as f32 * 0.10), sz * 0.03, dark);
                }
            }
            draw_ellipse(fx(0.5), fy(0.80), sz * 0.32, sz * 0.09, 0.0, c);
        }
        // Two figures bound at the wrist.
        MonsterSprite::Wedding => {
            for f in [0.32f32, 0.68] {
                draw_circle(fx(f), fy(0.28), sz * 0.11, c);
                draw_ellipse(fx(f), fy(0.60), sz * 0.14, sz * 0.24, 0.0, c);
            }
            draw_line(fx(0.32), fy(0.54), fx(0.68), fy(0.54), t * 1.4, c);
            draw_circle(fx(0.5), fy(0.54), sz * 0.07, dark);
            draw_circle_lines(fx(0.5), fy(0.54), sz * 0.07, t, c);
            // A veil over both.
            draw_triangle(
                Vec2::new(fx(0.18), fy(0.20)),
                Vec2::new(fx(0.82), fy(0.20)),
                Vec2::new(fx(0.5), fy(0.02)),
                c,
            );
        }
        // The same figure twice, one of them not quite right.
        MonsterSprite::Twin => {
            draw_circle(fx(0.36), fy(0.32), sz * 0.13, c);
            draw_ellipse(fx(0.36), fy(0.68), sz * 0.17, sz * 0.24, 0.0, c);
            draw_circle(fx(0.36), fy(0.30), sz * 0.03, dark);
            // The other, offset and hollow.
            draw_circle_lines(fx(0.64), fy(0.36), sz * 0.13, t, c);
            draw_ellipse(fx(0.64), fy(0.72), sz * 0.17, sz * 0.22, 0.0, dark);
            draw_circle(fx(0.64), fy(0.34), sz * 0.03, c);
        }
        // Tall, narrow, and giving nothing back.
        MonsterSprite::Mirror => {
            draw_rectangle_lines(fx(0.30), fy(0.06), sz * 0.40, sz * 0.86, t * 1.8, c);
            draw_rectangle(fx(0.34), fy(0.10), sz * 0.32, sz * 0.78, dark);
            // A long diagonal glint.
            draw_line(fx(0.36), fy(0.72), fx(0.64), fy(0.16), t * 1.2, c);
            draw_line(fx(0.36), fy(0.84), fx(0.50), fy(0.56), t * 0.8, c);
        }
        // A heavy body carrying a great many small ones.
        MonsterSprite::Sootmother => {
            draw_ellipse(fx(0.5), fy(0.60), sz * 0.36, sz * 0.30, 0.0, c);
            draw_circle(fx(0.5), fy(0.28), sz * 0.13, c);
            draw_circle(fx(0.45), fy(0.26), sz * 0.03, dark);
            draw_circle(fx(0.55), fy(0.26), sz * 0.03, dark);
            for (a, b) in [
                (0.24f32, 0.46f32), (0.34, 0.72), (0.5, 0.80), (0.66, 0.72), (0.76, 0.46),
            ] {
                draw_circle(fx(a), fy(b), sz * 0.07, dark);
                draw_circle(fx(a), fy(b), sz * 0.03, c);
            }
        }
        // Nine motes and nothing holding them together.
        MonsterSprite::Ashes => {
            for i in 0..9 {
                let a = i as f32 * std::f32::consts::TAU / 9.0;
                let r = if i % 2 == 0 { 0.34 } else { 0.20 };
                draw_circle(
                    fx(0.5) + a.cos() * sz * r,
                    fy(0.5) + a.sin() * sz * r,
                    sz * 0.06,
                    c,
                );
            }
            draw_circle(fx(0.5), fy(0.5), sz * 0.05, dark);
        }
        // A crown with nobody left under it.
        MonsterSprite::Crown => {
            for i in 0..5 {
                let f = 0.24 + i as f32 * 0.13;
                draw_triangle(
                    Vec2::new(fx(f - 0.05), fy(0.50)),
                    Vec2::new(fx(f), fy(0.22)),
                    Vec2::new(fx(f + 0.05), fy(0.50)),
                    c,
                );
            }
            draw_rectangle(fx(0.18), fy(0.48), sz * 0.64, sz * 0.10, c);
            draw_circle(fx(0.5), fy(0.53), sz * 0.04, dark);
            // The shape of a head, in outline only.
            draw_circle_lines(fx(0.5), fy(0.72), sz * 0.16, t, dark);
        }
        // A court, seen from under the water.
        MonsterSprite::Drowned => {
            for (f, top) in [(0.24f32, 0.44f32), (0.5, 0.36), (0.76, 0.46)] {
                draw_circle(fx(f), fy(top), sz * 0.10, c);
                draw_ellipse(fx(f), fy(top + 0.26), sz * 0.12, sz * 0.20, 0.0, c);
            }
            // The surface, above all of them: a solid band, because three thin
            // zigzags vanished at this size and left three floating blobs.
            draw_rectangle(fx(0.0), fy(0.06), sz, sz * 0.10, c);
            for i in 0..5 {
                let a = i as f32 * 0.20;
                draw_triangle(
                    Vec2::new(fx(a), fy(0.16)),
                    Vec2::new(fx(a + 0.20), fy(0.16)),
                    Vec2::new(fx(a + 0.10), fy(0.24)),
                    c,
                );
            }
            draw_line(fx(0.0), fy(0.06), fx(1.0), fy(0.06), t, dark);
        }
        // An anvil, with something beating inside it.
        MonsterSprite::Anvil => {
            draw_rectangle(fx(0.16), fy(0.32), sz * 0.68, sz * 0.20, c);
            draw_triangle(
                Vec2::new(fx(0.84), fy(0.32)),
                Vec2::new(fx(0.98), fy(0.40)),
                Vec2::new(fx(0.84), fy(0.52)),
                c,
            );
            draw_rectangle(fx(0.36), fy(0.52), sz * 0.28, sz * 0.20, c);
            draw_rectangle(fx(0.22), fy(0.72), sz * 0.56, sz * 0.16, c);
            // The heart of it.
            draw_circle(fx(0.42), fy(0.42), sz * 0.06, dark);
            draw_circle(fx(0.54), fy(0.42), sz * 0.06, dark);
            draw_triangle(
                Vec2::new(fx(0.36), fy(0.44)),
                Vec2::new(fx(0.60), fy(0.44)),
                Vec2::new(fx(0.48), fy(0.60)),
                dark,
            );
        }
        // A great many small figures, none of them in charge.
        MonsterSprite::Parliament => {
            for row in 0..3 {
                let n = 4 - row.min(1);
                for i in 0..n {
                    let f = 0.5 + (i as f32 - (n - 1) as f32 / 2.0) * 0.22;
                    let y = 0.30 + row as f32 * 0.22;
                    draw_circle(fx(f), fy(y), sz * 0.06, c);
                    draw_ellipse(fx(f), fy(y + 0.11), sz * 0.07, sz * 0.08, 0.0, c);
                }
            }
            draw_line(fx(0.08), fy(0.24), fx(0.92), fy(0.24), t, dark);
        }
        // Hooded, with a bell where a face should be.
        MonsterSprite::Abbot => {
            draw_triangle(
                Vec2::new(fx(0.20), fy(0.86)),
                Vec2::new(fx(0.80), fy(0.86)),
                Vec2::new(fx(0.5), fy(0.10)),
                c,
            );
            // The hood, hollow where a face would be.
            draw_ellipse(fx(0.5), fy(0.34), sz * 0.16, sz * 0.19, 0.0, dark);
            // The bell hangs in it. Cut out of the hood rather than laid over
            // it: drawn in the body colour it was invisible against its own
            // shoulders.
            draw_circle(fx(0.5), fy(0.30), sz * 0.09, c);
            draw_triangle(
                Vec2::new(fx(0.39), fy(0.44)),
                Vec2::new(fx(0.61), fy(0.44)),
                Vec2::new(fx(0.5), fy(0.24)),
                c,
            );
            draw_rectangle(fx(0.37), fy(0.44), sz * 0.26, sz * 0.05, dark);
            draw_circle(fx(0.5), fy(0.50), sz * 0.045, dark);
        }
        // A gearwright that has been gilded until it can barely turn.
        MonsterSprite::Gilt => {
            draw_poly(fx(0.5), fy(0.44), 8, sz * 0.30, 22.0, c);
            draw_circle(fx(0.5), fy(0.44), sz * 0.14, dark);
            draw_circle(fx(0.5), fy(0.44), sz * 0.06, c);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                draw_circle(
                    fx(0.5) + a.cos() * sz * 0.30,
                    fy(0.44) + a.sin() * sz * 0.30,
                    sz * 0.05,
                    c,
                );
            }
            // A heap of coin it is standing in.
            draw_ellipse(fx(0.5), fy(0.84), sz * 0.36, sz * 0.10, 0.0, c);
            for (a, b) in [(0.34f32, 0.80f32), (0.5, 0.86), (0.66, 0.80)] {
                draw_circle(fx(a), fy(b), sz * 0.05, dark);
            }
        }
        // A rat that has been given a crown and taken it seriously.
        MonsterSprite::Vermin => {
            draw_ellipse(fx(0.44), fy(0.66), sz * 0.28, sz * 0.19, 0.0, c);
            draw_circle(fx(0.72), fy(0.54), sz * 0.13, c);
            draw_triangle(
                Vec2::new(fx(0.82), fy(0.48)),
                Vec2::new(fx(0.99), fy(0.56)),
                Vec2::new(fx(0.82), fy(0.62)),
                c,
            );
            draw_circle(fx(0.77), fy(0.52), sz * 0.02, dark);
            for i in 0..3 {
                let f = 0.60 + i as f32 * 0.10;
                draw_triangle(
                    Vec2::new(fx(f - 0.04), fy(0.42)),
                    Vec2::new(fx(f), fy(0.26)),
                    Vec2::new(fx(f + 0.04), fy(0.42)),
                    c,
                );
            }
            draw_rectangle(fx(0.54), fy(0.40), sz * 0.30, sz * 0.05, c);
            draw_line(fx(0.18), fy(0.68), fx(0.04), fy(0.46), t, c);
        }
        // A toad that kept going.
        MonsterSprite::Behemoth => {
            draw_ellipse(fx(0.5), fy(0.62), sz * 0.44, sz * 0.30, 0.0, c);
            draw_ellipse(fx(0.5), fy(0.36), sz * 0.28, sz * 0.16, 0.0, c);
            draw_circle(fx(0.36), fy(0.30), sz * 0.08, c);
            draw_circle(fx(0.64), fy(0.30), sz * 0.08, c);
            draw_circle(fx(0.36), fy(0.30), sz * 0.035, dark);
            draw_circle(fx(0.64), fy(0.30), sz * 0.035, dark);
            draw_line(fx(0.32), fy(0.44), fx(0.68), fy(0.44), t * 1.4, dark);
            // Squat legs, wide apart.
            for f in [0.16f32, 0.84] {
                draw_ellipse(fx(f), fy(0.80), sz * 0.12, sz * 0.09, 0.0, c);
            }
        }
        // A skeleton mid-note.
        MonsterSprite::Cantor => {
            draw_circle(fx(0.5), fy(0.26), sz * 0.14, c);
            draw_ellipse(fx(0.5), fy(0.33), sz * 0.05, sz * 0.07, 0.0, dark);
            draw_circle(fx(0.44), fy(0.22), sz * 0.035, dark);
            draw_circle(fx(0.56), fy(0.22), sz * 0.035, dark);
            draw_line(fx(0.5), fy(0.40), fx(0.5), fy(0.78), t * 1.6, c);
            for i in 0..4 {
                let y = 0.46 + i as f32 * 0.09;
                draw_line(fx(0.34), fy(y), fx(0.66), fy(y), t, c);
            }
            // The note going up.
            draw_circle(fx(0.82), fy(0.24), sz * 0.05, c);
            draw_line(fx(0.86), fy(0.24), fx(0.86), fy(0.08), t, c);
        }
        // A warm mote. Same family as the frost wisp, opposite temperament:
        // curling arms rather than straight spines.
        MonsterSprite::Ember => {
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                let mx0 = fx(0.5) + a.cos() * sz * 0.22;
                let my0 = fy(0.5) + a.sin() * sz * 0.22;
                let a2 = a + 0.9;
                draw_line(
                    mx0,
                    my0,
                    fx(0.5) + a2.cos() * sz * 0.44,
                    fy(0.5) + a2.sin() * sz * 0.44,
                    t,
                    c,
                );
            }
            draw_circle(fx(0.5), fy(0.5), sz * 0.16, c);
            draw_circle(fx(0.5), fy(0.5), sz * 0.07, dark);
        }
        // The wisp's elder: a broad frost figure rather than a mote.
        MonsterSprite::Rimefather => {
            draw_ellipse(fx(0.5), fy(0.64), sz * 0.34, sz * 0.28, 0.0, c);
            draw_circle(fx(0.5), fy(0.30), sz * 0.16, c);
            draw_circle(fx(0.44), fy(0.28), sz * 0.03, dark);
            draw_circle(fx(0.56), fy(0.28), sz * 0.03, dark);
            // A beard of icicles.
            for i in 0..5 {
                let f = 0.32 + i as f32 * 0.09;
                draw_triangle(
                    Vec2::new(fx(f - 0.03), fy(0.42)),
                    Vec2::new(fx(f + 0.03), fy(0.42)),
                    Vec2::new(fx(f), fy(0.42 + 0.16 + (i as f32 % 2.0) * 0.10)),
                    c,
                );
            }
            for (a, b) in [(0.14f32, 0.36f32), (0.86, 0.36)] {
                draw_line(fx(a), fy(b), fx(a), fy(b + 0.24), t, c);
            }
        }
        // A warden running molten.
        MonsterSprite::Slag => {
            draw_rectangle(fx(0.30), fy(0.24), sz * 0.40, sz * 0.52, c);
            draw_triangle(
                Vec2::new(fx(0.30), fy(0.24)),
                Vec2::new(fx(0.70), fy(0.24)),
                Vec2::new(fx(0.5), fy(0.06)),
                c,
            );
            draw_rectangle(fx(0.38), fy(0.34), sz * 0.24, sz * 0.06, dark);
            // Running off the bottom edge.
            for (i, f) in [0.34f32, 0.48, 0.62].iter().enumerate() {
                let d = 0.76 + i as f32 * 0.04;
                draw_line(fx(*f), fy(0.76), fx(*f), fy(d + 0.14), t * 1.4, c);
                draw_circle(fx(*f), fy(d + 0.14), sz * 0.045, c);
            }
        }
        // All angles and no curves.
        MonsterSprite::Obsidian => {
            draw_triangle(
                Vec2::new(fx(0.5), fy(0.04)),
                Vec2::new(fx(0.90), fy(0.56)),
                Vec2::new(fx(0.10), fy(0.56)),
                c,
            );
            draw_triangle(
                Vec2::new(fx(0.10), fy(0.56)),
                Vec2::new(fx(0.90), fy(0.56)),
                Vec2::new(fx(0.5), fy(0.94)),
                c,
            );
            draw_line(fx(0.5), fy(0.04), fx(0.5), fy(0.94), t, dark);
            draw_line(fx(0.10), fy(0.56), fx(0.90), fy(0.56), t, dark);
            draw_circle(fx(0.38), fy(0.42), sz * 0.04, dark);
            draw_circle(fx(0.62), fy(0.42), sz * 0.04, dark);
        }
        // A frame, and something hanging in it.
        MonsterSprite::Gallows => {
            draw_line(fx(0.16), fy(0.08), fx(0.16), fy(0.92), t * 1.6, c);
            draw_line(fx(0.16), fy(0.08), fx(0.76), fy(0.08), t * 1.6, c);
            draw_line(fx(0.16), fy(0.20), fx(0.30), fy(0.08), t, c);
            draw_line(fx(0.70), fy(0.08), fx(0.70), fy(0.30), t, c);
            draw_circle(fx(0.70), fy(0.42), sz * 0.12, c);
            draw_ellipse(fx(0.70), fy(0.68), sz * 0.14, sz * 0.18, 0.0, c);
            draw_circle(fx(0.66), fy(0.40), sz * 0.025, dark);
            draw_circle(fx(0.74), fy(0.40), sz * 0.025, dark);
            draw_ellipse(fx(0.5), fy(0.92), sz * 0.42, sz * 0.05, 0.0, c);
        }
        // A gearwright that took orders.
        MonsterSprite::CogPriest => {
            draw_triangle(
                Vec2::new(fx(0.22), fy(0.88)),
                Vec2::new(fx(0.78), fy(0.88)),
                Vec2::new(fx(0.5), fy(0.16)),
                c,
            );
            draw_poly(fx(0.5), fy(0.36), 6, sz * 0.13, 0.0, dark);
            draw_circle(fx(0.5), fy(0.36), sz * 0.05, c);
            // A smaller cog turning off it.
            draw_poly(fx(0.72), fy(0.56), 6, sz * 0.08, 20.0, c);
            draw_poly(fx(0.28), fy(0.60), 6, sz * 0.07, 12.0, c);
            draw_line(fx(0.5), fy(0.16), fx(0.5), fy(0.04), t, c);
            draw_line(fx(0.42), fy(0.08), fx(0.58), fy(0.08), t, c);
        }
        // The hound, gone to pieces and still coming.
        MonsterSprite::RuinHound => {
            draw_ellipse(fx(0.46), fy(0.54), sz * 0.28, sz * 0.13, 0.0, c);
            // Ribs showing through.
            for i in 0..4 {
                let f = 0.30 + i as f32 * 0.11;
                draw_line(fx(f), fy(0.46), fx(f), fy(0.62), t, dark);
            }
            draw_circle(fx(0.76), fy(0.48), sz * 0.11, c);
            draw_triangle(
                Vec2::new(fx(0.84), fy(0.46)),
                Vec2::new(fx(0.99), fy(0.52)),
                Vec2::new(fx(0.84), fy(0.58)),
                c,
            );
            draw_circle(fx(0.80), fy(0.45), sz * 0.02, dark);
            // Three legs, and a gap where a fourth was.
            for f in [0.28f32, 0.48, 0.66] {
                draw_line(fx(f), fy(0.64), fx(f), fy(0.86), t, c);
            }
            draw_line(fx(0.18), fy(0.50), fx(0.04), fy(0.38), t, c);
        }

        // A pillar of salt, cracked through.
        MonsterSprite::Salt => {
            draw_triangle(
                Vec2::new(fx(0.30), fy(0.92)),
                Vec2::new(fx(0.70), fy(0.92)),
                Vec2::new(fx(0.5), fy(0.08)),
                c,
            );
            for (a, b, d, e) in [
                (0.44f32, 0.30f32, 0.56f32, 0.44f32),
                (0.56, 0.50, 0.40, 0.64),
                (0.40, 0.70, 0.60, 0.82),
            ] {
                draw_line(fx(a), fy(b), fx(d), fy(e), t, dark);
            }
            // Grains coming off it.
            for (a, b) in [(0.20f32, 0.60f32), (0.82, 0.52), (0.16, 0.80), (0.86, 0.76)] {
                draw_circle(fx(a), fy(b), sz * 0.03, c);
            }
        }
        // Metal that has gone green and kept its shape anyway.
        MonsterSprite::Verdigris => {
            draw_rectangle(fx(0.28), fy(0.26), sz * 0.44, sz * 0.56, c);
            draw_triangle(
                Vec2::new(fx(0.28), fy(0.26)),
                Vec2::new(fx(0.72), fy(0.26)),
                Vec2::new(fx(0.5), fy(0.08)),
                c,
            );
            // Bloom eating into the edges.
            for (a, b, r) in [
                (0.28f32, 0.40f32, 0.07f32),
                (0.72, 0.34, 0.06),
                (0.30, 0.66, 0.08),
                (0.70, 0.72, 0.07),
                (0.50, 0.82, 0.06),
            ] {
                draw_circle(fx(a), fy(b), sz * r, dark);
            }
            draw_rectangle(fx(0.38), fy(0.36), sz * 0.24, sz * 0.05, dark);
        }
        // A column on the move, seen end-on.
        MonsterSprite::March => {
            for i in 0..4 {
                let f = 0.20 + i as f32 * 0.20;
                let y = 0.34 + (i as f32 % 2.0) * 0.06;
                draw_circle(fx(f), fy(y), sz * 0.08, c);
                draw_ellipse(fx(f), fy(y + 0.20), sz * 0.09, sz * 0.15, 0.0, c);
                // Spears, all at the same angle.
                draw_line(fx(f + 0.06), fy(y + 0.30), fx(f + 0.12), fy(y - 0.22), t, c);
            }
            draw_ellipse(fx(0.5), fy(0.90), sz * 0.44, sz * 0.05, 0.0, dark);
        }
        // Bells where a body should be.
        MonsterSprite::Bells => {
            // Hung from a beam, so the silhouette is a row under a line rather
            // than a heap of triangles - which is what it read as before.
            draw_line(fx(0.06), fy(0.14), fx(0.94), fy(0.14), t * 1.6, c);
            for (f, drop, r) in [(0.24f32, 0.30f32, 0.13f32), (0.52, 0.24, 0.17), (0.80, 0.34, 0.11)] {
                let top = 0.14 + drop * 0.35;
                draw_line(fx(f), fy(0.14), fx(f), fy(top), t, c);
                // A rounded shoulder and a flared mouth: a bell, not a cone.
                draw_circle(fx(f), fy(top + r * 0.5), sz * r * 0.7, c);
                draw_triangle(
                    Vec2::new(fx(f - r), fy(top + r * 1.4)),
                    Vec2::new(fx(f + r), fy(top + r * 1.4)),
                    Vec2::new(fx(f), fy(top)),
                    c,
                );
                draw_rectangle(
                    fx(f - r) ,
                    fy(top + r * 1.4),
                    sz * r * 2.0,
                    sz * 0.045,
                    c,
                );
                // The clapper, showing under the mouth.
                draw_circle(fx(f), fy(top + r * 1.4 + 0.05), sz * 0.035, c);
            }
        }

        // Bigger than the golem it grew out of, and put together worse: a
        // stacked, top-heavy thing rather than a squat one.
        MonsterSprite::Colossus => {
            draw_rectangle(fx(0.18), fy(0.10), sz * 0.64, sz * 0.30, c);
            draw_rectangle(fx(0.28), fy(0.42), sz * 0.44, sz * 0.24, c);
            draw_rectangle(fx(0.34), fy(0.68), sz * 0.32, sz * 0.24, c);
            // Seams where the courses meet.
            draw_line(fx(0.18), fy(0.40), fx(0.82), fy(0.40), t, dark);
            draw_line(fx(0.28), fy(0.66), fx(0.72), fy(0.66), t, dark);
            draw_circle(fx(0.36), fy(0.24), sz * 0.05, dark);
            draw_circle(fx(0.64), fy(0.24), sz * 0.05, dark);
            // Arms hanging off the widest course.
            draw_rectangle(fx(0.06), fy(0.16), sz * 0.10, sz * 0.34, c);
            draw_rectangle(fx(0.84), fy(0.16), sz * 0.10, sz * 0.34, c);
        }
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

// You against them, in a pair that survives colour blindness. Green versus
// red is the distinction red-green colour blindness is named for: simulated,
// the two health bars came out the same shade of khaki, and the same grey in
// full monochromacy. Blue against orange stays apart under all three
// dichromacies, and these two are set far enough apart in brightness to stay
// apart with no colour at all.
fn col_you() -> Color {
    Color::from_rgba(116, 184, 246, 255)
}
fn col_you_dim() -> Color {
    Color::from_rgba(74, 118, 160, 255)
}
fn col_foe() -> Color {
    Color::from_rgba(198, 108, 30, 255)
}
fn col_foe_dim() -> Color {
    Color::from_rgba(132, 74, 26, 255)
}

// A piece's tile has to answer two questions - which slot does this belong
// to, and which part of the recipe is it - and it has to answer both without
// relying on colour, because some players see none of it.
//
//   slot -> a motif stamped on every cell   (see `slot_motif`)
//   role -> the tile's lightness            (see `kind_lightness`)
//
// Both survive being reduced to greyscale. Hue then repeats the slot on top,
// for players who do see colour, which is why the hues below are the
// Color Universal Design set rather than an even spread around the wheel:
// evenly spaced hues collapse into pairs under red-green colour blindness.

/// Hue per slot, from the Okabe-Ito colour-blind-safe palette: vermillion,
/// sky blue, bluish green, reddish purple and yellow.
fn slot_hue(slot: SlotKind) -> f32 {
    match slot {
        SlotKind::Weapon => 0.073,
        SlotKind::Helmet => 0.552,
        SlotKind::Chest => 0.443,
        SlotKind::Gloves => 0.912,
        SlotKind::Greaves => 0.156,
    }
}

/// Saturation per slot. Okabe-Ito's colours are not equally saturated, and
/// evening them out is what pushes the blue pair and the warm pair together.
fn slot_sat(slot: SlotKind) -> f32 {
    match slot {
        SlotKind::Weapon => 0.80,
        SlotKind::Helmet => 0.68,
        SlotKind::Chest => 0.72,
        SlotKind::Gloves => 0.44,
        SlotKind::Greaves => 0.74,
    }
}

/// Brightness per role, so the piece a recipe is built around reads darkest.
/// This is the channel that carries the role once colour is gone, so the three
/// steps are stated as brightness rather than as HSL lightness: the same
/// lightness lands at wildly different brightness depending on the hue, and
/// yellow in particular flattens its top two steps into one.
fn kind_luminance(kind: PieceKind) -> f32 {
    match kind {
        // Cores darkest, the middle of a recipe next, the trim lightest. A
        // book or an orb anchors a spell exactly as a handle anchors a weapon,
        // so it reads at the same brightness.
        PieceKind::Handle
        | PieceKind::Frame
        | PieceKind::Base
        | PieceKind::Material
        | PieceKind::Book
        | PieceKind::Orb => 0.22,
        PieceKind::Damaging
        | PieceKind::Plating
        | PieceKind::Layer
        | PieceKind::Mold
        | PieceKind::Ink => 0.45,
        PieceKind::Accessory
        | PieceKind::Crest
        | PieceKind::Spell
        | PieceKind::Ring
        | PieceKind::Alignment => 0.72,
        // Terrain is drawn beneath the grid, so it wants to read as ground
        // rather than as gear: lighter than anything standing on it.
        PieceKind::Terrain => 0.85,
    }
}


/// A slot's hue at a given brightness. Luminance rises monotonically with HSL
/// lightness, so a short bisection lands on the lightness that hits the target
/// whatever the hue happens to be worth.
fn slot_color(slot: SlotKind, target: f32) -> Color {
    let (hue, sat) = (slot_hue(slot), slot_sat(slot));
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    let mut out = macroquad::color::hsl_to_rgb(hue, sat, 0.5);
    for _ in 0..16 {
        let mid = 0.5 * (lo + hi);
        out = macroquad::color::hsl_to_rgb(hue, sat, mid);
        if luminance(out) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    out
}

/// Perceived brightness, 0..1. Used to pick an ink that will actually show up
/// on a tile whatever colour it is - and to check, in the tests, that the
/// roles stay apart once colour is taken away.
fn luminance(c: Color) -> f32 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
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

/// The shape stamped on every cell of a piece. This is the channel that says
/// which slot a tile belongs to when colour says nothing at all, so the five
/// have to stay distinct from each other - which is what the tests check.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Motif {
    /// A blade's edge.
    Diagonal,
    /// A helm's dome.
    Dome,
    /// The bands of a cuirass.
    Bands,
    /// A gauntlet's weave.
    Weave,
    /// The straps of a greave.
    Straps,
    /// Not any one slot's mark: the piece fits more than one grid and has not
    /// been put in either yet.
    Shared,
}

fn slot_motif(slot: SlotKind) -> Motif {
    match slot {
        SlotKind::Weapon => Motif::Diagonal,
        SlotKind::Helmet => Motif::Dome,
        SlotKind::Chest => Motif::Bands,
        SlotKind::Gloves => Motif::Weave,
        SlotKind::Greaves => Motif::Straps,
    }
}

/// The grey a shared piece wears before it is placed, at the brightness its
/// role calls for. Grey has no hue to carry a slot, which is the point - a
/// steel material is not a glove or a greave until it is in one. Role
/// brightness still reads, so the three-step role scale survives.
fn unplaced_color(kind: PieceKind) -> Color {
    // luminance() weights sum to 1, so a neutral grey's luminance is its own
    // channel value and no bisection is needed.
    let l = kind_luminance(kind);
    Color::new(l, l, l, 1.0)
}

/// Stamp a slot's motif into one cell. Sized to still read at the 15px cells
/// the shop cards use.
/// Rage, faith and nature, in a fixed order. Mana is tracked separately
/// because it predates the other three.
fn pool_index(what: &str) -> Option<usize> {
    match what {
        "rage" => Some(0),
        "faith" => Some(1),
        "nature" => Some(2),
        // The fusions. A player can hold Druidic Might and the interface said
        // nothing about it at all, which for a pool worth four ordinary points
        // a point is the one thing it cannot afford to be quiet about.
        "druidic might" => Some(3),
        "communion" => Some(4),
        "zealotry" => Some(5),
        _ => None,
    }
}

/// Every keyword a component touches, so a card can show at a glance what it
/// deals in without anyone reading the tooltip.
///
/// Derived from what the piece actually carries - its stats, its adjacency
/// bonus, its effect and its triggers - rather than from a hand-written list,
/// so a new component is described correctly the moment it exists.
fn note(k: &'static str, out: &mut Vec<&'static str>) {
    if !out.contains(&k) {
        out.push(k);
    }
}

fn keywords_of(def: &PieceDef) -> Vec<&'static str> {
    use gearmaster_engine::piece::{Action, Trigger};
    let mut out: Vec<&'static str> = Vec::new();
    fn from_stats(st: &gearmaster_engine::stats::Stats, out: &mut Vec<&'static str>) {
        for (v, k) in [
            (st.mana, "mana"),
            (st.rage, "rage"),
            (st.faith, "faith"),
            (st.nature, "nature"),
            (st.armor, "armor"),
            (st.mind, "mind"),
            (st.magic_damage + st.magic_resist + st.magic_pierce + st.magic_harden, "magic"),
            (st.physical_damage + st.physical_resist + st.physical_pierce + st.physical_harden, "physical"),
        ] {
            if v != 0 {
                note(k, out);
            }
        }
    }
    from_stats(&def.base, &mut out);
    if let Some(a) = def.adjacency {
        from_stats(&a.stats, &mut out);
    }
    if let Some(e) = def.effect {
        if let gearmaster_engine::piece::EffectKind::Flat { stats } = e.kind {
            from_stats(&stats, &mut out);
        }
    }
    if def.speed_bonus != 0 {
        note("speed", &mut out);
    }
    if def.quest.is_some() {
        note("quest", &mut out);
    }

    fn from_action(a: &Action, out: &mut Vec<&'static str>) { match a {
        Action::Curse { .. } => note("curse", out),
        Action::StunStrongest { .. } => note("stun", out),
        Action::Drain { what, .. } => note(
            what.name(),
            out,
        ),
        Action::GainMana(_) => note("mana", out),
        Action::Gain { what, .. } => note(
            what.name(),
            out,
        ),
        Action::GainArmor(_) => note("armor", out),
        Action::MindDamage { .. } => note("mind", out),
        Action::Damage { .. } => note("damage", out),
        Action::ReduceCooldown(_) => note("speed", out),
        Action::GainEmpowerment(_) | Action::GainShield(_) | Action::GainForking(_) => {
            note("mana", out)
        }
        Action::Grow(_) => note("health", out),
        Action::Fuse { into, .. } => note(into.name(), out),
    } }
    // A repeat carries a trigger, so unwrap it once and read that: a piece
    // whose only trigger is a repeat would otherwise show no icons at all.
    let unwrap = |t: &'static Trigger| -> &'static Trigger {
        match t {
            Trigger::PerAdjacentEmpty(inner) => inner,
            other => other,
        }
    };
    for t in def.triggers.iter().map(unwrap) {
        match t {
            Trigger::PerAdjacentEmpty(_) => {}
            Trigger::OnActivate(a)
            | Trigger::OnBattleStart(a)
            | Trigger::PerAdjacentItem { action: a, .. }
            | Trigger::OnAdjacentActivate(a)
            | Trigger::OnAlignedActivate(a)
            | Trigger::OnDiagonalActivate(a)
            | Trigger::OnOtherCast(a) => from_action(a, &mut out),
            Trigger::Watch { then, .. } => from_action(then, &mut out),
            Trigger::SpendGold { on_success, .. } => {
                note("fnorp", &mut out);
                from_action(on_success, &mut out);
            }
            Trigger::SpendMana { on_success, on_failure, .. } => {
                note("mana", &mut out);
                from_action(on_success, &mut out);
                from_action(on_failure, &mut out);
            }
            Trigger::Consume { what, per, .. } => {
                note(
                    what.name(),
                    &mut out,
                );
                from_action(per, &mut out);
            }
            Trigger::Spend { what, on_success, on_failure, .. } => {
                note(
                    what.name(),
                    &mut out,
                );
                from_action(on_success, &mut out);
                from_action(on_failure, &mut out);
            }
        }
    }
    out
}

/// One keyword's mark. The four pools reuse the glyphs the battle screen
/// draws, so the same thing looks the same wherever it appears.
fn draw_keyword(x: f32, y: f32, s: f32, keyword: &str) {
    let c = keyword_color(keyword);
    match keyword {
        "mana" | "rage" | "faith" | "nature" | "armor" => draw_pool_glyph(x, y, s, keyword, c),
        // A curse: a crooked horn.
        "curse" => {
            draw_line(x + s * 0.20, y + s * 0.85, x + s * 0.5, y + s * 0.15, s * 0.14, c);
            draw_line(x + s * 0.5, y + s * 0.15, x + s * 0.80, y + s * 0.55, s * 0.14, c);
        }
        // Speed: a chevron.
        "speed" => {
            draw_line(x + s * 0.20, y + s * 0.20, x + s * 0.70, y + s * 0.5, s * 0.14, c);
            draw_line(x + s * 0.70, y + s * 0.5, x + s * 0.20, y + s * 0.80, s * 0.14, c);
        }
        // Mind: a spiral of sorts.
        "mind" => {
            draw_circle_lines(x + s * 0.5, y + s * 0.5, s * 0.34, s * 0.12, c);
            draw_circle(x + s * 0.5, y + s * 0.5, s * 0.12, c);
        }
        // Magic: a four-pointed star.
        "magic" => {
            draw_triangle(
                Vec2::new(x + s * 0.5, y),
                Vec2::new(x + s * 0.64, y + s * 0.5),
                Vec2::new(x + s * 0.36, y + s * 0.5),
                c,
            );
            draw_triangle(
                Vec2::new(x + s * 0.5, y + s),
                Vec2::new(x + s * 0.64, y + s * 0.5),
                Vec2::new(x + s * 0.36, y + s * 0.5),
                c,
            );
            draw_line(x, y + s * 0.5, x + s, y + s * 0.5, s * 0.14, c);
        }
        // Physical: a blunt wedge.
        "physical" => {
            draw_triangle(
                Vec2::new(x + s * 0.5, y + s * 0.05),
                Vec2::new(x + s * 0.9, y + s * 0.95),
                Vec2::new(x + s * 0.1, y + s * 0.95),
                c,
            );
        }
        // A quest: a small flag.
        "quest" => {
            draw_line(x + s * 0.25, y + s * 0.05, x + s * 0.25, y + s * 0.95, s * 0.13, c);
            draw_triangle(
                Vec2::new(x + s * 0.30, y + s * 0.10),
                Vec2::new(x + s * 0.88, y + s * 0.32),
                Vec2::new(x + s * 0.30, y + s * 0.55),
                c,
            );
        }
        // Plain damage: a slash.
        _ => draw_line(x + s * 0.18, y + s * 0.82, x + s * 0.82, y + s * 0.18, s * 0.16, c),
    }
}

fn keyword_color(k: &str) -> Color {
    match k {
        "mana" | "rage" | "faith" | "nature" | "armor" => pool_color(k),
        "curse" => col_trigger(),
        "speed" => Color::from_rgba(150, 220, 240, 255),
        "mind" => Color::from_rgba(200, 160, 220, 255),
        "magic" => Color::from_rgba(170, 150, 245, 255),
        "physical" => Color::from_rgba(226, 150, 110, 255),
        "quest" => Color::from_rgba(150, 220, 190, 255),
        _ => Color::from_rgba(220, 200, 180, 255),
    }
}

/// A small mark for each banked pool, drawn from primitives so the four read
/// apart at a glance instead of being four numbers in a row.
fn draw_pool_glyph(x: f32, y: f32, s: f32, which: &str, c: Color) {
    let t = (s * 0.14).max(1.5);
    match which {
        // Mana: a droplet.
        "mana" => {
            draw_circle(x + s * 0.5, y + s * 0.62, s * 0.28, c);
            draw_triangle(
                Vec2::new(x + s * 0.26, y + s * 0.62),
                Vec2::new(x + s * 0.74, y + s * 0.62),
                Vec2::new(x + s * 0.5, y + s * 0.10),
                c,
            );
        }
        // Rage: a jagged spark.
        "rage" => {
            draw_triangle(
                Vec2::new(x + s * 0.58, y + s * 0.05),
                Vec2::new(x + s * 0.20, y + s * 0.58),
                Vec2::new(x + s * 0.50, y + s * 0.52),
                c,
            );
            draw_triangle(
                Vec2::new(x + s * 0.50, y + s * 0.48),
                Vec2::new(x + s * 0.80, y + s * 0.42),
                Vec2::new(x + s * 0.42, y + s * 0.95),
                c,
            );
        }
        // Faith: a cross.
        "faith" => {
            draw_line(x + s * 0.5, y + s * 0.06, x + s * 0.5, y + s * 0.94, t, c);
            draw_line(x + s * 0.22, y + s * 0.36, x + s * 0.78, y + s * 0.36, t, c);
        }
        // Nature: a leaf on a stem.
        "nature" => {
            draw_line(x + s * 0.5, y + s * 0.95, x + s * 0.5, y + s * 0.35, t, c);
            draw_triangle(
                Vec2::new(x + s * 0.5, y + s * 0.05),
                Vec2::new(x + s * 0.88, y + s * 0.48),
                Vec2::new(x + s * 0.5, y + s * 0.58),
                c,
            );
            draw_triangle(
                Vec2::new(x + s * 0.5, y + s * 0.05),
                Vec2::new(x + s * 0.12, y + s * 0.48),
                Vec2::new(x + s * 0.5, y + s * 0.58),
                c,
            );
        }
        // Armour: a shield.
        _ => {
            draw_triangle(
                Vec2::new(x + s * 0.12, y + s * 0.12),
                Vec2::new(x + s * 0.88, y + s * 0.12),
                Vec2::new(x + s * 0.5, y + s * 0.95),
                c,
            );
            draw_rectangle(x + s * 0.12, y + s * 0.10, s * 0.76, s * 0.22, c);
        }
    }
}

fn pool_color(which: &str) -> Color {
    match which {
        "mana" => Color::from_rgba(140, 200, 240, 255),
        "rage" => Color::from_rgba(232, 108, 92, 255),
        "faith" => Color::from_rgba(240, 208, 120, 255),
        "nature" => Color::from_rgba(140, 220, 150, 255),
        // Each fusion is painted between its parents: nature-and-rage comes
        // out amber, faith-and-nature pale green-gold, rage-and-faith orange.
        "druidic might" => Color::from_rgba(196, 168, 90, 255),
        "communion" => Color::from_rgba(196, 214, 130, 255),
        "zealotry" => Color::from_rgba(238, 158, 96, 255),
        _ => Color::from_rgba(170, 190, 220, 255),
    }
}

/// Stamp the mark for the grid a piece is sitting in, or the shared mark if it
/// is not in one yet.
fn draw_motif(x: f32, y: f32, cell: f32, motif: Motif, ink: Color) {
    let t = (cell * 0.11).max(1.5);
    match motif {
        Motif::Diagonal => {
            draw_line(x + cell * 0.24, y + cell * 0.76, x + cell * 0.76, y + cell * 0.24, t, ink);
        }
        Motif::Dome => {
            draw_circle(x + cell * 0.5, y + cell * 0.52, cell * 0.20, ink);
        }
        Motif::Bands => {
            for row in [0.34f32, 0.64] {
                draw_line(x + cell * 0.22, y + cell * row, x + cell * 0.78, y + cell * row, t, ink);
            }
        }
        Motif::Weave => {
            draw_line(x + cell * 0.5, y + cell * 0.22, x + cell * 0.5, y + cell * 0.78, t, ink);
            draw_line(x + cell * 0.22, y + cell * 0.5, x + cell * 0.78, y + cell * 0.5, t, ink);
        }
        Motif::Straps => {
            for col in [0.34f32, 0.64] {
                draw_line(x + cell * col, y + cell * 0.22, x + cell * col, y + cell * 0.78, t, ink);
            }
        }
        // A hollow diamond: no straight run and no filled centre, so it is not
        // mistaken for any of the five slot marks at a 15px shop cell.
        Motif::Shared => {
            let (cx, cy, r) = (x + cell * 0.5, y + cell * 0.5, cell * 0.26);
            let pts = [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
            for i in 0..4 {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % 4];
                draw_line(ax, ay, bx, by, t, ink);
            }
        }
    }
}

/// The mark and fill a piece wears: its grid's if it is in one, the shared
/// mark and no colour at all if it fits several and is in none.
fn piece_look(def: &PieceDef, worn_in: Option<SlotKind>) -> (Color, Motif) {
    match worn_in {
        Some(slot) => (slot_color(slot, kind_luminance(def.kind)), slot_motif(slot)),
        // A piece that only ever goes one place is drawn as that place even
        // when it is loose - there is no ambiguity to represent.
        None if !def.shared() => {
            (slot_color(def.slot, kind_luminance(def.kind)), slot_motif(def.slot))
        }
        None => (unplaced_color(def.kind), Motif::Shared),
    }
}

/// Colour of a rarity badge. Brightness climbs with the tier so the pips read
/// as a rank without needing the colours told apart.
fn rarity_color(r: Rarity) -> Color {
    match r {
        Rarity::Common => col_dim(),
        Rarity::Rare => Color::from_rgba(120, 186, 240, 255),
        Rarity::Epic => Color::from_rgba(196, 150, 244, 255),
        Rarity::Legendary => Color::from_rgba(250, 206, 110, 255),
    }
}

/// The badge an item wears: one pip for rare, two for epic, three for
/// legendary. Diamonds rather than dots, so it does not read as one of the
/// component markers. Returns the width it used.
fn draw_rarity_pips(x: f32, y: f32, r: Rarity, scale: f32) -> f32 {
    let n = r.marks();
    if n == 0 {
        return 0.0;
    }
    let c = rarity_color(r);
    let rad = 4.0 * scale;
    let step = rad * 2.4;
    for i in 0..n {
        let cx = x + rad + i as f32 * step;
        draw_triangle(
            Vec2::new(cx, y - rad),
            Vec2::new(cx + rad, y),
            Vec2::new(cx, y + rad),
            c,
        );
        draw_triangle(
            Vec2::new(cx, y - rad),
            Vec2::new(cx, y + rad),
            Vec2::new(cx - rad, y),
            c,
        );
    }
    n as f32 * step
}

/// The three things a component can carry. Each has its own corner of the
/// tile, its own shape and its own colour, so no one of the three is doing the
/// work alone.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Marker {
    /// An assembly bonus: fires when the item comes together. Top-right.
    Bonus,
    /// A positional effect: worth something depending on where it sits.
    /// Bottom-right.
    Effect,
    /// A combat trigger. Bottom-left.
    Trigger,
}

impl Marker {
    fn color(self) -> Color {
        match self {
            Marker::Bonus => col_gold(),
            Marker::Effect => col_effect(),
            Marker::Trigger => col_trigger(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Marker::Bonus => "assembly bonus",
            Marker::Effect => "positional effect",
            Marker::Trigger => "combat trigger",
        }
    }
}

/// Draw one marker at `(cx, cy)`. `lit` means its condition currently holds:
/// lit markers are solid, unlit ones are the same shape in outline, so the
/// difference is fill and not hue.
fn draw_marker(cx: f32, cy: f32, marker: Marker, lit: bool) {
    draw_marker_sized(cx, cy, 4.6, marker, lit)
}

fn draw_marker_sized(cx: f32, cy: f32, r: f32, marker: Marker, lit: bool) {
    let c = if lit { marker.color() } else { col_dim() };
    match marker {
        // A disc.
        Marker::Bonus => {
            if lit {
                draw_circle(cx, cy, r, c);
            } else {
                draw_circle_lines(cx, cy, r, 1.5, c);
            }
        }
        // A diamond.
        Marker::Effect => {
            let pts = [
                Vec2::new(cx, cy - r),
                Vec2::new(cx + r, cy),
                Vec2::new(cx, cy + r),
                Vec2::new(cx - r, cy),
            ];
            if lit {
                draw_triangle(pts[0], pts[1], pts[2], c);
                draw_triangle(pts[0], pts[2], pts[3], c);
            } else {
                for i in 0..4 {
                    draw_line(pts[i].x, pts[i].y, pts[(i + 1) % 4].x, pts[(i + 1) % 4].y, 1.5, c);
                }
            }
        }
        // A triangle, point up.
        Marker::Trigger => {
            let (a, b, t) = (
                Vec2::new(cx, cy - r),
                Vec2::new(cx + r, cy + r * 0.8),
                Vec2::new(cx - r, cy + r * 0.8),
            );
            if lit {
                draw_triangle(a, b, t, c);
            } else {
                draw_line(a.x, a.y, b.x, b.y, 1.5, c);
                draw_line(b.x, b.y, t.x, t.y, 1.5, c);
                draw_line(t.x, t.y, a.x, a.y, 1.5, c);
            }
        }
    }
}

/// An ink that will show up on `fill`: dark on a light tile, light on a dark
/// one. Fixing a single ink colour would hide the motif on one end of the
/// role lightness range or the other.
fn motif_ink(fill: Color, alpha: f32) -> Color {
    if luminance(fill) > 0.46 {
        Color::new(0.0, 0.0, 0.0, 0.42 * alpha)
    } else {
        Color::new(1.0, 1.0, 1.0, 0.40 * alpha)
    }
}

/// Draw one component as a single shape rather than as a row of tiles.
///
/// The cells of a piece are filled edge to edge with no line between them, and
/// the outline is drawn only where the piece actually ends. That way a
/// four-cell blade reads as one blade, and the lines you *do* see inside an
/// assembled item are the seams between its components - which is the thing
/// worth being able to see at a glance.
fn draw_shape(
    shape: &Shape,
    ox: f32,
    oy: f32,
    cell: f32,
    def: &PieceDef,
    worn_in: Option<SlotKind>,
    alpha: f32,
) {
    // Materials and plating are shared between two grids, so a piece reads as
    // the slot it is actually in - a steel material in the greaves is greaves
    // coloured, not gloves coloured. Before it is in either it wears no slot
    // colour at all, because it does not belong to one yet.
    let (color, motif) = piece_look(def, worn_in);
    let ink = motif_ink(color, alpha);
    let cells = shape.cells();

    // Fill and motif first, with no inset, so neighbouring cells of the same
    // piece meet without a gap.
    for &(dx, dy) in cells {
        let x = ox + dx as f32 * cell;
        let y = oy + dy as f32 * cell;
        draw_rectangle(x, y, cell, cell, with_alpha(color, alpha));
        draw_motif(x, y, cell, motif, ink);
    }

    // Then trace the outside edge only. An edge is outside when the cell
    // across it belongs to some other piece, or to nothing.
    let edge = with_alpha(Color::from_rgba(0, 0, 0, 190), alpha);
    let t = (cell * 0.09).clamp(1.5, 3.0);
    let here = |dx: i8, dy: i8| cells.contains(&(dx, dy));
    for &(dx, dy) in cells {
        let x = ox + dx as f32 * cell;
        let y = oy + dy as f32 * cell;
        if !here(dx, dy - 1) {
            draw_line(x, y, x + cell, y, t, edge);
        }
        if !here(dx, dy + 1) {
            draw_line(x, y + cell, x + cell, y + cell, t, edge);
        }
        if !here(dx - 1, dy) {
            draw_line(x, y, x, y + cell, t, edge);
        }
        if !here(dx + 1, dy) {
            draw_line(x + cell, y, x + cell, y + cell, t, edge);
        }
    }
}

/// Everything on screen is drawn through `text`/`text_width`, so this single
/// constant controls how large the whole interface reads.
const TEXT_SCALE: f32 = 1.70;

/// Draw text at the interface's scale.
fn ui_text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    // Snap to whole logical pixels. Glyph strokes are one pixel wide at these
    // sizes, and drawing them at a fractional offset lets the letterbox
    // rescale sample one away - a capital O loses its right side and reads as
    // a C.
    draw_text(s, x.round(), y.round(), (size * TEXT_SCALE).round(), color);
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

/// Wrap to a pixel width rather than a character count. Text is scaled by
/// `TEXT_SCALE`, so counting characters means re-tuning every call site
/// whenever the scale moves; measuring does not.
fn wrap_px(s: &str, max_w: f32, size: f32) -> Vec<String> {
    wrap_measured(s, max_w, &|t| text_width(t, size))
}

/// The wrapping itself, over any width function. Split out so the line
/// breaking can be tested without a graphics context to measure against.
fn wrap_measured(s: &str, max_w: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let candidate =
            if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
        if !cur.is_empty() && measure(&candidate) > max_w {
            out.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = candidate;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The largest of `sizes` (given largest first) that renders `s` inside
/// `max_w`, or the smallest if none of them do. Lets a label be as big as its
/// column allows instead of being pinned to whatever happened to fit at the
/// scale it was written at.
fn fitting_size(s: &str, max_w: f32, sizes: &[f32]) -> f32 {
    largest_fitting(sizes, max_w, &|sz| text_width(s, sz))
}

/// The choice itself, over any width function - see `wrap_measured`.
fn largest_fitting(sizes: &[f32], max_w: f32, width_at: &dyn Fn(f32) -> f32) -> f32 {
    sizes
        .iter()
        .copied()
        .find(|&sz| width_at(sz) <= max_w)
        .unwrap_or_else(|| *sizes.last().unwrap())
}

/// Draw `s` wrapped into at most `max_lines`, and say whether anything was cut.
/// A caller that gets `true` back should offer the full text on hover.
fn draw_capped(
    s: &str,
    x: f32,
    y: f32,
    max_w: f32,
    size: f32,
    color: Color,
    max_lines: usize,
) -> bool {
    let lines = wrap_px(s, max_w, size);
    let cut = lines.len() > max_lines;
    let lh = line_h(size);
    for (i, line) in lines.iter().take(max_lines).enumerate() {
        let last = cut && i + 1 == max_lines;
        ui_text(
            &if last { format!("{} ...", line) } else { line.clone() },
            x,
            y + i as f32 * lh,
            size,
            color,
        );
    }
    cut
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

/// The bounding size, in cells, of a group of pieces laid out at the given
/// offsets. Used to centre a dragged group on the cursor and to size its card.
fn group_cells(run: &Run, pieces: &[(PieceId, u8, u8)]) -> (u8, u8) {
    let (mut w, mut h) = (0u8, 0u8);
    for &(p, dx, dy) in pieces {
        let s = run.registry.shape(p);
        w = w.max(dx + s.width() as u8);
        h = h.max(dy + s.height() as u8);
    }
    (w.max(1), h.max(1))
}

/// A tooltip's content: lines, plus line ranges to draw a panel behind.
///
/// The panels are what let a slot that can be built several ways put each way
/// in a box of its own, instead of running them together into one list where
/// it is not clear which requirement belongs to which.
#[derive(Default)]
struct Tip {
    lines: Vec<(String, Color)>,
    /// Half-open `[start, end)` line ranges, each drawn inside its own frame.
    boxes: Vec<(usize, usize)>,
}

impl Tip {
    fn plain(lines: Vec<(String, Color)>) -> Tip {
        Tip { lines, boxes: Vec::new() }
    }
}

/// One line of a pinned overflow list, and enough to describe it in full.
///
/// A "+3 more" that lists three names answers "how many" and not "what",
/// which is the question you were asking. Pinning the list lets you hover its
/// rows, and a row needs to carry the thing itself to do that.
#[derive(Clone)]
enum PinnedEntry {
    Item(ItemProfile),
    Class(&'static gearmaster_engine::class::ClassDef),
}

impl PinnedEntry {
    fn label(&self) -> String {
        match self {
            PinnedEntry::Item(p) => words::retell(&p.name),
            PinnedEntry::Class(c) => words::class(c.name).to_string(),
        }
    }

    fn note(&self) -> String {
        match self {
            PinnedEntry::Item(p) => format!("{:.2}s   {}", p.cooldown_ms as f32 / 1000.0, Rarity::of(p.rating).name()),
            PinnedEntry::Class(c) => words::retell(&c.power.short()),
        }
    }
}

/// A list the player has pinned open by clicking a "+ N more".
#[derive(Clone)]
struct Pinned {
    title: String,
    at: Vec2,
    entries: Vec<PinnedEntry>,
}

/// A tooltip some render pass asked for while drawing.
///
/// Tooltips have to be painted after everything else or the panel and the
/// boards draw over them, but the thing that knows a tooltip is wanted is
/// whatever is being hovered halfway through the frame. Render passes leave
/// their request here and the frame drains it at the end. Later requests win,
/// which is what you want: the panel is drawn over the boards, so if the
/// cursor is in both, the panel's answer is the relevant one.
#[derive(Default)]
struct Hover {
    tip: Option<Tip>,
    /// Set when the cursor is over the class block. Drawn after everything
    /// else, like a tooltip, but it is a chart rather than lines of text.
    class_card: bool,
    /// Set when the cursor is over the next opponent. Same idea, but big
    /// enough that it wants the whole board area rather than a tooltip frame.
    enemy_card: bool,
    /// A "+ N more" the cursor is over, and what it is hiding. Clicking pins
    /// it; until then it is shown as an ordinary tooltip.
    overflow: Option<Pinned>,
}

impl Hover {
    /// Register a tooltip if `region` is under the cursor. Returns whether it
    /// was, so the caller can also light the region up.
    fn over(
        &mut self,
        region: Rect,
        mx: f32,
        my: f32,
        lines: impl FnOnce() -> Vec<(String, Color)>,
    ) -> bool {
        self.over_tip(region, mx, my, || Tip::plain(lines()))
    }

    /// The same, for a tooltip that wants framed groups.
    fn over_tip(&mut self, region: Rect, mx: f32, my: f32, tip: impl FnOnce() -> Tip) -> bool {
        if !region.contains(Vec2::new(mx, my)) {
            return false;
        }
        let tip = tip();
        if !tip.lines.is_empty() {
            self.tip = Some(tip);
        }
        true
    }
}

// ============================================================ drag state

enum Drag {
    None,
    Held {
        /// What is on the cursor, and where each piece sits relative to the
        /// group's top-left corner in cells.
        ///
        /// One entry for an ordinary piece. A locked item puts all of its
        /// pieces here and travels as one thing - which is the whole point of
        /// locking it, and the reason its pieces cannot be pulled out
        /// individually.
        pieces: Vec<(PieceId, u8, u8)>,
        /// Cursor offset from the group's top-left, so it doesn't snap its
        /// corner to the mouse.
        grab: (f32, f32),
        /// Where it came from, so an invalid drop can put it back.
        restore: Option<(SlotKind, u8, u8)>,
    },
}

impl Drag {
    /// The piece a single-piece drag is carrying, and the anchor piece of a
    /// locked item's drag. `None` when nothing is held.
    fn held_id(&self) -> Option<PieceId> {
        match self {
            Drag::Held { pieces, .. } => pieces.first().map(|&(p, ..)| p),
            Drag::None => None,
        }
    }

    /// Is this piece on the cursor? A locked item hides all of its pieces, not
    /// just the one that was clicked.
    fn holds(&self, id: PieceId) -> bool {
        match self {
            Drag::Held { pieces, .. } => pieces.iter().any(|&(p, ..)| p == id),
            Drag::None => false,
        }
    }
}

// ============================================================== playback

/// Replays a finished `CombatLog` against wall-clock time. The fight is already
/// decided in the engine — this only decides what is on screen, so it can be
/// sped up or skipped without changing the result.
/// One curse riding on one side, as the replay sees it. Stun is not among
/// them - it holds an item rather than a fighter, and lives in `player_stuns`.
#[derive(Clone)]
struct ActiveCurse {
    /// The engine's key, not the themed word - comparisons are made on it.
    name: &'static str,
    until_ms: u32,
    stacks: u32,
    /// What this many stacks work out to right now, worked out by the engine
    /// from the same constants the fight reads.
    effect: String,
}

/// Everything the replay tracks about one foe.
///
/// A brawl has more than one of these; a duel has exactly one, and every
/// reader that only ever wanted the one goes through `foe()`. The paired
/// `enemy_*` fields this replaced could not have held a second creature at
/// all, which is most of what made the battle screen a duel-only screen.
struct FoeView {
    hp: i32,
    max: i32,
    armor: i32,
    pools: [i32; 6],
    curses: Vec<ActiveCurse>,
    /// Stunned items, as (item index, started, ends).
    stuns: Vec<(usize, u32, u32)>,
    empower: u32,
    shield: u32,
    fork: u32,
    flash: f64,
    schedule: Vec<Vec<u32>>,
    /// This creature's gear, laid out once when the fight starts.
    reg: PieceRegistry,
    loadout: Loadout,
    reports: Vec<SlotReport>,
    /// Full profiles, for the hover summaries. A monster's item list puts its
    /// innate attacks first, so its profiles start `attack_count` in.
    profiles: Vec<ItemProfile>,
    attack_count: usize,
}

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
    /// What is left of the run's gold, for gear that spends it mid-fight.
    purse: i32,
    player_pools: [i32; 6],
    /// Curses on the player. The start is kept as well as the end because the
    /// stun meter fills against the whole span, not against a fixed 1.2s -
    /// stun stacks add to the clock, so no two stuns are the same length.
    player_curses: Vec<ActiveCurse>,
    /// Stunned items, as (item index, started, ends). A stun stops one item
    /// rather than a whole side, so this is per item and not per fighter.
    player_stuns: Vec<(usize, u32, u32)>,
    lines: Vec<String>,
    flash_player: f64,
    now_ms: u32,
    done: bool,
    player_empower: u32,
    player_shield: u32,
    player_fork: u32,
    /// When each of the player's items fired, indexed the same way as the
    /// combatant's item list. Cooldown bars are drawn straight from these,
    /// which is why a frost-slowed item's bar visibly crawls: the gap between
    /// two real activations *is* the slowdown, so nothing here has to know
    /// what frost does.
    player_schedule: Vec<Vec<u32>>,
    player_profiles: Vec<ItemProfile>,
    /// Everything on the other side, indexed the way `LogEntry::who` is.
    foes: Vec<FoeView>,
}

impl Playback {
    fn foe_mut(&mut self, who: u8) -> &mut FoeView {
        let i = (who as usize).min(self.foes.len() - 1);
        &mut self.foes[i]
    }

    fn is_brawl(&self) -> bool {
        self.foes.len() > 1
    }
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
/// When each of one combatant's items fired.
///
/// `who` matters now: in a brawl both foes log their activations as
/// `Side::Enemy`, and only the entry's foe index tells them apart.
/// Start whatever fight is next: the rung's own creature, or the party an
/// event has put in front of you.
/// Start the next fight, unless something is standing in the road.
///
/// `None` means a town, a fountain or an event is waiting and the caller
/// should put the player back in front of it rather than fighting. Every path
/// that starts a fight goes through here, which is the point: the one that did
/// not - REMATCH, straight off the battle screen - is how runs were walking
/// past the first fountain.
/// "a town" -> "A town", for a sentence that starts with it.
fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn begin_next_fight(run: &mut Run, speed: f32) -> Option<Playback> {
    // A brawl an event arranged is the thing in the road, not a detour round
    // it, so it goes ahead.
    if let Some(specs) = run.pending_brawl() {
        let profiles = run.combat_items();
        return Some(Playback::new(run.fight_party(&specs), &profiles, speed));
    }
    if run.road_is_blocked().is_some() {
        return None;
    }
    let profiles = run.combat_items();
    Some(Playback::new(run.fight_next(), &profiles, speed))
}

fn schedule_for(log: &CombatLog, want: Side, who: u8, count: usize) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::new(); count];
    for e in &log.entries {
        if let Event::Activate { side, index, .. } = &e.event {
            if *side == want && (want == Side::Player || e.who == who) {
                if let Some(slot) = out.get_mut(*index) {
                    slot.push(e.at_ms);
                }
            }
        }
    }
    out
}

impl Playback {
    fn new(log: &CombatLog, player_profiles: &[ItemProfile], speed: f32) -> Self {
        let foes: Vec<FoeView> = log
            .specs
            .iter()
            .zip(log.enemies.iter())
            .enumerate()
            .map(|(i, (spec, body))| {
                let (reg, loadout) = spec.loadout();
                let reports = loadout.reports(&reg);
                let profiles = loadout.combat_items(&reg);
                FoeView {
                    hp: body.health,
                    max: body.max_health,
                    armor: 0,
                    pools: [0; 6],
                    curses: Vec::new(),
                    stuns: Vec::new(),
                    empower: 0,
                    shield: 0,
                    fork: 0,
                    flash: -10.0,
                    schedule: schedule_for(log, Side::Enemy, i as u8, body.items.len()),
                    reg,
                    loadout,
                    reports,
                    profiles,
                    attack_count: spec.attacks.len(),
                }
            })
            .collect();
        let pprof = player_profiles.to_vec();
        Playback {
            last_wall: get_time(),
            sim_ms: 0,
            speed,
            cursor: 0,
            player_hp: log.player.health,
            player_max: log.player.max_health,
            player_armor: 0,
            player_mana: 0,
            purse: 0,
            player_pools: [0; 6],
            player_curses: Vec::new(),
            player_stuns: Vec::new(),
            lines: Vec::new(),
            flash_player: -10.0,
            now_ms: 0,
            done: false,
            player_empower: 0,
            player_shield: 0,
            player_fork: 0,
            player_schedule: schedule_for(log, Side::Player, 0, log.player.items.len()),
            player_profiles: pprof,
            foes,
        }
    }

    fn apply(&mut self, log: &CombatLog, index: usize) {
        let entry = &log.entries[index];
        let now = get_time();
        // Which foe this entry is about. In a duel it is always the one.
        let who = entry.who;
        match &entry.event {
            Event::Activate { .. } => return, // shown as a bar, not a log line
            // Worth a line: an item coming round and doing nothing is the sort
            // of thing you want to see explained rather than wonder about.
            Event::Misfired { .. } | Event::Warded { .. } => {}
            // A fusion moves three pools at once: one of each parent down,
            // one of the child up. All three chips have to follow or the two
            // that quietly emptied look like a bug.
            Event::Fused { side, what, total, from, and } => {
                let pools = if matches!(side, Side::Player) {
                    &mut self.player_pools
                } else {
                    &mut self.foe_mut(who).pools
                };
                for (name, v) in [(*what, *total), (from.0, from.1), (and.0, and.1)] {
                    if let Some(i) = pool_index(name) {
                        pools[i] = v;
                    }
                }
            }
            Event::Watched { .. } => {}
            // Growth changes the bar itself, not just what is in it.
            Event::Grew { side, total, .. } => match side {
                Side::Player => self.player_max = *total,
                Side::Enemy => self.foe_mut(who).max = *total,
            },
            // Keeps the mana read-out honest: a cast spends from the same pool
            // everything else banks into.
            Event::Cast { side, remaining, .. } => {
                let pools = if matches!(side, Side::Player) {
                    &mut self.player_pools
                } else {
                    &mut self.foe_mut(who).pools
                };
                if let Some(i) = pool_index("mana") {
                    pools[i] = *remaining;
                }
            }
            Event::ResourceCheck { side, what, remaining, .. } => {
                if let Some(i) = pool_index(what) {
                    let pools =
                        if matches!(side, Side::Player) { &mut self.player_pools } else { &mut self.foe_mut(who).pools };
                    pools[i] = *remaining;
                }
            }
            Event::GainResource { side, what, total, .. } => {
                if let Some(i) = pool_index(what) {
                    let pools =
                        if matches!(side, Side::Player) { &mut self.player_pools } else { &mut self.foe_mut(who).pools };
                    pools[i] = *total;
                }
            }
            Event::Hit { by, target_health, target_armor, .. } => match by {
                Side::Player => {
                    self.foe_mut(who).hp = (*target_health).max(0);
                    self.foe_mut(who).armor = *target_armor;
                    self.foe_mut(who).flash = now;
                }
                Side::Enemy => {
                    self.player_hp = (*target_health).max(0);
                    self.player_armor = *target_armor;
                    self.flash_player = now;
                }
            },
            Event::MindHit { by, target_max_health, .. } => match by {
                Side::Player => self.foe_mut(who).max = *target_max_health,
                Side::Enemy => {
                    self.player_max = *target_max_health;
                    self.player_hp = self.player_hp.min(self.player_max);
                    self.flash_player = now;
                }
            },
            Event::GainArmor { side, total, .. } => match side {
                Side::Player => self.player_armor = *total,
                Side::Enemy => self.foe_mut(who).armor = *total,
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
            // Sudden death takes health off everybody at once, straight past
            // armour, so the bars have to follow it down rather than waiting
            // for a Hit that never comes.
            Event::SuddenDeath { pct } => {
                let bite = |max: i32| (max * pct / 100).max(1);
                self.player_hp = (self.player_hp - bite(self.player_max)).max(0);
                for f in self.foes.iter_mut() {
                    f.hp = (f.hp - bite(f.max)).max(0);
                }
            }
            Event::Spent { side, remaining, .. } => {
                // The purse is shown on the loadout screen, but a fight that
                // is eating it should say so while it happens.
                if matches!(side, Side::Player) {
                    self.purse = *remaining;
                }
            }
            Event::Drained { on, what, total, .. } => {
                // Pools are shown from the playback's own tally, so the bar
                // has to follow a drain down as well as a gain up.
                let pools = match on {
                    Side::Player => &mut self.player_pools,
                    Side::Enemy => &mut self.foe_mut(who).pools,
                };
                match *what {
                    "rage" => pools[0] = *total,
                    "faith" => pools[1] = *total,
                    "nature" => pools[2] = *total,
                    _ => {}
                }
                if *what == "mana" && matches!(on, Side::Player) {
                    self.player_mana = *total;
                }
            }
            Event::Stunned { on, index, duration_ms, .. } => {
                let now_ms = self.now_ms;
                let until = now_ms + duration_ms;
                let list = match on {
                    Side::Player => &mut self.player_stuns,
                    Side::Enemy => &mut self.foe_mut(who).stuns,
                };
                match list.iter_mut().find(|(i, _, _)| i == index) {
                    // Stacks pile onto that item's clock, so the meter restarts
                    // full rather than jumping mid-drain.
                    Some(e) => *e = (*index, now_ms, until),
                    None => list.push((*index, now_ms, until)),
                }
            }
            Event::Cursed { on, kind, duration_ms, stacks } => {
                let until = self.now_ms + duration_ms;
                let effect = kind.effect_at(*stacks);
                let list = match on {
                    Side::Player => &mut self.player_curses,
                    Side::Enemy => &mut self.foe_mut(who).curses,
                };
                match list.iter_mut().find(|c| c.name == kind.name()) {
                    Some(e) => {
                        e.until_ms = e.until_ms.max(until);
                        e.stacks = *stacks;
                        e.effect = effect;
                    }
                    None => list.push(ActiveCurse {
                        name: kind.name(),
                        until_ms: until,
                        stacks: *stacks,
                        effect,
                    }),
                }
            }
            Event::Burn { side, health, .. } => match side {
                Side::Player => self.player_hp = (*health).max(0),
                Side::Enemy => self.foe_mut(who).hp = (*health).max(0),
            },
            Event::Regen { side, health, .. } => {
                if *side == Side::Player {
                    self.player_hp = *health;
                }
                return; // healing ticks would drown the log
            }
            Event::Empowered { side, total, .. } => {
                if *side == Side::Player {
                    self.player_empower = *total;
                } else {
                    self.foe_mut(who).empower = *total;
                }
            }
            Event::Shielded { side, total, .. } => {
                if *side == Side::Player {
                    self.player_shield = *total;
                } else {
                    self.foe_mut(who).shield = *total;
                }
            }
            Event::Forking { side, total } => {
                if *side == Side::Player {
                    self.player_fork = *total;
                } else {
                    self.foe_mut(who).fork = *total;
                }
            }
            Event::Hastened { .. } => {}
            Event::Fell { .. } => {}
            Event::End { .. } => self.done = true,
        }
        self.lines.push(words::retell(&log.describe(entry)));
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
        let now = self.now_ms;
        self.player_curses.retain(|c| c.until_ms > now);
        self.player_stuns.retain(|(_, _, until)| *until > now);
        for f in self.foes.iter_mut() {
            f.curses.retain(|c| c.until_ms > now);
            f.stuns.retain(|(_, _, until)| *until > now);
        }
        // GEARMASTER_STUN=<n> keeps a rolling stun on the player's first n
        // items, so the meter can be looked at without hunting the ladder for
        // a fight that lands one at the moment the screenshot is taken.
        if let Some(n) = std::env::var("GEARMASTER_STUN").ok().and_then(|v| v.parse().ok()) {
            let span = 2_400;
            let from = self.now_ms / span * span;
            self.player_stuns.clear();
            for i in 0..n {
                self.player_stuns.push((i, from, from + span));
            }
        }
    }



    /// How much of a stun is left on one item, and how long in milliseconds.
    ///
    /// A stun stops one item rather than a side, so this is asked per row.
    fn stun_of(&self, side: Side, who: usize, index: usize) -> Option<(f32, u32)> {
        let list = match side {
            Side::Player => &self.player_stuns,
            Side::Enemy => &self.foes[who.min(self.foes.len() - 1)].stuns,
        };
        list.iter().find(|(i, _, _)| *i == index).map(|&(_, from, until)| {
            let span = until.saturating_sub(from).max(1);
            (until.saturating_sub(self.now_ms) as f32 / span as f32, until - self.now_ms)
        })
    }

    fn stunned_count(&self, side: Side, who: usize) -> usize {
        match side {
            Side::Player => self.player_stuns.len(),
            Side::Enemy => self.foes[who.min(self.foes.len() - 1)].stuns.len(),
        }
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
        self.player_stuns.clear();
        for f in self.foes.iter_mut() {
            f.curses.clear();
            f.stuns.clear();
        }
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
        // Assembled or not is brightness and weight, not gold against red.
        // A gold-versus-red pair is the one distinction red-green colour
        // blindness is worst at, and the gold read as the greaves besides.
        let color = if item.assembled {
            let p = ((get_time() * 3.0).sin() * 0.5 + 0.5) as f32;
            Color::new(1.0, 1.0, 1.0, 0.72 + 0.28 * p)
        } else {
            Color::from_rgba(24, 22, 30, 235)
        };
        let t = if item.assembled { 3.5 } else { 2.0 };

        // The badge sits on the item's topmost-then-leftmost cell.
        if item.assembled {
            let rarity = Rarity::of(item.rating);
            if rarity.marks() > 0 {
                if let Some(&(bx, by)) = cells.iter().min_by_key(|(x, y)| (*y, *x)) {
                    let (px, py) = view.cell_origin(bx, by);
                    draw_rarity_pips(px + 3.0, py + 5.0, rarity, 0.8);
                }
            }
        }

        for &(x, y) in &cells {
            let (px, py) = view.cell_origin(x, y);
            let up = y > 0 && cells.contains(&(x, y - 1));
            let down = y + 1 < view.rows && cells.contains(&(x, y + 1));
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

fn render_slots(
    layout: &Layout,
    run: &Run,
    reports: &[SlotReport],
    drag: &Drag,
    hover: &mut Hover,
    mx: f32,
    my: f32,
) {

    // What the item under the cursor is watching. Triggers that read a
    // neighbour or an aligned item are the hardest part of a build to see,
    // because the thing they depend on is somewhere else on the board - so
    // hovering one outlines whatever it is actually looking at.
    let watching = layout.slot_hit(mx, my).and_then(|(kind, gx, gy)| {
        let piece = run.loadout.slot(kind).get(gx, gy)?;
        let report = &reports[kind.index()];
        let item = report.items.iter().find(|i| i.assembled && i.pieces.contains(&piece))?;
        let mut reads_neighbours = false;
        let mut reads_rows = false;
        for p in &item.pieces {
            for t in run.registry.def(*p).triggers {
                use gearmaster_engine::piece::Trigger;
                match t {
                    Trigger::OnAdjacentActivate(_) | Trigger::PerAdjacentItem { .. } => {
                        reads_neighbours = true
                    }
                    Trigger::OnAlignedActivate(_) => reads_rows = true,
                    _ => {}
                }
            }
        }
        if !reads_neighbours && !reads_rows {
            return None;
        }
        let rows = run.loadout.slot(kind).row_span(&item.pieces);
        Some((kind, item.pieces.clone(), reads_neighbours, reads_rows, rows))
    });

    for view in &layout.slots {
        let report = &reports[view.kind.index()];
        let any_assembled = report.assembled_count() > 0;
        let (gw, gh) = view.size();
        let (ox, oy) = view.origin;

        // Header: the slot name, with the recipe behind a hover. Spelling the
        // recipe out here cost three wrapped lines per slot and still ran into
        // the board below it once the text got big enough to read.
        let head = Rect::new(ox, oy - 52.0, gw, 40.0);
        let head_hot = head.contains(Vec2::new(mx, my));
        let name_size = fitting_size(view.kind.name(), gw - 30.0, &[22.0, 20.0, 18.0, 16.0]);
        ui_text(view.kind.name(), ox, oy - 22.0, name_size, WHITE);
        // The marker sits at the column's right edge, so a long slot name can
        // never push it into the next column.
        let mark_c = if head_hot { col_gold() } else { col_dim() };
        draw_circle_lines(ox + gw - 10.0, oy - 29.0, 9.0, 1.5, mark_c);
        centered_text("?", ox + gw - 10.0, oy - 24.0, 13.0, mark_c);
        hover.over_tip(head, mx, my, || recipe_tip(view.kind));

        // Slot border lights up once at least one item has come together.
        // Brightness carries this, not hue - gold against grey would have put
        // the greaves board's border the same colour as its tiles.
        let border = if any_assembled {
            let p = ((get_time() * 3.0).sin() * 0.5 + 0.5) as f32;
            let v = 0.80 + 0.20 * p;
            Color::new(v, v, v, 1.0)
        } else {
            Color::from_rgba(70, 70, 92, 255)
        };
        draw_rectangle(ox - 3.0, oy - 3.0, gw + 6.0, gh + 6.0, border);

        for gy in 0..view.rows {
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

        // Terrain first, or it paints over the gear standing on it: `pieces`
        // walks the gear layer before the one underneath, which is the right
        // order for everything except drawing.
        let mut to_draw = run.loadout.slot(view.kind).pieces();
        to_draw.sort_by_key(|&id| !run.registry.def(id).kind.is_underlay());
        for id in to_draw {
            if drag.holds(id) {
                continue; // it's on the cursor instead
            }
            let Some((ax, ay)) = run.loadout.slot(view.kind).anchor_of(id) else { continue };
            let def = run.registry.def(id);
            let shape = run.registry.shape(id);
            let (px, py) = view.cell_origin(ax, ay);
            draw_shape(&shape, px, py, SLOT_CELL, def, Some(view.kind), 1.0);

            if let Some(&(dx, dy)) = shape.cells().first() {
                let tx = px + dx as f32 * SLOT_CELL + SLOT_CELL / 2.0;
                let ty = py + dy as f32 * SLOT_CELL + SLOT_CELL / 2.0 + 4.0;
                // A plate behind the initials: they sit on a cell that now
                // carries a motif, and two overlapping marks read as neither.
                // Sized to the cell, since a three-word name abbreviates to
                // three letters and those do not fit at the nominal size.
                let label = abbrev(words::piece(def.name));
                let size = fitting_size(&label, SLOT_CELL - 7.0, &[13.0, 12.0, 11.0, 10.0, 9.0]);
                let lw = text_width(&label, size).min(SLOT_CELL - 5.0);
                draw_rectangle(
                    tx - lw / 2.0 - 2.0,
                    ty - 11.0,
                    lw + 4.0,
                    14.0,
                    Color::from_rgba(238, 238, 244, 205),
                );
                centered_text(&label, tx, ty, size, Color::from_rgba(15, 15, 20, 255));

                let live = assembled_piece(id);
                // Top-right: assembly bonus. Filled once its item is finished.
                if def.adjacency.is_some() {
                    let cx = px + dx as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + 6.0;
                    draw_marker(cx, cy, Marker::Bonus, live);
                }
                // Bottom-right: positional effect. Filled while its condition
                // currently holds.
                if let Some(eff) = def.effect {
                    let cx = px + dx as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    draw_marker(cx, cy, Marker::Effect, eff.when.holds(live));
                }
                if !def.triggers.is_empty() {
                    let cx = px + dx as f32 * SLOT_CELL + 6.0;
                    let cy = py + dy as f32 * SLOT_CELL + SLOT_CELL - 6.0;
                    draw_marker(cx, cy, Marker::Trigger, live);
                }
            }
        }

        render_item_outlines(view, run, report);

        // A locked item wears a solid gold border rather than the pulsing
        // white one, so "I decided this" reads differently from "this happens
        // to be assembled".
        for set in &run.loadout.locks {
            let slot = run.loadout.slot(view.kind);
            let cells: HashSet<(u8, u8)> =
                set.pieces.iter().flat_map(|&p| slot.cells_of(p)).collect();
            if cells.is_empty() {
                continue;
            }
            for &(cx, cy) in &cells {
                let (px, py) = view.cell_origin(cx, cy);
                let edge = |dx: i32, dy: i32| {
                    let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                    !(nx >= 0 && ny >= 0 && cells.contains(&(nx as u8, ny as u8)))
                };
                if edge(0, -1) {
                    draw_line(px, py, px + SLOT_CELL, py, 3.0, col_gold());
                }
                if edge(0, 1) {
                    draw_line(px, py + SLOT_CELL, px + SLOT_CELL, py + SLOT_CELL, 3.0, col_gold());
                }
                if edge(-1, 0) {
                    draw_line(px, py, px, py + SLOT_CELL, 3.0, col_gold());
                }
                if edge(1, 0) {
                    draw_line(px + SLOT_CELL, py, px + SLOT_CELL, py + SLOT_CELL, 3.0, col_gold());
                }
            }
        }

        // Draw the watched cells over the top of the ordinary outlines.
        if let Some((from_slot, ref source, neighbours, rows, span)) = watching {
            let slot = run.loadout.slot(view.kind);
            let pulse = ((get_time() * 4.0).sin() * 0.5 + 0.5) as f32;
            let mark = Color::new(0.42, 0.86, 1.0, 0.45 + 0.35 * pulse);
            for item in &report.items {
                if !item.assembled || item.pieces.iter().any(|p| source.contains(p)) {
                    continue;
                }
                let watched = if view.kind == from_slot {
                    // Same grid: whatever this item is packed against.
                    neighbours && slot.sets_touch(source, &item.pieces)
                } else {
                    // Another grid: whatever shares its rows.
                    rows
                        && match (span, slot.row_span(&item.pieces)) {
                            (Some((a0, a1)), Some((b0, b1))) => a0 <= b1 && b0 <= a1,
                            _ => false,
                        }
                };
                if !watched {
                    continue;
                }
                for p in &item.pieces {
                    for (cx, cy) in slot.cells_of(*p) {
                        let (px, py) = view.cell_origin(cx, cy);
                        draw_rectangle_lines(px + 1.0, py + 1.0, SLOT_CELL - 2.0, SLOT_CELL - 2.0, 2.5, mark);
                    }
                }
            }
        }

        // What each slot holds is listed under it by `render_slot_items`,
        // which owns everything below the grid now.
    }
}

/// The finished items in each slot, listed under the board they were built
/// in - one line each, the way the battle screen lists cooldowns.
///
/// They used to be cards in one shared strip below all five boards, which ran
/// off the bottom of the screen once a build had more than a few items and
/// never said which slot anything came from. A line under its own board says
/// both, in a fraction of the space.
fn render_slot_items(
    layout: &Layout,
    run: &Run,
    reports: &[SlotReport],
    profiles: &[ItemProfile],
    hover: &mut Hover,
    mx: f32,
    my: f32,
) {
    for view in &layout.slots {
        let (gw, gh) = view.size();
        let (ox, oy) = view.origin;
        let mut y = oy + gh + STRIP_TOP;

        let mine: Vec<&ItemProfile> = profiles.iter().filter(|p| p.slot == view.kind).collect();
        for (i, p) in mine.iter().enumerate() {
            if i == STRIP_ROWS && mine.len() > STRIP_ROWS {
                // The overflow says how many and, on hover, what they are.
                // A count with no way to read it is worse than no count.
                let rest = &mine[i..];
                let row = Rect::new(ox, y, gw, STRIP_ROW_H);
                let hot = row.contains(Vec2::new(mx, my));
                if hot {
                    draw_rectangle(row.x, row.y, row.w, row.h, Color::from_rgba(255, 255, 255, 18));
                }
                ui_text(
                    &format!("+{} more", rest.len()),
                    ox + 4.0,
                    y + 13.0,
                    12.0,
                    if hot { col_gold() } else { col_dim() },
                );
                if hot {
                    hover.overflow = Some(Pinned {
                        title: format!("{} MORE", rest.len()),
                        at: Vec2::new(ox, y + STRIP_ROW_H + 4.0),
                        entries: rest.iter().map(|p| PinnedEntry::Item((*p).clone())).collect(),
                    });
                }
                y += STRIP_ROW_H;
                break;
            }
            let row = Rect::new(ox, y, gw, STRIP_ROW_H);
            let hot = row.contains(Vec2::new(mx, my));
            if hot {
                draw_rectangle(row.x, row.y, row.w, row.h, Color::from_rgba(255, 255, 255, 18));
            }
            draw_item_sigil(ox + 9.0, y + 9.0, 15.0, Some(p.slot), p.sigil_seed, LIGHTGRAY);
            // The cadence is pinned right, so a long name shortens rather than
            // pushing the number off the end of the board.
            let time = format!("{:.1}s", p.cooldown_ms as f32 / 1000.0);
            let tw = text_width(&time, 12.0);
            ui_text(&time, ox + gw - tw - 2.0, y + 14.0, 12.0, col_dim());
            let room = gw - 22.0 - tw - 22.0;
            // The short name goes through the theme too, or an item is called
            // one thing in the list and another on its own card.
            let shown = words::retell(&p.name);
            let size = fitting_size(&shown, room, &[14.0, 13.0, 12.0, 11.0]);
            draw_capped(&shown, ox + 20.0, y + 14.0, room, size, WHITE, 1);
            draw_rarity_pips(
                ox + 20.0 + text_width(&shown, size).min(room) + 6.0,
                y + 9.0,
                p.rarity(),
                0.7,
            );
            if hot {
                let profile = (*p).clone();
                hover.over_tip(row, mx, my, || Tip::plain(item_summary_lines(&profile, run)));
            }
            y += STRIP_ROW_H;
        }

        // Anything that has not come together yet, under its finished items.
        let report = &reports[view.kind.index()];
        if report.loose_count() > 0 {
            let short = short_summary(report);
            let size = fitting_size(&short, gw - 8.0, &[13.0, 12.0, 11.0]);
            let row = Rect::new(ox, y, gw, STRIP_ROW_H);
            draw_capped(&short, ox + 4.0, y + 14.0, gw - 8.0, size, col_bad(), 1);
            let full = report.summary();
            hover.over(row, mx, my, || {
                let mut lines = vec![(full.clone(), col_bad())];
                for item in report.items.iter().filter(|i| !i.assembled) {
                    lines.push((
                        format!("  {}: {}", words::retell(&item.name.short), item.status),
                        LIGHTGRAY,
                    ));
                }
                lines
            });
        }
    }
}

/// Where the shop's first card starts: clear of the gold column and the
/// reroll button beneath it, both of which grow with the text scale.
/// What the reroll button says, in this theme.
/// A number of coins, in this theme's currency.
///
/// "1g" reads fine while the currency is gold. A theme that spends something
/// else has to spell it out, or a button says "1g" directly under a purse
/// that says "28 fnorp" - which is exactly what three separate call sites
/// were doing before this existed.
fn coins(n: i32) -> String {
    match words::word("gold-suffix", "") {
        "" => format!("{}g", n),
        unit => format!("{} {}", n, unit),
    }
}

fn reroll_label(cost: i32) -> String {
    format!("{} {}", words::word("reroll", "REROLL"), coins(cost))
}

fn shop_cards_x(shop: Rect) -> f32 {
    reroll_rect(shop).right() + 18.0
}

/// Where the reroll button sits inside the shop strip. Sized to its label so
/// the text cannot outgrow the box.
fn reroll_rect(shop: Rect) -> Rect {
    // Sized to the dearest it gets, so the button does not grow under the
    // cursor as the price doubles.
    let w = text_width(&reroll_label(8), 18.0) + 26.0;
    Rect::new(shop.x + 12.0, shop.y + 78.0, w, 34.0)
}

/// The shelf. Clicking a card buys it if you can afford it.
/// "Word of the Kolok Hatter" -> "Kolok Hatter", so a price reads as a price
/// and not as a second name.
fn trim_article(s: &str) -> &str {
    for lead in ["A Word About the ", "A Word About ", "Word of the ", "Word of ", "The ", "A "] {
        if let Some(rest) = s.strip_prefix(lead) {
            return rest;
        }
    }
    s
}

fn render_shop(layout: &Layout, run: &Run, mx: f32, my: f32) {
    let r = layout.shop;
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(28, 26, 22, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(96, 84, 52, 255));

    let shop_label = words::word("shop", "SHOP");
    ui_text(shop_label, r.x + 14.0, r.y + 26.0, 18.0, col_gold());
    ui_text(
        words::word("shop-hint", "right-click a card to pin it"),
        r.x + 24.0 + text_width(shop_label, 18.0),
        r.y + 26.0,
        12.0,
        col_dim(),
    );

    ui_text(
        &format!("{} {}", run.gold, words::word("gold-lower", "gold")),
        r.x + 14.0,
        r.y + 50.0,
        20.0,
        WHITE,
    );
    // A bar does not take money, and a shop that said "click to buy" over a
    // row of things money cannot buy would be lying twice.
    let bar = layout.shop_cards.iter().any(|c| {
        gearmaster_engine::rumour::is_rumour(c.def.name)
            || c.def.name == gearmaster_engine::rumour::TROPHY_SHELF
    });
    ui_text(
        if bar {
            words::word("barter-hint", "click a rumour, then the piece that pays for it")
        } else {
            words::word("buy-hint", "click to buy")
        },
        r.x + 14.0,
        r.y + 68.0,
        12.0,
        col_dim(),
    );
    let cost = run.reroll_cost();
    // Nothing to reroll at a bar: these two are the only rumours there are.
    if !bar {
        button(reroll_rect(r), &reroll_label(cost), run.gold >= cost, mx, my);
    }

    for card in &layout.shop_cards {
        let def = card.def;
        let afford = run.gold >= shop_price(def);
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
            card.rect.y + 8.0 + (44.0 - sh) / 2.0,
            INV_CELL,
            def,
            None,
            alpha,
        );

        // Flowed, not pinned: the price sits under the role, the role under
        // the name, and a two-line name pushes them down rather than being
        // landed on.
        let cx = card.rect.x + card.rect.w / 2.0;
        let mut ty = card.rect.y + 62.0;
        let shown = words::piece(def.name);
        let ns = fitting_size(shown, card.rect.w - 12.0, &[13.0, 12.0, 11.0]);
        let name_lines = wrap_px(shown, card.rect.w - 12.0, ns);
        // A rumour prints no role, so its name gets that line back. Three is
        // what "A Word About the Crownwright" needs on a card this narrow.
        let name_room = if gearmaster_engine::rumour::is_rumour(def.name) { 3 } else { 2 };
        for line in name_lines.iter().take(name_room) {
            centered_text(line, cx, ty, ns, if afford { WHITE } else { col_dim() });
            ty += line_h(ns);
        }
        // The price always. The role only when the name left room for it - a
        // two-line name and a role and a price do not all fit on a card this
        // short, and of the three the role is the one a hover already gives
        // you.
        let price_y = card.rect.bottom() - 8.0;
        if ty < price_y - 14.0 {
            let role = def.kind.name_in(def.slot);
            let rs = fitting_size(&role, card.rect.w - 10.0, &[12.0, 11.0, 10.0]);
            centered_text(&role, cx, ty, rs, col_dim());
        }
        if def.name == gearmaster_engine::rumour::TROPHY_SHELF {
            let label = words::word("trophy-price", "one boss trophy");
            let ps = fitting_size(label, card.rect.w - 8.0, &[12.0, 11.0, 10.0, 9.0]);
            draw_capped(label, card.rect.x + 4.0, price_y, card.rect.w - 8.0, ps, col_gold(), 1);
        } else {
        match gearmaster_engine::rumour::by_name(def.name) {
            // Priced in gear. The label is what they want, not what it costs.
            Some(word) => {
                // The themed name, not the canonical one: a bar that wants
                // "the Crownwright" in a game full of Kolok is the theme
                // giving itself away.
                let label = match word.price.named() {
                    Some(n) => format!("the {}", trim_article(words::piece(n))),
                    None => words::retell(&word.price.label()),
                };
                let ps = fitting_size(&label, card.rect.w - 8.0, &[12.0, 11.0, 10.0, 9.0]);
                draw_capped(
                    &label,
                    card.rect.x + 4.0,
                    price_y,
                    card.rect.w - 8.0,
                    ps,
                    col_gold(),
                    1,
                );
            }
            None => centered_text(
                &format!("{} {}", shop_price(def), words::word("gold-lower", "gold")),
                cx,
                price_y,
                14.0,
                if afford { col_gold() } else { col_bad() },
            ),
        }
        }

        // What this piece deals in, stacked down the left edge - mana above
        // rage above whatever else it touches - so the shelves can be read
        // without opening a single tooltip.
        let keys = keywords_of(def);
        let mut ky = card.rect.y + 8.0;
        for k in keys.iter().take(6) {
            draw_keyword(card.rect.x + 4.0, ky, 13.0, k);
            ky += 15.0;
        }

        // A pinned shelf gets a bright border and a mark, so it reads as held
        // rather than merely hovered.
        if run.shop.is_locked(card.slot_index) {
            draw_rectangle_lines(
                card.rect.x - 2.0,
                card.rect.y - 2.0,
                card.rect.w + 4.0,
                card.rect.h + 4.0,
                3.0,
                col_gold(),
            );
            let (px, py) = (card.rect.x + card.rect.w - 13.0, card.rect.y + card.rect.h - 13.0);
            draw_circle(px, py, 6.0, col_gold());
            draw_rectangle(px - 2.0, py - 1.0, 4.0, 7.0, Color::from_rgba(20, 18, 12, 255));
            draw_circle_lines(px, py - 4.0, 4.0, 1.8, col_gold());
        }

        // Same markers as the inventory, so a triggered piece is obvious
        // before you pay for it.
        if def.adjacency.is_some() {
            draw_marker(card.rect.x + card.rect.w - 12.0, card.rect.y + 12.0, Marker::Bonus, true);
        }
        if def.effect.is_some() {
            draw_marker(card.rect.x + 12.0, card.rect.y + 12.0, Marker::Effect, true);
        }
        if !def.triggers.is_empty() {
            draw_marker(card.rect.x + 12.0, card.rect.y + 27.0, Marker::Trigger, true);
        }
    }
}

/// Where the SELL badge sits on an inventory card. The same rect draws it and
/// hit-tests it, so the two can never drift.
fn sell_badge_rect(card: Rect) -> Rect {
    Rect::new(card.x + 6.0, card.y + card.h - 30.0, card.w - 12.0, 24.0)
}

/// The inventory card under the cursor whose SELL badge is being pointed at.
fn sell_hit(layout: &Layout, mx: f32, my: f32) -> Option<PieceId> {
    layout
        .cards
        .iter()
        .find(|c| {
            c.rect.contains(Vec2::new(mx, my))
                && sell_badge_rect(c.rect).contains(Vec2::new(mx, my))
        })
        .map(|c| c.id)
}

fn render_inventory(
    layout: &Layout,
    run: &Run,
    drag: &Drag,
    bartering: Option<usize>,
    mx: f32,
    my: f32,
) {
    // Anything the bar would take for the rumour that is waiting. Nothing at
    // all when no trade is open, which is almost always.
    let wanted: Vec<PieceId> = bartering.map(|i| run.payment_for(i)).unwrap_or_default();
    draw_rectangle(layout.inv.x, layout.inv.y, layout.inv.w, layout.inv.h, col_tray());
    draw_rectangle_lines(
        layout.inv.x,
        layout.inv.y,
        layout.inv.w,
        layout.inv.h,
        2.0,
        Color::from_rgba(60, 60, 82, 255),
    );
    let inv_label = words::word("inventory", "INVENTORY");
    ui_text(inv_label, layout.inv.x + 14.0, layout.inv.y + 24.0, 18.0, WHITE);
    // How full it is, next to the heading. A limit you cannot see is a limit
    // you only find out about at the moment it stops you.
    let held = run.inventory().len();
    let cap = gearmaster_engine::run::INVENTORY_CAP;
    let count = format!("{} / {}", held, cap);
    let cw = text_width(&count, 15.0);
    ui_text(
        &count,
        layout.inv.x + layout.inv.w - cw - 14.0,
        layout.inv.y + 24.0,
        15.0,
        if held >= cap { col_bad() } else { col_dim() },
    );
    // The hint sits after the heading with whatever room is left, so it never
    // grows back into the word "INVENTORY" as the text scale moves.
    let hint_x = layout.inv.x + 30.0 + text_width("INVENTORY", 18.0);
    let hint = words::word(
        "inventory-hint",
        "drag onto a slot  ·  right-click rotates  ·  shift-click to lock an item",
    );
    let hint_size = fitting_size(hint, layout.inv.w - (hint_x - layout.inv.x) - 16.0, &[14.0, 13.0, 12.0, 11.0]);
    ui_text(hint, hint_x, layout.inv.y + 24.0, hint_size, col_dim());

    for card in &layout.cards {
        if drag.holds(card.id) {
            continue;
        }
        let def = run.registry.def(card.id);
        let shape = run.registry.shape(card.id);
        let hovered = card.rect.contains(Vec2::new(mx, my));
        // Lit up while a trade is open, so "hand something over" is a thing
        // you can see rather than a thing you have to work out.
        if wanted.contains(&card.id) {
            draw_rectangle(
                card.rect.x - 3.0,
                card.rect.y - 3.0,
                card.rect.w + 6.0,
                card.rect.h + 6.0,
                Color::from_rgba(252, 205, 88, 38),
            );
            draw_rectangle_lines(
                card.rect.x - 3.0,
                card.rect.y - 3.0,
                card.rect.w + 6.0,
                card.rect.h + 6.0,
                2.0,
                col_gold(),
            );
        }

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
        // A stowed locked item keeps the gold edge it wore on the board, so it
        // is recognisable as the thing you decided to keep together.
        let group = run.locked_shape(card.id);
        draw_rectangle_lines(
            card.rect.x,
            card.rect.y,
            card.rect.w,
            card.rect.h,
            if group.is_some() { 2.0 } else { 1.5 },
            match (&group, hovered) {
                (_, true) => col_gold(),
                (Some(_), false) => Color::from_rgba(150, 122, 52, 255),
                (None, false) => Color::from_rgba(58, 58, 78, 255),
            },
        );

        // Shape preview, centred in the upper part of the card. A locked item
        // is drawn whole - it is one thing now, and a card showing only its
        // handle would say the opposite.
        match &group {
            Some(pieces) => {
                let (gw, gh) = group_cells(run, pieces);
                // Its footprint can be far larger than a single piece, so the
                // preview is scaled down to fit rather than overflowing.
                let cell = INV_CELL.min(46.0 / gh as f32).min((card.rect.w - 16.0) / gw as f32);
                let (sw, sh) = (gw as f32 * cell, gh as f32 * cell);
                let (bx, by) = (
                    card.rect.x + (card.rect.w - sw) / 2.0,
                    card.rect.y + 8.0 + (46.0 - sh) / 2.0,
                );
                for &(p, dx, dy) in pieces {
                    draw_shape(
                        &run.registry.shape(p),
                        bx + dx as f32 * cell,
                        by + dy as f32 * cell,
                        cell,
                        run.registry.def(p),
                        None,
                        1.0,
                    );
                }
            }
            None => {
                let sw = shape.width() as f32 * INV_CELL;
                let sh = shape.height() as f32 * INV_CELL;
                draw_shape(
                    &shape,
                    card.rect.x + (card.rect.w - sw) / 2.0,
                    card.rect.y + 8.0 + (46.0 - sh) / 2.0,
                    INV_CELL,
                    def,
                    None,
                    1.0,
                );
            }
        }

        // Name (wrapped) and role. The role follows the name rather than being
        // pinned to the bottom edge: a two-line name and a fixed footer had
        // nowhere to be but on top of each other once the card got shorter.
        let cx = card.rect.x + card.rect.w / 2.0;
        let mut ty = card.rect.y + 70.0;
        let label = match &group {
            // Named for the piece it is built around, the way the board names
            // it, with the rest of the item accounted for.
            Some(pieces) => {
                let core = pieces
                    .iter()
                    .map(|&(p, ..)| run.registry.def(p))
                    .find(|d| d.kind.is_core())
                    .unwrap_or(def);
                format!("{} +{}", words::piece(core.name), pieces.len() - 1)
            }
            None => words::piece(def.name).to_string(),
        };
        // One line if it will fit at all, two only if it must - and shrink
        // before wrapping, since a shorter card has room for neither.
        let name_size = fitting_size(&label, card.rect.w - 12.0, &[13.0, 12.0, 11.0]);
        for line in wrap_px(&label, card.rect.w - 12.0, name_size).into_iter().take(2) {
            centered_text(&line, cx, ty, name_size, Color::from_rgba(215, 218, 235, 255));
            ty += line_h(name_size);
        }
        if hovered {
            // Selling is the card's only click action, so it only appears
            // while you are pointing at one - the tray would be a wall of
            // buttons otherwise.
            let b = sell_badge_rect(card.rect);
            let hot = b.contains(Vec2::new(mx, my));
            draw_rectangle(
                b.x,
                b.y,
                b.w,
                b.h,
                if hot { Color::from_rgba(96, 70, 24, 255) } else { Color::from_rgba(52, 48, 40, 255) },
            );
            draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.5, if hot { col_gold() } else { col_dim() });
            centered_text(
                &format!("SELL {}", coins(resale_price(def))),
                b.x + b.w / 2.0,
                b.y + b.h - 7.0,
                13.0,
                if hot { col_gold() } else { LIGHTGRAY },
            );
        } else {
            let role = def.kind.name_in(def.slot);
            let rs = fitting_size(&role, card.rect.w - 10.0, &[12.0, 11.0, 10.0]);
            centered_text(&role, cx, ty.min(card.rect.bottom() - 8.0), rs, col_dim());
        }

        if def.adjacency.is_some() {
            draw_marker(card.rect.x + card.rect.w - 12.0, card.rect.y + 12.0, Marker::Bonus, true);
        }
        if def.effect.is_some() {
            draw_marker(card.rect.x + 12.0, card.rect.y + 12.0, Marker::Effect, true);
        }
        if !def.triggers.is_empty() {
            draw_marker(card.rect.x + 12.0, card.rect.y + 27.0, Marker::Trigger, true);
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
    render_def_tooltip_inner(
        run.registry.def(id),
        item_name,
        Some(run.quest_progress(id)),
        mx,
        my,
    );
}

fn render_def_tooltip(def: &'static PieceDef, mx: f32, my: f32) {
    render_def_tooltip_inner(def, None, None, mx, my);
}

/// `item_name` is the procedurally generated name of the assembled item this
/// piece belongs to, shown above the component's own details.
fn render_def_tooltip_inner(
    def: &'static PieceDef,
    item_name: Option<&str>,
    quest_progress: Option<u32>,
    mx: f32,
    my: f32,
) {
    // Collected plainly and re-told once at the end - the same treatment the
    // item card gets, and for the same reason.
    let mut lines: Vec<(String, Color)> = Vec::new();
    if let Some(n) = item_name {
        lines.push((n.to_string(), col_gold()));
        lines.push(("part of".to_string(), col_dim()));
    }
    lines.push((words::piece(def.name).to_string(), WHITE));
    // A rumour is not gear and its card must not read like gear: it has one
    // empty cell and no stats, so the ordinary treatment would print a name
    // and then nothing at all. What it has instead is a hint, and the hint is
    // vague on purpose - working out what it means is the whole of it.
    if def.name == gearmaster_engine::rumour::TROPHY_SHELF {
        lines.push((words::word("a-standing-offer", "A STANDING OFFER").to_string(), col_dim()));
        lines.push((String::new(), WHITE));
        for l in wrap_px(
            &words::retell_naming(
                "Hand over anything you took off a named creature and they will show you how                  to look at gear. A stack of Recycler: every adjacency bonus on your boards                  counts ten percent more.",
            ),
            420.0,
            14.0,
        ) {
            lines.push((l, Color::from_rgba(214, 200, 170, 255)));
        }
        lines.push((String::new(), WHITE));
        lines.push((
            words::word(
                "trophy-note",
                "The counter pays nothing for a trophy. This is the only other thing that takes one.",
            )
            .to_string(),
            col_dim(),
        ));
        draw_tooltip_with_sigil(&lines, Some((Some(def.slot), 0)), mx, my);
        return;
    }
    if let Some(word) = gearmaster_engine::rumour::by_name(def.name) {
        lines.push((words::word("a-rumour", "A RUMOUR").to_string(), col_dim()));
        lines.push((String::new(), WHITE));
        for l in wrap_px(&words::retell(word.hint), 420.0, 14.0) {
            lines.push((l, Color::from_rgba(214, 200, 170, 255)));
        }
        lines.push((String::new(), WHITE));
        lines.push((
            format!("they want {} for it", words::retell(&word.price.label())),
            col_gold(),
        ));
        lines.push((
            words::word("rumour-note", "It never goes on a board. It only has to be carried.")
                .to_string(),
            col_dim(),
        ));
        draw_tooltip_with_sigil(&lines, Some((Some(def.slot), 0)), mx, my);
        return;
    }
    // A shared piece names every grid it fits, not just the one it is filed
    // under - naming one would be the same lie the old colouring told.
    let where_it_goes = def
        .slots()
        .iter()
        .map(|s| s.name())
        .collect::<Vec<_>>()
        .join(" or ");
    lines.push((format!("{} · {}", where_it_goes, def.kind.name_in(def.slot)), col_dim()));

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
    // What an ink is for, and now a book or an orb too: it adds to the
    // multiplier of the one item it is part of and never to the wearer.
    // Written as "x0.45" it read as though the item ended up at less than
    // half - it is a bonus on top of a base of one, not a replacement for it.
    if def.power_bonus != 0 {
        lines.push((
            format!(
                "+{}.{:02}x to this item's own multiplier, and nothing else",
                def.power_bonus / 100,
                def.power_bonus.abs() % 100
            ),
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
    // Same rule as the item card: an unconditional pool gain reads as a stat,
    // because that is what it is. Only the conditional triggers - spending a
    // pool, answering a neighbour - earn a line of their own.
    let mut banked: Vec<(Resource, i32)> = Vec::new();
    let mut conditional: Vec<&Trigger> = Vec::new();
    for t in def.triggers {
        let plain = match t {
            Trigger::OnActivate(Action::GainMana(n)) => Some((Resource::Mana, *n)),
            Trigger::OnActivate(Action::Gain { what, amount }) => Some((*what, *amount)),
            _ => None,
        };
        match plain {
            Some((what, amount)) => match banked.iter_mut().find(|(w, _)| *w == what) {
                Some(entry) => entry.1 += amount,
                None => banked.push((what, amount)),
            },
            None => conditional.push(t),
        }
    }
    if !banked.is_empty() {
        let each: Vec<String> =
            banked.iter().map(|(w, n)| format!("{:+} {}", n, w.name())).collect();
        lines.push((
            format!("{} each time its item fires", each.join(", ")),
            Color::from_rgba(190, 210, 245, 255),
        ));
    }
    for t in conditional {
        for l in wrap(&t.describe(), 46) {
            lines.push((l, col_trigger()));
        }
    }
    if let Some(q) = def.quest {
        let done = quest_progress.unwrap_or(0).min(q.goal);
        lines.push((
            format!("QUEST  {} / {}", done, q.goal),
            Color::from_rgba(150, 220, 190, 255),
        ));
        for l in wrap(&q.track.describe(q.goal), 46) {
            lines.push((format!("  {}", l), Color::from_rgba(150, 220, 190, 255)));
        }
        lines.push((format!("  then it becomes {}", words::piece(q.becomes)), col_gold()));
        lines.push(("  only counts while assembled".to_string(), col_dim()));
    }
    lines.push((
        format!("{} {}", shop_price(def), words::word("gold-lower", "gold")),
        col_gold(),
    ));
    // Re-told once, now the whole card is collected. The piece's own name went
    // through the theme when it was pushed; this catches everything the engine
    // wrote about it.
    let lines: Vec<(String, Color)> =
        lines.into_iter().map(|(s, c)| (words::retell(&s), c)).collect();

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

/// Armour, on the same scale and at the same size as the health bar above it.
///
/// The two read as a pair because they are the same measurement: a full armour
/// bar means as much armour as you have health, and a pixel means the same
/// number of points in both. It used to be half the height and to clamp at
/// full, so every amount from "exactly enough" to "four times over" drew an
/// identical bar and the difference between them was invisible.
///
/// Past full it wraps. Each complete bar is another layer, drawn darker than
/// the one under it, so depth reads as depth without a number to parse.
fn armor_bar(x: f32, y: f32, w: f32, h: f32, armor: i32, max: i32) {
    /// The base coat, and how much of it survives each layer down.
    fn shade(layer: u32) -> Color {
        let f = 0.72f32.powi(layer.min(5) as i32);
        Color::from_rgba(
            (150.0 * f) as u8 + 18,
            (172.0 * f) as u8 + 20,
            (214.0 * f) as u8 + 26,
            255,
        )
    }
    draw_rectangle(x, y, w, h, Color::from_rgba(30, 30, 42, 255));
    let max = max.max(1);
    if armor > 0 {
        let full = (armor / max) as u32;
        let rest = (armor % max) as f32 / max as f32;
        // The last completed layer fills the track; the remainder goes over it
        // one shade darker.
        if full > 0 {
            draw_rectangle(x, y, w, h, shade(full - 1));
        }
        if rest > 0.0 {
            draw_rectangle(x, y, w * rest, h, shade(full));
        }
    }
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(80, 80, 105, 255));
    if armor > 0 {
        // Only worth the words once there is something to say; an empty track
        // reading "0 / 400" is noise on a screen that is already busy.
        let label = if armor > max {
            format!("{} armour  ({:.1}x)", armor, armor as f32 / max as f32)
        } else {
            format!("{} armour", armor)
        };
        centered_text(&label, x + w / 2.0, y + h / 2.0 + 5.0, 14.0, WHITE);
    }
}

/// One item's cooldown bar: name, a filling track, and the interval it is
/// actually running at. Flashes on the frame it fires.
#[allow(clippy::too_many_arguments)]
/// The colour a stun owns. Nothing else on the battle screen uses it, which is
/// most of what makes the meter readable at a glance.
fn col_stun() -> Color {
    Color::from_rgba(252, 205, 88, 255)
}

/// The stun meter: the same bar space as a cooldown, read the other way round.
///
/// A cooldown grows from the left as its item approaches firing. A stun does
/// the reverse - it starts full and retreats to the right as it wears off - so
/// the two can never be mistaken for one another even in the corner of your
/// eye. The ribbon undulates while the stun is live and flattens out as it
/// ends, which is the part you notice from across the screen; a straight bar
/// draining is just a cooldown running backwards.
fn draw_stun_bar(track_x: f32, y: f32, track_w: f32, h: f32, left: f32, t: f64) {
    let left = left.clamp(0.0, 1.0);
    draw_rectangle(track_x, y, track_w, h, Color::from_rgba(46, 36, 12, 255));

    let fill_w = track_w * left;
    let fill_x = track_x + track_w - fill_w;
    // Slice thin enough that the wave reads as a curve rather than a staircase.
    let step: f32 = 3.0;
    let right = track_x + track_w;
    let mut sx = fill_x;
    while sx < right {
        let sw = step.min(right - sx);
        let u = (sx - track_x) / track_w.max(1.0);
        // The wave travels along the bar, and its amplitude fades with the
        // stun - so "nearly over" is legible without reading the number.
        let amp = h * 0.34 * left;
        let wave = (u * 20.0 - t as f32 * 8.0).sin() * amp;
        let hh = (h - wave.abs()).max(2.0);
        draw_rectangle(sx, y + (h - hh) * 0.5, sw, hh, col_stun());
        sx += step;
    }
    draw_rectangle_lines(track_x, y, track_w, h, 1.0, Color::from_rgba(158, 126, 40, 255));
}

fn render_cooldown_row(
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    slot: Option<SlotKind>,
    sigil_seed: u64,
    cooldown_ms: u32,
    schedule: &[u32],
    now_ms: u32,
    tint: Color,
    rarity: Rarity,
    // `stun` is set while this item's owner is stunned: how much of the stun
    // is left, and how long that is. Every row on a stunned side gets it.
    stun: Option<(f32, u32)>,
) {
    let icon = 24.0;
    let label_w = 232.0;
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

    // A stunned row owns the whole row's colour, not just its bar: the sigil
    // and the name go amber too, so "this cannot advance" is one glance rather
    // than a bar you have to go and read.
    let fg = match (stun.is_some(), just_fired) {
        (true, _) => col_stun(),
        (false, true) => WHITE,
        (false, false) => Color::from_rgba(178, 180, 200, 255),
    };
    let sigil_col = match (stun.is_some(), just_fired) {
        (true, _) => col_stun(),
        (false, true) => WHITE,
        (false, false) => tint,
    };
    draw_item_sigil(x, y - 5.0, icon, slot, sigil_seed, sigil_col);
    // Names are procedurally generated, so their length is not something the
    // layout can assume; shrink rather than run into the bar.
    let name_x = x + icon + 8.0;
    // Leave room for the badge so a legendary's pips never sit on the name.
    let pips_w = rarity.marks() as f32 * 10.0;
    let size = fitting_size(name, track_x - name_x - pips_w - 12.0, &[15.0, 14.0, 13.0, 12.0, 11.0]);
    ui_text(name, name_x, y + 12.0, size, fg);
    draw_rarity_pips(name_x + text_width(name, size) + 6.0, y + 7.0, rarity, 1.0);

    // The stun meter takes the cooldown's place rather than sitting beside it.
    // While a stun is up the cooldown is not advancing at all, so drawing it
    // would be drawing a lie - a bar frozen part-way with no reason on screen.
    if let Some((left, left_ms)) = stun {
        draw_stun_bar(track_x, y, track_w, h, left, get_time());
        ui_text(
            &format!("{:.1}s", left_ms as f32 / 1000.0),
            track_x + track_w + 8.0,
            y + 12.0,
            12.0,
            col_stun(),
        );
        return;
    }

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

/// How many of the enemy's items the preview lists before counting the rest.
const ITEMS_SHOWN: usize = 8;

/// What the next fight is bringing, drawn over the loadout screen.
///
/// The shop is a decision you cannot make well without this. Piercing beats
/// resistance and hardening beats piercing, so "should I buy the warded plate
/// or the sunder haft" has a right answer that depends entirely on what is
/// waiting - and until now the only way to find out was to lose to it once.
///
/// Their gear is built with the same `loadout_at` the fight uses, at the run's
/// own difficulty, so this is not an approximation of the enemy: it is the
/// enemy.
fn render_enemy_preview(r: Rect, spec: &MonsterSpec, difficulty: Difficulty, rung: usize) {
    let (reg, loadout) = spec.loadout_at(difficulty);
    let reports = loadout.reports(&reg);
    let (stats, profiles) = spec.outfit_at(difficulty);

    // Sized to its contents rather than to the space available. A panel that
    // takes the whole screen to show six lines and a board reads as a bug, and
    // the loadout underneath is worth leaving visible.
    let shown = profiles.len().div_ceil(2).max(1).min(ITEMS_SHOWN);
    let content_h = 54.0
        + 18.0
        + 16.0
        + 34.0
        + 26.0
        + 18.0
        + shown as f32 * STRIP_ROW_H
        + 8.0
        + SLOT_H as f32 * MINI_CELL
        + 44.0;
    let r = Rect::new(r.x, r.y, r.w, content_h);

    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(14, 12, 18, 250));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(150, 90, 78, 255));

    let mname = words::monster(spec.name);
    ui_text(mname, r.x + 18.0, r.y + 28.0, 20.0, Color::from_rgba(235, 145, 122, 255));
    let sub = format!(
        "rung {} of {}   {} hp   {} strength   {} regen/s",
        rung + 1,
        LADDER.len(),
        stats.health,
        stats.strength,
        stats.regen
    );
    ui_text(&sub, r.x + 26.0 + text_width(mname, 20.0), r.y + 28.0, 13.0, col_dim());

    // ---- the defence triangle, which is the whole reason to look ----
    let mut y = r.y + 54.0;
    ui_text(
        words::word("their-defences", "WHAT THEY ANSWER TO"),
        r.x + 18.0,
        y,
        12.0,
        col_dim(),
    );
    y += 18.0;
    // Fixed offsets, not fractions of the panel: the panel is as wide as five
    // boards and these are three short numbers, so a third of it each put the
    // headings a hand's width from the figures under them.
    let c1 = r.x + 150.0;
    let c2 = r.x + 240.0;
    let c3 = r.x + 330.0;
    // Piercing is only worth buying against resistance, and hardening only
    // against piercing, so the three are read together or not at all.
    let rows: [(&str, i32, i32, i32); 2] = [
        (
            words::word("physical", "PHYSICAL"),
            stats.physical_resist,
            stats.physical_pierce,
            stats.physical_harden,
        ),
        (
            words::word("magic", "MAGIC"),
            stats.magic_resist,
            stats.magic_pierce,
            stats.magic_harden,
        ),
    ];
    ui_text("resist", c1, y, 11.0, col_dim());
    ui_text("pierce", c2, y, 11.0, col_dim());
    ui_text("harden", c3, y, 11.0, col_dim());
    y += 16.0;
    for (label, resist, pierce, harden) in rows {
        ui_text(label, r.x + 18.0, y, 13.0, Color::from_rgba(200, 200, 220, 255));
        // A zero is worth drawing dim rather than leaving out: "they have no
        // hardening" is a thing you want to be able to see at a glance.
        let cell = |v: i32, at: f32| {
            let t = format!("{}%", v);
            ui_text(
                &t,
                at,
                y,
                13.0,
                if v == 0 { Color::from_rgba(96, 96, 116, 255) } else { WHITE },
            );
        };
        cell(resist, c1);
        cell(pierce, c2);
        cell(harden, c3);
        y += 17.0;
    }
    let extra = format!(
        "{} {}%    {} {}%",
        words::word("mind-resist", "MIND RESIST"),
        stats.mind_resist,
        words::word("curse-resist", "CURSE RESIST"),
        stats.curse_resist
    );
    ui_text(&extra, r.x + 18.0, y + 2.0, 12.0, col_dim());
    y += 24.0;

    // ---- what they will actually be swinging ----
    ui_text(
        words::word("their-gear", "WHAT THEY BRING"),
        r.x + 18.0,
        y,
        12.0,
        col_dim(),
    );
    y += 18.0;
    // Two columns rather than a cap. This panel is a preview - it exists so you
    // can shop against what is coming - and "+ 7 more" in the middle of it
    // withholds exactly the half you would have wanted. There is no room to
    // hover an overflow here either: the card only lives while the cursor is on
    // the portrait, so reaching for the list dismisses the list.
    let per_col = profiles.len().div_ceil(2).max(1);
    let col_w = (r.w - 36.0) / 2.0;
    for (i, p) in profiles.iter().enumerate() {
        let cx = r.x + 26.0 + (i / per_col) as f32 * col_w;
        let ly = y + (i % per_col) as f32 * STRIP_ROW_H;
        let name = words::retell(&p.name);
        let size = fitting_size(&name, col_w * 0.56, &[13.0, 12.0, 11.0]);
        ui_text(&name, cx, ly, size, Color::from_rgba(214, 200, 200, 255));
        let hit = p.hit_for(stats.strength);
        let what = if hit > 0 {
            format!("hits {}", hit)
        } else if p.stats.armor > 0 {
            format!("+{} armour", p.stats.armor)
        } else if p.cooldown_ms > 0 {
            format!("{:.1}s", p.cooldown_ms as f32 / 1000.0)
        } else {
            String::new()
        };
        draw_capped(&what, cx + col_w * 0.58, ly, col_w * 0.4, 12.0, Color::from_rgba(200, 170, 150, 255), 1);
    }
    y += shown as f32 * STRIP_ROW_H + 8.0;

    // ---- their board, drawn exactly as the fight will show it ----
    let bw = 5.0 * (SLOT_W as f32 * MINI_CELL) + 4.0 * MINI_GAP;
    render_mini_board(
        r.x + (r.w - bw) / 2.0,
        y,
        &reg,
        &loadout,
        &reports,
        col_foe_dim(),
        &Shakes::new(),
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
    cd_x: f32,
    cd_w: f32,
    log: Rect,
    buttons: [Rect; 5],
    /// The one thing to do once the fight is over. It sits above the ordinary
    /// row and is much larger, because a finished fight goes nowhere until it
    /// is clicked and people were not seeing that.
    primary: Rect,
}

/// `done` moves the ordinary button row down to make room for the primary
/// action, which only exists once the fight has finished.
fn battle_geom(done: bool) -> BattleGeom {
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
    // The primary takes the first two slots of the same row and grows upward,
    // so its bottom still lines up with the ordinary buttons. There is no room
    // below them: the row already sits near the bottom of the viewport.
    let _ = done;
    let pw = 2.0 * w + gap;
    BattleGeom {
        primary: Rect::new(x0, btn_y - 18.0, pw, 56.0),
        board_x,
        board_w,
        player_board_y,
        enemy_board_y,
        player_bar_y,
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

/// A procedurally generated emblem for one assembled item.
///
/// The slot fixes the archetype — a weapon always reads as a blade — while the
/// item's hash varies the proportions and ornament. That is the same number
/// its name is drawn from, so an item that is renamed by moving a piece is
/// redrawn to match.
fn draw_item_sigil(x: f32, y: f32, sz: f32, slot: Option<SlotKind>, seed: u64, c: Color) {
    // A handful of independent knobs off different slices of the hash.
    let bit = |shift: u32, n: u64| ((seed >> shift) % n) as u32;
    let frac = |shift: u32, lo: f32, hi: f32| {
        lo + (((seed >> shift) % 1000) as f32 / 1000.0) * (hi - lo)
    };
    let t = (sz * 0.075).max(1.4);
    let fx = |f: f32| x + sz * f;
    let fy = |f: f32| y + sz * f;
    let dark = Color::new(c.r * 0.45, c.g * 0.45, c.b * 0.45, c.a);

    match slot {
        // A creature's own armament: a pair of fangs, so a bite never reads as
        // a piece of equipment it does not have.
        None => {
            for side in [-1.0f32, 1.0] {
                let cx = 0.5 + side * frac(7, 0.11, 0.20);
                draw_triangle(
                    Vec2::new(fx(cx - 0.11), fy(0.14)),
                    Vec2::new(fx(cx + 0.11), fy(0.14)),
                    Vec2::new(fx(cx), fy(frac(23, 0.72, 0.92))),
                    c,
                );
            }
            draw_line(fx(0.14), fy(0.16), fx(0.86), fy(0.16), t, c);
        }
        Some(SlotKind::Weapon) => {
            let len = frac(3, 0.52, 0.76);
            let wide = frac(11, 0.06, 0.13);
            // Blade: a tapered quad, sometimes leaf-shaped.
            draw_triangle(
                Vec2::new(fx(0.5 - wide), fy(0.72 - len * 0.35)),
                Vec2::new(fx(0.5 + wide), fy(0.72 - len * 0.35)),
                Vec2::new(fx(0.5), fy(0.72 - len)),
                c,
            );
            draw_rectangle(fx(0.5 - wide), fy(0.72 - len * 0.35), sz * wide * 2.0, sz * len * 0.35, c);
            // Guard.
            match bit(19, 3) {
                0 => draw_line(fx(0.22), fy(0.72), fx(0.78), fy(0.72), t, c),
                1 => {
                    draw_line(fx(0.24), fy(0.74), fx(0.76), fy(0.74), t, c);
                    draw_line(fx(0.24), fy(0.74), fx(0.30), fy(0.64), t, c);
                    draw_line(fx(0.76), fy(0.74), fx(0.70), fy(0.64), t, c);
                }
                _ => draw_poly_lines(fx(0.5), fy(0.72), 6, sz * 0.14, 0.0, t, c),
            }
            draw_line(fx(0.5), fy(0.74), fx(0.5), fy(0.92), t, c); // grip
            match bit(27, 3) {
                0 => draw_circle(fx(0.5), fy(0.94), sz * 0.06, c),
                1 => draw_poly(fx(0.5), fy(0.94), 4, sz * 0.07, 45.0, c),
                _ => {}
            }
            // Fuller.
            if bit(31, 2) == 0 {
                draw_line(fx(0.5), fy(0.70), fx(0.5), fy(0.76 - len), t * 0.6, dark);
            }
        }
        Some(SlotKind::Helmet) => {
            let brow = frac(5, 0.30, 0.42);
            draw_line(fx(0.18), fy(0.80), fx(0.18), fy(brow), t, c);
            draw_line(fx(0.18), fy(brow), fx(0.5), fy(0.12), t, c);
            draw_line(fx(0.5), fy(0.12), fx(0.82), fy(brow), t, c);
            draw_line(fx(0.82), fy(brow), fx(0.82), fy(0.80), t, c);
            draw_line(fx(0.12), fy(0.80), fx(0.88), fy(0.80), t, c);
            // Visor slits.
            for i in 0..(1 + bit(13, 3)) {
                let vy = fy(0.52 + i as f32 * 0.10);
                draw_line(fx(0.28), vy, fx(0.72), vy, t * 0.8, c);
            }
            // Crest.
            match bit(23, 4) {
                0 => draw_line(fx(0.5), fy(0.12), fx(0.5), fy(-0.02), t, c),
                1 => {
                    draw_line(fx(0.28), fy(0.26), fx(0.14), fy(0.06), t, c);
                    draw_line(fx(0.72), fy(0.26), fx(0.86), fy(0.06), t, c);
                }
                2 => draw_poly_lines(fx(0.5), fy(0.08), 3, sz * 0.10, 90.0, t, c),
                _ => {}
            }
        }
        Some(SlotKind::Chest) => {
            let w = frac(7, 0.24, 0.32);
            draw_line(fx(0.5 - w), fy(0.26), fx(0.5 + w), fy(0.26), t, c);
            draw_line(fx(0.5 - w), fy(0.26), fx(0.5 - w * 0.7), fy(0.86), t, c);
            draw_line(fx(0.5 + w), fy(0.26), fx(0.5 + w * 0.7), fy(0.86), t, c);
            draw_line(fx(0.5 - w * 0.7), fy(0.86), fx(0.5 + w * 0.7), fy(0.86), t, c);
            // Neckline.
            draw_line(fx(0.5 - 0.10), fy(0.26), fx(0.5), fy(0.40), t, c);
            draw_line(fx(0.5 + 0.10), fy(0.26), fx(0.5), fy(0.40), t, c);
            // Panel divisions.
            for i in 0..(1 + bit(17, 3)) {
                let py = fy(0.50 + i as f32 * 0.13);
                draw_line(fx(0.5 - w * 0.8), py, fx(0.5 + w * 0.8), py, t * 0.7, dark);
            }
            if bit(29, 2) == 0 {
                draw_poly_lines(fx(0.5), fy(0.56), 6, sz * 0.09, 0.0, t * 0.7, dark);
            }
        }
        Some(SlotKind::Gloves) => {
            draw_rectangle_lines(fx(0.30), fy(0.46), sz * 0.42, sz * 0.42, t, c);
            let fingers = 3 + bit(9, 2);
            for i in 0..fingers {
                let gx = fx(0.34 + i as f32 * (0.34 / fingers as f32));
                draw_line(gx, fy(0.46), gx, fy(0.46 - frac(21 + i, 0.16, 0.30)), t, c);
            }
            draw_line(fx(0.30), fy(0.58), fx(0.10), fy(0.40), t, c); // thumb
            for i in 0..bit(25, 3) {
                draw_circle(fx(0.40 + i as f32 * 0.11), fy(0.66), sz * 0.035, dark);
            }
        }
        Some(SlotKind::Greaves) => {
            let shin = frac(15, 0.34, 0.52);
            draw_rectangle_lines(fx(0.34), fy(0.86 - shin), sz * 0.28, sz * shin, t, c);
            draw_rectangle_lines(fx(0.34), fy(0.86), sz * 0.46, sz * 0.14, t, c);
            for i in 0..(1 + bit(33, 3)) {
                let py = fy(0.86 - shin + 0.10 + i as f32 * 0.12);
                if py < fy(0.84) {
                    draw_line(fx(0.34), py, fx(0.62), py, t * 0.7, dark);
                }
            }
            if bit(37, 2) == 0 {
                draw_triangle(
                    Vec2::new(fx(0.34), fy(0.86 - shin)),
                    Vec2::new(fx(0.62), fy(0.86 - shin)),
                    Vec2::new(fx(0.48), fy(0.86 - shin - 0.12)),
                    c,
                );
            }
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
/// How tall each cooldown row is, and how many of them get drawn.
///
/// A deep boss can field far more gear than the eight or so a band was drawn
/// for. Left alone the two lists ran into each other and into the log strip,
/// so the rows tighten to fit the space they have and, past the point where
/// tightening would make them unreadable, the tail is summarised instead.
fn cooldown_fit(count: usize, avail: f32) -> (f32, usize) {
    const IDEAL: f32 = 28.0;
    const FLOOR: f32 = 17.0;
    if count == 0 {
        return (IDEAL, 0);
    }
    let pitch = (avail / count as f32).clamp(FLOOR, IDEAL);
    let fits = (avail / pitch).floor() as usize;
    if fits >= count {
        (pitch, count)
    } else {
        // Leave a row spare to say what was left out.
        (pitch, fits.saturating_sub(1))
    }
}

/// Room a side's cooldown list has before it would run into whatever is under
/// it: the enemy's half for the player, the log strip for the enemy.
fn cooldown_room(g: &BattleGeom, top: f32, share: f32) -> f32 {
    // Your column stops above the enemy half. Theirs stops above the log - or,
    // in a brawl, above the next creature's column: three lists all measuring
    // their room down to the log is three lists drawn on top of each other.
    let floor = if top < g.enemy_board_y {
        g.enemy_board_y - 26.0
    } else {
        (g.log.y - 16.0).min(top + share - 8.0)
    };
    (floor - (top + 30.0)).max(40.0)
}

fn cooldown_row_rect(g: &BattleGeom, top: f32, i: usize, pitch: f32) -> Rect {
    Rect::new(g.cd_x, top + 30.0 + i as f32 * pitch - 5.0, g.cd_w, pitch - 2.0)
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
    render_mini_board_at(x0, y0, MINI_CELL, MINI_GAP, reg, loadout, reports, accent, shakes)
}

/// The same, at whatever scale it has been given. Two boards only fit across
/// the screen at a smaller cell than a duel uses.
#[allow(clippy::too_many_arguments)]
fn render_mini_board_at(
    x0: f32,
    y0: f32,
    cell: f32,
    slot_gap: f32,
    reg: &PieceRegistry,
    loadout: &Loadout,
    reports: &[SlotReport],
    accent: Color,
    shakes: &Shakes,
) {
    let rows = loadout.rows();
    let gw = SLOT_W as f32 * cell;
    let gh = rows as f32 * cell;

    for (i, &kind) in SlotKind::ALL.iter().enumerate() {
        let gx = x0 + i as f32 * (gw + slot_gap);
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
        for cy in 0..rows {
            for cx in 0..SLOT_W {
                let (px, py) = (gx + cx as f32 * cell, y0 + cy as f32 * cell);
                let c = if (cx + cy) % 2 == 0 { col_cell_a() } else { col_cell_b() };
                draw_rectangle(px, py, cell, cell, c);
            }
        }

        for id in slot.pieces() {
            let Some((ax, ay)) = slot.anchor_of(id) else { continue };
            let def = reg.def(id);
            let shape = reg.shape(id);
            let (dx, dy) = shakes.get(&id).copied().unwrap_or((0.0, 0.0));
            draw_shape(
                &shape,
                gx + ax as f32 * cell + dx,
                y0 + ay as f32 * cell + dy,
                cell,
                def,
                Some(kind),
                1.0,
            );
        }

        // Outline each finished item so the two boards read as gear, not
        // confetti. White rather than gold: gold is a slot colour now.
        let outline = Color::new(1.0, 1.0, 1.0, 0.88);
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
                    gx + cx as f32 * cell + odx,
                    y0 + cy as f32 * cell + ody,
                );
                if cy == 0 || !cells.contains(&(cx, cy - 1)) {
                    draw_line(px, py, px + cell, py, 2.0, outline);
                }
                if cy + 1 >= rows || !cells.contains(&(cx, cy + 1)) {
                    draw_line(px, py + cell, px + cell, py + cell, 2.0, outline);
                }
                if cx == 0 || !cells.contains(&(cx - 1, cy)) {
                    draw_line(px, py, px, py + cell, 2.0, outline);
                }
                if cx + 1 >= SLOT_W || !cells.contains(&(cx + 1, cy)) {
                    draw_line(px + cell, py, px + cell, py + cell, 2.0, outline);
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
/// `log_scroll` is how many lines back from the newest the panel is showing.
/// Zero follows the fight; anything else holds still while it runs.
#[allow(clippy::too_many_arguments)]
fn render_battle(
    run: &Run,
    pb: &Playback,
    log_expanded: bool,
    log_scroll: usize,
    hover: &mut Hover,
    mx: f32,
    my: f32,
) {
    let Some(log) = run.log.as_ref() else { return };
    let g = battle_geom(pb.done);
    let reports = run.reports();

    // ---- your half ----
    ui_text(
        "YOUR GEAR",
        g.board_x,
        g.player_board_y - 12.0,
        18.0,
        col_you(),
    );
    let player_shakes =
        shake_offsets(&pb.player_profiles, &pb.player_schedule, 0, pb.now_ms);
    render_mini_board(
        g.board_x,
        g.player_board_y,
        &run.registry,
        &run.loadout,
        &reports,
        col_you_dim(),
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
        pb.player_empower,
        pb.player_shield,
        pb.player_fork,
        &pb.player_curses,
        pb.now_ms,
        pb.player_pools,
        pb.flash_player,
        col_you(),
    );

    // ---- their half ----
    //
    // One creature gets the whole width. Two share it, at a smaller cell -
    // five 6x8 grids side by side is already most of the screen, so two sets
    // of them only fit by shrinking. Everything else about the half is the
    // same for both, which is why it is a loop rather than two branches.
    let n = pb.foes.len().max(1);
    let (cell, sgap) = if n > 1 { (15.0, 7.0) } else { (MINI_CELL, MINI_GAP) };
    let set_w = 5.0 * (SLOT_W as f32 * cell) + 4.0 * sgap;
    let between = if n > 1 { (g.board_w - 2.0 * set_w).max(16.0) } else { 0.0 };
    let foe_gh = SLOT_H as f32 * cell;

    for (i, foe) in pb.foes.iter().enumerate() {
        let fx = g.board_x + i as f32 * (set_w + between);
        // The spec carries the canonical name, which is what the theme is
        // keyed on; the body carries the health.
        let canonical = log.specs.get(i).map(|m| m.name).unwrap_or("?");
        let name = words::monster(canonical);
        let label = format!("{}'s GEAR", name.to_uppercase());
        let size = fitting_size(&label, set_w - 8.0, &[18.0, 16.0, 14.0, 12.0]);
        ui_text(&label, fx, g.enemy_board_y - 12.0, size, col_foe());

        if foe.loadout.slots.iter().all(|s| s.is_empty()) {
            draw_rectangle_lines(
                fx,
                g.enemy_board_y,
                set_w,
                foe_gh,
                2.0,
                Color::from_rgba(70, 54, 54, 255),
            );
            centered_text(
                "no gear - it just has teeth",
                fx + set_w / 2.0,
                g.enemy_board_y + foe_gh / 2.0,
                if n > 1 { 14.0 } else { 18.0 },
                col_dim(),
            );
        } else {
            let shakes =
                shake_offsets(&foe.profiles, &foe.schedule, foe.attack_count, pb.now_ms);
            render_mini_board_at(
                fx,
                g.enemy_board_y,
                cell,
                sgap,
                &foe.reg,
                &foe.loadout,
                &foe.reports,
                col_foe_dim(),
                &shakes,
            );
        }
        render_battle_side(
            fx,
            // Clear of the slot captions the board prints under itself.
            // Those sit a fixed distance below the grid whatever the cell
            // size, so a smaller board needs more clearance, not less.
            g.enemy_board_y + foe_gh + if n > 1 { 60.0 } else { 42.0 },
            set_w,
            &name,
            foe.hp,
            foe.max,
            foe.armor,
            None,
            foe.empower,
            foe.shield,
            foe.fork,
            &foe.curses,
            pb.now_ms,
            foe.pools,
            foe.flash,
            col_foe(),
        );
    }

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

    // Your column, then one per creature. In a brawl the two share the space
    // the single enemy column had, which is why the header names them: "THEIR
    // COOLDOWNS" is no answer when there are two of them.
    #[allow(clippy::type_complexity)]
    let mut columns: Vec<(
        String,
        &Vec<RunningItem>,
        &Vec<Vec<u32>>,
        f32,
        Color,
        Side,
        usize,
        &Vec<ItemProfile>,
        usize,
        f32,
    )> = vec![(
        words::word("your-cooldowns", "YOUR COOLDOWNS").to_string(),
        &log.player.items,
        &pb.player_schedule,
        g.player_board_y,
        col_you(),
        Side::Player,
        0,
        &pb.player_profiles,
        0,
        f32::MAX,
    )];
    {
        let n = pb.foes.len().max(1);
        let room = (LOGICAL_H - g.enemy_board_y - 150.0).max(60.0);
        for (i, foe) in pb.foes.iter().enumerate() {
            let canonical = log.specs.get(i).map(|m| m.name).unwrap_or("?");
            let label = if n > 1 {
                words::monster(canonical).to_uppercase()
            } else {
                words::word("their-cooldowns", "THEIR COOLDOWNS").to_string()
            };
            let top = g.enemy_board_y + i as f32 * (room / n as f32);
            let items = log.enemies.get(i).map(|e| &e.items).unwrap_or(&log.player.items);
            columns.push((
                label,
                items,
                &foe.schedule,
                top,
                col_foe(),
                Side::Enemy,
                i,
                &foe.profiles,
                foe.attack_count,
                room / n as f32,
            ));
        }
    }

    for (label, items, sched, top, tint, side, who, profiles, offset, share) in columns {
        let label = label.as_str();
        ui_text(label, g.cd_x, top + 14.0, 13.0, col_dim());
        // A stun takes one item, so the header counts them rather than
        // declaring the whole side stopped.
        let stunned = pb.stunned_count(side, who);
        if stunned > 0 {
            let note = if stunned == 1 {
                "1 ITEM STUNNED".to_string()
            } else {
                format!("{} ITEMS STUNNED", stunned)
            };
            ui_text(
                &note,
                g.cd_x + g.cd_w - text_width(&note, 13.0),
                top + 14.0,
                13.0,
                col_stun(),
            );
        }
        let order = cooldown_order(items);
        let (pitch, shown) = cooldown_fit(order.len(), cooldown_room(&g, top, share));
        for (row, &i) in order.iter().take(shown).enumerate() {
            let it = &items[i];
            // `i` is the item's index in its owner's list, which is what the
            // stun was recorded against - `row` is only where it is drawn.
            let stun = pb.stun_of(side, who, i);
            if stun.is_some() {
                draw_rectangle(
                    g.cd_x - 6.0,
                    top + 25.0 + row as f32 * pitch,
                    g.cd_w + 12.0,
                    pitch - 2.0,
                    Color::from_rgba(252, 205, 88, 22),
                );
            }
            if cooldown_row_rect(&g, top, row, pitch).contains(Vec2::new(mx, my)) {
                draw_rectangle(
                    g.cd_x - 6.0,
                    top + 25.0 + row as f32 * pitch,
                    g.cd_w + 12.0,
                    pitch - 2.0,
                    Color::from_rgba(255, 255, 255, 14),
                );
            }
            render_cooldown_row(
                g.cd_x,
                top + 30.0 + row as f32 * pitch,
                g.cd_w,
                &words::retell(&it.name),
                it.slot,
                it.sigil_seed,
                it.cooldown_ms,
                sched.get(i).map(|v| v.as_slice()).unwrap_or(&[]),
                pb.now_ms,
                tint,
                Rarity::of(it.rating),
                stun,
            );
        }
        if shown < order.len() {
            let ry = top + 30.0 + shown as f32 * pitch;
            let row = Rect::new(g.cd_x, ry, g.cd_w, 20.0);
            let hot = row.contains(Vec2::new(mx, my));
            ui_text(
                &format!("+ {} more", order.len() - shown),
                g.cd_x + 32.0,
                ry + 12.0,
                13.0,
                if hot { col_gold() } else { col_dim() },
            );
            // What the list could not fit, on hover - and held open on click,
            // because a name and a cooldown is not what the item does.
            if hot {
                hover.overflow = Some(Pinned {
                    title: format!("{} MORE", order.len() - shown),
                    at: Vec2::new(g.cd_x, ry + 20.0),
                    entries: order[shown..]
                        .iter()
                        .filter_map(|&i| i.checked_sub(offset).and_then(|j| profiles.get(j)))
                        .map(|p| PinnedEntry::Item(p.clone()))
                        .collect(),
                });
            }
        }
    }

    // The creature itself, in the clear space under its cooldown list. It
    // takes whatever room is left between the last row and the log strip.
    if !pb.is_brawl() {
        let below = g.enemy_board_y + 30.0 + log.enemy().items.len() as f32 * 28.0 + 14.0;
        let room = (g.log.y - 16.0) - below;
        // A very heavily geared monster leaves no gap; drop the portrait
        // rather than draw it over the log.
        if room >= 48.0 {
            let sz = room.min(g.cd_w * 0.5).min(190.0);
            draw_monster(
                g.cd_x + (g.cd_w - sz) / 2.0,
                below + (room - sz).max(0.0) / 2.0,
                sz,
                log.spec().sprite,
                col_foe(),
                Color::from_rgba(40, 22, 20, 255),
            );
        }
    }

    // ---- the quiet log strip ----
    let r = g.log;
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(20, 20, 30, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, Color::from_rgba(56, 56, 76, 255));
    let lh = line_h(14.0);
    let visible = (((r.h - 12.0) / lh) as usize).max(1);
    // Scrolled back from the newest line, clamped so it can never run off
    // either end of what there is.
    let newest = pb.lines.len().saturating_sub(visible);
    let start = newest.saturating_sub(log_scroll.min(newest));
    let end = (start + visible).min(pb.lines.len());
    for (i, line) in pb.lines[start..end].iter().enumerate() {
        let is_last = start + i + 1 == pb.lines.len();
        ui_text(
            line,
            r.x + 14.0,
            r.y + lh + i as f32 * lh,
            14.0,
            if is_last { WHITE } else { Color::from_rgba(128, 130, 150, 255) },
        );
    }
    // Say so when it is not following, or a player who scrolled up and forgot
    // thinks the fight has stopped.
    if start < newest {
        let held = format!("{} lines back  ·  scroll down to follow", newest - start);
        let w = text_width(&held, 12.0);
        ui_text(&held, r.x + r.w - w - 14.0, r.y + r.h - 8.0, 12.0, col_gold());
    }

    if pb.done {
        let (label, color) = match log.outcome {
            Outcome::Victory => ("VICTORY", col_ok()),
            Outcome::Defeat => ("DEFEAT", col_bad()),
            Outcome::Stalemate => ("STALEMATE", col_gold()),
        };
        // Over the loser's boards rather than the right column: that space is
        // the opponent's portrait and cooldown list, and the banner used to
        // sit right on top of it.
        let bw = text_width(label, 40.0) + 72.0;
        let bh = line_h(40.0) + 40.0;
        let bx = g.board_x + (g.board_w - bw) / 2.0;
        let by = g.enemy_board_y + (SLOT_H as f32 * MINI_CELL - bh) / 2.0;
        draw_rectangle(bx, by, bw, bh, Color::from_rgba(18, 18, 28, 250));
        draw_rectangle_lines(bx, by, bw, bh, 3.0, color);
        centered_text(label, bx + bw / 2.0, by + bh / 2.0 + line_h(40.0) * 0.34, 40.0, color);
    }

    let btn = g.buttons;
    if pb.done {
        // Nothing happens until this is clicked, and people were missing that
        // - so it is large, it is named for what just happened, and it moves.
        let (label, color) = match log.outcome {
            Outcome::Victory => ("VICTORY - NEXT FIGHT", col_ok()),
            Outcome::Defeat => ("DEFEAT - TRY AGAIN", col_bad()),
            Outcome::Stalemate => ("STALEMATE - TRY AGAIN", col_gold()),
        };
        let r = g.primary;
        let hot = r.contains(Vec2::new(mx, my));
        // A slow pulse on the fill and the border. Brightness rather than hue,
        // so it still reads as movement without colour.
        let p = ((get_time() * 3.4).sin() * 0.5 + 0.5) as f32;
        let lift = if hot { 0.30 } else { 0.16 * p };
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            Color::new(color.r * lift, color.g * lift, color.b * lift, 1.0),
        );
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0 + 1.5 * p, color);
        let size = fitting_size(label, r.w - 28.0, &[24.0, 22.0, 20.0, 18.0]);
        centered_text(label, r.x + r.w / 2.0, r.y + 26.0, size, WHITE);
        centered_text("click to continue", r.x + r.w / 2.0, r.y + 46.0, 13.0, color);
    } else {
        button(btn[0], "BACK TO GEAR", true, mx, my);
        button(btn[1], "SKIP", true, mx, my);
    }
    button(btn[2], "REMATCH", true, mx, my);
    button(btn[3], if log_expanded { "HIDE LOG" } else { "FULL LOG" }, true, mx, my);
    button(btn[4], &format!("SPEED {}x", speed_label(pb.speed)), true, mx, my);

    // The full transcript is an overlay, so it never pushes the boards around.
    if log_expanded {
        render_log_overlay(pb, log, log_scroll);
    } else {
        // Hovering a cooldown row explains what that item is worth. Same
        // columns the bars were drawn in, so the hover lands where you look.
        #[allow(clippy::type_complexity)]
        let mut hoverable: Vec<(&Vec<RunningItem>, f32, &Vec<ItemProfile>, usize, f32)> =
            vec![(&log.player.items, g.player_board_y, &pb.player_profiles, 0usize, f32::MAX)];
        {
            let n = pb.foes.len().max(1);
            let room = (LOGICAL_H - g.enemy_board_y - 150.0).max(60.0);
            for (i, foe) in pb.foes.iter().enumerate() {
                let items = log.enemies.get(i).map(|e| &e.items).unwrap_or(&log.player.items);
                let top = g.enemy_board_y + i as f32 * (room / n as f32);
                hoverable.push((items, top, &foe.profiles, foe.attack_count, room / n as f32));
            }
        }
        for (items, top, profiles, offset, share) in hoverable {
            let order = cooldown_order(items);
            let (pitch, shown) = cooldown_fit(order.len(), cooldown_room(&g, top, share));
            for (row, &i) in order.iter().take(shown).enumerate() {
                if !cooldown_row_rect(&g, top, row, pitch).contains(Vec2::new(mx, my)) {
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

/// The list behind a "+ N more", and a full card for whichever row you point
/// at.
///
/// Returns the panel's rect so the caller can tell a click inside it from a
/// click that should put it away.
fn render_pinned(pin: &Pinned, run: &Run, pinned: bool, mx: f32, my: f32) -> Rect {
    let row_h = 30.0;
    let w = 300.0;
    let h = 34.0 + pin.entries.len() as f32 * row_h + 10.0;
    let x = pin.at.x.min(LOGICAL_W - w - 8.0).max(4.0);
    let y = pin.at.y.min(LOGICAL_H - h - 8.0).max(4.0);

    draw_rectangle(x, y, w, h, Color::from_rgba(14, 14, 22, 250));
    draw_rectangle_lines(x, y, w, h, 1.5, if pinned { col_gold() } else { Color::from_rgba(120, 120, 155, 255) });
    ui_text(&pin.title, x + 12.0, y + 21.0, 13.0, col_gold());
    if !pinned {
        let hint = words::word("click-to-pin", "click to hold it open");
        ui_text(&hint, x + w - 10.0 - text_width(hint, 11.0), y + 21.0, 11.0, col_dim());
    }

    let mut hovered: Option<&PinnedEntry> = None;
    for (i, e) in pin.entries.iter().enumerate() {
        let ry = y + 34.0 + i as f32 * row_h;
        let row = Rect::new(x + 4.0, ry, w - 8.0, row_h - 2.0);
        // Only a pinned list answers the pointer: an unpinned one is a tooltip
        // and the cursor is still over the "+ N more" that summoned it.
        let hot = pinned && row.contains(Vec2::new(mx, my));
        if hot {
            draw_rectangle(row.x, row.y, row.w, row.h, Color::from_rgba(255, 255, 255, 20));
            hovered = Some(e);
        }
        if let PinnedEntry::Item(p) = e {
            draw_item_sigil(x + 10.0, ry + 4.0, 16.0, Some(p.slot), p.sigil_seed, LIGHTGRAY);
        }
        let label = e.label();
        let size = fitting_size(&label, w - 46.0, &[14.0, 13.0, 12.0, 11.0]);
        ui_text(&label, x + 32.0, ry + 14.0, size, if hot { WHITE } else { LIGHTGRAY });
        draw_capped(&e.note(), x + 32.0, ry + 26.0, w - 44.0, 11.0, col_dim(), 1);
    }

    // The card for whatever is under the pointer, beside the list rather than
    // under it, so the list stays readable while you read the card. Whichever
    // side of the list has room takes it - the class band is pinned to the
    // right edge, so "always to the right" puts the card off the screen.
    if let Some(e) = hovered {
        let card_w = 420.0;
        let at = if x + w + 12.0 + card_w <= LOGICAL_W {
            Vec2::new(x + w + 12.0, my)
        } else {
            Vec2::new((x - card_w - 12.0).max(4.0), my)
        };
        match e {
            PinnedEntry::Item(p) => render_item_summary(p, run, at.x, at.y),
            PinnedEntry::Class(c) => {
                let mut lines = vec![
                    (words::class(c.name).to_string(), col_gold()),
                    (String::new(), WHITE),
                ];
                for l in wrap_px(&words::retell(&c.power.describe()), card_w - 40.0, 14.0) {
                    lines.push((l, LIGHTGRAY));
                }
                draw_tooltip(&lines, at.x, at.y);
            }
        }
    }
    Rect::new(x, y, w, h)
}

/// Everything one assembled item is worth: what it adds to you all the time,
/// and what it does each time its cooldown comes round.
fn render_item_summary(p: &ItemProfile, run: &Run, mx: f32, my: f32) {
    let lines = item_summary_lines(p, run);
    draw_tooltip_with_sigil(&lines, Some((Some(p.slot), p.sigil_seed)), mx, my);
}

/// The body of that card, so the loadout screen can show the same thing in a
/// hover of its own rather than keeping a second, worse description in sync.
/// What kind of harm a hit is made of, when it is made of more than one.
///
/// A single figure says how hard it hits and nothing about what it hits with,
/// which matters: physical and magic answer to different resistances, and a
/// build that is half of each is a different proposition from one that is all
/// of either. Silent when it is only one kind - there is nothing to break down.
fn damage_breakdown(p: &ItemProfile, strength: i32) -> String {
    // Strength rides with the physical half, the same way it does in a swing.
    let phys = p.stats.physical_damage + if p.slot == SlotKind::Weapon { strength } else { 0 };
    let magic = p.stats.magic_damage;
    let parts: Vec<String> = [(phys, "physical"), (magic, "magic")]
        .into_iter()
        .filter(|(v, _)| *v > 0)
        .map(|(v, n)| {
            // Through the item's multiplier, so the parts add up to the total
            // printed beside them.
            format!("{} {}", (v as i64 * p.power as i64 / 100) as i32, n)
        })
        .collect();
    match parts.len() {
        0 => String::new(),
        // One kind: name it, because "hits for 61" says how hard and nothing
        // about what answers it.
        1 => format!(" {}", parts[0].split_once(' ').map(|(_, k)| k).unwrap_or("")),
        // Several: the total is already printed, so this is the split.
        _ => format!(" ({})", parts.join(" + ")),
    }
}

fn item_summary_lines(p: &ItemProfile, run: &Run) -> Vec<(String, Color)> {
    // Everything this card says is the engine's vocabulary, so the whole
    // thing is re-told at the end rather than word by word as it is built.
    let lines = item_summary_lines_plain(p, run);
    lines.into_iter().map(|(s, c)| (words::retell(&s), c)).collect()
}

fn item_summary_lines_plain(p: &ItemProfile, run: &Run) -> Vec<(String, Color)> {
    let total = run.player_stats();
    let st = p.stats;
    let mut lines: Vec<(String, Color)> = vec![
        (p.full_name.clone(), col_gold()),
        (
            format!("{} · built on a {}", p.slot.name(), p.core),
            col_dim(),
        ),
        {
            let r = p.rarity();
            let tail = match r.next_at() {
                Some(next) => format!("  ({} more for the next mark)", next - p.rating),
                None => String::new(),
            };
            (
                format!("{} · rating {}{}", r.name().to_uppercase(), p.rating, tail),
                rarity_color(r),
            )
        },
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
    for (v, label) in [
        (st.physical_resist, "physical resist"),
        (st.magic_resist, "magic resist"),
        (st.physical_pierce, "physical piercing"),
        (st.magic_pierce, "magic piercing"),
        (st.physical_harden, "physical hardening"),
        (st.magic_harden, "magic hardening"),
    ] {
        if v != 0 {
            passive.push(format!("{:+}% {}", v, label));
        }
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
    let hit = p.hit_for(total.strength);
    if hit > 0 {
        let dps = p.dps_milli(total.strength);
        // A cast has two intensities and the printed figure is neither of
        // them: paid it lands at EMPOWERED_CAST_PCT, unpaid at WEAK_CAST_PCT.
        // The card used to show the bare number and then mention the weak
        // branch as a footnote, which read as though the number *was* the paid
        // one - so a crystal ball looked like less than half of what it does.
        if p.casts.is_empty() {
            lines.push((
                format!(
                    "  hits for {}{}  ({}.{} a second)",
                    hit,
                    damage_breakdown(p, total.strength),
                    dps / 1000,
                    (dps % 1000) / 100
                ),
                Color::from_rgba(240, 210, 190, 255),
            ));
        } else {
            use gearmaster_engine::combat::{EMPOWERED_CAST_PCT, WEAK_CAST_PCT};
            let paid = hit * EMPOWERED_CAST_PCT / 100;
            let weak = hit * WEAK_CAST_PCT / 100;
            let paid_dps = dps * EMPOWERED_CAST_PCT as i64 / 100;
            lines.push((
                format!(
                    "  casts for {}{} paid, {} unpaid  ({}.{} a second paid)",
                    paid,
                    damage_breakdown(p, total.strength),
                    weak,
                    paid_dps / 1000,
                    (paid_dps % 1000) / 100
                ),
                Color::from_rgba(240, 210, 190, 255),
            ));
        }
    }
    // An unconditional pool gain is a stat wearing a trigger's clothes. Fold
    // those into the figures below, so a piece that banks two faith reads
    // "2 faith" like every other piece that banks two faith - rather than
    // "on activation, gain 2 faith" in trigger colours. Anything conditional -
    // spending a pool, answering a neighbour, landing a curse - keeps its own
    // line, because there the wording is the information.
    let mut banked = [0i32; 4];
    // Only the four bankable pools fold into a summary line. A fusion is made
    // rather than granted, so it never reaches here - `Action::Gain` cannot
    // name one - and it keeps its own line with the rest of the conditionals.
    let slot_of = |r: Resource| r.index().min(3);
    let mut conditional: Vec<&Trigger> = Vec::new();
    for t in &p.triggers {
        match t {
            Trigger::OnActivate(Action::GainMana(n)) => banked[0] += n,
            Trigger::OnActivate(Action::Gain { what, amount }) => {
                banked[slot_of(*what)] += amount
            }
            other => conditional.push(other),
        }
    }

    let mut acts: Vec<String> = Vec::new();
    if st.physical_damage > 0 {
        acts.push(format!("{} physical damage", st.physical_damage));
    }
    if st.magic_damage > 0 {
        acts.push(format!("{} magic damage", st.magic_damage));
    }
    if st.mind > 0 {
        acts.push(format!("{} mind damage", st.mind));
    }
    if st.armor > 0 {
        acts.push(format!("{} armor", st.armor));
    }
    for (i, name) in ["mana", "rage", "faith", "nature"].iter().enumerate() {
        let total = banked[i]
            + match i {
                0 => st.mana,
                1 => st.rage,
                2 => st.faith,
                _ => st.nature,
            };
        if total > 0 {
            acts.push(format!("{} {}", total, name));
        }
    }
    // What the two figures above cost. One price per activation however many
    // voices the item has, which is what a crystal ball is for.
    if !p.casts.is_empty() {
        let voices = p.casts.len();
        acts.push(if voices > 1 {
            format!(
                "costs {} mana an activation, whichever of its {} spells come up",
                gearmaster_engine::combat::SPELL_MANA_COST,
                voices
            )
        } else {
            format!("costs {} mana a cast", gearmaster_engine::combat::SPELL_MANA_COST)
        });
    }
    // The item's whole multiplier, once. It used to name only the ink's
    // share, which said nothing about what the item actually multiplies by -
    // and everything above is already multiplied, so the figure has to be the
    // total or the card contradicts itself.
    if p.power != 100 {
        acts.push(format!(
            "everything above is already x{}.{:02} - this item's own power",
            p.power / 100,
            p.power.abs() % 100
        ));
    }
    let nothing = hit == 0 && acts.is_empty() && conditional.is_empty();
    for a in &acts {
        lines.push((format!("  {}", a), Color::from_rgba(240, 210, 190, 255)));
    }
    for t in conditional {
        for l in wrap(&t.describe(), 52) {
            lines.push((format!("  {}", l), col_trigger()));
        }
    }
    if nothing {
        lines.push(("  ticks over doing nothing".to_string(), col_dim()));
    }
    lines
}

/// A monster's innate attack, which has no components behind it.
fn render_innate_summary(it: &gearmaster_engine::combat::RunningItem, mx: f32, my: f32) {
    let mut lines: Vec<(String, Color)> = vec![
        (words::retell(&it.name), col_gold()),
        ("innate - not gear".to_string(), col_dim()),
        (
            format!("IN COMBAT - every {:.2}s", it.cooldown_ms as f32 / 1000.0),
            Color::from_rgba(240, 190, 140, 255),
        ),
    ];
    if it.physical_damage > 0 {
        lines.push((
            format!("  {} physical damage", it.physical_damage),
            Color::from_rgba(240, 210, 190, 255),
        ));
    }
    if it.magic_damage > 0 {
        lines.push((
            format!("  {} magic damage", it.magic_damage),
            Color::from_rgba(240, 210, 190, 255),
        ));
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
fn draw_tooltip_with_sigil(
    lines: &[(String, Color)],
    sigil: Option<(Option<SlotKind>, u64)>,
    mx: f32,
    my: f32,
) {
    let art = if sigil.is_some() { 62.0 } else { 0.0 };
    let lh = line_h(14.0);
    let w = lines
        .iter()
        .map(|(s, _)| text_width(s, 14.0))
        .fold(0.0_f32, f32::max)
        + 26.0
        + art;
    let h = (lines.len() as f32 * lh + 18.0).max(art + 18.0);
    let x = (mx + 18.0).min(LOGICAL_W - w - 6.0).max(4.0);
    let y = (my + 18.0).min(LOGICAL_H - h - 6.0).max(4.0);
    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 248));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(120, 120, 155, 255));
    if let Some((slot, seed)) = sigil {
        draw_item_sigil(x + 10.0, y + 10.0, 54.0, slot, seed, Color::from_rgba(228, 214, 170, 255));
    }
    for (i, (s, c)) in lines.iter().enumerate() {
        ui_text(s, x + 13.0 + art, y + lh + i as f32 * lh, 14.0, *c);
    }
}

fn draw_tooltip(lines: &[(String, Color)], mx: f32, my: f32) {
    draw_tip(&Tip::plain(lines.to_vec()), mx, my);
}

/// What a slot needs, as a tooltip.
///
/// The split that matters is between what makes an item work at all and what
/// merely makes it better, so those get a row each and the optional row is
/// indented under the one it adds to. A slot that can be built more than one
/// way puts each way in its own frame, because the requirements of one have
/// nothing to do with the requirements of another.
fn recipe_tip(slot: SlotKind) -> Tip {
    let parts = gearmaster_engine::piece::recipe_parts(slot);
    let many = parts.len() > 1;
    let mut lines = vec![(slot.name().to_string(), WHITE)];
    if many {
        lines.push((String::from("any one of these:"), col_dim()));
    }
    let mut boxes = Vec::new();

    for p in &parts {
        let start = lines.len();
        if many && !p.title.is_empty() {
            lines.push((p.title.to_string(), col_gold()));
        }
        lines.push((format!("needs {}", p.required.join(" + ")), LIGHTGRAY));
        if p.optional.is_empty() {
            lines.push((String::from("    nothing else can be added"), col_dim()));
        } else {
            // Indented, and worded so it is clear these are improvements to
            // gear that already counts rather than more requirements.
            lines.push((format!("    may add {}", p.optional.join(" and ")), col_dim()));
        }
        if many {
            boxes.push((start, lines.len()));
        }
    }
    Tip { lines, boxes }
}

fn draw_tip(tip: &Tip, mx: f32, my: f32) {
    let lines = &tip.lines;
    // Boxed lines are inset, so they need the extra room reserved up front or
    // the frame would run through the text.
    let pad = if tip.boxes.is_empty() { 0.0 } else { 16.0 };
    let w = lines.iter().map(|(s, _)| text_width(s, 14.0)).fold(0.0_f32, f32::max) + 26.0 + pad;
    let lh = line_h(14.0);
    // A gap above each frame, so consecutive boxes do not share an edge and
    // read as one banded list instead of as separate cards.
    let gap = 13.0;
    let h = lines.len() as f32 * lh + 18.0 + tip.boxes.len() as f32 * gap;
    let x = (mx + 18.0).min(LOGICAL_W - w - 6.0).max(4.0);
    let y = (my + 18.0).min(LOGICAL_H - h - 6.0).max(4.0);
    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 248));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(120, 120, 155, 255));

    // Baseline of each line, shifted down by the frames opened above it.
    let top_of = |i: usize| -> f32 {
        let opened = tip.boxes.iter().filter(|(s, _)| *s <= i).count() as f32;
        y + lh + i as f32 * lh + opened * gap
    };
    for &(s, e) in &tip.boxes {
        if s >= lines.len() {
            continue;
        }
        let e = e.min(lines.len());
        let top = top_of(s) - lh + 3.0;
        let bot = top_of(e.saturating_sub(1)) + 7.0;
        // Filled as well as framed: against a near-black tooltip a thin line
        // alone reads as a rule between rows rather than as a card around one.
        draw_rectangle(x + 7.0, top, w - 14.0, bot - top, Color::from_rgba(30, 30, 44, 255));
        draw_rectangle_lines(
            x + 7.0,
            top,
            w - 14.0,
            bot - top,
            1.5,
            Color::from_rgba(132, 132, 172, 255),
        );
    }
    for (i, (s, c)) in lines.iter().enumerate() {
        let inset = if tip.boxes.iter().any(|&(a, b)| i >= a && i < b) { 8.0 } else { 0.0 };
        ui_text(s, x + 13.0 + inset, top_of(i), 14.0, *c);
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
/// One stat's history over the fight, as sampled from the log.
struct Series {
    label: &'static str,
    color: Color,
    points: Vec<(u32, i32)>,
}

impl Series {
    fn new(label: &'static str, color: Color) -> Self {
        Series { label, color, points: vec![(0, 0)] }
    }
    fn at(&mut self, t: u32, v: i32) {
        self.points.push((t, v));
    }
    fn last(&self) -> i32 {
        self.points.last().map(|(_, v)| *v).unwrap_or(0)
    }
}

/// Replay the log into one series per stat per side.
///
/// The fight is already decided; this is only reading back what happened, so
/// it can be recomputed whenever the overlay opens rather than carried around.
fn build_series(log: &CombatLog, side: Side) -> Vec<Series> {
    let start_hp = if matches!(side, Side::Player) { log.player.max_health } else { log.enemy().max_health };
    let mut hp = Series::new("health", if matches!(side, Side::Player) { col_you() } else { col_foe() });
    hp.points[0] = (0, start_hp);
    let mut armor = Series::new("armour", pool_color("armor"));
    let mut mana = Series::new("mana", pool_color("mana"));
    let mut rage = Series::new("rage", pool_color("rage"));
    let mut faith = Series::new("faith", pool_color("faith"));
    let mut nature = Series::new("nature", pool_color("nature"));

    for e in &log.entries {
        let t = e.at_ms;
        match &e.event {
            Event::Hit { by, target_health, target_armor, .. } if *by != side => {
                hp.at(t, *target_health);
                armor.at(t, *target_armor);
            }
            Event::GainArmor { side: s, total, .. } if *s == side => armor.at(t, *total),
            Event::GainMana { side: s, total, .. } if *s == side => mana.at(t, *total),
            Event::ManaCheck { side: s, remaining, .. } if *s == side => mana.at(t, *remaining),
            Event::GainResource { side: s, what, total, .. } if *s == side => match *what {
                "rage" => rage.at(t, *total),
                "faith" => faith.at(t, *total),
                _ => nature.at(t, *total),
            },
            Event::ResourceCheck { side: s, what, remaining, .. } if *s == side => match *what {
                "rage" => rage.at(t, *remaining),
                "faith" => faith.at(t, *remaining),
                _ => nature.at(t, *remaining),
            },
            Event::Burn { side: s, health, .. } if *s == side => hp.at(t, *health),
            Event::Regen { side: s, health, .. } if *s == side => hp.at(t, *health),
            Event::MindHit { by, target_max_health, .. } if *by != side => {
                // Maximum health falling is worth seeing on the same line.
                let now = hp.last().min(*target_max_health);
                hp.at(t, now);
            }
            _ => {}
        }
    }
    vec![hp, armor, mana, rage, faith, nature]
        .into_iter()
        .filter(|s| s.points.iter().any(|(_, v)| *v != 0))
        .collect()
}

/// One line chart. Time runs left to right; the peak sets the top.
fn draw_series(r: Rect, s: &Series, duration_ms: u32) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(14, 14, 22, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::from_rgba(52, 52, 72, 255));

    let peak = s.points.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1) as f32;
    let span = duration_ms.max(1) as f32;
    let at = |(t, v): (u32, i32)| {
        (r.x + (t as f32 / span) * r.w, r.y + r.h - (v.max(0) as f32 / peak) * (r.h - 4.0) - 2.0)
    };

    // A step chart: these are discrete events, not a smooth signal, and
    // joining them with slopes would imply values that never existed.
    let mut prev = at(s.points[0]);
    for &p in &s.points[1..] {
        let next = at(p);
        draw_line(prev.0, prev.1, next.0, prev.1, 1.5, s.color);
        draw_line(next.0, prev.1, next.0, next.1, 1.5, s.color);
        prev = next;
    }
    draw_line(prev.0, prev.1, r.x + r.w, prev.1, 1.5, s.color);

    ui_text(s.label, r.x + 5.0, r.y + 14.0, 11.0, s.color);
    let peak_label = format!("{}", peak as i32);
    ui_text(&peak_label, r.x + r.w - text_width(&peak_label, 11.0) - 5.0, r.y + 14.0, 11.0, col_dim());
}

/// The full transcript: what each side's stats did over the fight, and the
/// blow-by-blow underneath, grouped by whatever set each exchange off.
fn render_log_overlay(pb: &Playback, log: &CombatLog, scroll: usize) {
    let pad = 60.0;
    let r = Rect::new(pad, pad, LOGICAL_W - 2.0 * pad, LOGICAL_H - 2.0 * pad - 40.0);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 226));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(110, 110, 145, 255));
    ui_text("COMBAT LOG", r.x + 16.0, r.y + 28.0, 18.0, col_gold());
    ui_text(
        &format!("{:.1}s  ·  {}", log.duration_ms as f32 / 1000.0, log.enemy().name),
        r.x + 30.0 + text_width("COMBAT LOG", 18.0),
        r.y + 28.0,
        13.0,
        col_dim(),
    );

    // ---- graphs, your side above theirs ----
    let chart_h = 62.0;
    let gap = 8.0;
    let mut gy = r.y + 46.0;
    for (side, who, tint) in
        [(Side::Player, "YOU", col_you()), (Side::Enemy, log.enemy().name.as_str(), col_foe())]
    {
        ui_text(who, r.x + 16.0, gy + 12.0, 13.0, tint);
        let series = build_series(log, side);
        let cols = series.len().max(1);
        let cw = (r.w - 32.0 - (cols as f32 - 1.0) * gap) / cols as f32;
        for (i, s) in series.iter().enumerate() {
            draw_series(
                Rect::new(r.x + 16.0 + i as f32 * (cw + gap), gy + 18.0, cw, chart_h),
                s,
                log.duration_ms,
            );
        }
        gy += chart_h + 34.0;
    }

    // ---- the blow-by-blow ----
    let list_top = gy + 6.0;
    draw_line(r.x + 16.0, list_top - 8.0, r.x + r.w - 16.0, list_top - 8.0, 1.0, Color::from_rgba(60, 60, 84, 255));

    let lh = line_h(13.0);
    let visible = (((r.y + r.h - list_top - 12.0) / lh) as usize).max(1);
    // The list is pinned to the newest line; scrolling walks that anchor back
    // up, and cannot walk past the top.
    let newest = log.entries.len().saturating_sub(visible);
    let start = newest.saturating_sub(scroll.min(newest));
    let end = (start + visible).min(log.entries.len());
    for (i, e) in log.entries[start..end].iter().enumerate() {
        // Who did it, and whether it is the thing that fired or a consequence
        // of it - an activation names its item and sits proud of the rest.
        let (indent, colour) = match &e.event {
            Event::Activate { side, .. } => {
                (0.0, if matches!(side, Side::Player) { col_you() } else { col_foe() })
            }
            Event::Hit { by, .. } | Event::MindHit { by, .. } => (
                18.0,
                if matches!(by, Side::Player) { col_you() } else { col_foe() },
            ),
            Event::GainArmor { side, .. }
            | Event::GainMana { side, .. }
            | Event::GainResource { side, .. }
            | Event::ManaCheck { side, .. }
            | Event::ResourceCheck { side, .. }
            | Event::Regen { side, .. }
            | Event::Burn { side, .. } => (
                18.0,
                if matches!(side, Side::Player) { col_you() } else { col_foe() },
            ),
            _ => (18.0, Color::from_rgba(170, 172, 192, 255)),
        };
        ui_text(
            &words::retell(&log.describe(e)),
            r.x + 16.0 + indent,
            list_top + lh + i as f32 * lh,
            13.0,
            colour,
        );
    }
    // Say where in the log you are, so a wheel that does nothing reads as
    // "already at the end" rather than "broken".
    if log.entries.len() > visible {
        let note = format!(
            "{}-{} of {}   wheel or up/down to scroll{}",
            start + 1,
            end,
            log.entries.len(),
            if scroll > 0 { "   END to jump to the end" } else { "" },
        );
        ui_text(&note, r.x + r.w - 16.0 - text_width(&note, 12.0), r.y + 28.0, 12.0, col_dim());
    }
    let _ = pb;
}

/// Plain-English meanings for the words the interface throws around. Opened
/// from the panel or with G, and available mid-fight too.
const GLOSSARY: &[(&str, &str)] = &[
    ("HEALTH", "How much damage you can take. At zero the fight is over."),
    ("ARMOR", "Temporary hit points. Starts every fight at ZERO - gear builds it up as it activates - and soaks damage before health does."),
    ("STRENGTH", "Added to every weapon hit, before power multiplies it."),
    ("POWER", "A multiplier on weapon damage, shown like 2.45x."),
    ("DAMAGE", "Flat damage a component lends the weapon it is built into."),
    ("A HIT", "(the item's flat damage + your strength) x your power. Only assembled weapons swing."),
    ("REGEN", "Health restored per second, never above your maximum."),
    ("MANA", "Banked by some items, spent by others. A trigger that cannot pay runs its failure branch instead - often a curse on you."),
    ("MIND DAMAGE", "Lowers your MAXIMUM health rather than your current, so regeneration can never win it back."),
    ("MIND RESIST", "Percent cut to incoming mind damage."),
    ("CURSE", "A timed effect landed on either fighter - yourself included."),
    ("  SEARING", "10 damage a second, for 10 seconds. Stacks burn together, so a second one landing doubles the rate."),
    ("  FROST", "ALL of the target's gear runs 50% slower, for 1 second - not just the item that was hit. Stacks add up to 75%, and never past it: frost slows gear, it never stops it."),
    ("CURSE RESIST", "Shortens curses landed on you. At 100% they never land."),
    ("STACKS", "Landing the same curse again while it is still up. Searing burns faster, frost slows harder, misfire eats more, and stun lasts longer - each up to its own ceiling. The count sits beside the curse's name on the bar."),
    (
        "MANA EMPOWERMENT",
        "Each stack adds 0.05x weapon power per point of mana you are STILL \
         holding. Stacks cost mana, so spending your whole pool for one leaves \
         it worth nothing - you need income above the cost.",
    ),
    (
        "MANA SHIELD",
        "Each stack cuts 1 off every incoming hit per point of mana you are still \
         holding - damage of any kind, before armour. Same catch: it scales off \
         what is left, not what you spent.",
    ),
    ("COOLDOWN", "Seconds between one item's activations. Every item runs its own."),
    ("CORE", "The component a recipe needs exactly one of: handle, frame, base, material, book or crystal ball. It anchors an item, which is why two items can touch and still count separately."),
    ("ASSEMBLED", "An item whose components match its slot's recipe. Only assembled items act in combat - loose pieces still give passive stats, but never fire."),
    ("ADJACENT", "Two items in the same slot whose cells orthogonally touch."),
    ("ALIGNED", "Two items in different slots whose rows overlap."),
    ("RECIPE", "What a slot will accept as a finished item. Most slots have one; the weapon slot has three, so the same grid builds either a weapon or a spell. Hover the ? beside a slot to read them."),
    ("REQUIRED / OPTIONAL", "Every recipe has a minimum that makes an item work at all, and room for more on top. A helmet is finished with a frame and one plating; the second plating and the crest only make it better. The slot's ? shows the two apart, with the optional part indented."),
    ("SHARED PIECES", "Materials go in gloves or greaves, and plating in helmets or greaves. A piece that fits more than one grid is drawn grey with a hollow diamond and takes that grid's colour and mark when you drop it in."),
    ("MOLDS", "Gloves and greaves both need a mold, but the two do not interchange - a gloves mold will not go on a shin. The role under a card's name says which it is."),
    ("LOCKED ITEM", "Shift-click a finished item to lock it. A locked item stops looking for other pieces to join, turns as one piece, and moves in and out of the inventory whole. Shift-click again to release it."),
    ("PINNED CARD", "Right-click a shop card to pin it. A reroll leaves pinned cards where they are, so you can hold something you cannot yet afford."),
    ("SPELL", "A weapon built the arcane way: a book or a crystal ball, ink, and the spell itself. It fills the weapon slot like any other item."),
    ("GROWING", "A few pieces raise your maximum health every time they fire, and you keep it - the health won in one fight is health you start the next one with, for the rest of the run. That is what makes them the dearest things in the shop. A stalemate banks nothing: surviving the clock would otherwise be the most profitable thing you could do."),
    ("STANDING ALONE", "Some gear multiplies every number on its item, but only while that item is alone - nothing else finished sharing its rows, or nothing overlapping it once the five grids are laid on top of one another. The multipliers are large and the conditions are easy to break by accident. That is the trade."),
    ("CASTING", "Every spell has two strengths. Paid for, it lands in full; with no mana to spend it still goes off, but at less than half. A build that runs dry gets weaker rather than stopping, so mana income is the difference between a spell that works and one that merely happens."),
    ("STUN", "ALL of their gear stops dead. Not slowed - stopped, and a cooldown part-way through resumes from where it stood rather than starting over. A stunned item shows an amber stun meter in place of its cooldown bar, running the other way and settling as it wears off. Stacks add to the clock, up to 3.6 seconds."),
    ("MISFIRE", "One activation in three does nothing at all. The cooldown comes round, and nothing comes of it. Two stacks or more makes it one in two, which is as bad as it gets."),
    ("BOOK", "The core of a spell, the way a handle is the core of a weapon. It sets how often the spell casts, and binds exactly one spell."),
    ("INK", "Multiplies the cast it is bound into, and only that one - ink never touches your own weapon power. Stronger inks want paying for."),
    ("CRYSTAL BALL", "The other kind of spell core. It holds two or three spells and casts a different one each time it comes round, where a book casts its one every time."),
    ("RATING", "How much an assembled item actually does per second, on a scale where the best possible item in its slot is 200. Scaled per slot, so a glove and a weapon can be compared."),
    ("RARE / EPIC / LEGENDARY", "Rating tiers, worn as one, two or three marks beside an item's name. Better gear costs far more in the shop."),
    ("QUEST", "A task carried by one component - so many activations, so many curses. It only counts while the component is part of an assembled item. Finishing it turns the component into something else, which may not belong in the slot it was sitting in."),
    ("DIFFICULTY", "Medium is the game as intended. Easy is half as hard, hard three times, insane nine. Most of that comes from the gear the opposition is wearing rather than from its numbers: a harder setting steps every one of its components up a rung, so the same creature turns up better equipped instead of merely inflated. What is left over splits evenly between staying alive and hitting back."),
    ("PASSIVE", "A standing rule on a combatant, granted by the difficulty. Hardened regenerates, Warded resists mind and curse, Relentless runs every item faster."),
    ("GRINDER", "A mode. Losing drops you to the rung you last cleared, so there is always something easier to farm."),
    ("ROGUE", "A mode. Losing costs one of three lives; the third ends the run and takes everything with it."),
    ("BOUNTY", "Paid whether you win or lose. Losing never moves you up the ladder, but it does pay - a run with no income cannot buy its way past whatever just beat it."),
    ("UNDO", "Steps the board back one change. It covers placing, moving, turning and clearing, but never a purchase."),
    ("PHYSICAL / MAGIC", "The two damage types. Every number a piece of gear deals is one or the other, and each has its own set of defences, so resistance is always worth something. Mind damage is the exception: it is not reduced by either, only by mind resistance."),
    ("RESISTANCE", "Cuts incoming damage of its type, in percent."),
    ("PIERCING", "Ignores that much of the target's resistance of that type. Stacking resistance alone loses to a pierced attacker."),
    ("HARDENING", "Blunts that much of an attacker's piercing. Stacking piercing alone loses to a hardened defender."),
    ("RAGE", "Banked by some gear. Every point adds physical damage while you hold it, and some triggers spend it for a burst."),
    ("FAITH", "Banked slowly. Every point adds resistance of both types while held, up to 40%."),
    ("NATURE", "Banked by growing things. Every point adds regeneration while held."),
    ("CLASS", "Read off your build at a fountain, never chosen. Three fountains stand on the ladder - two that name you something new and a deep one that doubles a title you already hold - and every class you are carrying applies at once. Some are not poured at all but earned, off the road: a dungeon, an event, a town. Hover the class panel to see your build drawn as a shape, what you would be given now, and what you are nearest to otherwise."),
    ("STACKING CLASSES", "Most titles are held once. Two are not: Piety and Tired are handed out by a town over and over, and each one you take counts. A stacked class is shown with its count beside the name, and its power is multiplied by it."),
    ("THE FOUNTAIN", "Not a fight, and not a rung: drinking costs you nothing and the creature standing there is still to be fought. It measures your gear along a set of axes - how much magic, how much iron, how fast, how woven together - and gives you the most demanding class you qualify for. The second fountain will not repeat the first."),
    ("A TROPHY", "What a named creature leaves behind. The counter pays nothing for one - they are priced off a scale the shop does not use, and one of them used to pay for a whole run - so selling is not what they are for. A town pub takes one for a stack of Recycler, and nothing else in the game will take one at all."),
    ("ADJACENCY BONUS", "A flat lump a component pays only once its item comes together. One piece in each recipe carries one, and it is the difference between a board that finishes what it seats and one that fills cells with loose pieces. Recycler counts them for more."),
    ("SUDDEN DEATH", "After 30 seconds both fighters start losing health every second - 1% of maximum, then 2%, then 3%, climbing until somebody falls. It goes through armour and resistance and answers to nothing. No fight can run for ever, and a wall that cannot kill is no longer a wall that cannot lose. If you both go down on the same tick, whoever was further from zero takes it, and a dead heat goes to you."),
    ("BRAWL", "Some events put more than one creature across the table. Your aim moves along after every attack, so a brawl comes down together rather than one at a time, and every one of them acts against you independently. A brawl is not a rung: whichever way it goes, the fight the ladder had waiting is still waiting."),
    ("TOWN", "A rung with nothing on it to fight, inserted between two that do have something. At the gate you either walk on - which pays the last bounty a second time - or go in, and going in buys exactly one of four things: the chapel, the pub, the factory, or the shop. One a visit, and a town is only ever visited once."),
    ("RUMOUR", "A component that is not gear. It has one empty cell, it never goes on a board, and all it has to do is be carried. What it is for is standing as the condition on a door that will not otherwise be there - and its description tells you roughly what sets it off rather than exactly, because working that out is the whole of it."),
    ("BARTER", "How the pub sells. It does not take money: a rumour is paid for by handing over a loose component of the kind it asks for, or another rumour. Click the shelf, and anything in the tray they would take lights up."),
    ("MANA DEBT", "Mana below zero, which is what a stack of Tired starts you on. Nothing that spends mana can pay while the pool is under water - your income has to carry it back above the cost first. A board that never spends mana never notices."),
    ("BANKED AT THE BELL", "Every pool starts a fight at zero and earns its way up, which is why the opening of every fight looks much the same whatever you are wearing. One thing changes that: a stack of Piety hands you a point of faith before the first tick."),
    ("A MISS", "An attack that comes to nothing at all - no damage, no curse, no drain. Ticket to Ride is the only thing that causes them, and it counts rather than rolls: every second attack made against you, per attacker. Exactly half, and it never streaks."),
    ("DRAIN", "Some gear takes a pool off the other side rather than adding to its own, and hurts them for what it took. It is the answer to a build that has banked more than it can spend."),
    ("EXTRA ROWS", "One reward in the game makes your grids taller rather than giving you something to put in them. Rows only ever go up - nothing that grants them can be sold - so a piece can never end up sitting in a row that is about to stop existing."),
    // Last, so it lands on the last page. It is a control as well as a
    // definition; see SKIP_TERM.
    (SKIP_TERM, "The road up the mountain, which most of us have walked more times than we care to count. Those who know it well are not made to walk it again: click these words and choose where to pick it up. Every rung on the way pays its bounty in full, as though each had been fought and won. It only runs upward. It keeps no quests. It asks no questions."),
];

/// What the fountain will hand over, as cards you choose between.
///
/// Returns the class chosen, if one was.
/// The tools drawer: a screenshot, and a run written down.
///
/// The code is the interesting half. It says what somebody built and how far
/// they got, in a string short enough to paste into a message, so a build can
/// be sent to somebody else and looked at rather than described.
///
/// Returns which button was pressed, if any.
fn render_tools(
    run: &Run,
    code: &str,
    imported: Option<&gearmaster_engine::share::Shared>,
    mx: f32,
    my: f32,
) -> Option<&'static str> {
    let pad = 120.0;
    let h = 470.0;
    let r = Rect::new(pad, (LOGICAL_H - h) / 2.0, LOGICAL_W - 2.0 * pad, h);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 236));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text("TOOLS", r.x + 28.0, r.y + 42.0, 24.0, col_gold());
    ui_text("Esc to close", r.x + 28.0, r.y + 64.0, 12.0, col_dim());

    let mut y = r.y + 100.0;
    ui_text("THIS RUN, WRITTEN DOWN", r.x + 28.0, y, 13.0, col_dim());
    y += 22.0;
    // Broken to a width rather than word-wrapped: a code is one long token
    // with no spaces in it, so `wrap_px` has nothing to break on and the line
    // simply ran off the right-hand edge. The whole thing has to be readable -
    // reading it off the screen is the fallback where there is no clipboard.
    let per_line = {
        let mut n = 8usize;
        while n < code.len() && text_width(&code[..n], 15.0) < r.w - 56.0 {
            n += 1;
        }
        n.saturating_sub(1).max(8)
    };
    for chunk in code.as_bytes().chunks(per_line) {
        let l = String::from_utf8_lossy(chunk).to_string();
        ui_text(&l, r.x + 28.0, y, 15.0, Color::from_rgba(210, 212, 230, 255));
        y += 19.0;
    }
    y += 6.0;
    ui_text(
        &format!(
            "rung {}  ·  {} won  ·  {} lost",
            run.rung + 1,
            run.wins,
            run.losses
        ),
        r.x + 28.0,
        y,
        12.0,
        col_dim(),
    );

    // What was pasted in, if anything.
    let mut iy = r.y + 250.0;
    ui_text("PASTED IN", r.x + 28.0, iy, 13.0, col_dim());
    iy += 22.0;
    match imported {
        None => {
            ui_text(
                "Nothing yet. Copy a friend's code, then press READ CLIPBOARD.",
                r.x + 28.0,
                iy,
                13.0,
                col_dim(),
            );
        }
        Some(sh) => {
            let (reg, lo) = sh.loadout();
            let items: usize = SlotKind::ALL
                .iter()
                .map(|k| lo.report(&reg, *k).items.iter().filter(|i| i.assembled).count())
                .sum();
            ui_text(
                &format!(
                    "rung {}  ·  {} won  ·  {} components in {} finished items",
                    sh.rung + 1,
                    sh.wins,
                    sh.placed.len(),
                    items
                ),
                r.x + 28.0,
                iy,
                14.0,
                col_ok(),
            );
            iy += 20.0;
            // The shared code carries class names as owned strings, so match
            // them back to the real ones before asking the theme.
            let titles: Vec<String> = sh
                .classes
                .iter()
                .map(|c| {
                    gearmaster_engine::class::CLASSES
                        .iter()
                        .find(|k| k.name == c)
                        .map(|k| words::class(k.name).to_string())
                        .unwrap_or_else(|| c.clone())
                })
                .collect();
            if !titles.is_empty() {
                ui_text(&titles.join(", "), r.x + 28.0, iy, 13.0, col_gold());
                iy += 20.0;
            }
            // The boards themselves, because the point of a shared code is
            // seeing how somebody packed. Sized to the room left above the
            // buttons rather than to the width, or five of them at full width
            // are taller than the drawer.
            let room_h = (r.y + r.h - 74.0) - iy - 14.0;
            let by_h = room_h / lo.rows() as f32;
            let by_w = (r.w - 56.0 - 4.0 * 10.0) / 5.0 / SLOT_W as f32;
            let cell = by_h.min(by_w);
            let bw = cell * SLOT_W as f32;
            for (i, kind) in SlotKind::ALL.iter().enumerate() {
                let bx = r.x + 28.0 + i as f32 * (bw + 10.0);
                render_share_board(&reg, &lo, *kind, bx, iy, bw);
            }
        }
    }

    let bw = 220.0;
    let by = r.y + r.h - 58.0;
    let mut hit = None;
    for (i, (id, label)) in
        [("shot", "SCREENSHOT"), ("copy", "COPY CODE"), ("paste", "READ CLIPBOARD")]
            .iter()
            .enumerate()
    {
        let b = Rect::new(r.x + 28.0 + i as f32 * (bw + 12.0), by, bw, 38.0);
        button(b, label, true, mx, my);
        if is_mouse_button_pressed(MouseButton::Left) && b.contains(Vec2::new(mx, my)) {
            hit = Some(*id);
        }
    }
    let close = Rect::new(r.x + r.w - 140.0, by, 120.0, 38.0);
    button(close, "CLOSE", true, mx, my);
    if is_mouse_button_pressed(MouseButton::Left) && close.contains(Vec2::new(mx, my)) {
        hit = Some("close");
    }
    hit
}

/// One shared board, drawn small enough that five fit in a row.
fn render_share_board(
    reg: &gearmaster_engine::piece::PieceRegistry,
    lo: &gearmaster_engine::loadout::Loadout,
    kind: SlotKind,
    x: f32,
    y: f32,
    w: f32,
) {
    let cell = w / SLOT_W as f32;
    let rows = lo.rows();
    draw_rectangle(x, y, w, cell * rows as f32, Color::from_rgba(24, 24, 34, 255));
    let slot = lo.slot(kind);
    for gy in 0..rows {
        for gx in 0..SLOT_W {
            let Some(id) = slot.get(gx, gy) else { continue };
            let c = slot_color(kind, kind_luminance(reg.def(id).kind));
            draw_rectangle(
                x + gx as f32 * cell,
                y + gy as f32 * cell,
                cell - 1.0,
                cell - 1.0,
                c,
            );
        }
    }
    draw_rectangle_lines(x, y, w, cell * rows as f32, 1.0, Color::from_rgba(70, 70, 95, 255));
    ui_text(kind.name(), x, y + cell * rows as f32 + 12.0, 10.0, col_dim());
}

/// An event standing in front of a rung: some prose and a row of choices.
///
/// One screen for all of them. An event is data - `EVENTS` in the engine - so
/// adding one is adding an entry there, and this draws whatever is in it.
/// Returns the choice clicked.
fn render_event(
    run: &Run,
    ev: &'static gearmaster_engine::event::LadderEvent,
    mx: f32,
    my: f32,
) -> Option<&'static gearmaster_engine::event::Choice> {
    let pad = 70.0;
    let w = LOGICAL_W - 2.0 * pad;
    // Sized to what it actually says. A fixed frame leaves a hand's width of
    // nothing between the last line and the buttons whenever an event is
    // brief, and there is no reason for a short scene to look like a long one
    // with something missing.
    let lines: usize =
        ev.prose.iter().map(|p| wrap_px(&words::retell_naming(p), w - 56.0, 15.0).len()).sum();
    let prose_h = lines as f32 * 20.0 + ev.prose.len() as f32 * 10.0;
    let h = (78.0 + prose_h + 24.0 + 120.0 + 30.0).clamp(300.0, LOGICAL_H - 40.0);
    let r = Rect::new(pad, (LOGICAL_H - h) / 2.0, w, h);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 236));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text(&words::retell(ev.title), r.x + 28.0, r.y + 42.0, 24.0, col_gold());

    let mut y = r.y + 78.0;
    for para in ev.prose {
        for l in wrap_px(&words::retell_naming(para), r.w - 56.0, 15.0) {
            ui_text(&l, r.x + 28.0, y, 15.0, Color::from_rgba(198, 200, 218, 255));
            y += 20.0;
        }
        y += 10.0;
    }

    let n = ev.choices.len().max(1);
    let gap = 18.0;
    let cw = (r.w - 56.0 - (n - 1) as f32 * gap) / n as f32;
    let top = r.y + r.h - 150.0;
    let mut chosen = None;
    for (i, c) in ev.choices.iter().enumerate() {
        let cell = Rect::new(r.x + 28.0 + i as f32 * (cw + gap), top, cw, 120.0);
        let open = run.choice_open(c);
        let hot = open && cell.contains(Vec2::new(mx, my));
        draw_rectangle(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { Color::from_rgba(46, 42, 30, 255) } else { Color::from_rgba(26, 26, 38, 255) },
        );
        draw_rectangle_lines(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { 2.5 } else { 1.5 },
            if hot {
                col_gold()
            } else if open {
                Color::from_rgba(64, 64, 88, 255)
            } else {
                Color::from_rgba(52, 40, 40, 255)
            },
        );
        let mut cy = cell.y + 28.0;
        ui_text(
            &words::retell(c.label),
            cell.x + 14.0,
            cy,
            18.0,
            if open { col_gold() } else { col_dim() },
        );
        cy += 22.0;
        // A shut door always says why it is shut.
        let text = if open { c.blurb } else { c.unmet };
        for l in wrap_px(&words::retell_naming(text), cell.w - 28.0, 13.0) {
            ui_text(&l, cell.x + 14.0, cy, 13.0, if open { col_ok() } else { col_bad() });
            cy += 16.0;
        }
        if open && is_mouse_button_pressed(MouseButton::Left) && cell.contains(Vec2::new(mx, my)) {
            chosen = Some(c);
        }
    }
    chosen
}

/// The gate, and the four doors behind it.
///
/// Not the event screen, though it looks related on purpose. An event is a
/// question about something that happened to you; a town is five things laid
/// out at once and one of them is walking past, and that wants the whole
/// screen rather than two buttons at the bottom of a paragraph.
///
/// `None` means nothing was clicked. `Some(None)` is walking on.
#[allow(clippy::type_complexity)]
fn render_town(
    run: &Run,
    town: &'static gearmaster_engine::town::Town,
    mx: f32,
    my: f32,
) -> Option<Option<gearmaster_engine::town::Action>> {
    use gearmaster_engine::town::Action;
    let pad = 56.0;
    let h = 620.0;
    let r = Rect::new(pad, (LOGICAL_H - h) / 2.0, LOGICAL_W - 2.0 * pad, h);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 236));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 20, 26, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text(&words::retell(town.name), r.x + 28.0, r.y + 42.0, 24.0, col_gold());
    let sub = words::word("town-one-thing", "ONE OF THEM. NOT TWO.");
    ui_text(&sub, r.x + r.w - 28.0 - text_width(&sub, 13.0), r.y + 42.0, 13.0, col_dim());

    let mut y = r.y + 76.0;
    for para in town.blurb {
        for l in wrap_px(&words::retell_naming(para), r.w - 56.0, 15.0) {
            ui_text(&l, r.x + 28.0, y, 15.0, Color::from_rgba(198, 200, 218, 255));
            y += 20.0;
        }
        y += 10.0;
    }

    let gap = 14.0;
    let cw = (r.w - 56.0 - 3.0 * gap) / 4.0;
    let top = y + 14.0;
    let ch = (r.y + r.h - 108.0) - top;
    let mut picked = None;
    for (i, a) in Action::ALL.into_iter().enumerate() {
        let cell = Rect::new(r.x + 28.0 + i as f32 * (cw + gap), top, cw, ch);
        let hot = cell.contains(Vec2::new(mx, my));
        draw_rectangle(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { Color::from_rgba(46, 42, 30, 255) } else { Color::from_rgba(26, 26, 38, 255) },
        );
        draw_rectangle_lines(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { 2.5 } else { 1.5 },
            if hot { col_gold() } else { Color::from_rgba(64, 64, 88, 255) },
        );
        let mut cy = cell.y + 28.0;
        let name = words::word(a.key(), a.name());
        ui_text(&name, cell.x + 14.0, cy, 17.0, col_gold());
        cy += 24.0;
        for l in wrap_px(&words::retell_naming(a.blurb()), cell.w - 28.0, 13.0) {
            ui_text(&l, cell.x + 14.0, cy, 13.0, Color::from_rgba(186, 190, 206, 255));
            cy += 16.0;
        }
        // What it is worth to *this* run, which is the only number that
        // matters at the gate. A stack count nobody can see is a stack count
        // nobody can plan around.
        let note = town_note(run, a);
        if !note.is_empty() {
            // Two lines at the foot of the card, so a long note wraps rather
            // than running under the one beside it.
            for (i, l) in wrap_px(&note, cell.w - 28.0, 13.0).into_iter().take(2).enumerate() {
                ui_text(&l, cell.x + 14.0, cell.y + ch - 34.0 + i as f32 * 17.0, 13.0, col_ok());
            }
        }
        if hot && left_pressed() {
            picked = Some(Some(a));
        }
    }

    // Walking on. A real offer, not a courtesy: a build one component short of
    // an item wants the money more than it wants a class.
    let on = Rect::new(r.x + r.w / 2.0 - 190.0, r.y + r.h - 76.0, 380.0, 46.0);
    let hot = on.contains(Vec2::new(mx, my));
    draw_rectangle(on.x, on.y, on.w, on.h, if hot { Color::from_rgba(42, 42, 58, 255) } else { Color::from_rgba(26, 26, 38, 255) });
    draw_rectangle_lines(on.x, on.y, on.w, on.h, 1.5, if hot { col_gold() } else { Color::from_rgba(64, 64, 88, 255) });
    let label = format!(
        "WALK ON  -  {} {}",
        run.last_bounty,
        words::word("gold-lower", "gold")
    );
    let lw = text_width(&label, 16.0);
    ui_text(&label, on.x + (on.w - lw) / 2.0, on.y + 29.0, 16.0, if hot { col_gold() } else { LIGHTGRAY });
    if hot && left_pressed() {
        picked = Some(None);
    }
    picked
}

/// What a visit is, said once, in the message strip.
fn town_message(run: &Run, v: &gearmaster_engine::run::TownVisit) -> String {
    use gearmaster_engine::town::Action;
    match v.did {
        Some(Action::Chapel) => match v.became {
            Some(name) => format!(
                "Five said, and something answers. You are {} now.",
                words::class(name)
            ),
            None => format!(
                "You kneel. {} x{} - that much devotion banked before every fight.",
                words::class("Piety"),
                v.stacks
            ),
        },
        Some(Action::Factory) => format!(
            "A shift done. {} gold, and {} x{}: you start {} mana in debt now.",
            v.paid,
            words::class("Tired"),
            v.stacks,
            v.stacks * 3
        ),
        Some(Action::Pub) => {
            "Rumours on the bar. They do not want money - they want what you are carrying."
                .to_string()
        }
        Some(Action::Shop) => {
            format!("Five things you will not see again. {} gold in hand.", run.gold)
        }
        None => String::new(),
    }
}

/// What a door is worth to this run in particular, under its description.
fn town_note(run: &Run, a: gearmaster_engine::town::Action) -> String {
    use gearmaster_engine::run::PIETY_FOR_A_TICKET;
    use gearmaster_engine::town::Action;
    match a {
        Action::Chapel => {
            let held = run.stacks_of("Piety");
            if run.stacks_of("Ticket to Ride") > 0 {
                "You already have the ticket.".to_string()
            } else if held + 1 >= PIETY_FOR_A_TICKET {
                "This one makes five.".to_string()
            } else {
                format!("{} of {} prayers said.", held, PIETY_FOR_A_TICKET)
            }
        }
        Action::Factory => {
            let n = run.stacks_of("Tired");
            words::retell(&format!(
                "{} now. You would start {} mana down.",
                run.last_bounty * 2,
                (n + 1) * 3
            ))
        }
        Action::Pub => {
            let n = run
                .inventory()
                .into_iter()
                .filter(|&id| {
                    let k = run.registry.def(id).kind;
                    gearmaster_engine::rumour::RUMOURS.iter().any(|r| {
                        matches!(r.price, gearmaster_engine::rumour::Barter::Kind(want) if want == k)
                    })
                })
                .count();
            if n == 0 {
                "Nothing loose they would take.".to_string()
            } else {
                format!("{} loose piece{} they would take.", n, if n == 1 { "" } else { "s" })
            }
        }
        Action::Shop => format!("{} in hand.", run.gold),
    }
}

/// The third fountain: it takes a title you already hold and doubles it.
///
/// Deliberately not the same screen as the other two. Those hand over
/// something new and the question is which; this one asks which of the things
/// you already are you want to be twice as much of, which is a different
/// question and should not look like the same one.
fn render_doubling_fountain(
    run: &Run,
    mx: f32,
    my: f32,
) -> Option<&'static gearmaster_engine::class::ClassDef> {
    let offer = run.doubling_offer();
    let pad = 70.0;
    let h = 470.0;
    let r = Rect::new(pad, (LOGICAL_H - h) / 2.0, LOGICAL_W - 2.0 * pad, h);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 236));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(20, 16, 30, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text(
        words::word("deep-fountain", "THE DEEP FOUNTAIN"),
        r.x + 28.0,
        r.y + 42.0,
        24.0,
        col_gold(),
    );
    ui_text(
        words::word(
            "deep-fountain-blurb",
            "Nothing new down here. It only knows how to give you more of what you already are.",
        ),
        r.x + 28.0,
        r.y + 68.0,
        13.0,
        col_dim(),
    );

    let n = offer.len().max(1);
    let gap = 18.0;
    let cw = ((r.w - 56.0 - (n - 1) as f32 * gap) / n as f32).min(460.0);
    let top = r.y + 100.0;
    let ch = r.h - 100.0 - 40.0;
    let mut chosen = None;

    for (i, c) in offer.iter().enumerate() {
        let cell = Rect::new(r.x + 28.0 + i as f32 * (cw + gap), top, cw, ch);
        let hot = cell.contains(Vec2::new(mx, my));
        draw_rectangle(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { Color::from_rgba(46, 42, 30, 255) } else { Color::from_rgba(26, 26, 38, 255) },
        );
        draw_rectangle_lines(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { 2.5 } else { 1.5 },
            if hot { col_gold() } else { Color::from_rgba(64, 64, 88, 255) },
        );
        let mut y = cell.y + 30.0;
        ui_text("TWICE OVER", cell.x + 14.0, y, 11.0, col_dim());
        y += 24.0;
        let title = words::class(c.name);
        let size = fitting_size(title, cell.w - 28.0, &[22.0, 20.0, 18.0, 16.0]);
        ui_text(title, cell.x + 14.0, y, size, col_gold());
        y += 26.0;
        // Both readings, so the trade is on the card rather than in the
        // player's head.
        ui_text("now", cell.x + 14.0, y, 11.0, col_dim());
        y += 15.0;
        for l in wrap_px(&words::retell(&c.power.describe()), cell.w - 28.0, 12.0) {
            ui_text(&l, cell.x + 14.0, y, 12.0, Color::from_rgba(170, 172, 190, 255));
            y += 15.0;
        }
        y += 10.0;
        ui_text("after", cell.x + 14.0, y, 11.0, col_dim());
        y += 15.0;
        if let Some(doubled) = c.power.doubled() {
            for l in wrap_px(&words::retell(&doubled.describe()), cell.w - 28.0, 12.0) {
                ui_text(&l, cell.x + 14.0, y, 12.0, col_ok());
                y += 15.0;
            }
        }

        let take = Rect::new(cell.x + 14.0, cell.y + cell.h - 46.0, cell.w - 28.0, 34.0);
        button(take, words::word("fountain-take", "DRINK"), true, mx, my);
        if is_mouse_button_pressed(MouseButton::Left) && take.contains(Vec2::new(mx, my)) {
            chosen = Some(*c);
        }
    }
    chosen
}

fn render_fountain(run: &Run, mx: f32, my: f32) -> Option<&'static gearmaster_engine::class::ClassDef> {
    let offer = run.fountain_offer();
    // Sized to what is in a card rather than to the viewport: four cards with
    // three quarters of their height empty read as though something failed to
    // load.
    let pad = 70.0;
    let h = 520.0;
    let r = Rect::new(pad, (LOGICAL_H - h) / 2.0, LOGICAL_W - 2.0 * pad, h);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 236));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text(words::word("fountain", "THE FOUNTAIN"), r.x + 28.0, r.y + 42.0, 24.0, col_gold());
    ui_text(
        words::word(
            "fountain-blurb",
            "It has read your gear. Take what it saw, or one of the two you came closest to, \
             or whatever is at the bottom of the water.",
        ),
        r.x + 28.0,
        r.y + 68.0,
        13.0,
        col_dim(),
    );

    let n = offer.len().max(1);
    let gap = 18.0;
    let cw = (r.w - 56.0 - (n - 1) as f32 * gap) / n as f32;
    let top = r.y + 96.0;
    let ch = r.h - 96.0 - 40.0;
    let mut chosen = None;

    for (i, c) in offer.iter().enumerate() {
        let cell = Rect::new(r.x + 28.0 + i as f32 * (cw + gap), top, cw, ch);
        let hot = cell.contains(Vec2::new(mx, my));
        // The last card is the wildcard. It is marked, because being handed
        // something your build was not pointing at should be a decision rather
        // than a surprise.
        let wild = i + 1 == offer.len() && offer.len() > 1;
        draw_rectangle(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { Color::from_rgba(46, 42, 30, 255) } else { Color::from_rgba(26, 26, 38, 255) },
        );
        draw_rectangle_lines(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { 2.5 } else { 1.5 },
            if hot { col_gold() } else { Color::from_rgba(64, 64, 88, 255) },
        );

        let mut y = cell.y + 30.0;
        let tag = match (i, wild) {
            (_, true) => "OUT OF THE WATER",
            (0, _) => "WHAT IT SAW IN YOU",
            _ => "WHAT YOU CAME NEAR",
        };
        ui_text(tag, cell.x + 14.0, y, 11.0, col_dim());
        y += 24.0;
        let title = words::class(c.name);
        let size = fitting_size(title, cell.w - 28.0, &[22.0, 20.0, 18.0, 16.0]);
        ui_text(title, cell.x + 14.0, y, size, col_gold());
        y += 24.0;
        for l in wrap_px(&words::retell(c.blurb), cell.w - 28.0, 13.0) {
            ui_text(&l, cell.x + 14.0, y, 13.0, Color::from_rgba(198, 200, 218, 255));
            y += 16.0;
        }
        y += 10.0;
        // What it asks for, and whether you have it. A wildcard is offered
        // whether or not you qualify, so this is the only place that says so.
        let fp = run.fingerprint();
        for &(axis, need) in c.requires {
            let have = fp.get(axis);
            ui_text(
                &format!("{} {}/{}", words::retell(&axis.name()), have, need),
                cell.x + 14.0,
                y,
                12.0,
                if have >= need { col_ok() } else { col_bad() },
            );
            y += 15.0;
        }
        if c.requires.is_empty() {
            let how = gearmaster_engine::class::how_you_get_it(c.name)
                .unwrap_or("asks for nothing");
            ui_text(&words::retell(how), cell.x + 14.0, y, 12.0, col_dim());
            y += 15.0;
        }
        y += 10.0;
        for l in wrap_px(&words::retell(&c.power.describe()), cell.w - 28.0, 12.0) {
            ui_text(&l, cell.x + 14.0, y, 12.0, col_ok());
            y += 15.0;
        }

        let take = Rect::new(cell.x + 14.0, cell.y + cell.h - 46.0, cell.w - 28.0, 34.0);
        button(take, words::word("fountain-take", "DRINK"), true, mx, my);
        if is_mouse_button_pressed(MouseButton::Left) && take.contains(Vec2::new(mx, my)) {
            chosen = Some(*c);
        }
    }
    chosen
}

/// The ladder, as a list you can click. Opened from the glossary's one entry
/// that is also a control.
///
/// Returns the rung chosen, and whether the picker should close.
fn render_ladder_picker(run: &Run, page: usize, mx: f32, my: f32) -> (Option<usize>, bool, usize) {
    let pad = 56.0;
    let r = Rect::new(pad, pad, LOGICAL_W - 2.0 * pad, LOGICAL_H - 2.0 * pad);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 232));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col_gold());
    ui_text("THE WORN PATH", r.x + 24.0, r.y + 38.0, 20.0, col_gold());
    ui_text(
        "Choose where to pick the road up. Every rung on the way pays its bounty, \
         and none of them can be walked back down.",
        r.x + 24.0,
        r.y + 62.0,
        13.0,
        col_dim(),
    );

    let close = Rect::new(r.x + r.w - 140.0, r.y + 16.0, 120.0, 34.0);
    button(close, "CLOSE", true, mx, my);

    // Only what is ahead. Behind you is not a choice, it is a memory.
    let ahead: Vec<usize> = (run.rung + 1..LADDER.len()).collect();
    let cols = 4usize;
    let gap = 16.0;
    let cw = (r.w - 48.0 - (cols - 1) as f32 * gap) / cols as f32;
    let rh = 46.0;
    let top = r.y + 92.0;
    let rows = (((r.y + r.h - 60.0) - top) / (rh + 8.0)) as usize;
    let per_page = rows * cols;
    let pages = ahead.len().div_ceil(per_page.max(1)).max(1);
    let page = page.min(pages - 1);

    let mut chosen = None;
    for (i, &rung) in ahead.iter().skip(page * per_page).take(per_page).enumerate() {
        let (cx, cy) = (i % cols, i / cols);
        let cell = Rect::new(
            r.x + 24.0 + cx as f32 * (cw + gap),
            top + cy as f32 * (rh + 8.0),
            cw,
            rh,
        );
        let m = &LADDER[rung];
        let hot = cell.contains(Vec2::new(mx, my));
        // Everything up to and including this rung is paid on arrival, so the
        // number shown is what taking it is actually worth.
        let purse: i32 = (run.rung..rung).map(|i| LADDER[i].bounty).sum();
        draw_rectangle(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            if hot { Color::from_rgba(52, 46, 30, 255) } else { Color::from_rgba(28, 28, 40, 255) },
        );
        draw_rectangle_lines(
            cell.x,
            cell.y,
            cell.w,
            cell.h,
            1.5,
            if hot { col_gold() } else { Color::from_rgba(64, 64, 88, 255) },
        );
        let label = format!("{}. {}", rung + 1, words::monster(m.name));
        let size = fitting_size(&label, cell.w - 16.0, &[14.0, 13.0, 12.0, 11.0]);
        draw_capped(&label, cell.x + 8.0, cell.y + 19.0, cell.w - 16.0, size, WHITE, 1);
        ui_text(
            &format!("{} hp  ·  {}g on the way", m.health, purse),
            cell.x + 8.0,
            cell.y + 36.0,
            11.0,
            col_dim(),
        );
        if hot && is_mouse_button_pressed(MouseButton::Left) {
            chosen = Some(rung);
        }
    }

    let next = Rect::new(r.x + r.w - 260.0, r.y + r.h - 46.0, 240.0, 34.0);
    if pages > 1 {
        ui_text(
            &format!("page {} of {}", page + 1, pages),
            r.x + 24.0,
            r.y + r.h - 22.0,
            14.0,
            col_dim(),
        );
        button(next, if page + 1 < pages { "FURTHER UP" } else { "BACK TO THE START" }, true, mx, my);
        if is_mouse_button_pressed(MouseButton::Left) && next.contains(Vec2::new(mx, my)) {
            return (None, false, (page + 1) % pages);
        }
    }
    let shut = is_mouse_button_pressed(MouseButton::Left) && close.contains(Vec2::new(mx, my));
    (chosen, shut, page)
}

/// The glossary entry that is also a button.
///
/// Skipping a rung is here rather than on a menu because it is not meant to be
/// the first thing a new player finds - but the early rungs get walked many
/// times over, once to learn them and once for every later idea that starts
/// from the bottom, and the numbers further up are far easier to test when
/// getting there is not itself the work.
const SKIP_TERM: &str = "THE WORN PATH";

/// The key to a tile: which motif means which slot, which corner mark means
/// what, and what the lightness of a tile is telling you. Returns its height
/// so the glossary below can start clear of it.
///
/// Without this the motifs are only learnable by association, which is fine
/// on the loadout boards - each is under a labelled column - but not on a
/// shop card, where a shape appears with no slot named anywhere near it.
/// The legend's height, drawing it only when asked.
///
/// The page count needs to know how tall it is even on pages that do not show
/// it, so measuring and drawing have to come apart.
fn draw_tile_legend_maybe(r: Rect, draw: bool) -> f32 {
    if draw {
        draw_tile_legend(r.x + 24.0, r.y + 86.0, r.w - 48.0)
    } else {
        // Same arithmetic, off-screen, so the number matches what page one
        // would actually reserve.
        draw_tile_legend(r.x + 24.0, -10_000.0, r.w - 48.0)
    }
}

fn draw_tile_legend(x: f32, y: f32, w: f32) -> f32 {
    let sample = 30.0;
    let row_h = 44.0;

    ui_text("READING A TILE", x, y, 14.0, col_gold());
    let mut ty = y + 26.0;

    // Slot motifs, laid out across the width.
    let per = SlotKind::ALL.len() as f32;
    let step = (w / per).min(240.0);
    for (i, &slot) in SlotKind::ALL.iter().enumerate() {
        let sx = x + i as f32 * step;
        let fill = slot_color(slot, 0.45);
        draw_rectangle(sx, ty, sample, sample, fill);
        draw_motif(sx, ty, sample, slot_motif(slot), motif_ink(fill, 1.0));
        draw_rectangle_lines(sx, ty, sample, sample, 1.0, Color::from_rgba(0, 0, 0, 110));
        ui_text(slot.name(), sx + sample + 8.0, ty + 20.0, 13.0, LIGHTGRAY);
    }
    ty += row_h;

    // The shared mark, which is the absence of a slot rather than one of them.
    let grey = unplaced_color(PieceKind::Plating);
    draw_rectangle(x, ty, sample, sample, grey);
    draw_motif(x, ty, sample, Motif::Shared, motif_ink(grey, 1.0));
    draw_rectangle_lines(x, ty, sample, sample, 1.0, Color::from_rgba(0, 0, 0, 110));
    ui_text(
        "grey, no slot mark: fits more than one grid. It takes the colour and mark of",
        x + sample + 8.0,
        ty + 15.0,
        12.0,
        LIGHTGRAY,
    );
    ui_text(
        "whichever grid you drop it into. Materials go in gloves or greaves, plating in helmets or greaves.",
        x + sample + 8.0,
        ty + 30.0,
        12.0,
        col_dim(),
    );
    ty += row_h;

    // Corner marks, then what lightness means.
    let marks = [Marker::Bonus, Marker::Effect, Marker::Trigger];
    for (i, &m) in marks.iter().enumerate() {
        let sx = x + i as f32 * step;
        draw_marker_sized(sx + 11.0, ty + 9.0, 9.0, m, true);
        ui_text(m.label(), sx + 28.0, ty + 15.0, 13.0, LIGHTGRAY);
    }
    // The remaining width explains the lightness ramp in place: darker tiles
    // are the piece a recipe is built around, lighter ones are its trim.
    let ramp_x = x + 3.0 * step;
    ui_text("lighter = further out from the core:", ramp_x, ty + 15.0, 12.0, col_dim());
    let bar_x = ramp_x + text_width("lighter = further out from the core:", 12.0) + 12.0;
    for (i, l) in [0.22f32, 0.45, 0.72].iter().enumerate() {
        let sx = bar_x + i as f32 * 26.0;
        let fill = slot_color(SlotKind::Chest, *l);
        draw_rectangle(sx, ty - 2.0, 24.0, 22.0, fill);
        draw_rectangle_lines(sx, ty - 2.0, 24.0, 22.0, 1.0, Color::from_rgba(0, 0, 0, 110));
    }

    ty += 30.0;
    draw_line(x, ty, x + w, ty, 1.0, Color::from_rgba(70, 70, 95, 255));
    ty - y + 10.0
}

/// A seed taken from the wall clock, so a fresh run stocks a different shop
/// every time. `miniquad::date::now` is used rather than `SystemTime` because
/// this has to work in the browser too, where `SystemTime::now` panics.
fn clock_seed() -> u64 {
    let t = macroquad::miniquad::date::now();
    // Milliseconds since the epoch, then stirred so nearby starts diverge.
    let ms = (t * 1000.0) as u64;
    let mut h = ms ^ 0x9E37_79B9_7F4A_7C15;
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

/// What the window is showing before the run proper starts.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Opening {
    /// Reading page N of the introduction.
    Intro(usize),
    /// Picking Grinder or Rogue, and which words to play in.
    ModeSelect,
    /// Reading who you are before the board appears.
    Story,
    /// Done; the board is live.
    Playing,
}

/// The pages a new run opens on: what the game is, then which way to play it.
///
/// Deliberately short. The glossary behind `G` is the reference; these are the
/// four things you cannot work out by looking at the board.
const INTRO: &[(&str, &[&str])] = &[
    (
        "GEAR IS BUILT, NOT BOUGHT",
        &[
            "Five slots - helmet, chestpiece, gloves, greaves, weapon. Each one",
            "is a grid you fill yourself.",
            "",
            "The shop sells parts, never whole weapons. Drag them in and make",
            "them touch. Parts that touch, and that add up to a real recipe,",
            "become an item. Only items fight.",
            "",
            "A part sitting on its own still gives you its stats. It just never",
            "does anything.",
        ],
    ),
    (
        "EVERY ITEM KEEPS ITS OWN TIME",
        &[
            "No turns. Every item runs its own clock and goes off when it comes",
            "round. A light blade swings several times before a heavy plate",
            "shifts once.",
            "",
            "When it goes off it does everything it carries at once - damage,",
            "armour, mana, curses. Armour is temporary and starts every fight at",
            "nothing, so gear has to build it up.",
            "",
            "Anything hanging off that moment is a trigger. Some cost mana, and",
            "do something worse when you cannot pay.",
        ],
    ),
    (
        "WHERE YOU PUT THINGS MATTERS",
        &[
            "Two items touching in one grid are neighbours. Two items in",
            "different grids sitting on the same rows are lined up.",
            "",
            "Triggers read both. An item can fire off its neighbour's clock, or",
            "off a glove three grids away that happens to share its rows.",
            "",
            "So the same parts are worth more or less depending on where you set",
            "them down. That is the game.",
        ],
    ),
    (
        "WINNING, LOSING, AND MARKS",
        &[
            "Beat a monster, climb a rung, take the gold. Lose and you still get",
            "the gold - you will need it - but the thing is still standing.",
            "",
            "Items are scored on how much they actually do per second. Good ones",
            "are marked rare, then epic, then legendary.",
            "",
            "Press G whenever you want to know what a word means.",
        ],
    ),
];

/// One intro page. Returns the BACK and NEXT rects for hit-testing.
fn render_intro(page: usize, mx: f32, my: f32) -> (Rect, Rect) {
    let (title, body) = INTRO[page.min(INTRO.len() - 1)];
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, col_bg());

    let pad = 90.0;
    let r = Rect::new(pad, 70.0, LOGICAL_W - 2.0 * pad, LOGICAL_H - 210.0);
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 255));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(110, 110, 145, 255));

    ui_text("GEAR MASTER", r.x + 34.0, r.y + 46.0, 26.0, col_gold());
    ui_text(title, r.x + 34.0, r.y + 92.0, 22.0, WHITE);

    let lh = line_h(16.0);
    for (i, line) in body.iter().enumerate() {
        ui_text(line, r.x + 34.0, r.y + 136.0 + i as f32 * lh, 16.0, LIGHTGRAY);
    }

    // Which page you are on.
    for i in 0..INTRO.len() {
        let cx = r.x + r.w / 2.0 - (INTRO.len() as f32 - 1.0) * 11.0 + i as f32 * 22.0;
        let cy = r.y + r.h - 30.0;
        if i == page {
            draw_circle(cx, cy, 6.0, col_gold());
        } else {
            draw_circle_lines(cx, cy, 6.0, 1.5, col_dim());
        }
    }

    let bw = 220.0;
    let by = r.y + r.h + 26.0;
    let back = Rect::new(r.x, by, bw, 44.0);
    let next = Rect::new(r.x + r.w - bw, by, bw, 44.0);
    if page > 0 {
        button(back, "BACK", true, mx, my);
    }
    button(next, if page + 1 == INTRO.len() { "CHOOSE A MODE" } else { "NEXT" }, true, mx, my);
    (back, next)
}

/// The mode and difficulty picker, shown once the intro is done. Returns the
/// rects for both rows so the caller can hit-test them.
/// A scene between fights: what just happened, and what it means. Drawn over
/// everything, dismissed by a button.
///
/// Returns that button.
fn render_scene(scene: &[&str], mx: f32, my: f32) -> Rect {
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 246));
    let w = 940.0;
    let x = (LOGICAL_W - w) / 2.0;

    // Vertically centred on what it actually holds, so a short scene does not
    // sit in the top corner of an empty screen.
    let mut height = 0.0;
    for para in scene {
        height += wrap_px(para, w, 17.0).len() as f32 * (line_h(17.0) + 2.0) + 22.0;
    }
    let mut y = ((LOGICAL_H - height) / 2.0).max(90.0);

    for para in scene {
        for l in wrap_px(para, w, 17.0) {
            ui_text(&l, x, y, 17.0, Color::from_rgba(214, 216, 232, 255));
            y += line_h(17.0) + 2.0;
        }
        y += 22.0;
    }

    let go = Rect::new((LOGICAL_W - 360.0) / 2.0, LOGICAL_H - 120.0, 360.0, 52.0);
    let hot = go.contains(Vec2::new(mx, my));
    draw_rectangle(
        go.x,
        go.y,
        go.w,
        go.h,
        if hot { Color::from_rgba(52, 46, 30, 255) } else { Color::from_rgba(28, 28, 40, 255) },
    );
    draw_rectangle_lines(go.x, go.y, go.w, go.h, if hot { 3.0 } else { 2.0 }, col_gold());
    centered_text("GO ON", go.x + go.w / 2.0, go.y + 34.0, 22.0, WHITE);
    go
}

/// Who you are, before the board appears. One page, then the game.
///
/// Every theme owes the player this: the boards do not explain themselves, and
/// "why am I doing this" is not something you can work out from a grid.
/// Returns the button that leaves it.
fn render_story(theme: &'static gearmaster_engine::theme::Theme, mx: f32, my: f32) -> Rect {
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, col_bg());
    let w = 980.0;
    let x = (LOGICAL_W - w) / 2.0;
    centered_text(theme.label, LOGICAL_W / 2.0, 128.0, 30.0, col_gold());

    // Sized to fit above the button rather than at a fixed size: a theme is
    // free to write a longer opening than the plain one, and the first draft
    // of the turtle story ran straight through BEGIN.
    let go = Rect::new((LOGICAL_W - 420.0) / 2.0, LOGICAL_H - 130.0, 420.0, 56.0);
    let top = 200.0;
    let room = go.y - 24.0 - top;
    let (size, gap) = [(17.0f32, 20.0f32), (16.0, 16.0), (15.0, 13.0), (14.0, 10.0), (13.0, 8.0)]
        .into_iter()
        .find(|&(size, gap)| {
            let head = 24.0f32.min(size + 7.0);
            let mut h = wrap_px(theme.story[0], w, head).len() as f32 * (line_h(head) + 2.0) + gap;
            for para in &theme.story[1..] {
                h += wrap_px(para, w, size).len() as f32 * (line_h(size) + 2.0) + gap;
            }
            h <= room
        })
        .unwrap_or((13.0, 8.0));

    let mut y = top;
    for (i, para) in theme.story.iter().enumerate() {
        // The first line is the premise and gets to be a headline; the rest is
        // the explanation.
        let size = if i == 0 { 24.0f32.min(size + 7.0) } else { size };
        let colour = if i == 0 { WHITE } else { Color::from_rgba(206, 208, 226, 255) };
        for l in wrap_px(para, w, size) {
            ui_text(&l, x, y, size, colour);
            y += line_h(size) + 2.0;
        }
        y += gap;
    }

    let hot = go.contains(Vec2::new(mx, my));
    draw_rectangle(
        go.x,
        go.y,
        go.w,
        go.h,
        if hot { Color::from_rgba(52, 46, 30, 255) } else { Color::from_rgba(28, 28, 40, 255) },
    );
    draw_rectangle_lines(go.x, go.y, go.w, go.h, if hot { 3.0 } else { 2.0 }, col_gold());
    centered_text("BEGIN", go.x + go.w / 2.0, go.y + 37.0, 24.0, WHITE);
    go
}

#[allow(clippy::type_complexity)]
fn render_mode_select(
    chosen: Difficulty,
    theme: &'static gearmaster_engine::theme::Theme,
    mx: f32,
    my: f32,
) -> ([(Mode, Rect); 2], Vec<(Difficulty, Rect)>, Vec<(&'static gearmaster_engine::theme::Theme, Rect)>) {
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, col_bg());
    centered_text("HOW DO YOU WANT TO PLAY?", LOGICAL_W / 2.0, 84.0, 28.0, col_gold());
    centered_text(
        "Losing pays either way. It just does not get you past the thing that beat you.",
        LOGICAL_W / 2.0,
        122.0,
        15.0,
        col_dim(),
    );

    // ---- which words ----
    //
    // Up here rather than at the bottom: it is a small choice and it needs
    // little room, and the two big grids below it were already using every
    // pixel they had.
    let ty = 152.0;
    centered_text("IN WHOSE WORDS?", LOGICAL_W / 2.0, ty, 16.0, col_gold());
    let tw = 430.0;
    let tgap = 30.0;
    let themes = gearmaster_engine::theme::THEMES;
    let n = themes.len() as f32;
    let tx0 = (LOGICAL_W - (n * tw + (n - 1.0) * tgap)) / 2.0;
    let mut theme_picks = Vec::new();
    for (i, &t) in themes.iter().enumerate() {
        let rect = Rect::new(tx0 + i as f32 * (tw + tgap), ty + 12.0, tw, 88.0);
        let hot = rect.contains(Vec2::new(mx, my));
        let picked = std::ptr::eq(t, theme);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if picked {
                Color::from_rgba(44, 40, 26, 255)
            } else if hot {
                Color::from_rgba(34, 34, 50, 255)
            } else {
                Color::from_rgba(22, 22, 34, 255)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if picked || hot { 3.0 } else { 1.5 },
            if picked {
                col_gold()
            } else if hot {
                LIGHTGRAY
            } else {
                Color::from_rgba(80, 80, 105, 255)
            },
        );
        let size = fitting_size(t.label, rect.w - 24.0, &[18.0, 16.0, 14.0]);
        centered_text(t.label, rect.x + rect.w / 2.0, rect.y + 26.0, size, WHITE);
        let mut by = rect.y + 48.0;
        for l in wrap_px(t.blurb, rect.w - 28.0, 12.0).into_iter().take(2) {
            centered_text(&l, rect.x + rect.w / 2.0, by, 12.0, col_dim());
            by += 14.0;
        }
        theme_picks.push((t, rect));
    }

    let (cw, ch) = (500.0, 300.0);
    let gap = 56.0;
    let x0 = (LOGICAL_W - (2.0 * cw + gap)) / 2.0;
    let y0 = 272.0;
    let modes = [Mode::Grinder, Mode::Rogue];
    let mut out = [(Mode::Grinder, Rect::new(0.0, 0.0, 0.0, 0.0)); 2];

    for (i, &mode) in modes.iter().enumerate() {
        let rect = Rect::new(x0 + i as f32 * (cw + gap), y0, cw, ch);
        let hot = rect.contains(Vec2::new(mx, my));
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if hot { Color::from_rgba(34, 34, 50, 255) } else { Color::from_rgba(22, 22, 34, 255) },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if hot { 3.0 } else { 1.5 },
            if hot { col_gold() } else { Color::from_rgba(80, 80, 105, 255) },
        );
        centered_text(mode.name(), rect.x + rect.w / 2.0, rect.y + 52.0, 26.0, WHITE);

        let mut y = rect.y + 100.0;
        for line in wrap_px(mode.blurb(), rect.w - 36.0, 15.0) {
            centered_text(&line, rect.x + rect.w / 2.0, y, 15.0, LIGHTGRAY);
            y += line_h(15.0);
        }

        // Flowed after the blurb rather than pinned to the bottom edge: the
        // Rogue blurb is a line longer than the Grinder one, and a fixed
        // offset put its life pips through the middle of its own text.
        y += 16.0;
        match mode {
            Mode::Grinder => {
                for step in 0..5 {
                    let sx = rect.x + rect.w / 2.0 - 100.0 + step as f32 * 50.0;
                    draw_rectangle(
                        sx,
                        y,
                        36.0,
                        20.0,
                        if step <= 2 { col_you() } else { Color::from_rgba(48, 48, 64, 255) },
                    );
                }
                centered_text(
                    "lose, and you slide back one",
                    rect.x + rect.w / 2.0,
                    y + 50.0,
                    14.0,
                    col_you(),
                );
            }
            Mode::Rogue => {
                for life in 0..ROGUE_LIVES {
                    let sx = rect.x + rect.w / 2.0 - 60.0 + life as f32 * 60.0;
                    if life < ROGUE_LIVES - 1 {
                        draw_circle(sx, y + 10.0, 13.0, col_foe());
                    } else {
                        draw_circle_lines(sx, y + 10.0, 13.0, 2.0, col_dim());
                    }
                }
                centered_text(
                    "three lives, then you start over",
                    rect.x + rect.w / 2.0,
                    y + 50.0,
                    14.0,
                    col_foe(),
                );
            }
        }
        out[i] = (mode, rect);
    }

    // ---- difficulty ----
    let dy = y0 + ch + 46.0;
    centered_text("HOW HARD?", LOGICAL_W / 2.0, dy, 22.0, col_gold());
    centered_text(
        "Bigger numbers mean tougher, meaner monsters. Medium is the fight the game was \
         built around.",
        LOGICAL_W / 2.0,
        dy + 30.0,
        14.0,
        col_dim(),
    );

    let n = Difficulty::ALL.len() as f32;
    let dw = 250.0;
    let dgap = 18.0;
    let dx0 = (LOGICAL_W - (n * dw + (n - 1.0) * dgap)) / 2.0;
    let mut picks = Vec::new();
    for (i, &d) in Difficulty::ALL.iter().enumerate() {
        let rect = Rect::new(dx0 + i as f32 * (dw + dgap), dy + 46.0, dw, 230.0);
        let hot = rect.contains(Vec2::new(mx, my));
        let picked = d == chosen;
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if picked {
                Color::from_rgba(44, 40, 26, 255)
            } else if hot {
                Color::from_rgba(34, 34, 50, 255)
            } else {
                Color::from_rgba(22, 22, 34, 255)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if picked || hot { 3.0 } else { 1.5 },
            if picked { col_gold() } else if hot { LIGHTGRAY } else { Color::from_rgba(80, 80, 105, 255) },
        );
        centered_text(d.name(), rect.x + rect.w / 2.0, rect.y + 34.0, 19.0, WHITE);
        centered_text(
            &d.label(),
            rect.x + rect.w / 2.0,
            rect.y + 66.0,
            24.0,
            if picked { col_gold() } else { col_dim() },
        );
        // What that actually buys the monster.
        let each = d.each_way();
        centered_text(
            &format!("{:.1}x tougher, {:.1}x deadlier", each, each),
            rect.x + rect.w / 2.0,
            rect.y + 92.0,
            12.0,
            col_dim(),
        );
        if d.passives().is_empty() {
            centered_text("nothing extra", rect.x + rect.w / 2.0, rect.y + 116.0, 12.0, col_dim());
        } else {
            let mut py = rect.y + 112.0;
            for p in d.passives() {
                centered_text(p.name(), rect.x + rect.w / 2.0, py, 12.0, col_foe());
                py += 14.0;
                for l in wrap_px(p.describe(), rect.w - 24.0, 10.0) {
                    centered_text(&l, rect.x + rect.w / 2.0, py, 10.0, col_dim());
                    py += 12.0;
                }
                py += 3.0;
            }
        }
        if d.is_default() {
            // Inside the card: above it, this ran into the neighbouring
            // headings.
            centered_text(
                "the intended fight",
                rect.x + rect.w / 2.0,
                rect.y + rect.h - 10.0,
                12.0,
                col_gold(),
            );
        }
        picks.push((d, rect));
    }

    centered_text(
        "pick a difficulty and a set of words, then a mode to start",
        LOGICAL_W / 2.0,
        dy + 300.0,
        15.0,
        LIGHTGRAY,
    );
    (out, picks, theme_picks)
}


/// The glossary, over the top of whatever is behind it. Returns the CLOSE
/// button and the NEXT PAGE button so the caller can hit-test them.
///
/// Paged rather than squeezed: the word list has outgrown one screenful, and
/// shrinking the text until it all fits would undo the point of making it
/// legible in the first place.
/// Returns the close button, the next-page button, the page count, and the
/// region of the one entry that is also a control - see `SKIP_TERM`.
fn render_glossary(tab: usize, page: usize, mx: f32, my: f32) -> GlossaryHit {
    let pad = 56.0;
    let r = Rect::new(pad, pad, LOGICAL_W - 2.0 * pad, LOGICAL_H - 2.0 * pad);
    draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(6, 6, 10, 228));
    draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(18, 18, 28, 252));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::from_rgba(120, 120, 155, 255));
    ui_text(words::word("glossary", "WHAT THE WORDS MEAN"), r.x + 24.0, r.y + 38.0, 20.0, col_gold());
    ui_text("G or Esc to close", r.x + 24.0, r.y + 62.0, 12.0, col_dim());

    let close = Rect::new(r.x + r.w - 140.0, r.y + 16.0, 120.0, 34.0);
    button(close, "CLOSE", true, mx, my);

    // Three shelves: the words, the classes, and the axes a fountain scores
    // a build on. Each of the last two wants a paragraph per entry, which
    // does not fit the four-column layout the plain definitions use.
    let tw = 150.0;
    let tabs = [
        Rect::new(r.x + 320.0, r.y + 20.0, tw, 30.0),
        Rect::new(r.x + 320.0 + tw + 10.0, r.y + 20.0, tw, 30.0),
        Rect::new(r.x + 320.0 + (tw + 10.0) * 2.0, r.y + 20.0, tw, 30.0),
    ];
    let tab_names = [
        words::word("classes", "CLASSES"),
        words::word("axes", "WHAT DECIDES"),
    ];
    for (i, (rect, name)) in
        tabs.iter().zip(["WORDS", tab_names[0], tab_names[1]]).enumerate()
    {
        let on = i == tab;
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if on { Color::from_rgba(52, 46, 30, 255) } else { Color::from_rgba(28, 28, 40, 255) },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.5,
            if on || rect.contains(Vec2::new(mx, my)) {
                col_gold()
            } else {
                Color::from_rgba(70, 70, 95, 255)
            },
        );
        centered_text(
            name,
            rect.x + rect.w / 2.0,
            rect.y + 20.0,
            14.0,
            if on { col_gold() } else { col_dim() },
        );
    }
    if tab == 1 || tab == 2 {
        let pages = if tab == 1 {
            render_class_pages(r, page, mx, my)
        } else {
            render_axis_pages(r, page)
        };
        let next = Rect::new(r.x + r.w - 260.0, r.y + r.h - 46.0, 240.0, 34.0);
        if pages > 1 {
            ui_text(
                &format!("page {} of {}", page + 1, pages),
                r.x + 24.0,
                r.y + r.h - 22.0,
                14.0,
                col_dim(),
            );
            button(next, if page + 1 < pages { "NEXT PAGE" } else { "BACK TO START" }, true, mx, my);
        }
        return GlossaryHit { close, next, pages, skip: None, tabs };
    }

    // The tile legend only belongs on the first page.
    // The tile legend only belongs on the first page - but it takes vertical
    // room, and the page *count* has to know that. Measuring it on whichever
    // page happens to be open and applying that height to every page is what
    // made the footer read "page 7 of 6": page one counted as though every
    // page were short, page seven as though none were.
    let legend_h = draw_tile_legend_maybe(r, page == 0);
    let top_of = |p: usize| r.y + 96.0 + if p == 0 { legend_h } else { 0.0 };
    let bottom = r.y + r.h - 54.0;
    let gap = 24.0;
    let cols = 4usize;
    let size = 14.0f32;
    let col_w = (r.w - 48.0 - (cols - 1) as f32 * gap) / cols as f32;
    let lh = line_h(size);

    // Walk the whole list, laying it out page by page, and only draw the one
    // asked for. Cheap enough at this size, and it means the page count and
    // the layout can never disagree.
    // The glossary in this theme's words: entries it replaces are swapped,
    // entries it adds come last, and everything else has its definition
    // re-told. Built once here rather than at each of the three places below
    // that walk it.
    let entries: Vec<(&str, String)> = GLOSSARY
        .iter()
        .map(|(term, meaning)| match words::current().glossary_entry(term) {
            Some((t, d)) => (t, d.to_string()),
            None => (*term, words::retell(meaning)),
        })
        .chain(words::current().extra_glossary().map(|(t, d)| (t, d.to_string())))
        .collect();

    let mut at = 0usize;
    let mut this_page = 0usize;
    let mut col = 0usize;
    let mut y = top_of(0);
    let mut pages = 1usize;
    let mut start_of_page = 0usize;
    let mut skip_hot: Option<Rect> = None;
    while at < entries.len() {
        let (term, meaning) = (&entries[at].0, &entries[at].1);
        let lines = wrap_px(meaning, col_w - 16.0, size);
        let needed = lh * (1.0 + lines.len() as f32) + 10.0;
        if y + needed > bottom {
            if col + 1 < cols {
                col += 1;
                y = top_of(this_page);
            } else {
                // Page full.
                if this_page == page {
                    break;
                }
                this_page += 1;
                pages += 1;
                col = 0;
                y = top_of(this_page);
                start_of_page = at;
                continue;
            }
        }
        if this_page == page {
            let x = r.x + 24.0 + col as f32 * (col_w + gap);
            // One entry is a control as well as a definition. It is not marked
            // as one - finding it is the point - but it does light up under
            // the cursor, so nobody has to click every word to be sure.
            let is_skip = *term == SKIP_TERM;
            let hot = is_skip
                && Rect::new(x, y - lh, col_w, lh * (1.0 + lines.len() as f32))
                    .contains(Vec2::new(mx, my));
            let head = if is_skip && hot {
                col_gold()
            } else {
                Color::from_rgba(150, 200, 240, 255)
            };
            let body = if is_skip && hot {
                Color::from_rgba(240, 226, 170, 255)
            } else {
                Color::from_rgba(198, 200, 218, 255)
            };
            let block_top = y - lh;
            ui_text(term, x, y, size, head);
            y += lh;
            for l in lines {
                ui_text(&l, x + 14.0, y, size, body);
                y += lh;
            }
            y += 10.0;
            if is_skip {
                skip_hot = Some(Rect::new(x, block_top, col_w, y - block_top));
            }
        } else {
            y += needed;
        }
        at += 1;
    }
    // Finish counting pages even after the drawn one ends.
    if at < entries.len() {
        let mut c = col;
        let mut yy = y;
        let mut p = this_page;
        for i in at..entries.len() {
            let needed =
                lh * (1.0 + wrap_px(&entries[i].1, col_w - 16.0, size).len() as f32) + 10.0;
            if yy + needed > bottom {
                if c + 1 < cols {
                    c += 1;
                    yy = top_of(p);
                } else {
                    pages += 1;
                    p += 1;
                    c = 0;
                    yy = top_of(p);
                }
            }
            yy += needed;
        }
    }
    let _ = start_of_page;

    let next = Rect::new(r.x + r.w - 260.0, r.y + r.h - 46.0, 240.0, 34.0);
    if pages > 1 {
        ui_text(
            &format!("page {} of {}", page + 1, pages),
            r.x + 24.0,
            r.y + r.h - 22.0,
            14.0,
            col_dim(),
        );
        button(next, if page + 1 < pages { "NEXT PAGE" } else { "BACK TO START" }, true, mx, my);
    }
    GlossaryHit { close, next, pages, skip: skip_hot, tabs }
}

/// What the glossary put on screen, so the frame can decide what a click did.
struct GlossaryHit {
    close: Rect,
    next: Rect,
    pages: usize,
    /// The one entry that is also a control - see `SKIP_TERM`.
    skip: Option<Rect>,
    tabs: [Rect; 3],
}

/// Every class, in full: what it asks for, how close you are to it, and what
/// it does for you. Returns the page count.
/// The axes a fountain scores you on, one paragraph each.
///
/// The fountain reads a build and says "Geomancer needs weave 0/70", which is
/// only useful to someone who already knows what weave is. This is where they
/// find out.
fn render_axis_pages(r: Rect, page: usize) -> usize {
    use gearmaster_engine::class::Axis;
    let entries = Axis::glossary();
    let top = r.y + 96.0;
    let bottom = r.y + r.h - 54.0;
    let gap = 26.0;
    let cols = 3usize;
    let col_w = (r.w - 48.0 - (cols - 1) as f32 * gap) / cols as f32;

    let block_h = |text: &str| -> f32 {
        22.0 + wrap_px(&words::retell(text), col_w - 12.0, 12.0).len() as f32 * 15.0 + 14.0
    };

    let mut at = 0usize;
    let mut this_page = 0usize;
    let mut col = 0usize;
    let mut y = top;
    let mut pages = 1usize;
    while at < entries.len() {
        let (name, text) = &entries[at];
        let needed = block_h(text);
        if y + needed > bottom {
            if col + 1 < cols {
                col += 1;
                y = top;
            } else {
                if this_page == page {
                    break;
                }
                this_page += 1;
                pages += 1;
                col = 0;
                y = top;
                continue;
            }
        }
        if this_page == page {
            let x = r.x + 24.0 + col as f32 * (col_w + gap);
            ui_text(&words::retell(name).to_uppercase(), x, y, 15.0, col_gold());
            y += 20.0;
            for l in wrap_px(&words::retell(text), col_w - 12.0, 12.0) {
                ui_text(&l, x + 8.0, y, 12.0, Color::from_rgba(198, 200, 218, 255));
                y += 15.0;
            }
            y += 14.0;
        } else {
            y += needed;
        }
        at += 1;
    }
    // Finish counting pages past the drawn one, the same way the other tabs
    // do - a count taken from the open page's geometry is what once made the
    // footer read "page 7 of 6".
    if at < entries.len() {
        let (mut c2, mut yy) = (col, y);
        for (_, text) in &entries[at..] {
            let needed = block_h(text);
            if yy + needed > bottom {
                if c2 + 1 < cols {
                    c2 += 1;
                    yy = top;
                } else {
                    pages += 1;
                    c2 = 0;
                    yy = top;
                }
            }
            yy += needed;
        }
    }
    pages
}

fn render_class_pages(r: Rect, page: usize, _mx: f32, _my: f32) -> usize {
    use gearmaster_engine::class::CLASSES;
    let top = r.y + 96.0;
    let bottom = r.y + r.h - 54.0;
    let gap = 26.0;
    let cols = 3usize;
    let col_w = (r.w - 48.0 - (cols - 1) as f32 * gap) / cols as f32;

    // Height of one entry, so the page break can be decided before drawing.
    let block_h = |c: &gearmaster_engine::class::ClassDef| -> f32 {
        let blurb = wrap_px(c.blurb, col_w - 12.0, 12.0).len() as f32;
        let power = wrap_px(&c.power.describe(), col_w - 12.0, 12.0).len() as f32;
        22.0 + blurb * 15.0 + 6.0 + c.requires.len().max(1) as f32 * 15.0 + 4.0 + power * 15.0 + 14.0
    };

    let mut at = 0usize;
    let mut this_page = 0usize;
    let mut col = 0usize;
    let mut y = top;
    let mut pages = 1usize;
    while at < CLASSES.len() {
        let c = &CLASSES[at];
        let needed = block_h(c);
        if y + needed > bottom {
            if col + 1 < cols {
                col += 1;
                y = top;
            } else {
                if this_page == page {
                    break;
                }
                this_page += 1;
                pages += 1;
                col = 0;
                y = top;
                continue;
            }
        }
        if this_page == page {
            let x = r.x + 24.0 + col as f32 * (col_w + gap);
            ui_text(words::class(c.name), x, y, 16.0, col_gold());
            y += 20.0;
            for l in wrap_px(&words::retell(c.blurb), col_w - 12.0, 12.0) {
                ui_text(&l, x + 8.0, y, 12.0, Color::from_rgba(198, 200, 218, 255));
                y += 15.0;
            }
            y += 6.0;
            if c.requires.is_empty() {
                // Nothing you wear points at these, so the line that would
                // list what to build says where to go instead.
                let how = gearmaster_engine::class::how_you_get_it(c.name)
                    .unwrap_or("asks for nothing");
                for l in wrap_px(&words::retell(how), col_w - 12.0, 12.0) {
                    ui_text(&l, x + 8.0, y, 12.0, col_dim());
                    y += 15.0;
                }
            } else {
                for &(axis, need) in c.requires {
                    ui_text(
                        &format!("{} {}+", words::retell(&axis.name()), need),
                        x + 8.0,
                        y,
                        12.0,
                        Color::from_rgba(150, 200, 240, 255),
                    );
                    y += 15.0;
                }
            }
            y += 4.0;
            for l in wrap_px(&words::retell(&c.power.describe()), col_w - 12.0, 12.0) {
                ui_text(&l, x + 8.0, y, 12.0, col_ok());
                y += 15.0;
            }
            y += 14.0;
        } else {
            y += needed;
        }
        at += 1;
    }
    // Finish counting pages past the drawn one.
    if at < CLASSES.len() {
        let (mut c2, mut yy) = (col, y);
        for c in &CLASSES[at..] {
            let needed = block_h(c);
            if yy + needed > bottom {
                if c2 + 1 < cols {
                    c2 += 1;
                    yy = top;
                } else {
                    pages += 1;
                    c2 = 0;
                    yy = top;
                }
            }
            yy += needed;
        }
    }
    pages
}


/// Name, health, armour, mana and curses for one side of the battle screen.
#[allow(clippy::too_many_arguments)]
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
    empower: u32,
    shield: u32,
    fork: u32,
    curses: &[ActiveCurse],
    // Playback clock, so a curse chip can count itself down.
    now_ms: u32,
    // Rage, faith and nature, in that order.
    pools: [i32; 6],
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
    // Twenty rather than thirty. The two read as a pair at this height, and
    // twenty is the most there is room for: the next board's label starts at
    // y+74 and the pool marks have to fit between.
    armor_bar(x, y + 32.0, w, 20.0, armor, max);

    // The pools, each behind its own mark rather than a row of words - during
    // a fight these change constantly and want reading fast. Armour keeps its
    // mark here too even though the bar is labelled, because this row is where
    // the eye goes for a number.
    let mut gx = x;
    let gy = y + 54.0;
    for (which, value) in [("armor", Some(armor)), ("mana", mana)]
        .into_iter()
        .chain(
            // Six now, in `pool_index` order. A fusion sits after its parents
            // and only draws once there is one to draw - the row already drops
            // an empty pool, so a board that never fuses looks exactly as it
            // did.
            ["rage", "faith", "nature", "druidic might", "communion", "zealotry"]
                .into_iter()
                .zip(pools.iter().copied())
                .map(|(n, v)| (n, if v > 0 { Some(v) } else { None })),
        )
    {
        let Some(v) = value else { continue };
        let c = pool_color(which);
        draw_pool_glyph(gx, gy, 15.0, which, c);
        let text = format!("{}", v);
        ui_text(&text, gx + 19.0, gy + 12.0, 14.0, c);
        gx += 19.0 + text_width(&text, 14.0) + 16.0;
    }

    // Buffs and curses share the row with the pools rather than starting at a
    // fixed offset.
    //
    // They used to be laid out independently: pools flowed rightward from the
    // left edge, the two mana buffs were tacked onto the end of that, and the
    // curses began at x + 210 whatever else was there. Any build with a few
    // pools banked ran its numbers straight through its own curses, and a
    // fight with three curses up wrote them over each other.
    let mut chips: Vec<(String, Color, bool)> = Vec::new();
    if let Some(m) = mana {
        // Both buffs multiply the mana you are still holding, so show what
        // they currently work out to rather than just the stack count.
        if empower > 0 {
            chips.push((
                format!(
                    "empower x{} (+{}.{:02}x)",
                    empower,
                    empower as i32 * 5 * m / 100,
                    (empower as i32 * 5 * m) % 100
                ),
                Color::from_rgba(160, 190, 225, 255),
                false,
            ));
        }
        if shield > 0 {
            chips.push((
                format!("shield x{} (-{})", shield, shield as i32 * m),
                Color::from_rgba(160, 190, 225, 255),
                false,
            ));
        }
    }
    if fork > 0 {
        chips.push((
            format!("forking x{}", fork),
            Color::from_rgba(210, 170, 240, 255),
            false,
        ));
    }
    for c in curses {
        // Not just "you are cursed" - which is the one thing already obvious
        // from the chip being there. Stacks, what they currently work out to,
        // and how long is left.
        let stacks = if c.stacks > 1 { format!(" x{}", c.stacks) } else { String::new() };
        let left = c.until_ms.saturating_sub(now_ms);
        chips.push((
            format!(
                "{}{} {} {:.0}s",
                words::retell(c.name),
                stacks,
                c.effect,
                (left as f32 / 1000.0).ceil()
            ),
            Color::from_rgba(240, 190, 240, 255),
            true,
        ));
    }

    // One cursor, one row. Anything that will not fit is counted rather than
    // drawn on top of what is already there.
    let size = 12.0;
    let right = x + w;
    let mut dropped = 0usize;
    for (text, col, boxed) in &chips {
        let tw = text_width(text, size);
        let need = tw + if *boxed { 18.0 } else { 14.0 };
        if gx + need > right - 40.0 {
            dropped += 1;
            continue;
        }
        if *boxed {
            // Sixteen tall, not nineteen: the next board's label starts at
            // y+74 and a taller box printed straight through it.
            draw_rectangle(gx - 4.0, gy, tw + 10.0, 16.0, Color::from_rgba(90, 40, 90, 230));
        }
        ui_text(text, gx + 1.0, gy + 12.0, size, *col);
        gx += need;
    }
    if dropped > 0 {
        ui_text(&format!("+{}", dropped), gx + 2.0, gy + 12.0, size, col_dim());
    }

    if hp <= 0 {
        ui_text("DOWN", x + w - 52.0, y - 6.0, 18.0, col_bad());
    }
}

/// The panel's buttons, in a fixed order:
///   0 BEGIN FIGHT   1 SPEED   2 UNDO   3 CLEAR ALL   4 GLOSSARY   5 SCREENSHOT
///
/// UNDO and CLEAR ALL share a row on purpose: taking a clear back is the main
/// reason anyone reaches for undo.
fn button_rects(panel_x: f32) -> [Rect; 6] {
    let w = PANEL_W - 40.0;
    let x = panel_x + 20.0;
    // Fifty pixels shorter than it was: taking a screenshot had a row to
    // itself, the same width as BEGIN FIGHT, for something almost nobody
    // presses twice. It shares the glossary's row now and the panel above
    // keeps the difference - which it needs, with two classes to show.
    let y = LOGICAL_H - 272.0;
    let half = w / 2.0 - 5.0;
    let shot = 54.0;
    [
        Rect::new(x, y, w, 46.0),
        Rect::new(x, y + 56.0, w, 40.0),
        Rect::new(x, y + 106.0, half, 40.0),
        Rect::new(x + w / 2.0 + 5.0, y + 106.0, half, 40.0),
        Rect::new(x, y + 156.0, w - shot - 8.0, 40.0),
        Rect::new(x + w - shot, y + 156.0, shot, 40.0),
    ]
}

/// What a build produces each second, beyond damage. Rates rather than totals:
/// an item granting 20 armour every four seconds and one granting 5 every
/// second are the same thing and should read the same.
struct BuildRates {
    armor: f32,
    mana: f32,
    rage: f32,
    faith: f32,
    nature: f32,
    mind: f32,
    curses: f32,
    activations: f32,
    triggers: usize,
}

fn build_rates(items: &[ItemProfile]) -> BuildRates {
    let mut r = BuildRates {
        armor: 0.0,
        mana: 0.0,
        rage: 0.0,
        faith: 0.0,
        nature: 0.0,
        mind: 0.0,
        curses: 0.0,
        activations: 0.0,
        triggers: 0,
    };
    for p in items {
        let rate = 1000.0 / p.cooldown_ms.max(1) as f32;
        r.activations += rate;
        r.armor += p.stats.armor as f32 * rate;
        r.mana += p.stats.mana as f32 * rate;
        r.rage += p.stats.rage as f32 * rate;
        r.faith += p.stats.faith as f32 * rate;
        r.nature += p.stats.nature as f32 * rate;
        r.mind += p.stats.mind as f32 * rate;
        r.triggers += p.triggers.len();
        for t in &p.triggers {
            if trigger_curses(t) {
                r.curses += rate;
            }
        }
    }
    r
}

fn trigger_curses(t: &gearmaster_engine::piece::Trigger) -> bool {
    use gearmaster_engine::piece::{Action, Target, Trigger};
    let curses = |a: &Action| matches!(a, Action::Curse { target: Target::Enemy, .. });
    match t {
        Trigger::PerAdjacentEmpty(inner) => trigger_curses(inner),
        Trigger::Consume { per, .. } => curses(per),
        Trigger::OnActivate(a)
        | Trigger::OnBattleStart(a)
        | Trigger::PerAdjacentItem { action: a, .. }
        | Trigger::OnAdjacentActivate(a)
        | Trigger::OnAlignedActivate(a)
        | Trigger::OnDiagonalActivate(a)
        | Trigger::OnOtherCast(a) => curses(a),
        Trigger::Watch { then, .. } => curses(then),
        Trigger::SpendGold { on_success, .. } => curses(on_success),
        Trigger::SpendMana { on_success, on_failure, .. }
        | Trigger::Spend { on_success, on_failure, .. } => curses(on_success) || curses(on_failure),
    }
}

#[allow(clippy::too_many_arguments)]
/// Which of a tile's marks a summary line is describing, so the panel can
/// show the same disc or diamond the piece itself is wearing.
fn note_marker(note: &str) -> Marker {
    if note.contains(':') && !note.contains(" from ") && !note.contains("doubled") {
        Marker::Bonus
    } else {
        Marker::Effect
    }
}

fn note_color(note: &str) -> Color {
    note_marker(note).color()
}

/// "1 item assembled" is too wide for the panel row once the text is large.
/// The full sentence is still under the board and in the row's tooltip.
fn short_summary(r: &SlotReport) -> String {
    let done = r.assembled_count();
    if done > 0 {
        format!("{} item{}", done, if done == 1 { "" } else { "s" })
    } else if r.is_empty() {
        String::from("empty")
    } else {
        String::from("unfinished")
    }
}

/// The eight axes the class card charts, and what to call them in the corner
/// of a chart that has to stay readable at 40 pixels.
///
/// Fixed, not "this build's strongest eight": the point of the shape is that
/// two builds can be compared by it, which stops being true if the axes move.
/// The pools get one corner between them, since a build almost always leans on
/// one of the four; the individual figures are printed under the chart.
const RADAR: &[(Axis, &str)] = &[
    (Axis::Arcana, "ARC"),
    (Axis::Brutality, "IRON"),
    (Axis::Cadence, "SPD"),
    (Axis::Bulwark, "ARM"),
    (Axis::Ward, "WARD"),
    (Axis::Mass, "MASS"),
    (Axis::Weave, "WEAVE"),
    (Axis::Malice, "MAL"),
];

/// The build, drawn as a shape, with the class it earns and what it is nearest
/// to otherwise.
fn render_class_card(run: &Run, mx: f32, my: f32) {
    let fp = run.fingerprint();
    let ranked = run.class_outlook();

    let (w, h) = (400.0, 452.0);
    let x = (mx - w - 18.0).max(4.0);
    let y = (my - 40.0).min(LOGICAL_H - h - 6.0).max(4.0);
    draw_rectangle(x, y, w, h, Color::from_rgba(12, 12, 20, 250));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::from_rgba(120, 120, 155, 255));

    ui_text("WHAT YOU ARE CARRYING", x + 16.0, y + 26.0, 14.0, col_gold());

    // ---- the chart -------------------------------------------------------
    let (cx, cy) = (x + w / 2.0, y + 150.0);
    let rad = 82.0;
    let n = RADAR.len();
    let point = |i: usize, frac: f32| -> Vec2 {
        // First corner at the top, then clockwise - the way these are read.
        let a = -std::f32::consts::FRAC_PI_2
            + i as f32 * std::f32::consts::TAU / n as f32;
        Vec2::new(cx + a.cos() * rad * frac, cy + a.sin() * rad * frac)
    };

    // Rings, so a corner can be read off as a rough number without a scale.
    for ring in [0.25f32, 0.5, 0.75, 1.0] {
        for i in 0..n {
            let a = point(i, ring);
            let b = point((i + 1) % n, ring);
            let c = if ring == 1.0 {
                Color::from_rgba(96, 96, 130, 255)
            } else {
                Color::from_rgba(52, 52, 72, 255)
            };
            draw_line(a.x, a.y, b.x, b.y, 1.0, c);
        }
    }
    for i in 0..n {
        let e = point(i, 1.0);
        draw_line(cx, cy, e.x, e.y, 1.0, Color::from_rgba(52, 52, 72, 255));
    }

    // The build itself: filled as a fan of triangles from the centre, so a
    // concave shape - which is most of them - still reads as solid.
    let vals: Vec<f32> =
        RADAR.iter().map(|&(a, _)| fp.get(a).clamp(0, 100) as f32 / 100.0).collect();
    let fill = Color::new(col_gold().r, col_gold().g, col_gold().b, 0.28);
    for i in 0..n {
        let a = point(i, vals[i].max(0.01));
        let b = point((i + 1) % n, vals[(i + 1) % n].max(0.01));
        draw_triangle(Vec2::new(cx, cy), a, b, fill);
    }
    for i in 0..n {
        let a = point(i, vals[i].max(0.01));
        let b = point((i + 1) % n, vals[(i + 1) % n].max(0.01));
        draw_line(a.x, a.y, b.x, b.y, 2.0, col_gold());
    }

    // Corner labels, pushed out past the outer ring and nudged so the ones on
    // the left do not overhang the panel edge.
    for (i, &(axis, tag)) in RADAR.iter().enumerate() {
        let p = point(i, 1.0);
        let dx = p.x - cx;
        let label = format!("{} {}", tag, fp.get(axis));
        let label = words::retell(&label);
        let tw = text_width(&label, 12.0);
        let lx = if dx.abs() < 6.0 {
            p.x - tw / 2.0
        } else if dx < 0.0 {
            p.x - tw - 8.0
        } else {
            p.x + 8.0
        };
        // Clear of the outer ring in every direction: a label sitting on the
        // polygon is unreadable against the fill.
        let ly = p.y
            + if (p.y - cy).abs() < 6.0 {
                4.0
            } else if p.y < cy {
                -9.0
            } else {
                18.0
            };
        ui_text(&label, lx, ly, 12.0, LIGHTGRAY);
    }

    // ---- everything the chart has no corner for --------------------------
    let mut ty = y + 272.0;
    let pools = [
        (Axis::Attunement, "mana"),
        (Axis::Wrath, "rage"),
        (Axis::Devotion, "faith"),
        (Axis::Growth, "nature"),
    ];
    let line: Vec<String> =
        pools.iter().map(|&(a, n)| format!("{} {}", words::retell(n), fp.get(a))).collect();
    ui_text(&line.join("   "), x + 16.0, ty, 12.0, col_dim());
    ty += 16.0;
    let more = format!(
        "sorcery {}   orbits {}   answering {}   pierce {}",
        fp.get(Axis::Sorcery),
        fp.get(Axis::Orbits),
        fp.get(Axis::Answering),
        fp.get(Axis::Puncture)
    );
    ui_text(&more, x + 16.0, ty, 12.0, col_dim());
    ty += 26.0;

    // ---- the class this earns, and the next two -------------------------
    let head = if run.classes.is_empty() { "YOU WOULD BE GIVEN" } else { "YOUR CLASSES" };
    ui_text(head, x + 16.0, ty, 12.0, col_dim());
    ty += 20.0;
    let mut shown: Vec<&'static gearmaster_engine::class::ClassDef> = run.classes.clone();
    if shown.is_empty() {
        shown.extend(ranked.iter().find(|m| m.eligible).map(|m| m.class));
    }
    for c in &shown {
        ui_text(words::class(c.name), x + 16.0, ty, 17.0, col_gold());
        ty += 18.0;
        let d = words::retell(&c.power.describe());
        for l in wrap_px(&d, w - 40.0, 12.0).into_iter().take(4) {
            ui_text(&l, x + 22.0, ty, 12.0, LIGHTGRAY);
            ty += 14.0;
        }
    }
    ty += 10.0;

    // The two it is nearest to otherwise - what to build toward next. Skips
    // whatever is already being worn or offered above.
    ui_text("NEAREST OTHERS", x + 16.0, ty, 12.0, col_dim());
    ty += 18.0;
    // Not the Wanderer: it has no requirements, always qualifies, and saying
    // so tells nobody anything about their build.
    for m in ranked
        .iter()
        .filter(|m| !shown.iter().any(|c| c.name == m.class.name))
        .filter(|m| !m.class.requires.is_empty())
        .take(2)
    {
        let short: Vec<String> = m
            .detail
            .iter()
            .filter(|(_, need, have)| have < need)
            .map(|(a, need, have)| format!("{} {}/{}", a.name(), have, need))
            .collect();
        let (tag, colour) = if m.eligible {
            ("also qualifies".to_string(), col_ok())
        } else {
            (short.join(", "), Color::from_rgba(150, 200, 240, 255))
        };
        ui_text(words::class(m.class.name), x + 22.0, ty, 13.0, WHITE);
        ty += 15.0;
        for l in wrap_px(&tag, w - 52.0, 11.0).into_iter().take(2) {
            ui_text(&l, x + 30.0, ty, 11.0, colour);
            ty += 13.0;
        }
        ty += 4.0;
    }
}

fn render_panel(
    layout: &Layout,
    run: &Run,
    reports: &[SlotReport],
    message: &str,
    speed: f32,
    hover: &mut Hover,
    mx: f32,
    my: f32,
) {
    let x = layout.panel_x;
    draw_rectangle(x, 0.0, PANEL_W, LOGICAL_H, col_panel());

    // The panel used to flow top to bottom throughout, and granting a class
    // inserts a block partway down - which pushed the opponent into the
    // message and the message into the buttons. So the bottom three sections
    // are pinned to fixed lines instead, and only the character read-out above
    // them flows, clipped where the class band begins.
    let msg_lines = 3.0;
    let msg_top = button_rects(layout.panel_x)[0].y - 12.0 - msg_lines * line_h(14.0);
    // The opponent block is pinned too, directly above the message. Only the
    // section above it flows, so nothing below can be pushed into anything
    // else however long a class description runs.
    let opp_top = msg_top - 118.0;
    // The class band is pinned as well, so the only thing that flows is the
    // character read-out above it - and that gets clipped at this line. It has
    // to hold two classes and a line about the next fountain, which is what
    // the space freed by shrinking the screenshot button pays for.
    let class_top = opp_top - 132.0;
    // And the run line above that. Bottom-up, every section below the gear
    // list has a fixed home, so nothing can be pushed into anything else.
    let run_top = class_top - 44.0;
    let room = |y: f32, need: f32| y + need <= msg_top;


    draw_line(x, 0.0, x, LOGICAL_H, 2.0, Color::from_rgba(60, 60, 85, 255));

    let mut y = 38.0;
    ui_text("GEAR MASTER", x + 20.0, y, 26.0, WHITE);
    y += 30.0;

    let stats = run.player_stats();
    ui_text(words::word("character", "YOUR CHARACTER"), x + 20.0, y, 14.0, col_dim());
    y += 22.0;
    for (label, value, color) in [
        (
            "Health",
            if run.grown_health > 0 {
                format!("{}  (+{} grown)", stats.health, run.grown_health)
            } else {
                format!("{}", stats.health)
            },
            Color::from_rgba(120, 220, 150, 255),
        ),
        ("Strength", format!("{}", stats.strength), Color::from_rgba(240, 170, 120, 255)),
        ("Regen", format!("{}/turn", stats.regen), Color::from_rgba(140, 200, 240, 255)),
        // No global weapon power line any more. Power belongs to the item
        // carrying it, so one figure here would be the sum of five slots'
        // worth of a multiplier that never applies together - which is exactly
        // the thing that was sending damage through the roof.
    ] {
        ui_text(label, x + 20.0, y, 16.0, LIGHTGRAY);
        let d_w = text_width(&value, 16.0);
        ui_text(&value, x + PANEL_W - 20.0 - d_w, y, 16.0, color);
        y += 17.0;
    }
    // Damage is per item now, so a single "damage per attack" figure would
    // lie. Total damage a second across every weapon is the honest summary.
    let items = run.combat_items();
    let dps_milli: i64 = items
        .iter()
        .map(|i| i.dps_milli(stats.strength))
        .sum();
    ui_text("Damage / second", x + 20.0, y, 17.0, WHITE);
    let label = format!("{}.{}", dps_milli / 1000, (dps_milli % 1000) / 100);
    let d_w = text_width(&label, 19.0);
    ui_text(&label, x + PANEL_W - 20.0 - d_w, y, 19.0, col_gold());
    y += 20.0;

    // What the build produces per second beyond damage.
    let rates = build_rates(&items);
    let mut cells: Vec<(String, Color)> = Vec::new();
    for (v, label, colour) in [
        (rates.armor, "armour", col_ok()),
        (rates.mana, "mana", Color::from_rgba(140, 200, 240, 255)),
        (rates.rage, "rage", col_foe()),
        (rates.faith, "faith", col_gold()),
        (rates.nature, "nature", Color::from_rgba(140, 220, 150, 255)),
        (rates.mind, "mind", Color::from_rgba(200, 160, 220, 255)),
        (rates.curses, "curse", col_trigger()),
    ] {
        if v >= 0.05 {
            cells.push((format!("{:.1} {}/s", v, words::retell(label)), colour));
        }
    }
    cells.push((format!("{} items", items.len()), col_dim()));
    cells.push((format!("{:.1} acts/s", rates.activations), col_dim()));
    if rates.triggers > 0 {
        cells.push((format!("{} triggers", rates.triggers), col_trigger()));
    }
    // One line only - the gear list below needs its five rows more than this
    // needs to be complete - with everything on hover.
    let line_y = y;
    let mut cx = x + 20.0;
    let mut shown = 0usize;
    for (text, colour) in &cells {
        let w = text_width(text, 12.0);
        if cx + w > x + PANEL_W - 24.0 {
            break;
        }
        ui_text(text, cx, line_y, 12.0, *colour);
        cx += w + 12.0;
        shown += 1;
    }
    if shown < cells.len() {
        ui_text("...", cx, line_y, 12.0, col_dim());
    }
    hover.over(
        Rect::new(x + 14.0, line_y - 13.0, PANEL_W - 28.0, 18.0),
        mx,
        my,
        || {
            let mut lines = vec![("PER SECOND".to_string(), col_gold())];
            lines.extend(cells.iter().cloned());
            // The defence triangle has no room in the panel proper, and a
            // player who has stacked 40% resist should be able to find out.
            let mut def: Vec<(String, Color)> = Vec::new();
            for (v, label) in [
                (stats.physical_resist, "physical resist"),
                (stats.physical_pierce, "physical piercing"),
                (stats.physical_harden, "physical hardening"),
                (stats.magic_resist, "magic resist"),
                (stats.magic_pierce, "magic piercing"),
                (stats.magic_harden, "magic hardening"),
                (stats.mind_resist, "mind resist"),
                (stats.curse_resist, "curse resist"),
            ] {
                if v != 0 {
                    def.push((format!("{}% {}", v, label), LIGHTGRAY));
                }
            }
            if !def.is_empty() {
                lines.push((String::new(), col_dim()));
                lines.push(("DEFENCES".to_string(), col_gold()));
                lines.extend(def);
            }
            lines
        },
    );
    y += 14.0;

    // Per-slot assembly readout. One row each; the bonus notes are what used
    // to run off the edge of the panel, so they live on the row's hover now.
    ui_text("GEAR", x + 20.0, y, 14.0, col_dim());
    y += 22.0;
    for r in reports {
        if y + 19.0 > run_top - 6.0 {
            break;
        }
        let done = r.assembled_count();
        let (mark, color) = if done > 0 { ("+", col_ok()) } else { ("-", col_dim()) };
        let notes = r.notes();
        let row = Rect::new(x + 14.0, y - 15.0, PANEL_W - 28.0, 21.0);
        let hot = row.contains(Vec2::new(mx, my));
        if hot && !notes.is_empty() {
            draw_rectangle(row.x, row.y, row.w, row.h, Color::from_rgba(255, 255, 255, 16));
        }
        ui_text(mark, x + 20.0, y, 16.0, color);
        ui_text(r.slot.name(), x + 36.0, y, 16.0, if done > 0 { WHITE } else { col_dim() });
        // A lit dot per bonus this slot is running, so the row still says at a
        // glance that there is something to hover for.
        let mut dot = x + 40.0 + text_width(r.slot.name(), 16.0) + 10.0;
        for note in &notes {
            draw_marker(dot, y - 5.0, note_marker(note), true);
            dot += 14.0;
        }
        let status = short_summary(r);
        let d_w = text_width(&status, 14.0);
        ui_text(
            &status,
            x + PANEL_W - 20.0 - d_w,
            y,
            14.0,
            if done > 0 { col_ok() } else if r.is_empty() { col_dim() } else { col_bad() },
        );
        y += 19.0;
        hover.over(row, mx, my, || {
            let mut lines =
                vec![(format!("{}  -  {}", r.slot.name(), r.summary()), WHITE)];
            let contrib = words::retell(&r.stats.summary());
            if !contrib.is_empty() {
                for l in wrap_px(&contrib, 380.0, 14.0) {
                    lines.push((l, LIGHTGRAY));
                }
            }
            for note in &notes {
                for (i, l) in wrap_px(note, 380.0, 14.0).into_iter().enumerate() {
                    lines.push((
                        if i == 0 { l } else { format!("  {}", l) },
                        note_color(note),
                    ));
                }
            }
            lines
        });
    }

    y = run_top;
    ui_text("RUN", x + 20.0, y, 14.0, col_dim());
    let mode_label = match run.lives_left() {
        Some(n) => format!("{} {}  ·  {} lives", run.mode.name(), run.difficulty.label(), n),
        None => format!("{} {}", run.mode.name(), run.difficulty.label()),
    };
    let m_w = text_width(&mode_label, 13.0);
    ui_text(
        &mode_label,
        x + PANEL_W - 20.0 - m_w,
        y,
        13.0,
        if run.lives_left() == Some(1) { col_bad() } else { col_dim() },
    );
    y += 22.0;
    // One line rather than three rows: the panel has to find room for a class
    // block, and three numbers this small do not need a row each.
    ui_text(&format!("{}", run.gold), x + 20.0, y, 17.0, col_gold());
    let gw = text_width(&format!("{}", run.gold), 17.0);
    ui_text(words::word("gold-lower", "gold"), x + 24.0 + gw, y, 13.0, col_dim());
    let record = format!("{} won  ·  {} lost", run.wins, run.losses);
    let r_w = text_width(&record, 13.0);
    ui_text(&record, x + PANEL_W - 20.0 - r_w, y, 13.0, col_dim());

    // ---- class ----
    // Shown every frame, not only at the fountain: the whole point is that the
    // outcome is something you build toward rather than find out about.
    let outlook = run.class_outlook();
    y = class_top;
    // The whole class block is one hover target: it is a chart, so it is drawn
    // after everything else rather than being squeezed into the panel.
    if Rect::new(x, class_top - 18.0, PANEL_W, opp_top - class_top)
        .contains(Vec2::new(mx, my))
    {
        hover.class_card = true;
    }
    let at_fountain = run.at_fountain() || run.at_doubling_fountain();
    if !run.classes.is_empty() {
        ui_text(
            if run.classes.len() > 1 {
                words::word("classes", "YOUR CLASSES")
            } else {
                words::word("class", "YOUR CLASS")
            },
            x + 20.0,
            y,
            14.0,
            col_dim(),
        );
        y += 22.0;
        // The band below this is pinned, so a long list has to stop somewhere.
        // Two lines each, and one line held back for the "+ N more" that says
        // where the rest went. Classes stack now - Piety and Tired both do -
        // so "however many there are" is no longer a small number.
        let room = ((opp_top - 26.0 - y) / 36.0).floor().max(1.0) as usize;
        let show = if run.classes.len() > room { room.saturating_sub(1).max(1) } else { run.classes.len() };
        for c in run.classes.iter().take(show) {
            ui_text(words::class(c.name), x + 20.0, y, 19.0, col_gold());
            y += 18.0;
            // The band is pinned, so the text shrinks to fit rather than being
            // cut off - a power whose description is a word longer than the
            // last one must not silently lose its ending.
            let text = words::retell(&c.power.short());
            let size = fitting_size(&text, PANEL_W - 48.0, &[13.0, 12.0, 11.0, 10.0]);
            draw_capped(&text, x + 24.0, y, PANEL_W - 48.0, size, LIGHTGRAY, 1);
            y += 18.0;
        }
        if show < run.classes.len() {
            let rest = &run.classes[show..];
            let row = Rect::new(x + 16.0, y - 4.0, PANEL_W - 32.0, 20.0);
            let hot = row.contains(Vec2::new(mx, my));
            ui_text(&format!("+{} more", rest.len()), x + 20.0, y + 10.0, 13.0, if hot { col_gold() } else { col_dim() });
            if hot {
                // Wins over the class chart: a cursor on this line is asking
                // about these, not about the fountain schedule behind them.
                hover.class_card = false;
                hover.overflow = Some(Pinned {
                    title: format!("{} MORE", rest.len()),
                    at: Vec2::new(x - 312.0, y),
                    entries: rest.iter().map(|c| PinnedEntry::Class(*c)).collect(),
                });
            }
            y += 24.0;
        }
    }
    // Everything below is pinned, so this block has a hard ceiling: it must
    // never grow into the opponent. Each line is asked for room first.
    let bottom = opp_top - 6.0;
    let fits = |y: f32, need: f32| y + need <= bottom;
    if run.classes.len() < Run::FOUNTAINS.len() && fits(y, 20.0) {
        ui_text(
            &if at_fountain {
                "THE FOUNTAIN IS WAITING".to_string()
            } else {
                match run.next_fountain() {
                    Some(r) => format!("A FOUNTAIN ON RUNG {}", r + 1),
                    None => "NO FOUNTAINS LEFT".to_string(),
                }
            },
            x + 20.0,
            y,
            14.0,
            if at_fountain { col_gold() } else { col_dim() },
        );
        y += 20.0;
        // What the *next* fountain would hand over, which is never something
        // you are already wearing. Saying "you would be given Geomancer" to
        // somebody who is standing there as a Geomancer is worse than saying
        // nothing.
        let held: Vec<&str> = run.classes.iter().map(|c| c.name).collect();
        let next = outlook.iter().find(|m| m.eligible && !held.contains(&m.class.name));
        if let Some(best) = next {
            // The label on the left and the name on the right, unless they
            // would meet. A long themed title - "Grand Calculator", "Galapagos
            // Timekeeper" - printed straight through "it would give you", so
            // the two share a line only when both actually fit on it.
            let title = words::class(best.class.name);
            let label = "it would give you";
            let lw = text_width(label, 12.0);
            let tw = text_width(title, 16.0);
            let both = lw + tw + 16.0 <= PANEL_W - 40.0;
            if both && fits(y, 18.0) {
                ui_text(label, x + 20.0, y, 12.0, col_dim());
                ui_text(title, x + PANEL_W - 20.0 - tw, y, 16.0, col_gold());
                y += 18.0;
            } else if fits(y, 34.0) {
                ui_text(label, x + 20.0, y, 12.0, col_dim());
                y += 16.0;
                let size = fitting_size(title, PANEL_W - 40.0, &[16.0, 15.0, 14.0, 13.0]);
                draw_capped(title, x + 20.0, y, PANEL_W - 40.0, size, col_gold(), 1);
                y += 18.0;
            }
        }
        // The nearest thing you have not got, and what it is short of - so a
        // player can chase a class instead of waiting to be told. Only if
        // there is genuinely room for both of its lines.
        if let Some(near) =
            outlook.iter().find(|m| !m.eligible && !held.contains(&m.class.name))
        {
            let short: Vec<String> = near
                .detail
                .iter()
                .filter(|(_, need, have)| have < need)
                .map(|(a, need, have)| format!("{} {}/{}", a.name(), have, need))
                .collect();
            if !short.is_empty() && fits(y, 30.0) {
                ui_text(
                    &format!("{} needs", words::class(near.class.name)),
                    x + 20.0,
                    y,
                    12.0,
                    Color::from_rgba(150, 200, 240, 255),
                );
                y += 15.0;
                let line = short.join(", ");
                let size = fitting_size(&line, PANEL_W - 52.0, &[12.0, 11.0, 10.0]);
                draw_capped(&line, x + 26.0, y, PANEL_W - 52.0, size, col_dim(), 1);
            }
        }
    }


    let mut y = opp_top;
    let m = run.monster();
    // The whole opponent header opens the preview. Not the sprite alone: a
    // 62px target for the most useful panel on the screen is a target nobody
    // finds.
    let opp_card = Rect::new(x, opp_top - 16.0, PANEL_W, 58.0);
    let opp_hot = opp_card.contains(Vec2::new(mx, my)) && !run.at_fountain();
    if opp_hot {
        hover.enemy_card = true;
        draw_rectangle(opp_card.x, opp_card.y, opp_card.w, opp_card.h, Color::from_rgba(255, 255, 255, 12));
    }
    ui_text(
        if let Some((d, floor)) = run.dungeon {
            // A dungeon is not the road, and the panel should not pretend it
            // is - "rung 10 of 50" while you are three floors under a hamlet
            // is the sort of thing that reads as a bug.
            Box::leak(format!("{}  ·  FLOOR {} OF {}", d.name, floor + 1, d.floors.len()).into_boxed_str())
        } else if run.at_fountain() || run.at_doubling_fountain() {
            words::word("beyond-fountain", "BEYOND THE FOUNTAIN")
        } else {
            words::word("opponent", "NEXT OPPONENT")
        },
        x + 20.0,
        y,
        14.0,
        col_dim(),
    );
    // On the label row, which has room. Anywhere lower collides with the
    // mini-boss warning.
    if !opp_hot {
        let hint = words::word("inspect-hint", "hover for their board");
        ui_text(
            hint,
            x + PANEL_W - 20.0 - text_width(hint, 11.0),
            y,
            11.0,
            Color::from_rgba(112, 112, 134, 255),
        );
    }
    y += 20.0;
    draw_monster(
        x + PANEL_W - 78.0,
        y - 16.0,
        62.0,
        m.sprite,
        col_foe(),
        Color::from_rgba(40, 22, 20, 255),
    );
    let mname = words::monster(m.name);
    ui_text(mname, x + 20.0, y, 17.0, Color::from_rgba(230, 140, 120, 255));
    let bounty = coins(m.bounty);
    ui_text(&bounty, x + 20.0 + text_width(mname, 17.0) + 14.0, y, 15.0, col_gold());
    y += 18.0;
    ui_text(
        &format!("rung {} of {}  ·  {} hp", run.rung + 1, LADDER.len(), m.health),
        x + 20.0,
        y,
        13.0,
        if opp_hot { Color::from_rgba(190, 190, 210, 255) } else { col_dim() },
    );
    y += 16.0;
    // How far off the next named fight is, and which kind. A boss carries
    // fifteen items of gear and a mini-boss ten; walking into one having just
    // spent everything is the kind of surprise that reads as unfairness rather
    // than as difficulty.
    if let Some((away, rank, name)) = run.next_named() {
        use gearmaster_engine::combat::Rank;
        let what = match rank {
            Rank::Boss => words::word("boss", "BOSS"),
            _ => words::word("miniboss", "MINI-BOSS"),
        };
        let line = match away {
            0 => format!("{} - {}", what, words::monster(name)),
            1 => format!("{} next fight: {}", what, words::monster(name)),
            n => format!("{} in {} fights: {}", what, n, words::monster(name)),
        };
        let col = match (rank, away) {
            (Rank::Boss, 0..=1) => col_bad(),
            (_, 0..=1) => col_gold(),
            (Rank::Boss, _) => Color::from_rgba(214, 150, 130, 255),
            _ => col_dim(),
        };
        // Stops short of the silhouette, which starts at PANEL_W - 78 and
        // hangs down over these lines.
        let room = PANEL_W - 108.0;
        let size = fitting_size(&line, room, &[13.0, 12.0, 11.0, 10.0]);
        draw_capped(&line, x + 20.0, y, room, size, col, 1);
        y += 16.0;
    }
    // What the theme has to say about this one - why it is in your way, or
    // why it is not. The superbosses at the top are marked here as optional
    // rather than as the plot.
    if let Some(note) = words::current().note(m.name) {
        for l in wrap_px(note, PANEL_W - 44.0, 12.0).into_iter().take(2) {
            if !room(y, 14.0) {
                break;
            }
            ui_text(&l, x + 20.0, y, 12.0, Color::from_rgba(170, 178, 200, 255));
            y += 14.0;
        }
        y += 4.0;
    }
    for a in m.attacks {
        if !room(y, 14.0) {
            break;
        }
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
    if (m.mind_resist > 0 || m.curse_resist > 0) && room(y, 14.0) {
        ui_text(
            &format!("  resists: {}% mind, {}% curse", m.mind_resist, m.curse_resist),
            x + 20.0,
            y,
            12.0,
            Color::from_rgba(190, 160, 200, 255),
        );
    }
    // Whatever the flow did above, the message lands here.
    let mut my = msg_top;
    for line in wrap_px(message, PANEL_W - 40.0, 14.0).into_iter().take(msg_lines as usize) {
        ui_text(&line, x + 20.0, my, 14.0, Color::from_rgba(225, 225, 240, 255));
        my += line_h(14.0);
    }

    // Buttons
    let r = button_rects(layout.panel_x);
    button(
        r[0],
        if run.at_fountain() || run.at_doubling_fountain() {
            words::word("fountain-take-btn", "DRINK FROM THE FOUNTAIN")
        } else {
            words::word("begin-fight", "BEGIN FIGHT")
        },
        true,
        mx,
        my,
    );
    button(r[1], &format!("SPEED {}x", speed_label(speed)), true, mx, my);

    let can_undo = run.undoable().is_some();
    button(r[2], "UNDO", can_undo, mx, my);
    if can_undo {
        hover.over(r[2], mx, my, || {
            vec![(format!("undo {}", run.undoable().unwrap_or("")), LIGHTGRAY)]
        });
    }
    button(r[3], "CLEAR ALL", true, mx, my);
    button(r[4], words::word("glossary", "WHAT THE WORDS MEAN"), true, mx, my);
    button(r[5], "TOOLS", true, mx, my);
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
    // A "+ N more" the player has clicked to hold open. Outlives the frame it
    // was opened in - that is the whole point of it.
    let mut pinned: Option<Pinned> = None;
    // The rumour shelf waiting to be paid for. A bar does not take money, so
    // buying one is two clicks: the shelf, then the piece that goes over it.
    let mut bartering: Option<usize> = None;
    let mut tools_open = std::env::var("GEARMASTER_TOOLS").is_ok();
    // GEARMASTER_PASTE=<code> loads a shared run at startup. The clipboard is
    // not readable before the window has focus, so this is the only way to put
    // somebody else's board on screen without a pair of hands.
    // GEARMASTER_PASTE=win is shorthand for the winning board, which is the
    // one worth looking at when the question is "what does a full slot do to
    // this screen".
    let mut imported: Option<gearmaster_engine::share::Shared> = std::env::var("GEARMASTER_PASTE")
        .ok()
        .map(|c| if c == "win" { gearmaster_engine::share::A_WINNING_RUN.to_string() } else { c })
        .as_deref()
        .and_then(gearmaster_engine::share::import);
    // Lines back from the newest the battle log is holding at. Zero follows.
    // GEARMASTER_LOG_SCROLL=<n> starts the log scrolled back, so a screenshot
    // can show a scrolled state - there is no way to send a wheel event to a
    // headless capture.
    let mut log_scroll: usize =
        std::env::var("GEARMASTER_LOG_SCROLL").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    // Kept between fights, and settable before one starts.
    let mut playback_speed = DEFAULT_SPEED;
    let mut glossary_open = std::env::var("GEARMASTER_GLOSSARY").is_ok();
    let mut fountain_open = std::env::var("GEARMASTER_FOUNTAIN").is_ok();
    let mut picker_open = std::env::var("GEARMASTER_PICKER").is_ok();
    let mut picker_page: usize = 0;
    let mut glossary_tab: usize = std::env::var("GEARMASTER_GLOSSARY_TAB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut glossary_page: usize = std::env::var("GEARMASTER_GLOSSARY_PAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // Where the game opens: the intro pages, then the mode picker, then play.
    // Any debug hook skips straight to the board.
    let skip_intro = std::env::var("GEARMASTER_PRESET").is_ok()
        || std::env::var("GEARMASTER_FIGHT").is_ok()
        || std::env::var("GEARMASTER_PLACE").is_ok()
        || std::env::var("GEARMASTER_SKIP_INTRO").is_ok();
    let mut chosen_theme: &'static gearmaster_engine::theme::Theme =
        match std::env::var("GEARMASTER_THEME") {
            Ok(id) => gearmaster_engine::theme::by_id(&id),
            Err(_) => gearmaster_engine::theme::THEMES[0],
        };
    let mut opening = if skip_intro { Opening::Playing } else { Opening::Intro(0) };
    let mut chosen_difficulty = Difficulty::Easy;
    if let Ok(d) = std::env::var("GEARMASTER_DIFFICULTY") {
        chosen_difficulty = Difficulty::ALL
            .iter()
            .copied()
            .find(|x| x.name().eq_ignore_ascii_case(&d))
            .unwrap_or(Difficulty::Easy);
        run.difficulty = chosen_difficulty;
    }
    // GEARMASTER_OPENING=mode|2|... opens on a given page, for screenshots.
    if let Ok(o) = std::env::var("GEARMASTER_OPENING") {
        opening = if o == "story" {
            Opening::Story
        } else if o == "mode" {
            Opening::ModeSelect
        } else {
            Opening::Intro(o.parse::<usize>().unwrap_or(0).min(INTRO.len() - 1))
        };
    }
    // GEARMASTER_CLASS=<name> grants a class without playing to the fountain,
    // so the panel can be inspected in the state a granted class puts it in.
    // GEARMASTER_PLACE="Name@Slot:x,y;..." puts an exact build on the board,
    // so a specific interaction can be inspected without playing to it.
    if let Ok(spec) = std::env::var("GEARMASTER_PLACE") {
        run = Run::with_all_pieces();
        for entry in spec.split(';').filter(|e| !e.is_empty()) {
            let (name, rest) = entry.split_once('@').expect("Name@Slot:x,y");
            let (slot_name, coords) = rest.split_once(':').expect("Slot:x,y");
            let (xs, ys) = coords.split_once(',').expect("x,y");
            let slot = SlotKind::ALL
                .iter()
                .copied()
                .find(|s| s.name().eq_ignore_ascii_case(slot_name))
                .expect("unknown slot");
            let id = run
                .owned
                .iter()
                .copied()
                .find(|&i| run.registry.def(i).name == name && !run.is_equipped(i))
                .expect("unknown or already-placed component");
            let (px, py) = (xs.parse().unwrap(), ys.parse().unwrap());
            if let Err(e) = run.equip(id, slot, px, py) {
                // A debug hook, but aborting with "fits" tells you nothing
                // about which entry was wrong or why.
                panic!("GEARMASTER_PLACE: {} at {} {},{}: {}", name, slot.name(), px, py, e);
            }
        }
    }
    // GEARMASTER_PIN=0,3 pins shelves, so the pinned state can be inspected.
    if let Ok(v) = std::env::var("GEARMASTER_PIN") {
        for n in v.split(',').filter_map(|n| n.trim().parse::<usize>().ok()) {
            run.shop.toggle_lock(n);
        }
    }
    // GEARMASTER_LOCK=1 locks the first assembled item it finds, so the
    // locked state can be inspected.
    // GEARMASTER_LOCK=stow also lifts it into the tray, so the stowed card can
    // be inspected.
    if let Ok(v) = std::env::var("GEARMASTER_LOCK") {
        let first = SlotKind::ALL.iter().find_map(|&k| {
            run.report(k).items.into_iter().find(|i| i.assembled).and_then(|i| i.pieces.first().copied())
        });
        if let Some(p) = first {
            run.toggle_lock_item(p);
            if v == "stow" {
                let _ = run.unequip_locked(p);
                // Nothing else in the tray, so the stowed card is the one you
                // are looking at rather than the hundredth one down.
                if let Some(set) = run.locked_set(p).map(|s| s.to_vec()) {
                    run.owned.retain(|o| set.contains(o));
                }
            }
        }
    }
    if let Ok(c) = std::env::var("GEARMASTER_CLASS") {
        // Comma-separated, so both fountains can be inspected at once.
        run.classes = c
            .split(',')
            .filter_map(|want| {
                gearmaster_engine::class::CLASSES
                    .iter()
                    .find(|d| d.name.eq_ignore_ascii_case(want.trim()))
            })
            .collect();
    }
    if std::env::var("GEARMASTER_THEME").is_ok() {
        run.set_theme(chosen_theme);
    }
    // GEARMASTER_SCENE=<canonical monster> opens on that creature's scene, so
    // it can be read without playing fifteen rungs to earn it.
    if let Ok(who) = std::env::var("GEARMASTER_SCENE") {
        run.pending_scene = run.theme.cutscene(&who);
    }
    if let Ok(m) = std::env::var("GEARMASTER_MODE") {
        run.mode = if m.eq_ignore_ascii_case("rogue") { Mode::Rogue } else { Mode::Grinder };
    }
    let mut message =
        String::from("Drag components into a slot. Pieces must touch to become gear.");

    // Debug hooks so this window can be inspected without a human at the
    // keyboard: GEARMASTER_PRESET=1 starts geared up, GEARMASTER_FIGHT=1 opens
    // mid-bout, and GEARMASTER_SHOT=<path> captures a frame and exits.
    // GEARMASTER_WIN=1 wears the board that cleared the game - the only way to
    // get a slot full enough to see what a crowded strip does to the screen.
    if std::env::var("GEARMASTER_WIN").is_ok() {
        if let Some(sh) = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN) {
            run.loadout.grow(sh.extra_rows);
            for (def, slot, x, y, rot) in &sh.placed {
                let id = run.registry.alloc(*def);
                run.owned.push(id);
                run.registry.set_rotation(id, *rot);
                if run.equip(id, *slot, *x, *y).is_err() {
                    run.owned.pop();
                }
            }
        }
        message = "Wearing the winning board.".to_string();
    }
    // GEARMASTER_CLASSES=<n> hands over the first n classes in the book. The
    // panel's class band is the one place a list can push another list off the
    // screen, and no ordinary run reaches enough of them to see it happen.
    if let Ok(n) = std::env::var("GEARMASTER_CLASSES").unwrap_or_default().parse::<usize>() {
        for c in gearmaster_engine::class::CLASSES.iter().take(n) {
            run.classes.push(c);
        }
        run.refresh_class_effects();
        message = format!("Wearing {} classes.", run.classes.len());
    }
    // GEARMASTER_EVENT=<id> stands you in front of one, with whatever it
    // wants to open its doors. Reaching most of them by playing takes twenty
    // rungs and a specific thing done on one of them.
    if let Ok(id) = std::env::var("GEARMASTER_EVENT") {
        if let Some(ev) = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == id) {
            run.rung = ev.at;
            // Only the stopwatch this door reads. Two events share rung eight
            // and setting both hands the casino every time.
            match ev.trigger {
                gearmaster_engine::event::Trigger::QuickKill { .. } => {
                    run.best_fight_ms = Some(1)
                }
                gearmaster_engine::event::Trigger::SlowKill { .. } => {
                    run.worst_fight_ms = Some(600_000)
                }
                _ => {}
            }
            for name in ["Platinum Chip", "A Word About the Crownwright", "A Word About the Green Ledger"] {
                if let Some(d) = gearmaster_engine::piece::CATALOG.iter().position(|d| d.name == name) {
                    let pid = run.registry.alloc(d);
                    run.owned.push(pid);
                }
            }
            message = format!("Standing in front of {}.", ev.id);
        }
    }
    // GEARMASTER_TOWN=<n> stands you at the nth town's gate. Getting there by
    // playing means winning six fights first, which is not a way to look at a
    // screen.
    if let Ok(n) = std::env::var("GEARMASTER_TOWN").unwrap_or_default().parse::<usize>() {
        if let Some(t) = gearmaster_engine::town::TOWNS.get(n) {
            run.rung = t.after + 1;
            run.town = Some(t);
            run.last_bounty = 42;
            message = format!("At the gate of {}.", t.name);
        }
    }
    // GEARMASTER_GIVE=Name[,Name] drops loose pieces in the tray, for looking
    // at anything that trades one.
    if let Ok(v) = std::env::var("GEARMASTER_GIVE") {
        for name in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(d) =
                gearmaster_engine::piece::CATALOG.iter().position(|d| d.name == name)
            {
                let id = run.registry.alloc(d);
                run.owned.push(id);
            }
        }
    }
    if std::env::var("GEARMASTER_PRESET").is_ok() {
        run.apply_preset();
        message = "Auto-built a complete loadout - every bonus is lit.".to_string();
    }
    if let Ok(r) = std::env::var("GEARMASTER_RUNG") {
        if let Ok(n) = r.parse::<usize>() {
            run.rung = n;
        }
    }
    // GEARMASTER_CHIP=1 puts a Platinum Chip in the tray, which is what opens
    // the door at rung thirty.
    if std::env::var("GEARMASTER_CHIP").is_ok() {
        if let Some(d) =
            gearmaster_engine::piece::CATALOG.iter().position(|d| d.name == "Platinum Chip")
        {
            let id = run.registry.alloc(d);
            run.owned.push(id);
        }
    }
    // GEARMASTER_ROWS=<n> gives the boards n extra rows, for looking at what
    // a grown board does to the layout.
    if let Some(n) = std::env::var("GEARMASTER_ROWS").ok().and_then(|v| v.parse::<u8>().ok()) {
        run.grow_boards(n);
    }
    // GEARMASTER_QUICK=<ms> pretends the run has already won a fight that
    // fast, which is what earns the casino.
    if let Some(ms) = std::env::var("GEARMASTER_QUICK").ok().and_then(|v| v.parse().ok()) {
        run.best_fight_ms = Some(ms);
    }
    // GEARMASTER_BRAWL=<n> starts the fight against n creatures at once, so
    // the two-board layout can be looked at before an event exists that sets
    // one up.
    if let Some(n) = std::env::var("GEARMASTER_BRAWL").ok().and_then(|v| v.parse::<usize>().ok()) {
        let here = run.rung.min(LADDER.len() - 1);
        let specs: Vec<_> =
            (0..n.max(1)).map(|k| LADDER[(here + k * 3) % LADDER.len()]).collect();
        pb = Some({
            let profiles = run.combat_items();
            Playback::new(run.fight_party(&specs), &profiles, playback_speed)
        });
        settled = false;
    } else if std::env::var("GEARMASTER_FIGHT").is_ok() {
        pb = begin_next_fight(&mut run, playback_speed);
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
        // Bumped at the top: the loop returns early for every overlay screen,
        // so a counter at the bottom stops moving whenever one is open.
        FRAME.fetch_add(1, Ordering::Relaxed);
        // Clear the whole window (letterbox bars included), then switch into
        // logical space for everything that follows.
        set_default_camera();
        clear_background(Color::from_rgba(8, 8, 12, 255));
        let viewport = Viewport::current();
        set_camera(&viewport.camera());
        clear_background(col_bg());

        let (mx, my) = viewport.mouse();
        // Everything drawn this frame speaks the run's theme.
        words::set(run.theme);
        let reports = run.reports();
        // The fullest single slot, not the total: each slot lists its own.
        let worn_count: usize = reports.iter().map(|r| r.assembled_count()).max().unwrap_or(0);
        let layout = Layout::build(&run, worn_count);
        // What is actually finished and worn, for the strip under the boards.
        // Only needed out of combat; during a fight the battle screen has its
        // own copy from the log.
        let worn: Vec<ItemProfile> =
            if run.phase == Phase::Loadout { run.combat_items() } else { Vec::new() };

        if let Some(p) = pb.as_mut() {
            p.advance(&run);
            if p.done && !settled {
                settled = true;
                let gold = run.settle();
                message = match (gold, run.last_settlement.clone()) {
                    (Some(g), Some(st)) if st.outcome == Outcome::Victory => {
                        format!(
                            "+{} {}. Next up: {}.",
                            g,
                            words::word("gold-lower", "gold"),
                            words::monster(run.monster().name)
                        )
                    }
                    (Some(g), Some(st)) if st.run_ended => format!(
                        "+{} gold, but that was your last life. Everything is gone - starting over.",
                        g
                    ),
                    (Some(g), Some(st)) if st.knocked_back => format!(
                        "+{} gold, but it still stands. Knocked back to {}.",
                        g,
                        run.monster().name
                    ),
                    (Some(g), Some(st)) => match st.lives_left {
                        Some(n) => format!("+{} gold. {} still stands - {} lives left.", g, run.monster().name, n),
                        None => format!("+{} gold. {} still stands.", g, run.monster().name),
                    },
                    _ => format!("{} still stands.", run.monster().name),
                };
                if let Some(st) = run.last_settlement.as_ref() {
                    // A dungeon landing takes the screen, the way a scene does.
                    if let Some(line) = st.landing {
                        let won = st.class_won;
                        run.pending_scene = Some(Box::leak(
                            match won {
                                Some(c) => vec![
                                    words::retell(line),
                                    format!(
                                        "You come back up holding a share of it. They call that {}.",
                                        words::class(c)
                                    ),
                                ],
                                None => vec![words::retell(line)],
                            }
                            .into_iter()
                            .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                        ));
                    }
                    // What a fight an event arranged was worth. Losing one of
                    // those is worth saying too, because it costs nothing and
                    // a player who does not know that will not risk it again.
                    if let Some(won) = st.won_item {
                        message = format!(
                            "Thrown out on your ear, holding the {}.",
                            words::piece(won)
                        );
                    } else if st.reward == 0
                        && st.outcome != Outcome::Victory
                        && st.quests_done.is_empty()
                        && st.dropped.is_none()
                    {
                        message =
                            "Thrown out on your ear. It cost you nothing but the chip.".to_string();
                    }
                    // Taking something off a named creature is the best news
                    // there is: it is gear no shop will ever stock.
                    if let Some(drop) = st.dropped {
                        message = format!(
                            "You take the {} off it. Nothing sells one.",
                            words::piece(drop)
                        );
                    }
                    // A transformation is the more interesting news, so it
                    // takes the line.
                    if let Some(q) = st.quests_done.first() {
                        message = format!(
                            "{} finished its quest and is now {}. Find it in your inventory.",
                            q.from, q.into
                        );
                    }
                }
            }
        }

        // The opening runs before the game proper and swallows the frame.
        if opening != Opening::Playing {
            let clicked = is_mouse_button_pressed(MouseButton::Left);
            match opening {
                Opening::Intro(page) => {
                    let (back, next) = render_intro(page, mx, my);
                    if clicked && page > 0 && back.contains(Vec2::new(mx, my)) {
                        opening = Opening::Intro(page - 1);
                    } else if clicked && next.contains(Vec2::new(mx, my)) {
                        opening = if page + 1 == INTRO.len() {
                            Opening::ModeSelect
                        } else {
                            Opening::Intro(page + 1)
                        };
                    }
                    if is_key_pressed(KeyCode::Escape) {
                        opening = Opening::ModeSelect;
                    }
                }
                Opening::ModeSelect => {
                    let (modes, difficulties, themes) =
                        render_mode_select(chosen_difficulty, chosen_theme, mx, my);
                    for (d, rect) in difficulties {
                        if clicked && rect.contains(Vec2::new(mx, my)) {
                            chosen_difficulty = d;
                        }
                    }
                    for (t, rect) in themes {
                        if clicked && rect.contains(Vec2::new(mx, my)) {
                            chosen_theme = t;
                        }
                    }
                    for (mode, rect) in modes {
                        if clicked && rect.contains(Vec2::new(mx, my)) {
                            // The shop's rolls come from the clock, so two
                            // runs started at different moments stock
                            // differently. Tests still pin their own seeds.
                            run = Run::start_themed(
                                clock_seed(),
                                mode,
                                chosen_difficulty,
                                chosen_theme,
                            );
                            message = format!(
                                "{} run, {} ({}). Drag components into a slot - they must touch to become gear.",
                                mode.name(),
                                chosen_difficulty.name(),
                                chosen_difficulty.label()
                            );
                            opening = Opening::Story;
                        }
                    }
                }
                Opening::Story => {
                    let go = render_story(run.theme, mx, my);
                    if clicked && go.contains(Vec2::new(mx, my)) {
                        opening = Opening::Playing;
                    }
                }
                Opening::Playing => {}
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                frame += 1;
                if let Some(path) = &shot_path {
                    if frame >= shot_after {
                        get_screen_data().export_png(path);
                        println!("screenshot: {}", path);
                        return;
                    }
                }
            }
            next_frame().await;
            continue;
        }

        // ---------------------------------------------------- render
        // Tooltips requested while drawing; painted after everything else.
        let mut hover = Hover::default();
        // Set when a click lands on a pinned list, so the handlers further
        // down do not also read it as a click on the board behind it.
        let mut click_taken = false;
        if run.phase == Phase::Fighting {
            if let Some(p) = pb.as_ref() {
                // The wheel scrolls the log back. With the full log open that
                // is the only thing on screen, so the whole window scrolls;
                // with just the inline strip, only the strip does, or the
                // wheel would fight the shop and the tray behind it.
                //
                // The keys are here because a trackpad's wheel delta arrives
                // as a fraction on some machines and a scroll that needs a
                // flick of exactly the right size is a scroll that does not
                // work.
                {
                    let g = battle_geom(p.done);
                    let over = log_expanded || g.log.contains(Vec2::new(mx, my));
                    let (_, wheel) = mouse_wheel();
                    let mut step = 0isize;
                    if over && wheel != 0.0 {
                        step += if wheel > 0.0 { 3 } else { -3 };
                    }
                    if is_key_pressed(KeyCode::Up) {
                        step += 3;
                    }
                    if is_key_pressed(KeyCode::Down) {
                        step -= 3;
                    }
                    if is_key_pressed(KeyCode::PageUp) {
                        step += 12;
                    }
                    if is_key_pressed(KeyCode::PageDown) {
                        step -= 12;
                    }
                    if is_key_pressed(KeyCode::End) {
                        log_scroll = 0;
                    }
                    if step != 0 {
                        log_scroll = (log_scroll as isize + step).max(0) as usize;
                    }
                }
                render_battle(&run, p, log_expanded, log_scroll, &mut hover, mx, my);
            }
        } else {
            render_slots(&layout, &run, &reports, &drag, &mut hover, mx, my);
            render_slot_items(&layout, &run, &reports, &worn, &mut hover, mx, my);
            render_shop(&layout, &run, mx, my);
            render_inventory(&layout, &run, &drag, bartering, mx, my);
        }

        // Drag ghost + placement preview.
        if let Drag::Held { pieces, grab, .. } = &drag {
            let gx = mx - grab.0;
            let gy = my - grab.1;
            let hit = layout.slot_hit(gx + SLOT_CELL * 0.5, gy + SLOT_CELL * 0.5);

            if let Some((kind, ax, ay)) = hit {
                // A locked item lands whole or not at all, so the preview is
                // green only when every one of its pieces has somewhere to go.
                let ok = pieces.iter().all(|&(p, dx, dy)| {
                    let (x, y) = (ax as u32 + dx as u32, ay as u32 + dy as u32);
                    x < SLOT_W as u32
                        && y < run.loadout.rows() as u32
                        && run.can_equip(p, kind, x as u8, y as u8).is_ok()
                });
                let view = layout.view(kind);
                let tint = if ok { col_ok() } else { col_bad() };
                // Show the footprint the drop would claim, clipped to the grid.
                for &(p, ox, oy) in pieces {
                    for &(dx, dy) in run.registry.shape(p).cells() {
                        let cx = ax as i32 + ox as i32 + dx as i32;
                        let cy = ay as i32 + oy as i32 + dy as i32;
                        if (0..SLOT_W as i32).contains(&cx) && (0..SLOT_H as i32).contains(&cy) {
                            let (px, py) = view.cell_origin(cx as u8, cy as u8);
                            draw_rectangle(px, py, SLOT_CELL, SLOT_CELL, with_alpha(tint, 0.38));
                        }
                    }
                }
            }
            for &(p, ox, oy) in pieces {
                let def = run.registry.def(p);
                // A shared piece on the cursor is grey until it is over a grid
                // that will take it, and takes that grid's colour and mark as
                // it crosses in - which shows the rule without anywhere having
                // to state it.
                let over = hit.map(|(k, _, _)| k).filter(|k| def.fits(*k));
                draw_shape(
                    &run.registry.shape(p),
                    gx + ox as f32 * SLOT_CELL,
                    gy + oy as f32 * SLOT_CELL,
                    SLOT_CELL,
                    def,
                    over,
                    0.92,
                );
            }
        }

        if run.phase != Phase::Fighting {
            render_panel(
                &layout,
                &run,
                &reports,
                &message,
                playback_speed,
                &mut hover,
                mx,
                my,
            );
        }

        // Tooltip for whatever is under the cursor (never while dragging).
        // A request left by a render pass wins: those regions are the panel
        // and the strips around each board, which nothing else claims.
        // A held-open overflow list, and the sub-card for whatever row you
        // point at inside it. Drawn before the other tooltips and instead of
        // them: it is the thing you are reading.
        if let Some(pin) = pinned.clone() {
            let r = render_pinned(&pin, &run, true, mx, my);
            if left_pressed() {
                click_taken = true;
                if !r.contains(Vec2::new(mx, my)) {
                    pinned = None;
                }
            }
            if is_key_pressed(KeyCode::Escape) {
                pinned = None;
            }
        } else if let Some(pin) = hover.overflow.take() {
            if matches!(drag, Drag::None) {
                render_pinned(&pin, &run, false, mx, my);
                if left_pressed() {
                    click_taken = true;
                    pinned = Some(pin);
                }
            }
        } else if matches!(drag, Drag::None) && hover.enemy_card {
            // Left of the side panel, so the panel it was opened from stays
            // uncovered - a preview that hides its own hover target flickers.
            let spec = *run.monster();
            let w = layout.panel_x - 48.0;
            render_enemy_preview(Rect::new(24.0, 40.0, w, 0.0), &spec, run.difficulty, run.rung);
        } else if matches!(drag, Drag::None) && hover.class_card {
            render_class_card(&run, mx, my);
        } else if let (Drag::None, Some(tip)) = (&drag, hover.tip.take()) {
            draw_tip(&tip, mx, my);
        } else if matches!(drag, Drag::None) {
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

        // GEARMASTER_BARS=1 draws the health and armour pair at a spread of
        // values. Armour past a full bar wraps into darker layers, and a real
        // fight almost never gets there - the preset peaks at an eighth of a
        // bar - so without this the layering is a thing you can write and
        // never once look at.
        if std::env::var("GEARMASTER_BARS").is_ok() {
            draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(14, 14, 22, 255));
            let max = 400;
            let gutter = 300.0;
            let w = LOGICAL_W - gutter - 40.0;
            for (i, armor) in [0, 60, 200, 400, 520, 800, 1000, 1600, 2400].iter().enumerate() {
                let y = 60.0 + i as f32 * 96.0;
                ui_text(&format!("{} armour", armor), 24.0, y + 4.0, 16.0, LIGHTGRAY);
                ui_text(&format!("of {} health", max), 24.0, y + 24.0, 13.0, col_dim());
                ui_text(
                    &format!("{:.2} bars", *armor as f32 / max as f32),
                    24.0,
                    y + 44.0,
                    13.0,
                    col_gold(),
                );
                hp_bar(gutter, y - 22.0, w, 30.0, max, max, col_ok());
                armor_bar(gutter, y + 10.0, w, 20.0, *armor, max);
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
            continue;
        }

        // GEARMASTER_SPRITES=1 lays every creature out in a grid. Sprites are
        // the one thing in here that cannot be checked by a test - "does a
        // Toad read differently from a Wisp" is a question about eyes - so
        // there has to be a way to put them all on one screen and look.
        if std::env::var("GEARMASTER_SPRITES").is_ok() {
            draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(14, 14, 22, 255));
            let per_row = 10usize;
            let cell = 148.0;
            for (i, m) in LADDER.iter().enumerate() {
                let cx = 20.0 + (i % per_row) as f32 * cell;
                let cy = 24.0 + (i / per_row) as f32 * (cell + 10.0);
                draw_monster(cx, cy, cell * 0.62, m.sprite, col_foe(), Color::from_rgba(14, 14, 22, 255));
                let label = m.name;
                let size = fitting_size(label, cell - 8.0, &[12.0, 11.0, 10.0, 9.0]);
                draw_capped(label, cx, cy + cell * 0.72, cell - 8.0, size, LIGHTGRAY, 1);
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
            continue;
        }

        // A scene the theme owes for the last fight comes before anything
        // else: it is the reason the next rung looks the way it does.
        if let Some(scene) = run.pending_scene {
            let go = render_scene(scene, mx, my);
            if is_mouse_button_pressed(MouseButton::Left) && go.contains(Vec2::new(mx, my)) {
                run.pending_scene = None;
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
            continue;
        }

        // The tools drawer sits over everything, like the fountain and the
        // glossary do.
        if tools_open {
            let code = gearmaster_engine::share::export(&run);
            match render_tools(&run, &code, imported.as_ref(), mx, my) {
                Some("close") => tools_open = false,
                Some("copy") => {
                    miniquad::window::clipboard_set(&code);
                    message = "Run code copied. Paste it to a friend.".into();
                }
                Some("paste") => {
                    let got = miniquad::window::clipboard_get().unwrap_or_default();
                    match gearmaster_engine::share::import(&got) {
                        Some(sh) => {
                            message = format!("Read a run from rung {}.", sh.rung + 1);
                            imported = Some(sh);
                        }
                        None => message = "That is not a run code.".into(),
                    }
                }
                Some("shot") => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let path =
                            format!("/tmp/gearmaster-{}.png", (get_time() * 1000.0) as u64);
                        get_screen_data().export_png(&path);
                        message = format!("Saved {}", path);
                    }
                }
                _ => {}
            }
            if is_key_pressed(KeyCode::Escape) {
                tools_open = false;
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
            continue;
        }

        // The town gate, ahead of the events: a town is a rung of its own and
        // the event on the next rung has not been arrived at yet.
        if let Some(t) = run.pending_town() {
            if let Some(pick) = render_town(&run, t, mx, my) {
                message = match pick {
                    None => {
                        let paid = run.skip_town();
                        format!("You keep walking. {} gold for the trouble.", paid)
                    }
                    Some(a) => {
                        let v = run.visit_town(a);
                        town_message(&run, &v)
                    }
                };
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
            continue;
        }

        // An event sits over everything while it is being answered, the same
        // way a fountain does.
        if let Some(ev) = run.pending_event() {
            if let Some(c) = render_event(&run, ev, mx, my) {
                let gave = run.take_choice(c);
                message = match gave {
                    Some(name) => format!(
                        "You hand over the {}. It counts it out twice.",
                        words::piece(name)
                    ),
                    None => format!("{}.", words::retell(c.label)),
                };
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
            continue;
        }

        // The fountain sits over everything while it is being answered.
        if fountain_open {
            if run.at_doubling_fountain() {
                if let Some(c) = render_doubling_fountain(&run, mx, my) {
                    let name = c.name;
                    if run.double_class(c) {
                        fountain_open = false;
                        message = format!(
                            "You drink, and there is twice as much {} in you.",
                            words::class(name)
                        );
                    }
                }
            } else if let Some(c) = render_fountain(&run, mx, my) {
                if let Some(taken) = run.drink_choosing(c) {
                    fountain_open = false;
                    message =
                        format!("You drink, and it names you {}.", words::class(taken.name));
                }
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
            continue;
        }

        // The ladder picker sits over everything and eats input, the same way
        // the glossary does.
        if picker_open {
            let (chosen, shut, page) = render_ladder_picker(&run, picker_page, mx, my);
            picker_page = page;
            if let Some(rung) = chosen {
                let name = LADDER[rung].name;
                message = match run.skip_to(rung) {
                    Some(gold) => {
                        picker_open = false;
                        format!("The road is behind you. {} gold, and {} ahead.", gold, name)
                    }
                    None => "That is not up the mountain.".to_string(),
                };
            }
            if shut || is_key_pressed(KeyCode::Escape) {
                picker_open = false;
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
            continue;
        }

        // The glossary sits over everything and eats input while open, so a
        // click meant for CLOSE never also lands on the board behind it.
        if glossary_open {
            let g = render_glossary(glossary_tab, glossary_page, mx, my);
            let click = is_mouse_button_pressed(MouseButton::Left);
            let tab_hit = g.tabs.iter().position(|t| t.contains(Vec2::new(mx, my)));
            if click && g.close.contains(Vec2::new(mx, my)) {
                glossary_open = false;
                glossary_page = 0;
            } else if click && tab_hit.is_some() && tab_hit != Some(glossary_tab) {
                // Each shelf keeps its own place, so switching back and forth
                // does not lose where you were.
                glossary_tab = tab_hit.unwrap();
                glossary_page = 0;
            } else if click && g.pages > 1 && g.next.contains(Vec2::new(mx, my)) {
                glossary_page = (glossary_page + 1) % g.pages;
            } else if click && g.skip.map_or(false, |r| r.contains(Vec2::new(mx, my))) {
                // The one entry that does something. It opens the ladder
                // rather than moving you: where to pick the road up is a
                // decision, and taking one rung at a time was not offering it.
                glossary_open = false;
                glossary_page = 0;
                picker_open = true;
                picker_page = 0;
            }
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::G) {
                glossary_open = false;
                glossary_page = 0;
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
            continue;
        }
        if is_key_pressed(KeyCode::G) {
            glossary_open = true;
        }

        // ----------------------------------------------------- input
        let rects = button_rects(layout.panel_x);
        let clicked_button = |i: usize| {
            is_mouse_button_pressed(MouseButton::Left) && rects[i].contains(Vec2::new(mx, my))
        };

        if run.phase == Phase::Fighting {
            let done = pb.as_ref().map(|p| p.done).unwrap_or(false);
            let g = battle_geom(done);
            let br = g.buttons;
            let hit = |i: usize| {
                is_mouse_button_pressed(MouseButton::Left)
                    && br[i].contains(Vec2::new(mx, my))
            };
            // Once the fight is over the big primary is the way out; before
            // then it is the ordinary first button. Only one exists at a time.
            let leave = if done {
                is_mouse_button_pressed(MouseButton::Left)
                    && g.primary.contains(Vec2::new(mx, my))
            } else {
                hit(0)
            };
            if leave {
                run.back_to_loadout();
                pb = None;
                log_expanded = false;
                message = "Rearrange your gear and fight again.".to_string();
            } else if hit(1) {
                if let Some(p) = pb.as_mut() {
                    p.skip_to_end(&run);
                }
            } else if hit(2) {
                // Straight into the next fight without going back to the
                // loadout - which is where the town gate, the events and the
                // fountain are all drawn. Anything waiting there gets looked
                // at first.
                match begin_next_fight(&mut run, playback_speed) {
                    Some(next) => {
                        pb = Some(next);
                        // A new fight follows itself again.
                        log_scroll = 0;
                        settled = false;
                    }
                    None => {
                        let what = run.road_is_blocked().unwrap_or("something");
                        run.back_to_loadout();
                        pb = None;
                        log_expanded = false;
                        message = format!("{} is standing in the road.", capitalise(what));
                    }
                }
            } else if hit(3) {
                log_expanded = !log_expanded;
            } else if hit(4) {
                playback_speed = next_speed(playback_speed);
                if let Some(p) = pb.as_mut() {
                    p.speed = playback_speed;
                }
                message = format!("Playback at {}x.", speed_label(playback_speed));
            }
        } else {
            if clicked_button(0) && (run.at_fountain() || run.at_doubling_fountain()) {
                // Not a fight, and not automatic: the fountain offers, and the
                // choosing is yours.
                fountain_open = true;
            } else if clicked_button(0) {
                match begin_next_fight(&mut run, playback_speed) {
                    Some(next) => {
                        pb = Some(next);
                        // A new fight follows itself again.
                        log_scroll = 0;
                        settled = false;
                        message = "Fight in progress.".to_string();
                    }
                    None => {
                        let what = run.road_is_blocked().unwrap_or("something");
                        message = format!("{} is standing in the road.", capitalise(what));
                    }
                }
            } else if clicked_button(1) {
                playback_speed = next_speed(playback_speed);
                message = format!("Fights will replay at {}x.", speed_label(playback_speed));
            } else if clicked_button(2) {
                drag = Drag::None;
                message = match run.undo() {
                    Some(what) => format!("Undid {}.", what),
                    None => "Nothing to undo.".to_string(),
                };
            } else if clicked_button(3) {
                run.clear_all();
                drag = Drag::None;
                message = "Cleared. Every slot is empty again. UNDO puts it back.".to_string();
            } else if clicked_button(4) {
                glossary_open = true;
            } else if clicked_button(5) {
                tools_open = true;
            }
        }

        // Buying, checked before the drag handler so clicking a shelf never
        // picks a piece up.
        let mut bought_this_frame = click_taken;
        if run.phase == Phase::Loadout && left_pressed() && !click_taken {
            if reroll_rect(layout.shop).contains(Vec2::new(mx, my)) {
                bought_this_frame = true;
                match run.reroll() {
                    Ok(()) => message = format!("Rerolled. {} gold left.", run.gold),
                    Err(e) => message = format!("{}", e),
                }
            } else if let Some(i) = layout.shop_hit(mx, my) {
                bought_this_frame = true;
                let name = run.shop.def(i).map(|d| d.name).unwrap_or("?");
                if run.trophy_shelf(i) {
                    bought_this_frame = true;
                    if run.payment_for(i).is_empty() {
                        bartering = None;
                        message =
                            "They want something you took off a named creature, and you have \
                             not got one loose."
                                .to_string();
                    } else {
                        bartering = Some(i);
                        message =
                            "Now hand over a trophy. Anything they will take is lit up below."
                                .to_string();
                    }
                } else {
                match run.rumour_on(i) {
                    // Not for sale. Pick it up and then hand something over.
                    Some(word) => {
                        if run.payment_for(i).is_empty() {
                            bartering = None;
                            message = format!(
                                "They want {} for that, and you have not got one loose.",
                                words::retell(&word.price.label())
                            );
                        } else {
                            bartering = Some(i);
                            message = format!(
                                "Now hand over {}. Anything they will take is lit up below.",
                                words::retell(&word.price.label())
                            );
                        }
                    }
                    None => match run.buy(i) {
                        Ok(_) => message = format!("Bought {}. {} gold left.", name, run.gold),
                        Err(e) => message = format!("{}", e),
                    },
                }
                }
            } else if let Some((slot, id)) = bartering
                .and_then(|slot| layout.cards.iter().find(|c| c.rect.contains(Vec2::new(mx, my))).map(|c| (slot, c.id)))
            {
                // The second half of a barter. Ahead of the sell badge and the
                // drag handler: while a trade is open, a click on a card in the
                // tray is an answer to it and nothing else.
                bought_this_frame = true;
                let paying = run.registry.def(id).name;
                let taking = run.shop.def(slot).map(|d| d.name).unwrap_or("?");
                let trophy = run.trophy_shelf(slot);
                match run.barter(slot, id) {
                    Ok(_) => {
                        bartering = None;
                        message = if trophy {
                            format!(
                                "The {} goes behind the bar. {} x{} - every adjacency bonus \
                                 you own counts {}% more.",
                                words::piece(paying),
                                words::class("Recycler"),
                                run.stacks_of("Recycler"),
                                run.loadout.adjacency_pct
                            )
                        } else {
                            format!(
                                "You hand over the {}. They tell you about {}.",
                                words::piece(paying),
                                words::piece(taking)
                            )
                        };
                    }
                    Err(_) => {
                        message = format!(
                            "They do not want the {}.",
                            words::piece(paying)
                        );
                    }
                }
            } else if let Some(id) = sell_hit(&layout, mx, my) {
                bought_this_frame = true;
                let name = run.registry.def(id).name;
                match run.sell(id) {
                    Ok(refund) => {
                        message = format!("Sold {} for {} gold. {} in hand.", name, refund, run.gold)
                    }
                    Err(e) => message = format!("{}", e),
                }
            }
        }

        // Drag and drop is only live while arranging gear.
        if run.phase == Phase::Loadout {
            let over_button =
                bought_this_frame || rects.iter().any(|r| r.contains(Vec2::new(mx, my)));

            // --- lock (shift-click) ---
            // Ahead of the pick-up, and it swallows the click: otherwise the
            // drag handler takes the piece on the same press and the item ends
            // up stuck to the cursor instead of locked.
            let shift =
                is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            if shift && is_mouse_button_pressed(MouseButton::Left) && !over_button {
                if let Some(id) =
                    layout.slot_hit(mx, my).and_then(|(k, x, y)| run.loadout.slot(k).get(x, y))
                {
                    let was_locked = run.is_locked_item(id);
                    if run.toggle_lock_item(id) {
                        message = "Locked. It turns and travels as one piece now.".to_string();
                    } else if was_locked {
                        message = "Released. Its pieces can be rearranged again.".to_string();
                    } else {
                        // Nothing happened, and silence is what "it does not
                        // work" looks like from the other side of the screen.
                        message = format!(
                            "{} is not part of a finished item yet - only assembled gear locks.",
                            run.registry.def(id).name
                        );
                    }
                }
            }

            // --- pick up ---
            if is_mouse_button_pressed(MouseButton::Left)
                && matches!(drag, Drag::None)
                && !over_button
                && !shift
            {
                if let Some((kind, gx, gy)) = layout.slot_hit(mx, my) {
                    if let Some(id) = run.loadout.slot(kind).get(gx, gy) {
                        // A locked item comes up whole. Taking one piece out of
                        // it is exactly what locking is meant to prevent, and
                        // lifting it all at once is what lets it be carried to
                        // the inventory in one go.
                        let set = run.locked_set(id).map(|s| s.to_vec());
                        let (pieces, anchor) = match &set {
                            Some(set) => {
                                let slot = run.loadout.slot(kind);
                                let anchors: Vec<(PieceId, u8, u8)> = set
                                    .iter()
                                    .map(|&p| {
                                        let a = slot.anchor_of(p).unwrap_or((0, 0));
                                        (p, a.0, a.1)
                                    })
                                    .collect();
                                let minx = anchors.iter().map(|(_, x, _)| *x).min().unwrap_or(0);
                                let miny = anchors.iter().map(|(_, _, y)| *y).min().unwrap_or(0);
                                (
                                    anchors
                                        .iter()
                                        .map(|&(p, x, y)| (p, x - minx, y - miny))
                                        .collect::<Vec<_>>(),
                                    (minx, miny),
                                )
                            }
                            None => {
                                let a = run
                                    .loadout
                                    .slot(kind)
                                    .anchor_of(id)
                                    .expect("a placed piece has an anchor");
                                (vec![(id, 0, 0)], a)
                            }
                        };
                        let (ox, oy) = layout.view(kind).cell_origin(anchor.0, anchor.1);
                        // Lift it out now, so the piece can't collide with its
                        // own old footprint and a rotation mid-drag is free.
                        if set.is_some() {
                            let _ = run.unequip_locked(id);
                        } else {
                            let _ = run.unequip(id);
                        }
                        drag = Drag::Held {
                            pieces,
                            grab: (mx - ox, my - oy),
                            restore: Some((kind, anchor.0, anchor.1)),
                        };
                    }
                } else if let Some(id) = layout.card_hit(mx, my) {
                    // A locked item in the tray is one card and comes back out
                    // in the shape it went in with.
                    let pieces = run
                        .locked_shape(id)
                        .unwrap_or_else(|| vec![(id, 0, 0)]);
                    let (w, h) = group_cells(&run, &pieces);
                    drag = Drag::Held {
                        pieces,
                        // Centre it on the cursor: it was drawn at a smaller
                        // scale, so there is no grab point to keep.
                        grab: (w as f32 * SLOT_CELL / 2.0, h as f32 * SLOT_CELL / 2.0),
                        restore: None,
                    };
                }
            }

            // --- rotate (right-click, held or in place) ---
            if is_mouse_button_pressed(MouseButton::Right) {
                if let Some(i) = layout.shop_hit(mx, my) {
                    let name = run.shop.def(i).map(|d| d.name).unwrap_or("that");
                    message = if run.shop.toggle_lock(i) {
                        format!("{} is pinned. A reroll will leave it there.", name)
                    } else {
                        format!("{} is loose again.", name)
                    };
                    next_frame().await;
                    continue;
                }
                if let Some(id) = layout
                    .slot_hit(mx, my)
                    .and_then(|(k, x, y)| run.loadout.slot(k).get(x, y))
                    .filter(|&id| run.is_locked_item(id))
                {
                    message = match run.rotate_locked(id) {
                        Ok(()) => "Turned the whole item.".to_string(),
                        Err(e) => format!("{}", e),
                    };
                    next_frame().await;
                    continue;
                }
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
                if let Drag::Held { pieces, grab, restore } = drag {
                    let gx = mx - grab.0 + SLOT_CELL * 0.5;
                    let gy = my - grab.1 + SLOT_CELL * 0.5;
                    let id = pieces[0].0;
                    let locked = pieces.len() > 1;

                    let placed = match layout.slot_hit(gx, gy) {
                        Some((kind, ax, ay)) => {
                            // A locked item goes down all at once, so a drop
                            // that will not fit leaves the board untouched
                            // rather than scattering it.
                            let r = if locked {
                                run.equip_locked_at(id, kind, ax, ay)
                            } else {
                                run.equip(id, kind, ax, ay)
                            };
                            match r {
                                Ok(()) => {
                                    let r = run.report(kind);
                                    message = format!(
                                        "{}: {}  {}",
                                        kind.name(),
                                        r.summary(),
                                        words::retell(&r.stats.summary())
                                    );
                                    true
                                }
                                Err(e) => {
                                    message = format!("{}", e);
                                    false
                                }
                            }
                        }
                        None => false,
                    };

                    if !placed {
                        // Dropped on the tray? Then leaving it unequipped IS
                        // the intent. Anywhere else, put it back where it was.
                        //
                        // The held piece is already counted as loose - it was
                        // lifted off the board to be dragged - so the tray is
                        // over its limit rather than at it when there is no
                        // room for this one.
                        let over = run.inventory().len() > gearmaster_engine::run::INVENTORY_CAP;
                        let on_tray = layout.inv.contains(Vec2::new(mx, my)) && !over;
                        if layout.inv.contains(Vec2::new(mx, my)) && over {
                            message = format!(
                                "The tray only holds {} pieces. Wear something or sell something.",
                                gearmaster_engine::run::INVENTORY_CAP
                            );
                        }
                        if on_tray {
                            message = if locked {
                                "Item stowed. It stays locked, and goes back down as one piece."
                                    .to_string()
                            } else {
                                format!("{} returned to inventory.", run.registry.def(id).name)
                            };
                        } else if let Some((kind, ax, ay)) = restore {
                            let _ = if locked {
                                run.equip_locked_at(id, kind, ax, ay)
                            } else {
                                run.equip(id, kind, ax, ay)
                            };
                        }
                    }
                    drag = Drag::None;
                }
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            if let Drag::Held { pieces, restore, .. } = drag {
                if let Some((kind, ax, ay)) = restore {
                    let _ = if pieces.len() > 1 {
                        run.equip_locked_at(pieces[0].0, kind, ax, ay)
                    } else {
                        run.equip(pieces[0].0, kind, ax, ay)
                    };
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
mod glossary_tests {
    use super::*;

    /// Every term the glossary defines, in the plain words, upper-cased.
    fn terms() -> Vec<String> {
        GLOSSARY.iter().map(|(t, _)| t.trim().to_uppercase()).collect()
    }

    fn defines(word: &str) -> bool {
        let want = word.to_uppercase();
        terms().iter().any(|t| t.contains(&want))
    }

    #[test]
    fn a_class_no_build_points_at_says_where_to_find_it() {
        use gearmaster_engine::class::{how_you_get_it, is_earned, CLASSES};
        // A class with no requirements is either the floor - which a fountain
        // pours when a build matched nothing, and for which "asks for nothing"
        // is the literal truth - or it is earned somewhere, and then the shelf
        // has to say where.
        for c in CLASSES.iter().filter(|c| c.requires.is_empty() && is_earned(c.name)) {
            let how = how_you_get_it(c.name);
            assert!(
                how.is_some(),
                "{} asks for nothing and the shelf cannot say where it comes from",
                c.name
            );
            assert!(how.unwrap().len() > 10, "{}: {:?} explains nothing", c.name, how);
        }
        // And the reverse: nothing that a build *can* reach pretends to be
        // earned, or the fountain would be offering something it will not.
        for c in CLASSES.iter().filter(|c| !c.requires.is_empty()) {
            assert!(how_you_get_it(c.name).is_none(), "{} is both built and earned", c.name);
        }
    }

    #[test]
    fn every_class_in_the_game_is_on_the_classes_shelf() {
        // The shelf walks CLASSES, so this is really a check that nothing is
        // filtered out of it - an earned class is still a class you can be
        // holding, and a player looking one up does not know or care how they
        // came by it.
        use gearmaster_engine::class::CLASSES;
        assert!(CLASSES.len() > 20, "only {} classes; the list is not being read", CLASSES.len());
        for c in CLASSES {
            assert!(!c.blurb.is_empty(), "{} has no blurb, so its card is a heading", c.name);
            let power = c.power.describe();
            assert!(
                power.len() > 30,
                "{}'s power describes itself in {:?}, which tells nobody anything",
                c.name,
                power
            );
            assert!(
                !c.power.short().is_empty(),
                "{} has nothing to show in the side panel",
                c.name
            );
        }
    }

    #[test]
    fn a_class_that_stacks_says_so_somewhere() {
        use gearmaster_engine::class::{stacks, CLASSES};
        for c in CLASSES.iter().filter(|c| stacks(c.name)) {
            let text = format!("{} {}", c.blurb, c.power.describe()).to_lowercase();
            assert!(
                text.contains("stack"),
                "{} can be held several times over and never mentions it: {:?}",
                c.name,
                c.power.describe()
            );
        }
    }

    #[test]
    fn every_resource_is_defined() {
        use gearmaster_engine::piece::Resource;
        for r in Resource::ALL {
            assert!(defines(r.name()), "the glossary never says what {} is", r.name());
        }
    }

    #[test]
    fn every_curse_is_defined() {
        use gearmaster_engine::curse::CurseKind;
        for k in CurseKind::ALL {
            assert!(defines(k.name()), "the glossary never says what a curse of {} does", k.name());
        }
    }

    #[test]
    fn every_rule_that_can_end_a_fight_is_defined() {
        // The three ways a fight stops that are not "somebody ran out of
        // health": these are the ones a player meets without being told.
        for word in ["SUDDEN DEATH", "BOUNTY", "BRAWL"] {
            assert!(defines(word), "the glossary never explains {word}");
        }
    }

    #[test]
    fn every_thing_a_town_hands_you_is_defined() {
        for word in ["TOWN", "RUMOUR", "BARTER", "MANA DEBT"] {
            assert!(defines(word), "the glossary never explains {word}");
        }
    }

    #[test]
    fn no_term_is_defined_twice() {
        let mut seen = terms();
        let n = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), n, "the glossary defines something twice");
    }

    #[test]
    fn every_definition_is_worth_reading() {
        for (term, meaning) in GLOSSARY {
            assert!(!term.trim().is_empty(), "a nameless entry");
            assert!(
                meaning.len() > 20,
                "{term} is defined in {:?}, which is not a definition",
                meaning
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> SlotView {
        SlotView { kind: SlotKind::Weapon, origin: (137.5, 112.0), rows: SLOT_H }
    }

    #[test]
    fn every_cell_round_trips_from_grid_to_pixels_and_back() {
        let v = view();
        for gy in 0..v.rows {
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
        for gy in 0..v.rows {
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
        diagonal_items: Vec::new(),
        open_cells: 0,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
            sigil_seed: 0,
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

    // Measuring needs a graphics context, so these exercise the layout maths
    // through a stand-in: every glyph is 10 wide at size 10.
    fn fake_width(s: &str, size: f32) -> f32 {
        s.chars().count() as f32 * size
    }

    #[test]
    fn pixel_wrapping_breaks_on_the_measured_width() {
        let text = "the quick brown fox";
        let lines = wrap_measured(text, 100.0, &|s| fake_width(s, 10.0));

        for l in &lines {
            assert!(fake_width(l, 10.0) <= 100.0, "line too wide: {:?}", l);
        }
        assert_eq!(lines.join(" "), text, "and no word is lost");
    }

    #[test]
    fn a_word_wider_than_the_line_still_gets_its_own_line() {
        let lines = wrap_measured("a supercalifragilistic b", 40.0, &|s| fake_width(s, 10.0));
        assert_eq!(lines, vec!["a", "supercalifragilistic", "b"]);
    }

    #[test]
    fn a_label_takes_the_largest_size_that_fits() {
        let sizes = [20.0f32, 16.0, 12.0];
        // Eight characters: 160 wide at 20, 128 at 16, 96 at 12.
        let width_at = |sz: f32| fake_width("eightchr", sz);

        assert_eq!(largest_fitting(&sizes, 200.0, &width_at), 20.0);
        assert_eq!(largest_fitting(&sizes, 130.0, &width_at), 16.0);
        assert_eq!(largest_fitting(&sizes, 100.0, &width_at), 12.0);
        assert_eq!(
            largest_fitting(&sizes, 10.0, &width_at),
            12.0,
            "nothing fits, so it settles for the smallest rather than refusing to draw"
        );
    }

    // ------------------------------------------------------- accessibility
    //
    // A tile says two things - which slot, and which part of the recipe - and
    // has to say both to a player who sees no colour. Slot rides on the motif,
    // role rides on lightness. These pin down that neither collapses.

    #[test]
    fn cooldown_rows_tighten_before_they_spill() {
        // A deep boss fields far more gear than the band was drawn for. What
        // must never happen is rows running past the space they were given -
        // that is what put the two lists on top of each other.
        let avail = 300.0;
        for count in 1..40usize {
            let (pitch, shown) = cooldown_fit(count, avail);
            assert!(pitch >= 17.0 && pitch <= 28.0, "{} rows gave pitch {}", count, pitch);
            assert!(
                shown as f32 * pitch <= avail + 0.01,
                "{} rows at pitch {} need {} of {}",
                shown,
                pitch,
                shown as f32 * pitch,
                avail
            );
            assert!(shown <= count);
        }
    }

    #[test]
    fn a_short_list_keeps_its_full_spacing_and_shows_everything() {
        let (pitch, shown) = cooldown_fit(6, 300.0);
        assert_eq!(pitch, 28.0);
        assert_eq!(shown, 6);
    }

    #[test]
    fn a_list_that_cannot_fit_leaves_a_row_to_say_so() {
        // 30 rows cannot fit 300px even at the floor, so some are summarised -
        // and a row is left spare for the "+ N more" line.
        let (pitch, shown) = cooldown_fit(30, 300.0);
        assert_eq!(pitch, 17.0);
        assert!(shown < 30);
        assert!((shown + 1) as f32 * pitch <= 300.0 + 0.01, "no room left for the tail line");
    }

    #[test]
    fn every_slot_has_its_own_motif() {
        let motifs: Vec<Motif> = SlotKind::ALL.iter().map(|&s| slot_motif(s)).collect();
        for (i, a) in motifs.iter().enumerate() {
            for b in &motifs[i + 1..] {
                assert_ne!(a, b, "two slots share the motif {:?}", a);
            }
        }
        assert_eq!(motifs.len(), 5);
    }

    #[test]
    fn the_three_roles_stay_apart_in_greyscale() {
        // The role is only legible without colour if the lightness steps
        // survive being flattened to brightness, in every slot's hue.
        let roles = [PieceKind::Handle, PieceKind::Damaging, PieceKind::Accessory];
        for slot in SlotKind::ALL {
            let lums: Vec<f32> = roles
                .iter()
                .map(|&k| luminance(slot_color(slot, kind_luminance(k))))
                .collect();
            for w in lums.windows(2) {
                assert!(
                    w[1] - w[0] > 0.08,
                    "{:?}: roles only {:.3} apart in brightness ({:?})",
                    slot,
                    w[1] - w[0],
                    lums
                );
            }
        }
    }

    #[test]
    fn a_shared_piece_is_colourless_until_it_is_placed() {
        use gearmaster_engine::piece::CATALOG;
        let shared: Vec<&PieceDef> = CATALOG.iter().filter(|d| d.shared()).collect();
        assert!(!shared.is_empty(), "no shared pieces to check");

        for def in &shared {
            // Loose: no slot's colour and no slot's mark.
            let (c, m) = piece_look(def, None);
            assert_eq!(m, Motif::Shared, "{} loose should wear the shared mark", def.name);
            assert!(
                (c.r - c.g).abs() < 0.001 && (c.g - c.b).abs() < 0.001,
                "{} loose should be grey, got {:?}",
                def.name,
                c
            );

            // Placed: it takes the grid it is in, in every grid it can go.
            for slot in def.slots() {
                let (c, m) = piece_look(def, Some(slot));
                assert_eq!(m, slot_motif(slot), "{} in {:?}", def.name, slot);
                assert_eq!(c, slot_color(slot, kind_luminance(def.kind)), "{}", def.name);
            }
        }
    }

    #[test]
    fn a_piece_that_goes_one_place_looks_the_same_loose_or_placed() {
        // Only ambiguity gets greyed out. Everything else would just be losing
        // information for no reason.
        use gearmaster_engine::piece::CATALOG;
        for def in CATALOG.iter().filter(|d| !d.shared()) {
            assert_eq!(piece_look(def, None), piece_look(def, Some(def.slot)), "{}", def.name);
        }
    }

    #[test]
    fn the_shared_grey_keeps_the_roles_apart_in_greyscale() {
        let roles = [PieceKind::Handle, PieceKind::Damaging, PieceKind::Accessory];
        let lums: Vec<f32> = roles.iter().map(|&k| luminance(unplaced_color(k))).collect();
        for w in lums.windows(2) {
            assert!(w[1] - w[0] > 0.08, "shared greys only {:.3} apart ({:?})", w[1] - w[0], lums);
        }
    }

    #[test]
    fn the_shared_mark_is_not_one_of_the_slot_marks() {
        for slot in SlotKind::ALL {
            assert_ne!(slot_motif(slot), Motif::Shared);
        }
    }

    #[test]
    fn the_motif_ink_contrasts_with_every_tile_it_lands_on() {
        for slot in SlotKind::ALL {
            for &kind in &[PieceKind::Handle, PieceKind::Damaging, PieceKind::Accessory] {
                let fill = slot_color(slot, kind_luminance(kind));
                let ink = motif_ink(fill, 1.0);
                // The ink is drawn over the fill, so what reaches the eye is
                // the two composited by the ink's alpha.
                let mixed = Color::new(
                    fill.r + (ink.r - fill.r) * ink.a,
                    fill.g + (ink.g - fill.g) * ink.a,
                    fill.b + (ink.b - fill.b) * ink.a,
                    1.0,
                );
                let gap = (luminance(mixed) - luminance(fill)).abs();
                assert!(
                    gap > 0.06,
                    "{:?}/{:?}: motif only {:.3} from its tile",
                    slot,
                    kind,
                    gap
                );
            }
        }
    }
}

#[cfg(test)]
mod radar_tests {
    use super::*;

    /// The five boards span exactly the shop's width: the H of HELMET starts
    /// on its left edge and the weapon board ends on its right. Sized by hand,
    /// so this is what stops a change to the side panel quietly breaking it.
    #[test]
    fn the_boards_line_up_with_the_shop() {
        let b = bands(0, SLOT_H);
        let gw = SLOT_W as f32 * SLOT_CELL;
        let boards = 5.0 * gw + 4.0 * SLOT_GAP;
        assert!(
            (boards - b.total).abs() < 1.0,
            "five boards span {:.1}, the shop {:.1} - SLOT_CELL should be {:.2}",
            boards,
            b.total,
            (b.total - 4.0 * SLOT_GAP) / (5.0 * SLOT_W as f32)
        );
    }

    /// Everything below the boards has to fit on the screen. Bigger cells cost
    /// vertical room, and the inventory is what pays for it.
    #[test]
    fn the_shop_and_tray_still_fit_under_the_boards() {
        for worn in [0usize, 2, 4, 8] {
            let b = bands(worn, SLOT_H);
            let board_bottom = SLOT_TOP + SLOT_H as f32 * SLOT_CELL;
            assert!(b.strip_y > board_bottom, "shop overlaps the boards at {} items", worn);
            assert!(
                b.inv_y + b.inv_h <= LOGICAL_H,
                "the tray runs off the bottom at {} items",
                worn
            );
            // The tray has to be able to show a full tray. A cap you cannot
            // see the contents of is worse than no cap.
            let per_row = (((b.total + CARD_GAP) / (CARD_W + CARD_GAP)) as usize).max(1);
            let rows_needed = gearmaster_engine::run::INVENTORY_CAP.div_ceil(per_row) as f32;
            let needed = 36.0 + rows_needed * (CARD_H + CARD_GAP);
            assert!(
                b.inv_h >= needed,
                "tray is {:.0} tall, needs {:.0} to show {} pieces in {} rows of {}",
                b.inv_h,
                needed,
                gearmaster_engine::run::INVENTORY_CAP,
                rows_needed,
                per_row
            );
        }
    }

    /// The chart's corners are fixed and distinct. Fixed is the point: two
    /// builds are meant to be comparable by their shape, which stops being
    /// true the moment the axes depend on the build.
    #[test]
    fn the_chart_has_eight_distinct_corners() {
        assert_eq!(RADAR.len(), 8);
        for (i, (a, tag)) in RADAR.iter().enumerate() {
            for (b, other) in RADAR.iter().skip(i + 1) {
                assert_ne!(a, b, "{} and {} are the same axis", tag, other);
                assert_ne!(tag, other, "two corners both labelled {}", tag);
            }
        }
    }

    /// Every corner label has to fit the space beside the chart at 12px.
    #[test]
    fn every_corner_label_is_short_enough_to_read() {
        for (axis, tag) in RADAR {
            assert!(tag.len() <= 5, "{} is too long a tag for {}", tag, axis.name());
        }
    }
}

#[cfg(test)]
mod tooltip_tests {
    use super::*;
    use gearmaster_engine::run::Run;

    fn inked_spell() -> (Run, ItemProfile) {
        let mut run = Run::with_all_pieces();
        for (name, x, y) in
            [("Oathbound Ink", 0u8, 0u8), ("Scholar's Codex", 2, 0), ("Absolution", 4, 0)]
        {
            let id = run
                .owned
                .iter()
                .copied()
                .find(|&i| run.registry.def(i).name == name)
                .expect("piece exists");
            run.equip(id, SlotKind::Weapon, x, y).expect("it fits");
        }
        let p = run.combat_items().into_iter().next().expect("one item");
        (run, p)
    }

    /// An unconditional pool gain is a stat, and reads like one. It used to
    /// appear only as trigger text, so a piece banking two faith looked
    /// different from a piece whose two faith happened to be a base stat.
    #[test]
    fn a_plain_pool_gain_reads_as_a_stat_not_a_trigger() {
        let (run, p) = inked_spell();
        let lines = item_summary_lines(&p, &run);
        let body: Vec<&str> = lines.iter().map(|(s, _)| s.as_str()).collect();
        // A bare "<n> faith" line, whatever n works out to once the item's
        // own power is on it - the point is that it reads as a stat rather
        // than as trigger text.
        let stat_line = body
            .iter()
            .find(|l| {
                let t = l.trim();
                t.ends_with(" faith") && t.split(' ').next().is_some_and(|n| n.parse::<i32>().is_ok())
            })
            .copied()
            .unwrap_or_else(|| panic!("no plain faith line in {:?}", body));
        let n: i32 = stat_line.trim().split(' ').next().unwrap().parse().unwrap();
        assert!(n > 0, "the faith line reads {:?}", stat_line);
        assert!(
            !body.iter().any(|l| l.contains(&format!("on activation, gain {} faith", n))),
            "the plain gain is still wearing trigger clothes: {:?}",
            body
        );
    }

    /// The card names the item's whole multiplier, not the ink's share of it,
    /// and everything it quotes already has that multiplier applied.
    #[test]
    fn the_card_names_the_items_own_power() {
        let (run, p) = inked_spell();
        assert!(p.power > 100, "the fixture should carry power");
        let lines = item_summary_lines(&p, &run);
        let joined: String = lines.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>().join(" ");
        assert!(
            joined.contains(&format!("x{}.{:02}", p.power / 100, p.power % 100)),
            "the card does not name the item's power of {}: {:?}",
            p.power,
            joined
        );
        assert!(
            !joined.contains("from the ink bound into it"),
            "the card still names only the ink's share: {:?}",
            joined
        );
    }

    /// A trigger that *spends* a pool keeps its own line: there the wording is
    /// the information, not decoration.
    #[test]
    fn a_conditional_trigger_keeps_its_line() {
        let (run, p) = inked_spell();
        let lines = item_summary_lines(&p, &run);
        assert!(
            lines.iter().any(|(s, c)| s.contains("spend 3 faith") && *c == col_trigger()),
            "the spend should still be called out in trigger colours"
        );
    }

    /// Ink multiplies the item it is bound into, and combat applies it. The
    /// card left it out, so a well-inked spell read as hitting for less than
    /// half what it actually lands.
    #[test]
    fn the_card_counts_the_ink_bound_into_the_item() {
        let (run, p) = inked_spell();
        assert!(p.power_bonus > 0, "the fixture should be inked");
        // The ink is inside the item's own multiplier now, rather than being
        // added to the wearer's at the call site.
        assert!(
            p.power >= 100 + p.power_bonus,
            "the item's multiplier should carry its ink: {} vs {}",
            p.power,
            p.power_bonus
        );
        let inked = p.hit_for(run.player_stats().strength);
        assert!(inked > 0, "and it should be hitting for something");

        // A caster's line reports both intensities, because the printed
        // number is neither of them.
        use gearmaster_engine::combat::{EMPOWERED_CAST_PCT, WEAK_CAST_PCT};
        let lines = item_summary_lines(&p, &run);
        let shown = lines
            .iter()
            .find(|(s, _)| s.contains("casts for"))
            .map(|(s, _)| s.clone())
            .expect("a spell says what it casts for");
        assert!(
            shown.contains(&(inked * EMPOWERED_CAST_PCT / 100).to_string()),
            "card shows {:?}, a paid cast lands {}",
            shown,
            inked * EMPOWERED_CAST_PCT / 100
        );
        assert!(
            shown.contains(&(inked * WEAK_CAST_PCT / 100).to_string()),
            "card shows {:?}, an unpaid cast lands {}",
            shown,
            inked * WEAK_CAST_PCT / 100
        );
    }

    /// A hit says what kind of harm it is. One kind is named; a mixed weapon
    /// shows the split beside the total, because physical and magic answer to
    /// different resistances and "hits for 80" says nothing about which.
    #[test]
    fn a_hit_says_what_kind_of_damage_it_is() {
        use gearmaster_engine::piece::SlotKind;
        use gearmaster_engine::run::Run;

        let build = |names: &[(&str, u8, u8)]| -> Run {
            let mut run = Run::with_all_pieces();
            for &(name, x, y) in names {
                let id = run
                    .owned
                    .iter()
                    .copied()
                    .find(|&i| run.registry.def(i).name == name)
                    .expect("piece exists");
                run.equip(id, SlotKind::Weapon, x, y).expect("it fits");
            }
            run
        };
        let run = build(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)]);
        let p = run.combat_items().into_iter().next().expect("a weapon");
        let lines = item_summary_lines(&p, &run);
        let hit = lines
            .iter()
            .find(|(s, _)| s.contains("hits for"))
            .map(|(s, _)| s.clone())
            .expect("it hits");
        assert!(hit.contains("physical"), "a plain blade should name its damage: {:?}", hit);

        // And a mixed one shows the parts.
        let mixed = build(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0), ("Hexbolt", 2, 0)]);
        let m = mixed.combat_items().into_iter().next().expect("a weapon");
        assert!(m.stats.physical_damage > 0 && m.stats.magic_damage > 0, "fixture is mixed");
        let lines = item_summary_lines(&m, &mixed);
        let hit = lines
            .iter()
            .find(|(s, _)| s.contains("hits for"))
            .map(|(s, _)| s.clone())
            .expect("it hits");
        assert!(hit.contains(" + "), "a mixed weapon should show the split: {:?}", hit);
        assert!(hit.contains("physical") && hit.contains("magic"), "{:?}", hit);
    }

    /// And it must not still be quoting only the weak branch, which read as
    /// though the printed figure were what a paid cast does.
    #[test]
    fn a_casters_card_does_not_quote_the_weak_branch_alone() {
        let (run, p) = inked_spell();
        let lines = item_summary_lines(&p, &run);
        let joined: String = lines.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>().join(" ");
        assert!(
            !joined.contains("lands at 45%"),
            "the card still describes only the unpaid cast: {:?}",
            joined
        );
        assert!(joined.contains("paid"), "and it should name the paid one");
    }
}
