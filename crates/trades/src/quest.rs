//! A quest, as the pathfinder sees one.
//!
//! `Goal` is one thing: a door, a dungeon, a town, a rung, a tile. Reaching it
//! is worth a lot and everything before it is worth nothing, which is a reward
//! an agent cannot climb - a chain twelve decisions long with one payout at the
//! end is the same fault the packer has, one agent along, and it is why eleven
//! doors in this game are unreached.
//!
//! So a quest is a **spec**: ordered stops, each recognisable from the screen,
//! each paying something, and a finish worth more than all of them.
//!
//! ## Everything here is read off the view
//!
//! `env::Walking::met` makes the argument for goals and it is the same one: a
//! goal an agent cannot recognise is a goal it cannot aim at. Every `Step`
//! below is a field of `View`, so the agent could in principle tell for itself
//! how far along it is - and it is told, because the progress is a feature.
//!
//! ## A stop keys on the outcome and never on the action
//!
//! The county door is at six towns and costs no visit; the chapel is at three
//! and costs one. "Visited the chapel" is satisfiable three times and "went
//! down into the county" six, free. A stop reading either is a farm. A stop
//! reading *what the run now has* is honest, and it is what the engine's own
//! `Requirement` says - which is why the derivation on the other side of the
//! boundary is written in that enum and this is written in its shadow.
//!
//! ## The tiers pay nothing, and that is the point
//!
//! Φ is potential-based: `F = γΦ(s′) − Φ(s)`. Over a whole episode that
//! telescopes to `γᵀΦ(s_T) − Φ(s_0)`, and both ends are zero here - a fresh run
//! has passed no stop, and `Φ` is **zeroed at the end of every episode however
//! it ended**. So the three cheap tiers contribute exactly nothing to the
//! return, and the finish is the only thing in it.
//!
//! That is a stronger guarantee than weighting the finish above the sum of the
//! steps: there is no sum to beat. What the tiers do instead is make the states
//! along a chain differ in *value* before the finish has ever been reached, so
//! there is a gradient to climb toward a thing random walking would never find.
//! The finish itself propagates the ordinary way, through the bootstrap.
//!
//! **Zeroing on truncation as well as termination is a choice**, and it is the
//! one the farm makes for us. An episode that runs out of road midway through a
//! chain has three tiers ticked and nothing finished; leaving Φ standing there
//! would hand it the tiers for a chain it abandoned, which is precisely the
//! farming trajectory. `crates/engine/tests/quest.rs` measured four rungs of
//! this road where exactly that happens. The cost is that a truncated episode's
//! bootstrap is biased low by `Φ(s_T)`, deliberately: a chain not finished was
//! not progress worth anything.

use gearmaster_console::view::View;
use gearmaster_console::Door;

/// What a stop is worth, cheapest first.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Tier {
    /// A door on the chain stood in front of the run.
    Offered,
    /// Something the chain needs that came from off it.
    Prerequisite,
    /// What the correct choice at a chain door produced.
    Chose,
    /// The objective.
    Finish,
}

impl Tier {
    /// What one stop at this tier adds to the potential.
    ///
    /// Ratios rather than magnitudes: `Φ` is a hint about where to look and the
    /// numbers only have to be far enough apart for a network to hear the
    /// difference. The packer's plateau was a shaping weight of `0.119` against
    /// values spread over 1.5 (`analysis/the-two-trades.md`, post-merge), so the
    /// lesson taken here is that a tier must be audible against the road's own
    /// `+1` a rung and `+3` for somewhere new.
    ///
    /// `Finish` is not in the potential and answers zero. It is paid once, at
    /// the end, outside `Φ` - see the module note.
    pub fn weight(self) -> f32 {
        match self {
            Tier::Offered => 1.0,
            Tier::Prerequisite => 2.0,
            Tier::Chose => 4.0,
            Tier::Finish => 0.0,
        }
    }
}

/// What shows a stop has been passed, in the words of the screen.
///
/// Named for the engine's `quest::Mark`, which is what the derivation on the
/// other side of the boundary produces - and not `Step`, which on this side is
/// already the pathfinder's own word for a move.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Mark {
    /// A door standing in front of the run, by id. `View::question`.
    Offered(String),
    /// A component owned, by name - loose in the tray or seated on a grid.
    /// Words are components, so this is how a rumour is asked about.
    Holding(String),
    /// A town's gate stood at, by name. `View::town`.
    Gate(String),
    /// A dungeon set foot in, by id. `View::dungeon`.
    Entered(String),
    /// A title worn. `View::classes`.
    Wearing(String),
    /// A rung behind the run. `View::rung_shown`.
    Cleared(usize),
    /// Standing in THE HUNDRED. `View::county`.
    InCounty,
}

