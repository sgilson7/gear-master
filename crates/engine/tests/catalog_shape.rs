//! The five slots keep their shapes.
//!
//! `rating.rs` pins the rarity curve so a batch of new components cannot
//! quietly make everything legendary. This is the same idea aimed at identity:
//! so a batch of new components cannot quietly dissolve a slot back into a stat
//! pile. Each slot is meant to answer one question - weapon conversion, gloves
//! reaction, greaves tempo, chest reserve, helmet economy - and the rules below
//! are what "meant to" cashes out as.
//!
//! **How this file is red.** The spec asks for a test written red and made
//! green by the sweep. A suite that stays red for eight pull requests is not a
//! safety net, though - it is a light nobody looks at, and the sweep needs a
//! green suite to notice what it breaks. So the rules carry two numbers each: a
//! `budget`, which is how far the catalogue misses today, and a `target`, which
//! is where the rewrite has to get it. The default test asserts the budget and
//! is **green**; it can only ever be tightened, so a new off-axis piece fails it
//! immediately. `the_catalog_keeps_every_rule` asserts the targets, is
//! **ignored and red**, and is the one the sweep is finished by:
//!
//!     cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture
//!
//! Lower a budget in the same commit that earns it, the way this repo re-pins
//! anything else. Never raise one.
//!
//! **Floating kinds.** `PieceDef::fits` lets a Material sit in gloves or
//! greaves and a Plating in helmet or greaves - 61 pieces that can be placed
//! outside the slot they were written for. A rule keyed on `def.slot` therefore
//! cannot promise a mechanic stays in its grid, only that it was authored
//! there. So the two floating kinds are barred from carrying identity mechanics
//! at all (`identity_carriers`), which is what makes the rest of the table mean
//! something on the board rather than only in the source. They are the bleed
//! carriers: deliberately neutral, deliberately shared.

mod common;

use common::{does, has};
use gearmaster_engine::curse::CurseKind;
use gearmaster_engine::piece::{
    Action, EffectKind, PieceDef, PieceKind, SlotKind, Trigger, CATALOG,
};
use gearmaster_engine::rating::{piece_rating, Rarity};

// ------------------------------------------------------------- vocabulary

fn effect_is(def: &PieceDef, want: fn(&EffectKind) -> bool) -> bool {
    def.effect.as_ref().map(|e| want(&e.kind)).unwrap_or(false)
}

/// A kind that `PieceDef::fits` lets into a grid other than its own.
fn floats(kind: PieceKind) -> bool {
    matches!(kind, PieceKind::Material | PieceKind::Plating)
}

/// Anything that reads or answers the board rather than only adding to it.
fn interacts(def: &PieceDef) -> bool {
    def.effect.is_some()
        || def.adjacency.is_some()
        || has(def, |t| {
            matches!(
                t,
                Trigger::OnAdjacentActivate(_)
                    | Trigger::OnAlignedActivate(_)
                    | Trigger::PerAdjacentItem { .. }
                    | Trigger::PerAdjacentEmpty(_)
                    | Trigger::OnOtherCast(_)
                    // The two the interaction fabric added. A watcher reads
                    // the board's event stream and a diagonal reads past its
                    // neighbours; both are interactions, and leaving them out
                    // would have let a slot satisfy the density quota only in
                    // the vocabulary it had before the primitives landed.
                    | Trigger::OnDiagonalActivate(_)
                    | Trigger::Watch { .. }
            )
        })
}

fn spends_a_pool(def: &PieceDef) -> bool {
    has(def, |t| {
        matches!(t, Trigger::SpendMana { .. } | Trigger::Spend { .. } | Trigger::Consume { .. })
    })
}

/// No trigger, no effect, no adjacency bonus. A stat line and nothing else.
fn inert(def: &PieceDef) -> bool {
    def.triggers.is_empty() && def.effect.is_none() && def.adjacency.is_none()
}

fn rarity(def: &PieceDef) -> Rarity {
    Rarity::of(piece_rating(def))
}

// ------------------------------------------------------- the five axes
//
// "Every slot may do defence and every slot may do offence, but only in its own
// vocabulary." These are those vocabularies, and they are what the axis quotas
// count. A piece expresses an axis if it speaks any word of it.

