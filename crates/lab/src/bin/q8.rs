//! Q8's gate: does conditioning on a brief help on a theme never trained on?
//!
//!     cargo run --release -p gearmaster-lab --bin q8
//!
//! Two networks, identical in shape and in training budget. One saw a brief
//! every episode, drawn from eight themes; the other saw thirteen zeros. Both
//! are then asked to pack for **Hollow** and **Warden**, which neither has
//! seen, and the boards are judged by A2's fidelity meter - the real one, on a
//! real fight, which the training could not afford to run.
//!
//! The comparison is only worth reading if both networks pack *something*, so
//! the table prints items and win rate beside the fidelity. A conditioned
//! packer that reads as a better Warden by building nothing has not won.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::bestiary::MonsterTheme;
use gearmaster_engine::combat::{simulate_at, Difficulty as D, Outcome, LADDER};
use gearmaster_engine::run::Run;
use gearmaster_lab::{boards, themes};
use gearmaster_oracle::gate::References;
use gearmaster_oracle::fidelity;
use gearmaster_trades::brief::Brief;
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::{feature, QNet};

/// One packer's answer to one brief.
struct Answer {
    items: f64,
    won: usize,
    of: usize,
    fidelity: f64,
}

fn pack_and_read(net: &QNet, w: &Brief, t: MonsterTheme, refs: &References) -> Answer {
    let (mut items, mut won, mut fid, mut n) = (0.0f64, 0usize, 0.0f64, 0usize);
    for i in 0..12u64 {
        let rung = (i as usize * 3) % 30;
        let mut run = Run::start(0x8_B41E + i * 7919, Mode::Grinder, Difficulty::Medium);
        if rung > 0 {
            run.skip_to(rung);
        }
        let mut c = Console::standing_in(run, i);
        let mut e = Packing::new(40);
        loop {
            let ms: Vec<Move> = e
                .moves(&c)
                .into_iter()
                .filter(|m| {
                    !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. }))
                })
                .collect();
            if ms.is_empty() {
                break;
            }
            let v = c.view();
            let b = feature::briefed(&feature::board(&v), w);
            let qs: Vec<f32> = ms
                .iter()
                .map(|m| match m {
                    Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                    Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
                })
                .collect();
            let at = qs
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();
            e.step(&mut c, ms[at]);
            if e.finished {
                break;
            }
        }
        let (stats, its) = c.board_for_scoring();
        items += its.len() as f64;
        n += 1;
        if its.is_empty() {
            continue;
        }
        let spec = &LADDER[rung.min(LADDER.len() - 1)];
        if simulate_at(stats, &its, spec, D::Medium).outcome == Outcome::Victory {
            won += 1;
        }
        // **A2's meter, on the board as a creature.** The packed board takes
        // the field against the first reference player and the meter reads the
        // fight from the creature's side, which is the only way to ask "does
        // this pack like a Warden" of a board somebody built as a player.
        let (_, rstats, rits, _) = &refs.boards[0];
        let run = c.into_run();
        let cut = boards::cut(&run, t.slots());
        if cut.gear.is_empty() {
            continue;
        }
        let mine = gearmaster_oracle::as_creature(
            spec,
            &gearmaster_oracle::Board { gear: cut.gear, chunks: cut.chunks, rows: [0; 5] },
        );
        let log = simulate_at(*rstats, rits, &mine, D::Medium);
        fid += fidelity::of(t, &log).score;
    }
    Answer { items: items / n as f64, won, of: n, fidelity: fid / n as f64 }
}

fn main() {
    let briefed = QNet::load(&std::env::var("Q8_BRIEFED").unwrap_or("q8/briefed/runs/quartermaster.txt".into()));
    let control = QNet::load(&std::env::var("Q8_CONTROL").unwrap_or("q8/control/runs/quartermaster.txt".into()));
    let (Some(briefed), Some(control)) = (briefed, control) else {
        println!("Both networks are needed and at least one is missing.");
        return;
    };
    let refs = References::standard();

    println!("# Q8 — does the brief carry to a theme it never saw?\n");
    println!(
        "`cargo run --release -p gearmaster-lab --bin q8`. Two networks of identical shape and\n\
         budget: one trained on eight briefs, one on thirteen zeros. Twelve situations each,\n\
         packed greedily, judged by A2's fidelity meter on a real fight.\n"
    );
    println!("| theme | packer | items | won | fidelity |");
    println!("|---|---|---:|---:|---:|");
    let mut lift: Vec<(String, f64)> = Vec::new();
    for t in themes::ALL {
        let held = themes::HELD_OUT.contains(&t);
        let w = themes::brief(t);
        let a = pack_and_read(&briefed, &w, t, &refs);
        let b = pack_and_read(&control, &Brief::NONE, t, &refs);
        let mark = if held { " *(held out)*" } else { "" };
        println!(
            "| {}{} | briefed | {:.1} | {}/{} | **{:.3}** |",
            themes::name(t),
            mark,
            a.items,
            a.won,
            a.of,
            a.fidelity
        );
        println!(
            "| | unconditioned | {:.1} | {}/{} | {:.3} |",
            b.items, b.won, b.of, b.fidelity
        );
        if held {
            lift.push((themes::name(t), a.fidelity - b.fidelity));
        }
    }
    println!("\n## The gate\n");
    for (n, d) in &lift {
        println!(
            "  - **{}** — the brief is worth {}{:.3} of fidelity.",
            n,
            if *d >= 0.0 { "+" } else { "" },
            d
        );
    }
    let met = lift.iter().all(|(_, d)| *d > 0.0);
    println!(
        "\n**{}** — asked that both held-out themes be packed better with the brief than without.",
        if met { "MET" } else { "MISSED" }
    );
}
