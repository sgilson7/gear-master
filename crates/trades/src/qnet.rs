//! A Q network, read as plain numbers.
//!
//! `Q(state, action)` rather than one output a action: the menu is 17 to 545
//! verbs and changes shape every step, so a head with a neuron per action does
//! not apply. The network scores **a pair** - the board, and one candidate
//! move - and the agent takes the argmax over whatever is legal. That is the
//! standard answer for a variable action space and it is why `feature::pair`
//! exists.
//!
//! Training lives in `gearmaster-lab` behind `--features nn` and writes six
//! matrices of floats. This reads them and multiplies them out, so **the agent
//! links no framework** and `cargo test --workspace` compiles none of it.
//! Training is privileged; acting is not.

use crate::brief::Brief;
use crate::feature::{self, PAIR};
use gearmaster_console::view::View;
use gearmaster_console::Verb;

/// Three layers, and the width they were trained at.
pub struct QNet {
    w1: Vec<f32>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: Vec<f32>,
    w3: Vec<f32>,
    b3: f32,
    hidden: usize,
    /// How many numbers the pair was when this file was written.
    ///
    /// **Read out of the file rather than assumed.** A net is keyed to the
    /// feature vector of the day it was saved, and this repo widens that
    /// vector: `BOARD` went from 30 to 270 in one commit and every checkpoint
    /// in `analysis/nets/` stopped loading the same afternoon, silently,
    /// because `parse` compared what it read against what the build happened
    /// to compile with. It compares the file against itself now, and the
    /// caller that needs a particular width asks for it by name.
    pair: usize,
    /// What the trainer said the pair *meant*, when it said anything.
    ///
    /// A width is not a version. A road net is saved at `feature::PAIR`,
    /// because `qroad` pads a road pair up to it before training, so the file's
    /// width is a fact about the *packing* vector of that day and says nothing
    /// about which road columns the net actually read. Two road nets on the
    /// shelf are 70 wide for that reason and the road pair is 64.
    ///
    /// So a trainer stamps what it fed the net, and a file that carries no
    /// stamp is a file from before this line: readable, and refused by
    /// `load_at` unless its raw width happens to be what the caller wants.
    declared: Option<usize>,
}

fn row(text: &str, name: &str) -> Vec<f32> {
    text.lines()
        .find(|l| l.starts_with(name) && l.as_bytes().get(name.len()) == Some(&b' '))
        .map(|l| l[name.len()..].split_whitespace().filter_map(|v| v.parse().ok()).collect())
        .unwrap_or_default()
}

impl QNet {
    /// The layers, for a harness that wants to look at them.
    ///
    /// Read-only, by reference, and with each layer's fan-in beside it so a
    /// caller can compare what a weight is now against what it was drawn as -
    /// `init` uses `sqrt(2/fan_in)`, and a bias is drawn as exactly zero.
    ///
    /// Nothing an agent does reads these; it calls `q`. This exists because
    /// "has this network learned anything" is a question about the numbers
    /// rather than about the behaviour, and six milestones of this mission
    /// were spent reading a behaviour and guessing.
    pub fn layers(&self) -> Vec<(&'static str, &[f32], usize)> {
        vec![
            ("w1", &self.w1, self.pair),
            ("b1", &self.b1, 0),
            ("w2", &self.w2, self.hidden),
            ("b2", &self.b2, 0),
            ("w3", &self.w3, self.hidden),
            ("b3", std::slice::from_ref(&self.b3), 0),
        ]
    }

    pub fn parse(text: &str) -> Option<QNet> {
        QNet::read(text).ok()
    }

    /// The same, saying what is wrong when it will not.
    ///
    /// Six milestones of this mission were spent reading a behaviour and
    /// guessing, and a whole afternoon of them was spent reading four
    /// diagnostics that had loaded nothing at all and said `did not load`.
    /// A refusal is worth as much as the thing it refuses and only if it says
    /// which number was wrong.
    pub fn read(text: &str) -> Result<QNet, String> {
        let (w1, b1, w2, b2, w3, b3) = (
            row(text, "w1"),
            row(text, "b1"),
            row(text, "w2"),
            row(text, "b2"),
            row(text, "w3"),
            row(text, "b3"),
        );
        let hidden = b1.len();
        if hidden == 0 {
            return Err("no b1 row, so there is no hidden width to read".into());
        }
        if w1.is_empty() || w1.len() % hidden != 0 {
            return Err(format!(
                "w1 is {} long, which is not a whole number of rows of {hidden}",
                w1.len()
            ));
        }
        let pair = w1.len() / hidden;
        let declared = row(text, "pair").first().map(|v| *v as usize);
        if let Some(d) = declared {
            if d > pair {
                return Err(format!(
                    "the stamp says the pair is {d} and there are only {pair} rows of weights"
                ));
            }
        }
        for (name, got, want) in
            [("b2", b2.len(), hidden), ("w2", w2.len(), hidden * hidden), ("w3", w3.len(), hidden)]
        {
            if got != want {
                return Err(format!("{name} is {got} long against a hidden width of {hidden}, which wants {want}"));
            }
        }
        if b3.len() != 1 {
            return Err(format!("b3 is {} long and the output is one number", b3.len()));
        }
        Ok(QNet { w1, b1, w2, b2, w3, b3: b3[0], hidden, pair, declared })
    }

