//! What the fountain gives you, and why.
//!
//! The rule these all rest on: a class is thresholds on abstract axes and may
//! never name a component. See `class.rs`.

mod common;

use common::equip;
use gearmaster_engine::class::{classify, rank, Axis, CLASSES};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;

/// The rule, stated once: you are given the most demanding class you qualify
/// for.
///
/// It used to be the class you cleared by the biggest surplus, which rewarded
/// a class for being cheap - Bulwark asks for ward and armour, both of which
/// are on nearly every piece in the game, so almost any build cleared it by
/// fifty points and out-scored whatever it was actually built for.
#[test]
fn the_class_you_get_is_the_most_demanding_one_you_qualify_for() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Echo Sigil", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Resonant Chord", SlotKind::Weapon, 4, 0);

    let fp = run.fingerprint();
    let ranked = rank(&fp);
    let eligible: Vec<_> = ranked.iter().filter(|m| m.eligible).collect();
    assert!(eligible.len() > 1, "needs a choice to be making one");

    let given = classify(&fp);
    let hardest = eligible.iter().map(|m| m.class.demand()).max().unwrap();
    assert_eq!(
        given.demand(),
        hardest,
        "given {} (demand {}) over something asking {}",
        given.name,
        given.demand(),
        hardest
    );
}

/// A crystal ball whose spells answer each other is an Oracle. Built by hand
/// rather than by the search tool, which caps its candidate pool by rating and
/// so never picks up an answering spell - those rate poorly alone, because the
/// rating cannot see the ball they will sit in.
#[test]
fn a_ball_of_answering_spells_is_an_oracle() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Echo Sigil", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Resonant Chord", SlotKind::Weapon, 4, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);

    let fp = run.fingerprint();
    assert!(fp.get(Axis::Answering) >= 45, "answering {}", fp.get(Axis::Answering));
    assert!(fp.get(Axis::Orbits) >= 50, "orbits {}", fp.get(Axis::Orbits));
    assert_eq!(classify(&fp).name, "Oracle");
}

/// An axis nothing can reach is a dead class. Wrath, cadence and weave were
/// all set against a much smaller catalogue and had drifted past what the game
/// could produce; this pins the ones the new classes depend on.
#[test]
fn the_axes_the_new_classes_want_are_reachable() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Echo Sigil", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Resonant Chord", SlotKind::Weapon, 4, 0);
    let fp = run.fingerprint();
    assert!(fp.get(Axis::Answering) > 0, "no build can answer its own spells");
    assert!(fp.get(Axis::Orbits) > 0, "no build can carry a ball");
}

/// Every class has to be gettable somehow, or it is decoration. This does not
/// prove reachability - that needs a build - but it catches the cheap mistake
/// of writing a threshold nothing could ever clear.
#[test]
fn no_class_asks_for_more_than_an_axis_can_give() {
    for c in CLASSES {
        for &(axis, need) in c.requires {
            assert!(
                (1..=100).contains(&need),
                "{} wants {} at {}, off the 0-100 scale",
                c.name,
                axis.name(),
                need
            );
        }
    }
}

/// The floor. A fountain always has something to hand over, whatever you are
/// wearing - including nothing.
#[test]
fn an_empty_build_still_gets_a_class() {
    let run = Run::new();
    assert_eq!(classify(&run.fingerprint()).name, "Wanderer");
}

/// Two builds that differ only in gear must be able to differ in class, or the
/// whole system is decoration. Armour and spells should not read alike.
#[test]
fn different_builds_get_different_classes() {
    let mut iron = Run::with_all_pieces();
    equip(&mut iron, "Bastion Base", SlotKind::Chest, 0, 0);
    equip(&mut iron, "Bulwark Layer", SlotKind::Chest, 0, 3);

    let mut spells = Run::with_all_pieces();
    equip(&mut spells, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut spells, "Echo Sigil", SlotKind::Weapon, 1, 3);
    equip(&mut spells, "Resonant Chord", SlotKind::Weapon, 4, 0);

    assert_ne!(
        classify(&iron.fingerprint()).name,
        classify(&spells.fingerprint()).name,
        "a wall and a crystal ball read as the same class"
    );
}

// ---------------------------------------------------------- the new powers

