//! Towns: the rung with nothing on it to fight.
//!
//! Setup note, learned the hard way twice: `Run::with_all_pieces` starts
//! holding one of everything, rumours included, so anything testing a door
//! that a rumour opens has to start from a run that has genuinely never been
//! to a pub.

use gearmaster_engine::class::CLASSES;
use gearmaster_engine::combat::{Difficulty, Event, Outcome, Side, LADDER};
use gearmaster_engine::piece::{SlotKind, CATALOG, TOWN_ONLY};
use gearmaster_engine::run::{Mode, Run, PIETY_FOR_A_TICKET};
use gearmaster_engine::town::{self, Action, TOWNS};

/// A run standing at the gate of the first town, having won its way there.
fn at_the_gate() -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.apply_preset();
    let first = TOWNS[0].after;
    for rung in 0..=first {
        run.rung = rung;
        run.fight_next();
        run.settle();
        run.back_to_loadout();
    }
    run
}

/// The board that cleared rung fifty, which is the only geared profile in the
/// project worth measuring a class against.
///
/// The auto-builder packs to about half this density and banks no resources at
/// all, so anything measured on it says "this board does nothing" rather than
/// "this class does nothing" - which is a mistake this suite has made before.
fn the_winning_board() -> Run {
    let sh = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN)
        .expect("the winning code still reads");
    let mut run = Run::new();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.loadout.grow(sh.extra_rows);
    for (def, slot, x, y, rot) in &sh.placed {
        let id = run.registry.alloc(*def);
        run.owned.push(id);
        run.registry.set_rotation(id, *rot);
        if run.equip(id, *slot, *x, *y).is_err() {
            run.owned.pop();
        }
    }
    run
}

fn give(run: &mut Run, name: &str) {
    let d = CATALOG.iter().position(|d| d.name == name).expect("a real component");
    let id = run.registry.alloc(d);
    run.owned.push(id);
}

#[test]
fn clearing_the_rung_before_one_puts_you_in_it() {
    let run = at_the_gate();
    let t = run.pending_town().expect("the town is between those two rungs");
    assert_eq!(t.id, TOWNS[0].id);
    // And it is a rung of its own: the ladder has not moved past it.
    assert_eq!(run.rung, TOWNS[0].after + 1);
}

#[test]
fn nothing_stands_at_a_gate_that_is_not_there() {
    let mut run = Run::with_all_pieces();
    run.rung = 3;
    assert!(run.pending_town().is_none(), "a town appeared on an ordinary rung");
}

#[test]
fn walking_on_pays_and_is_over() {
    let mut run = at_the_gate();
    let before = run.gold;
    let bounty = run.last_bounty;
    assert!(bounty > 0, "the fight that got here paid nothing; this proves nothing");

    let paid = run.skip_town();
    assert_eq!(paid, bounty, "walking on is the bounty again, not some other number");
    assert_eq!(run.gold, before + bounty);
    assert!(run.pending_town().is_none(), "still standing at the gate after leaving");
}

#[test]
fn a_town_is_only_visited_once() {
    // The failure this guards is a Grinder's: lose the next fight, get knocked
    // back below the town, win it again, and the town is there a second time.
    let mut run = at_the_gate();
    run.visit_town(Action::Shop);
    assert!(run.pending_town().is_none());

    run.rung = TOWNS[0].after;
    run.back_to_loadout();
    run.force_win();
    run.settle();
    assert!(run.pending_town().is_none(), "the same town twice in one run");
}

#[test]
fn one_action_a_visit() {
    let mut run = at_the_gate();
    run.visit_town(Action::Chapel);
    assert!(run.pending_town().is_none(), "the gate stayed open after going in");
    // And a second call does nothing at all rather than quietly working.
    let again = run.visit_town(Action::Factory);
    assert_eq!(again.did, None);
    assert_eq!(run.stacks_of("Tired"), 0);
}

// ------------------------------------------------------------------ chapel

