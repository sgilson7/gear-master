//! The two environments, as Markov decision processes.
//!
//! Both are `reset` / `legal` / `step` / `reward`, and both live here rather
//! than in `lab` because **an environment is what an agent sees**, and an
//! agent may not see anything the console does not draw. What `lab` adds is
//! the reward for the quartermaster, which is a fight - privileged, and
//! therefore handed in from outside rather than reached for.
//!
//! ## The one asymmetry, and why it is safe
//!
//! The quartermaster's reward needs a fight, which a player cannot run. So the
//! **trainer** scores an episode and the **agent** never sees the score: it
//! observes, it acts, and something outside tells it afterwards how it did.
//! That is the asymmetric actor-critic, and the crate graph is what enforces
//! it - `Scored` is a trait this crate declares and `lab` implements.

use gearmaster_console::{Console, Verb};

use crate::partition::{owner, Trade};

/// What an episode of either trade looks like, step by step.
pub trait Episode {
    /// Everything legal right now, for this trade only.
    fn legal(&self, c: &Console) -> Vec<Verb>;
    /// Whether the episode is over.
    fn done(&self, c: &Console) -> bool;
}

/// The quartermaster's episode: buy and pack until it says it is finished.
///
/// **Ends on `Done`**, which is not a `Verb` - it is the agent saying it has
/// nothing more to do, and it is the action that makes this an episode rather
/// than a loop. Without it a packer dithers, and a step cost alone does not
/// teach it to stop; it teaches it to press the cheapest key.
pub struct Packing {
    /// Presses spent so far.
    pub steps: usize,
    /// The most it may take. A budget rather than a rule: an episode that runs
    /// out is a failure the reward should be able to see.
    pub budget: usize,
    /// Set when the agent has said it is done.
    pub finished: bool,
}

/// One move a packing agent can make.
///
/// The verbs it owns, plus the one thing that is not a verb.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Move {
    Press(Verb),
    /// "I am finished." Hands the board back to whoever asked for it.
    Done,
}

impl Default for Packing {
    fn default() -> Self {
        Packing { steps: 0, budget: 60, finished: false }
    }
}

impl Packing {
    pub fn new(budget: usize) -> Packing {
        Packing { steps: 0, budget, finished: false }
    }

    /// Everything the quartermaster may do here, `Done` included.
    ///
    /// Q0 measured the control at thirteen decisions a typical episode and
    /// forty-seven at the worst, so a budget of sixty is generous rather than
    /// binding - and an episode that hits it has gone wrong in a way the
    /// reward should notice.
    pub fn moves(&self, c: &Console) -> Vec<Move> {
        if self.finished || self.steps >= self.budget {
            return Vec::new();
        }
        let mut out: Vec<Move> = c
            .menu()
            .into_iter()
            .filter(|&v| owner(v) == Trade::Quartermaster)
            .map(Move::Press)
            .collect();
        out.push(Move::Done);
        out
    }

    /// Take a move. `false` means the console refused it, which is a bug in
    /// the caller: everything `moves` offers is legal.
    pub fn step(&mut self, c: &mut Console, m: Move) -> bool {
        self.steps += 1;
        match m {
            Move::Done => {
                self.finished = true;
                true
            }
            Move::Press(v) => c.apply(v).ok,
        }
    }
}

impl Episode for Packing {
    fn legal(&self, c: &Console) -> Vec<Verb> {
        c.menu().into_iter().filter(|&v| owner(v) == Trade::Quartermaster).collect()
    }
    fn done(&self, c: &Console) -> bool {
        let _ = c;
        self.finished || self.steps >= self.budget
    }
}

/// The pathfinder's episode: one run, with `Pack` as one action.
pub struct Walking {
    pub steps: usize,
    pub budget: usize,
    /// The rung it has stood highest on, for the reward's "somewhere new".
    pub best_rung: usize,
    /// What it was sent to reach, if anything.
    pub goal: Option<Goal>,
    pub reached: bool,
}

/// What the pathfinder was told to do.
///
/// `None` is "climb", which is the ordinary game. Anything else is the
/// validity solver being pointed at something, and reaching it ends the
/// episode with the large reward - which is what makes this a solver rather
/// than a climber.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Stand in front of a door, by id.
    Door(String),
    /// Set foot in a dungeon, by id.
    Dungeon(String),
    /// Reach a town's gate, by name.
    Town(String),
    /// Clear a rung.
    Rung(usize),
    /// Stand on a county tile, by reference.
    County(String),
}

/// One move the pathfinder can make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Press(Verb),
    /// Hand the board to the quartermaster.
    Pack,
}

impl Walking {
    pub fn new(goal: Option<Goal>, budget: usize) -> Walking {
        Walking { steps: 0, budget, best_rung: 1, goal, reached: false }
    }

    pub fn moves(&self, c: &Console) -> Vec<Step> {
        if self.reached || self.steps >= self.budget {
            return Vec::new();
        }
        let mut out: Vec<Step> = c
            .menu()
            .into_iter()
            .filter(|&v| owner(v) == Trade::Pathfinder)
            .map(Step::Press)
            .collect();
        // Packing is only worth offering when there is something to pack with.
        if !c.tray_ids().is_empty() || !c.view().shop.is_empty() {
            out.push(Step::Pack);
        }
        out
    }

    /// Has the goal been reached, as of now?
    ///
    /// Read off the screen, so the pathfinder could in principle tell for
    /// itself - which matters, because a goal it cannot recognise is a goal it
    /// cannot aim at.
    pub fn met(&self, c: &Console) -> bool {
        let Some(goal) = &self.goal else { return false };
        let v = c.view();
        match goal {
            Goal::Door(id) => v.question.as_ref().is_some_and(|q| &q.id == id),
            Goal::Dungeon(id) => v.dungeon.as_ref().is_some_and(|d| &d.id == id),
            Goal::Town(name) => v.town.as_ref().is_some_and(|t| &t.name == name),
            Goal::Rung(n) => v.rung_shown > *n,
            Goal::County(at) => v.county.as_ref().is_some_and(|c| &c.reference == at),
        }
    }
}
