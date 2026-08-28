//! Enchantments: pieces that lie under the grid instead of in it.
//!
//! An enchantment is the one thing on a board that gear may stand on. The rules
//! are deliberately narrow - it is always loose, it never overlaps another
//! enchantment, it is one layer deep, and it can never join or merge an item -
//! and this is where each of those is held to.
//!
//! Two conditions decide what one is worth, and they pull opposite ways.
//! **Live** is read on the enchantment layer: nothing else on that layer may
//! touch it, so enchantments want spreading out. **Bonded** is read on the gear
//! layer: one item must cover every one of its cells, so gear wants packing
//! tight and shaped to fit. A bonded item is doubled and handed a trigger.
//!
//! It was called enchantment, which was the wrong word for four grids out of five.
//! Only the greaves have ground under them.

mod common;

use gearmaster_engine::loadout::Loadout;
use gearmaster_engine::piece::{
    Effect, EffectKind, PieceDef, PieceKind, PieceRegistry, SlotKind, When, CATALOG,
};
use gearmaster_engine::slot::{Slot, SLOT_W};
use gearmaster_engine::stats::{StatKind, Stats};

/// The smallest chest pieces there are, so a test board has room for enchantment
/// and something standing on it. Looked up rather than named: a name is a key
/// and these tests have no business pinning one.
fn chest_layer() -> usize {
    CATALOG
        .iter()
        .position(|d| d.slot == SlotKind::Chest && d.kind == PieceKind::Layer && d.cells.len() == 1)
        .expect("a one-cell chest layer to stand on things with")
}

fn chest_base() -> usize {
    CATALOG
        .iter()
        .enumerate()
        .filter(|(_, d)| d.slot == SlotKind::Chest && d.kind == PieceKind::Base)
        .min_by_key(|(_, d)| d.cells.len())
        .map(|(i, _)| i)
        .expect("a chest base")
}

// ------------------------------------------------------------- the kind

#[test]
fn an_enchantment_is_named_by_no_recipe_at_all() {
    // This is the whole of how "an enchantment is never part of an item" is
    // enforced. If a recipe ever names Enchantment, assembly would start pulling
    // enchantments into items and every other rule here would need a special case
    // to stop it.
    for slot in SlotKind::ALL {
        for recipe in gearmaster_engine::piece::recipes(slot) {
            for (kind, _, _) in recipe.iter() {
                assert!(
                    !kind.is_enchantment(),
                    "{:?}'s recipe names {:?}, which is enchantment",
                    slot,
                    kind
                );
            }
        }
    }
}

#[test]
fn an_enchantment_is_not_a_core() {
    // A core anchors an item. Enchantment is not in an item, so it cannot be one -
    // and `PerOverlappingCore` counts cores standing on enchantment, which would
    // be nonsense if the enchantment counted itself.
    assert!(!PieceKind::Enchantment.is_core());
    assert!(PieceKind::Enchantment.is_enchantment());
    for kind in [
        PieceKind::Handle,
        PieceKind::Frame,
        PieceKind::Base,
        PieceKind::Material,
        PieceKind::Book,
        PieceKind::Orb,
        PieceKind::Layer,
        PieceKind::Ring,
    ] {
        assert!(!kind.is_enchantment(), "{:?} came back as enchantment", kind);
    }
}

// -------------------------------------------------------------- placement
//
// Placement is the part that had to change, so the first thing to establish is
// that the gear layer behaves exactly as it always did.

#[test]
fn ordinary_gear_still_collides_with_ordinary_gear() {
    let mut reg = PieceRegistry::new();
    let (a, b) = (reg.alloc(chest_layer()), reg.alloc(chest_layer()));
    let mut slot = Slot::new(SlotKind::Chest);

    assert!(slot.can_place(&reg, a, 2, 2).is_ok());
    slot.place(&reg, a, 2, 2);
    assert!(slot.can_place(&reg, b, 2, 2).is_err(), "two pieces took the same cell");
    assert!(slot.can_place(&reg, b, 3, 2).is_ok());
}