fn conversion(def: &PieceDef) -> bool {
    def.power_bonus != 0
        || def.base.physical_damage != 0
        || def.base.magic_damage != 0
        || def.base.strength != 0
        || matches!(
            def.kind,
            PieceKind::Damaging
                | PieceKind::Spell
                | PieceKind::Ink
                | PieceKind::Alignment
                | PieceKind::Book
                | PieceKind::Orb
        )
        || does(def, |a| {
            // Mind damage counts. It is damage - it takes maximum health and
            // that health does not come back - and §2 names it as exactly the
            // helmet's bleed into the weapon: mind and magic as cast support.
            // Leaving it out meant the one slot whose bleed the spec spells
            // out could not express it.
            matches!(a, Action::Damage { .. } | Action::GainForking(_) | Action::MindDamage { .. })
        })
}

fn economy(def: &PieceDef) -> bool {
    def.base.mana != 0
        || def.base.mind_resist != 0
        || def.base.rage != 0
        || def.base.faith != 0
        || def.base.nature != 0
        || spends_a_pool(def)
        || does(def, |a| {
            matches!(
                a,
                Action::GainMana(_)
                    | Action::Gain { .. }
                    | Action::GainEmpowerment(_)
                    | Action::GainShield(_)
                    | Action::MindDamage { .. }
            )
        })
}

fn reserve(def: &PieceDef) -> bool {
    def.base.health != 0
        || def.base.armor != 0
        || def.base.regen != 0
        || def.base.physical_harden != 0
        || def.base.magic_harden != 0
        || does(def, |a| matches!(a, Action::Grow(_) | Action::GainArmor(_)))
}

fn reaction(def: &PieceDef) -> bool {
    has(def, |t| {
        matches!(
            t,
            Trigger::OnAdjacentActivate(_)
                | Trigger::OnAlignedActivate(_)
                | Trigger::PerAdjacentItem { .. }
        )
    }) || does(def, |a| matches!(a, Action::Drain { .. } | Action::StunStrongest { .. }))
        || effect_is(def, |e| {
            matches!(
                e,
                EffectKind::DoubleAdjacentItemStat { .. }
                    | EffectKind::DoubleNeighbor { .. }
                    | EffectKind::SelfPerNeighborKind { .. }
            )
        })
}

fn tempo(def: &PieceDef) -> bool {
    def.speed_bonus != 0
        || def.base.curse_resist != 0
        || has(def, |t| matches!(t, Trigger::OnBattleStart(_)))
        || does(def, |a| {
            matches!(a, Action::ReduceCooldown(_))
                || matches!(
                    a,
                    Action::Curse { kind: CurseKind::Frost | CurseKind::Stun | CurseKind::Misfire, .. }
                )
        })
}

/// Each slot's own axis, and the one it is allowed to bleed into. The bleed
/// relation is the directed cycle W -> G -> Gr -> C -> H -> W.
fn axes(slot: SlotKind) -> (fn(&PieceDef) -> bool, fn(&PieceDef) -> bool) {
    match slot {
        SlotKind::Weapon => (conversion, reaction),
        SlotKind::Gloves => (reaction, tempo),
        SlotKind::Greaves => (tempo, reserve),
        SlotKind::Chest => (reserve, economy),
        SlotKind::Helmet => (economy, conversion),
    }
}

// --------------------------------------------------------------- the rules

#[derive(Copy, Clone)]
enum Level {
    /// Only the home slot may carry it.
    Only,
    /// At least this percentage of instances live in the home slot.
    Mostly(usize),
}

/// One mechanic, where it belongs, and how far the catalogue is from putting it
/// there. `budget` is today's distance and `target` is the rewrite's.
struct Rule {
    what: &'static str,
    home: SlotKind,
    level: Level,
    /// Slots that may also carry it. The weapon keeps cadence tools it already
    /// had; the spec's wording is "outside the weapon slot".
    shared_with: &'static [SlotKind],
    budget: usize,
    target: usize,
    carries: fn(&PieceDef) -> bool,
}

impl Rule {
    /// Pieces that would have to change for this rule to hold, and their names.
    fn offenders(&self) -> Vec<&'static str> {
        let carried: Vec<&PieceDef> = CATALOG.iter().filter(|d| (self.carries)(d)).collect();
        let allowed = |s: SlotKind| s == self.home || self.shared_with.contains(&s);
        let mut out: Vec<&'static str> =
            carried.iter().filter(|d| !allowed(d.slot)).map(|d| d.name).collect();
        if let Level::Mostly(pct) = self.level {
            // A majority rule is not broken by any one piece - it is broken by
            // there being too many of them elsewhere. The distance is how many
            // would have to come home, so keep that many of the strays.
            let home = carried.iter().filter(|d| d.slot == self.home).count();
            let need = carried.len() * pct / 100;
            let must_move = need.saturating_sub(home);
            out.truncate(must_move);
        }
        out.sort_unstable();
        out
    }
}

