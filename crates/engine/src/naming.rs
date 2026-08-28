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
use crate::rating::Rarity;
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

/// Nouns an item can be built around, one pool per slot.
const WEAPON_BASES: &[&str] = &[
            "Blade", "Edge", "Cleaver", "Fang", "Sliver", "Reaver", "Talon", "Sabre",
            "Thorn", "Lance", "Hewer", "Splitter", "Falchion", "Glaive", "Sting", "Bite",
            "Rend", "Scar", "Warblade", "Kris", "Shiv", "Ripper", "Pike", "Cudgel",
            "Sickle", "Razor", "Spine", "Hook", "Gutter", "Tooth", "Barb", "Skewer",
];

const HELMET_BASES: &[&str] = &[
            "Crown", "Helm", "Visage", "Casque", "Coif", "Diadem", "Circlet", "Mask",
            "Skullcap", "Barbute", "Sallet", "Gaze", "Brow", "Vigil", "Cowl", "Hood",
            "Faceplate", "Crest", "Halo", "Veil", "Bascinet", "Headpiece", "Wreath", "Horn",
            "Antler", "Beak", "Muzzle", "Blinder", "Watcher", "Sentinel", "Eye", "Mind",
];

const CHEST_BASES: &[&str] = &[
            "Carapace", "Cuirass", "Hauberk", "Aegis", "Shell", "Vestment", "Mantle",
            "Plating", "Ribcage", "Bulwark", "Shroud", "Harness", "Brigandine", "Jerkin",
            "Weave", "Lattice", "Husk", "Chassis", "Frame", "Barrel", "Girdle", "Wrap",
            "Sheath", "Bark", "Scale", "Hide", "Casing", "Cradle", "Vault", "Keel",
            "Hollow", "Cage",
];

const GLOVE_BASES: &[&str] = &[
            "Grasp", "Gauntlet", "Clutch", "Fist", "Grip", "Talons", "Handwraps",
            "Knuckles", "Palm", "Vise", "Claw", "Mitt", "Cuff", "Hold", "Pinch", "Snare",
            "Bracer", "Digit", "Thumbscrew", "Wringer", "Catcher", "Hand", "Finger",
            "Crusher", "Squeeze", "Latch", "Clamp", "Nail", "Paw", "Grapple", "Hook",
            "Cinch",
];

const GREAVE_BASES: &[&str] = &[
            "Stride", "Greave", "Tread", "Step", "Sabaton", "Legguard", "Gait", "Pace",
            "Boot", "March", "Footfall", "Shin", "Heel", "Kick", "Runner", "Walker",
            "Trudge", "Lope", "Vault", "Spur", "Stirrup", "Anklet", "Sole", "Track",
            "Trail", "Wander", "Roam", "Prowl", "Creep", "Bound", "Leap", "Dance",
];


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

/// Words that sit between the qualifier and the noun on a common item, where
/// there is no room for an "of the ..." tail. Places and materials: a common
/// item should read like something made locally out of what was to hand.
const ATTRIBUTIVES: &[&str] = &[
    "Iron", "Ash", "Salt", "Bone", "Rust", "Pale", "Grave", "Cold", "Thin", "Low",
    "Deep", "Quiet", "Slow", "Broken", "Hollow", "Long", "Kiln", "Fen", "Barrow",
    "Silt", "Tallow", "Shale", "Bell", "Gate", "Cinder", "Fallow", "Glass", "Sunken",
    "Drowned", "Weeping", "Winnow", "Hush",
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
    "Shunting",
    "Ballasted",
    "Derailing",
    "Accruing",
];