#[test]
fn nothing_may_hang_off_the_edge_however_tall_the_board_is() {
    // The amendment to the spec: grids are six by eight to start with and can
    // be granted rows. Legality is judged against the rows the board has now,
    // not against the constant it started at.
    let mut reg = PieceRegistry::new();
    let id = reg.alloc(chest_layer());
    let mut slot = Slot::new(SlotKind::Chest);
    let start = slot.rows();

    assert!(slot.can_place(&reg, id, 0, start).is_err(), "placed below the last row");
    slot.grow(2);
    assert_eq!(slot.rows(), start + 2);
    assert!(slot.can_place(&reg, id, 0, start).is_ok(), "the granted rows are not usable");
    assert!(slot.can_place(&reg, id, 0, slot.rows()).is_err(), "still bounded, just lower");
    assert!(slot.can_place(&reg, id, SLOT_W, 0).is_err(), "placed past the last column");
}

#[test]
fn growing_a_board_moves_nothing_in_either_layer() {
    // `taller_boards` already holds this for gear. The enchantment layer is a
    // second vector of the same shape and has to resize the same way, or a
    // board granted a row would drop everything laid under it.
    let mut reg = PieceRegistry::new();
    let id = reg.alloc(chest_layer());
    let mut slot = Slot::new(SlotKind::Chest);
    slot.place(&reg, id, 3, 4);
    let before = slot.cells_of(id);

    slot.grow(3);
    assert_eq!(slot.cells_of(id), before, "growing the board moved a piece");
    assert_eq!(slot.anchor_of(id), Some((3, 4)));
}

// ---------------------------------------------------------- what covers what

/// Seat a base and a layer as one chest item, and report the slot.
fn a_board() -> (PieceRegistry, Loadout) {
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let (core, skin) = (reg.alloc(chest_base()), reg.alloc(chest_layer()));
    for id in [core, skin] {
        let seated = (0..8u8).any(|y| {
            if lo.can_place(&reg, id, SlotKind::Chest, 0, y).is_ok() {
                lo.slot_mut(SlotKind::Chest).place(&reg, id, 0, y);
                true
            } else {
                false
            }
        });
        assert!(seated, "could not seat a chest piece on an empty board");
    }
    (reg, lo)
}

#[test]
fn nothing_covers_anything_when_there_is_no_enchantment() {
    // `covering` answers for enchantment and is empty for everything else, so a
    // board with no enchantment on it has nothing standing on anything.
    let (reg, lo) = a_board();
    let slot = lo.slot(SlotKind::Chest);
    for id in slot.pieces() {
        assert!(
            slot.covering(id).is_empty(),
            "{} reported something standing on it",
            reg.def(id).name
        );
    }
}

#[test]
fn the_enchantment_layer_starts_empty_and_stays_out_of_the_way() {
    let (_, lo) = a_board();
    let slot = lo.slot(SlotKind::Chest);
    let mut gear = 0;
    for y in 0..slot.rows() {
        for x in 0..SLOT_W {
            assert_eq!(slot.enchant_at(x, y), None, "something was laid under ({x},{y})");
            gear += slot.get(x, y).is_some() as usize;
        }
    }
    assert!(gear > 0, "the test board seated nothing at all");
}

// ------------------------------------------------------------- the payloads

/// The two overlap effects, spelled out so their shape is pinned even before a
/// catalogue piece carries one. If either variant's fields change, this stops
/// compiling, which is the point.
#[test]
fn the_overlap_payloads_are_shaped_the_way_the_catalogue_will_write_them() {
    let per_item = Effect {
        label: "for each thing standing on it",
        kind: EffectKind::PerOverlappingItem { stat: StatKind::Health, amount: 5 },
        when: When::Always,
    };
    let per_core = Effect {
        label: "for each item built on it",
        kind: EffectKind::PerOverlappingCore { stat: StatKind::Power, amount: 10 },
        when: When::Always,
    };
    for eff in [per_item, per_core] {
        // Enchantment never assembles, so an enchantment effect must be live while
        // *not* assembled or it would be silent for ever.
        assert!(eff.when.holds(false), "{} would never fire on enchantment", eff.label);
    }
}

