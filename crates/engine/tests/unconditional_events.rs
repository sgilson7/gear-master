//! Five that always happen.
//!
//! No rumour, no chain, no flag, no fast kill. A run that touches nothing else
//! in this mission meets all five, which is what they are for: the road is
//! never bare, and the first of them is how a blind run learns the chain is in
//! the game at all.
//!
//! They are also where the mission's **gold rule** is easiest to check. Every
//! figure in this spec is a multiple of the standing rung's bounty rather than
//! a constant, because a constant means one thing at rung four and something
//! else entirely at rung forty: the body's own numbers were two and a half
//! times everything a run had ever seen at one end and one bounty at the
//! other.

mod common;

use gearmaster_engine::combat::{Difficulty, LADDER};
use gearmaster_engine::event::{Outcome, Requirement, EVENTS};
use gearmaster_engine::run::{Mode, Run};

const FIVE: [&str; 5] = [
    "back-in-a-minute",
    "the-teller",
    "the-dispenser",
    "what-the-table-said",
    "the-bird-problem",
];

fn a_run() -> Run {
    let mut run = Run::seeded(0xF1_5E);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

fn event(id: &str) -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("{} is not authored", id))
}

#[test]
fn all_five_stand_on_the_road_and_ask_for_nothing() {
    for id in FIVE {
        let e = event(id);
        assert!(
            matches!(e.trigger, gearmaster_engine::event::Trigger::Rung),
            "{} is conditional on something",
            id
        );
        assert!(e.blocked_by.is_empty(), "{} can be shut by something else", id);
        assert_eq!(LADDER[e.at].name, e.expects, "{} stands in front of the wrong fight", id);
    }
}

#[test]
fn a_run_that_only_ever_fights_still_meets_every_one_of_them() {
    let mut run = a_run();
    let mut met: Vec<&str> = Vec::new();
    for rung in 0..30usize {
        run.rung = rung;
        while let Some(e) = run.pending_event() {
            if FIVE.contains(&e.id) {
                met.push(e.id);
            }
            // Whatever needs nothing, which every one of them has.
            let c = e
                .choices
                .iter()
                .find(|c| c.requires == Requirement::None)
                .expect("a way through that costs nothing");
            if run.take_choice(c).is_none() && run.last_receipt.is_none() {
                break;
            }
            run.take_receipt();
            run.brawl = None;
            run.substitute = None;
        }
        run.back_to_loadout();
    }
    for id in FIVE {
        assert!(met.contains(&id), "{} was never met by a run that walked past it", id);
    }
}

#[test]
fn none_of_them_shares_a_rung_with_anything_else_scheduled() {
    // A rung with two doors on it is a rung where one of them is a surprise.
    for id in FIVE {
        let at = event(id).at;
        let here: Vec<&str> = EVENTS
            .iter()
            .filter(|e| matches!(e.trigger, gearmaster_engine::event::Trigger::Rung))
            .filter(|e| e.at == at)
            .map(|e| e.id)
            .collect();
        assert_eq!(here, vec![id], "rung {} is crowded", at + 1);
    }
}

// ------------------------------------------------------------- the money

#[test]
fn every_figure_in_them_is_a_multiple_of_the_rung_rather_than_a_number() {
    // The whole of RECONCILIATION II #16, said as a lint: nothing in this
    // mission's events prices anything in absolute gold.
    fn walk(o: &Outcome, out: &mut Vec<i32>) {
        match o {
            Outcome::Pay { times } => out.push(*times),
            Outcome::All(each) => each.iter().for_each(|x| walk(x, out)),
            Outcome::Gamble { won, lost, .. } => {
                walk(won, out);
                walk(lost, out);
            }
            _ => {}
        }
    }
    let mut any = false;
    for id in FIVE {
        for c in event(id).choices {
            let mut times = Vec::new();
            walk(&c.outcome, &mut times);
            for t in times {
                any = true;
                assert!(t > 0 && t <= 20, "{}: {} pays {} bounties", id, c.label, t);
            }
            if let Requirement::Purse { times } = c.requires {
                any = true;
                assert!(times > 0 && times <= 20, "{}: {} costs {} bounties", id, c.label, times);
            }
        }
    }
    assert!(any, "not one of the five deals in money, which is not what they were written as");
}

#[test]
fn a_price_is_paid_when_it_is_taken() {
    let mut run = a_run();
    run.rung = event("the-dispenser").at;
    let e = run.pending_event().expect("the machine");
    let coin = e.choices.iter().find(|c| c.label == "One coin").expect("authored");
    let Requirement::Purse { times } = coin.requires else { panic!("the coin is free") };
    let cost = run.rung_bounty() * times;

    run.gold = cost - 1;
    assert!(!run.choice_open(coin), "the slot took a coin that was not there");
    run.gold = cost + 500;
    let before = run.gold;
    run.take_choice(coin);
    assert_eq!(run.gold, before - cost, "the coin did not go in");
    let receipt = run.take_receipt().expect("a resolution");
    assert!(receipt[0].starts_with(&format!("-{}g", cost)), "{:?}", receipt);
}

