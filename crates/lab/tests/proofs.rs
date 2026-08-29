//! Every committed proof still replays.
//!
//! `#[ignore]`d by default and run by `make eval`, per the plan's D6: a proof
//! is an artifact, and a test that loads an artifact does not belong in a
//! suite that has to stay fast. It belongs *somewhere*, though - a proof
//! nobody checks is a claim.
//!
//! When an engine commit breaks one of these, the diff between the rung it
//! reached and the rung it reaches now is the report.

use gearmaster_console::{Console, Difficulty, Mode, Verb};

#[test]
#[ignore = "reads analysis/proofs; run with --ignored"]
fn every_proof_replays_to_the_rung_it_claims() {
    let dir = std::path::Path::new("../../analysis/proofs");
    let Ok(entries) = std::fs::read_dir(dir) else {
        panic!("no proofs directory at {:?}", dir);
    };
    let mut checked = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "proof") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable");
        let claimed: usize = text
            .lines()
            .find_map(|l| l.strip_prefix("# reached     rung "))
            .and_then(|r| r.split_whitespace().next())
            .and_then(|r| r.parse().ok())
            .unwrap_or_else(|| panic!("{:?} has no `# reached` header", path));
        let seed: u64 = text
            .lines()
            .find_map(|l| l.strip_prefix("# seed        0x"))
            .and_then(|r| u64::from_str_radix(r.trim(), 16).ok())
            .unwrap_or_else(|| panic!("{:?} has no `# seed` header", path));

        let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        let mut best = 1;
        let mut refused = 0;
        for line in text.lines() {
            let Some(v) = Verb::parse(line) else { continue };
            if !c.apply(v).ok {
                refused += 1;
            }
            best = best.max(c.view().rung_shown);
        }
        assert_eq!(refused, 0, "{:?}: {} presses were refused on replay", path, refused);
        assert_eq!(
            best, claimed,
            "{:?}: claims rung {} and replays to {}",
            path, claimed, best
        );
        checked += 1;
    }
    assert!(checked > 0, "no proofs were checked");
    println!("{} proofs replayed", checked);
}
