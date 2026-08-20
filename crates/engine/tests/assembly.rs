//! Placement and assembly rules: recipes, the touching requirement, several
//! finished items per slot, and adjacency bonuses firing only on success.

mod common;

use common::{build_full_loadout, equip, piece};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;
use gearmaster_engine::slot::PlaceError;
use gearmaster_engine::stats::Stats;

// ------------------------------------------------------------- placement

#[test]
fn a_piece_only_goes_in_its_own_slot() {
    let mut run = Run::with_all_pieces();
    let blade = piece(&run, "Iron Blade");

    let err = run.equip(blade, SlotKind::Helmet, 0, 0).unwrap_err();
    assert_eq!(err.to_string(), PlaceError::WrongSlot.to_string());
    assert!(!run.is_equipped(blade), "a rejected equip must not place it");
}

#[test]
fn a_shape_may_not_hang_off_the_edge() {
    let run = Run::with_all_pieces();
    let base = piece(&run, "Padded Base"); // 4 wide, 3 tall, in a 6x8 slot

    assert!(run.can_equip(base, SlotKind::Chest, 2, 5).is_ok(), "fits at the far corner");
    assert_eq!(
        run.can_equip(base, SlotKind::Chest, 3, 0).unwrap_err().to_string(),
        PlaceError::OutOfBounds.to_string(),
        "one column too far right"
    );
}

#[test]
fn pieces_may_not_overlap() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0); // occupies (0, 0..3)
    let blade = piece(&run, "Iron Blade");

    assert_eq!(
        run.can_equip(blade, SlotKind::Weapon, 0, 2).unwrap_err().to_string(),
        PlaceError::Occupied.to_string()
    );
    assert!(run.can_equip(blade, SlotKind::Weapon, 1, 0).is_ok(), "the next column is free");
}

#[test]
fn equipping_removes_a_piece_from_the_inventory() {
    let mut run = Run::with_all_pieces();
    let before = run.inventory().len();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);

    assert_eq!(run.inventory().len(), before - 1);
    assert_eq!(run.loadout.slot_holding(piece(&run, "Balanced Grip")), Some(SlotKind::Weapon));
}

#[test]
fn unequipping_returns_a_piece_to_the_inventory() {
    let mut run = Run::with_all_pieces();
    let grip = piece(&run, "Balanced Grip");
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);

    run.unequip(grip).expect("equipped, so it can come off");

    assert!(!run.is_equipped(grip));
    assert!(run.inventory().contains(&grip));
    assert_eq!(run.inventory().len(), run.owned.len());
}

#[test]
fn moving_a_piece_within_its_slot_does_not_collide_with_itself() {
    let mut run = Run::with_all_pieces();
    let grip = piece(&run, "Balanced Grip");
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0); // (0, 0..3)

    // Shift down one row — the new footprint overlaps the old one.
    run.equip(grip, SlotKind::Weapon, 0, 1).expect("a piece never blocks itself");

    assert_eq!(run.loadout.slot(SlotKind::Weapon).anchor_of(grip), Some((0, 1)));
    assert_eq!(run.loadout.slot(SlotKind::Weapon).get(0, 0), None, "old cell released");
}

// -------------------------------------------------------------- recipes

#[test]
fn an_empty_slot_holds_no_items() {
    let run = Run::with_all_pieces();
    for slot in SlotKind::ALL {
        let r = run.report(slot);
        assert!(r.is_empty(), "{} should start empty", slot.name());
        assert_eq!(r.summary(), "empty");
        assert_eq!(r.stats, Stats::ZERO);
    }
}

#[test]
fn a_weapon_needs_a_damaging_piece_as_well_as_a_handle() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);

    let r = run.report(SlotKind::Weapon);
    assert_eq!(r.assembled_count(), 0);
    assert_eq!(r.items[0].status, "needs 1 more damaging");
}

#[test]
fn a_weapon_assembles_from_a_handle_and_a_blade() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);

    let r = run.report(SlotKind::Weapon);
    assert_eq!(r.assembled_count(), 1, "{}", r.summary());
    assert_eq!(r.summary(), "1 item assembled");
}

