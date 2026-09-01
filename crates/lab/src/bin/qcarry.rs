//! Does the packing at a rung decide the rung?
//!
//!     cargo run --release -p gearmaster-lab --bin qcarry
//!
//! `qmind` asks what is in a net and `qhand` asks how deep it gets. Neither
//! asks the question underneath both, which is whether the decisions the agent
//! is being graded on **matter at the rung they are made at**.
//!
//! They might not, and the game says why: a board persists. Nothing takes your
//! gear off between rungs, so the creature at rung nine is met by whatever
//! cleared rung eight plus whatever was done since. If what cleared rung eight
//! also clears nine, ten and eleven untouched, then every press made at nine
//! led to a win and every press made at nine led to a win *whatever it was* -
//! and a value function fitted to those returns is being taught that the
//! actions were the reason.
//!
//! ## What it measures
//!
//! At every rung a run stands on, three clones of that exact console:
//!
//! * **none** - press nothing and fight;
//! * **random** - press the budget out uniformly over the legal menu, and fight;
//! * **control** - the written packer, which is what the run actually does.
//!
//! Combat is a pure function of two boards (`CLAUDE.md` §2), so each of those is
//! a fact and not a rate: the same board against the same creature is the same
//! fight every time. The rate is across rungs and seeds.
//!
//! And from the same three states, the **carry**: run forward with no packing
//! at all and see how far the board gets on its own.
//!
//! ## What the answer decides
//!
//! If `none` clears the rung as often as `control` does over some stretch of the
//! ladder, that stretch contributes gradient with no signal in it - the return
//! is the same whatever was pressed, so the advantage of every action there is
//! zero and the only thing separating them in the fitted values is noise. That
//! is a property of the game rather than of the optimiser, and no loss, target
//! net or exploration schedule touches it.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::{packers, row};
use gearmaster_trades::env::Move;

/// `qrow`'s seed, so this is measured over the runs the trainer trains on.
const ROW_SEED: u64 = 0x0D0E_5EED;

/// `qrow`'s packing budget, in decisions.
const PACK_BUDGET: usize = 40;

/// The deepest rung this report has a row for.
const BANDS: usize = 16;

/// Fight whatever is in front of this console, and say whether the rung moved.
///
/// Takes the console by value because every caller is asking a counterfactual
/// and wants its copy consumed.
fn clears(mut c: Console) -> bool {
    let before = c.view().rung_shown;
    let fight = if c.view().brawl_waiting { Verb::FightParty } else { Verb::Fight };
    if !c.menu().contains(&fight) || !c.apply(fight).ok {
        return false;
    }
    c.view().rung_shown > before
}

/// Press the budget out at random over whatever is legal.
fn scramble(c: &mut Console, rng: &mut Rng) {
    row::pack_with(c, PACK_BUDGET, |_, ms| (rng.next_u64() % ms.len() as u64) as usize);
}

/// How far this board gets from here with nothing further done to it.
///
/// The run carries on - road walked, fights fought - and the packing closure
/// presses nothing at all. What comes back is the deepest rung it stood on, so
/// the caller subtracts where it started.
fn carry(c: Console) -> usize {
    row::run_from(c, &mut |_| Vec::new()).1.deepest
}

/// One rung a run stood on, and what happened to three copies of it.
struct Probe {
    rung: usize,
    none: bool,
    random: bool,
    control: bool,
    /// Rungs cleared from here with no packing at all, from the board the run
    /// actually took into the fight.
    carried: usize,
}

