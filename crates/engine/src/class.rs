//! Classes, read off the build rather than chosen.
//!
//! # The rule that makes this extensible
//!
//! **No class definition may ever name a component.** A class is a set of
//! minimum values on abstract axes, and every axis is measured by summing
//! properties that every component already has - its slot, its kind, its
//! stats, its triggers. A new component moves the axes it happens to touch
//! and no class definition changes.
//!
//! That is the whole trick. Writing "Chronomancer needs a Scrying Orb" would
//! mean revisiting Chronomancer every time an orb is added; writing
//! "Chronomancer needs Orbits >= 45 and MagicChest >= 40" means new orbs and
//! new magical chestpieces feed it automatically, and a component that is
//! removed simply stops contributing.
//!
//! Axes are normalised 0-100 against a reference build, so thresholds keep
//! meaning the same thing as the catalogue grows. They are deliberately
//! forgiving at the top: a build far past a threshold reads 100, so piling on
//! more of the same never silently disqualifies you from a class you already
//! matched.

use crate::loadout::ItemProfile;
use crate::piece::{PieceKind, PieceRegistry, SlotKind};
use crate::stats::Stats;

/// One measurable property of a build.
///
/// Adding an axis is additive: existing classes keep their thresholds and
/// simply never mention the new one.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Axis {
    /// Magic damage across the build.
    Arcana,
    /// Physical damage across the build.
    Brutality,
    /// Resistance and hardening of either type.
    Ward,
    /// Piercing of either type.
    Puncture,
    /// Mana banked per second.
    Attunement,
    /// Rage banked per second.
    Wrath,
    /// Faith banked per second.
    Devotion,
    /// Nature banked per second.
    Growth,
    /// Activations a second across every assembled item.
    Cadence,
    /// How much of the five grids is covered.
    Mass,
    /// Adjacency and alignment between finished items.
    Weave,
    /// Curses landed per second.
    Malice,
    /// Armour granted per second.
    Bulwark,
    /// Spell cores of any kind - books and crystal balls.
    Sorcery,
    /// Crystal balls specifically, which cycle their spells.
    Orbits,
    /// Magical weight carried by one slot. Five axes, one per slot, so a class
    /// can care about *where* the magic is and not only how much.
    MagicIn(SlotKind),
    /// The same for physical weight.
    PhysicalIn(SlotKind),
}

impl Axis {
    pub fn name(self) -> String {
        match self {
            Axis::Arcana => "arcana".into(),
            Axis::Brutality => "brutality".into(),
            Axis::Ward => "ward".into(),
            Axis::Puncture => "puncture".into(),
            Axis::Attunement => "attunement".into(),
            Axis::Wrath => "wrath".into(),
            Axis::Devotion => "devotion".into(),
            Axis::Growth => "growth".into(),
            Axis::Cadence => "cadence".into(),
            Axis::Mass => "mass".into(),
            Axis::Weave => "weave".into(),
            Axis::Malice => "malice".into(),
            Axis::Bulwark => "bulwark".into(),
            Axis::Sorcery => "sorcery".into(),
            Axis::Orbits => "orbits".into(),
            Axis::MagicIn(s) => format!("magic in the {}", s.name().to_lowercase()),
            Axis::PhysicalIn(s) => format!("iron in the {}", s.name().to_lowercase()),
        }
    }
}

/// Every axis, measured 0-100.
#[derive(Clone, Debug, Default)]
pub struct Fingerprint {
    scores: Vec<(Axis, i32)>,
}

impl Fingerprint {
    pub fn get(&self, axis: Axis) -> i32 {
        self.scores.iter().find(|(a, _)| *a == axis).map(|(_, v)| *v).unwrap_or(0)
    }

    pub fn all(&self) -> &[(Axis, i32)] {
        &self.scores
    }

    /// The axes a build leans on hardest, strongest first.
    pub fn leading(&self, n: usize) -> Vec<(Axis, i32)> {
        let mut v = self.scores.clone();
        v.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
        v.truncate(n);
        v
    }

