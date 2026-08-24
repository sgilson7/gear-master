//! The numbers before anything moves.
//!
//! Every acceptance criterion in the slot rewrite is a ratio against a figure
//! nobody has written down. The weapon's share of damage is meant to fall to
//! 55-65% from a baseline that was estimated rather than measured; early-game
//! time-to-kill is meant to stay within a fifth of a number that does not
//! exist yet. This file is where those numbers come from.
//!
//! Nothing here asserts a balance figure. A test that pinned the weapon at
//! today's share would go red on the first commit of the sweep and stay red
//! until the last, which is not a safety net but a nuisance. What is pinned is
//! the *method*: that attribution accounts for the damage it claims to, that
//! the reference boards still assemble into something that fights, and that
//! two replays of one board agree down to the millisecond. The figures
//! themselves are printed, and committed to `analysis/baseline.md`.
//!
//!     cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture

mod common;

use common::{does, has};
use gearmaster_engine::class::{ClassDef, ClassPower, CLASSES};
use gearmaster_engine::combat::{
    simulate_with_class, CombatLog, Difficulty, Event, MonsterSpec, Outcome, Side, LADDER,
};
use gearmaster_engine::loadout::{ItemProfile, Loadout};
use gearmaster_engine::piece::{Action, PieceKind, PieceRegistry, SlotKind, Trigger, CATALOG};
use gearmaster_engine::run::Run;
use gearmaster_engine::share;
use gearmaster_engine::stats::Stats;

/// Difficulty every measurement is taken at. A run opens here, so this is the
/// setting the balance story is actually told in.
const AT: Difficulty = Difficulty::Easy;

/// The ladder is indexed from zero and spoken about from one: the spec's
/// "rung 15 (The Hollow King)" is `LADDER[14]`, and its rungs 10, 25 and 40 are
/// indices 9, 24 and 39. Everything below prints the spoken number and works in
/// the index, because getting that backwards measures the wrong creature and
/// says so convincingly.
const HOLLOW_KING: usize = 14;

/// Rungs the spec asks for, plus the first, because "the early game feels the
/// same" is the one criterion a player would actually notice.
const AT_RUNGS: [usize; 4] = [0, 9, 24, 39];

fn rung(i: usize) -> String {
    format!("{} {}", i + 1, LADDER[i].name)
}

fn slot_ix(s: SlotKind) -> usize {
    SlotKind::ALL.iter().position(|&k| k == s).expect("every slot is in ALL")
}

fn short(s: SlotKind) -> &'static str {
    match s {
        SlotKind::Helmet => "helmet",
        SlotKind::Chest => "chest",
        SlotKind::Gloves => "gloves",
        SlotKind::Greaves => "greaves",
        SlotKind::Weapon => "weapon",
    }
}

// ------------------------------------------------------------- attribution

/// Damage the player dealt, split by which grid was acting when it landed.
///
/// The log never says "the greaves did this". It says an item activated, and
/// then it says a hit landed, and the order of those two lines is the whole of
/// the evidence: `Event::Activate` is documented to precede its own item's
/// effects, it carries that item's index, and `RunningItem` remembers which
/// grid the item came out of. So attribution is a walk that remembers the last
/// thing to activate and hands every hit that follows to it. No engine change
/// is needed to measure this, which is the reason to measure it this way.
///
/// Two kinds of damage will not sit in that scheme and are counted apart
/// rather than quietly folded in. A burn arrives on its own clock, a second at
/// a time, long after the activation that lit it - it is real damage with no
/// activation to pin it to. And an activation the walk never saw leaves its
/// hit unattributed, which should be nothing, and is checked.
///
/// The figure is the **swing**, not what survived the defences: `Event::Hit`
/// reports gross damage on purpose, and gross is what "which slot produces the
/// damage" wants. A slot's contribution should not read differently because
/// the thing in front of it happened to be armoured.
#[derive(Default, Clone, Debug)]
struct Damage {
    by_slot: [i64; 5],
    /// Damage from an item with no slot. Only monsters have those.
    slotless: i64,
    burn: i64,
    mind: [i64; 5],
    unattributed: i64,
}

impl Damage {
    fn attributed(&self) -> i64 {
        self.by_slot.iter().sum::<i64>() + self.slotless
    }

    /// Everything the walk saw, however it was classified.
    fn total(&self) -> i64 {
        self.attributed() + self.burn + self.unattributed
    }

