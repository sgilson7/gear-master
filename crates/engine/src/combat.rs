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

#[derive(Copy, Clone, Debug)]
pub struct MonsterSpec {
    pub name: &'static str,
    pub health: i32,
    pub regen: i32,
    pub mind_resist: i32,
    pub curse_resist: i32,
    pub attacks: &'static [MonsterAttack],
    /// Gold awarded for beating it.
    pub bounty: i32,
}

/// The original opponent, named because several tests predate the ladder.
pub const RUST_GOLEM: MonsterSpec = MonsterSpec {
    name: "Rust Golem",
    health: 400,
    regen: 0,
    mind_resist: 0,
    curse_resist: 0,
    attacks: &[MonsterAttack::hit("slam", 1000, 10)],
    bounty: 10,
};

/// The monster ladder, easiest first. Beating one pays its bounty and moves
/// you along; each teaches a different defensive stat.
pub const LADDER: &[MonsterSpec] = &[
    MonsterSpec {
        name: "Cave Rat",
        health: 60,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[MonsterAttack::hit("bite", 800, 4)],
        bounty: 6,
    },
    MonsterSpec {
        name: "Bog Toad",
        health: 120,
        regen: 1,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[MonsterAttack::hit("tongue", 1200, 8)],
        bounty: 8,
    },
    MonsterSpec {
        name: "Bone Archer",
        health: 140,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        // Fast and weak: armour that regenerates beats it, raw health doesn't.
        attacks: &[MonsterAttack::hit("arrow", 600, 5)],
        bounty: 9,
    },
    RUST_GOLEM,
    MonsterSpec {
        name: "Frost Wisp",
        health: 170,
        regen: 0,
        mind_resist: 0,
        curse_resist: 25,
        // Slows your gear, so fast cheap items suffer least.
        attacks: &[MonsterAttack::cursing("chill", 1500, 4, CurseKind::Frost)],
        bounty: 12,
    },
    MonsterSpec {
        name: "Plague Hound",
        health: 210,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        attacks: &[MonsterAttack::cursing("foul bite", 2500, 6, CurseKind::Searing)],
        bounty: 14,
    },
    MonsterSpec {
        name: "Iron Sentinel",
        health: 260,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        // Shields itself, so burst has to out-pace the plating going back up.
        attacks: &[
            MonsterAttack::hit("hammer", 1500, 12),
            MonsterAttack::shielding("re-plate", 2000, 14),
        ],
        bounty: 16,
    },
    MonsterSpec {
        name: "Whisperling",
        health: 180,
        regen: 0,
        mind_resist: 0,
        curse_resist: 0,
        // Almost no direct damage: it lowers your ceiling until there is none.
        attacks: &[MonsterAttack::mind("whisper", 1200, 3), MonsterAttack::hit("claw", 2000, 3)],
        bounty: 18,
    },
    MonsterSpec {
        name: "Warded Idol",
        health: 320,
        regen: 2,
        mind_resist: 0,
        curse_resist: 75,
        // Curse builds fall flat here; bring something that just hits.
        attacks: &[MonsterAttack::hit("smite", 1300, 11)],
        bounty: 20,
    },
    MonsterSpec {
        name: "Mirror Fiend",
        health: 280,
        regen: 0,
        mind_resist: 60,
        curse_resist: 30,
        attacks: &[MonsterAttack::mind("gaze", 1500, 5), MonsterAttack::hit("strike", 1000, 9)],
        bounty: 24,
    },
    MonsterSpec {
        name: "The Hollow King",
        health: 520,
        regen: 3,
        mind_resist: 40,
        curse_resist: 40,
        attacks: &[
            MonsterAttack::hit("greatsword", 1100, 14),
            MonsterAttack::cursing("wail", 3000, 5, CurseKind::Searing),
            MonsterAttack::mind("dread", 2500, 4),
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
    pub items: Vec<RunningItem>,
    /// Sub-point accumulators, so 10 damage a second spread over 50ms ticks
    /// loses nothing to rounding.
    dot_milli: i32,
    regen_milli: i32,
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
            items: profiles.iter().map(RunningItem::from_profile).collect(),
            dot_milli: 0,
            regen_milli: 0,
        }
    }

    pub fn monster(spec: &MonsterSpec) -> Self {
        Combatant {
            name: spec.name.to_string(),
            max_health: spec.health,
            health: spec.health,
            armor: 0,
            mana: 0,
            strength: 0,
            power: 100,
            regen: spec.regen,
            mind_resist: spec.mind_resist,
            curse_resist: spec.curse_resist,
            curses: Curses::new(),
            items: spec.attacks.iter().map(RunningItem::from_attack).collect(),
            dot_milli: 0,
            regen_milli: 0,
        }
    }

    pub fn is_down(&self) -> bool {
        self.health <= 0 || self.max_health <= 0
    }

    /// Armour first, then health. Returns (absorbed, through to health).
    fn take_damage(&mut self, amount: i32) -> (i32, i32) {
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
    Activate { side: Side, item: String },
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
            Event::Activate { side, item } => format!("{} {} activates {}", t, self.who(*side), item),
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
            Event::Burn { side, damage, health } => {
                format!("{} {} burns for {} -> {} hp", t, self.who(*side), damage, (*health).max(0))
            }
            Event::Regen { side, amount, health } => {
                format!("{} {} regenerates {} -> {} hp", t, self.who(*side), amount, health)
            }
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
                let hp = c.health;
                log.push(LogEntry { at_ms: t, event: Event::Burn { side, damage: whole, health: hp } });
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
    CombatLog { player: start_player, enemy: start_enemy, entries: log, outcome, duration_ms: t }
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
    log.push(LogEntry { at_ms: t, event: Event::Activate { side, item: item.name.clone() } });

    // Weapons swing; everything else just does its job. A monster's attacks
    // have no slot and always count as weapons.
    let is_weapon = item.slot.map(|s| s == SlotKind::Weapon).unwrap_or(true);
    if is_weapon {
        let (strength, power) = {
            let me = pick(p, e, side);
            (me.strength, me.power)
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
        apply(p, e, side, Action::Curse { kind, target: Target::Enemy }, t, log);
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
            Trigger::OnActivate(action) => apply(p, e, side, action, t, log),
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
                apply(p, e, side, if paid { on_success } else { on_failure }, t, log);
            }
            Trigger::PerAdjacentItem { action, same_slot_only: _ } => {
                for _ in 0..item.adjacent_assembled_same_slot {
                    apply(p, e, side, action, t, log);
                }
            }
        }
    }
}

fn apply(
    p: &mut Combatant,
    e: &mut Combatant,
    side: Side,
    action: Action,
    t: u32,
    log: &mut Vec<LogEntry>,
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
    }
}
