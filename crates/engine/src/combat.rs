//! Combat: a fixed-timestep simulation where every assembled item runs its own
//! cooldown.
//!
//! There are no turns. The fight is stepped in [`TICK_MS`] slices and each item
//! fills its own bar independently, so a fast weapon really does swing more
//! often than a slow one. Nothing is random — the same loadout against the same
//! monster always produces the same log, which is what lets the tests assert on
//! exact numbers and lets the GUI replay a fight it did not simulate.

use crate::curse::{mind_damage_after_resist, CurseKind, Curses, TICK_MS};
use crate::loadout::ItemProfile;
use crate::piece::{Action, SlotKind, Target, Trigger};
use crate::stats::Stats;

/// How often damage-over-time is summarised into the log.
pub const BURN_REPORT_MS: u32 = 1000;

/// A fight this long is called a draw, so a build that cannot finish the job
/// doesn't hang the simulation.
pub const MAX_DURATION_MS: u32 = 60_000;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Side {
    Player,
    Enemy,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Player => Side::Enemy,
            Side::Enemy => Side::Player,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Side::Player => "You",
            Side::Enemy => "Enemy",
        }
    }
}

// ------------------------------------------------------------- monsters

/// One repeating attack belonging to a monster. Monsters use the same cooldown
/// machinery as the player's gear rather than a special case.
#[derive(Copy, Clone, Debug)]
pub struct MonsterAttack {
    pub name: &'static str,
    pub cooldown_ms: u32,
    pub damage: i32,
    pub mind: i32,
    pub armor: i32,
    /// Landed on the player each time this attack resolves.
    pub curse: Option<CurseKind>,
}

impl MonsterAttack {
    pub const fn hit(name: &'static str, cooldown_ms: u32, damage: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage, mind: 0, armor: 0, curse: None }
    }
    pub const fn cursing(
        name: &'static str,
        cooldown_ms: u32,
        damage: i32,
        curse: CurseKind,
    ) -> Self {
        MonsterAttack { name, cooldown_ms, damage, mind: 0, armor: 0, curse: Some(curse) }
    }
    pub const fn mind(name: &'static str, cooldown_ms: u32, mind: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage: 0, mind, armor: 0, curse: None }
    }
    pub const fn shielding(name: &'static str, cooldown_ms: u32, armor: i32) -> Self {
        MonsterAttack { name, cooldown_ms, damage: 0, mind: 0, armor, curse: None }
    }
}

