//! The room behind the velvet rope.
//!
//! Note on setup: `Run::with_all_pieces` starts holding one of every component
//! in the game, the Platinum Chip included, so a run built that way walks
//! straight past a door that is meant to be locked. Anything testing the lock
//! has to start from a run that has genuinely never been to the casino.

use gearmaster_engine::combat::{Difficulty, Outcome, LADDER};
use gearmaster_engine::event::{Outcome as ChoiceOutcome, EVENTS};
use gearmaster_engine::piece::{is_off_the_scale, SlotKind, CATALOG, VIP_ONLY};
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::slot::SLOT_H;

fn vip() -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == "the-vip-area").expect("the VIP area is authored")
}

/// A run that owns nothing but its starting kit - no chip.
fn a_plain_run() -> Run {
    let mut run = Run::new();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.rung = vip().at;
    run
}

fn give(run: &mut Run, name: &str) {
    let d = CATALOG.iter().position(|d| d.name == name).expect("a real component");
    let id = run.registry.alloc(d);
    run.owned.push(id);
}

#[test]
fn the_door_is_always_described_and_not_always_open() {
    let mut run = a_plain_run();
    let ev = run.pending_event().expect("the VIP area stands here whatever you are carrying");
    assert_eq!(ev.id, "the-vip-area");

    // Without the chip, the two interesting branches are shut and say so.
    let gated: Vec<&gearmaster_engine::event::Choice> = ev
        .choices
        .iter()
        .filter(|c| !matches!(c.outcome, ChoiceOutcome::FightAsWritten))
        .collect();
    assert_eq!(gated.len(), 2, "keeping cover and getting them out");
    for c in &gated {
        assert!(!run.choice_open(c), "{} opened without the chip", c.label);
        assert!(!c.unmet.is_empty(), "{} closes without saying why", c.label);
    }
    // And there is always a way past.
    assert!(ev.choices.iter().any(|c| run.choice_open(c)), "no way out of the event at all");

    give(&mut run, "Platinum Chip");
    for c in &gated {
        assert!(run.choice_open(c), "{} stayed shut while holding the chip", c.label);
    }
}

#[test]
fn keeping_your_cover_stocks_the_shelves_and_costs_you_healing() {
    let mut run = a_plain_run();
    give(&mut run, "Platinum Chip");
    let ev = run.pending_event().expect("open");
    let deal = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Stock { .. }))
        .expect("the bargain is a choice");
    run.take_choice(deal);

    let on_sale: Vec<&str> =
        run.shop.stock_defs().iter().map(|d| d.name).collect();
    assert_eq!(on_sale.len(), VIP_ONLY.len(), "the shelves are not what was laid out");
    for name in VIP_ONLY {
        assert!(on_sale.contains(name), "{name} was not on the table");
    }

    assert!(
        run.classes.iter().any(|c| c.name == "Immense Guilt"),
        "walked out of there feeling fine"
    );
    // The chip is a key, not a toll.
    assert!(run.owned.iter().any(|&i| run.registry.def(i).name == "Platinum Chip"));
}

#[test]
fn immense_guilt_actually_stops_you_healing() {
    use gearmaster_engine::combat::{simulate_with_class, Event, Side};
    use gearmaster_engine::class::CLASSES;

    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.apply_preset();
    let (stats, items) = (run.player_stats(), run.combat_items());
    assert!(stats.regen > 0, "a build with no regeneration proves nothing");

    let guilt = *CLASSES.iter().find(|c| c.name == "Immense Guilt").expect("authored");
    let healed = |classes: &[gearmaster_engine::class::ClassDef]| -> usize {
        LADDER
            .iter()
            .take(20)
            .map(|spec| {
                let log = simulate_with_class(stats, &items, spec, Difficulty::Medium, classes);
                log.entries
                    .iter()
                    .filter(|e| matches!(e.event, Event::Regen { side: Side::Player, .. }))
                    .count()
            })
            .sum()
    };
    assert!(healed(&[]) > 0, "the control never healed either");
    assert_eq!(healed(&[guilt]), 0, "guilt let a point of health back");
}

