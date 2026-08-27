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

impl Axis {
    /// What this axis actually measures, and what raises it.
    ///
    /// The fountain reads a build and hands you a title for it, and until now
    /// the only thing it would tell you was the name of the number it had
    /// scored you on. "Geomancer needs weave 0/70" is not a sentence a player
    /// can act on unless they already know what weave is.
    pub fn explain(self) -> &'static str {
        match self {
            Axis::Arcana => "Magic damage a second, across every finished item. \
                Spells, books, balls and inks all count - and so does making \
                those items faster, because it is measured per second rather \
                than per cast.",
            Axis::Brutality => "Physical damage a second, the same way: \
                blades, claws, the strength on your character sheet, and any \
                rage you are holding.",
            Axis::Ward => "Resistance and hardening of both types, added up. \
                Flat defence only - the numbers that cut a hit before it \
                lands, not anything conditional.",
            Axis::Puncture => "Piercing of both types. Worth only what the \
                other side is resisting, so it reads high on a build that has \
                decided to go through armour rather than around it.",
            Axis::Attunement => "Mana banked a second. It counts mana granted \
                per activation, so a fast item granting 1 beats a slow one \
                granting 2.",
            Axis::Wrath => "Rage banked a second. Every point of rage you are \
                holding adds to physical damage.",
            Axis::Devotion => "Faith banked a second. Every point of faith you \
                are holding adds to both resistances.",
            Axis::Growth => "Nature banked a second. Every point of nature you \
                are holding adds to regeneration.",
            Axis::Cadence => "Activations a second, added up across every \
                finished item. Many fast items read higher than a few slow \
                ones, whatever any of them actually do.",
            Axis::Mass => "How many of the 240 cells across the five grids are \
                covered. It does not care what is in them.",
            Axis::Weave => "How connected the build is, per item: finished \
                items touching each other inside one grid, and items in \
                different grids sharing rows. Divided by how many items you \
                have, so simply owning more gear does not raise it - that is \
                Mass.",
            Axis::Malice => "Curses landed a second. Any trigger that puts a \
                curse on the enemy counts, whatever the curse is.",
            Axis::Bulwark => "Armour granted a second. Armour starts every \
                fight at zero and soaks damage before health does.",
            Axis::Sorcery => "How many spell cores you are holding. Books and \
                crystal balls both count.",
            Axis::Orbits => "Crystal balls only. A ball cycles through its \
                spells; a book casts the one thing it is bound to.",
            Axis::Answering => "Spells that answer their siblings going off. \
                Only a ball holds more than one spell, so this reads a build \
                that has committed to a ball rather than merely bought one.",
            Axis::MagicIn(_) => "Magical weight sitting in one particular \
                grid. There are five of these, one per slot, so a title can \
                care about where the magic is and not only how much of it \
                there is.",
            Axis::PhysicalIn(_) => "The same for physical weight: blades, \
                plating and armour concentrated in one grid rather than \
                spread across all five.",
        }
    }

    /// The axes as the glossary shows them: every distinct one once, with the
    /// five per-slot pairs collapsed to the one entry that explains them all.
    pub fn glossary() -> Vec<(String, &'static str)> {
        let mut out: Vec<(String, &'static str)> = [
            Axis::Arcana,
            Axis::Brutality,
            Axis::Ward,
            Axis::Puncture,
            Axis::Attunement,
            Axis::Wrath,
            Axis::Devotion,
            Axis::Growth,
            Axis::Cadence,
            Axis::Mass,
            Axis::Weave,
            Axis::Malice,
            Axis::Bulwark,
            Axis::Sorcery,
            Axis::Orbits,
            Axis::Answering,
        ]
        .iter()
        .map(|a| (a.name(), a.explain()))
        .collect();
        out.push(("magic in a slot".into(), Axis::MagicIn(SlotKind::Weapon).explain()));
        out.push(("iron in a slot".into(), Axis::PhysicalIn(SlotKind::Weapon).explain()));
        out
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
            physical += s.physical_damage as f32 * rate;
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
                (s.physical_damage + s.physical_resist + s.physical_pierce) as f32;

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
        Trigger::PerAdjacentEmpty(inner) => trigger_lands_a_curse(inner),
        Trigger::Consume { per, .. } => is_curse(per),
        Trigger::OnBattleStart(a) => is_curse(a),
        Trigger::OnEnemyActivate(a) => is_curse(a),
        Trigger::OnActivate(a)
        | Trigger::PerAdjacentItem { action: a, .. }
        | Trigger::OnAdjacentActivate(a)
        | Trigger::OnAlignedActivate(a)
        | Trigger::OnDiagonalActivate(a)
        | Trigger::OnOtherCast(a) => is_curse(a),
        Trigger::Watch { then, .. } => is_curse(then),
        Trigger::SpendGold { on_success, .. } => is_curse(on_success),
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
    /// Damage arrives spread over `n` seconds instead of all at once, which
    /// gives regeneration and armour time to answer it.
    SlowTime(u32),
    /// A share of the damage you deal comes back as health, in percent.
    Leeching(i32),
    /// Every point of a resource you are holding counts `n` times.
    Overflowing(i32),
    /// Every `n`th activation fires its payload twice.
    Echo(u32),
    /// A share of what your armour absorbs is handed straight back as armour,
    /// so a wall keeps rebuilding itself under fire.
    Bastion(i32),
    /// Landing a curse lands `n` more of the other kind alongside it.
    Contagion(u32),
    /// Everything runs `per_second` percent faster for every second the fight
    /// has been going, up to twice speed.
    ///
    /// The mirror of `Trundle`, and the pay-off for having asked rather than
    /// taken: a build that cannot win in the first ten seconds gets better the
    /// longer it is asked to keep going.
    Longhaul { per_second: i32 },
    /// Everything runs `slower` percent slower, and every scrap of armour you
    /// pick up counts `armour` percent.
    ///
    /// A trade rather than a gift, and the only class that changes the shape of
    /// a fight rather than its numbers.
    ///
    /// The slowdown started at fifty percent, which made it a tax rather than
    /// a trade: half the activations for plates worth double left armour per
    /// second exactly where it was and halved everything else. At twenty-five
    /// it buys about half again as much armour for a quarter less of
    /// everything, which is a decision.
    Trundle { slower: i32, armour: i32 },
    /// Every assembled item's assembly bonus counts `pct` percent more, per
    /// stack held.
    ///
    /// An assembly bonus is the flat lump a component pays only once its item
    /// comes together, so this rewards a board that finishes what it seats
    /// rather than one that fills cells with loose pieces - which is the
    /// difference between the two finished boards in `share`.
    ///
    /// Traded for, never poured: the pub takes a boss trophy for a stack, and
    /// nothing else in the game will take one at all.
    Recycler { pct: i32 },
    /// Start every fight with `n` devotion, per stack held.
    ///
    /// The first class in the game you can hold more than one of. Five of them
    /// are taken away and replaced with Ticket to Ride - see `Ticket`.
    Piety { faith: i32 },
    /// Start every fight `n` mana in debt, per stack held.
    ///
    /// Debt is mana below zero, so nothing that spends mana can pay until
    /// income has carried the pool back above the cost. A mana engine feels
    /// this immediately; a board that never spends mana does not feel it at
    /// all, which is the trade.
    Tired { mana: i32 },
    /// Every `nth` attack they make misses entirely.
    ///
    /// Written as a count rather than a chance because combat consults no
    /// RNG - a share code has to reproduce a fight exactly. Counting is also
    /// better than rolling here: it cannot streak, and half of everything is
    /// half of everything whether you fought for four seconds or forty.
    Ticket { nth: u32 },
    /// You do not heal. Regeneration on your gear stops working, for good.
    ///
    /// The only power in the game that is purely a cost, and it is meant to
    /// be: it is what you carry out of the VIP area for keeping your mouth
    /// shut. Nothing offers it - see `is_earned`.
    Guilt,
    /// Taking a hit banks faith, so being ground down is itself a resource.
    Reprisal(i32),
    /// Every enemy activation pushes all of your cooldowns forward by `ms`.
    Riposte(u32),
    /// Strength climbs by `per_sec` for every second the fight lasts.
    Momentum(i32),
    /// Reactions - the triggers that answer a neighbour or an aligned item -
    /// fire `n` times.
    Resonance(u32),
    /// A share of your physical damage lands again as magic, in percent.
    Transmute(i32),
    /// Every activation banks `n` of each of the four pools.
    Adaptable(i32),
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
    /// Every item takes `pct` of the best multiplier on the board on top of
    /// its own - the wisdom, split into pieces and handed round.
    Splintered(i32),
    /// Start every fight with `armor` per stack already on.
    ///
    /// The second stacking class, and the first one that is unambiguously
    /// good: Piety stacks into something else and Tired is a debt. This just
    /// accumulates, because a picket line honoured twice is two picket lines.
    Unionized { armor: i32 },
    /// A fight won inside `under_ms` pays `pct` more.
    ///
    /// Written to rhyme with the casino's door, which opens on a quick kill
    /// and is the only other thing in the game that rewards speed as such.
    Showstopper { pct: i32, under_ms: u32 },
    /// Named creatures leave `n` more pieces of their gear behind.
    ///
    /// A trophy is the only way any of that gear is ever obtainable, and it is
    /// one piece off a creature carrying fifteen. This is the only thing in
    /// the game that changes what a corpse is worth.
    Prospector(usize),
    /// Your first hit each fight cannot miss and cannot be turned aside.
    ///
    /// A rule rather than a number, and the one class that answers the two
    /// mechanics nothing else can be built against: Ticket to Ride eats every
    /// second swing and Deflection turns a flat share off every one. The first
    /// one goes through.
    FirstBlood,
    /// Mind damage you deal ignores this percentage of their mind resistance.
    ///
    /// The third lane had an amplifier, a pool and an answer, and no way at
    /// all through the answer - which the other two lanes have had since the
    /// day typed damage landed. This is that: piercing, for the one lane that
    /// did not have it, handed out by the one dungeon that is about seeing
    /// things the way a plane does not.
    WrongSense(i32),
    /// Start every fight already holding `n` rage. You came in angry.
    Avenged(i32),
}

