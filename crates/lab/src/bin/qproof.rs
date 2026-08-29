//! Write a proof from the **learned** agent, not the written pilot.
//!
//!     QPROOF_QUEST=pathfinder_threshold \
//!     QPROOF_ROAD=analysis/nets/pathfinder-threshold.txt \
//!         cargo run --release -p gearmaster-lab --bin qproof
//!
//! `proof` writes a transcript from `pilot::play`, which is the hand-written
//! control. This one walks the road with the trained pathfinder and packs with
//! the trained quartermaster, and records **both halves** into one transcript -
//! the packing included, because a transcript missing the packing replays into
//! a different board and is not a proof of anything.
//!
//! The result is watchable in the window the same way any proof is:
//!
//!     GEARMASTER_WATCH=analysis/proofs/<file>.proof cargo run -p gearmaster-gui
//!
//! Either network may be missing, and the header says which were present. A run
//! with no pathfinder takes the first legal step every time, which is a control
//! rather than an agent - so read the header before believing the picture.
//!
//! With `QPROOF_QUEST` the run is aimed at a chain: the road features carry how
//! far along it is, the shopping list buys what the chain needs, and the header
//! says which stops were passed. `QPROOF_ROAD=written` plays the plan-follower
//! instead of a network, which is the upper bound rather than an agent and
//! says so in the header too.
//!
//! **All three halves are in the transcript** - the road decisions, the
//! shopping and the packing - because a transcript missing any of them replays
//! into a different run and is not a proof of anything. Only the presses that
//! *stuck* are written: four fifths of what the hands do is seat-and-undo.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_lab::packers::Packer;
use gearmaster_lab::{quests, roads, shopping};
use gearmaster_trades::env::{Step as RoadStep, Walking};
use gearmaster_trades::pathfinder;
use gearmaster_trades::quest::{Progress, Quest};
use gearmaster_trades::QNet;

/// How many decisions the learned packer gets each time the road asks for one.
/// The written control has its own press budget - see `lab::packers`.
const PACK_BUDGET: usize = 40;
/// The most road decisions a run may take.
const BUDGET: usize = 320;

