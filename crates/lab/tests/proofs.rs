//! Every committed proof still replays.
//!
//! `#[ignore]`d by default and run by `make eval`, per the plan's D6: a proof
//! is an artifact, and a test that loads an artifact does not belong in a
//! suite that has to stay fast. It belongs *somewhere*, though - a proof
//! nobody checks is a claim.
//!
//! When an engine commit breaks one of these, the diff between the rung it
//! reached and the rung it reaches now is the report.
//!
//! **It read every proof as Grinder for its whole life.** The header carries a
//! mode and this parsed the seed and the rung out of it and never that, so six
//! Rogue proofs were replayed in the wrong mode - and it stayed green, because
//! all six claim **rung 1**, which any mode replays to having pressed almost
//! nothing. The three that go deep are all Grinder. So the guard was never once
//! asked the question it exists to ask, and the first Rogue proof to reach a
//! real rung would have failed here reading `claims rung 9 and replays to 4`,
//! which looks exactly like a determinism bug in the engine and is a header
//! field nobody parsed. `qrow` writes Rogue proofs now.

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
        // Named rather than defaulted: a proof with no mode is a proof this
        // test cannot check, and quietly picking one is how it went wrong.
        let mode = match text
            .lines()
            .find_map(|l| l.strip_prefix("# mode        "))
            .map(str::trim)
            .unwrap_or_else(|| panic!("{:?} has no `# mode` header", path))
        {
            "Rogue" => Mode::Rogue,
            "Grinder" => Mode::Grinder,
            other => panic!("{:?} has an unknown mode {other:?}", path),
        };

        let mut c = Console::start(seed, mode, Difficulty::Medium);
        let mut best = 1;
        let mut refused = 0;
        for line in text.lines() {
            let Some(v) = Verb::parse(line) else { continue };
            if !c.apply(v).ok {
                refused += 1;
            }
            best = best.max(c.view().rung_shown);
        }
        assert_eq!(
            refused, 0,
            "{:?} ({:?}): {} presses were refused on replay",
            path, mode, refused
        );
        assert_eq!(
            best, claimed,
            "{:?} ({:?}): claims rung {} and replays to {}",
            path, mode, claimed, best
        );
        checked += 1;
    }
    assert!(checked > 0, "no proofs were checked");
    println!("{} proofs replayed", checked);
}