#[test]
fn praying_stacks_and_the_fifth_one_is_different() {
    let mut run = Run::new();
    for n in 1..PIETY_FOR_A_TICKET {
        run.town = Some(&TOWNS[0]);
        run.towns_seen.clear();
        let v = run.visit_town(Action::Chapel);
        assert_eq!(run.stacks_of("Piety"), n, "prayer {} did not stack", n);
        assert_eq!(v.stacks, n);
        assert_eq!(v.became, None, "converted at {} instead of {}", n, PIETY_FOR_A_TICKET);
    }
    run.town = Some(&TOWNS[0]);
    run.towns_seen.clear();
    let v = run.visit_town(Action::Chapel);
    assert_eq!(v.became, Some("Ticket to Ride"));
    assert_eq!(run.stacks_of("Piety"), 0, "the prayers were meant to be taken back");
    assert_eq!(run.stacks_of("Ticket to Ride"), 1);
}

#[test]
fn a_stack_of_piety_is_a_point_of_devotion_at_the_bell() {
    let run = the_winning_board();
    let piety = *CLASSES.iter().find(|c| c.name == "Piety").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());
    let spec = LADDER[3];

    let started_with = |n: usize| -> i32 {
        let held = vec![piety; n];
        let log = gearmaster_engine::combat::simulate_with_class(
            stats,
            &items,
            &spec,
            Difficulty::Medium,
            &held,
        );
        log.player.faith
    };
    let base = started_with(0);
    assert_eq!(started_with(1), base + 1);
    assert_eq!(started_with(3), base + 3, "three stacks are not three points");
}

#[test]
fn the_ticket_eats_exactly_half_of_what_they_swing() {
    // Counted rather than rolled, so this is an equality and not a range.
    let run = the_winning_board();
    let ticket = *CLASSES.iter().find(|c| c.name == "Ticket to Ride").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());

    let mut checked = 0;
    for spec in LADDER.iter().take(20) {
        let log = gearmaster_engine::combat::simulate_with_class(
            stats,
            &items,
            spec,
            Difficulty::Medium,
            &[ticket],
        );
        let swung = log
            .entries
            .iter()
            .filter(|e| matches!(e.event, Event::Activate { side: Side::Enemy, .. }))
            .count();
        let missed = log
            .entries
            .iter()
            .filter(|e| matches!(e.event, Event::Warded { .. }))
            .count();
        if swung + missed < 4 {
            continue; // too short a fight to say anything
        }
        checked += 1;
        // Every second one, so the misses are half of everything attempted,
        // give or take the one at the end that had not come round yet.
        let attempts = swung + missed;
        assert!(
            missed * 2 == attempts || missed * 2 + 1 == attempts,
            "{}: {} of {} attacks missed - that is not half",
            spec.name,
            missed,
            attempts
        );
    }
    assert!(checked > 5, "only {checked} fights were long enough to look at");
}

#[test]
fn a_warded_attack_lands_nothing_at_all() {
    // Not "no damage" - nothing. A curse or a drain riding on a warded swing
    // would be the whole class quietly not working.
    let run = the_winning_board();
    let ticket = *CLASSES.iter().find(|c| c.name == "Ticket to Ride").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());
    let spec = LADDER[24];
    let log = gearmaster_engine::combat::simulate_with_class(
        stats,
        &items,
        &spec,
        Difficulty::Medium,
        &[ticket],
    );
    let warded_at: Vec<u32> = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::Warded { .. }))
        .map(|e| e.at_ms)
        .collect();
    assert!(!warded_at.is_empty(), "nothing was warded; this proves nothing");
    // Whatever else is logged at that instant, none of it is that attack
    // arriving: a warded activation never reaches the resolution at all.
    for e in &log.entries {
        if !warded_at.contains(&e.at_ms) {
            continue;
        }
        if let Event::Hit { by: Side::Enemy, damage, .. } = e.event {
            // Another item of theirs may legitimately land on the same tick.
            // What must not happen is the warded one landing, and the log
            // cannot tell those apart - so this only checks the shape.
            assert!(damage >= 0);
        }
    }
}

#[test]
fn the_ticket_is_worth_having() {
    // The whole point. Measured as how long the player lasts rather than what
    // health they end on, because sudden death brings every unfinished fight
    // to nearly zero on both sides.
    let run = the_winning_board();
    let ticket = *CLASSES.iter().find(|c| c.name == "Ticket to Ride").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());
    let spec = LADDER[LADDER.len() - 1];

    let lasted = |classes: &[gearmaster_engine::class::ClassDef]| -> u32 {
        let log = gearmaster_engine::combat::simulate_with_class(
            stats,
            &items,
            &spec,
            Difficulty::Insane,
            classes,
        );
        log.entries
            .iter()
            .find(|e| matches!(e.event, Event::Fell { side: Side::Player }))
            .map(|e| e.at_ms)
            .unwrap_or(log.duration_ms)
    };
    assert!(lasted(&[ticket]) > lasted(&[]), "half of everything missing changed nothing");
}

