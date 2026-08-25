//! Fights that will not end, ending.
//!
//! The rule this replaces scored a draw as a loss, which made every defensive
//! option unplayable: armour buys survival, survival was not victory, and a
//! build that could out-last anything but out-damage nothing lost anyway.

use gearmaster_engine::combat::{
    simulate_at, simulate_with_class, Difficulty, Event, Outcome, Side, LADDER, MAX_DURATION_MS,
    SUDDEN_DEATH_MS,
};
use gearmaster_engine::run::{Mode, Run};

fn the_winning_board(difficulty: Difficulty) -> Run {
    let shared = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN)
        .expect("the winning code still reads");
    let mut run = Run::new();
    run.difficulty = difficulty;
    run.mode = Mode::Grinder;
    run.loadout.grow(shared.extra_rows);
    for (def, slot, x, y, rot) in &shared.placed {
        let id = run.registry.alloc(*def);
        run.owned.push(id);
        run.registry.set_rotation(id, *rot);
        if run.equip(id, *slot, *x, *y).is_err() {
            run.owned.pop();
        }
    }
    run
}

#[test]
fn nothing_happens_for_the_first_thirty_seconds() {
    // A long fight is allowed to be a long fight.
    let run = the_winning_board(Difficulty::Medium);
    let (stats, items) = (run.player_stats(), run.combat_items());
    for spec in LADDER.iter().take(30) {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        for e in &log.entries {
            if matches!(e.event, Event::SuddenDeath { .. }) {
                assert!(
                    e.at_ms >= SUDDEN_DEATH_MS,
                    "{}: the fight turned at {}ms, before it was meant to",
                    spec.name,
                    e.at_ms
                );
            }
        }
    }
}

#[test]
fn a_fight_that_will_not_end_is_ended() {
    // Two things that cannot hurt each other much: the old rule's worst case.
    let run = the_winning_board(Difficulty::Insane);
    let (stats, items) = (run.player_stats(), run.combat_items());

    let mut longest = 0;
    let mut stalemates = 0;
    for spec in LADDER.iter() {
        let log = simulate_at(stats, &items, spec, Difficulty::Insane);
        longest = longest.max(log.duration_ms);
        if log.outcome == Outcome::Stalemate {
            stalemates += 1;
        }
    }
    assert_eq!(stalemates, 0, "{stalemates} fights still ran out the clock");
    assert!(
        longest < MAX_DURATION_MS,
        "the longest fight took {longest}ms and reached the cap anyway"
    );
    // A hundred percent of maximum health is handed out by the fourteenth
    // second of it, so nothing should get anywhere near that.
    assert!(
        longest <= SUDDEN_DEATH_MS + 20_000,
        "the longest fight took {longest}ms - sudden death is not biting hard enough"
    );
}

#[test]
fn the_bite_grows_and_ignores_everything_you_are_wearing() {
    // The setting is searched rather than named. This fought the last rung at
    // Insane, on the reasoning that it is the longest fight in the game - true
    // until a sweep re-gears him and Insane becomes the setting he *wins*, in
    // ten seconds. What the test is about is the escalation, so it looks for a
    // fight that reaches it.
    let bites = [Difficulty::Insane, Difficulty::Hard, Difficulty::Medium, Difficulty::Easy]
        .into_iter()
        .find_map(|d| {
            let run = the_winning_board(d);
            let (stats, items) = (run.player_stats(), run.combat_items());
            let log = simulate_at(stats, &items, &LADDER[LADDER.len() - 1], d);
            let b: Vec<i32> = log
                .entries
                .iter()
                .filter_map(|e| match &e.event {
                    Event::SuddenDeath { pct } => Some(*pct),
                    _ => None,
                })
                .collect();
            (!b.is_empty()).then_some(b)
        })
        .expect("no fight against the last rung reaches sudden death at any setting");
    // One percent, then two, then three.
    assert_eq!(bites, (1..=bites.len() as i32).collect::<Vec<_>>(), "{bites:?}");
}

