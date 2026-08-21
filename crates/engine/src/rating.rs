//! How good is a piece of gear, in one number.
//!
//! Everything a component does falls into one of two shapes:
//!
//!   * a **standing** benefit - health, strength, regen, power, resistances.
//!     You have it for the whole fight whatever the item's cooldown is.
//!   * a **per-activation** benefit - damage, armour, mana, mind, and every
//!     trigger. You get it once each time the item goes off.
//!
//! The second kind is worthless without knowing how often the item fires, so
//! everything is converted to a per-second figure and added up. A chestpiece
//! granting 20 armour every 4 seconds and one granting 5 every second are the
//! same rating, which is the point: the number is meant to survive comparing
//! a weapon against a pair of greaves.
//!
//! The result is deliberately coarse. It drives a rarity badge, not a
//! simulation - the combat engine remains the authority on what actually
//! happens in a fight.

use crate::piece::{
    default_cooldown_ms, Action, Adjacency, Effect, EffectKind, PieceDef, PieceId, PieceKind,
    PieceRegistry, SlotKind, Trigger, When, CATALOG,
};
use crate::stats::Stats;
use std::sync::OnceLock;

/// Points per unit of each standing stat.
mod weight {
    /// Health is plentiful, so each point is worth little.
    pub const HEALTH: f32 = 0.55;
    /// Strength is added to every weapon hit before power multiplies it.
    pub const STRENGTH: f32 = 3.2;
    /// Regen already is a per-second figure.
    pub const REGEN: f32 = 5.0;
    /// Power is in hundredths: +100 is a whole extra multiple of weapon damage.
    pub const POWER: f32 = 0.45;
    pub const MIND_RESIST: f32 = 0.7;
    pub const CURSE_RESIST: f32 = 0.7;
    /// Resistance answers the two damage types most attacks are made of, so a
    /// point of it is worth more than a point of the niche resistances.
    pub const RESIST: f32 = 1.0;
    /// Piercing is only worth what the other side is resisting, and hardening
    /// only worth what they are piercing - both are situational, so both are
    /// discounted against flat resistance.
    pub const PIERCE: f32 = 0.5;
    pub const HARDEN: f32 = 0.55;

    /// Points per point-per-second of each activated stat.
    pub const DAMAGE_PS: f32 = 2.6;
    pub const ARMOR_PS: f32 = 1.5;
    pub const MANA_PS: f32 = 4.0;
    /// Rage, faith and nature are banked the same way mana is and, like mana,
    /// pay out while merely held. Worth the same per point.
    pub const RESOURCE_PS: f32 = 4.0;
    /// Mind damage eats maximum health, which regen can never win back.
    pub const MIND_PS: f32 = 7.0;

    /// A curse landed per second. Searing is a burn, frost is a slow; both are
    /// worth appreciably more than a point of damage.
    pub const CURSE_PS: f32 = 14.0;
    /// A second shaved off a cooldown, per second.
    pub const HASTE_PS: f32 = 9.0;
    /// A stack of empowerment or shield per second. Both scale off held mana,
    /// so their real worth depends on a build the rating cannot see; this is
    /// the value of a stack in a build that is actually banking mana.
    pub const STACK_PS: f32 = 11.0;

    /// Speed is a percentage on the whole item, so it is scored against
    /// whatever the item is already worth rather than on its own.
    pub const SPEED_PCT: f32 = 0.006;
}

/// The rating a slot's best possible item is worth. Everything is expressed
/// as a fraction of this, so the tiers mean the same thing in every slot.
///
/// Without it the badge would be dead weight on half the gear: a weapon holds
/// five components and a glove holds two, so their raw totals are not
/// comparable and one flat breakpoint would put every glove ever built in the
/// same tier as the worst weapon.
pub const FULL_MARKS: i32 = 200;

