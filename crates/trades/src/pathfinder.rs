//! The pathfinder: walks the road, and can be told where to go.
//!
//! Q-learning over the road, with `pack` as one macro-action into a frozen
//! packer, and a **goal** in the state - which is what turns an agent that
//! plays well into a solver that can be asked whether a door is reachable.
//!
//! The goal is one-hot in the features and `+50` in the reward, so reaching it
//! outweighs any amount of laddering. That asymmetry is the product: a
//! goal-seeker that reaches every door is worth more than a climber that
//! reaches one dungeon.

use crate::env::{Goal, Walking};
use crate::feature;
use crate::qnet::QNet;
use gearmaster_console::view::View;
use gearmaster_console::{Console, Verb};

/// How many numbers describe the road and the goal.
pub const ROAD: usize = 22;

/// The road, the run, and what it was sent to reach.
pub fn road(v: &View, goal: Option<&Goal>) -> [f32; ROAD] {
    let mut f = [0.0f32; ROAD];
    f[0] = v.rung_shown as f32 / 50.0;
    f[1] = (v.gold as f32 / 400.0).min(4.0);
    f[2] = v.lives_left.unwrap_or(9) as f32 / 5.0;
    f[3] = v.wins as f32 / 50.0;
    f[4] = v.losses as f32 / 20.0;
    f[5] = v.classes.len() as f32 / 8.0;
    f[6] = v.tray.len() as f32 / v.tray_cap.max(1) as f32;
    f[7] = v.question.is_some() as u8 as f32;
    f[8] = v.town.is_some() as u8 as f32;
    f[9] = v.fountain.is_some() as u8 as f32;
    f[10] = v.in_dungeon as u8 as f32;
    f[11] = v.county.is_some() as u8 as f32;
    f[12] = v.points.is_some() as u8 as f32;
    f[13] = v.brawl_waiting as u8 as f32;
    let items: usize =
        v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum();
    f[14] = items as f32 / 8.0;
    f[15] = (v.stats.health as f32 / 1500.0).min(4.0);
    f[16] = (v.coming.stats.health as f32 / 4000.0).min(4.0);
    // The goal, one-hot over its kinds, and how close it looks.
    match goal {
        None => f[17] = 1.0,
        Some(Goal::Door(_)) => f[18] = 1.0,
        Some(Goal::Dungeon(_)) => f[19] = 1.0,
        Some(Goal::Town(_)) => f[20] = 1.0,
        Some(Goal::Rung(_)) | Some(Goal::County(_)) => f[21] = 1.0,
    }
    f
}

/// The pair a road Q network scores: the road, and one candidate step.
pub const PAIR: usize = ROAD + feature::MOVE;

pub fn pair(r: &[f32; ROAD], m: &[f32; feature::MOVE]) -> [f32; PAIR] {
    let mut out = [0.0f32; PAIR];
    out[..ROAD].copy_from_slice(r);
    out[ROAD..].copy_from_slice(m);
    out
}

/// A step described for the network. `Pack` is the all-zero action, the same
/// convention `Done` uses on the other side.
pub fn describe(v: &View, s: &crate::env::Step) -> [f32; feature::MOVE] {
    match s {
        crate::env::Step::Pack => [0.0; feature::MOVE],
        crate::env::Step::Press(verb) => feature::mv(v, *verb),
    }
}

/// What a run is worth, step by step.
///
/// `+1` a rung, `+3` for a rung never stood on this episode, `-1` a lost
/// fight, and **`+50` on reaching the goal**, which ends it.
pub struct Reward {
    pub rung_before: usize,
    pub best_before: usize,
}

impl Reward {
    pub fn of(&self, after: &Console, w: &Walking, lost: bool) -> f32 {
        let v = after.view();
        let mut r = -0.01;
        if v.rung_shown > self.rung_before {
            r += 1.0;
            if v.rung_shown > self.best_before {
                r += 3.0;
            }
        }
        if lost {
            r -= 1.0;
        }
        if w.met(after) {
            r += 50.0;
        }
        r
    }
}

/// Play a run with a network deciding, and a frozen packer doing the packing.
pub struct Pathfinder<'a> {
    pub net: Option<&'a QNet>,
    pub goal: Option<Goal>,
    pub budget: usize,
    /// How many presses the packer may spend when asked.
    pub pack_budget: usize,
}

/// What happened.
#[derive(Clone, Debug, Default)]
pub struct Walked {
    pub best_rung: usize,
    pub steps: usize,
    pub packs: usize,
    pub reached: bool,
    pub transcript: Vec<String>,
}

impl Pathfinder<'_> {
    /// One run. `pack` is handed in so the caller decides which packer is
    /// frozen - which is the parameter the spec's generations turn.
    pub fn walk(
        &self,
        c: &mut Console,
        pack: &mut dyn FnMut(&mut Console, usize),
        explore: &mut dyn FnMut(usize) -> Option<usize>,
    ) -> Walked {
        let mut w = Walking::new(self.goal.clone(), self.budget);
        let mut out = Walked { best_rung: 1, ..Walked::default() };
        loop {
            let ms = w.moves(c);
            if ms.is_empty() || w.steps >= self.budget {
                break;
            }
            let v = c.view();
            let at = match (explore(ms.len()), self.net) {
                (Some(i), _) => i,
                (None, Some(net)) => {
                    let r = road(&v, self.goal.as_ref());
                    let mut best = (0usize, f32::MIN);
                    for (i, s) in ms.iter().enumerate() {
                        let q = net.q_pair(&pair(&r, &describe(&v, s)));
                        if q > best.1 {
                            best = (i, q);
                        }
                    }
                    best.0
                }
                (None, None) => 0,
            };
            match &ms[at] {
                crate::env::Step::Pack => {
                    pack(c, self.pack_budget);
                    out.packs += 1;
                }
                crate::env::Step::Press(verb) => {
                    out.transcript.push(c.annotate(*verb));
                    if !c.apply(*verb).ok {
                        break;
                    }
                }
            }
            w.steps += 1;
            out.steps += 1;
            out.best_rung = out.best_rung.max(c.view().rung_shown);
            if w.met(c) {
                w.reached = true;
                out.reached = true;
                break;
            }
        }
        out
    }
}

impl QNet {
    /// Score a road pair. The road network is wider than the packing one, so
    /// this is the same arithmetic against a different width.
    pub fn q_pair(&self, x: &[f32; PAIR]) -> f32 {
        let mut v = [0.0f32; feature::PAIR];
        let n = v.len().min(x.len());
        v[..n].copy_from_slice(&x[..n]);
        self.q(&v)
    }
}

/// A verb the pathfinder can take, for a caller that wants the list.
pub fn steps_of(c: &Console) -> Vec<Verb> {
    c.menu().into_iter().filter(|&v| crate::partition::owner(v) == crate::Trade::Pathfinder).collect()
}
