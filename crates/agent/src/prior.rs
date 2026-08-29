//! A learned prior over seats, read as plain numbers.
//!
//! **Training is privileged and inference is not.** The model is trained in
//! `crates/lab` behind `--features nn`, and what comes out is six matrices of
//! floats in a text file. This reads them and multiplies them out. The pilot
//! links no framework, and `cargo test --workspace` compiles nothing new -
//! which is the plan's §2 rule kept rather than argued with.
//!
//! It is also the honest shape of what a prior *is* here: it does not decide
//! anything. It orders the seats the hands were going to try anyway, so the
//! hands can try eight instead of ninety and check each one against the real
//! board exactly as before. Every board it produces was measured, not
//! predicted.

use crate::hands::Prior;
use crate::lesson::FEATURES;

/// Three layers of weights, and how many seats to keep.
pub struct Learned {
    w1: Vec<f32>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: Vec<f32>,
    w3: Vec<f32>,
    b3: Vec<f32>,
    hidden: usize,
    keep: usize,
}

fn row(text: &str, name: &str) -> Vec<f32> {
    text.lines()
        .find(|l| l.starts_with(name) && l.as_bytes().get(name.len()) == Some(&b' '))
        .map(|l| l[name.len()..].split_whitespace().filter_map(|v| v.parse().ok()).collect())
        .unwrap_or_default()
}

impl Learned {
    /// Read the weights a training run wrote.
    pub fn parse(text: &str, keep: usize) -> Option<Learned> {
        let w1 = row(text, "w1");
        let b1 = row(text, "b1");
        let w2 = row(text, "w2");
        let b2 = row(text, "b2");
        let w3 = row(text, "w3");
        let b3 = row(text, "b3");
        if w1.is_empty() || b1.is_empty() {
            return None;
        }
        let hidden = b1.len();
        if w1.len() != FEATURES * hidden || w2.len() != hidden * hidden || w3.len() != hidden {
            return None;
        }
        Some(Learned { w1, b1, w2, b2, w3, b3, hidden, keep })
    }

    pub fn load(path: &str, keep: usize) -> Option<Learned> {
        Learned::parse(&std::fs::read_to_string(path).ok()?, keep)
    }
}

impl Prior for Learned {
    fn score(&self, x: &[f32; FEATURES]) -> f32 {
        let mut h1 = vec![0.0f32; self.hidden];
        for j in 0..self.hidden {
            let mut acc = self.b1[j];
            for (i, xi) in x.iter().enumerate() {
                acc += xi * self.w1[i * self.hidden + j];
            }
            h1[j] = acc.max(0.0);
        }
        let mut h2 = vec![0.0f32; self.hidden];
        for j in 0..self.hidden {
            let mut acc = self.b2[j];
            for (i, hi) in h1.iter().enumerate() {
                acc += hi * self.w2[i * self.hidden + j];
            }
            h2[j] = acc.max(0.0);
        }
        let mut out = self.b3[0];
        for (i, hi) in h2.iter().enumerate() {
            out += hi * self.w3[i];
        }
        out
    }

    fn keep(&self) -> usize {
        self.keep
    }
}