#[test]
fn an_enchantment_definition_is_expressible() {
    // The Keystone Base from the design brief, as it will be written. Not in
    // `CATALOG` yet - that is the chest sweep - but the type has to admit it.
    const KEYSTONE: PieceDef = PieceDef {
        name: "Keystone Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Enchantment,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(10),
        assembly_bonus: None,
        effect: Some(Effect {
            label: "for each item built on top of it",
            kind: EffectKind::PerOverlappingCore { stat: StatKind::Power, amount: 10 },
            when: When::Always,
        }),
        cooldown_ms: 0,
        quest: None,
        power_bonus: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 30,
    };
    assert!(KEYSTONE.kind.is_enchantment());
    assert!(!KEYSTONE.kind.is_core());
    // Power is in hundredths, so ten is a tenth of a multiple per core.
    assert!(matches!(
        KEYSTONE.effect.map(|e| e.kind),
        Some(EffectKind::PerOverlappingCore { stat: StatKind::Power, amount: 10 })
    ));
}

#[test]
fn an_enchantment_is_worth_something_before_anything_stands_on_it() {
    // An enchantment is rated by expected coverage, so it has to be worth more
    // than its bare stats - otherwise the shop would price enchantment as though
    // its whole payload were never going to happen.
    use gearmaster_engine::rating::piece_rating;
    const BARE: PieceDef = PieceDef {
        name: "Bare Ground",
        slot: SlotKind::Chest,
        kind: PieceKind::Enchantment,
        cells: &[(0, 0)],
        base: Stats::health(10),
        assembly_bonus: None,
        effect: None,
        cooldown_ms: 0,
        quest: None,
        power_bonus: 0,
        speed_bonus: 0,
        triggers: &[],
        price: 10,
    };
    const BEARING: PieceDef = PieceDef {
        effect: Some(Effect {
            label: "for each thing standing on it",
            kind: EffectKind::PerOverlappingItem { stat: StatKind::Health, amount: 20 },
            when: When::Always,
        }),
        ..BARE
    };
    assert!(
        piece_rating(&BEARING) > piece_rating(&BARE),
        "an enchantment that pays for coverage rated no higher than one that does not: {} vs {}",
        piece_rating(&BEARING),
        piece_rating(&BARE)
    );
}

// ------------------------------------------------- overlap, for real
//
// Everything above tests the shape of the mechanic. These test the mechanic,
// against the first enchantment piece in the catalogue.

fn keystone() -> usize {
    CATALOG.iter().position(|d| d.name == "Keystone Base").expect("the Keystone Base")
}

/// Lay enchantment at `(0, 0)` and seat one chest item on the board, and hand back
/// the registry, the loadout, and the enchantment's id.
fn terrain_and_gear() -> (PieceRegistry, Loadout, gearmaster_engine::piece::PieceId) {
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let ground = reg.alloc(keystone());
    assert!(lo.can_place(&reg, ground, SlotKind::Chest, 0, 0).is_ok(), "enchantment would not lie down");
    lo.slot_mut(SlotKind::Chest).place(&reg, ground, 0, 0);

    // A base and a layer standing on it, both anchored inside the enchantment's
    // two-by-two footprint so they really are on top of it.
    for def in [chest_base(), chest_layer()] {
        let id = reg.alloc(def);
        let seated = (0..2u8).any(|x| {
            (0..2u8).any(|y| {
                if lo.can_place(&reg, id, SlotKind::Chest, x, y).is_ok() {
                    lo.slot_mut(SlotKind::Chest).place(&reg, id, x, y);
                    true
                } else {
                    false
                }
            })
        });
        assert!(seated, "gear would not stand on the enchantment");
    }
    (reg, lo, ground)
}

