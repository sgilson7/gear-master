//! The screen, in fields.
//!
//! Every field below is something one of the two shipped interfaces draws, and
//! the table in `tests/view.rs` names which. Nothing here is read out of a
//! table the player cannot see - which is why there is no `MonsterSpec`, no
//! catalogue index and no ladder beyond the next rung.
//!
//! ## The one that had to be checked
//!
//! `Run::monster()` hands back the coming creature's whole spec, gear
//! included, and the spec for this mission listed that as a leak to close.
//! **It is not one.** `gui/src/main.rs:4768` draws "WHAT THEY BRING" - every
//! item the creature will swing - and `:4803` draws its whole board, under a
//! comment saying the panel exists so you can shop against what is coming and
//! that withholding half of it would defeat the point. So `Coming` carries the
//! creature's stats, its items and its board, because that is what is on the
//! screen.
//!
//! What it does **not** carry is the rest of the ladder. The CLI's `ladder`
//! verb prints every creature's outfit at every rung; the GUI shows only the
//! next one. The two interfaces disagree, and this takes the GUI's answer,
//! because the GUI is the game people play and because being told *less* than
//! a player can only make a reachability claim stronger.

use gearmaster_engine::piece::{PieceId, PieceKind, SlotKind};
use gearmaster_engine::stats::Stats;

/// A board's six figures - what it does a second, in thousandths.
///
/// Mirrored rather than re-exported: `loadout::Figures` is constructed from
/// `ItemProfile`s, and handing an agent the constructor would hand it a way to
/// score boards it is not standing in front of. The county tab draws these
/// numbers, so the agent may read them.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Figures {
    pub flow: i64,
    pub physical_dps: i64,
    pub magic_dps: i64,
    pub armour_ps: i64,
    pub fastest_ms: Option<u32>,
    pub curse_resist: i32,
}

/// One cell of one grid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub piece: Option<PieceId>,
    /// Which assembled item this cell belongs to, if any.
    pub item: Option<usize>,
}

/// One way of building an item in this slot.
///
/// What the `?` beside a grid draws (`gui/src/main.rs:6183`). A player is told
/// the recipes, so an agent is too - the alternative is an agent that has to
/// rediscover by trial what the interface prints on a tooltip.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub title: String,
    /// The minimum that makes an item.
    pub required: Vec<String>,
    /// What may be added beyond it.
    pub optional: Vec<String>,
}

/// One of the five grids.
#[derive(Clone, Debug)]
pub struct Grid {
    pub slot: SlotKind,
    pub rows: u8,
    /// The ways an item can be built here.
    pub recipes: Vec<Recipe>,
    /// Row-major, `rows` rows of `SLOT_W`.
    pub cells: Vec<Cell>,
    /// What the slot panel says: "2 items", "unfinished", and so on.
    pub summary: String,
    /// What the loose and assembled pieces here contribute.
    pub stats: Stats,
    pub items: Vec<Item>,
}

/// An item, finished or not, as the slot panel lists it.
#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub assembled: bool,
    pub locked: bool,
    pub status: String,
    pub stats: Stats,
    pub pieces: Vec<PieceId>,
    pub notes: Vec<String>,
}

/// A piece, as the tray and the shelf draw one.
///
/// `stats` is the whole block and `when` is the same block grouped the way the
/// card groups it - damage, passive, and what one activation hands over.
/// The split is not cosmetic: `+2 nature` on a 2.8-second item is a *rate* and
/// `+175 hp` is a quantity, and the card used to print them in one line and one
/// colour. A packer that reads them as the same kind of number prices a board
/// wrong, so the console carries the classification the interface draws rather
/// than making an agent guess it back.
#[derive(Clone, Debug)]
pub struct Piece {
    pub id: Option<PieceId>,
    pub name: String,
    pub slot: SlotKind,
    pub kind: PieceKind,
    /// What the kind is called in this slot - "handle", "spell", "plating".
    pub role: String,
    pub width: u8,
    pub height: u8,
    pub cells: u8,
    pub stats: Stats,
    /// The stat block in the card's four groups: `("damage"|"passive"|
    /// "on activation", the figures)`.
    pub when: Vec<(String, String)>,
    pub price: i32,
    pub triggers: Vec<String>,
    pub effect: Option<String>,
    pub assembly_bonus: Option<String>,
}

