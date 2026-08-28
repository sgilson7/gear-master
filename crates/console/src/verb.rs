//! Everything a player can do, and nothing else.
//!
//! One variant per affordance the two shipped interfaces offer, plus the four
//! the engine has and neither interface reached (`ClearSlot`, `Crush`,
//! `Grow`, `Perambulate` - see `tests/parity.rs`, which is what found them).
//!
//! **What is deliberately absent** is as much of the definition as what is
//! here: `force_win`, `skip_to`, `skip_fight`, `with_all_pieces`,
//! `apply_preset`, `wipe`, `give`, `grant_life`, `grant_quest`, `begin_fight`
//! and `set_theme` are all `pub` on `Run` and none of them is a verb. A pilot
//! holding a `Verb` cannot spell them, because a `Verb` is the only thing it
//! can hand to a `Console`.
//!
//! ## Rotation is a verb, not a coordinate
//!
//! `design/rl-agent-plan.md` §3 encodes a placement as
//! `{tray, slot, x, y, rot}`. A player has no such action: they turn the piece
//! in their hand and then put it down, which is two presses. Folding the
//! rotation into the placement would hand the agent an action the game does
//! not have, so `Place` carries no rotation and `Rotate` is its own verb. The
//! search may still explore all four - it costs it one extra step, exactly as
//! it costs a person one.

use gearmaster_engine::county::Step;
use gearmaster_engine::piece::{PieceId, SlotKind};
use gearmaster_engine::town::Action as Door;

/// One press.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    // ---- the board -------------------------------------------------------
    /// Put a loose piece down at an anchor.
    Place { piece: PieceId, slot: SlotKind, x: u8, y: u8 },
    /// Put a whole assembled item down, as one shape.
    PlaceLocked { piece: PieceId, slot: SlotKind, x: u8, y: u8 },
    /// Back to the tray.
    Unequip { piece: PieceId },
    /// Lift a whole assembled item off the board.
    UnequipLocked { piece: PieceId },
    /// A quarter turn.
    Rotate { piece: PieceId },
    /// A quarter turn of a whole assembled item.
    RotateLocked { piece: PieceId },
    /// Lock an assembled item, or unlock it again.
    Lock { piece: PieceId },
    /// Empty one grid.
    ClearSlot { slot: SlotKind },
    /// Empty all five.
    ClearAll,
    /// Take back the last board change. Depth is `UNDO_DEPTH`.
    Undo,
    /// Spend an owed row on one grid.
    Grow { slot: SlotKind },

    // ---- the shop --------------------------------------------------------
    Buy { shelf: usize },
    Sell { piece: PieceId },
    /// Pay for a shelf with a piece instead of with gold.
    Barter { shelf: usize, paying: PieceId },
    Reroll,
    /// Hold a shelf across a restock.
    Pin { shelf: usize },

    // ---- the road --------------------------------------------------------
    /// Take choice `n` at whatever is asking.
    Answer { choice: usize },
    /// The one door that asks for a number.
    AnswerWith { choice: usize, figure: i32 },
    /// The rung's own creature.
    Fight,
    /// A fight an event arranged, standing beside the rung.
    FightParty,

    // ---- towns -----------------------------------------------------------
    Town { door: Door },
    /// Walk past the gate and take the bounty again.
    WalkOn,

    // ---- dungeons --------------------------------------------------------
    //
    // **There is no `Enter`.** `tests/parity.rs` refused one: nothing in either
    // interface calls `Run::enter_dungeon`, because a run never walks into a
    // dungeon. It is put in one - by answering a door whose outcome is
    // `Enter`, by the cellar door at a town, or by feeding an orb to a
    // pedestal - so the verb that gets you in is `Answer`, `Town` or
    // `Pedestal`, and the only dungeon verbs are the ones for being inside it.
    /// At a set of points, take road `n`.
    ThrowPoints { exit: usize },
    /// Out of a dungeon, keeping what was cleared.
    Leave,

    // ---- THE HUNDRED -----------------------------------------------------
    /// One tile.
    Walk { step: Step },
    /// Back up to the road.
    Out,
    /// Set out on a route rather than to a destination.
    Perambulate { mouth: (u8, u8) },

    // ---- fountains -------------------------------------------------------
    /// Take what the fountain offers, its own way.
    Drink,
    /// Take the `n`th of what it is offering.
    DrinkChoosing { class: usize },
    /// The third fountain doubles one you hold.
    Double { class: usize },

    // ---- the workshop ----------------------------------------------------
    /// Feed an Orb of Travel to a pedestal.
    Pedestal { piece: PieceId },
    /// Break a relic for what is inside it.
    Crush { piece: PieceId },
}

