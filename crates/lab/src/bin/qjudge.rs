//! What the two judges see when they look at the same board.
//!
//!     cargo run --release -p gearmaster-lab --bin qjudge
//!
//! `lab::scoring` prices a board against a **window** of the rungs ahead rather
//! than against the one it is standing at, and the two modes read that window
//! differently: Grinder averages it, Rogue averages it and then pays again for
//! the worst thing in it.
//!
//! This prints the window for a spread of real boards - packed by the written
//! control out of the shop a run actually had - so the difference between the
//! two judges is a table rather than an argument. The column that matters is
//! the last one: where the two disagree about which board is better, there is a
//! Rogue quartermaster to train. Where they never disagree, there is not.

use gearmaster_console::{Difficulty, Mode};
use gearmaster_lab::curriculum;
use gearmaster_lab::scoring::{self, Judge};

fn main() {
    let seeds = [0x1212u64, 0x6060, 0xAA8D95DE31880461, 0xF1418AF3EDF965FD, 0x1111, 0x5EED1234];
    let rungs = [4usize, 9, 14, 19, 24, 29];

    println!("A board packed by the written control, judged two ways.\n");
    println!(
        "  {:<20} {:>4}  {:>26}  {:>8} {:>8}",
        "seed", "rung", "the window, rung by rung", "grinder", "rogue"
    );

    let mut all: Vec<(String, f32, f32)> = Vec::new();
    for seed in seeds {
        for rung in rungs {
            // Walked, not skipped: the board a run actually had at this rung.
            let (c, walked) = curriculum::walk_to(seed, Mode::Grinder, Difficulty::Medium, rung);
            if !walked.arrived {
                continue;
            }
            let (stats, items) = c.board_for_scoring();
            if items.is_empty() {
                continue;
            }
            let each = scoring::window(stats, &items, rung);
            let g = scoring::score(stats, &items, rung, Judge::Grinder);
            let r = scoring::score(stats, &items, rung, Judge::Rogue);
            println!(
                "  {:<20} {:>4}  {:>26}  {:>8.2} {:>8.2}",
                format!("{seed:#x}"),
                rung + 1,
                each.iter().map(|x| format!("{x:+.2}")).collect::<Vec<_>>().join(" "),
                g,
                r
            );
            all.push((format!("{seed:#x} rung {}", rung + 1), g, r));
        }
    }

    // **How much each judge distinguishes one board from another.**
    //
    // The owner's reading of why the Grinder packer values every state alike:
    // in Grinder a run retries for ever, so no board is really different from
    // another. The packer's reward never sees a retry - it is a fight - but the
    // same thing shows up one level down, in the judges themselves. Grinder
    // takes the mean of the window and a board that wins its whole window
    // scores about 1.7, which is most of them. Rogue subtracts the worst rung,
    // so the same set splits in two.
    //
    // If that is right the Grinder column has little spread and the Rogue
    // column has a lot, and there is nothing for a network to learn from the
    // first.
    let sd = |f: &dyn Fn(&(String, f32, f32)) -> f32| {
        let n = all.len().max(1) as f32;
        let mean = all.iter().map(f).sum::<f32>() / n;
        (all.iter().map(|x| (f(x) - mean) * (f(x) - mean)).sum::<f32>() / n).sqrt()
    };
    let gsd = sd(&|x| x.1);
    let rsd = sd(&|x| x.2);
    println!("\nHow much each judge tells one board from another, over {} boards:", all.len());
    println!("  grinder   sd {gsd:.3}");
    println!("  rogue     sd {rsd:.3}");
    println!(
        "  the rogue judge spreads the same boards {:.1} times as widely",
        if gsd > 0.0 { rsd / gsd } else { 0.0 }
    );

    // Where the two disagree, which is the whole question.
    let mut inversions = 0;
    let mut shown = 0;
    println!("\nPairs the two judges rank in opposite orders:\n");
    for (i, a) in all.iter().enumerate() {
        for b in all.iter().skip(i + 1) {
            let g = a.1.partial_cmp(&b.1).expect("real numbers");
            let r = a.2.partial_cmp(&b.2).expect("real numbers");
            if g != r && g != std::cmp::Ordering::Equal && r != std::cmp::Ordering::Equal {
                inversions += 1;
                if shown < 8 {
                    shown += 1;
                    println!(
                        "  grinder prefers {:<22} ({:+.2} vs {:+.2}),  rogue prefers {:<22} ({:+.2} vs {:+.2})",
                        if a.1 > b.1 { &a.0 } else { &b.0 },
                        a.1,
                        b.1,
                        if a.2 > b.2 { &a.0 } else { &b.0 },
                        a.2,
                        b.2
                    );
                }
            }
        }
    }
    let pairs = all.len() * all.len().saturating_sub(1) / 2;
    println!(
        "\n  {inversions} of {pairs} pairs. Nought would mean the Rogue judge is the\n\
         Grinder judge with extra steps, and there would be no second packer to train."
    );
}
