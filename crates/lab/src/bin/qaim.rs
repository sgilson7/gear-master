//! Aim the composition at a chain and say how far it gets.
//!
//!     cargo run --release -p gearmaster-lab --bin qaim
//!     QAIM_QUEST=pathfinder_threshold QAIM_NET=runs/pathfinder_threshold.txt \
//!         cargo run --release -p gearmaster-lab --bin qaim
//!
//! The measurement C7 is graded on. A trained pathfinder, a frozen packer, the
//! shopping list, and one chain - played over a spread of seeds in both modes,
//! reporting **which stop each run got to** rather than only whether it
//! finished. A chain that stalls always at the same stop is a chain with one
//! problem; a chain that stalls in four different places is a chain with four.
//!
//! With no `QAIM_NET` the road policy is the first legal step every time, which
//! is a control rather than an agent - so read the header before believing the
//! table. That control is also the honest baseline for §3.5's gate: a named
//! model has to beat *itself with no weights* before it can be said to have
//! learned the chain rather than inherited it from the packer.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::rng::Rng;
use gearmaster_lab::packers::Packer;
use gearmaster_lab::{quests, shopping};
use gearmaster_trades::env::{Step as RoadStep, Walking};
use gearmaster_trades::pathfinder;
use gearmaster_trades::quest::{Progress, Quest};
use gearmaster_trades::QNet;

/// The most road decisions one run may take. `horizons` measured 204.
const BUDGET: usize = 320;

fn main() {
    let name = std::env::var("QAIM_QUEST").unwrap_or_else(|_| "pathfinder_threshold".into());
    let q = match quests::by_name(&name) {
        Ok(q) => q,
        Err(why) => {
            eprintln!("{name}: {why:?}");
            return;
        }
    };
    let net_path = std::env::var("QAIM_NET").unwrap_or_default();
    let net = if net_path.is_empty() { None } else { QNet::load(&net_path) };
    // `written` follows the plan and is an upper bound rather than a baseline -
    // it is told which choice at each door passes which stop. It answers "is
    // this chain reachable at all by this composition", which is the question
    // to settle before spending an hour training against it.
    let written = std::env::var("QAIM_ROAD").as_deref() == Ok("written");
    let packer = Packer::named(&std::env::var("QAIM_PACKER").unwrap_or_else(|_| "control".into()));
    let runs: usize = std::env::var("QAIM_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let pack_budget: usize =
        std::env::var("QAIM_PACK_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(40);

    println!("chain:  {} - {} stops", q.name, q.stops.len());
    println!(
        "road:   {}",
        match (written, &net, net_path.is_empty()) {
            (true, _, _) => "written, following the plan - an upper bound, not a baseline".into(),
            (_, Some(_), _) => format!("trained, from {net_path}"),
            (_, None, true) => "the first legal step every time - a control".into(),
            (_, None, false) =>
                format!("NOTHING - {net_path} did not load, so this is the control"),
        }
    );
    println!("packer: {}\n", packer.describe("QAIM_PACKER"));

    // How many runs got at least this far, one column a stop.
    let mut reached = vec![0usize; q.stops.len()];
    let mut deepest_rung = 0usize;
    let mut r = Rng::new(0x0A1_1E5);
    for i in 0..runs {
        let seed = r.next_u64();
        let mode = if i % 2 == 0 { Mode::Grinder } else { Mode::Rogue };
        let (p, rung) = aim(&q, net.as_ref(), written, &packer, seed, mode, pack_budget);
        for (j, hit) in reached.iter_mut().enumerate() {
            if p.has(j) {
                *hit += 1;
            }
        }
        deepest_rung = deepest_rung.max(rung);
        // The seed and how far it got, so a run worth watching can be found
        // and handed to `qproof` - which is how any of this becomes a picture.
        println!(
            "  seed {seed:#018X} {:<8} rung {rung:>2}  stops {}/{}{}",
            format!("{mode:?}"),
            p.passed(),
            q.stops.len(),
            if q.done(&p) { "  FINISHED" } else { "" }
        );
    }

    println!("\n  {:<14} {:<46} {:<9} reached", "tier", "stop", "rungs");
    for (s, hit) in q.stops.iter().zip(&reached) {
        println!(
            "  {:<14} {:<46} {:<9} {:>3}/{} ",
            format!("{:?}", s.tier),
            format!("{:?}", s.mark),
            format!("{}-{}", s.window.0 + 1, s.window.1 + 1),
            hit,
            runs
        );
    }
    println!("\n  deepest rung any run stood on: {deepest_rung}");
    // Where it stops is the useful half. A chain whose first unreached stop is
    // the same one every time has one cause, and it has a name.
    match q.stops.iter().zip(&reached).find(|(_, &h)| h == 0) {
        Some((s, _)) => println!("  the wall is {:?}, which no run passed", s.mark),
        None => println!("  every stop was passed by somebody"),
    }
}

/// One run, aimed.
fn aim(
    q: &Quest,
    net: Option<&QNet>,
    written: bool,
    packer: &Packer,
    seed: u64,
    mode: Mode,
    pack_budget: usize,
) -> (Progress, usize) {
    let mut c = Console::start(seed, mode, Difficulty::Medium);
    let mut w = Walking::new(None, BUDGET);
    let mut p = Progress::new(q);
    let mut deepest = 1usize;
    let mut plan = gearmaster_lab::roads::Written::default();
    // Bounded by the budget and by the move list running dry, which is what a
    // finished or dead run looks like from here.
    loop {
        let ms = w.moves(&c);
        if ms.is_empty() || w.steps >= BUDGET {
            break;
        }
        let v = c.view();
        let along = q.features(&p);
        let at = match net.filter(|_| !written) {
            _ if written => plan.choose(q, &p, &c, &ms),
            Some(n) => {
                let r = pathfinder::road_on_quest(&v, None, along);
                ms.iter()
                    .map(|s| n.q_pair(&pathfinder::pair(&r, &pathfinder::describe(&v, s))))
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(i, _)| i)
                    .expect("the list is not empty")
            }
            None => 0,
        };
        if std::env::var("QAIM_TRACE").is_ok() && w.steps < 24 {
            println!(
                "    step {:>3} rung {:>2} gold {:>4} tray {:>2} items {} W{} L{} last {:?}  taking {:?}",
                w.steps,
                v.rung_shown,
                v.gold,
                v.tray.len(),
                v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum::<usize>(),
                v.wins,
                v.losses,
                v.last_fight.as_ref().map(|f| (f.outcome.clone(), f.duration_ms)),
                ms[at]
            );
        }
        match &ms[at] {
            RoadStep::Pack => {
                shopping::fetch(q, &p, &mut c);
                packer.pack(&mut c, pack_budget);
            }
            RoadStep::Press(verb) => {
                if !c.apply(*verb).ok {
                    break;
                }
            }
        }
        w.steps += 1;
        // A Rogue run out of lives is replaced rather than ended, so nothing
        // else in this loop can tell. The run being measured is over.
        if c.view().wiped {
            break;
        }
        q.observe(&mut p, &c.view());
        deepest = deepest.max(c.view().rung_shown);
        if q.done(&p) {
            break;
        }
    }
    (p, deepest)
}
