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
}

/// What a proof's header says about the run it proves.
pub struct Header {
    pub seed: u64,
    pub mode: Mode,
    pub difficulty: Difficulty,
    pub reached: usize,
}

impl Watcher {
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
            Watcher { verbs, lines, at: 0, every, next_at: 0.0, paused: false, name },
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
