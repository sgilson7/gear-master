//! When each figure on a stat block actually happens.
//!
//! A `Stats` is not a block of passive numbers. Eight of its fields are handed
//! over on **every activation**, by the same code path an `OnActivate` trigger
//! uses, and for two missions every card in the game printed them beside
//! `+175 hp` as though they were the same kind of thing.
//!
//! Rootbound Material is the piece that gives it away, because its wording
//! says "each time its item fires" while its colour and its position say
//! otherwise. The other two hundred say nothing at all.
//!
//! The classification lives in `stats.rs` so three surfaces cannot disagree
//! about it. **This file is what stops it being a hand-written table that is
//! right for one mission**: it fires a probe item once and asks the fight
//! which figures moved.

mod common;

use gearmaster_engine::combat::{simulate_at, Difficulty, Event, Side};
use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_engine::run::Run;
use gearmaster_engine::stats::{Stats, When};

/// Every field `parts` can print says when it happens.
///
/// The cheap half, and the one that catches a field added later: a `Stats`
/// with everything set at once must classify every figure it prints.
#[test]
fn every_figure_a_stat_block_prints_says_when_it_happens() {
    let all = Stats {
        health: 1,
        strength: 1,
        regen: 1,
        power: 1,
        armor: 1,
        mana: 1,
        mind: 1,
        mind_resist: 1,
        curse_resist: 1,
        physical_damage: 1,
        magic_damage: 1,
        rage: 1,
        faith: 1,
        nature: 1,
        physical_resist: 1,
        physical_pierce: 1,
        physical_harden: 1,
        magic_resist: 1,
        magic_pierce: 1,
        magic_harden: 1,
        ..Stats::ZERO
    };
    let with = all.parts_when();
    assert_eq!(
        with.len(),
        all.parts().len(),
        "`parts` and `parts_when` disagree about how many figures a block has"
    );
    assert!(with.len() >= 20, "only {} figures classified, which is fewer than the fields", with.len());
    // And the two readings agree line for line, so `parts` cannot drift.
    for ((a, ga), (b, gb, _)) in all.parts().into_iter().zip(with) {
        assert_eq!(a, b, "the two walks printed different text");
        assert_eq!(ga, gb, "the two walks printed different glyphs");
    }
}

/// The classification is the fight's, not a table somebody kept up to date.
///
/// One item, fired once, and every figure marked `OnActivation` or `Damage`
/// has to be a figure the activation actually hands over. A hand-written
/// table would be wrong within two missions; this is wrong the moment the
/// fire path changes, which is when somebody should hear about it.
#[test]
fn what_fires_is_what_the_fight_hands_over() {
    // A pool that only a per-activation field can fill, on a board built to
    // do nothing else. `nature` is the one Rootbound Material is about.
    let mut run = Run::with_all_pieces();
    let id = |run: &Run, n: &str| {
        run.owned.iter().copied().find(|&p| run.registry.def(p).name == n).expect(n)
    };
    let a = id(&run, "Rootbound Material");
    let partner = CATALOG
        .iter()
        .find(|d| d.slot == SlotKind::Greaves && d.kind == gearmaster_engine::piece::PieceKind::Mold)
        .expect("a mold to build it with");
    let b = id(&run, partner.name);
    run.equip(a, SlotKind::Greaves, 0, 0).expect("seats");
    'seat: for y in 0..8u8 {
        for x in 0..6u8 {
            if run.equip(b, SlotKind::Greaves, x, y).is_ok() {
                if run.report(SlotKind::Greaves).assembled_count() > 0 {
                    break 'seat;
                }
                run.unequip(b);
            }
        }
    }
    assert!(
        run.report(SlotKind::Greaves).assembled_count() > 0,
        "the probe never assembled, so nothing fired and this proves nothing"
    );

    let spec = gearmaster_engine::combat::creature("Cave Rat").expect("exists");
    let log = simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium);

    // Nature arrived, more than once, which is the whole claim: it is a rate.
    let banked: Vec<i32> = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::GainResource { side: Side::Player, what: "nature", amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert!(
        !banked.is_empty(),
        "no nature was banked, so the probe is not exercising the path this file is about"
    );

    // And the classification agrees with what just happened.
    let nature = Stats { nature: 2, ..Stats::ZERO };
    let (_, _, when) = nature.parts_when().into_iter().next().expect("one figure");
    assert_eq!(
        when,
        When::OnActivation,
        "the fight hands nature over on every activation and the block says otherwise"
    );

    // The mirror: health is not handed over by firing, it is simply true.
    let health = Stats { health: 175, ..Stats::ZERO };
    let (_, _, when) = health.parts_when().into_iter().next().expect("one figure");
    assert_eq!(when, When::Passive, "health is not a per-activation figure");
}

