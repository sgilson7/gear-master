//! Why the greedy policy collapses: what the network thinks each action is worth.
//!
//!     QWHY_NET=analysis/nets/pathfinder-grinder.txt \
//!       cargo run --release -p gearmaster-lab --bin qwhy
//!
//! Both trained pathfinders reach rung 53 and 47 while exploring and then settle
//! onto one verb pressed three hundred times. That is a policy whose argmax does
//! not depend on the state, and there are three quite different reasons a Q
//! function ends up like that:
//!
//! * **it is flat** - the network has nothing to say, and the argmax is
//!   whichever action happens to sit highest by a hair;
//! * **it is state-blind** - the spread is real but the same action wins
//!   everywhere, so the action half of the pair is all that is being read;
//! * **it is out of scale** - the values are spread over tens while the thing
//!   that should discourage a wasted press is worth a third of one.
//!
//! The three want different fixes and the printout tells them apart: the spread
//! per state, whether the winner changes, and where the values sit against the
//! rewards that made them.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_lab::packers::Packer;
use gearmaster_trades::env::{Step as RoadStep, Walking};
use gearmaster_trades::pathfinder;
use gearmaster_trades::QNet;
use std::collections::BTreeMap;

fn main() {
    let path = std::env::var("QWHY_NET")
        .unwrap_or_else(|_| "analysis/nets/pathfinder-grinder.txt".into());
    let Some(net) = QNet::load(&path) else {
        eprintln!("{path} did not load");
        return;
    };
    let mode = if std::env::var("QWHY_MODE").as_deref() == Ok("rogue") {
        Mode::Rogue
    } else {
        Mode::Grinder
    };
    let steps: usize = std::env::var("QWHY_STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    println!("{path}, {mode:?}\n");

    let mut c = Console::start(0x1212, mode, Difficulty::Medium);
    let packer = Packer::named("control");
    let mut w = Walking::new(None, 320);
    let mut chosen: BTreeMap<String, usize> = BTreeMap::new();
    let mut spreads: Vec<f32> = Vec::new();

    for i in 0..320 {
        let ms = w.moves(&c);
        if ms.is_empty() {
            break;
        }
        let v = c.view();
        let r = pathfinder::road(&v, None);
        let qs: Vec<(String, f32)> = ms
            .iter()
            .map(|s| {
                let q = net.q_pair(&pathfinder::pair(&r, &pathfinder::describe(&v, s)));
                let name = match s {
                    RoadStep::Pack => "pack".to_string(),
                    RoadStep::Press(verb) => verb.line(),
                };
                (name, q)
            })
            .collect();
        let hi = qs.iter().map(|(_, q)| *q).fold(f32::MIN, f32::max);
        let lo = qs.iter().map(|(_, q)| *q).fold(f32::MAX, f32::min);
        spreads.push(hi - lo);
        let at = qs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).expect("real numbers"))
            .map(|(i, _)| i)
            .expect("the list is not empty");
        *chosen.entry(qs[at].0.clone()).or_default() += 1;

        if i < steps {
            println!(
                "  step {:>3}  rung {:>2}  items {}   {}",
                i,
                v.rung_shown,
                v.grids.iter().map(|g| g.items.iter().filter(|x| x.assembled).count()).sum::<usize>(),
                qs.iter()
                    .map(|(n, q)| format!("{n} {q:+.3}"))
                    .collect::<Vec<_>>()
                    .join("   ")
            );
        }
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

    let n = spreads.len().max(1) as f32;
    println!(
        "\n  mean spread between best and worst action, per state: {:.3}",
        spreads.iter().sum::<f32>() / n
    );
    println!("  what it pressed, over {} decisions:", spreads.len());
    for (verb, times) in &chosen {
        println!("    {times:>4}  {verb}");
    }
    println!(
        "\n  for scale, the rewards that made these values:\n    \
         a rung of new ground +4, reaching a goal +50, a lost fight -1.0 (grinder)\n    \
         or -2.5 (rogue), a wipe -10, a decision that changed nothing -0.36."
    );
}