const RULES: &[Rule] = &[
    // Weapon - Conversion. Most of this is already true and the test is here to
    // keep it true once 170 weapon pieces start being edited.
    Rule { what: "power_bonus", home: SlotKind::Weapon, level: Level::Only, shared_with: &[],
        budget: 0, target: 0, carries: |d| d.power_bonus != 0 },
    Rule { what: "the casting kinds (Ink/Spell/Alignment/Book/Orb)", home: SlotKind::Weapon,
        level: Level::Only, shared_with: &[], budget: 0, target: 0,
        carries: |d| matches!(d.kind, PieceKind::Ink | PieceKind::Spell | PieceKind::Alignment
            | PieceKind::Book | PieceKind::Orb) },
    Rule { what: "GainForking", home: SlotKind::Weapon, level: Level::Only, shared_with: &[],
        budget: 5, target: 0, carries: |d| does(d, |a| matches!(a, Action::GainForking(_))) },
    Rule { what: "OnOtherCast", home: SlotKind::Weapon, level: Level::Only, shared_with: &[],
        budget: 0, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::OnOtherCast(_))) },
    Rule { what: "PerAdjacentEmpty", home: SlotKind::Weapon, level: Level::Only, shared_with: &[],
        budget: 0, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::PerAdjacentEmpty(_))) },
    // Searing is damage wearing a curse costume, so it stays with the damage.
    Rule { what: "searing", home: SlotKind::Weapon, level: Level::Mostly(70), shared_with: &[],
        budget: 0, target: 0,
        carries: |d| does(d, |a| matches!(a, Action::Curse { kind: CurseKind::Searing, .. })) },

    // Helmet - Economy. What the pools are for.
    Rule { what: "Consume", home: SlotKind::Helmet, level: Level::Only, shared_with: &[],
        budget: 9, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::Consume { .. })) },
    Rule { what: "GainEmpowerment", home: SlotKind::Helmet, level: Level::Only, shared_with: &[],
        budget: 7, target: 0, carries: |d| does(d, |a| matches!(a, Action::GainEmpowerment(_))) },
    Rule { what: "GainShield", home: SlotKind::Helmet, level: Level::Only, shared_with: &[],
        budget: 7, target: 0, carries: |d| does(d, |a| matches!(a, Action::GainShield(_))) },
    Rule { what: "MindDamage", home: SlotKind::Helmet, level: Level::Only, shared_with: &[],
        budget: 9, target: 0, carries: |d| does(d, |a| matches!(a, Action::MindDamage { .. })) },
    Rule { what: "mind_resist", home: SlotKind::Helmet, level: Level::Only, shared_with: &[],
        budget: 4, target: 0, carries: |d| d.base.mind_resist != 0 },

    // Chest - Reserve. Outlasting is its offence.
    Rule { what: "Grow", home: SlotKind::Chest, level: Level::Only, shared_with: &[],
        budget: 10, target: 0, carries: |d| does(d, |a| matches!(a, Action::Grow(_))) },
    Rule { what: "harden", home: SlotKind::Chest, level: Level::Only, shared_with: &[],
        budget: 8, target: 0,
        carries: |d| d.base.physical_harden != 0 || d.base.magic_harden != 0 },
    Rule { what: "health above 15", home: SlotKind::Chest, level: Level::Mostly(70),
        shared_with: &[], budget: 30, target: 0, carries: |d| d.base.health > 15 },

    // Gloves - Reaction. The hands answer.
    Rule { what: "OnAdjacentActivate", home: SlotKind::Gloves, level: Level::Only, shared_with: &[],
        budget: 2, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::OnAdjacentActivate(_))) },
    Rule { what: "PerAdjacentItem", home: SlotKind::Gloves, level: Level::Only, shared_with: &[],
        budget: 0, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::PerAdjacentItem { .. })) },
    Rule { what: "Drain", home: SlotKind::Gloves, level: Level::Only, shared_with: &[],
        budget: 6, target: 0, carries: |d| does(d, |a| matches!(a, Action::Drain { .. })) },
    Rule { what: "StunStrongest", home: SlotKind::Gloves, level: Level::Only, shared_with: &[],
        budget: 0, target: 0, carries: |d| does(d, |a| matches!(a, Action::StunStrongest { .. })) },
    Rule { what: "DoubleAdjacentItemStat", home: SlotKind::Gloves, level: Level::Only,
        shared_with: &[], budget: 1, target: 0,
        carries: |d| effect_is(d, |e| matches!(e, EffectKind::DoubleAdjacentItemStat { .. })) },
    Rule { what: "OnAlignedActivate", home: SlotKind::Gloves, level: Level::Mostly(70),
        shared_with: &[], budget: 0, target: 0,
        carries: |d| has(d, |t| matches!(t, Trigger::OnAlignedActivate(_))) },

    // Greaves - Tempo. Who moves, how often, and first. The weapon keeps its
    // own cadence tools; everything else gives them up.
    Rule { what: "OnBattleStart", home: SlotKind::Greaves, level: Level::Only, shared_with: &[],
        budget: 9, target: 0, carries: |d| has(d, |t| matches!(t, Trigger::OnBattleStart(_))) },
    Rule { what: "speed_bonus outside the weapon", home: SlotKind::Greaves, level: Level::Only,
        shared_with: &[SlotKind::Weapon], budget: 10, target: 0, carries: |d| d.speed_bonus != 0 },
    // Gloves share this one. The bleed cycle has the hands bleeding into the
    // feet, and §3.4 names the piece that does it: a reaction whose payout is
    // tempo. Barring gloves outright made the slot's own designed bleed
    // illegal, which is the table being stricter than the cycle it encodes.
    Rule { what: "ReduceCooldown outside the weapon", home: SlotKind::Greaves, level: Level::Only,
        shared_with: &[SlotKind::Weapon, SlotKind::Gloves], budget: 0, target: 0,
        carries: |d| does(d, |a| matches!(a, Action::ReduceCooldown(_))) },
    // Terrain is the body's and the feet's: a thing to stand on, or ground to
    // cross. Nothing is laid under a helmet.
    Rule { what: "terrain", home: SlotKind::Chest, level: Level::Only,
        shared_with: &[SlotKind::Greaves], budget: 0, target: 0,
        carries: |d| d.kind.is_underlay() },
    Rule { what: "frost, stun and misfire", home: SlotKind::Greaves, level: Level::Mostly(70),
        shared_with: &[], budget: 7, target: 0,
        carries: |d| does(d, |a| matches!(a, Action::Curse {
            kind: CurseKind::Frost | CurseKind::Stun | CurseKind::Misfire, .. })) },
];

