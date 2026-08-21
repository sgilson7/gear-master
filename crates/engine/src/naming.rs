//! Procedural names for assembled gear.
//!
//! Two halves, deliberately:
//!
//! * The **base noun and epithet** come from a hash of the run seed plus every
//!   piece in the item, where it sits, and which way round it is. Nudge one
//!   piece a cell over and you get a different weapon with a different name.
//! * The **qualifier** comes from what the item actually *does* — its triggers
//!   first, then its positional effects, then its loudest stat. A weapon that
//!   burns things is Searing whatever seed you rolled.
//!
//! So the name is stable, reproducible, and tells you something true.

use crate::piece::{Action, EffectKind, PieceId, PieceRegistry, SlotKind, Trigger};
use crate::slot::Slot;

// ------------------------------------------------------------------ hash

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Fingerprint of one assembled item.
///
/// Hashes `(definition, anchor x, anchor y, rotation)` for every piece, sorted
/// so the result depends on the arrangement rather than on the order the
/// pieces happened to be placed in.
pub fn item_hash(seed: u64, reg: &PieceRegistry, slot: &Slot, pieces: &[PieceId]) -> u64 {
    let mut entries: Vec<(u32, u8, u8, u8)> = pieces
        .iter()
        .map(|&p| {
            let (ax, ay) = slot.anchor_of(p).unwrap_or((0, 0));
            (reg.def_index(p) as u32, ax, ay, reg.rotation(p))
        })
        .collect();
    entries.sort_unstable();

    let mut h = fnv(FNV_OFFSET, &seed.to_le_bytes());
    h = fnv(h, &[slot.kind.index() as u8]);
    for (def, ax, ay, rot) in entries {
        h = fnv(h, &def.to_le_bytes());
        h = fnv(h, &[ax, ay, rot]);
    }
    h
}

/// Pick from a list by one slice of the hash, so several independent choices
/// can be drawn from a single fingerprint.
fn pick<'a>(h: u64, shift: u32, corpus: &[&'a str]) -> &'a str {
    corpus[((h >> shift) as usize) % corpus.len()]
}

// --------------------------------------------------------------- corpora

fn bases(slot: SlotKind) -> &'static [&'static str] {
    match slot {
        SlotKind::Weapon => &[
            "Blade", "Edge", "Cleaver", "Fang", "Sliver", "Reaver", "Talon", "Sabre",
            "Thorn", "Lance", "Hewer", "Splitter", "Falchion", "Glaive", "Sting", "Bite",
            "Rend", "Scar", "Warblade", "Kris", "Shiv", "Ripper", "Pike", "Cudgel",
            "Sickle", "Razor", "Spine", "Hook", "Gutter", "Tooth", "Barb", "Skewer",
        ],
        SlotKind::Helmet => &[
            "Crown", "Helm", "Visage", "Casque", "Coif", "Diadem", "Circlet", "Mask",
            "Skullcap", "Barbute", "Sallet", "Gaze", "Brow", "Vigil", "Cowl", "Hood",
            "Faceplate", "Crest", "Halo", "Veil", "Bascinet", "Headpiece", "Wreath", "Horn",
            "Antler", "Beak", "Muzzle", "Blinder", "Watcher", "Sentinel", "Eye", "Mind",
        ],
        SlotKind::Chest => &[
            "Carapace", "Cuirass", "Hauberk", "Aegis", "Shell", "Vestment", "Mantle",
            "Plating", "Ribcage", "Bulwark", "Shroud", "Harness", "Brigandine", "Jerkin",
            "Weave", "Lattice", "Husk", "Chassis", "Frame", "Barrel", "Girdle", "Wrap",
            "Sheath", "Bark", "Scale", "Hide", "Casing", "Cradle", "Vault", "Keel",
            "Hollow", "Cage",
        ],
        SlotKind::Gloves => &[
            "Grasp", "Gauntlet", "Clutch", "Fist", "Grip", "Talons", "Handwraps",
            "Knuckles", "Palm", "Vise", "Claw", "Mitt", "Cuff", "Hold", "Pinch", "Snare",
            "Bracer", "Digit", "Thumbscrew", "Wringer", "Catcher", "Hand", "Finger",
            "Crusher", "Squeeze", "Latch", "Clamp", "Nail", "Paw", "Grapple", "Hook",
            "Cinch",
        ],
        SlotKind::Greaves => &[
            "Stride", "Greave", "Tread", "Step", "Sabaton", "Legguard", "Gait", "Pace",
            "Boot", "March", "Footfall", "Shin", "Heel", "Kick", "Runner", "Walker",
            "Trudge", "Lope", "Vault", "Spur", "Stirrup", "Anklet", "Sole", "Track",
            "Trail", "Wander", "Roam", "Prowl", "Creep", "Bound", "Leap", "Dance",
        ],
    }
}

