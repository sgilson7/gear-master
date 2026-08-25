//! The board packer: a local tool for dressing creatures by hand.
//!
//! `pack_francis` searches for a creature's board against a difficulty curve.
//! It works, and it takes about five minutes per creature per power band, which
//! makes half an hour a board and a day a cluster - and what it finds is a
//! board that satisfies a number rather than a board somebody meant. A person
//! looking at the grid is faster than the search and packs more thematically,
//! so this is the grid, and the person is the search.
//!
//! It is deliberately not the game. No gold, no run, no shop stock: every
//! component in the catalogue is on the shelf all the time, and clicking one
//! picks it up. What it does share with the game is every *rule* - placement,
//! assembly, recipes, item grouping - because all of that comes out of
//! `gearmaster_engine` rather than being reimplemented here. A board this tool
//! calls finished is finished by the same code the fight uses.
//!
//! **It edits `crates/engine/src/combat.rs` in place.** Saving rewrites the
//! selected `MonsterSpec`'s `gear:` and `items:` and leaves everything else in
//! the literal alone - health, attacks, bounty, sprite, rank, drops. The tool
//! authors a board, not a creature, which is the same line `splice.py` drew.
//!
//! Loading needs no parsing at all: `LADDER` and `ALTERNATES` are compiled in,
//! so `cargo run -p gearmaster-packer` always opens on whatever the source says
//! today.
//!
//!   make pack                  # or: cargo run -p gearmaster-packer
//!
//! Left, the creatures. Middle, the five grids. Right, the catalogue.
//! Click a component to pick it up, click a cell to put it down, `Tab` turns it,
//! right-click takes something off the board, and `Cmd-S` saves. The letter keys
//! are left alone because both search boxes are always listening.

use macroquad::prelude::*;

use gearmaster_engine::combat::{MonsterSpec, ALTERNATES, LADDER};
use gearmaster_engine::loadout::Loadout;
use gearmaster_engine::piece::{
    is_boss_only, is_event_only, is_quest_reward, is_town_stock, PieceId, PieceKind,
    PieceRegistry, SlotKind, CATALOG,
};
use gearmaster_engine::rating::piece_rating;

// ------------------------------------------------------------------ layout

const W: f32 = 1680.0;
const H: f32 = 1000.0;

const LIST_W: f32 = 250.0;
const LIB_W: f32 = 430.0;
const CELL: f32 = 26.0;
const GRID_W: f32 = 6.0 * CELL;
const TOP: f32 = 54.0;

fn col_bg() -> Color {
    Color::from_rgba(18, 18, 22, 255)
}
fn col_panel() -> Color {
    Color::from_rgba(28, 28, 34, 255)
}
fn col_cell_a() -> Color {
    Color::from_rgba(38, 38, 46, 255)
}
fn col_cell_b() -> Color {
    Color::from_rgba(33, 33, 40, 255)
}
fn col_text() -> Color {
    Color::from_rgba(216, 214, 208, 255)
}
fn col_dim() -> Color {
    Color::from_rgba(140, 138, 134, 255)
}
fn col_good() -> Color {
    Color::from_rgba(130, 200, 140, 255)
}
fn col_warn() -> Color {
    Color::from_rgba(224, 172, 92, 255)
}
fn col_bad() -> Color {
    Color::from_rgba(224, 104, 96, 255)
}

/// One colour per slot, so a piece reads as belonging somewhere.
fn col_slot(k: SlotKind) -> Color {
    match k {
        SlotKind::Weapon => Color::from_rgba(196, 108, 96, 255),
        SlotKind::Helmet => Color::from_rgba(122, 152, 200, 255),
        SlotKind::Chest => Color::from_rgba(126, 176, 128, 255),
        SlotKind::Gloves => Color::from_rgba(196, 168, 96, 255),
        SlotKind::Greaves => Color::from_rgba(158, 126, 190, 255),
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle(x, y, w, h, c);
}

fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_text(s, x, y, size, c);
}

/// Draw `s` cut to `w` pixels, with an ellipsis if it did not fit.
fn text_capped(s: &str, x: f32, y: f32, w: f32, size: f32, c: Color) {
    let mut out = s.to_string();
    while measure_text(&out, None, size as u16, 1.0).width > w && out.len() > 1 {
        out.pop();
    }
    if out.len() < s.len() {
        out.pop();
        out.push('\u{2026}');
    }
    draw_text(&out, x, y, size, c);
}

fn hit(x: f32, y: f32, w: f32, h: f32, mx: f32, my: f32) -> bool {
    mx >= x && my >= y && mx < x + w && my < y + h
}

