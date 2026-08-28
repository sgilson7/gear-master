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
    default_cooldown_ms, Action, AssemblyBonus, Effect, EffectKind, PieceDef, PieceId, PieceKind,
    PieceRegistry, SlotKind, Trigger, When, CATALOG,
};
use crate::stats::Stats;
use std::sync::OnceLock;

/// Points per unit of each standing stat.
mod weight {
    /// Health is plentiful, so each point is worth little - and since every
    /// health bonus in the game was multiplied by five, each point is now
    /// worth a fifth of what it was. Without this the scale would have tipped
    /// entirely: health-heavy pieces set the ceiling, and everything that was
    /// not health deflated against them.
    pub const HEALTH: f32 = 0.11;
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
    /// Reflection returns a percentage of what your armour ate, as damage, to
    /// whoever swung.
    ///
    /// Priced off a stated assumption rather than a feel, the way
    /// `EXPECTED_COVERAGE` is: a board built to be hit soaks something like six
    /// hundred points into armour over a fight, so one percent of reflect sends
    /// six of them back - a tenth of a point a second across a sixty-second
    /// fight, which at `DAMAGE_PS` is about a quarter of a point of worth.
    ///
    /// It had **no weight at all**. Seventeen chest pieces carry it and every
    /// one was priced as though its only offensive verb did not exist - which
    /// also meant `stepped_component` sorted them to the bottom of their
    /// footprint families and the interaction quotas, which measure the dearest
    /// third of a slot, could not see them.
    pub const REFLECT: f32 = 0.26;

    /// Points per point-per-second of each activated stat.
    pub const DAMAGE_PS: f32 = 2.6;
    pub const ARMOR_PS: f32 = 1.5;
    pub const MANA_PS: f32 = 4.0;
    /// Rage, faith and nature are banked the same way mana is and, like mana,
    /// pay out while merely held. Worth the same per point when spent.
    pub const RESOURCE_PS: f32 = 4.0;
    /// What one point of a banked pool is worth for the time you sit on it,
    /// before anything spends it.
    ///
    /// This was missing entirely, and it is most of what these pools are: a
    /// point of faith is a point of both resistances for the rest of the
    /// fight, a point of nature is a point of regeneration, a point of rage is
    /// a point on every physical swing. Leaving it out priced every pool
    /// piece in the game at nearly nothing - every faith-carrying component
    /// rated between 0 and 13, which put all of them outside the top of their
    /// own bucket and made two classes look unreachable.
    ///
    /// Held against the naive figure: a pool climbs all fight, so the average
    /// holding is about half the total banked, and triggers spend some of it
    /// back down. A quarter of the accumulation is the honest discount.
    pub const HELD_SHARE: f32 = 0.25;
    /// Faith is a point of each resistance; nature a point of regen; rage a
    /// point on every swing. Averaged, since a piece grants one pool and the
    /// rating is one number.
    pub const HELD_PER_POINT: f32 = 2.6;
    /// Hundredths of weapon power that apply to this item alone.
    ///
    /// Worth less than the wearer's own `POWER`, which multiplies everything
    /// they hold - but not much less on the piece that carries the payload,
    /// which for a caster is the whole item. This was not scored at all, so
    /// every ink in the game was priced as though it were blank while being
    /// the largest damage multiplier available.
    pub const POWER_BONUS: f32 = 0.30;
    /// Mind damage eats maximum health, which regen can never win back.
    pub const MIND_PS: f32 = 7.0;

    /// A curse landed per second. Searing is a burn, frost is a slow; both are
    /// worth appreciably more than a point of damage.
    pub const CURSE_PS: f32 = 14.0;
    /// What a second of the other side's gear being stopped is worth.
    ///
    /// Curses were all priced at `CURSE_PS`, whichever one they were. That was
    /// survivable while only searing and frost existed; nineteen pieces now
    /// land stun or misfire, and those deny output rather than dealing damage.
    /// Frost is half a second of slow, a stun is 1.2 seconds of nothing, and a
    /// misfire is a third of six seconds - so they are worth 0.5, 1.2 and 2.0
    /// seconds of denial respectively, and pricing them the same made the two
    /// best curses in the game the two cheapest.
    pub const DENIAL_S: f32 = 13.0;
    /// What choosing the target multiplies a stun by. Against a five-item
    /// board a blind stun finds the item that mattered one time in five; this
    /// is not five, because the other four are not worthless and because a
    /// price nobody can pay is not a price.
    pub const AIMED: f32 = 2.4;
    /// A second shaved off a cooldown, per second.
    pub const HASTE_PS: f32 = 9.0;
    /// A second of cooldown *moved* from one item to another, per second.
    ///
    /// Time is conserved, so the naive price is zero. What is bought is where
    /// the second is spent: a second on a 5,000 ms chest item is worth more
    /// than a second on a 1,500 ms weapon by roughly the ratio of what those
    /// items carry. A third of `HASTE_PS`, and the third is the discount for
    /// the fact that the rating cannot see which neighbour will be found - or
    /// whether there will be one at all.
    pub const SHUNT_PS: f32 = 3.0;
    /// The share of `Grow` that a `Ballast` of the same size is worth.
    ///
    /// **Measured, at M8.** It was two-thirds - the flat discount every
    /// condition in this file used to take for "what a build that wanted it
    /// will actually manage" - and fighting it says 0.87.
    ///
    /// The measurement is `prices::report_what_the_two_conditionals_actually_
    /// manage`: a chest item asking for 10, 20 or 30 against a rung-29
    /// creature, over three sizes of armour income, spends all of what it asks
    /// for in seven of nine configurations and a third to a half in the two
    /// where the income cannot keep up. The mean is 0.87.
    ///
    /// The number it is *not* is zero, which is what the same probe read on
    /// its first run: a wall granted once at the bell is gone before a
    /// five-second chest item comes round, because the creature is hitting
    /// you. A build that wants Ballast wants armour income, and that is the
    /// build this discount is for.
    pub const BALLAST_FUNDED: f32 = 0.87;
    /// How often a `Derail` finds anything at all.
    ///
    /// **Measured, at M8.** It was 0.4, derived as the share of a *single*
    /// item's duty cycle that a 1,000 ms window covers on a board cadenced
    /// around 2,500 ms. That is the right arithmetic for the wrong question: a
    /// creature at the bands the yard stands on wears fourteen to twenty-six
    /// items, and the chance that *one of them* is within a second of firing
    /// is nearly one.
    ///
    /// `prices::report_what_the_two_conditionals_actually_manage` fights a
    /// Derail on a 2,500 ms glove against the four creatures at bands 27 to
    /// 30: 59% against the thinnest board and 100% against the two densest,
    /// **0.79 overall**.
    ///
    /// It is still a discount rather than 1.0, and the discount is the thin
    /// boards - which is the honest shape, because a creature with three items
    /// is exactly the one a denial is worth least against.
    pub const DERAIL_WINDOW: f32 = 0.79;

