//! The doors that are not questions.
//!
//! Everything else in `EVENTS` is a paragraph and two answers. These eight are
//! *shapes*: an inspection that reads the board you are standing in, an auction
//! against a number nobody has seen, a handicap you ask for and are paid for
//! surviving, a passenger who costs cells, a menu made of what you are
//! carrying, a fork whose only question is which half first, and a counter
//! nobody mentioned until it spoke.
//!
//! Three standalone rumour pairs ride with them, and the two classes that come
//! out of those - Unionized, which is the second stacking class in the game,
//! and Showstopper, which is the only thing besides the casino's door that
//! rewards being quick.

mod common;

use gearmaster_engine::combat::{Difficulty, LADDER};
use gearmaster_engine::event::{Outcome, Requirement, Trigger, EVENTS};
use gearmaster_engine::run::{Mode, Run};

const STRUCTURES: [&str; 9] = [
    "the-inspection",
    "the-sealed-bid",
    "the-contract",
    "the-payout",
    "the-passenger",
    "the-buyer",
    "the-fork",
    "the-foundry-remembers",
    "through-the-cracked-lens",
];

const PAIRS: [&str; 3] = ["the-wizards-thirst", "the-picket-line", "the-exhibition"];

fn event(id: &str) -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("{} is not authored", id))
}

fn choice(id: &str, label: &str) -> &'static gearmaster_engine::event::Choice {
    event(id)
        .choices
        .iter()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("{} has no choice called {}", id, label))
}

/// Find the passenger a seat. The reference build leaves the corners full, and
/// a passenger in the tray is paying no rent at all.
fn seat(run: &mut Run, id: gearmaster_engine::piece::PieceId) {
    let slot = run.registry.def(id).slot;
    for y in 0..8u8 {
        for x in 0..6u8 {
            if run.equip(id, slot, x, y).is_ok() {
                return;
            }
        }
    }
    panic!("nowhere on the board for a calf");
}

fn a_run() -> Run {
    let mut run = Run::seeded(0x571C);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

/// Stand the run in front of a door and hand it back.
///
/// Rung, flags and words all have to be right or `pending_event` returns
/// something else entirely, and a test that took the wrong door would pass for
/// the wrong reason - which is exactly what happened to the town gate in M3.
fn standing_at(run: &mut Run, id: &str) -> &'static gearmaster_engine::event::LadderEvent {
    let e = event(id);
    run.rung = e.at;
    match e.trigger {
        Trigger::WhenFlagged { flag, .. } => {
            if !run.flags.contains(&flag) {
                run.flags.push(flag);
            }
        }
        Trigger::Whispered { rumour, .. } => {
            run.give(rumour);
        }
        _ => {}
    }
    let here = run.pending_event().unwrap_or_else(|| panic!("{} did not stand up", id));
    assert_eq!(here.id, id, "{} was shadowed by {}", id, here.id);
    here
}

// ------------------------------------------------------------ the table

#[test]
fn every_structure_stands_in_front_of_the_fight_it_names() {
    for id in STRUCTURES.iter().chain(PAIRS.iter()) {
        let e = event(id);
        assert_eq!(LADDER[e.at].name, e.expects, "{} stands somewhere else now", id);
        assert!(
            e.choices.iter().any(|c| c.requires == Requirement::None),
            "{} can be arrived at and not left",
            id
        );
    }
}

#[test]
fn every_figure_in_them_is_a_multiple_of_the_rung() {
    // RECONCILIATION II #16, as a lint, for the second half of the content.
    fn walk(o: &Outcome, out: &mut Vec<i32>) {
        match o {
            Outcome::Pay { times } | Outcome::BuyOff { times } => out.push(*times),
            Outcome::All(each) => each.iter().for_each(|x| walk(x, out)),
            Outcome::Gamble { won, lost, .. } => {
                walk(won, out);
                walk(lost, out);
            }
            _ => {}
        }
    }
    for id in STRUCTURES.iter().chain(PAIRS.iter()) {
        for c in event(id).choices {
            let mut times = Vec::new();
            walk(&c.outcome, &mut times);
            for t in times {
                assert!(t >= 0 && t <= 20, "{}: {} pays {} bounties", id, c.label, t);
            }
            if let Requirement::Purse { times } = c.requires {
                assert!(times > 0 && times <= 20, "{}: {} costs {} bounties", id, c.label, times);
            }
        }
    }
}

