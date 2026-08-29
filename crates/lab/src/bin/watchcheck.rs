//! Does a proof drive a run the way the window will drive it?
//!
//! The window presses a proof's verbs through `Console` and hands fights to
//! its own playback. This is that path without a window: the same verbs, the
//! same console, and a fight taken the same way - so a proof that walks here
//! walks there, and a screenshot is not the only way to find out.
use gearmaster_console::{Console, Difficulty, Mode, Verb};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        "analysis/proofs/C434E4A68C5906EE-grinder-medium.proof".into()
    });
    let text = std::fs::read_to_string(&path).expect("a proof");
    let seed = text
        .lines()
        .find_map(|l| l.strip_prefix("# seed        0x"))
        .and_then(|v| u64::from_str_radix(v.trim(), 16).ok())
        .expect("a seed in the header");
    let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
    let (mut pressed, mut fights, mut refused) = (0, 0, 0);
    let mut best = 1;
    for line in text.lines() {
        let Some(v) = Verb::parse(line) else { continue };
        if matches!(v, Verb::Fight | Verb::FightParty) {
            fights += 1;
        }
        if !c.apply(v).ok {
            refused += 1;
        }
        pressed += 1;
        best = best.max(c.view().rung_shown);
    }
    println!(
        "{}\n  {} presses, {} of them fights, {} refused\n  reached rung {}",
        path, pressed, fights, refused, best
    );
    println!(
        "\n  watch it:  GEARMASTER_WATCH={} cargo run -p gearmaster-gui\n  \
         faster:    GEARMASTER_WATCH_MS=20 GEARMASTER_WATCH={} cargo run -p gearmaster-gui",
        path, path
    );
}
