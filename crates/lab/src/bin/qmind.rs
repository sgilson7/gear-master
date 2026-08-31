//! What is actually in a trained net, and how far it is from the one it started as.
//!
//!     cargo run --release -p gearmaster-lab --bin qmind
//!
//! `QNet::read` takes them out of a text file, one line a tensor, and the shape
//! is read out of the file rather than assumed: a packing net is
//! `feature::PAIR -> 96 -> 96 -> 1` with ReLU and a road net is stored at the
//! same width with a narrower pair inside it. Both numbers have moved twice, so
//! the width is printed beside every net below - a checkpoint whose width is
//! not this build's is a measurement nobody can repeat, and it used to say
//! `did not load` and nothing else.
//!
//! Give it `QMIND_NET=<path>` to look at one net, `QMIND_KIND=road|packing` to
//! say which agent's it is when the file does not.
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

/// Which agent's net this is, which decides what it may be fed.
#[derive(Copy, Clone, PartialEq)]
enum Kind {
    Packing,
    Road,
}

impl Kind {
    fn wants(self) -> usize {
        match self {
            Kind::Packing => feature::PAIR,
            Kind::Road => pathfinder::PAIR,
        }
    }
}

/// What a file says it is, when nobody has said.
///
/// The stamp if there is one. Failing that the width, which is the right
/// question for a packing net and only half of one for a road net - so the
/// guess is printed rather than assumed.
fn guess(path: &str) -> Kind {
    let Ok(text) = std::fs::read_to_string(path) else { return Kind::Packing };
    let Ok(net) = QNet::read(&text) else { return Kind::Packing };
    match net.declared() {
        Some(w) if w == pathfinder::PAIR => Kind::Road,
        Some(_) => Kind::Packing,
        None if net.width() == feature::PAIR => Kind::Packing,
        None => Kind::Road,
    }
}

fn main() {
    // **The two nets this instrument was needed for and never listed.** The
    // collapse is a comparison between the best block and the one the run
    // ended on, and `qmind` looked at four published nets and neither of these.
    let shelf: Vec<(String, String, Kind)> = match std::env::var("QMIND_NET") {
        Ok(path) => {
            let kind = match std::env::var("QMIND_KIND").as_deref() {
                Ok("road") => Kind::Road,
                Ok("packing") => Kind::Packing,
                _ => guess(&path),
            };
            vec![("the net asked for".into(), path, kind)]
        }
        Err(_) => [
            ("the row, best block", "runs/quartermaster_row.txt", Kind::Packing),
            ("the row, where it ended", "runs/quartermaster_row_last.txt", Kind::Packing),
            ("rogue quartermaster", "analysis/nets/quartermaster_rogue.txt", Kind::Packing),
            ("grinder quartermaster", "analysis/nets/quartermaster_grinder.txt", Kind::Packing),
            ("rogue pathfinder", "analysis/nets/pathfinder-rogue.txt", Kind::Road),
            ("grinder pathfinder", "analysis/nets/pathfinder-grinder.txt", Kind::Road),
        ]
        .iter()
        .map(|(w, p, k)| (w.to_string(), p.to_string(), *k))
        .collect(),
    };

    for (what, path, kind) in shelf {
        println!("\n================ {what}\n{path}");
        let net = match QNet::load_at(&path, kind.wants()) {
            Ok(net) => net,
            // The refusal is the measurement here as much as anything below it.
            Err(why) => {
                println!("  {why}");
                continue;
            }
        };
        println!(
            "  {} wide, stamped {}, hidden 96",
            net.width(),
            match net.declared() {
                Some(w) => w.to_string(),
                None => "nothing".into(),
            }
        );
        weights(&net);
        match kind {
            Kind::Packing => {
                packing_values(&net);
                briefed_against_bare(&net);
            }
            Kind::Road => {
                road_values(&net, if what.contains("rogue") { Mode::Rogue } else { Mode::Grinder })
            }
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

/// **The same packer, asked for a theme and asked for nothing.**
///
/// `qpack` trains every episode under a real brief and evaluates under a fixed
/// one; `lab::packers::learned` hands over `Brief::NONE`, thirteen zeros,
/// because nothing in the road agent's world asks for a theme. So the packer
/// plays under conditioning it has never been trained on, and the figure that
/// says whether that matters is this one: the same situations packed twice.
fn briefed_against_bare(net: &QNet) {
    use gearmaster_lab::scoring::{self, Judge};
    use gearmaster_lab::themes;
    let themed = themes::brief(themes::trained()[0]);
    println!("\n  the same boards, packed under two briefs:");
    println!("    {:<22} {:>7} {:>9}", "asked for", "items", "score");
    for (name, w) in [("a theme", themed), ("nothing", Brief::NONE)] {
        let (mut items, mut score, mut n) = (0usize, 0.0f32, 0usize);
        for seed in [0x1212u64, 0xAA8D95DE31880461, 0xF1418AF3EDF965FD] {
            for rung in [0usize, 4, 9] {
                let (mut c, walked) =
                    curriculum::repack_at(seed, Mode::Rogue, Difficulty::Medium, rung);
                if !walked.arrived {
                    continue;
                }
                let mut e = Packing::new(40);
                loop {
                    let ms: Vec<Move> = e.moves(&c);
                    if ms.is_empty() {
                        break;
                    }
                    let v = c.view();
                    let b = feature::briefed(&feature::board(&v), &w);
                    let at = ms
                        .iter()
                        .map(|m| match m {
                            Move::Press(verb) => {
                                net.q(&feature::pair(&b, &feature::mv(&v, *verb)))
                            }
                            Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                        })
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                        .map(|(i, _)| i)
                        .expect("not empty");
                    e.step(&mut c, ms[at]);
                    if e.finished {
                        break;
                    }
                }
                let (stats, built) = c.board_for_scoring();
                items += built.len();
                score += scoring::score(stats, &built, rung, Judge::Rogue);
                n += 1;
            }
        }
        let n = n.max(1) as f32;
        println!("    {name:<22} {:>7.1} {:>9.2}", items as f32 / n, score / n);
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
            RoadStep::Pack => {
                let before = c.clone();
                packer.pack(&mut c, 40);
                w.packed(&before, &c);
            }
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
