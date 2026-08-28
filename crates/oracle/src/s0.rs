//! **S0** - what a board is worth, without fighting anything.
//!
//! Forty-two nanoseconds, measured (A0). A rung-fifty fight is 768,000 of
//! them, and the four-board acceptance gate is five million. So the inner loop
//! of any search belongs here and the fights belong at the end of it - which
//! is the one structural difference between this and the packer it replaces.
//!
//! ## What it reads, and why each is player-visible
//!
//! Everything below is on a screen a person looks at:
//!
//! * the six county figures (`Figures`), drawn on the county tab - what the
//!   board does *a second*, which is the question a river, a ford and a scarp
//!   are each a version of;
//! * the character sheet (`Stats`), drawn on every panel;
//! * how many items assembled, drawn on every slot.
//!
//! That matters beyond tidiness: a blind pilot may compute S0 for itself out
//! of its own `View`, so the same number scores a board on both sides of the
//! boundary. This crate has it because the packer is privileged; the pilot
//! reaches the same figure by reading its screen.
//!
//! ## Rates and quantities are not the same number
//!
//! `Stats::parts_when` (landed by T3) classifies each of the twenty stat
//! fields as `Passive`, `OnActivation` or `Damage`. **Eight of them are handed
//! over on every activation.** `+2 nature` on a 2.8-second item is a rate and
//! `+175 hp` is a quantity, and summing a stat block without that split prices
//! them the same - which on a thirty-second fight is wrong by an order of
//! magnitude. The activation group is therefore weighted by the item's own
//! cadence, and the classification is the engine's, checked against the fight
//! rather than kept in a table here.

use crate::Board;
use gearmaster_engine::loadout::{Figures, ItemProfile};
use gearmaster_engine::stats::{Stats, When};

/// A board's fight-free score.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Surrogate {
    /// The six county figures, in milli-units a second.
    pub flow: i64,
    pub physical_dps: i64,
    pub magic_dps: i64,
    pub armour_ps: i64,
    pub fastest_ms: Option<u32>,
    pub curse_resist: i32,
    /// Health and strength: what a fight has to chew through.
    pub health: i32,
    pub strength: i32,
    /// Items that actually assembled. A loose piece pays its passive stats and
    /// never acts, and only weapons swing (`CLAUDE.md` §6 trap 36).
    pub items: usize,
    /// Everything the board hands over per second, from the activation group,
    /// weighted by each item's cadence. Milli-units a second.
    pub per_second: i64,
    /// Everything true while it is worn, summed once.
    pub passive: i64,
}

/// How long a fight is assumed to last when turning a rate into a total.
///
/// Sudden death owns everything past thirty seconds, so a board is scored over
/// the window a fight is actually decided in. Ten seconds is the median clear
/// on the owner's board (A0's printer), and the figure only has to be a
/// consistent yardstick rather than a prediction.
pub const WINDOW_MS: i64 = 10_000;

/// Every field of a `Stats`, with a way to read it.
///
/// A list of *fields*, not of classifications - which of the three groups each
/// one belongs to is asked of the engine below and never written down here.
/// `tests/s0.rs` checks the list is complete by counting what `parts_when`
/// returns for a block with all of them set.
const FIELDS: &[(&str, fn(&Stats) -> i32)] = &[
    ("health", |s| s.health),
    ("strength", |s| s.strength),
    ("regen", |s| s.regen),
    ("power", |s| s.power),
    ("armor", |s| s.armor),
    ("mana", |s| s.mana),
    ("mind", |s| s.mind),
    ("mind_resist", |s| s.mind_resist),
    ("curse_resist", |s| s.curse_resist),
    ("physical_damage", |s| s.physical_damage),
    ("physical_resist", |s| s.physical_resist),
    ("physical_pierce", |s| s.physical_pierce),
    ("physical_harden", |s| s.physical_harden),
    ("magic_damage", |s| s.magic_damage),
    ("magic_resist", |s| s.magic_resist),
    ("magic_pierce", |s| s.magic_pierce),
    ("magic_harden", |s| s.magic_harden),
    ("reflect", |s| s.reflect),
    ("rage", |s| s.rage),
    ("faith", |s| s.faith),
    ("nature", |s| s.nature),
];