/// Trailing "of the ___". Deliberately atmospheric rather than descriptive —
/// the qualifier already carries the meaning.
const SUFFIXES: &[&str] = &[
    "Ember", "Deep", "Long Night", "Third Vow", "Quiet", "Nine Coils", "Rust",
    "Late Hour", "Grave Tide", "Pale Fen", "Broken Oath", "Salt Road", "Low Sun",
    "Bell", "Kiln", "Undertow", "Slow Wound", "Hollow King", "Ash Field", "Split Moon",
    "Cold Forge", "Last Lamp", "Winnowing", "Thin Veil", "Drowned Choir", "Gate",
    "Silt", "Wake", "Hunger", "Threadbare Crown", "Iron Fen", "Weeping Gate",
    "Sunken Mile", "Barrow", "Glass Waste", "First Frost", "Red Hour", "Mourning",
    "Fallow Year", "Tallow", "Shale", "Hush", "Cinder Vow", "Long Silence",
];

/// Used when an item does nothing distinctive enough to earn a real qualifier.
const PLAIN_EPITHETS: &[&str] = &[
    "Plain", "Honest", "Serviceable", "Blunt", "Worn", "Simple", "Sturdy", "Rough",
    "Old", "Common", "Practical", "Unadorned", "Weathered", "Solid", "Modest", "Bare",
];

/// Fallback flavour when an item has no triggers or effects to name it after.
/// Each stat gets a set rather than a single word — armour and mana are on so
/// much gear that one word each would flatten half the catalogue into the same
/// name.
const STAT_WORDS: &[(&str, &[&str])] = &[
    ("damage", &["Keen", "Cruel", "Vicious", "Honed", "Wicked", "Savage", "Jagged"]),
    ("mind", &["Whispering", "Murmuring", "Insidious", "Fevered", "Maddening"]),
    ("armor", &["Warded", "Girded", "Ironclad", "Bulwark", "Steadfast", "Bastion", "Shielded"]),
    ("mana", &["Welling", "Brimming", "Charged", "Runed", "Suffused", "Deepwell"]),
    ("regen", &["Mending", "Quickening", "Verdant", "Patient", "Renewing"]),
    ("strength", &["Brutal", "Heavy", "Mighty", "Grim", "Bruising"]),
];

// ------------------------------------------------------------ qualifiers

/// Qualifiers in priority order. The earlier one wins when an item earns
/// several, so the most distinctive behaviour is the one that names it.
const PRIORITY: &[&str] = &[
    "Searing",
    "Martyr's",
    "Frostbitten",
    "Rimebound",
    "Whispering",
    "Conducting",
    "Resonant",
    "Hollow",
    "Empowered",
    "Shielded",
    "Unbound",
    "Blessed",
    "Hastening",
    "Chained",
    "Aligned",
    "Attuned",
    "Echoing",
    "Quickened",
    "Warded",
    "Welling",
    "Striking",
    "Keen",
];

fn action_word(a: &Action) -> Option<&'static str> {
    use crate::curse::CurseKind::*;
    use crate::piece::Target::*;
    Some(match a {
        Action::Gain { .. } => "Brimming",
        Action::Curse { kind: Searing, target: Enemy } => "Searing",
        Action::Curse { kind: Searing, target: Yourself } => "Martyr's",
        Action::Curse { kind: Frost, target: Enemy } => "Rimebound",
        Action::Curse { kind: Frost, target: Yourself } => "Frostbitten",
        Action::MindDamage { .. } => "Whispering",
        Action::GainMana(_) => "Welling",
        Action::GainArmor(_) => "Warded",
        Action::Damage { .. } => "Striking",
        Action::ReduceCooldown(_) => "Hastening",
        Action::GainEmpowerment(_) => "Empowered",
        Action::GainShield(_) => "Shielded",
    })
}