    fn share(&self, s: SlotKind) -> f64 {
        let a = self.attributed();
        if a == 0 {
            return 0.0;
        }
        100.0 * self.by_slot[slot_ix(s)] as f64 / a as f64
    }
}

/// Walk one fight and split the player's output by slot.
fn attribute(log: &CombatLog) -> Damage {
    let mut d = Damage::default();
    // Which item the player last set going. `None` until the first activation,
    // which is the only window in which a hit can go unattributed.
    let mut acting: Option<usize> = None;

    for e in &log.entries {
        match &e.event {
            Event::Activate { side: Side::Player, index, .. } => acting = Some(*index),
            Event::Hit { by: Side::Player, damage, .. } => match acting
                .and_then(|i| log.player.items.get(i))
            {
                Some(item) => match item.slot {
                    Some(s) => d.by_slot[slot_ix(s)] += *damage as i64,
                    None => d.slotless += *damage as i64,
                },
                None => d.unattributed += *damage as i64,
            },
            Event::MindHit { by: Side::Player, amount, .. } => {
                if let Some(Some(s)) = acting.and_then(|i| log.player.items.get(i)).map(|i| i.slot)
                {
                    d.mind[slot_ix(s)] += *amount as i64;
                }
            }
            // A burn is logged against whoever is burning, so the player's
            // burn damage is the one landing on the other side.
            Event::Burn { side: Side::Enemy, damage, .. } => d.burn += *damage as i64,
            _ => {}
        }
    }
    d
}

// -------------------------------------------------------- reference builds

/// A board to measure, and what it is meant to represent.
///
/// The grids are kept rather than the stats and profiles they produce, because
/// the interesting question is what a build loses when a grid is emptied - and
/// emptying a grid takes the flat stats of every loose piece in it with the
/// items. Holding only the derived figures would answer a much weaker
/// question: what a build loses when a grid stops *acting* but goes on paying.
/// Half the armour game is flat stats on pieces that never assemble, so that
/// difference is most of the measurement.
struct Build {
    name: &'static str,
    note: &'static str,
    reg: PieceRegistry,
    lo: Loadout,
    classes: Vec<ClassDef>,
    /// Standing class bonuses, which sit outside the grids and so survive
    /// emptying one.
    standing: Stats,
}

impl Build {
    fn stats(&self) -> Stats {
        let mut s = self.lo.total_stats(&self.reg);
        s += self.standing;
        s
    }

    fn profiles(&self) -> Vec<ItemProfile> {
        self.lo.combat_items(&self.reg)
    }

    fn fight(&self, spec: &MonsterSpec) -> CombatLog {
        simulate_with_class(self.stats(), &self.profiles(), spec, AT, &self.classes)
    }

    /// The same build with one grid emptied, items and stats together - what
    /// the player would see having never bought anything for that slot.
    fn without(&self, s: SlotKind) -> Build {
        let mut lo = self.lo.clone();
        lo.slot_mut(s).clear();
        // A lock records the pieces of an item and how they sat. One naming a
        // piece that is no longer anywhere on the board would outlive the gear
        // it locked, so it goes when its gear does.
        let still_placed: std::collections::HashSet<_> =
            SlotKind::ALL.iter().flat_map(|&k| lo.slot(k).pieces()).collect();
        lo.locks.retain(|l| l.pieces.iter().all(|p| still_placed.contains(p)));
        Build {
            name: self.name,
            note: self.note,
            reg: self.reg.clone(),
            lo,
            classes: self.classes.clone(),
            standing: self.standing,
        }
    }
}

/// Look class names up and fold the standing ones into a bonus the grids
/// cannot take away, exactly as `Run::player_stats` does. A shared code records
/// the titles a run was played with, and measuring the board without them
/// measures a build nobody played.
fn classes_of(names: &[String]) -> (Stats, Vec<ClassDef>) {
    let mut standing = Stats::ZERO;
    let mut defs = Vec::new();
    for n in names {
        let Some(c) = CLASSES.iter().find(|c| c.name == n) else { continue };
        if let ClassPower::Standing(bonus) = c.power {
            standing += bonus;
        }
        defs.push(*c);
    }
    (standing, defs)
}

/// A board read back out of a shared run code.
fn from_code(name: &'static str, note: &'static str, code: &str) -> Build {
    let sh = share::import(code).expect("the code reads");
    let (reg, lo) = sh.loadout();
    let (standing, classes) = classes_of(&sh.classes);
    Build { name, note, reg, lo, classes, standing }
}

