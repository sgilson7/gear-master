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
    /// The value tower, when this net was trained as a dueling one.
    duel: Option<Duel>,
}

/// The second tower of a dueling net, and where the state stops.
///
/// **Why a dueling net here, and why this form of one.** `analysis/the-action-gap.md`
/// measures the gap between the best action and the next best at 0.15% to 2.1%
/// of the state's own value, and the ablation there says two of 321 inputs move
/// the answer more than the other 319 together. That is a `Q` which is very
/// nearly a `V`, and it is the situation the dueling architecture was written
/// for: separate the two, and the action stream stops having to reproduce the
/// state stream inside every estimate before it can say anything about a move.
///
/// The usual identifiability constraint subtracts the mean advantage over the
/// legal menu, and the menu is exactly what a replay buffer here does not keep -
/// `Trans` holds the chosen pair and the *next* state's candidates, and nothing
/// else. So the anchor is a **reference action** instead: the all-zero move,
/// which is not an invention but the encoding `Move::Done` already has.
///
/// ```text
///   Q(s,a) = V(s) + A(s,a) - A(s, done)
/// ```
///
/// It is identifiable, it costs two extra forward passes for a whole menu
/// rather than one per candidate, and it anchors on the one action this game's
/// own measurements are about: `Q(s, done)` is exactly `V(s)`, and every other
/// key is priced as what it is worth over stopping here.
struct Duel {
    /// Where the move band starts, read out of the file. The state is
    /// everything before it. A width is not a version and neither is a split.
    split: usize,
    hidden: usize,
    w1: Vec<f32>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: Vec<f32>,
    w3: Vec<f32>,
    b3: f32,
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
        let mut out = vec![
            ("w1", &self.w1[..], self.pair),
            ("b1", &self.b1[..], 0),
            ("w2", &self.w2[..], self.hidden),
            ("b2", &self.b2[..], 0),
            ("w3", &self.w3[..], self.hidden),
            ("b3", std::slice::from_ref(&self.b3), 0),
        ];
        // The advantage tower stays first, because every caller that indexes
        // this reads `[0]` for the pair-wide first layer and `[1]` for the
        // hidden width. A value tower is six more rows on the end.
        if let Some(d) = &self.duel {
            out.extend([
                ("v1", &d.w1[..], d.split),
                ("vb1", &d.b1[..], 0),
                ("v2", &d.w2[..], d.hidden),
                ("vb2", &d.b2[..], 0),
                ("v3", &d.w3[..], d.hidden),
                ("vb3", std::slice::from_ref(&d.b3), 0),
            ]);
        }
        out
    }

    /// Whether this net prices a move as an advantage over stopping.
    pub fn is_dueling(&self) -> bool {
        self.duel.is_some()
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
        // The value tower, if there is one. Absent is the ordinary case and
        // not an error: every net written before the dueling arm is a plain
        // one, and a plain one is what `Duel: None` means.
        let (v1, vb1, v2, vb2, v3, vb3) = (
            row(text, "v1"),
            row(text, "vb1"),
            row(text, "v2"),
            row(text, "vb2"),
            row(text, "v3"),
            row(text, "vb3"),
        );
        let duel = if vb1.is_empty() {
            None
        } else {
            let vh = vb1.len();
            let Some(split) = row(text, "split").first().map(|v| *v as usize) else {
                return Err("there is a value tower and no `split` row saying where the state ends"
                    .into());
            };
            if split == 0 || split > pair {
                return Err(format!("the split is {split} and the pair is {pair}"));
            }
            if v1.len() != split * vh {
                return Err(format!(
                    "v1 is {} long against a split of {split} and a hidden width of {vh}",
                    v1.len()
                ));
            }
            for (name, got, want) in
                [("vb2", vb2.len(), vh), ("v2", v2.len(), vh * vh), ("v3", v3.len(), vh)]
            {
                if got != want {
                    return Err(format!(
                        "{name} is {got} long against a value hidden width of {vh}, which wants {want}"
                    ));
                }
            }
            if vb3.len() != 1 {
                return Err(format!("vb3 is {} long and the value is one number", vb3.len()));
            }
            Some(Duel {
                split,
                hidden: vh,
                w1: v1,
                b1: vb1,
                w2: v2,
                b2: vb2,
                w3: v3,
                b3: vb3[0],
            })
        };
        Ok(QNet { w1, b1, w2, b2, w3, b3: b3[0], hidden, pair, declared, duel })
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

    /// What each of a set of pairs that **share a state** is worth.
    ///
    /// A menu is one board and many moves, and so is a bootstrap's candidate
    /// set. For a plain net that is no different from scoring them one at a
    /// time; for a dueling one the two state terms - `V(s)` and the advantage
    /// of stopping - are the same for every candidate, so they are computed
    /// once rather than once a key. Sixteen candidates cost eighteen forward
    /// passes rather than forty-eight.
    ///
    /// The caller owes it the shared state. Handing this pairs from different
    /// boards prices all of them against the first one's.
    pub fn q_set(&self, xs: &[[f32; PAIR]]) -> Vec<f32> {
        let Some(d) = &self.duel else {
            return xs.iter().map(|x| self.eval(x)).collect();
        };
        let Some(first) = xs.first() else { return Vec::new() };
        let mut anchor = *first;
        anchor[d.split..].fill(0.0);
        let base = tower(&anchor[..d.split], &d.w1, &d.b1, &d.w2, &d.b2, &d.w3, d.b3, d.hidden)
            - self.advantage(&anchor);
        xs.iter().map(|x| base + self.advantage(x)).collect()
    }

    /// The two halves of a dueling net's answer, for a diagnostic.
    ///
    /// `(V(s), A(s,a) - A(s, done))`, which sum to `q(x)`. `None` for a plain
    /// net, where there is no decomposition to report and the whole of the
    /// answer is one number.
    ///
    /// This exists because "did the split work" is a question about the two
    /// magnitudes and not about the behaviour: an advantage stream that stayed
    /// at nothing is the same failure in a new shape, and only printing both
    /// tells that from a policy that is merely bad.
    pub fn parts(&self, x: &[f32; PAIR]) -> Option<(f32, f32)> {
        let d = self.duel.as_ref()?;
        let mut anchor = *x;
        anchor[d.split..].fill(0.0);
        let v = tower(&anchor[..d.split], &d.w1, &d.b1, &d.w2, &d.b2, &d.w3, d.b3, d.hidden);
        Some((v, self.advantage(x) - self.advantage(&anchor)))
    }

    /// The advantage tower alone, over as much of `x` as it is wide.
    fn advantage(&self, x: &[f32]) -> f32 {
        let n = self.pair.min(x.len());
        tower(&x[..n], &self.w1, &self.b1, &self.w2, &self.b2, &self.w3, self.b3, self.hidden)
    }

    /// The arithmetic, over as much of `x` as this net is wide.
    ///
    /// An input shorter than the net is read as though the rest were zero,
    /// which is not a convenience: it is what lets a road pair of 64 numbers
    /// be scored by a net stored at 315, and it is what `q_pair` used to do by
    /// hand with a buffer of its own.
    pub(crate) fn eval(&self, x: &[f32]) -> f32 {
        let a = self.advantage(x);
        let Some(d) = &self.duel else { return a };
        // `Q(s,a) = V(s) + A(s,a) - A(s, done)`, where "done" is the all-zero
        // move the console already encodes. See `Duel`.
        let mut anchor = [0.0f32; PAIR];
        let n = d.split.min(x.len());
        anchor[..n].copy_from_slice(&x[..n]);
        let v = tower(&anchor[..d.split], &d.w1, &d.b1, &d.w2, &d.b2, &d.w3, d.b3, d.hidden);
        v + a - self.advantage(&anchor)
    }

    /// Score every legal move against this board, for a given brief.
    ///
    /// Through `q_set`, because a menu is one board and many moves and a
    /// dueling net's state terms are the same for all of them.
    pub fn rank(&self, v: &View, moves: &[Verb], w: &Brief) -> Vec<f32> {
        let b = feature::briefed(&feature::board(v), w);
        let pairs: Vec<[f32; PAIR]> =
            moves.iter().map(|&m| feature::pair(&b, &feature::mv(v, m))).collect();
        self.q_set(&pairs)
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

/// Three layers with two rectifiers, over as much of `x` as `w1` is wide.
///
/// Both towers of a dueling net are this shape, and so is a plain net, so the
/// arithmetic is written once. An input shorter than the layer is read as
/// though the rest were zero, which is what lets a road pair of 64 numbers be
/// scored by a net stored at 315.
#[allow(clippy::too_many_arguments)]
fn tower(
    x: &[f32],
    w1: &[f32],
    b1: &[f32],
    w2: &[f32],
    b2: &[f32],
    w3: &[f32],
    b3: f32,
    hidden: usize,
) -> f32 {
    let mut h1 = vec![0.0f32; hidden];
    for j in 0..hidden {
        let mut a = b1[j];
        for (i, xi) in x.iter().enumerate() {
            a += xi * w1[i * hidden + j];
        }
        h1[j] = a.max(0.0);
    }
    let mut h2 = vec![0.0f32; hidden];
    for j in 0..hidden {
        let mut a = b2[j];
        for (i, hi) in h1.iter().enumerate() {
            a += hi * w2[i * hidden + j];
        }
        h2[j] = a.max(0.0);
    }
    let mut out = b3;
    for (i, hi) in h2.iter().enumerate() {
        out += hi * w3[i];
    }
    out
}
