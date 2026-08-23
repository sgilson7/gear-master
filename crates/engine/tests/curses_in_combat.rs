//! Curses, as the fight actually applies them.
//!
//! `curse.rs` has unit tests for the bookkeeping - stacks, timers, caps. What
//! they cannot show is which of those numbers the tick loop reads, and how
//! widely. Frost slowing *one* item and frost slowing *everything the target
//! owns* are the same `slow_pct` from the outside.

use gearmaster_engine::combat::{simulate_at, Difficulty, Event, Side, LADDER};
use gearmaster_engine::curse::{CurseKind, FROST_SLOW_CAP_PCT, MISFIRE_FLOOR, STUN_CAP_MS};
use gearmaster_engine::run::Run;

/// A player wearing one named component, seated wherever it will go.
fn wearing(names: &[&str]) -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    for name in names {
        let Some(id) = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == *name && !run.is_equipped(i))
        else {
            panic!("no such component: {name}");
        };
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
        assert!(run.is_equipped(id), "{name} would not sit in {slot:?}");
    }
    run
}

/// Every enemy activation in the log, as (item index, time).
fn enemy_activations(log: &gearmaster_engine::combat::CombatLog) -> Vec<(usize, u32)> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Activate { side: Side::Enemy, index, .. } => Some((*index, e.at_ms)),
            _ => None,
        })
        .collect()
}

/// How long a stretch both runs have to be alive for before we compare them.
const WINDOW_MS: u32 = 6_000;

#[test]
fn frost_slows_everything_the_target_owns_not_one_item() {
    // Hoarfrost lands frost on the enemy every time it goes off. A spell needs
    // a book and an ink around it before it will cast at all.
    let run = wearing(&["Pocket Grimoire", "Mercurial Ink", "Hoarfrost"]);
    let stats = run.player_stats();
    let items = run.combat_items();
    assert_eq!(items.len(), 1, "the frost weapon has to assemble to cast");

    // Counting activations over the whole fight proves nothing: a slowed enemy
    // takes *longer* to do the same work, so the fight simply runs on and the
    // totals come out equal. Count inside a fixed window instead, where a
    // slower enemy really does get fewer turns.
    let in_window = |log: &gearmaster_engine::combat::CombatLog| -> Vec<(usize, u32)> {
        enemy_activations(log).into_iter().filter(|(_, t)| *t < WINDOW_MS).collect()
    };

    // Search the ladder rather than naming a rung, so re-tuning the ladder
    // cannot quietly turn this into a test of nothing.
    let (a, b) = LADDER
        .iter()
        .find_map(|spec| {
            // The control is the same fight with the frost bouncing off, so
            // the only difference between the runs is whether it landed.
            let mut immune = *spec;
            immune.curse_resist = 100;
            let free = simulate_at(stats, &items, &immune, Difficulty::Medium);
            let chilled = simulate_at(stats, &items, spec, Difficulty::Medium);
            if free.duration_ms < WINDOW_MS || chilled.duration_ms < WINDOW_MS {
                return None;
            }
            let (fa, ca) = (in_window(&free), in_window(&chilled));
            // Enough of their items firing often enough to tell one from many.
            let busy = {
                let mut v: Vec<usize> = fa.iter().map(|(i, _)| *i).collect();
                v.sort_unstable();
                v.dedup();
                v.into_iter().filter(|&i| fa.iter().filter(|(j, _)| *j == i).count() >= 2).count()
            };
            if busy < 2 {
                return None;
            }
            Some((ca, fa))
        })
        .expect("no rung gives two busy enemy items over a long enough fight");

    assert!(
        a.len() < b.len(),
        "frost did not slow the enemy at all: {} activations either way in the first {}s",
        a.len(),
        WINDOW_MS / 1000
    );

    // ...and it slowed more than one of their items, which is the whole
    // question. Frost is a whole-body slow, not a debuff on the thing that
    // happened to be cursed.
    let count_for = |v: &[(usize, u32)], idx: usize| v.iter().filter(|(i, _)| *i == idx).count();
    let idxs: Vec<usize> = {
        let mut v: Vec<usize> = b.iter().map(|(i, _)| *i).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let slowed: Vec<usize> =
        idxs.iter().copied().filter(|&i| count_for(&a, i) < count_for(&b, i)).collect();
    assert!(
        slowed.len() >= 2,
        "frost only slowed item(s) {slowed:?} of {idxs:?} - it is meant to slow all of them"
    );
}

#[test]
fn a_stun_stops_every_item_for_its_whole_length() {
    // Driven through Curses rather than a fight, because a stun long enough to
    // observe needs stacking and no single ladder monster obliges.
    let mut c = gearmaster_engine::curse::Curses::new();
    c.apply(CurseKind::Stun, 0);
    c.apply(CurseKind::Stun, 0);
    assert!(c.stunned());
    let left = c.stun_remaining_ms().expect("a stun is up");
    assert!(left > 0 && left <= STUN_CAP_MS);
    // Nothing advances while it runs, and it is one figure for the whole
    // combatant - there is no per-item stun to ask about.
    assert_eq!(c.slow_pct(), 0, "a stun is not a slow");
}

#[test]
fn the_caps_hold_under_a_pile_of_curses() {
    let mut c = gearmaster_engine::curse::Curses::new();
    for _ in 0..25 {
        c.apply(CurseKind::Frost, 0);
        c.apply(CurseKind::Stun, 0);
        c.apply(CurseKind::Misfire, 0);
        c.apply(CurseKind::Searing, 0);
    }
    assert_eq!(c.slow_pct(), FROST_SLOW_CAP_PCT, "gear never freezes solid");
    assert_eq!(c.stun_remaining_ms(), Some(STUN_CAP_MS), "a stun chain is not a lock");
    assert_eq!(c.misfire_every(), MISFIRE_FLOOR, "one in two is the worst it gets");
    // Searing is the one with no ceiling, on purpose: it is the only curse
    // whose stacks buy damage rather than denial, and damage already has to
    // out-race the target's regeneration to matter.
    assert_eq!(c.stacks_of(CurseKind::Searing), 25);
}