fn main() {
    let runs: usize =
        std::env::var("QCARRY_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let mode = match std::env::var("QCARRY_MODE").as_deref() {
        Ok("grinder") => Mode::Grinder,
        _ => Mode::Rogue,
    };
    println!(
        "\n  {runs} runs, {mode:?}, Medium, packed by the written control.\n  \
         Every rung it stands on, three copies of that console, and the fight is\n  \
         a pure function of the two boards - so each cell is a fact, not a rate."
    );

    let mut seeds = Rng::new(ROW_SEED);
    let mut rng = Rng::new(ROW_SEED ^ 0xC0FF_EE00);
    let mut probes: Vec<Probe> = Vec::new();
    let mut deepest = Vec::new();

    for _ in 0..runs {
        let seed = seeds.next_u64();
        let mut here: Vec<Probe> = Vec::new();
        {
            let rng = &mut rng;
            let here = &mut here;
            let mut pack = |c: &mut Console| -> Vec<Verb> {
                let rung = c.view().rung_shown;
                // The three arms, from this exact state, before anything is
                // pressed for real.
                let none = clears(c.clone());
                let random = {
                    let mut s = c.clone();
                    scramble(&mut s, rng);
                    clears(s)
                };
                let mut ctrl = c.clone();
                packers::control(&mut ctrl, PACK_BUDGET);
                let control = clears(ctrl.clone());
                // And how far the board the run is about to fight with gets on
                // its own, with nothing further done to it.
                let carried = carry(ctrl).saturating_sub(rung);
                here.push(Probe { rung, none, random, control, carried });
                // Now the run's own packing, which is the control arm again.
                packers::control(c, PACK_BUDGET);
                Vec::new()
            };
            let (_, ran) = row::run(seed, mode, Difficulty::Medium, &mut pack);
            deepest.push(ran.deepest);
        }
        probes.extend(here);
    }

    let mean = |v: &[usize]| v.iter().sum::<usize>() as f32 / v.len().max(1) as f32;
    println!(
        "\n  the runs themselves: mean deepest {:.2}, best {}",
        mean(&deepest),
        deepest.iter().copied().max().unwrap_or(0)
    );

    println!(
        "\n  rung   visits |  cleared by: none  random  control |  carry from here\n  \
           -------------- |  ---------------------------------- |  mean   max"
    );
    let mut agree = (0usize, 0usize);
    for band in 1..=BANDS {
        let at: Vec<&Probe> = probes.iter().filter(|p| p.rung == band).collect();
        if at.is_empty() {
            continue;
        }
        let pc = |f: fn(&Probe) -> bool| {
            100.0 * at.iter().filter(|p| f(p)).count() as f32 / at.len() as f32
        };
        let carried: Vec<usize> = at.iter().map(|p| p.carried).collect();
        println!(
            "  {band:>4}   {:>6} |  {:>11.0}% {:>6.0}% {:>7.0}% |  {:>4.1}  {:>4}",
            at.len(),
            pc(|p| p.none),
            pc(|p| p.random),
            pc(|p| p.control),
            mean(&carried),
            carried.iter().copied().max().unwrap_or(0),
        );
    }
    for p in &probes {
        agree.1 += 1;
        if p.none == p.control {
            agree.0 += 1;
        }
    }
    println!(
        "\n  the rung came out the same packed and unpacked on {} of {} visits ({:.0}%)",
        agree.0,
        agree.1,
        100.0 * agree.0 as f32 / agree.1.max(1) as f32
    );

    // **The one that decides it.** A decision only carries signal where the
    // arms disagree; everywhere else the return is the same whatever was
    // pressed, and the gradient taken there is fitting noise.
    let live: Vec<&Probe> = probes.iter().filter(|p| p.none != p.control).collect();
    println!(
        "  packing changed the outcome on {} of {} visits ({:.0}%), at rungs {:?}",
        live.len(),
        probes.len(),
        100.0 * live.len() as f32 / probes.len().max(1) as f32,
        {
            let mut r: Vec<usize> = live.iter().map(|p| p.rung).collect();
            r.sort_unstable();
            r.dedup();
            r
        }
    );

    // How much of an episode's presses are spent inside a rung whose outcome
    // was already decided before the packing started.
    let dead: usize = probes.iter().filter(|p| p.none == p.control).count();
    println!(
        "  so {:.0}% of every episode's {PACK_BUDGET}-decision packings are graded on\n  \
         a rung that came out the same either way",
        100.0 * dead as f32 / probes.len().max(1) as f32
    );

    // And what the discount does to that, which is the other half.
    let presses: usize = probes.len() * PACK_BUDGET;
    let per_run = presses as f32 / runs as f32;
    println!(
        "\n  an episode is about {:.0} decisions, and at gamma 0.999 the terminal\n  \
         reward reaches the first of them at {:.2} of its face value - so the\n  \
         run's whole worth is smeared nearly evenly over every press in it.",
        per_run,
        0.999f32.powf(per_run)
    );

    // **And the design experiment the table above implies.** If the rungs where
    // packing changes the outcome are the first few, then a packer that works
    // for those and then stops should reach as deep as one that works all the
    // way up - and how deep it gets is the whole objective the trainer optimises.
    println!("\n  whole runs, same {runs} seeds, packed by four different policies");
    println!("    {:<34} {:>8} {:>7} {:>8}", "policy", "mean", "best", "vs control");
    let mut arms: Vec<(String, f32, usize)> = Vec::new();
    for (name, until) in [
        ("the written control, every rung", usize::MAX),
        ("control to rung 3, then nothing", 3),
        ("control to rung 1, then nothing", 1),
        ("nothing at all, ever", 0),
    ] {
        let mut seeds = Rng::new(ROW_SEED);
        let mut got = Vec::new();
        for _ in 0..runs {
            let seed = seeds.next_u64();
            let mut pack = |c: &mut Console| -> Vec<Verb> {
                if c.view().rung_shown <= until {
                    packers::control(c, PACK_BUDGET);
                }
                Vec::new()
            };
            got.push(row::run(seed, mode, Difficulty::Medium, &mut pack).1.deepest);
        }
        arms.push((name.to_string(), mean(&got), got.iter().copied().max().unwrap_or(0)));
    }
    // And a random arm, which is the one the table says should be worst.
    {
        let mut seeds = Rng::new(ROW_SEED);
        let mut got = Vec::new();
        for _ in 0..runs {
            let seed = seeds.next_u64();
            let mut pack = |c: &mut Console| -> Vec<Verb> {
                scramble(c, &mut rng);
                Vec::new()
            };
            got.push(row::run(seed, mode, Difficulty::Medium, &mut pack).1.deepest);
        }
        arms.push(("random presses, every rung".into(), mean(&got), got.iter().copied().max().unwrap_or(0)));
    }
    let base = arms[0].1;
    for (name, m, best) in &arms {
        println!("    {name:<34} {m:>8.2} {best:>7} {:>+8.2}", m - base);
    }
    let _ = Move::Done;
}