// --------------------------------------------------------------- the quotas

/// Which pieces of a slot a quota is taken over.
///
/// The spec words the density quotas as "each slot's above-common pieces" and
/// "every Epic or Legendary non-weapon piece", which reads as though component
/// rarity had a spread. It does not: `RARE_AT` is 90 on a scale where full
/// marks is the best a whole *item* can do, so a single component almost never
/// clears it and only **ten pieces in the catalogue of 446** are above Common
/// (helmet 2, chest 2, gloves 2, greaves 1, weapon 3). A quota over those ten
/// would be satisfied by editing ten pieces and would mean nothing.
///
/// So the intent - "the more a component is worth, the more it should interact"
/// - is kept and the measure is changed: the dearest third of each slot by
/// `piece_rating`. That is the same sentence in a currency the catalogue
/// actually has. The literal rarity rule survives as its own small test, which
/// costs nothing and starts meaning something the day component ratings spread.
#[derive(Copy, Clone)]
enum Scope {
    Whole,
    /// The top `n` percent of the slot by component rating.
    Dearest(usize),
}

/// A share of one slot that has to hold some property.
struct Quota {
    what: &'static str,
    slot: SlotKind,
    /// Inclusive percentage band the share must land in.
    want: (usize, usize),
    budget: usize,
    target: usize,
    holds: fn(&PieceDef) -> bool,
    scope: Scope,
}