    /// Load a **packing** net: one this build can feed a board and a placement.
    ///
    /// Strict on purpose, and strict in exactly the way it always was. `parse`
    /// used to refuse anything that was not `feature::PAIR` wide and now reads
    /// a file at whatever width it was written, which is what lets a road net
    /// be inspected - but it also means a caller that loads a stale checkpoint
    /// and calls `q` would score the first seventy columns of a three-hundred
    /// and fifteen number question and never know. So the door most callers go
    /// through keeps the check, and a caller that wants a road net names the
    /// width it wants through `load_at`.
    pub fn load(path: &str) -> Option<QNet> {
        QNet::load_at(path, PAIR).ok()
    }

    /// Load a net that has to be a given width, and say why not when it is not.
    ///
    /// The width a caller wants is a fact about what it is going to feed the
    /// net - `feature::PAIR` for a board and a placement, `pathfinder::PAIR`
    /// for a road and a step - and it is not a fact about the file. Anything
    /// that loads a checkpoint to *play* with should come through here, so a
    /// net saved against a dead feature vector is refused in a sentence rather
    /// than quietly scoring the first few hundred numbers of a different
    /// question.
    pub fn load_at(path: &str, want: usize) -> Result<QNet, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let net = QNet::read(&text).map_err(|e| format!("{path}: {e}"))?;
        match net.declared {
            Some(d) if d == want => Ok(net),
            Some(d) => Err(format!(
                "{path}: stamped as a pair of {d}, and this build wants {want} - \
                 the net was trained against a different feature vector"
            )),
            // No stamp, so the raw width is all there is to go on. It is the
            // right question for a packing net, which is stored at the width it
            // reads, and only half of one for a road net.
            None if net.pair == want => Ok(net),
            None => Err(format!(
                "{path}: {} wide and unstamped, and this build wants {want} - \
                 the net was saved against a different feature vector",
                net.pair
            )),
        }
    }

    /// How many numbers this net reads: the rows of `w1`, not its columns.
    pub fn width(&self) -> usize {
        self.pair
    }

    /// What the trainer stamped the pair as, if it stamped one at all.
    pub fn declared(&self) -> Option<usize> {
        self.declared
    }

    /// What this pair is worth.
    pub fn q(&self, x: &[f32; PAIR]) -> f32 {
        self.eval(x)
    }

    /// The arithmetic, over as much of `x` as this net is wide.
    ///
    /// An input shorter than the net is read as though the rest were zero,
    /// which is not a convenience: it is what lets a road pair of 64 numbers
    /// be scored by a net stored at 315, and it is what `q_pair` used to do by
    /// hand with a buffer of its own.
    pub(crate) fn eval(&self, x: &[f32]) -> f32 {
        let n = self.pair.min(x.len());
        let mut h1 = vec![0.0f32; self.hidden];
        for j in 0..self.hidden {
            let mut a = self.b1[j];
            for (i, xi) in x[..n].iter().enumerate() {
                a += xi * self.w1[i * self.hidden + j];
            }
            h1[j] = a.max(0.0);
        }
        let mut h2 = vec![0.0f32; self.hidden];
        for j in 0..self.hidden {
            let mut a = self.b2[j];
            for (i, hi) in h1.iter().enumerate() {
                a += hi * self.w2[i * self.hidden + j];
            }
            h2[j] = a.max(0.0);
        }
        let mut out = self.b3;
        for (i, hi) in h2.iter().enumerate() {
            out += hi * self.w3[i];
        }
        out
    }

    /// Score every legal move against this board, for a given brief.
    pub fn rank(&self, v: &View, moves: &[Verb], w: &Brief) -> Vec<f32> {
        let b = feature::briefed(&feature::board(v), w);
        moves.iter().map(|&m| self.q(&feature::pair(&b, &feature::mv(v, m)))).collect()
    }

    /// The best legal move, and what it is worth.
    pub fn best(&self, v: &View, moves: &[Verb], w: &Brief) -> Option<(usize, f32)> {
        let scores = self.rank(v, moves, w);
        scores
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}
