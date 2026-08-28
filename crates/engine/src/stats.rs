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
    /// Percent of what your armour absorbs that is turned back on whoever
    /// swung. The body's only attack: it does nothing on a board that dies
    /// fast, and everything on one built to be hit.
    pub reflect: i32,

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
/// Deliberately NOT scaled with the gear. Gear health went up fivefold because
/// a late build had too little of it; the bare character did not, because a
/// character who cannot be killed by the first creature on the ladder has no
/// early game at all. What you are wearing is what keeps you alive now.
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
    /// Every stat, so something that acts on "all the numbers" can do so
    /// without listing them and forgetting one when a new field is added.
    pub const ALL: [StatKind; 20] = [
        StatKind::Health,
        StatKind::Strength,
        StatKind::Regen,
        StatKind::Power,
        StatKind::Armor,
        StatKind::Mana,
        StatKind::Mind,
        StatKind::MindResist,
        StatKind::CurseResist,
        StatKind::PhysicalDamage,
        StatKind::PhysicalResist,
        StatKind::PhysicalPierce,
        StatKind::PhysicalHarden,
        StatKind::MagicDamage,
        StatKind::MagicResist,
        StatKind::MagicPierce,
        StatKind::MagicHarden,
        StatKind::Rage,
        StatKind::Faith,
        StatKind::Nature,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StatKind::Health => "health",
            StatKind::Strength => "strength",
            StatKind::Regen => "regen",
            StatKind::Power => "power",
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
        reflect: 0,
        health: 0,
        strength: 0,
        regen: 0,
        power: 0,
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

    pub const fn physical(physical_damage: i32) -> Self {
        Stats { physical_damage, ..Stats::ZERO }
    }
    pub const fn magic(magic_damage: i32) -> Self {
        Stats { magic_damage, ..Stats::ZERO }
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

    /// Every number multiplied. What a solitude bonus does when it lands.
    pub fn times(self, n: i32) -> Stats {
        if n <= 1 {
            return self;
        }
        let mut out = self;
        for k in StatKind::ALL {
            let v = out.get(k);
            out.set(k, v * n);
        }
        out
    }

    /// Everything this contributes, multiplied by `pct` hundredths.
    ///
    /// What an item's own power does to its own numbers. `power` itself is
    /// left alone - it is the multiplier, not a thing being multiplied - and
    /// so are the percentage stats, which are already proportions and would
    /// mean nothing scaled: a piece with 40% resistance and a 3x multiplier
    /// does not resist 120% of anything.
    pub fn powered(self, pct: i32) -> Stats {
        if pct == 100 {
            return self;
        }
        let m = |v: i32| ((v as i64 * pct as i64) / 100) as i32;
        Stats {
            health: m(self.health),
            strength: m(self.strength),
            regen: m(self.regen),
            physical_damage: m(self.physical_damage),
            magic_damage: m(self.magic_damage),
            armor: m(self.armor),
            mana: m(self.mana),
            mind: m(self.mind),
            rage: m(self.rage),
            faith: m(self.faith),
            nature: m(self.nature),
            ..self
        }
    }

    /// Damage per attack: strength scaled by the weapon multiplier.
    /// `power` is in hundredths, so this is `strength * power / 100`.
    pub fn damage_per_attack(&self) -> i32 {
        (self.strength * self.power / 100).max(0)
    }

    /// Short "+5 str, +12 hp" style summary. Empty string when nothing is set.
    /// Every non-zero field, as the words for it and the symbol that stands
    /// for it.
    ///
    /// `summary` is this joined with commas and has been the only reading of a
    /// `Stats` for as long as there has been one - which is why a card could
    /// draw a nature glyph in its keyword rail and the words "+1 nature" in
    /// its body and never put the two together. The second element is the key
    /// `draw_keyword` draws, empty where nothing draws it.
    ///
    /// One traversal, two outputs. The alternative is a second walk over these
    /// twenty fields in the interface, and this repository has just finished
    /// deleting four copies of a walker for exactly that reason.
    pub fn parts(&self) -> Vec<(String, &'static str)> {
        self.parts_when().into_iter().map(|(t, g, _)| (t, g)).collect()
    }

    /// The block as text, grouped by when each figure happens.
    ///
    /// `summary` is the same figures run together with commas, which is right
    /// for a one-line total and wrong for a card: it prints a rate beside a
    /// quantity and says nothing about which is which. This is what a driver
    /// with no colours to work with prints instead.
    ///
    /// Empty groups are left out, so a piece that is only passive reads the
    /// way it always did.
    pub fn summary_by_when(&self) -> Vec<(When, String)> {
        let mut out = Vec::new();
        for group in [When::Damage, When::Passive, When::OnActivation] {
            let joined: Vec<String> = self
                .parts_when()
                .into_iter()
                .filter(|(_, _, w)| *w == group)
                .map(|(t, ..)| t)
                .collect();
            if !joined.is_empty() {
                out.push((group, joined.join(", ")));
            }
        }
        out
    }

    /// The same figures, each saying when it happens.
    ///
    /// `parts` is this with the third element dropped, so the two cannot
    /// disagree about what a block contains - the same argument that put
    /// `parts` and `summary` on one traversal in the first place.
    pub fn parts_when(&self) -> Vec<(String, &'static str, When)> {
        let mut parts: Vec<(String, &'static str, When)> = Vec::new();
        if self.health != 0 {
            parts.push((format!("{:+} hp", self.health), "", When::Passive));
        }
        if self.strength != 0 {
            parts.push((format!("{:+} str", self.strength), "", When::Passive));
        }
        if self.regen != 0 {
            parts.push((format!("{:+} regen", self.regen), "nature", When::Passive));
        }
        if self.power != 0 {
            // Power reaches the item carrying it and nothing else, so the
            // summary says whose it is.
            parts.push((
                format!("{:+}.{:02}x its own power", self.power / 100, (self.power % 100).abs()),
                "speed",
                When::Passive,
            ));
        }
        if self.armor != 0 {
            parts.push((format!("{:+} armor", self.armor), "armor", When::OnActivation));
        }
        if self.mana != 0 {
            parts.push((format!("{:+} mana", self.mana), "mana", When::OnActivation));
        }
        if self.mind != 0 {
            parts.push((format!("{:+} mind", self.mind), "mind", When::Damage));
        }
        if self.mind_resist != 0 {
            parts.push((format!("{:+}% mind res", self.mind_resist), "mind", When::Passive));
        }
        if self.curse_resist != 0 {
            parts.push((format!("{:+}% curse res", self.curse_resist), "curse", When::Passive));
        }
        for (v, label, glyph, when) in [
            (self.physical_damage, "phys dmg", "physical", When::Damage),
            (self.magic_damage, "magic dmg", "magic", When::Damage),
            (self.rage, "rage", "rage", When::OnActivation),
            (self.faith, "faith", "faith", When::OnActivation),
            (self.nature, "nature", "nature", When::OnActivation),
        ] {
            if v != 0 {
                parts.push((format!("{:+} {}", v, label), glyph, when));
            }
        }
        for (v, label, glyph) in [
            (self.physical_resist, "phys res", "physical"),
            (self.physical_pierce, "phys pierce", "physical"),
            (self.physical_harden, "phys harden", "physical"),
            (self.magic_resist, "magic res", "magic"),
            (self.magic_pierce, "magic pierce", "magic"),
            (self.magic_harden, "magic harden", "magic"),
        ] {
            if v != 0 {
                parts.push((format!("{:+}% {}", v, label), glyph, When::Passive));
            }
        }
        parts
    }

    pub fn summary(&self) -> String {
        self.parts().into_iter().map(|(t, _)| t).collect::<Vec<_>>().join(", ")
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

/// When a figure on a stat block actually happens.
///
/// A `Stats` is not a block of passive numbers and never was. Eight of its
/// fields are handed over on **every activation**, by the same code path an
/// `OnActivate` trigger uses - so a card that prints `+2 nature` beside
/// `+175 hp` is printing a rate beside a quantity and saying they are the
/// same kind of thing. Over a thirty-second fight on a 2.8-second item they
/// are not close.
///
/// Kept here rather than in the interface because three surfaces print these
/// figures and they already disagreed about two of them. A field added later
/// cannot be printed by anything until somebody has said when it happens.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum When {
    /// True while it is worn, and true between fights.
    Passive,
    /// Handed over once, every time the item fires.
    OnActivation,
    /// Handed over on activation, and aimed at somebody.
    ///
    /// Split from `OnActivation` because damage is totalled rather than
    /// listed, and the total is the figure a reader came for. It is also the
    /// only group whose parts go through the item's own power before they
    /// mean anything.
    Damage,
}

impl When {
    /// The heading this group is drawn under.
    pub fn heading(self) -> &'static str {
        match self {
            When::Damage => "DAMAGE",
            When::Passive => "PASSIVE",
            When::OnActivation => "EVERY TIME IT FIRES",
        }
    }
}

impl Stats {
    /// Every field taken to `percent` of itself, rounding toward zero.
    ///
    /// Used for assembly bonuses under Recycler. Percentages are applied to
    /// the whole lump rather than field by field on purpose: a bonus is one
    /// thing a component pays, and scaling half of it would be a different
    /// bonus.
    pub fn scaled(self, percent: i32) -> Stats {
        let pct = |v: i32| (v as i64 * percent as i64 / 100) as i32;
        Stats {
            reflect: pct(self.reflect),
            health: pct(self.health),
            strength: pct(self.strength),
            regen: pct(self.regen),
            power: pct(self.power),
            armor: pct(self.armor),
            mana: pct(self.mana),
            mind: pct(self.mind),
            mind_resist: pct(self.mind_resist),
            curse_resist: pct(self.curse_resist),
            physical_damage: pct(self.physical_damage),
            physical_resist: pct(self.physical_resist),
            physical_pierce: pct(self.physical_pierce),
            physical_harden: pct(self.physical_harden),
            magic_damage: pct(self.magic_damage),
            magic_resist: pct(self.magic_resist),
            magic_pierce: pct(self.magic_pierce),
            magic_harden: pct(self.magic_harden),
            rage: pct(self.rage),
            faith: pct(self.faith),
            nature: pct(self.nature),
        }
    }
}

impl Add for Stats {
    type Output = Stats;
    fn add(self, o: Stats) -> Stats {
        Stats {
            reflect: self.reflect + o.reflect,
            health: self.health + o.health,
            strength: self.strength + o.strength,
            regen: self.regen + o.regen,
            power: self.power + o.power,
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
        assert_eq!(s.health, BASE_HEALTH + 20);
        assert_eq!(s.strength, 8);
        assert_eq!(s.power, 100, "power untouched by a health/strength bonus");
    }
}
