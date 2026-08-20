use std::ops::{Add, AddAssign};

/// Every number the game tracks, in one flat bag so pieces, bonuses and
/// characters all speak the same language.
///
/// `power` is the weapon damage multiplier expressed in **hundredths** — a
/// character with `power = 250` swings at 2.50x. Integers keep combat exactly
/// reproducible, which is what lets the tests assert on damage numbers.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub health: i32,
    pub strength: i32,
    pub regen: i32,
    pub power: i32,
}

/// A character with no gear at all. An unequipped run is a losing run — that
/// is deliberate, it is what makes assembling gear matter.
pub const BASE_HEALTH: i32 = 100;
pub const BASE_STRENGTH: i32 = 5;
pub const BASE_REGEN: i32 = 0;
/// 100 hundredths == a bare-handed 1.00x multiplier.
pub const BASE_POWER: i32 = 100;

/// Names one field of `Stats`, so effects can talk about "the strength of that
/// piece" without hard-coding which field they mean.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum StatKind {
    Health,
    Strength,
    Regen,
    Power,
}

impl StatKind {
    pub fn name(self) -> &'static str {
        match self {
            StatKind::Health => "health",
            StatKind::Strength => "strength",
            StatKind::Regen => "regen",
            StatKind::Power => "power",
        }
    }
}

impl Stats {
    pub const ZERO: Stats = Stats { health: 0, strength: 0, regen: 0, power: 0 };

    pub fn get(&self, k: StatKind) -> i32 {
        match k {
            StatKind::Health => self.health,
            StatKind::Strength => self.strength,
            StatKind::Regen => self.regen,
            StatKind::Power => self.power,
        }
    }

    pub fn set(&mut self, k: StatKind, v: i32) {
        match k {
            StatKind::Health => self.health = v,
            StatKind::Strength => self.strength = v,
            StatKind::Regen => self.regen = v,
            StatKind::Power => self.power = v,
        }
    }

    pub fn add(&mut self, k: StatKind, v: i32) {
        let cur = self.get(k);
        self.set(k, cur + v);
    }

    pub const fn new(health: i32, strength: i32, regen: i32, power: i32) -> Self {
        Stats { health, strength, regen, power }
    }

    pub const fn health(health: i32) -> Self {
        Stats { health, ..Stats::ZERO }
    }
    pub const fn strength(strength: i32) -> Self {
        Stats { strength, ..Stats::ZERO }
    }
    pub const fn regen(regen: i32) -> Self {
        Stats { regen, ..Stats::ZERO }
    }
    pub const fn power(power: i32) -> Self {
        Stats { power, ..Stats::ZERO }
    }

    /// The character's starting point before any gear is considered.
    pub const fn base_character() -> Self {
        Stats::new(BASE_HEALTH, BASE_STRENGTH, BASE_REGEN, BASE_POWER)
    }

    /// Damage per attack: strength scaled by the weapon multiplier.
    /// `power` is in hundredths, so this is `strength * power / 100`.
    pub fn damage_per_attack(&self) -> i32 {
        (self.strength * self.power / 100).max(0)
    }

    /// Short "+5 str, +12 hp" style summary. Empty string when nothing is set.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.health != 0 {
            parts.push(format!("{:+} hp", self.health));
        }
        if self.strength != 0 {
            parts.push(format!("{:+} str", self.strength));
        }
        if self.regen != 0 {
            parts.push(format!("{:+} regen", self.regen));
        }
        if self.power != 0 {
            parts.push(format!("{:+}.{:02}x power", self.power / 100, (self.power % 100).abs()));
        }
        parts.join(", ")
    }
}

impl Add for Stats {
    type Output = Stats;
    fn add(self, o: Stats) -> Stats {
        Stats {
            health: self.health + o.health,
            strength: self.strength + o.strength,
            regen: self.regen + o.regen,
            power: self.power + o.power,
        }
    }
}

impl AddAssign for Stats {
    fn add_assign(&mut self, o: Stats) {
        *self = *self + o;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_scales_strength_by_the_power_multiplier() {
        assert_eq!(Stats::new(0, 10, 0, 100).damage_per_attack(), 10);
        assert_eq!(Stats::new(0, 10, 0, 250).damage_per_attack(), 25);
        assert_eq!(Stats::new(0, 24, 0, 325).damage_per_attack(), 78);
    }

    #[test]
    fn stats_add_componentwise() {
        let mut s = Stats::base_character();
        s += Stats::health(20) + Stats::strength(3);
        assert_eq!(s.health, 120);
        assert_eq!(s.strength, 8);
        assert_eq!(s.power, 100, "power untouched by a health/strength bonus");
    }
}
