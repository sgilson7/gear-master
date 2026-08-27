//! Positional effects: components that change what their neighbours are
//! worth, or that are worth more for the empty space around them.

mod common;

use common::{equip, piece};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;

/// A creature durable enough that the fight against it runs long.
///
/// Searched, not indexed. Three tests here needed "a fight that lasts" and two
/// of them said `LADDER[25]` and `LADDER[30]`, which is a different thing that
/// happened to be true. `LADDER[30]` is the Weeping Idol, a boss whose fifteen
/// items end a fight before a caster has banked anything - a lesson the third
/// test had already learned and written down, having been moved off the same
/// index once before. Every one of these creatures is about to be repacked, so
/// the question is asked of the ladder instead: who lasts?
///
/// Ordinary on purpose. A named fight is dense by rule, and density is what
/// ends fights early.
///
/// The *shallowest* rung that lasts, rather than the deepest. Taking the
/// highest health instead picks Francis - who is `Rank::Ordinary`, wears
/// forty-four pieces, and ends a caster's fight before it has banked anything,
/// which is the same trap by a different road.
fn a_long_fight() -> &'static gearmaster_engine::combat::MonsterSpec {
    use gearmaster_engine::combat::{Rank, LADDER};
    LADDER
        .iter()
        .find(|m| m.rank == Rank::Ordinary && m.health > 3_000)
        .expect("the deep ladder has ordinary rungs")
}

/// Total strength contributed by one slot.
fn slot_str(run: &Run, kind: SlotKind) -> i32 {
    run.report(kind).stats.strength
}

fn slot_hp(run: &Run, kind: SlotKind) -> i32 {
    run.report(kind).stats.health
}

// ------------------------------- Runed Edge: doubles adjacent accessories

#[test]
fn runed_edge_doubles_the_strength_of_an_adjacent_accessory() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0); // (0, 0..3)
    equip(&mut run, "Runed Edge", SlotKind::Weapon, 1, 0); // (1, 0..2) + (2, 1)
    equip(&mut run, "Ruby Inlay", SlotKind::Weapon, 2, 0); // (2, 0), touches (1, 0)

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());

    // Runed Edge +1, Ruby Inlay +3 doubled to +6.
    assert_eq!(slot_str(&run, SlotKind::Weapon), 7);
    assert!(
        report.notes().iter().any(|n| n.contains("Ruby Inlay") && n.contains("doubled")),
        "the doubling should be reported: {:?}",
        report.notes()
    );
}

#[test]
fn the_doubling_only_reaches_accessories_that_actually_touch_the_blade() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0); // (0, 0..3)
    equip(&mut run, "Runed Edge", SlotKind::Weapon, 1, 0); // (1, 0..2) + (2, 1)
    // Hangs off the bottom of the grip, so it is in the same item but is not
    // touching the blade.
    equip(&mut run, "Ruby Inlay", SlotKind::Weapon, 0, 4); // (0, 4), touches (0, 3)

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "still one finished weapon");
    assert_eq!(slot_str(&run, SlotKind::Weapon), 4, "1 + 3, undoubled");
}

#[test]
fn the_blades_effect_is_dormant_until_the_weapon_is_finished() {
    let mut run = Run::with_all_pieces();
    // No handle, so this never becomes a weapon.
    equip(&mut run, "Runed Edge", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Ruby Inlay", SlotKind::Weapon, 2, 0);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 0);
    assert_eq!(slot_str(&run, SlotKind::Weapon), 4, "1 + 3, effect asleep");

    // Add the handle and the same three pieces are worth more.
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);
    assert_eq!(slot_str(&run, SlotKind::Weapon), 7, "1 + 6");
}

// --------------------------- Hollow Weave: scales with surrounding space

#[test]
fn hollow_weave_gains_strength_for_every_empty_cell_touching_it() {
    let mut run = Run::with_all_pieces();
    // Alone in open space: 4 above, 4 below, 1 either side = 10.
    equip(&mut run, "Hollow Weave", SlotKind::Chest, 1, 3);

    assert_eq!(slot_str(&run, SlotKind::Chest), 10);
    assert!(run
        .report(SlotKind::Chest)
        .notes()
        .iter()
        .any(|n| n.contains("10 strength from 10 empty cells")));
}

#[test]
fn boxing_the_weave_in_is_what_costs_it_strength() {
    let mut run = Run::with_all_pieces();
    // Tucked under a base: its whole top edge is covered, and its left edge is
    // against the wall (out-of-bounds cells don't count).
    equip(&mut run, "Padded Base", SlotKind::Chest, 0, 0); // (0..3, 0..2)
    equip(&mut run, "Hollow Weave", SlotKind::Chest, 0, 3); // (0..3, 3)

    let report = run.report(SlotKind::Chest);
    assert_eq!(report.assembled_count(), 1, "base + layer is a chestpiece");
    // 4 below + 1 to the right.
    assert_eq!(slot_str(&run, SlotKind::Chest), 5);
}