/// Raw points the best legal item in `slot` could reach, from the catalogue
/// and the slot's own recipe. Computed once and cached: it is a pure function
/// of `CATALOG`, but not a cheap one.
fn slot_ceiling(slot: SlotKind) -> f32 {
    static CEILINGS: OnceLock<[f32; 5]> = OnceLock::new();
    let all = CEILINGS.get_or_init(|| {
        let mut out = [1.0f32; 5];
        for s in SlotKind::ALL {
            // Across every recipe the slot offers, not just the first. The
            // weapon slot builds martial weapons and spells, and rating a
            // spell against a ceiling made of handles and blades would scale
            // it against a denominator it has nothing to do with.
            let mut ceiling = 0.0f32;
            for recipe in crate::piece::recipes(s) {
                let mut total = 0.0f32;
                for &(kind, _, max) in *recipe {
                    let mut best: Vec<f32> = CATALOG
                        .iter()
                        // `fits`, not `slot ==`: shared materials and plating
                        // are wearable here even though they are filed
                        // elsewhere, and a ceiling blind to them is too low.
                        .filter(|d| d.fits(s) && d.kind == kind && !crate::piece::is_boss_only(d.name))
                        .map(|d| piece_points(d, 0))
                        .collect();
                    best.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                    total += best.into_iter().take(max).filter(|v| *v > 0.0).sum::<f32>();
                }
                ceiling = ceiling.max(total);
            }
            out[s.index()] = ceiling.max(1.0);
        }
        out
    });
    all[slot.index()]
}

/// Rarity of an assembled item, from its total rating.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

/// The rating an assembled item must reach for each tier. Calibrated against
/// the catalogue: see the tests, which pin the shape of the distribution so a
/// batch of new components cannot quietly make everything legendary.
pub const RARE_AT: i32 = 90;
pub const EPIC_AT: i32 = 130;
pub const LEGENDARY_AT: i32 = 170;

impl Rarity {
    pub fn of(rating: i32) -> Rarity {
        if rating >= LEGENDARY_AT {
            Rarity::Legendary
        } else if rating >= EPIC_AT {
            Rarity::Epic
        } else if rating >= RARE_AT {
            Rarity::Rare
        } else {
            Rarity::Common
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
            Rarity::Legendary => "legendary",
        }
    }

    /// How many marks the badge carries: one for rare, two for epic, three for
    /// legendary. Nothing for common.
    pub fn marks(self) -> usize {
        match self {
            Rarity::Common => 0,
            Rarity::Rare => 1,
            Rarity::Epic => 2,
            Rarity::Legendary => 3,
        }
    }

    /// The rating at which the next tier starts, if there is one.
    pub fn next_at(self) -> Option<i32> {
        match self {
            Rarity::Common => Some(RARE_AT),
            Rarity::Rare => Some(EPIC_AT),
            Rarity::Epic => Some(LEGENDARY_AT),
            Rarity::Legendary => None,
        }
    }
}

/// Standing stats: worth the same however often the item fires.
fn standing_points(s: &Stats) -> f32 {
    s.health as f32 * weight::HEALTH
        + s.strength as f32 * weight::STRENGTH
        + s.regen as f32 * weight::REGEN
        + s.power as f32 * weight::POWER
        + s.mind_resist as f32 * weight::MIND_RESIST
        + s.curse_resist as f32 * weight::CURSE_RESIST
        + (s.physical_resist + s.magic_resist) as f32 * weight::RESIST
        + (s.physical_pierce + s.magic_pierce) as f32 * weight::PIERCE
        + (s.physical_harden + s.magic_harden) as f32 * weight::HARDEN
}

/// Stats granted once per activation, scored at `rate` activations a second.
fn activated_points(s: &Stats, rate: f32) -> f32 {
    ((s.damage + s.physical_damage + s.magic_damage) as f32 * weight::DAMAGE_PS
        + s.armor as f32 * weight::ARMOR_PS
        + s.mana as f32 * weight::MANA_PS
        + (s.rage + s.faith + s.nature) as f32 * weight::RESOURCE_PS
        + s.mind as f32 * weight::MIND_PS)
        * rate
}

/// What one action is worth each time it happens.
fn action_points(a: &Action) -> f32 {
    match a {
        Action::Curse { target, .. } => {
            // A curse on yourself is a cost, not a benefit.
            if matches!(target, crate::piece::Target::Yourself) {
                -weight::CURSE_PS
            } else {
                weight::CURSE_PS
            }
        }
        Action::Damage { amount, target } => {
            let v = *amount as f32 * weight::DAMAGE_PS;
            if matches!(target, crate::piece::Target::Yourself) {
                -v
            } else {
                v
            }
        }
        Action::MindDamage { amount, target } => {
            let v = *amount as f32 * weight::MIND_PS;
            if matches!(target, crate::piece::Target::Yourself) {
                -v
            } else {
                v
            }
        }
        Action::GainMana(n) => *n as f32 * weight::MANA_PS,
        // The other pools are each worth roughly what mana is: all four are
        // banked the same way and all four pay out while merely held.
        Action::Gain { amount, .. } => *amount as f32 * weight::RESOURCE_PS,
        Action::GainArmor(n) => *n as f32 * weight::ARMOR_PS,
        Action::ReduceCooldown(ms) => *ms as f32 / 1000.0 * weight::HASTE_PS,
        Action::GainEmpowerment(n) => *n as f32 * weight::STACK_PS,
        Action::GainShield(n) => *n as f32 * weight::STACK_PS,
    }
}