#[test]
fn components_that_do_not_touch_are_judged_as_separate_items() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0); // column 0
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 3, 0); // column 3 — a gap between

    let r = run.report(SlotKind::Weapon);
    assert_eq!(r.items.len(), 2, "two groups, not one weapon");
    assert_eq!(r.assembled_count(), 0);
    // Each half complains about what it is missing on its own.
    let statuses: Vec<&str> = r.items.iter().map(|i| i.status.as_str()).collect();
    assert!(statuses.contains(&"needs 1 more damaging"), "{:?}", statuses);
    assert!(statuses.contains(&"needs 1 more handle"), "{:?}", statuses);
}

#[test]
fn too_many_components_of_one_kind_in_a_single_item_is_rejected() {
    let mut run = Run::with_all_pieces();
    // One base with four layers glued to it: one layer over the maximum.
    equip(&mut run, "Padded Base", SlotKind::Chest, 0, 0); // (0..3, 0..2)
    equip(&mut run, "Chain Layer", SlotKind::Chest, 0, 3);
    equip(&mut run, "Plate Layer", SlotKind::Chest, 0, 4);
    equip(&mut run, "Woven Underlayer", SlotKind::Chest, 0, 5);
    assert_eq!(run.report(SlotKind::Chest).assembled_count(), 1, "three layers is the max");

    equip(&mut run, "Hollow Weave", SlotKind::Chest, 0, 6);

    let r = run.report(SlotKind::Chest);
    assert_eq!(r.items.len(), 1, "all five are touching, so it is one item");
    assert_eq!(r.items[0].status, "too many layer (max 3)");
    assert_eq!(r.assembled_count(), 0);
}

// -------------------------------------------------- several items a slot

#[test]
fn one_slot_can_hold_two_finished_items() {
    let mut run = Run::with_all_pieces();
    // Two complete gloves, kept apart by empty rows 2 and 3.
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0); // (0..1, 0..1)
    equip(&mut run, "Gripping Mold", SlotKind::Gloves, 2, 0); // (2..3, 0), (2, 1)
    equip(&mut run, "Steel Material", SlotKind::Gloves, 0, 4); // (0..1, 4..6)
    equip(&mut run, "Gauntlet Mold", SlotKind::Gloves, 2, 4); // (2, 4..6), (3, 6)

    let r = run.report(SlotKind::Gloves);
    assert_eq!(r.items.len(), 2);
    assert_eq!(r.assembled_count(), 2, "{}", r.summary());
    assert_eq!(r.summary(), "2 items assembled");
    // Both items' stats count: 2 + 15x power, then 5 hp + 4 + 1 str + 2 bonus.
    assert_eq!((r.stats.health, r.stats.strength, r.stats.power), (5, 9, 15));
}

#[test]
fn two_items_may_sit_flush_against_each_other() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0); // (0..1, 0..1)
    equip(&mut run, "Gripping Mold", SlotKind::Gloves, 2, 0); // touches the leather
    // Butted straight up against the first glove, with no gap at all.
    equip(&mut run, "Steel Material", SlotKind::Gloves, 0, 2); // (0..1, 2..4)

    let r = run.report(SlotKind::Gloves);
    // Two materials means two cores, so two items — even though every piece
    // here is one connected lump.
    assert_eq!(r.items.len(), 2, "each core anchors its own item");
    assert_eq!(r.assembled_count(), 1, "leather + mold is a finished glove");
    assert_eq!(r.loose_count(), 1, "the steel material still wants a mold");
}

#[test]
fn a_loose_piece_joins_whichever_core_it_is_nearest() {
    let mut run = Run::with_all_pieces();
    // Two handles in a row with a single blade hanging off the second one.
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0); // (0, 0..2)
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 1, 0); // (1, 0..3)
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 2, 0); // (2, 0..3), touches the grip

    let r = run.report(SlotKind::Weapon);
    assert_eq!(r.items.len(), 2, "two handles, two weapons");

    let grip = piece(&run, "Balanced Grip");
    let blade = piece(&run, "Iron Blade");
    let with_grip = r.items.iter().find(|i| i.pieces.contains(&grip)).unwrap();
    assert!(
        with_grip.pieces.contains(&blade),
        "the blade belongs to the handle it actually touches"
    );
    assert!(with_grip.assembled, "handle + blade is a weapon");

    let oak = piece(&run, "Oak Handle");
    let lonely = r.items.iter().find(|i| i.pieces.contains(&oak)).unwrap();
    assert!(!lonely.assembled);
    assert_eq!(lonely.status, "needs 1 more damaging");
}

