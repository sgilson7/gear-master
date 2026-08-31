//! Watching the pilot play.
//!
//! `GEARMASTER_WATCH=analysis/proofs/<file>.proof cargo run -p gearmaster-gui`
//!
//! A proof is `(seed, mode, difficulty, [verb])` and nothing else - the same
//! file `crates/lab`'s replay test reads. This presses those keys into the
//! window, one every so often, through the **same `Console` the agent uses**.
//! There is no second implementation of what a verb does, which is the whole
//! reason the window can be trusted to be showing what the agent did.
//!
//! Fights are the exception, and deliberately: a `Fight` verb is handed to
//! `begin_next_fight` instead, so the battle screen plays out exactly as it
//! does for a person. Watching a run means watching the fights.
//!
//! Keys, live only while watching: **space** pauses, **right arrow** steps one
//! press while paused, **up/down** change the pace.
//!
//! ## A directory, for a trainer that is still running
//!
//! `GEARMASTER_WATCH` also takes a **directory**. `qrow` writes a proof every
//! so many episodes into one (`QROW_WATCH`), and the window plays the newest.
//!
//! It cannot keep up and is not trying to. An episode is about 1.8 s of
//! training and about 18 s to replay at this pace, so the window is ten times
//! slower than the trainer by construction; what it does is **sample**. When an
//! episode ends it takes the newest proof on disk rather than the next one in
//! sequence, because the backlog is by definition stale and the question is
//! what the packer does *now*.
//!
//! And it finishes the episode it is playing first. A window that cut away
//! every time a new file landed would show a montage of openings, and the
//! reason to watch at all is to see a run.

use gearmaster_console::{Difficulty, Mode, Verb};

/// A transcript, part-way through.
pub struct Watcher {
    verbs: Vec<Verb>,
    lines: Vec<String>,
    at: usize,
    /// Seconds between presses.
    pub every: f64,
    pub next_at: f64,
    pub paused: bool,
    pub name: String,
    /// The file this came out of, so a directory can tell it from a newer one.
    pub path: String,
    /// The directory to look in for the next one, when there is one.
    pub dir: Option<String>,
}

/// What a proof's header says about the run it proves.
pub struct Header {
    pub seed: u64,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub reached: usize,
}

/// Every proof in a directory, sorted by name.
///
/// **By name and not by modification time.** `lab::proof` puts the episode
/// number in the filename, and a counter that is already in the name is a
/// better answer than a clock - two files written in the same second have an
/// order, and a copied directory keeps it.
pub fn proofs_in(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "proof"))
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Which proof a directory should be playing, given what is playing now.
///
/// The newest, and `None` when that is already the one in hand. A `current`
/// that is not in the list at all has been pruned out from under the window,
/// which is not an error: the trainer keeps the last few and the window is
/// slow, so falling off the end is the expected case rather than a surprise.
///
/// Pure over names, so the interesting part is testable without a filesystem.
pub fn next_in(entries: &[String], current: Option<&str>) -> Option<String> {
    let newest = entries.last()?;
    match current {
        Some(c) if c == newest => None,
        _ => Some(newest.clone()),
    }
}

impl Watcher {
    /// Open a proof, or a directory of them.
    ///
    /// A directory plays its newest and remembers where it came from; a file
    /// plays once and has no next, which is what every existing caller wants.
    pub fn open(path: &str) -> Option<(Watcher, Header)> {
        if std::path::Path::new(path).is_dir() {
            let newest = next_in(&proofs_in(path), None)?;
            let (mut w, h) = Watcher::load(&newest)?;
            w.dir = Some(path.to_string());
            return Some((w, h));
        }
        Watcher::load(path)
    }

    /// The next proof to play, if this one is finished and a newer exists.
    ///
    /// `None` while it is still playing: the episode gets to end. See the
    /// module note.
    pub fn next(&self) -> Option<String> {
        if !self.done() {
            return None;
        }
        let dir = self.dir.as_ref()?;
        next_in(&proofs_in(dir), Some(&self.path))
    }

    pub fn load(path: &str) -> Option<(Watcher, Header)> {
        let text = std::fs::read_to_string(path).ok()?;
        let field = |k: &str| -> Option<String> {
            text.lines()
                .find(|l| l.starts_with(&format!("# {}", k)))
                .map(|l| l[k.len() + 2..].trim().to_string())
        };
        let seed = field("seed")
            .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())?;
        let mode = match field("mode").as_deref() {
            Some("Rogue") => Mode::Rogue,
            _ => Mode::Grinder,
        };
        let difficulty = Difficulty::ALL
            .iter()
            .copied()
            .find(|d| field("difficulty").is_some_and(|v| d.name().eq_ignore_ascii_case(&v)))
            .unwrap_or(Difficulty::Medium);
        let reached = field("reached")
            .and_then(|v| v.split_whitespace().nth(1).and_then(|n| n.parse().ok()))
            .unwrap_or(0);

