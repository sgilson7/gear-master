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
    // Runed Material alone: base +25 health, and its +75 bonus must NOT fire.
    equip(&mut run, "Runed Material", SlotKind::Greaves, 0, 0);

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 0);
    assert_eq!(r.stats.health, 25, "only the base contribution");
    assert!(r.notes().is_empty());

    // Add the mold next to it and the greaves come together.
    equip(&mut run, "Greave Mold", SlotKind::Greaves, 2, 0);

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 1, "{}", r.summary());
    assert_eq!((r.stats.health, r.stats.regen), (100, 1), "base 25 + bonus 75 health, +1 regen");
    assert_eq!(r.notes(), vec!["Runed: +75 health"]);
}

#[test]
fn breaking_the_assembly_switches_the_bonus_back_off() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Runed Material", SlotKind::Greaves, 0, 0);
    equip(&mut run, "Greave Mold", SlotKind::Greaves, 2, 0);
    assert_eq!(run.report(SlotKind::Greaves).stats.health, 100);

    // Slide the mold away so nothing touches any more.
    let mold = piece(&run, "Greave Mold");
    run.equip(mold, SlotKind::Greaves, 4, 4).expect("legal placement");

    let r = run.report(SlotKind::Greaves);
    assert_eq!(r.assembled_count(), 0);
    assert_eq!(r.stats.health, 25, "the +75 bonus is withdrawn");
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
        "Runed: +75 health",
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

    // Two spells around an orb are exactly what it asks for - and no ink,
    // which an orb has not wanted since alignments took over that job.
    let mut orb = Run::with_all_pieces();
    equip(&mut orb, "Scrying Orb", SlotKind::Weapon, 0, 0);
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

// ---------------------------------------------------------------- quests

/// Sit a Helm of Blades on rows a bladed weapon also occupies, and fight until
/// its tally is met.
fn blades_run() -> Run {
    let mut run = Run::with_all_pieces();
    // A weapon built with an Iron Blade, on rows 0-2.
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    // The helm, on the same rows in another slot: that is what "aligned" is.
    equip(&mut run, "Helm of Blades", SlotKind::Helmet, 0, 0);
    equip(&mut run, "Warding Plate", SlotKind::Helmet, 0, 2);
    run
}

#[test]
fn a_quest_only_counts_while_its_item_is_assembled() {
    let mut run = blades_run();
    let helm = piece(&run, "Helm of Blades");
    assert_eq!(run.report(SlotKind::Helmet).assembled_count(), 1);

    run.fight_next();
    run.settle();
    let progressed = run.quest_progress(helm);
    assert!(progressed > 0, "an assembled helm should have counted something");

    // Break the helm apart and the tally stops moving.
    let mut loose = blades_run();
    let loose_helm = piece(&loose, "Helm of Blades");
    loose.unequip(piece(&loose, "Warding Plate")).unwrap();
    assert_eq!(loose.report(SlotKind::Helmet).assembled_count(), 0);
    loose.fight_next();
    loose.settle();
    assert_eq!(loose.quest_progress(loose_helm), 0, "a loose piece is inert, quests included");
}

#[test]
fn the_helm_of_blades_becomes_the_blade_of_helms() {
    let mut run = blades_run();
    let helm = piece(&run, "Helm of Blades");
    assert_eq!(run.registry.def(helm).name, "Helm of Blades");

    // Fight until the tally is met. The rat is easy, so this loops.
    for _ in 0..12 {
        if run.registry.def(helm).name != "Helm of Blades" {
            break;
        }
        run.rung = 0;
        run.fight_next();
        run.settle();
        run.back_to_loadout();
    }

    let now = run.registry.def(helm);
    assert_eq!(now.name, "Blade of Helms", "the quest should have come good");
    assert_eq!(now.slot, SlotKind::Weapon, "and it is not a helmet any more");
    assert_eq!(now.kind, gearmaster_engine::piece::PieceKind::Damaging);

    // It could not stay on a helmet board, so it went back to the inventory.
    assert!(!run.is_equipped(helm), "a transformed piece comes off the board");
    assert!(run.inventory().contains(&helm));
}

#[test]
fn a_finished_quest_is_reported_once() {
    let mut run = blades_run();
    let mut announced = 0;
    for _ in 0..12 {
        run.rung = 0;
        run.fight_next();
        run.settle();
        announced += run.last_settlement.as_ref().unwrap().quests_done.len();
        run.back_to_loadout();
    }
    assert_eq!(announced, 1, "the transformation is announced exactly once");
}

