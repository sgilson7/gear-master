//! Does the "completes a recipe" feature fire, and does it point at the right
//! placements?
//!
//!     cargo run --release -p gearmaster-lab --bin completes
//!
//! Q7 ended on the claim that a placement's value is whether it finishes an
//! item, and that the move features could not say so. Three numbers were added
//! (`feature::mv` f[26..29]). This checks they are not always zero, that they
//! separate placements from each other, and - the only test that matters -
//! that a placement the feature calls *completing* actually assembles an item.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::run::Run;
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::feature;

fn main() {
    let (mut fired, mut agreed, mut lied, mut missed, mut seen) = (0, 0, 0, 0, 0);
    for s in 0..40u64 {
        let seed = 0xC0FFEE + s * 7919;
        let rung = (s as usize * 3) % 24;
        let fresh = || {
            let mut r = Run::start(seed, Mode::Grinder, Difficulty::Medium);
            r.skip_to(rung);
            Console::standing_in(r, s)
        };
        // **Play in first.** From an empty board no single placement can
        // finish a two-piece recipe, so measuring the opening move measures
        // nothing - the first version of this reported 2,640 placements and
        // zero completions of any kind, which was the harness and not the
        // feature.
        let steps = (s % 5) as usize + 1;
        let played: Vec<Verb> = {
            let mut c = fresh();
            let e = Packing::new(60);
            let mut out = Vec::new();
            for k in 0..steps {
                let places: Vec<Verb> = e
                    .moves(&c)
                    .into_iter()
                    .filter_map(|m| match m {
                        Move::Press(v @ Verb::Place { .. }) => Some(v),
                        _ => None,
                    })
                    .collect();
                if places.is_empty() {
                    break;
                }
                let v = places[(s as usize * 31 + k * 7) % places.len()];
                c.apply(v);
                out.push(v);
            }
            out
        };
        let fresh = || {
            let mut r = Run::start(seed, Mode::Grinder, Difficulty::Medium);
            r.skip_to(rung);
            let mut c = Console::standing_in(r, s);
            for &v in &played {
                c.apply(v);
            }
            c
        };
        let c = fresh();
        let e = Packing::new(60);
        let v = c.view();
        for m in e.moves(&c) {
            let Move::Press(verb @ Verb::Place { .. }) = m else { continue };
            let f = feature::mv(&v, verb);
            seen += 1;
            if f[27] > 0.0 {
                fired += 1;
            }
            // The only check with teeth: press it, on a run built the same way,
            // and count items before and after.
            let before = c.figures().2;
            let mut c2 = fresh();
            c2.apply(verb);
            let after = c2.figures().2;
            match (f[27] > 0.0, after > before) {
                (true, true) => agreed += 1,
                (true, false) => lied += 1,
                (false, true) => missed += 1,
                _ => {}
            }
        }
    }
    println!("# Does the completion feature mean anything?\n");
    println!(
        "`cargo run --release -p gearmaster-lab --bin completes`. Forty situations, every \
         legal\nplacement in each, the feature read and then the key actually pressed.\n"
    );
    println!("| | count |");
    println!("|---|---:|");
    println!("| placements offered | {} |", seen);
    println!("| feature said \"this completes an item\" | {} |", fired);
    println!("| **and it did** | **{}** |", agreed);
    println!("| and it did not | {} |", lied);
    println!("| feature was silent and an item assembled anyway | {} |", missed);
    if fired > 0 {
        println!("\nPrecision **{:.0}%**", 100.0 * agreed as f64 / fired as f64);
    }
    let real = agreed + missed;
    if real > 0 {
        println!("Recall **{:.0}%**", 100.0 * agreed as f64 / real as f64);
    }
}
