//! What the revisit charge has to be worth to change a decision.
//!
//!     cargo run --release -p gearmaster-lab --bin qcharge
//!
//! `row::revisit` is 0.02 a press, sized in `analysis/the-action-gap.md` against
//! the **episode return** - the total charge came to about one assembled item,
//! which is the right way to size something that must not swamp the objective
//! and the wrong way to size something that must flip an argmax.
//!
//! A greedy policy presses `argmax Q`. For a charge to stop it revisiting, what
//! it has to overcome is the amount by which the revisiting move currently
//! *out-scores the best move that is not one*. That number has never been
//! measured. It is the quantity `CLAUDE.md` trap 44 is really about: *a no-op
//! cost 0.01 while the value estimates were spread over 1.70*.
//!
//! So this prints three gaps at real decisions under a real net:
//!
//! * `best - second`, the ordinary action gap, for reference;
//! * **`best - best clean`**, where clean means the press does not put the board
//!   somewhere this packing has already been. This is the one the charge has to
//!   close, and it is zero at every decision where the policy was going to do
//!   something productive anyway;
//! * `best - done`, because `Move::Done` is offered at every single decision and
//!   chosen on 0.0% to 0.9% of them, and it is the one press that is guaranteed
//!   to cost nothing further.
//!
//! Whether a candidate is a revisit is answered by **pressing it on a clone** and
//! fingerprinting the result, which is exact rather than inferred from the verb.
//! `Console` is `Clone` for this kind of question.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::row;
use gearmaster_trades::brief::Brief;
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::{feature, QNet};

const ROW_SEED: u64 = 0x0D0E_5EED;
const PACK_BUDGET: usize = 40;

/// One decision, taken apart.
struct Seen {
    /// The ordinary action gap: best minus the next best, whatever it is.
    gap: f32,
    /// Best minus the best move that does not revisit. Zero when the best move
    /// already does not.
    to_clean: f32,
    /// Best minus `Done`.
    to_done: f32,
    /// Whether the policy's own choice was a revisit.
    chose_revisit: bool,
    /// Whether any non-revisiting press was available at all.
    had_clean: bool,
    /// How much of the menu revisits.
    menu: usize,
    revisiting: usize,
}

fn play(seed: u64, net: &QNet, out: &mut Vec<Seen>) {
    let mut pack = |c: &mut Console| -> Vec<Verb> {
        let mut e = Packing::new(PACK_BUDGET);
        let mut seen: Vec<u64> = vec![row::fingerprint(c)];
        loop {
            let ms: Vec<Move> = e
                .moves(c)
                .into_iter()
                .filter(|m| {
                    !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. }))
                })
                .collect();
            if ms.is_empty() {
                break;
            }
            let v = c.view();
            let b = feature::briefed(&feature::board(&v), &Brief::NONE);
            let pairs: Vec<[f32; feature::PAIR]> = ms
                .iter()
                .map(|m| match m {
                    Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                    Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                })
                .collect();
            let qs = net.q_set(&pairs);

            // **Pressed on a clone, so "would this revisit" is a fact.** A verb
            // does not say where it lands; `Undo` on the first press of a
            // packing goes somewhere new, and a placement can complete a cycle.
            let revisits: Vec<bool> = ms
                .iter()
                .map(|m| match m {
                    // `Done` leaves the board where it is and is never a
                    // revisit - see `row::revisit_penalty`.
                    Move::Done => false,
                    Move::Press(verb) => {
                        let mut probe = c.clone();
                        if !probe.apply(*verb).ok {
                            return false;
                        }
                        seen.contains(&row::fingerprint(&probe))
                    }
                })
                .collect();

            let best = qs
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                .map(|(i, _)| i)
                .expect("not empty");
            let mut rest: Vec<f32> =
                qs.iter().enumerate().filter(|(i, _)| *i != best).map(|(_, q)| *q).collect();
            rest.sort_by(|a, b| b.partial_cmp(a).expect("real"));
            let second = rest.first().copied().unwrap_or(qs[best]);
            let clean = qs
                .iter()
                .enumerate()
                .filter(|(i, _)| !revisits[*i])
                .map(|(_, q)| *q)
                .fold(f32::MIN, f32::max);
            let done = ms
                .iter()
                .position(|m| matches!(m, Move::Done))
                .map(|i| qs[i])
                .unwrap_or(f32::MIN);

            out.push(Seen {
                gap: qs[best] - second,
                to_clean: if clean == f32::MIN { 0.0 } else { (qs[best] - clean).max(0.0) },
                to_done: qs[best] - done,
                chose_revisit: revisits[best],
                had_clean: clean != f32::MIN,
                menu: ms.len(),
                revisiting: revisits.iter().filter(|r| **r).count(),
            });

            e.step(c, ms[best]);
            // **Into the set, which the first version never did.** `seen` held
            // only the packing's opening board, so "revisit" meant "back to
            // where this packing started" and read as 1% of the menu against
            // the trainer's 74%. `rustc` said `variable does not need to be
            // mutable` about it and that was the whole diagnosis, printed, and
            // read past.
            seen.push(row::fingerprint(c));
            if e.finished {
                break;
            }
        }
        Vec::new()
    };
    let _ = row::run(seed, Mode::Rogue, Difficulty::Medium, &mut pack);
}