    /// How much of an action a reaction to the *other* side is worth.
    ///
    /// The board-side reactions - adjacent, aligned, diagonal - are priced at
    /// zero here and handled by the caller, which knows how many neighbours an
    /// item has. There is no equivalent for the opposition: how often they act
    /// is a fact about their board and not about this one.
    ///
    /// `trigger_points` is *per activation of its own item*, so this is the
    /// ratio of how often the other side acts to how often this item comes
    /// round. A greaves item fires about three times in a fight; a board acts
    /// far more than that, and the first measurement of this said so loudly -
    /// Cog Priest wears a Worldstrider Sole and the preset board went from
    /// finishing rung 25 with 28 health to finishing it with -6.
    ///
    /// **Two**, and it is a compromise the scale cannot avoid making. What
    /// this is worth depends on the *other* board, which `piece_rating` never
    /// sees: worn by a player it answers a creature's four or five items, and
    /// worn by a creature it answers a packed board's nineteen. Priced between
    /// the two rather than at either, and deliberately not at the higher one -
    /// the shop sells to the player.
    pub const REACTION: f32 = 2.0;
    /// A stack of empowerment or shield per second. Both scale off held mana,
    /// so their real worth depends on a build the rating cannot see; this is
    /// the value of a stack in a build that is actually banking mana.
    pub const STACK_PS: f32 = 11.0;
    /// A stack of Spellblade or Deflection per second - the physical lane's
    /// twins of the pair above.
    ///
    /// Neither scales off a pool, so neither has the mana pair's ceiling and
    /// neither has its condition: a stack is worth the same to a board that
    /// banks nothing as to one that banks forty. Spellblade sits level with a
    /// mana stack because those two things cancel. Deflection sits under it
    /// because it answers one lane where the shield answers one lane and asks
    /// for mana to do it - the discount is for the half of the fight it is not
    /// in, not for the mana it does not want.
    pub const SPELLBLADE_PS: f32 = 11.0;
    pub const DEFLECTION_PS: f32 = 9.0;
    /// A stack of spell forking: every cast lands once more.
    ///
    /// Worth more than a shield stack because it multiplies a payload rather
    /// than shaving a hit, and less than it looks because only a caster can
    /// use one at all - a blade swings once however many stacks are up.
    pub const FORK_PS: f32 = 26.0;

    /// Speed is a percentage on the whole item, so it is scored against
    /// whatever the item is already worth rather than on its own.
    pub const SPEED_PCT: f32 = 0.006;

    /// What a spell's printed payload is actually worth, once the two
    /// intensities are taken into account.
    ///
    /// A cast lands at `EMPOWERED_CAST_PCT` when it is paid for and
    /// `WEAK_CAST_PCT` when it is not, and the same two-thirds assumption the
    /// spending triggers use applies here: mana income is finite. The scale
    /// used the printed number, which is neither. Spells were the one kind of
    /// gear whose rating did not describe what it does.
    pub const CAST_INTENSITY: f32 = 0.66 * (crate::combat::EMPOWERED_CAST_PCT as f32 / 100.0)
        + 0.34 * (crate::combat::WEAK_CAST_PCT as f32 / 100.0);
}

/// How many seconds of growth a growing piece is rated for.
///
/// Not one fight's worth. What a growing piece banks it keeps for the whole
/// run, so the health it wins in this fight is health you start the next one
/// with - it compounds for as long as the run does. Three fights' worth is a
/// deliberate understatement of that: enough to price these pieces as the
/// strongest things a player can buy, without a single item running away with
/// a scale that has to hold the rest of the catalogue too.
const TYPICAL_FIGHT_S: f32 = 60.0;

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
                        .filter(|d| d.fits(s) && d.kind == kind && !crate::piece::is_off_the_scale(d.name))
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
        + s.reflect as f32 * weight::REFLECT
}

/// Stats granted once per activation, scored at `rate` activations a second.
fn activated_points(s: &Stats, rate: f32) -> f32 {
    ((s.physical_damage + s.magic_damage) as f32 * weight::DAMAGE_PS
        + s.armor as f32 * weight::ARMOR_PS
        + s.mana as f32 * weight::MANA_PS
        + (s.rage + s.faith + s.nature) as f32 * weight::RESOURCE_PS
        + s.mind as f32 * weight::MIND_PS)
        * rate
}

/// What the pools a piece banks are worth for the time you hold them.
///
/// Distinct from `activated_points`, which prices the same points for being
/// spent. A pool is both: you bank it, it works for you while it sits there,
/// and then a trigger spends it. Scoring only the spending is what made every
/// faith piece in the catalogue rate near zero.
fn held_points(s: &Stats, rate: f32) -> f32 {
    let banked_per_fight = (s.rage + s.faith + s.nature) as f32 * rate * TYPICAL_FIGHT_S;
    banked_per_fight * weight::HELD_SHARE * weight::HELD_PER_POINT
}

/// What one curse is worth, by what it actually does.
///
/// Searing keeps `CURSE_PS` so nothing that was already balanced against it
/// moves. The other three are priced on how much of the other side's output
/// they take away, in seconds, from the durations in `curse.rs`.
fn curse_points(kind: crate::curse::CurseKind) -> f32 {
    use crate::curse::{CurseKind, FROST_MS, FROST_SLOW_PCT, MISFIRE_EVERY, MISFIRE_MS, STUN_MS};
    let secs = |ms: u32| ms as f32 / 1000.0;
    match kind {
        // Damage over time rather than denial, and it does not stack.
        CurseKind::Searing => weight::CURSE_PS,
        // Frost slows *everything* the other side owns, and this model counts
        // denial in seconds of one item's output - so a curse that takes a
        // fifth off eight items was being priced as though it took a fifth off
        // one. That was noted here for a long time and left alone because
        // repricing it moves a lot of gear; it is greaves' curse now, and the
        // slot it belongs to was being paid a fifth of what its signature
        // mechanic is worth.
        CurseKind::Frost => {
            secs(FROST_MS) * FROST_SLOW_PCT as f32 / 100.0 * weight::DENIAL_S * SLOWED_ITEMS
        }
        // A stun stops one item for its whole length. This figure did not
        // change when stun stopped being side-wide, and did not need to: the
        // model has always counted denial in seconds of *one* item's output,
        // so 1.2 was already what a per-item stun is worth. What it was
        // underpricing was the old side-wide version. Frost is still measured
        // that way and still slows everything, so it remains cheap for what it
        // does - deliberately noted rather than quietly corrected, because
        // repricing frost moves the cost of a lot of existing gear.
        CurseKind::Stun => secs(STUN_MS) * weight::DENIAL_S,
        CurseKind::Misfire => secs(MISFIRE_MS) / MISFIRE_EVERY as f32 * weight::DENIAL_S,
    }
}

