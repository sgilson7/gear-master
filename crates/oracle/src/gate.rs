//! The acceptance gate, **ported rather than reinvented**.
//!
//! `design/the-apprentice.md` §8 says to port `pack_francis`'s gate and not to
//! write a new one, and the reason is that the gate is where four missions of
//! balance judgement is written down: the curve, the band, the flat window,
//! the casino's bar, the preset corridor, the rank rule. Every constant below
//! is that file's, with its argument kept beside it in short form; the long
//! version is in `tests/pack_francis.rs` and stays the original.
//!
//! `tests/gate.rs` checks the port against the original on a board the
//! original accepted, which is what makes this a port rather than a rewrite.

use crate::{Board, Fight, Oracle};
use gearmaster_engine::combat::{Difficulty, MonsterSpec, Rank};
use gearmaster_engine::piece::SlotKind;

/// One fight, as the gate reads it.
pub type Beat = Fight;

/// Rungs 1-10 all want the same target, because `pieces_for` holds the early
/// ladder at four or five pieces and a curve that climbs through that window
/// asks the search to make the same four pieces harder out of nothing.
pub const FLAT_UNTIL: usize = 10;

/// Where the line starts. Measured, not chosen: the binding rung is 3, whose
/// hardest reachable striker dies to the owner's board in 2.0 s.
pub const FLOOR_MS: u32 = 2_000;

/// How far off the curve a board may land.
pub const BAND: f64 = 0.30;

/// And wider while the curve is flat, because a theme has its own natural
/// speed and the flat window asks a wall and a striker for the same number.
pub const FLAT_BAND: f64 = 0.60;

/// The casino's bar, which the shallow end must stay the wrong side of.
pub const CASINO_BAR_MS: u32 = 3_000;

/// The line a creature is packed against.
pub fn target_ms(rung: usize) -> u32 {
    if rung < FLAT_UNTIL {
        FLOOR_MS
    } else {
        FLOOR_MS + 490 * (rung + 1 - FLAT_UNTIL) as u32
    }
}

pub fn band_for(rung: usize) -> f64 {
    if rung < FLAT_UNTIL {
        FLAT_BAND
    } else {
        BAND
    }
}

/// How far off the curve a candidate lands, as a fraction. A loss is
/// infinitely far: a creature the reference board cannot beat is not on the
/// curve at all.
pub fn off_curve(owner_medium: Beat, rung: usize) -> f64 {
    if !owner_medium.won {
        return f64::MAX;
    }
    let want = target_ms(rung) as f64;
    (owner_medium.ms as f64 - want).abs() / want
}

/// A fight the preset used to win, it must still win - and not take more than
/// twice as long doing it. Deeper than the preset can reach, it says nothing.
pub fn preset_holds(before: Beat, after: Beat) -> bool {
    if !before.won {
        return true;
    }
    after.won && after.ms as f64 <= before.ms as f64 * 2.0
}

pub fn in_the_shallow_window(rung: usize) -> bool {
    gearmaster_engine::event::SHALLOW.contains(&rung)
}

/// Does this board hold what its rank owes every slot it wears?
pub fn rank_is_satisfied(rank: Rank, board: &Board) -> bool {
    let mut per: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut at = 0usize;
    for &c in &board.chunks {
        if at >= board.gear.len() {
            break;
        }
        *per.entry(board.gear[at].1.index()).or_default() += 1;
        at += c;
    }
    per.len() >= rank.min_slots()
        && per.iter().all(|(&slot, &n)| n >= rank.min_items_in(SlotKind::ALL[slot]))
}

/// Why a candidate was refused, or that it was not.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Inside the band, and every corridor held. `off` is how far from the
    /// line it landed - smaller is better, and it is the thing to minimise.
    Accepted { off: f64 },
    /// Held nothing its rank owes every slot it wears.
    RankUnmet,
    /// Could not land a blow on an ordinary board. Not an easier fight - not
    /// a fight.
    Harmless,
    /// A board a player might actually have at this rung stopped getting past.
    PresetBroken { which: &'static str },
    /// It would fall to an ordinary board fast enough to hand the casino to a
    /// run that was meant to walk the long way.
    ShallowTooFast { ms: u32 },
    /// Outside the band around the curve.
    OffCurve { off: f64, allowed: f64, lost: bool },
}

impl Verdict {
    pub fn accepted(&self) -> bool {
        matches!(self, Verdict::Accepted { .. })
    }
    /// How far off the line, for ranking two accepted boards.
    pub fn off(&self) -> f64 {
        match self {
            Verdict::Accepted { off } | Verdict::OffCurve { off, .. } => *off,
            _ => f64::MAX,
        }
    }
}

/// The four reference boards, weakest first, as the gate reads them.
///
/// Order matters and the original says why: two ladder-clearing boards beat a
/// rung-two creature whatever it wears, so a gate holding only those left the
/// search free to over-pack the shallow end and call the profile unchanged.
pub struct References {
    pub boards: Vec<(&'static str, gearmaster_engine::stats::Stats, Vec<gearmaster_engine::loadout::ItemProfile>, Board)>,
}

/// A row of four fights - one board at each difficulty.
pub type Row = [Beat; 4];

/// The whole gate: what the creature was, what the candidate is, and the
/// verdict on the change.
pub struct Gate<'a> {
    pub refs: &'a References,
    pub rung: usize,
    pub rank: Rank,
}