impl Mark {
    /// Is this true of the screen as it is now?
    ///
    /// Every arm reads one field. Nothing here consults a table, because there
    /// is no table on this side of the boundary to consult.
    pub fn seen(&self, v: &View) -> bool {
        match self {
            Mark::Offered(id) => v.question.as_ref().is_some_and(|q| &q.id == id),
            // The tray and the slot panels. A word has one cell and nothing
            // on it, so nobody seats one and the tray is where it always is -
            // but a component the chain asks for might be built into something,
            // and the panel lists what it built.
            Mark::Holding(name) => {
                v.tray.iter().any(|p| &p.name == name)
                    || v.grids.iter().any(|g| g.items.iter().any(|i| &i.name == name))
            }
            Mark::Gate(name) => v.town.as_ref().is_some_and(|t| &t.name == name),
            Mark::Entered(id) => v.dungeon.as_ref().is_some_and(|d| &d.id == id),
            Mark::Wearing(name) => v.classes.iter().any(|c| c == name),
            Mark::Cleared(n) => v.rung_shown > *n,
            Mark::InCounty => v.county.is_some(),
        }
    }
}

/// One stop on a chain.
#[derive(Clone, Debug)]
pub struct Stop {
    pub tier: Tier,
    pub mark: Mark,
    /// The choice labels that pass this stop, and the town doors that do.
    ///
    /// **A set and not a label** (§3.6): a stop with two acceptable answers has
    /// two, and a spec that named one would be wrong about the road. The cellar
    /// word has two - the astronomer hears you out, or the Slagworks' foreman
    /// tells you - and a driver that only knew the first would call the second
    /// route a failure.
    ///
    /// Nothing in the reward reads these and nothing in the features carries
    /// them. They are here so a **written** driver can follow the plan, which is
    /// what makes a control worth comparing a learned policy against; handing
    /// them to the network would be handing it the answer.
    pub by: Vec<String>,
    pub doors: Vec<Door>,
    /// The earliest and latest rung this can be passed on, as the derivation
    /// tightened it. Carried for the report and for the features; nothing in
    /// the reward reads it, because a reward that punished being early would be
    /// a reward about the road rather than about the chain.
    pub window: (usize, usize),
}

/// A chain the pathfinder can be paid along.
#[derive(Clone, Debug)]
pub struct Quest {
    /// What the trained model will be called. Never read by the reward.
    pub name: String,
    pub stops: Vec<Stop>,
}

/// What a run has passed, this episode.
///
/// **One-shot per stop.** A stop passed twice pays once, which is the whole of
/// what stops the repeatable doors being a farm: the county door is free at six
/// towns and the chapel costs a visit at three, and neither of them can be
/// worth anything twice.
#[derive(Clone, Debug)]
pub struct Progress {
    done: Vec<bool>,
}

impl Progress {
    pub fn new(q: &Quest) -> Progress {
        Progress { done: vec![false; q.stops.len()] }
    }

    /// Forget everything: this is a different run now.
    ///
    /// **A Rogue run that loses its last life is replaced rather than ended**,
    /// at rung one, with the gold and the board gone - and an episode does not
    /// stop, because `Console::over` never sees the zero. So a chain half
    /// walked by a run that died would otherwise carry its passed stops into
    /// the run that replaced it, which is a word in a tray that burned.
    ///
    /// One-shot per stop is per **run**, not per episode. Those were the same
    /// thing until Rogue.
    pub fn wiped(&mut self) {
        for d in &mut self.done {
            *d = false;
        }
    }
    pub fn passed(&self) -> usize {
        self.done.iter().filter(|&&d| d).count()
    }
    pub fn has(&self, i: usize) -> bool {
        self.done.get(i).copied().unwrap_or(false)
    }
}

/// Why an episode stopped, which decides what happens to `Φ`.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum End {
    /// It has not.
    Running,
    /// It is over: the finish, a death, the road run out.
    Terminated,
    /// The budget ended it and the run was still going.
    ///
    /// Treated the same as `Terminated` by the potential and **deliberately**:
    /// see the module note. The trainer may still bootstrap the value here, and
    /// should - what is being given up is the shaping and not the estimate.
    Truncated,
}

/// What one step of a quest-conditioned episode paid.
#[derive(Copy, Clone, Debug, Default)]
pub struct Paid {
    /// `γΦ(s′) − Φ(s)`. Sums to zero over a whole episode.
    pub shaped: f32,
    /// The finish, once, outside the potential.
    pub finish: f32,
    /// How many stops were newly passed, for the report.
    pub passed: usize,
}

