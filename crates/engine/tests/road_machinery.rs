//! The things the road can ask, and the things it can hand over.
//!
//! Every mechanic in this file ships **dark**: no event in the game names any
//! of them yet, and the road is byte-identical to the one before them. That is
//! the phase discipline, and it is also the only way a mechanic this wide
//! lands without moving the balance underneath it - the last mission's lesson
//! was that arming a slot arms the monsters first, and the same is true of
//! arming a road.
//!
//! So the tests below build their own doors out of the machinery rather than
//! reaching for a shipped one. Where a shipped event *is* used it is because
//! the assertion is about the road rather than about the outcome.

mod common;

use gearmaster_engine::event::{Choice, Outcome, Requirement, Standing};
use gearmaster_engine::piece::{PieceKind, SlotKind};
use gearmaster_engine::rating::Rarity;
use gearmaster_engine::run::{Run, MELT_SPREAD, UNDERWRITTEN_FOR};

fn a_run() -> Run {
    let mut run = Run::seeded(0x4004);
    common::build_full_loadout(&mut run);
    run
}

/// A door with no requirement and one outcome, applied straight.
fn door(outcome: Outcome) -> Choice {
    Choice { label: "a door", blurb: "", requires: Requirement::None, outcome, unmet: "" }
}

fn apply(run: &mut Run, outcome: Outcome) -> Vec<String> {
    let c = door(outcome);
    run.apply_outcome(&c.outcome, c.requires).1
}

// ------------------------------------------------------------- conditions

#[test]
fn a_flag_is_set_by_one_door_and_read_by_another() {
    let mut run = a_run();
    let asks = door(Outcome::FightAsWritten);
    let gated = Choice { requires: Requirement::Flag("heard-the-astronomer"), ..asks };
    assert!(!run.choice_open(&gated));
    apply(&mut run, Outcome::Flag("heard-the-astronomer"));
    assert!(run.choice_open(&gated));
    // Twice is not two flags.
    apply(&mut run, Outcome::Flag("heard-the-astronomer"));
    assert_eq!(run.flags.len(), 1);
}

#[test]
fn a_counter_counts_without_saying_so() {
    let mut run = a_run();
    let gated = Choice {
        requires: Requirement::Counter { what: "crucible-melts", at_least: 2 },
        ..door(Outcome::FightAsWritten)
    };
    assert!(!run.choice_open(&gated));
    let receipt = apply(&mut run, Outcome::Count("crucible-melts"));
    // The whole mechanic: the receipt is where a player would look for an
    // explanation, and there is not one until the thing counting speaks.
    assert_eq!(receipt, vec!["Nothing you could point to".to_string()]);
    assert_eq!(run.counted("crucible-melts"), 1);
    assert!(!run.choice_open(&gated));
    apply(&mut run, Outcome::Count("crucible-melts"));
    assert!(run.choice_open(&gated));
}

#[test]
fn a_door_can_ask_for_an_assembled_item_of_a_rarity() {
    let run = a_run();
    let common = Choice {
        requires: Requirement::AssembledOfRarity(Rarity::Common),
        ..door(Outcome::FightAsWritten)
    };
    let legendary = Choice {
        requires: Requirement::AssembledOfRarity(Rarity::Legendary),
        ..door(Outcome::FightAsWritten)
    };
    assert!(!run.combat_items().is_empty(), "the fixture built nothing");
    assert!(run.choice_open(&common), "every assembled item is at least common");
    assert!(!run.choice_open(&legendary), "a full auto-build is not carrying a legendary");
}

#[test]
fn the_inspector_reads_the_live_board_rather_than_the_tray() {
    let mut run = a_run();
    let some = run.most_aligned();
    let cleared = {
        run.clear_all();
        run.most_aligned()
    };
    assert!(some >= cleared, "a board with items on it aligns no worse than an empty one");
    assert_eq!(cleared, 0, "an empty board shares nothing with itself");
    let asks_one = Choice { requires: Requirement::AlignedItems(1), ..door(Outcome::FightAsWritten) };
    assert!(!run.choice_open(&asks_one), "an empty board satisfied an inspection");
}