impl Quota {
    fn pool(&self) -> Vec<&'static PieceDef> {
        let mut mine: Vec<&'static PieceDef> =
            CATALOG.iter().filter(|d| d.slot == self.slot).collect();
        match self.scope {
            Scope::Whole => mine,
            Scope::Dearest(pct) => {
                // Descending by rating, then by name so ties break the same way
                // on every run - a quota that shuffles under itself is not a
                // pin.
                mine.sort_by(|a, b| {
                    piece_rating(b).cmp(&piece_rating(a)).then_with(|| a.name.cmp(b.name))
                });
                mine.truncate((mine.len() * pct / 100).max(1));
                mine
            }
        }
    }

    /// How many pieces would have to change for the share to land in the band.
    fn distance(&self) -> usize {
        let pool = self.pool();
        if pool.is_empty() {
            return 0;
        }
        let held = pool.iter().filter(|d| (self.holds)(d)).count();
        let (lo, hi) = self.want;
        let least = pool.len() * lo / 100;
        let most = pool.len() * hi / 100;
        if held < least {
            least - held
        } else {
            held.saturating_sub(most)
        }
    }

    fn share(&self) -> f64 {
        let pool = self.pool();
        if pool.is_empty() {
            return 0.0;
        }
        100.0 * pool.iter().filter(|d| (self.holds)(d)).count() as f64 / pool.len() as f64
    }
}

/// The filler quota this rewrite has to reach is 30%. The one it is aiming at
/// afterwards is this, and getting there means writing mechanical content for
/// roughly a hundred and forty pieces - which is a job of its own, not a rider
/// on this one.
const EVENTUAL_FILLER_PCT: usize = 15;

/// How far each slot is from each quota today, read off `report_shape`. Lower a
/// figure in the commit that earns it; never raise one.
const QUOTA_BUDGETS: &[(SlotKind, &str, usize)] = &[
    (SlotKind::Helmet, "expresses its own axis", 3),
    (SlotKind::Helmet, "expresses its bleed axis", 6),
    (SlotKind::Helmet, "plain flat-stat filler", 0),
    (SlotKind::Helmet, "the dearest third interacts", 2),
    (SlotKind::Chest, "expresses its own axis", 0),
    (SlotKind::Chest, "expresses its bleed axis", 5),
    (SlotKind::Chest, "plain flat-stat filler", 0),
    (SlotKind::Chest, "the dearest third interacts", 0),
    (SlotKind::Chest, "pool-spend texture", 0),
    (SlotKind::Gloves, "expresses its own axis", 0),
    (SlotKind::Gloves, "expresses its bleed axis", 0),
    (SlotKind::Gloves, "plain flat-stat filler", 0),
    (SlotKind::Gloves, "the dearest third interacts", 0),
    (SlotKind::Gloves, "pool-spend texture", 0),
    (SlotKind::Greaves, "expresses its own axis", 9),
    (SlotKind::Greaves, "expresses its bleed axis", 22),
    (SlotKind::Greaves, "plain flat-stat filler", 5),
    (SlotKind::Greaves, "the dearest third interacts", 0),
    (SlotKind::Greaves, "pool-spend texture", 0),
    (SlotKind::Weapon, "the dearest third interacts", 0),
    (SlotKind::Weapon, "pool-spend texture", 5),
];

fn budget_for(slot: SlotKind, what: &str) -> usize {
    QUOTA_BUDGETS
        .iter()
        .find(|(s, w, _)| *s == slot && *w == what)
        .map(|(_, _, n)| *n)
        .unwrap_or_else(|| panic!("no budget recorded for {:?} {}", slot, what))
}

/// Built rather than declared, because every non-weapon slot gets the same four
/// quotas and only the axis differs. Spelling them out five times invites the
/// copy that says "gloves" and means greaves.
fn quotas() -> Vec<Quota> {
    let mut out = Vec::new();
    for slot in SlotKind::ALL {
        let (primary, bleed) = axes(slot);
        if slot != SlotKind::Weapon {
            let what = "expresses its own axis";
            out.push(Quota { what, slot, want: (60, 100),
                budget: budget_for(slot, what), target: 0, holds: primary, scope: Scope::Whole });
            let what = "expresses its bleed axis";
            out.push(Quota { what, slot, want: (20, 25),
                budget: budget_for(slot, what), target: 0, holds: bleed, scope: Scope::Whole });
            // The settled figure is 30% now and 15% when the rewrite is done.
            // Holding 15% from the start means writing mechanical content for
            // about 140 pieces before any axis lands, which is the wrong order.
            let what = "plain flat-stat filler";
            out.push(Quota { what, slot, want: (0, 30),
                budget: budget_for(slot, what), target: 0, holds: inert, scope: Scope::Whole });
        }
        // Part II's density quotas apply to every slot, weapon included.
        let what = "the dearest third interacts";
        out.push(Quota { what, slot, want: (35, 100),
            budget: budget_for(slot, what), target: 0, holds: interacts, scope: Scope::Dearest(33) });
        if slot != SlotKind::Helmet {
            let what = "pool-spend texture";
            out.push(Quota { what, slot, want: (0, 15),
                budget: budget_for(slot, what), target: 0, holds: spends_a_pool, scope: Scope::Whole });
        }
    }
    out
}

