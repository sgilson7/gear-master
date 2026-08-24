//! Terrain: pieces that lie under the grid instead of in it.
//!
//! An underlay is the one thing on a board that other pieces may stand on. The
//! rules are deliberately narrow - it is always loose, it never overlaps other
//! terrain, and it is one layer deep - and this is where each of those is held
//! to.
//!
//! The catalogue holds exactly one underlay - the Keystone Base - which is here
//! rather than in the chest sweep because a placement rule nothing has ever
//! exercised is a placement rule that turns out to be wrong later. The tests
//! below the halfway mark use it; the ones above it state their own definitions
//! instead, which is the only way to pin the shape of a payload no piece
//! carries yet.

mod common;

use gearmaster_engine::loadout::Loadout;
use gearmaster_engine::piece::{
    Effect, EffectKind, PieceDef, PieceKind, PieceRegistry, SlotKind, When, CATALOG,
};
use gearmaster_engine::slot::{Slot, SLOT_W};
use gearmaster_engine::stats::{StatKind, Stats};

/// The smallest chest pieces there are, so a test board has room for terrain
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
fn terrain_is_named_by_no_recipe_at_all() {
    // This is the whole of how "an underlay is never part of an item" is
    // enforced. If a recipe ever names Terrain, assembly would start pulling
    // underlays into items and every other rule here would need a special case
    // to stop it.
    for slot in SlotKind::ALL {
        for recipe in gearmaster_engine::piece::recipes(slot) {
            for (kind, _, _) in recipe.iter() {
                assert!(
                    !kind.is_underlay(),
                    "{:?}'s recipe names {:?}, which is terrain",
                    slot,
                    kind
                );
            }
        }
    }
}