impl Gate<'_> {
    /// Fight one creature with all four reference boards at all four settings.
    pub fn rows(&self, oracle: &Oracle, spec: &MonsterSpec) -> Vec<Row> {
        self.refs
            .boards
            .iter()
            .map(|(_, stats, items, board)| {
                let mut row = [Beat {
                    won: false,
                    ms: 0,
                    health_left: 0,
                    enemy_health_left: 0,
                    hurt: false,
                    board_decided: false,
                }; 4];
                for (i, d) in Difficulty::ALL.iter().enumerate() {
                    row[i] = oracle.fight(board, *stats, items, spec, *d);
                }
                row
            })
            .collect()
    }

    /// Judge a candidate against what the creature already gives.
    ///
    /// `was` is the incumbent's rows, `got` the candidate's. Both are indexed
    /// the way `References` is ordered, and Medium is index 1.
    pub fn judge(&self, was: &[Row], got: &[Row], board: &Board) -> Verdict {
        const MEDIUM: usize = 1;
        const EARLY: usize = 0;
        const PRESET: usize = 1;
        const OWNER: usize = 2;

        if !rank_is_satisfied(self.rank, board) {
            return Verdict::RankUnmet;
        }
        if !got[PRESET][MEDIUM].hurt {
            return Verdict::Harmless;
        }
        for (i, which) in [(EARLY, "four-piece"), (PRESET, "preset")] {
            if !preset_holds(was[i][MEDIUM], got[i][MEDIUM]) {
                return Verdict::PresetBroken { which };
            }
        }
        if in_the_shallow_window(self.rung) && got[PRESET][MEDIUM].ms < CASINO_BAR_MS {
            return Verdict::ShallowTooFast { ms: got[PRESET][MEDIUM].ms };
        }
        let off = off_curve(got[OWNER][MEDIUM], self.rung);
        let allowed = band_for(self.rung);
        if off > allowed {
            return Verdict::OffCurve { off, allowed, lost: !got[OWNER][MEDIUM].won };
        }
        Verdict::Accepted { off }
    }
}

impl References {
    /// The four the gate is read off, weakest first.
    ///
    /// Rebuilt through `Board`, which locks each item as it completes - the
    /// one reconstruction there is (`board.rs`). The original hand-rolled this
    /// loop once without locking as it went, which is the fault that made a
    /// finished build come back as thirteen items instead of nineteen, and it
    /// mattered there more than anywhere because the curve every creature is
    /// packed against is read off the owner's board.
    pub fn standard() -> References {
        let mut boards = Vec::new();
        for (label, code) in [
            ("early", ""),
            ("preset", ""),
            ("owner", gearmaster_engine::share::A_WINNING_RUN),
            ("friend", gearmaster_engine::share::A_FRIENDS_RUN),
        ] {
            let board = if code.is_empty() {
                if label == "early" {
                    early_board()
                } else {
                    preset_board()
                }
            } else {
                from_code(code)
            };
            let (stats, items) = board.profiles();
            boards.push((label, stats, items, board));
        }
        References { boards }
    }
}

fn from_code(code: &str) -> Board {
    let shared = gearmaster_engine::share::import(code).expect("a code the repo ships");
    Board {
        gear: shared.placed.iter().map(|&(d, s, x, y, r)| (d, s, x, y, r)).collect(),
        chunks: Vec::new(),
        rows: shared.slot_rows.map(|r| r + shared.extra_rows),
    }
}

/// A handle, a blade, and something to stand up in - what `earned_events`
/// walks the shallow end with, and the yardstick the bottom of the ladder is
/// really written for.
fn early_board() -> Board {
    board_of(&["Oak Handle", "Iron Blade", "Adamant Base", "Riveted Layer"])
}

/// What the auto-builder puts down: twenty-two placements somebody chose.
fn preset_board() -> Board {
    let mut run = gearmaster_engine::run::Run::new();
    run.apply_preset();
    let mut gear = Vec::new();
    for k in gearmaster_engine::piece::SlotKind::ALL {
        let slot = run.loadout.slot(k);
        for id in slot.pieces() {
            if let Some((x, y)) = slot.anchor_of(id) {
                gear.push((run.registry.def_index(id), k, x, y, run.registry.rotation(id)));
            }
        }
    }
    Board { gear, chunks: Vec::new(), rows: run.slot_rows() }
}

/// Seat a named list at the first place each will sit.
fn board_of(names: &[&str]) -> Board {
    let mut b = Board::default();
    for name in names {
        let Some(def) = gearmaster_engine::piece::CATALOG.iter().position(|d| &d.name == name)
        else {
            continue;
        };
        let slot = gearmaster_engine::piece::CATALOG[def].slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                let mut trial = b.clone();
                trial.gear.push((def, slot, x, y, 0));
                // Seated only if the rebuild kept it - which is the same
                // question `can_place` asks, asked through the one path.
                let (_, lo) = trial.rebuild();
                if lo.slot(slot).pieces().len() > b.rebuild().1.slot(slot).pieces().len() {
                    b = trial;
                    break 'seat;
                }
            }
        }
    }
    b
}
