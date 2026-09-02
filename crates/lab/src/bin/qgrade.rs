//! Grade a packer on the board it builds, in rungs, instead of on where the run
//! happened to stop.
//!
//!     cargo run --release -p gearmaster-lab --bin qgrade
//!
//! Five arms of `analysis/the-action-gap.md` were graded on the depth a run
//! reached, and J.4 is the conclusion that came out of it: **depth is not a
//! sensitive instrument for packing quality.** A.1 measured 78% of rungs as
//! coming out the same however they were packed, so a run's final rung is a
//! board-quality measurement taken once, through nine-tenths of noise, per two
//! hours of training.
//!
//! ## The instrument, which was already here
//!
//! `scoring::reach` asks the question directly: **how many consecutive rungs
//! does this board clear from where it stands?** It is a pure function of the
//! board and the rung - `combat.rs` consults no randomness - and it is measured
//! in rungs, which is the point. Depth is not being replaced by a proxy; it is
//! being *asked of every board* rather than once of a whole run.
//!
//! You do not reach a deeper rung unless the board is good enough, and that
//! implication is exactly what `reach` evaluates: it walks the ladder ahead of
//! the board and stops at the first fight the board loses, which is where a
//! Rogue run would stop.
//!
//! ## Why it is a hundred times the measurement for the same compute
//!
//! A run gives one depth. The same run gives a `reach` at **every packing** -
//! ten or twenty per run - and each one is a fact rather than a draw, because
//! the fight is deterministic. Ten runs here carry more signal about a packer
//! than three thousand training episodes graded on their final rung.
//!
//! ## And the confound, which is printed rather than hidden
//!
//! `reach` is measured where the board stands, so a board at rung 2 is being
//! asked to beat rungs 2 to 12 and one at rung 12 to beat 12 to 22. Those are
//! not the same question, and a packer that never gets past rung 3 is graded
//! entirely on easy creatures. So the table bands by rung, and the second half
//! of this binary re-packs **the same situations** with every packer, which
//! removes the confound entirely at the cost of no longer being the agent's own
//! economy.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::{curriculum, packers, row, scoring};
use gearmaster_trades::brief::Brief;
use gearmaster_trades::env::Move;
use gearmaster_trades::{feature, QNet};

const ROW_SEED: u64 = 0x0D0E_5EED;
const PACK_BUDGET: usize = 40;

/// What one packing produced.
struct Board {
    rung: usize,
    /// Consecutive rungs this board clears from here, capped by `REACH_CAP`.
    reach: usize,
    /// What that is worth given how deep it already is.
    gain: f32,
    items: usize,
}

fn grade(c: &Console) -> (usize, usize, f32) {
    let rung = c.view().rung_shown;
    let (stats, items) = c.board_for_scoring();
    // `reach` counts from a zero-based rung, the way `LADDER` is indexed.
    let at = rung.saturating_sub(1);
    let r = scoring::reach(stats, &items, at);
    (r, items.len(), scoring::depth_gain(at, r))
}

/// Play a run under a packer, grading the board at every packing.
fn play(seed: u64, net: Option<&QNet>, out: &mut Vec<Board>) -> usize {
    let mut pack = |c: &mut Console| -> Vec<Verb> {
        match net {
            None => {
                packers::control(c, PACK_BUDGET);
            }
            Some(net) => {
                row::pack_with(c, PACK_BUDGET, |c, ms| {
                    let v = c.view();
                    let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                    let pairs: Vec<[f32; feature::PAIR]> = ms
                        .iter()
                        .map(|m| match m {
                            Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                            Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                        })
                        .collect();
                    net.q_set(&pairs)
                        .into_iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                        .map(|(i, _)| i)
                        .expect("not empty")
                });
            }
        }
        // **After the packing and before the fight**, which is the board the
        // run is actually about to take in.
        let (reach, items, gain) = grade(c);
        out.push(Board { rung: c.view().rung_shown, reach, gain, items });
        Vec::new()
    };
    row::run(seed, Mode::Rogue, Difficulty::Medium, &mut pack).1.deepest
}