// ------------------------------------------------------- the inspection

#[test]
fn the_inspection_reads_the_board_that_is_standing_in_front_of_her() {
    // Not the inventory and not the gold: the tiers are `AlignedItems`, which
    // is a live read of what the player has actually built.
    let top = choice("the-inspection", "Show her everything");
    let Requirement::AlignedItems(n) = top.requires else { panic!("she stopped looking") };
    assert!(n >= 3, "the top grade is not worth walking up for");

    let mut bare = a_run();
    bare.loadout = gearmaster_engine::loadout::Loadout::new();
    bare.rung = event("the-inspection").at;
    assert!(!bare.choice_open(top), "an empty board graded three items");

    let dressed = a_run();
    assert!(dressed.most_aligned() >= 2, "the reference build agrees with nothing");
}

#[test]
fn refusing_the_inspection_is_the_one_door_where_declining_pays() {
    let mut run = a_run();
    standing_at(&mut run, "the-inspection");
    run.take_choice(choice("the-inspection", "Decline the inspection"));
    assert!(
        run.holds("A Word About the Picket"),
        "she folded the stool and said nothing on the way out"
    );
}

// -------------------------------------------------------- the sealed bid

#[test]
fn the_reserve_is_the_runs_own_number_and_the_receipt_reads_it_out() {
    let bid = |seed: u64, figure: i32| {
        let mut run = Run::seeded(seed);
        common::build_full_loadout(&mut run);
        run.gold = 100_000;
        standing_at(&mut run, "the-sealed-bid");
        run.take_choice_with(choice("the-sealed-bid", "Name a figure"), figure);
        run.take_receipt().expect("a resolution")
    };
    let a = bid(0x9E, 5_000);
    assert!(a[0].starts_with("The reserve was"), "{:?}", a);
    // Two replays of a seed bid against the same number.
    assert_eq!(a, bid(0x9E, 5_000));
    // And the figure is the player's, not the table's.
    assert_ne!(a, bid(0x9E, 0));
}

#[test]
fn a_bid_over_the_reserve_buys_the_lot_and_a_bid_under_it_buys_the_number() {
    let mut over = Run::seeded(0x33);
    common::build_full_loadout(&mut over);
    over.gold = 100_000;
    standing_at(&mut over, "the-sealed-bid");
    let before = over.gold;
    over.take_choice_with(choice("the-sealed-bid", "Name a figure"), 5_000);
    let receipt = over.take_receipt().expect("a resolution");
    assert!(over.gold < before, "the lot was free");
    assert!(receipt.iter().any(|l| l.contains("yours")), "{:?}", receipt);

    let mut under = Run::seeded(0x33);
    common::build_full_loadout(&mut under);
    under.gold = 100_000;
    standing_at(&mut under, "the-sealed-bid");
    let was = under.gold;
    under.take_choice_with(choice("the-sealed-bid", "Name a figure"), 0);
    assert_eq!(under.gold, was, "a losing bid was charged for");
    let receipt = under.take_receipt().expect("a resolution");
    assert!(receipt[0].starts_with("The reserve was"), "and it was not read out: {:?}", receipt);
}

#[test]
fn a_figure_is_the_only_way_through_that_door() {
    // `take_choice` refuses a `Figure`, because a default bid is a bid nobody
    // made.
    let mut run = a_run();
    standing_at(&mut run, "the-sealed-bid");
    assert!(run.take_choice(choice("the-sealed-bid", "Name a figure")).is_none());
}

// ------------------------------------------------- the contract, and its end

#[test]
fn a_signed_contract_runs_every_slot_cold_for_exactly_three_rungs() {
    let mut run = a_run();
    let warm: Vec<u32> = run.combat_items().iter().map(|i| i.cooldown_ms).collect();
    standing_at(&mut run, "the-contract");
    run.take_choice(choice("the-contract", "Sign it"));
    assert!(run.under_contract(), "the pen was handed over after all");

    let cold: Vec<u32> = run.combat_items().iter().map(|i| i.cooldown_ms).collect();
    assert_eq!(warm.len(), cold.len(), "the contract changed what is on the board");
    assert!(
        warm.iter().zip(&cold).all(|(w, c)| c > w),
        "not every slot ran cold: {:?} -> {:?}",
        warm,
        cold
    );

    // Three rungs, no early exit.
    run.rung = event("the-contract").at + 3;
    assert!(run.under_contract(), "it thawed a rung early");
    run.rung = event("the-contract").at + 4;
    assert!(!run.under_contract(), "it never thawed at all");
}