/// A shelf, with what it would cost.
#[derive(Clone, Debug)]
pub struct Shelf {
    pub index: usize,
    pub piece: Piece,
    pub price: Option<i32>,
    pub pinned: bool,
    pub affordable: bool,
    /// What could be handed over instead of gold.
    pub barter: Vec<PieceId>,
}

/// One choice at a door.
#[derive(Clone, Debug)]
pub struct Choice {
    pub index: usize,
    pub label: String,
    pub blurb: String,
    pub open: bool,
    /// What it asks for, said plainly. Drawn under a shut choice.
    pub requires: String,
    pub unmet: String,
    /// Set on the one door that asks for a number.
    pub figure: Option<(i32, i32)>,
}

/// Whatever is asking something.
#[derive(Clone, Debug)]
pub struct Question {
    pub id: String,
    pub title: String,
    pub scene: Vec<String>,
    pub choices: Vec<Choice>,
}

/// A gate, and the doors behind it.
#[derive(Clone, Debug)]
pub struct Town {
    pub name: String,
    pub blurb: Vec<String>,
    pub doors: Vec<(gearmaster_engine::town::Action, String)>,
}

/// One rung's worth of road, as the banner draws it.
#[derive(Clone, Debug)]
pub struct RoadItem {
    pub kind: String,
    pub describe: String,
}

/// Standing at a set of points.
#[derive(Clone, Debug)]
pub struct Points {
    pub fork: Vec<String>,
    pub exits: Vec<(usize, String, String, bool)>,
}

/// Standing in THE HUNDRED.
#[derive(Clone, Debug)]
pub struct County {
    pub at: (u8, u8),
    pub reference: String,
    pub here: String,
    pub moves_left: u8,
    /// North, south, east, west: where it goes and what is there.
    pub around: Vec<(String, Option<Neighbour>)>,
    pub trips_left: usize,
    pub clock: usize,
    pub figures: Figures,
    pub checklist: Vec<(String, bool)>,
}

#[derive(Clone, Debug)]
pub struct Neighbour {
    pub at: (u8, u8),
    pub reference: String,
    pub what: String,
    pub cleared: bool,
    pub sealed: bool,
    /// The toll, if it can be read from where you are standing.
    pub threshold: Option<String>,
}

/// What is coming, drawn the way the portrait card draws it.
#[derive(Clone, Debug)]
pub struct Coming {
    pub name: String,
    pub rung_shown: usize,
    pub stats: Stats,
    /// Every item it will swing, by name and cadence.
    pub brings: Vec<(String, u32)>,
    pub innate: Vec<String>,
    pub bounty: i32,
}

/// The fight that just happened.
#[derive(Clone, Debug)]
pub struct Fight {
    pub outcome: String,
    pub won: bool,
    pub duration_ms: u32,
    pub board_decided: bool,
    pub against: String,
    pub entries: usize,
    pub health_left: i32,
    pub enemy_health_left: i32,
}

/// A fountain, and what it is offering.
#[derive(Clone, Debug)]
pub struct Fountain {
    pub doubling: bool,
    pub offer: Vec<(String, String)>,
}

/// The whole screen.
#[derive(Clone, Debug)]
pub struct View {
    pub rung_shown: usize,
    pub gold: i32,
    pub wins: u32,
    pub losses: u32,
    pub lives_left: Option<u32>,
    pub grinder: bool,
    pub fighting: bool,
    pub over: bool,
    pub classes: Vec<String>,
    pub stats: Stats,
    pub figures: Figures,
    pub grids: Vec<Grid>,
    pub tray: Vec<Piece>,
    pub tray_cap: usize,
    pub shop: Vec<Shelf>,
    pub reroll_cost: i32,
    pub road: Vec<RoadItem>,
    pub blocked: Option<String>,
    pub question: Option<Question>,
    pub town: Option<Town>,
    pub points: Option<Points>,
    pub county: Option<County>,
    pub fountain: Option<Fountain>,
    pub in_dungeon: bool,
    pub brawl_waiting: bool,
    pub coming: Coming,
    pub last_fight: Option<Fight>,
    pub receipt: Vec<String>,
    pub undoable: Option<String>,
}