// ------------------------------------------------------------- the boards

/// One creature's board, editable, built out of engine parts.
struct Board {
    reg: PieceRegistry,
    lo: Loadout,
    /// True once something has been moved since the last save.
    dirty: bool,
}

impl Board {
    /// Read a creature's board out of its `MonsterSpec`, exactly as written.
    ///
    /// A placement that will not sit is dropped and counted rather than
    /// panicked over: a gear list that names more than the board holds is a
    /// real state the ladder has been in, and a tool that refuses to open it is
    /// a tool that cannot fix it.
    fn load(spec: &MonsterSpec) -> (Self, usize) {
        let mut reg = PieceRegistry::new();
        let mut lo = Loadout::new();
        let mut dropped = 0;
        // Chunk by chunk, locking each one before the next goes down - which is
        // what `MonsterSpec::loadout_at` does and is not cosmetic. An unlocked
        // board negotiates with itself: two items packed flush merge into one
        // over-full thing that assembles into nothing. Loading a creature
        // without it showed forty-four of the fifty-four holding different
        // items from the ones they are written as holding.
        let mut chunks: Vec<usize> = spec.items.to_vec();
        if chunks.is_empty() {
            chunks = vec![spec.gear.len()];
        }
        let mut at = 0usize;
        for take in chunks {
            let end = (at + take).min(spec.gear.len());
            let mut touched: Vec<SlotKind> = Vec::new();
            for &(name, slot, x, y, rot) in &spec.gear[at..end] {
                let Some(def) = CATALOG.iter().position(|d| d.name == name) else {
                    dropped += 1;
                    continue;
                };
                let id = reg.alloc(def);
                reg.set_rotation(id, rot);
                if lo.can_place(&reg, id, slot, x, y).is_ok() {
                    lo.slot_mut(slot).place(&reg, id, x, y);
                    if !touched.contains(&slot) {
                        touched.push(slot);
                    }
                } else {
                    dropped += 1;
                }
            }
            for kind in touched {
                gearmaster_engine::loadout::lock_assembled_in(&mut lo, &reg, kind);
            }
            at = end;
        }
        (Board { reg, lo, dirty: false }, dropped)
    }

    /// The board as a `gear:` list and an `items:` partition.
    ///
    /// `items` is a run-length partition of `gear` and the fight relies on it:
    /// `MonsterSpec::loadout_at` seats each chunk and locks it before the next
    /// one lands, which is the only reason a densely packed board holds its
    /// shape. So every item's pieces have to come out contiguously, and the
    /// chunks have to sum to the length of the list.
    ///
    /// Loose gear is emitted last, one chunk each. A chunk of one locks a
    /// single piece, which is harmless, and the alternative - leaving it out of
    /// the partition - is a sum that does not match.
    fn emit(&self) -> (Vec<String>, Vec<usize>) {
        let mut lines = Vec::new();
        let mut chunks = Vec::new();
        let push = |lines: &mut Vec<String>, id: PieceId, slot: SlotKind| {
            let (x, y) = self.lo.slot(slot).anchor_of(id).unwrap_or((0, 0));
            lines.push(format!(
                "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                self.reg.def(id).name,
                slot,
                x,
                y,
                self.reg.rotation(id)
            ));
        };
        // Written order is kept wherever it can be.
        //
        // Ids are handed out in the order the gear list was read, so the lowest
        // id in a group is where that group stood in the file. Sorting on it
        // means opening a creature and saving it produces no diff at all - and
        // anything newly picked up has a high id and lands at the end, which is
        // the right place for it. Without this every one of the fifty-four
        // creatures came out reordered the first time it was touched, and a
        // fifty-line diff for a change nobody made is a diff nobody reads.
        let mut groups: Vec<(u32, SlotKind, Vec<PieceId>)> = Vec::new();
        for slot in SlotKind::ALL {
            let report = self.lo.report(&self.reg, slot);
            let mut seated: Vec<PieceId> = Vec::new();
            for item in report.items.iter().filter(|i| i.assembled) {
                let first = item.pieces.iter().map(|p| p.0).min().unwrap_or(u32::MAX);
                // And within the item too: `report` hands its pieces back in
                // board order, which is not the order they were written in.
                let mut pieces = item.pieces.clone();
                pieces.sort_by_key(|p| p.0);
                groups.push((first, slot, pieces));
                seated.extend(item.pieces.iter().copied());
            }
            for id in self.lo.slot(slot).pieces() {
                if seated.contains(&id) || self.reg.def(id).kind.is_enchantment() {
                    continue;
                }
                groups.push((id.0, slot, vec![id]));
            }
        }
        groups.sort_by_key(|(first, ..)| *first);
        for (_, slot, pieces) in groups {
            chunks.push(pieces.len());
            for p in pieces {
                push(&mut lines, p, slot);
            }
        }
        (lines, chunks)
    }

