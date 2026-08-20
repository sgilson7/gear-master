#!/usr/bin/env bash
# Scaffold a Rust game-prototype workspace: pure engine crate + macroquad GUI
# + headless CLI driver. Produces a workspace that builds, tests, and opens a
# window with a working click interaction.
#
#   ./scaffold.sh <ProjectDir> [crate-prefix]
#
#   ./scaffold.sh GearMaster           -> ./GearMaster, crates gearmaster-{engine,gui,cli}
#   ./scaffold.sh . gearmaster         -> scaffold into the current directory
#
# Refuses to overwrite an existing Cargo.toml.
set -euo pipefail

DIR="${1:-}"
if [ -z "$DIR" ]; then
  echo "usage: scaffold.sh <ProjectDir> [crate-prefix]" >&2
  exit 2
fi

mkdir -p "$DIR"
cd "$DIR"

TITLE=$(basename "$PWD")
# Cargo package prefix: lowercase, non-alphanumerics collapsed to '-'.
CRATE="${2:-$(printf '%s' "$TITLE" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | sed 's/-$//')}"
# Rust identifier for the engine crate (cargo maps '-' to '_' in paths).
ENGINE_RS="$(printf '%s' "$CRATE" | tr '-' '_')_engine"

if [ -e Cargo.toml ]; then
  echo "error: $PWD/Cargo.toml already exists — refusing to overwrite." >&2
  exit 1
fi

emit() {
  mkdir -p "$(dirname "$1")"
  sed -e "s/__ENGINE_RS__/$ENGINE_RS/g" -e "s/__CRATE__/$CRATE/g" -e "s/__TITLE__/$TITLE/g" > "$1"
}

# ---------------------------------------------------------------- workspace

