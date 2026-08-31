//! What a packing net does when it is given a whole run.
//!
//!     cargo run --release -p gearmaster-lab --bin qhand
//!
//! `qmind` reports on a net's numbers and `qpack`'s own evaluation counts items
//! on boards drawn out of a pool. Neither plays **the row**, which is the loop
//! the collapse happened in, and so neither can say which of two checkpoints is
//! the one that reached rung 11.
//!
//! That question came up as bookkeeping and is worth more than that. `qrow`
//! writes `runs/quartermaster_row.txt` for the best block and
//! `runs/quartermaster_row_last.txt` for the end of the run, and the log that
//! documents the collapse - `analysis/nets/qrow-r12.log` - ends with the
//! message the trainer printed *before* it learned to keep both. So the labels
//! on those two files are a claim nobody has checked, and three milestones of
//! triage are a comparison between them.
//!
//! The seeds are `qrow`'s own, drawn from `ROW_SEED` in the same order, so the
//! written control's column here is the same measurement the trainer prints
//! before it takes a gradient: **mean rung 6.0, best 13**. If that number moves,
//! the harness moved and nothing else here means anything.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::{packers, row};
use gearmaster_trades::brief::Brief;
use gearmaster_trades::env::Move;
use gearmaster_trades::{feature, QNet};

/// `qrow`'s seed, so the runs below are the runs it trained and measured on.
const ROW_SEED: u64 = 0x0D0E_5EED;

/// `qrow`'s packing budget, in decisions.
///
/// Not presses. `hands::pack` is exhaustive and its median is 492 presses, and
/// handing a walker one budget where it wanted the other is how a run once
/// bought four pieces, seated none, and lost rung one for ever.
const PACK_BUDGET: usize = 40;

/// Play one run under a net, with `eps` of the presses taken at random.
///
/// The same chooser `qrow` trains with: the rotation filter is `pack_with`'s,
/// the brief is `Brief::NONE`, and `Move::Done` is the all-zero move. Exploring
/// is in here because the column this binary exists to reconstruct was printed
/// by a **behaviour** policy and not a greedy one, and five per cent of presses
/// is the floor `qrow` decays to.
fn play(seed: u64, mode: Mode, net: Option<&QNet>, eps: f32, rng: &mut Rng) -> row::Ran {
    let mut pack = |c: &mut Console| -> Vec<gearmaster_console::Verb> {
        let Some(net) = net else {
            // The written control does not report what it pressed, so a run
            // packed by it has the road and the fights on its tape and no
            // packing - which is not a proof and does not claim to be.
            packers::control(c, PACK_BUDGET);
            return Vec::new();
        };
        let pressed = row::pack_with(c, PACK_BUDGET, |c, ms| {
            if eps > 0.0 && (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                return (rng.next_u64() % ms.len() as u64) as usize;
            }
            let v = c.view();
            let b = feature::briefed(&feature::board(&v), &Brief::NONE);
            ms.iter()
                .map(|m| match m {
                    Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                    Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                })
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                .map(|(i, _)| i)
                .expect("not empty")
        });
        row::keys(&pressed)
    };
    row::run(seed, mode, Difficulty::Medium, &mut pack).1
}

