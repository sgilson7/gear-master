//! A transcript is the proof, so it has to round-trip and it has to replay.
//!
//! `acceptance::e6_1` is the precedent: two replays of a seed agree about
//! everything that rolls. This is the same claim one level up - two replays of
//! a *transcript* agree about everything, including the fights, which is what
//! makes a proof a proof rather than a claim.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;

fn play(seed: u64, steps: usize) -> (Console, Vec<Verb>) {
    let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
    let mut rng = Rng::new(seed ^ 0xBEE5);
    for _ in 0..steps {
        let menu = c.menu();
        if menu.is_empty() {
            break;
        }
        let v = menu[(rng.next_u64() % menu.len() as u64) as usize];
        if !c.apply(v).ok {
            break;
        }
    }
    let h = c.history().to_vec();
    (c, h)
}

#[test]
fn every_verb_survives_a_round_trip_through_text() {
    let (_, history) = play(0x5EED_1234_ABCD_0001, 300);
    assert!(history.len() > 50, "only pressed {}", history.len());
    for v in history {
        let line = v.line();
        assert_eq!(Verb::parse(&line), Some(v), "line {:?} did not read back", line);
    }
}

#[test]
fn a_comment_and_the_annotation_do_not_change_the_verb() {
    let c = Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium);
    let v = c.menu().into_iter().find(|v| matches!(v, Verb::Place { .. })).expect("a placement");
    let annotated = c.annotate(v);
    assert!(annotated.contains(';'), "the annotation carries a name: {}", annotated);
    assert_eq!(Verb::parse(&annotated), Some(v));
    assert_eq!(Verb::parse("   ; a whole line of comment"), None);
}

#[test]
fn replaying_a_transcript_reproduces_the_run_exactly() {
    for seed in [0x5EED_1234_ABCD_0001u64, 0x6060, 0x1212] {
        let (first, history) = play(seed, 300);

        // Write it out the way a proof file is written, read it back, and play
        // it into a fresh run.
        let text: Vec<String> = history.iter().map(|v| first.annotate(*v)).collect();
        let read: Vec<Verb> = text
            .iter()
            .filter_map(|l| Verb::parse(l))
            .collect();
        assert_eq!(read.len(), history.len(), "a line failed to read back");

        let mut second = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        for v in &read {
            let out = second.apply(*v);
            assert!(out.ok, "the transcript pressed {:?} and it was refused", v);
        }

        assert_eq!(
            first.screen(),
            second.screen(),
            "seed {:#x}: the replay drew a different screen",
            seed
        );
        assert_eq!(first.history(), second.history());
    }
}

#[test]
fn the_same_seed_twice_is_the_same_run() {
    // The environment has no noise in it. If this fails, nothing above it
    // means anything (`design/the-apprentice.md` §15.2).
    let (a, ha) = play(0x1111, 250);
    let (b, hb) = play(0x1111, 250);
    assert_eq!(ha, hb);
    assert_eq!(a.screen(), b.screen());
}