#[test]
fn the_blade_of_helms_gives_armour_where_a_damaging_piece_gives_damage() {
    use gearmaster_engine::piece::CATALOG;
    let d = CATALOG.iter().find(|d| d.name == "Blade of Helms").expect("it exists");
    assert_eq!(d.base.physical_damage, 0, "it is a damaging piece that deals none");
    assert!(
        d.triggers.iter().any(|t| matches!(
            t,
            gearmaster_engine::piece::Trigger::OnActivate(
                gearmaster_engine::piece::Action::GainArmor(_)
            )
        )),
        "it should hand out armour instead"
    );
}

#[test]
fn every_quest_names_a_component_that_exists() {
    use gearmaster_engine::piece::CATALOG;
    let mut with_quests = 0;
    for d in CATALOG {
        if let Some(q) = d.quest {
            with_quests += 1;
            assert!(
                CATALOG.iter().any(|t| t.name == q.becomes),
                "{} finishes into {}, which is not a component",
                d.name,
                q.becomes
            );
            assert!(q.goal > 0, "{} has a quest with no goal", d.name);
            assert!(
                CATALOG.iter().find(|t| t.name == q.becomes).unwrap().quest.is_none(),
                "{} finishes into something that is itself a quest, which would chain",
                d.name
            );
        }
    }
    assert!(with_quests >= 5, "only {} components carry quests", with_quests);
}

// ------------------------------------------------- shared pools and rings

#[test]
fn a_material_goes_on_a_hand_or_a_foot_alike() {
    use gearmaster_engine::piece::CATALOG;
    let mat = CATALOG.iter().find(|d| d.name == "Steel Material").expect("it exists");
    assert!(mat.fits(SlotKind::Gloves));
    assert!(mat.fits(SlotKind::Greaves));
    assert!(!mat.fits(SlotKind::Helmet), "but not on a head");

    // And it really places in the grid it was not declared for.
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Steel Material", SlotKind::Greaves, 0, 0);
    equip(&mut run, "Runner's Mold", SlotKind::Greaves, 2, 0);
    assert_eq!(run.report(SlotKind::Greaves).assembled_count(), 1);
}

#[test]
fn plating_covers_a_head_or_a_shin() {
    use gearmaster_engine::piece::CATALOG;
    let plate = CATALOG.iter().find(|d| d.name == "Iron Plating").expect("it exists");
    assert!(plate.fits(SlotKind::Helmet) && plate.fits(SlotKind::Greaves));
    assert!(!plate.fits(SlotKind::Chest));
}

#[test]
fn a_glove_takes_two_rings_and_a_greave_takes_none() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Steel Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Gauntlet Mold", SlotKind::Gloves, 2, 0);
    equip(&mut run, "Iron Band", SlotKind::Gloves, 0, 3);
    equip(&mut run, "Bloodring", SlotKind::Gloves, 1, 3);
    let report = run.report(SlotKind::Gloves);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());

    // A third ring is one too many.
    let mut over = Run::with_all_pieces();
    equip(&mut over, "Steel Material", SlotKind::Gloves, 0, 0);
    equip(&mut over, "Gauntlet Mold", SlotKind::Gloves, 2, 0);
    for (i, r) in ["Iron Band", "Bloodring", "Oathring"].iter().enumerate() {
        equip(&mut over, r, SlotKind::Gloves, i as u8, 3);
    }
    assert_eq!(over.report(SlotKind::Gloves).assembled_count(), 0, "three rings is too many");

    // Rings belong on hands only.
    let ring = gearmaster_engine::piece::CATALOG.iter().find(|d| d.name == "Iron Band").unwrap();
    assert!(!ring.fits(SlotKind::Greaves));
}

#[test]
fn a_greave_can_take_a_plate() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Runed Material", SlotKind::Greaves, 0, 0);
    equip(&mut run, "Runner's Mold", SlotKind::Greaves, 2, 0);
    equip(&mut run, "Warding Plate", SlotKind::Greaves, 0, 2);
    let report = run.report(SlotKind::Greaves);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());
}

// ------------------------------------------- spells packed against weapons