/// What one trigger is worth per activation of its item.
///
/// The conditional ones are discounted rather than guessed at: how often a
/// neighbour fires, or how many items touch this one, depends on a build this
/// function cannot see. The discounts are what a reasonable build gets.
fn trigger_points(t: &Trigger) -> f32 {
    match t {
        Trigger::OnActivate(a) => action_points(a),
        // Mana income is finite, so assume it pays about two thirds of the
        // time and eats the failure branch the rest.
        Trigger::Spend { cost, on_success, on_failure, .. }
        | Trigger::SpendMana { cost, on_success, on_failure } => {
            0.66 * action_points(on_success) + 0.34 * action_points(on_failure)
                - *cost as f32 * weight::MANA_PS * 0.66
        }
        // A piece with room around it touches one or two finished items.
        Trigger::PerAdjacentItem { action, .. } => 1.3 * action_points(action),
        // Reactions fire off someone else's cooldown, which is usually faster
        // than your own, but they need a neighbour to exist at all.
        Trigger::OnAdjacentActivate(a) => 1.1 * action_points(a),
        Trigger::OnAlignedActivate(a) => 0.9 * action_points(a),
        // Pays on every activation of the ball except its own turn in the
        // cycle - so on a three-spell ball, two casts out of three. Worth more
        // than a neighbour reaction because the item it waits on is itself.
        Trigger::OnOtherCast(a) => 1.5 * action_points(a),
    }
}

/// A positional effect's worth. These depend on where the piece sits, so each
/// gets the value of a fair placement rather than its best case.
fn effect_points(e: &Effect, rate: f32) -> f32 {
    let scale = match e.when {
        When::Always => 1.0,
        // Rating an item means rating it assembled.
        When::Assembled => 1.0,
        // Only pays while the item is *not* built, which is not the thing
        // being rated - but it is the whole point of those pieces, so it is
        // worth something rather than nothing.
        When::NotAssembled => 0.35,
    };
    let raw = match e.kind {
        // Worth roughly two neighbours of the right sort, which is what a
        // build that wants this effect will actually manage.
        EffectKind::SelfPerNeighborKind { per, stat, .. } => {
            2.0 * per as f32
                * match stat {
                    crate::stats::StatKind::Strength => weight::STRENGTH,
                    crate::stats::StatKind::Health => weight::HEALTH,
                    crate::stats::StatKind::Power => weight::POWER,
                    _ => 2.0,
                }
        }
        // Doubling a neighbour is worth about what a good neighbour carries.
        EffectKind::DoubleNeighbor { .. } => 16.0,
        EffectKind::DoubleAdjacentItemStat { .. } => 20.0,
        // A piece out in the open touches four or five empty cells.
        EffectKind::SelfPerEmptyCell { per, .. } => 4.5 * per as f32 * weight::STRENGTH,
        EffectKind::Flat { stats } => standing_points(&stats) + activated_points(&stats, rate),
    };
    raw * scale
}

fn adjacency_points(a: &Adjacency, rate: f32) -> f32 {
    standing_points(&a.stats) + activated_points(&a.stats, rate)
}

/// What one component is worth, assuming its item fires every `cooldown_ms`.
///
/// `cooldown_ms` of 0 means "use the slot's default", which is what a piece
/// gets rated at on a shop shelf, before you know what it will be built into.
fn piece_points(def: &PieceDef, cooldown_ms: u32) -> f32 {
    let cd = if cooldown_ms == 0 { default_cooldown_ms(def.slot) } else { cooldown_ms };
    let rate = 1000.0 / cd.max(1) as f32;

    let mut points = standing_points(&def.base) + activated_points(&def.base, rate);
    if let Some(adj) = def.adjacency {
        points += adjacency_points(&adj, rate);
    }
    if let Some(eff) = def.effect {
        points += effect_points(&eff, rate);
    }
    for t in def.triggers {
        points += trigger_points(t) * rate;
    }
    // Speed lifts everything the item does, so it is a percentage of the rest.
    points += points.abs() * def.speed_bonus as f32 * weight::SPEED_PCT;
    points
}

