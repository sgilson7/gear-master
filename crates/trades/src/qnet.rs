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
}

fn row(text: &str, name: &str) -> Vec<f32> {
    text.lines()
        .find(|l| l.starts_with(name) && l.as_bytes().get(name.len()) == Some(&b' '))
        .map(|l| l[name.len()..].split_whitespace().filter_map(|v| v.parse().ok()).collect())
        .unwrap_or_default()
}

impl QNet {
    pub fn parse(text: &str) -> Option<QNet> {
        let (w1, b1, w2, b2, w3, b3) = (
            row(text, "w1"),
            row(text, "b1"),
            row(text, "w2"),
            row(text, "b2"),
            row(text, "w3"),
            row(text, "b3"),
        );
        let hidden = b1.len();
        if hidden == 0
            || w1.len() != PAIR * hidden
            || w2.len() != hidden * hidden
            || w3.len() != hidden
            || b3.len() != 1
        {
            return None;
        }
        Some(QNet { w1, b1, w2, b2, w3, b3: b3[0], hidden })
    }

    pub fn load(path: &str) -> Option<QNet> {
        QNet::parse(&std::fs::read_to_string(path).ok()?)
    }

    /// What this pair is worth.
    pub fn q(&self, x: &[f32; PAIR]) -> f32 {
        let mut h1 = vec![0.0f32; self.hidden];
        for j in 0..self.hidden {
            let mut a = self.b1[j];
            for (i, xi) in x.iter().enumerate() {
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