#[test]
fn a_spell_and_a_weapon_can_sit_flush_without_confusing_each_other() {
    // The whole point of the spell recipes: books and orbs anchor items of
    // their own, so a spell can be packed hard against a martial weapon and
    // neither steals the other's parts. Three damaging pieces in one grid
    // would be illegal in one weapon - split across two items it is fine.
    let mut run = Run::with_all_pieces();
    // A martial weapon on the left.
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    // A spell immediately beside it, touching.
    equip(&mut run, "Pocket Grimoire", SlotKind::Weapon, 2, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(
        report.assembled_count(),
        2,
        "a weapon and a spell, side by side: {}",
        report.summary()
    );
}

#[test]
fn two_spell_cores_can_be_neighbours() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Pocket Grimoire", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 0, 2);
    equip(&mut run, "Apprentice's Primer", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Prismatic Ink", SlotKind::Weapon, 3, 2);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 3, 3);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 2, "two books, two spells: {}", report.summary());
}

#[test]
fn packing_a_spell_beside_a_weapon_beats_leaving_the_room_empty() {
    // The claim the spell system rests on: access to it lets you fit more
    // into one grid than the martial recipe alone allows.
    let mut martial = Run::with_all_pieces();
    equip(&mut martial, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut martial, "Iron Blade", SlotKind::Weapon, 1, 0);
    let alone = martial.report(SlotKind::Weapon).stats;

    let mut both = Run::with_all_pieces();
    equip(&mut both, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut both, "Iron Blade", SlotKind::Weapon, 1, 0);
    equip(&mut both, "Pocket Grimoire", SlotKind::Weapon, 2, 0);
    equip(&mut both, "Soot Ink", SlotKind::Weapon, 3, 0);
    equip(&mut both, "Emberburst", SlotKind::Weapon, 3, 1);
    let packed = both.report(SlotKind::Weapon);

    assert_eq!(packed.assembled_count(), 2);
    assert!(
        packed.stats.magic_damage > alone.magic_damage,
        "the spell should be adding a payload the weapon alone had no room for: {:?} vs {:?}",
        packed.stats.magic_damage,
        alone.magic_damage
    );
}

// -------------------------------------------------------- locked items

/// A weapon and a spell packed flush, which is exactly the case where the
/// split can be argued with.
fn packed_pair() -> Run {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Pocket Grimoire", SlotKind::Weapon, 2, 0);
    equip(&mut run, "Soot Ink", SlotKind::Weapon, 3, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 3, 1);
    run
}

#[test]
fn locking_an_item_stops_it_negotiating_with_its_neighbours() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 2);

    assert!(run.toggle_lock_item(handle), "an assembled item can be locked");
    let set = run.locked_set(handle).expect("it is locked").to_vec();
    assert!(set.contains(&piece(&run, "Iron Blade")), "the lock holds the whole item");
    assert_eq!(set.len(), 2);

    // Still two items, and the locked one is unchanged.
    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 2, "{}", report.summary());

    assert!(!run.toggle_lock_item(handle), "and it can be released again");
    assert!(run.locked_set(handle).is_none());
}

#[test]
fn a_locked_item_will_not_absorb_a_piece_dropped_beside_it() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    run.toggle_lock_item(handle);
    let locked = run.locked_set(handle).unwrap().to_vec();

    // Drop another damaging piece against the locked weapon.
    equip(&mut run, "Serrated Edge", SlotKind::Weapon, 0, 4);

    assert_eq!(
        run.locked_set(handle).unwrap(),
        locked.as_slice(),
        "the locked item is exactly what it was"
    );
    let report = run.report(SlotKind::Weapon);
    assert!(
        report.items.iter().any(|i| i.pieces == locked && i.assembled),
        "and it is still assembled: {}",
        report.summary()
    );
}

#[test]
fn a_locked_item_turns_as_one_piece() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.toggle_lock_item(handle);

    let before = (
        run.loadout.slot(SlotKind::Weapon).cells_of(handle).len(),
        run.loadout.slot(SlotKind::Weapon).cells_of(blade).len(),
    );
    run.rotate_locked(handle).expect("there is room to turn");

    // Both pieces turned, both are still on the board, and the item is intact.
    let after = (
        run.loadout.slot(SlotKind::Weapon).cells_of(handle).len(),
        run.loadout.slot(SlotKind::Weapon).cells_of(blade).len(),
    );
    assert_eq!(before, after, "no piece lost a cell");
    assert_eq!(run.registry.rotation(handle), 1);
    assert_eq!(run.registry.rotation(blade), 1);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1, "still one item");
}

#[test]
fn a_locked_item_comes_off_the_board_as_one_thing() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    let handle = piece(&run, "Oak Handle");
    run.toggle_lock_item(handle);

    run.unequip_locked(handle).expect("it can come off");
    assert!(!run.is_equipped(handle));
    assert!(!run.is_equipped(piece(&run, "Iron Blade")), "and so did the rest of it");

    // And the inventory carries it as a single entry.
    let groups = run.inventory_groups();
    let together = groups.iter().find(|g| g.contains(&handle)).expect("it is in there");
    assert_eq!(together.len(), 2, "carried as one thing, not two");
}