/// Which group each field belongs to, asked once.
///
/// **Not a table kept up to date by hand.** Each field is set on its own in an
/// otherwise empty block, `parts_when` is asked what that block contains, and
/// the answer is the classification - which is the same discipline T3 used to
/// build `parts_when` in the first place, checked against the fight rather
/// than written down. Asking per call cost 14 µs a board, because
/// `parts_when` formats every figure into a `String`; asking once costs
/// twenty-one probes at startup and nothing afterwards.
fn groups() -> &'static [(fn(&Stats) -> i32, When)] {
    static ONCE: std::sync::OnceLock<Vec<(fn(&Stats) -> i32, When)>> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let mut out = Vec::new();
        for &(name, get) in FIELDS {
            let mut probe = Stats::default();
            set(&mut probe, name, 1_000);
            let parts = probe.parts_when();
            // Exactly one figure, because exactly one field is set. A field
            // that draws nothing at all is a field with no `When`, and there
            // is no honest group to put it in.
            if let Some(&(_, _, when)) = parts.first() {
                out.push((get, when));
            }
        }
        out
    })
}

fn set(s: &mut Stats, field: &str, v: i32) {
    match field {
        "health" => s.health = v,
        "strength" => s.strength = v,
        "regen" => s.regen = v,
        "power" => s.power = v,
        "armor" => s.armor = v,
        "mana" => s.mana = v,
        "mind" => s.mind = v,
        "mind_resist" => s.mind_resist = v,
        "curse_resist" => s.curse_resist = v,
        "physical_damage" => s.physical_damage = v,
        "physical_resist" => s.physical_resist = v,
        "physical_pierce" => s.physical_pierce = v,
        "physical_harden" => s.physical_harden = v,
        "magic_damage" => s.magic_damage = v,
        "magic_resist" => s.magic_resist = v,
        "magic_pierce" => s.magic_pierce = v,
        "magic_harden" => s.magic_harden = v,
        "reflect" => s.reflect = v,
        "rage" => s.rage = v,
        "faith" => s.faith = v,
        "nature" => s.nature = v,
        other => panic!("no field {}", other),
    }
}

fn sum_group(stats: &Stats, want: When) -> i64 {
    groups()
        .iter()
        .filter(|(_, w)| *w == want)
        .map(|(get, _)| get(stats) as i64)
        .sum()
}

/// How many fields the engine classifies, for a test that wants to know the
/// list above is complete.
pub fn classified() -> usize {
    groups().len()
}

impl Surrogate {
    pub fn of(stats: Stats, items: &[ItemProfile]) -> Surrogate {
        let f = Figures::of(items);
        let mut per_second = 0i64;
        for it in items {
            if it.cooldown_ms == 0 {
                continue;
            }
            let handed = sum_group(&it.stats, When::OnActivation) + sum_group(&it.stats, When::Damage);
            per_second += handed * 1_000_000 / it.cooldown_ms as i64;
        }
        Surrogate {
            flow: f.flow,
            physical_dps: f.physical_dps,
            magic_dps: f.magic_dps,
            armour_ps: f.armour_ps,
            fastest_ms: f.fastest_ms,
            curse_resist: f.curse_resist,
            health: stats.health,
            strength: stats.strength,
            items: items.len(),
            per_second,
            passive: sum_group(&stats, When::Passive),
        }
    }

    pub fn of_board(b: &Board) -> Surrogate {
        let (stats, items) = b.profiles();
        Surrogate::of(stats, &items)
    }

    /// One number, for ordering candidates before any of them is fought.
    ///
    /// Deliberately crude: its job is to throw away the bottom nine tenths, not
    /// to rank the top. A search that trusts this to pick a winner has skipped
    /// the tier that exists to pick winners.
    pub fn rank(&self) -> i64 {
        let over_the_window = self.per_second * WINDOW_MS / 1_000_000;
        over_the_window + self.passive + self.health as i64 + self.strength as i64 * 10
            + (self.physical_dps + self.magic_dps) * WINDOW_MS / 1_000_000
            + self.armour_ps * WINDOW_MS / 1_000_000
            + self.items as i64 * 50
    }

    /// Nothing assembled at all: no item, and therefore nothing that acts.
    pub fn is_inert(&self) -> bool {
        self.items == 0
    }
}