/// **Is `Lock` a no-op key?**
///
/// `Verb::Lock` is a *toggle* - `Console::apply` calls `toggle_lock_item` and
/// answers "locked" or "unlocked" - and `menu` offers it for every assembled
/// item whether it is locked or not. So pressing it twice on the same item
/// returns the board to where it was, for two presses of the forty, which is
/// exactly the shape `CLAUDE.md` trap 44 has caught three times already:
/// `Rotate`, then `Pin`, then `Undo`.
///
/// I.4 read a packer pressing `lock` on 38.6% of its choices as the first
/// policy to hold a multi-item board. This counts how many of those presses
/// *unlocked* something, which is the difference between that reading and trap
/// 44 a fourth time.
///
/// Driven through `Packing` rather than `row::pack_with`, because the chooser
/// is called *before* its press lands and cannot see what the press did.
fn lock_audit(seed: u64, net: Option<&QNet>, runs: usize) -> (usize, usize) {
    let (mut locked, mut unlocked) = (0usize, 0usize);
    let mut seeds = Rng::new(seed);
    for _ in 0..runs {
        let mut pack = |c: &mut Console| -> Vec<Verb> {
            let Some(net) = net else {
                packers::control(c, PACK_BUDGET);
                return Vec::new();
            };
            let held = |c: &Console| -> usize {
                c.view().grids.iter().flat_map(|g| g.items.iter()).filter(|i| i.locked).count()
            };
            let mut e = gearmaster_trades::env::Packing::new(PACK_BUDGET);
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
                let at = net
                    .q_set(&pairs)
                    .into_iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                    .map(|(i, _)| i)
                    .expect("not empty");
                let is_lock = matches!(ms[at], Move::Press(Verb::Lock { .. }));
                let before = if is_lock { held(c) } else { 0 };
                e.step(c, ms[at]);
                if is_lock {
                    // One more item locked, or one fewer: the toggle's two faces.
                    match held(c).cmp(&before) {
                        std::cmp::Ordering::Greater => locked += 1,
                        std::cmp::Ordering::Less => unlocked += 1,
                        std::cmp::Ordering::Equal => {}
                    }
                }
                if e.finished {
                    break;
                }
            }
            Vec::new()
        };
        let _ = row::run(seeds.next_u64(), Mode::Rogue, Difficulty::Medium, &mut pack);
    }
    (locked, unlocked)
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}
fn sd(v: &[f32]) -> f32 {
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / v.len().max(1) as f32).sqrt()
}