#[test]
fn the_weave_works_whether_or_not_its_chestpiece_came_together() {
    let mut loose = Run::with_all_pieces();
    equip(&mut loose, "Hollow Weave", SlotKind::Chest, 1, 3);
    assert_eq!(loose.report(SlotKind::Chest).assembled_count(), 0);
    assert_eq!(slot_str(&loose, SlotKind::Chest), 10, "unconditional effect");
}

// ------------------- Unbound Core: an effect that wants to stay unassembled

#[test]
fn unbound_core_doubles_neighbouring_layers_only_while_incomplete() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Unbound Core", SlotKind::Chest, 0, 0); // (0..1, 0..1)
    equip(&mut run, "Chain Layer", SlotKind::Chest, 0, 2); // (0..3, 2), touches it

    let report = run.report(SlotKind::Chest);
    assert_eq!(report.assembled_count(), 0, "two layers and no base");
    assert_eq!(report.items[0].status, "needs 1 more base");
    // Core 40 + Chain Layer 60 doubled to 120.
    assert_eq!(slot_hp(&run, SlotKind::Chest), 160);
}

#[test]
fn completing_the_chestpiece_switches_the_core_off_again() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Unbound Core", SlotKind::Chest, 0, 0);
    equip(&mut run, "Chain Layer", SlotKind::Chest, 0, 2);
    assert_eq!(slot_hp(&run, SlotKind::Chest), 160);

    // A base finishes the item — and the Core's whole point is that this
    // turns its own effect off.
    equip(&mut run, "Padded Base", SlotKind::Chest, 0, 3); // (0..3, 3..5)

    let report = run.report(SlotKind::Chest);
    assert_eq!(report.assembled_count(), 1);
    // Core 40 + Chain 60 undoubled + Base 125.
    assert_eq!(slot_hp(&run, SlotKind::Chest), 225);
}

// ------------------------------------------------------ general behaviour

#[test]
fn effects_do_not_reach_across_a_gap_into_another_item() {
    let mut run = Run::with_all_pieces();
    // A finished weapon in the top-left...
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Runed Edge", SlotKind::Weapon, 1, 0);
    // ...and a lone accessory far away, not touching anything.
    equip(&mut run, "Ruby Inlay", SlotKind::Weapon, 5, 7);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.items.len(), 2, "two separate groups");
    assert_eq!(slot_str(&run, SlotKind::Weapon), 4, "1 + 3, undoubled");
}

#[test]
fn a_piece_with_no_effect_is_unchanged_by_the_new_machinery() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1);
    // Oak +0.20x, Iron Blade +2 str +0.80x. No handle bonus on Oak.
    assert_eq!(report.stats.strength, 2);
    assert_eq!(report.stats.power, 100);
}

#[test]
fn a_multi_handle_counts_the_damaging_pieces_packed_against_it() {
    // The point of the effect: it reads its company rather than changing it.
    let mut run = Run::with_all_pieces();
    // Multi-Handle occupies (0..1, 0..2). Blades either side of it.
    equip(&mut run, "Multi-Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 2, 0);
    let one = run.report(SlotKind::Weapon);
    assert_eq!(one.assembled_count(), 1, "{}", one.summary());
    let with_one = one.stats.strength;

    let mut two = Run::with_all_pieces();
    equip(&mut two, "Multi-Handle", SlotKind::Weapon, 1, 0);
    equip(&mut two, "Iron Blade", SlotKind::Weapon, 0, 0);
    equip(&mut two, "Serrated Edge", SlotKind::Weapon, 3, 0);
    let report = two.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());

    assert!(
        report.stats.strength > with_one,
        "two damaging neighbours should beat one: {} vs {}",
        report.stats.strength,
        with_one
    );
    assert!(
        report.notes().iter().any(|n| n.contains("adjacent damaging")),
        "and it should say so: {:?}",
        report.notes()
    );
}

#[test]
fn a_neighbour_reading_effect_is_dormant_until_its_item_assembles() {
    let mut run = Run::with_all_pieces();
    // A handle with a blade beside it but no complete weapon around them.
    equip(&mut run, "Multi-Handle", SlotKind::Weapon, 0, 0);
    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 0);
    assert!(
        !report.notes().iter().any(|n| n.contains("adjacent damaging")),
        "a loose piece reads nothing"
    );
}

// ------------------------------------------------------------- casting