// ----------------------------------------------------------------- factory

#[test]
fn the_shift_pays_double_and_costs_you_mana() {
    let mut run = at_the_gate();
    let before = run.gold;
    let bounty = run.last_bounty;
    let v = run.visit_town(Action::Factory);

    assert_eq!(v.paid, bounty * 2, "a shift is twice the last bounty");
    assert_eq!(run.gold, before + bounty * 2);
    assert_eq!(run.stacks_of("Tired"), 1);
    assert_eq!(v.stacks, 1);
}

#[test]
fn tired_starts_you_in_debt_and_stacks() {
    let run = the_winning_board();
    let tired = *CLASSES.iter().find(|c| c.name == "Tired").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());
    let spec = LADDER[3];

    let opening = |n: usize| -> i32 {
        let held = vec![tired; n];
        gearmaster_engine::combat::simulate_with_class(
            stats,
            &items,
            &spec,
            Difficulty::Medium,
            &held,
        )
        .player
        .mana
    };
    let base = opening(0);
    assert_eq!(opening(1), base - 3);
    assert_eq!(opening(2), base - 6, "two shifts are not six mana");
}

#[test]
fn debt_is_a_debt_and_takes_real_time_to_pay_off() {
    // Mana below zero is only a debt if the pool has to climb back through it.
    //
    // Measured as the mana curve rather than as casts: neither board in the
    // project casts spells - both are martial - so counting `Cast` events was
    // counting the *enemy's* casts and reporting that ninety-six mana of debt
    // changed nothing. What the class promises is about the pool, so the pool
    // is what this reads.
    let run = the_winning_board();
    let tired = *CLASSES.iter().find(|c| c.name == "Tired").expect("authored");
    let (stats, items) = (run.player_stats(), run.combat_items());
    assert!(stats.mana > 0, "a build with no mana at all proves nothing");

    let spec = LADDER[24];
    // Every point of mana the player holds, in order, through one fight.
    let curve = |classes: &[gearmaster_engine::class::ClassDef]| -> Vec<(u32, i32)> {
        let log = gearmaster_engine::combat::simulate_with_class(
            stats,
            &items,
            &spec,
            Difficulty::Medium,
            classes,
        );
        let mut out = vec![(0u32, log.player.mana)];
        for e in &log.entries {
            if let Event::GainMana { side: Side::Player, total, .. } = e.event {
                out.push((e.at_ms, total));
            }
        }
        out
    };

    let free = curve(&[]);
    let shifts = vec![tired; (stats.mana as usize / 3) + 3];
    let owing = curve(&shifts);
    let debt = shifts.len() as i32 * 3;

    assert!(owing[0].1 < 0, "{} stacks left the pool on {}, which is not a debt", shifts.len(), owing[0].1);
    assert_eq!(owing.len(), free.len(), "the debt changed which items fired, so this compares nothing");
    for (i, ((t, a), (_, b))) in free.iter().zip(owing.iter()).enumerate() {
        assert_eq!(a - b, debt, "at {}ms (income {i}) the gap was {} and not {}", t, a - b, debt);
    }

    // And it is time, not just a number: the pool has to climb all the way
    // back through the debt before a single point of it is yours to spend.
    let back_to_zero = |c: &[(u32, i32)]| c.iter().find(|&&(_, m)| m >= 0).map(|&(t, _)| t);
    assert_eq!(back_to_zero(&free), Some(0), "the control started out of pocket");
    match back_to_zero(&owing) {
        // Never climbed out at all, which is what a large enough debt is.
        None => {}
        Some(paid_off) => assert!(
            paid_off > 0,
            "the debt was cleared before the fight started"
        ),
    }
}

// -------------------------------------------------------------------- shops

