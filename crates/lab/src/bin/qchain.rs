//! Walk the Manse chain forward, rung by rung, and say where it fits.
//!
//!     cargo run -p gearmaster-lab --bin qchain
//!     QCHAIN_WORD_AT=24 cargo run -p gearmaster-lab --bin qchain
//!
//! `tests/chain.rs` proves each station of the chain opens the next, and it
//! does that by standing the run at each door in turn - which means it sets
//! `run.rung` **backwards** between two of them: the locked gate is answered at
//! rung 26 and the Manse's own gate stands after rung 25. That is a true claim
//! about the doors and not a claim about the road. Nothing in the suite walked
//! the chain in the order a player meets it.
//!
//! This does. One rung at a time, every fight won by fiat, every chain door
//! answered the moment it stands, and the rung each thing happened on printed -
//! so the window is a measurement rather than an argument.
//!
//! The sweep at the end is the deadline: the latest rung the first word can
//! arrive on and still buy the class at the bottom of the stair.
//!
//! `crates/engine/tests/quest.rs` pins what this prints.

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::town::Action;

const WRONG_STARS: &str = "A Word About the Wrong Stars";
const THRESHOLD: &str = "Threshold-Sighted";

/// The doors this walk answers, and the choice it takes at each.
///
/// Everything else on the road is answered with the first choice that is open,
/// so a walk is never stopped by a door it had no key to.
const TAKE: &[(&str, &str)] =
    &[("the-astronomer", "Hear him out"), ("the-locked-gate", "Use the word")];

/// What one walk met, by rung index.
#[derive(Default)]
struct Walk {
    /// The rung each chain door was answered on.
    answered: Vec<(&'static str, usize)>,
    /// The rung the Manse's gate stood on, if it ever did.
    gate: Option<usize>,
    /// Whether the gate opened onto a town at all - the reveal happened.
    revealed: bool,
    /// The rung the class was won on.
    class: Option<usize>,
    /// The highest rung the walk stood on.
    deepest: usize,
}

fn main() {
    // The rung index the first word is handed over on. The bar sells it and the
    // first town's gate stands after rung index 6, so 7 is the earliest a run
    // can walk out of a pub holding it.
    let at: usize =
        std::env::var("QCHAIN_WORD_AT").ok().and_then(|s| s.parse().ok()).unwrap_or(7);

    println!("A word handed over on rung {}, and the chain walked forward.\n", at + 1);
    let w = walk(at, true);
    println!(
        "\n  manse gate {}   class {}   walked to rung {}",
        w.gate.map_or("never stood".into(), |r| format!("rung {}", r + 1)),
        w.class.map_or("never won".into(), |r| format!("rung {}", r + 1)),
        w.deepest + 1
    );

    println!("\nThe deadline.\n");
    println!("  word on rung | doors answered | town revealed | gate stood | class");
    for at in 0..=30 {
        let w = walk(at, false);
        println!(
            "  {:>12} | {:>14} | {:>13} | {:>10} | {}",
            at + 1,
            w.answered.len(),
            if w.revealed { "yes" } else { "no" },
            w.gate.map_or("no".into(), |r| format!("rung {}", r + 1)),
            if w.class.is_some() { "won" } else { "-" }
        );
    }
}

/// Walk one run from rung one, answering the chain as early as it can be
/// answered.
///
/// Bounded twice over - by the rung it stops at and by the number of times
/// round the loop - because a walk that runs until it runs out is a hang
/// (`CLAUDE.md` trap 24), and this one can meet a set of points.
fn walk(word_at: usize, verbose: bool) -> Walk {
    let mut run = Run::seeded(0xC4A1);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    let mut w = Walk::default();

    for _ in 0..600 {
        w.deepest = w.deepest.max(run.rung);
        if run.rung >= 40 || w.class.is_some() {
            break;
        }
        run.back_to_loadout();
        if run.rung == word_at && !run.holds(WRONG_STARS) {
            run.give(WRONG_STARS);
            if verbose {
                println!("  rung {:>2}  word    {}", run.rung + 1, WRONG_STARS);
            }
        }
        // A fountain stands in front of whatever else is on its rung, and
        // `pending_event` refuses to answer over the top of one.
        if run.at_fountain() || run.at_doubling_fountain() {
            let c = run.drink();
            if verbose {
                println!("  rung {:>2}  drink   {}", run.rung + 1, c.name);
            }
            continue;
        }
        if let Some(e) = run.pending_event() {
            let wanted = TAKE
                .iter()
                .find(|(id, _)| *id == e.id)
                .and_then(|(_, l)| e.choices.iter().find(|c| c.label == *l))
                .filter(|c| run.choice_open(c));
            let Some(c) = wanted.or_else(|| e.choices.iter().find(|c| run.choice_open(c))) else {
                break;
            };
            let (label, on_the_chain, at) = (c.label, wanted.is_some(), run.rung);
            run.take_choice(c);
            run.take_receipt();
            // `run.answered`, not the return: `take_choice` hands back the
            // component it took and most doors take nothing, so `is_some()` is
            // not "the door was answered" (`CLAUDE.md` trap 21).
            if !run.answered.contains(&e.id) {
                if verbose {
                    println!("  rung {:>2}  stuck   {} refused {:?}", at + 1, e.id, label);
                }
                break;
            }
            if on_the_chain {
                w.answered.push((TAKE.iter().find(|(id, _)| *id == e.id).expect("just found").0, at));
                if verbose {
                    println!("  rung {:>2}  door    {} -> {:?}", at + 1, e.id, label);
                }
            }
            w.revealed = run.towns_revealed.contains(&"the-manse");
            continue;
        }
        if let Some(t) = run.pending_town() {
            if verbose {
                println!("  rung {:>2}  town    {}", run.rung + 1, t.id);
            }
            if t.id == "the-manse" {
                w.gate = Some(run.rung);
                run.visit_town(Action::CellarDoor);
                run.take_receipt();
                continue;
            }
            run.skip_town();
            continue;
        }
        // A dungeon's floors are a graph, so a walk that only fights stalls at
        // the lever and reads as a board that stopped (`CLAUDE.md` trap 23).
        if run.at_points {
            run.throw_points(0);
            continue;
        }
        if run.dungeon.is_some() {
            run.pending_scene = None;
            run.force_win();
            if run.classes.iter().any(|c| c.name == THRESHOLD) {
                w.class = Some(run.rung);
                if verbose {
                    println!(
                        "  rung {:>2}  class   {}   insight {}",
                        run.rung + 1,
                        THRESHOLD,
                        run.insight_unlocked
                    );
                }
            }
            continue;
        }
        run.force_win();
    }
    w
}