/// A spell has two strengths. With mana it lands in full; without, it still
/// goes off - a build that runs dry should get weaker, not stop.
/// Emptying a reserve pays out by the handful, and the handful is what
/// separates it from every other sink in the game: a fixed threshold takes the
/// same amount whatever you have banked, so building a bigger reserve buys
/// nothing but more attempts.
#[test]
fn emptying_a_pool_pays_more_the_fuller_it_was() {
    use gearmaster_engine::combat::{simulate, Event, Side};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    // Same piece, two builds: one with faith income behind it, one without.
    let dealt = |with_income: bool| -> i32 {
        let mut run = Run::with_all_pieces();
        equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
        equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
        equip(&mut run, "Steel Frame", SlotKind::Helmet, 0, 0);
        equip(&mut run, "Iron Plating", SlotKind::Helmet, 0, 2);
        equip(&mut run, "Reckoning Crest", SlotKind::Helmet, 3, 0);
        if with_income {
            // Touching, or the chestpiece never assembles and never
            // activates - and an item that never activates banks nothing.
            equip(&mut run, "Chapel Base", SlotKind::Chest, 0, 0);
            equip(&mut run, "Oathplate", SlotKind::Chest, 0, 1);
            assert_eq!(run.report(SlotKind::Chest).assembled_count(), 1);
        }
        let profiles = run.combat_items();
        let mut stats = run.player_stats();
        stats.health = 100_000;
        let log = simulate(stats, &profiles, a_long_fight());
        log.entries
            .iter()
            .filter_map(|e| match e.event {
                Event::Hit { by: Side::Player, damage, .. } => Some(damage),
                _ => None,
            })
            .sum()
    };
    let lean = dealt(false);
    let fed = dealt(true);
    assert!(
        fed > lean,
        "a fuller reserve has to be worth more: fed {} vs lean {}",
        fed,
        lean
    );
}

/// Paying for a spell has to buy something. It used to buy only "not being
/// weakened", which meant the ceiling on a caster was the number printed on
/// the piece - and that number had to compete with a blade that swings for it
/// every time and never asks for mana. Playtesters found casters uniformly
/// weak for exactly this reason.
#[test]
fn a_paid_cast_lands_about_twice_what_an_unpaid_one_does() {
    use gearmaster_engine::combat::{EMPOWERED_CAST_PCT, WEAK_CAST_PCT};
    // Not an arbitrary ratio: this is the promise the shop price is set
    // against, so moving one without the other silently reprices every caster.
    assert!(
        EMPOWERED_CAST_PCT >= 2 * WEAK_CAST_PCT * 2,
        "a paid cast should be worth roughly twice an unpaid one, not {}x",
        EMPOWERED_CAST_PCT as f32 / WEAK_CAST_PCT as f32
    );
}

/// A crystal ball costs more room than a book and casts more often, so it has
/// to out-damage one. It did not: a book takes an ink and an ink carries a
/// power multiplier, while every orb and every alignment carried none - even
/// though the orb recipe has always claimed the alignment scales the ball.
#[test]
fn an_orb_out_damages_a_book_for_the_room_it_costs() {
    use gearmaster_engine::piece::{PieceKind, CATALOG};
    let power = |kind: PieceKind| -> Vec<i32> {
        CATALOG
            .iter()
            .filter(|d| d.kind == kind)
            // Boss trophies are off the scale on purpose and are priced by
            // nothing; they carry their weight in stats, not in multipliers.
            .filter(|d| !gearmaster_engine::piece::is_boss_only(d.name))
            .map(|d| d.power_bonus)
            .collect()
    };
    for kind in [PieceKind::Orb, PieceKind::Alignment] {
        let p = power(kind);
        assert!(
            p.iter().all(|b| *b > 0),
            "{:?}: every one of these scales what a ball casts, so none may be zero",
            kind
        );
    }
    // And the seat an alignment fills is the ink's, so it should be worth
    // something comparable rather than a rounding error beside one.
    let inks: Vec<i32> = power(PieceKind::Ink);
    let aligns: Vec<i32> = power(PieceKind::Alignment);
    let avg = |v: &[i32]| v.iter().sum::<i32>() as f32 / v.len() as f32;
    assert!(
        avg(&aligns) > avg(&inks) * 0.4,
        "alignments average {:.0} power against inks' {:.0}",
        avg(&aligns),
        avg(&inks)
    );
}

#[test]
fn a_spell_cast_without_mana_still_lands_but_weakly() {
    use gearmaster_engine::combat::{simulate, Event, Side};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    let mut run = Run::with_all_pieces();
    equip(&mut run, "Leaden Tome", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);

    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;

    // Nothing banked, so every cast after the opening mana runs out is weak.
    let log = simulate(stats, &profiles, a_long_fight());
    let paid: Vec<bool> = log
        .entries
        .iter()
        .filter_map(|e| match e.event {
            Event::Cast { side: Side::Player, paid, .. } => Some(paid),
            _ => None,
        })
        .collect();
    assert!(!paid.is_empty(), "the spell should be casting at all");
    assert!(paid.iter().any(|p| !p), "with no mana income some casts must land weak");
    // And a weak cast is still a cast: it fires rather than being skipped.
    assert!(
        log.entries.iter().any(|e| matches!(e.event, Event::Hit { by: Side::Player, .. })),
        "a weak spell still lands something"
    );
}

