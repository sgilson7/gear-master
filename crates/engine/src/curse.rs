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
    /// Stops the target's gear dead. Nothing of theirs advances at all while
    /// it lasts, so every cooldown they were part-way through resumes from
    /// where it stood rather than starting over.
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

impl CurseKind {
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
            CurseKind::Searing => "10 damage a second for 10 seconds",
            CurseKind::Frost => "gear runs 50% slower for 1 second",
            CurseKind::Stun => "gear stops dead for 1.2 seconds, then carries on from where it stood",
            CurseKind::Misfire => "one activation in three does nothing, for 6 seconds",
        }
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
    /// is dropped entirely. Returns the duration that actually stuck, so the
    /// combat log can report what happened rather than what was intended.
    pub fn apply(&mut self, kind: CurseKind, curse_resist: i32) -> u32 {
        let resist = curse_resist.clamp(0, 100);
        let base = kind.base_duration_ms() as i64;
        let scaled = (base * (100 - resist) as i64 / 100) as u32;
        // Round down to whole ticks so duration maths stays exact.
        let duration = scaled / TICK_MS * TICK_MS;
        if duration == 0 {
            return 0;
        }
        match self.active.iter_mut().find(|c| c.kind == kind) {
            Some(existing) => {
                existing.stacks += 1;
                existing.remaining_ms = existing.remaining_ms.max(duration);
            }
            None => self.active.push(Curse { kind, remaining_ms: duration, stacks: 1 }),
        }
        duration
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

    /// How much slower this combatant's items run, as a percentage. Frost
    /// stacks add up but can never stop the gear completely.
    pub fn slow_pct(&self) -> i32 {
        let raw: i32 = self
            .active
            .iter()
            .map(|c| match c.kind {
                CurseKind::Frost => FROST_SLOW_PCT * c.stacks as i32,
                CurseKind::Searing | CurseKind::Stun | CurseKind::Misfire => 0,
            })
            .sum();
        raw.min(90)
    }

    /// Is the gear stopped dead? A stun does not slow anything; it stops
    /// everything, and what was part-way through stays part-way through.
    pub fn stunned(&self) -> bool {
        self.has(CurseKind::Stun)
    }

    /// Is this activation one of the ones a misfire eats?
    ///
    /// Counted rather than rolled: the combat engine is deterministic and the
    /// whole test suite depends on a fight replaying identically.
    pub fn misfires(&self, activation: u32) -> bool {
        self.has(CurseKind::Misfire) && activation % MISFIRE_EVERY == 0
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
        for _ in 0..10 {
            c.apply(CurseKind::Frost, 0);
        }
        assert_eq!(c.slow_pct(), 90, "capped, so items always still fire");
    }

    #[test]
    fn mind_resistance_scales_mind_damage() {
        assert_eq!(mind_damage_after_resist(10, 0), 10);
        assert_eq!(mind_damage_after_resist(10, 50), 5);
        assert_eq!(mind_damage_after_resist(10, 100), 0);
    }
}