fn action_word(a: &Action) -> Option<&'static str> {
    use crate::curse::CurseKind::*;
    use crate::piece::Target::*;
    Some(match a {
        Action::Gain { .. } => "Brimming",
        // The only piece that gives something up, so it gets the word for it.
        Action::SeeWithTheWrongSense => "Blind",
        // The cadence three. A name is the only place a player meets these
        // before the card, so each gets its own rather than falling into a
        // shared word.
        Action::Prime { .. } => "Waiting",
        Action::PrimeBoard { .. } => "Marshalled",
        Action::Drift { .. } => "Tiring",
        Action::Unshakable => "Unshakable",
        // A fusion is named for the thing it makes, which is the whole reason
        // anybody builds one.
        Action::Fuse { into, .. } => match into {
            crate::piece::Resource::DruidicMight => "Druidic",
            crate::piece::Resource::Communion => "Communing",
            _ => "Zealous",
        },
        // Named for what it takes, not for what it leaves.
        Action::Drain { hurt, target: Enemy, .. } if *hurt > 0 => "Bloodletting",
        Action::Drain { target: Enemy, .. } => "Siphoning",
        Action::Drain { target: Yourself, .. } => "Squandering",
        Action::Curse { kind: Searing, target: Enemy } => "Searing",
        Action::Curse { kind: Searing, target: Yourself } => "Martyr's",
        Action::Curse { kind: Frost, target: Enemy } => "Rimebound",
        Action::Curse { kind: Frost, target: Yourself } => "Frostbitten",
        Action::Curse { kind: Stun, target: Enemy } => "Stilling",
        Action::StunStrongest { target: Enemy } => "Singling",
        Action::StunStrongest { target: Yourself } => "Self-Stilling",
        Action::Curse { kind: Stun, target: Yourself } => "Palsied",
        Action::Curse { kind: Misfire, target: Enemy } => "Faltering",
        Action::Curse { kind: Misfire, target: Yourself } => "Cursed",
        Action::MindDamage { .. } => "Whispering",
        Action::GainMana(_) => "Welling",
        Action::GainArmor(_) => "Warded",
        Action::Grow(_) => "Everlasting",
        Action::Damage { .. } => "Striking",
        Action::ReduceCooldown(_) => "Hastening",
        Action::GainEmpowerment(_) => "Empowered",
        Action::GainShield(_) => "Shielded",
        Action::GainSpellblade(_) => "Whetted",
        Action::GainDread(_) => "Foreboding",
        Action::GainDeflection(_) => "Glancing",
        Action::GainForking(_) => "Forked",
        // A shunt moves time between bars; the word is the yard's own.
        Action::Shunt { .. } => "Shunting",
        Action::Ballast(_) => "Ballasted",
        Action::Derail { .. } => "Derailing",
        Action::Accrue { .. } => "Accruing",
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
                Trigger::PerAdjacentEmpty(_) => note(Some("Unbounded")),
                Trigger::OnEnemyActivate(_) => note(Some("Answering")),
                Trigger::Consume { .. } => note(Some("Emptying")),
                Trigger::SpendGold { on_success, .. } => {
                    note(Some("Gilded"));
                    note(action_word(on_success));
                }
                Trigger::OnBattleStart(a) => {
                    note(Some("Prepared"));
                    note(action_word(a));
                }
                Trigger::OnDiagonalActivate(a) => {
                    note(Some("Oblique"));
                    note(action_word(a));
                }
                Trigger::Watch { then, .. } => {
                    note(Some("Tallying"));
                    note(action_word(then));
                }
                Trigger::Spend { what, on_success, on_failure, .. } => {
                    note(Some(match what {
                        crate::piece::Resource::Mana => "Attuned",
                        crate::piece::Resource::Rage => "Furious",
                        crate::piece::Resource::Faith => "Devout",
                        crate::piece::Resource::Nature => "Verdant",
                        // A fusion is not spendable, so this arm is
                        // unreachable from a legal `Spend`. Named anyway
                        // rather than left to a catch-all, so adding a
                        // spendable pool later has to come back here.
                        crate::piece::Resource::DruidicMight => "Druidic",
                        crate::piece::Resource::Communion => "Communing",
                        crate::piece::Resource::Zealotry => "Zealous",
                        // Nor is Insight - it is fuel for Dread and nothing
                        // spends it directly. Named for the same reason.
                        crate::piece::Resource::Insight => "Knowing",
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
                Trigger::OnOtherCast(a) => {
                    note(Some("Answering"));
                    note(action_word(a));
                }
            }
        }
        if let Some(eff) = def.effect {
            note(Some(match eff.kind {
                EffectKind::SelfPerNeighborKind { .. } => "Clustered",
                // What it gives up, which is the only thing about it.
                EffectKind::WrongSense => "Blind",
                EffectKind::SoleIf { .. } => "Solitary",
                EffectKind::Flat { .. } => {
                    if eff.when == crate::piece::When::NotAssembled { "Unbound" } else { "Blessed" }
                }
                EffectKind::DoubleNeighbor { .. } => "Resonant",
                EffectKind::SelfPerEmptyCell { .. } => "Hollow",
                EffectKind::DoubleAdjacentItemStat { .. } => "Conducting",
                // Named for the thing standing on it, which is what an
                // underlay is for.
                EffectKind::PerOverlappingItem { .. } => "Bearing",
                EffectKind::PerOverlappingCore { .. } => "Foundational",
                // The county's three. "Bearing" is taken by
                // `PerOverlappingItem` above and has been since the gear-slot
                // rewrite, so the greaves effect that shares its name does not
                // share its word - a name is a word a player reads and two
                // mechanics answering to one is a name that says nothing.
                EffectKind::Bearing => "Sole",
                EffectKind::Overtake => "Overtaking",
                EffectKind::Commons => "Common",
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
            "damage" => total.physical_damage + total.magic_damage > 0,
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
/// The words one theme names items out of.
///
/// Everything the generator draws on, so a theme can replace the whole corpus
/// without touching how names are built. The rule that a name grows with its
/// rarity is the generator's, not a theme's.
#[derive(Debug)]
pub struct Naming {
    pub weapon_bases: &'static [&'static str],
    pub helmet_bases: &'static [&'static str],
    pub chest_bases: &'static [&'static str],
    pub glove_bases: &'static [&'static str],
    pub greave_bases: &'static [&'static str],
    /// Sits between qualifier and noun on a common item.
    pub attributives: &'static [&'static str],
    /// Tails. One- and two-word entries may be mixed; the generator picks by
    /// length, so a legendary always gets a longer one than an epic.
    pub suffixes: &'static [&'static str],
    /// For an item that earned no qualifier of its own.
    pub epithets: &'static [&'static str],
}

