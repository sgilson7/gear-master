//! How many of its own verbs can the pathfinder tell apart?
//!
//!     cargo run --release -p gearmaster-lab --bin qmoves
//!
//! `feature::mv` describes a candidate move for the network. It was written for
//! the **quartermaster**, whose moves are placements, purchases and rotations,
//! and its one-hot has eight shapes plus `_ => 8` for everything else.
//!
//! Every verb the pathfinder owns falls into that eighth bucket. This walks a
//! run, collects the road verbs actually on offer at each decision, describes
//! each one the way the network sees it, and counts the **distinct** vectors.
//! If that count is one, the road network is choosing between "pack" and "some
//! road verb", and which road verb is a coin.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_lab::packers::Packer;
use gearmaster_trades::env::{Step as RoadStep, Walking};
use gearmaster_trades::{feature, pathfinder};
use std::collections::BTreeMap;

const BUDGET: usize = 320;

fn key(f: &[f32; feature::MOVE]) -> String {
    f.iter().map(|x| format!("{x:.3}")).collect::<Vec<_>>().join(",")
}

fn main() {
    let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut offers = 0usize;
    let packer = Packer::named("control");

    // A few seeds, so the sample covers doors, towns, dungeons and fountains
    // rather than one run's worth of fighting.
    for (i, seed) in [0x1212u64, 0x6060, 0xAA8D95DE31880461, 0xF1418AF3EDF965FD].iter().enumerate() {
        let mode = if i % 2 == 0 { Mode::Grinder } else { Mode::Rogue };
        let mut c = Console::start(*seed, mode, Difficulty::Medium);
        let mut w = Walking::new(None, BUDGET);
        let mut plan_packed = None;
        loop {
            let ms = w.moves(&c);
            if ms.is_empty() || w.steps >= BUDGET {
                break;
            }
            let v = c.view();
            for s in &ms {
                if let RoadStep::Press(verb) = s {
                    offers += 1;
                    let d = pathfinder::describe(&v, s);
                    seen.entry(key(&d)).or_default().push(verb.line());
                }
            }
            // Walk the run the way a control does, so the sample is a real run
            // rather than one rung repeated: pack once a rung, then act.
            let at = if plan_packed != Some(v.rung_shown)
                && ms.iter().any(|s| matches!(s, RoadStep::Pack))
            {
                plan_packed = Some(v.rung_shown);
                ms.iter().position(|s| matches!(s, RoadStep::Pack)).expect("just checked")
            } else {
                ms.iter().position(|s| matches!(s, RoadStep::Press(_))).unwrap_or(0)
            };
            match &ms[at] {
                RoadStep::Pack => packer.pack(&mut c, 40),
                RoadStep::Press(verb) => {
                    if !c.apply(*verb).ok {
                        break;
                    }
                }
            }
            w.steps += 1;
        }
    }

    println!("{offers} road verbs offered across four runs.\n");
    println!("Distinct feature vectors the network can see: {}\n", seen.len());
    for (k, verbs) in &seen {
        let mut kinds: Vec<String> =
            verbs.iter().map(|l| l.split_whitespace().next().unwrap_or("?").to_string()).collect();
        kinds.sort();
        kinds.dedup();
        println!("  {:>5} presses, {:>2} verb kinds: {}", verbs.len(), kinds.len(), kinds.join(" "));
        println!("        {}", &k[..k.len().min(96)]);
    }
    println!(
        "\nAnd `Pack` is the all-zero vector, which is the one thing that is\n\
         distinguishable from all of them."
    );
}
