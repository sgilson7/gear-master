//! Positional effects: components that change what their neighbours are
//! worth, or that are worth more for the empty space around them.

mod common;

use common::{equip, piece};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;

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
#[test]
fn a_spell_cast_without_mana_still_lands_but_weakly() {
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
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
    let log = simulate(stats, &profiles, &LADDER[30]);
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
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
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
    let log = simulate(stats, &profiles, &LADDER[30]);
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