/// The four boards every figure below is taken from.
///
/// All four are fixtures the repo already keeps, rather than builds invented
/// for this harness. That is deliberate: a reference build assembled to taste
/// measures the taste. The starter kit is what the game hands you; the preset
/// is the engine's own auto-build, which `two_runs` already leans on because
/// hand-seating produces a weaker board than the one you meant; and the two
/// codes are the only complete runs any human has actually played here.
fn reference_builds() -> Vec<Build> {
    let mut out = Vec::new();

    // Rung one, as the game deals it: two pieces, one weapon, nothing else.
    let mut run = Run::new();
    let handle = run.find_by_name("Oak Handle").expect("starter handle");
    let blade = run.find_by_name("Iron Blade").expect("starter blade");
    run.equip(handle, SlotKind::Weapon, 0, 0).expect("handle seats");
    run.equip(blade, SlotKind::Weapon, 1, 0).expect("blade seats");
    out.push(Build {
        name: "starter",
        note: "the opening weapon and nothing else",
        reg: run.registry.clone(),
        lo: run.loadout.clone(),
        classes: Vec::new(),
        standing: Stats::ZERO,
    });

    // The engine's own reference board: all five grids, every adjacency lit.
    let mut run = Run::new();
    run.apply_preset();
    out.push(Build {
        name: "preset",
        note: "the auto-builder's five-slot board",
        reg: run.registry.clone(),
        lo: run.loadout.clone(),
        classes: Vec::new(),
        standing: Stats::ZERO,
    });

    out.push(from_code(
        "owner",
        "a finished run - 75 pieces, Berserker and Chronomancer",
        share::A_WINNING_RUN,
    ));
    out.push(from_code(
        "friend",
        "a finished run - 76 pieces, half of it deliberately loose",
        share::A_FRIENDS_RUN,
    ));
    out
}

// ------------------------------------------------------------- the census

