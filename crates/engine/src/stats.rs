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
    /// Flat damage an item deals each time it activates.
    pub damage: i32,
    /// Temporary hit points granted per activation. Armour starts every combat
    /// at zero and soaks damage before health does.
    pub armor: i32,
    /// Mana granted per activation. Items spend it to trigger extra effects.
    pub mana: i32,
    /// Mind damage per activation: small numbers, but it eats *maximum*
    /// health, so it can never be healed back.
    pub mind: i32,
    /// Percent reduction to incoming mind damage.
    pub mind_resist: i32,
    /// Percent reduction to the duration of curses landed on you.
    pub curse_resist: i32,

    // ---- typed damage ----------------------------------------------------
    //
    // Damage carries a type, and each type has a matching triangle of
    // defences: resistance cuts it, piercing ignores resistance, hardening
    // blunts piercing. All in whole percent.
    //
    //   effective piercing  = piercing  x (1 - hardening / 100)
    //   effective resistance= resistance x (1 - effective piercing / 100)
    //   damage taken        = raw        x (1 - effective resistance / 100)
    //
    // So stacking resistance alone loses to a pierced attacker, and stacking
    // piercing alone loses to a hardened one.
    /// Flat physical damage added to what an item lands.
    pub physical_damage: i32,
    pub physical_resist: i32,
    pub physical_pierce: i32,
    pub physical_harden: i32,
    /// Flat magic damage added to what an item lands.
    pub magic_damage: i32,
    pub magic_resist: i32,
    pub magic_pierce: i32,
    pub magic_harden: i32,

    // ---- stacking resources ---------------------------------------------
    //
    // Banked between activations and spent by triggers, exactly like mana.
    // Each also does something merely by being held.
    /// Fury. Every point adds to physical damage while you hold it.
    pub rage: i32,
    /// Conviction. Every point adds resistance of both types while held.
    pub faith: i32,
    /// Growth. Every point adds regeneration while held.
    pub nature: i32,
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
    Damage,
    Armor,
    Mana,
    Mind,
    MindResist,
    CurseResist,
    PhysicalDamage,
    PhysicalResist,
    PhysicalPierce,
    PhysicalHarden,
    MagicDamage,
    MagicResist,
    MagicPierce,
    MagicHarden,
    Rage,
    Faith,
    Nature,
}

impl StatKind {
    pub fn name(self) -> &'static str {
        match self {
            StatKind::Health => "health",
            StatKind::Strength => "strength",
            StatKind::Regen => "regen",
            StatKind::Power => "power",
            StatKind::Damage => "damage",
            StatKind::Armor => "armor",
            StatKind::Mana => "mana",
            StatKind::Mind => "mind damage",
            StatKind::MindResist => "mind resist",
            StatKind::CurseResist => "curse resist",
            StatKind::PhysicalDamage => "physical damage",
            StatKind::PhysicalResist => "physical resist",
            StatKind::PhysicalPierce => "physical piercing",
            StatKind::PhysicalHarden => "physical hardening",
            StatKind::MagicDamage => "magic damage",
            StatKind::MagicResist => "magic resist",
            StatKind::MagicPierce => "magic piercing",
            StatKind::MagicHarden => "magic hardening",
            StatKind::Rage => "rage",
            StatKind::Faith => "faith",
            StatKind::Nature => "nature",
        }
    }
}

impl Stats {
    pub const ZERO: Stats = Stats {
        health: 0,
        strength: 0,
        regen: 0,
        power: 0,
        damage: 0,
        armor: 0,
        mana: 0,
        mind: 0,
        mind_resist: 0,
        curse_resist: 0,
        physical_damage: 0,
        physical_resist: 0,
        physical_pierce: 0,
        physical_harden: 0,
        magic_damage: 0,
        magic_resist: 0,
        magic_pierce: 0,
        magic_harden: 0,
        rage: 0,
        faith: 0,
        nature: 0,
    };

    pub fn get(&self, k: StatKind) -> i32 {
        match k {
            StatKind::Health => self.health,
            StatKind::Strength => self.strength,
            StatKind::Regen => self.regen,
            StatKind::Power => self.power,
            StatKind::Damage => self.damage,
            StatKind::Armor => self.armor,
            StatKind::Mana => self.mana,
            StatKind::Mind => self.mind,
            StatKind::MindResist => self.mind_resist,
            StatKind::CurseResist => self.curse_resist,
            StatKind::PhysicalDamage => self.physical_damage,
            StatKind::PhysicalResist => self.physical_resist,
            StatKind::PhysicalPierce => self.physical_pierce,
            StatKind::PhysicalHarden => self.physical_harden,
            StatKind::MagicDamage => self.magic_damage,
            StatKind::MagicResist => self.magic_resist,
            StatKind::MagicPierce => self.magic_pierce,
            StatKind::MagicHarden => self.magic_harden,
            StatKind::Rage => self.rage,
            StatKind::Faith => self.faith,
            StatKind::Nature => self.nature,
        }
    }

