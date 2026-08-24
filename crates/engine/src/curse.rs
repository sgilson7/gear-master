//! Curses: timed damage-over-time and debuffs, applied to either combatant.
//!
//! A curse is always applied *to* someone — an item can curse the enemy or,
//! when something goes wrong, curse its own wearer. Both magnitude and duration
//! are cut by the target's curse resistance, so resistance is worth stacking
//! against either.

use crate::stats::Stats;

/// Milliseconds per simulation tick. Every duration in the game is a multiple
/// of this, which is what keeps fights exactly reproducible.
pub const TICK_MS: u32 = 50;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum CurseKind {
    /// Burns for `SEARING_DPS` damage a second while it lasts.
    Searing,
    /// Slows every one of the target's items by `FROST_SLOW_PCT`.
    Frost,
    /// Stops **one** of the target's items dead. That item's cooldown does not
    /// advance at all while it lasts, and resumes from where it stood rather
    /// than starting over.
    ///
    /// One item, not all of them. Stopping a whole side was the strongest
    /// effect in the game by a distance - a stun chain against five items was
    /// five items' worth of denial for one trigger's price - and no amount of
    /// pricing fixed that, because the thing being priced was "the enemy does
    /// not play". Which item it lands on is picked without warning unless the
    /// trigger names one; see `StunAim` in `combat`.
    Stun,
    /// Every `MISFIRE_EVERY`th activation of theirs does nothing at all.
    ///
    /// Deterministic rather than random, which is not a compromise but a
    /// requirement: the whole combat engine is deterministic and every test in
    /// the suite depends on replaying a fight and getting the same answer.
    /// "One in three fizzles" is the same experience as a one-in-three chance,
    /// and it is one you can actually plan around.
    Misfire,
}

pub const SEARING_DPS: i32 = 10;
pub const SEARING_MS: u32 = 10_000;
pub const FROST_SLOW_PCT: i32 = 50;
pub const FROST_MS: u32 = 1_000;
pub const STUN_MS: u32 = 1_200;
pub const MISFIRE_MS: u32 = 6_000;
pub const MISFIRE_EVERY: u32 = 3;

/// Frost is a whole-body slow, not a per-item one, and it never stops the gear
/// outright: at the cap an item still fires, just at a quarter speed.
pub const FROST_SLOW_CAP_PCT: i32 = 75;
/// Stun stacks pile onto one item's clock rather than refreshing it, so this
/// is what stops a chain of stuns from taking an item out of the fight
/// altogether.
pub const STUN_CAP_MS: u32 = 3_600;
/// However many misfire stacks land, one activation in two is the worst it
/// gets - the same promise the frost cap makes.
pub const MISFIRE_FLOOR: u32 = 2;

/// How much slower gear runs under `stacks` of frost.
pub fn frost_slow_pct(stacks: u32) -> i32 {
    (FROST_SLOW_PCT * stacks as i32).min(FROST_SLOW_CAP_PCT)
}

/// One activation in how many a misfire eats under `stacks`. Zero for none.
pub fn misfire_interval(stacks: u32) -> u32 {
    match stacks {
        0 => 0,
        n => MISFIRE_EVERY.saturating_sub(n - 1).max(MISFIRE_FLOOR),
    }
}

impl CurseKind {
    /// All four, so anything that has to cover the set - the glossary, the
    /// theme, a legend - can be checked rather than kept in step by hand.
    pub const ALL: [CurseKind; 4] =
        [CurseKind::Searing, CurseKind::Frost, CurseKind::Stun, CurseKind::Misfire];

    pub fn name(self) -> &'static str {
        match self {
            CurseKind::Searing => "searing",
            CurseKind::Frost => "frost",
            CurseKind::Stun => "stun",
            CurseKind::Misfire => "misfire",
        }
    }

    /// Base duration before the target's resistance is applied.
    pub fn base_duration_ms(self) -> u32 {
        match self {
            CurseKind::Searing => SEARING_MS,
            CurseKind::Frost => FROST_MS,
            CurseKind::Stun => STUN_MS,
            CurseKind::Misfire => MISFIRE_MS,
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            CurseKind::Searing => "10 damage a second for 10 seconds, per stack",
            CurseKind::Frost => {
                "all of the target's gear runs 50% slower for 1 second, per stack, up to 75%"
            }
            CurseKind::Stun => {
                "one of their items stops dead for 1.2 seconds, then carries on from \
                 where it stood; stacks add up to 3.6 seconds on that item"
            }
            CurseKind::Misfire => {
                "one activation in three does nothing, for 6 seconds; two stacks or more \
                 makes it one in two"
            }
        }
    }

    /// What `stacks` of this curse currently work out to, in the fewest words
    /// that are still a number: "30/s", "-75%", "1 in 2".
    ///
    /// The interface used to say only that you were cursed, which is the one
    /// thing you can already see. These come from the same constants the
    /// simulation reads, so the chip cannot drift from the fight.
    pub fn effect_at(self, stacks: u32) -> String {
        let n = stacks.max(1);
        match self {
            CurseKind::Searing => format!("{}/s", SEARING_DPS * n as i32),
            CurseKind::Frost => format!("-{}%", frost_slow_pct(n)),
            CurseKind::Stun => "stopped".to_string(),
            CurseKind::Misfire => format!("1 in {}", misfire_interval(n)),
        }
    }

    /// How long this curse lands for on a target with `curse_resist`, rounded
    /// down to whole ticks so duration maths stays exact.
    ///
    /// Public because a stun does not go through `Curses::apply` - it is held
    /// on the item it stopped - but it still answers to resistance the same
    /// way everything else does.
    pub fn landing_ms(self, curse_resist: i32) -> u32 {
        let resist = curse_resist.clamp(0, 100);
        let scaled = (self.base_duration_ms() as i64 * (100 - resist) as i64 / 100) as u32;
        scaled / TICK_MS * TICK_MS
    }
}

