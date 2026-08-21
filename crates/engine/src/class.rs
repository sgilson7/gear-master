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
    /// Spells that answer their siblings going off, per second. Only a crystal
    /// ball holds more than one spell, so this measures a build that has
    /// committed to a ball rather than merely owning one.
    Answering,
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
            Axis::Answering => "answering".into(),
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
        let mut answering = 0.0f32;
        let mut weave = 0.0f32;
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
                if matches!(t, crate::piece::Trigger::OnOtherCast(_)) {
                    answering += rate;
                }
            }
            // Adjacency only. Alignment was measured here too, and it turned
            // out to carry no information: across five grids nearly all gear
            // sits on the top rows, so almost every item lines up with almost
            // every other and the axis read the same for every build. Packing
            // two finished items against each other inside one grid is a real
            // choice, and it is the one worth measuring.
            weave += p.adjacent_items.len() as f32 + p.aligned_items.len() as f32 * 0.5;
        }

        // Reference values: roughly what a strong, focused build reaches. A
        // build past one reads 100 rather than overflowing, so more of the
        // same never costs you a class you already qualified for.
        //
        // These have to be revisited when the catalogue grows, and there is a
        // test that says so: `every_axis_is_reachable` builds toward each one
        // and fails if the best the game can do falls short. Wrath, cadence
        // and weave were all set against a much smaller catalogue and had
        // drifted to where nothing could reach them.
        let n = |v: f32, full: f32| -> i32 { ((v / full) * 100.0).clamp(0.0, 100.0) as i32 };

        let mut scores = vec![
            (Axis::Arcana, n(magic, 24.0)),
            (Axis::Brutality, n(physical, 40.0)),
            (Axis::Ward, n(ward as f32, 90.0)),
            (Axis::Puncture, n(pierce as f32, 70.0)),
            (Axis::Attunement, n(mana, 4.0)),
            (Axis::Wrath, n(rage, 1.3)),
            (Axis::Devotion, n(faith, 1.6)),
            (Axis::Growth, n(nature, 1.4)),
            (Axis::Cadence, n(rate_total, 2.6)),
            (Axis::Mass, n(filled_cells as f32, 130.0)),
            // Per item, not in total: otherwise simply owning more gear maxes
            // it, and "how interconnected is this build" becomes "how much of
            // it is there", which `Mass` already measures.
            (
                Axis::Weave,
                n(weave / (profiles.len().max(1) as f32), 1.8),
            ),
            (Axis::Malice, n(curses, 0.9)),
            (Axis::Bulwark, n(armor, 14.0)),
            (Axis::Sorcery, n(sorcery as f32, 1.6)),
            (Axis::Orbits, n(orbits as f32, 2.0)),
            (Axis::Answering, n(answering, 1.1)),
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
        | Trigger::OnAlignedActivate(a)
        | Trigger::OnOtherCast(a) => is_curse(a),
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
    /// Every `n`th activation of yours stops their gear dead and leaves it
    /// misfiring. The only way anyone gets at the two curses that work on time
    /// rather than on flesh.
    Untimely(u32),
    /// Every activation shortens every *other* item's cooldown by `ms`, so a
    /// fast build compounds on itself.
    Cascade(u32),
    /// Armour is worth `pct` more against the damage type you have most
    /// resistance to already.
    Consecrate(i32),
    /// Landing a curse also banks that much rage.
    Bloodscent(i32),
    /// Spending any pool refunds `pct` of it to every *other* pool.
    Confluence(i32),
}