    pub fn set(&mut self, k: StatKind, v: i32) {
        match k {
            StatKind::Health => self.health = v,
            StatKind::Strength => self.strength = v,
            StatKind::Regen => self.regen = v,
            StatKind::Power => self.power = v,
            StatKind::Damage => self.damage = v,
            StatKind::Armor => self.armor = v,
            StatKind::Mana => self.mana = v,
            StatKind::Mind => self.mind = v,
            StatKind::MindResist => self.mind_resist = v,
            StatKind::CurseResist => self.curse_resist = v,
            StatKind::PhysicalDamage => self.physical_damage = v,
            StatKind::PhysicalResist => self.physical_resist = v,
            StatKind::PhysicalPierce => self.physical_pierce = v,
            StatKind::PhysicalHarden => self.physical_harden = v,
            StatKind::MagicDamage => self.magic_damage = v,
            StatKind::MagicResist => self.magic_resist = v,
            StatKind::MagicPierce => self.magic_pierce = v,
            StatKind::MagicHarden => self.magic_harden = v,
            StatKind::Rage => self.rage = v,
            StatKind::Faith => self.faith = v,
            StatKind::Nature => self.nature = v,
        }
    }

    pub fn add(&mut self, k: StatKind, v: i32) {
        let cur = self.get(k);
        self.set(k, cur + v);
    }

    /// The four original stats; everything added later defaults to zero.
    pub const fn new(health: i32, strength: i32, regen: i32, power: i32) -> Self {
        Stats { health, strength, regen, power, ..Stats::ZERO }
    }

    pub const fn damage(damage: i32) -> Self {
        Stats { damage, ..Stats::ZERO }
    }
    pub const fn armor(armor: i32) -> Self {
        Stats { armor, ..Stats::ZERO }
    }
    pub const fn mana(mana: i32) -> Self {
        Stats { mana, ..Stats::ZERO }
    }
    pub const fn mind(mind: i32) -> Self {
        Stats { mind, ..Stats::ZERO }
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
        if self.damage != 0 {
            parts.push(format!("{:+} dmg", self.damage));
        }
        if self.armor != 0 {
            parts.push(format!("{:+} armor", self.armor));
        }
        if self.mana != 0 {
            parts.push(format!("{:+} mana", self.mana));
        }
        if self.mind != 0 {
            parts.push(format!("{:+} mind", self.mind));
        }
        if self.mind_resist != 0 {
            parts.push(format!("{:+}% mind res", self.mind_resist));
        }
        if self.curse_resist != 0 {
            parts.push(format!("{:+}% curse res", self.curse_resist));
        }
        for (v, label) in [
            (self.physical_damage, "phys dmg"),
            (self.magic_damage, "magic dmg"),
            (self.rage, "rage"),
            (self.faith, "faith"),
            (self.nature, "nature"),
        ] {
            if v != 0 {
                parts.push(format!("{:+} {}", v, label));
            }
        }
        for (v, label) in [
            (self.physical_resist, "phys res"),
            (self.physical_pierce, "phys pierce"),
            (self.physical_harden, "phys harden"),
            (self.magic_resist, "magic res"),
            (self.magic_pierce, "magic pierce"),
            (self.magic_harden, "magic harden"),
        ] {
            if v != 0 {
                parts.push(format!("{:+}% {}", v, label));
            }
        }
        parts.join(", ")
    }
}

/// What is left of `raw` after the defender's resistance, the attacker's
/// piercing and the defender's hardening have had their say. See the note on
/// `Stats` for the shape of it.
pub fn after_defences(raw: i32, resist: i32, pierce: i32, harden: i32) -> i32 {
    if raw <= 0 {
        return 0;
    }
    let harden = harden.clamp(0, 100);
    let pierce = pierce.max(0);
    let effective_pierce = (pierce * (100 - harden) / 100).clamp(0, 100);
    let resist = resist.clamp(0, 95);
    let effective_resist = resist * (100 - effective_pierce) / 100;
    let kept = 100 - effective_resist;
    ((raw as i64 * kept as i64) / 100).max(0) as i32
}

impl Add for Stats {
    type Output = Stats;
    fn add(self, o: Stats) -> Stats {
        Stats {
            health: self.health + o.health,
            strength: self.strength + o.strength,
            regen: self.regen + o.regen,
            power: self.power + o.power,
            damage: self.damage + o.damage,
            armor: self.armor + o.armor,
            mana: self.mana + o.mana,
            mind: self.mind + o.mind,
            mind_resist: self.mind_resist + o.mind_resist,
            curse_resist: self.curse_resist + o.curse_resist,
            physical_damage: self.physical_damage + o.physical_damage,
            physical_resist: self.physical_resist + o.physical_resist,
            physical_pierce: self.physical_pierce + o.physical_pierce,
            physical_harden: self.physical_harden + o.physical_harden,
            magic_damage: self.magic_damage + o.magic_damage,
            magic_resist: self.magic_resist + o.magic_resist,
            magic_pierce: self.magic_pierce + o.magic_pierce,
            magic_harden: self.magic_harden + o.magic_harden,
            rage: self.rage + o.rage,
            faith: self.faith + o.faith,
            nature: self.nature + o.nature,
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
