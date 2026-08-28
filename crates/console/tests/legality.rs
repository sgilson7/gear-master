//! The menu and the engine, checked against each other in both directions.
//!
//! Half a lint is not a lint (`CLAUDE.md` §6 trap 19), so this asserts two
//! things and not one: everything the menu offers is accepted, and every
//! placement the engine would accept is offered. The second direction is the
//! one that matters for an agent - a menu that quietly omits a legal placement
//! is a menu that makes the game look harder than it is.

use gearmaster_console::{Console, Difficulty, Mode, SlotKind, Verb};
use gearmaster_engine::rng::Rng;

/// Walk a run by pressing random legal verbs, checking the menu at every step.
fn fuzz(seed: u64, steps: usize) -> (usize, Vec<usize>) {
    let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
    let mut rng = Rng::new(seed ^ 0xA11CE);
    let mut sizes = Vec::new();
    let mut states = 0;
    for _ in 0..steps {
        let menu = c.menu();
        if menu.is_empty() {
            break;
        }
        states += 1;
        sizes.push(menu.len());

        // Direction one: everything offered is accepted. Checked on a sample
        // rather than all of them, because accepting one changes the state -
        // so the sample is "the one we are about to press".
        let pick = menu[(rng.next_u64() % menu.len() as u64) as usize];

        // Direction two: no legal placement is missing from the menu. Walk
        // every tray piece against every anchor the engine would allow and
        // assert the menu already had it.
        for id in c.tray_ids() {
            for kind in SlotKind::ALL {
                for (x, y) in c.anchors_for(id, kind) {
                    let v = Verb::Place { piece: id, slot: kind, x, y };
                    assert!(
                        menu.contains(&v),
                        "the engine accepts {:?} and the menu did not offer it",
                        v
                    );
                }
            }
        }

        let out = c.apply(pick);
        assert!(
            out.ok,
            "the menu offered {:?} and the engine refused it: {:?}",
            pick, out.lines
        );

        // **No verb may leave a fight standing.**
        //
        // The general form of the freeze `f4354ec` fixed in the window:
        // walking onto a pinnacle starts a fight, and while one is unsettled
        // the county's own controls refuse - so a driver that does not settle
        // it has no move left and no way out. Written as "after any press,
        // there is something to press" rather than as a check on the verbs
        // that can do it, because the fault was nobody asking which those
        // were.
        assert!(
            !c.menu().is_empty() || c.over(),
            "{:?} left the run with nothing to press and the run is not over",
            pick
        );
    }
    (states, sizes)
}

#[test]
fn every_offered_verb_is_accepted_and_every_legal_placement_is_offered() {
    let mut all = Vec::new();
    let mut states = 0;
    for seed in [0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212, 7, 99] {
        let (n, sizes) = fuzz(seed, 180);
        states += n;
        all.extend(sizes);
    }
    assert!(states >= 1_000, "only walked {} states", states);
    all.sort_unstable();
    println!(
        "legal verbs over {} states: min {}, median {}, max {}",
        states,
        all[0],
        all[all.len() / 2],
        all[all.len() - 1]
    );
}

#[test]
fn a_fight_is_never_offered_while_a_door_stands() {
    // The road stack pops in an order and a door in front of another is not a
    // queue (trap 35). If the menu ever offers `Fight` under a question, an
    // agent will walk past content nobody meant it to walk past.
    let mut c = Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium);
    let mut rng = Rng::new(4);
    for _ in 0..400 {
        let menu = c.menu();
        if menu.is_empty() {
            break;
        }
        if c.view().question.is_some() {
            assert!(
                !menu.contains(&Verb::Fight),
                "a fight was offered while a door was standing"
            );
        }
        let pick = menu[(rng.next_u64() % menu.len() as u64) as usize];
        if !c.apply(pick).ok {
            break;
        }
    }
}