/// Every qualifier this item has earned, most distinctive first.
pub fn qualifiers(reg: &PieceRegistry, pieces: &[PieceId]) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    let mut note = |w: Option<&'static str>| {
        if let Some(w) = w {
            if !found.contains(&w) {
                found.push(w);
            }
        }
    };

    for &p in pieces {
        let def = reg.def(p);
        for t in def.triggers {
            match t {
                Trigger::Spend { what, on_success, on_failure, .. } => {
                    note(Some(match what {
                        crate::piece::Resource::Mana => "Attuned",
                        crate::piece::Resource::Rage => "Furious",
                        crate::piece::Resource::Faith => "Devout",
                        crate::piece::Resource::Nature => "Verdant",
                    }));
                    note(action_word(on_success));
                    note(action_word(on_failure));
                }
                Trigger::OnActivate(a) => note(action_word(a)),
                Trigger::SpendMana { on_success, on_failure, .. } => {
                    note(Some("Attuned"));
                    note(action_word(on_success));
                    note(action_word(on_failure));
                }
                Trigger::PerAdjacentItem { action, .. } => {
                    note(Some("Echoing"));
                    note(action_word(action));
                }
                Trigger::OnAdjacentActivate(a) => {
                    note(Some("Chained"));
                    note(action_word(a));
                }
                Trigger::OnAlignedActivate(a) => {
                    note(Some("Aligned"));
                    note(action_word(a));
                }
            }
        }
        if let Some(eff) = def.effect {
            note(Some(match eff.kind {
                EffectKind::Flat { .. } => {
                    if eff.when == crate::piece::When::NotAssembled { "Unbound" } else { "Blessed" }
                }
                EffectKind::DoubleNeighbor { .. } => "Resonant",
                EffectKind::SelfPerEmptyCell { .. } => "Hollow",
                EffectKind::DoubleAdjacentItemStat { .. } => "Conducting",
            }));
        }
        if def.speed_bonus > 0 {
            note(Some("Quickened"));
        }
    }

    found.sort_by_key(|w| PRIORITY.iter().position(|p| p == w).unwrap_or(usize::MAX));
    found
}

/// Flavour drawn from whichever stats the item actually has. Which stat is
/// used is hash-picked among those present, so two armoured items are not
/// automatically namesakes.
fn stat_qualifier(h: u64, reg: &PieceRegistry, pieces: &[PieceId]) -> Option<&'static str> {
    let mut total = crate::stats::Stats::ZERO;
    for &p in pieces {
        total += reg.def(p).base;
    }
    let present: Vec<&(&str, &[&str])> = STAT_WORDS
        .iter()
        .filter(|(name, _)| match *name {
            "damage" => total.damage > 0,
            "mind" => total.mind > 0,
            "armor" => total.armor > 0,
            "mana" => total.mana > 0,
            "regen" => total.regen > 0,
            "strength" => total.strength > 0,
            _ => false,
        })
        .collect();
    if present.is_empty() {
        return None;
    }
    let chosen = present[((h >> 33) as usize) % present.len()];
    Some(pick(h, 42, chosen.1))
}

// ----------------------------------------------------------------- names

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemName {
    /// "Searing Warblade of the Late Hour" — tooltips and the panel.
    pub full: String,
    /// "Searing Warblade" — anywhere space is tight, like a cooldown bar.
    pub short: String,
}

