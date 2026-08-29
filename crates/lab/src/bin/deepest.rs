//! Which seeds a run gets deepest on, and what board it is holding there.
//!
//!     cargo run --release -p gearmaster-lab --bin deepest
//!
//! Q9's harvest turns a board **the economy produced** into a creature, and a
//! creature at band 41 needs a board from a run that reached band 41. The
//! first harvest ran on a seed that stopped at rung 10 and the gate said
//! `RankUnmet` at 2.8 s against a 17.2 s line - which is not a harvest
//! failing, it is a harvest asked for something it was never given.
//!
//! So: play a lot of seeds, and report the ones that go deep, with the item
//! count they hold when they get there.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Difficulty, Mode};

fn main() {
    let n: u64 = std::env::var("DEEPEST_N").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
    let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };
    let mut rows: Vec<(usize, u64, usize, Mode)> = Vec::new();
    for i in 0..n {
        let seed = 0xC434_E4A6_8C59_0000u64.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for mode in [Mode::Grinder, Mode::Rogue] {
            let e = pilot::play(seed, mode, Difficulty::Medium, d);
            rows.push((e.best_rung, seed, e.board_clears, mode));
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    println!("# The deepest runs\n");
    println!(
        "`cargo run --release -p gearmaster-lab --bin deepest`. {} seeds in both modes, the \
         A-series\npilot, Medium.\n",
        n
    );
    println!("| seed | mode | deepest rung | board clears |");
    println!("|---|---|---:|---:|");
    for (deep, seed, items, mode) in rows.iter().take(15) {
        println!("| `{:#018X}` | {:?} | **{}** | {} |", seed, mode, deep, items);
    }
    let best = rows.first().unwrap();
    println!(
        "\nDeepest: `{:#018X}` in {:?}, rung {}.\n\n    HARVEST_SEED={:#X} HARVEST_FOR=\"THE DROVER\" \\\n      cargo run --release -p gearmaster-lab --bin harvest",
        best.1, best.3, best.0, best.1
    );
}