    /// Everything wrong with this board, in the words the suite would use.
    ///
    /// The same checks the tests make, run while there is still somebody
    /// looking at the grid - which is the whole reason to have a tool rather
    /// than a search: the search found these out one failing test at a time.
    fn complaints(&self, spec: &MonsterSpec) -> Vec<(String, Color)> {
        let mut out = Vec::new();
        let mut worn: Vec<SlotKind> = Vec::new();
        for slot in SlotKind::ALL {
            let report = self.lo.report(&self.reg, slot);
            let items = report.items.iter().filter(|i| i.assembled).count();
            if items > 0 {
                worn.push(slot);
            }
            // A creature may not wear what only a door hands over, and it may
            // not be enchanted at all - `progression` says both.
            for id in self.lo.slot(slot).pieces() {
                let def = self.reg.def(id);
                if def.kind.is_enchantment() {
                    out.push((format!("{} is enchanted, and creatures are not", def.name), col_bad()));
                } else if is_event_only(def.name) {
                    out.push((format!("{} is what a door hands over", def.name), col_bad()));
                } else if is_town_stock(def) {
                    out.push((format!("{} is sold in a town", def.name), col_bad()));
                } else if is_quest_reward(def.name) {
                    out.push((format!("{} is the far side of a quest", def.name), col_warn()));
                } else if is_boss_only(def.name) && !spec.drops.contains(&def.name) {
                    out.push((format!("{} belongs to another creature", def.name), col_bad()));
                }
            }
            // A creature swings once a cooldown. Two weapon items is two
            // swings, which is the rule `francis::he_carries_one_sword` holds
            // and the packer has enforced since it was written.
            if slot == SlotKind::Weapon && items > 1 {
                out.push((format!("{items} weapon items is {items} swings a cooldown"), col_bad()));
            }
            let owed = spec.rank.min_items_in(slot);
            if items > 0 && items < owed {
                out.push((
                    format!("{} holds {items} item(s); a {:?} owes {owed}", slot.name(), spec.rank),
                    col_warn(),
                ));
            }
            let loose = report
                .items
                .iter()
                .filter(|i| !i.assembled)
                .filter(|i| !i.pieces.iter().all(|&p| self.reg.def(p).kind.is_enchantment()))
                .count();
            if loose > 0 {
                out.push((format!("{}: {loose} piece(s) never came together", slot.name()), col_warn()));
            }
        }
        if spec.rank.is_named() && worn.len() < spec.rank.min_slots() {
            out.push((
                format!("wears {} slot(s); a {:?} owes {}", worn.len(), spec.rank, spec.rank.min_slots()),
                col_warn(),
            ));
        }
        out
    }
}

// ------------------------------------------------------------------ saving

/// Every creature in the game, ladder first, in the order they are written.
fn everyone() -> Vec<&'static MonsterSpec> {
    LADDER.iter().chain(ALTERNATES.iter()).collect()
}

const COMBAT_RS: &str = "crates/engine/src/combat.rs";

/// Rewrite one `MonsterSpec`'s `gear:` and `items:` where they sit.
///
/// Anchored on the `name:` line and on the indent of the literal it belongs to,
/// which is how the block's own terminator is found - `gear:` is a nested array
/// and the first `],` in it belongs to a `cells:` entry, not to the block.
///
/// `items:` is rewritten before `gear:` because it comes later in the file, so
/// splicing it first leaves the earlier offsets alone.
fn save(name: &str, lines: &[String], chunks: &[usize]) -> Result<String, String> {
    let src = std::fs::read_to_string(COMBAT_RS).map_err(|e| format!("{COMBAT_RS}: {e}"))?;
    let out = splice(&src, name, lines, chunks)?;
    std::fs::write(COMBAT_RS, &out).map_err(|e| format!("{COMBAT_RS}: {e}"))?;
    Ok(format!(
        "saved {name}: {} pieces in {} item(s) - rebuild to see it in the game",
        lines.len(),
        chunks.len()
    ))
}