/// What one component contributes, on the shared scale where `FULL_MARKS` is
/// the best its slot can do. This is the figure the shop shows, and item
/// ratings are the sum of it - so a component's worth reads the same whether
/// you are looking at it on a shelf or in a finished item.
pub fn piece_rating_at(def: &PieceDef, cooldown_ms: u32) -> f32 {
    piece_rating_in(def, def.slot, cooldown_ms)
}

/// The same, scaled against the slot the piece is actually worn in.
///
/// A shared material or plating is filed under one slot but wearable in
/// another, and the two slots have different ceilings. Scaling it by where it
/// is filed rather than where it sits measures it against a denominator it has
/// nothing to do with - which is what pushed greaves 8 marks past the top of
/// the scale every other slot is held to. A piece that is worn nowhere yet
/// (in the shop, say) falls back to its home slot, which is the only answer
/// available before it is placed.
pub fn piece_rating_in(def: &PieceDef, slot: SlotKind, cooldown_ms: u32) -> f32 {
    piece_points(def, cooldown_ms) * FULL_MARKS as f32 / slot_ceiling(slot)
}

/// The same at the slot's default cadence, rounded.
pub fn piece_rating(def: &PieceDef) -> i32 {
    piece_rating_at(def, 0).round() as i32
}

/// What an assembled item made of `pieces` is worth, at the cadence it will
/// actually run at. The sum of what its components contribute, each measured
/// against the slot the item is worn in rather than where its piece is filed.
pub fn item_rating(
    reg: &PieceRegistry,
    pieces: &[PieceId],
    cooldown_ms: u32,
    slot: SlotKind,
) -> i32 {
    pieces
        .iter()
        .map(|&p| piece_rating_in(reg.def(p), slot, cooldown_ms))
        .sum::<f32>()
        .round() as i32
}

/// What the shop charges for a component, from what it is actually worth.
///
/// Deliberately steeper than linear: a component twice as effective is worth
/// far more than twice as much, because slots are scarce and the strong parts
/// are what a build is actually short of. A component good enough to carry an
/// item to legendary on its own costs a fortune.
pub fn shop_price(def: &PieceDef) -> i32 {
    let r = piece_rating(def).max(0) as f32;
    // 3 gold at nothing, ~14 at a middling 40, ~120 at a slot-carrying 140.
    (3.0 + (r / 9.0).powf(1.9)).round() as i32
}

/// Half of what it cost, rounded down - what selling one back pays.
pub fn resale_price(def: &PieceDef) -> i32 {
    shop_price(def) / 2
}

