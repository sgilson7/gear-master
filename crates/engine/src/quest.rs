//! A chain, derived backwards from the thing at the end of it.
//!
//! Nothing in this module is a rule. It is a **reading** of the tables that
//! already exist: `EVENTS`, `TOWNS`, `DUNGEONS`, `RUMOURS` and `pedestal`, in
//! the direction nobody writes them. Forwards a door says what it does;
//! backwards you ask what would have had to happen for a thing to be true, and
//! keep asking until the answer is "walk up the road".
//!
//! It lives here rather than in a tool because two walkers already want it and
//! neither can reach the other. `tests/completable.rs` asks whether a key can
//! exist before its door shuts; a quest spec asks the same graph which keys
//! there are and in what order. Two walkers over one graph that disagree is a
//! bug in whichever is newer, so they share one - the earliest-rung arithmetic
//! below is `completable.rs`'s, moved rather than copied.
//!
//! ## Why derived and not written down
//!
//! A hand-written chain is a second copy of `EVENTS`. It goes stale the first
//! time a door moves, silently, and everything trained against it is training
//! against a road the game does not have. This repo has paid for that shape
//! four times (`CLAUDE.md` trap 20) and the payment is always the same: a green
//! suite over content nobody can reach.
//!
//! ## The deadline, which is the thing a derivation gets right and a list does not
//!
//! Every station carries the window it can be passed in, and the windows are
//! then **tightened against each other**: a station cannot be passed later than
//! anything that depends on it, and cannot be passed earlier than anything it
//! depends on. That is a two-line pass and it is the whole reason this is worth
//! deriving.
//!
//! `Station::by_when` is where the sharp edge is. A town gate stands on exactly
//! one rung - `town::between` matches `after + 1` - and `Run::settle` asks for
//! it the moment a rung is cleared. So a reveal landing on the gate's own rung
//! lands after the question was asked, and everything a gate depends on is due
//! **a rung early**. Nothing in the tables says that; it is a fact about the
//! order two functions run in, and it is worth four rungs of the Manse chain.

use crate::event::{every_outcome, LadderEvent, Outcome, Requirement, Trigger, EVENTS};
use crate::town::{Unlock, TOWNS};

/// The last rung there is. A window with no upper bound of its own gets this.
pub fn last_rung() -> usize {
    crate::combat::LADDER.len() - 1
}

// ------------------------------------------------- the earliest-rung arithmetic
//
// `completable.rs`'s, moved here so the two walkers cannot drift. The audit
// calls these and so does the derivation below.

/// The first rung a door can be answered on, and the last.
///
/// A `Rung` event stands on exactly one rung. `Trigger::from` returns 0 for it
/// - the earliest a *window* opens, which is not the same question - and
/// reading one for the other is how the first version of the audit came back
/// clean on a table with three broken doors in it.
pub fn door_window(e: &LadderEvent) -> (usize, usize) {
    match e.trigger {
        Trigger::Rung => (e.at, e.at),
        _ => (e.trigger.from(), e.at),
    }
}

/// A door pushed onto the road stack from off the road entirely.
pub fn off_the_road(e: &LadderEvent) -> bool {
    matches!(e.trigger, Trigger::WhenFlagged { flag: "never", .. })
}

/// The first rung a pinned town can be walked into. The bar is in every one.
pub fn first_town() -> usize {
    TOWNS
        .iter()
        .filter(|t| matches!(t.unlock, Unlock::Pinned))
        .map(|t| t.after + 1)
        .min()
        .expect("the road has a town on it")
}

/// Does this door hand over the named component?
pub fn gives(e: &LadderEvent, name: &str) -> bool {
    e.choices.iter().any(|c| choice_gives(c, name))
}

fn choice_gives(c: &'static crate::event::Choice, name: &str) -> bool {
    every_outcome(&c.outcome).iter().any(|o| match o {
        Outcome::Give(n) => *n == name,
        Outcome::Step(b) => b.win == name,
        Outcome::SealedBid { lots } => lots.contains(&name),
        Outcome::Passenger { pays, .. } => *pays == name,
        _ => false,
    })
}

