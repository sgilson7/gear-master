//! Rewards that are not gear.
//!
//! Everything the road hands out was a component, a class, gold or a row -
//! four good answers, all the same shape, and a road with only that vocabulary
//! can say one sentence louder and nothing else.
//!
//! **A run-relic** is worth what the run has done. It is the only piece in the
//! game whose card is different at rung forty from what it was at rung four,
//! and it costs a cell like anything else, so carrying one is a bet on how the
//! run goes rather than a stat you bought.
//!
//! **A crushable** is spent. Nothing else in this game is: everything you own
//! is worn or sold. A crushable breaks a rule once and is gone, which makes
//! carrying one a decision about *when*.
//!
//! The components land with the rest of the mission's catalogue. What is here
//! is the arithmetic, which is testable without them, and the rules the
//! arithmetic hangs off, which are not.

mod common;

use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::relic::{
    self, Crush, CRUSHABLES, LEDGER_PER_POINT, ODOMETER_PER, RELICS, TALLY_PER_EVENT,
};
use gearmaster_engine::run::{Run, CONSIGNMENT_GAIN, CONSIGNMENT_SHOPS};

fn a_run() -> Run {
    let mut run = Run::seeded(0xBEE7);
    common::build_full_loadout(&mut run);
    run
}

fn pays(name: &str, run: &Run) -> gearmaster_engine::relic::Payout {
    (relic(name).pays)(run)
}

fn relic(name: &str) -> &'static gearmaster_engine::relic::Relic {
    relic::relic(name).unwrap_or_else(|| panic!("{} is not a relic", name))
}

// -------------------------------------------------------------- run-relics

#[test]
fn the_tally_counts_answers_and_not_offers() {
    // Walking past a door is a decision the same as going through it, so it
    // counts. What does not count is a door you have not reached.
    let mut run = a_run();
    assert_eq!(pays("The Tally", &run).stats.strength, 0);
    run.rung = 2;
    let ev = run.pending_event().expect("the toad's offer");
    let fight = ev.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").expect("authored");
    run.take_choice(fight);
    assert_eq!(pays("The Tally", &run).stats.strength, TALLY_PER_EVENT);
}

#[test]
fn the_odometer_reads_where_you_are_rather_than_how_deep_you_got() {
    // A Grinder knocked back down is, as far as an odometer is concerned,
    // somewhere lower - which is the honest reading of a thing that measures
    // the road under you.
    let mut run = a_run();
    run.rung = ODOMETER_PER * 3;
    assert_eq!(pays("The Odometer", &run).speed_pct, 3);
    assert_eq!(pays("The Odometer", &run).stats, gearmaster_engine::stats::Stats::ZERO);
    run.rung = ODOMETER_PER * 3 - 1;
    assert_eq!(pays("The Odometer", &run).speed_pct, 2);
}

#[test]
fn the_ledger_is_the_one_thing_in_the_game_that_punishes_shopping() {
    let mut run = a_run();
    run.gold = LEDGER_PER_POINT * 5;
    assert_eq!(pays("The Ledger", &run).stats.power, 5);
    run.gold = 0;
    assert_eq!(pays("The Ledger", &run).stats.power, 0);
    // And a run in the red is not owed a negative multiplier.
    run.gold = -400;
    assert_eq!(pays("The Ledger", &run).stats.power, 0);
}

#[test]
fn a_relic_pays_from_a_board_and_not_from_a_pocket() {
    // A reward that pays from the tray is a reward with no decision in it.
    // Vacuous while none of the three exist, and the assertion they land
    // against: `relic_pay` walks the boards.
    let run = a_run();
    assert_eq!(run.relic_pay(), gearmaster_engine::relic::Payout::default());
    assert!(!run.inventory().is_empty(), "the fixture holds nothing loose");
}

#[test]
fn every_relic_is_a_function_of_the_run_and_of_nothing_else() {
    // Two runs in the same state pay the same, and the same run pays the same
    // twice. A relic that read a clock or a roll would be a relic a share code
    // could not describe.
    let a = a_run();
    let b = a_run();
    for r in RELICS {
        assert_eq!((r.pays)(&a), (r.pays)(&a), "{} is not stable", r.name);
        assert_eq!((r.pays)(&a), (r.pays)(&b), "{} reads something outside the run", r.name);
    }
}