#[test]
fn gear_may_stand_on_an_enchantment() {
    let (reg, lo, ground) = terrain_and_gear();
    let slot = lo.slot(SlotKind::Chest);
    let on_top = slot.covering(ground);
    assert!(!on_top.is_empty(), "nothing was standing on the enchantment");
    for id in &on_top {
        assert!(!reg.def(*id).kind.is_enchantment(), "enchantment was reported as standing on enchantment");
    }
    // And the enchantment is still there underneath all of it.
    assert_eq!(slot.enchant_at(0, 0), Some(ground));
    assert!(slot.get(0, 0).is_some(), "the cell above the enchantment is empty");
}

#[test]
fn an_enchantment_never_lies_on_an_enchantment() {
    // One layer deep. Two enchantments may not share a cell even though gear may
    // share one with either of them.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let (a, b) = (reg.alloc(keystone()), reg.alloc(keystone()));
    lo.slot_mut(SlotKind::Chest).place(&reg, a, 0, 0);
    assert!(
        lo.can_place(&reg, b, SlotKind::Chest, 0, 0).is_err(),
        "two enchantments took the same ground"
    );
    assert!(
        lo.can_place(&reg, b, SlotKind::Chest, 1, 1).is_err(),
        "two enchantments overlapped at a corner"
    );
    assert!(lo.can_place(&reg, b, SlotKind::Chest, 2, 2).is_ok(), "clear ground was refused");
}

#[test]
fn an_enchantment_is_never_part_of_an_item() {
    let (reg, lo, ground) = terrain_and_gear();
    let report = lo.report(&reg, SlotKind::Chest);
    for it in &report.items {
        if it.pieces.contains(&ground) {
            assert!(!it.assembled, "an enchantment ended up inside a finished item");
            assert_eq!(it.pieces.len(), 1, "an enchantment was grouped with other pieces");
        }
    }
    // And it never reaches combat, because only assembled items do.
    for p in lo.combat_items(&reg) {
        assert!(!p.pieces.contains(&ground), "enchantment reached the fight as an item");
    }
}

#[test]
fn an_enchantment_pays_for_every_core_standing_on_it() {
    let (reg, lo, ground) = terrain_and_gear();
    let slot = lo.slot(SlotKind::Chest);
    let cores = slot
        .covering(ground)
        .into_iter()
        .filter(|&c| reg.def(c).kind.is_core())
        .count() as i32;
    assert!(cores > 0, "no core ended up on the enchantment, so there is nothing to measure");

    // Power is in hundredths and the Keystone pays ten a core, on top of its
    // own ten health.
    let report = lo.report(&reg, SlotKind::Chest);
    let enchantment = report
        .items
        .iter()
        .find(|i| i.pieces == vec![ground])
        .expect("the enchantment is not in the report at all");
    assert_eq!(enchantment.stats.power, 10 * cores, "the Keystone did not pay for its cores");
    assert_eq!(enchantment.stats.health, 10, "the enchantment lost its own base stats");
    assert!(
        enchantment.notes.iter().any(|n| n.contains("covering it")),
        "the report does not say why the enchantment is worth what it is: {:?}",
        enchantment.notes
    );
}

#[test]
fn a_bare_enchantment_pays_nothing_for_coverage() {
    // The same piece with nothing on it is worth its base stats and no more,
    // so the bonus really is coming from what is standing there.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let ground = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, ground, 0, 0);

    let report = lo.report(&reg, SlotKind::Chest);
    let enchantment = report.items.iter().find(|i| i.pieces == vec![ground]).expect("in the report");
    assert_eq!(enchantment.stats.power, 0, "bare enchantment paid for coverage it does not have");
    assert_eq!(enchantment.stats.health, 10);
}