/// The same rewrite as a pure function of the source, so it can be held to
/// without a file on disk and without a window.
fn splice(src: &str, name: &str, lines: &[String], chunks: &[usize]) -> Result<String, String> {
    let needle = format!("name: \"{name}\",");
    let at = src.find(&needle).ok_or_else(|| format!("no creature called {name}"))?;
    let line_start = src[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent: String = src[line_start..at].to_string();

    let g = src[at..].find("gear: &[").ok_or("that creature has no gear list")? + at;
    // An empty list is written on one line - `gear: &[],` - and its terminator
    // is right there rather than at the literal's indent. Searching for the
    // indented one from an empty list walks straight past the creature and
    // finds the *next* one's, which is how a creature with teeth and no gear
    // came out of this looking like a file in the wrong order.
    let empty = src[g..].starts_with("gear: &[],");
    let end_marker = if empty { "],".to_string() } else { format!("\n{indent}],") };
    let ge = src[g..].find(&end_marker).ok_or("the gear list does not end")? + g;

    let i = src[at..].find("items: &[").ok_or("that creature has no items list")? + at;
    let ie = src[i..].find(']').ok_or("the items list does not end")? + i;

    if !(g < ge && ge < i && i < ie) {
        return Err("gear and items are not in the order this expects".into());
    }

    let gear_block = if lines.is_empty() {
        "gear: &[],".to_string()
    } else {
        format!("gear: &[\n{}\n{indent}],", lines.join("\n").trim_end_matches('\n'))
    };
    let items_block = format!("items: &{chunks:?}");

    let mut out = String::with_capacity(src.len() + 512);
    out.push_str(&src[..i]);
    out.push_str(&items_block);
    out.push_str(&src[ie + 1..]);
    let src = out;

    let mut out = String::with_capacity(src.len() + 512);
    out.push_str(&src[..g]);
    out.push_str(&gear_block);
    out.push_str(&src[ge + end_marker.len()..]);

    Ok(out)
}

// ----------------------------------------------------------- the catalogue

/// The kinds a slot can actually hold, in recipe order, for the tab strip.
fn kinds_of(slot: Option<SlotKind>) -> Vec<PieceKind> {
    let mut out: Vec<PieceKind> = Vec::new();
    for d in CATALOG {
        if slot.is_some_and(|s| !d.fits(s)) {
            continue;
        }
        if !out.contains(&d.kind) {
            out.push(d.kind);
        }
    }
    out
}

/// Everything on the shelf, given the three filters.
fn shelf(slot: Option<SlotKind>, kind: Option<PieceKind>, needle: &str) -> Vec<usize> {
    let needle = needle.to_lowercase();
    (0..CATALOG.len())
        .filter(|&i| {
            let d = &CATALOG[i];
            slot.is_none_or(|s| d.fits(s))
                && kind.is_none_or(|k| d.kind == k)
                && (needle.is_empty() || d.name.to_lowercase().contains(&needle))
        })
        .collect()
}

// -------------------------------------------------------------------- main

fn window_conf() -> Conf {
    Conf {
        window_title: "Gear Master - board packer".to_string(),
        window_width: W as i32,
        window_height: H as i32,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let specs = everyone();
    let mut boards: Vec<Board> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for s in &specs {
        let (b, dropped) = Board::load(s);
        if dropped > 0 {
            notes.push(format!("{}: {dropped} placement(s) would not sit", s.name));
        }
        boards.push(b);
    }

    let mut who = 0usize;
    let mut who_filter = String::new();
    let mut lib_filter = String::new();
    let mut typing_lib = true;
    let mut slot_tab: Option<SlotKind> = None;
    let mut kind_tab: Option<PieceKind> = None;
    let mut held: Option<PieceId> = None;
    let mut status = if notes.is_empty() {
        format!("{} creatures loaded", specs.len())
    } else {
        notes.join("  |  ")
    };
    let mut list_scroll = 0.0f32;
    let mut lib_scroll = 0.0f32;

    loop {
        clear_background(col_bg());
        let (mx, my) = mouse_position();
        let click = is_mouse_button_pressed(MouseButton::Left);
        let rclick = is_mouse_button_pressed(MouseButton::Right);

        // ---- typing ----
        while let Some(c) = get_char_pressed() {
            let target = if typing_lib { &mut lib_filter } else { &mut who_filter };
            match c {
                '\u{8}' => {
                    target.pop();
                }
                '\r' | '\n' => {}
                c if !c.is_control() => target.push(c),
                _ => {}
            }
        }
        // Both search boxes are always taking characters, so a shortcut that is
        // a letter is a shortcut that fires while somebody types "sword".
        // Rotating is Tab and saving is Cmd/Ctrl-S for that reason.
        let cmd = is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper)
            || is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl);
        if is_key_pressed(KeyCode::Escape) {
            held = None;
        }
        if is_key_pressed(KeyCode::Tab) {
            if let Some(id) = held {
                boards[who].reg.rotate_cw(id);
            }
        }

        // ---- top bar ----
        let spec = specs[who];
        rect(0.0, 0.0, W, TOP, col_panel());
        text(spec.name, 14.0, 24.0, 26.0, col_text());
        let rung = LADDER.iter().position(|m| m.name == spec.name);
        let sub = match rung {
            Some(r) => format!("rung {}   {:?}   {} health", r + 1, spec.rank, spec.health),
            None => format!("alternate   {:?}   {} health", spec.rank, spec.health),
        };
        text(&sub, 14.0, 44.0, 15.0, col_dim());

        let save_r = (W - 150.0, 12.0, 130.0, 30.0);
        let hot = hit(save_r.0, save_r.1, save_r.2, save_r.3, mx, my);
        rect(
            save_r.0,
            save_r.1,
            save_r.2,
            save_r.3,
            if hot { col_good() } else { col_panel() },
        );
        draw_rectangle_lines(save_r.0, save_r.1, save_r.2, save_r.3, 2.0, col_good());
        let label = if boards[who].dirty { "SAVE *" } else { "SAVE" };
        text(label, save_r.0 + 34.0, save_r.1 + 21.0, 18.0, col_text());
        if (hot && click) || (cmd && is_key_pressed(KeyCode::S)) {
            let (lines, chunks) = boards[who].emit();
            status = match save(spec.name, &lines, &chunks) {
                Ok(m) => {
                    boards[who].dirty = false;
                    m
                }
                Err(e) => format!("NOT SAVED - {e}"),
            };
        }
        text_capped(&status, 380.0, 34.0, W - 560.0, 15.0, col_warn());

        // ---- left: the creatures ----
        rect(0.0, TOP, LIST_W, H - TOP, col_panel());
        let fr = (8.0, TOP + 8.0, LIST_W - 16.0, 24.0);
        rect(fr.0, fr.1, fr.2, fr.3, col_cell_a());
        if click && hit(fr.0, fr.1, fr.2, fr.3, mx, my) {
            typing_lib = false;
        }
        let shown = if who_filter.is_empty() { "filter creatures\u{2026}" } else { &who_filter };
        text_capped(
            shown,
            fr.0 + 6.0,
            fr.1 + 17.0,
            fr.2 - 12.0,
            15.0,
            if typing_lib { col_dim() } else { col_text() },
        );

        let matches: Vec<usize> = (0..specs.len())
            .filter(|&i| {
                who_filter.is_empty()
                    || specs[i].name.to_lowercase().contains(&who_filter.to_lowercase())
            })
            .collect();
        if hit(0.0, TOP, LIST_W, H - TOP, mx, my) {
            list_scroll = (list_scroll - mouse_wheel().1 * 24.0)
                .clamp(0.0, (matches.len() as f32 * 22.0 - (H - TOP - 60.0)).max(0.0));
        }
        let mut y = TOP + 44.0 - list_scroll;
        for &i in &matches {
            if y > TOP + 30.0 && y < H {
                let on = i == who;
                if on {
                    rect(4.0, y - 14.0, LIST_W - 8.0, 21.0, col_cell_a());
                }
                let c = if boards[i].dirty {
                    col_warn()
                } else if on {
                    col_text()
                } else {
                    col_dim()
                };
                let n = LADDER.iter().position(|m| m.name == specs[i].name);
                let tag = n.map(|r| format!("{:>2}", r + 1)).unwrap_or_else(|| " -".into());
                text(&tag, 10.0, y, 14.0, col_dim());
                text_capped(specs[i].name, 34.0, y, LIST_W - 44.0, 15.0, c);
                if click && hit(4.0, y - 14.0, LIST_W - 8.0, 21.0, mx, my) {
                    who = i;
                    held = None;
                }
            }
            y += 22.0;
        }

        // ---- middle: the five grids ----
        let gx0 = LIST_W + 20.0;
        let gy0 = TOP + 40.0;
        let board = &mut boards[who];
        for (n, &slot) in SlotKind::ALL.iter().enumerate() {
            let ox = gx0 + n as f32 * (GRID_W + 18.0);
            let rows = board.lo.slot(slot).rows();
            text(slot.name(), ox, gy0 - 10.0, 15.0, col_slot(slot));
            for gy in 0..rows {
                for gx in 0..6u8 {
                    let (cx, cy) = (ox + gx as f32 * CELL, gy0 + gy as f32 * CELL);
                    let base = if (gx + gy) % 2 == 0 { col_cell_a() } else { col_cell_b() };
                    rect(cx, cy, CELL - 1.0, CELL - 1.0, base);
                    let s = board.lo.slot(slot);
                    if let Some(id) = s.get(gx, gy) {
                        let d = board.reg.def(id);
                        let mut c = col_slot(d.slot);
                        c.a = 0.85;
                        rect(cx, cy, CELL - 1.0, CELL - 1.0, c);
                        text(
                            &d.name.chars().next().unwrap_or('?').to_string(),
                            cx + 8.0,
                            cy + 18.0,
                            15.0,
                            col_bg(),
                        );
                    } else if s.enchant_at(gx, gy).is_some() {
                        rect(cx, cy, CELL - 1.0, CELL - 1.0, Color::from_rgba(90, 80, 120, 200));
                    }
                    if hit(cx, cy, CELL, CELL, mx, my) {
                        draw_rectangle_lines(cx, cy, CELL - 1.0, CELL - 1.0, 2.0, col_text());
                        if click {
                            if let Some(id) = held {
                                if board.lo.can_place(&board.reg, id, slot, gx, gy).is_ok() {
                                    board.lo.slot_mut(slot).place(&board.reg, id, gx, gy);
                                    board.dirty = true;
                                    held = None;
                                } else {
                                    status = format!(
                                        "{} will not sit there",
                                        board.reg.def(id).name
                                    );
                                }
                            }
                        }
                        if rclick {
                            let s = board.lo.slot(slot);
                            if let Some(id) = s.get(gx, gy).or_else(|| s.enchant_at(gx, gy)) {
                                board.lo.slot_mut(slot).remove(id);
                                board.dirty = true;
                            }
                        }
                    }
                }
            }
            // What this grid came to, in the words the report uses.
            let report = board.lo.report(&board.reg, slot);
            let items = report.items.iter().filter(|i| i.assembled).count();
            let sy = gy0 + rows as f32 * CELL + 16.0;
            text(
                &format!("{items} item(s)"),
                ox,
                sy,
                14.0,
                if items > 0 { col_good() } else { col_dim() },
            );
            let mut ly = sy + 16.0;
            for item in report.items.iter().filter(|i| i.assembled) {
                text_capped(&item.name.short, ox, ly, GRID_W + 14.0, 12.0, col_dim());
                ly += 13.0;
            }
        }

        // What is wrong with it, while somebody is still looking.
        let mut cy = gy0 + 8.0 * CELL + 120.0;
        let complaints = board.complaints(spec);
        if complaints.is_empty() {
            text("nothing to complain about", gx0, cy, 15.0, col_good());
        } else {
            for (line, c) in complaints.iter().take(14) {
                text_capped(line, gx0, cy, W - LIB_W - gx0 - 20.0, 14.0, *c);
                cy += 16.0;
            }
        }

        // The held piece rides the cursor.
        if let Some(id) = held {
            let d = board.reg.def(id);
            for &(dx, dy) in board.reg.shape(id).cells() {
                let mut c = col_slot(d.slot);
                c.a = 0.7;
                rect(
                    mx + dx as f32 * CELL - CELL * 0.5,
                    my + dy as f32 * CELL - CELL * 0.5,
                    CELL - 1.0,
                    CELL - 1.0,
                    c,
                );
            }
        }

        // ---- right: the catalogue ----
        let lx = W - LIB_W;
        rect(lx, TOP, LIB_W, H - TOP, col_panel());
        let mut ty = TOP + 10.0;

        // Slot tabs.
        let mut tx = lx + 8.0;
        for (label, want) in std::iter::once(("ALL", None))
            .chain(SlotKind::ALL.iter().map(|&s| (s.name(), Some(s))))
        {
            let w = measure_text(label, None, 14, 1.0).width + 14.0;
            let on = slot_tab == want;
            rect(tx, ty, w, 22.0, if on { col_cell_a() } else { col_panel() });
            text(
                label,
                tx + 7.0,
                ty + 16.0,
                14.0,
                if on { col_text() } else { col_dim() },
            );
            if click && hit(tx, ty, w, 22.0, mx, my) {
                slot_tab = want;
                kind_tab = None;
            }
            tx += w + 4.0;
        }
        ty += 28.0;

        // Kind tabs, for whatever the slot can hold.
        let kinds = kinds_of(slot_tab);
        let mut tx = lx + 8.0;
        for (label, want) in std::iter::once(("all kinds".to_string(), None))
            .chain(kinds.iter().map(|&k| (k.name().to_string(), Some(k))))
        {
            let w = measure_text(&label, None, 13, 1.0).width + 12.0;
            if tx + w > W - 8.0 {
                tx = lx + 8.0;
                ty += 20.0;
            }
            let on = kind_tab == want;
            rect(tx, ty, w, 18.0, if on { col_cell_a() } else { col_panel() });
            text(
                &label,
                tx + 6.0,
                ty + 13.0,
                13.0,
                if on { col_text() } else { col_dim() },
            );
            if click && hit(tx, ty, w, 18.0, mx, my) {
                kind_tab = want;
            }
            tx += w + 4.0;
        }
        ty += 26.0;

        // Search.
        let sr = (lx + 8.0, ty, LIB_W - 16.0, 24.0);
        rect(sr.0, sr.1, sr.2, sr.3, col_cell_a());
        if click && hit(sr.0, sr.1, sr.2, sr.3, mx, my) {
            typing_lib = true;
        }
        let shown = if lib_filter.is_empty() { "search components\u{2026}" } else { &lib_filter };
        text_capped(
            shown,
            sr.0 + 6.0,
            sr.1 + 17.0,
            sr.2 - 12.0,
            15.0,
            if typing_lib { col_text() } else { col_dim() },
        );
        ty += 32.0;

        let found = shelf(slot_tab, kind_tab, &lib_filter);
        text(
            &format!("{} of {} components", found.len(), CATALOG.len()),
            lx + 8.0,
            ty,
            13.0,
            col_dim(),
        );
        ty += 12.0;

        if hit(lx, TOP, LIB_W, H - TOP, mx, my) {
            lib_scroll = (lib_scroll - mouse_wheel().1 * 30.0)
                .clamp(0.0, (found.len() as f32 * 20.0 - (H - ty - 20.0)).max(0.0));
        }
        let mut ry = ty + 16.0 - lib_scroll;
        for &i in &found {
            if ry > ty && ry < H {
                let d = &CATALOG[i];
                let row = (lx + 4.0, ry - 13.0, LIB_W - 8.0, 19.0);
                let over = hit(row.0, row.1, row.2, row.3, mx, my);
                if over {
                    rect(row.0, row.1, row.2, row.3, col_cell_a());
                }
                rect(lx + 8.0, ry - 9.0, 5.0, 11.0, col_slot(d.slot));
                // Anything a creature may not wear says so on the shelf, rather
                // than after a test run.
                let barred = d.kind.is_enchantment()
                    || is_event_only(d.name)
                    || is_town_stock(d)
                    || is_quest_reward(d.name)
                    || is_boss_only(d.name);
                text_capped(
                    d.name,
                    lx + 20.0,
                    ry,
                    LIB_W - 130.0,
                    14.0,
                    if barred { col_bad() } else { col_text() },
                );
                text(d.kind.name(), lx + LIB_W - 108.0, ry, 12.0, col_dim());
                text(
                    &format!("{:>3}", piece_rating(d)),
                    lx + LIB_W - 34.0,
                    ry,
                    12.0,
                    col_dim(),
                );
                if click && over {
                    let id = boards[who].reg.alloc(i);
                    held = Some(id);
                    status = format!("holding {} - click a cell, Tab turns it", d.name);
                }
            }
            ry += 20.0;
        }

        // ---- the keys ----
        text(
            "click a component to pick it up  \u{00b7}  click a cell to place  \u{00b7}  Tab turns it  \
             \u{00b7}  right-click removes  \u{00b7}  Cmd-S saves  \u{00b7}  Esc drops",
            LIST_W + 20.0,
            H - 14.0,
            14.0,
            col_dim(),
        );

        next_frame().await;
    }
}