// ---------------------------------------------------------------- the tests

/// Identity mechanics may not ride a kind that can leave its grid.
///
/// This is the rule that makes the exclusivity table mean something on the
/// board. Without it "greaves-exclusive" is a claim about where a piece was
/// written, and a greaves Material carrying `OnBattleStart` sits in the gloves
/// grid making a liar of it.
fn identity_carriers() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for d in CATALOG.iter().filter(|d| floats(d.kind)) {
        for r in RULES {
            if (r.carries)(d) {
                out.push((d.name, r.what));
            }
        }
    }
    out.sort_unstable();
    out
}

/// Forty-three pieces of a floating kind carry something the table calls an
/// identity mechanic. Most are `health above 15` on a Material or Plating,
/// which the chest sweep takes; the rest go as their mechanic finds its home.
const FLOATING_CARRIER_BUDGET: usize = 43;

/// §10.2 as written: rarity buys interestingness. Only four non-weapon pieces
/// are above Common today, so this is a small rule - but it is the exact
/// sentence the spec asks for, and it costs nothing to hold.
fn dull_treasures() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = CATALOG
        .iter()
        .filter(|d| d.slot != SlotKind::Weapon)
        .filter(|d| matches!(rarity(d), Rarity::Epic | Rarity::Legendary))
        .filter(|d| !interacts(d))
        .map(|d| d.name)
        .collect();
    out.sort_unstable();
    out
}

const DULL_TREASURE_BUDGET: usize = 4;

#[test]
fn the_catalog_stays_within_its_budgets() {
    let mut over = Vec::new();
    for r in RULES {
        let n = r.offenders().len();
        if n > r.budget {
            over.push(format!(
                "{} ({:?}): {} pieces out of place, budget {} - {}",
                r.what,
                r.home,
                n,
                r.budget,
                r.offenders().join(", ")
            ));
        }
    }
    for q in quotas() {
        let d = q.distance();
        if d > q.budget {
            over.push(format!(
                "{:?} {}: {:.1}% against a wanted {}-{}%, {} pieces away, budget {}",
                q.slot, q.what, q.share(), q.want.0, q.want.1, d, q.budget
            ));
        }
    }
    let floating = identity_carriers();
    if floating.len() > FLOATING_CARRIER_BUDGET {
        over.push(format!(
            "{} floating pieces carry an identity mechanic, budget {} - {}",
            floating.len(),
            FLOATING_CARRIER_BUDGET,
            floating.iter().map(|(n, w)| format!("{n} ({w})")).collect::<Vec<_>>().join(", ")
        ));
    }
    let dull = dull_treasures();
    if dull.len() > DULL_TREASURE_BUDGET {
        over.push(format!(
            "{} epic or legendary non-weapon pieces do nothing positional, budget {} - {}",
            dull.len(),
            DULL_TREASURE_BUDGET,
            dull.join(", ")
        ));
    }
    assert!(over.is_empty(), "the catalogue moved away from its shape:\n  {}", over.join("\n  "));
}

#[test]
#[ignore]
fn the_catalog_keeps_every_rule() {
    // The finish line. Red until the sweep lands, and the thing that says it
    // has.
    let mut broken = Vec::new();
    for r in RULES {
        let o = r.offenders();
        if o.len() > r.target {
            broken.push(format!("{} belongs to {:?}: {}", r.what, r.home, o.join(", ")));
        }
    }
    for q in quotas() {
        if q.distance() > q.target {
            broken.push(format!(
                "{:?} {}: {:.1}%, wanted {}-{}%",
                q.slot, q.what, q.share(), q.want.0, q.want.1
            ));
        }
    }
    for (name, what) in identity_carriers() {
        broken.push(format!("{name} is a floating kind carrying {what}"));
    }
    for name in dull_treasures() {
        broken.push(format!("{name} is epic or better and does nothing positional"));
    }
    assert!(broken.is_empty(), "{} rules unmet:\n  {}", broken.len(), broken.join("\n  "));
}

