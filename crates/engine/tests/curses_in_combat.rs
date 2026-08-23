//! Curses, as the fight actually applies them.
//!
//! `curse.rs` has unit tests for the bookkeeping - stacks, timers, caps. What
//! they cannot show is which of those numbers the tick loop reads, and how
//! widely. Frost slowing *one* item and frost slowing *everything the target
//! owns* are the same `slow_pct` from the outside.

use gearmaster_engine::combat::{simulate_at, CombatLog, Difficulty, Event, Side, LADDER};
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
fn enemy_activations(log: &CombatLog) -> Vec<(usize, u32)> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Activate { side: Side::Enemy, index, .. } => Some((*index, e.at_ms)),
            _ => None,
        })
        .collect()
}

/// Every stun landed on the enemy, as (item index, total duration).
fn enemy_stuns(log: &CombatLog) -> Vec<(usize, u32)> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Stunned { on: Side::Enemy, index, duration_ms, .. } => {
                Some((*index, *duration_ms))
            }
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
    let in_window = |log: &CombatLog| -> Vec<(usize, u32)> {
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

/// A caster that reliably fires a stun. Kingsbane wants nine mana to aim; with
/// none banked it takes its failure branch, which is the ordinary unaimed
/// curse of stun - and that is what these two want to watch.
fn a_stunning_caster() -> Run {
    wearing(&["Archmage's Primer", "Deepwater Ink", "Kingsbane", "Empowering Focus"])
}

#[test]
fn a_stun_stops_one_item_and_leaves_the_rest_running() {
    // The point of the whole change: a side with a stunned item still plays.
    let run = a_stunning_caster();
    let stats = run.player_stats();
    let items = run.combat_items();

    let found = LADDER.iter().find_map(|spec| {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        if log.enemy().items.len() < 2 {
            return None;
        }
        let (idx, from, until) = log.entries.iter().find_map(|e| match &e.event {
            Event::Stunned { on: Side::Enemy, index, duration_ms, .. } => {
                Some((*index, e.at_ms, e.at_ms + *duration_ms))
            }
            _ => None,
        })?;
        // While that item is stopped, something else of theirs still fires.
        let others = enemy_activations(&log)
            .into_iter()
            .filter(|(i, t)| *i != idx && *t >= from && *t <= until)
            .count();
        if others == 0 {
            // Nothing else was due in that window; not a failure, just a fight
            // that cannot answer the question.
            return None;
        }
        Some((idx, others, spec.name))
    });
    let (idx, others, name) = found.expect(
        "no rung landed a stun while another of their items was due - a stun that stopped the \
         whole side would look exactly like this",
    );
    assert!(others > 0, "{name}: item {idx} was stunned and nothing else of theirs fired");
}

#[test]
fn every_stun_in_a_fight_names_one_item_and_respects_the_cap() {
    let run = a_stunning_caster();
    let stats = run.player_stats();
    let items = run.combat_items();

    let mut landed = 0usize;
    let mut hit: Vec<usize> = Vec::new();
    for spec in LADDER.iter() {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        for (idx, duration) in enemy_stuns(&log) {
            landed += 1;
            assert!(
                idx < log.enemy().items.len(),
                "{}: a stun named item {idx} of {}",
                spec.name,
                log.enemy().items.len()
            );
            assert!(duration <= STUN_CAP_MS, "{}: a stun ran past the cap: {duration}", spec.name);
            if !hit.contains(&idx) {
                hit.push(idx);
            }
        }
        // Unpaid, Kingsbane takes its failure branch, which never aims.
        assert!(
            log.entries.iter().all(|e| !matches!(e.event, Event::Stunned { aimed: true, .. })),
            "{}: an unpaid stun reported itself as aimed",
            spec.name
        );
    }
    assert!(landed > 0, "no stun landed anywhere on the ladder");
    assert!(
        hit.len() >= 2,
        "every unaimed stun across the whole ladder landed on item {hit:?} - it is meant to \
         pick without warning. The precise rule is pinned in combat::stun_aim_tests."
    );
}

#[test]
fn the_caps_hold_under_a_pile_of_curses() {
    let mut c = gearmaster_engine::curse::Curses::new();
    for _ in 0..25 {
        c.apply(CurseKind::Frost, 0);
        c.apply(CurseKind::Misfire, 0);
        c.apply(CurseKind::Searing, 0);
    }
    assert_eq!(c.slow_pct(), FROST_SLOW_CAP_PCT, "gear never freezes solid");
    assert_eq!(c.misfire_every(), MISFIRE_FLOOR, "one in two is the worst it gets");
    // Searing is the one with no ceiling, on purpose: it is the only curse
    // whose stacks buy damage rather than denial, and damage already has to
    // out-race the target's regeneration to matter.
    assert_eq!(c.stacks_of(CurseKind::Searing), 25);
}