fn stats(v: &[f32]) -> (f32, f32, f32) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).expect("real"));
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    (mean, s[s.len() / 2], s[(s.len() * 9 / 10).min(s.len() - 1)])
}

fn main() {
    let runs: usize =
        std::env::var("QCHARGE_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let shelf: Vec<String> = match std::env::var("QCHARGE_NETS") {
        Ok(v) => v.split(',').map(|s| s.trim().to_string()).collect(),
        Err(_) => ["runs/rv-control.txt", "runs/rv-abstract.txt"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };
    let charge = row::revisit();
    println!(
        "\n  The revisit charge is {charge} a press. What it has to overcome is how far\n  \
         the policy's chosen revisit out-scores the best press that is not one -\n  \
         `best - best clean` below. Greedy, {runs} runs a net, revisits decided by\n  \
         pressing each candidate on a clone."
    );

    for path in &shelf {
        let net = match QNet::load_at(path, feature::PAIR) {
            Ok(n) => n,
            Err(why) => {
                println!("\n  {why}");
                continue;
            }
        };
        let mut out: Vec<Seen> = Vec::new();
        let mut seeds = Rng::new(ROW_SEED);
        for _ in 0..runs {
            play(seeds.next_u64(), &net, &mut out);
        }
        if out.is_empty() {
            continue;
        }
        let n = out.len();
        let chose = out.iter().filter(|s| s.chose_revisit).count();
        // The gap only has to be closed where the policy actually chose a
        // revisit *and* had something else to press.
        let live: Vec<&Seen> = out.iter().filter(|s| s.chose_revisit && s.had_clean).collect();
        let (m, med, p90) = stats(&live.iter().map(|s| s.to_clean).collect::<Vec<_>>());
        let (gm, gmed, _) = stats(&out.iter().map(|s| s.gap).collect::<Vec<_>>());
        let (dm, dmed, _) = stats(&out.iter().map(|s| s.to_done).collect::<Vec<_>>());
        let menu = out.iter().map(|s| s.menu).sum::<usize>() as f32 / n as f32;
        let rev = out.iter().map(|s| s.revisiting).sum::<usize>() as f32
            / out.iter().map(|s| s.menu).sum::<usize>().max(1) as f32;

        println!("\n================ {path}");
        println!("  {n} decisions, menu {menu:.0} keys of which {:.0}% revisit", 100.0 * rev);
        println!(
            "  it chose a revisit on {:.0}% of them, and had a clean press available on {:.0}%",
            100.0 * chose as f32 / n as f32,
            100.0 * out.iter().filter(|s| s.had_clean).count() as f32 / n as f32
        );
        println!("\n    {:<34} {:>9} {:>9} {:>9}", "", "mean", "median", "90th");
        println!("    {:<34} {:>9.4} {:>9.4} {:>9}", "best - second (the action gap)", gm, gmed, "-");
        println!("    {:<34} {:>9.4} {:>9.4} {:>9.4}", "best - best clean, where it chose one", m, med, p90);
        println!("    {:<34} {:>9.4} {:>9.4} {:>9}", "best - done", dm, dmed, "-");
        println!(
            "\n    the charge is {charge}, which is {:.2}x the median gap it has to close\n    \
             and {:.2}x the 90th percentile of it",
            if med > 0.0 { charge / med } else { f32::INFINITY },
            if p90 > 0.0 { charge / p90 } else { f32::INFINITY }
        );
        // **And what one press of charge is worth to a value function.** The
        // charge does not act on this comparison directly; it acts by lowering
        // the target for a revisiting pair, and how far it lowers it depends on
        // how many further revisits follow. Bounded above by the whole packing.
        println!(
            "    a whole packing of revisits is worth {:.3} of charge, against a gap of {:.4}",
            charge * PACK_BUDGET as f32 * rev,
            med
        );
    }
}
