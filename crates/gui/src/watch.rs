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

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::run::Run;
#[cfg(test)]
use gearmaster_engine::run::Phase;

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

    /// The whole tape, for a driver that plays it rather than watching it.
    ///
    /// Only `drive` wants this, and only a test drives.
    #[cfg(test)]
    pub fn tape(&self) -> &[Verb] {
        &self.verbs
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

/// Start whatever fight is standing in front of the run, if one is.
///
/// Returns whether one started, and leaves the log in `Run::log` for whoever
/// asked. The window builds a `Playback` out of it and the headless driver
/// below settles it - so *which* fight a `Fight` verb means is answered once
/// rather than twice.
///
/// A brawl an event arranged is the thing in the road, not a detour round it,
/// so it goes ahead of the rung's own creature.
pub fn start_fight(run: &mut Run) -> bool {
    if let Some(specs) = run.pending_brawl() {
        run.fight_party(&specs);
        return true;
    }
    if run.road_is_blocked().is_some() {
        return false;
    }
    run.fight_next();
    true
}

/// What one press did.
#[derive(Clone, Debug, Default)]
pub struct Pressed {
    /// Whether the menu carried the verb at all. `false` is a **divergence**:
    /// the tape is naming a key this run does not have, which means the run
    /// stopped being the one the tape was recorded from.
    pub offered: bool,
    /// Whether the press stuck.
    pub ok: bool,
    /// What the screen said about it, one line each.
    pub lines: Vec<String>,
    /// Whether it started a fight. The window has an animation to build; a
    /// headless driver has a settle to do.
    pub fighting: bool,
}

/// One press, the way the window presses it, with the drawing taken out.
///
/// The window is a **second reader** of a proof. The file's own verification
/// replays it through a `Console` (`lab::proof::write` refuses to write one
/// that does not), and the window puts fights through the fight arm instead, so
/// that the battle screen plays out. Two readers of one artefact is the shape
/// this repo keeps finding faults in, so the half of the window that is not
/// drawing lives here, where `drive` can walk it without a graphics context.
///
/// It does not settle. The window settles over the frames the animation takes;
/// `drive` settles at once, because it has no bar to advance.
pub fn press(run: &mut Run, v: Verb) -> Pressed {
    match v {
        Verb::Fight | Verb::FightParty => {
            let fighting = start_fight(run);
            Pressed { offered: true, ok: fighting, lines: Vec::new(), fighting }
        }
        other => {
            let held = std::mem::replace(run, Run::seeded(0));
            let mut c = Console::standing_in(held, 0);
            // **Offered before pressed.** A tape is only replayable while the
            // run matches the one that recorded it, and a verb the menu does
            // not carry names a piece this run has never owned - which panics
            // inside the registry rather than being refused. Asking first turns
            // a crash into a sentence saying where it parted company.
            let offered = c.menu().contains(&other);
            let mut p = Pressed { offered, ..Default::default() };
            if offered {
                let out = c.apply(other);
                p.ok = out.ok;
                p.lines = out.lines;
            }
            *run = c.into_run();
            p
        }
    }
}

/// Where a tape got, driven the way the window drives it.
///
/// The window itself never asks: it plays a tape a press at a time and draws
/// the answer. `Driven`, `drive` and `console_reference` are what a test asks,
/// which is why they are not in the shipped binary.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Driven {
    /// The deepest rung the run ever stood on, displayed - `run.rung + 1`,
    /// which is what `Console::view` calls `rung_shown` and what a proof's
    /// header claims. `CLAUDE.md` trap 9.
    pub deepest: usize,
    /// Presses the window would have refused. A proof's own verification
    /// requires none, so anything here is the window disagreeing with the file.
    pub refusals: usize,
    /// Presses whose verb the window's menu did not carry at all.
    pub unoffered: usize,
}

/// Play a whole tape into a fresh run, the way the window plays it.
#[cfg(test)]
pub fn drive(h: &Header, tape: &[Verb]) -> Driven {
    let mut run = Run::start(h.seed, h.mode, h.difficulty);
    let mut out = Driven { deepest: 1, refusals: 0, unoffered: 0 };
    for v in tape {
        // A scene is prose with a button and no verb clears it. The window
        // spends one beat on it before pressing anything else; here there are
        // no beats, so it goes now.
        run.pending_scene = None;
        let p = press(&mut run, *v);
        if p.fighting {
            // Everything the window does over the frames the animation takes.
            run.settle();
            if run.phase == Phase::Fighting {
                run.back_to_loadout();
            }
        }
        if !p.offered {
            out.unoffered += 1;
        }
        if !p.ok {
            out.refusals += 1;
        }
        out.deepest = out.deepest.max(run.rung + 1);
    }
    out
}