/// Regen is the one most likely to be filed by eye, and it is passive.
///
/// It wears the leaf glyph, it is paid every second rather than every
/// activation, and it is the field a reader sorting these by feel gets wrong.
#[test]
fn regen_is_a_second_not_an_activation() {
    let s = Stats { regen: 4, ..Stats::ZERO };
    let (_, _, when) = s.parts_when().into_iter().next().expect("one figure");
    assert_eq!(when, When::Passive, "regen is per second and passive, whatever its glyph says");
}

/// Damage is its own group, and it is not the weapon's.
///
/// `item.mind` is handled outside the weapon branch precisely so a helmet can
/// reach you, so mind belongs in `Damage` from any slot.
#[test]
fn damage_is_its_own_group_and_mind_is_in_it() {
    for (s, what) in [
        (Stats { physical_damage: 9, ..Stats::ZERO }, "physical"),
        (Stats { magic_damage: 9, ..Stats::ZERO }, "magic"),
        (Stats { mind: 9, ..Stats::ZERO }, "mind"),
    ] {
        let (_, _, when) = s.parts_when().into_iter().next().expect("one figure");
        assert_eq!(when, When::Damage, "{what} damage is not in the damage group");
    }
    // And armour is not, though it arrives at the same moment.
    let armor = Stats { armor: 22, ..Stats::ZERO };
    let (_, _, when) = armor.parts_when().into_iter().next().expect("one figure");
    assert_eq!(when, When::OnActivation, "armour is not damage");
}

// ------------------------------------------------------------- T1, the audit

/// Every piece, every figure, and when it happens.
///
/// The list the owner asked for. Three things it names that nothing else does:
/// the pieces whose card shows a per-activation figure where a passive one
/// belongs, the pieces spelling one effect two ways, and the components
/// carrying damage the fight cannot land.
///
///     cargo test -p gearmaster-engine --test what_happens_when -- \
///         --ignored --nocapture audit
#[test]
#[ignore = "printer"]
fn audit() {
    use gearmaster_engine::piece::{Action, PieceKind, Resource, Trigger};

    println!("# What happens when — every piece in the catalogue\n");
    println!("Written by `what_happens_when::audit`. Do not hand-edit.\n");

    let mut per_activation = 0usize;
    let mut damage_carriers: Vec<&str> = Vec::new();
    let mut inert_damage: Vec<&str> = Vec::new();
    let mut both_spellings: Vec<&str> = Vec::new();
    let mut trigger_spelling: Vec<&str> = Vec::new();

    for d in CATALOG {
        let parts = d.base.parts_when();
        let has_act = parts.iter().any(|(_, _, w)| *w == When::OnActivation);
        let has_dmg = parts.iter().any(|(_, _, w)| *w == When::Damage);
        if has_act {
            per_activation += 1;
        }
        if has_dmg {
            damage_carriers.push(d.name);
            // Only a weapon swings. Mind lands from anywhere, so a piece whose
            // only damage is mind is not inert.
            let swings = d.slot == SlotKind::Weapon;
            let only_mind = d.base.physical_damage == 0 && d.base.magic_damage == 0;
            if !swings && !only_mind {
                inert_damage.push(d.name);
            }
        }
        let stat_pool = d.base.mana != 0 || d.base.rage != 0 || d.base.faith != 0 || d.base.nature != 0;
        let trig_pool = d.triggers.iter().any(|t| {
            matches!(
                t,
                Trigger::OnActivate(Action::Gain {
                    what: Resource::Mana | Resource::Rage | Resource::Faith | Resource::Nature,
                    ..
                }) | Trigger::OnActivate(Action::GainMana(_))
            )
        });
        if stat_pool && trig_pool {
            both_spellings.push(d.name);
        } else if trig_pool {
            trigger_spelling.push(d.name);
        }
    }

    println!("## The shape of it\n");
    println!("| | Pieces |");
    println!("|---|---:|");
    println!("| catalogue | {} |", CATALOG.len());
    println!("| carrying a per-activation figure in `Stats` | {per_activation} |");
    println!("| carrying a damage figure | {} |", damage_carriers.len());
    println!("| **carrying damage the fight cannot land** | **{}** |", inert_damage.len());
    println!("| granting a pool as a trigger, not a stat | {} |", trigger_spelling.len());
    println!("| spelling one pool grant both ways | {} |", both_spellings.len());

    println!("\n## Damage the fight cannot land\n");
    println!("Only a weapon swings (`loadout.rs`, `hit_for` returns 0 elsewhere), and");
    println!("`rating.rs` prices every point of this. Mind is exempt: it lands from any");
    println!("slot, which is why the helmets are not here.\n");
    for n in &inert_damage {
        let d = CATALOG.iter().find(|d| d.name == *n).expect("just walked it");
        println!("- {:24} {:8} {}", n, d.slot.name(), d.base.summary());
    }

    println!("\n## One effect, two spellings\n");
    println!("`Stats {{ nature: 2 }}` and `OnActivate(Gain {{ Nature, 2 }})` are the same");
    println!("thing to the fight. These say it as a trigger:\n");
    for n in &trigger_spelling {
        println!("- {n}");
    }
    println!("\nAnd these say it both ways at once, so their card adds the two together:\n");
    for n in &both_spellings {
        println!("- {n}");
    }

    println!("\n## Every piece\n");
    for d in CATALOG {
        let parts = d.base.parts_when();
        let group = |w: When| -> Vec<String> {
            parts.iter().filter(|(_, _, x)| *x == w).map(|(t, ..)| t.clone()).collect()
        };
        let trig: Vec<String> = d
            .triggers
            .iter()
            .filter(|t| !matches!(t, Trigger::OnActivate(_)))
            .map(|t| t.describe())
            .collect();
        let on_act: Vec<String> = d
            .triggers
            .iter()
            .filter_map(|t| match t {
                Trigger::OnActivate(a) => Some(a.describe()),
                _ => None,
            })
            .collect();
        let kind = if d.kind == PieceKind::Quest { "quest" } else { d.kind.name() };
        println!("\n### {} — {} {}", d.name, d.slot.name(), kind);
        for (label, v) in [
            ("DAMAGE", group(When::Damage)),
            ("PASSIVE", group(When::Passive)),
            ("EVERY TIME IT FIRES", group(When::OnActivation)),
        ] {
            if !v.is_empty() {
                println!("  {label}: {}", v.join(", "));
            }
        }
        if !on_act.is_empty() {
            println!("  EVERY TIME IT FIRES (triggered): {}", on_act.join("; "));
        }
        if !trig.is_empty() {
            println!("  TRIGGERS: {}", trig.join("; "));
        }
    }
}