#[test]
fn a_blob_with_no_core_at_all_is_one_unfinished_item() {
    let mut run = Run::with_all_pieces();
    // Two layers touching, and not a base between them.
    equip(&mut run, "Chain Layer", SlotKind::Chest, 0, 0);
    equip(&mut run, "Plate Layer", SlotKind::Chest, 0, 1);

    let r = run.report(SlotKind::Chest);
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].status, "needs 1 more base");
}

#[test]
fn a_slot_can_hold_a_finished_item_and_loose_pieces_at_once() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Gripping Mold", SlotKind::Gloves, 2, 0);
    equip(&mut run, "Steel Material", SlotKind::Gloves, 0, 4); // no mold to pair with

    let r = run.report(SlotKind::Gloves);
    assert_eq!(r.assembled_count(), 1);
    assert_eq!(r.loose_count(), 1);
    assert_eq!(r.summary(), "1 assembled, 1 loose");
    // The loose material still contributes its base stats.
    assert_eq!((r.stats.health, r.stats.strength, r.stats.power), (5, 6, 15));
}

#[test]
fn every_slot_assembles_on_the_preset_loadout() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);

    for slot in SlotKind::ALL {
        let r = run.report(slot);
        assert!(
            r.assembled_count() >= 1,
            "{} failed to assemble: {}",
            slot.name(),
            r.summary()
        );
        assert_eq!(r.loose_count(), 0, "{} left loose pieces", slot.name());
    }
    // Chest, gloves and greaves each carry two separate items.
    assert_eq!(run.report(SlotKind::Chest).assembled_count(), 2);
    assert_eq!(run.report(SlotKind::Gloves).assembled_count(), 2);
    assert_eq!(run.report(SlotKind::Greaves).assembled_count(), 2);
}

// ---------------------------------------------------- adjacency bonuses

#[test]
fn an_adjacency_bonus_stays_dormant_until_the_item_assembles() {
    let mut run = Run::with_all_pieces();
    // Runed Material alone: base +5 health, and its +15 bonus must NOT fire.
    equip(&mut run, "Runed Material", SlotKind::Greaves, 0, 0);

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 0);
    assert_eq!(r.stats.health, 5, "only the base contribution");
    assert!(r.notes().is_empty());

    // Add the mold next to it and the greaves come together.
    equip(&mut run, "Greave Mold", SlotKind::Greaves, 2, 0);

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 1, "{}", r.summary());
    assert_eq!((r.stats.health, r.stats.regen), (20, 1), "base 5 + bonus 15 health, +1 regen");
    assert_eq!(r.notes(), vec!["Runed: +15 health"]);
}

#[test]
fn breaking_the_assembly_switches_the_bonus_back_off() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Runed Material", SlotKind::Greaves, 0, 0);
    equip(&mut run, "Greave Mold", SlotKind::Greaves, 2, 0);
    assert_eq!(run.report(SlotKind::Greaves).stats.health, 20);

    // Slide the mold away so nothing touches any more.
    let mold = piece(&run, "Greave Mold");
    run.equip(mold, SlotKind::Greaves, 4, 4).expect("legal placement");

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 0);
    assert_eq!(r.stats.health, 5, "the +15 bonus is withdrawn");
    assert!(r.notes().is_empty());
}

#[test]
fn each_slots_bonus_fires_exactly_once_on_the_preset() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);

    let notes: Vec<String> = run.reports().iter().flat_map(|r| r.notes()).collect();
    for label in [
        "Focused: +3 strength",
        "Woven: +2 regen",
        "Gauntleted: +2 strength",
        "Runed: +15 health",
        "Balanced: +0.50x weapon power",
    ] {
        assert_eq!(
            notes.iter().filter(|n| n.as_str() == label).count(),
            1,
            "expected {:?} exactly once in {:?}",
            label,
            notes
        );
    }
}

// ------------------------------------------------------------- rotation

