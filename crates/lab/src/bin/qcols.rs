//! Where a trained net's weight went, and which of its verbs it can tell apart.
//!
//!     cargo run --release -p gearmaster-lab --bin qcols
//!
//! `qmind` prints a layer's mean, spread and largest, which answers "did
//! anything happen" and nothing after it. The question after it is **what the
//! network is looking at**, and for this architecture that is a question about
//! `w1`: the net scores a state-action pair of `feature::PAIR` numbers, so row
//! `i` of `w1` is every weight that input `i` can reach, and its size is how
//! much of the first layer that input can move.
//!
//! The bands come out of `feature` rather than out of this file, because a
//! diagnostic that hard-codes an offset drifts the day one moves - which is the
//! fault that made every checkpoint in the repo unreadable for two commits
//! (`analysis/the-collapse.md` M0.1).
//!
//! ## And why the weights are only half of it
//!
//! A row norm says what an input *can* move and not what it *does*, because the
//! inputs are not on the same scale and most of them are zero most of the time:
//! a move's kind is one-hot over thirteen, so twelve of those rows contribute
//! nothing to any given pair. So the second half of this report is behavioural -
//! real menus at real rungs, scored - and it prints per verb kind what the net
//! thinks that kind is worth, how often it is offered and how often it wins.
//!
//! `QCOLS_NET=<path>` looks at one net; with nothing set it walks the shelf.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::{packers, row};
use gearmaster_trades::brief::{Brief, BRIEF};
use gearmaster_trades::env::Move;
use gearmaster_trades::{feature, QNet};

const ROW_SEED: u64 = 0x0D0E_5EED;
const PACK_BUDGET: usize = 40;

/// The bands of the pair, named, in the order `feature::pair` lays them out.
///
/// Read off the constants so this cannot drift from the vector it describes.
fn bands() -> Vec<(&'static str, usize, usize)> {
    let b = feature::BOARD;
    let m = b + BRIEF;
    vec![
        ("pools, the four totals", 0, 4),
        ("pools, per resource", 4, 16),
        ("what the board is", 16, 24),
        ("purse, tray, what is coming", 24, 28),
        ("the rung, and the lives left", 28, 30),
        ("the layout, 5 grids x 48 cells", 30, b),
        ("the brief", b, m),
        ("the move's kind, one-hot", m, m + feature::KINDS),
        ("the piece it is about", m + feature::PIECE, m + feature::WHERE),
        ("where it goes", m + feature::WHERE, m + feature::LOCK),
        ("what locking would fix", m + feature::LOCK, m + feature::MOVE),
    ]
}

/// The verb kinds, in the order `feature::mv` makes them one-hot.
///
/// Kept beside the match it mirrors; the length is asserted against
/// `feature::KINDS` so an added kind cannot go unnamed.
const KIND_NAMES: [&str; 13] = [
    "place", "buy", "sell", "barter", "reroll", "rotate", "unequip", "clear", "lock", "grow",
    "undo", "pin", "other",
];

fn kind_of(m: Verb) -> usize {
    match m {
        Verb::Place { .. } => 0,
        Verb::Buy { .. } => 1,
        Verb::Sell { .. } => 2,
        Verb::Barter { .. } => 3,
        Verb::Reroll => 4,
        Verb::Rotate { .. } | Verb::RotateLocked { .. } => 5,
        Verb::Unequip { .. } | Verb::UnequipLocked { .. } => 6,
        Verb::ClearSlot { .. } | Verb::ClearAll => 7,
        Verb::Lock { .. } => 8,
        Verb::Grow { .. } => 9,
        Verb::Undo => 10,
        Verb::Pin { .. } => 11,
        _ => 12,
    }
}

/// The root-mean-square of one row of `w1`, which is what one input can move.
fn row_rms(net: &QNet, i: usize, hidden: usize, w1: &[f32]) -> f32 {
    let _ = net;
    let row = &w1[i * hidden..(i + 1) * hidden];
    (row.iter().map(|x| x * x).sum::<f32>() / hidden as f32).sqrt()
}

/// What a freshly drawn row of `w1` is, as a root-mean-square.
///
/// `init` draws uniform on `+/- sqrt(2/fan_in)`, whose rms is `a/sqrt(3)`. Every
/// row is drawn from the same distribution, so this is the line every band
/// below is being compared against and a band at 1.00x has not been learned.
fn init_rms(fan_in: usize) -> f32 {
    (2.0 / fan_in as f32).sqrt() / 3f32.sqrt()
}