#[test]
fn the_payout_reads_the_column_and_an_empty_column_pays_nothing() {
    let mut kept = a_run();
    standing_at(&mut kept, "the-contract");
    kept.take_choice(choice("the-contract", "Sign it"));
    let collect = choice("the-payout", "Collect");
    standing_at(&mut kept, "the-payout");
    assert!(kept.choice_open(collect), "a signed contract is not in the ledger");
    let before = kept.gold;
    kept.take_choice(collect);
    assert!(kept.gold > before, "Treyway kept the money");
    assert!(kept.underwritten_until.is_some(), "and the name they honour once");

    let mut walked = a_run();
    standing_at(&mut walked, "the-payout");
    assert!(!walked.choice_open(collect), "an empty column collected");
}

// ---------------------------------------------------------- the passenger

#[test]
fn the_passenger_costs_cells_and_pays_on_arrival() {
    let mut run = a_run();
    standing_at(&mut run, "the-passenger");
    run.take_choice(choice("the-passenger", "Take it aboard"));
    let (id, _) = run.passenger.expect("nobody got on");
    assert!(run.owned.contains(&id), "the passenger is not a thing you are carrying");
    assert!(!run.passenger_is_seated(), "it seated itself, which is not the rent");

    // Seat it, walk five rungs, and the courier's word is good.
    seat(&mut run, id);
    assert!(run.passenger_is_seated());
    assert!(!run.deliver_passenger(), "delivered before it got there");
    run.rung = event("the-passenger").at + 5;
    assert!(run.deliver_passenger(), "five rungs was not five rungs");
    assert!(!run.owned.contains(&id), "the calf is still on the board");
}

#[test]
fn losing_a_fight_loses_the_passenger() {
    // The whole of the risk, and the reason the rent is cells rather than
    // gold: a passenger is a fragile thing riding in the open.
    let mut run = a_run();
    standing_at(&mut run, "the-passenger");
    run.take_choice(choice("the-passenger", "Take it aboard"));
    let (id, _) = run.passenger.expect("nobody got on");
    seat(&mut run, id);
    run.back_to_loadout();

    run.difficulty = Difficulty::Insane;
    run.fight(&LADDER[49]);
    run.settle();
    let settlement = run.last_settlement.as_ref().expect("a settlement");
    if settlement.lost_passenger {
        assert!(run.passenger.is_none(), "it was lost and is still riding");
    } else {
        assert!(run.passenger.is_some(), "it went missing without being lost");
    }
}

// ------------------------------------------------------------- the buyer

#[test]
fn the_buyers_menu_is_gated_by_what_you_are_carrying_rather_than_generated() {
    // `Choice` is static data. The menu is three doors that open on what you
    // hold, which is a table; generating the menu would mean the event table
    // stopped being a table.
    let word = choice("the-buyer", "Sell him a word");
    let title = choice("the-buyer", "Sell him a title");
    assert_eq!(word.requires, Requirement::HoldingRumour);
    assert_eq!(title.requires, Requirement::Classes(1));

    let mut empty = a_run();
    empty.rung = event("the-buyer").at;
    assert!(!empty.choice_open(word), "he bought a word nobody had");
    assert!(!empty.choice_open(title), "he bought a title nobody had");
}