impl ClassPower {
    /// This power, twice as strong - what the third fountain hands out.
    ///
    /// Every power doubles. Five of them used to be switches with nothing to
    /// turn, and the fountain simply did not offer those - which meant a
    /// player holding two of them never saw the third fountain at all, and
    /// nothing told them why. They all carry a number now.
    pub fn doubled(self) -> Option<ClassPower> {
        use ClassPower::*;
        // Guilt has no number to double, and doubling a cost would be a
        // fountain offering to make your run worse. It is not doublable.
        if matches!(self, Guilt) {
            return None;
        }
        Some(match self {
            Guilt => return None,
            // A town class is not something a fountain has in front of it, so
            // there is nothing for the doubling fountain to double.
            Piety { .. } | Tired { .. } | Ticket { .. } | Recycler { .. } => return None,
            // Doubling the slowdown as well as the armour would not be the
            // same bargain twice - it would be a different and much worse one.
            Trundle { .. } => return None,
            // Earned on the road, and doubling it would make the back half of
            // every fight a formality.
            Longhaul { .. } => return None,
            Standing(s) => Standing(s + s),
            Leeching(p) => Leeching(p * 2),
            Bastion(p) => Bastion(p * 2),
            Reprisal(n) => Reprisal(n * 2),
            Riposte(ms) => Riposte(ms * 2),
            Momentum(n) => Momentum(n * 2),
            Transmute(p) => Transmute(p * 2),
            Cascade(ms) => Cascade(ms * 2),
            Consecrate(p) => Consecrate(p * 2),
            Bloodscent(n) => Bloodscent(n * 2),
            Confluence(p) => Confluence(p * 2),
            Splintered(p) => Splintered(p * 2),
            WrongSense(p) => WrongSense((p * 2).min(100)),
            Prospector(n) => Prospector(n * 2),
            Unionized { armor } => Unionized { armor: armor * 2 },
            Showstopper { pct, under_ms } => Showstopper { pct: pct * 2, under_ms },
            // A switch has no second helping, so the fountain does not offer
            // it - `doubled` returning None is how that is said.
            FirstBlood => return None,
            Avenged(n) => Avenged(n * 2),
            // Twice as often, which for these means halving the interval.
            Echo(n) => Echo((n / 2).max(2)),
            Untimely(n) => Untimely((n / 2).max(2)),
            // The five that used to be switches rather than numbers. They
            // carry one now, because a fountain that cannot double what you
            // happen to be holding does not appear at all - and it did not say
            // so. A player who drank Geomancer and Wanderer simply never met
            // the third fountain.
            SlowTime(n) => SlowTime(n * 2),
            Contagion(n) => Contagion(n + 1),
            Resonance(n) => Resonance(n + 1),
            Adaptable(n) => Adaptable(n * 2),
            Overflowing(n) => Overflowing(n + 1),
        })
    }