// ---------------------------------------------------------------- recipe text

/// The split the interface draws: what makes an item work, and what only makes
/// it better. A helmet is finished with a frame and one plating - the second
/// plating and the crest are improvements to gear that already counts.
#[test]
fn a_recipe_separates_what_is_required_from_what_is_extra() {
    use gearmaster_engine::piece::recipe_parts;

    let helm = &recipe_parts(SlotKind::Helmet)[0];
    assert_eq!(helm.required, vec!["1 frame", "1 plating"]);
    assert_eq!(helm.optional, vec!["1 more plating", "1 crest"]);

    // "gloves mold", not "mold": greaves want a mold too and the two do not
    // interchange, so the bare word would invite exactly the wrong guess.
    let gloves = &recipe_parts(SlotKind::Gloves)[0];
    assert_eq!(gloves.required, vec!["1 material", "1 gloves mold"]);
    assert_eq!(gloves.optional, vec!["2 rings"]);

    let greaves = &recipe_parts(SlotKind::Greaves)[0];
    assert_eq!(greaves.required, vec!["1 material", "1 greaves mold"]);
}

/// A role that two slots want but whose pieces do not carry between them is
/// named for its slot; one whose pieces really are shared keeps the bare name.
#[test]
fn only_roles_that_do_not_interchange_are_qualified_by_slot() {
    use gearmaster_engine::piece::PieceKind;

    assert!(PieceKind::Mold.is_slot_specific(), "gloves and greaves molds do not swap");
    assert_eq!(PieceKind::Mold.name_in(SlotKind::Gloves), "gloves mold");
    assert_eq!(PieceKind::Mold.name_in(SlotKind::Greaves), "greaves mold");

    // These do swap, so qualifying them would be the lie.
    for kind in [PieceKind::Material, PieceKind::Plating] {
        assert!(!kind.is_slot_specific(), "{:?} is shared between slots", kind);
        assert_eq!(kind.name_in(SlotKind::Greaves), kind.name());
    }

    // A role only one slot uses needs no qualifying either.
    assert!(!PieceKind::Ring.is_slot_specific());
    assert!(!PieceKind::Crest.is_slot_specific());
}

/// Every way of building a slot is described separately, and each is named for
/// the piece it is built around.
#[test]
fn a_slot_with_several_recipes_describes_each_one() {
    use gearmaster_engine::piece::recipe_parts;

    let ways = recipe_parts(SlotKind::Weapon);
    assert_eq!(ways.len(), 3, "weapon builds three ways");
    let titles: Vec<&str> = ways.iter().map(|w| w.title).collect();
    assert_eq!(titles, vec!["Martial weapon", "Book spell", "Crystal ball"]);

    assert_eq!(ways[1].required, vec!["1 book", "1 ink", "1 spell"]);
    assert_eq!(ways[1].optional, vec!["1 accessory"]);
}

/// The required half is exactly what the assembly rule enforces. If a recipe's
/// minimums change, the text changes with them rather than going stale.
#[test]
fn the_required_half_matches_the_recipe_minimums() {
    use gearmaster_engine::piece::{recipe_parts, recipes};

    for slot in SlotKind::ALL {
        for (r, text) in recipes(slot).iter().zip(recipe_parts(slot)) {
            let wanted = r.iter().filter(|(_, min, _)| *min > 0).count();
            assert_eq!(
                text.required.len(),
                wanted,
                "{:?}: {} required entries for {} non-zero minimums",
                slot,
                text.required.len(),
                wanted
            );
        }
    }
}

/// Counts read as English on both sides of the split.
#[test]
fn recipe_counts_are_pluralised() {
    use gearmaster_engine::piece::recipe_parts;

    let all: Vec<String> = SlotKind::ALL
        .iter()
        .flat_map(|&s| recipe_parts(s))
        .flat_map(|p| p.required.into_iter().chain(p.optional))
        .collect();

    assert!(all.iter().any(|s| s == "2 rings"), "plural noun: {:?}", all);
    assert!(all.iter().any(|s| s == "2 more layers"), "plural after 'more': {:?}", all);
    // Mass nouns stay singular however many there are.
    assert!(all.iter().any(|s| s == "1 more damaging"), "{:?}", all);
    assert!(!all.iter().any(|s| s.contains("platings")), "no 'platings': {:?}", all);
    // "accessory" -> "accessories", not "accessorys".
    assert!(all.iter().any(|s| s == "2 accessories"), "{:?}", all);
    assert!(!all.iter().any(|s| s.contains("accessorys")), "{:?}", all);
}

