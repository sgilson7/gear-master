use crate::stats::Stats;

/// Enemy definition for the prototype. Deals a flat 10 damage a turn, as
/// specified: strength 10 at a bare-handed 1.00x multiplier.
pub const ENEMY_NAME: &str = "Rust Golem";
pub const ENEMY_HEALTH: i32 = 400;
pub const ENEMY_STRENGTH: i32 = 10;
pub const ENEMY_POWER: i32 = 100;
pub const ENEMY_REGEN: i32 = 0;

/// A fight that reaches this many turns is called a stalemate, so a build that
/// can't out-damage the enemy's regen can't hang forever.
pub const MAX_TURNS: u32 = 60;

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
}

#[derive(Clone, Debug)]
pub struct Fighter {
    pub name: String,
    pub max_health: i32,
    pub health: i32,
    pub strength: i32,
    pub regen: i32,
    pub power: i32,
}

impl Fighter {
    pub fn from_stats(name: &str, s: Stats) -> Self {
        Fighter {
            name: name.to_string(),
            max_health: s.health,
            health: s.health,
            strength: s.strength,
            regen: s.regen,
            power: s.power,
        }
    }

    pub fn enemy() -> Self {
        Fighter::from_stats(
            ENEMY_NAME,
            Stats::new(ENEMY_HEALTH, ENEMY_STRENGTH, ENEMY_REGEN, ENEMY_POWER),
        )
    }

    /// Damage dealt per attack: strength times the weapon multiplier.
    pub fn damage(&self) -> i32 {
        Stats::new(0, self.strength, 0, self.power).damage_per_attack()
    }

    pub fn is_down(&self) -> bool {
        self.health <= 0
    }
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

/// One visible beat of the fight. The GUI replays these against wall-clock
/// time; each carries the resulting health so playback never has to re-derive
/// the simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Attack { by: Side, damage: i32, target_health: i32 },
    Regen { side: Side, amount: i32, health: i32 },
    Fell { side: Side },
    End { outcome: Outcome },
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub turn: u32,
    pub event: Event,
}

#[derive(Clone, Debug)]
pub struct CombatLog {
    /// Both fighters as they stood at the opening bell.
    pub player: Fighter,
    pub enemy: Fighter,
    pub entries: Vec<LogEntry>,
    pub outcome: Outcome,
    pub turns: u32,
}

impl CombatLog {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Plain-text rendering of one entry, for the CLI and the on-screen log.
    pub fn describe(&self, entry: &LogEntry) -> String {
        match &entry.event {
            Event::Attack { by, damage, target_health } => {
                let (attacker, target) = match by {
                    Side::Player => (&self.player.name, &self.enemy.name),
                    Side::Enemy => (&self.enemy.name, &self.player.name),
                };
                format!(
                    "T{}: {} hits {} for {} ({} hp left)",
                    entry.turn,
                    attacker,
                    target,
                    damage,
                    (*target_health).max(0)
                )
            }
            Event::Regen { side, amount, health } => {
                let who = match side {
                    Side::Player => &self.player.name,
                    Side::Enemy => &self.enemy.name,
                };
                format!("T{}: {} regenerates {} ({} hp)", entry.turn, who, amount, health)
            }
            Event::Fell { side } => {
                let who = match side {
                    Side::Player => &self.player.name,
                    Side::Enemy => &self.enemy.name,
                };
                format!("T{}: {} falls!", entry.turn, who)
            }
            Event::End { outcome } => format!("-- {} --", outcome.label()),
        }
    }
}

/// Run the whole fight to completion, deterministically. No RNG: the same
/// loadout always produces the same log, which is what makes the outcome
/// assertable in tests.
///
/// Each turn, in strict order:
///   1. the player attacks
///   2. if the enemy is down, the fight ends in victory
///   3. the enemy attacks
///   4. if the player is down, the fight ends in defeat
///   5. both sides regenerate, capped at their maximum health
pub fn simulate(player_stats: Stats, enemy: Fighter) -> CombatLog {
    let player = Fighter::from_stats("You", player_stats);
    let mut p = player.clone();
    let mut e = enemy.clone();
    let mut entries = Vec::new();
    let mut outcome = Outcome::Stalemate;
    let mut turn = 0;

    while turn < MAX_TURNS {
        turn += 1;

        // 1. player attacks
        let dmg = p.damage();
        e.health -= dmg;
        entries.push(LogEntry {
            turn,
            event: Event::Attack { by: Side::Player, damage: dmg, target_health: e.health },
        });
        // 2. enemy down?
        if e.is_down() {
            entries.push(LogEntry { turn, event: Event::Fell { side: Side::Enemy } });
            outcome = Outcome::Victory;
            break;
        }

        // 3. enemy attacks
        let edmg = e.damage();
        p.health -= edmg;
        entries.push(LogEntry {
            turn,
            event: Event::Attack { by: Side::Enemy, damage: edmg, target_health: p.health },
        });
        // 4. player down?
        if p.is_down() {
            entries.push(LogEntry { turn, event: Event::Fell { side: Side::Player } });
            outcome = Outcome::Defeat;
            break;
        }

        // 5. regeneration, capped
        for (side, f) in [(Side::Player, &mut p), (Side::Enemy, &mut e)] {
            if f.regen > 0 && f.health < f.max_health {
                let healed = f.regen.min(f.max_health - f.health);
                f.health += healed;
                entries.push(LogEntry {
                    turn,
                    event: Event::Regen { side, amount: healed, health: f.health },
                });
            }
        }
    }

    entries.push(LogEntry { turn, event: Event::End { outcome } });

    CombatLog { player, enemy, entries, outcome, turns: turn }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unarmed_character_loses_to_the_golem() {
        let log = simulate(Stats::base_character(), Fighter::enemy());
        assert_eq!(log.outcome, Outcome::Defeat);
        // 100 hp against 10 damage a turn.
        assert_eq!(log.turns, 10);
    }

    #[test]
    fn enough_damage_wins_and_the_log_ends_with_the_outcome() {
        // 100 strength at 1.00x = 100 damage; the golem's 400 hp lasts 4 turns.
        let log = simulate(Stats::new(200, 100, 0, 100), Fighter::enemy());
        assert_eq!(log.outcome, Outcome::Victory);
        assert_eq!(log.turns, 4);
        assert!(matches!(
            log.entries.last().map(|e| &e.event),
            Some(Event::End { outcome: Outcome::Victory })
        ));
    }

    #[test]
    fn regeneration_is_capped_at_maximum_health() {
        let log = simulate(Stats::new(100, 20, 50, 100), Fighter::enemy());
        for entry in &log.entries {
            if let Event::Regen { side: Side::Player, health, .. } = entry.event {
                assert!(health <= 100, "healed past max: {}", health);
            }
        }
    }

    #[test]
    fn a_harmless_build_stalemates_rather_than_looping_forever() {
        // No damage at all, but regen outpaces the golem's 10 a turn.
        let log = simulate(Stats::new(500, 0, 20, 100), Fighter::enemy());
        assert_eq!(log.outcome, Outcome::Stalemate);
        assert_eq!(log.turns, MAX_TURNS);
    }

    #[test]
    fn the_golem_deals_ten_damage_a_turn() {
        assert_eq!(Fighter::enemy().damage(), 10);
    }
}