impl ClassPower {
    pub fn describe(self) -> String {
        match self {
            ClassPower::Standing(s) => s.summary(),
            ClassPower::SlowTime => {
                "incoming damage arrives over 5s".into()
            }
            ClassPower::Leeching(pct) => format!("{}% of damage dealt heals you", pct),
            ClassPower::Overflowing => "held resources count double".into(),
            ClassPower::Echo(n) => format!("every {}rd activation fires twice", n),
            ClassPower::Bastion(pct) => {
                format!("armour returns {}% of what it soaks", pct)
            }
            ClassPower::Contagion => "curses land in pairs".into(),
            ClassPower::Reprisal(n) => format!("taking a hit banks {} faith", n),
            ClassPower::Riposte(ms) => format!("their every act speeds you {:.2}s", ms as f32 / 1000.0),
            ClassPower::Momentum(n) => {
                format!("+{} strength per second elapsed", n)
            }
            ClassPower::Resonance => "reactions fire twice".into(),
            ClassPower::Transmute(pct) => {
                format!("{}% of physical lands again as magic", pct)
            }
            ClassPower::Untimely(n) => format!(
                "every {}th activation of yours freezes their gear for {:.1}s and leaves \
                 one activation in three misfiring for {:.0}s afterwards",
                n,
                crate::curse::STUN_MS as f32 / 1000.0,
                crate::curse::MISFIRE_MS as f32 / 1000.0,
            ),
            ClassPower::Cascade(ms) => {
                format!("each activation cuts {:.1}s off every other item", ms as f32 / 1000.0)
            }
            ClassPower::Consecrate(pct) => {
                format!("armour is {}% stronger where you already resist", pct)
            }
            ClassPower::Bloodscent(n) => format!("landing a curse banks {} rage", n),
            ClassPower::Confluence(pct) => {
                format!("spending a pool refunds {}% to the others", pct)
            }
            ClassPower::Adaptable => "every act banks all four pools".into(),
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

impl ClassDef {
    /// How much this class asks for in total. What decides which of the
    /// classes you qualify for you are actually given - see `rank`.
    pub fn demand(&self) -> i32 {
        self.requires.iter().map(|&(_, n)| n).sum()
    }
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
        blurb: "Gear packed so tightly it talks to its neighbours, in every grid at once.",
        // Weave alone made this the default: it reads much the same for any
        // full build, so a single threshold on it caught everything. Paired
        // with mass it means what it says - a lot of gear, densely laid out.
        requires: &[(Axis::Weave, 70), (Axis::Mass, 55)],
        power: ClassPower::Resonance,
    },
    ClassDef {
        name: "Spellblade",
        blurb: "Half sword, half spellbook, and unwilling to choose.",
        requires: &[(Axis::Arcana, 30), (Axis::Brutality, 22), (Axis::Sorcery, 25)],
        power: ClassPower::Transmute(50),
    },
    // ---- built around the gear the crystal ball rework brought in ----------
    ClassDef {
        name: "Oracle",
        blurb: "A ball whose spells answer each other, and the only hands that can stop a clock.",
        requires: &[(Axis::Orbits, 50), (Axis::Answering, 45)],
        power: ClassPower::Untimely(4),
    },
    ClassDef {
        name: "Stormcaller",
        blurb: "Magic that arrives faster than it can be answered.",
        requires: &[(Axis::Arcana, 55), (Axis::Cadence, 45)],
        power: ClassPower::Cascade(120),
    },
    ClassDef {
        name: "Warpriest",
        blurb: "Faith banked behind a wall, and a wall that faith keeps standing.",
        requires: &[(Axis::Devotion, 45), (Axis::Bulwark, 50)],
        power: ClassPower::Consecrate(40),
    },
    ClassDef {
        name: "Bloodletter",
        blurb: "Rage kept boiling, and something rotting on the other side of it.",
        requires: &[(Axis::Wrath, 45), (Axis::Malice, 40)],
        power: ClassPower::Bloodscent(3),
    },
    ClassDef {
        name: "Wellspring",
        blurb: "Every pool at once, and every drop of it worth twice what it looks.",
        requires: &[
            (Axis::Attunement, 35),
            (Axis::Devotion, 25),
            (Axis::Growth, 30),
            (Axis::Wrath, 25),
        ],
        power: ClassPower::Confluence(50),
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
    // The rule, in one sentence: you are given the most demanding class you
    // qualify for.
    //
    // Sorting by surplus instead - which is what this used to do - rewards a
    // class for being easy. Bulwark asks for ward 45 and bulwark 40, and
    // armour is on almost every piece in the game, so nearly any build cleared
    // both by fifty points and out-scored the class it was actually built
    // for. Nine of the twelve best builds came back Bulwark.
    //
    // Total demand is the right tiebreak because a demanding threshold is a
    // distinctive one: anything can stumble into ward 45, but arcana 50 and
    // sorcery 50 together mean you are genuinely carrying spells. Surplus
    // still decides between classes that ask for the same amount.
    out.sort_by_key(|m| (!m.eligible, std::cmp::Reverse(m.class.demand()), std::cmp::Reverse(m.margin)));
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
