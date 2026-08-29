//! What Rogue actually changes, measured rather than read off the mode card.
//!
//!     cargo run --release -p gearmaster-lab --bin qrogue
//!
//! The card says one thing differs: what a loss takes off you. That is true of
//! the *rules* and it is not the whole of what differs for an **agent**, which
//! sees a run through `View` and is paid by a reward somebody wrote. This asks
//! four questions the training loops depend on and none of them had an answer.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::run::ROGUE_LIVES;
use gearmaster_lab::packers::Packer;
use gearmaster_trades::env::{Step as RoadStep, Walking};

const BUDGET: usize = 320;

fn main() {
    println!("ROGUE_LIVES = {ROGUE_LIVES}\n");
    q1_does_a_dead_rogue_run_end();
    q2_does_dying_pay();
    q3_what_the_screen_says_about_the_mode();
    q4_how_many_doors_change_value_with_the_mode();
    q5_is_it_the_mode_or_the_walker();
}

/// 1. Does an episode end when a Rogue run dies?
fn q1_does_a_dead_rogue_run_end() {
    // A starter board loses rung one on Medium, so fighting with it is the
    // cheapest way to spend four lives.
    println!("1. What happens when a Rogue run runs out of lives");
    let mut c = Console::start(0xC4A1, Mode::Rogue, Difficulty::Medium);
    let mut ever_over = false;
    for i in 0..8 {
        let before = c.view();
        if !c.menu().contains(&Verb::Fight) {
            break;
        }
        c.apply(Verb::Fight);
        let after = c.view();
        ever_over |= c.over();
        println!(
            "   loss {}: rung {} -> {}, lives {:?} -> {:?}, gold {} -> {}{}",
            i + 1,
            before.rung_shown,
            after.rung_shown,
            before.lives_left,
            after.lives_left,
            before.gold,
            after.gold,
            if after.lives_left > before.lives_left { "   <- WIPED" } else { "" }
        );
    }
    println!("   `Console::over()` ever true across those: {ever_over}");
    println!("   an episode driven by `over()` never ends in Rogue: the run restarts\n");
}

/// 2. Does the road reward pay for climbing the same rungs twice?
fn q2_does_dying_pay() {
    println!("2. Whether the road reward pays the same rung more than once");
    let seeds = [0xAA8D95DE31880461u64, 0x1212, 0x6060, 0xF1418AF3EDF965FD, 0x1111, 0x5EED1234];
    for mode in [Mode::Grinder, Mode::Rogue] {
      let (mut sum_best, mut sum_paid, mut sum_resets, mut sum_banked) = (0usize, 0i32, 0usize, 0.0f32);
      for seed in seeds {
        let mut c = Console::start(seed, mode, Difficulty::Medium);
        let mut w = Walking::new(None, BUDGET);
        let packer = Packer::named("control");
        // `+1` a rung, the way `pathfinder::Reward` pays it: every step where
        // the rung went up. Summed over a whole episode.
        let (mut paid, mut best, mut packed_at, mut resets) = (0i32, 1usize, None, 0usize);

        // And what the reward actually hands over, which is the question the
        // count of rung-steps was standing in for.
        let mut reward = gearmaster_trades::pathfinder::Reward::new(mode == Mode::Rogue);
        let mut banked = 0.0f32;
        let mut losses_before = 0u32;
        loop {
            let ms = w.moves(&c);
            if ms.is_empty() || w.steps >= BUDGET {
                break;
            }
            let before = c.view().rung_shown;
            let at = if packed_at != Some(before) && ms.iter().any(|s| matches!(s, RoadStep::Pack)) {
                packed_at = Some(before);
                ms.iter().position(|s| matches!(s, RoadStep::Pack)).expect("just checked")
            } else {
                ms.iter().position(|s| matches!(s, RoadStep::Press(_))).unwrap_or(0)
            };
            match &ms[at] {
                RoadStep::Pack => packer.pack(&mut c, 40),
                RoadStep::Press(v) => {
                    if !c.apply(*v).ok {
                        break;
                    }
                }
            }
            w.steps += 1;
            let v = c.view();
            let after = v.rung_shown;
            if after > before {
                paid += 1;
            }
            let lost = v.losses > losses_before;
            losses_before = v.losses;
            banked += reward.value(after, v.wiped, lost, false);
            // A Rogue episode is a run and a wipe ends it, so the walk stops
            // where a trainer's would - which is the whole difference between
            // measuring the mode and measuring the engine's kindness to a
            // player who has just lost everything.
            if v.wiped {
                resets += 1;
                break;
            }

            best = best.max(after);
        }
        sum_best += best;
        sum_paid += paid;
        sum_resets += resets;
        sum_banked += banked;
      }
      let n = seeds.len();
      println!(
          "   {:<8} mean best rung {:>5.1}   rung-steps climbed {:>5.1}   \
           climbs/best {:.2}   wipes {:>3}   reward paid {:>7.1}   paid/best {:>5.2}",
          format!("{mode:?}"),
          sum_best as f32 / n as f32,
          sum_paid as f32 / n as f32,
          sum_paid as f32 / sum_best as f32,
          sum_resets,
          sum_banked / n as f32,
          sum_banked / (sum_best as f32 * 4.0 / n as f32) / n as f32
      );
    }
    println!(
        "   `climbs/best` above one is the road being walked more than once, which is\n\
         the game. `reward paid` is what the agent is given for it: at most four a\n\
         rung of new ground, and nothing at all for the second walk.\n"
    );
}