impl Paid {
    pub fn total(&self) -> f32 {
        self.shaped + self.finish
    }
}

impl Quest {
    /// The finishing stop, which is the last and the only one at its tier.
    pub fn finish(&self) -> Option<&Stop> {
        self.stops.last().filter(|s| s.tier == Tier::Finish)
    }

    /// How far along a run is, as a potential.
    ///
    /// `Finish` weighs nothing, so a finished chain and a chain one stop short
    /// have the same potential - which is correct and is the design: the finish
    /// is not a bigger hint, it is the reward.
    pub fn potential(&self, p: &Progress) -> f32 {
        self.stops
            .iter()
            .enumerate()
            .filter(|(i, _)| p.has(*i))
            .map(|(_, s)| s.tier.weight())
            .sum()
    }

    /// Has the chain been finished this episode?
    pub fn done(&self, p: &Progress) -> bool {
        !self.stops.is_empty() && p.has(self.stops.len() - 1)
    }

    /// Tick every stop the screen now shows, and say how many are new.
    pub fn observe(&self, p: &mut Progress, v: &View) -> usize {
        self.observe_by(p, |m| m.seen(v))
    }

    /// The same, against any answer to "is this true now".
    ///
    /// A screen is one such answer and the only one the agent ever uses. The
    /// seam is here so the payment rule can be tested against a trajectory
    /// somebody wrote down, rather than only against a game that has to be
    /// played into the right state - and the two trajectories the payment rule
    /// has to get right are a run that finishes and a run that farms, which is
    /// four rungs of road apart and nothing else.
    pub fn observe_by(&self, p: &mut Progress, seen: impl Fn(&Mark) -> bool) -> usize {
        let mut fresh = 0;
        for (i, s) in self.stops.iter().enumerate() {
            if !p.done[i] && seen(&s.mark) {
                p.done[i] = true;
                fresh += 1;
            }
        }
        fresh
    }

    /// Tick the stops, and clear them all if the run was replaced.
    ///
    /// Reads `View::wiped`, so a caller that pays through `pay` gets this for
    /// nothing and one that drives `observe` itself has to ask.
    pub fn observe_run(&self, p: &mut Progress, v: &View) -> usize {
        if v.wiped {
            p.wiped();
            return 0;
        }
        self.observe(p, v)
    }

    /// What this step of the episode pays.
    ///
    /// Call once per decision, after the console has been stepped. `finish` is
    /// what the caller pays for reaching the objective and is handed in rather
    /// than held here, because how much a finish is worth is the trainer's
    /// question and how far along the run is, is this module's.
    pub fn pay(&self, p: &mut Progress, v: &View, gamma: f32, end: End, finish: f32) -> Paid {
        // A replaced run keeps none of what the dead one passed, and the
        // potential goes with it - which is the same rule as an episode ending,
        // for the same reason: nothing that stopped short banks the tiers.
        if v.wiped {
            let before = self.potential(p);
            p.wiped();
            return Paid { shaped: -before, finish: 0.0, passed: 0 };
        }
        self.pay_by(p, |m| m.seen(v), gamma, end, finish)
    }

    /// The payment rule itself, against any answer to "is this true now".
    pub fn pay_by(
        &self,
        p: &mut Progress,
        seen: impl Fn(&Mark) -> bool,
        gamma: f32,
        end: End,
        finish: f32,
    ) -> Paid {
        // **On the transition, not on the state.** `done(p)` stays true once it
        // is true, so paying whenever it holds and something was ticked pays
        // the finish again the next time any *other* stop is passed. For a
        // chain whose last stop needs all the others that cannot happen; for
        // one where a run might already be holding something, it can - and a
        // reward that can be paid twice is the farm this module exists to stop,
        // arrived at from inside.
        let was_done = self.done(p);
        let before = self.potential(p);
        let fresh = self.observe_by(p, seen);
        let won = self.done(p) && !was_done;
        // Zero at the end whichever way it ended. The tiers give back what they
        // lent, so nothing that stopped short banks them.
        let after = if end == End::Running { self.potential(p) } else { 0.0 };
        Paid {
            shaped: gamma * after - before,
            finish: if won { finish } else { 0.0 },
            passed: fresh,
        }
    }

    /// The two numbers a road feature carries about a chain: how far along, and
    /// whether the last stop passed was the finish.
    ///
    /// Without these the potential telescopes over something the network cannot
    /// see, and a shaped reward the agent cannot predict is noise.
    pub fn features(&self, p: &Progress) -> [f32; 2] {
        let n = self.stops.len().max(1) as f32;
        [p.passed() as f32 / n, self.done(p) as u8 as f32]
    }
}