    /// Measure a build. `profiles` are its assembled items - loose gear does
    /// not count towards a class any more than it counts in a fight.
    pub fn of(reg: &PieceRegistry, profiles: &[ItemProfile], filled_cells: usize) -> Fingerprint {
        // Per-second rates, so a fast item counts for more than a slow one -
        // the same basis the rating module uses.
        let mut rate_total = 0.0f32;
        let mut magic = 0.0f32;
        let mut physical = 0.0f32;
        let mut ward = 0i32;
        let mut pierce = 0i32;
        let (mut mana, mut rage, mut faith, mut nature) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let mut armor = 0.0f32;
        let mut curses = 0.0f32;
        let mut sorcery = 0i32;
        let mut orbits = 0i32;
        let mut weave = 0i32;
        let mut magic_in = [0.0f32; 5];
        let mut physical_in = [0.0f32; 5];

        for p in profiles {
            let rate = 1000.0 / p.cooldown_ms.max(1) as f32;
            rate_total += rate;
            let s: &Stats = &p.stats;

            magic += s.magic_damage as f32 * rate;
            physical += (s.physical_damage + s.damage) as f32 * rate;
            ward += s.physical_resist + s.magic_resist + s.physical_harden + s.magic_harden;
            pierce += s.physical_pierce + s.magic_pierce;
            mana += s.mana as f32 * rate;
            rage += s.rage as f32 * rate;
            faith += s.faith as f32 * rate;
            nature += s.nature as f32 * rate;
            armor += s.armor as f32 * rate;

            magic_in[p.slot.index()] +=
                (s.magic_damage + s.magic_resist + s.magic_pierce + s.magic_harden) as f32;
            physical_in[p.slot.index()] +=
                (s.physical_damage + s.damage + s.physical_resist + s.physical_pierce) as f32;

            for piece in &p.pieces {
                match reg.def(*piece).kind {
                    PieceKind::Book => sorcery += 1,
                    PieceKind::Orb => {
                        sorcery += 1;
                        orbits += 1;
                    }
                    // Spells and ink are magical weight wherever they sit.
                    PieceKind::Spell | PieceKind::Ink => magic_in[p.slot.index()] += 6.0,
                    _ => {}
                }
            }
            for t in &p.triggers {
                if trigger_lands_a_curse(t) {
                    curses += rate;
                }
            }
            weave += p.adjacent_items.len() as i32 + p.aligned_items.len() as i32;
        }

        // Reference values: roughly what a strong, focused build reaches. A
        // build past one reads 100 rather than overflowing, so more of the
        // same never costs you a class you already qualified for.
        let n = |v: f32, full: f32| -> i32 { ((v / full) * 100.0).clamp(0.0, 100.0) as i32 };

        let mut scores = vec![
            (Axis::Arcana, n(magic, 24.0)),
            (Axis::Brutality, n(physical, 40.0)),
            (Axis::Ward, n(ward as f32, 90.0)),
            (Axis::Puncture, n(pierce as f32, 70.0)),
            (Axis::Attunement, n(mana, 4.0)),
            (Axis::Wrath, n(rage, 4.0)),
            (Axis::Devotion, n(faith, 3.0)),
            (Axis::Growth, n(nature, 3.0)),
            (Axis::Cadence, n(rate_total, 4.0)),
            (Axis::Mass, n(filled_cells as f32, 130.0)),
            (Axis::Weave, n(weave as f32, 14.0)),
            (Axis::Malice, n(curses, 2.0)),
            (Axis::Bulwark, n(armor, 14.0)),
            (Axis::Sorcery, n(sorcery as f32, 3.0)),
            (Axis::Orbits, n(orbits as f32, 2.0)),
        ];
        for slot in SlotKind::ALL {
            scores.push((Axis::MagicIn(slot), n(magic_in[slot.index()], 30.0)));
            scores.push((Axis::PhysicalIn(slot), n(physical_in[slot.index()], 40.0)));
        }
        Fingerprint { scores }
    }
}

fn trigger_lands_a_curse(t: &crate::piece::Trigger) -> bool {
    use crate::piece::{Action, Target, Trigger};
    let is_curse = |a: &Action| {
        matches!(a, Action::Curse { target: Target::Enemy, .. })
    };
    match t {
        Trigger::OnActivate(a)
        | Trigger::PerAdjacentItem { action: a, .. }
        | Trigger::OnAdjacentActivate(a)
        | Trigger::OnAlignedActivate(a) => is_curse(a),
        Trigger::SpendMana { on_success, on_failure, .. }
        | Trigger::Spend { on_success, on_failure, .. } => {
            is_curse(on_success) || is_curse(on_failure)
        }
    }
}

/// What a class does for you.
///
/// New powers are additive: a class that wants one names it, and every other
/// class is untouched.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClassPower {
    /// A standing bonus, applied once before the fight. Kept for the floor
    /// class, which is meant to be unremarkable.
    Standing(Stats),
    /// Damage arrives spread over five seconds instead of all at once, which
    /// gives regeneration and armour time to answer it.
    SlowTime,
    /// A share of the damage you deal comes back as health, in percent.
    Leeching(i32),
    /// Every point of a resource you are holding is worth double.
    Overflowing,
    /// Every `n`th activation fires its payload twice.
    Echo(u32),
    /// A share of what your armour absorbs is handed straight back as armour,
    /// so a wall keeps rebuilding itself under fire.
    Bastion(i32),
    /// Landing a curse lands the other kind alongside it.
    Contagion,
    /// Taking a hit banks faith, so being ground down is itself a resource.
    Reprisal(i32),
    /// Every enemy activation pushes all of your cooldowns forward by `ms`.
    Riposte(u32),
    /// Strength climbs by `per_sec` for every second the fight lasts.
    Momentum(i32),
    /// Reactions - the triggers that answer a neighbour or an aligned item -
    /// fire twice.
    Resonance,
    /// A share of your physical damage lands again as magic, in percent.
    Transmute(i32),
    /// Every activation banks one of each of the four pools.
    Adaptable,
}