#[test]
fn a_gamble_says_what_happened_and_never_the_odds() {
    // A machine at the roadside does not print its probabilities on the
    // front, and being told them afterwards turns a story into a spreadsheet.
    let mut run = a_run();
    run.rung = event("the-dispenser").at;
    run.gold = 10_000;
    let e = run.pending_event().expect("the machine");
    let shake = e.choices.iter().find(|c| c.label == "Shake it").expect("authored");
    run.take_choice(shake);
    let receipt = run.take_receipt().expect("a resolution");
    let all = receipt.join(" ");
    assert!(!all.contains('%'), "the receipt quoted odds: {:?}", receipt);
    assert!(!all.contains("Either"), "the receipt described what might have happened");
}

#[test]
fn the_same_seed_shakes_the_machine_the_same_way() {
    // E6.1, for the one thing in these five that rolls.
    let result = |seed: u64| {
        let mut run = Run::seeded(seed);
        common::build_full_loadout(&mut run);
        run.rung = event("the-dispenser").at;
        run.gold = 10_000;
        let e = run.pending_event().expect("the machine");
        let shake = e.choices.iter().find(|c| c.label == "Shake it").expect("authored");
        run.take_choice(shake);
        run.take_receipt()
    };
    assert_eq!(result(0xAB), result(0xAB));
}

// ------------------------------------------------------- the fresh spaces

#[test]
fn the_teller_is_the_only_thing_that_trades_maximum_health_away() {
    let mut run = a_run();
    run.rung = event("the-teller").at;
    let before = run.player_stats().health;
    let e = run.pending_event().expect("Songil");
    let whole = e.choices.iter().find(|c| c.label == "Hear it all").expect("authored");
    run.take_choice(whole);
    assert!(run.player_stats().health < before, "hearing it all cost nothing");
    assert!(run.holds("The Cracked Lens"), "and paid nothing");
    // And the Manse's long table is its mirror, which is what makes the trade
    // a trade rather than a tax.
    assert!(gearmaster_engine::run::LONG_TABLE_HEALTH > 0);
}

#[test]
fn plugging_your_ears_is_the_one_that_matters_later() {
    // G1 waits on it: because you kept your head whole you notice a second
    // sign behind the first. A `Took` pair, the way GERALD and AHEAD OF
    // SCHEDULE already are.
    let mut run = a_run();
    run.rung = event("the-teller").at;
    let e = run.pending_event().expect("Songil");
    let ears = e.choices.iter().find(|c| c.label == "Plug your ears").expect("authored");
    run.take_choice(ears);
    assert!(run.took.contains(&"Plug your ears"));
}

#[test]
fn the_table_is_the_only_content_that_touches_the_quest_system() {
    let mut run = a_run();
    run.rung = event("what-the-table-said").at;
    let loose = *run.inventory().first().expect("something loose");
    let had = run.quest_of(loose).is_some();
    let e = run.pending_event().expect("the inn");
    let set = e.choices.iter().find(|c| c.label == "Set a piece on it").expect("authored");
    run.take_choice(set);
    assert!(run.quest_of(loose).is_some(), "the table said nothing");
    if !had {
        let q = run.quest_of(loose).expect("spoken");
        assert!(gearmaster_engine::piece::CATALOG.iter().any(|d| d.name == q.becomes));
    }
}

#[test]
fn ignoring_the_memo_changes_the_shape_of_the_next_fight() {
    // The only event that does. Everything else stands in front of a rung and
    // hands it back unchanged.
    let mut run = a_run();
    run.rung = event("the-bird-problem").at;
    let e = run.pending_event().expect("the courier");
    let ignore = e.choices.iter().find(|c| c.label == "Ignore the memo").expect("authored");
    run.take_choice(ignore);
    let party = run.pending_brawl().expect("company");
    assert!(party.iter().any(|m| m.name == "THE FLOCK"));
    // And losing to birds costs nothing, because a memo is not a rung.
    assert!(
        matches!(ignore.outcome, Outcome::Step(b) if b.forgiving),
        "the birds cost a life"
    );
}

#[test]
fn each_of_the_five_opens_a_space_nothing_else_uses() {
    // The reason there are five rather than one: a parcel with a pointer in
    // it, a trade in maximum health, a machine that gambles what you do not
    // own, a table that speaks to a quest, and a memo that changes the next
    // fight. No two of them are the same kind of thing.
    let kinds: Vec<&str> = FIVE
        .iter()
        .map(|id| {
            let e = event(id);
            if e.choices.iter().any(|c| matches!(c.outcome, Outcome::Gamble { .. })) {
                "gamble"
            } else if e.choices.iter().any(|c| matches!(c.outcome, Outcome::GrantQuest(_))) {
                "quest"
            } else if e.choices.iter().any(|c| matches!(c.outcome, Outcome::Step(_))) {
                "fight"
            } else if e
                .choices
                .iter()
                .any(|c| matches!(c.outcome, Outcome::All(x) if x.iter().any(|o| matches!(o, Outcome::Health(_)))))
            {
                "health"
            } else {
                "gift"
            }
        })
        .collect();
    let mut sorted = kinds.clone();
    sorted.sort_unstable();
    let n = sorted.len();
    sorted.dedup();
    assert_eq!(sorted.len(), n, "two of the five are the same kind of thing: {:?}", kinds);
}