fn main() {
    let seed = std::env::var("QPROOF_SEED")
        .ok()
        .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x1212);
    let mode = if std::env::var("QPROOF_MODE").as_deref() == Ok("rogue") {
        Mode::Rogue
    } else {
        Mode::Grinder
    };
    let road_path = std::env::var("QPROOF_ROAD").unwrap_or_else(|_| "runs/pathfinder.txt".into());
    let written = road_path == "written";
    let road_net = if written { None } else { QNet::load(&road_path) };
    let pack_path = std::env::var("QPROOF_PACKER").unwrap_or_else(|_| "control".into());
    let packer = Packer::named(&pack_path);
    let quest: Option<Quest> = match std::env::var("QPROOF_QUEST") {
        Ok(name) => match quests::by_name(&name) {
            Ok(q) => Some(q),
            Err(why) => {
                eprintln!("QPROOF_QUEST={name}: {why:?}");
                return;
            }
        },
        Err(_) => None,
    };
    let road_says = match (written, &road_net) {
        (true, _) => "WRITTEN - follows the plan, an upper bound rather than an agent".to_string(),
        (_, Some(_)) => format!("trained, from {road_path}"),
        (_, None) => format!("MISSING ({road_path}) - first legal step every time"),
    };
    println!("pathfinder:    {road_says}");
    println!("quartermaster: {}", packer.describe(&pack_path));
    println!(
        "quest:         {}\n",
        quest.as_ref().map(|q| q.name.clone()).unwrap_or_else(|| "none".into())
    );

    let mut c = Console::start(seed, mode, Difficulty::Medium);
    let mut w = Walking::new(None, BUDGET);
    let mut said: Vec<String> = Vec::new();
    let mut best = 1usize;
    let mut packs = 0usize;
    let mut progress = quest.as_ref().map(Progress::new);
    let mut plan = roads::Written::default();

    loop {
        let ms = w.moves(&c);
        if ms.is_empty() || w.steps >= BUDGET {
            break;
        }
        let v = c.view();
        let along = match (&quest, &progress) {
            (Some(q), Some(p)) => q.features(p),
            _ => [0.0; 2],
        };
        let at = match (&road_net, written) {
            (_, true) => match (&quest, &progress) {
                (Some(q), Some(p)) => plan.choose(q, p, &c, &ms),
                _ => 0,
            },
            (Some(net), _) => {
                let r = pathfinder::road_on_quest(&v, None, along);
                ms.iter()
                    .map(|s| net.q_pair(&pathfinder::pair(&r, &pathfinder::describe(&v, s))))
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(i, _)| i)
                    .unwrap()
            }
            (None, _) => 0,
        };
        match &ms[at] {
            RoadStep::Pack => {
                if let (Some(q), Some(p)) = (&quest, &progress) {
                    said.extend(shopping::fetch(q, p, &mut c));
                }
                packer.pack_recording(&mut c, PACK_BUDGET, &mut said);
                packs += 1;
            }
            RoadStep::Press(verb) => {
                said.push(verb.line());
                if !c.apply(*verb).ok {
                    break;
                }
            }
        }
        w.steps += 1;
        best = best.max(c.view().rung_shown);
        if let (Some(q), Some(p)) = (&quest, &mut progress) {
            q.observe(p, &c.view());
            if q.done(p) {
                break;
            }
        }
    }
    println!(
        "seed {:#018X}: rung {}, {} road decisions, {} packs, {} presses",
        seed, best, w.steps, packs, said.len()
    );
    let stops = match (&quest, &progress) {
        (Some(q), Some(p)) => {
            println!("stops passed: {}/{}", p.passed(), q.stops.len());
            for (i, s) in q.stops.iter().enumerate() {
                println!("  {} {:?}", if p.has(i) { "x" } else { " " }, s.mark);
            }
            format!("{}/{}", p.passed(), q.stops.len())
        }
        _ => "-".to_string(),
    };

    // ---- it is only a proof if it replays -------------------------------
    let mut c2 = Console::start(seed, mode, Difficulty::Medium);
    let mut refused = 0;
    let mut replayed = 1;
    for line in &said {
        match Verb::parse(line) {
            Some(v) => {
                if !c2.apply(v).ok {
                    refused += 1;
                }
            }
            None => refused += 1,
        }
        replayed = replayed.max(c2.view().rung_shown);
    }
    println!(
        "replayed to rung {} with {} refusals{}",
        replayed,
        refused,
        if replayed == best && refused == 0 { "  - identical" } else { "  - DIFFERENT" }
    );

    let out = std::env::var("QPROOF_OUT").unwrap_or_else(|_| "analysis/proofs".into());
    std::fs::create_dir_all(&out).ok();
    let path = format!(
        "{}/{:016X}-{}-medium-{}.proof",
        out,
        seed,
        format!("{:?}", mode).to_lowercase(),
        quest.as_ref().map(|q| q.name.as_str()).unwrap_or("learned")
    );
    let header = format!(
        "# seed        {:#018X}\n# mode        {:?}\n# difficulty  Medium\n\
         # pathfinder  {}\n# packer      {}\n# quest       {}\n# stops       {}\n\
         # reached     rung {}\n# presses     {} ({} road decisions, {} packs)\n#\n\
         # Every line below is a key a person could press. All three halves are\n\
         # here: the road decisions, the shopping the chain asked for, and the\n\
         # packing - only the presses that stuck, because four fifths of what\n\
         # the hands do is seat-and-undo.\n#\n\
         # Watch it:\n\
         #   GEARMASTER_WATCH={}/{:016X}-{}-medium-{}.proof \\\n\
         #     cargo run -p gearmaster-gui\n\n",
        seed,
        mode,
        road_says,
        packer.describe(&pack_path),
        quest.as_ref().map(|q| q.name.as_str()).unwrap_or("none"),
        stops,
        best,
        said.len(),
        w.steps,
        packs,
        out,
        seed,
        format!("{:?}", mode).to_lowercase(),
        quest.as_ref().map(|q| q.name.as_str()).unwrap_or("learned"),
    );
    std::fs::write(&path, header + &said.join("\n") + "\n").expect("wrote the proof");
    println!("wrote {}", path);
}
