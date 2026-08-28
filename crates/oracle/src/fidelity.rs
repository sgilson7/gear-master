//! Does the fight read as the theme?
//!
//! `MonsterTheme::allows` (`bestiary.rs:149`) filters the *pool* a creature
//! may draw from: does this piece speak the theme's language. Nothing in this
//! repo has ever asked the question the theme is actually a claim about -
//! **does the fight it gives read as what it says it is**. A Burner packed out
//! of burning words that kills with one big blow passes every test there is.
//!
//! So: ten signatures, one per theme, each computed from a `CombatLog`, each a
//! number between zero and one.
//!
//! ## Every signature is a ratio over the fight
//!
//! This is the rule the whole module hangs on and it is `CLAUDE.md` §6 trap 29
//! stated for a metric an agent will optimise: **ask what the cheapest way to
//! satisfy a check is before shipping it.** "Carries a Searing curse" is
//! satisfied by seating one piece and says nothing. "More than half the damage
//! arrived as Searing ticks" cannot be satisfied except by building the
//! creature the theme describes. Where a signature below looks like a property
//! of the gear, it is a bug in this file.

use gearmaster_engine::bestiary::{MonsterTheme, SWARM_BLOW};
use gearmaster_engine::combat::{CombatLog, Event, Side};

/// What one fight said about one theme.
///
/// `parts` is carried rather than recomputed, for `stats.rs`'s reason: two
/// walks over one classification are two things that will disagree about it
/// one day. The signature is written down once, in `score`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fidelity {
    /// Zero to one. One is "this fight is exactly what the theme describes".
    pub score: f64,
    /// Each half of the claim, named, and how well the fight bore it out.
    pub parts: Vec<(&'static str, f64)>,
    /// The figures behind them, so a failure says which half was missing.
    pub reading: Reading,
}

/// Everything the meter measures, read off one log, from the creature's side.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Reading {
    /// Blows the creature landed, and their total.
    pub blows: u32,
    pub blow_damage: i64,
    /// The largest single blow, and the mean.
    pub biggest_blow: i32,
    pub mean_blow: i64,
    /// Damage the player took from burning rather than from blows.
    pub burn_damage: i64,
    /// Mind damage: maximum health taken away, which never comes back.
    pub mind_damage: i64,
    /// Curses the creature applied, and how many kinds.
    pub curses: u32,
    pub curse_kinds: u32,
    /// Pools taken off the player.
    pub drained: i64,
    /// Times the creature's items came round.
    pub activations: u32,
    /// Damage the player's armour turned back on them.
    pub reflected: i64,
    /// Armour the creature gained during the fight.
    pub armour_gained: i64,
    /// Items the creature had stopped, by a stun of its own.
    pub stuns: u32,
    /// How long the fight lasted.
    pub ms: u32,
    /// Everything that reached the player, by any route.
    pub total_dealt: i64,
}

impl Reading {
    /// Read one fight from the creature's side.
    pub fn of(log: &CombatLog) -> Reading {
        let mut r = Reading { ms: log.duration_ms, ..Reading::default() };
        let mut kinds: Vec<gearmaster_engine::curse::CurseKind> = Vec::new();
        for e in &log.entries {
            match e.event {
                Event::Hit { by: Side::Enemy, damage, .. } => {
                    r.blows += 1;
                    r.blow_damage += damage as i64;
                    r.biggest_blow = r.biggest_blow.max(damage);
                }
                // A burn on the player's side is the creature's damage over
                // time - the only lane whose total does not arrive as a blow.
                Event::Burn { side: Side::Player, damage, .. } => r.burn_damage += damage as i64,
                Event::MindHit { by: Side::Enemy, amount, .. } => r.mind_damage += amount as i64,
                Event::Cursed { on: Side::Player, kind, .. } => {
                    r.curses += 1;
                    if !kinds.contains(&kind) {
                        kinds.push(kind);
                    }
                }
                Event::Drained { on: Side::Player, amount, .. } => r.drained += amount as i64,
                Event::Activate { side: Side::Enemy, .. } => r.activations += 1,
                // Reflection is the player's armour answering, so a creature
                // that provokes a lot of it is one being hit a lot.
                Event::Reflected { side: Side::Player, damage } => r.reflected += damage as i64,
                Event::GainArmor { side: Side::Enemy, amount, .. } => {
                    r.armour_gained += amount as i64
                }
                Event::Stunned { on: Side::Player, .. } => r.stuns += 1,
                _ => {}
            }
        }
        r.mean_blow = if r.blows > 0 { r.blow_damage / r.blows as i64 } else { 0 };
        r.total_dealt = r.blow_damage + r.burn_damage + r.mind_damage;
        r
    }

    fn share(&self, part: i64) -> f64 {
        if self.total_dealt <= 0 {
            return 0.0;
        }
        part as f64 / self.total_dealt as f64
    }

    /// A **negative** claim about a share: "not mostly blows".
    ///
    /// Zero when there is no damage at all, because absence is not evidence.
    /// `fall(share, ..)` on an empty fight reads 1.0 - so a creature that did
    /// nothing whatsoever satisfies "rather than lands" perfectly, and the
    /// cheapest way to read as a Burner becomes doing nothing. That is
    /// `CLAUDE.md` §6 trap 29 exactly, and `tests/fidelity.rs` found it here
    /// before this meter had scored anything.
    fn not_mostly(&self, part: i64, lo: f64, hi: f64) -> f64 {
        if self.total_dealt <= 0 {
            return 0.0;
        }
        fall(self.share(part), lo, hi)
    }

