//! Five short chains off the side of the road, and what each of them pays.
//!
//! A dungeon does not advance the ladder. It stands *beside* a rung, and
//! coming out puts you back in front of the fight you had not got to - which
//! is the whole reason a run can afford to take one.
//!
//! The four the mission adds are packed for their **entry** bands rather than
//! for the rung whose event opened them. A dungeon met by a formed build is a
//! dungeon that can be hard; packing one for the rung that unlocked it would
//! make the whole set trivial, and `design/monster-themes.md` §4 already
//! exempts anything standing beside the road from the curve.

mod common;

use gearmaster_engine::bestiary::{frame, is_unpacked, MonsterTheme};
use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::dungeon::{by_id, DUNGEONS};
use gearmaster_engine::event::Outcome;
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::{Mode, Run};

fn a_run() -> Run {
    let mut run = Run::seeded(0xD0A9);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

/// Walk one from the top, and hand back what came out of it.
fn walk(run: &mut Run, id: &'static str) {
    run.enter_dungeon(id);
    let floors = run.dungeon.expect("in it").0.floors.len();
    for _ in 0..floors {
        run.pending_scene = None;
        run.force_win();
        run.settle();
        run.back_to_loadout();
    }
    assert!(run.dungeon.is_none(), "{} did not end", id);
}

#[test]
fn the_mission_adds_four_and_they_all_stand_beside_the_road() {
    assert_eq!(DUNGEONS.len(), 6, "one shipped, and five the chain and the orbs add");
    for d in DUNGEONS {
        assert!(!d.floors.is_empty());
        assert_eq!(d.landings.len(), d.floors.len(), "{}: one landing a floor", d.id);
        assert!(!d.entry.is_empty(), "{} lets you in without a word", d.id);
        // Nothing on the ladder: a dungeon is reached by an event, a town door
        // or a pedestal, and never by climbing.
        for f in d.floors {
            assert!(
                !gearmaster_engine::combat::LADDER.iter().any(|m| m.name == *f),
                "{} is on the road as well as beside it",
                f
            );
        }
    }
}

#[test]
fn every_floor_is_a_frame_with_a_band_and_a_theme() {
    for d in DUNGEONS.iter().filter(|d| d.id != "the-crevice") {
        for f in d.floors {
            let fr = frame(f).unwrap_or_else(|| panic!("{} has no frame", f));
            assert!(fr.band >= 20, "{} packs to rung {}", f, fr.band);
            assert!(!fr.note.is_empty());
            assert!(is_unpacked(f), "{} has a board already; lower the frame budget", f);
        }
    }
}

#[test]
fn a_dungeon_reads_as_one_creature_all_the_way_down() {
    // Two floors of the same idea, getting harder. A dungeon whose floors
    // disagree is two dungeons somebody stapled together.
    for d in DUNGEONS.iter().filter(|d| d.id != "the-crevice") {
        let themes: Vec<MonsterTheme> =
            d.floors.iter().filter_map(|f| frame(f)).map(|f| f.theme).collect();
        assert!(!themes.is_empty());
        let bands: Vec<usize> = d.floors.iter().filter_map(|f| frame(f)).map(|f| f.band).collect();
        for w in bands.windows(2) {
            assert!(w[1] >= w[0], "{}: the floors get easier as you go down", d.id);
        }
        // WUMPUS WORLD is the one that changes, and it changes on purpose: the
        // dark floor is what *lives near* a wumpus, and the wumpus is not
        // that. Everything else holds one idea.
        if d.id != "wumpus-world" {
            assert!(
                themes.windows(2).all(|w| w[0] == w[1]),
                "{}: {:?} is two dungeons stapled together",
                d.id,
                themes
            );
        }
    }
}

#[test]
fn every_dungeon_pays_something_and_two_of_them_pay_no_class_at_all() {
    let no_class: Vec<&str> =
        DUNGEONS.iter().filter(|d| d.reward.is_empty()).map(|d| d.id).collect();
    assert_eq!(no_class, vec!["the-undertow", "den-rivals"]);
    for d in DUNGEONS {
        assert!(
            !d.reward.is_empty() || !d.also.is_empty(),
            "{} is a walk there and a walk back",
            d.id
        );
    }
}

#[test]
fn the_antechamber_pays_the_pool_and_the_class_is_only_the_marker() {
    let mut run = a_run();
    walk(&mut run, "the-threshold");
    assert!(run.insight_unlocked);
    assert!(run.classes.iter().any(|c| c.name == "Threshold-Sighted"));
}

#[test]
fn the_undertow_pays_a_row_on_a_board_of_your_choice() {
    // H3 cuts its class in favour of the Depth, and E6.10 asks that the row
    // move no placed piece and that its receipt name the slot.
    let mut run = a_run();
    let before: Vec<(SlotKind, u8)> =
        SlotKind::ALL.iter().map(|&k| (k, run.loadout.slot(k).rows())).collect();
    walk(&mut run, "the-undertow");
    assert_eq!(run.owed_rows, 1, "the Undertow paid nothing");
    for &(k, rows) in &before {
        assert_eq!(run.loadout.slot(k).rows(), rows, "a board grew before it was chosen");
    }
    assert!(run.grow_slot(SlotKind::Helmet));
    assert_eq!(run.loadout.slot(SlotKind::Helmet).rows(), before[SlotKind::Helmet.index()].1 + 1);
    let receipt = run.take_receipt().expect("a row is a resolution");
    assert!(receipt[0].contains("helmet"), "{:?}", receipt);
}

#[test]
fn den_rivals_pays_the_hide_the_exhibit_promised() {
    let mut run = a_run();
    walk(&mut run, "den-rivals");
    assert!(run.holds("Bearhide"), "the museum lied after all");
    assert!(
        gearmaster_engine::piece::is_event_only("Bearhide"),
        "the hide could be bought off a shelf"
    );
}

#[test]
fn the_mine_and_the_hunt_pay_classes_nothing_else_hands_out() {
    for (id, class) in [("the-under-mine", "Prospector"), ("wumpus-world", "Wumpus Hunter")] {
        let mut run = a_run();
        walk(&mut run, id);
        assert!(run.classes.iter().any(|c| c.name == class), "{} paid no {}", id, class);
        // Nothing you build points at one, so no fountain may pour it.
        let def = gearmaster_engine::class::CLASSES
            .iter()
            .find(|c| c.name == class)
            .expect("authored");
        assert!(def.requires.is_empty());
        assert!(gearmaster_engine::class::is_earned(class));
    }
}

#[test]
fn a_prospector_pries_gear_off_a_named_creature() {
    // The only thing in the game that changes what a corpse is worth. A
    // trophy is one piece off a creature carrying fifteen, and every one of
    // those fifteen is barred from every shelf there is.
    let mut run = a_run();
    let named = gearmaster_engine::combat::LADDER
        .iter()
        .position(|m| m.rank.is_named() && !m.gear.is_empty())
        .expect("the road is full of them");
    run.rung = named;
    run.force_win();
    run.settle();
    let without = run.last_settlement.clone().expect("settled").pried_off.len();
    assert_eq!(without, 0, "gear came off without the class");

    let mut run = a_run();
    run.classes
        .push(gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Prospector").unwrap());
    run.rung = named;
    run.force_win();
    run.settle();
    let with = run.last_settlement.clone().expect("settled").pried_off;
    assert_eq!(with.len(), 1, "a prospector took {:?}", with);
}

#[test]
fn a_hunter_lands_the_first_one_whatever_is_in_front_of_it() {
    use gearmaster_engine::combat::{Combatant, DamageType};
    use gearmaster_engine::stats::Stats;
    // Deflection turns a flat share off every physical blow. It does not
    // touch the first one.
    let mut target = Combatant::player(Stats::new(10_000, 0, 0, 100), &[]);
    target.deflection = 5;
    assert_eq!(target.take_typed(100, DamageType::Physical, 0).1, 50);
    assert_eq!(
        target.take_typed_with(100, DamageType::Physical, 0, true).1,
        100,
        "the first one was turned aside"
    );
}

#[test]
fn coming_out_of_one_puts_you_where_you_went_in() {
    let mut run = a_run();
    run.rung = 20;
    let rung = run.rung;
    walk(&mut run, "wumpus-world");
    assert_eq!(run.rung, rung, "a dungeon moved the ladder");
}

#[test]
fn losing_in_one_puts_you_out_of_it_and_the_door_does_not_reopen() {
    let mut run = a_run();
    run.rung = 20;
    run.enter_dungeon("the-under-mine");
    assert!(run.dungeon.is_some());
    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert!(run.dungeon.is_none(), "still down there after losing");
}

#[test]
fn every_dungeon_can_be_reached_by_something() {
    // A dungeon nobody can open is content nobody sees. Three routes exist:
    // an event's choice, a town door, and a pedestal.
    // A ratchet, not an exemption: these are the ones whose opener has not
    // been authored yet, and the list only ever gets shorter. THE FORK is
    // M14's and the two destinations are M12's.
    const NOT_YET: &[&str] = &["the-under-mine", "den-rivals", "wumpus-world"];
    for d in DUNGEONS {
        let by_event = gearmaster_engine::event::EVENTS.iter().any(|e| {
            e.choices.iter().any(|c| {
                matches!(c.outcome, Outcome::Enter(id) | Outcome::StartDungeon(id) if id == d.id)
            })
        });
        let by_door = gearmaster_engine::town::TOWNS.iter().any(|t| {
            t.actions.iter().any(|a| a.opens() == Some(d.id))
        });
        let by_orb = gearmaster_engine::pedestal::DESTINATIONS.iter().any(|x| {
            matches!(x.kind, gearmaster_engine::pedestal::Where::Dungeon(id) if id == d.id)
        });
        let opened = by_event || by_door || by_orb;
        if NOT_YET.contains(&d.id) {
            assert!(!opened, "{} has an opener now - take it off NOT_YET", d.id);
            continue;
        }
        assert!(opened, "{} is a dungeon nobody can open", d.id);
        assert!(by_id(d.id).is_some());
    }
}