        let mut verbs = Vec::new();
        let mut lines = Vec::new();
        for l in text.lines() {
            if let Some(v) = Verb::parse(l) {
                verbs.push(v);
                lines.push(l.trim().to_string());
            }
        }
        if verbs.is_empty() {
            return None;
        }
        let every = std::env::var("GEARMASTER_WATCH_MS")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(90.0)
            / 1000.0;
        let name = std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        Some((
            Watcher {
                verbs,
                lines,
                at: 0,
                every,
                next_at: 0.0,
                paused: false,
                name,
                path: path.to_string(),
                dir: None,
            },
            Header { seed, mode, difficulty, reached },
        ))
    }

    pub fn done(&self) -> bool {
        self.at >= self.verbs.len()
    }

    pub fn peek(&self) -> Option<Verb> {
        self.verbs.get(self.at).copied()
    }

    pub fn advance(&mut self) {
        self.at += 1;
    }

    pub fn at(&self) -> usize {
        self.at
    }

    pub fn len(&self) -> usize {
        self.verbs.len()
    }

    /// The last few presses, newest last, for a strip that shows what it did.
    pub fn recent(&self, n: usize) -> Vec<&str> {
        let from = self.at.saturating_sub(n);
        self.lines[from..self.at].iter().map(|s| s.as_str()).collect()
    }

    /// Is it time for the next press?
    pub fn ready(&self, now: f64) -> bool {
        !self.paused && !self.done() && now >= self.next_at
    }

    pub fn schedule(&mut self, now: f64) {
        self.next_at = now + self.every;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo build` does not compile this module - `CLAUDE.md` trap 14.
    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_directory_plays_its_newest_and_then_stays_there() {
        let dir = names(&["w/ep-000000.proof", "w/ep-000025.proof", "w/ep-000050.proof"]);
        assert_eq!(next_in(&dir, None).as_deref(), Some("w/ep-000050.proof"));
        // Already on the newest: nothing to move to, and that has to be `None`
        // rather than the same file again, or the window restarts the episode
        // it has just finished for ever.
        assert_eq!(next_in(&dir, Some("w/ep-000050.proof")), None);
        // Behind: catch up to the newest, not to the next one along. The window
        // is ten times slower than the trainer and the backlog is stale.
        assert_eq!(next_in(&dir, Some("w/ep-000000.proof")).as_deref(), Some("w/ep-000050.proof"));
    }

    /// The expected case, not an error.
    ///
    /// `lab::proof::prune` keeps the last few and the window is slow, so the
    /// file being played is routinely deleted out from under it. That must read
    /// as "take the newest", not as "stop".
    #[test]
    fn a_proof_pruned_out_from_under_the_window_is_not_an_error() {
        let dir = names(&["w/ep-000075.proof", "w/ep-000100.proof"]);
        assert_eq!(next_in(&dir, Some("w/ep-000000.proof")).as_deref(), Some("w/ep-000100.proof"));
    }

    #[test]
    fn an_empty_or_missing_directory_has_nothing_to_play() {
        assert_eq!(next_in(&[], None), None);
        assert_eq!(next_in(&[], Some("w/ep-000000.proof")), None);
        assert!(proofs_in("/no/such/directory/anywhere").is_empty());
    }

    /// Ordering is by name, and the name carries the episode number.
    ///
    /// Zero-padded by `lab::proof`, so lexicographic order is numeric order.
    /// If that padding ever goes, `ep-1000` sorts before `ep-99` and the window
    /// plays the wrong episode while looking perfectly healthy.
    #[test]
    fn the_newest_is_the_highest_episode_and_not_the_last_written() {
        let dir = names(&["w/ep-000099.proof", "w/ep-001000.proof"]);
        assert_eq!(next_in(&dir, None).as_deref(), Some("w/ep-001000.proof"));
        let unpadded = names(&["w/ep-1000.proof", "w/ep-99.proof"]);
        assert_eq!(
            next_in(&unpadded, None).as_deref(),
            Some("w/ep-99.proof"),
            "unpadded names sort wrong - this is why `lab::proof` pads to six"
        );
    }
}