#[test]
fn no_budget_is_slack() {
    // A budget above the real distance is a rule with nothing behind it: two
    // pieces could go off-axis before anything complained. So every budget has
    // to be exactly today's figure, which also means this test fails the moment
    // a sweep improves something - on purpose. It is the same re-pinning the
    // rarity distribution asks for, and the message says what to write.
    let mut slack = Vec::new();
    for r in RULES {
        let n = r.offenders().len();
        if n < r.budget {
            slack.push(format!("{} is budgeted {} and costs {n} - lower it", r.what, r.budget));
        }
    }
    for q in quotas() {
        let d = q.distance();
        if d < q.budget {
            slack.push(format!(
                "{:?} {} is budgeted {} and costs {d} - lower it",
                q.slot, q.what, q.budget
            ));
        }
    }
    if identity_carriers().len() < FLOATING_CARRIER_BUDGET {
        slack.push(format!(
            "FLOATING_CARRIER_BUDGET is {} and costs {} - lower it",
            FLOATING_CARRIER_BUDGET,
            identity_carriers().len()
        ));
    }
    if dull_treasures().len() < DULL_TREASURE_BUDGET {
        slack.push(format!(
            "DULL_TREASURE_BUDGET is {} and costs {} - lower it",
            DULL_TREASURE_BUDGET,
            dull_treasures().len()
        ));
    }
    assert!(
        slack.is_empty(),
        "the catalogue improved and the budgets did not follow:\n  {}",
        slack.join("\n  ")
    );
}

#[test]
fn every_rule_names_a_mechanic_that_exists() {
    // A rule matching nothing at all is a typo that would sit here reading
    // green forever.
    for r in RULES {
        assert!(
            CATALOG.iter().any(|d| (r.carries)(d)),
            "no piece in the catalogue carries {} - is the predicate right?",
            r.what
        );
    }
    for q in quotas() {
        assert!(!q.pool().is_empty(), "{:?} {} scores an empty pool", q.slot, q.what);
    }
}

#[test]
#[ignore]
fn report_shape() {
    println!("\n## Exclusivity - pieces out of place\n");
    println!("{:<44}{:>9}{:>9}{:>9}", "mechanic", "home", "away", "budget");
    for r in RULES {
        let carried = CATALOG.iter().filter(|d| (r.carries)(d)).count();
        let home = CATALOG.iter().filter(|d| (r.carries)(d) && d.slot == r.home).count();
        println!(
            "{:<44}{:>4}/{:<4}{:>9}{:>9}",
            r.what,
            home,
            carried,
            r.offenders().len(),
            r.budget
        );
    }

    println!("\n## Rarity of the catalogue, per slot\n");
    println!("{:<12}{:>9}{:>9}{:>9}{:>9}{:>9}", "slot", "common", "rare", "epic", "legend", "total");
    for slot in SlotKind::ALL {
        let mine: Vec<_> = CATALOG.iter().filter(|d| d.slot == slot).collect();
        let n = |r: Rarity| mine.iter().filter(|d| rarity(d) == r).count();
        println!(
            "{:<12}{:>9}{:>9}{:>9}{:>9}{:>9}",
            format!("{:?}", slot),
            n(Rarity::Common),
            n(Rarity::Rare),
            n(Rarity::Epic),
            n(Rarity::Legendary),
            mine.len()
        );
    }

    println!(
        "\n## Quotas  (filler is held at 30% for this rewrite, {}% after it)\n",
        EVENTUAL_FILLER_PCT
    );
    println!(
        "{:<12}{:<34}{:>7}{:>9}{:>11}{:>7}",
        "slot", "quota", "of", "share", "wanted", "away"
    );
    for q in quotas() {
        println!(
            "{:<12}{:<34}{:>7}{:>8.1}%{:>10}{:>7}",
            format!("{:?}", q.slot),
            q.what,
            q.pool().len(),
            q.share(),
            format!("{}-{}%", q.want.0, q.want.1),
            q.distance()
        );
    }

    let floating = identity_carriers();
    println!("\n## Identity mechanics on floating kinds: {}\n", floating.len());
    for (name, what) in &floating {
        println!("  {:<32}{}", name, what);
    }
}