fn main() {
    let runs: usize =
        std::env::var("QGRADE_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let shelf: Vec<String> = match std::env::var("QGRADE_NETS") {
        Ok(v) => v.split(',').map(|s| s.trim().to_string()).collect(),
        Err(_) => ["control", "runs/duel-control.txt", "runs/redist.txt", "runs/abstract.txt"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };

    println!(
        "\n  Grading the board rather than the run: `scoring::reach` is how many\n  \
         consecutive rungs the board clears from where it stands, and the fight is\n  \
         a pure function of two boards - so every figure below is a fact, and there\n  \
         is one per packing rather than one per run.\n\n  \
         {runs} runs a packer, Rogue, Medium, greedy."
    );

    println!(
        "\n  {:<26} {:>8} {:>10} {:>8} {:>10} {:>8} {:>8}",
        "packer", "boards", "reach", "sd", "gain", "items", "depth"
    );
    let mut kept: Vec<(String, Vec<Board>)> = Vec::new();
    for path in &shelf {
        let net = if path == "control" {
            None
        } else {
            match QNet::load_at(path, feature::PAIR) {
                Ok(n) => Some(n),
                Err(why) => {
                    println!("  {why}");
                    continue;
                }
            }
        };
        let mut boards: Vec<Board> = Vec::new();
        let mut depths: Vec<f32> = Vec::new();
        let mut seeds = Rng::new(ROW_SEED);
        for _ in 0..runs {
            depths.push(play(seeds.next_u64(), net.as_ref(), &mut boards) as f32);
        }
        let r: Vec<f32> = boards.iter().map(|b| b.reach as f32).collect();
        let g: Vec<f32> = boards.iter().map(|b| b.gain).collect();
        let it: Vec<f32> = boards.iter().map(|b| b.items as f32).collect();
        println!(
            "  {:<26} {:>8} {:>10.3} {:>8.3} {:>10.4} {:>8.2} {:>8.2}",
            path.rsplit('/').next().unwrap_or(path),
            boards.len(),
            mean(&r),
            sd(&r) / (r.len().max(1) as f32).sqrt(),
            mean(&g),
            mean(&it),
            mean(&depths)
        );
        kept.push((path.clone(), boards));
    }

    // **What `lock` is actually doing.** See `lock_audit`.
    if std::env::var("QGRADE_LOCKS").is_ok() {
        println!("\n  what the `lock` key does when it is pressed");
        println!("  {:<26} {:>9} {:>10} {:>12}", "packer", "locked", "unlocked", "unlock share");
        for path in &shelf {
            if path == "control" {
                continue;
            }
            let Ok(n) = QNet::load_at(path, feature::PAIR) else { continue };
            let (l, u) = lock_audit(ROW_SEED, Some(&n), runs.min(10));
            println!(
                "  {:<26} {l:>9} {u:>10} {:>11.0}%",
                path.rsplit('/').next().unwrap_or(path),
                100.0 * u as f32 / (l + u).max(1) as f32
            );
        }
    }

    // **The confound, printed.** `reach` at rung 2 and `reach` at rung 12 are
    // not the same question, and a packer that never leaves the shallow end is
    // graded entirely on easy creatures.
    println!("\n  mean reach by the rung it was measured at");
    print!("  {:<26}", "packer");
    for band in [1usize, 2, 3, 4, 6, 9, 13] {
        print!(" {:>7}", format!("r{band}+"));
    }
    println!();
    for (path, boards) in &kept {
        print!("  {:<26}", path.rsplit('/').next().unwrap_or(path));
        let bands = [1usize, 2, 3, 4, 6, 9, 13, 99];
        for w in bands.windows(2) {
            let v: Vec<f32> = boards
                .iter()
                .filter(|b| b.rung >= w[0] && b.rung < w[1])
                .map(|b| b.reach as f32)
                .collect();
            if v.is_empty() {
                print!(" {:>7}", "-");
            } else {
                print!(" {:>7.2}", mean(&v));
            }
        }
        println!();
    }

    // **The same situations, packed by everybody.** No economy of its own, so
    // the rung a board is graded at is the same for every packer and the
    // confound above is gone.
    if std::env::var("QGRADE_SAME").is_ok() {
        println!("\n  the same situations, re-packed by each");
        println!("  {:<26} {:>8} {:>10} {:>8}", "packer", "boards", "reach", "sd");
        let situations: Vec<(u64, usize)> = [0x1212u64, 0xAA8D95DE31880461, 0xF1418AF3EDF965FD]
            .iter()
            .flat_map(|&s| [0usize, 4, 9, 14].iter().map(move |&r| (s, r)))
            .collect();
        for path in &shelf {
            let net = if path == "control" {
                None
            } else {
                match QNet::load_at(path, feature::PAIR) {
                    Ok(n) => Some(n),
                    Err(_) => continue,
                }
            };
            let mut r: Vec<f32> = Vec::new();
            for &(seed, rung) in &situations {
                let (mut c, walked) =
                    curriculum::repack_at(seed, Mode::Rogue, Difficulty::Medium, rung);
                if !walked.arrived {
                    continue;
                }
                let mut sink: Vec<Board> = Vec::new();
                match net.as_ref() {
                    None => packers::control(&mut c, PACK_BUDGET),
                    Some(n) => {
                        row::pack_with(&mut c, PACK_BUDGET, |c, ms| {
                            let v = c.view();
                            let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                            let pairs: Vec<[f32; feature::PAIR]> = ms
                                .iter()
                                .map(|m| match m {
                                    Move::Press(verb) => {
                                        feature::pair(&b, &feature::mv(&v, *verb))
                                    }
                                    Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                                })
                                .collect();
                            n.q_set(&pairs)
                                .into_iter()
                                .enumerate()
                                .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                                .map(|(i, _)| i)
                                .expect("not empty")
                        });
                    }
                }
                let _ = &mut sink;
                r.push(grade(&c).0 as f32);
            }
            println!(
                "  {:<26} {:>8} {:>10.3} {:>8.3}",
                path.rsplit('/').next().unwrap_or(path),
                r.len(),
                mean(&r),
                sd(&r) / (r.len().max(1) as f32).sqrt()
            );
        }
    }
}