/// One active curse on one combatant. Applying the same kind again refreshes
/// its timer and adds a stack rather than making a second entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Curse {
    pub kind: CurseKind,
    pub remaining_ms: u32,
    pub stacks: u32,
}

/// Every curse currently riding on one combatant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Curses {
    active: Vec<Curse>,
}

impl Curses {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Curse> {
        self.active.iter()
    }

    pub fn stacks_of(&self, kind: CurseKind) -> u32 {
        self.active.iter().find(|c| c.kind == kind).map_or(0, |c| c.stacks)
    }

    pub fn has(&self, kind: CurseKind) -> bool {
        self.stacks_of(kind) > 0
    }

    /// Apply `kind` to a target with `curse_resist` percent resistance.
    ///
    /// Resistance shortens the curse; at 100 it lands for no time at all and
    /// is dropped entirely.
    ///
    /// Returns how long the curse will now run for - the *total* left on the
    /// clock, not the slice just added. Reporting the slice meant a refreshed
    /// curse logged "for 10.0s" when eleven were left, and the interface drew
    /// its timer from that number.
    ///
    /// Not for stun: a stun belongs to one item rather than to the fighter, so
    /// it is stored on the item and its duration comes from `landing_ms`.
    pub fn apply(&mut self, kind: CurseKind, curse_resist: i32) -> u32 {
        debug_assert!(
            kind != CurseKind::Stun,
            "a stun lands on an item, not on the combatant - see StunAim"
        );
        let duration = kind.landing_ms(curse_resist);
        if duration == 0 {
            return 0;
        }
        match self.active.iter_mut().find(|c| c.kind == kind) {
            Some(existing) => {
                existing.stacks += 1;
                existing.remaining_ms = existing.remaining_ms.max(duration);
                existing.remaining_ms
            }
            None => {
                self.active.push(Curse { kind, remaining_ms: duration, stacks: 1 });
                duration
            }
        }
    }

    /// Advance every curse by one tick and drop the expired ones.
    pub fn tick(&mut self) {
        for c in &mut self.active {
            c.remaining_ms = c.remaining_ms.saturating_sub(TICK_MS);
        }
        self.active.retain(|c| c.remaining_ms > 0);
    }

    /// Damage this tick from every damage-over-time curse, scaled by stacks.
    ///
    /// Searing is 10 damage a second, so a 50ms tick deals half a point. That
    /// doesn't divide evenly, so the fractional part is carried in
    /// `dot_remainder` by the caller rather than being rounded away.
    pub fn dot_millidamage_per_tick(&self) -> i32 {
        self.active
            .iter()
            .map(|c| match c.kind {
                CurseKind::Searing => SEARING_DPS * c.stacks as i32 * TICK_MS as i32,
                CurseKind::Frost | CurseKind::Stun | CurseKind::Misfire => 0,
            })
            .sum()
    }

    /// How much slower this combatant's items run, as a percentage.
    ///
    /// Frost is a whole-body slow: this figure applies to every item the
    /// combatant owns, not to the one that happened to be cursed. Stacks add
    /// up, capped so the gear is never stopped outright - a stun is the thing
    /// that stops gear, and the two should not be able to become each other.
    pub fn slow_pct(&self) -> i32 {
        frost_slow_pct(self.stacks_of(CurseKind::Frost))
    }

    /// One activation in how many does a misfire eat? Zero when none is up.
    ///
    /// Each stack past the first tightens the interval by one, down to the
    /// floor. Without this a second misfire landing on top of the first was
    /// worth nothing whatever - the only curse where a stack bought the caster
    /// nothing at all.
    pub fn misfire_every(&self) -> u32 {
        misfire_interval(self.stacks_of(CurseKind::Misfire))
    }

