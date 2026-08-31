//! Watching a trainer's log as a curve.
//!
//! `GEARMASTER_CURVE=analysis/nets/qrow-r13.log cargo run -p gearmaster-gui`
//!
//! A trainer prints a line every block and then runs for two hours. This reads
//! that file, re-reads it while it grows, and draws what it says - so a run in
//! progress can be looked at rather than tailed.
//!
//! **It draws the mean and the maximum on the same axes on purpose.** The
//! column `qrow` printed for its whole life was a maximum over a hundred
//! episodes, and depth in this game is heavy-tailed enough that the maximum
//! wanders between 3 and 13 while the policy under it does not move at all.
//! A whole handoff was written about that wander. The two series together are
//! the picture of the difference, and pointing this at a log from before the
//! mean existed - `analysis/nets/qrow-r12.log` - draws the maximum alone,
//! which is exactly what was being read.
//!
//! Nothing here knows about macroquad. The parsing and the scaling are plain
//! functions with tests; `main.rs` does the drawing, because a layout computed
//! between `draw_*` calls cannot be tested (`CLAUDE.md` trap 32).

/// One block of episodes, as a trainer printed it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub episode: usize,
    pub eps: f32,
    pub buffer: usize,
    /// The block's mean depth.
    ///
    /// `None` in a log written before the trainer printed one, which is every
    /// log in `analysis/nets/`. A missing mean is drawn as a missing series
    /// rather than as a zero, because the run had one and nobody wrote it down.
    pub mean: Option<f32>,
    /// The deepest single run in the block. A maximum, and read as one.
    pub deepest: usize,
    pub ever: usize,
    pub spread: f32,
}

/// A trainer's log, and where it was read from.
pub struct Curve {
    path: String,
    pub blocks: Vec<Block>,
    /// The trainer's own settings line, whatever it says.
    pub settings: String,
    /// What the written control does through the same loop, if the log says.
    ///
    /// This is the only reference line worth drawing: it is measured by the
    /// trainer itself, in the same harness, before a gradient is taken.
    pub control_mean: Option<f32>,
    pub control_best: Option<usize>,
    /// The last line, once there is one.
    pub finished: Option<String>,
    /// When the file was last read, in the caller's clock.
    pub read_at: f64,
    /// Seconds between re-reads. A log is a few dozen lines; this is cheap.
    pub every: f64,
}

/// The token after a key, where the key may be more than one word.
///
/// `mean rung  1.95` and `deepest rung   4` both put a number after `rung`, and
/// they mean different things, so a caller has to be able to ask for the pair.
fn after<'a>(tokens: &[&'a str], key: &str) -> Option<&'a str> {
    let want: Vec<&str> = key.split_whitespace().collect();
    if tokens.len() <= want.len() {
        return None;
    }
    tokens.windows(want.len() + 1).find(|w| w[..want.len()] == want[..]).map(|w| w[want.len()])
}

/// The same, as a number, ignoring whatever punctuation is stuck to it.
///
/// `(ever   7)` and `mean rung 6.0,` are both real tokens in these logs.
fn num<T: std::str::FromStr>(tokens: &[&str], key: &str) -> Option<T> {
    let t = after(tokens, key)?;
    t.trim_matches(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).parse().ok()
}

/// Read a log's text. Nothing here fails: a half-written line is skipped and
/// picked up on the next read, which is what following a live file means.
pub fn parse(text: &str) -> (Vec<Block>, String, Option<f32>, Option<usize>, Option<String>) {
    let mut blocks = Vec::new();
    let mut settings = String::new();
    let (mut cmean, mut cbest, mut finished) = (None, None, None);
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        if line.contains("gamma") && settings.is_empty() {
            settings = line.trim().to_string();
            continue;
        }
        if line.contains("the written control") {
            cmean = num(&tokens, "rung");
            cbest = num(&tokens, "best");
            continue;
        }
        if line.trim_start().starts_with("wrote ") {
            finished = Some(line.trim().to_string());
            continue;
        }
        if tokens[0] != "episode" {
            continue;
        }
        // A block line, and every field of it optional: a log being written
        // while it is read can end mid-number.
        let (Some(episode), Some(spread)) = (num(&tokens, "episode"), num(&tokens, "spread"))
        else {
            continue;
        };
        blocks.push(Block {
            episode,
            eps: num(&tokens, "eps").unwrap_or(0.0),
            buffer: num(&tokens, "buffer").unwrap_or(0),
            mean: num(&tokens, "mean rung"),
            // The old format wrote `deepest rung 4`; the new one writes
            // `mean rung 1.95   deepest 4`. Ask for the pair first.
            deepest: num(&tokens, "deepest rung").or_else(|| num(&tokens, "deepest")).unwrap_or(0),
            ever: num(&tokens, "(ever").unwrap_or(0),
            spread,
        });
    }
    (blocks, settings, cmean, cbest, finished)
}

