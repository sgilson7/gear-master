//! The pilot: an agent that plays through the console and nothing else.
//!
//! A0 measured the ground; A1 built the door. What lives here now is the
//! smallest thing that proves the door works - `starter`, which seats what a
//! run begins with and fights until it stops getting anywhere. It is the
//! `starter` row of the baseline printer played through the economy for the
//! first time in this repo's life, and its clear rate is meant to be dismal:
//! it is the control every later milestone is measured against.
//!
//! It uses exactly two pieces of judgement, and both are things a person does
//! at the same screen: **put a piece down, look at whether the slot says it
//! assembled, and take it back if it did not**; and **stop fighting a rung
//! that keeps winning**. The first is why `Undo` is a verb.

pub mod hands;
pub mod sense;

use gearmaster_console::{Console, Difficulty, Mode, Verb};

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub rung_reached: usize,
    pub best_rung: usize,
    pub board_clears: usize,
    pub game_clears: usize,
    pub losses: usize,
    pub presses: usize,
    pub why: String,
}

/// How many times to lose one rung before calling it.
///
/// A Grinder may farm a rung for ever and the wall-clock budget is the only
/// thing that stops it (§14 E12). This is the control's own patience, so that
/// a run that cannot pass rung one says so in ten fights rather than in four
/// thousand presses.
const PATIENCE: usize = 10;

fn assembled(c: &Console) -> usize {
    c.view().grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum()
}

pub fn starter(seed: u64, mode: Mode, difficulty: Difficulty, max_presses: usize) -> Ended {
    let mut c = Console::start(seed, mode, difficulty);
    let mut board_clears = 0;
    let mut game_clears = 0;
    let mut losses = 0;
    let mut presses = 0;
    let mut best = 1;
    let mut stuck = 0;
    let mut why = "out of presses";

    while presses < max_presses && !c.over() {
        let menu = c.menu();
        if menu.is_empty() {
            why = "nothing left to press";
            break;
        }

        // ---- put the tray on the board -----------------------------------
        //
        // Try each seat, keep the one that finished an item, take back the
        // ones that did not. A person does this with the mouse and their eyes;
        // the console gives an agent the same two presses.
        if let Some(&first) = menu.iter().find(|v| matches!(v, Verb::Place { .. })) {
            let before = assembled(&c);
            let mut seated = false;
            for v in menu.iter().filter(|v| matches!(v, Verb::Place { .. })) {
                if !c.apply(*v).ok {
                    continue;
                }
                presses += 1;
                if assembled(&c) > before {
                    seated = true;
                    break;
                }
                c.apply(Verb::Undo);
                presses += 1;
                if presses >= max_presses {
                    break;
                }
            }
            if !seated && presses < max_presses {
                // Nothing finished an item. Put it somewhere anyway - a loose
                // piece still pays its flat stats.
                c.apply(first);
                presses += 1;
            }
            continue;
        }

        // ---- everything else, in the order the road offers it -------------
        let pick = menu
            .iter()
            .find(|v| matches!(v, Verb::Answer { .. }))
            .or_else(|| menu.iter().find(|v| matches!(v, Verb::Drink)))
            .or_else(|| menu.iter().find(|v| matches!(v, Verb::WalkOn)))
            .or_else(|| menu.iter().find(|v| matches!(v, Verb::FightParty)))
            .or_else(|| menu.iter().find(|v| matches!(v, Verb::Fight)))
            .copied();
        let Some(v) = pick else {
            why = "nothing worth pressing";
            break;
        };

        if !c.apply(v).ok {
            why = "a press was refused";
            break;
        }
        presses += 1;

        if matches!(v, Verb::Fight | Verb::FightParty) {
            let after = c.view();
            if let Some(f) = &after.last_fight {
                if f.won {
                    game_clears += 1;
                    if f.board_decided {
                        board_clears += 1;
                    }
                } else {
                    losses += 1;
                }
            }
            // Patience is measured against the **best** rung and not the last
            // one: a Grinder that wins a rung and loses the next one is back
            // where it started, and counting "did this fight go up" would call
            // that progress for ever.
            if after.rung_shown > best {
                best = after.rung_shown;
                stuck = 0;
            } else {
                stuck += 1;
                if stuck >= PATIENCE {
                    why = "stuck below its ceiling";
                    break;
                }
            }
        }
    }

    let v = c.view();
    Ended {
        rung_reached: v.rung_shown,
        best_rung: best,
        board_clears,
        game_clears,
        losses,
        presses,
        why: if c.over() { "the run ended".into() } else { why.into() },
    }
}
