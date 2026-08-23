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

    let hits = |class: &[gearmaster_engine::class::ClassDef]| -> i32 {
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
    let stuns = |class: &[gearmaster_engine::class::ClassDef]| -> usize {
        let log = simulate_with_class(stats, &profiles, tough, Difficulty::Medium, class);
        log.entries
            .iter()
            // A stun holds an item rather than a fighter, so it has an event
            // of its own carrying which item it took.
            .filter(|e| matches!(e.event, Event::Stunned { on: Side::Enemy, .. }))
            .count()
    };

    let oracle = CLASSES.iter().find(|c| c.name == "Oracle").expect("Oracle exists");
    // Not "nothing else in the game lands a stun" any more - nineteen pieces
    // do. This build is not carrying one, which is the control.
    assert_eq!(stuns(&[]), 0, "this build has no other source of stun");
    assert!(stuns(&[*oracle]) > 0, "an Oracle should be stopping their gear");
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

    let rage = |class: &[gearmaster_engine::class::ClassDef]| -> i32 {
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
    assert!(rage(&[*bl]) > rage(&[]), "curses should have banked rage");
}

// ------------------------------------------------------- what they say

/// A description that says something different from what the code does is
/// worse than one that says nothing. Consecrate is here by name because it
/// was exactly that: it read "armour is stronger where you already resist",
/// which is a rule that was never written. What it does is key off held faith.
#[test]
fn consecrate_describes_the_rule_that_was_actually_built() {
    let warpriest = CLASSES.iter().find(|c| c.name == "Warpriest").expect("Warpriest exists");
    let text = warpriest.power.describe().to_lowercase();
    assert!(text.contains("faith"), "it keys off faith, and should say so: {}", text);
    assert!(text.contains("armour"), "{}", text);
    assert!(
        !text.contains("already resist"),
        "still describing a rule nobody wrote: {}",
        text
    );
}

/// Every power owes the player a sentence with something concrete in it: a
/// number, or a named condition. "Held resources count double" told nobody
/// what a resource does held.
#[test]
fn every_class_power_says_something_concrete() {
    for c in CLASSES {
        let full = c.power.describe();
        assert!(full.len() >= 30, "{}: '{}' is too vague to act on", c.name, full);
        // A number, or a word that stands in for one, or a named condition.
        let concrete = full.chars().any(|ch| ch.is_ascii_digit())
            || ["twice", "double", "all four", "opposite"]
                .iter()
                .any(|w| full.contains(w));
        assert!(concrete, "{}: '{}' names no number and no condition", c.name, full);
    }
}

/// The panel has one line for this and the glossary has a paragraph, so the
/// short form has to actually be shorter - otherwise it is cut off on screen.
#[test]
fn the_short_form_fits_where_the_long_one_does_not() {
    for c in CLASSES {
        let short = c.power.short();
        assert!(!short.is_empty(), "{} has no short form", c.name);
        assert!(
            short.len() <= 46,
            "{}: '{}' is {} chars, too long for the panel",
            c.name,
            short,
            short.len()
        );
    }
}

// --------------------------------------------------------- the choosing

/// The fountain offers rather than decides: what your gear earned, the two you
/// came nearest to, and one out of the water.
#[test]
fn the_fountain_offers_four_ways_to_go() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let offer = run.fountain_offer();
    assert_eq!(offer.len(), 4, "three read off the build and one wildcard");

    // The first is what the build actually earns - the same answer the panel
    // has been showing all along, so the offer is never a surprise.
    assert_eq!(offer[0].name, classify(&run.fingerprint()).name);

    let names: Vec<&str> = offer.iter().map(|c| c.name).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "no class offered twice: {:?}", names);
}

/// A fountain never offers what you already hold, so the second one is always
/// worth stopping for.
#[test]
fn the_fountain_never_offers_what_you_already_have() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.rung = Run::FOUNTAINS[0];
    let first = run.drink().name;

    run.rung = Run::FOUNTAINS[1];
    let offer = run.fountain_offer();
    assert!(
        !offer.iter().any(|c| c.name == first),
        "the second fountain offered {} again",
        first
    );
}

/// You may only take what is on the table. Otherwise the offer is decoration.
#[test]
fn the_fountain_refuses_a_class_it_did_not_offer() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.rung = Run::FOUNTAINS[0];
    let offer = run.fountain_offer();
    let not_offered = CLASSES
        .iter()
        .find(|c| !offer.iter().any(|o| o.name == c.name))
        .expect("seventeen classes, four offered");

    assert!(run.drink_choosing(not_offered).is_none(), "took something off the menu");
    assert!(run.classes.is_empty(), "and it should have changed nothing");

    let wanted = offer[1];
    assert_eq!(run.drink_choosing(wanted).map(|c| c.name), Some(wanted.name));
    assert_eq!(run.classes.len(), 1);
}

