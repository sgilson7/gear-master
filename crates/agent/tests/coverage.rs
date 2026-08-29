//! The dial, and the memory behind it.
//!
//! `Seen` counts rather than remembers-that-it-happened, and the difference is
//! not a detail: with a set, the first run to take a branch closes it for
//! every later run, so a sweep is diverse and each run in it is monotonous.
//! Content that needs *repetition* to reach - a county trip out of a town
//! gate, ten trips a run - gets visited once and never again.
//!
//! Measured: switching the dial from untried-first to least-visited-first took
//! door coverage from 51% to 70% and the county from 8 tiles to 20.
//!
//! Two of these play whole runs, and A7's rebuild made a run cost minutes
//! rather than seconds - so they are `#[ignore]`d, on the repo's own rule
//! that nothing over a second belongs in a suite that has to stay fast. They
//! are what `make eval` runs, beside the proofs.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::seen::Seen;
use gearmaster_console::{Difficulty, Mode};

#[test]
fn the_memory_counts_rather_than_remembers() {
    let mut s = Seen::default();
    assert_eq!(s.times("a-door", 0), 0);
    *s.choices_taken.entry("a-door".into()).or_default().entry(0).or_default() += 1;
    *s.choices_taken.entry("a-door".into()).or_default().entry(0).or_default() += 1;
    assert_eq!(s.times("a-door", 0), 2);
    assert_eq!(s.times("a-door", 1), 0, "an untaken branch is the least-taken one");
    assert_eq!(s.branches(), 1);
}

#[test]
#[ignore = "plays two whole runs; run with --ignored"]
fn a_run_writes_what_it_met_into_the_memory() {
    let mut seen = Seen::default();
    let e = pilot::play_remembering(
        0x1212,
        Mode::Grinder,
        Difficulty::Medium,
        // Enough presses to reach a door and few enough fights to stay a
        // test. A6's shop verbs made a rung cost several thousand presses,
        // and this used to pass at forty thousand because a run got nowhere.
        Doctrine { coverage: 1.0, budget: 300_000, patience: 6 },
        &mut seen,
    );
    assert_eq!(seen.runs, 1);
    assert!(seen.deepest_rung >= e.best_rung.min(seen.deepest_rung));
    assert!(seen.doors() > 0, "it answered {} doors and remembered none", e.doors);
    assert!(!seen.rungs_stood.is_empty());
    // A rung is counted once a run, not once a press - a Grinder farming rung
    // twelve stood on it once as far as coverage is concerned.
    assert!(
        seen.rungs_stood.values().all(|&n| n <= 1),
        "one run counted a rung more than once"
    );
}

#[test]
#[ignore = "plays two whole runs; run with --ignored"]
fn two_runs_sharing_a_memory_take_different_branches() {
    // The whole point of the dial. Both runs meet the same doors on the same
    // seed; the second should not repeat the first's answers where another
    // branch was open.
    let mut seen = Seen::default();
    let d = Doctrine { coverage: 1.0, budget: 300_000, patience: 6 };
    pilot::play_remembering(0x6060, Mode::Grinder, Difficulty::Medium, d, &mut seen);
    let after_one: usize = seen.branches();
    pilot::play_remembering(0x6060, Mode::Grinder, Difficulty::Medium, d, &mut seen);
    let after_two: usize = seen.branches();
    assert!(
        after_two > after_one,
        "the second run took no branch the first had not: {} then {}",
        after_one,
        after_two
    );
}