    /// A few words for the side panel, where there is one line and it must
    /// not shrink to nothing. The full sentence lives in `describe`, which is
    /// what the glossary and the hover card show.
    pub fn short(self) -> String {
        match self {
            ClassPower::Guilt => "you cannot heal".to_string(),
            ClassPower::Recycler { pct } => format!("+{}% assembly bonuses", pct),
            ClassPower::Piety { faith } => format!("start with {} faith", faith),
            ClassPower::Tired { mana } => format!("start {} mana in debt", mana),
            ClassPower::Ticket { nth } => {
                format!("every {} attack on you misses", ordinal(nth))
            }
            ClassPower::Trundle { slower, armour } => {
                format!("{}% slower, {}% armour", slower, armour)
            }
            ClassPower::Longhaul { per_second } => {
                format!("{}% faster a second, to 2x", per_second)
            }
            ClassPower::Standing(s) => s.summary(),
            ClassPower::SlowTime(n) => format!("damage arrives over {}s", n),
            ClassPower::Leeching(pct) => format!("{}% of damage dealt heals you", pct),
            ClassPower::Overflowing(n) => format!("rage, faith and nature count {} times", n),
            ClassPower::Echo(n) => format!("every {}rd activation fires twice", n),
            ClassPower::Bastion(pct) => format!("armour returns {}% of what it soaks", pct),
            ClassPower::Contagion(n) => format!("每 curse brings {} more", n).replace("每", "every"),
            ClassPower::Reprisal(n) => format!("being hit banks {} faith", n),
            ClassPower::Riposte(ms) => {
                format!("their every act speeds you {:.2}s", ms as f32 / 1000.0)
            }
            ClassPower::Momentum(n) => format!("+{} strength a second elapsed", n),
            ClassPower::Resonance(n) => format!("reactions pay out {} times", n),
            ClassPower::Transmute(pct) => format!("{}% of iron lands again as magic", pct),
            ClassPower::Untimely(n) => format!("every {}th act stops their gear", n),
            ClassPower::Cascade(ms) => {
                format!("each act speeds the rest {:.2}s", ms as f32 / 1000.0)
            }
            ClassPower::Consecrate(pct) => format!("holding faith: {}% more armour", pct),
            ClassPower::Bloodscent(n) => format!("landing a curse banks {} rage", n),
            ClassPower::Confluence(pct) => format!("spending a pool refunds {}% to each other", pct),
            ClassPower::WrongSense(pct) => {
                format!("your mind damage pierces {}% of mind resist", pct)
            }
            ClassPower::Prospector(n) => format!("named creatures drop {} more piece(s)", n),
            ClassPower::Unionized { armor } => format!("start every fight with {} armor", armor),
            ClassPower::Showstopper { pct, under_ms } => {
                format!("+{}% bounty on a win under {}s", pct, under_ms / 1000)
            }
            ClassPower::FirstBlood => "your first hit each fight always lands".into(),
            ClassPower::Splintered(pct) => {
                format!("every item shares {}% of the best", pct)
            }
            ClassPower::Avenged(n) => format!("start every fight with {} fury", n),
            ClassPower::Adaptable(n) => format!("every act banks {} of all four pools", n),
        }
    }