/// Every catalogue entry's rating, for calibration and for the tests.
pub fn catalog_ratings() -> Vec<(&'static str, SlotKind, PieceKind, i32)> {
    CATALOG
        .iter()
        .map(|d| (d.name, d.slot, d.kind, piece_rating(d)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tiers_are_ordered_and_the_marks_climb() {
        assert!(RARE_AT < EPIC_AT && EPIC_AT < LEGENDARY_AT);
        assert_eq!(Rarity::of(RARE_AT - 1), Rarity::Common);
        assert_eq!(Rarity::of(RARE_AT), Rarity::Rare);
        assert_eq!(Rarity::of(EPIC_AT), Rarity::Epic);
        assert_eq!(Rarity::of(LEGENDARY_AT), Rarity::Legendary);
        assert_eq!(Rarity::of(LEGENDARY_AT + 1000), Rarity::Legendary);

        let marks: Vec<usize> = [Rarity::Common, Rarity::Rare, Rarity::Epic, Rarity::Legendary]
            .iter()
            .map(|r| r.marks())
            .collect();
        assert_eq!(marks, vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_faster_item_rates_higher_for_the_same_payload() {
        // Two identical chestpieces, one firing twice as often.
        let def = CATALOG.iter().find(|d| d.base.armor > 0).expect("some piece grants armour");
        let slow = piece_rating_at(def, 4000);
        let fast = piece_rating_at(def, 2000);
        assert!(fast > slow, "{}: {} at 2s vs {} at 4s", def.name, fast, slow);
    }

    /// Best and worst legal item in a slot, by rating.
    fn slot_bounds(slot: SlotKind) -> (i32, i32) {
        let (mut best, mut worst) = (0, i32::MAX);
        for recipe in crate::piece::recipes(slot) {
            let (mut b, mut w) = (0, 0);
            for &(kind, min, max) in *recipe {
                let mut v: Vec<i32> = CATALOG
                    .iter()
                    .filter(|d| d.fits(slot) && d.kind == kind && !crate::piece::is_boss_only(d.name))
                    .map(|d| piece_rating_in(d, slot, 0).round() as i32)
                    .collect();
                v.sort_unstable();
                w += v.iter().take(min).sum::<i32>();
                b += v.iter().rev().take(max).filter(|r| **r > 0).sum::<i32>();
            }
            best = best.max(b);
            worst = worst.min(w);
        }
        (worst, best)
    }

    #[test]
    fn every_slot_can_reach_every_tier() {
        // The badge is dead weight in a slot whose best possible item cannot
        // clear the top breakpoint, or whose worst already clears the bottom
        // one. A glove holds two components and a weapon holds five, which is
        // exactly why the rating is scaled per slot.
        for slot in SlotKind::ALL {
            let (worst, best) = slot_bounds(slot);
            assert_eq!(
                Rarity::of(worst),
                Rarity::Common,
                "{}: the crudest legal item already rates {}",
                slot.name(),
                worst
            );
            assert_eq!(
                Rarity::of(best),
                Rarity::Legendary,
                "{}: the best possible item only rates {}",
                slot.name(),
                best
            );
        }
    }

    #[test]
    fn a_slots_ceiling_is_full_marks() {
        // What the scaling is for: the top of every slot lands in the same
        // place, so one set of breakpoints can serve all five.
        for slot in SlotKind::ALL {
            let (_, best) = slot_bounds(slot);
            assert!(
                (best - FULL_MARKS).abs() <= 3,
                "{} tops out at {}, not {}",
                slot.name(),
                best,
                FULL_MARKS
            );
        }
    }

    #[test]
    fn every_component_has_a_rating_and_none_of_them_is_absurd() {
        // Boss gear is exempt by design: it is meant to be off the scale, and
        // it is kept out of the ceiling so that being off the scale does not
        // drag every ordinary piece down with it.
        for (name, _, _, r) in catalog_ratings() {
            if crate::piece::is_boss_only(name) {
                continue;
            }
            assert!(
                (-40..=FULL_MARKS).contains(&r),
                "{} rates {}, outside anything a single component should reach",
                name,
                r
            );
        }
    }

    /// The point of the exemption: an absurd boss piece must not move the
    /// scale every other piece in its slot is measured against.
    #[test]
    fn boss_gear_does_not_move_the_scale_for_anything_else() {
        for name in crate::piece::BOSS_ONLY {
            let d = CATALOG.iter().find(|c| c.name == *name).expect("boss gear exists");
            let best = CATALOG
                .iter()
                .filter(|c| c.slot == d.slot && c.kind == d.kind && !crate::piece::is_boss_only(c.name))
                .map(piece_rating)
                .max()
                .unwrap_or(0);
            assert!(
                piece_rating(d) > best,
                "{} is not actually stronger than anything a player can buy",
                name
            );
            assert!(best <= FULL_MARKS, "the scale moved: best ordinary is {}", best);
        }
    }

    #[test]
    fn a_curse_on_yourself_counts_against_the_piece() {
        use crate::curse::CurseKind;
        use crate::piece::Target;
        let good = action_points(&Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Enemy,
        });
        let bad = action_points(&Action::Curse {
            kind: CurseKind::Searing,
            target: Target::Yourself,
        });
        assert!(good > 0.0 && bad < 0.0, "{} vs {}", good, bad);
    }
}

#[cfg(test)]
mod calib {
    use super::*;
    #[test]
    #[ignore]
    fn dump() {
        let mut v: Vec<&crate::piece::PieceDef> = CATALOG.iter().collect();
        v.sort_by_key(|d| (format!("{:?}", d.slot), -piece_rating(d)));
        for d in v {
            let w = d.cells.iter().map(|c| c.0).max().unwrap_or(0) + 1;
            let h = d.cells.iter().map(|c| c.1).max().unwrap_or(0) + 1;
            println!(
                "{:?}|{:?}|{}|r={}|cells={}|{}x{}|cd={}|spd={}",
                d.slot, d.kind, d.name, piece_rating(d), d.cells.len(), w, h,
                d.cooldown_ms, d.speed_bonus
            );
        }
    }
}