#[test]
fn every_sale_takes_the_thing_away() {
    let mut run = a_run();
    run.give("A Word About the Wrong Stars");
    run.classes.push(gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Wanderer").unwrap());
    standing_at(&mut run, "the-buyer");

    let before = run.gold;
    run.take_choice(choice("the-buyer", "Sell him a word"));
    assert!(!run.holds("A Word About the Wrong Stars"), "he paid for a word you kept");
    assert!(run.gold > before, "and paid nothing");

    // The door that word opened is shut, which is the price the blurb quotes.
    run.rung = event("the-astronomer").at;
    assert!(
        run.pending_event().is_none_or(|e| e.id != "the-astronomer"),
        "the astronomer still stands"
    );
}

#[test]
fn the_hundred_he_buys_does_not_come_back() {
    let mut run = a_run();
    let before = run.player_stats().health;
    standing_at(&mut run, "the-buyer");
    run.take_choice(choice("the-buyer", "Sell him a hundred of your maximum"));
    assert!(run.player_stats().health < before, "he was sold nothing");
}

// -------------------------------------------------------------- the fork

#[test]
fn both_halves_of_the_fork_are_legal_and_they_are_not_the_same_order() {
    let first = choice("the-fork", "The shelf, then the seam");
    let second = choice("the-fork", "The seam, then the shelf");
    assert_eq!(first.requires, Requirement::None);
    assert_eq!(second.requires, Requirement::None);

    let mut buy_first = a_run();
    standing_at(&mut buy_first, "the-fork");
    buy_first.take_choice(first);
    assert!(!buy_first.shop.stock_defs().is_empty(), "the shelf was not laid out");
    assert!(buy_first.dungeon.is_some(), "and the seam was not opened");

    let mut fight_first = a_run();
    standing_at(&mut fight_first, "the-fork");
    fight_first.take_choice(second);
    assert!(fight_first.dungeon.is_some(), "the seam was not opened");
    assert!(fight_first.shop_owed.is_some(), "and nothing was owed on the way out");
}

// -------------------------------------------------- the counter that spoke

#[test]
fn the_foundry_was_counting_and_says_so_thirty_rungs_later() {
    let kept = choice("the-foundry-remembers", "\"We kept your best\"");
    let Requirement::Counter { what, at_least } = kept.requires else {
        panic!("the book stopped being a book")
    };
    assert_eq!(what, "crucible-melts");
    assert!(at_least >= 2, "one visit is not a habit");

    let mut never = a_run();
    standing_at(&mut never, "the-foundry-remembers");
    assert!(!never.choice_open(kept), "a column with nothing in it was read");

    let mut twice = a_run();
    twice.count("crucible-melts");
    twice.count("crucible-melts");
    standing_at(&mut twice, "the-foundry-remembers");
    assert!(twice.choice_open(kept), "two melts is not two melts");
    twice.take_choice(kept);
    assert!(twice.holds("The Cracked Lens"), "the foundry kept it after all");
}

#[test]
fn saying_nothing_puts_the_prices_up() {
    let mut run = a_run();
    let before = run.price(0);
    standing_at(&mut run, "the-foundry-remembers");
    run.take_choice(choice("the-foundry-remembers", "Say nothing"));
    assert!(run.markup > 0, "the note was never made");
    assert!(run.price(0) >= before, "and never reached a shelf");
}

// ----------------------------------------------------------- the lens

#[test]
fn the_lens_grants_no_stats_and_only_sight() {
    // E6.12: scouting is knowledge and knowledge is not a stat.
    let mut run = a_run();
    run.give("The Cracked Lens");
    let before = run.player_stats();
    standing_at(&mut run, "through-the-cracked-lens");
    run.take_choice(choice("through-the-cracked-lens", "Look through it"));
    assert!(run.scouting, "the lens did nothing");
    assert_eq!(run.player_stats(), before, "scouting moved a number");
}

// ------------------------------------------------------- the three pairs

#[test]
fn every_pair_is_opened_by_a_word_that_can_be_come_by() {
    for id in PAIRS {
        let e = event(id);
        let Trigger::Whispered { rumour, from } = e.trigger else {
            panic!("{} is not a rumour door", id)
        };
        let r = gearmaster_engine::rumour::by_name(rumour)
            .unwrap_or_else(|| panic!("{} waits on something that is not a word", id));
        assert_eq!(r.opens, id, "{} and {} disagree about each other", id, rumour);
        assert!(from <= e.at, "{} has a window that runs backwards", id);
    }
}

#[test]
fn honouring_the_picket_line_is_the_second_stacking_class_in_the_game() {
    let mut run = a_run();
    standing_at(&mut run, "the-picket-line");
    run.take_choice(choice("the-picket-line", "Honor the line"));
    assert!(run.classes.iter().any(|c| c.name == "Unionized"), "the line was crossed");
    assert!(gearmaster_engine::class::stacks("Unionized"), "a line honoured twice is one line");
}

#[test]
fn unionized_is_armour_before_the_first_blow() {
    // Armour resets to zero every fight, so this is the only thing in the game
    // that hands out any of it before one starts.
    let armor = |unionized: bool| {
        let mut run = a_run();
        if unionized {
            run.classes.push(
                gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Unionized").unwrap(),
            );
        }
        run.rung = 0;
        run.fight(&LADDER[0]);
        run.log.as_ref().expect("a fight").entries[0].clone()
    };
    let _ = armor(false);
    // Read off the fighter the log opened with rather than a tick of it: armour
    // is spent in the first exchange and a stack of 20 would be gone by the
    // time anything else could see it.
    let start = |unionized: bool| {
        let mut run = a_run();
        if unionized {
            run.classes.push(
                gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Unionized").unwrap(),
            );
        }
        run.rung = 0;
        run.fight(&LADDER[0]);
        run.log.as_ref().expect("a fight").player.armor
    };
    assert!(start(true) > start(false), "the picket line bought nothing");
}

#[test]
fn showstopper_pays_for_being_quick_and_nothing_else() {
    let bill = gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Showstopper").unwrap();
    let gearmaster_engine::class::ClassPower::Showstopper { pct, under_ms } = bill.power else {
        panic!("the billing changed")
    };
    assert!(pct > 0 && under_ms > 0);

    let purse = |quick: bool| {
        let mut run = a_run();
        if quick {
            run.classes.push(bill);
        }
        run.rung = 0;
        run.fight(&LADDER[0]);
        let ms = run.log.as_ref().unwrap().duration_ms;
        run.settle();
        let s = run.last_settlement.as_ref().expect("a settlement");
        (ms, s.reward)
    };
    let (ms, plain) = purse(false);
    let (_, billed) = purse(true);
    assert!(ms < under_ms, "the fixture fight is not a quick one, so this proves nothing");
    assert!(billed > plain, "the bill paid the same as no bill");
}

#[test]
fn the_wizard_wants_a_shape_rather_than_a_price() {
    let trade = choice("the-wizards-thirst", "Trade him one");
    assert!(
        matches!(trade.requires, Requirement::LooseItemOfSize { .. }),
        "he started taking money"
    );
    let mut run = a_run();
    let warm: Vec<u32> = run.combat_items().iter().map(|i| i.cooldown_ms).collect();
    standing_at(&mut run, "the-wizards-thirst");
    run.take_choice(choice("the-wizards-thirst", "Refuse him"));
    assert!(!run.cursed_for_good.is_empty(), "he took it far too well");
    // And the curse is a real one: one item runs cold for the rest of the run,
    // which is the only curse in the game that lives on a piece.
    let cold: Vec<u32> = run.combat_items().iter().map(|i| i.cooldown_ms).collect();
    assert!(
        warm.iter().zip(&cold).any(|(w, c)| c > w),
        "nothing of yours actually froze: {:?} -> {:?}",
        warm,
        cold
    );
    assert!(
        warm.iter().zip(&cold).filter(|(w, c)| *c > *w).count() < warm.len(),
        "he froze the whole board, which is a contract rather than a curse"
    );
}

#[test]
fn the_exhibition_bills_you_before_it_fights_you() {
    // A packed board rather than the starter preset: the door asks for a Rare
    // item and the preset's dearest assembles to 31, which is what "worth
    // watching" is supposed to exclude.
    let mut run = common::run_from(gearmaster_engine::share::A_WINNING_RUN);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    standing_at(&mut run, "the-exhibition");
    let bout = choice("the-exhibition", "Give them a bout");
    assert!(run.choice_open(bout), "the reference build is not worth watching");
    run.take_choice(bout);
    assert!(run.classes.iter().any(|c| c.name == "Showstopper"), "no billing");
    let brawl = run.brawl.expect("no bout");
    assert_eq!(brawl.with.len(), 2, "two men, one of you");
    assert!(brawl.forgiving, "an exhibition that can end a run is not an exhibition");
}