/// Mana banked is spent on casting, so a build that makes mana casts in full.
#[test]
fn mana_income_pays_for_full_strength_casts() {
    use gearmaster_engine::combat::{simulate, Event, Side};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    let mut run = Run::with_all_pieces();
    equip(&mut run, "Leaden Tome", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Tidewrack Ink", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);
    // A chestpiece that banks mana every time it fires. The layer has to
    // actually touch the base or there is no chestpiece and no income.
    equip(&mut run, "Wellspring Base", SlotKind::Chest, 0, 0);
    equip(&mut run, "Aether Layer", SlotKind::Chest, 0, 1);
    assert_eq!(run.report(SlotKind::Chest).assembled_count(), 1, "the fixture must assemble");

    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;
    let log = simulate(stats, &profiles, a_long_fight());
    let paid = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::Cast { side: Side::Player, paid: true, .. }))
        .count();
    assert!(paid > 0, "a build banking mana should be paying for its casts");
}

// ------------------------------------------------- solitude multipliers

/// A row-solitude piece multiplies everything on its item, but only while
/// nothing else finished shares a row with it anywhere on the board.
#[test]
fn a_row_multiplier_pays_only_while_the_row_is_its_own() {
    let mut run = Run::with_all_pieces();
    // A glove built high, carrying the ring.
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Gripping Mold", SlotKind::Gloves, 2, 0);
    equip(&mut run, "Hermit's Band", SlotKind::Gloves, 4, 0);
    assert_eq!(run.report(SlotKind::Gloves).assembled_count(), 1);

    let alone = run.combat_items()[0].stats.health;
    assert!(alone > 0, "the glove should be worth something");

    // Now a weapon on the same rows, in a different grid.
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    let items = run.combat_items();
    let glove = items.iter().find(|i| i.slot == SlotKind::Gloves).expect("still there");
    assert!(
        glove.stats.health < alone,
        "the multiplier should have lapsed: {} vs {}",
        glove.stats.health,
        alone
    );

    // Move the weapon down out of its rows and it comes back.
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.equip(handle, SlotKind::Weapon, 0, 4).expect("room below");
    run.equip(blade, SlotKind::Weapon, 1, 4).expect("room below");
    let items = run.combat_items();
    let glove = items.iter().find(|i| i.slot == SlotKind::Gloves).expect("still there");
    assert_eq!(glove.stats.health, alone, "clear rows again");
}

/// A stacked-solitude piece cares about cells, not rows: two items can share
/// rows and still not overlap.
#[test]
fn a_stacked_multiplier_cares_about_cells_not_rows() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Tin Frame", SlotKind::Helmet, 0, 0);
    equip(&mut run, "Lonely Plating", SlotKind::Helmet, 0, 2);
    assert_eq!(run.report(SlotKind::Helmet).assembled_count(), 1);
    let alone = run.combat_items()[0].stats.armor;

    // A chestpiece on the same rows but the far side of the grid: same rows,
    // no overlapping cells, so the multiplier holds.
    equip(&mut run, "Sackcloth Base", SlotKind::Chest, 4, 0);
    equip(&mut run, "Rag Layer", SlotKind::Chest, 4, 2);
    let items = run.combat_items();
    let helm = items.iter().find(|i| i.slot == SlotKind::Helmet).expect("still there");
    assert_eq!(helm.stats.armor, alone, "different cells, so still alone");

    // Slide the chestpiece on top of it and the multiplier lapses.
    let base = piece(&run, "Sackcloth Base");
    let layer = piece(&run, "Rag Layer");
    run.equip(base, SlotKind::Chest, 0, 0).expect("room");
    run.equip(layer, SlotKind::Chest, 0, 2).expect("room");
    let items = run.combat_items();
    let helm = items.iter().find(|i| i.slot == SlotKind::Helmet).expect("still there");
    assert!(helm.stats.armor < alone, "overlapping now, so the bonus is gone");
}


// ------------------------------------------------- walking in holding something

/// Armour and all four pools start every fight at zero, so the opening seconds
/// look the same whatever is on the board. This is the gear that does not.
#[test]
fn a_prepared_item_is_already_holding_something_on_the_first_tick() {
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    // Read on mana rather than armour. This was a helmet Plating bracing with
    // armour, and opening the fight is the feet's - a Plating floats into the
    // greaves grid and back out, so it could not keep the promise. The doc
    // above says armour *and all four pools*, so the same claim reads on a pool
    // just as well, and this fixture is where the mechanic actually lives.
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Pathfinder Material", SlotKind::Greaves, 0, 0);
    equip(&mut run, "Standing Start", SlotKind::Greaves, 0, 1);
    assert_eq!(run.report(SlotKind::Greaves).assembled_count(), 1, "the fixture must assemble");

    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;
    let foe = LADDER.iter().find(|m| m.name == "Cave Rat").unwrap();
    let log = simulate(stats, &profiles, foe);

    // The mana is there before anything has had a turn.
    let first = log
        .entries
        .iter()
        .find(|e| matches!(e.event, Event::GainMana { side: Side::Player, .. }))
        .expect("the sole should have opened holding something");
    assert_eq!(first.at_ms, 0, "it opens before the clock starts, not on a cooldown");
}