/// Each class has to bring a rule of its own. A new class sharing an old
/// class's power is a new name, not a new way to play.
#[test]
fn every_class_power_is_used_once() {
    let mut seen: Vec<String> = Vec::new();
    for c in CLASSES {
        let d = format!("{:?}", c.power);
        assert!(!seen.contains(&d), "{} duplicates another class's power", c.name);
        seen.push(d);
    }
}

/// A crystal ball speaks with two voices, and does so for anyone - no class
/// required. A ball that cast one spell at a time was just a book that could
/// not make up its mind.
#[test]
fn a_crystal_ball_casts_two_spells_at_once_by_default() {
    use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side, LADDER};

    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 4, 0);
    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;

    let hits = |class: &[&'static gearmaster_engine::class::ClassDef]| -> i32 {
        let log =
            simulate_with_class(stats, &profiles, &LADDER[0], Difficulty::Medium, class);
        log.entries
            .iter()
            .filter_map(|e| match e.event {
                Event::Hit { by: Side::Player, damage, .. } => Some(damage),
                _ => None,
            })
            .sum()
    };

    // Both spells land on every activation, so the ball out-damages the sum of
    // what either would do alone at that cadence.
    let both = hits(&[]);
    assert!(both > 0, "the ball should be landing something");

    let single: i32 = {
        let mut solo = Run::with_all_pieces();
        equip(&mut solo, "Pocket Grimoire", SlotKind::Weapon, 0, 0);
        equip(&mut solo, "Soot Ink", SlotKind::Weapon, 2, 0);
        equip(&mut solo, "Emberburst", SlotKind::Weapon, 3, 0);
        let profiles = solo.combat_items();
        let mut st = solo.player_stats();
        st.health = 100_000;
        let log = simulate_with_class(st, &profiles, &LADDER[0], Difficulty::Medium, &[]);
        log.entries
            .iter()
            .filter_map(|e| match e.event {
                Event::Hit { by: Side::Player, damage, .. } => Some(damage),
                _ => None,
            })
            .sum()
    };
    assert!(both > single, "a ball ({}) should out-hit a book ({})", both, single);
}

/// The Oracle reaches at the clock rather than at flesh: it is the only way
/// anyone gets the two curses that stop gear rather than hurting it.
#[test]
fn an_oracle_stops_their_gear() {
    use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side, LADDER};
    use gearmaster_engine::curse::CurseKind;

    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 4, 0);
    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;

    // A deep monster, not a rat: an Oracle needs four activations to reach the
    // clock and a rat does not last four activations.
    let tough = &LADDER[30];
    let stuns = |class: &[&'static gearmaster_engine::class::ClassDef]| -> usize {
        let log = simulate_with_class(stats, &profiles, tough, Difficulty::Medium, class);
        log.entries
            .iter()
            .filter(|e| {
                matches!(
                    e.event,
                    Event::Cursed { on: Side::Enemy, kind: CurseKind::Stun, .. }
                )
            })
            .count()
    };

    let oracle = CLASSES.iter().find(|c| c.name == "Oracle").expect("Oracle exists");
    assert_eq!(stuns(&[]), 0, "nothing else in the game lands a stun");
    assert!(stuns(&[oracle]) > 0, "an Oracle should be stopping their gear");
}

/// Bloodscent: what a Bloodletter rots, it feeds on.
#[test]
fn bloodscent_banks_rage_when_a_curse_lands() {
    use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side, LADDER};

    let mut run = Run::with_all_pieces();
    // Hexbrand curses the enemy on every activation. Cursed Blade looks like
    // the obvious choice and is not: it curses its own wearer.
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Hexbrand", SlotKind::Weapon, 1, 0);
    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;

    let rage = |class: &[&'static gearmaster_engine::class::ClassDef]| -> i32 {
        let log =
            simulate_with_class(stats, &profiles, &LADDER[0], Difficulty::Medium, class);
        log.entries
            .iter()
            .filter(|e| {
                matches!(e.event, Event::GainResource { side: Side::Player, what, .. } if what == "rage")
            })
            .count() as i32
    };

    let bl = CLASSES.iter().find(|c| c.name == "Bloodletter").expect("Bloodletter exists");
    assert!(rage(&[bl]) > rage(&[]), "curses should have banked rage");
}
