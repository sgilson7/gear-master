//! The privileged half: fights, figures and what a theme is worth.
//!
//! Three tiers of score, cheapest first, each one filtering for the next
//! (`design/the-apprentice.md` §5). A0 measured all three on this machine and
//! the spread is the whole design:
//!
//! | | what | cost |
//! |---|---|---|
//! | **S0** | the board's own figures - no fight at all | **42 ns** |
//! | **S1** | one fight against one creature | 0.5-0.8 ms |
//! | **S2** | the acceptance gate: four boards by four settings | 5.0 ms |
//!
//! S0 is seventeen thousand times cheaper than a rung-50 fight, so a search
//! that rejects a candidate on figures pays nothing for the rejection. That is
//! why `pack_francis` is slow: it pays S2 on all three hundred of its draws.

pub mod board;
pub mod fidelity;
pub mod gate;
pub mod s0;

pub use board::{Board, Placement};
pub use fidelity::{Fidelity, Reading};
pub use gate::{Beat, Gate, Verdict};
pub use s0::Surrogate;

use gearmaster_engine::combat::{simulate_at, Difficulty, MonsterSpec, Outcome};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::stats::Stats;
use std::cell::RefCell;
use std::collections::HashMap;

/// One fight, reduced to what a search wants from it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Fight {
    pub won: bool,
    pub ms: u32,
    /// Health left on the winning side, which is the tiebreak that gives a
    /// loss a gradient. Win or lose, "how close was it" is the only thing
    /// separating two boards that both lost.
    pub health_left: i32,
    pub enemy_health_left: i32,
    /// Did the creature land anything at all? A creature that cannot reach you
    /// is not an easier fight, it is not a fight.
    pub hurt: bool,
    /// A win inside sudden death. A win past it was the clock's.
    pub board_decided: bool,
}

impl Fight {
    fn of(log: &gearmaster_engine::combat::CombatLog) -> Fight {
        use gearmaster_engine::combat::{Event, Side};
        let won = log.outcome == Outcome::Victory;
        Fight {
            won,
            ms: log.duration_ms,
            health_left: log.player.health,
            enemy_health_left: log.enemy().health,
            hurt: log.entries.iter().any(|e| {
                matches!(
                    e.event,
                    Event::Hit { by: Side::Enemy, .. }
                        | Event::MindHit { by: Side::Enemy, .. }
                        | Event::Burn { side: Side::Player, .. }
                )
            }),
            board_decided: won
                && log.duration_ms < gearmaster_engine::combat::SUDDEN_DEATH_MS,
        }
    }
}

/// What a cached answer is keyed on.
///
/// The plan's key (§4.3), with one addition: the player board as well as the
/// creature, because this oracle scores both sides of the same fight. A purse
/// is bucketed rather than exact - `SpendGold` reaches into run gold during a
/// fight and the answer changes at the thresholds, not at every coin.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub player: u64,
    pub creature: u64,
    pub difficulty: u8,
    pub purse_bucket: u8,
}

/// A memoised fight.
///
/// Local search revisits the same board constantly - remove a piece, put it
/// back, try the next seat - so revisits are the common case rather than the
/// exception. The cache is not an optimisation on top of the design; it is
/// what makes local search cheaper than resampling, which is the whole
/// argument against the incumbent packer.
pub struct Oracle {
    seen: RefCell<HashMap<Key, Fight>>,
    hits: RefCell<u64>,
    misses: RefCell<u64>,
}

impl Default for Oracle {
    fn default() -> Self {
        Oracle::new()
    }
}

impl Oracle {
    pub fn new() -> Oracle {
        Oracle {
            seen: RefCell::new(HashMap::new()),
            hits: RefCell::new(0),
            misses: RefCell::new(0),
        }
    }

    /// Hits, misses.
    pub fn tally(&self) -> (u64, u64) {
        (*self.hits.borrow(), *self.misses.borrow())
    }

    pub fn clear(&self) {
        self.seen.borrow_mut().clear();
    }

    /// **S1** - one fight, memoised.
    pub fn fight(
        &self,
        player: &Board,
        stats: Stats,
        items: &[ItemProfile],
        spec: &MonsterSpec,
        d: Difficulty,
    ) -> Fight {
        let key = Key {
            player: player.key(),
            creature: spec_key(spec),
            difficulty: d as u8,
            purse_bucket: 0,
        };
        if let Some(&f) = self.seen.borrow().get(&key) {
            *self.hits.borrow_mut() += 1;
            return f;
        }
        *self.misses.borrow_mut() += 1;
        let f = Fight::of(&simulate_at(stats, items, spec, d));
        self.seen.borrow_mut().insert(key, f);
        f
    }

    /// The same fight, without the cache. What the cache is checked against.
    pub fn fight_uncached(
        stats: Stats,
        items: &[ItemProfile],
        spec: &MonsterSpec,
        d: Difficulty,
    ) -> Fight {
        Fight::of(&simulate_at(stats, items, spec, d))
    }
}

/// A creature's identity, for a cache key: its name and everything it wears.
///
/// The name alone will not do - the whole point of a packer is that two
/// creatures with one name and two boards are two different fights.
pub fn spec_key(spec: &MonsterSpec) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut eat = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    };
    for b in spec.name.bytes() {
        eat(b as u64);
    }
    eat(spec.health as u64);
    eat(spec.strength as u64);
    eat(spec.gear_offset as i64 as u64);
    for &(name, slot, x, y, rot) in spec.gear {
        for b in name.bytes() {
            eat(b as u64);
        }
        eat(slot.index() as u64);
        eat(((x as u64) << 16) | ((y as u64) << 8) | rot as u64);
    }
    for &c in spec.items {
        eat(c as u64 | 0x1_0000_0000);
    }
    h
}

/// Turn a candidate board into a creature.
///
/// `MonsterSpec` holds `&'static` slices, so a candidate has to be leaked -
/// which `pack_francis:942` also does. One leak per **distinct** board rather
/// than per trial, because the caller holds the result and the search revisits
/// the same board constantly.
pub fn as_creature(base: &MonsterSpec, board: &Board) -> MonsterSpec {
    let gear: &'static [(&'static str, gearmaster_engine::piece::SlotKind, u8, u8, u8)] =
        Box::leak(board.named().into_boxed_slice());
    let items: &'static [usize] = Box::leak(board.chunks.clone().into_boxed_slice());
    MonsterSpec { gear, items, ..*base }
}