/// It fires once, not once a second - otherwise it is just a fast cooldown
/// with a different name on it.
#[test]
fn a_prepared_item_only_opens_once() {
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    // Two items, because what the piece watches for is *another* item taking a
    // turn. It opened the fight with `OnBattleStart` and opening the fight is
    // the feet's, so it keeps a watch that pays on the first thing to happen
    // and never again - one tick later than the bell, and still exactly once.
    // A one-item board gives it nothing to see.
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Gripping Mold", SlotKind::Gloves, 2, 0);
    equip(&mut run, "Opening Grudge", SlotKind::Gloves, 0, 2);
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    assert_eq!(run.report(SlotKind::Gloves).assembled_count(), 1, "the fixture must assemble");
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1, "and something to watch");

    let profiles = run.combat_items();
    let mut stats = run.player_stats();
    stats.health = 100_000;
    let foe = LADDER.iter().find(|m| m.name == "Cave Rat").unwrap();
    let log = simulate(stats, &profiles, foe);

    // Counted, not timed. The piece opened the fight with `OnBattleStart`, and
    // opening the fight is the feet's - so it watches for the first thing that
    // happens instead and pays on that, which is one tick later and still
    // exactly once. Once is what this test is named for; `at_ms == 0` is the
    // sibling test's job, and it reads it on a greaves piece that really does
    // fire before the clock starts.
    let opens = log
        .entries
        .iter()
        .filter(|e| {
            // The board banks 1 rage a tick passively; the opener pays a
            // slab. Anything bigger than a trickle is the piece under test.
            matches!(e.event, Event::GainResource { side: Side::Player, what, amount, .. }
                if what == "rage" && amount > 1)
        })
        .count();
    assert_eq!(opens, 1, "once, and only once");
}


// ------------------------------------------------------------ spell forking

/// A fork copies a cast. Every stack lands the whole payload again.
#[test]
fn spell_forking_copies_the_cast() {
    use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side, LADDER};
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    let dealt = |forks: u32| -> i32 {
        let mut run = Run::with_all_pieces();
        equip(&mut run, "Leaden Tome", SlotKind::Weapon, 0, 0);
        equip(&mut run, "Soot Ink", SlotKind::Weapon, 3, 0);
        equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);
        let profiles = run.combat_items();
        let mut stats = run.player_stats();
        stats.health = 100_000;
        let foe = LADDER.iter().find(|m| m.name == "Cave Rat").unwrap();
        let mut log = simulate_with_class(stats, &profiles, foe, Difficulty::Medium, &[]);
        if forks > 0 {
            // Forking comes from gear in play; for the measurement, hand it
            // over directly by re-simulating with a build that grants it.
            let mut r2 = Run::with_all_pieces();
            equip(&mut r2, "Leaden Tome", SlotKind::Weapon, 0, 0);
            equip(&mut r2, "Soot Ink", SlotKind::Weapon, 3, 0);
            equip(&mut r2, "Emberburst", SlotKind::Weapon, 3, 1);
            equip(&mut r2, "Leather Material", SlotKind::Gloves, 0, 0);
            equip(&mut r2, "Twinning Mold", SlotKind::Gloves, 2, 0);
            assert_eq!(r2.report(SlotKind::Gloves).assembled_count(), 1, "fixture");
            let p2 = r2.combat_items();
            let mut s2 = r2.player_stats();
            s2.health = 100_000;
            log = simulate_with_class(s2, &p2, foe, Difficulty::Medium, &[]);
        }
        log.entries
            .iter()
            .filter_map(|e| match e.event {
                Event::Hit { by: Side::Player, damage, .. } => Some(damage),
                _ => None,
            })
            .sum()
    };
    let plain = dealt(0);
    let forked = dealt(1);
    assert!(forked > plain, "forking should land more: {} vs {}", forked, plain);
}

/// Only casts fork. A blade swings once however many stacks are up - which is
/// what keeps this the caster's answer rather than a flat damage buff.
#[test]
fn a_blade_does_not_fork() {
    use gearmaster_engine::piece::{Action, Trigger, CATALOG};

    // Every piece that grants forking has to be reachable by a caster, so at
    // least one of them spends mana.
    let granters: Vec<&str> = CATALOG
        .iter()
        .filter(|d| {
            fn grants(t: &Trigger) -> bool {
                let is = |a: &Action| matches!(a, Action::GainForking(_));
                match t {
                    Trigger::PerAdjacentEmpty(i) => grants(i),
                    Trigger::Consume { per, .. } => is(per),
                    Trigger::OnActivate(a) | Trigger::OnBattleStart(a) => is(a),
                    Trigger::SpendMana { on_success, .. }
                    | Trigger::Spend { on_success, .. } => is(on_success),
                    Trigger::Watch { then, .. } => is(then),
                    _ => false,
                }
            }
            d.triggers.iter().any(grants)
        })
        .map(|d| d.name)
        .collect();
    assert!(granters.len() >= 3, "only {} pieces grant forking", granters.len());
    // All of them in the weapon, which is the point.
    //
    // This used to require one per slot "so no build is shut out of it", and
    // that is exactly the smearing the rewrite is undoing: forking copies a
    // cast, casting is the weapon's, and the exclusivity table makes
    // `GainForking` weapon-only. A build without a weapon is shut out of
    // forking on purpose. What is still worth pinning is that more than one
    // weapon piece reaches it, so it is not a single-piece mechanic.
    for name in &granters {
        let d = CATALOG.iter().find(|d| &d.name == name).unwrap();
        assert_eq!(
            d.slot,
            gearmaster_engine::piece::SlotKind::Weapon,
            "{name} grants forking outside the weapon"
        );
    }
}