/// Name one assembled item.
pub fn name_item(seed: u64, reg: &PieceRegistry, slot: &Slot, pieces: &[PieceId]) -> ItemName {
    let h = item_hash(seed, reg, slot, pieces);
    let suffix = pick(h, 21, SUFFIXES);

    // A trigger or effect names the item if it has one; otherwise fall back to
    // whichever stat it actually carries, picked by hash so gear that all
    // grants armour doesn't all end up called the same thing.
    let earned = qualifiers(reg, pieces);
    let qualifier = match earned.first() {
        Some(q) => *q,
        None => stat_qualifier(h, reg, pieces).unwrap_or_else(|| pick(h, 42, PLAIN_EPITHETS)),
    };

    // Draw the noun from a corpus with the qualifier removed, so "Hollow
    // Hollow" cannot happen. Retrying a different hash slice would only make
    // it rare, not impossible.
    let corpus: Vec<&str> =
        bases(slot.kind).iter().copied().filter(|b| *b != qualifier).collect();
    let base = pick(h, 0, &corpus);

    let short = format!("{} {}", qualifier, base);
    ItemName { full: format!("{} of the {}", short, suffix), short }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::{PieceRegistry, CATALOG};
    use crate::slot::Slot;

    fn place(names: &[(&str, u8, u8)], kind: SlotKind) -> (PieceRegistry, Slot, Vec<PieceId>) {
        let mut reg = PieceRegistry::new();
        let mut slot = Slot::new(kind);
        let mut ids = Vec::new();
        for (name, x, y) in names {
            let d = CATALOG.iter().position(|p| &p.name == name).unwrap();
            let id = reg.alloc(d);
            slot.place(&reg, id, *x, *y);
            ids.push(id);
        }
        (reg, slot, ids)
    }

    #[test]
    fn the_same_arrangement_always_gets_the_same_name() {
        let (reg, slot, ids) = place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let a = name_item(7, &reg, &slot, &ids);
        let b = name_item(7, &reg, &slot, &ids);
        assert_eq!(a, b);
        assert!(!a.short.is_empty());
    }

    #[test]
    fn moving_a_piece_one_cell_renames_the_item() {
        let (r1, s1, i1) = place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let (r2, s2, i2) = place(&[("Oak Handle", 0, 1), ("Iron Blade", 1, 1)], SlotKind::Weapon);
        assert_ne!(name_item(7, &r1, &s1, &i1), name_item(7, &r2, &s2, &i2));
    }

    #[test]
    fn a_different_seed_renames_everything() {
        let (reg, slot, ids) = place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        assert_ne!(name_item(1, &reg, &slot, &ids), name_item(2, &reg, &slot, &ids));
    }

    #[test]
    fn rotation_counts_as_part_of_the_arrangement() {
        let (mut reg, slot, ids) = place(&[("Gauntlet Mold", 0, 0)], SlotKind::Gloves);
        let before = name_item(3, &reg, &slot, &ids);
        reg.rotate_cw(ids[0]);
        assert_ne!(name_item(3, &reg, &slot, &ids), before);
    }

    #[test]
    fn the_order_pieces_were_placed_in_does_not_matter() {
        let (r1, s1, mut i1) =
            place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let name = name_item(9, &r1, &s1, &i1);
        i1.reverse();
        assert_eq!(name_item(9, &r1, &s1, &i1), name, "the arrangement is what counts");
    }

    #[test]
    fn the_base_noun_suits_the_slot() {
        let (reg, slot, ids) = place(&[("Steel Frame", 0, 0)], SlotKind::Helmet);
        let n = name_item(11, &reg, &slot, &ids);
        let word = n.short.split_whitespace().last().unwrap();
        assert!(bases(SlotKind::Helmet).contains(&word), "{} is not a helmet word", word);
    }

    #[test]
    fn a_burning_weapon_is_named_for_its_curse() {
        let (reg, slot, ids) =
            place(&[("Cursed Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids);
        assert!(n.short.starts_with("Searing"), "got {:?}", n.short);
    }

    #[test]
    fn a_self_cursing_blade_is_named_for_that_instead() {
        let (reg, slot, ids) =
            place(&[("Oak Handle", 0, 0), ("Cursed Blade", 1, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids);
        assert!(n.short.starts_with("Martyr's"), "got {:?}", n.short);
    }

    #[test]
    fn a_plain_item_still_gets_a_name() {
        let (reg, slot, ids) = place(&[("Oak Handle", 0, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids);
        assert!(!n.short.is_empty());
        assert!(n.full.contains("of the"));
    }

    #[test]
    fn the_qualifier_never_repeats_the_base_noun() {
        // "Hollow Hollow" reads like a bug. Sweep a lot of arrangements to be
        // sure the nudge always finds a different word.
        for seed in 0..200u64 {
            for x in 0..4u8 {
                let (reg, slot, ids) =
                    place(&[("Hollow Weave", x, 2), ("Padded Base", x, 3)], SlotKind::Chest);
                let n = name_item(seed, &reg, &slot, &ids);
                let mut words = n.short.split_whitespace();
                let q = words.next().unwrap();
                let b = words.next().unwrap();
                assert_ne!(q, b, "{:?} repeats itself", n.short);
            }
        }
    }

    #[test]
    fn gear_that_only_grants_armour_still_gets_varied_names() {
        // Nearly every defensive piece grants armour. If that produced one
        // word, half the catalogue would share a name.
        let mut seen = std::collections::HashSet::new();
        for x in 0..4u8 {
            for y in 0..4u8 {
                let (reg, slot, ids) =
                    place(&[("Steel Frame", x, y), ("Iron Plating", x, y + 2)], SlotKind::Helmet);
                let n = name_item(77, &reg, &slot, &ids);
                seen.insert(n.short.split_whitespace().next().unwrap().to_string());
            }
        }
        assert!(seen.len() >= 3, "only {:?} across 16 arrangements", seen);
    }

    #[test]
    fn a_trigger_beats_a_stat_when_naming() {
        // Cursed Handle grants power and has a searing trigger; the trigger
        // must win, because that is what the item is actually about.
        let (reg, slot, ids) =
            place(&[("Cursed Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        for seed in 0..50u64 {
            let n = name_item(seed, &reg, &slot, &ids);
            assert!(n.short.starts_with("Searing"), "seed {} gave {:?}", seed, n.short);
        }
    }

    #[test]
    fn names_spread_out_rather_than_clumping() {
        // Every two-piece weapon arrangement across the grid should produce a
        // good spread of names, not the same handful over and over.
        let mut seen = std::collections::HashSet::new();
        let mut total = 0;
        for y in 0..5u8 {
            for x in 0..4u8 {
                let (reg, slot, ids) =
                    place(&[("Oak Handle", x, y), ("Iron Blade", x + 1, y)], SlotKind::Weapon);
                seen.insert(name_item(1234, &reg, &slot, &ids).full);
                total += 1;
            }
        }
        assert!(
            seen.len() * 10 >= total * 9,
            "only {} distinct names from {} arrangements",
            seen.len(),
            total
        );
    }
}
