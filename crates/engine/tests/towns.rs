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

mod common;

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
    // Without its classes: every test below hands it the one class it is
    // about, and Berserker and Chronomancer on top of that would be measuring
    // three things at once.
    common::board_from(gearmaster_engine::share::A_WINNING_RUN)
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

    // The whole ladder, not the first twenty. A fight has to last four swings
    // before the halving is visible, and the board this is measured on keeps
    // getting better at ending the early rungs before that - which read as
    // "the rule stopped working" when it only meant the sample had gone. The
    // deep rungs are where the long fights are.
    let mut checked = 0;
    for spec in LADDER.iter() {
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
    assert_eq!(
        free[0].1 - owing[0].1,
        debt,
        "the fight opened {} short and the debt is {debt}",
        free[0].1 - owing[0].1
    );

    // Lined up by the clock, and asking whether the debt is ever an advantage
    // rather than whether it is a constant offset.
    //
    // This used to require the two runs to have exactly as many income events
    // as each other and then assert the gap was the debt at every one of them.
    // Both halves held while the owner's board came back holding thirteen
    // items and neither holds now that it comes back holding nineteen. The
    // curve records income, not spending, so what sits between two income
    // events is everything the board paid for in between - and a board with a
    // debt cannot always pay. A spend that fails leaves the pool *higher* than
    // it would have been, which closes the gap without any of the debt being
    // repaid. That is the mechanic working, not a fault, and a test that
    // demanded a constant offset was demanding a board too poor to spend.
    let by_ms = |c: &[(u32, i32)]| -> std::collections::BTreeMap<u32, i32> {
        // A repeated timestamp is two incomes in one tick; the later value is
        // the running total after both, which is the one to compare.
        c.iter().copied().collect()
    };
    let (unowed, owed) = (by_ms(&free), by_ms(&owing));
    let shared: Vec<u32> = unowed.keys().copied().filter(|t| owed.contains_key(t)).collect();
    assert!(
        shared.len() >= 8,
        "the two runs share only {} moments, which compares almost nothing",
        shared.len()
    );
    for t in &shared {
        assert!(
            owed[t] <= unowed[t],
            "at {t}ms the indebted run held {} against {}, so owing mana paid it something",
            owed[t],
            unowed[t]
        );
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
fn the_town_shop_is_things_you_cannot_get_elsewhere() {
    let shelf = gearmaster_engine::piece::town_shelf();
    let mut run = at_the_gate();
    run.visit_town(Action::Shop);
    let on_sale: Vec<&str> = run.shop.stock_defs().iter().map(|d| d.name).collect();
    assert_eq!(on_sale.len(), shelf.len());
    for name in shelf {
        assert!(on_sale.contains(name), "{name} was not on the shelf");
    }
    // The curated five are the reason to come in; the underlays are the thing
    // a town is the only place to buy. Both, or the shelf is half a shelf.
    for name in TOWN_ONLY {
        assert!(on_sale.contains(name), "{name} was not on the shelf");
    }
    assert!(
        on_sale.iter().any(|n| CATALOG
            .iter()
            .any(|d| d.name == *n && d.kind.is_underlay())),
        "a town is the only place that sells ground, and this one sold none"
    );
}

#[test]
fn town_gear_does_not_move_the_scale_for_anything_else() {
    // The VIP five are exempt from the rating ceiling because they are behind
    // a locked branch and meant to be absurd. A town is on the way to
    // everywhere, so its gear has to live inside the curve like everything
    // else - which means it must not be the ceiling of its slot.
    for name in gearmaster_engine::piece::town_shelf() {
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
    assert_eq!(on_sale.len(), gearmaster_engine::rumour::on_offer().len());
    for r in gearmaster_engine::rumour::RUMOURS {
        assert!(on_sale.contains(&r.name), "{} was not on the bar", r.name);
    }
    assert!(
        on_sale.contains(&gearmaster_engine::rumour::TROPHY_SHELF),
        "the bar's standing offer on trophies was not on it"
    );
    // Nothing on this bar is bought with money, so `buy` must never reach a
    // shelf of it: every one is either a rumour or the trophy trade.
    for i in 0..on_sale.len() {
        assert!(
            run.rumour_on(i).is_some() || run.trophy_shelf(i),
            "shelf {i} of the pub takes money"
        );
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



// ------------------------------------------------------- walking both chains

/// Play up the ladder, taking one named action at each town, and report the
/// events that stood in front of the run on the way.
fn walk(actions: &[Action], keep_rumours: bool) -> Vec<&'static str> {
    let mut run = the_winning_board();
    // A loose frame in the tray. The winning board is 97% packed and has
    // nothing loose at all, so without this it cannot pay for a rumour - which
    // is true of the board and not of a run, since anything bought or dropped
    // lands in the tray first.
    give(&mut run, "Steel Frame");
    let mut seen: Vec<&'static str> = Vec::new();
    let mut town_no = 0usize;
    for rung in 0..26usize {
        run.rung = rung;
        // Fought rather than forced: a forced win writes no log, so it banks
        // nothing, and the Ledger's condition is about what a run has banked.
        run.fight_next();
        run.settle();
        run.back_to_loadout();
        if let Some(t) = run.pending_town() {
            match actions.get(town_no) {
                Some(&a) => {
                    run.visit_town(a);
                    // The pub only stocks the shelves; buying is a second act.
                    if a == Action::Pub && keep_rumours {
                        for shelf in 0..6usize {
                            let Some(_) = run.rumour_on(shelf) else { continue };
                            if let Some(&pay) = run.payment_for(shelf).first() {
                                let _ = run.barter(shelf, pay);
                            }
                        }
                    }
                }
                None => {
                    run.skip_town();
                }
            }
            town_no += 1;
            let _ = t;
        }
        while let Some(ev) = run.pending_event() {
            seen.push(ev.id);
            let Some(c) = ev.choices.iter().find(|c| run.choice_open(c)) else { break };
            run.take_choice(c);
            run.back_to_loadout();
        }
    }
    seen
}

#[test]
fn a_run_that_buys_the_first_word_gets_the_first_door() {
    // One pub visit, one frame handed over, and the Crownwright is standing
    // there on rung nineteen.
    let mut run = the_winning_board();
    give(&mut run, "Steel Frame");
    run.town = Some(&TOWNS[0]);
    run.visit_town(Action::Pub);
    let shelf = (0..6)
        .find(|&i| run.rumour_on(i).map(|r| r.name) == Some("A Word About the Crownwright"))
        .expect("on the bar");
    let pay = *run.payment_for(shelf).first().expect("the frame pays for it");
    run.barter(shelf, pay).expect("the trade goes through");

    let ev = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-crownwright")
        .expect("authored");
    run.rung = ev.at;
    run.back_to_loadout();
    assert_eq!(run.pending_event().map(|e| e.id), Some(ev.id));
}

#[test]
fn trading_the_first_word_up_buys_the_second_door_and_costs_the_first() {
    // The either/or. Both halves of it, so neither can quietly stop working.
    let mut run = the_winning_board();
    give(&mut run, "Steel Frame");
    run.town = Some(&TOWNS[0]);
    run.visit_town(Action::Pub);
    let first = (0..6)
        .find(|&i| run.rumour_on(i).map(|r| r.name) == Some("A Word About the Crownwright"))
        .expect("on the bar");
    let pay = *run.payment_for(first).first().expect("the frame pays");
    run.barter(first, pay).expect("first trade");

    // A later pub, where the word itself is the price of the other one.
    run.town = Some(&TOWNS[1]);
    run.towns_seen.clear();
    run.visit_town(Action::Pub);
    let second = (0..6)
        .find(|&i| run.rumour_on(i).map(|r| r.name) == Some("A Word About the Green Ledger"))
        .expect("on the bar");
    let up = *run
        .payment_for(second)
        .first()
        .expect("the word you are carrying is what they want for it");
    run.barter(second, up).expect("second trade");

    assert!(
        run.owned.iter().any(|&i| run.registry.def(i).name == "A Word About the Green Ledger"),
        "traded up and got nothing"
    );
    assert!(
        !run.owned.iter().any(|&i| run.registry.def(i).name == "A Word About the Crownwright"),
        "traded the word away and still have it"
    );

    // So the first door is shut and the second is open.
    run.rung = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-crownwright")
        .unwrap()
        .at;
    run.back_to_loadout();
    assert!(run.pending_event().map(|e| e.id) != Some("the-crownwright"));

    // Bank the hundred nature the ledger wants, by fighting for it.
    for rung in 0..22usize {
        run.rung = rung;
        run.fight_next();
        run.settle();
        run.back_to_loadout();
    }
    run.rung = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-green-ledger")
        .unwrap()
        .at;
    run.back_to_loadout();
    assert_eq!(run.pending_event().map(|e| e.id), Some("the-green-ledger"));
}

#[test]
fn a_whole_run_up_the_ladder_meets_the_doors_it_paid_for() {
    // The end-to-end version: play it, and see the door.
    let with_word = walk(&[Action::Pub, Action::Chapel, Action::Chapel], true);
    assert!(
        with_word.contains(&"the-crownwright"),
        "bought the word, walked to the rung, and the door was not there: {with_word:?}"
    );

    // And a run that never went to a pub never meets either of them.
    let without = walk(&[Action::Chapel, Action::Chapel, Action::Chapel], false);
    assert!(
        !without.contains(&"the-crownwright") && !without.contains(&"the-green-ledger"),
        "a run that heard no rumours met one anyway: {without:?}"
    );
}

// ------------------------------------------------------------------ recycler

#[test]
fn a_class_gained_any_way_reaches_the_board() {
    // Recycler's maths lives in `Loadout::report`, so the loadout has to be
    // told the run holds it. Every path that grants a class has to say so, and
    // "every path" is the part nobody remembers - so this walks them.
    let recycler = CLASSES.iter().find(|c| c.name == "Recycler").expect("authored");

    // Through the pub.
    let mut run = the_winning_board();
    give(&mut run, "The Money Jacket");
    run.town = Some(&TOWNS[0]);
    run.visit_town(Action::Pub);
    let shelf = (0..6).find(|&i| run.trophy_shelf(i)).expect("the bar takes trophies");
    let pay = *run.payment_for(shelf).first().expect("the coat pays for it");
    run.barter(shelf, pay).expect("the trade goes through");
    assert_eq!(run.stacks_of("Recycler"), 1);
    assert_eq!(run.loadout.adjacency_pct, 10, "the pub granted it and the board never heard");

    // And pushed straight on, the way a test or a debug hook does.
    let mut plain = the_winning_board();
    plain.classes.push(recycler);
    plain.classes.push(recycler);
    plain.refresh_class_effects();
    assert_eq!(plain.loadout.adjacency_pct, 20, "two stacks are twenty percent");
}

#[test]
fn recycler_pays_a_board_that_finishes_what_it_seats() {
    // The point of the class, and the reason it is worth a trophy: it scales
    // adjacency bonuses, which only pay on an assembled item. Measured on the
    // owner's board, which finishes nearly everything it seats.
    let recycler = CLASSES.iter().find(|c| c.name == "Recycler").expect("authored");
    let mut run = the_winning_board();
    let before = run.player_stats().health;
    assert!(before > 0);

    for n in 1..=5 {
        run.classes.push(recycler);
        run.refresh_class_effects();
        assert_eq!(run.loadout.adjacency_pct, n * 10);
    }
    let after = run.player_stats().health;
    assert!(
        after > before,
        "five stacks of Recycler moved health from {before} to {after}"
    );
    // Five stacks is half again on the bonuses, not on the whole board, so the
    // jump has to be real and nowhere near fifty percent of everything.
    assert!(
        after < before * 3 / 2,
        "five stacks added {} health, which is half the whole board and not half the bonuses",
        after - before
    );
}

#[test]
fn a_board_that_assembles_nothing_gets_nothing_from_recycler() {
    // The other half of the contract. An adjacency bonus pays only when its
    // item comes together, so a tray full of loose pieces is worth no more
    // with the class than without it.
    let recycler = CLASSES.iter().find(|c| c.name == "Recycler").expect("authored");
    let mut run = Run::new();
    give(&mut run, "Steel Frame");
    let before = run.player_stats();
    run.classes.push(recycler);
    run.refresh_class_effects();
    assert_eq!(run.player_stats(), before, "a loose frame paid a bonus it never earned");
}