/// The first rung a word can be in your hands, whoever hands it over.
pub fn word_by(name: &str) -> Option<usize> {
    let r = crate::rumour::RUMOURS.iter().find(|r| r.name == name)?;
    if r.on_the_bar {
        return Some(first_town());
    }
    let by_door = EVENTS.iter().filter(|e| gives(e, name)).map(|e| door_window(e).0).min();
    let by_town = TOWNS
        .iter()
        .filter(|t| t.actions.iter().any(|a| a.gives() == Some(name)))
        .map(|t| t.after + 1)
        .min();
    // A county tile is the fourth way to come by a word, and the earliest of
    // the three, because the earliest way down is the first town's own steps.
    //
    // Not `first_town() + 1`: the way down is not a door and does not cost the
    // visit, so a run standing at the first gate can be in the county on the
    // same rung it arrived at the gate on.
    let dug_up = crate::event::COUNTY_EVENTS.iter().any(|e| gives(e, name)).then(first_town);
    [by_door, by_town, dug_up].into_iter().flatten().min()
}

/// The first rung a flag can be set on, by a door or by a dungeon floor.
pub fn flag_by(flag: &str) -> Option<usize> {
    let by_door = EVENTS
        .iter()
        .filter(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
            })
        })
        .map(|e| door_window(e).0)
        .min();
    // A dungeon stands beside the road rather than on it, so the rung it can
    // first be cleared on is the rung its mouth first stands on.
    let by_floor = crate::dungeon::DUNGEONS
        .iter()
        .filter(|d| {
            d.also.iter().flat_map(every_outcome).any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
        })
        .filter_map(mouth_of)
        .min();
    match (by_door, by_floor) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// The first rung a dungeon's mouth stands on: a door that opens it, or the
/// town whose action does.
pub fn mouth_of(d: &crate::dungeon::Dungeon) -> Option<usize> {
    let by_door = EVENTS
        .iter()
        .filter(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(
                    |o| matches!(o, Outcome::Enter(id) | Outcome::StartDungeon(id) if *id == d.id),
                )
            })
        })
        .map(|e| door_window(e).0)
        .min();
    let by_town = TOWNS
        .iter()
        .filter(|t| t.actions.iter().any(|a| a.opens() == Some(d.id)))
        .map(|t| t.after + 1)
        .min();
    match (by_door, by_town) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

// ------------------------------------------------------------- the vocabulary