fn weights(net: &QNet) {
    let layers = net.layers();
    let (_, w1, fan_in) = layers[0];
    let hidden = layers[1].1.len();
    let want = init_rms(fan_in);
    println!("\n  what the first layer looks at, by band of the pair");
    println!("    {:<32} {:>6} {:>10} {:>10}", "band", "cols", "rms", "vs init");
    let mut all: Vec<(f32, usize)> = Vec::new();
    for (name, from, to) in bands() {
        let to = to.min(fan_in);
        if from >= to {
            continue;
        }
        let rows: Vec<f32> = (from..to).map(|i| row_rms(net, i, hidden, w1)).collect();
        let m = rows.iter().sum::<f32>() / rows.len() as f32;
        println!(
            "    {name:<32} {:>6} {:>10.5} {:>9.2}x",
            to - from,
            m,
            m / want
        );
        for (k, r) in rows.iter().enumerate() {
            all.push((*r, from + k));
        }
    }
    all.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("real"));
    let name_of = |i: usize| -> String {
        for (name, from, to) in bands() {
            if i >= from && i < to {
                if name.contains("one-hot") {
                    return format!("{}: {}", name, KIND_NAMES[i - from]);
                }
                return format!("{name} [+{}]", i - from);
            }
        }
        "past the end".into()
    };
    println!("\n    the ten single inputs with the most weight on them");
    for (r, i) in all.iter().take(10) {
        println!("      {:>4}  {:>9.5}  {:>6.2}x  {}", i, r, r / want, name_of(*i));
    }
}

/// What moved between two checkpoints of the same run, band by band.
///
/// **The honest version of the column above.** An rms at 1.00x of its
/// initialisation is evidence that a band was never learned and not proof of
/// it: weights can move and keep their spread. Two checkpoints out of one
/// training run cannot - if a band is identical in both, nothing wrote to it in
/// between, whatever its spread says.
fn moved(a: &QNet, b: &QNet) {
    let (la, lb) = (a.layers(), b.layers());
    let (w1a, w1b) = (la[0].1, lb[0].1);
    let hidden = la[1].1.len();
    if w1a.len() != w1b.len() {
        println!("\n  the two nets are different widths and cannot be differenced");
        return;
    }
    // **And whether it moved in a direction.** A band being written to is not
    // a band being fitted: gradient with no consistent sign is a random walk,
    // and a random walk ends up distributed exactly like the draw it started
    // from - which is what an rms of 1.00x its initialisation looks like from
    // the outside. A weight that kept its sign across a thousand updates was
    // pushed; one that did not was jostled. Half is a coin.
    println!("\n  what moved between the two checkpoints, by band");
    println!(
        "    {:<32} {:>6} {:>12} {:>12} {:>10}",
        "band", "cols", "mean |d|", "as % of rms", "sign kept"
    );
    for (name, from, to) in bands() {
        let to = to.min(la[0].2);
        if from >= to {
            continue;
        }
        let (mut d, mut mag, mut n, mut kept) = (0.0f64, 0.0f64, 0usize, 0usize);
        for i in from..to {
            for j in 0..hidden {
                let k = i * hidden + j;
                d += (w1a[k] - w1b[k]).abs() as f64;
                mag += w1a[k].abs() as f64;
                if w1a[k].signum() == w1b[k].signum() {
                    kept += 1;
                }
                n += 1;
            }
        }
        let (d, mag) = (d / n as f64, mag / n as f64);
        println!(
            "    {name:<32} {:>6} {:>12.6} {:>11.2}% {:>9.0}%",
            to - from,
            d,
            100.0 * d / mag.max(1e-9),
            100.0 * kept as f32 / n as f32
        );
    }
}

/// What the output actually moves when a band of the input is taken away.
///
/// **The measurement the two above are only evidence for.** A row of `w1` says
/// what an input *can* reach, and a difference between checkpoints says what was
/// written to it - neither says what the network's answer depends on, because
/// the inputs are not on one scale and two ReLUs sit between them and the
/// output. This zeroes one band at a time over real pairs from real runs and
/// reports how far `Q` moves.
///
/// Zeroing is the right ablation here because every feature in this vector is
/// already zero when it has nothing to say: an empty cell, a move that is about
/// no piece, a brief that asks for nothing. So a zeroed band is a legal input
/// and not an out-of-distribution one.
fn ablate(net: &QNet, pairs: &[[f32; feature::PAIR]]) {
    let base: Vec<f32> = pairs.iter().map(|p| net.q(p)).collect();
    let spread = {
        let m = base.iter().sum::<f32>() / base.len() as f32;
        (base.iter().map(|q| (q - m) * (q - m)).sum::<f32>() / base.len() as f32).sqrt()
    };
    println!(
        "\n  what the answer depends on: Q with one band zeroed, over {} real pairs",
        pairs.len()
    );
    println!("    across states, Q has a standard deviation of {spread:.3}");
    println!("    {:<32} {:>6} {:>12} {:>12}", "band zeroed", "cols", "mean |dQ|", "of that sd");
    for (name, from, to) in bands() {
        let mut d = 0.0f64;
        for (k, p) in pairs.iter().enumerate() {
            let mut q = *p;
            for i in from..to {
                q[i] = 0.0;
            }
            d += (net.q(&q) - base[k]).abs() as f64;
        }
        let d = d / pairs.len() as f64;
        println!(
            "    {name:<32} {:>6} {:>12.4} {:>11.0}%",
            to - from,
            d,
            100.0 * d / spread.max(1e-9) as f64
        );
    }
}