#[test]
fn rotating_an_equipped_piece_that_no_longer_fits_changes_nothing() {
    let mut run = Run::with_all_pieces();
    let base = piece(&run, "Padded Base"); // 4 wide x 3 tall
    equip(&mut run, "Padded Base", SlotKind::Chest, 2, 0); // occupies x 2..5

    // Rotated it is 3 wide x 4 tall — still fine — so confirm the legal case
    // first, then wedge it where the turn cannot happen.
    run.rotate(base).expect("3x4 fits at x=2");
    assert_eq!(run.registry.rotation(base), 1);

    run.equip(base, SlotKind::Chest, 3, 4).expect("3x4 fits at (3, 4)");
    let err = run.rotate(base).unwrap_err();

    assert_eq!(err.to_string(), PlaceError::OutOfBounds.to_string());
    assert_eq!(run.registry.rotation(base), 1, "rotation rolled back");
    assert_eq!(
        run.loadout.slot(SlotKind::Chest).anchor_of(base),
        Some((3, 4)),
        "and the piece stayed put"
    );
}

#[test]
fn rotating_a_piece_in_the_inventory_always_works() {
    let mut run = Run::with_all_pieces();
    let mold = piece(&run, "Gauntlet Mold");
    let before = run.registry.shape(mold);

    run.rotate(mold).expect("nothing constrains an unequipped piece");

    assert_ne!(run.registry.shape(mold), before);
}

// ------------------------------------------------------------------ art

// The GUI draws each finished item from `sigil_seed`, so the emblem is only
// meaningful if the seed behaves the same way the generated name does: stable
// for a given build, different for a different one.

#[test]
fn an_items_emblem_seed_is_stable_for_the_same_build() {
    let mut a = Run::with_all_pieces();
    equip(&mut a, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut a, "Iron Blade", SlotKind::Weapon, 1, 0);

    let mut b = Run::with_all_pieces();
    equip(&mut b, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut b, "Iron Blade", SlotKind::Weapon, 1, 0);

    let (pa, pb) = (a.combat_items(), b.combat_items());
    assert_eq!(pa.len(), 1);
    assert_eq!(pa[0].sigil_seed, pb[0].sigil_seed);
    assert_eq!(pa[0].name, pb[0].name, "and it agrees with the name");
}

#[test]
fn moving_a_piece_redraws_the_emblem() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    let blade = piece(&run, "Iron Blade");
    run.equip(blade, SlotKind::Weapon, 1, 0).unwrap();
    let before = run.combat_items()[0].sigil_seed;

    run.equip(blade, SlotKind::Weapon, 1, 1).unwrap();
    let after = run.combat_items()[0].sigil_seed;

    assert_ne!(before, after, "a different placement is a different item");
}

#[test]
fn different_items_get_different_emblems() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);

    let mut seeds = std::collections::HashSet::new();
    for p in run.combat_items() {
        assert!(seeds.insert(p.sigil_seed), "{} reused an emblem seed", p.name);
    }
    assert!(seeds.len() >= 5, "a full loadout should assemble several items");
}

// ----------------------------------------------------------------- undo

#[test]
fn undo_puts_a_piece_back_where_it_was() {
    let mut run = Run::with_all_pieces();
    let handle = piece(&run, "Oak Handle");
    run.equip(handle, SlotKind::Weapon, 0, 0).unwrap();
    run.equip(handle, SlotKind::Weapon, 3, 2).unwrap();

    assert_eq!(run.loadout.slot(SlotKind::Weapon).anchor_of(handle), Some((3, 2)));
    assert!(run.undo().is_some());
    assert_eq!(
        run.loadout.slot(SlotKind::Weapon).anchor_of(handle),
        Some((0, 0)),
        "back to where it was before the move"
    );
    assert!(run.undo().is_some());
    assert!(!run.is_equipped(handle), "and back off the board entirely");
    assert!(run.undo().is_none(), "nothing left to take back");
}

#[test]
fn undo_restores_a_rotation() {
    let mut run = Run::with_all_pieces();
    let mold = piece(&run, "Gauntlet Mold");
    let before = run.registry.shape(mold);

    run.rotate(mold).unwrap();
    assert_ne!(run.registry.shape(mold), before);

    run.undo();
    assert_eq!(run.registry.shape(mold), before, "rotations live on the registry too");
}

#[test]
fn a_refused_rotation_leaves_nothing_to_undo() {
    let mut run = Run::with_all_pieces();
    let base = piece(&run, "Padded Base"); // 4 wide x 3 tall
    run.equip(base, SlotKind::Chest, 2, 0).unwrap();
    run.rotate(base).expect("3x4 still fits at x=2");
    run.equip(base, SlotKind::Chest, 3, 4).expect("3x4 fits at (3, 4)");
    // Now wedged: turning back to 4x3 would hang off the right edge.
    let depth_before = run.undoable().map(|s| s.to_string());

    assert!(run.rotate(base).is_err());

    assert_eq!(
        run.undoable().map(|s| s.to_string()),
        depth_before,
        "a rotation that could not happen must not push history"
    );
}