impl Naming {
    pub fn bases(&self, kind: SlotKind) -> &'static [&'static str] {
        match kind {
            SlotKind::Weapon => self.weapon_bases,
            SlotKind::Helmet => self.helmet_bases,
            SlotKind::Chest => self.chest_bases,
            SlotKind::Gloves => self.glove_bases,
            SlotKind::Greaves => self.greave_bases,
        }
    }
}

/// The game's own words.
pub static PLAIN_NAMING: Naming = Naming {
    weapon_bases: WEAPON_BASES,
    helmet_bases: HELMET_BASES,
    chest_bases: CHEST_BASES,
    glove_bases: GLOVE_BASES,
    greave_bases: GREAVE_BASES,
    attributives: ATTRIBUTIVES,
    suffixes: SUFFIXES,
    epithets: PLAIN_EPITHETS,
};

pub fn name_item(
    seed: u64,
    reg: &PieceRegistry,
    slot: &Slot,
    pieces: &[PieceId],
    rarity: Rarity,
    naming: &Naming,
) -> ItemName {
    let h = item_hash(seed, reg, slot, pieces);

    // A trigger or effect names the item if it has one; otherwise fall back to
    // whichever stat it actually carries, picked by hash so gear that all
    // grants armour doesn't all end up called the same thing.
    let earned = qualifiers(reg, pieces);
    let qualifier = match earned.first() {
        Some(q) => *q,
        None => stat_qualifier(h, reg, pieces).unwrap_or_else(|| pick(h, 42, naming.epithets)),
    };

    // Draw the noun from a corpus with the qualifier removed, so "Hollow
    // Hollow" cannot happen. Retrying a different hash slice would only make
    // it rare, not impossible.
    let corpus: Vec<&str> =
        naming.bases(slot.kind).iter().copied().filter(|b| *b != qualifier).collect();
    let base = pick(h, 0, &corpus);
    let short = format!("{} {}", qualifier, base);

    // A name grows with what it is worth, so rarity is audible before the
    // badge is read: three words common, four rare, five epic, six legendary.
    // A tail is one or two words of its own, and either of them can be a word
    // the name has already used: "Bastion Hollow of the Hollow King" is a
    // qualifier, a noun and a two-word tail that all agree with each other.
    // The noun and the attributive were already filtered against the
    // qualifier; the tail never was, so it was the one place a repeat could
    // still get through.
    let taken = [qualifier.to_lowercase(), base.to_lowercase()];
    let clean = |s: &&'static str| -> bool {
        !s.split_whitespace().any(|w| taken.contains(&w.to_lowercase()))
    };
    let tail = |want: usize| -> &'static str {
        let pool: Vec<&str> = naming
            .suffixes
            .iter()
            .copied()
            .filter(|s| s.split_whitespace().count() == want)
            .filter(clean)
            .collect();
        // A theme with no clean tail of that length widens rather than gives
        // up: any length that does not repeat, and only then anything at all.
        // The word count is a target, not a promise it can keep on somebody
        // else's corpus.
        if !pool.is_empty() {
            return pick(h, 21, &pool);
        }
        let any_clean: Vec<&str> = naming.suffixes.iter().copied().filter(clean).collect();
        if !any_clean.is_empty() {
            return pick(h, 21, &any_clean);
        }
        pick(h, 21, naming.suffixes)
    };
    let full = match rarity {
        Rarity::Common => {
            let attr: Vec<&str> =
                naming.attributives.iter().copied().filter(|a| *a != qualifier && *a != base).collect();
            format!("{} {} {}", qualifier, pick(h, 33, &attr), base)
        }
        Rarity::Rare => format!("{} of {}", short, tail(1)),
        Rarity::Epic => format!("{} of the {}", short, tail(1)),
        Rarity::Legendary => format!("{} of the {}", short, tail(2)),
    };
    ItemName { full, short }
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
        let a = name_item(7, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        let b = name_item(7, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        assert_eq!(a, b);
        assert!(!a.short.is_empty());
    }

    #[test]
    fn moving_a_piece_one_cell_renames_the_item() {
        let (r1, s1, i1) = place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let (r2, s2, i2) = place(&[("Oak Handle", 0, 1), ("Iron Blade", 1, 1)], SlotKind::Weapon);
        assert_ne!(name_item(7, &r1, &s1, &i1, Rarity::Epic, &PLAIN_NAMING), name_item(7, &r2, &s2, &i2, Rarity::Epic, &PLAIN_NAMING));
    }

    #[test]
    fn a_different_seed_renames_everything() {
        let (reg, slot, ids) = place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        assert_ne!(name_item(1, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING), name_item(2, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING));
    }

    #[test]
    fn rotation_counts_as_part_of_the_arrangement() {
        let (mut reg, slot, ids) = place(&[("Gauntlet Mold", 0, 0)], SlotKind::Gloves);
        let before = name_item(3, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        reg.rotate_cw(ids[0]);
        assert_ne!(name_item(3, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING), before);
    }

    #[test]
    fn the_order_pieces_were_placed_in_does_not_matter() {
        let (r1, s1, mut i1) =
            place(&[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let name = name_item(9, &r1, &s1, &i1, Rarity::Epic, &PLAIN_NAMING);
        i1.reverse();
        assert_eq!(name_item(9, &r1, &s1, &i1, Rarity::Epic, &PLAIN_NAMING), name, "the arrangement is what counts");
    }

    #[test]
    fn the_base_noun_suits_the_slot() {
        let (reg, slot, ids) = place(&[("Steel Frame", 0, 0)], SlotKind::Helmet);
        let n = name_item(11, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        let word = n.short.split_whitespace().last().unwrap();
        assert!(PLAIN_NAMING.bases(SlotKind::Helmet).contains(&word), "{} is not a helmet word", word);
    }

    #[test]
    fn a_burning_weapon_is_named_for_its_curse() {
        let (reg, slot, ids) =
            place(&[("Cursed Handle", 0, 0), ("Iron Blade", 1, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        assert!(n.short.starts_with("Searing"), "got {:?}", n.short);
    }

    #[test]
    fn a_self_cursing_blade_is_named_for_that_instead() {
        let (reg, slot, ids) =
            place(&[("Oak Handle", 0, 0), ("Cursed Blade", 1, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
        assert!(n.short.starts_with("Martyr's"), "got {:?}", n.short);
    }

    #[test]
    fn a_plain_item_still_gets_a_name() {
        let (reg, slot, ids) = place(&[("Oak Handle", 0, 0)], SlotKind::Weapon);
        let n = name_item(4, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
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
                let n = name_item(seed, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
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
                // Anvil Frame rather than Steel Frame: the helmet sweep gave
                // Steel Frame a mana trigger, and a trigger beats a stat when
                // naming - so the fixture stopped being armour-only and every
                // arrangement came back "Welling". This one is still nothing
                // but armour, health and hardening, which is what the test is
                // about.
                let (reg, slot, ids) =
                    place(&[("Anvil Frame", x, y), ("Iron Plating", x, y + 2)], SlotKind::Helmet);
                let n = name_item(77, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
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
            let n = name_item(seed, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING);
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
                seen.insert(name_item(1234, &reg, &slot, &ids, Rarity::Epic, &PLAIN_NAMING).full);
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

#[cfg(test)]
mod rarity_names {
    use super::*;
    use crate::piece::{PieceRegistry, CATALOG};
    use crate::slot::Slot;

    fn built(kind: SlotKind, names: &[(&str, u8, u8)]) -> (PieceRegistry, Slot, Vec<PieceId>) {
        let mut reg = PieceRegistry::new();
        let mut slot = Slot::new(kind);
        let mut ids = Vec::new();
        for (name, x, y) in names {
            let d = CATALOG.iter().position(|c| c.name == *name).expect("known component");
            let id = reg.alloc(d);
            slot.place(&reg, id, *x, *y);
            ids.push(id);
        }
        (reg, slot, ids)
    }

    /// The rule: a name grows with what the item is worth, so rarity is
    /// audible before the badge is read. Three words common, four rare, five
    /// epic, six legendary - counting every token, "of" and "the" included.
    #[test]
    fn a_name_is_as_long_as_the_item_is_good() {
        let (reg, slot, ids) = built(SlotKind::Weapon, &[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)]);
        for (rarity, want) in [
            (Rarity::Common, 3),
            (Rarity::Rare, 4),
            (Rarity::Epic, 5),
            (Rarity::Legendary, 6),
        ] {
            for seed in 0..64u64 {
                let n = name_item(seed, &reg, &slot, &ids, rarity, &PLAIN_NAMING);
                assert_eq!(
                    n.full.split_whitespace().count(),
                    want,
                    "{:?} name {:?} is the wrong length",
                    rarity,
                    n.full
                );
            }
        }
    }

    /// The short form is what the cooldown bars show and it has to stay two
    /// words whatever the item is worth, or the bars start wrapping.
    #[test]
    fn the_short_name_does_not_grow() {
        let (reg, slot, ids) = built(SlotKind::Weapon, &[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)]);
        for rarity in [Rarity::Common, Rarity::Rare, Rarity::Epic, Rarity::Legendary] {
            let n = name_item(3, &reg, &slot, &ids, rarity, &PLAIN_NAMING);
            assert_eq!(n.short.split_whitespace().count(), 2, "{:?}: {:?}", rarity, n.short);
        }
    }

    /// A word must not appear twice in one name. "Hollow Hollow Cage" is the
    /// failure the corpus filtering exists to prevent, and the common tier
    /// draws from one more pool than the others.
    #[test]
    fn no_name_repeats_a_word() {
        for kind in SlotKind::ALL {
            let (reg, slot, ids) = match kind {
                SlotKind::Weapon => built(kind, &[("Oak Handle", 0, 0), ("Iron Blade", 1, 0)]),
                SlotKind::Helmet => built(kind, &[("Tin Frame", 0, 0), ("Tin Plating", 0, 2)]),
                SlotKind::Chest => built(kind, &[("Sackcloth Base", 0, 0), ("Rag Layer", 0, 2)]),
                SlotKind::Gloves => built(kind, &[("Leather Material", 0, 0), ("Gripping Mold", 2, 0)]),
                SlotKind::Greaves => built(kind, &[("Leather Material", 0, 0), ("Greave Mold", 2, 0)]),
            };
            for seed in 0..96u64 {
                for rarity in [Rarity::Common, Rarity::Rare, Rarity::Epic, Rarity::Legendary] {
                    let n = name_item(seed, &reg, &slot, &ids, rarity, &PLAIN_NAMING);
                    let mut words: Vec<String> =
                        n.full.split_whitespace().map(|w| w.to_lowercase()).collect();
                    words.retain(|w| w != "of" && w != "the");
                    let mut seen = Vec::new();
                    for w in &words {
                        assert!(!seen.contains(w), "{:?} repeats {:?}", n.full, w);
                        seen.push(w.clone());
                    }
                }
            }
        }
    }

    /// Every corpus a theme supplies has to be big enough to draw from, or
    /// names collapse onto a handful of repeats.
    #[test]
    fn the_plain_corpora_are_deep_enough() {
        for kind in SlotKind::ALL {
            assert!(PLAIN_NAMING.bases(kind).len() >= 24, "{:?} has too few nouns", kind);
        }
        assert!(PLAIN_NAMING.attributives.len() >= 16);
        assert!(PLAIN_NAMING.epithets.len() >= 8);
        for want in [1usize, 2] {
            let n = PLAIN_NAMING
                .suffixes
                .iter()
                .filter(|s| s.split_whitespace().count() == want)
                .count();
            assert!(n >= 8, "only {} tails of {} word(s)", n, want);
        }
    }
}