    /// One sentence saying what actually happens.
    ///
    /// These are read by somebody deciding what to build, so vagueness is a
    /// bug: "held resources count double" tells nobody what a resource does
    /// held, and "armour is stronger where you already resist" described a
    /// rule that was never written. Name the numbers and the condition.
    pub fn describe(self) -> String {
        match self {
            ClassPower::Recycler { pct } => format!(
                "Every assembly bonus on your boards counts {}% more, for each stack of \
                 Recycler you are carrying. Five stacks is half again on all five slots. \
                 An assembly bonus is the lump a component pays only when its item comes \
                 together, so this pays a board that finishes what it seats.",
                pct
            ),
            ClassPower::Piety { faith } => format!(
                "Every fight starts with {} devotion already banked, for each stack of \
                 Piety you are carrying. Five stacks are taken away and given back as \
                 Ticket to Ride.",
                faith
            ),
            ClassPower::Tired { mana } => format!(
                "Every fight starts {} mana in debt, for each stack of Tired you are \
                 carrying. Debt is mana below zero: nothing that spends mana can pay \
                 until your income has carried the pool back above what it costs.",
                mana
            ),
            ClassPower::Ticket { nth } => format!(
                "Every {} attack made against you misses entirely - no damage, no curse, \
                 nothing. Counted rather than rolled, one count per attacker, so it is \
                 exactly half of everything and it never streaks.",
                ordinal(nth)
            ),
            ClassPower::Guilt => "Your regeneration is 0 a second for the rest of the run. \
                 Not slowed - stopped, whatever your gear says it heals for."
                .to_string(),
            ClassPower::Longhaul { per_second } => format!(
                "Everything you own runs {}% faster for every second the fight has been \
                 going, up to twice speed. A fight you cannot finish quickly is a fight \
                 you finish anyway.",
                per_second
            ),
            ClassPower::Trundle { slower, armour } => format!(
                "Every cooldown you own runs {}% slower, and every point of armour you gain \
                 counts {}%. Half the turns, twice the wall - which is a different game, not \
                 a better one.",
                slower, armour
            ),
            ClassPower::Standing(s) => s.summary(),
            ClassPower::SlowTime(n) => format!(
                "damage against you arrives in slices over {}s instead of all at once, so \
                 regeneration and armour get a chance to answer it",
                n
            ),
            ClassPower::Leeching(pct) => {
                format!("{}% of the damage you deal comes back to you as health", pct)
            }
            ClassPower::Overflowing(n) => format!(
                "rage, faith and nature count {} times over while you hold them - so {} times \
                 the physical damage from rage, the resistance from faith and the \
                 regeneration from nature",
                n, n
            ),
            ClassPower::Echo(n) => {
                format!("every {}rd time one of your items fires, it fires again immediately", n)
            }
            ClassPower::Bastion(pct) => format!(
                "whenever your armour soaks a hit, {}% of what it soaked is handed back as \
                 fresh armour",
                pct
            ),
            ClassPower::Contagion(n) => format!(
                "every curse you land brings its opposite with it {} time(s) over - searing \
                 pulls in frost, a stun pulls in a misfire",
                n
            ),
            ClassPower::Reprisal(n) => {
                format!("every hit that lands on you banks {} faith", n)
            }
            ClassPower::Riposte(ms) => format!(
                "every time they activate anything, all of your cooldowns jump {:.2}s closer \
                 to firing",
                ms as f32 / 1000.0
            ),
            ClassPower::Momentum(n) => format!(
                "+{} strength for every second the fight has lasted, so a long fight is one \
                 you get better at",
                n
            ),
            ClassPower::Resonance(n) => format!(
                "triggers that answer something else - a touching item firing, or one lined \
                 up across the grids - pay out {} times instead of once",
                n
            ),
            ClassPower::Transmute(pct) => format!(
                "{}% of every point of physical damage you deal lands a second time as \
                 magic, against their magic defences",
                pct
            ),
            ClassPower::Untimely(n) => format!(
                "every {}th time one of your items fires, their gear stops dead for {:.1}s \
                 and then misfires one activation in three for {:.0}s",
                n,
                crate::curse::STUN_MS as f32 / 1000.0,
                crate::curse::MISFIRE_MS as f32 / 1000.0,
            ),
            ClassPower::Cascade(ms) => format!(
                "every time one of your items fires, every OTHER item of yours jumps {:.2}s \
                 closer to firing",
                ms as f32 / 1000.0
            ),
            ClassPower::Consecrate(pct) => format!(
                "while you are holding any faith at all, every point of armour you gain is \
                 worth {}% more",
                pct
            ),
            ClassPower::Bloodscent(n) => {
                format!("every curse you land on them banks {} rage for you", n)
            }
            ClassPower::Confluence(pct) => format!(
                "whenever you spend one pool, {}% of what you spent is paid into each of the \
                 other three",
                pct
            ),
            ClassPower::Avenged(n) => format!(
                "you start every fight already holding {} rage - which is {} physical damage \
                 on every swing before anything has happened, and a pool to spend besides",
                n,
                n
            ),
            ClassPower::Splintered(pct) => format!(
                "whatever the strongest item on your board multiplies by, every other item \
                 takes {}% of that on top of its own - the wisdom split into pieces and \
                 handed round rather than kept",
                pct
            ),
            ClassPower::WrongSense(pct) => format!(
                "Mind damage you deal goes through {}% of whatever they have against it. \
                 Mind damage takes maximum health and none of it ever comes back, and \
                 mind resistance is the only thing standing in front of that - so this \
                 is the third lane's piercing, and until now the third lane had none.",
                pct
            ),
            ClassPower::Unionized { armor } => format!(
                "Every fight starts with {} armour already on, for each stack of Unionized \
                 you are carrying. Armour resets to zero at the start of every fight and \
                 soaks damage before health does, so this is the only thing in the game \
                 that hands you any of it before the first blow.",
                armor
            ),
            ClassPower::Showstopper { pct, under_ms } => format!(
                "A fight won in under {} seconds pays {}% more. Nothing else in the game \
                 rewards being quick except the casino's door, and that opens once.",
                under_ms / 1000,
                pct
            ),
            ClassPower::Prospector(n) => format!(
                "Every named creature leaves {} more piece(s) of its gear behind. A trophy \
                 is the only way any of that gear is ever obtainable - it is barred from \
                 every shelf in the game - and a boss is carrying fifteen items of it.",
                n
            ),
            ClassPower::FirstBlood => String::from(
                "The 1st hit you land in a fight cannot miss and cannot be turned aside. \
                 Ticket to Ride eats every 2nd swing and Deflection takes 10 off every one \
                 per stack; neither of them touches the first. It comes back every fight.",
            ),
            ClassPower::Adaptable(n) => format!(
                "every time any of your items fires, you bank {} of all four pools at once - \
                 {} mana, {} rage, {} faith, {} nature",
                n, n, n, n, n
            ),
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
        power: ClassPower::SlowTime(5),
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
        name: "Longhauler",
        blurb: "It got where it was going. So will you.",
        requires: &[],
        power: ClassPower::Longhaul { per_second: 4 },
    },
    ClassDef {
        // Claimed on the road, never poured: `is_earned` keeps it out of the
        // fountain ranking the same way the dungeon classes are kept out.
        name: "Trundle",
        blurb: "You learned the pace. Nothing hurries; nothing breaks.",
        requires: &[],
        power: ClassPower::Trundle { slower: 25, armour: 200 },
    },
    ClassDef {
        // Not on offer anywhere: `is_earned` keeps it out of the fountain
        // ranking, and the only way to hold it is to have walked past what
        // was going on in the back room.
        name: "Immense Guilt",
        blurb: "You saw. You said nothing. Nothing heals after that.",
        requires: &[],
        power: ClassPower::Guilt,
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
        power: ClassPower::Contagion(1),
    },
    ClassDef {
        name: "Druid",
        blurb: "Growth banked faster than anything can take it off you.",
        requires: &[(Axis::Growth, 45), (Axis::Ward, 25)],
        power: ClassPower::Overflowing(2),
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
        power: ClassPower::Resonance(2),
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
    // Only from the crevice under Corrqk's Cavern. It asks for nothing,
    // because nothing you build can qualify you for it - you have to go and
    // get it.
    ClassDef {
        name: "Ascendant",
        blurb: "What one part of you is worth, every part of you is worth a share of.",
        requires: &[],
        power: ClassPower::Splintered(50),
    },
    // Only from the man himself, and only from finishing him.
    ClassDef {
        name: "Avenged",
        blurb: "You did not come here to talk.",
        requires: &[],
        power: ClassPower::Avenged(2),
    },
    ClassDef {
        name: "Wanderer",
        blurb: "No particular commitment to anything, and a little of everything.",
        // The floor: something you can always reach, so a fountain is never
        // wasted on a build that matched nothing.
        requires: &[],
        power: ClassPower::Adaptable(1),
    },
    // ---- the three you can only pick up in a town ----
    //
    // None of these has requirements, because nothing you wear points at
    // them: they are places you went, not builds you made. `is_earned` keeps
    // them off the fountain.
    ClassDef {
        name: "Piety",
        blurb: "You knelt on the chapel floor in Sump Bottom, and it cut, and you stayed down.",
        requires: &[],
        power: ClassPower::Piety { faith: 1 },
    },
    ClassDef {
        name: "Ticket to Ride",
        blurb: "Five prayers in, somebody hands you a small printed card and will not say who from.",
        requires: &[],
        power: ClassPower::Ticket { nth: 2 },
    },
    ClassDef {
        name: "Tired",
        blurb: "You took the shift. They paid on the hour, in full, which is the part you keep telling people.",
        requires: &[],
        power: ClassPower::Tired { mana: 3 },
    },
    ClassDef {
        name: "Recycler",
        blurb: "You carried a Scrap Ticket into a bar and left with a way of looking at gear.",
        requires: &[],
        power: ClassPower::Recycler { pct: 10 },
    },
    // Only from the antechamber under the Manse. The class is the marker; the
    // prize is the pool, which is a thing a `ClassDef` cannot say and a
    // `Dungeon`'s `also` list can.
    ClassDef {
        name: "Threshold-Sighted",
        blurb: "You came back up seeing with the wrong sense, and it does not stop.",
        requires: &[],
        // The class is the marker and the pool is the prize, and a class that
        // is only a stat bundle is a class that says nothing. So it says the
        // one thing the antechamber is for: you came back seeing with the
        // wrong sense, and what you can see is *more* of what you are holding.
        power: ClassPower::WrongSense(60),
    },
    // The two the destinations pay. Neither is on any axis, because neither is
    // a way of building - one changes what a corpse leaves behind and one
    // changes what the first swing of a fight is worth, and no fountain has
    // anything to read.
    ClassDef {
        name: "Prospector",
        blurb: "You have learned what is worth prying off a thing that has stopped moving.",
        requires: &[],
        power: ClassPower::Prospector(1),
    },
    ClassDef {
        name: "Wumpus Hunter",
        blurb: "The first one counts. It counted for the wumpus and now it counts for you.",
        requires: &[],
        power: ClassPower::FirstBlood,
    },
    // Part D's two. Neither is on an axis: one is a promise you kept and one
    // is a thing you did in under ten seconds, and no fountain can read
    // either.
    ClassDef {
        name: "Unionized",
        blurb: "You did not cross. Nettle chalked six demands and one of them was about armour.",
        requires: &[],
        power: ClassPower::Unionized { armor: 20 },
    },
    ClassDef {
        name: "Showstopper",
        blurb: "They came to see a bout. You gave them an incident.",
        requires: &[],
        power: ClassPower::Showstopper { pct: 50, under_ms: 10_000 },
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
/// Is this class handed over rather than qualified for?
///
/// A dungeon reward or an event's spoils. Nothing you build points at one -
/// you go and get it, or you make the choice that earns it - so they are kept
/// out of the ranking entirely and a fountain can never pour one. They are
/// also the only classes allowed to ask for nothing, which is what the floor
/// class does, so every invariant about requirements has to know about them.
/// Classes handed out by a town, which no fountain offers and no build
/// qualifies for.
pub const TOWN_CLASSES: &[&str] = &["Piety", "Ticket to Ride", "Tired", "Recycler"];

/// Classes you can hold more than one of at once.
///
/// Every other class in the game is unique - the fountains never pour the same
/// one twice - which is why `simulate_party` can assign each power to its field
/// and be done. These two accumulate instead, so they are listed here rather
/// than discovered by reading the match arms.
pub fn stacks(name: &str) -> bool {
    // A picket line honoured twice is two picket lines.
    matches!(name, "Piety" | "Tired" | "Recycler" | "Unionized")
}

/// "2nd", "3rd", "4th". Only ever small numbers, so no special cases past the
/// teens are needed.
fn ordinal(n: u32) -> String {
    let suffix = match n % 10 {
        1 if n % 100 != 11 => "st",
        2 if n % 100 != 12 => "nd",
        3 if n % 100 != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

/// Where a class that no fountain pours actually comes from, in a few words.
///
/// The shelf used to print "asks for nothing" for these, which is true and
/// useless: a class you cannot build toward and cannot be told where to find
/// is a class the glossary has listed and not explained.
pub fn how_you_get_it(name: &str) -> Option<&'static str> {
    if !is_earned(name) {
        return None;
    }
    Some(match name {
        "Piety" => "prayed for, at a town chapel",
        "Ticket to Ride" => "five prayers, at a town chapel",
        "Tired" => "worked for, at a town factory",
        "Recycler" => "traded for at a town pub, one boss trophy a stack",
        _ if crate::dungeon::is_dungeon_only(name) => "carried out of a dungeon",
        _ => "taken at an event, off the road",
    })
}

/// Every class, in the order a share code writes them down.
///
/// This is the load-bearing detail of the whole sharing feature: a code stores
/// classes as positions in `CLASSES`, so the array is a wire format and not
/// just a list. Inserting a class anywhere but the end silently re-points
/// every code ever written - and quietly, because the code still reads, it
/// just names different classes.
///
/// That has happened once. The three town classes went in at the top, and from
/// then until it was noticed, every saved build reported somebody else's
/// titles. `A_FRIENDS_RUN` was shared during that window and had to be
/// re-pointed by hand.
///
/// Add a class by appending to `CLASSES` and appending here. Never reorder.
pub const CLASS_ORDER: &[&str] = &[
    "Chronomancer",
    "Archmage",
    "Berserker",
    "Longhauler",
    "Trundle",
    "Immense Guilt",
    "Bulwark",
    "Hexweaver",
    "Druid",
    "Templar",
    "Duelist",
    "Juggernaut",
    "Geomancer",
    "Spellblade",
    "Oracle",
    "Stormcaller",
    "Warpriest",
    "Bloodletter",
    "Wellspring",
    "Ascendant",
    "Avenged",
    "Wanderer",
    "Piety",
    "Ticket to Ride",
    "Tired",
    "Recycler",
    "Threshold-Sighted",
    "Prospector",
    "Wumpus Hunter",
    "Unionized",
    "Showstopper",
];

pub fn is_earned(name: &str) -> bool {
    if crate::dungeon::is_dungeon_only(name) || TOWN_CLASSES.contains(&name) {
        return true;
    }
    crate::event::EVENTS
        .iter()
        .any(|e| e.choices.iter().any(|c| claims(&c.outcome, name)))
}

/// Does this outcome hand over that class, at any depth?
///
/// `All` is a list of outcomes and a class claimed inside one is claimed just
/// as hard as a class claimed on its own - the exhibition bills you and then
/// starts the bout, in that order, in one choice. Written recursively because
/// `All` is the only nesting there is and flattening it by hand somewhere else
/// would be the same fact written down twice.
fn claims(outcome: &crate::event::Outcome, name: &str) -> bool {
    match *outcome {
        crate::event::Outcome::Claim(n) => n == name,
        // A class that comes bundled with a bargain is no more on offer
        // at a fountain than one you claim outright.
        crate::event::Outcome::Stock { class, .. } => class == name,
        crate::event::Outcome::All(each) => each.iter().any(|o| claims(o, name)),
        _ => false,
    }
}

pub fn rank(fp: &Fingerprint) -> Vec<Match> {
    let mut out: Vec<Match> = CLASSES
        .iter()
        // A dungeon class is not something a build can qualify for. Nothing
        // you wear points at it: you go and get it, or you never have it.
        .filter(|c| !is_earned(c.name))
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
mod axis_tests {
    use super::{Axis, CLASSES};

    /// The fountain tells you which axis you are short on. If the glossary
    /// cannot say what that axis is, the message is a number and a word the
    /// player has no way to look up.
    #[test]
    fn every_axis_the_fountain_can_name_is_explained() {
        for (name, text) in Axis::glossary() {
            assert!(!name.is_empty() && text.len() > 40, "{} is not explained", name);
        }
        // Every axis a class actually asks for has to be in that list, under
        // the name the class requirement prints.
        let listed: Vec<String> = Axis::glossary().into_iter().map(|(n, _)| n).collect();
        for class in CLASSES {
            for (axis, _) in class.requires {
                let name = match axis {
                    Axis::MagicIn(_) => "magic in a slot".to_string(),
                    Axis::PhysicalIn(_) => "iron in a slot".to_string(),
                    other => other.name(),
                };
                assert!(
                    listed.contains(&name),
                    "{} asks for {:?} and the glossary has no entry for it",
                    class.name,
                    axis
                );
            }
        }
    }
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
        // Dungeon classes are the exception and have to be: nothing you build
        // can qualify you for one, which is the point - you have to go and get
        // it. They are kept out of `rank`, so a fountain can never pour one.
        let floor: Vec<&str> = CLASSES
            .iter()
            .filter(|c| c.requires.is_empty() && !is_earned(c.name))
            .map(|c| c.name)
            .collect();
        assert_eq!(floor, vec!["Wanderer"], "exactly one class should be the fallback");
        assert_eq!(CLASSES.last().unwrap().requires.len(), 0, "and it must sort last");
    }

    /// A dungeon class must be unreachable by any amount of building, or the
    /// dungeon is not the only way to it.
    #[test]
    fn a_fountain_can_never_pour_an_earned_class() {
        let mut scores: Vec<(Axis, i32)> = Vec::new();
        for c in CLASSES {
            for &(a, _) in c.requires {
                scores.push((a, 100));
            }
        }
        let fp = Fingerprint { scores };
        for m in rank(&fp) {
            assert!(!is_earned(m.class.name), "{} turned up in the ranking", m.class.name);
        }
        assert!(!is_earned(classify(&fp).name));
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

#[cfg(test)]
mod order_tests {
    use super::*;

    #[test]
    fn the_class_order_is_append_only() {
        // `CLASSES` is a wire format: a share code stores a class as its
        // position in it. Reordering re-points every code anybody has saved,
        // and does it silently - the code still reads, it just hands back
        // different titles.
        let live: Vec<&str> = CLASSES.iter().map(|c| c.name).collect();
        for (i, name) in CLASS_ORDER.iter().enumerate() {
            assert_eq!(
                live.get(i),
                Some(name),
                "position {i} is {:?} and every share code written so far says it is {name:?}. \
                 Append new classes; never insert or reorder.",
                live.get(i)
            );
        }
        assert_eq!(
            live.len(),
            CLASS_ORDER.len(),
            "a class was added to CLASSES without being appended to CLASS_ORDER, so it \
             cannot be shared"
        );
    }
}