impl Curve {
    pub fn open(path: &str) -> Option<Curve> {
        let text = std::fs::read_to_string(path).ok()?;
        let (blocks, settings, control_mean, control_best, finished) = parse(&text);
        Some(Curve {
            path: path.to_string(),
            blocks,
            settings,
            control_mean,
            control_best,
            finished,
            read_at: 0.0,
            every: std::env::var("GEARMASTER_CURVE_MS")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1000.0)
                / 1000.0,
        })
    }

    /// Re-read if it is time. True when the file was read this call.
    pub fn follow(&mut self, now: f64) -> bool {
        if now < self.read_at + self.every {
            return false;
        }
        self.read_at = now;
        if let Ok(text) = std::fs::read_to_string(&self.path) {
            let (blocks, settings, cmean, cbest, finished) = parse(&text);
            self.blocks = blocks;
            if !settings.is_empty() {
                self.settings = settings;
            }
            self.control_mean = cmean;
            self.control_best = cbest;
            self.finished = finished;
        }
        true
    }

    /// What to call it on screen.
    pub fn name(&self) -> String {
        std::path::Path::new(&self.path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.clone())
    }

    /// The newest block, which is what the footer reads.
    pub fn last(&self) -> Option<&Block> {
        self.blocks.last()
    }

    /// Whether this log ever carried a mean, which decides what can be drawn.
    pub fn has_mean(&self) -> bool {
        self.blocks.iter().any(|b| b.mean.is_some())
    }
}

/// What the axes have to cover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub first: usize,
    pub last: usize,
    pub top: f32,
}

/// The axes for a set of blocks, with the control's line inside them.
///
/// The top is rounded up to an even rung and never below four, so an early run
/// that has only reached rung two is not drawn as though it filled the window.
pub fn bounds(blocks: &[Block], control: Option<f32>) -> Option<Bounds> {
    if blocks.is_empty() {
        return None;
    }
    let mut top = control.unwrap_or(0.0);
    for b in blocks {
        top = top.max(b.deepest as f32);
        if let Some(m) = b.mean {
            top = top.max(m);
        }
    }
    let top = ((top / 2.0).ceil() * 2.0).max(4.0);
    Some(Bounds {
        first: blocks.first().expect("not empty").episode,
        last: blocks.last().expect("not empty").episode,
        top,
    })
}