/// How many items a side-wide slow is assumed to be slowing.
///
/// A built board runs eight to nineteen items; the preset runs eight and the
/// two finished human boards run seventeen and nineteen. **Two**, which is far
/// below any of them, and deliberately: the design says stun and misfire are
/// the premium curses and frost is the cheap one, and that ordering is a
/// decision rather than an accident of arithmetic. Two doubles frost - which is
/// the error this corrects - and leaves it the cheapest of the three, which is
/// what it is meant to be. Frost is worth more than this against a real board.
const SLOWED_ITEMS: f32 = 2.0;

/// What a drain of "everything they have" is priced as holding.
///
/// A pool nobody is banking cannot be drained and a pool somebody has built
/// their whole run around can be enormous; neither is what this gear is for.
/// Eight is a build that banks deliberately without being about it.
const DRAINED_ASSUMED: i32 = 8;

/// The pool an `Accrue` is assumed to be reading.
///
/// The same shape `DRAINED_ASSUMED` has and deliberately not the same number.
/// That one is "a deep-but-not-absurd pool as a *victim* holds one"; this is
/// what a build that wanted proportional income holds itself, which
/// `design/towns.md`'s table puts at 46 on the winning board and 6 on the
/// auto-built one. Thirty is between them, leaning towards the build that
/// went looking for it.
const ACCRUED_ASSUMED: i32 = 30;

