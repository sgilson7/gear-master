//! Turning a board somebody played into a board a creature can wear.
//!
//! Three rules, and every one of them is the engine's rather than mine: a
//! creature's gear must all assemble (`MonsterSpec::unassembled`), it may not
//! wear boss or event gear, and a theme fills two or three grids.
//!
//! This lives in `lab` because it reads `Run` directly. It was `harvest`'s
//! private middle until Q8 needed the same cut to ask A2's meter what a packed
//! board reads as - and two copies of a cut like this is exactly how a
//! measurement and the thing it measures drift apart.

use gearmaster_engine::piece::{is_boss_only, is_event_only, SlotKind};
use gearmaster_engine::run::Run;

/// What came off a board, and what would not go on a creature.
pub struct Cut {
    pub gear: Vec<(usize, SlotKind, u8, u8, u8)>,
    pub chunks: Vec<usize>,
    pub dropped: Vec<String>,
}

/// Keep the assembled items in `wanted`, drop what a creature may not wear.
pub fn cut(run: &Run, wanted: &[SlotKind]) -> Cut {
    let mut out = Cut { gear: Vec::new(), chunks: Vec::new(), dropped: Vec::new() };
    for k in wanted.iter().copied() {
        for item in run.report(k).items.iter().filter(|i| i.assembled) {
            let names: Vec<&str> = item.pieces.iter().map(|&p| run.registry.def(p).name).collect();
            if names.iter().any(|n| is_boss_only(n) || is_event_only(n)) {
                out.dropped.push(format!("{} - holds gear a creature may not wear", item.name.full));
                continue;
            }
            let slot = run.loadout.slot(k);
            let mut placed = 0;
            for &p in &item.pieces {
                let Some((x, y)) = slot.anchor_of(p) else { continue };
                out.gear.push((run.registry.def_index(p), k, x, y, run.registry.rotation(p)));
                placed += 1;
            }
            if placed > 0 {
                out.chunks.push(placed);
            }
        }
    }
    out
}