impl Verb {
    /// The group this verb belongs to, for a screen that wants to sort them.
    pub fn group(self) -> &'static str {
        match self {
            Verb::Place { .. }
            | Verb::PlaceLocked { .. }
            | Verb::Unequip { .. }
            | Verb::UnequipLocked { .. }
            | Verb::Rotate { .. }
            | Verb::RotateLocked { .. }
            | Verb::Lock { .. }
            | Verb::ClearSlot { .. }
            | Verb::ClearAll
            | Verb::Undo
            | Verb::Grow { .. } => "board",
            Verb::Buy { .. }
            | Verb::Sell { .. }
            | Verb::Barter { .. }
            | Verb::Reroll
            | Verb::Pin { .. } => "shop",
            Verb::Answer { .. } | Verb::AnswerWith { .. } => "door",
            Verb::Fight | Verb::FightParty => "fight",
            Verb::Town { .. } | Verb::WalkOn => "town",
            Verb::ThrowPoints { .. } | Verb::Leave => "dungeon",
            Verb::Walk { .. } | Verb::Out | Verb::Perambulate { .. } => "county",
            Verb::Drink | Verb::DrinkChoosing { .. } | Verb::Double { .. } => "fountain",
            Verb::Pedestal { .. } | Verb::Crush { .. } => "workshop",
        }
    }
}

/// The canonical spelling of a slot, for a transcript.
pub fn slot_key(k: SlotKind) -> &'static str {
    match k {
        SlotKind::Helmet => "helmet",
        SlotKind::Chest => "chest",
        SlotKind::Gloves => "gloves",
        SlotKind::Greaves => "greaves",
        SlotKind::Weapon => "weapon",
    }
}

pub fn parse_slot(s: &str) -> Option<SlotKind> {
    Some(match s.to_ascii_lowercase().as_str() {
        "helmet" | "helm" | "h" => SlotKind::Helmet,
        "chest" | "chestpiece" | "c" => SlotKind::Chest,
        "gloves" | "glove" | "g" => SlotKind::Gloves,
        "greaves" | "greave" | "r" => SlotKind::Greaves,
        "weapon" | "w" => SlotKind::Weapon,
        _ => return None,
    })
}

/// The canonical spelling of a town door.
pub fn door_key(d: Door) -> String {
    format!("{:?}", d).to_ascii_lowercase()
}

pub fn parse_door(s: &str) -> Option<Door> {
    let s = s.to_ascii_lowercase();
    // The aliases the CLI has always taken, kept so old scripts still replay.
    let s = match s.as_str() {
        "works" => "factory",
        "cart" => "shop",
        "socket" => "pedestal",
        "steps" | "down" => "county",
        other => other,
    };
    Door::EVERY.into_iter().find(|d| door_key(*d) == s)
}

/// The county's own spelling, so `walk` and the map agree.
fn step_key(s: Step) -> &'static str {
    s.key()
}

impl Verb {
    /// One line of a transcript: what a person would type.
    ///
    /// Pieces are addressed by `#id` rather than by name because two pieces in
    /// one tray can share a name and a transcript that picks the wrong one is
    /// not a replay. `Console::line_for` writes the name after it as a
    /// comment, so the file is still readable.
    pub fn line(self) -> String {
        match self {
            Verb::Place { piece, slot, x, y } => {
                format!("place #{} {} {} {}", piece.0, slot_key(slot), x, y)
            }
            Verb::PlaceLocked { piece, slot, x, y } => {
                format!("placelocked #{} {} {} {}", piece.0, slot_key(slot), x, y)
            }
            Verb::Unequip { piece } => format!("unequip #{}", piece.0),
            Verb::UnequipLocked { piece } => format!("unequiplocked #{}", piece.0),
            Verb::Rotate { piece } => format!("rotate #{}", piece.0),
            Verb::RotateLocked { piece } => format!("rotatelocked #{}", piece.0),
            Verb::Lock { piece } => format!("lock #{}", piece.0),
            Verb::ClearSlot { slot } => format!("clear {}", slot_key(slot)),
            Verb::ClearAll => "clear".into(),
            Verb::Undo => "undo".into(),
            Verb::Grow { slot } => format!("grow {}", slot_key(slot)),
            Verb::Buy { shelf } => format!("buy {}", shelf),
            Verb::Sell { piece } => format!("sell #{}", piece.0),
            Verb::Barter { shelf, paying } => format!("barter {} #{}", shelf, paying.0),
            Verb::Reroll => "reroll".into(),
            Verb::Pin { shelf } => format!("pin {}", shelf),
            Verb::Answer { choice } => format!("answer {}", choice),
            Verb::AnswerWith { choice, figure } => format!("answer {} {}", choice, figure),
            Verb::Fight => "fight".into(),
            Verb::FightParty => "brawl".into(),
            Verb::Town { door } => format!("town {}", door_key(door)),
            Verb::WalkOn => "town on".into(),
            Verb::ThrowPoints { exit } => format!("throw {}", exit),
            Verb::Leave => "leave".into(),
            Verb::Walk { step } => format!("walk {}", step_key(step)),
            Verb::Out => "out".into(),
            Verb::Perambulate { mouth } => format!("perambulate {} {}", mouth.0, mouth.1),
            Verb::Drink => "drink".into(),
            Verb::DrinkChoosing { class } => format!("drink {}", class),
            Verb::Double { class } => format!("double {}", class),
            Verb::Pedestal { piece } => format!("pedestal #{}", piece.0),
            Verb::Crush { piece } => format!("crush #{}", piece.0),
        }
    }