/// One effect, one spelling. Budget zero, and it may not rise.
///
/// `Stats { nature: 2 }` and `OnActivate(Gain { Nature, 2 })` hand over the
/// same amount, so the catalogue says it one way. Not because the alternative
/// was wrong, but because two spellings meant every reader had to know both -
/// and three of them did not. `Figures::of` reads `stats.mana` and nothing
/// else, so eighteen pieces' worth of mana a second was invisible to every
/// toll in the county until this was folded.
///
/// `Action::Gain` stays in the language. It is still how a *conditional*
/// trigger grants a pool - `Consume`, `Watch`, `SpendMana`, `PerAdjacentEmpty`
/// - and those are a different claim: they happen sometimes.
#[test]
fn a_pool_grant_has_one_spelling() {
    use gearmaster_engine::piece::{Action, Resource, Trigger};
    let mut both = Vec::new();
    for d in CATALOG {
        let top_level_gain = d.triggers.iter().any(|t| {
            matches!(
                t,
                Trigger::OnActivate(Action::Gain {
                    what: Resource::Mana | Resource::Rage | Resource::Faith | Resource::Nature,
                    ..
                }) | Trigger::OnActivate(Action::GainMana(_))
            )
        });
        if top_level_gain {
            both.push(d.name);
        }
    }
    assert!(
        both.is_empty(),
        "these grant a pool as an unconditional trigger rather than as a stat: {both:?}. \
         The two are identical in amount and not in reading - see \
         `analysis/second-order.md` 29 for the two places they were not identical at all."
    );
}

/// Every surface files a figure under the same group.
///
/// The disagreement this whole mission is about: the piece card put an
/// unconditional pool gain in the stat block, the item card folded it into an
/// IN COMBAT figure, and the CLI printed one flat line for both. Three
/// readings of one number.
///
/// The engine settles it, so the test is that the engine's own two readings
/// cannot come apart - `summary_by_when` groups exactly what `parts_when`
/// classifies, and `summary` is still every figure with nothing left out.
#[test]
fn the_groups_and_the_flat_summary_describe_the_same_block() {
    for d in CATALOG {
        let grouped = d.base.summary_by_when();
        let flat = d.base.summary();
        // Nothing invented: every group's text appears in the flat reading.
        for (_, text) in &grouped {
            for figure in text.split(", ") {
                assert!(
                    flat.contains(figure),
                    "{}: grouped says {figure:?} and the flat summary does not",
                    d.name
                );
            }
        }
        // Nothing lost: as many figures grouped as the block prints.
        let grouped_count: usize =
            grouped.iter().map(|(_, t)| t.split(", ").count()).sum();
        assert_eq!(
            grouped_count,
            d.base.parts().len(),
            "{}: {} figures grouped against {} printed",
            d.name,
            grouped_count,
            d.base.parts().len()
        );
    }
}

/// A piece with nothing to say has no groups at all.
///
/// 29 pieces are shape and a price. A card that headed four empty groups for
/// them would be the rewrite making the common case worse.
#[test]
fn a_piece_with_nothing_to_say_heads_nothing() {
    let empty = Stats::ZERO;
    assert!(empty.summary_by_when().is_empty(), "an empty block grew a group");
    let quiet = CATALOG.iter().filter(|d| d.base == Stats::ZERO && d.triggers.is_empty()).count();
    assert!(quiet > 0, "no piece is quiet any more, so this test guards nothing");
}