#[test]
fn a_door_that_wants_a_figure_cannot_be_answered_without_one() {
    let mut run = a_run();
    run.rung = 2;
    let bid = Choice {
        requires: Requirement::Figure { min: 0, max: 500 },
        ..door(Outcome::FightAsWritten)
    };
    // Always open - anybody can say a number - and yet not takeable, because
    // there is nothing here to take it with.
    assert!(run.choice_open(&bid));
    assert!(run.take_choice(&bid).is_none(), "a default bid is a bid nobody made");
    assert_eq!(run.last_figure, None);
    // Out of bounds is refused rather than clamped.
    assert!(run.take_choice_with(&bid, 900).is_none());
    assert_eq!(run.last_figure, None);
    assert!(bid.requires.describe().contains("between 0 and 500"));
}

// --------------------------------------------------------------- outcomes

#[test]
fn revealing_a_town_puts_it_on_the_road_and_saying_it_twice_does_not() {
    let mut run = a_run();
    let before = run.towns_revealed.len();
    apply(&mut run, Outcome::RevealTown("high-wick"));
    assert_eq!(run.towns_revealed.len(), before + 1);
    let again = apply(&mut run, Outcome::RevealTown("nowhere-at-all"));
    assert_eq!(again, vec!["Nothing: there is no such place".to_string()]);
}

#[test]
fn a_curated_shelf_replaces_the_shop_and_costs_nothing() {
    let mut run = a_run();
    const SHELF: &[&str] = &["Oak Handle", "Iron Blade"];
    let classes = run.classes.len();
    apply(&mut run, Outcome::OpenShop { shelves: SHELF });
    assert_eq!(run.shop.stock.len(), SHELF.len());
    assert_eq!(run.classes.len(), classes, "a curated shelf is not a bargain");
}

#[test]
fn a_granted_row_waits_for_the_board_it_is_for() {
    // E6.10: no placed piece moves, and the receipt names the slot it grew.
    let mut run = a_run();
    let before: Vec<(SlotKind, u8)> =
        SlotKind::ALL.iter().map(|&k| (k, run.loadout.slot(k).rows())).collect();
    let where_everything_was: Vec<_> = SlotKind::ALL
        .iter()
        .flat_map(|&k| {
            let s = run.loadout.slot(k);
            s.pieces().into_iter().filter_map(move |p| s.anchor_of(p).map(|a| (k, p, a)))
        })
        .collect();
    assert!(!where_everything_was.is_empty(), "the fixture placed nothing");

    apply(&mut run, Outcome::GrantRow);
    assert_eq!(run.owed_rows, 1);
    for &(k, rows) in &before {
        assert_eq!(run.loadout.slot(k).rows(), rows, "a grant grew a board before it was chosen");
    }

    assert!(run.grow_slot(SlotKind::Chest));
    assert_eq!(run.owed_rows, 0);
    assert!(!run.grow_slot(SlotKind::Chest), "a row cannot be spent twice");
    for &(k, rows) in &before {
        let want = if k == SlotKind::Chest { rows + 1 } else { rows };
        assert_eq!(run.loadout.slot(k).rows(), want, "{:?} is the wrong height", k);
    }
    for (k, p, a) in where_everything_was {
        assert_eq!(run.loadout.slot(k).anchor_of(p), Some(a), "a placed piece moved");
    }
    let receipt = run.take_receipt().expect("a row is a resolution");
    assert!(receipt[0].contains("chest"), "{:?}", receipt);
}