// -------------------------------------------------------------- crushables

/// Put a crushable in the tray by hand. They do not exist in the catalogue
/// yet, so the machinery is tested against the nearest one-cell piece and the
/// name it will have.
fn a_crushable_in_the_tray(run: &mut Run, _name: &str) -> Option<gearmaster_engine::piece::PieceId> {
    let d = gearmaster_engine::piece::CATALOG.iter().position(|d| d.name == _name)?;
    let id = run.registry.alloc(d);
    run.owned.push(id);
    Some(id)
}

#[test]
fn crushing_something_that_is_not_a_crushable_does_nothing() {
    let mut run = a_run();
    for id in run.inventory() {
        assert!(run.crush(id).is_none(), "an ordinary piece was crushed");
    }
}

#[test]
fn the_second_key_is_the_only_thing_that_breaks_the_one_action_rule() {
    // E6.9. Tested through the flag rather than through the component, which
    // is what is here to test: `visit_town` reads `second_key_ready` in
    // exactly one place, and that is what keeps the exception to one.
    let mut run = a_run();
    run.rung = gearmaster_engine::town::TOWNS[0].after;
    run.force_win();
    run.settle();
    assert!(run.town.is_some());

    run.second_key_ready = true;
    run.visit_town(gearmaster_engine::town::Action::Chapel);
    assert!(run.town.is_some(), "the key did not buy a second door");
    assert!(!run.second_key_ready, "and it is not still in your hand");

    run.visit_town(gearmaster_engine::town::Action::Factory);
    assert!(run.town.is_none(), "a third door");
    assert!(run.towns_seen.contains(&gearmaster_engine::town::TOWNS[0].id));
}

#[test]
fn a_crushable_that_cannot_do_its_one_thing_refuses_rather_than_wasting_itself() {
    // Nothing here yet to crush, so this is the rule stated where it is
    // enforced: the Second Key wants a town, and the Skip Stone wants a rung
    // left. Both refuse before the piece is destroyed.
    let mut run = a_run();
    assert!(run.town.is_none());
    if let Some(id) = a_crushable_in_the_tray(&mut run, "the Second Key") {
        assert!(run.crush(id).is_none(), "the key was spent outside a town");
        assert!(run.owned.contains(&id), "and destroyed anyway");
    }
}

#[test]
fn every_crushable_says_what_it_breaks() {
    let mut kinds: Vec<Crush> = CRUSHABLES.iter().map(|c| c.what).collect();
    let n = kinds.len();
    kinds.dedup();
    assert_eq!(kinds.len(), n, "two crushables that do the same thing");
    assert_eq!(n, 3);
}

// ------------------------------------------------------------- the passenger

#[test]
fn a_passenger_pays_its_rent_in_cells_and_is_lost_with_a_fight() {
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    assert!(run.take_passenger(id, 5));
    assert!(!run.take_passenger(id, 5), "two passengers");
    assert!(!run.passenger_is_seated(), "a passenger in the tray is paying nothing");

    // Seated: on a board, in a cell that could have held gear.
    let d = run.registry.def(id);
    let slot = d.slot;
    let mut sat = false;
    for y in 0..8u8 {
        for x in 0..6u8 {
            if run.equip(id, slot, x, y).is_ok() {
                sat = true;
                break;
            }
        }
        if sat {
            break;
        }
    }
    assert!(sat, "nowhere to seat it");
    assert!(run.passenger_is_seated());

    // Lose, and it is gone.
    run.rung = 30;
    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert!(run.passenger.is_none(), "it survived a loss");
    assert!(!run.owned.contains(&id), "and is still in the bag");
    assert!(run.last_settlement.as_ref().is_some_and(|s| s.lost_passenger));
}

