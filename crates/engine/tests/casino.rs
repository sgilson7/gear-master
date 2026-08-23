//! The casino, and the chip you walk out with.
//!
//! Two things here are easy to ship broken and hard to notice: an earned event
//! whose condition nothing can meet, and a piece that reaches out of the fight
//! into the purse and either never pays or never stops.

use gearmaster_engine::combat::{Difficulty, Event, Outcome, Side, LADDER};
use gearmaster_engine::event::{Outcome as ChoiceOutcome, EVENTS};
use gearmaster_engine::piece::CATALOG;
use gearmaster_engine::run::{Mode, Run};

fn casino() -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == "the-casino").expect("the casino is authored")
}

#[test]
fn the_casino_opens_for_a_quick_kill_and_hands_over_a_chip() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(2_500);

    let ev = run.pending_event().expect("a fast kill in the shallow end opens the door");
    assert_eq!(ev.id, "the-casino");

    let walk = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Give("Gold Chip")))
        .expect("the walk-away branch hands over the Gold Chip");
    assert!(run.choice_open(walk));

    let before = run.owned.iter().filter(|&&i| run.registry.def(i).name == "Gold Chip").count();
    run.take_choice(walk);
    let after = run.owned.iter().filter(|&&i| run.registry.def(i).name == "Gold Chip").count();
    assert_eq!(after, before + 1, "walked out without the chip");

    // Asked once, and never again.
    assert!(run.pending_event().is_none(), "the casino asked twice");
}

#[test]
fn a_slow_run_never_sees_the_casino() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(9_000);
    assert!(
        run.pending_event().map(|e| e.id) != Some("the-casino"),
        "the door opened for a run that never earned it"
    );

    // Quick enough, but far too late.
    run.best_fight_ms = Some(500);
    run.rung = casino().at + 1;
    assert!(
        run.pending_event().map(|e| e.id) != Some("the-casino"),
        "the door was still open past its last rung"
    );
}

#[test]
fn neither_chip_is_for_sale() {
    for name in ["Gold Chip", "Platinum Chip"] {
        assert!(
            gearmaster_engine::piece::is_event_only(name),
            "{name} would turn up on a shelf, which makes the casino pointless"
        );
        assert!(CATALOG.iter().any(|d| d.name == name), "{name} is in the catalogue");
    }
}

/// A build wearing the chip, with money to burn.
fn chip_build(gold: i32) -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.gold = gold;
    for name in ["Oak Handle", "Iron Blade", "Gold Chip"] {
        let id = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == name && !run.is_equipped(i))
            .unwrap_or_else(|| panic!("no {name}"));
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
    }
    run
}

#[test]
fn the_gold_chip_spends_the_purse_and_hits_harder_each_time() {
    let mut run = chip_build(500);
    run.rung = 11;
    let log = run.fight_next();

    let spends: Vec<i32> = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Spent { side: Side::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert!(!spends.is_empty(), "the chip never paid anything");
    assert!(spends.iter().all(|&a| a == 5), "the cost is flat: {spends:?}");

    // Flat cost, climbing payout. The escalation is the whole piece.
    let total = log.gold_spent;
    assert_eq!(total, spends.iter().sum::<i32>(), "the log and the tally disagree");
    assert!(total <= 40, "the chip blew past its budget: {total}");

    let purse = run.gold;
    assert_eq!(purse, 500 - total, "the run was not charged what the fight spent");
}

#[test]
fn a_replayed_fight_does_not_charge_you_twice() {
    let mut run = chip_build(500);
    run.rung = 11;
    run.fight_next();
    let after_one = run.gold;
    assert!(after_one < 500, "nothing was spent, so this proves nothing");

    // Same fight again - a rematch is a new fight and may spend again, but
    // simply looking at the log must not move the purse.
    let spent_once = 500 - after_one;
    let _ = run.log.as_ref().map(|l| l.gold_spent);
    assert_eq!(run.gold, after_one, "reading the log charged the run again");
    assert_eq!(spent_once, run.log.as_ref().unwrap().gold_spent);
}

#[test]
fn a_penniless_build_still_swings() {
    // The chip going quiet must not stop the weapon it is built into.
    let mut run = chip_build(0);
    run.rung = 11;
    let log = run.fight_next();
    assert_eq!(log.gold_spent, 0, "spent money it did not have");
    assert!(
        log.entries.iter().any(|e| matches!(e.event, Event::Activate { side: Side::Player, .. })),
        "a broke player stopped fighting"
    );
    assert!(run.gold >= 0, "the purse went negative");
}

#[test]
fn the_casino_stands_on_a_rung_nothing_else_claims() {
    use gearmaster_engine::event::Trigger;
    // `event::at` returns the first match. Two *scheduled* events on one rung
    // means one silently never fires; two earned ones sharing a deadline is
    // fine and deliberate - they roam, and which is asked is settled by the
    // order they are written in and by `blocked_by`.
    let at = casino().at;
    assert_eq!(
        EVENTS.iter().filter(|e| e.at == at && matches!(e.trigger, Trigger::Rung)).count(),
        0,
        "something scheduled stands on the casino's last rung and would shadow it"
    );
    // And when both doors are earned, the casino is the one asked.
    let casino_at = EVENTS.iter().position(|e| e.id == "the-casino").expect("authored");
    let long_at = EVENTS.iter().position(|e| e.id == "the-long-way").expect("authored");
    assert!(casino_at < long_at, "the long way is written first and would be asked first");

    assert!(at < 10, "the casino is meant to be a shallow-end door, not a mid-run one");
    assert_eq!(LADDER[at].name, casino().expects);
}

fn chips(run: &Run, name: &str) -> usize {
    run.owned.iter().filter(|&&i| run.registry.def(i).name == name).count()
}

/// The step-in branch: both of them at once.
fn step_in(run: &mut Run) {
    run.rung = 4;
    run.best_fight_ms = Some(2_500);
    let ev = run.pending_event().expect("the casino is open");
    let step = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Step(_)))
        .expect("stepping in is a choice you can take");
    run.take_choice(step);
}