#[test]
fn an_enchantment_survives_a_share_code() {
    // A share code stores a piece as its index in `CATALOG` and its anchor. An
    // enchantment has to come back down in the enchantment layer, or a shared board
    // reads back with its ground missing and everything standing on nothing.
    use gearmaster_engine::share;
    let mut run = gearmaster_engine::run::Run::with_all_pieces();
    let ground = run.find_by_name("Keystone Base").expect("the catalogue has one");
    run.equip(ground, SlotKind::Chest, 0, 0).expect("enchantment seats in a run");

    let code = share::export(&run);
    let back = share::import(&code).expect("it reads back");
    let (reg, lo) = back.loadout();
    let slot = lo.slot(SlotKind::Chest);
    let laid: Vec<&str> = slot
        .pieces()
        .into_iter()
        .filter(|&p| reg.def(p).kind.is_enchantment())
        .map(|p| reg.def(p).name)
        .collect();
    assert_eq!(laid, vec!["Keystone Base"], "the enchantment did not survive the round trip");
    assert!(slot.enchant_at(0, 0).is_some(), "it came back in the wrong layer");
}

// ------------------------------------------------- live, bonded, and the lock

#[test]
fn an_enchantment_touching_another_one_is_smothered() {
    // Live is read on the enchantment's own layer, and the rule is a moat:
    // nothing else on that layer may touch it. Two keystones side by side
    // smother each other, and neither is worth anything at all.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let a = reg.alloc(keystone());
    let b = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, a, 0, 0);
    assert!(lo.slot(SlotKind::Chest).enchant_is_live(a), "alone, it is live");

    lo.slot_mut(SlotKind::Chest).place(&reg, b, 2, 0);
    let slot = lo.slot(SlotKind::Chest);
    assert!(!slot.enchant_is_live(a), "a is touching b");
    assert!(!slot.enchant_is_live(b), "and b is touching a");

    // A dead enchantment gives nothing, stats included.
    let r = lo.report(&reg, SlotKind::Chest);
    assert_eq!(r.stats.health, 0, "a smothered enchantment still paid its base stats");
}

#[test]
fn the_edge_of_the_board_is_not_something_to_be_crowded_by() {
    // An enchantment cannot be laid out of bounds, so a rule that read the rim
    // as occupied would punish the only cells with nowhere to crowd from.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let a = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, a, 0, 0);
    assert!(lo.slot(SlotKind::Chest).enchant_is_live(a), "a corner is clear on two sides");
}

#[test]
fn one_item_covering_all_of_it_is_doubled_and_handed_a_trigger() {
    // The bond. Both halves have to hold: live on its own layer, and every one
    // of its cells covered by pieces belonging to the same item.
    //
    // Measured as a difference against the same board with the keystone taken
    // out from under it, because an item's power is the sum of what its pieces
    // carry and that is not something this test should be pinning a number for.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();

    // One chest item filling a 2x2 exactly: the smallest base in the corner and
    // one-cell layers over whatever of the square it leaves. A chest recipe is
    // a base and one to three layers, so the square has to be four cells or
    // fewer of gear - which it is.
    let base = reg.alloc(chest_base());
    assert!(lo.can_place(&reg, base, SlotKind::Chest, 0, 0).is_ok(), "the base fits the corner");
    lo.slot_mut(SlotKind::Chest).place(&reg, base, 0, 0);
    for (x, y) in [(0u8, 0u8), (1, 0), (0, 1), (1, 1)] {
        if lo.slot(SlotKind::Chest).get(x, y).is_some() {
            continue;
        }
        let id = reg.alloc(chest_layer());
        assert!(lo.can_place(&reg, id, SlotKind::Chest, x, y).is_ok(), "layer at {x},{y}");
        lo.slot_mut(SlotKind::Chest).place(&reg, id, x, y);
    }
    for (x, y) in [(0u8, 0u8), (1, 0), (0, 1), (1, 1)] {
        assert!(lo.slot(SlotKind::Chest).get(x, y).is_some(), "the square is not full at {x},{y}");
    }
    let bare = lo
        .combat_items(&reg)
        .into_iter()
        .find(|p| p.slot == SlotKind::Chest)
        .expect("a chest item with no enchantment under it");

    let stone = reg.alloc(keystone());
    assert!(lo.can_place(&reg, stone, SlotKind::Chest, 0, 0).is_ok(), "under the gear");
    lo.slot_mut(SlotKind::Chest).place(&reg, stone, 0, 0);

    let slot = lo.slot(SlotKind::Chest);
    assert!(slot.enchant_is_live(stone), "nothing else is on its layer");
    assert!(slot.enchant_is_buried(stone), "every cell is covered");

    let bonded = lo
        .combat_items(&reg)
        .into_iter()
        .find(|p| p.slot == SlotKind::Chest)
        .expect("a chest item");
    assert_eq!(
        bonded.power,
        bare.power + 100,
        "bonding is worth one whole multiple of power and it did not arrive"
    );
    assert!(
        bonded.triggers.len() > bare.triggers.len()
            || CATALOG[keystone()].triggers.is_empty(),
        "the enchantment's triggers did not reach the item it is under"
    );
}

