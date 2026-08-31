//! A run's tape, written as a proof - and refused when it does not replay.
//!
//! A proof is `(seed, mode, difficulty, [verb])` and a header saying what it
//! claims. `GEARMASTER_WATCH` plays one into the window through the same
//! `Console` the agent uses, so a proof that does not replay does not show a
//! wrong run: it shows **a different run**, silently, which is the class of
//! fault this repo keeps finding.
//!
//! So `write` replays before it writes, exactly as `qproof` does, and hands
//! back an `Err` naming the disagreement rather than a file. A trainer counts
//! those and says how many there were; nobody has to notice.
//!
//! The header is byte-compatible with the one `qproof` writes, because
//! `gui::watch` and `lab/tests/proofs.rs` both parse it by column - `# seed`
//! with eight spaces after it, `# reached     rung `. Anything else a caller
//! wants to say goes in as an extra `#` line, which `Verb::parse` ignores: it
//! matches exact token slices, and `["#", "epsilon", "0.07"]` is not a verb.

use gearmaster_console::{Console, Difficulty, Mode, Verb};

/// Where a tape replayed to, and how much of it the console refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Replayed {
    pub rung: usize,
    pub refusals: usize,
}

/// Feed a tape to a fresh console and see where it gets.
pub fn replay(seed: u64, mode: Mode, difficulty: Difficulty, tape: &[Verb]) -> Replayed {
    let mut c = Console::start(seed, mode, difficulty);
    let (mut rung, mut refusals) = (1usize, 0usize);
    for v in tape {
        if !c.apply(*v).ok {
            refusals += 1;
        }
        rung = rung.max(c.view().rung_shown);
    }
    Replayed { rung, refusals }
}

/// The file, header and all.
pub fn text(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    tape: &[Verb],
    reached: usize,
    notes: &[(&str, String)],
) -> String {
    let mut out = format!(
        "# seed        {:#018X}\n# mode        {:?}\n# difficulty  {:?}\n\
         # reached     rung {}\n# presses     {}\n",
        seed,
        mode,
        // `{:?}` and not `name()`: the debug spelling is `Medium`, which is
        // what every proof in `analysis/proofs/` says and what a person reading
        // one expects. `name()` shouts it. `gui::watch` compares case-blind so
        // both work, and a file format with two spellings is still a bug.
        difficulty,
        reached,
        tape.len()
    );
    for (k, v) in notes {
        out.push_str(&format!("# {k:<11} {v}\n"));
    }
    out.push_str(
        "#\n# Every line below is a key a person could press, and only the ones\n\
         # that stuck. Written by a trainer mid-run, so the presses include\n\
         # whatever exploration was on at the time - see the epsilon above\n\
         # before reading any of it as a policy's opinion.\n#\n",
    );
    out.push_str("\n");
    for v in tape {
        out.push_str(&v.line());
        out.push('\n');
    }
    out
}

/// Write it, if it replays. The `Err` is the sentence saying why not.
///
/// `reached` is what the run claims; the replay has to agree with it and refuse
/// nothing. Those are the two things `lab/tests/proofs.rs` asserts about a
/// committed proof, asked here before the file exists.
#[allow(clippy::too_many_arguments)]
pub fn write(
    dir: &str,
    name: &str,
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    tape: &[Verb],
    reached: usize,
    notes: &[(&str, String)],
) -> Result<String, String> {
    let r = replay(seed, mode, difficulty, tape);
    if r.refusals > 0 {
        return Err(format!("{name}: {} of {} keys were refused", r.refusals, tape.len()));
    }
    if r.rung != reached {
        return Err(format!("{name}: claims rung {reached} and replays to {}", r.rung));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
    let path = format!("{dir}/{name}.proof");
    std::fs::write(&path, text(seed, mode, difficulty, tape, reached, notes))
        .map_err(|e| format!("{path}: {e}"))?;
    Ok(path)
}

/// Every proof in a directory, by name, oldest first.
///
/// **By name and not by modification time.** A watcher picks the newest and a
/// trainer drops the oldest, and both want the same order; a clock is a worse
/// answer than a counter that is already in the filename.
pub fn listed(dir: &str) -> Vec<std::path::PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "proof"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Drop all but the newest `keep`. Returns how many went.
///
/// Four thousand episodes at one proof in twenty-five is a hundred and sixty
/// files, and the watcher is ten times slower than the trainer, so it will
/// never look at more than the last few. Keeping the rest is hoarding.
pub fn prune(dir: &str, keep: usize) -> usize {
    let all = listed(dir);
    let drop = all.len().saturating_sub(keep);
    for p in all.iter().take(drop) {
        std::fs::remove_file(p).ok();
    }
    drop
}