/// What one action is worth each time it happens.
fn action_points(a: &Action) -> f32 {
    match a {
        // **Zero, and on purpose.** The wrong sense is a trade, and what it is
        // worth depends entirely on the board around it - a board with no mind
        // damage that wears this crest deals nothing at all. Pricing it as a
        // benefit would put it at the top of its slot for every board, which
        // is the one thing `boss_gear_does_not_move_the_scale` exists to stop.
        //
        // It is priced by hand instead, in the piece's own `price`, which is
        // what `is_off_the_scale` is for elsewhere.
        Action::SeeWithTheWrongSense => 0.0,
        Action::Curse { kind, target } => {
            let v = curse_points(*kind);
            // A curse on yourself is a cost, not a benefit.
            if matches!(target, crate::piece::Target::Yourself) {
                -v
            } else {
                v
            }
        }
        // An unaimed stun takes whatever item it catches, so against a full
        // board it finds the one that mattered about one time in five. Aiming
        // it is worth more than the stun: what it denies is not 1.2 seconds of
        // output but 1.2 seconds of *their best* output, every single time.
        Action::StunStrongest { target } => {
            let v = curse_points(crate::curse::CurseKind::Stun) * weight::AIMED;
            if matches!(target, crate::piece::Target::Yourself) {
                -v
            } else {
                v
            }
        }
        // Growth is worth what it will have granted, and it is never given
        // back. The caller turns this into a per-second figure, so multiplying
        // by a span in seconds converts "health per activation" into "the flat
        // health this will be worth" - which is the thing to compare it
        // against.
        // Growth arrives over the fight rather than at the bell, so the health
        // it is actually worth is the average it stood at, not the total it
        // reached: a board that ends a fight three hundred health taller spent
        // most of that fight less than three hundred taller. Halved for that.
        // Left alone, and the attempt is worth recording. Halving this - on the
        // argument that growth arrives over a fight, so the health it is worth
        // is the average it stood at rather than the total it reached - is a
        // reasonable model and had no fault behind it. What it did have was a
        // consequence: it moved Grow-carrying pieces down their footprint
        // families, `stepped_component` sorts those families by rating to
        // choose a creature's gear above Medium, and Francis's Insane step
        // stopped picking Berserker's Crest and started picking Tithe
        // Collector - a drain, against a board that banks nothing. The best
        // board in the project then lost to him on Hard and beat him on
        // Insane. A final boss who gets easier as the setting rises is worse
        // than a mechanic priced by the wrong model, so the model stands until
        // there is a fault to fix rather than a preference to express.
        Action::Grow(n) => *n as f32 * weight::HEALTH * TYPICAL_FIGHT_S,
        Action::Damage { amount, target, .. } => {
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
        // Denying a pool is worth about what banking it is worth, plus
        // whatever the loss is made to hurt for. `amount: 0` takes the lot,
        // which cannot be priced against a build this function cannot see, so
        // it is priced at a deep-but-not-absurd pool - the thing it is for.
        Action::Drain { what, amount, hurt, target } => {
            let taken = if *amount == 0 { DRAINED_ASSUMED } else { *amount };
            let v = taken as f32 * pool_weight(*what) + (taken * hurt) as f32 * weight::DAMAGE_PS;
            if matches!(target, crate::piece::Target::Yourself) {
                -v
            } else {
                v
            }
        }
        // Two ordinary points become one worth four of them, so the trade is
        // worth the two points of standing value it adds. Discounted because
        // it does nothing at all unless both parents have something in them,
        // which early in a fight they do not.
        Action::Fuse { .. } => 0.66 * 2.0 * weight::HELD_PER_POINT,
        Action::GainArmor(n) => *n as f32 * weight::ARMOR_PS,
        Action::ReduceCooldown(ms) => *ms as f32 / 1000.0 * weight::HASTE_PS,
        Action::GainEmpowerment(n) => *n as f32 * weight::STACK_PS,
        Action::GainShield(n) => *n as f32 * weight::STACK_PS,
        Action::GainSpellblade(n) => *n as f32 * weight::SPELLBLADE_PS,
        // A stack of Dread is worth what a stack of empowerment is worth: both
        // multiply a lane by a pool, and neither is worth anything without the
        // pool. Priced beside it and re-visited when the Insight family lands.
        Action::GainDread(n) => *n as f32 * weight::STACK_PS,
        Action::GainDeflection(n) => *n as f32 * weight::DEFLECTION_PS,
        // A fork copies a cast, so a stack is worth roughly what the cast was
        // - which is more than a shield stack, and only to a build that has
        // something worth copying.
        Action::GainForking(n) => *n as f32 * weight::FORK_PS,
        // Haste at a third of the price, because the time is not created -
        // only moved somewhere it is worth more.
        Action::Shunt { ms } => *ms as f32 / 1000.0 * weight::SHUNT_PS,
        // Growth, funded. Priced as `Grow` and then discounted for the
        // condition, which is that there was armour there to spend.
        Action::Ballast(n) => {
            *n as f32 * weight::HEALTH * TYPICAL_FIGHT_S * weight::BALLAST_FUNDED
        }
        // Denial, aimed, and found only some of the time. Aimed because it
        // picks the best item there is; discounted because most activations
        // find nothing in the window at all.
        Action::Derail { back_ms, .. } => {
            *back_ms as f32 / 1000.0 * weight::DENIAL_S * weight::AIMED * weight::DERAIL_WINDOW
        }
        // Income the rating cannot see the size of, so it is priced against an
        // assumed balance the same way a drain is.
        Action::Accrue { what, pct } => {
            ACCRUED_ASSUMED as f32 * *pct as f32 / 100.0 * pool_weight(*what)
        }
        // ---- the cadence three ----
        //
        // None of these is a stat, so nothing above could see them and the
        // shop would have priced every one at the floor. That is the failure
        // this file was written about: an ink was priced as a blank page for a
        // long time because the scale could not see a multiplier.
        //
        // A head start is haste that happens once. `Prime` is a percentage of
        // this item's own cooldown, and `piece_points` divides a battle-start
        // trigger by the fight length rather than multiplying it by the
        // activation rate - so the seconds saved are priced at `HASTE_PS` and
        // the once-a-fight discount is applied by the caller, exactly as it is
        // for every other `OnBattleStart`.
        Action::Prime { pct } => {
            *pct as f32 / 100.0 * TYPICAL_COOLDOWN_S * weight::HASTE_PS
        }
        // The same head start, for everything on the board. Worth more than
        // one item's, and not five times more: the fifth item primed is the
        // fifth-best one, and a board that is not full gets less. `BOARD_ITEMS`
        // is what a good board actually finishes.
        Action::PrimeBoard { pct } => {
            *pct as f32 / 100.0 * TYPICAL_COOLDOWN_S * weight::HASTE_PS * BOARD_ITEMS
        }
        // Haste's opposite, and it costs the wearer rather than paying them -
        // so it is negative, and it compounds: an item that fires ten times in
        // a fight has added `ms` ten times over. Priced at half of `HASTE_PS`,
        // because the last additions land when the fight is nearly over and
        // buy nothing.
        Action::Drift { ms } => {
            -(*ms as f32 / 1000.0) * weight::HASTE_PS * DRIFT_FIGHT_FIRINGS * 0.5
        }
        // What it saves is the activations a stun would have eaten. `STUN_CAP_MS`
        // is what one item can be stopped for, and `StunStrongest` aims at the
        // best item a fighter owns - which is what this protects, so it is
        // priced at the aimed rate rather than the ordinary one.
        Action::Unshakable => {
            crate::curse::STUN_CAP_MS as f32 / 1000.0 * weight::DENIAL_S * weight::AIMED
        }
    }
}

/// A middling cooldown, for pricing a head start as the seconds it saves.
///
/// The five slots run 1,500 to 5,000 ms and the mean of the defaults is a
/// little over three seconds.
const TYPICAL_COOLDOWN_S: f32 = 3.4;

/// What a finished board actually carries, for a board-wide effect.
///
/// Not five: `report_damage_share_and_ttk` puts the owner's board at nineteen
/// items across the five grids, and the marginal one primed is the worst one.
/// Three is the number of items a head start is really worth paying for.
const BOARD_ITEMS: f32 = 3.0;

/// How many times a drifting item fires before the fight ends.
///
/// `TYPICAL_FIGHT_S` over `TYPICAL_COOLDOWN_S`, rounded down, which is what
/// decides how much cadence it has given away by the end.
const DRIFT_FIGHT_FIRINGS: f32 = 3.0;

/// What a point of `what` costs to spend.
///
/// Not the same number for all four, and it took a survey of the catalogue to
/// see why. Mana does nothing while you merely hold it - its standing value is
/// conditional on owning empowerment or shield stacks - so spending it costs
/// you the point and nothing else. Rage, faith and nature all pay out for
/// every second they sit in the reserve: a point of rage is a point on every
/// swing, a point of faith is two points of both resistances, a point of
/// nature is a point of regeneration. Spending one of those destroys a bonus
/// you were already collecting, and the scale used to charge nothing for it -
/// so a trigger that burned faith was priced exactly like one that burned
/// mana, which made every hold-pool sink in the game look cheaper than it is.
///
/// Half of `HELD_PER_POINT`, because a point spent is a point that was going
/// to keep paying for the rest of the fight and the average spend lands
/// somewhere in the middle of one.
/// Friendly activations a second on a board worth having.
///
/// Measured rather than assumed, and **re-measured in M16 against boards that
/// had grown since**. The figures this was set by - 1.8 for the preset, 2.3
/// and 4.8 for the two finished runs - are now 2.06, 3.43 and 6.60, because
/// the gear-slot rewrite gave every slot something to do on a cooldown and
/// finished boards got busier.
///
/// **Five**, the mean of the two finished human boards. Two was a third of
/// what a real board does, and everything in this file that watches - which
/// is the whole of the gloves' axis, forty-seven reaction triggers - was
/// priced at a third of what it sees. The preset sits below this figure
/// because it is a reference build rather than a finished one; the starter,
/// at 0.5, is one item and is not evidence of anything.
const ACTIVATIONS_PER_S: f32 = 5.0;

/// How often a watcher of each kind sees the thing it is counting.
///
/// Shares of `ACTIVATIONS_PER_S`, discounted by how much of the board a
/// watcher of that kind can see. Anything counts everything; a neighbour
/// watcher only sees the items it touches, and a diagonal one fewer still,
/// because a packed board makes edges by accident and corners on purpose.
/// Curses are not activations at all and are much rarer than them.
fn watched_per_s(what: crate::piece::Watched) -> f32 {
    use crate::piece::Watched;
    ACTIVATIONS_PER_S
        * match what {
            Watched::AnyActivation => 1.0,
            Watched::AlignedActivation => 0.3,
            Watched::AdjacentActivation => 0.2,
            Watched::DiagonalActivation => 0.15,
            Watched::CurseApplied => 0.1,
        }
}

fn pool_weight(what: crate::piece::Resource) -> f32 {
    use crate::piece::Resource;
    match what {
        // Mana is fuel: it pays nothing while merely held and is worth exactly
        // what the stacks it feeds are worth.
        Resource::Mana => weight::MANA_PS,
        // Insight is not fuel, and pricing it as fuel was M2 filing it beside
        // the pool it was modelled on rather than measuring what it does.
        // Nothing spends it. What it does is multiply every point of Dread for
        // the rest of the fight, which is the shape of a *held* pool - so it
        // is priced as one, with the same conditionality mana's stacks carry:
        // worth this to a board that also carries Dread, and worth nothing to
        // a board that does not.
        Resource::Insight => weight::RESOURCE_PS + weight::HELD_PER_POINT / 2.0,
        Resource::Rage | Resource::Faith | Resource::Nature => {
            weight::RESOURCE_PS + weight::HELD_PER_POINT / 2.0
        }
        // Nothing spends a fusion, so the only figure it needs is what it is
        // worth to hold - and what it is worth to lose, which is the same
        // number seen from a `Drain`. Both parents at double is four times an
        // ordinary pool point, and none of the spend half applies.
        Resource::DruidicMight | Resource::Communion | Resource::Zealotry => {
            4.0 * weight::HELD_PER_POINT
        }
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
        // Handled by the caller: it happens once a fight, not once an
        // activation, so multiplying it by the activation rate is exactly
        // backwards. See `piece_points`.
        Trigger::OnBattleStart(_) => 0.0,
        // Answering the other side. Discounted like the other reactions: how
        // often they act is a fact about their board, not this one, and a
        // creature that fires twice a fight pays this nothing.
        Trigger::OnEnemyActivate(a) => action_points(a) * weight::REACTION,
        // Mana income is finite, so assume it pays about two thirds of the
        // time and eats the failure branch the rest.
        Trigger::Spend { what, cost, on_success, on_failure } => {
            0.66 * action_points(on_success) + 0.34 * action_points(on_failure)
                - *cost as f32 * pool_weight(*what) * 0.66
        }
        // Money is not a combat resource, so spending it is not priced like
        // one: what a fight costs you is a shop decision you make later, and
        // the rating cannot see the shop. Priced on the payout alone, at the
        // average escalation the budget allows - which is what the piece is
        // worth to a player who can afford it, and it is only ever bought by
        // players who can afford it.
        Trigger::SpendGold { cost, budget, on_success } => {
            let times = (budget / (*cost).max(1)).max(1);
            let average = (times + 1) as f32 / 2.0;
            action_points(on_success) * average
        }
        Trigger::SpendMana { cost, on_success, on_failure } => {
            0.66 * action_points(on_success) + 0.34 * action_points(on_failure)
                - *cost as f32 * weight::MANA_PS * 0.66
        }
        // Emptying a reserve pays by the handful, so what it is worth depends
        // on how full the reserve was - which this function cannot see. Three
        // handfuls is what a build that actually feeds the thing gets to; a
        // build that does not gets nothing at all, and the discount for that
        // is already in the fact that three is a modest guess.
        Trigger::Consume { what, each, per } => {
            let times = 3.0;
            times * action_points(per) - times * *each as f32 * pool_weight(*what)
        }
        // A piece with room around it touches one or two finished items.
        Trigger::PerAdjacentItem { action, .. } => 1.3 * action_points(action),
        // Room around an item is bought with the gear you did not pack there.
        // Four open cells is what a build that is trying gets; more is
        // possible and costs more than it is worth.
        Trigger::PerAdjacentEmpty(inner) => 4.0 * trigger_points(inner),
        // Reactions fire off somebody else's cooldown. Handled by the caller,
        // for exactly the reason `OnBattleStart` and `Watch` are: multiplying
        // them by *this* item's cadence prices a reaction by the wrong clock,
        // so a fast item and a slow one carrying the same reaction came out
        // worth different amounts when the thing they answer is the neighbour.
        // The comment two lines above used to say "fires off someone else's
        // cooldown" and then multiply by this one's.
        Trigger::OnAdjacentActivate(_)
        | Trigger::OnAlignedActivate(_)
        | Trigger::OnDiagonalActivate(_) => 0.0,
        // Handled by the caller for the same reason `OnBattleStart` is: it
        // fires on a count of events, not on this item coming round, so
        // multiplying it by this item's cadence is backwards. See
        // `piece_points`.
        Trigger::Watch { .. } => 0.0,
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
        // **Zero, and on purpose.** The wrong sense is a trade, and what it is
        // worth depends entirely on the board around it - a board with no mind
        // damage that wears this crest deals nothing at all. Pricing it as a
        // benefit would put it at the top of its slot for every board, which
        // is what `boss_gear_does_not_move_the_scale` exists to stop. It is
        // priced by hand, in the piece's own `price`.
        EffectKind::WrongSense => 0.0,
        // A multiplier on everything, if you can keep the board clear enough
        // to earn it. Rated as though it pays about a third of the time: it is
        // enormous when it lands and very easy to break by accident, and the
        // rating cannot see the board it will end up on.
        EffectKind::SoleIf { times, .. } => (times - 1) as f32 * 22.0,
        // Terrain is worth what ends up standing on it, and the rating cannot
        // see the board. Two covering pieces is what a build that bothered to
        // lay an underlay will manage - it is the same "what a reasonable build
        // gets" the conditional triggers are discounted by.
        EffectKind::PerOverlappingItem { amount, stat } => {
            EXPECTED_COVERAGE * amount as f32 * stat_weight(stat)
        }
        // A core is one piece an item, so covering an underlay with two of
        // them means two whole items standing on it. Rarer, and worth more per
        // point because of it.
        EffectKind::PerOverlappingCore { amount, stat } => {
            EXPECTED_COVERAGE * 0.6 * amount as f32 * stat_weight(stat)
        }
        // Worth roughly two neighbours of the right sort, which is what a
        // build that wants this effect will actually manage.
        EffectKind::SelfPerNeighborKind { per, stat, .. } => {
            2.0 * per as f32 * stat_weight(stat)
        }
        // Doubling a neighbour is worth about what a good neighbour carries.
        EffectKind::DoubleNeighbor { .. } => 16.0,
        EffectKind::DoubleAdjacentItemStat { .. } => 20.0,
        // A piece out in the open touches four or five empty cells.
        EffectKind::SelfPerEmptyCell { per, .. } => 4.5 * per as f32 * weight::STRENGTH,
        // Bearing doubles everything this item is - but only while the slot
        // holds one item, which is a whole grid spent on one thing. Rated
        // against `SoleIf`'s 22 a multiple, discounted because the condition
        // is easier to hold than a solitude (you have to *not build* rather
        // than build carefully) and paid for in the grid it costs.
        EffectKind::Bearing => BEARING,
        // One extra activation in a fight. A fight is thirty to forty seconds
        // and a glove is a three-second item, so this is roughly a twelfth of
        // what the item does all fight - and it lands at the start, which is
        // worth more than a twelfth because a fight decided early was decided
        // by the first ten seconds.
        EffectKind::Overtake => OVERTAKE,
        // The adjacency it claims, priced as the adjacency it claims:
        // `DoubleAdjacentItemStat` is 20 for the neighbours an item actually
        // has, and this one has all of them. Not 20 times anything - a board
        // has five or six assembled items and the effects that read adjacency
        // are per-neighbour, so what Commons buys is the difference between
        // one or two neighbours and all of them.
        EffectKind::Commons => COMMONS,
        EffectKind::Flat { stats } => standing_points(&stats) + activated_points(&stats, rate),
    };
    raw * scale
}

/// THE HUNDRED's three effects, priced.
///
/// **Measured at F13.** Two of the three moved and the third did not, and each
/// is a figure off a measurement rather than an argument.
///
/// **`BEARING` 26.0, unmoved.** It contributes +22 to Trig Pillar's rating,
/// which puts it at 64 - level with Ridge Runner and under Worldweave
/// Material at 68, which is the family a greaves enchantment belongs in. A
/// doubling conditional on a whole grid is the biggest of the three and the
/// dearest to earn, and the rating says so.
///
/// **`OVERTAKE` 14.0 to 10.5.** One extra activation of the item, at the bell.
/// Measured over a whole fight on a gloves-default 3,000 ms item it is
/// **+7.1%**; at the four-board table's median time-to-kill of nine seconds it
/// is **33%**, because a nine-second fight is three activations and this is a
/// fourth. Weighted toward where fights are decided rather than where they end
/// up, call it a fifth of what Bearing's unconditional doubling would be worth
/// - and 10.5 is that fifth. It went **down**, which is the right direction
/// for a weight that was a guess.
///
/// **`COMMONS` 24.0 to 30.0.** Measured structurally: on the two finished
/// reference boards an item has **2.2 neighbours** on average, and a commons
/// item would have **eighteen** - eight times the reach.
/// `DoubleAdjacentItemStat` is 20 for the neighbours an item actually has. The
/// discount is heavy and deliberate, because Commons pays nothing on its own:
/// it makes a relation exist and only what reads adjacency collects on it.
pub const BEARING: f32 = 26.0;
pub const OVERTAKE: f32 = 10.5;
pub const COMMONS: f32 = 30.0;

/// How many covering pieces an underlay can expect to end up under.
///
/// The rating cannot see the board, so this is the same standard every
/// conditional trigger in this file is discounted by: what a build that
/// actually wanted the effect will manage. Somebody who lays terrain lays gear
/// on it.
const EXPECTED_COVERAGE: f32 = 2.0;

/// What bonding is worth, before anything the enchantment itself carries.
///
/// Doubling one item is worth roughly what a strong piece is, and the two
/// conditions it asks for pull against each other on purpose - enchantments
/// have to be spread out and gear has to be packed tight on the one of them
/// you mean to bond. A board that does both has given up cells to do it.
const BOND_POINTS: f32 = 45.0;

/// What one point of a stat is worth to an effect that grants it directly.
///
/// Was written inline inside `SelfPerNeighborKind` and is now wanted by the two
/// overlap effects as well; three copies of a four-arm match is how they drift
/// apart.
fn stat_weight(stat: crate::stats::StatKind) -> f32 {
    match stat {
        crate::stats::StatKind::Strength => weight::STRENGTH,
        crate::stats::StatKind::Health => weight::HEALTH,
        crate::stats::StatKind::Power => weight::POWER,
        _ => 2.0,
    }
}

fn adjacency_points(a: &AssemblyBonus, rate: f32) -> f32 {
    standing_points(&a.stats) + activated_points(&a.stats, rate)
}

/// What one component is worth, assuming its item fires every `cooldown_ms`.
///
/// `cooldown_ms` of 0 means "use the slot's default", which is what a piece
/// gets rated at on a shop shelf, before you know what it will be built into.
fn piece_points(def: &PieceDef, cooldown_ms: u32) -> f32 {
    let cd = if cooldown_ms == 0 { default_cooldown_ms(def.slot) } else { cooldown_ms };
    let rate = 1000.0 / cd.max(1) as f32;

    // A spell's payload is the thing that gets cast, so it is the thing the
    // two intensities scale. Everything else on the piece is not.
    let intensity = if def.kind == crate::piece::PieceKind::Spell {
        weight::CAST_INTENSITY
    } else {
        1.0
    };
    // The bond, which is most of what an enchantment is worth and none of what
    // its stat line says.
    //
    // A bonded item is doubled - `+1.00x power`, which multiplies its stats and
    // what its triggers pay out - and handed this piece's triggers on top. The
    // rating cannot see the board, and this needs two things of one at once:
    // the enchantment layer clear all round it, and one item shaped to cover
    // every cell. So it is discounted hard from "worth a whole item", the same
    // way every conditional in this file is: what a build that actually wanted
    // it will manage.
    //
    // Flat rather than a share of the slot ceiling, because the ceiling is a
    // maximum over pieces and a piece whose worth is a fraction of the ceiling
    // would be defining the thing it is measured against.
    let bond = if def.kind.is_enchantment() { BOND_POINTS } else { 0.0 };
    let mut points = bond
        + standing_points(&def.base)
        + activated_points(&def.base, rate) * intensity
        + held_points(&def.base, rate)
        + def.power_bonus as f32 * weight::POWER_BONUS;
    if let Some(adj) = def.assembly_bonus {
        points += adjacency_points(&adj, rate);
    }
    if let Some(eff) = def.effect {
        points += effect_points(&eff, rate);
    }
    for t in def.triggers {
        // Everything else is worth its value once per activation, so it scales
        // with how often the item comes round. An opening happens once a
        // fight however fast the item is - which makes it worth its value
        // spread across the fight, not multiplied by the cadence. Getting that
        // backwards priced a 90-armour opening as though it arrived every two
        // seconds, and the weapon slot's ceiling went up enough to deflate
        // every ink in the game to nothing.
        if let Trigger::OnBattleStart(a) = t {
            points += action_points(a) / TYPICAL_FIGHT_S;
        } else if let Trigger::Watch { what, count, then, repeats } = t {
            // A watcher runs on the board's clock, not the item's. Its rate is
            // how often the thing it counts happens, divided by how many it
            // waits for - so a fast item and a slow one carrying the same
            // watcher are worth the same, which is the point of the trigger.
            let seen = watched_per_s(*what);
            let per = seen / (*count).max(1) as f32;
            points += action_points(then)
                * if *repeats {
                    per
                } else {
                    // One payout at most, and none at all if the fight ends
                    // before the count is reached.
                    (per * TYPICAL_FIGHT_S).min(1.0) / TYPICAL_FIGHT_S
                };
        } else if let Some((a, seen)) = match t {
            // A reaction answers a neighbour, so it runs at the rate that
            // neighbour comes round - the same board clock a watcher counts
            // on, which `watched_per_s` already models.
            Trigger::OnAdjacentActivate(a) => {
                Some((a, crate::piece::Watched::AdjacentActivation))
            }
            Trigger::OnAlignedActivate(a) => Some((a, crate::piece::Watched::AlignedActivation)),
            Trigger::OnDiagonalActivate(a) => {
                Some((a, crate::piece::Watched::DiagonalActivation))
            }
            _ => None,
        } {
            points += action_points(a) * watched_per_s(seen);
        } else {
            points += trigger_points(t) * rate;
        }
    }
    // Speed lifts everything the item does - but only score it here when the
    // caller could not. Given a real cooldown, the item's speed is already in
    // `rate` and every per-second figure above; adding a percentage on top
    // would be counting it twice. Zero means "not placed yet", which is the
    // shop card, and there the bonus is all the information there is.
    if cooldown_ms == 0 {
        points += points.abs() * def.speed_bonus as f32 * weight::SPEED_PCT;
    }
    points
}

/// What a piece is worth **to a creature**, which is not what it is worth in a
/// shop.
///
/// `piece_rating` prices an item for a player who can build a run around it.
/// `stepped_component` uses that ordering to choose a monster's gear above
/// Medium, and the two questions are different enough to invert the difficulty
/// ladder: a drain rates well because a build that banks pools will feel it,
/// and against a board that banks nothing it does exactly nothing. Francis's
/// Insane step picked up Tithe Collector over a damage crest on that reasoning
/// and got *easier* than his Hard step.
///
/// So the mechanics whose worth depends on what the other side happens to be
/// carrying are discounted here, and only here:
///
/// - **Drains** need the target to have banked the pool.
/// - **Pool spending** - `Consume`, `Spend`, `SpendMana` - needs the creature
///   to have banked it first, and a creature's gear is fixed, so it usually has
///   not.
/// - **Mind damage** is answered by `mind_resist`, which finished boards carry
///   and the shop model does not know about.
///
/// Everything that lands regardless - damage, curses, health, armour,
/// resistance, regeneration - counts in full. This is deliberately a *coarse*
/// correction: the point is that the ordering a monster is dressed from should
/// track what wins fights, not that this function is the last word on it.
pub fn monster_value(def: &PieceDef) -> f32 {
    let mut v = piece_points(def, def.cooldown_ms);
    // A holding pool is priced as a pool for a player and as what it converts
    // to for a creature.
    //
    // `RESOURCE_PS` prices a point of rage, faith or nature at what it is
    // worth to somebody who will decide what to spend it on. A creature never
    // decides anything: its gear is fixed, it rarely carries a sink, and every
    // point it starts with sits there paying `held_bonus` for the whole fight
    // - a point of nature is a point of regeneration, a point of faith is two
    // of each resistance, a point of rage is a point of physical damage. So
    // the pool stats are re-priced here at their conversion.
    //
    // This is not a rounding difference. Stepping *down* walked Francis into
    // three crowns carrying nature between them; his regeneration on Easy came
    // out four times what it was on Medium, and the best board in the project
    // lost to him on the easiest setting and beat him on the next two.
    let held = &def.base;
    v -= (held.rage + held.faith + held.nature) as f32 * weight::RESOURCE_PS;
    v += held.nature as f32 * weight::REGEN;
    v += held.rage as f32 * weight::DAMAGE_PS;
    v += held.faith as f32 * 2.0 * weight::RESIST * 2.0;
    for t in def.triggers {
        let mut discount = 0.0f32;
        let mut premium = 0.0f32;
        crate::piece::walk_actions(t, &mut |a| {
            discount += match a {
                Action::Drain { .. } | Action::MindDamage { .. } => action_points(a),
                _ => 0.0,
            };
            // Banking a holding pool compounds, and only for a creature.
            //
            // A point of rage, faith or nature pays its `held_bonus` for the
            // rest of the fight, and a creature never spends any of it - its
            // gear is fixed and rarely carries a sink. So a piece that banks
            // every time it comes round is not worth one payout, it is worth
            // one payout still running when the next arrives. Priced flat, it
            // sorts below the piece it should sort above, and stepping *down*
            // walked Francis into three crowns that each banked nature: his
            // regeneration on Easy came out four times what it was on Medium,
            // and the best board in the project lost to him on the easiest
            // setting and beat him on the next two.
            //
            // Mana is left out on purpose. It is fuel rather than a holding,
            // it pays nothing passively, and a creature with no sink for it
            // has banked nothing at all.
            premium += match a {
                Action::Gain { what, .. } if *what != crate::piece::Resource::Mana => {
                    action_points(a)
                }
                _ => 0.0,
            };
        });
        if matches!(t, Trigger::Consume { .. } | Trigger::Spend { .. } | Trigger::SpendMana { .. }) {
            discount += trigger_points(t).max(0.0);
        }
        v -= discount;
        v += premium;
    }
    v
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
    // Priced against what a run actually earns. A rung pays between 6 and 500
    // gold, and a bounty should buy roughly one piece worth having at that
    // stage - so a middling piece is a few fights' income and a slot-carrying
    // one is most of a late fight's.
    //
    //   rating  10 ->    6g      rating  80 ->  207g
    //   rating  40 ->   59g      rating 140 ->  581g
    //
    // The old curve topped out around 120g for the best piece in the game,
    // against 688 gold banked by rung seventeen, so everything was free from
    // the early game onward and the shop stopped being a decision.
    (2.0 + (r / 4.5).powf(1.85)).round() as i32
}

/// Half of what it cost, rounded down - what selling one back pays.
/// What the counter pays for a component.
///
/// Boss gear pays nothing, on purpose. It is priced off a rating that is
/// deliberately outside the scale - the Money Jacket came to 1685 against 131
/// for the best thing anybody can buy - so one trophy paid for the rest of the
/// run and the shop stopped being a decision. What it is worth instead is a
/// trade: the pub takes one for a stack of Recycler, and that is the only
/// thing in the game that will take one.
pub fn resale_price(def: &PieceDef) -> i32 {
    if crate::piece::is_boss_only(def.name) {
        return 0;
    }
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
                    .filter(|d| {
                        d.fits(slot)
                            && d.kind == kind
                            && !crate::piece::is_off_the_scale(d.name)
                    })
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

    /// An ink is the largest damage multiplier in the game - 90 to 240
    /// hundredths of weapon power on the item it is bound into - and for a
    /// long time the scale could not see it at all, so every ink was priced
    /// as though it were a blank page.
    #[test]
    fn a_piece_that_multiplies_an_item_is_not_priced_as_a_blank_one() {
        use crate::piece::PieceKind;
        for kind in [PieceKind::Ink, PieceKind::Orb, PieceKind::Alignment] {
            let cheapest = CATALOG
                .iter()
                .filter(|d| d.kind == kind && !crate::piece::is_boss_only(d.name))
                .map(shop_price)
                .min()
                .expect("the catalogue has these");
            // **Three, not four, since the book recipe caught up with §2.2.**
            // A book may hold two spells, two inks, an alignment and an
            // accessory now, so the best *possible* weapon item is a bigger
            // item than it was - and every rating in the slot is a fraction of
            // that ceiling, so every weapon piece deflated slightly.
            //
            // Exactly one piece crossed a boundary: **Stray Orb, 4g to 3g**.
            // Nothing else moved - not a rarity row, not a quota, not one
            // figure of the four-board table. The catalogue's own floor is
            // **2g**, so a multiplier still costs more than a blank, which is
            // what this test is about and what it still says.
            assert!(
                cheapest >= 3,
                "{:?} start at {}g, and the cheapest piece in the game is 2g - so this one \
                 is priced as a piece that does nothing",
                kind,
                cheapest
            );
        }
    }

    /// A banked pool is worth two things: what it buys when a trigger spends
    /// it, and what it does for the whole fight while it sits there - a point
    /// of faith is a point of both resistances, a point of nature a point of
    /// regeneration. Only the first was ever scored, which rated every
    /// faith-carrying component in the game between 0 and 13 and put all of
    /// them outside the top of their own bucket.
    #[test]
    fn banked_pools_are_worth_holding_not_only_spending() {
        let best_faith = CATALOG
            .iter()
            .filter(|d| d.base.faith > 0 && !crate::piece::is_boss_only(d.name))
            .map(piece_rating)
            .max()
            .expect("faith pieces exist");
        assert!(
            best_faith >= 25,
            "the best faith piece in the game rates {}, which is noise",
            best_faith
        );
    }

    /// The two curses that stop the other side's gear are worth more than the
    /// one that slows it, and all four used to cost the same. Nineteen pieces
    /// carry stun or misfire now, so pricing them as interchangeable made the
    /// two best curses in the game the two cheapest.
    #[test]
    fn a_curse_is_priced_by_what_it_does() {
        use crate::curse::CurseKind;
        let frost = curse_points(CurseKind::Frost);
        let stun = curse_points(CurseKind::Stun);
        let misfire = curse_points(CurseKind::Misfire);
        // `stun > frost` rather than `stun > frost * 2`.
        //
        // Frost used to be priced as though it slowed one item, which is what
        // made the gap that wide; it slows everything, and it is greaves' own
        // curse now, so the slot's signature mechanic was being paid a fraction
        // of what it does. Corrected against a deliberately low assumption -
        // two items, where a built board runs eight to nineteen - so the gap
        // narrows and the ordering the design asks for survives. The ordering
        // is the claim; the size of the gap never was.
        assert!(stun > frost, "a stun denies more than a frost: {} vs {}", stun, frost);
        assert!(misfire > stun, "a misfire denies more than a stun: {} vs {}", misfire, stun);
    }

    /// Speed must not be counted twice. Given the cadence an item actually
    /// runs at, its speed is already in every per-second figure; the
    /// percentage on top is only for a piece nobody has placed yet.
    #[test]
    fn speed_is_counted_once() {
        let fast = CATALOG
            .iter()
            .filter(|d| d.speed_bonus > 30 && !crate::piece::is_boss_only(d.name))
            .max_by_key(|d| d.speed_bonus)
            .expect("something in the game is quick");
        let cd = default_cooldown_ms(fast.slot);
        // Rated at its own slot's default cadence, explicitly, against rated
        // with "work it out yourself". The first has speed in the rate; the
        // second does not and adds the percentage instead. They must not both
        // apply, so the explicit one is the lower of the two.
        let explicit = piece_rating_at(fast, cd);
        let implicit = piece_rating_at(fast, 0);
        assert!(
            explicit < implicit,
            "{}: {} at its real cadence vs {} unplaced - speed is being counted twice",
            fast.name,
            explicit,
            implicit
        );
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
                // Everything off the scale, not just boss gear: a VIP piece
                // is meant to out-rate the shop too, and comparing a trophy
                // against one would be comparing two exemptions.
                //
                // **Event-only pieces are a third exemption**, and this said
                // "anything a player can buy" while comparing against things
                // no shop stocks. The Green Ledger is handed over by a door
                // and is priced like it; it out-rated a boss trophy the moment
                // T2 spelled its faith the way the other 158 pieces spell it,
                // and it had been out of scope for this question all along.
                .filter(|c| {
                    c.slot == d.slot
                        && c.kind == d.kind
                        && !crate::piece::is_off_the_scale(c.name)
                        && !crate::piece::is_event_only(c.name)
                })
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