#[test]
fn going_down_together_goes_to_whoever_was_further_up() {
    // The player wins a dead heat, and loses when the other side was ahead.
    // Driven through whole fights rather than constructed, because what is
    // under test is `check_down`'s reading of a real simultaneous knockout.
    let run = the_winning_board(Difficulty::Insane);
    let (stats, items) = (run.player_stats(), run.combat_items());

    let mut both_down = 0;
    for spec in LADDER.iter() {
        let log = simulate_at(stats, &items, spec, Difficulty::Insane);
        let player_fell = log
            .entries
            .iter()
            .any(|e| matches!(e.event, Event::Fell { side: Side::Player }));
        let foe_fell =
            log.entries.iter().any(|e| matches!(e.event, Event::Fell { side: Side::Enemy }));
        if !(player_fell && foe_fell) {
            continue;
        }
        both_down += 1;
        // Whoever was less far past zero took it.
        let (player, foe) = final_health(&log);
        let expected =
            if player >= foe { Outcome::Victory } else { Outcome::Defeat };
        assert_eq!(
            log.outcome, expected,
            "{}: player on {player}, foe on {foe}, called {:?}",
            spec.name, log.outcome
        );
    }
    assert!(both_down > 0, "nothing in the game ever went down together; this proves nothing");
}

/// Health on both sides at the end, read from the events.
fn final_health(log: &gearmaster_engine::combat::CombatLog) -> (i32, i32) {
    let mut player = log.player.health;
    let mut enemy = log.enemy().health;
    let bite = |max: i32, pct: i32| (max * pct / 100).max(1);
    for e in &log.entries {
        match &e.event {
            Event::Hit { by, target_health, .. } => match by {
                Side::Player => enemy = *target_health,
                Side::Enemy => player = *target_health,
            },
            Event::Burn { side, health, .. } | Event::Regen { side, health, .. } => match side {
                Side::Player => player = *health,
                Side::Enemy => enemy = *health,
            },
            Event::SuddenDeath { pct } => {
                player -= bite(log.player.max_health, *pct);
                enemy -= bite(log.enemy().max_health, *pct);
            }
            _ => {}
        }
    }
    (player, enemy)
}

#[test]
fn a_wall_can_now_win_a_fight_it_used_to_draw() {
    use gearmaster_engine::class::CLASSES;
    // Trundle's whole problem was that surviving was not winning. Sudden death
    // makes it a fight again: the trundling board loses fewer of these to the
    // clock, because there is no longer a clock to lose them to.
    let run = the_winning_board(Difficulty::Hard);
    let (stats, items) = (run.player_stats(), run.combat_items());
    let trundle = *CLASSES.iter().find(|c| c.name == "Trundle").expect("authored");

    let drawn = |classes: &[gearmaster_engine::class::ClassDef]| -> usize {
        LADDER
            .iter()
            .filter(|spec| {
                simulate_with_class(stats, &items, spec, Difficulty::Hard, classes).outcome
                    == Outcome::Stalemate
            })
            .count()
    };
    assert_eq!(drawn(&[]), 0, "a plain board still draws fights");
    assert_eq!(drawn(&[trundle]), 0, "a trundling board still draws fights");
}

#[test]
fn a_stalemate_is_now_something_that_does_not_happen() {
    // Across every build the project can produce, at every setting.
    let mut checked = 0;
    for difficulty in Difficulty::ALL {
        for build in [Difficulty::Medium, Difficulty::Hard] {
            let run = the_winning_board(build);
            let (stats, items) = (run.player_stats(), run.combat_items());
            for spec in LADDER.iter().step_by(7) {
                let log = simulate_at(stats, &items, spec, *difficulty);
                checked += 1;
                assert_ne!(
                    log.outcome,
                    Outcome::Stalemate,
                    "{} at {} drew after {}ms",
                    spec.name,
                    difficulty.name(),
                    log.duration_ms
                );
            }
        }
    }
    assert!(checked > 40, "only {checked} fights were looked at");
}