/// Something a chain can be aimed at.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Objective {
    /// A title the run is wearing.
    Class(&'static str),
    /// A dungeon set foot in.
    Dungeon(&'static str),
    /// A town's gate stood at.
    Town(&'static str),
    /// A door answered.
    Door(&'static str),
    /// A flag the run has set.
    Flag(&'static str),
    /// A component owned, worn or loose. Words are components.
    Holding(&'static str),
    /// A rung cleared.
    Rung(usize),
    /// A chain of THE HUNDRED finished.
    CountyChain(crate::county::Chain),
}

/// What a station is worth, cheapest first.
///
/// The order is the design (`design/HANDOFF-two-agents.md` §3.6) and the
/// weights are not here: a tier says what *kind* of thing happened and whoever
/// is paying decides what that is worth. Putting a number here would put the
/// reward in the engine.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Tier {
    /// A door on the chain stood in front of the run: it is in the right part
    /// of the road.
    Offered,
    /// Something the chain needs that came from **off** it - bought at a bar,
    /// picked up somewhere else. Easy to walk past, and cheap for that reason.
    Prerequisite,
    /// What the correct choice at a chain door produced. Never the choice
    /// itself: a step keying on the button is a step a repeatable door farms,
    /// and the outcome is the thing that is actually irreversible.
    Chose,
    /// The objective.
    Finish,
}

/// What shows a station has been passed.
///
/// Where the game already knows how to ask the question, the station asks it in
/// the game's own words - `Requirement` is what `choice_open` checks, so a step
/// written in it cannot drift from what the road enforces. Where it does not,
/// the station names the thing rather than the action, which is the same rule
/// one level down.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Mark {
    /// A door standing in front of the run. The only mark keyed on a place
    /// rather than on something the run has, and the cheapest for that reason.
    Offered(&'static str),
    /// A question the engine already asks: `Holding`, `Flag`, `Counter`,
    /// `HoldingRumour`, `CountyCleared`.
    Asked(Requirement),
    /// A town's gate stood at.
    Gate(&'static str),
    /// A dungeon set foot in.
    Entered(&'static str),
    /// A title worn.
    Wearing(&'static str),
    /// A rung behind the run.
    Cleared(usize),
    /// Standing in THE HUNDRED at all.
    ///
    /// The county is generated from a seed rather than tabled, so a chain that
    /// ends down there has no door to walk backwards through. What is still
    /// derivable is the way in - every town has steps and they cost no visit -
    /// and this is that station.
    InCounty,
}

/// How a station gets passed. A set rather than one label, because a chain
/// with two acceptable answers has two and a spec that named one would be
/// wrong about the road (§3.6).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Answer {
    /// A door, and the choice at it.
    Choice { door: &'static str, label: &'static str },
    /// A town, and the door of it.
    TownDoor { town: &'static str, action: crate::town::Action },
    /// The bar. A town door like any other, and the only one that trades
    /// rather than gives, so it is worth saying apart.
    Bar,
    /// Walking a dungeon to a buffer stop. A dungeon's `also` is the third
    /// place an outcome can be written and the only one that is not a choice.
    Dungeon(&'static str),
    /// Walking up the road, which is not a decision.
    Road,
    /// A rule in engine code that no table can see, and where it lives.
    ///
    /// The list is short on purpose. A gate written in `run.rs` is a gate
    /// nothing can grep for, and the cost of that is this variant existing -
    /// `completable.rs` pays it as `ENGINE_SETS` for the same reason.
    Engine(&'static str),
}

/// One thing that has to have happened.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Station {
    pub tier: Tier,
    pub mark: Mark,
    pub by: Vec<Answer>,
    /// The earliest and latest rung this can be passed on, after the whole
    /// chain has been tightened against itself.
    pub window: (usize, usize),
}

impl Station {
    /// The latest rung anything this station depends on may be passed on.
    ///
    /// Its own deadline, except at a gate. `town::between` matches `after + 1`
    /// exactly and `Run::settle` asks it the instant a rung is cleared, so a
    /// reveal that lands on the gate's own rung lands after the question was
    /// asked. Everything a gate depends on is due a rung early, and that one
    /// subtraction is the Manse chain's whole deadline.
    pub fn by_when(&self) -> usize {
        match self.mark {
            Mark::Gate(_) => self.window.1.saturating_sub(1),
            _ => self.window.1,
        }
    }
}

/// A chain, in the order it has to be walked.
#[derive(Clone, Debug)]
pub struct Quest {
    pub goal: Objective,
    pub stations: Vec<Station>,
}

impl Quest {
    /// The finish, which is the last station and the only one at its tier.
    pub fn finish(&self) -> Option<&Station> {
        self.stations.last().filter(|s| s.tier == Tier::Finish)
    }

    /// The tightest deadline anywhere in the chain: the rung by which a run has
    /// either done the thing or lost it.
    pub fn deadline(&self) -> Option<usize> {
        self.stations.iter().map(|s| s.window.1).min()
    }
}

// ------------------------------------------------------------- the derivation

/// A door the engine forces onto the road rather than the tables standing.
///
/// A door with `Trigger::WhenFlagged { flag: "never" }` is not waiting for a
/// flag; it is waiting for `run.rs`. That gate is not in any table, so no walk
/// of `EVENTS` can find it, and this is what a walk gets instead.
///
/// The list is short on purpose and every entry names where the rule lives,
/// which is the same bargain `completable.rs` makes with `ENGINE_SETS`. It is
/// also the one place in this module where a chain is typed rather than read,
/// so it says exactly as much as the rule does and no more.
pub struct Forced {
    pub door: &'static str,
    /// Where the rule is written, in words a grep will find.
    pub rule: &'static str,
    /// The rung the rule fires on, which for a forced door is not `at`.
    pub at: usize,
    /// What the rule asks for besides the rung.
    pub needs: &'static [Objective],
}

pub const FORCED: &[Forced] = &[Forced {
    door: "the-unwound",
    rule: "run.rs, Run::settle - the ladder cleared, holding the mainspring \
           or having looked through the lens",
    // Fifty, not the forty-nine in the table. `at: 49` is decorative on a door
    // nothing schedules: `standing_events` never reaches it, because the flag
    // it waits on is "never" and the only thing that puts it up is
    // `forced_event`, which does not consult a rung. The rule fires at
    // `rung == LADDER.len()`.
    at: crate::combat::LADDER.len(),
    // The `||` in the rule is not here. Either the mainspring or the lens makes
    // the door *stand*; only the mainspring opens the way down, which the
    // choice's own `Requirement::Holding` has always said - so the mainspring
    // is what a chain aimed at the road past Francis is actually about.
    needs: &[Objective::Holding("An Unwound Mainspring")],
}];

/// The chain that ends at `goal`, deepest prerequisite first.
///
/// Follows the **earliest** source at every branch, because that is the one a
/// run would use and a chain has to be an order rather than a graph; every
/// source it saw is kept in the station's `by`, which is what §3.6 asks for
/// when a door has two acceptable answers.
pub fn chain_to(goal: Objective) -> Quest {
    let mut out: Vec<Station> = Vec::new();
    let mut seen: Vec<Objective> = Vec::new();
    resolve(goal, true, &mut seen, &mut out, 0);
    tighten(&mut out);
    Quest { goal, stations: out }
}

/// Push everything `o` depends on, then `o` itself. Returns `o`'s own window
/// before tightening, or `None` if it cannot be derived at all.
fn resolve(
    o: Objective,
    top: bool,
    seen: &mut Vec<Objective>,
    out: &mut Vec<Station>,
    depth: usize,
) -> Option<(usize, usize)> {
    // Bounded, because a chain that walks until it runs out is a hang and this
    // one walks a graph somebody else wrote.
    if depth > 16 || seen.contains(&o) {
        return None;
    }
    seen.push(o);
    let end = last_rung();

    let (tier, mark, by, window) = match o {
        // ---- a title ----------------------------------------------------
        Objective::Class(name) => {
            // A dungeon that pays it, or a door that claims it.
            if let Some(d) = crate::dungeon::DUNGEONS.iter().find(|d| d.reward == name) {
                let w = resolve(Objective::Dungeon(d.id), false, seen, out, depth + 1)?;
                (
                    tier_for(top, Tier::Chose),
                    Mark::Wearing(name),
                    vec![Answer::Road],
                    w,
                )
            } else {
                let mut by = Vec::new();
                let mut first = None;
                for e in EVENTS {
                    for c in e.choices {
                        if every_outcome(&c.outcome)
                            .iter()
                            .any(|x| matches!(x, Outcome::Claim(n) if *n == name))
                        {
                            by.push(Answer::Choice { door: e.id, label: c.label });
                            let w = door_window(e);
                            first = Some(first.map_or(w, |f: (usize, usize)| (f.0.min(w.0), f.1.max(w.1))));
                        }
                    }
                }
                let w = first?;
                let door = by.iter().find_map(|a| match a {
                    Answer::Choice { door, .. } => Some(*door),
                    _ => None,
                })?;
                resolve(Objective::Door(door), false, seen, out, depth + 1);
                (tier_for(top, Tier::Chose), Mark::Wearing(name), by, w)
            }
        }

        // ---- a dungeon ---------------------------------------------------
        Objective::Dungeon(id) => {
            let d = crate::dungeon::by_id(id)?;
            let mut by = Vec::new();
            for t in TOWNS.iter() {
                for a in t.actions {
                    if a.opens() == Some(id) {
                        by.push(Answer::TownDoor { town: t.id, action: *a });
                    }
                }
            }
            for e in EVENTS {
                for c in e.choices {
                    if every_outcome(&c.outcome).iter().any(
                        |x| matches!(x, Outcome::Enter(k) | Outcome::StartDungeon(k) if *k == id),
                    ) {
                        by.push(Answer::Choice { door: e.id, label: c.label });
                    }
                }
            }
            let w = mouth_of(d)?;
            // The earliest way in is the one the chain follows.
            match by.iter().find(|a| matches!(a, Answer::TownDoor { town, .. }
                if TOWNS.iter().any(|t| t.id == *town && t.after + 1 == w)))
            {
                Some(Answer::TownDoor { town, .. }) => {
                    resolve(Objective::Town(town), false, seen, out, depth + 1);
                }
                _ => {
                    if let Some(Answer::Choice { door, .. }) = by.iter().find(|a| {
                        matches!(a, Answer::Choice { door, .. }
                            if EVENTS.iter().any(|e| e.id == *door && door_window(e).0 == w))
                    }) {
                        resolve(Objective::Door(door), false, seen, out, depth + 1);
                    }
                }
            }
            (tier_for(top, Tier::Chose), Mark::Entered(id), by, (w, end))
        }

        // ---- a town ------------------------------------------------------
        Objective::Town(id) => {
            let t = crate::town::by_id(id)?;
            let at = t.after + 1;
            match t.unlock {
                // Furniture. It is there because the road is there.
                Unlock::Pinned => {
                    (tier_for(top, Tier::Offered), Mark::Gate(id), vec![Answer::Road], (at, at))
                }
                Unlock::Hidden => {
                    let mut by = Vec::new();
                    let mut earliest: Option<(usize, &'static str)> = None;
                    for e in EVENTS {
                        for c in e.choices {
                            if every_outcome(&c.outcome)
                                .iter()
                                .any(|x| matches!(x, Outcome::RevealTown(k) if *k == id))
                            {
                                by.push(Answer::Choice { door: e.id, label: c.label });
                                let w = door_window(e).0;
                                if earliest.is_none_or(|(f, _)| w < f) {
                                    earliest = Some((w, e.id));
                                }
                            }
                        }
                    }
                    let (_, door) = earliest?;
                    resolve(Objective::Door(door), false, seen, out, depth + 1);
                    (tier_for(top, Tier::Chose), Mark::Gate(id), by, (at, at))
                }
            }
        }

        // ---- a door ------------------------------------------------------
        Objective::Door(id) => {
            let e = EVENTS.iter().find(|e| e.id == id)?;
            let w = door_window(e);
            if let Some(f) = FORCED.iter().find(|f| f.door == id) {
                for o in f.needs {
                    resolve(*o, false, seen, out, depth + 1);
                }
                (
                    tier_for(top, Tier::Offered),
                    Mark::Offered(id),
                    vec![Answer::Engine(f.rule)],
                    (f.at, f.at),
                )
            } else {
                match e.trigger {
                    Trigger::Whispered { rumour, .. } => {
                        resolve(Objective::Holding(rumour), false, seen, out, depth + 1);
                    }
                    Trigger::WhenFlagged { flag, .. } => {
                        resolve(Objective::Flag(flag), false, seen, out, depth + 1);
                    }
                    _ => {}
                }
                // The objective's own key. An intermediate door only has to
                // *stand*; the door a chain is aimed at has to be answerable,
                // and what its choices ask for is written in the same
                // `Requirement` a station is written in. Not done for
                // intermediate doors, because every alternative route through
                // every gated choice is a graph rather than a chain.
                if top {
                    for c in e.choices {
                        match c.requires {
                            Requirement::Holding(n) => {
                                resolve(Objective::Holding(n), false, seen, out, depth + 1);
                            }
                            Requirement::Flag(f) => {
                                resolve(Objective::Flag(f), false, seen, out, depth + 1);
                            }
                            _ => {}
                        }
                    }
                }
                (tier_for(top, Tier::Offered), Mark::Offered(id), vec![Answer::Road], w)
            }
        }

        // ---- a flag ------------------------------------------------------
        Objective::Flag(f) => {
            let mut by: Vec<Answer> =
                crate::event::set_by(f).into_iter().map(|(door, label)| Answer::Choice { door, label }).collect();
            let mut by_dungeon: Option<&'static str> = None;
            for d in crate::dungeon::DUNGEONS {
                if d.also.iter().flat_map(every_outcome).any(|x| matches!(x, Outcome::Flag(k) if *k == f))
                {
                    by.push(Answer::Dungeon(d.id));
                    if mouth_of(d) == Some(flag_by(f).unwrap_or(usize::MAX)) {
                        by_dungeon = Some(d.id);
                    }
                }
            }
            let w = flag_by(f)?;
            // A flag set by walking a dungeon gets the dungeon behind it, and
            // that is not tidiness: `View` carries no flags and never should -
            // a flag is bookkeeping and a player is shown no list of them - so
            // a station marked by a flag alone cannot cross the boundary. The
            // dungeon that sets it can, and it is the same event.
            if let Some(id) = by_dungeon {
                resolve(Objective::Dungeon(id), false, seen, out, depth + 1);
            } else if let Some(Answer::Choice { door, .. }) = by.iter().find(|a| {
                matches!(a, Answer::Choice { door, .. }
                    if EVENTS.iter().any(|e| e.id == *door && door_window(e).0 == w))
            }) {
                resolve(Objective::Door(door), false, seen, out, depth + 1);
            }
            (tier_for(top, Tier::Chose), Mark::Asked(Requirement::Flag(f)), by, (w, end))
        }

        // ---- a component -------------------------------------------------
        Objective::Holding(name) => {
            let mut by = Vec::new();
            let on_the_bar =
                crate::rumour::RUMOURS.iter().any(|r| r.name == name && r.on_the_bar);
            if on_the_bar {
                by.push(Answer::Bar);
            }
            let mut from_door: Option<(usize, &'static str)> = None;
            for e in EVENTS {
                for c in e.choices {
                    if choice_gives(c, name) {
                        by.push(Answer::Choice { door: e.id, label: c.label });
                        let w = door_window(e).0;
                        if from_door.is_none_or(|(f, _)| w < f) {
                            from_door = Some((w, e.id));
                        }
                    }
                }
            }
            for t in TOWNS {
                for a in t.actions {
                    if a.gives() == Some(name) {
                        by.push(Answer::TownDoor { town: t.id, action: *a });
                    }
                }
            }
            // A word has a rung; a piece the shop sells does not, and a chain
            // that asked for one would be asking for a shelf.
            let w = word_by(name).or_else(|| from_door.map(|(f, _)| f))?;
            // A word off the bar came from outside the chain, which is what
            // makes it the cheap tier. One handed over by a door is what that
            // door's correct choice produced, and is the dear one.
            let tier = if on_the_bar && w == first_town() {
                Tier::Prerequisite
            } else {
                if let Some((_, door)) = from_door {
                    resolve(Objective::Door(door), false, seen, out, depth + 1);
                }
                Tier::Chose
            };
            (tier_for(top, tier), Mark::Asked(Requirement::Holding(name)), by, (w, end))
        }

        // ---- a rung ------------------------------------------------------
        Objective::Rung(n) => (tier_for(top, Tier::Offered), Mark::Cleared(n), vec![Answer::Road], (n, n)),

        // ---- a chain of the county ---------------------------------------
        //
        // THE HUNDRED is generated rather than tabled, so there is no door to
        // walk back through and the derivation says so instead of inventing
        // one. What it can say is where the steps are: the county is entered
        // from a town, and the earliest town is the earliest way down.
        Objective::CountyChain(chain) => {
            let down: Vec<Answer> = TOWNS
                .iter()
                .filter(|t| t.actions.contains(&crate::town::Action::County))
                .map(|t| Answer::TownDoor { town: t.id, action: crate::town::Action::County })
                .collect();
            let first = TOWNS
                .iter()
                .filter(|t| matches!(t.unlock, Unlock::Pinned))
                .filter(|t| t.actions.contains(&crate::town::Action::County))
                .map(|t| t.after + 1)
                .min()
                .unwrap_or_else(first_town);
            let s = Station {
                tier: Tier::Prerequisite,
                mark: Mark::InCounty,
                by: down.clone(),
                window: (first, end),
            };
            if !out.iter().any(|x| x.mark == s.mark) {
                out.push(s);
            }
            (
                tier_for(top, Tier::Chose),
                Mark::Asked(Requirement::CountyCleared(chain)),
                down,
                (first, end),
            )
        }
    };

    let s = Station { tier, mark, by, window };
    if !out.iter().any(|x| x.mark == s.mark) {
        out.push(s);
    }
    Some(window)
}

/// The top of the chain is the objective whatever else it looks like.
fn tier_for(top: bool, otherwise: Tier) -> Tier {
    if top {
        Tier::Finish
    } else {
        otherwise
    }
}

/// Tighten every window against its neighbours.
///
/// Forwards: nothing can be passed before the thing it depends on. Backwards:
/// nothing can be passed after the thing that depends on it - through
/// `by_when`, so a gate takes its rung off everything behind it.
///
/// Two passes over a list, and it is what turns a set of independent windows
/// into a deadline.
///
/// **It tightens adjacent pairs, and that is exact only while a chain is a
/// line.** `resolve` emits a post-order walk, which is a topological order of
/// the dependency graph - so "i comes after i-1 in the list" always holds, but
/// "i *depends on* i-1" only holds when the graph has no branches in it. All
/// three chains derived today are lines, and a branch would over-constrain
/// rather than under-constrain: a window would come out narrower than the road
/// really is, and `no_derived_chain_has_a_station_that_cannot_be_passed` fires
/// loudly rather than a deadline quietly being wrong. The day a chain branches,
/// this wants the real predecessor rather than the previous line.
fn tighten(out: &mut [Station]) {
    for i in 1..out.len() {
        let floor = out[i - 1].window.0;
        out[i].window.0 = out[i].window.0.max(floor);
    }
    for i in (0..out.len().saturating_sub(1)).rev() {
        let ceiling = out[i + 1].by_when();
        out[i].window.1 = out[i].window.1.min(ceiling);
    }
}