#[test]
fn terrain_is_not_a_core() {
    // A core anchors an item. Terrain is not in an item, so it cannot be one -
    // and `PerOverlappingCore` counts cores standing on terrain, which would
    // be nonsense if the terrain counted itself.
    assert!(!PieceKind::Terrain.is_core());
    assert!(PieceKind::Terrain.is_underlay());
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
        assert!(!kind.is_underlay(), "{:?} came back as terrain", kind);
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
    // `taller_boards` already holds this for gear. The terrain layer is a
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
fn nothing_covers_anything_when_there_is_no_terrain() {
    // `covering` answers for terrain and is empty for everything else, so a
    // board with no underlay on it has nothing standing on anything.
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
fn the_terrain_layer_starts_empty_and_stays_out_of_the_way() {
    let (_, lo) = a_board();
    let slot = lo.slot(SlotKind::Chest);
    let mut gear = 0;
    for y in 0..slot.rows() {
        for x in 0..SLOT_W {
            assert_eq!(slot.under_at(x, y), None, "something was laid under ({x},{y})");
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
        // Terrain never assembles, so an underlay effect must be live while
        // *not* assembled or it would be silent for ever.
        assert!(eff.when.holds(false), "{} would never fire on terrain", eff.label);
    }
}

#[test]
fn a_terrain_definition_is_expressible() {
    // The Keystone Base from the design brief, as it will be written. Not in
    // `CATALOG` yet - that is the chest sweep - but the type has to admit it.
    const KEYSTONE: PieceDef = PieceDef {
        name: "Keystone Base",
        slot: SlotKind::Chest,
        kind: PieceKind::Terrain,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        base: Stats::health(10),
        adjacency: None,
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
    assert!(KEYSTONE.kind.is_underlay());
    assert!(!KEYSTONE.kind.is_core());
    // Power is in hundredths, so ten is a tenth of a multiple per core.
    assert!(matches!(
        KEYSTONE.effect.map(|e| e.kind),
        Some(EffectKind::PerOverlappingCore { stat: StatKind::Power, amount: 10 })
    ));
}

#[test]
fn terrain_is_worth_something_before_anything_stands_on_it() {
    // An underlay is rated by expected coverage, so it has to be worth more
    // than its bare stats - otherwise the shop would price terrain as though
    // its whole payload were never going to happen.
    use gearmaster_engine::rating::piece_rating;
    const BARE: PieceDef = PieceDef {
        name: "Bare Ground",
        slot: SlotKind::Chest,
        kind: PieceKind::Terrain,
        cells: &[(0, 0)],
        base: Stats::health(10),
        adjacency: None,
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
        "an underlay that pays for coverage rated no higher than one that does not: {} vs {}",
        piece_rating(&BEARING),
        piece_rating(&BARE)
    );
}

// ------------------------------------------------- overlap, for real
//
// Everything above tests the shape of the mechanic. These test the mechanic,
// against the first terrain piece in the catalogue.

fn keystone() -> usize {
    CATALOG.iter().position(|d| d.name == "Keystone Base").expect("the Keystone Base")
}

/// Lay terrain at `(0, 0)` and seat one chest item on the board, and hand back
/// the registry, the loadout, and the terrain's id.
fn terrain_and_gear() -> (PieceRegistry, Loadout, gearmaster_engine::piece::PieceId) {
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let ground = reg.alloc(keystone());
    assert!(lo.can_place(&reg, ground, SlotKind::Chest, 0, 0).is_ok(), "terrain would not lie down");
    lo.slot_mut(SlotKind::Chest).place(&reg, ground, 0, 0);

    // A base and a layer standing on it, both anchored inside the terrain's
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
        assert!(seated, "gear would not stand on the terrain");
    }
    (reg, lo, ground)
}

#[test]
fn gear_may_stand_on_terrain() {
    let (reg, lo, ground) = terrain_and_gear();
    let slot = lo.slot(SlotKind::Chest);
    let on_top = slot.covering(ground);
    assert!(!on_top.is_empty(), "nothing was standing on the terrain");
    for id in &on_top {
        assert!(!reg.def(*id).kind.is_underlay(), "terrain was reported as standing on terrain");
    }
    // And the terrain is still there underneath all of it.
    assert_eq!(slot.under_at(0, 0), Some(ground));
    assert!(slot.get(0, 0).is_some(), "the cell above the terrain is empty");
}

#[test]
fn terrain_never_lies_on_terrain() {
    // One layer deep. Two underlays may not share a cell even though gear may
    // share one with either of them.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let (a, b) = (reg.alloc(keystone()), reg.alloc(keystone()));
    lo.slot_mut(SlotKind::Chest).place(&reg, a, 0, 0);
    assert!(
        lo.can_place(&reg, b, SlotKind::Chest, 0, 0).is_err(),
        "two underlays took the same ground"
    );
    assert!(
        lo.can_place(&reg, b, SlotKind::Chest, 1, 1).is_err(),
        "two underlays overlapped at a corner"
    );
    assert!(lo.can_place(&reg, b, SlotKind::Chest, 2, 2).is_ok(), "clear ground was refused");
}

#[test]
fn terrain_is_never_part_of_an_item() {
    let (reg, lo, ground) = terrain_and_gear();
    let report = lo.report(&reg, SlotKind::Chest);
    for it in &report.items {
        if it.pieces.contains(&ground) {
            assert!(!it.assembled, "an underlay ended up inside a finished item");
            assert_eq!(it.pieces.len(), 1, "an underlay was grouped with other pieces");
        }
    }
    // And it never reaches combat, because only assembled items do.
    for p in lo.combat_items(&reg) {
        assert!(!p.pieces.contains(&ground), "terrain reached the fight as an item");
    }
}

#[test]
fn an_underlay_pays_for_every_core_standing_on_it() {
    let (reg, lo, ground) = terrain_and_gear();
    let slot = lo.slot(SlotKind::Chest);
    let cores = slot
        .covering(ground)
        .into_iter()
        .filter(|&c| reg.def(c).kind.is_core())
        .count() as i32;
    assert!(cores > 0, "no core ended up on the terrain, so there is nothing to measure");

    // Power is in hundredths and the Keystone pays ten a core, on top of its
    // own ten health.
    let report = lo.report(&reg, SlotKind::Chest);
    let terrain = report
        .items
        .iter()
        .find(|i| i.pieces == vec![ground])
        .expect("the terrain is not in the report at all");
    assert_eq!(terrain.stats.power, 10 * cores, "the Keystone did not pay for its cores");
    assert_eq!(terrain.stats.health, 10, "the terrain lost its own base stats");
    assert!(
        terrain.notes.iter().any(|n| n.contains("covering it")),
        "the report does not say why the terrain is worth what it is: {:?}",
        terrain.notes
    );
}

#[test]
fn bare_terrain_pays_nothing_for_coverage() {
    // The same piece with nothing on it is worth its base stats and no more,
    // so the bonus really is coming from what is standing there.
    let mut reg = PieceRegistry::new();
    let mut lo = Loadout::new();
    let ground = reg.alloc(keystone());
    lo.slot_mut(SlotKind::Chest).place(&reg, ground, 0, 0);

    let report = lo.report(&reg, SlotKind::Chest);
    let terrain = report.items.iter().find(|i| i.pieces == vec![ground]).expect("in the report");
    assert_eq!(terrain.stats.power, 0, "bare terrain paid for coverage it does not have");
    assert_eq!(terrain.stats.health, 10);
}

#[test]
fn terrain_survives_a_share_code() {
    // A share code stores a piece as its index in `CATALOG` and its anchor. An
    // underlay has to come back down in the terrain layer, or a shared board
    // reads back with its ground missing and everything standing on nothing.
    use gearmaster_engine::share;
    let mut run = gearmaster_engine::run::Run::with_all_pieces();
    let ground = run.find_by_name("Keystone Base").expect("the catalogue has one");
    run.equip(ground, SlotKind::Chest, 0, 0).expect("terrain seats in a run");

    let code = share::export(&run);
    let back = share::import(&code).expect("it reads back");
    let (reg, lo) = back.loadout();
    let slot = lo.slot(SlotKind::Chest);
    let laid: Vec<&str> = slot
        .pieces()
        .into_iter()
        .filter(|&p| reg.def(p).kind.is_underlay())
        .map(|p| reg.def(p).name)
        .collect();
    assert_eq!(laid, vec!["Keystone Base"], "the terrain did not survive the round trip");
    assert!(slot.under_at(0, 0).is_some(), "it came back in the wrong layer");
}