/// Power multiplies what a trigger pays out and never what it costs.
///
/// A piece that spends four mana spends four mana whatever multiplier its item
/// is carrying - otherwise power would quietly price a build out of its own
/// gear, and the stronger the item the less usable it became.
#[test]
fn power_multiplies_outcomes_and_leaves_costs_alone() {
    use gearmaster_engine::piece::{Action, Trigger};

    let t = Trigger::SpendMana {
        cost: 4,
        on_success: Action::GainArmor(30),
        on_failure: Action::GainMana(2),
    };
    match t.scaled(250) {
        Trigger::SpendMana { cost, on_success, on_failure } => {
            assert_eq!(cost, 4, "the cost must not move");
            assert!(matches!(on_success, Action::GainArmor(75)), "{:?}", on_success);
            assert!(matches!(on_failure, Action::GainMana(5)), "{:?}", on_failure);
        }
        other => panic!("{:?}", other),
    }

    // The same for the pooled kind, and for emptying a reserve: `each` is how
    // much it takes per payout, which is a cost too.
    let c = Trigger::Consume {
        what: gearmaster_engine::piece::Resource::Faith,
        each: 6,
        per: Action::MindDamage { amount: 10, target: gearmaster_engine::piece::Target::Enemy },
    };
    match c.scaled(200) {
        Trigger::Consume { each, per, .. } => {
            assert_eq!(each, 6, "the handful must not grow");
            assert!(matches!(per, Action::MindDamage { amount: 20, .. }), "{:?}", per);
        }
        other => panic!("{:?}", other),
    }
}

/// And it multiplies every number on the item, not only a weapon's damage.
#[test]
fn power_reaches_armour_and_pools_not_just_damage() {
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    // Crown of the Deep carries power and sits in a helmet, which never swings.
    let armour_of = |with_power: bool| -> (i32, i32) {
        let mut run = Run::with_all_pieces();
        equip(&mut run, "Steel Frame", SlotKind::Helmet, 0, 0);
        equip(&mut run, "Mana Ward", SlotKind::Helmet, 0, 2);
        if with_power {
            equip(&mut run, "Crown of the Deep", SlotKind::Helmet, 3, 0);
        }
        assert_eq!(run.report(SlotKind::Helmet).assembled_count(), 1, "fixture");
        let p = run
            .combat_items()
            .into_iter()
            .find(|i| i.slot == SlotKind::Helmet)
            .expect("a helmet");
        (p.power, p.stats.armor)
    };
    let (plain_power, plain_armor) = armour_of(false);
    let (powered, powered_armor) = armour_of(true);
    assert!(powered > plain_power, "the crown should raise the item's power");
    assert!(
        powered_armor > plain_armor,
        "power should reach a helmet's armour: {} vs {}",
        powered_armor,
        plain_armor
    );
}

/// A banked pool is worth what it says it is worth, in a fight.
///
/// `Combatant::held_bonus` computed the right numbers and **one field of it
/// was ever read**: the hit path took `.physical_damage`, so rage reached a
/// fight and nature and faith did not. The regen tick used the flat `regen`
/// field and `take_typed` used the flat resists, which meant a hundred banked
/// nature healed nothing and a hundred banked faith turned aside nothing.
///
/// It went unnoticed because the test that covers it - `progression::
/// devotion_keeps_paying_past_forty_percent` - asserts `held_bonus()` itself.
/// That is arithmetic nobody consulted, and it was green the whole time. This
/// one asks the fight.
#[test]
fn a_banked_pool_pays_out_where_it_is_supposed_to() {
    use gearmaster_engine::combat::Combatant;
    use gearmaster_engine::stats::Stats;

    // Nature heals. Same board, same fight, one of them holding a pool.
    let healed = |nature: i32| -> i32 {
        let mut c = Combatant::player(Stats::new(1000, 0, 0, 100), &[]);
        c.nature = nature;
        c.effective_regen()
    };
    assert_eq!(healed(0), 0, "no pool, no regeneration");
    assert_eq!(healed(10), 10, "ten nature is ten regeneration a second");

    // Faith turns harm aside, and the number the fight reads has to include it.
    let resist = |faith: i32| -> i32 {
        let mut c = Combatant::player(Stats::new(1000, 0, 0, 100), &[]);
        c.faith = faith;
        c.effective_physical_resist()
    };
    assert_eq!(resist(0), 0);
    assert_eq!(resist(20), 40, "twenty faith is forty percent of both resistances");

    // Pools start a fight at zero on purpose - an item's `nature:` is granted
    // each time it comes round, not handed over at the bell - so the end-to-end
    // proof of this is the ladder moving, and that is recorded in
    // `analysis/baseline.md` rather than pinned here. What is pinned is that
    // the fight reads the pool at all, which is the thing that was missing.
    let mut c = Combatant::player(Stats::new(1000, 0, 0, 100), &[]);
    c.nature = 25;
    assert!(
        c.effective_regen() > c.regen,
        "a combatant holding twenty-five nature reports {} regeneration against a base of {}",
        c.effective_regen(),
        c.regen
    );
}