/// **The trainer's column, rebuilt from one fixed policy.**
///
/// `qrow` prints `deepest rung` every hundred episodes and that figure is a
/// **maximum over the block**, not a mean. Depth in this game is heavy-tailed
/// and largely a property of the seed - the written control's own hundred runs
/// are mean 5.3 and max 47 - so a max of a hundred draws can wander a long way
/// while the policy behind it does not move at all.
///
/// This holds the policy still and prints the same column. Whatever it does
/// here, it did in training for free.
fn blocks(net: Option<&QNet>, mode: Mode, eps: f32, blocks: usize, per: usize) {
    let mut seeds = Rng::new(ROW_SEED);
    let mut explore = Rng::new(ROW_SEED ^ 0xA5A5_A5A5);
    println!("
  the same column, from a policy that is not changing:");
    println!("  {:>7} {:>8} {:>8} {:>8}", "block", "deepest", "mean", "past 6");
    for b in 0..blocks {
        let (mut deep, mut sum, mut tail) = (0usize, 0usize, 0usize);
        for _ in 0..per {
            let out = play(seeds.next_u64(), mode, net, eps, &mut explore);
            deep = deep.max(out.deepest);
            sum += out.deepest;
            tail += usize::from(out.deepest > 6);
        }
        println!(
            "  {:>7} {:>8} {:>8.2} {:>8}",
            b * per,
            deep,
            sum as f32 / per as f32,
            tail
        );
    }
}

/// **What it presses, and what becomes of what it builds.**
///
/// The collapse brief asked for a key histogram and said the single cheapest
/// diagnostic in two missions was one. This is it, plus the question the
/// histogram alone cannot answer: an item the packer finished is paid for on
/// the press that finishes it, **once, on a new high** - and an unlocked item
/// negotiates with whatever it is touching (`loadout::lock_assembled_in`), so
/// seating the next piece can take it apart again. The reward is not refunded.
///
/// So the number worth printing is the gap between the items an episode was
/// *paid* for and the items it was still holding at the end.
fn keys(net: Option<&QNet>, mode: Mode, runs: usize) {
    let mut seeds = Rng::new(ROW_SEED);
    let mut explore = Rng::new(ROW_SEED ^ 0xA5A5_A5A5);
    let mut count: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let (mut paid, mut held, mut broke, mut refused) = (0usize, 0usize, 0usize, 0usize);
    let (mut deepest, mut presses) = (0usize, 0usize);
    // Was locking ever on the table, and could the network tell it from the
    // other verb in its feature bucket? `Lock` and `Pin` both fall into
    // `feature::mv`'s `_ => 8`, and neither carries a piece, so both are the
    // same thirty-two numbers. If they score identically then the choice
    // between them is not the policy's - it is `max_by`, which returns the
    // *last* maximum, and `Pin` is pushed onto the menu after `Lock`.
    let (mut lock_offered, mut lock_tied) = (0usize, 0usize);

    for _ in 0..runs {
        let seed = seeds.next_u64();
        // The reward's own bookkeeping, copied: `best_items` runs across the
        // whole episode and pays only when the count passes its own high.
        let mut best_items = 0usize;
        let mut last_items = 0usize;
        let mut pressed: Vec<row::Pressed> = Vec::new();
        let mut pack = |c: &mut Console| {
            let Some(net) = net else {
                packers::control(c, PACK_BUDGET);
                return Vec::new();
            };
            let done = row::pack_with(c, PACK_BUDGET, |c, ms| {
                let v = c.view();
                let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                let qs: Vec<f32> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                        Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                    })
                    .collect();
                let find = |want: fn(&Verb) -> bool| {
                    ms.iter().position(|m| matches!(m, Move::Press(v) if want(v)))
                };
                let l = find(|v| matches!(v, Verb::Lock { .. }));
                let p = find(|v| matches!(v, Verb::Pin { .. }));
                if l.is_some() {
                    lock_offered += 1;
                    if let (Some(l), Some(p)) = (l, p) {
                        if qs[l] == qs[p] {
                            lock_tied += 1;
                        }
                    }
                }
                qs.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                    .map(|(i, _)| i)
                    .expect("not empty")
            });
            let out = row::keys(&done);
            pressed.extend(done);
            out
        };
        let (_, ran) = row::run(seed, mode, Difficulty::Medium, &mut pack);
        deepest = deepest.max(ran.deepest);
        let _ = &mut explore;

        for p in &pressed {
            presses += 1;
            let name = match p.verb {
                Some(v) => v.line().split_whitespace().next().unwrap_or("?").to_string(),
                None => "done".to_string(),
            };
            *count.entry(name).or_default() += 1;
            if !p.stuck {
                refused += 1;
            }
            if p.items_after > best_items {
                paid += p.items_after - best_items;
                best_items = p.items_after;
            }
            if p.items_after < last_items {
                broke += last_items - p.items_after;
            }
            last_items = p.items_after;
        }
        held += last_items;
    }

    println!("\n  over {runs} episodes, {presses} presses, deepest rung {deepest}");
    println!("  {:<14} {:>8} {:>8}", "key", "times", "share");
    for (k, n) in &count {
        println!("  {k:<14} {n:>8} {:>7.1}%", 100.0 * *n as f32 / presses.max(1) as f32);
    }
    println!("\n  items the reward paid for:        {paid}");
    println!("  items still standing at the end:  {held}");
    println!("  paid for and gone by the end:     {}", paid.saturating_sub(held));
    println!("  presses that took an item apart:  {broke}");
    println!("  presses the console refused:      {refused}");
    println!("\n  decisions where locking was offered:  {lock_offered}");
    println!(
        "  ...and scored *identically* to pin:   {lock_tied}  ({:.0}%)",
        100.0 * lock_tied as f32 / lock_offered.max(1) as f32
    );
}