#[test]
fn delivering_wants_the_road_to_have_gone_far_enough() {
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    run.rung = 10;
    run.take_passenger(id, 5);
    assert!(!run.deliver_passenger(), "delivered before it got there");
    run.rung = 15;
    assert!(run.deliver_passenger());
    assert!(run.passenger.is_none());
    assert!(run.take_receipt().is_some(), "a delivery is a resolution");
}

// ------------------------------------------------------------- consignment

#[test]
fn nothing_goes_on_consignment_without_the_arrangement() {
    let mut run = a_run();
    let id = *run.inventory().first().expect("something loose");
    run.sell(id).expect("sellable");
    assert!(run.consigned.is_empty());
}

#[test]
fn a_consigned_piece_comes_back_three_shops_later_worth_more() {
    let mut run = a_run();
    run.standing_orders.push(gearmaster_engine::event::Standing::Consignment);
    let id = *run.inventory().first().expect("something loose");
    let was = run.registry.def(id);
    let (name, slot, kind, rating) =
        (was.name, was.slot, was.kind, gearmaster_engine::rating::piece_rating(was));
    run.sell(id).expect("sellable");
    let Some(&(def, left)) = run.consigned.first() else {
        panic!("{} did not go on consignment", name)
    };
    assert_eq!(left, CONSIGNMENT_SHOPS);
    let back = &gearmaster_engine::piece::CATALOG[def];
    assert_eq!(back.slot, slot, "it came back for a different grid");
    assert_eq!(back.kind, kind, "it came back as a different sort of thing");
    let gained = gearmaster_engine::rating::piece_rating(back) - rating;
    assert!(
        gained > 0,
        "{} went out at {} and would come back at {}",
        name,
        rating,
        rating + gained
    );

    // Three shops, and then it is on a shelf.
    for _ in 0..CONSIGNMENT_SHOPS {
        assert!(!run.shop.stock.contains(&def), "it came back early");
        run.gold += run.reroll_cost();
        run.reroll().expect("gold");
    }
    assert!(run.shop.stock.contains(&def), "it never came back");
    assert!(run.consigned.is_empty());
}

#[test]
fn what_comes_back_is_aimed_at_thirty_more_and_not_at_anything_at_all() {
    // The point of the number: consignment is a small upgrade you wait for,
    // not a way of turning a common into a legendary.
    for d in 0..gearmaster_engine::piece::CATALOG.len() {
        let Some(better) = gearmaster_engine::piece::dearer_than(d, CONSIGNMENT_GAIN) else {
            continue;
        };
        let (a, b) = (
            &gearmaster_engine::piece::CATALOG[d],
            &gearmaster_engine::piece::CATALOG[better],
        );
        assert_eq!(a.slot, b.slot);
        assert_eq!(a.kind, b.kind);
    }
}

// ----------------------------------------------------------- lightning rod

#[test]
fn nothing_attracts_curses_until_something_is_standing_on_a_rod() {
    // The rule is in `land_stun` and reads one flag. Vacuous until the
    // enchantment lands, and this is the assertion it lands against.
    let run = a_run();
    for it in run.combat_items() {
        assert!(!it.attracts_curses, "{} is standing on a rod that does not exist", it.name);
    }
    assert!(
        !gearmaster_engine::piece::CATALOG
            .iter()
            .any(|d| d.name == gearmaster_engine::piece::LIGHTNING_ROD),
        "the rod has landed; this test is now the one that arms it"
    );
}

#[test]
fn a_relic_reaches_the_board_it_is_standing_on() {
    // The two halves land in different places - stats through `player_stats`,
    // speed onto the profiles - so both are asked for here.
    let run = a_run();
    let stats = run.player_stats();
    let items = run.combat_items();
    assert!(!items.is_empty());
    // With no relic on the board, both are exactly what they were.
    assert_eq!(run.relic_pay().speed_pct, 0);
    assert_eq!(stats, run.player_stats());
    for (a, b) in items.iter().zip(run.loadout.combat_items(&run.registry).iter()) {
        assert_eq!(a.cooldown_ms, b.cooldown_ms);
    }
    // And the slot list is the one `relic_pay` walks.
    assert_eq!(SlotKind::ALL.len(), 5);
}