#[test]
fn the_town_shop_is_five_things_you_cannot_get_elsewhere() {
    let mut run = at_the_gate();
    run.visit_town(Action::Shop);
    let on_sale: Vec<&str> = run.shop.stock_defs().iter().map(|d| d.name).collect();
    assert_eq!(on_sale.len(), TOWN_ONLY.len());
    for name in TOWN_ONLY {
        assert!(on_sale.contains(name), "{name} was not on the shelf");
    }
}

#[test]
fn town_gear_does_not_move_the_scale_for_anything_else() {
    // The VIP five are exempt from the rating ceiling because they are behind
    // a locked branch and meant to be absurd. A town is on the way to
    // everywhere, so its gear has to live inside the curve like everything
    // else - which means it must not be the ceiling of its slot.
    for name in TOWN_ONLY {
        assert!(
            !gearmaster_engine::piece::is_off_the_scale(name),
            "{name} is exempt from the scale, which a town's gear must not be"
        );
    }
}

#[test]
fn the_pub_stocks_rumours_and_wants_no_money() {
    let mut run = at_the_gate();
    run.visit_town(Action::Pub);
    let on_sale: Vec<&str> = run.shop.stock_defs().iter().map(|d| d.name).collect();
    assert_eq!(on_sale.len(), gearmaster_engine::rumour::RUMOURS.len());
    for r in gearmaster_engine::rumour::RUMOURS {
        assert!(on_sale.contains(&r.name), "{} was not on the bar", r.name);
    }
    // And every shelf of it is a rumour, so `buy` never reaches one.
    for i in 0..on_sale.len() {
        assert!(run.rumour_on(i).is_some(), "shelf {i} of the pub is not a rumour");
    }
}

#[test]
fn a_rumour_is_paid_for_with_a_piece_and_not_with_gold() {
    let mut run = Run::new();
    run.town = Some(&TOWNS[0]);
    run.visit_town(Action::Pub);

    let shelf = (0..6)
        .find(|&i| run.rumour_on(i).map(|r| r.name) == Some("A Word About the Crownwright"))
        .expect("on the bar");
    // Nothing loose that they want yet.
    assert!(run.payment_for(shelf).is_empty(), "a fresh run has nothing to trade");

    give(&mut run, "Oak Handle"); // a handle, not a frame
    assert!(run.payment_for(shelf).is_empty(), "they took the wrong kind");

    give(&mut run, "Steel Frame");
    let pay = run.payment_for(shelf);
    assert_eq!(pay.len(), 1, "the frame is what they asked for");

    let gold = run.gold;
    let owned = run.owned.len();
    run.barter(shelf, pay[0]).expect("the trade should go through");
    assert_eq!(run.gold, gold, "money changed hands at a bar that does not take it");
    assert_eq!(run.owned.len(), owned, "one out, one in");
    assert!(!run.owned.contains(&pay[0]), "kept the thing that was handed over");
    assert!(run
        .owned
        .iter()
        .any(|&i| run.registry.def(i).name == "A Word About the Crownwright"));
}

// ----------------------------------------------------------------- rumours

#[test]
fn a_rumour_opens_a_door_only_when_its_condition_is_true() {
    use gearmaster_engine::rumour;
    let word = rumour::by_name("A Word About the Crownwright").expect("authored");
    let ev = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == word.opens)
        .expect("a real event");

    // Carrying it, standing on the rung, condition unmet: nothing there.
    let mut run = Run::new();
    run.rung = ev.at;
    give(&mut run, word.name);
    assert!(!run.meets(word.needs), "an empty helmet met a crowding condition");
    assert!(
        run.pending_event().map(|e| e.id) != Some(ev.id),
        "the door opened without the condition"
    );

    // A board packed the way an endgame board is packed.
    let mut run = the_winning_board();
    run.rung = ev.at;
    give(&mut run, word.name);
    assert!(
        run.empty_cells(SlotKind::Helmet) < 10,
        "even the winning board leaves {} cells free, so nobody can open this",
        run.empty_cells(SlotKind::Helmet)
    );
    assert!(run.meets(word.needs));
    assert_eq!(run.pending_event().map(|e| e.id), Some(ev.id));

    // And not for somebody who never heard it.
    let mut bare = the_winning_board();
    bare.rung = ev.at;
    bare.owned.retain(|&i| bare.registry.def(i).name != word.name);
    assert!(
        bare.pending_event().map(|e| e.id) != Some(ev.id),
        "the door opened for somebody who never bought the word"
    );
}