    /// Read a transcript line back. `None` is "that is not a verb", which is
    /// not the same as "that verb is illegal here" - the console answers the
    /// second question.
    pub fn parse(line: &str) -> Option<Verb> {
        // `;` starts a comment, because `#` is already the piece marker.
        let raw = line.split(';').next().unwrap_or("").trim();
        let parts: Vec<&str> = raw.split_whitespace().collect();
        let id = |s: &str| -> Option<PieceId> {
            s.strip_prefix('#').and_then(|n| n.parse().ok()).map(PieceId)
        };
        Some(match parts.as_slice() {
            ["place", p, sl, x, y] => Verb::Place {
                piece: id(p)?,
                slot: parse_slot(sl)?,
                x: x.parse().ok()?,
                y: y.parse().ok()?,
            },
            ["placelocked", p, sl, x, y] => Verb::PlaceLocked {
                piece: id(p)?,
                slot: parse_slot(sl)?,
                x: x.parse().ok()?,
                y: y.parse().ok()?,
            },
            ["unequip", p] => Verb::Unequip { piece: id(p)? },
            ["unequiplocked", p] => Verb::UnequipLocked { piece: id(p)? },
            ["rotate", p] => Verb::Rotate { piece: id(p)? },
            ["rotatelocked", p] => Verb::RotateLocked { piece: id(p)? },
            ["lock", p] => Verb::Lock { piece: id(p)? },
            ["clear"] => Verb::ClearAll,
            ["clear", sl] => Verb::ClearSlot { slot: parse_slot(sl)? },
            ["undo"] => Verb::Undo,
            ["grow", sl] => Verb::Grow { slot: parse_slot(sl)? },
            ["buy", n] => Verb::Buy { shelf: n.parse().ok()? },
            ["sell", p] => Verb::Sell { piece: id(p)? },
            ["barter", n, p] => Verb::Barter { shelf: n.parse().ok()?, paying: id(p)? },
            ["reroll"] => Verb::Reroll,
            ["pin", n] => Verb::Pin { shelf: n.parse().ok()? },
            ["answer", n] => Verb::Answer { choice: n.parse().ok()? },
            ["answer", n, f] => {
                Verb::AnswerWith { choice: n.parse().ok()?, figure: f.parse().ok()? }
            }
            ["fight"] => Verb::Fight,
            ["brawl"] => Verb::FightParty,
            ["town", "on"] => Verb::WalkOn,
            ["town", d] => Verb::Town { door: parse_door(d)? },
            ["throw", n] => Verb::ThrowPoints { exit: n.parse().ok()? },
            ["leave"] => Verb::Leave,
            ["walk", d] => Verb::Walk { step: Step::parse(d)? },
            ["out"] => Verb::Out,
            ["perambulate", x, y] => {
                Verb::Perambulate { mouth: (x.parse().ok()?, y.parse().ok()?) }
            }
            ["drink"] => Verb::Drink,
            ["drink", n] => Verb::DrinkChoosing { class: n.parse().ok()? },
            ["double", n] => Verb::Double { class: n.parse().ok()? },
            ["pedestal", p] => Verb::Pedestal { piece: id(p)? },
            ["crush", p] => Verb::Crush { piece: id(p)? },
            _ => return None,
        })
    }
}