#[test]
fn a_taller_board_survives_being_written_down_and_read_back() {
    // Version 3 of the code carries five row counts, because one number was
    // the whole answer only while the only thing handing out room handed it to
    // every board at once.
    let mut run = a_run();
    run.owed_rows = 1;
    run.grow_slot(SlotKind::Greaves);
    let code = gearmaster_engine::share::export(&run);
    let back = gearmaster_engine::share::import(&code).expect("a code this build wrote");
    assert_eq!(back.slot_rows[SlotKind::Greaves.index()], 1);
    assert_eq!(back.slot_rows[SlotKind::Chest.index()], 0);
    let (_, lo) = back.loadout();
    assert_eq!(lo.slot(SlotKind::Greaves).rows(), run.loadout.slot(SlotKind::Greaves).rows());
    assert_eq!(lo.slot(SlotKind::Chest).rows(), run.loadout.slot(SlotKind::Chest).rows());
}

#[test]
fn an_older_code_still_reads_and_says_every_board_was_the_same_height() {
    // The two shipped boards were written at version 2 and are not ours to
    // invalidate.
    let back = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN)
        .expect("a version 2 code");
    assert_eq!(back.slot_rows, [0; 5]);
}

#[test]
fn the_underwriter_eats_exactly_one_loss_and_says_which() {
    // E6.11.
    let mut run = a_run();
    run.rung = 20;
    apply(&mut run, Outcome::Underwrite);
    assert_eq!(run.underwritten_until, Some(20 + UNDERWRITTEN_FOR));
    run.take_receipt();

    let (rung, losses) = (run.rung, run.losses);
    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert_eq!(run.rung, rung, "an eaten loss still knocked the run back");
    assert_eq!(run.losses, losses + 1, "the loss happened; it just did not count");
    assert!(run.underwritten_until.is_none(), "it was not spent");
    let settled = run.last_settlement.clone().expect("a settlement");
    assert!(settled.underwrote.is_some(), "the receipt does not say which fight it ate");
    run.back_to_loadout();

    // And the next one costs what losing costs.
    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert!(run.rung < rung, "a second loss inside the five rungs was also forgiven");
}

#[test]
fn an_underwritten_loss_expires() {
    let mut run = a_run();
    run.rung = 20;
    apply(&mut run, Outcome::Underwrite);
    run.rung = 20 + UNDERWRITTEN_FOR + 1;
    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert!(run.underwritten_until.is_some(), "it was spent after it had run out");
    assert!(run.last_settlement.as_ref().is_some_and(|s| s.underwrote.is_none()));
}

#[test]
fn scouting_grants_no_stats_whatsoever() {
    // E6.12. The board view is the entire reward, which is a thing worth
    // pinning rather than trusting.
    let mut run = a_run();
    let before = run.player_stats();
    let items = run.combat_items().len();
    apply(&mut run, Outcome::Scout);
    assert!(run.scouting);
    assert_eq!(run.player_stats(), before);
    assert_eq!(run.combat_items().len(), items);
}

#[test]
fn a_claim_ticket_is_held_until_it_is_spent() {
    let mut run = a_run();
    apply(&mut run, Outcome::ClaimTicket);
    assert_eq!(run.claim_tickets, 1);
    apply(&mut run, Outcome::ClaimTicket);
    assert_eq!(run.claim_tickets, 2, "two tickets are two claims");
}

#[test]
fn a_standing_order_for_a_kind_puts_one_on_every_shelf() {
    let mut run = a_run();
    apply(&mut run, Outcome::StandingOrder(Standing::GuaranteedKind(PieceKind::Ring)));
    for _ in 0..40 {
        assert!(
            run.shop
                .stock
                .iter()
                .any(|&i| gearmaster_engine::piece::CATALOG[i].kind == PieceKind::Ring),
            "a shelf with no ring on it, and an order that says otherwise"
        );
        // The reroll price doubles each time, which is a different mechanic
        // and not this one's subject.
        run.gold += run.reroll_cost();
        run.reroll().expect("gold");
    }
}