/// 3. What can an agent see about which mode it is in?
fn q3_what_the_screen_says_about_the_mode() {
    println!("3. What the features carry about the mode");
    for mode in [Mode::Grinder, Mode::Rogue] {
        let c = Console::start(0x1212, mode, Difficulty::Medium);
        let v = c.view();
        let r = gearmaster_trades::pathfinder::road(&v, None);
        println!(
            "   {:<8} view.grinder {:<5} lives {:?}  wiped {:<5}  \
             road f[2]={:.2} f[24]={:.2} f[25]={:.2}",
            format!("{mode:?}"),
            v.grinder,
            v.lives_left,
            v.wiped,
            r[2],
            r[24],
            r[25]
        );
    }
    println!(
        "   f[2] is the lives as a fraction of what a Rogue run gets, f[24] is the\n\
         mode and f[25] is a wipe. f[2] used to be `lives.unwrap_or(9)/5`, which put\n\
         Grinder on 1.80 and separated the modes by accident.\n"
    );
}

/// 4. How much of the road is priced differently by mode?
fn q4_how_many_doors_change_value_with_the_mode() {
    use gearmaster_engine::event::{every_outcome, Outcome, EVENTS};
    let mut spare = Vec::new();
    let mut underwrite = Vec::new();
    for e in EVENTS {
        for ch in e.choices {
            for o in every_outcome(&ch.outcome) {
                match o {
                    Outcome::Spare => spare.push((e.id, ch.label)),
                    Outcome::Underwrite => underwrite.push((e.id, ch.label)),
                    _ => {}
                }
            }
        }
    }
    println!("4. Doors whose worth depends on the mode");
    println!("   Outcome::Spare      {:>2} choices - a life, and in Grinder nothing", spare.len());
    println!("   Outcome::Underwrite {:>2} choices - one loss forgiven for five rungs", underwrite.len());
    for (id, label) in spare.iter().chain(underwrite.iter()) {
        println!("     {id}  {label:?}");
    }
}

/// 5. Is the low Rogue ceiling the mode, or the walker?
///
/// `curriculum::walk_to` presses the first road verb and packs once a rung. The
/// pilot in `gearmaster-agent` is the control every benchmark in
/// `analysis/the-two-trades.md` was measured against, and it does more: it
/// barters, sells, rerolls and grows. If the pilot gets far in Rogue and the
/// walker does not, the walker is the problem and a curriculum should use the
/// pilot. If neither does, the mode is the problem and a Rogue curriculum
/// cannot be walked at all with the packer this repo has.
fn q5_is_it_the_mode_or_the_walker() {
    use gearmaster_agent::pilot::{self, Doctrine};
    use gearmaster_lab::curriculum;

    println!("\n5. How far each control gets, by mode");
    let seeds = [0x1212u64, 0x6060, 0xAA8D95DE31880461, 0xF1418AF3EDF965FD];
    let d = Doctrine { patience: 12, budget: 400_000, coverage: 0.0 };
    for mode in [Mode::Grinder, Mode::Rogue] {
        let pilot_best: usize = seeds
            .iter()
            .map(|&s| pilot::play(s, mode, Difficulty::Medium, d).best_rung)
            .sum::<usize>()
            / seeds.len();
        let walker_best: usize = seeds
            .iter()
            .map(|&s| curriculum::walk_to(s, mode, Difficulty::Medium, 40).1.rung)
            .sum::<usize>()
            / seeds.len();
        println!(
            "   {:<8} pilot reaches rung {:>2}   the curriculum walker reaches rung {:>2}",
            format!("{mode:?}"),
            pilot_best,
            walker_best
        );
    }
}
