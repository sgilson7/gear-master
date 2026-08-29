//! Turning a `MonsterTheme` into a brief.
//!
//! This lives in `lab` and not in `trades` because it reads the engine, and
//! the packer may not. What crosses the boundary is thirteen numbers with no
//! name on them (`trades::brief`).
//!
//! ## What a theme actually is
//!
//! Two things: the grids it fills (`MonsterTheme::slots`) and a filter over
//! the catalogue (`MonsterTheme::allows`). The first is five numbers directly.
//! The second is not a number at all, so it is **measured**: walk every piece
//! the theme allows, average what those pieces produce and consume per pool,
//! and the result is a profile of what building for this theme feels like
//! from the inside. A Drainer's allowed pieces move mana and drain; a Beast's
//! move rage and nothing else. That difference is real, it is in the numbers,
//! and it is the thing a held-out theme can be interpolated from.

use gearmaster_console::read::pools_of;
use gearmaster_engine::bestiary::MonsterTheme;
use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_trades::brief::{Brief, BRIEF};

/// Every theme, in a fixed order so a split is reproducible.
pub const ALL: [MonsterTheme; 10] = [
    MonsterTheme::Striker,
    MonsterTheme::Wall,
    MonsterTheme::Burner,
    MonsterTheme::Slower,
    MonsterTheme::Drainer,
    MonsterTheme::Caster,
    MonsterTheme::Hollow,
    MonsterTheme::Swarm,
    MonsterTheme::Beast,
    MonsterTheme::Warden,
];

/// The two held out of training, and why these two.
///
/// **Hollow and Warden.** Both are off `theme_for`'s table - they belong to
/// things standing beside the road rather than on it - so holding them out
/// costs the ladder nothing. And they sit at opposite ends of what the brief
/// can express: Warden is three grids all of which some trained theme also
/// uses, and Hollow is the only theme whose damage is mind. If the brief
/// generalises to Warden and not to Hollow, that is a result about *which*
/// part of a theme thirteen numbers carry, which is worth more than a pass.
pub const HELD_OUT: [MonsterTheme; 2] = [MonsterTheme::Hollow, MonsterTheme::Warden];

/// The eight a Q8 packer trains on.
pub fn trained() -> Vec<MonsterTheme> {
    ALL.iter().copied().filter(|t| !HELD_OUT.contains(t)).collect()
}

/// The brief for a theme.
pub fn brief(t: MonsterTheme) -> Brief {
    let mut f = [0.0f32; BRIEF];
    for (i, k) in SlotKind::ALL.iter().enumerate() {
        if t.slots().contains(k) {
            f[i] = 1.0;
        }
    }
    // The pool profile, averaged over what the theme lets you build with.
    let mut n = 0.0f32;
    let mut acc = [0.0f32; 8];
    for def in CATALOG.iter().filter(|d| t.allows(d)) {
        let p = pools_of(def);
        for j in 0..8 {
            // Produced and consumed both count, and count the same. A theme
            // that leans on a pool is one whose pieces *touch* it; whether a
            // given piece fills the tank or empties it is the packer's problem
            // and precisely what it has to learn.
            acc[j] += (p.produces[j] + p.consumes[j]) as f32;
        }
        n += 1.0;
    }
    if n > 0.0 {
        // Scaled so the largest affinity is 1: the brief says *what this theme
        // leans on*, not how big the catalogue slice is. Without this a theme
        // that allows four hundred pieces and one that allows forty differ
        // mostly in how many pieces they allow, which is not a brief.
        let peak = acc.iter().cloned().fold(0.0f32, f32::max).max(1.0);
        for j in 0..8 {
            f[5 + j] = acc[j] / peak;
        }
    }
    Brief(f)
}

/// The name, for a report. Never crosses into `trades`.
pub fn name(t: MonsterTheme) -> String {
    format!("{:?}", t)
}
