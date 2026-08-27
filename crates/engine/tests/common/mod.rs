//! Shared fixtures for the integration tests.
#![allow(dead_code)] // each test binary uses a different subset

use gearmaster_engine::piece::{Action, PieceDef, PieceId, SlotKind, Trigger};
use gearmaster_engine::run::Run;

/// Run `f` over every action a trigger can reach.
///
/// This was a copy of `piece::walk_actions` and is now a call to it. The
/// engine's own doc had already noticed: *"The test suite has carried a copy
/// of this for a while; `rating.rs` needs the same answer, and two of them
/// would drift."* They drifted the moment a trigger variant was added - the
/// engine's walker knew about `OnEnemyActivate` and this one did not, and the
/// only reason that was caught is that the match was exhaustive.
///
/// One walker. A lint over the catalogue that misses a payload is a lint that
/// reports a clean catalogue.
pub fn actions_of(t: &Trigger, f: &mut impl FnMut(&Action)) {
    gearmaster_engine::piece::walk_actions(t, f)
}

/// Does any action this piece can reach satisfy `want`?
pub fn does(def: &PieceDef, want: fn(&Action) -> bool) -> bool {
    let mut hit = false;
    for t in def.triggers {
        actions_of(t, &mut |a| hit |= want(a));
    }
    hit
}

/// Does this piece carry a trigger satisfying `want`?
pub fn has(def: &PieceDef, want: fn(&Trigger) -> bool) -> bool {
    def.triggers.iter().any(want)
}

/// Look a starting component up by name.
pub fn piece(run: &Run, name: &str) -> PieceId {
    run.owned
        .iter()
        .copied()
        .find(|&id| run.registry.def(id).name == name)
        .unwrap_or_else(|| panic!("no piece named {}", name))
}

/// Equip by name, failing loudly with the reason if the placement is illegal.
pub fn equip(run: &mut Run, name: &str, slot: SlotKind, ax: u8, ay: u8) {
    let id = piece(run, name);
    run.equip(id, slot, ax, ay)
        .unwrap_or_else(|e| panic!("failed to equip {} at ({}, {}): {}", name, ax, ay, e));
}

/// A complete, legal loadout that assembles all five slots and lights every
/// assembly bonus. Delegates to the engine's own preset so the tests assert
/// against the same arrangement the GUI's auto-build button produces.
pub fn build_full_loadout(run: &mut Run) {
    run.apply_preset();
}

/// The board a share code describes, seated the way its owner seated it.
///
/// One way to do this, because there used to be four. `Shared::loadout` locks
/// each item the moment it assembles; three tests hand-rolled the same
/// placement loop without that step and got a different board out of the same
/// code. The engine never locks on its own - locking is something the player
/// does, with a button - so replaying placements without replaying the locks
/// replays half of what was done. On a board packed to ninety-seven percent of
/// its cells the difference is not subtle: the owner's nineteen weapon pieces
/// came back as one item, and the perfect run's eleven came back as none.
///
/// The classes the code recorded are *not* applied here. A class is a rule
/// about how the board fights rather than part of the board, and most callers
/// want the gear on its own. `run_from` is the one that wants both.
pub fn board_from(code: &str) -> Run {
    let sh = gearmaster_engine::share::import(code).expect("the code still reads");
    let (reg, lo) = sh.loadout();
    let mut run = Run::new();
    run.owned = (0..reg.count()).map(|i| PieceId(i as u32)).collect();
    run.registry = reg;
    run.loadout = lo;
    run.mode = gearmaster_engine::run::Mode::Grinder;
    run.difficulty = gearmaster_engine::combat::Difficulty::Medium;
    run
}

/// The same board, wearing the classes the run finished with.
pub fn run_from(code: &str) -> Run {
    let sh = gearmaster_engine::share::import(code).expect("the code still reads");
    let mut run = board_from(code);
    for c in &sh.classes {
        if let Some(k) = gearmaster_engine::class::CLASSES.iter().find(|k| k.name == *c) {
            run.classes.push(k);
        }
    }
    run.refresh_class_effects();
    run
}

// -------------------------------------------------- a dungeon with points in it

/// A four-room dungeon with a set of points at the top, for proving the graph
/// primitive before any content exists.
///
/// It is not in `DUNGEONS` and never will be. That is the point of it: the
/// transitions - clearing, throwing, leaving, losing, coming back - are worth
/// testing against a shape the shipped six do not have, and adding a ninth
/// dungeon to the table to get one would put a test fixture on the road.
/// `Run::enter_dungeon_at` takes the dungeon rather than an id for exactly
/// this reason.
///
/// ```text
///        [0] The Reciter
///         /            \
///   [1] The Long Haul   [2] The Watchers
///         |
///   [3] The Current
/// ```
///
/// Four creatures that already exist in `ALTERNATES`, so nothing here is a
/// `MonsterSpec`. Two roads of unequal length on purpose: the longest road out
/// of floor 0 is three fights and the short one is two, which is what makes
/// `fights_ahead` say something a room count could not.
pub static A_YARD: gearmaster_engine::dungeon::Dungeon = gearmaster_engine::dungeon::Dungeon {
    id: "a-test-yard",
    name: "A TEST YARD",
    blurb: &["A yard that is not on the road, and never will be."],
    entry: &["The gate is open and the rails go two ways from it."],
    floors: &[
        gearmaster_engine::dungeon::Floor {
            creature: "The Reciter",
            landing: "The recitation stops, and past it the rails part.",
            exits: &[
                gearmaster_engine::dungeon::Exit {
                    to: 1,
                    label: "The long road",
                    blurb: "Two more fights, and a siding at the end of it.",
                },
                gearmaster_engine::dungeon::Exit {
                    to: 2,
                    label: "The short road",
                    blurb: "One fight, and then the buffer stop.",
                },
            ],
            fork: &["The rails part at a lever nobody is standing at."],
            entry: &[],
            also: &[],
        },
        gearmaster_engine::dungeon::Floor {
            creature: "The Long Haul",
            landing: "The train goes over on the bend, and the road goes on.",
            exits: &[gearmaster_engine::dungeon::Exit {
                to: 3,
                label: "",
                blurb: "",
            }],
            fork: &[],
            entry: &["The siding puts you down halfway along the long road."],
            also: &[],
        },
        gearmaster_engine::dungeon::Floor {
            creature: "The Watchers",
            landing: "The short road ends at a buffer stop, as painted.",
            exits: &[],
            fork: &[],
            entry: &[],
            also: &[gearmaster_engine::event::Outcome::Flag("took-the-short-road")],
        },
        gearmaster_engine::dungeon::Floor {
            creature: "The Current",
            landing: "The long road ends at a buffer stop, as painted.",
            exits: &[],
            fork: &[],
            entry: &[],
            also: &[gearmaster_engine::event::Outcome::Flag("took-the-long-road")],
        },
    ],
    reward: "",
    also: &[],
};
