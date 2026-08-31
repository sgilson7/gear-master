//! Four models, two modes, and whether either pair is actually a pair.
//!
//!     cargo run --release -p gearmaster-lab --bin qcross
//!
//! R5's gate, and the only thing that makes a Rogue quartermaster and a Rogue
//! pathfinder **different agents** rather than two files with different names:
//! each mode's pair has to beat the other mode's pair *in its own mode*.
//!
//! The table is deliberately wider than the gate. Two controls sit in it - the
//! written pilot, which is what every benchmark in `analysis/the-two-trades.md`
//! was measured against, and a road policy with no weights at all - because a
//! learned pair that loses to both is not a pair with a mode problem, it is a
//! pair that has not learned anything, and a table without those rows cannot
//! tell the two apart.
//!
//! Each cell is the mean best rung over `QCROSS_RUNS` seeds. A Rogue run that
//! dies is over, so its cell is what it reached before it died.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::packers::Packer;
use gearmaster_trades::env::{Step as RoadStep, Walking};
use gearmaster_trades::pathfinder;
use gearmaster_trades::QNet;

const BUDGET: usize = 320;

/// A pair: who walks the road, and who packs the board.
struct Pair {
    what: String,
    road: Option<QNet>,
    /// The pilot plays both halves itself, so it is a row rather than a pair.
    written: bool,
    packer: Packer,
}

fn main() {
    let runs: usize = std::env::var("QCROSS_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    // **Rogue by default, and Grinder only when asked.**
    //
    // Grinder is designed to always be possible: a loss costs a rung and still
    // pays a bounty, so a run farms its way past anything it can eventually
    // beat. That makes it the uninteresting half of the solving problem, and
    // it is also the expensive half - a Grinder episode runs to the decision
    // budget where a Rogue one ends at the wipe, which is about 227 decisions
    // against 14.
    let modes: Vec<Mode> = match std::env::var("QCROSS_MODES").as_deref() {
        Ok("both") => vec![Mode::Grinder, Mode::Rogue],
        Ok("grinder") => vec![Mode::Grinder],
        _ => vec![Mode::Rogue],
    };
    let net = |p: &str| QNet::load(p);

    // **Two rows a model, and the reason is §C1.** A pair is a road policy and
    // a packer, and if the packer cannot clear rung three then the pair's
    // number is the packer's and the road policy is unmeasured. The first
    // version of this table had one row a mode, both halves learned, and
    // returned rung 1.0 for everything - which read as "the pathfinders learned
    // nothing" and was really "the packers cannot walk".
    //
    // So each learned road policy appears twice: once behind the written
    // control, which isolates it, and once behind its own mode's packer, which
    // is the pair the mission asked for.
    let q = |m: &str| format!("analysis/nets/quartermaster_{m}.txt");
    let pairs = vec![
        Pair {
            what: "the written pilot".into(),
            road: None,
            written: true,
            packer: Packer::named("control"),
        },
        Pair {
            what: "no weights + control packer".into(),
            road: None,
            written: false,
            packer: Packer::named("control"),
        },
        Pair {
            what: "grinder road + control packer".into(),
            road: net("analysis/nets/pathfinder-grinder.txt"),
            written: false,
            packer: Packer::named("control"),
        },
        Pair {
            what: "rogue road + control packer".into(),
            road: net("analysis/nets/pathfinder-rogue.txt"),
            written: false,
            packer: Packer::named("control"),
        },
        Pair {
            what: "grinder pair".into(),
            road: net("analysis/nets/pathfinder-grinder.txt"),
            written: false,
            packer: Packer::named(&q("grinder")),
        },
        Pair {
            what: "rogue pair".into(),
            road: net("analysis/nets/pathfinder-rogue.txt"),
            written: false,
            packer: Packer::named(&q("rogue")),
        },
    ];

    // **What actually loaded.** `Packer::named` on a path that does not load is
    // a packer that seats nothing, and a table that does not say so is a table
    // about nothing. Same for a road net.
    println!("what each row is actually running:");
    for p in &pairs {
        println!(
            "  {:<32} road: {:<28} packer: {}",
            p.what,
            if p.written {
                "the pilot's own".to_string()
            } else {
                match &p.road {
                    Some(_) => "trained, loaded".into(),
                    None => "none - first legal step".into(),
                }
            },
            p.packer.describe("(as named above)")
        );
    }
    println!();

    println!("Mean best rung over {runs} seeds.\n");
    print!("  {:<32}", "pair");
    for m in &modes {
        print!(" {:>10}", format!("{m:?}").to_lowercase());
    }
    println!();
    let mut table: Vec<(String, Vec<f64>)> = Vec::new();
    for p in &pairs {
        let cells: Vec<f64> = modes.iter().map(|m| mean_best(p, *m, runs)).collect();
        print!("  {:<32}", p.what);
        for c in &cells {
            print!(" {:>10.1}", c);
        }
        println!();
        table.push((p.what.clone(), cells));
    }

    // What the learned road policy is worth against knowing nothing, in each
    // mode that was run. The cross-mode gate belongs to `QCROSS_MODES=both`.
    let by = |name: &str| table.iter().find(|(w, _)| w == name).cloned();
    let floor = by("no weights + control packer");
    println!("\n  what the weights are worth, against no weights at all:");
    for (name, cells) in table.iter().filter(|(w, _)| w.contains("road +") || w.contains("pair")) {
        print!("  {name:<32}");
        for (k, c) in cells.iter().enumerate() {
            let f = floor.as_ref().map(|(_, v)| v[k]).unwrap_or(0.0);
            print!(" {:>+10.1}", c - f);
        }
        println!();
    }
    println!(
        "\n  A row that is not ahead of the floor has not learned to walk a road,\n           and the mode it was trained for is not what is being measured."
    );
}

fn mean_best(p: &Pair, mode: Mode, runs: usize) -> f64 {
    let mut r = Rng::new(0xC205_5EED);
    let mut total = 0usize;
    for _ in 0..runs {
        let seed = r.next_u64();
        total += if p.written {
            let d = Doctrine { patience: 12, budget: 400_000, coverage: 0.0 };
            pilot::play(seed, mode, Difficulty::Medium, d).best_rung
        } else {
            walk(p, seed, mode)
        };
    }
    total as f64 / runs as f64
}

/// One run, the pair deciding.
fn walk(p: &Pair, seed: u64, mode: Mode) -> usize {
    let mut c = Console::start(seed, mode, Difficulty::Medium);
    let mut w = Walking::new(None, BUDGET);
    let mut best = 1usize;
    loop {
        let ms = w.moves(&c);
        if ms.is_empty() || w.steps >= BUDGET {
            break;
        }
        let v = c.view();
        let at = match &p.road {
            Some(n) => {
                let road = pathfinder::road(&v, None);
                ms.iter()
                    .map(|s| n.q_pair(&pathfinder::pair(&road, &pathfinder::describe(&v, s))))
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real numbers"))
                    .map(|(i, _)| i)
                    .expect("the list is not empty")
            }
            None => 0,
        };
        match &ms[at] {
            RoadStep::Pack => {
                let before = c.clone();
                p.packer.pack(&mut c, 40);
                w.packed(&before, &c);
            }
            RoadStep::Press(verb) => {
                if !c.apply(*verb).ok {
                    break;
                }
            }
        }
        w.steps += 1;
        let v = c.view();
        best = best.max(v.rung_shown);
        // A Rogue run out of lives is replaced rather than ended, so nothing
        // else here can tell. The run being measured is over.
        if v.wiped {
            break;
        }
    }
    best
}