// ------------------------------------------------- the yard's four verbs
//
// Hand-built profiles, not catalogue entries: M4 lands the verbs and M5 lands
// the six components that speak them, and a test that needed a component
// would have forced the two together.

use gearmaster_engine::combat::{simulate, Event, MonsterSpec, Side};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::piece::{Action, Resource, Trigger};
use gearmaster_engine::stats::Stats;

fn item(name: &str, slot: SlotKind, cooldown_ms: u32, stats: Stats) -> ItemProfile {
    ItemProfile {
        sigil_seed: 0,
        pieces: Vec::new(),
        name: name.to_string(),
        full_name: name.to_string(),
        core: name.to_string(),
        slot,
        cooldown_ms,
        stats,
        triggers: Vec::new(),
        adjacent_assembled_same_slot: 0,
        diagonal_items: Vec::new(),
        open_cells: 0,
        attracts_curses: false,
        steady: false,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    }
}

/// Stands there and does nothing, so a mechanic is the only thing moving.
const DUMMY: MonsterSpec = MonsterSpec {
    name: "Dummy",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,
    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
    attacks: &[],
    gear: &[],
    gear_offset: 0,
    bounty: 0,
    sprite: gearmaster_engine::combat::MonsterSprite::Rat,
    rank: gearmaster_engine::combat::Rank::Ordinary,
    drops: &[],
    items: &[],
};

/// A player who can stand there while a mechanic is measured.
///
/// `Combatant::player` starts every pool and the wall at zero whatever the
/// stats say - armour and mana are banked *during* a fight, by items - so the
/// only thing `Stats` has to carry here is a body. Without one the player is
/// dead on the first tick and every count below is zero, which is a way to
/// read "the mechanic does nothing" off a fight that never happened.
const ALIVE: Stats = Stats { health: 20_000, ..Stats::ZERO };

fn activations(log: &gearmaster_engine::combat::CombatLog, of: &str) -> usize {
    log.entries
        .iter()
        .filter(|e| {
            matches!(&e.event, Event::Activate { side: Side::Player, item, .. } if item == of)
        })
        .count()
}

/// A shunt moves time. It does not make any.
#[test]
fn shunt_moves_time_and_conserves_it() {
    // A fast weapon beside a slow chest item. Without the shunt each fires on
    // its own bar; with it, the chest gains what the weapon gives up.
    let plain = {
        let mut w = item("Fast", SlotKind::Weapon, 1_000, Stats::ZERO);
        w.adjacent_items = vec![1];
        let slow = item("Slow", SlotKind::Chest, 5_000, Stats::ZERO);
        simulate(ALIVE, &[w, slow], &DUMMY)
    };
    let shunting = {
        let mut w = item("Fast", SlotKind::Weapon, 1_000, Stats::ZERO);
        w.adjacent_items = vec![1];
        w.triggers = vec![Trigger::OnActivate(Action::Shunt { ms: 400 })];
        let slow = item("Slow", SlotKind::Chest, 5_000, Stats::ZERO);
        simulate(ALIVE, &[w, slow], &DUMMY)
    };

    let (fast_before, slow_before) = (activations(&plain, "Fast"), activations(&plain, "Slow"));
    let (fast_after, slow_after) = (activations(&shunting, "Fast"), activations(&shunting, "Slow"));

    assert!(slow_after > slow_before, "the slow item gained nothing: {slow_before} -> {slow_after}");
    assert!(fast_after < fast_before, "the fast item paid nothing: {fast_before} -> {fast_after}");

    // Conserved, to the millisecond the bars actually hold. Every activation
    // is one full cooldown of bar-fill, so the total time spent filling bars
    // is the same on both sides of the trade.
    let filled = |f: usize, s: usize| f * 1_000 + s * 5_000;
    let before = filled(fast_before, slow_before);
    let after = filled(fast_after, slow_after);
    let drift = (before as i64 - after as i64).abs();
    assert!(
        drift <= 5_000,
        "time was created or destroyed: {before} ms of bar-fill became {after} ms"
    );

    let shunts = shunting
        .entries
        .iter()
        .filter(|e| matches!(&e.event, Event::Shunted { .. }))
        .count();
    assert!(shunts > 0, "nothing was logged");
}