/// The wildcard is fixed to the fountain, not rerolled on every redraw - a
/// choice that changes while you are looking at it is not a choice.
#[test]
fn the_wildcard_is_the_same_every_time_you_look() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.rung = Run::FOUNTAINS[0];
    let first: Vec<&str> = run.fountain_offer().iter().map(|c| c.name).collect();
    for _ in 0..5 {
        let again: Vec<&str> = run.fountain_offer().iter().map(|c| c.name).collect();
        assert_eq!(first, again, "the offer moved while it was being read");
    }
}


// ------------------------------------------------- the third fountain

/// The third fountain does not hand over a new title. It takes one you hold
/// and gives you twice as much of it.
#[test]
fn the_deep_fountain_doubles_something_you_already_are() {
    use gearmaster_engine::class::{ClassPower, CLASSES};
    use gearmaster_engine::run::Run;

    let mut run = Run::new();
    let bl = CLASSES.iter().find(|c| c.name == "Bloodletter").expect("exists");
    run.classes.push(bl);
    let before = match run.effective_classes()[0].power {
        ClassPower::Bloodscent(n) => n,
        other => panic!("expected Bloodscent, got {:?}", other),
    };

    // It only stands at its own rung, and only once.
    run.skip_to(Run::DOUBLING_FOUNTAIN);
    assert!(run.at_doubling_fountain(), "it should be standing here");
    assert!(run.double_class(bl), "and it should take the offer");
    assert!(!run.at_doubling_fountain(), "and then be gone");
    assert!(!run.double_class(bl), "and refuse a second helping");

    let after = match run.effective_classes()[0].power {
        ClassPower::Bloodscent(n) => n,
        other => panic!("expected Bloodscent, got {:?}", other),
    };
    assert_eq!(after, before * 2, "twice as much");
}

/// It offers only what it can actually double. A power that is a switch
/// rather than a number has no second helping, and the fountain saying it
/// would give you one would be a lie.
#[test]
fn the_deep_fountain_never_offers_what_it_cannot_give() {
    use gearmaster_engine::class::CLASSES;
    use gearmaster_engine::run::Run;
    for c in CLASSES {
        let mut run = Run::new();
        run.classes.push(c);
        let offered = !run.doubling_offer().is_empty();
        assert_eq!(
            offered,
            c.power.doubled().is_some(),
            "{} is offered={} but doubles={}",
            c.name,
            offered,
            c.power.doubled().is_some()
        );
    }
    // Every one a fountain can pour doubles. Five used to be switches with
    // nothing to turn, and the fountain quietly did not appear for a player
    // holding two of those - which is how the third fountain came to be "not
    // working".
    //
    // An earned class is exempt, and has to be: no fountain offers one, so
    // holding it cannot be what makes a fountain skip you. Immense Guilt is
    // the case - doubling a pure cost would be a fountain offering to make
    // your run worse.
    for c in CLASSES {
        if gearmaster_engine::class::is_earned(c.name) {
            continue;
        }
        assert!(
            c.power.doubled().is_some(),
            "{} cannot be doubled, so the fountain would skip a player holding it",
            c.name
        );
    }
}


/// Walk a whole run and meet all three fountains.
///
/// The third one used to be reachable only if you happened to be holding a
/// class whose power was a number. Five of the seventeen were switches, and a
/// player who drank Geomancer and Wanderer - which is what the preset build
/// reads as - simply never saw it, with nothing on screen to say why.
#[test]
fn a_whole_run_meets_every_fountain() {
    use gearmaster_engine::run::Run;

    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let mut drank = 0;
    let mut doubled = false;
    for _ in 0..80 {
        if run.at_fountain() {
            let offer = run.fountain_offer();
            assert!(!offer.is_empty(), "a fountain with nothing in it");
            run.drink_choosing(offer[0]).expect("it should pour");
            drank += 1;
            continue;
        }
        if run.at_doubling_fountain() {
            let offer = run.doubling_offer();
            assert!(!offer.is_empty(), "a deep fountain with nothing in it");
            assert!(run.double_class(offer[0]), "it should pour");
            doubled = true;
            continue;
        }
        if run.rung + 1 >= gearmaster_engine::combat::LADDER.len() {
            break;
        }
        run.skip_to(run.rung + 1);
    }
    assert_eq!(drank, 2, "both class fountains");
    assert!(doubled, "and the deep one");
    assert_eq!(run.classes.len(), 2);
    assert!(run.doubled.is_some());
}