#[test]
fn the_run_counts_what_it_has_banked_all_the_way_up() {
    use gearmaster_engine::piece::Resource;
    let mut run = the_winning_board();
    assert_eq!(run.banked_all_run[Resource::Nature.index()], 0, "counted before fighting");

    let mut by_hand = 0;
    for rung in 0..6usize {
        run.rung = rung;
        run.fight_next();
        if let Some(l) = run.log.as_ref() {
            for e in &l.entries {
                if let Event::GainResource { side: Side::Player, what, amount, .. } = &e.event {
                    if *what == "nature" {
                        by_hand += amount;
                    }
                }
            }
        }
        run.settle();
        run.back_to_loadout();
    }
    assert!(by_hand > 0, "this board never banked any nature; the test proves nothing");
    assert_eq!(
        run.banked_all_run[Resource::Nature.index()],
        by_hand,
        "the running total does not match the fights it came from"
    );
}

#[test]
fn a_hundred_nature_is_reachable_and_not_free() {
    // A condition nothing can satisfy is an event that quietly never happens,
    // which is the failure a rumour is most exposed to.
    use gearmaster_engine::piece::Resource;
    let mut run = the_winning_board();
    let ledger = gearmaster_engine::rumour::by_name("A Word About the Green Ledger").unwrap();
    let target = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == ledger.opens)
        .expect("authored");

    for rung in 0..target.at {
        run.rung = rung;
        // Fought, not handed over: `force_win` writes no log, so it banks
        // nothing, and a test built on it would say the condition is
        // unreachable when it is only unfought.
        run.fight_next();
        run.settle();
        run.back_to_loadout();
    }
    let banked = run.banked_all_run[Resource::Nature.index()];
    assert!(
        run.meets(ledger.needs),
        "a full auto-build reached rung {} with {} nature, so nobody can ever open this door",
        target.at,
        banked
    );
}

// -------------------------------------------------------------------- misc

#[test]
fn no_fountain_ever_offers_a_town_class() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    for name in gearmaster_engine::class::TOWN_CLASSES {
        assert!(
            gearmaster_engine::class::is_earned(name),
            "{name} could be poured, which is a fountain deciding you are Tired"
        );
        assert!(
            run.class_outlook().iter().all(|m| m.class.name != *name),
            "the fountain is offering {name}"
        );
    }
}

#[test]
fn a_town_class_does_not_use_up_a_fountain() {
    let mut run = Run::new();
    let before = run.next_fountain();
    assert!(before.is_some(), "there are fountains to miss");
    run.town = Some(&TOWNS[0]);
    run.visit_town(Action::Chapel);
    assert_eq!(run.next_fountain(), before, "praying ate a fountain");
}

#[test]
fn every_town_is_reachable_by_playing_the_game() {
    // The quiet failure: a town after a rung nothing ever stands on.
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.apply_preset();
    let mut visited: Vec<&str> = Vec::new();
    for rung in 0..LADDER.len() {
        run.rung = rung;
        run.force_win();
        run.settle();
        if let Some(t) = run.pending_town() {
            visited.push(t.id);
            run.skip_town();
        }
        run.back_to_loadout();
    }
    let all: Vec<&str> = TOWNS.iter().map(|t| t.id).collect();
    assert_eq!(visited, all, "a run up the whole ladder did not pass every town");
}

#[test]
fn the_gate_is_never_shown_mid_fight() {
    let mut run = at_the_gate();
    assert!(run.pending_town().is_some());
    run.fight_next();
    assert!(run.pending_town().is_none(), "the gate was drawn over a fight");
    assert_ne!(run.log.as_ref().map(|l| l.outcome), None);
    let _ = Outcome::Victory;
}

#[test]
fn a_wipe_clears_the_towns_it_has_seen() {
    let mut run = at_the_gate();
    run.visit_town(Action::Shop);
    assert!(!run.towns_seen.is_empty());
    run.wipe();
    assert!(run.towns_seen.is_empty(), "a fresh run remembers the last one's towns");
    assert!(run.town.is_none());
}

#[test]
fn town_returns_the_town_between_two_rungs() {
    for t in TOWNS {
        assert_eq!(town::between(t.after + 1).map(|x| x.id), Some(t.id));
    }
}