/// One decision, seen from the value function's side.
#[derive(Default, Clone)]
struct Seen {
    offered: usize,
    chosen: usize,
    q_sum: f64,
}

fn behaviour(net: &QNet, runs: usize) {
    let mut seeds = Rng::new(ROW_SEED);
    let mut kinds: Vec<Seen> = vec![Seen::default(); feature::KINDS + 1];
    // Per rung: how many decisions, and the spread of the menu at each.
    let mut by_rung: Vec<(usize, f64, f64)> = vec![(0, 0.0, 0.0); 32];
    let mut done_q = (0usize, 0.0f64);
    // Every twentieth pair the policy actually scored, for the ablation.
    let mut pairs: Vec<[f32; feature::PAIR]> = Vec::new();
    let mut nth = 0usize;

    for _ in 0..runs {
        let seed = seeds.next_u64();
        let kinds = &mut kinds;
        let by_rung = &mut by_rung;
        let done_q = &mut done_q;
        let pairs = &mut pairs;
        let nth = &mut nth;
        let mut pack = |c: &mut Console| -> Vec<Verb> {
            let pressed = row::pack_with(c, PACK_BUDGET, |c, ms| {
                let v = c.view();
                let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                let made: Vec<[f32; feature::PAIR]> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                        Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                    })
                    .collect();
                let scores: Vec<f32> = made.iter().map(|p| net.q(p)).collect();
                for p in &made {
                    *nth += 1;
                    if *nth % 500 == 0 && pairs.len() < 3000 {
                        pairs.push(*p);
                    }
                }
                let at = scores
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                    .map(|(i, _)| i)
                    .expect("not empty");
                for (i, m) in ms.iter().enumerate() {
                    let k = match m {
                        Move::Press(verb) => kind_of(*verb),
                        Move::Done => {
                            done_q.0 += 1;
                            done_q.1 += scores[i] as f64;
                            feature::KINDS
                        }
                    };
                    kinds[k].offered += 1;
                    kinds[k].q_sum += scores[i] as f64;
                    if i == at {
                        kinds[k].chosen += 1;
                    }
                }
                // **The spread across the menu at one state**, which is how
                // much the network thinks this decision matters. Reported by
                // rung, because whether a decision matters is exactly what is
                // in question further up the ladder.
                let hi = scores.iter().cloned().fold(f32::MIN, f32::max);
                let mean = scores.iter().sum::<f32>() / scores.len() as f32;
                let r = (v.rung_shown).min(by_rung.len() - 1);
                by_rung[r].0 += 1;
                by_rung[r].1 += (hi - mean) as f64;
                by_rung[r].2 += hi as f64;
                at
            });
            row::keys(&pressed)
        };
        row::run(seed, Mode::Rogue, Difficulty::Medium, &mut pack);
    }

    if !pairs.is_empty() {
        // **Did the split take?** A dueling net that put everything in `V` and
        // nothing in `A` is the same failure wearing a new architecture, and
        // the two magnitudes are the only thing that says which happened.
        let parts: Vec<(f32, f32)> = pairs.iter().filter_map(|p| net.parts(p)).collect();
        if !parts.is_empty() {
            let n = parts.len() as f64;
            let v = parts.iter().map(|p| p.0.abs() as f64).sum::<f64>() / n;
            let a = parts.iter().map(|p| p.1.abs() as f64).sum::<f64>() / n;
            println!(
                "\n  the split, over {} pairs: mean |V| {v:.4}   mean |A| {a:.4}   A is {:.1}% of the answer",
                parts.len(),
                100.0 * a / (v + a).max(1e-9)
            );
        }
        ablate(net, &pairs);
    }
    println!("\n  what it thinks each kind of key is worth, over {runs} runs");
    println!("    {:<10} {:>9} {:>9} {:>8} {:>10}", "kind", "offered", "chosen", "share", "mean Q");
    let total_chosen: usize = kinds.iter().map(|k| k.chosen).sum();
    for (i, s) in kinds.iter().enumerate() {
        if s.offered == 0 {
            continue;
        }
        let name = if i == feature::KINDS { "done" } else { KIND_NAMES[i] };
        println!(
            "    {name:<10} {:>9} {:>9} {:>7.1}% {:>10.3}",
            s.offered,
            s.chosen,
            100.0 * s.chosen as f32 / total_chosen.max(1) as f32,
            s.q_sum / s.offered as f64
        );
    }

    println!("\n  and how much it thinks the decision matters, by rung");
    println!("    {:<6} {:>10} {:>12} {:>12}", "rung", "decisions", "max - mean", "max Q");
    for (r, (n, spread, hi)) in by_rung.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        println!(
            "    {r:<6} {:>10} {:>12.4} {:>12.3}",
            n,
            spread / *n as f64,
            hi / *n as f64
        );
    }
}