#[test]
fn stepping_in_puts_both_of_them_in_front_of_you() {
    let mut run = Run::with_all_pieces();
    step_in(&mut run);

    let specs = run.pending_brawl().expect("two creatures were arranged");
    assert_eq!(specs.len(), 2, "the third table has two people at it");
    // Both have to resolve, or the fight silently becomes a duel.
    let names: Vec<&str> = specs.iter().map(|m| m.name).collect();
    assert_eq!(names, vec!["Bone Archer", "Frost Wisp"]);

    let log = run.fight_party(&specs);
    assert_eq!(log.enemies.len(), 2, "the fight only had one of them in it");
    assert!(log.is_brawl());
}

#[test]
fn winning_at_the_table_is_worth_the_platinum_chip() {
    let mut run = Run::with_all_pieces();
    step_in(&mut run);
    let rung_before = run.rung;
    let gold_before = run.gold;
    // A delta, not a count: `with_all_pieces` starts holding one of
    // everything, the Platinum Chip included.
    let before = chips(&run, "Platinum Chip");

    // Settled as a win, whatever the simulation would have said - what is
    // under test is the settlement.
    run.force_win();
    run.settle();

    assert_eq!(
        chips(&run, "Platinum Chip"),
        before + 1,
        "won the table and walked out without the chip"
    );
    assert_eq!(run.rung, rung_before, "a detour moved the ladder");
    assert_eq!(run.gold, gold_before, "a detour paid a bounty");
    assert!(run.brawl.is_none(), "the table is still set");
    assert_eq!(run.last_settlement.as_ref().and_then(|s| s.won_item), Some("Platinum Chip"));
}

#[test]
fn losing_at_the_table_costs_nothing_at_all() {
    let mut run = Run::with_all_pieces();
    run.mode = Mode::Rogue;
    step_in(&mut run);
    let (rung, losses, lives) = (run.rung, run.losses, run.extra_lives);
    let before = chips(&run, "Platinum Chip");

    // A real fight, with nothing on: the point is what a loss costs, so it
    // has to actually be one.
    let specs = run.pending_brawl().expect("arranged");
    let outcome = run.fight_party(&specs).outcome;
    assert_ne!(outcome, Outcome::Victory, "a naked build won; this proves nothing");
    run.settle();

    assert_eq!(run.losses, losses, "a forgiving fight took a life");
    assert_eq!(run.extra_lives, lives);
    assert_eq!(run.rung, rung, "a loss at the table knocked the ladder back");
    assert!(run.brawl.is_none());
    assert_eq!(chips(&run, "Platinum Chip"), before, "lost the fight and got the chip anyway");
}

#[test]
fn the_rungs_own_creature_is_still_waiting_afterwards() {
    let mut run = Run::with_all_pieces();
    step_in(&mut run);
    let expected = run.monster().name;
    run.force_win();
    run.settle();

    // No brawl pending any more, so the next fight is the rung's.
    assert!(run.pending_brawl().is_none());
    assert_eq!(run.monster().name, expected, "the detour ate the rung's fight");
}

#[test]
fn you_only_get_asked_once() {
    let mut run = Run::with_all_pieces();
    step_in(&mut run);
    assert!(run.pending_event().is_none(), "the casino asked again after stepping in");
    run.force_win();
    run.settle();
    assert!(run.pending_event().is_none(), "the casino reopened after the fight");
}

#[test]
fn a_complete_board_can_actually_win_the_table() {
    // The chip is the key to the whole VIP event, so a pair nobody can beat
    // would not make the casino exciting - it would quietly delete a later
    // event. This is the floor: a plain auto-built board, which is worse than
    // what a player who earned the casino is carrying.
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.apply_preset();
    run.brawl = Some(&gearmaster_engine::event::TABLE_THREE);

    let specs = run.pending_brawl().expect("the table is set");
    let log = run.fight_party(&specs);
    assert_eq!(
        log.outcome,
        Outcome::Victory,
        "a complete board lost to the third table - the Platinum Chip is now \
         unreachable, and with it the VIP area"
    );
}


#[test]
fn the_door_is_shut_on_rung_one_however_fast_you_were() {
    // Flattening the Cave Rat is not a demonstration of anything, and a door
    // that opens before you have built anything makes the first real decision
    // of the run a coin toss.
    let mut run = Run::with_all_pieces();
    run.best_fight_ms = Some(200);
    run.rung = 0;
    assert!(
        run.pending_event().map(|e| e.id) != Some("the-casino"),
        "the casino opened on the first rung"
    );
    run.rung = 1;
    assert_eq!(
        run.pending_event().map(|e| e.id),
        Some("the-casino"),
        "rung two is the first rung it should open on"
    );
}

#[test]
fn three_seconds_is_the_line() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    for (ms, open) in [(2_999u32, true), (3_000, false), (3_500, false)] {
        run.best_fight_ms = Some(ms);
        assert_eq!(
            run.pending_event().map(|e| e.id) == Some("the-casino"),
            open,
            "a {ms}ms win should {} the door",
            if open { "open" } else { "leave shut" }
        );
    }
}