#[test]
fn an_enchantment_cannot_merge_two_items_into_one() {
    // The thing the layer is kept separate to prevent. `groups` walks the gear
    // layer, so an enchantment is not there to be walked through: two items
    // standing on one are still two items.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let stone = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, stone, 0, 0);

    // Two chest items, one on each half of the keystone, with a clear row
    // between them so nothing but the enchantment could join them.
    for (bx, by, lx, ly) in [(0u8, 0u8, 0u8, 1u8), (3, 0, 3, 1)] {
        let core = reg.alloc(chest_base());
        assert!(lo.can_place(&reg, core, SlotKind::Chest, bx, by).is_ok());
        lo.slot_mut(SlotKind::Chest).place(&reg, core, bx, by);
        let skin = reg.alloc(chest_layer());
        if lo.can_place(&reg, skin, SlotKind::Chest, lx, ly).is_ok() {
            lo.slot_mut(SlotKind::Chest).place(&reg, skin, lx, ly);
        }
    }

    let r = lo.report(&reg, SlotKind::Chest);
    let assembled = r.items.iter().filter(|i| i.assembled).count();
    assert_eq!(assembled, 2, "the enchantment merged what stands on it into {assembled} item(s)");
}

#[test]
fn half_a_cover_is_no_bond_at_all() {
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let stone = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, stone, 0, 0);
    let skin = reg.alloc(chest_layer());
    lo.slot_mut(SlotKind::Chest).place(&reg, skin, 0, 0);
    assert!(
        !lo.slot(SlotKind::Chest).enchant_is_buried(stone),
        "one cell of four is not a cover"
    );
}

// ------------------------------------------- the yard's ground, and only it

/// The four the Switchyard pays out, in the order their buffer stops pay them.
///
/// THE HUNDRED's three are the same law and are held by
/// `the_countys_ground_is_dug_up_and_never_sold` below rather than added here,
/// because "the yard's ground" is what this constant means and a list that
/// grows every mission is a list that stops naming anything.
const THE_YARDS_GROUND: [&str; 4] =
    ["Ballast Bed", "Points Rodding", "Booking Hall", "Signal Wire"];