/// The same tape through a plain `Console`, which is what wrote the proof.
///
/// The reference the window is checked against. `lab::proof::replay` is this
/// function, and cannot be called from here: `crates/lab` is not a dependency
/// of the window and must not become one.
#[cfg(test)]
pub fn console_reference(h: &Header, tape: &[Verb]) -> Driven {
    let mut c = Console::start(h.seed, h.mode, h.difficulty);
    let mut out = Driven { deepest: 1, refusals: 0, unoffered: 0 };
    for v in tape {
        if !c.apply(*v).ok {
            out.refusals += 1;
        }
        out.deepest = out.deepest.max(c.view().rung_shown);
    }
    out
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
    /// What `Layout::build` would card, out of a run.
    ///
    /// The same expression `Layout::build` uses, kept here so the property
    /// below is about the list the window actually draws rather than about a
    /// list that resembles it.
    fn cards(run: &Run) -> Vec<gearmaster_engine::piece::PieceId> {
        run.inventory_groups().into_iter().filter_map(|g| g.first().copied()).collect()
    }

    /// Would drawing this card list against this run panic?
    fn any_card_is_gone(run: &Run, cards: &[gearmaster_engine::piece::PieceId]) -> bool {
        let reg = run.registry.clone();
        cards.iter().any(|id| {
            let id = *id;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reg.def(id))).is_err()
        })
    }

    /// **A card list built before a press does not always survive the press.**
    ///
    /// `Layout::build` cards `run.inventory_groups()` and the drawing resolves
    /// each card against `run.registry`, which panics on an id it does not
    /// hold. Almost every verb leaves an earlier card list valid. `undo` does
    /// not: it restores a whole `BoardSnapshot`, so an undone purchase makes
    /// the registry **smaller** and the card for the piece that was bought
    /// names an instance that no longer exists.
    ///
    /// A person never met it. Every hand-driven control on the loadout screen
    /// is read *after* the drawing, so their press lands on the next frame's
    /// layout. The watcher is the one hand that presses in the middle of a
    /// frame, and the window built its layout above the watcher - so it drew
    /// last frame's cards against this frame's registry and panicked in
    /// `PieceRegistry::instance`, measured at press 254 of 665 of
    /// `deep-rung13-239A554D7F922603`.
    ///
    /// This is the witness for why `Layout::build` sits *below* the watcher in
    /// `main.rs` and why nothing between there and the drawing may touch the
    /// run. It asserts that the hazard is real rather than that the ordering is
    /// right, because the ordering is a fact about a frame and this crate has
    /// no frame to ask.
    #[test]
    fn an_undone_purchase_leaves_an_earlier_card_list_naming_a_piece_that_is_gone() {
        let mut run = Run::start(0x1212, Mode::Grinder, Difficulty::Medium);
        // The cheapest shelf this run can afford, so the test is about undo
        // rather than about the shop's prices.
        let bought = (0..6).find(|&shelf| press(&mut run, Verb::Buy { shelf }).ok);
        let shelf = bought.expect("a starting purse buys something off a full shelf");

        let before = cards(&run);
        assert!(!before.is_empty(), "a bought piece goes in the tray");
        assert!(!any_card_is_gone(&run, &before), "the list is good the moment it is built");

        assert!(press(&mut run, Verb::Undo).ok, "a purchase is undoable");
        assert!(
            any_card_is_gone(&run, &before),
            "buying off shelf {shelf} and undoing it left every card resolvable, so this \
             test no longer witnesses anything - find out what changed before deleting it"
        );

        // And the list built *after* the press is what the window now draws.
        let after = cards(&run);
        assert!(!any_card_is_gone(&run, &after));
    }

    /// The window's path gets a committed proof where the console's does.
    ///
    /// A proof is written only if it replays through a `Console`
    /// (`lab::proof::write`), and the window is a **second reader** of that
    /// file: fights go through the fight arm so the battle screen plays out.
    /// Nothing checked that the two readers agreed until this - the window
    /// could not be driven at all without a graphics context, which is
    /// `CLAUDE.md` trap 46 in the one crate that has a window.
    ///
    /// Only what `.gitignore` keeps: a proof is as long as the run it proves
    /// and they are made on demand, so this walks whatever is on disk and says
    /// how many it found rather than pinning a count.
    ///
    /// `GEARMASTER_PROOFS=<dir>` points it somewhere else - at a trainer's
    /// whole watch directory, say, which is a hundred and seventy-seven tapes
    /// rather than one.
    #[test]
    fn a_proof_reaches_the_same_rung_through_the_window_as_through_the_console() {
        let dir = std::env::var("GEARMASTER_PROOFS")
            .unwrap_or_else(|_| "../../analysis/proofs".into());
        let mut played = 0;
        for path in proofs_in(&dir) {
            let Some((w, h)) = Watcher::load(&path) else { continue };
            // The long ones are whole fifty-rung runs - forty-five thousand
            // presses, and every fight in them simulated twice. One is the
            // evidence for a mission; this is a test that has to run every
            // time, so it takes the ones a person would wait for.
            if w.tape().len() > 4_000 {
                continue;
            }
            let want = console_reference(&h, w.tape());
            let got = drive(&h, w.tape());
            assert_eq!(
                got, want,
                "{path}: the window's path and the console's disagree about the same tape"
            );
            assert_eq!(got.refusals, 0, "{path}: a written proof refuses nothing");
            assert_eq!(got.unoffered, 0, "{path}: the window was offered every key on the tape");
            assert_eq!(
                got.deepest, h.reached,
                "{path}: the header claims a rung the window does not reach"
            );
            played += 1;
        }
        assert!(played > 0, "no proof on disk to drive - {dir} is empty");
    }

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