#[test]
fn undo_takes_back_a_clear_all() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);
    let before: Vec<usize> =
        SlotKind::ALL.iter().map(|&k| run.loadout.slot(k).pieces().len()).collect();
    assert!(before.iter().sum::<usize>() > 0);

    run.clear_all();
    assert!(SlotKind::ALL.iter().all(|&k| run.loadout.slot(k).is_empty()));

    run.undo();
    let after: Vec<usize> =
        SlotKind::ALL.iter().map(|&k| run.loadout.slot(k).pieces().len()).collect();
    assert_eq!(after, before, "the whole board comes back");
}

#[test]
fn undo_does_not_hand_gold_back() {
    // Undo is for "wrong square", not for unwinding a purchase. A board step
    // that also moved money would let you rebuild your purse by tapping it.
    let mut run = Run::new();
    run.gold = 400; // the strong shelves are expensive now
    let gold_before = run.gold;
    let id = run.buy(0).expect("affordable");
    let spent = gold_before - run.gold;
    assert!(spent > 0);

    run.equip(id, run.registry.def(id).slot, 0, 0).unwrap();
    run.undo();

    assert_eq!(run.gold, gold_before - spent, "the purchase stands");
    assert!(run.owned.contains(&id));
}

#[test]
fn starting_a_fight_drops_the_history() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.equip(piece(&run, "Oak Handle"), SlotKind::Weapon, 5, 7).ok();
    run.begin_fight();
    assert!(run.undoable().is_none(), "the board it described is gone");
}

// ---------------------------------------------------------------- spells

#[test]
fn a_book_an_ink_and_a_spell_make_a_weapon() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Pocket Grimoire", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 2, 0);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());
}

#[test]
fn a_martial_weapon_still_assembles_alongside_the_new_recipes() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);
}

#[test]
fn a_book_will_not_take_a_second_spell_but_an_orb_wants_one() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Pocket Grimoire", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 2, 0);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 2, 2);
    assert_eq!(
        run.report(SlotKind::Weapon).assembled_count(),
        0,
        "a book binds one spell, not two"
    );

    // The same parts around an orb are exactly what it asks for.
    let mut orb = Run::with_all_pieces();
    equip(&mut orb, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut orb, "Soot Ink", SlotKind::Weapon, 0, 3);
    equip(&mut orb, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut orb, "Rime Nova", SlotKind::Weapon, 4, 0);
    let report = orb.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());
}

#[test]
fn ink_scales_its_own_cast_and_nobody_elses() {
    use gearmaster_engine::stats::Stats;
    let mut run = Run::with_all_pieces();
    // A spell with strong ink, and a plain martial weapon beside it.
    equip(&mut run, "Leaden Tome", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Bloodletter's Ink", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);

    let items = run.combat_items();
    let spell = items.iter().find(|i| i.power_bonus > 0).expect("the spell is here");
    assert!(spell.power_bonus >= 240, "ink and book both add to it: {}", spell.power_bonus);

    // The wearer's own power is untouched by ink.
    let base = Stats::base_character().power;
    assert_eq!(run.player_stats().power, base, "ink never reaches the wearer");
}

#[test]
fn an_orb_casts_a_different_spell_each_time() {
    use gearmaster_engine::combat::{simulate, Event, Side, LADDER};
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 0, 3);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 4, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);

    let profiles = run.combat_items();
    let orb = profiles.iter().find(|p| p.casts.len() > 1).expect("an orb holds several");
    assert_eq!(orb.casts.len(), 2);

    // Over a long fight the log should name both spells. The player is given
    // enough health to survive one, since the point is the orb's rotation.
    let mut stats = run.player_stats();
    stats.health = 100_000;
    let log = simulate(stats, &profiles, &LADDER[LADDER.len() - 1]);
    let mut named: Vec<String> = Vec::new();
    for entry in &log.entries {
        if let Event::Activate { side: Side::Player, item, .. } = &entry.event {
            if item.contains('(') && !named.contains(item) {
                named.push(item.clone());
            }
        }
    }
    assert!(named.len() >= 2, "the orb should cycle its spells, saw {:?}", named);
}