/// Ground is bought in a town, or dug up. It is never for sale on the road.
///
/// The law used to read "ground is bought where somebody has a floor to sell,
/// never off the road", and read against the code that was two facts wearing
/// one sentence. **An enchantment never reaches a road shelf** is the law, it
/// is enforced three times in `shop.rs` by kind, and nothing here touches it.
/// **Every enchantment reaches every town cart** was a consequence of
/// `town_shelf` collecting by kind - written so that a new underlay would be
/// town gear without anybody having to remember - and it is the half that had
/// to give, because a cart stocking these would be selling what the yard is
/// for.
///
/// So: `is_town_stock` is still true of all four, every road shelf still
/// refuses them for that reason, and the cart refuses them for a second one.
#[test]
fn the_yards_ground_is_dug_up_and_never_sold() {
    use gearmaster_engine::piece::{is_event_only, is_town_stock, town_shelf, CATALOG};

    let cart = town_shelf();
    for name in THE_YARDS_GROUND {
        let d = CATALOG.iter().find(|d| d.name == name).unwrap_or_else(|| panic!("{name}"));
        assert!(d.kind.is_enchantment(), "{name} is not ground at all");
        assert!(is_event_only(name), "{name} could be dealt somewhere");
        // Still ground, so `shop.rs`'s three filters still refuse it: the law
        // is unchanged and this is the assertion that says so.
        assert!(is_town_stock(d), "{name} stopped being ground, which is not the change");
        assert!(!cart.contains(&name), "{name} is on a town cart");
    }

    // And the six that came before are still on it. A filter that took the
    // wrong four would read green above and be a silent loss here.
    let shipped = CATALOG
        .iter()
        .filter(|d| d.kind.is_enchantment() && !is_event_only(d.name))
        .count();
    assert_eq!(shipped, 6, "the cart should hold the six that are sold");
    for d in CATALOG.iter().filter(|d| d.kind.is_enchantment() && !is_event_only(d.name)) {
        assert!(cart.contains(&d.name), "{} fell off the cart", d.name);
    }
    assert_eq!(
        CATALOG.iter().filter(|d| d.kind.is_enchantment()).count(),
        13,
        "six sold, four dug out of a switchyard and three dug out of a county"
    );
}

/// THE HUNDRED's three, dug up and never sold.
///
/// One law, applied twice. The Switchyard settled it (E-4) and this is what it
/// looks like the second time somebody uses it: an enchantment that is
/// `EVENT_ONLY` is still ground - `shop.rs`'s three filters still refuse it on
/// the road - and it is not on the cart a town puts out, because the county's
/// ground is dug up rather than bought.
const THE_COUNTYS_GROUND: [&str; 3] = ["Trig Pillar", "Drove Way", "The Common Ground"];

#[test]
fn the_countys_ground_is_dug_up_and_never_sold() {
    use gearmaster_engine::piece::{is_event_only, is_town_stock, town_shelf, CATALOG};
    let cart = town_shelf();
    for name in THE_COUNTYS_GROUND {
        let d = CATALOG.iter().find(|d| d.name == name).unwrap_or_else(|| panic!("{name}"));
        assert!(d.kind.is_enchantment(), "{name} is not ground at all");
        assert!(is_event_only(name), "{name} could be dealt somewhere");
        assert!(is_town_stock(d), "{name} stopped being ground, which is not the change");
        assert!(!cart.contains(&name), "{name} is on a town cart");
    }
    // One per chain, in the slot that chain taxes.
    use gearmaster_engine::piece::SlotKind;
    let slots: Vec<SlotKind> = THE_COUNTYS_GROUND
        .iter()
        .map(|n| CATALOG.iter().find(|d| d.name == *n).unwrap().slot)
        .collect();
    assert_eq!(slots, vec![SlotKind::Greaves, SlotKind::Gloves, SlotKind::Chest]);
}

/// The crucible cannot make one and cannot eat one.
#[test]
fn the_yards_ground_cannot_be_melted_into_or_out_of() {
    use gearmaster_engine::run::Run;

    for name in THE_YARDS_GROUND {
        let mut run = Run::with_all_pieces();
        let id = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == name)
            .unwrap_or_else(|| panic!("{name} is not in the sandbox"));
        assert!(run.melt(id).is_none(), "{name} went into the crucible");
    }
}

/// Nothing steps into one, on any setting.
///
/// `stepped_component` sorts a footprint family by `monster_value` and walks
/// along it, and it filters event-only pieces out of every family - so asking
/// for two steps either way off one of these comes back with the piece itself.
/// That is the mechanism behind "no creature can ever wear one", and it is
/// worth asserting at the source as well as through the `gear_at` fixture.
#[test]
fn nothing_steps_into_the_yards_ground() {
    use gearmaster_engine::combat::stepped_component;

    for name in THE_YARDS_GROUND.iter().chain(&["Shunter's Orb", "Signalman's Orb"]) {
        for step in [-2, -1, 1, 2] {
            assert_eq!(
                stepped_component(name, step),
                *name,
                "{name} was stepped {step} into something else"
            );
        }
    }
}
