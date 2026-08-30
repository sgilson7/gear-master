//! What is actually in a trained net, and how far it is from the one it started as.
//!
//!     cargo run --release -p gearmaster-lab --bin qmind
//!
//! Both agents are the same shape: 70 -> 96 -> 96 -> 1 with ReLU, 16,225
//! numbers. `QNet::parse` reads them out of a text file, one line a tensor.
//!
//! ## Why the biases are the honest column
//!
//! The weights start uniform on `+/- sqrt(2/fan_in)` and the **biases start at
//! zero**. So a weight that has moved is hard to see against its own spread,
//! and a bias that is not zero is learning and nothing else. The three bias
//! rows below are the least deniable measurement of whether anything happened.
//!
//! ## And why two different Q spreads
//!
//! *Across states* is how much the network thinks the situation matters.
//! *Across actions at one state* is how much it thinks the decision matters,
//! and it is the one that decides what gets pressed. A net can have a large
//! first and a tiny second - that is a value function that knows where it is
//! and not what to do - and only printing both tells them apart.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_lab::curriculum;
use gearmaster_lab::packers::Packer;
use gearmaster_trades::brief::Brief;
use gearmaster_trades::env::{Move, Packing, Step as RoadStep, Walking};
use gearmaster_trades::{feature, pathfinder, QNet};

/// The standard deviation a freshly initialised layer has.
///
/// `init` draws uniform on `+/- sqrt(2/rows)`, and a uniform on `+/-a` has
/// standard deviation `a/sqrt(3)`.
fn init_std(rows: usize) -> f32 {
    (2.0 / rows as f32).sqrt() / 3f32.sqrt()
}

fn stats(v: &[f32]) -> (f32, f32, f32) {
    let n = v.len().max(1) as f32;
    let mean = v.iter().sum::<f32>() / n;
    let sd = (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n).sqrt();
    let big = v.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));
    (mean, sd, big)
}

fn main() {
    for (what, path) in [
        ("rogue quartermaster", "analysis/nets/quartermaster_rogue.txt"),
        ("grinder quartermaster", "analysis/nets/quartermaster_grinder.txt"),
        ("rogue pathfinder", "analysis/nets/pathfinder-rogue.txt"),
        ("grinder pathfinder", "analysis/nets/pathfinder-grinder.txt"),
    ] {
        println!("\n================ {what}\n{path}");
        let Some(net) = QNet::load(path) else {
            println!("  did not load");
            continue;
        };
        weights(&net);
        if what.contains("quartermaster") {
            packing_values(&net);
        } else {
            road_values(&net, if what.contains("rogue") { Mode::Rogue } else { Mode::Grinder });
        }
    }
}

fn weights(net: &QNet) {
    println!(
        "\n  layer   count      mean        sd    largest    sd at init   moved",
    );
    for (name, v, fan_in) in net.layers() {
        let (mean, sd, big) = stats(v);
        if fan_in == 0 {
            // A bias starts at exactly zero, so any spread at all is learning.
            println!(
                "  {name:<6} {:>6}  {:>8.4}  {:>8.4}  {:>9.4}    {:>8}   {}",
                v.len(),
                mean,
                sd,
                big,
                "0 (exact)",
                if sd > 0.0 || big > 0.0 { "yes" } else { "NO" }
            );
        } else {
            let want = init_std(fan_in);
            println!(
                "  {name:<6} {:>6}  {:>8.4}  {:>8.4}  {:>9.4}    {:>8.4}   {:+.0}%",
                v.len(),
                mean,
                sd,
                big,
                want,
                (sd / want - 1.0) * 100.0
            );
        }
    }
}

/// What the packer's values look like on boards a run actually had.
fn packing_values(net: &QNet) {
    let mut across_states: Vec<f32> = Vec::new();
    let mut within: Vec<f32> = Vec::new();
    let mut menu = 0usize;
    let mut seen = 0usize;
    for seed in [0x1212u64, 0xAA8D95DE31880461] {
        for rung in [4usize, 14] {
            let (mut c, walked) = curriculum::repack_at(seed, Mode::Rogue, Difficulty::Medium, rung);
            if !walked.arrived {
                continue;
            }
            let mut e = Packing::new(40);
            for _ in 0..40 {
                let ms: Vec<Move> = e.moves(&c);
                if ms.is_empty() {
                    break;
                }
                let v = c.view();
                let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                let qs: Vec<f32> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                        Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                    })
                    .collect();
                let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
                let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
                within.push(hi - lo);
                across_states.push(hi);
                menu += ms.len();
                seen += 1;
                let at = qs
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                    .map(|(i, _)| i)
                    .expect("not empty");
                e.step(&mut c, ms[at]);
                if e.finished {
                    break;
                }
            }
        }
    }
    report(&across_states, &within, menu, seen, "placements and buys");
}

/// And the pathfinder's, on a road it actually walks.
fn road_values(net: &QNet, mode: Mode) {
    let packer = Packer::named("control");
    let mut across_states: Vec<f32> = Vec::new();
    let mut within: Vec<f32> = Vec::new();
    let (mut menu, mut seen) = (0usize, 0usize);
    let mut c = Console::start(0x1212, mode, Difficulty::Medium);
    let mut w = Walking::new(None, 200);
    for _ in 0..200 {
        let ms = w.moves(&c);
        if ms.is_empty() {
            break;
        }
        let v = c.view();
        let r = pathfinder::road(&v, None);
        let qs: Vec<f32> = ms
            .iter()
            .map(|s| net.q_pair(&pathfinder::pair(&r, &pathfinder::describe(&v, s))))
            .collect();
        let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
        let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
        within.push(hi - lo);
        across_states.push(hi);
        menu += ms.len();
        seen += 1;
        let at = qs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
            .map(|(i, _)| i)
            .expect("not empty");
        match &ms[at] {
            RoadStep::Pack => packer.pack(&mut c, 40),
            RoadStep::Press(verb) => {
                if !c.apply(*verb).ok {
                    break;
                }
            }
        }
        w.steps += 1;
        if c.view().wiped {
            break;
        }
    }
    report(&across_states, &within, menu, seen, "road decisions");
}

fn report(across: &[f32], within: &[f32], menu: usize, seen: usize, what: &str) {
    if across.is_empty() {
        println!("\n  no {what} to look at");
        return;
    }
    let (amean, asd, _) = stats(across);
    let (wmean, wsd, wbig) = stats(within);
    let lo = across.iter().cloned().fold(f32::MAX, f32::min);
    let hi = across.iter().cloned().fold(f32::MIN, f32::max);
    println!("\n  over {seen} {what}, {:.1} choices apiece", menu as f32 / seen.max(1) as f32);
    println!("    best action's value, across states:  {lo:+.4} to {hi:+.4}   sd {asd:.4}  mean {amean:+.4}");
    println!("    best minus worst, at one state:      mean {wmean:.4}  sd {wsd:.4}  largest {wbig:.4}");
    println!(
        "    the decision is worth {:.1}% of what the situation is worth",
        if asd > 0.0 { 100.0 * wmean / asd } else { 0.0 }
    );
}