    /// Did this creature do anything at all?
    fn fought(&self) -> bool {
        self.activations > 0 || self.total_dealt > 0 || self.curses > 0 || self.drained > 0
    }

    /// Activations a second. A swarm's whole identity is this number.
    fn cadence(&self) -> f64 {
        if self.ms == 0 {
            return 0.0;
        }
        self.activations as f64 * 1000.0 / self.ms as f64
    }
}

/// Rise from zero at `lo` to one at `hi`, and stay there.
fn ramp(v: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Fall from one at `lo` to zero at `hi`.
fn fall(v: f64, lo: f64, hi: f64) -> f64 {
    1.0 - ramp(v, lo, hi)
}

/// Score one fight against one theme.
///
/// The parts are averaged rather than multiplied: a creature that is three
/// quarters of what its theme describes should read as three quarters, not as
/// nothing. A multiplied score would make every signature a veto and the
/// number would be zero nearly everywhere, which is a meter that cannot say
/// anything about *how far off* a board is - and how far off is exactly what a
/// search needs to climb.
pub fn score(theme: MonsterTheme, r: &Reading) -> Fidelity {
    let parts: Vec<(&'static str, f64)> = match theme {
        // Fast and fragile; punishes a slow board. Few blows, large ones, and
        // a fight that ends.
        MonsterTheme::Striker => vec![
            ("blows carry it", r.share(r.blow_damage)),
            ("the blows are big", ramp(r.mean_blow as f64, 8.0, 40.0)),
            ("and it is over quickly", fall(r.ms as f64, 6_000.0, 20_000.0)),
        ],
        // Slow, heavy, hits back harder when hit.
        MonsterTheme::Wall => vec![
            ("it lasts", ramp(r.ms as f64, 8_000.0, 25_000.0)),
            ("it answers being hit", ramp(r.reflected as f64, 0.0, 60.0)),
            ("and it keeps putting armour on", ramp(r.armour_gained as f64, 0.0, 120.0)),
        ],
        // Kills on the clock, not the swing. The one theme whose damage does
        // not arrive as a blow.
        MonsterTheme::Burner => vec![
            ("most of it burns", r.share(r.burn_damage)),
            ("rather than lands", r.not_mostly(r.blow_damage, 0.35, 0.8)),
        ],
        // Denies tempo; deals little itself.
        MonsterTheme::Slower => vec![
            ("it stops things", ramp(r.stuns as f64 + r.curses as f64, 1.0, 8.0)),
            ("and does little else", fall(r.total_dealt as f64 / r.ms.max(1) as f64 * 1000.0, 8.0, 30.0)),
        ],
        // Starves a build that banks pools.
        MonsterTheme::Drainer => vec![
            ("it takes what you banked", ramp(r.drained as f64, 1.0, 25.0)),
            ("and some of it never comes back", ramp(r.mind_damage as f64, 0.0, 60.0)),
        ],
        // Bursty and mana-gated: magic, in bursts rather than evenly.
        MonsterTheme::Caster => vec![
            ("it is not a brawler", r.not_mostly(r.blow_damage, 0.5, 1.0)),
            ("and it comes in bursts", fall(r.cadence(), 1.2, 0.2)),
        ],
        // Takes your maximum away, and none of it comes back. Its damage never
        // appears in a damage share, which is `bestiary.rs`'s own claim about
        // it - so this is the one signature that asks for the *absence* of a
        // blow share.
        MonsterTheme::Hollow => vec![
            ("it takes the bar itself", r.share(r.mind_damage)),
            ("not your health", r.not_mostly(r.blow_damage, 0.2, 0.6)),
        ],
        // Everywhere at once, and nowhere for long.
        MonsterTheme::Swarm => vec![
            ("it acts constantly", ramp(r.cadence(), 0.6, 2.5)),
            ("and no one blow is the problem", fall(r.mean_blow as f64, SWARM_BLOW as f64 * 0.6, SWARM_BLOW as f64 * 1.6)),
        ],
        // No trick at all, and enough of everything else.
        MonsterTheme::Beast => vec![
            ("it hits you", r.share(r.blow_damage)),
            ("and does nothing clever", fall(r.curses as f64 + (r.drained > 0) as u32 as f64, 0.0, 3.0)),
        ],
        // Out-waits you rather than out-hitting you.
        MonsterTheme::Warden => vec![
            ("it makes you pay for time", ramp(r.ms as f64, 10_000.0, 28_000.0)),
            ("with curses", ramp(r.curses as f64, 1.0, 6.0)),
            ("rather than damage", fall(r.total_dealt as f64 / r.ms.max(1) as f64 * 1000.0, 10.0, 35.0)),
        ],
    };
    // A claim about a fight needs a fight. Every theme has a negative half -
    // "and does little else", "and does nothing clever" - and a creature that
    // did nothing at all bears every one of them out for free. So an empty
    // fight bears out nothing, which is the only honest reading of it.
    let score = if parts.is_empty() || !r.fought() {
        0.0
    } else {
        parts.iter().map(|(_, v)| v).sum::<f64>() / parts.len() as f64
    };
    Fidelity { score, parts, reading: *r }
}

/// Score a creature's own fight against its own theme.
pub fn of(theme: MonsterTheme, log: &CombatLog) -> Fidelity {
    score(theme, &Reading::of(log))
}