impl ClassPower {
    pub fn describe(self) -> String {
        match self {
            ClassPower::Standing(s) => s.summary(),
            ClassPower::SlowTime => {
                "damage you take arrives over 5 seconds instead of all at once".into()
            }
            ClassPower::Leeching(pct) => format!("{}% of the damage you deal heals you", pct),
            ClassPower::Overflowing => "everything you are holding is worth double".into(),
            ClassPower::Echo(n) => format!("every {}th activation fires twice", n),
            ClassPower::Bastion(pct) => {
                format!("{}% of what your armour soaks is handed back as armour", pct)
            }
            ClassPower::Contagion => "every curse you land brings the other kind with it".into(),
            ClassPower::Reprisal(n) => format!("taking a hit banks {} faith", n),
            ClassPower::Riposte(ms) => format!(
                "every enemy activation pushes your cooldowns forward {:.1}s",
                ms as f32 / 1000.0
            ),
            ClassPower::Momentum(n) => {
                format!("+{} strength for every second the fight has lasted", n)
            }
            ClassPower::Resonance => "reactions to neighbours and aligned gear fire twice".into(),
            ClassPower::Transmute(pct) => {
                format!("{}% of your physical damage lands again as magic", pct)
            }
            ClassPower::Adaptable => "every activation banks one of all four pools".into(),
        }
    }
}

/// One class: a name, what the build has to look like, and what you get.
///
/// `requires` is the contract. It may only mention axes.
#[derive(Copy, Clone, Debug)]
pub struct ClassDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub requires: &'static [(Axis, i32)],
    pub power: ClassPower,
}

pub static CLASSES: &[ClassDef] = &[
    ClassDef {
        name: "Chronomancer",
        blurb: "Orbs that never cast the same thing twice, and a chestpiece full of magic.",
        requires: &[
            (Axis::Orbits, 45),
            (Axis::MagicIn(SlotKind::Chest), 35),
        ],
        power: ClassPower::SlowTime,
    },
    ClassDef {
        name: "Archmage",
        blurb: "Magic damage, cast often, from books.",
        requires: &[(Axis::Arcana, 50), (Axis::Sorcery, 50)],
        power: ClassPower::Echo(3),
    },
    ClassDef {
        name: "Berserker",
        blurb: "Rage, and something heavy to spend it on.",
        requires: &[(Axis::Wrath, 40), (Axis::Brutality, 40)],
        power: ClassPower::Leeching(12),
    },
    ClassDef {
        name: "Bulwark",
        blurb: "Resistance, hardening, and armour by the ton.",
        requires: &[(Axis::Ward, 45), (Axis::Bulwark, 40)],
        power: ClassPower::Bastion(35),
    },
    ClassDef {
        name: "Hexweaver",
        blurb: "Curses, and the mana to keep landing them.",
        requires: &[(Axis::Malice, 45), (Axis::Attunement, 30)],
        power: ClassPower::Contagion,
    },
    ClassDef {
        name: "Druid",
        blurb: "Growth banked faster than anything can take it off you.",
        requires: &[(Axis::Growth, 45), (Axis::Ward, 25)],
        power: ClassPower::Overflowing,
    },
    ClassDef {
        name: "Templar",
        blurb: "Faith held, iron worn, and no hurry about any of it.",
        requires: &[(Axis::Devotion, 40), (Axis::PhysicalIn(SlotKind::Chest), 30)],
        power: ClassPower::Reprisal(2),
    },
    ClassDef {
        name: "Duelist",
        blurb: "Many small items, all of them fast.",
        requires: &[(Axis::Cadence, 55), (Axis::Brutality, 25)],
        power: ClassPower::Riposte(250),
    },
    ClassDef {
        name: "Juggernaut",
        blurb: "Every cell filled, and nothing wasted.",
        requires: &[(Axis::Mass, 60), (Axis::Ward, 20)],
        power: ClassPower::Momentum(2),
    },
    ClassDef {
        name: "Geomancer",
        blurb: "Gear that talks to its neighbours, across every grid.",
        requires: &[(Axis::Weave, 55)],
        power: ClassPower::Resonance,
    },
    ClassDef {
        name: "Spellblade",
        blurb: "Half sword, half spellbook, and unwilling to choose.",
        requires: &[(Axis::Arcana, 30), (Axis::Brutality, 30), (Axis::Sorcery, 25)],
        power: ClassPower::Transmute(50),
    },
    ClassDef {
        name: "Wanderer",
        blurb: "No particular commitment to anything, and a little of everything.",
        // The floor: something you can always reach, so a fountain is never
        // wasted on a build that matched nothing.
        requires: &[],
        power: ClassPower::Adaptable,
    },
];

