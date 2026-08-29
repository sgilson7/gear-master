//! Q2's numbers: how fast an episode of each is, and how long it runs.
//!
//!     cargo run --release -p gearmaster-lab --bin envcheck
//!
//! The whole training budget is priced on these. If a packing episode costs
//! more than a few milliseconds, ten million steps is not a night's work.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::combat::{simulate_at, Outcome, LADDER};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::run::Run;
use gearmaster_trades::env::{Packing, Step, Walking};
use std::time::Instant;

/// Stand a run at a rung with a purse and a shop, ready to be packed for.
///
/// **`skip_to` is privileged and training-only.** The pathfinder cannot reach
/// it - it is not a verb - and the quartermaster never sees how it got here.
/// This is the curriculum: a packer trained only on rung one is a packer that
/// has never seen a board worth packing.
pub fn situation(seed: u64, rung: usize) -> Console {
    let mut run = Run::start(seed, Mode::Grinder, Difficulty::Medium);
    if rung > 0 {
        run.skip_to(rung);
    }
    Console::standing_in(run, seed)
}

fn main() {
    let mut rng = Rng::new(0x5171_0A71);
    println!("Q2 throughput, one performance core, release.\n");

    // ---- the quartermaster ------------------------------------------
    let t = Instant::now();
    let (mut episodes, mut steps, mut fights) = (0usize, 0usize, 0usize);
    let mut lengths: Vec<usize> = Vec::new();
    for i in 0..300 {
        let rung = (rng.next_u64() % 50) as usize;
        let mut c = situation(rng.next_u64(), rung);
        let mut e = Packing::new(60);
        let mut n = 0;
        loop {
            let ms = e.moves(&c);
            if ms.is_empty() {
                break;
            }
            let m = ms[(rng.next_u64() % ms.len() as u64) as usize];
            e.step(&mut c, m);
            n += 1;
            if e.finished {
                break;
            }
        }
        // The reward: one fight against what is coming.
        let (stats, items) = c.board_for_scoring();
        let spec = &LADDER[rung.min(LADDER.len() - 1)];
        let won = simulate_at(stats, &items, spec, Difficulty::Medium).outcome == Outcome::Victory;
        fights += won as usize;
        episodes += 1;
        steps += n;
        lengths.push(n);
        let _ = i;
    }
    let el = t.elapsed().as_secs_f64();
    lengths.sort_unstable();
    println!(
        "quartermaster, random policy:\n  {} episodes in {:.2}s = **{:.2} ms an episode**\n  \
         {} steps total, {:.1} a episode (median {}, max {})\n  \
         {} of them won the fight they were packed for",
        episodes,
        el,
        el * 1000.0 / episodes as f64,
        steps,
        steps as f64 / episodes as f64,
        lengths[lengths.len() / 2],
        lengths[lengths.len() - 1],
        fights
    );
    println!(
        "  => 10^6 steps is about {:.0} s of environment on one core",
        1e6 / (steps as f64 / el)
    );

    // ---- the pathfinder ---------------------------------------------
    //
    // With `pack` as a no-op, because Q2 is measuring the *shell*: what the
    // road costs when the board is somebody else's problem.
    let t = Instant::now();
    let (mut runs, mut pf_steps) = (0usize, 0usize);
    let mut reached: Vec<usize> = Vec::new();
    for _ in 0..12 {
        let mut c = Console::start(rng.next_u64(), Mode::Grinder, Difficulty::Medium);
        let mut e = Walking::new(None, 600);
        let mut n = 0;
        loop {
            let ms = e.moves(&c);
            if ms.is_empty() {
                break;
            }
            let s = ms[(rng.next_u64() % ms.len() as u64) as usize].clone();
            match s {
                Step::Pack => {}
                Step::Press(v) => {
                    if !c.apply(v).ok {
                        break;
                    }
                }
            }
            e.steps += 1;
            n += 1;
            if n >= 600 {
                break;
            }
        }
        reached.push(c.view().rung_shown);
        runs += 1;
        pf_steps += n;
    }
    let el = t.elapsed().as_secs_f64();
    reached.sort_unstable();
    println!(
        "\npathfinder, random policy, `pack` a no-op:\n  {} runs in {:.2}s = **{:.0} ms a run**\n  \
         {:.0} steps a run; deepest rung reached at random {}",
        runs,
        el,
        el * 1000.0 / runs as f64,
        pf_steps as f64 / runs as f64,
        reached[reached.len() - 1]
    );

    println!(
        "\nthe gate: a packing episode under 60 decisions - median {}.\n\
         The walking figure above is the **budget**, not the horizon: a random\n\
         policy never fights, so it never finishes and never terminates. Q0\n\
         measured the control at 204 decisions a run, and that is the number.",
        lengths[lengths.len() / 2]
    );
}