/// Every mechanic the exclusivity table in the spec names, counted per slot.
///
/// Reproduced in Rust rather than left in a one-off script so it goes stale
/// loudly: when `catalog_shape.rs` starts asserting these, both will be
/// reading the same catalog through the same code.
#[derive(Default, Clone)]
struct Census {
    /// Per slot, in `SlotKind::ALL` order.
    rows: Vec<(&'static str, [usize; 5])>,
    total: [usize; 5],
}

fn census() -> Census {
    let mut c = Census::default();
    let mut rows: Vec<(&'static str, [usize; 5])> = Vec::new();
    let mut add = |label: &'static str, pick: &dyn Fn(&gearmaster_engine::piece::PieceDef) -> bool| {
        let mut row = [0usize; 5];
        for d in CATALOG {
            if pick(d) {
                row[slot_ix(d.slot)] += 1;
            }
        }
        rows.push((label, row));
    };

    add("pieces", &|_| true);
    add("inert (no trigger, effect or adjacency)", &|d| {
        d.triggers.is_empty() && d.effect.is_none() && d.adjacency.is_none()
    });
    add("positional (effect, adjacency or reaction)", &|d| {
        d.effect.is_some()
            || d.adjacency.is_some()
            || d.triggers.iter().any(|t| {
                matches!(
                    t,
                    Trigger::OnAdjacentActivate(_)
                        | Trigger::OnAlignedActivate(_)
                        | Trigger::PerAdjacentItem { .. }
                        | Trigger::PerAdjacentEmpty(_)
                )
            })
    });
    add("- effect", &|d| d.effect.is_some());
    add("- adjacency bonus", &|d| d.adjacency.is_some());

    add("curse application", &|d| does(d, |a| matches!(a, Action::Curse { .. })));
    add("- searing", &|d| {
        does(d, |a| {
            matches!(a, Action::Curse { kind: gearmaster_engine::curse::CurseKind::Searing, .. })
        })
    });
    add("- frost", &|d| {
        does(d, |a| {
            matches!(a, Action::Curse { kind: gearmaster_engine::curse::CurseKind::Frost, .. })
        })
    });
    add("- stun", &|d| {
        does(d, |a| {
            matches!(a, Action::Curse { kind: gearmaster_engine::curse::CurseKind::Stun, .. })
        })
    });
    add("- misfire", &|d| {
        does(d, |a| {
            matches!(a, Action::Curse { kind: gearmaster_engine::curse::CurseKind::Misfire, .. })
        })
    });

    add("reaction trigger", &|d| {
        has(d, |t| {
            matches!(
                t,
                Trigger::OnAdjacentActivate(_)
                    | Trigger::OnAlignedActivate(_)
                    | Trigger::PerAdjacentItem { .. }
            )
        })
    });
    add("- OnAdjacentActivate", &|d| {
        has(d, |t| matches!(t, Trigger::OnAdjacentActivate(_)))
    });
    add("- OnAlignedActivate", &|d| {
        has(d, |t| matches!(t, Trigger::OnAlignedActivate(_)))
    });
    add("- PerAdjacentItem", &|d| {
        has(d, |t| matches!(t, Trigger::PerAdjacentItem { .. }))
    });

    add("OnBattleStart", &|d| has(d, |t| matches!(t, Trigger::OnBattleStart(_))));
    add("Drain", &|d| does(d, |a| matches!(a, Action::Drain { .. })));
    add("StunStrongest", &|d| does(d, |a| matches!(a, Action::StunStrongest { .. })));
    add("Grow", &|d| does(d, |a| matches!(a, Action::Grow(_))));
    add("MindDamage", &|d| does(d, |a| matches!(a, Action::MindDamage { .. })));
    add("GainEmpowerment", &|d| does(d, |a| matches!(a, Action::GainEmpowerment(_))));
    add("GainShield", &|d| does(d, |a| matches!(a, Action::GainShield(_))));
    add("GainForking", &|d| does(d, |a| matches!(a, Action::GainForking(_))));
    add("ReduceCooldown", &|d| does(d, |a| matches!(a, Action::ReduceCooldown(_))));

    add("pool spend (SpendMana / Spend / Consume)", &|d| {
        has(d, |t| {
            matches!(
                t,
                Trigger::SpendMana { .. } | Trigger::Spend { .. } | Trigger::Consume { .. }
            )
        })
    });
    add("- Consume", &|d| has(d, |t| matches!(t, Trigger::Consume { .. })));

    add("power_bonus", &|d| d.power_bonus != 0);
    add("speed_bonus", &|d| d.speed_bonus != 0);
    add("mind_resist", &|d| d.base.mind_resist != 0);
    add("harden (physical or magic)", &|d| {
        d.base.physical_harden != 0 || d.base.magic_harden != 0
    });
    add("health above 15", &|d| d.base.health > 15);
    add("crosses grids (Material or Plating)", &|d| {
        matches!(d.kind, PieceKind::Material | PieceKind::Plating)
    });

    c.total = rows[0].1;
    c.rows = rows;
    c
}

// ------------------------------------------------------------- the pinning
//
// Three tests that hold the harness itself honest. None of them asserts a
// balance number - they assert that the instrument is reading something.

#[test]
fn attribution_accounts_for_every_blow_the_player_lands() {
    // If a hit ever precedes the first activation, the walk's premise is
    // wrong and every share below is quietly short. It has to be exactly zero
    // rather than merely small.
    for b in reference_builds() {
        for spec in LADDER.iter().take(25) {
            let log = b.fight(spec);
            let d = attribute(&log);
            assert_eq!(
                d.unattributed, 0,
                "{} vs {}: {} damage landed before anything activated",
                b.name, spec.name, d.unattributed
            );
        }
    }
}

#[test]
fn the_reference_boards_all_assemble_into_something_that_fights() {
    // A board that decodes into nothing would report a tidy 0% weapon share
    // and mean nothing at all.
    for b in reference_builds() {
        assert!(!b.profiles().is_empty(), "{} assembled no items", b.name);
        let log = b.fight(&LADDER[0]);
        assert!(
            attribute(&log).total() > 0,
            "{} fought the first rung and dealt nothing",
            b.name
        );
    }
}

#[test]
fn a_measured_fight_replays_identically() {
    // The whole harness rests on combat being a pure function of the two
    // boards. If it is not, every number below is a sample rather than a
    // measurement.
    for b in reference_builds() {
        for spec in [&LADDER[0], &LADDER[24], &LADDER[49]] {
            let (a, c) = (b.fight(spec), b.fight(spec));
            assert_eq!(a.duration_ms, c.duration_ms, "{} vs {}", b.name, spec.name);
            assert_eq!(a.outcome, c.outcome, "{} vs {}", b.name, spec.name);
            assert_eq!(a.entries.len(), c.entries.len(), "{} vs {}", b.name, spec.name);
            assert_eq!(
                format!("{:?}", attribute(&a)),
                format!("{:?}", attribute(&c)),
                "{} vs {}: the same fight attributed two ways",
                b.name,
                spec.name
            );
        }
    }
}

#[test]
fn the_census_agrees_with_the_catalog_it_counts() {
    let c = census();
    assert_eq!(c.total.iter().sum::<usize>(), CATALOG.len());
    // The row order is load-bearing for the printout; keep the first one first.
    assert_eq!(c.rows[0].0, "pieces");
}

// ------------------------------------------------------------- the reports

#[test]
#[ignore]
fn report_catalog_census() {
    let c = census();
    println!("\n## Catalog census - {} pieces\n", CATALOG.len());
    print!("{:<44}", "");
    for s in SlotKind::ALL {
        print!("{:>9}", short(s));
    }
    println!("{:>9}", "total");
    for (label, row) in &c.rows {
        print!("{:<44}", label);
        for v in row {
            print!("{:>9}", v);
        }
        println!("{:>9}", row.iter().sum::<usize>());
    }

    println!("\n### As a share of each slot\n");
    print!("{:<44}", "");
    for s in SlotKind::ALL {
        print!("{:>9}", short(s));
    }
    println!();
    for (label, row) in c.rows.iter().skip(1).take(4) {
        print!("{:<44}", label);
        for (i, v) in row.iter().enumerate() {
            print!("{:>8.1}%", 100.0 * *v as f64 / c.total[i] as f64);
        }
        println!();
    }
}

#[test]
#[ignore]
fn report_damage_share_and_ttk() {
    // Rungs the spec asks for, plus the first, because "the early game feels
    // the same" is the one criterion a player would actually notice.
    const AT_RUNGS: [usize; 4] = [0, 9, 24, 39];

    for b in reference_builds() {
        println!("\n## {} - {}\n", b.name, b.note);
        println!(
            "{:<22}{:>8}{:>10}{:>9}{:>9}{:>9}{:>9}{:>9}{:>8}",
            "rung", "result", "ttk", "helmet", "chest", "gloves", "greaves", "weapon", "burn"
        );
        for &r in &AT_RUNGS {
            let spec = &LADDER[r];
            let log = b.fight(spec);
            let d = attribute(&log);
            let outcome = match log.outcome {
                Outcome::Victory => "win",
                Outcome::Defeat => "loss",
                Outcome::Stalemate => "stale",
            };
            print!("{:<22}{:>8}{:>9.2}s", rung(r), outcome, log.duration_ms as f64 / 1000.0);
            for s in SlotKind::ALL {
                print!("{:>8.1}%", d.share(s));
            }
            let burn = if d.total() == 0 {
                0.0
            } else {
                100.0 * d.burn as f64 / d.total() as f64
            };
            println!("{:>7.1}%", burn);
        }
    }

    println!("\n## Weapon share across the whole ladder\n");
    println!("{:<12}{:>10}{:>12}{:>12}{:>10}", "build", "cleared", "weapon %", "median ttk", "burn %");
    let mut minds: Vec<(&'static str, [i64; 5])> = Vec::new();
    for b in reference_builds() {
        let mut cleared = 0;
        let mut ttks: Vec<f64> = Vec::new();
        let mut total = Damage::default();
        for spec in LADDER {
            let log = b.fight(spec);
            let d = attribute(&log);
            if log.outcome == Outcome::Victory {
                cleared += 1;
                ttks.push(log.duration_ms as f64 / 1000.0);
            }
            for i in 0..5 {
                total.by_slot[i] += d.by_slot[i];
                total.mind[i] += d.mind[i];
            }
            total.burn += d.burn;
            total.slotless += d.slotless;
        }
        ttks.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a duration"));
        let median = ttks.get(ttks.len() / 2).copied().unwrap_or(0.0);
        let burn = if total.total() == 0 {
            0.0
        } else {
            100.0 * total.burn as f64 / total.total() as f64
        };
        println!(
            "{:<12}{:>7}/{:<2}{:>11.1}%{:>11.2}s{:>9.1}%",
            b.name,
            cleared,
            LADDER.len(),
            total.share(SlotKind::Weapon),
            median,
            burn
        );
        minds.push((b.name, total.mind));
    }

    // Mind damage never appears in `Event::Hit` - it removes maximum health
    // instead, and cannot be healed - so none of the shares above can see it.
    // Reported apart rather than folded in, because a slot reading 0% of the
    // damage while quietly eating the enemy's health bar would be a fault in
    // the instrument rather than a finding about the slot.
    println!("\n## Mind damage across the whole ladder (max health removed, not in the shares above)\n");
    print!("{:<12}", "build");
    for s in SlotKind::ALL {
        print!("{:>9}", short(s));
    }
    println!();
    for (name, mind) in minds {
        print!("{:<12}", name);
        for s in SlotKind::ALL {
            print!("{:>9}", mind[slot_ix(s)]);
        }
        println!();
    }
}

#[test]
#[ignore]
fn report_slot_mattering() {
    // Criterion two, measured against today: strip one grid and see what the
    // build loses. Anything that only costs stats is a slot the player is
    // buying by the pound.
    for b in reference_builds().into_iter().filter(|b| b.profiles().len() > 3) {
        println!("\n## {} - time-to-kill with one grid emptied\n", b.name);
        print!("{:<24}{:>10}", "rung", "intact");
        for s in SlotKind::ALL {
            print!("{:>11}", short(s));
        }
        println!();

        for &r in AT_RUNGS.iter().skip(1) {
            let spec = &LADDER[r];
            let full = b.fight(spec);
            print!("{:<24}", rung(r));
            match full.outcome {
                Outcome::Victory => print!("{:>9.2}s", full.duration_ms as f64 / 1000.0),
                _ => print!("{:>10}", "-"),
            }
            for s in SlotKind::ALL {
                let log = b.without(s).fight(spec);
                match (full.outcome, log.outcome) {
                    (Outcome::Victory, Outcome::Victory) => {
                        let delta = 100.0
                            * (log.duration_ms as f64 - full.duration_ms as f64)
                            / full.duration_ms as f64;
                        print!("{:>10.0}%", delta);
                    }
                    (Outcome::Victory, _) => print!("{:>11}", "flips"),
                    _ => print!("{:>11}", "-"),
                }
            }
            println!();
        }
    }
}

#[test]
#[ignore]
fn report_no_weapon_viability() {
    // Criterion three: a best-effort build with an empty weapon grid clears
    // rung 15. Which is a question about the other four axes, so the answer
    // has to say *how* it was won - a fight the player survived into sudden
    // death is the clock killing the monster, not the gear.
    println!("\n## With the weapon grid emptied\n");
    println!("(rung {} is {})\n", HOLLOW_KING + 1, LADDER[HOLLOW_KING].name);
    println!(
        "{:<10}{:>11}{:>11}{:>12}{:>9}{:>24}",
        "build", "rungs won", "best rung", "rung 15", "ttk", "what carried it"
    );
    for b in reference_builds() {
        let bare = b.without(SlotKind::Weapon);
        let (mut won, mut best) = (0, None);
        for (i, spec) in LADDER.iter().enumerate() {
            if bare.fight(spec).outcome == Outcome::Victory {
                won += 1;
                best = Some(i + 1);
            }
        }

        let log = bare.fight(&LADDER[HOLLOW_KING]);
        let d = attribute(&log);
        // Burn is a contributor like any other here, and on a board with no
        // weapon it is often the only one - a searing curse goes on burning
        // whatever is left holding the leash. Leaving it out of the comparison
        // reported "nothing" for fights that were plainly won by something.
        let mut carried = "nothing".to_string();
        if log.entries.iter().any(|e| matches!(e.event, Event::SuddenDeath { .. })) {
            carried = "the clock, not the gear".into();
        } else if d.total() > 0 {
            let mut parts: Vec<(&str, i64)> =
                SlotKind::ALL.iter().map(|&s| (short(s), d.by_slot[slot_ix(s)])).collect();
            parts.push(("burn", d.burn));
            if let Some(&(label, amount)) = parts.iter().max_by_key(|(_, n)| *n).filter(|(_, n)| *n > 0)
            {
                carried = format!("{} {:.0}%", label, 100.0 * amount as f64 / d.total() as f64);
            }
        }
        println!(
            "{:<10}{:>8}/{:<2}{:>11}{:>12}{:>8.1}s{:>24}",
            b.name,
            won,
            LADDER.len(),
            best.map(|i| i.to_string()).unwrap_or_else(|| "none".into()),
            format!("{:?}", log.outcome),
            log.duration_ms as f64 / 1000.0,
            carried
        );
    }
}