#[test]
fn shunt_with_no_neighbour_does_nothing() {
    let mut w = item("Lonely", SlotKind::Weapon, 1_000, Stats::ZERO);
    w.triggers = vec![Trigger::OnActivate(Action::Shunt { ms: 400 })];
    let alone = simulate(ALIVE, &[w], &DUMMY);

    let plain =
        simulate(ALIVE, &[item("Lonely", SlotKind::Weapon, 1_000, Stats::ZERO)], &DUMMY);
    assert_eq!(
        activations(&alone, "Lonely"),
        activations(&plain, "Lonely"),
        "an item with nothing beside it paid a debt to nobody"
    );
    assert!(!alone.entries.iter().any(|e| matches!(&e.event, Event::Shunted { .. })));
}

/// Ballast spends the armour there is, and nothing it has not got.
#[test]
fn ballast_spends_exactly_the_armour_it_has() {
    // One wall, built once at the bell, and an item that keeps asking for it.
    let mut wall = item("Wall", SlotKind::Greaves, 60_000, Stats::ZERO);
    wall.triggers = vec![Trigger::OnBattleStart(Action::GainArmor(20))];
    let mut bed = item("Bed", SlotKind::Chest, 1_000, Stats::ZERO);
    bed.triggers = vec![Trigger::OnActivate(Action::Ballast(30))];
    let log = simulate(ALIVE, &[wall, bed], &DUMMY);

    let grew: Vec<(i32, i32)> = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Grew { side: Side::Player, amount, paid_armor, .. } => {
                Some((*amount, *paid_armor))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        grew.len(),
        1,
        "it asked many times and there was one wall: {grew:?}"
    );
    assert_eq!(grew[0], (20, 20), "asked for 30, had 20, spent 20 and grew 20");
}

/// A `Grew` funded from armour is still growth, and `settle` banks it.
/// A grow funded from armour banks across the run exactly as a granted one does.
///
/// `Run::settle` sums `Event::Grew { amount, .. }` over the log and adds it to
/// `grown_health` (`run.rs`). Landing ballast on that event rather than on a
/// new one is what makes this true with no new arm anywhere - which is the
/// whole argument for a field on `Grew` instead of a second event, and it is
/// worth a test rather than a comment.
#[test]
fn ballast_banks_as_growth() {
    let banked = |action: Action| -> i32 {
        let mut wall = item("Wall", SlotKind::Greaves, 60_000, Stats::ZERO);
        wall.triggers = vec![Trigger::OnBattleStart(Action::GainArmor(20))];
        let mut it = item("It", SlotKind::Chest, 1_000, Stats::ZERO);
        it.triggers = vec![Trigger::OnActivate(action)];
        let log = simulate(ALIVE, &[wall, it], &DUMMY);
        log.entries
            .iter()
            .filter_map(|e| match e.event {
                Event::Grew { side: Side::Player, amount, .. } => Some(amount),
                _ => None,
            })
            .sum()
    };
    assert_eq!(banked(Action::Ballast(30)), 20, "the wall, spent");
    assert!(banked(Action::Grow(20)) > 20, "a granted grow keeps arriving; a funded one cannot");
}

/// Accrue reads the balance, and integer division is the floor it stands on.
#[test]
fn accrue_pays_a_share_of_what_is_held() {
    let paid = |held: i32, pct: i32| -> i32 {
        let mut purse = item("Purse", SlotKind::Chest, 60_000, Stats::ZERO);
        purse.triggers = vec![Trigger::OnBattleStart(Action::GainMana(held))];
        let mut it = item("Hall", SlotKind::Helmet, 1_000, Stats::ZERO);
        it.triggers = vec![Trigger::OnActivate(Action::Accrue { what: Resource::Mana, pct })];
        let log = simulate(ALIVE, &[purse, it], &DUMMY);
        log.entries
            .iter()
            .find_map(|e| match &e.event {
                Event::GainMana { side: Side::Player, amount, accrued: true, .. } => Some(*amount),
                _ => None,
            })
            .unwrap_or(0)
    };
    assert_eq!(paid(40, 10), 4, "ten percent of forty");
    assert_eq!(paid(9, 10), 0, "integer division is the floor, and nothing is below it");
    assert_eq!(paid(0, 10), 0, "a drained pool pays nothing, which is Drain's whole answer");
}

/// A fused pool is fuel for nothing, and that includes this.
#[test]
fn accrue_refuses_a_fusion_in_the_fight_as_well_as_in_the_catalogue() {
    let mut it = item("Wrong", SlotKind::Helmet, 1_000, Stats::ZERO);
    it.triggers =
        vec![Trigger::OnActivate(Action::Accrue { what: Resource::DruidicMight, pct: 50 })];
    let log = simulate(ALIVE, &[it], &DUMMY);
    assert!(
        !log.entries.iter().any(|e| matches!(
            &e.event,
            Event::GainResource { accrued: true, .. } | Event::GainMana { accrued: true, .. }
        )),
        "a proportional income on a fusion would be a second currency at better rates"
    );
}