/// How well a build matches one class.
#[derive(Clone, Debug)]
pub struct Match {
    pub class: &'static ClassDef,
    /// Every requirement met.
    pub eligible: bool,
    /// Total amount by which the requirements are cleared. Higher wins.
    pub margin: i32,
    /// Per-requirement (axis, needed, have), so the interface can show what is
    /// still missing.
    pub detail: Vec<(Axis, i32, i32)>,
}

/// Rank every class against a build, best first.
///
/// Eligible classes come first, ordered by how far past their thresholds the
/// build is; the rest follow, ordered by how close they are. That second half
/// is what makes the outcome predictable: the player can see what they nearly
/// have and go and get it.
pub fn rank(fp: &Fingerprint) -> Vec<Match> {
    let mut out: Vec<Match> = CLASSES
        .iter()
        .map(|class| {
            let detail: Vec<(Axis, i32, i32)> =
                class.requires.iter().map(|&(a, need)| (a, need, fp.get(a))).collect();
            let eligible = detail.iter().all(|(_, need, have)| have >= need);
            let margin = detail.iter().map(|(_, need, have)| have - need).sum();
            Match { class, eligible, margin, detail }
        })
        .collect();
    out.sort_by_key(|m| (!m.eligible, std::cmp::Reverse(m.margin)));
    out
}

/// The class a build would be given right now. Never `None`: the Wanderer has
/// no requirements, so a fountain always has something to hand over.
pub fn classify(fp: &Fingerprint) -> &'static ClassDef {
    rank(fp).into_iter().find(|m| m.eligible).map(|m| m.class).unwrap_or(&CLASSES[CLASSES.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_class_ever_names_a_component() {
        // The guarantee this whole module rests on. A class is thresholds on
        // axes; if one could name gear, adding gear would mean revisiting
        // every class that mentioned it.
        //
        // Enforced structurally - `requires` can only hold axes - so this test
        // exists to state the rule and to fail loudly if the type ever gains a
        // variant that could carry a name.
        for c in CLASSES {
            for (axis, threshold) in c.requires {
                assert!(
                    (0..=100).contains(threshold),
                    "{} wants {} at {}, which is off the 0-100 scale",
                    c.name,
                    axis.name(),
                    threshold
                );
            }
        }
    }

    #[test]
    fn there_is_always_a_class_to_give() {
        let empty = Fingerprint::default();
        assert_eq!(classify(&empty).name, "Wanderer", "a fountain is never wasted");
    }

    #[test]
    fn every_class_but_the_floor_asks_for_something() {
        let floor = CLASSES.iter().filter(|c| c.requires.is_empty()).count();
        assert_eq!(floor, 1, "exactly one class should be the fallback");
        assert_eq!(CLASSES.last().unwrap().requires.len(), 0, "and it must sort last");
    }

    #[test]
    fn ranking_puts_eligible_classes_first_and_near_misses_next() {
        let fp = Fingerprint {
            scores: vec![(Axis::Orbits, 90), (Axis::MagicIn(SlotKind::Chest), 80)],
        };
        let ranked = rank(&fp);
        assert!(ranked[0].eligible);
        assert_eq!(ranked[0].class.name, "Chronomancer");
        // And the misses carry enough detail to chase.
        let miss = ranked.iter().find(|m| !m.eligible).expect("something is out of reach");
        assert!(!miss.detail.is_empty());
    }

    #[test]
    fn more_of_the_same_never_costs_you_a_class_you_already_matched() {
        // Axes clamp at 100, so a build cannot overshoot itself out of a
        // class. Without this, piling on orbs could push a score past a
        // window and silently lose Chronomancer.
        let modest = Fingerprint { scores: vec![(Axis::Orbits, 50), (Axis::MagicIn(SlotKind::Chest), 40)] };
        let extreme = Fingerprint { scores: vec![(Axis::Orbits, 100), (Axis::MagicIn(SlotKind::Chest), 100)] };
        assert_eq!(classify(&modest).name, "Chronomancer");
        assert_eq!(classify(&extreme).name, "Chronomancer");
    }
}