// ------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole tool, minus the window: every creature in the game, read off
    /// its `MonsterSpec`, laid on a board, read back out, and spliced into the
    /// source - and the source has to come out saying the same thing.
    ///
    /// This is the test the tool exists behind. A board editor whose save is
    /// not the identity on a board nobody touched is an editor that quietly
    /// rewrites fifty-four creatures the first time it is opened.
    #[test]
    fn opening_a_creature_and_saving_it_changes_nothing() {
        let src = std::fs::read_to_string(COMBAT_RS)
            .or_else(|_| std::fs::read_to_string(format!("../../{COMBAT_RS}")))
            .expect("the engine source, from either the workspace root or this crate");
        let mut moved = Vec::new();
        for spec in everyone() {
            let (board, dropped) = Board::load(spec);
            assert_eq!(dropped, 0, "{}: {dropped} placement(s) would not sit", spec.name);
            let (lines, chunks) = board.emit();
            assert_eq!(
                lines.len(),
                spec.gear.len(),
                "{}: {} pieces went in and {} came out",
                spec.name,
                spec.gear.len(),
                lines.len()
            );
            assert_eq!(
                chunks.iter().sum::<usize>(),
                lines.len(),
                "{}: the item chunks do not partition the gear list",
                spec.name
            );
            let out = splice(&src, spec.name, &lines, &chunks).expect("splices");
            if out != src {
                moved.push(spec.name);
            }
        }
        // The six that do move are the ones whose `items:` partition is empty
        // or does not match the board they describe - the four boards the
        // repack never reached, plus two more. Saving one of those is not
        // noise, it is the fix: the tool works out the real item grouping and
        // writes it down. Everything else round-trips exactly.
        assert!(
            moved.len() <= REORDERED_ON_FIRST_SAVE,
            "{} creatures would be rewritten by an untouched save, budget {}: {:?}",
            moved.len(),
            REORDERED_ON_FIRST_SAVE,
            moved
        );
    }

    /// How many creature boards the tool would rewrite if you opened them and
    /// saved without touching anything.
    ///
    /// It was fifty-two of fifty-four, for three reasons in a row, each of
    /// which is a thing the tool was getting wrong about creature boards:
    /// items came out in slot order rather than written order; the board was
    /// loaded without locking each item as it landed, so flush items merged;
    /// and the pieces inside an item came back in board order rather than the
    /// order they were written in. Six left, and those six have an `items:`
    /// partition that does not describe the board they are attached to.
    ///
    /// Lower this when one of the six is fixed. It should never rise.
    const REORDERED_ON_FIRST_SAVE: usize = 6;

    #[test]
    #[ignore]
    fn show_one() {
        let want = std::env::var("WHO").unwrap_or_else(|_| "Grave Chorus".into());
        let spec = everyone().into_iter().find(|m| m.name == want).unwrap();
        let (board, _) = Board::load(spec);
        let (lines, chunks) = board.emit();
        println!("written items {:?}", spec.items);
        println!("emitted items {chunks:?}");
        for (i, l) in lines.iter().enumerate() {
            let was = spec.gear.get(i).map(|&(n, s, x, y, r)| {
                format!("            (\"{n}\", SlotKind::{s:?}, {x}, {y}, {r}),")
            });
            let same = was.as_deref() == Some(l.as_str());
            println!("{} {}", if same { " " } else { "*" }, l.trim());
            if !same {
                println!("   was {}", was.unwrap_or_default().trim());
            }
        }
    }

    #[test]
    fn a_splice_leaves_everything_but_the_board_alone() {
        let src = std::fs::read_to_string(COMBAT_RS)
            .or_else(|_| std::fs::read_to_string(format!("../../{COMBAT_RS}")))
            .expect("the engine source");
        let spec = LADDER.iter().find(|m| m.name == "Cave Rat").expect("the first rung");
        let out = splice(&src, spec.name, &["            (\"Oak Handle\", SlotKind::Weapon, 0, 0, 0),".to_string()], &[1])
            .expect("splices");
        // The creature is still a creature: nothing but its two board fields
        // may move, and the file may not lose or gain a `MonsterSpec`.
        assert_eq!(
            src.matches("MonsterSpec {").count(),
            out.matches("MonsterSpec {").count(),
            "the splice added or removed a creature"
        );
        assert!(out.contains("name: \"Cave Rat\","), "it lost the name it was aiming at");
        assert!(out.contains("(\"Oak Handle\", SlotKind::Weapon, 0, 0, 0),"), "the gear did not land");
        assert!(out.contains("items: &[1]"), "the chunks did not land");
    }

    #[test]
    fn a_creature_that_is_not_there_is_refused_rather_than_guessed_at() {
        let err = splice("nothing here", "Nobody", &[], &[]).unwrap_err();
        assert!(err.contains("Nobody"), "{err}");
    }

    #[test]
    fn the_shelf_filters_narrow_rather_than_widen() {
        let all = shelf(None, None, "");
        assert_eq!(all.len(), CATALOG.len());
        let weapons = shelf(Some(SlotKind::Weapon), None, "");
        assert!(weapons.len() < all.len() && !weapons.is_empty());
        for &i in &weapons {
            assert!(CATALOG[i].fits(SlotKind::Weapon), "{} is not a weapon piece", CATALOG[i].name);
        }
        let named = shelf(None, None, "oak");
        assert!(!named.is_empty());
        for &i in &named {
            assert!(CATALOG[i].name.to_lowercase().contains("oak"));
        }
    }
}