#[test]
fn no_fountain_ever_offers_guilt() {
    assert!(
        gearmaster_engine::class::is_earned("Immense Guilt"),
        "a fountain could pour it, which would be a fountain making your run worse"
    );
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    assert!(
        run.class_outlook().iter().all(|m| m.class.name != "Immense Guilt"),
        "the fountain is offering guilt"
    );
}

#[test]
fn an_earned_class_does_not_use_up_a_fountain() {
    // The bug this guards: `at_fountain` counted every class held, so a class
    // won anywhere else advanced the schedule past a fountain the player had
    // not been to. Clearing the crevice before rung fourteen used to mean the
    // second fountain simply never appeared.
    let mut run = Run::new();
    let before = run.next_fountain();
    assert!(before.is_some(), "there are fountains to miss");

    let earned = gearmaster_engine::class::CLASSES
        .iter()
        .find(|c| gearmaster_engine::class::is_earned(c.name))
        .expect("some class is earned rather than poured");
    run.classes.push(earned);

    assert_eq!(run.next_fountain(), before, "an earned class ate a fountain");
}

#[test]
fn getting_them_out_is_two_hard_ones_and_a_row() {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.rung = vip().at;
    let ev = run.pending_event().expect("open");
    let rescue = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Step(_)))
        .expect("the rescue is a choice");
    run.take_choice(rescue);

    let specs = run.pending_brawl().expect("two guards");
    assert_eq!(specs.len(), 2);

    let rows_before = run.loadout.rows();
    let rung_before = run.rung;
    run.force_win();
    run.settle();

    assert_eq!(run.loadout.rows(), rows_before + 1, "no extra row for the trouble");
    assert_eq!(run.extra_rows, 1);
    assert_eq!(run.rung, rung_before, "a detour moved the ladder");
    assert!(
        run.owned.iter().filter(|&&i| run.registry.def(i).name == "Sprocketman's Gratitude").count()
            >= 1
    );
    assert_eq!(run.last_settlement.as_ref().map(|s| s.rows_won), Some(1));
}

#[test]
fn losing_the_back_room_costs_a_life() {
    // Unlike the casino table. This one is a decision about somebody else.
    let b = &gearmaster_engine::event::THE_BACK_ROOM;
    assert!(!b.forgiving, "the back room is not a free bet");
    assert_eq!(b.and_grow, 1);
    assert_eq!(b.win, "Sprocketman's Gratitude");
}

#[test]
fn the_five_are_off_the_scale_and_off_the_shelves() {
    for name in VIP_ONLY {
        assert!(is_off_the_scale(name), "{name} would deflate the price of its whole slot");
        let d = CATALOG.iter().find(|d| d.name == *name).expect("in the catalogue");
        assert!(!d.name.is_empty());
    }
    // One per slot, so every build has something to want.
    let mut slots: Vec<SlotKind> = VIP_ONLY
        .iter()
        .map(|n| CATALOG.iter().find(|d| d.name == *n).unwrap().slot)
        .collect();
    slots.sort_by_key(|s| s.index());
    slots.dedup_by_key(|s| s.index());
    assert_eq!(slots.len(), 5, "two of the five share a slot");
}

#[test]
fn the_gratitude_is_not_for_sale() {
    assert!(gearmaster_engine::piece::is_event_only("Sprocketman's Gratitude"));
}

#[test]
fn the_vip_area_stands_where_it_says() {
    let e = vip();
    assert_eq!(LADDER[e.at].name, e.expects);
    assert!((25..35).contains(&e.at), "the VIP area drifted off rung thirty");
    assert_eq!(EVENTS.iter().filter(|o| o.at == e.at).count(), 1);
    // The row it hands out is the largest single reward in the game, so it
    // must not be reachable before the boards are worth growing.
    assert!(e.at > 20, "thirty more cells handed out in the shallow end");
    let _ = SLOT_H;
}