#[test]
fn a_free_first_reroll_is_free_once_per_restock() {
    let mut run = a_run();
    let paid = run.reroll_cost();
    assert!(paid > 0);
    apply(&mut run, Outcome::StandingOrder(Standing::FreeFirstReroll));
    assert_eq!(run.reroll_cost(), 0);
    let gold = run.gold;
    run.reroll().expect("free");
    assert_eq!(run.gold, gold, "the free one cost something");
    assert!(run.reroll_cost() > 0, "the second one is free as well");
}

#[test]
fn a_granted_quest_is_a_fact_about_this_piece_in_this_run() {
    use gearmaster_engine::piece::{Quest, QuestTrack};
    static SMALL: Quest = Quest {
        label: "go off thirty times",
        goal: 30,
        track: QuestTrack::SelfActivations,
        becomes: "Iron Blade",
    };
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    assert!(run.quest_of(id).is_none() || run.registry.def(id).quest.is_some());
    run.grant_quest(id, &SMALL);
    assert_eq!(run.quest_of(id).map(|q| q.label), Some(SMALL.label));
}

// ---------------------------------------------------------------- the melt

#[test]
fn the_crucible_gives_back_something_of_about_the_same_worth() {
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    let was = run.registry.def(id);
    let (name, slot, rating) =
        (was.name, was.slot, gearmaster_engine::rating::piece_rating(was));
    assert!(run.melt(id).is_some(), "an ordinary piece refused the pot");
    let now = run.registry.def(id);
    assert_ne!(now.name, name, "the melt gave back what went in");
    assert_eq!(now.slot, slot, "the melt changed which grid it belongs to");
    assert!(
        (gearmaster_engine::rating::piece_rating(now) - rating).abs() <= MELT_SPREAD,
        "{} to {} is further than a melt should carry",
        name,
        now.name
    );
    let receipt = run.take_receipt().expect("a melt is a resolution");
    assert!(receipt[0].contains(name), "{:?}", receipt);
}

#[test]
fn the_melt_is_seeded_and_replays() {
    let out = |seed: u64| {
        let mut run = Run::seeded(seed);
        common::build_full_loadout(&mut run);
        let id = *run.inventory().first().expect("something loose");
        run.melt(id);
        run.registry.def(id).name
    };
    assert_eq!(out(0x9001), out(0x9001), "two runs of one seed melted differently");
}

#[test]
fn a_rumour_and_a_quest_piece_refuse_the_pot() {
    let mut run = a_run();
    let d = gearmaster_engine::piece::CATALOG
        .iter()
        .position(|d| d.name == "A Word About the Crownwright")
        .expect("a rumour");
    let id = run.registry.alloc(d);
    run.owned.push(id);
    assert!(run.melt(id).is_none(), "a key went into the melt");

    let carrying = run
        .inventory()
        .into_iter()
        .find(|&i| run.registry.def(i).quest.is_some());
    if let Some(id) = carrying {
        assert!(run.melt(id).is_none(), "the far side of a task went into the melt");
    }
}

#[test]
fn every_melt_is_counted_whether_or_not_it_worked() {
    // The foundry is counting visits, not successes.
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    assert_eq!(run.counted("crucible-melts"), 0);
    run.melt(id);
    assert_eq!(run.counted("crucible-melts"), 1);
}

// ------------------------------------------------------------- and rung 51

#[test]
fn the_road_ends_at_fifty_for_anybody_who_is_not_carrying_the_mainspring() {
    let mut run = a_run();
    run.rung = gearmaster_engine::combat::LADDER.len();
    assert!(run.ladder_complete());
    assert!(!run.past_the_top(), "the road opened for a run that finished nothing");
}

#[test]
fn a_share_code_accepts_a_rung_past_the_ladder() {
    let mut run = a_run();
    run.rung = gearmaster_engine::combat::LADDER.len();
    let code = gearmaster_engine::share::export(&run);
    let back = gearmaster_engine::share::import(&code).expect("a code");
    assert_eq!(back.rung, gearmaster_engine::combat::LADDER.len());
}
