//! What the quartermaster is asked to build, as numbers.
//!
//! A theme reaches this crate as a **brief** and never as a name. That is the
//! boundary doing its job - `gearmaster-trades` cannot see `MonsterTheme`, so
//! whoever asks for a board has to say what they want in terms the packer
//! already understands.
//!
//! ## Why not a one-hot
//!
//! Q8's gate is that a **held-out** theme is packed better than by an
//! unconditioned packer. A one-hot cannot pass it, and not because the
//! training is bad: a class the network has never seen is a coordinate that
//! was zero in every gradient it ever took, so the conditioning contributes
//! exactly nothing and the two packers are the same packer. The gate would be
//! unpassable by construction, which is a worse outcome than failing it.
//!
//! So a brief is a **description**, and the description is in the packer's own
//! vocabulary: which grids to fill, and which pools the pieces it is allowed
//! to use tend to move. Warden - held out - is chest, greaves and weapon, and
//! the network has seen chest-and-weapon from Beast, greaves-and-weapon from
//! Burner and a three-grid brief from Wall. It has never seen Warden and it
//! has seen every part of Warden. That is what makes generalisation a question
//! worth measuring rather than a foregone answer.

/// How many numbers a brief is.
pub const BRIEF: usize = 13;

/// A board that has been asked for.
///
/// Five grid weights, then eight pool affinities in `Resource` order. Nothing
/// here names anything; the caller decides what the numbers mean and the
/// packer only ever learns which ones move together.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Brief(pub [f32; BRIEF]);

impl Brief {
    /// The brief that asks for nothing in particular.
    ///
    /// This is the control in Q8's comparison, and it is a real thing rather
    /// than a missing value: an unconditioned packer is one that was handed
    /// zeros every episode, so it is the same network with the same shape and
    /// the only difference is that the conditioning never varied.
    pub const NONE: Brief = Brief([0.0; BRIEF]);

    /// Which grids this brief wants filled, in `SlotKind::ALL` order.
    pub fn slots(&self) -> &[f32] {
        &self.0[..5]
    }

    /// Which pools it leans on, in `Resource` order.
    pub fn pools(&self) -> &[f32] {
        &self.0[5..]
    }

    /// Whether this brief asks for anything at all.
    pub fn is_none(&self) -> bool {
        self.0.iter().all(|&x| x == 0.0)
    }

    /// How alike two briefs are, as the cosine of the angle between them.
    ///
    /// Used by Q8's report to say *which* of the trained themes a held-out one
    /// is being generalised from, which is the difference between "it worked"
    /// and "here is why it worked".
    pub fn likeness(&self, other: &Brief) -> f32 {
        let dot: f32 = self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum();
        let na: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