/// The action gap, at states this net could never walk to on its own.
///
/// **The gap and not the spread.** `Q(s,a*) - max Q(s,a)` over the rest is the
/// quantity the approximation-error argument is about: it is what a numerical
/// error has to exceed to reorder the greedy policy, and it is what the
/// gap-increasing operators exist to widen. A spread against the mean flatters
/// it, because most of a 700-verb menu is nothing like the best move.
///
/// And the states are the **written control's**, not the net's own. A packer
/// that dies at rung three cannot be asked what it thinks about rung twelve by
/// playing it there, and rung twelve is exactly where the question is - so the
/// control walks the ladder and the net is asked to score the menu it finds.
fn gap_by_rung(net: &QNet, runs: usize) {
    let mut seeds = Rng::new(ROW_SEED);
    // rung -> (states, sum gap, sum best, sum ties within 1%, sum menu size)
    let mut by: Vec<(usize, f64, f64, f64, f64)> = vec![(0, 0.0, 0.0, 0.0, 0.0); 32];
    for _ in 0..runs {
        let seed = seeds.next_u64();
        let by = &mut by;
        let mut pack = |c: &mut Console| -> Vec<Verb> {
            let v = c.view();
            let ms = gearmaster_trades::env::Packing::new(PACK_BUDGET).moves(c);
            if !ms.is_empty() {
                let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                let mut q: Vec<f32> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                        Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                    })
                    .collect();
                q.sort_by(|a, b| b.partial_cmp(a).expect("real"));
                let best = q[0];
                let gap = best - q.get(1).copied().unwrap_or(best);
                let ties = q.iter().filter(|x| (best - **x).abs() <= best.abs() * 0.01).count();
                let r = v.rung_shown.min(by.len() - 1);
                by[r].0 += 1;
                by[r].1 += gap as f64;
                by[r].2 += best as f64;
                by[r].3 += ties as f64;
                by[r].4 += q.len() as f64;
            }
            // The control walks; the net only ever watches.
            packers::control(c, PACK_BUDGET);
            Vec::new()
        };
        row::run(seed, Mode::Rogue, Difficulty::Medium, &mut pack);
    }
    println!("\n  the action gap at the written control's states, by rung");
    println!(
        "    {:<6} {:>7} {:>8} {:>10} {:>10} {:>10} {:>16}",
        "rung", "states", "menu", "best Q", "gap", "gap/best", "within 1% of best"
    );
    for (r, (n, gap, best, ties, menu)) in by.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        let n = *n as f64;
        println!(
            "    {r:<6} {:>7.0} {:>8.0} {:>10.3} {:>10.4} {:>9.2}% {:>15.0}",
            n,
            menu / n,
            best / n,
            gap / n,
            100.0 * (gap / n) / (best / n).abs().max(1e-9),
            ties / n
        );
    }
}

fn main() {
    assert_eq!(KIND_NAMES.len(), feature::KINDS, "a verb kind was added and not named here");
    let runs: usize = std::env::var("QCOLS_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let shelf: Vec<(String, String)> = match std::env::var("QCOLS_NET") {
        Ok(p) => vec![("the net asked for".into(), p)],
        Err(_) => [
            ("r18, squared, the baseline", "analysis/nets/qrow-r18-best.txt"),
            ("r23, cubed", "analysis/nets/qrow-r23-best.txt"),
            ("the last run's best block", "runs/quartermaster_row.txt"),
            ("the last run's final weights", "runs/quartermaster_row_last.txt"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect(),
    };

    println!(
        "\n  the pair is {} numbers: board {}, brief {}, move {}",
        feature::PAIR,
        feature::BOARD,
        BRIEF,
        feature::MOVE
    );
    for (what, path) in shelf {
        println!("\n================ {what}\n{path}");
        let net = match QNet::load_at(&path, feature::PAIR) {
            Ok(n) => n,
            Err(why) => {
                println!("  {why}");
                continue;
            }
        };
        weights(&net);
        if let Ok(other) = std::env::var("QCOLS_AGAINST") {
            match QNet::load_at(&other, feature::PAIR) {
                Ok(b) => {
                    println!("\n  against {other}");
                    moved(&net, &b);
                }
                Err(why) => println!("  {why}"),
            }
        }
        if std::env::var("QCOLS_WEIGHTS_ONLY").is_err() {
            behaviour(&net, runs);
            gap_by_rung(&net, runs);
        }
    }
    let _ = packers::control;
}