/// The bug: a locked item's pieces could still be dragged out one at a time,
/// which is exactly what locking is supposed to prevent. The interface now
/// lifts the whole set, and the shape it needs to do that lives on the lock.
#[test]
fn a_locked_item_knows_its_own_shape() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.toggle_lock_item(handle);

    let shape = run.locked_shape(handle).expect("a locked item carries its shape");
    assert_eq!(shape.len(), 2, "both pieces are in it");
    // Oak Handle at (0,0) and Iron Blade at (1,0): offsets relative to the
    // item's own corner, so they survive being put down somewhere else.
    let at = |id| shape.iter().find(|&&(p, ..)| p == id).map(|&(_, x, y)| (x, y));
    assert_eq!(at(handle), Some((0, 0)));
    assert_eq!(at(blade), Some((1, 0)));

    // Any piece of it answers with the same shape - you can grab it anywhere.
    assert_eq!(run.locked_shape(blade), run.locked_shape(handle));
}

/// The other bug: a whole assembled item could not be moved to the inventory
/// and back. It comes off as one thing and goes down as one thing.
#[test]
fn a_locked_item_travels_to_the_inventory_and_back_whole() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.toggle_lock_item(handle);

    run.unequip_locked(handle).expect("it comes off as one piece");
    assert!(run.loadout.slot(SlotKind::Weapon).anchor_of(handle).is_none());
    assert!(run.loadout.slot(SlotKind::Weapon).anchor_of(blade).is_none());
    assert!(run.is_locked_item(handle), "stowing it does not release it");

    // The inventory carries it as one entry, not two.
    let groups = run.inventory_groups();
    let together = groups.iter().find(|g| g.contains(&handle)).expect("it is in the tray");
    assert_eq!(together.len(), 2, "carried as one thing");

    // And it goes back down somewhere else with its shape intact.
    run.equip_locked_at(handle, SlotKind::Weapon, 2, 3).expect("it fits there");
    assert_eq!(run.loadout.slot(SlotKind::Weapon).anchor_of(handle), Some((2, 3)));
    assert_eq!(run.loadout.slot(SlotKind::Weapon).anchor_of(blade), Some((3, 3)));
    assert!(run.report(SlotKind::Weapon).items.iter().any(|i| i.assembled));
}

/// A drop that will not fit must leave the board alone rather than strewing
/// half an item across it.
#[test]
fn a_locked_item_that_does_not_fit_is_refused_whole() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.toggle_lock_item(handle);
    run.unequip_locked(handle).unwrap();

    // Hard against the right edge: the handle would fit, the blade would not.
    let w = gearmaster_engine::slot::SLOT_W - 1;
    assert!(run.equip_locked_at(handle, SlotKind::Weapon, w, 0).is_err());
    assert!(
        run.loadout.slot(SlotKind::Weapon).anchor_of(handle).is_none(),
        "a refused drop places nothing at all"
    );
    assert!(run.loadout.slot(SlotKind::Weapon).anchor_of(blade).is_none());
}

/// Turning a locked item changes its shape, and the stored shape has to follow
/// or it would go back down in its old arrangement.
#[test]
fn turning_a_locked_item_updates_the_shape_it_travels_with() {
    // Not `packed_pair`: the spell there sits flush against the weapon, so
    // there is genuinely nowhere for it to turn into.
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    let handle = piece(&run, "Oak Handle");
    run.toggle_lock_item(handle);

    let before = run.locked_shape(handle).unwrap();
    run.rotate_locked(handle).expect("there is room to turn");
    let after = run.locked_shape(handle).unwrap();
    assert_ne!(before, after, "the stored shape turned with the item");

    // And it still goes back down as the shape it is now.
    run.unequip_locked(handle).unwrap();
    run.equip_locked_at(handle, SlotKind::Weapon, 0, 0).expect("it fits");
    let slot = run.loadout.slot(SlotKind::Weapon);
    for &(p, dx, dy) in &after {
        assert_eq!(slot.anchor_of(p), Some((dx, dy)), "piece landed where the shape says");
    }
}

