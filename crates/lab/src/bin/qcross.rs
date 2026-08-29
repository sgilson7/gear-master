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

    println!("Mean best rung over {runs} seeds, each pair in each mode.\n");
    println!("  {:<30} {:>10} {:>10}", "pair", "grinder", "rogue");
    let mut table: Vec<(String, f64, f64)> = Vec::new();
    for p in &pairs {
        let g = mean_best(p, Mode::Grinder, runs);
        let r = mean_best(p, Mode::Rogue, runs);
        println!("  {:<30} {:>10.1} {:>10.1}", p.what, g, r);
        table.push((p.what.clone(), g, r));
    }

    // The gate, stated rather than implied.
    let by = |name: &str| table.iter().find(|(w, _, _)| w == name).cloned();
    // The gate is about the road policies, so it is read off the rows where
    // the packer is held constant.
    let (Some(g), Some(r)) =
        (by("grinder road + control packer"), by("rogue road + control packer"))
    else {
        return;
    };
    println!("\n  the gate: each road policy ahead of the other in its own mode,");
    println!("  with the same packer behind both");
    println!(
        "    in grinder  {:>6.1} against {:>6.1}   {}",
        g.1,
        r.1,
        if g.1 > r.1 { "met" } else { "MISSED" }
    );
    println!(
        "    in rogue    {:>6.1} against {:>6.1}   {}",
        r.2,
        g.2,
        if r.2 > g.2 { "met" } else { "MISSED" }
    );
    let control = by("no weights + control packer").map(|c| (c.1, c.2));
    if let Some((cg, cr)) = control {
        println!(
            "\n  and against no weights at all: grinder {:+.1}, rogue {:+.1}.\n  \
             A pair that is not ahead of that has not learned to walk a road, and \n  \
             the mode it was trained for is not what is being measured.",
            g.1 - cg,
            r.2 - cr
        );
    }
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
            RoadStep::Pack => p.packer.pack(&mut c, 40),
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