emit Cargo.toml <<'EOF'
[workspace]
resolver = "2"
members = ["crates/engine", "crates/gui", "crates/cli"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
EOF

emit .gitignore <<'EOF'
/target
**/*.rs.bk
.DS_Store
EOF

# ------------------------------------------------------------------- engine

emit crates/engine/Cargo.toml <<'EOF'
[package]
name = "__CRATE__-engine"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

# This crate must never depend on macroquad or any rendering library. Keeping
# it graphics-free is what makes the rules testable without a window.
[dependencies]
EOF

emit crates/engine/src/lib.rs <<'EOF'
//! Pure game rules. No rendering dependencies — everything here is testable
//! with `cargo test` and no window.

pub mod entity;
pub mod grid;
pub mod run;
EOF

emit crates/engine/src/entity.rs <<'EOF'
use std::collections::{HashMap, HashSet};

/// Opaque handle. Grids store these, never the entity data itself, so one
/// entity can occupy several cells (or several grids) without cloning.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct EntityId(pub u32);

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct EntityMeta {
    pub id: EntityId,
    pub name: String,
    /// Cells occupied, relative to the anchor cell. `[(0, 0)]` is a 1x1.
    /// Grow this into real polyomino shapes (with rotation) as needed.
    pub cells: Vec<(i8, i8)>,
}

/// Single source of truth for entity data and liveness.
#[derive(Clone, Default)]
pub struct EntityRegistry {
    entities: HashMap<EntityId, EntityMeta>,
    alive: HashSet<EntityId>,
    next_id: u32,
}

impl EntityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, name: &str, cells: Vec<(i8, i8)>) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.entities
            .insert(id, EntityMeta { id, name: name.to_string(), cells });
        self.alive.insert(id);
        id
    }

    /// Panics on an unknown id: a dangling `EntityId` is a bug in the caller,
    /// not a runtime condition worth handling.
    pub fn meta(&self, id: EntityId) -> &EntityMeta {
        self.entities.get(&id).expect("missing entity meta")
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.alive.contains(&id)
    }

    /// Clears the alive flag only. Callers must also remove the id from any
    /// grid that holds it.
    pub fn mark_dead(&mut self, id: EntityId) {
        self.alive.remove(&id);
    }

    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }
}
EOF

emit crates/engine/src/grid.rs <<'EOF'
use crate::entity::{EntityId, EntityRegistry};

/// A rectangular container of cells. Cells hold ids, never entity data.
#[derive(Clone, Debug)]
pub struct Grid {
    pub w: u8,
    pub h: u8,
    cells: Vec<Option<EntityId>>,
}

impl Grid {
    pub fn new(w: u8, h: u8) -> Self {
        Self { w, h, cells: vec![None; w as usize * h as usize] }
    }

    #[inline]
    fn idx(&self, x: u8, y: u8) -> usize {
        debug_assert!(x < self.w && y < self.h);
        y as usize * self.w as usize + x as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w as i32 && y < self.h as i32
    }

    pub fn get(&self, x: u8, y: u8) -> Option<EntityId> {
        self.cells[self.idx(x, y)]
    }

    pub fn set(&mut self, x: u8, y: u8, id: Option<EntityId>) {
        let i = self.idx(x, y);
        self.cells[i] = id;
    }

    /// First (topmost, then leftmost) cell holding `id`.
    pub fn find(&self, id: EntityId) -> Option<(u8, u8)> {
        for y in 0..self.h {
            for x in 0..self.w {
                if self.get(x, y) == Some(id) {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Clear every cell holding `id`.
    pub fn remove(&mut self, id: EntityId) {
        for cell in &mut self.cells {
            if *cell == Some(id) {
                *cell = None;
            }
        }
    }

    /// Would `id`'s shape fit with its anchor at `(ax, ay)`? Cells already
    /// held by `id` itself don't count as collisions, so this also answers
    /// "can it be nudged there" for an entity still on the grid.
    pub fn can_place(&self, reg: &EntityRegistry, id: EntityId, ax: u8, ay: u8) -> bool {
        reg.meta(id).cells.iter().all(|&(dx, dy)| {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if !self.in_bounds(nx, ny) {
                return false;
            }
            match self.get(nx as u8, ny as u8) {
                None => true,
                Some(occupant) => occupant == id,
            }
        })
    }

    /// Write `id` into every cell of its shape. Check `can_place` first.
    pub fn place(&mut self, reg: &EntityRegistry, id: EntityId, ax: u8, ay: u8) {
        for &(dx, dy) in &reg.meta(id).cells {
            let (nx, ny) = (ax as i32 + dx as i32, ay as i32 + dy as i32);
            if self.in_bounds(nx, ny) {
                let i = self.idx(nx as u8, ny as u8);
                self.cells[i] = Some(id);
            }
        }
    }

    /// Every anchor at which `id` currently fits. This is what the GUI
    /// highlights — the renderer must never work out fit for itself.
    pub fn legal_anchors(&self, reg: &EntityRegistry, id: EntityId) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if self.can_place(reg, id, x, y) {
                    out.push((x, y));
                }
            }
        }
        out
    }
}
EOF

emit crates/engine/src/run.rs <<'EOF'
use crate::entity::{EntityId, EntityRegistry};
use crate::grid::Grid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleError {
    NoEntityThere,
    DoesNotFit,
    NotAlive,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::NoEntityThere => write!(f, "no entity there"),
            RuleError::DoesNotFit => write!(f, "doesn't fit there"),
            RuleError::NotAlive => write!(f, "entity is not alive"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    /// Move the entity occupying `from` so its anchor lands on `to`.
    Move { from: (u8, u8), to: (u8, u8) },
}

/// Everything the player should SEE happen as a result of one action. The GUI
/// animates from this instead of diffing snapshots.
#[derive(Clone, Debug)]
pub struct Effect {
    pub moved: Vec<(EntityId, (u8, u8), (u8, u8))>,
    pub message: String,
}

pub struct Run {
    pub grid: Grid,
    pub registry: EntityRegistry,
    pub turn: u32,
    /// Set by every successful `apply`. `None` until the first one.
    pub last_effect: Option<Effect>,
}

impl Run {
    /// Starting state. Replace with real content constructors — plain Rust
    /// functions, one per level / encounter / loadout.
    pub fn new_demo() -> Self {
        let mut registry = EntityRegistry::new();
        let mut grid = Grid::new(8, 6);

        let blob = registry.alloc("blob", vec![(0, 0)]);
        let bar = registry.alloc("bar", vec![(0, 0), (1, 0)]);
        let ell = registry.alloc("ell", vec![(0, 0), (0, 1), (1, 1)]);

        grid.place(&registry, blob, 1, 1);
        grid.place(&registry, bar, 4, 2);
        grid.place(&registry, ell, 2, 3);

        Self { grid, registry, turn: 0, last_effect: None }
    }

    pub fn entity_at(&self, x: u8, y: u8) -> Option<EntityId> {
        self.grid.get(x, y)
    }

    /// Anchors the entity occupying `(x, y)` could legally move to.
    pub fn legal_anchors_from(&self, x: u8, y: u8) -> Vec<(u8, u8)> {
        match self.grid.get(x, y) {
            Some(id) => self.grid.legal_anchors(&self.registry, id),
            None => Vec::new(),
        }
    }

    /// The single mutation entry point. Ordering:
    ///   1. resolve + validate the action against current state
    ///   2. mutate the world
    ///   3. record `last_effect`, bump `turn`
    ///
    /// A rejected action must leave the world exactly as it found it.
    pub fn apply(&mut self, action: &Action) -> Result<(), RuleError> {
        match *action {
            Action::Move { from, to } => {
                let id = self.grid.get(from.0, from.1).ok_or(RuleError::NoEntityThere)?;
                if !self.registry.is_alive(id) {
                    return Err(RuleError::NotAlive);
                }
                let anchor = self.grid.find(id).ok_or(RuleError::NoEntityThere)?;

                // Lift first so the entity's own cells can't block its own fit
                // check, then restore it if the destination turns out illegal.
                self.grid.remove(id);
                if !self.grid.can_place(&self.registry, id, to.0, to.1) {
                    self.grid.place(&self.registry, id, anchor.0, anchor.1);
                    return Err(RuleError::DoesNotFit);
                }
                self.grid.place(&self.registry, id, to.0, to.1);

                self.last_effect = Some(Effect {
                    moved: vec![(id, anchor, to)],
                    message: format!(
                        "{} moved ({}, {}) -> ({}, {})",
                        self.registry.meta(id).name,
                        anchor.0,
                        anchor.1,
                        to.0,
                        to.1
                    ),
                });
                self.turn += 1;
                Ok(())
            }
        }
    }
}
EOF

emit crates/engine/tests/grid_rules.rs <<'EOF'
use __ENGINE_RS__::entity::EntityRegistry;
use __ENGINE_RS__::grid::Grid;
use __ENGINE_RS__::run::{Action, Run, RuleError};

#[test]
fn demo_run_starts_with_three_placed_entities() {
    let run = Run::new_demo();
    assert_eq!(run.turn, 0);
    assert!(run.last_effect.is_none());
    assert_eq!(run.registry.ids().count(), 3);
    assert!(run.entity_at(1, 1).is_some(), "blob should be at (1, 1)");
}

#[test]
fn multi_cell_entity_occupies_every_cell_of_its_shape() {
    let mut reg = EntityRegistry::new();
    let mut grid = Grid::new(4, 4);
    let ell = reg.alloc("ell", vec![(0, 0), (0, 1), (1, 1)]);

    grid.place(&reg, ell, 1, 1);

    assert_eq!(grid.get(1, 1), Some(ell));
    assert_eq!(grid.get(1, 2), Some(ell));
    assert_eq!(grid.get(2, 2), Some(ell));
    assert_eq!(grid.get(2, 1), None, "a cell outside the shape must stay empty");
}

#[test]
fn can_place_rejects_overlap_but_allows_self_overlap() {
    let mut reg = EntityRegistry::new();
    let mut grid = Grid::new(4, 4);
    let bar = reg.alloc("bar", vec![(0, 0), (1, 0)]);
    let blob = reg.alloc("blob", vec![(0, 0)]);

    grid.place(&reg, bar, 0, 0);
    grid.place(&reg, blob, 3, 3);

    assert!(!grid.can_place(&reg, blob, 1, 0), "blob must not overlap bar");
    assert!(grid.can_place(&reg, bar, 0, 0), "an entity may overlap itself");
}

#[test]
fn can_place_rejects_shapes_hanging_off_the_edge() {
    let mut reg = EntityRegistry::new();
    let grid = Grid::new(4, 4);
    let bar = reg.alloc("bar", vec![(0, 0), (1, 0)]);

    assert!(grid.can_place(&reg, bar, 2, 0));
    assert!(!grid.can_place(&reg, bar, 3, 0), "second cell would be out of bounds");
}

#[test]
fn move_updates_the_grid_and_records_an_effect() {
    let mut run = Run::new_demo();
    let id = run.entity_at(1, 1).expect("blob at (1, 1)");

    run.apply(&Action::Move { from: (1, 1), to: (6, 5) }).expect("legal move");

    assert_eq!(run.entity_at(1, 1), None, "the old cell must be cleared");
    assert_eq!(run.entity_at(6, 5), Some(id));
    assert_eq!(run.turn, 1);

    let effect = run.last_effect.as_ref().expect("effect recorded");
    assert_eq!(effect.moved, vec![(id, (1, 1), (6, 5))]);
}

#[test]
fn illegal_move_leaves_the_world_untouched() {
    let mut run = Run::new_demo();
    let bar = run.entity_at(4, 2).expect("bar at (4, 2)");

    // (7, 2) would put the bar's second cell out of bounds on an 8-wide grid.
    let err = run.apply(&Action::Move { from: (4, 2), to: (7, 2) }).unwrap_err();

    assert_eq!(err, RuleError::DoesNotFit);
    assert_eq!(run.entity_at(4, 2), Some(bar), "bar must be restored");
    assert_eq!(run.entity_at(5, 2), Some(bar));
    assert_eq!(run.turn, 0, "a rejected action must not consume a turn");
}

#[test]
fn legal_anchors_excludes_cells_blocked_by_other_entities() {
    let run = Run::new_demo();
    let anchors = run.legal_anchors_from(1, 1);

    assert!(!anchors.is_empty());
    assert!(anchors.contains(&(1, 1)), "the current anchor is trivially legal");
    assert!(!anchors.contains(&(4, 2)), "bar occupies (4, 2)");
}
EOF

# ---------------------------------------------------------------------- gui

emit crates/gui/Cargo.toml <<'EOF'
[package]
name = "__CRATE__-gui"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "__CRATE__-gui"
path = "src/main.rs"

[dependencies]
__CRATE__-engine = { path = "../engine" }
macroquad = "0.4"
EOF

emit crates/gui/src/main.rs <<'EOF'
//! Rendering and input only. No game rules live here — every legality
//! question goes to the engine.

use __ENGINE_RS__::entity::EntityId;
use __ENGINE_RS__::run::{Action, Run};
use macroquad::prelude::*;

const PANEL_W: f32 = 300.0;
const CELL: f32 = 64.0;
const PAD: f32 = 16.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "__TITLE__".to_string(),
        window_width: 1100,
        window_height: 700,
        window_resizable: true,
        ..Default::default()
    }
}

// ----------------------------------------------------------------- layout

/// Derived every frame from the engine state and the current window size.
/// Owns BOTH coordinate conversions so they can never drift apart.
struct Layout {
    origin: (f32, f32),
    w: u8,
    h: u8,
}

impl Layout {
    fn from(run: &Run) -> Self {
        let (w, h) = (run.grid.w, run.grid.h);
        let area_w = screen_width() - PANEL_W;
        Self {
            origin: (
                ((area_w - w as f32 * CELL) / 2.0).max(PAD),
                ((screen_height() - h as f32 * CELL) / 2.0).max(PAD),
            ),
            w,
            h,
        }
    }

    /// grid -> pixels (top-left of the cell)
    fn cell_origin(&self, x: u8, y: u8) -> (f32, f32) {
        (self.origin.0 + x as f32 * CELL, self.origin.1 + y as f32 * CELL)
    }

    fn cell_center(&self, x: u8, y: u8) -> (f32, f32) {
        let (px, py) = self.cell_origin(x, y);
        (px + CELL / 2.0, py + CELL / 2.0)
    }

    /// pixels -> grid
    fn hit(&self, mx: f32, my: f32) -> Option<(u8, u8)> {
        let gx = ((mx - self.origin.0) / CELL).floor() as i32;
        let gy = ((my - self.origin.1) / CELL).floor() as i32;
        if !(0..self.w as i32).contains(&gx) || !(0..self.h as i32).contains(&gy) {
            return None;
        }
        Some((gx as u8, gy as u8))
    }
}

// ----------------------------------------------------------------- palette

fn col_bg() -> Color {
    Color::from_rgba(15, 15, 25, 255)
}
fn col_cell_a() -> Color {
    Color::from_rgba(38, 38, 52, 255)
}
fn col_cell_b() -> Color {
    Color::from_rgba(46, 46, 62, 255)
}
fn col_panel() -> Color {
    Color::from_rgba(20, 20, 30, 255)
}
fn col_legal() -> Color {
    Color::from_rgba(80, 180, 255, 80)
}

/// Stable per-entity hue, so entities are distinguishable with no art.
/// `hsl_to_rgb` is NOT in the prelude — only `Color` and the constants are.
fn col_entity(id: EntityId) -> Color {
    macroquad::color::hsl_to_rgb((id.0 as f32 * 0.37) % 1.0, 0.55, 0.60)
}

// ------------------------------------------------------------------ render

fn render_grid(layout: &Layout) {
    for y in 0..layout.h {
        for x in 0..layout.w {
            let (px, py) = layout.cell_origin(x, y);
            let c = if (x + y) % 2 == 0 { col_cell_a() } else { col_cell_b() };
            draw_rectangle(px, py, CELL, CELL, c);
            draw_rectangle_lines(px, py, CELL, CELL, 1.0, Color::from_rgba(0, 0, 0, 70));
        }
    }
}

fn render_legal(layout: &Layout, anchors: &[(u8, u8)]) {
    for &(x, y) in anchors {
        let (px, py) = layout.cell_origin(x, y);
        draw_rectangle(px, py, CELL, CELL, col_legal());
    }
}

fn render_entities(layout: &Layout, run: &Run) {
    for y in 0..layout.h {
        for x in 0..layout.w {
            let Some(id) = run.entity_at(x, y) else { continue };
            let (px, py) = layout.cell_origin(x, y);
            draw_rectangle(px + 3.0, py + 3.0, CELL - 6.0, CELL - 6.0, col_entity(id));
        }
    }
    // Label each entity once, at its anchor cell.
    for id in run.registry.ids() {
        let Some((x, y)) = run.grid.find(id) else { continue };
        let (cx, cy) = layout.cell_center(x, y);
        let name = run.registry.meta(id).name.clone();
        let d = measure_text(&name, None, 16, 1.0);
        draw_text(
            &name,
            cx - d.width / 2.0,
            cy + 5.0,
            16.0,
            Color::from_rgba(20, 20, 28, 235),
        );
    }
}

fn render_selection(layout: &Layout, sel: (u8, u8)) {
    let (px, py) = layout.cell_origin(sel.0, sel.1);
    draw_rectangle_lines(px, py, CELL, CELL, 4.0, YELLOW);
}

fn render_panel(run: &Run, msg: &str, selected: Option<(u8, u8)>) {
    let x = screen_width() - PANEL_W;
    draw_rectangle(x, 0.0, PANEL_W, screen_height(), col_panel());

    let mut y = 32.0;
    draw_text("__TITLE__", x + 16.0, y, 22.0, WHITE);
    y += 30.0;
    draw_text(&format!("Turn: {}", run.turn), x + 16.0, y, 18.0, LIGHTGRAY);
    y += 34.0;

    draw_text("Controls", x + 16.0, y, 18.0, WHITE);
    y += 24.0;
    for line in [
        "  click   select / move",
        "  Esc     clear selection",
        "  R       restart",
        "  F12     save a screenshot",
    ] {
        draw_text(line, x + 16.0, y, 15.0, LIGHTGRAY);
        y += 19.0;
    }
    y += 16.0;

    if let Some((sx, sy)) = selected {
        draw_text(&format!("Selected: ({}, {})", sx, sy), x + 16.0, y, 16.0, YELLOW);
        y += 26.0;
    }

    for line in wrap_lines(msg, 30) {
        draw_text(&line, x + 16.0, y, 15.0, WHITE);
        y += 19.0;
    }
}

fn wrap_lines(s: &str, width: usize) -> Vec<String> {
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

// -------------------------------------------------------------------- main

#[macroquad::main(window_conf)]
async fn main() {
    macroquad::rand::srand(macroquad::miniquad::date::now() as u64);

    let mut run = Run::new_demo();
    let mut selected: Option<(u8, u8)> = None;
    let mut message = String::from("Click an entity, then click where it should go.");

    loop {
        clear_background(col_bg());

        let layout = Layout::from(&run);

        // Ask the ENGINE what is legal; never work it out here.
        let anchors: Vec<(u8, u8)> = selected
            .map(|(x, y)| run.legal_anchors_from(x, y))
            .unwrap_or_default();

        render_grid(&layout);
        render_legal(&layout, &anchors);
        render_entities(&layout, &run);
        if let Some(sel) = selected {
            render_selection(&layout, sel);
        }
        render_panel(&run, &message, selected);

        // --- input last, so a click acts on what the player just saw ---
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if let Some((x, y)) = layout.hit(mx, my) {
                match selected {
                    Some(sel) if sel == (x, y) => selected = None,
                    Some(sel) => {
                        if anchors.contains(&(x, y)) {
                            match run.apply(&Action::Move { from: sel, to: (x, y) }) {
                                Ok(()) => {
                                    message = run
                                        .last_effect
                                        .as_ref()
                                        .map(|e| e.message.clone())
                                        .unwrap_or_default();
                                    selected = None;
                                }
                                Err(e) => message = e.to_string(),
                            }
                        } else if run.entity_at(x, y).is_some() {
                            selected = Some((x, y));
                        } else {
                            // Never fail silently — a dead click reads as a bug.
                            message = format!("({}, {}) is not a legal destination", x, y);
                        }
                    }
                    None => {
                        if run.entity_at(x, y).is_some() {
                            selected = Some((x, y));
                        }
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            selected = None;
        }
        if is_key_pressed(KeyCode::R) {
            run = Run::new_demo();
            selected = None;
            message = "Restarted.".to_string();
        }
        // Lets a human hand a screenshot to an agent that can't see the
        // window. Worth keeping for the life of the prototype.
        if is_key_pressed(KeyCode::F12) {
            let path = format!("/tmp/__CRATE__-{}.png", (get_time() * 1000.0) as u64);
            get_screen_data().export_png(&path);
            println!("screenshot: {}", path);
            message = format!("Saved {}", path);
        }

        next_frame().await;
    }
}
EOF

# --------------------------------------------------------------------- cli

emit crates/cli/Cargo.toml <<'EOF'
[package]
name = "__CRATE__-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "__CRATE__-cli"
path = "src/main.rs"

[dependencies]
__CRATE__-engine = { path = "../engine" }
EOF

emit crates/cli/src/main.rs <<'EOF'
//! Headless driver. Lets an agent (or a script) play the game end to end
//! without a window — the only way to exercise real gameplay in CI.
//!
//!   printf 'show\nmove 1 1 6 5\nshow\n' | cargo run -q -p __CRATE__-cli

use std::io::{self, BufRead, Write};

use __ENGINE_RS__::run::{Action, Run};

fn main() {
    let mut run = Run::new_demo();
    println!("__TITLE__ — type `help` for commands.");
    render(&run);

    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("[turn {}]> ", run.turn);
        io::stdout().flush().ok();

        line.clear();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            [] => continue,
            ["quit"] | ["exit"] => break,
            ["help"] => println!("show | moves <x> <y> | move <x> <y> <tx> <ty> | restart | quit"),
            ["show"] => render(&run),
            ["restart"] => {
                run = Run::new_demo();
                render(&run);
            }
            ["moves", x, y] => match (x.parse(), y.parse()) {
                (Ok(x), Ok(y)) => println!("{:?}", run.legal_anchors_from(x, y)),
                _ => println!("error: expected two numbers"),
            },
            ["move", x, y, tx, ty] => match (x.parse(), y.parse(), tx.parse(), ty.parse()) {
                (Ok(x), Ok(y), Ok(tx), Ok(ty)) => {
                    match run.apply(&Action::Move { from: (x, y), to: (tx, ty) }) {
                        Ok(()) => {
                            if let Some(e) = &run.last_effect {
                                println!("{}", e.message);
                            }
                            render(&run);
                        }
                        Err(e) => println!("error: {}", e),
                    }
                }
                _ => println!("error: expected four numbers"),
            },
            _ => println!("unknown command; try `help`"),
        }
    }
}

fn render(run: &Run) {
    for y in 0..run.grid.h {
        let row: String = (0..run.grid.w)
            .map(|x| match run.entity_at(x, y) {
                Some(id) => char::from(b'A' + (id.0 % 26) as u8),
                None => '.',
            })
            .collect();
        println!("{:2} {}", y, row);
    }
    print!("   ");
    for x in 0..run.grid.w {
        print!("{}", x % 10);
    }
    println!();
}
EOF

cat <<EOF

Scaffolded '$CRATE' in $PWD

  crates/engine  pure rules  — no graphics deps; all the tests live here
  crates/gui     macroquad   — rendering + input only
  crates/cli     headless    — drive the game without a window

Next:
  cd $PWD
  cargo test -p $CRATE-engine    # fast feedback loop, no window
  cargo run  -p $CRATE-gui       # opens a window: click an entity, then a cell
  cargo run  -p $CRATE-cli       # headless repl

The macroquad build is the slow part (~38 transitive crates). Warm it now.
EOF