/// Selling a piece out of a locked item ends the lock - otherwise the lock
/// keeps naming a piece that no longer exists.
#[test]
fn selling_a_piece_of_a_locked_item_releases_it() {
    let mut run = packed_pair();
    let handle = piece(&run, "Oak Handle");
    let blade = piece(&run, "Iron Blade");
    run.toggle_lock_item(handle);
    assert!(run.is_locked_item(handle));

    run.sell(blade).expect("it can be sold");
    assert!(!run.is_locked_item(handle), "the lock went with it");
    assert!(
        run.loadout.locks.iter().all(|l| !l.pieces.contains(&blade)),
        "no lock still names the sold piece"
    );
}

// ------------------------------------------------------- orbs and alignments

/// An orb takes no ink any more: two spells are the whole requirement, and an
/// alignment is the optional third part.
#[test]
fn an_orb_assembles_from_spells_alone_and_takes_an_alignment() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 4, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1, "two spells is enough");

    // And the alignment joins without breaking it.
    equip(&mut run, "Azure Alignment", SlotKind::Weapon, 4, 2);
    let r = run.report(SlotKind::Weapon);
    assert_eq!(r.assembled_count(), 1, "{}", r.summary());
}

/// An alignment is never cast itself - it colours every spell the ball holds.
#[test]
fn an_alignment_colours_every_spell_in_the_ball() {
    let mut plain = Run::with_all_pieces();
    equip(&mut plain, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut plain, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut plain, "Rime Nova", SlotKind::Weapon, 4, 0);
    let before = plain.combat_items();
    let before = before.iter().find(|i| i.casts.len() > 1).expect("an orb");
    assert_eq!(before.casts.len(), 2, "the alignment is not itself a cast");
    let mana_before: Vec<i32> = before.casts.iter().map(|c| c.stats.mana).collect();

    let mut aligned = Run::with_all_pieces();
    equip(&mut aligned, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut aligned, "Emberburst", SlotKind::Weapon, 1, 3);
    equip(&mut aligned, "Rime Nova", SlotKind::Weapon, 4, 0);
    equip(&mut aligned, "Azure Alignment", SlotKind::Weapon, 4, 2);
    let after = aligned.combat_items();
    let after = after.iter().find(|i| i.casts.len() > 1).expect("an orb");
    assert_eq!(after.casts.len(), 2, "still two casts, not three");

    // Every spell gained, and gained the same - which is the point of an
    // alignment. Not by exactly two: the alignment also carries power, and an
    // item's power multiplies its own numbers, so what reaches each cast is
    // the alignment's mana with the ball's multiplier already on it.
    let gained: Vec<i32> =
        after.casts.iter().enumerate().map(|(i, c)| c.stats.mana - mana_before[i]).collect();
    assert!(gained.iter().all(|g| *g > 0), "no spell gained anything: {:?}", gained);
    assert!(
        gained.windows(2).all(|w| w[0] == w[1]),
        "the alignment reached one spell and not the others: {:?}",
        gained
    );
}

/// A spell that answers its siblings pays out when one of them is cast, which
/// is what a ball can do and a book cannot.
#[test]
fn a_spell_answers_its_siblings_going_off() {
    use gearmaster_engine::piece::{Action, Trigger};
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Scrying Orb", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Echo Sigil", SlotKind::Weapon, 1, 3);
    equip(&mut run, "Rime Nova", SlotKind::Weapon, 4, 0);
    assert_eq!(run.report(SlotKind::Weapon).assembled_count(), 1);

    let items = run.combat_items();
    let orb = items.iter().find(|i| i.casts.len() > 1).expect("an orb");
    let echo = orb
        .casts
        .iter()
        .find(|c| c.name == "Echo Sigil")
        .expect("the sigil is one of its casts");
    assert!(
        echo.triggers.iter().any(|t| matches!(t, Trigger::OnOtherCast(Action::GainMana(_)))),
        "it carries the answering trigger: {:?}",
        echo.triggers
    );
}

/// Every spell that answers a sibling is worth having: a book holds one spell,
/// so the trigger would be dead weight there. This pins that they exist and
/// that the catalogue offers enough of them to build a ball around.
#[test]
fn there_are_spells_that_answer_other_spells() {
    use gearmaster_engine::piece::{PieceKind, Trigger, CATALOG};
    let answering = CATALOG
        .iter()
        .filter(|d| d.kind == PieceKind::Spell)
        .filter(|d| d.triggers.iter().any(|t| matches!(t, Trigger::OnOtherCast(_))))
        .count();
    assert!(answering >= 5, "only {} spells answer their siblings", answering);
}