/// One entry in a monster's loadout: `(component, slot, x, y, quarter turns)`.
pub type GearPlacement = (&'static str, SlotKind, u8, u8, u8);

#[derive(Copy, Clone, Debug)]
pub struct MonsterSpec {
    pub name: &'static str,
    /// Innate stats before gear: mostly just how much health it has.
    pub health: i32,
    /// Innate strength, which its weapons then scale.
    pub strength: i32,
    pub regen: i32,
    pub mind_resist: i32,
    pub curse_resist: i32,
    /// Innate attacks — a rat's teeth, not equipment. Most of the ladder
    /// leaves this empty and fights with gear instead.
    pub attacks: &'static [MonsterAttack],
    /// Real components in real slots, assembled by the same rules the player
    /// plays by. This is what actually sets a monster's difficulty: to make one
    /// harder, give it better gear.
    pub gear: &'static [GearPlacement],
    /// Gold awarded for beating it.
    pub bounty: i32,
}

impl MonsterSpec {
    /// Lay this monster's gear out in real slots. Returned so the interface can
    /// draw an enemy's board exactly the way it draws yours.
    pub fn loadout(&self) -> (crate::piece::PieceRegistry, crate::loadout::Loadout) {
        let mut reg = crate::piece::PieceRegistry::new();
        let mut loadout = crate::loadout::Loadout::new();
        // Seed names off the monster's own name so its gear is named too, and
        // named the same way every run.
        loadout.name_seed = self.name.bytes().fold(0xA5A5_u64, |a, b| {
            a.rotate_left(7) ^ b as u64
        });

        for &(name, slot, x, y, rot) in self.gear {
            let Some(def) = crate::piece::CATALOG.iter().position(|d| d.name == name) else {
                continue;
            };
            let id = reg.alloc(def);
            reg.set_rotation(id, rot);
            if loadout.can_place(&reg, id, slot, x, y).is_ok() {
                loadout.slot_mut(slot).place(&reg, id, x, y);
            }
        }
        (reg, loadout)
    }

    /// Build this monster's loadout and reduce it to stats plus activation
    /// profiles — the exact pipeline the player's gear goes through.
    pub fn outfit(&self) -> (Stats, Vec<ItemProfile>) {
        let (reg, loadout) = self.loadout();
        let mut stats = loadout.total_stats(&reg);
        // `total_stats` starts from the player's baseline; swap in the
        // monster's own.
        stats.health = stats.health - crate::stats::BASE_HEALTH + self.health;
        // Swap the player's baseline strength for the monster's own.
        stats.strength = stats.strength - crate::stats::BASE_STRENGTH + self.strength;
        stats.regen += self.regen;
        stats.mind_resist += self.mind_resist;
        stats.curse_resist += self.curse_resist;
        (stats, loadout.combat_items(&reg))
    }

    /// Which of its gear failed to assemble, if any. A monster whose loadout
    /// silently falls apart is a monster that does nothing.
    pub fn unassembled(&self) -> Vec<String> {
        let mut reg = crate::piece::PieceRegistry::new();
        let mut loadout = crate::loadout::Loadout::new();
        let mut missing = Vec::new();
        for &(name, slot, x, y, rot) in self.gear {
            match crate::piece::CATALOG.iter().position(|d| d.name == name) {
                None => missing.push(format!("{}: no such component", name)),
                Some(def) => {
                    let id = reg.alloc(def);
                    reg.set_rotation(id, rot);
                    match loadout.can_place(&reg, id, slot, x, y) {
                        Ok(()) => loadout.slot_mut(slot).place(&reg, id, x, y),
                        Err(e) => missing.push(format!("{} at ({}, {}): {}", name, x, y, e)),
                    }
                }
            }
        }
        for kind in SlotKind::ALL {
            for item in loadout.report(&reg, kind).items {
                if !item.assembled {
                    missing.push(format!("{} item: {}", kind.name(), item.status));
                }
            }
        }
        missing
    }
}

/// The original opponent, named because several tests predate the ladder.
pub const RUST_GOLEM: MonsterSpec = MonsterSpec {
    name: "Rust Golem",
    health: 300,
    strength: 13,
    regen: 0,
    mind_resist: 0,
    curse_resist: 0,
    attacks: &[],
    gear: &[
        ("Executioner's Haft", SlotKind::Weapon, 0, 0, 0),
        ("Iron Blade", SlotKind::Weapon, 1, 0, 0),
        ("Padded Base", SlotKind::Chest, 0, 0, 0),
        ("Ironbark Layer", SlotKind::Chest, 0, 3, 0),
    ],
    bounty: 10,
};

/// The monster ladder, easiest first.
///
/// Difficulty is set by what each one is *wearing*, not by hand-tuned numbers:
/// they buy from the same catalogue and assemble by the same rules. Making a
/// monster harder means giving it better gear.
pub const LADDER: &[MonsterSpec] = &[
    MonsterSpec {
        name: "Cave Rat",
        health: 55,
        strength: 2,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        // No gear at all — it just has teeth.
        attacks: &[MonsterAttack::hit("bite", 900, 4)],
        gear: &[],
        bounty: 6,
    },
    MonsterSpec {
        name: "Bog Toad",
        health: 110,
        strength: 5,
        regen: 1,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[],
        // A crude club and nothing else.
        gear: &[
            ("Oak Handle", SlotKind::Weapon, 0, 0, 0),
            ("Iron Blade", SlotKind::Weapon, 1, 0, 0),
        ],
        bounty: 8,
    },
    MonsterSpec {
        name: "Bone Archer",
        health: 120,
        strength: 5,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[],
        // Fast, light hits: a duelling grip made faster still.
        gear: &[
            ("Duelist's Grip", SlotKind::Weapon, 0, 0, 0),
            ("Bonesaw", SlotKind::Weapon, 1, 0, 0),
            ("Leather Material", SlotKind::Gloves, 0, 0, 0),
            ("Featherweight Mold", SlotKind::Gloves, 2, 0, 0),
        ],
        bounty: 9,
    },
    RUST_GOLEM,
    MonsterSpec {
        name: "Frost Wisp",
        health: 150,
        strength: 6,
        regen: 0,
        mind_resist: 0,
        curse_resist: 25,
        attacks: &[],
        // A witch's hat freezes your gear every few seconds.
        gear: &[
            ("Witch's Hat", SlotKind::Helmet, 0, 0, 0),
            ("Iron Plating", SlotKind::Helmet, 0, 3, 0),
            ("Oak Handle", SlotKind::Weapon, 0, 0, 0),
            ("Hexbolt", SlotKind::Weapon, 1, 0, 0),
        ],
        bounty: 12,
    },
    MonsterSpec {
        name: "Plague Hound",
        health: 190,
        strength: 8,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[],
        // Claws that chill, and a mana engine to keep hexing.
        gear: &[
            ("Witch's Claw", SlotKind::Gloves, 0, 0, 0),
            ("Hexer's Mold", SlotKind::Gloves, 2, 0, 0),
            ("Mage's Rod", SlotKind::Weapon, 0, 0, 0),
            ("Iron Blade", SlotKind::Weapon, 1, 0, 0),
        ],
        bounty: 14,
    },
    MonsterSpec {
        name: "Iron Sentinel",
        health: 240,
        strength: 10,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[],
        // Piles on armour faster than light hits can strip it.
        gear: &[
            ("Padded Base", SlotKind::Chest, 0, 0, 0),
            ("Ironbark Layer", SlotKind::Chest, 0, 3, 0),
            ("Thornmail Layer", SlotKind::Chest, 0, 5, 0),
            ("Executioner's Haft", SlotKind::Weapon, 0, 0, 0),
            ("Serrated Edge", SlotKind::Weapon, 1, 0, 0),
        ],
        bounty: 16,
    },
    MonsterSpec {
        name: "Whisperling",
        health: 160,
        strength: 7,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[],
        // Barely scratches you; lowers your ceiling until there is none.
        gear: &[
            ("Oak Handle", SlotKind::Weapon, 0, 0, 0),
            ("Hexbolt", SlotKind::Weapon, 1, 0, 0),
            ("Bileglass Vial", SlotKind::Weapon, 2, 0, 0),
            ("Mage's Circlet", SlotKind::Helmet, 0, 0, 0),
            ("Scrying Lens", SlotKind::Helmet, 0, 2, 0),
        ],
        bounty: 18,
    },
    MonsterSpec {
        name: "Warded Idol",
        health: 280,
        strength: 12,
        regen: 2,
        mind_resist: 0,
        curse_resist: 55,
        attacks: &[],
        // Shrugs off curses and just keeps hitting.
        gear: &[
            ("Hexweave Shroud", SlotKind::Chest, 0, 0, 0),
            ("Runed Lining", SlotKind::Chest, 0, 3, 0),
            ("Executioner's Haft", SlotKind::Weapon, 0, 0, 0),
            ("Iron Blade", SlotKind::Weapon, 1, 0, 0),
            ("Whetstone", SlotKind::Weapon, 2, 0, 0),
        ],
        bounty: 20,
    },
    MonsterSpec {
        name: "Mirror Fiend",
        health: 250,
        strength: 11,
        regen: 0,
        mind_resist: 45,
        curse_resist: 20,
        attacks: &[],
        gear: &[
            ("Mirrored Visor", SlotKind::Helmet, 0, 0, 0),
            ("Steel Frame", SlotKind::Helmet, 0, 2, 0),
            ("Duelist's Grip", SlotKind::Weapon, 0, 0, 0),
            ("Hexbolt", SlotKind::Weapon, 1, 0, 0),
            ("Bileglass Vial", SlotKind::Weapon, 2, 0, 0),
            ("Steel Material", SlotKind::Gloves, 0, 0, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 0, 0),
        ],
        bounty: 24,
    },
    MonsterSpec {
        name: "The Hollow King",
        health: 400,
        strength: 18,
        regen: 3,
        mind_resist: 30,
        curse_resist: 30,
        attacks: &[],
        // A full five-slot loadout with a reactive charm feeding the blade.
        gear: &[
            ("Cursed Handle", SlotKind::Weapon, 0, 0, 0),
            ("Cursed Blade", SlotKind::Weapon, 1, 0, 0),
            ("Quickening Charm", SlotKind::Weapon, 2, 0, 0),
            ("Witch's Hat", SlotKind::Helmet, 0, 0, 0),
            ("Warding Plate", SlotKind::Helmet, 0, 3, 0),
            ("Mana Loom", SlotKind::Chest, 0, 0, 0),
            ("Ironbark Layer", SlotKind::Chest, 0, 3, 0),
            ("Bulwark Material", SlotKind::Gloves, 0, 0, 0),
            ("Channeling Mold", SlotKind::Gloves, 0, 2, 0),
            ("Boiled Leather", SlotKind::Greaves, 0, 0, 0),
            ("Grave-Iron Mold", SlotKind::Greaves, 0, 2, 0),
        ],
        bounty: 40,
    },
];

// ----------------------------------------------------------- combatants

/// An item mid-fight: its profile plus how far its cooldown has filled.
#[derive(Clone, Debug)]
pub struct RunningItem {
    pub name: String,
    pub slot: Option<SlotKind>,
    pub cooldown_ms: u32,
    pub progress_ms: u32,
    pub damage: i32,
    pub mind: i32,
    pub armor: i32,
    pub mana: i32,
    pub triggers: Vec<Trigger>,
    pub adjacent_assembled_same_slot: usize,
    /// Indices, in the owner's item list, of items this one reacts to.
    pub adjacent_items: Vec<usize>,
    pub aligned_items: Vec<usize>,
    /// Monster attacks can carry a curse; player items use triggers instead.
    pub curse: Option<CurseKind>,
}

impl RunningItem {
    fn from_profile(p: &ItemProfile) -> Self {
        RunningItem {
            name: p.name.clone(),
            slot: Some(p.slot),
            cooldown_ms: p.cooldown_ms,
            progress_ms: 0,
            damage: p.stats.damage,
            mind: p.stats.mind,
            armor: p.stats.armor,
            mana: p.stats.mana,
            triggers: p.triggers.clone(),
            adjacent_assembled_same_slot: p.adjacent_assembled_same_slot,
            adjacent_items: p.adjacent_items.clone(),
            aligned_items: p.aligned_items.clone(),
            curse: None,
        }
    }

    fn from_attack(a: &MonsterAttack) -> Self {
        RunningItem {
            name: a.name.to_string(),
            slot: None,
            cooldown_ms: a.cooldown_ms.max(TICK_MS),
            progress_ms: 0,
            damage: a.damage,
            mind: a.mind,
            armor: a.armor,
            mana: 0,
            triggers: Vec::new(),
            adjacent_assembled_same_slot: 0,
            adjacent_items: Vec::new(),
            aligned_items: Vec::new(),
            curse: a.curse,
        }
    }

    /// Fraction of the way to the next activation, for cooldown bars.
    pub fn progress(&self) -> f32 {
        if self.cooldown_ms == 0 {
            return 0.0;
        }
        (self.progress_ms as f32 / self.cooldown_ms as f32).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Combatant {
    pub name: String,
    pub max_health: i32,
    pub health: i32,
    /// Temporary hit points. Always starts a fight at zero — gear has to build
    /// it up — and soaks damage before health does.
    pub armor: i32,
    pub mana: i32,
    pub strength: i32,
    pub power: i32,
    pub regen: i32,
    pub mind_resist: i32,
    pub curse_resist: i32,
    pub curses: Curses,
    /// Stacks of mana empowerment and mana shield. Both scale off *current*
    /// mana, and both are bought with mana — so stacking them hard drains the
    /// very pool they multiply. That tension is the point.
    pub empowerment: u32,
    pub shield: u32,
    pub items: Vec<RunningItem>,
    /// Sub-point accumulators, so 10 damage a second spread over 50ms ticks
    /// loses nothing to rounding.
    dot_milli: i32,
    regen_milli: i32,
    /// Burn damage already taken but not yet written to the log, and how long
    /// since the last entry. Damage-over-time lands every tick; logging it
    /// every tick buries everything else under a wall of "burns for 1".
    burn_acc: i32,
    burn_timer: u32,
}

impl Combatant {
    pub fn player(stats: Stats, profiles: &[ItemProfile]) -> Self {
        Combatant {
            name: "You".to_string(),
            max_health: stats.health,
            health: stats.health,
            armor: 0,
            mana: 0,
            strength: stats.strength,
            power: stats.power,
            regen: stats.regen,
            mind_resist: stats.mind_resist,
            curse_resist: stats.curse_resist,
            curses: Curses::new(),
            empowerment: 0,
            shield: 0,
            items: profiles.iter().map(RunningItem::from_profile).collect(),
            dot_milli: 0,
            regen_milli: 0,
            burn_acc: 0,
            burn_timer: 0,
        }
    }

    pub fn monster(spec: &MonsterSpec) -> Self {
        let (stats, profiles) = spec.outfit();
        // Innate attacks first, then anything its gear assembles.
        let mut items: Vec<RunningItem> =
            spec.attacks.iter().map(RunningItem::from_attack).collect();
        items.extend(profiles.iter().map(RunningItem::from_profile));
        Combatant {
            name: spec.name.to_string(),
            max_health: stats.health,
            health: stats.health,
            armor: 0,
            mana: 0,
            strength: stats.strength,
            power: stats.power,
            regen: stats.regen,
            mind_resist: stats.mind_resist,
            curse_resist: stats.curse_resist,
            curses: Curses::new(),
            empowerment: 0,
            shield: 0,
            items,
            dot_milli: 0,
            regen_milli: 0,
            burn_acc: 0,
            burn_timer: 0,
        }
    }

    pub fn is_down(&self) -> bool {
        self.health <= 0 || self.max_health <= 0
    }

    /// Weapon power after mana empowerment: 0.05x per stack per point of mana.
    pub fn effective_power(&self) -> i32 {
        self.power + self.empowerment as i32 * 5 * self.mana.max(0)
    }

    /// Flat reduction mana shield applies to any incoming damage.
    pub fn damage_reduction(&self) -> i32 {
        self.shield as i32 * self.mana.max(0)
    }

    /// Mana shield first, then armour, then health. Returns (absorbed by
    /// armour, through to health).
    fn take_damage(&mut self, amount: i32) -> (i32, i32) {
        let amount = (amount - self.damage_reduction()).max(0);
        if amount <= 0 {
            return (0, 0);
        }
        let absorbed = amount.min(self.armor.max(0));
        self.armor -= absorbed;
        let through = amount - absorbed;
        self.health -= through;
        (absorbed, through)
    }

    /// Mind damage eats maximum health, so it can never be healed back off.
    fn take_mind(&mut self, raw: i32) -> i32 {
        // "whatever the damage type" — mana shield blunts mind damage too.
        let raw = (raw - self.damage_reduction()).max(0);
        let dealt = mind_damage_after_resist(raw, self.mind_resist);
        if dealt <= 0 {
            return 0;
        }
        self.max_health = (self.max_health - dealt).max(0);
        if self.health > self.max_health {
            self.health = self.max_health;
        }
        dealt
    }
}

// ----------------------------------------------------------------- log

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An item finished its cooldown. Always precedes that item's effects.
    /// `index` is the item's position in its owner's list, so two items with
    /// the same name stay distinguishable.
    Activate { side: Side, item: String, index: usize },
    Hit { by: Side, damage: i32, absorbed: i32, target_health: i32, target_armor: i32 },
    MindHit { by: Side, amount: i32, target_max_health: i32 },
    GainArmor { side: Side, amount: i32, total: i32 },
    GainMana { side: Side, amount: i32, total: i32 },
    /// `paid` says which branch of a mana trigger ran.
    ManaCheck { side: Side, cost: i32, paid: bool, remaining: i32 },
    Cursed { on: Side, kind: CurseKind, duration_ms: u32 },
    /// Damage-over-time landing this tick.
    Burn { side: Side, damage: i32, health: i32 },
    Regen { side: Side, amount: i32, health: i32 },
    /// A reaction pushed an item's cooldown forward.
    Hastened { side: Side, item: String, by_ms: u32 },
    /// A mana buff gained stacks. `total` is the new stack count.
    Empowered { side: Side, total: u32, power_bonus: i32 },
    Shielded { side: Side, total: u32, reduction: i32 },
    Fell { side: Side },
    End { outcome: Outcome },
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub at_ms: u32,
    pub event: Event,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Outcome {
    Victory,
    Defeat,
    Stalemate,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Victory => "VICTORY",
            Outcome::Defeat => "DEFEAT",
            Outcome::Stalemate => "STALEMATE",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CombatLog {
    pub player: Combatant,
    pub enemy: Combatant,
    /// The monster fought, so the interface can lay its gear out beside yours
    /// without having to guess which rung the run has moved on to.
    pub spec: MonsterSpec,
    pub entries: Vec<LogEntry>,
    pub outcome: Outcome,
    pub duration_ms: u32,
}

impl CombatLog {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn who(&self, s: Side) -> &str {
        match s {
            Side::Player => &self.player.name,
            Side::Enemy => &self.enemy.name,
        }
    }

    /// One line of plain text, for the CLI and the on-screen log.
    pub fn describe(&self, e: &LogEntry) -> String {
        let t = format!("{:>5.1}s", e.at_ms as f32 / 1000.0);
        match &e.event {
            Event::Activate { side, item, .. } => {
                format!("{} {} activates {}", t, self.who(*side), item)
            }
            Event::Hit { by, damage, absorbed, target_health, target_armor } => {
                let soak = if *absorbed > 0 {
                    format!(" ({} soaked, {} armor left)", absorbed, target_armor)
                } else {
                    String::new()
                };
                format!(
                    "{} {} hits {} for {}{} -> {} hp",
                    t,
                    self.who(*by),
                    self.who(by.other()),
                    damage,
                    soak,
                    (*target_health).max(0)
                )
            }
            Event::MindHit { by, amount, target_max_health } => format!(
                "{} {} deals {} MIND damage -> max hp now {}",
                t,
                self.who(*by),
                amount,
                target_max_health
            ),
            Event::GainArmor { side, amount, total } => {
                format!("{} {} gains {} armor ({})", t, self.who(*side), amount, total)
            }
            Event::GainMana { side, amount, total } => {
                format!("{} {} gains {} mana ({})", t, self.who(*side), amount, total)
            }
            Event::ManaCheck { side, cost, paid, remaining } => {
                if *paid {
                    format!("{} {} spends {} mana ({} left)", t, self.who(*side), cost, remaining)
                } else {
                    format!(
                        "{} {} cannot pay {} mana (has {})",
                        t,
                        self.who(*side),
                        cost,
                        remaining
                    )
                }
            }
            Event::Cursed { on, kind, duration_ms } => format!(
                "{} curse of {} on {} for {:.1}s",
                t,
                kind.name(),
                self.who(*on),
                *duration_ms as f32 / 1000.0
            ),
            Event::Burn { side, damage, health } => format!(
                "{} {} burns for {} -> {} hp",
                t,
                self.who(*side),
                damage,
                (*health).max(0)
            ),
            Event::Regen { side, amount, health } => {
                format!("{} {} regenerates {} -> {} hp", t, self.who(*side), amount, health)
            }
            Event::Hastened { side, item, by_ms } => format!(
                "{} {}'s {} hastened by {:.1}s",
                t,
                self.who(*side),
                item,
                *by_ms as f32 / 1000.0
            ),
            Event::Empowered { side, total, power_bonus } => format!(
                "{} {} empowered x{} (+{}.{:02}x power)",
                t,
                self.who(*side),
                total,
                power_bonus / 100,
                power_bonus % 100
            ),
            Event::Shielded { side, total, reduction } => format!(
                "{} {} mana shield x{} (-{} per hit)",
                t,
                self.who(*side),
                total,
                reduction
            ),
            Event::Fell { side } => format!("{} {} falls!", t, self.who(*side)),
            Event::End { outcome } => format!("-- {} --", outcome.label()),
        }
    }
}

// ------------------------------------------------------------ simulate

/// Run the whole fight to completion.
///
/// Each [`TICK_MS`] slice, in strict order:
///   1. curses burn, then regeneration heals, on both sides
///   2. curse timers advance and expired curses drop
///   3. every item advances its cooldown — slowed if its owner is frosted —
///      and activates if full. The player's items resolve before the enemy's,
///      and within a side they resolve in loadout order.
///   4. deaths are checked
///
/// Nothing here consults a random number generator.
pub fn simulate(player_stats: Stats, profiles: &[ItemProfile], spec: &MonsterSpec) -> CombatLog {
    let start_player = Combatant::player(player_stats, profiles);
    let start_enemy = Combatant::monster(spec);
    let mut p = start_player.clone();
    let mut e = start_enemy.clone();
    let mut log: Vec<LogEntry> = Vec::new();
    let mut outcome = Outcome::Stalemate;
    let mut t: u32 = 0;

    'fight: while t < MAX_DURATION_MS {
        t += TICK_MS;

        // 1. Damage over time, then healing.
        for side in [Side::Player, Side::Enemy] {
            let c = pick(&mut p, &mut e, side);
            c.dot_milli += c.curses.dot_millidamage_per_tick();
            let whole = c.dot_milli / 1000;
            if whole > 0 {
                c.dot_milli %= 1000;
                c.health -= whole;
                c.burn_acc += whole;
            }
            // Report burn once a second, or immediately if it just killed
            // them, rather than a line per tick.
            c.burn_timer += TICK_MS;
            if c.burn_acc > 0 && (c.burn_timer >= BURN_REPORT_MS || c.health <= 0) {
                let (dmg, hp) = (c.burn_acc, c.health);
                c.burn_acc = 0;
                c.burn_timer = 0;
                log.push(LogEntry { at_ms: t, event: Event::Burn { side, damage: dmg, health: hp } });
            }
            if c.regen > 0 && c.health < c.max_health {
                c.regen_milli += c.regen * TICK_MS as i32;
                let heal = (c.regen_milli / 1000).min(c.max_health - c.health);
                if heal > 0 {
                    c.regen_milli %= 1000;
                    c.health += heal;
                    let hp = c.health;
                    log.push(LogEntry {
                        at_ms: t,
                        event: Event::Regen { side, amount: heal, health: hp },
                    });
                }
            }
        }
        if check_down(&p, &e, t, &mut log, &mut outcome) {
            break 'fight;
        }

        // 2. Curse timers.
        p.curses.tick();
        e.curses.tick();

        // 3. Cooldowns and activations.
        for side in [Side::Player, Side::Enemy] {
            let count = pick(&mut p, &mut e, side).items.len();
            for idx in 0..count {
                let ready = {
                    let c = pick(&mut p, &mut e, side);
                    // Frost stretches the cooldown by slowing how fast the bar
                    // fills, rather than by rewriting the cooldown itself.
                    let slow = c.curses.slow_pct();
                    let step = (TICK_MS as i32 * (100 - slow) / 100).max(1) as u32;
                    let item = &mut c.items[idx];
                    item.progress_ms += step;
                    if item.progress_ms >= item.cooldown_ms {
                        item.progress_ms -= item.cooldown_ms;
                        true
                    } else {
                        false
                    }
                };
                if ready {
                    activate(&mut p, &mut e, side, idx, t, &mut log);
                    if check_down(&p, &e, t, &mut log, &mut outcome) {
                        break 'fight;
                    }
                }
            }
        }
    }

    log.push(LogEntry { at_ms: t, event: Event::End { outcome } });
    CombatLog {
        player: start_player,
        enemy: start_enemy,
        spec: *spec,
        entries: log,
        outcome,
        duration_ms: t,
    }
}

fn pick<'a>(p: &'a mut Combatant, e: &'a mut Combatant, side: Side) -> &'a mut Combatant {
    match side {
        Side::Player => p,
        Side::Enemy => e,
    }
}

fn check_down(
    p: &Combatant,
    e: &Combatant,
    t: u32,
    log: &mut Vec<LogEntry>,
    outcome: &mut Outcome,
) -> bool {
    if e.is_down() {
        log.push(LogEntry { at_ms: t, event: Event::Fell { side: Side::Enemy } });
        *outcome = Outcome::Victory;
        return true;
    }
    if p.is_down() {
        log.push(LogEntry { at_ms: t, event: Event::Fell { side: Side::Player } });
        *outcome = Outcome::Defeat;
        return true;
    }
    false
}

/// Resolve one item firing: its flat effects, then its triggers in order.
fn activate(
    p: &mut Combatant,
    e: &mut Combatant,
    side: Side,
    idx: usize,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let item = pick(p, e, side).items[idx].clone();
    log.push(LogEntry {
        at_ms: t,
        event: Event::Activate { side, item: item.name.clone(), index: idx },
    });

    // Weapons swing; everything else just does its job. A monster's attacks
    // have no slot and always count as weapons.
    let is_weapon = item.slot.map(|s| s == SlotKind::Weapon).unwrap_or(true);
    if is_weapon {
        let (strength, power) = {
            let me = pick(p, e, side);
            (me.strength, me.effective_power())
        };
        let raw = (item.damage + strength) as i64 * power as i64 / 100;
        let raw = raw.max(0) as i32;
        if raw > 0 {
            let target = pick(p, e, side.other());
            let (absorbed, _) = target.take_damage(raw);
            let (hp, ar) = (target.health, target.armor);
            log.push(LogEntry {
                at_ms: t,
                event: Event::Hit {
                    by: side,
                    damage: raw,
                    absorbed,
                    target_health: hp,
                    target_armor: ar,
                },
            });
        }
    }

    if let Some(kind) = item.curse {
        apply(p, e, side, Action::Curse { kind, target: Target::Enemy }, t, log, Some(idx));
    }

    if item.mind > 0 {
        let target = pick(p, e, side.other());
        let dealt = target.take_mind(item.mind);
        let mh = target.max_health;
        if dealt > 0 {
            log.push(LogEntry {
                at_ms: t,
                event: Event::MindHit { by: side, amount: dealt, target_max_health: mh },
            });
        }
    }

    if item.armor > 0 {
        let me = pick(p, e, side);
        me.armor += item.armor;
        let total = me.armor;
        log.push(LogEntry {
            at_ms: t,
            event: Event::GainArmor { side, amount: item.armor, total },
        });
    }

    if item.mana > 0 {
        let me = pick(p, e, side);
        me.mana += item.mana;
        let total = me.mana;
        log.push(LogEntry { at_ms: t, event: Event::GainMana { side, amount: item.mana, total } });
    }

    for trigger in &item.triggers {
        match *trigger {
            Trigger::OnActivate(action) => apply(p, e, side, action, t, log, Some(idx)),
            Trigger::SpendMana { cost, on_success, on_failure } => {
                let paid = {
                    let me = pick(p, e, side);
                    if me.mana >= cost {
                        me.mana -= cost;
                        true
                    } else {
                        false
                    }
                };
                let remaining = pick(p, e, side).mana;
                log.push(LogEntry {
                    at_ms: t,
                    event: Event::ManaCheck { side, cost, paid, remaining },
                });
                apply(p, e, side, if paid { on_success } else { on_failure }, t, log, Some(idx));
            }
            Trigger::PerAdjacentItem { action, same_slot_only: _ } => {
                for _ in 0..item.adjacent_assembled_same_slot {
                    apply(p, e, side, action, t, log, Some(idx));
                }
            }
            // These wait for someone else to act.
            Trigger::OnAdjacentActivate(_) | Trigger::OnAlignedActivate(_) => {}
        }
    }

    // Finally, let the neighbours react. A reaction never emits an activation
    // of its own, so two items that react to each other cannot loop.
    notify_reactors(p, e, side, idx, t, log);
}

/// Run every reaction owed to `actor_idx` firing.
fn notify_reactors(
    p: &mut Combatant,
    e: &mut Combatant,
    side: Side,
    actor_idx: usize,
    t: u32,
    log: &mut Vec<LogEntry>,
) {
    let count = pick(p, e, side).items.len();
    for j in 0..count {
        if j == actor_idx {
            continue;
        }
        let (touches, lines_up, triggers) = {
            let c = pick(p, e, side);
            let it = &c.items[j];
            (
                it.adjacent_items.contains(&actor_idx),
                it.aligned_items.contains(&actor_idx),
                it.triggers.clone(),
            )
        };
        for tr in &triggers {
            match *tr {
                Trigger::OnAdjacentActivate(a) if touches => {
                    apply(p, e, side, a, t, log, Some(j))
                }
                Trigger::OnAlignedActivate(a) if lines_up => {
                    apply(p, e, side, a, t, log, Some(j))
                }
                _ => {}
            }
        }
    }
}

/// `owner` is the item the action belongs to, needed by effects that act on
/// the item itself rather than on a combatant.
fn apply(
    p: &mut Combatant,
    e: &mut Combatant,
    side: Side,
    action: Action,
    t: u32,
    log: &mut Vec<LogEntry>,
    owner: Option<usize>,
) {
    // `Target::Yourself` means the side that owns the item, not the item's
    // victim — several strong items pay for themselves this way.
    let resolve = |target: Target| match target {
        Target::Enemy => side.other(),
        Target::Yourself => side,
    };

    match action {
        Action::Curse { kind, target } => {
            let on = resolve(target);
            let c = pick(p, e, on);
            let duration = c.curses.apply(kind, c.curse_resist);
            if duration > 0 {
                log.push(LogEntry { at_ms: t, event: Event::Cursed { on, kind, duration_ms: duration } });
            }
        }
        Action::Damage { amount, target } => {
            let on = resolve(target);
            let c = pick(p, e, on);
            let (absorbed, _) = c.take_damage(amount);
            let (hp, ar) = (c.health, c.armor);
            log.push(LogEntry {
                at_ms: t,
                event: Event::Hit {
                    by: on.other(),
                    damage: amount,
                    absorbed,
                    target_health: hp,
                    target_armor: ar,
                },
            });
        }
        Action::MindDamage { amount, target } => {
            let on = resolve(target);
            let c = pick(p, e, on);
            let dealt = c.take_mind(amount);
            let mh = c.max_health;
            if dealt > 0 {
                log.push(LogEntry {
                    at_ms: t,
                    event: Event::MindHit { by: on.other(), amount: dealt, target_max_health: mh },
                });
            }
        }
        Action::GainMana(n) => {
            let c = pick(p, e, side);
            c.mana += n;
            let total = c.mana;
            log.push(LogEntry { at_ms: t, event: Event::GainMana { side, amount: n, total } });
        }
        Action::GainArmor(n) => {
            let c = pick(p, e, side);
            c.armor += n;
            let total = c.armor;
            log.push(LogEntry { at_ms: t, event: Event::GainArmor { side, amount: n, total } });
        }
        Action::GainEmpowerment(n) => {
            let c = pick(p, e, side);
            c.empowerment += n;
            let (total, bonus) = (c.empowerment, c.effective_power() - c.power);
            log.push(LogEntry {
                at_ms: t,
                event: Event::Empowered { side, total, power_bonus: bonus },
            });
        }
        Action::GainShield(n) => {
            let c = pick(p, e, side);
            c.shield += n;
            let (total, reduction) = (c.shield, c.damage_reduction());
            log.push(LogEntry { at_ms: t, event: Event::Shielded { side, total, reduction } });
        }
        Action::ReduceCooldown(ms) => {
            let Some(idx) = owner else { return };
            let c = pick(p, e, side);
            let Some(it) = c.items.get_mut(idx) else { return };
            // Push the bar forward rather than shortening the cooldown, so the
            // effect is "fires sooner once" and cannot stack into a free item.
            it.progress_ms = (it.progress_ms + ms).min(it.cooldown_ms.saturating_sub(1));
            let name = it.name.clone();
            log.push(LogEntry { at_ms: t, event: Event::Hastened { side, item: name, by_ms: ms } });
        }
    }
}
