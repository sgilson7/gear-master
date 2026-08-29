//! Where the damage goes.
//!
//! The plateau test says the failures are wide - 53 of 56 runs lose with most
//! of the creature still standing - and calls that exploration. Before
//! believing it, ask the fight what happened to the blows that were thrown.
//! A board landing plenty of damage that is all absorbed is not a board that
//! never found the right family; it is a board being scored by an objective
//! that cannot see the difference.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::combat::{simulate_at, Event, Side, LADDER};
use gearmaster_engine::rng::Rng;

fn seeds(n: usize) -> Vec<u64> {
    let mut out = vec![0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

fn main() {
    let n: usize = std::env::var("ABS_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let wall = LADDER.iter().position(|s| s.name == "Ashen Marshal").expect("on the ladder");
    println!(
        "Every board that got stuck, thrown at rung {} - {}.\n",
        wall + 1,
        LADDER[wall].name
    );
    println!(
        "{:<14} {:>5} {:>7} {:>8} {:>7} {:>8} {:>8} {:>7} {:>6}",
        "seed", "rung", "blows", "biggest", "dealt", "your hp", "taken", "secs", "eaten"
    );
    println!("{}", "-".repeat(84));

    let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };
    let mut total_thrown = 0i64;
    let mut total_absorbed = 0i64;
    for seed in seeds(n) {
        let e = pilot::play(seed, Mode::Grinder, Difficulty::Medium, d);
        if e.best_rung > 15 {
            continue;
        }
        let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        for line in &e.transcript {
            if let Some(v) = Verb::parse(line) {
                c.apply(v);
            }
        }
        let (stats, items) = c.board_for_scoring();
        let log = simulate_at(stats, &items, &LADDER[wall], Difficulty::Medium);
        let (mut blows, mut biggest, mut landed, mut absorbed) = (0u32, 0i64, 0i64, 0i64);
        for entry in &log.entries {
            if let Event::Hit { by: Side::Player, damage, absorbed: ab, .. } = entry.event {
                blows += 1;
                biggest = biggest.max(damage as i64 + ab as i64);
                landed += damage as i64;
                absorbed += ab as i64;
            }
        }
        let thrown = landed + absorbed;
        total_thrown += thrown;
        total_absorbed += absorbed;
        let taken: i64 = log
            .entries
            .iter()
            .filter_map(|en| match en.event {
                Event::Hit { by: Side::Enemy, damage, .. } => Some(damage as i64),
                Event::Burn { side: Side::Player, damage, .. } => Some(damage as i64),
                _ => None,
            })
            .sum();
        println!(
            "{:<14} {:>5} {:>7} {:>8} {:>7} {:>8} {:>8} {:>6.1} {:>5.0}%",
            format!("{:#08X}", seed),
            e.best_rung,
            blows,
            biggest,
            landed,
            log.player.max_health,
            taken,
            log.duration_ms as f64 / 1000.0,
            if thrown > 0 { 100.0 * absorbed as f64 / thrown as f64 } else { 0.0 }
        );
    }
    println!(
        "\nacross all of them: {} thrown, {} absorbed - **{:.0}% eaten before it landed**",
        total_thrown,
        total_absorbed,
        if total_thrown > 0 { 100.0 * total_absorbed as f64 / total_thrown as f64 } else { 0.0 }
    );
    println!(
        "\nFor comparison, the same creature against the owner's finished board:"
    );
    let refs = gearmaster_oracle::gate::References::standard();
    for (label, stats, items, _) in &refs.boards {
        let log = simulate_at(*stats, items, &LADDER[wall], Difficulty::Medium);
        let (mut blows, mut biggest, mut landed, mut absorbed) = (0u32, 0i64, 0i64, 0i64);
        for entry in &log.entries {
            if let Event::Hit { by: Side::Player, damage, absorbed: ab, .. } = entry.event {
                blows += 1;
                biggest = biggest.max(damage as i64 + ab as i64);
                landed += damage as i64;
                absorbed += ab as i64;
            }
        }
        let thrown = landed + absorbed;
        let _ = thrown;
        let taken: i64 = log
            .entries
            .iter()
            .filter_map(|en| match en.event {
                Event::Hit { by: Side::Enemy, damage, .. } => Some(damage as i64),
                Event::Burn { side: Side::Player, damage, .. } => Some(damage as i64),
                _ => None,
            })
            .sum();
        println!(
            "  {:<8} {:>4} blows, biggest {:>4}, dealt {:>5}, your hp {:>5}, took {:>5}, {:>5.1}s  {}",
            label,
            blows,
            biggest,
            landed,
            log.player.max_health,
            taken,
            log.duration_ms as f64 / 1000.0,
            log.outcome.label()
        );
    }
}