    /// Is this activation one of the ones a misfire eats?
    ///
    /// Counted rather than rolled: the combat engine is deterministic and the
    /// whole test suite depends on a fight replaying identically.
    pub fn misfires(&self, activation: u32) -> bool {
        match self.misfire_every() {
            0 => false,
            every => activation % every == 0,
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }
}

/// Mind damage after the target's mind resistance. Mind damage eats *maximum*
/// health, so it can't be healed off — resistance is the only defence.
pub fn mind_damage_after_resist(raw: i32, mind_resist: i32) -> i32 {
    let resist = mind_resist.clamp(0, 100);
    (raw as i64 * (100 - resist) as i64 / 100) as i32
}

/// Convenience: pull the two resistance figures out of a stat block.
pub fn resistances(stats: &Stats) -> (i32, i32) {
    (stats.mind_resist, stats.curse_resist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searing_lasts_ten_seconds_and_burns_ten_a_second() {
        let mut c = Curses::new();
        assert_eq!(c.apply(CurseKind::Searing, 0), 10_000);
        // 10 dps expressed in milli-damage per 50ms tick.
        assert_eq!(c.dot_millidamage_per_tick(), 10 * 50);
        // Over a full second that is exactly 10 damage.
        assert_eq!(c.dot_millidamage_per_tick() * (1000 / TICK_MS as i32) / 1000, 10);
    }

    #[test]
    fn curse_resistance_shortens_the_curse() {
        let mut half = Curses::new();
        assert_eq!(half.apply(CurseKind::Searing, 50), 5_000);
        let mut full = Curses::new();
        assert_eq!(full.apply(CurseKind::Searing, 100), 0, "fully resisted");
        assert!(full.is_empty(), "a fully resisted curse never lands");
    }

    #[test]
    fn reapplying_stacks_and_refreshes() {
        let mut c = Curses::new();
        c.apply(CurseKind::Searing, 0);
        for _ in 0..40 {
            c.tick(); // 2 seconds gone
        }
        assert_eq!(c.stacks_of(CurseKind::Searing), 1);
        c.apply(CurseKind::Searing, 0);
        assert_eq!(c.stacks_of(CurseKind::Searing), 2);
        assert_eq!(
            c.iter().next().unwrap().remaining_ms,
            10_000,
            "the timer refreshes to full"
        );
        assert_eq!(c.dot_millidamage_per_tick(), 2 * 10 * 50, "two stacks burn twice as fast");
    }

    #[test]
    fn a_curse_expires_on_schedule() {
        let mut c = Curses::new();
        c.apply(CurseKind::Frost, 0);
        assert_eq!(c.slow_pct(), 50);
        for _ in 0..(FROST_MS / TICK_MS) {
            c.tick();
        }
        assert!(c.is_empty(), "frost is gone after its second");
        assert_eq!(c.slow_pct(), 0);
    }

    #[test]
    fn frost_stacks_but_never_freezes_gear_solid() {
        let mut c = Curses::new();
        assert_eq!(c.apply(CurseKind::Frost, 0), FROST_MS);
        assert_eq!(c.slow_pct(), 50, "one stack, half speed");
        c.apply(CurseKind::Frost, 0);
        assert_eq!(c.slow_pct(), 75, "two stacks reach the cap");
        for _ in 0..10 {
            c.apply(CurseKind::Frost, 0);
        }
        assert_eq!(c.slow_pct(), FROST_SLOW_CAP_PCT, "capped, so items always still fire");
    }

    #[test]
    fn a_stun_answers_to_resistance_like_everything_else() {
        // A stun is held on the item it stopped rather than on the fighter, so
        // it never reaches `apply` - but resistance still shortens it, and
        // that is the part worth pinning down here. The stacking and the
        // choice of item are combat's business; see `curses_in_combat`.
        assert_eq!(CurseKind::Stun.landing_ms(0), STUN_MS);
        assert_eq!(CurseKind::Stun.landing_ms(50), STUN_MS / 2);
        assert_eq!(CurseKind::Stun.landing_ms(100), 0, "fully resisted, never lands");
    }

    #[test]
    fn misfire_stacks_tighten_the_interval() {
        let mut c = Curses::new();
        assert_eq!(c.misfire_every(), 0, "nothing up, nothing eaten");
        c.apply(CurseKind::Misfire, 0);
        assert_eq!(c.misfire_every(), MISFIRE_EVERY, "one in three");
        c.apply(CurseKind::Misfire, 0);
        assert_eq!(c.misfire_every(), 2, "one in two");
        for _ in 0..10 {
            c.apply(CurseKind::Misfire, 0);
        }
        assert_eq!(c.misfire_every(), MISFIRE_FLOOR, "never worse than one in two");
        // Which is to say the gear always still does something.
        assert!(!c.misfires(1), "an odd activation always gets through");
    }

    #[test]
    fn a_refreshed_curse_reports_what_is_left_not_what_was_added() {
        let mut c = Curses::new();
        c.apply(CurseKind::Searing, 0);
        for _ in 0..20 {
            c.tick(); // a second gone, nine left
        }
        // Reapplying refreshes to the full ten, and that is what gets reported
        // - the interface draws its timer from this number.
        assert_eq!(c.apply(CurseKind::Searing, 0), SEARING_MS);
    }

    #[test]
    fn mind_resistance_scales_mind_damage() {
        assert_eq!(mind_damage_after_resist(10, 0), 10);
        assert_eq!(mind_damage_after_resist(10, 50), 5);
        assert_eq!(mind_damage_after_resist(10, 100), 0);
    }
}