fn main() {
    let runs: usize =
        std::env::var("QHAND_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let mode = if std::env::var("QHAND_MODE").as_deref() == Ok("grinder") {
        Mode::Grinder
    } else {
        Mode::Rogue
    };

    let asked: Vec<String> = match std::env::var("QHAND_NETS") {
        Ok(list) => list.split(',').map(|s| s.trim().to_string()).collect(),
        Err(_) => vec![
            "control".into(),
            "runs/quartermaster_row.txt".into(),
            "runs/quartermaster_row_last.txt".into(),
        ],
    };

    // **Ask for a deep episode rather than wait for one.**
    //
    // `qrow` samples one episode in twenty-five, and a policy whose mean is
    // rung two almost never has a deep one in the sample - twenty proofs off a
    // live run were rung 1 and 2 without exception. This plays until it finds
    // one that got where you asked and writes that as a proof.
    if let Ok(want) = std::env::var("QHAND_DEEP") {
        let want: usize = want.parse().unwrap_or(7);
        let which = std::env::var("QHAND_NET").unwrap_or_else(|_| "control".into());
        let out = std::env::var("QHAND_OUT").unwrap_or_else(|_| "runs/deep".into());
        let tries: usize =
            std::env::var("QHAND_TRIES").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
        let net = if which == "control" {
            None
        } else {
            match QNet::load_at(&which, feature::PAIR) {
                Ok(n) => Some(n),
                Err(why) => {
                    eprintln!("  {why}");
                    return;
                }
            }
        };
        println!("  {which}, {mode:?}: looking for a run that reaches rung {want}, {tries} tries");
        let mut seeds = Rng::new(ROW_SEED);
        let (mut found, mut best) = (0usize, 0usize);
        for i in 0..tries {
            let seed = seeds.next_u64();
            let mut pack = |c: &mut Console| -> Vec<gearmaster_console::Verb> {
                match &net {
                    Some(n) => {
                        let done = row::pack_with(c, PACK_BUDGET, |c, ms| {
                            let v = c.view();
                            let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                            ms.iter()
                                .map(|m| match m {
                                    Move::Press(verb) => {
                                        n.q(&feature::pair(&b, &feature::mv(&v, *verb)))
                                    }
                                    Move::Done => n.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                                })
                                .enumerate()
                                .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                                .map(|(i, _)| i)
                                .expect("not empty")
                        });
                        row::keys(&done)
                    }
                    None => {
                        // The written control records lines rather than verbs,
                        // and a line is what a proof is made of anyway - the
                        // round trip is the format, and `proof::write` refuses
                        // anything that does not replay.
                        let mut said = Vec::new();
                        packers::control_recording(c, PACK_BUDGET, &mut said);
                        said.iter().filter_map(|l| gearmaster_console::Verb::parse(l)).collect()
                    }
                }
            };
            let (_, ran) = row::run(seed, mode, Difficulty::Medium, &mut pack);
            best = best.max(ran.deepest);
            if ran.deepest < want {
                continue;
            }
            let notes = [
                ("packed by", which.clone()),
                ("try", i.to_string()),
                ("epsilon", "0.00  (greedy)".to_string()),
            ];
            let name = format!("deep-rung{:02}-{:016X}", ran.deepest, seed);
            match gearmaster_lab::proof::write(
                &out,
                &name,
                seed,
                mode,
                Difficulty::Medium,
                &ran.tape,
                ran.deepest,
                &notes,
            ) {
                Ok(path) => {
                    found += 1;
                    println!("  rung {:>2}   {path}", ran.deepest);
                }
                Err(why) => println!("  rung {:>2}   REFUSED: {why}", ran.deepest),
            }
        }
        println!("\n  {found} written, deepest seen {best}");
        return;
    }

    if let Ok(path) = std::env::var("QHAND_KEYS") {
        let net = if path == "control" { None } else { QNet::load_at(&path, feature::PAIR).ok() };
        println!("  {path}, {mode:?}");
        keys(net.as_ref(), mode, runs);
        return;
    }

    // Rebuilding the trainer's column wants a net, an exploration rate and a
    // number of blocks; the default run is the greedy table.
    if let Ok(path) = std::env::var("QHAND_BLOCKS") {
        let eps: f32 = std::env::var("QHAND_EPS").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let per: usize =
            std::env::var("QHAND_PER").ok().and_then(|v| v.parse().ok()).unwrap_or(100);
        let n: usize = std::env::var("QHAND_N").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
        let net = QNet::load_at(&path, feature::PAIR).ok();
        println!(
            "  {path}, {mode:?}, {n} blocks of {per} at eps {eps}{}",
            if net.is_none() { "   (NOTHING LOADED - this is the control)" } else { "" }
        );
        blocks(net.as_ref(), mode, eps, n, per);
        return;
    }

    println!("  the row, played greedily: {runs} runs of {mode:?} at Medium, {PACK_BUDGET} decisions a rung\n");
    println!("  {:<38} {:>8} {:>7} {:>7}", "packed by", "mean", "best", "runs");

    for name in &asked {
        let net = if name == "control" {
            None
        } else {
            match QNet::load_at(name, feature::PAIR) {
                Ok(net) => Some(net),
                Err(why) => {
                    println!("  {name:<38}   {why}");
                    continue;
                }
            }
        };

        // Drawn afresh for each net, so every packer meets the same six runs.
        let mut seeds = Rng::new(ROW_SEED);
        let (mut sum, mut best) = (0usize, 0usize);
        let mut depths: Vec<usize> = Vec::new();
        for _ in 0..runs {
            let seed = seeds.next_u64();
            // The greedy table wants depth and not a tape, so neither packer
            // reports here. `play` is the one that tapes.
            let mut pack = |c: &mut gearmaster_console::Console| {
                match &net {
                    Some(n) => packers::learned(Some(n), c, PACK_BUDGET),
                    None => packers::control(c, PACK_BUDGET),
                }
                Vec::new()
            };
            let (_, out) = row::run(seed, mode, Difficulty::Medium, &mut pack);
            sum += out.deepest;
            best = best.max(out.deepest);
            depths.push(out.deepest);
        }
        println!(
            "  {:<38} {:>8.1} {:>7} {:>7}   {:?}",
            short(name),
            sum as f32 / runs as f32,
            best,
            runs,
            depths
        );
    }
}

/// A path is too wide for a column and its last two parts are the whole name.
fn short(name: &str) -> String {
    let parts: Vec<&str> = name.rsplit('/').take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}
