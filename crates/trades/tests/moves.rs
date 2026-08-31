//! The quartermaster can tell its own verbs apart.
//!
//! `trades::quest` has the same ratchet for the pathfinder, and it exists
//! because `feature::mv` sorted every road verb into one bucket: 1,341 verbs
//! offered, four kinds among them, **one** distinct vector. That was written up
//! as the road agent's fault, because `mv` is the *quartermaster's* move
//! description and it was the road borrowing it.
//!
//! Nobody asked what fell into the same bucket from the packing side. Four
//! verbs did - `Lock`, `Grow`, `Undo` and `Pin` - and none of them is in the
//! `piece` match either, so all four were the same thirty-two numbers. Measured
//! over forty episodes (`--bin qhand`, `QHAND_KEYS`): locking was offered on
//! 6,198 decisions and scored identically to pinning a shop shelf on **every
//! one of them**, and `Iterator::max_by` returns the last maximum, so the agent
//! pressed that vector 1,653 times and the console executed `Pin` every time.
//! It never locked once, in 7,968 presses.
//!
//! This is the lint that would have said so on the day.

use gearmaster_console::{Console, Difficulty, Mode, PieceId, SlotKind, Verb};
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::feature;

/// A real board with a real assembled item on it.
///
/// Built by pressing the first key on the menu until something assembles,
/// rather than by hand: `View` is the screen and a hand-built one is a claim
/// about the screen rather than a reading of it.
fn a_board_with_an_item() -> Console {
    for seed in [0xC0FFEEu64, 0x1212, 0x0D0E_5EED, 0x1111] {
        let mut c = Console::start(seed, Mode::Rogue, Difficulty::Medium);
        let mut e = Packing::new(200);
        for _ in 0..200 {
            if c.view().grids.iter().any(|g| g.items.iter().any(|i| i.assembled)) {
                return c;
            }
            let ms = e.moves(&c);
            let Some(at) = ms.iter().position(|m| matches!(m, Move::Press(_))) else { break };
            e.step(&mut c, ms[at]);
        }
        if c.view().grids.iter().any(|g| g.items.iter().any(|i| i.assembled)) {
            return c;
        }
    }
    panic!("no seed assembled an item, so this file is testing nothing");
}

/// The item on that board, and one of its pieces - what `Lock` is offered on.
fn an_assembled_piece(c: &Console) -> PieceId {
    c.view()
        .grids
        .iter()
        .flat_map(|g| g.items.iter())
        .find(|i| i.assembled)
        .and_then(|i| i.pieces.first().copied())
        .expect("a board with an item on it")
}

/// Every verb the quartermaster owns, as `mv` would be asked to describe it.
///
/// Constructed rather than taken from the menu, because the menu offers `Lock`
/// only when something is assembled and `Grow` only when a row is owed, and the
/// claim being made is about the description rather than about the offer.
fn every_kind(p: PieceId) -> Vec<(&'static str, Verb)> {
    vec![
        ("place", Verb::Place { piece: p, slot: SlotKind::Weapon, x: 0, y: 0 }),
        ("buy", Verb::Buy { shelf: 0 }),
        ("sell", Verb::Sell { piece: p }),
        ("barter", Verb::Barter { shelf: 0, paying: p }),
        ("reroll", Verb::Reroll),
        ("rotate", Verb::Rotate { piece: p }),
        ("unequip", Verb::Unequip { piece: p }),
        ("clear", Verb::ClearSlot { slot: SlotKind::Weapon }),
        ("lock", Verb::Lock { piece: p }),
        ("grow", Verb::Grow { slot: SlotKind::Weapon }),
        ("undo", Verb::Undo),
        ("pin", Verb::Pin { shelf: 0 }),
    ]
}

#[test]
fn no_two_of_the_quartermasters_verbs_describe_identically() {
    let c = a_board_with_an_item();
    let v = c.view();
    let p = an_assembled_piece(&c);
    let seen: Vec<(&str, [f32; feature::MOVE])> =
        every_kind(p).into_iter().map(|(n, m)| (n, feature::mv(&v, m))).collect();
    let mut same: Vec<String> = Vec::new();
    for (i, (an, a)) in seen.iter().enumerate() {
        for (bn, b) in seen.iter().skip(i + 1) {
            if a == b {
                same.push(format!("{an} and {bn}"));
            }
        }
    }
    assert!(
        same.is_empty(),
        "these describe identically, so no network can prefer one: {}",
        same.join(", ")
    );
}

/// The pair this mission was actually lost to.
///
/// Kept as its own test with its own name, because a general lint reads as
/// housekeeping and this one is a finding: the agent pressed the shared vector
/// 1,653 times and got `Pin` on every one of them.
#[test]
fn locking_an_item_does_not_look_like_pinning_a_shop_shelf() {
    let c = a_board_with_an_item();
    let v = c.view();
    let lock = feature::mv(&v, Verb::Lock { piece: an_assembled_piece(&c) });
    let pin = feature::mv(&v, Verb::Pin { shelf: 0 });
    assert_ne!(lock, pin, "6,198 decisions out of 6,198 scored these equal");
}

/// And a lock says *which* item it would fix.
///
/// A kind alone would separate it from pinning and still leave every lock on
/// the board identical, which is the same fault one level down: what decides
/// whether locking is worth a press is how much there is to lose and how
/// crowded the grid is.
#[test]
fn two_different_locks_do_not_describe_identically() {
    let c = a_board_with_an_item();
    let v = c.view();
    let real = feature::mv(&v, Verb::Lock { piece: an_assembled_piece(&c) });
    // A piece that is on no board at all: the same verb with nothing to fix.
    let nothing = feature::mv(&v, Verb::Lock { piece: PieceId(9999) });
    assert_ne!(
        real, nothing,
        "every lock on every board is the same press unless it says what it would fix"
    );
}