/// Where one point sits inside a rectangle, in the caller's units.
///
/// Y grows downward, which is the window's convention and not the graph's, so
/// the value is subtracted rather than added.
pub fn at(b: &Bounds, episode: usize, value: f32, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let span = b.last.saturating_sub(b.first).max(1) as f32;
    let fx = episode.saturating_sub(b.first) as f32 / span;
    let fy = (value / b.top).clamp(0.0, 1.0);
    (x + fx.clamp(0.0, 1.0) * w, y + h - fy * h)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo build` does not compile this module - `CLAUDE.md` trap 14.
    /// `cargo test -p gearmaster-gui` does.
    const NEW: &str = "\
  the row: one episode is one run, from rung one until it dies
  Rogue   lr 0.05   updates 24   huber knee 120   gamma 0.999
  the written control through this loop: mean rung 6.0, best 13
  episode     0   eps 1.00   buffer     98   mean rung  1.00   deepest   1 (ever   1)   spread  0.068
  episode   100   eps 0.96   buffer  12003   mean rung  1.75   deepest   4 (ever   4)   spread  0.072
wrote runs/quartermaster_row.txt (best block, mean rung 2.05) and runs/quartermaster_row_last.txt (final)
";

    /// The format every log in `analysis/nets/` is in: a maximum and no mean.
    const OLD: &str = "\
  Rogue   lr 0.05   updates 24   huber knee 120   gamma 0.999
  the written control through this loop: mean rung 6.0, best 13
  episode  2600   eps 0.07   buffer  70000   deepest rung  11 (ever  11)   spread  0.210
  episode  3300   eps 0.05   buffer  67035   deepest rung   3 (ever  11)   spread  0.255
";

    #[test]
    fn a_new_log_carries_a_mean_and_a_maximum() {
        let (blocks, settings, cmean, cbest, finished) = parse(NEW);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].episode, 100);
        assert_eq!(blocks[1].mean, Some(1.75));
        assert_eq!(blocks[1].deepest, 4);
        assert_eq!(blocks[1].ever, 4);
        assert_eq!(blocks[1].buffer, 12003);
        assert!((blocks[1].eps - 0.96).abs() < 1e-6);
        assert!((blocks[1].spread - 0.072).abs() < 1e-6);
        assert!(settings.contains("gamma 0.999"));
        assert_eq!(cmean, Some(6.0), "the control's mean is the reference line");
        assert_eq!(cbest, Some(13));
        assert!(finished.is_some_and(|f| f.starts_with("wrote ")));
    }

    /// The distinction this whole module exists for.
    ///
    /// `deepest rung 11` is a maximum and must not be read as a mean, or the
    /// picture would show a policy reaching rung 11 - which is the reading that
    /// cost this mission a handoff.
    #[test]
    fn an_old_log_has_a_maximum_and_no_mean_at_all() {
        let (blocks, _, cmean, _, finished) = parse(OLD);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].deepest, 11, "`deepest rung 11` is the maximum");
        assert_eq!(blocks[0].mean, None, "and there is no mean in this format");
        assert_eq!(blocks[1].deepest, 3);
        assert_eq!(cmean, Some(6.0));
        assert!(finished.is_none(), "this log was cut off before its last line");
    }

    #[test]
    fn a_half_written_line_is_skipped_rather_than_guessed() {
        let text = format!("{NEW}  episode   200   eps 0.93   buffer  242");
        let (blocks, ..) = parse(&text);
        assert_eq!(blocks.len(), 2, "the line without a spread is not a block yet");
    }

    #[test]
    fn the_axes_hold_the_control_line_and_never_squash_an_early_run() {
        let (blocks, ..) = parse(NEW);
        let b = bounds(&blocks, Some(6.0)).expect("blocks");
        assert_eq!(b.first, 0);
        assert_eq!(b.last, 100);
        assert!(b.top >= 6.0, "the control's 6.0 has to fit inside the picture");

        // A run that has only reached rung two still gets a four-rung axis.
        let low = [Block {
            episode: 0,
            eps: 1.0,
            buffer: 0,
            mean: Some(1.0),
            deepest: 2,
            ever: 2,
            spread: 0.0,
        }];
        assert_eq!(bounds(&low, None).expect("a block").top, 4.0);
        assert_eq!(bounds(&[], Some(6.0)), None, "nothing to draw yet");
    }

    #[test]
    fn a_point_lands_where_the_rectangle_says() {
        let b = Bounds { first: 0, last: 100, top: 10.0 };
        // The origin is the bottom left, because y grows downward on a screen.
        assert_eq!(at(&b, 0, 0.0, 10.0, 20.0, 200.0, 100.0), (10.0, 120.0));
        assert_eq!(at(&b, 100, 10.0, 10.0, 20.0, 200.0, 100.0), (210.0, 20.0));
        assert_eq!(at(&b, 50, 5.0, 10.0, 20.0, 200.0, 100.0), (110.0, 70.0));
        // A value past the top is clamped rather than drawn off the panel.
        assert_eq!(at(&b, 100, 40.0, 10.0, 20.0, 200.0, 100.0).1, 20.0);
    }

    /// One block is a single point and a span of zero, which must not divide.
    #[test]
    fn one_block_does_not_divide_by_zero() {
        let one = [Block {
            episode: 700,
            eps: 0.75,
            buffer: 1,
            mean: Some(1.92),
            deepest: 4,
            ever: 7,
            spread: 0.119,
        }];
        let b = bounds(&one, None).expect("a block");
        let (x, _) = at(&b, 700, 1.92, 0.0, 0.0, 100.0, 50.0);
        assert!(x.is_finite(), "a span of zero is one, not a division by it");
    }
}
