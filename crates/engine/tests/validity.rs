//! Can a real build actually get to every door, and through it?
//!
//! The repository's answer to "is this road walkable" has always been
//! `force_win` and `skip_to`: assign a rung, win by fiat, assert the door is
//! standing. That proves the road *graph* and says nothing about whether a
//! build anybody could have can fight its way there
//! (`post-unwinding.md` §10.6, and the gap `design/rl-agent-plan.md` exists to
//! close).
//!
//! This file is the other answer. A **validity solver**: load a strong board,
//! hand it a path, and make it fight. A test passes when the door it was
//! written for was reached, was standing, and was answered - with the fights
//! actually simulated by the oracle rather than granted.
//!
//! Two halves:
//!
//! * `Walk` - the solver. It fights, answers, visits, throws levers and feeds
//!   orbs, and records everything it met. It never calls `force_win`.
//! * The audit - every event in the game with its access conditions spelled
//!   out, and lints over the shapes those conditions can take.
//!
//! What it cannot do is prove a door is *un*reachable: a walk that fails to
//! reach one may be a bad path rather than a shut door. So the lints are about
//! conditions that are impossible in principle, and the walks are existence
//! proofs.

mod common;

use gearmaster_engine::combat::{Difficulty, Outcome};
use gearmaster_engine::event::{Requirement, Trigger, EVENTS};
use gearmaster_engine::run::{Mode, Phase, Run};

// ---------------------------------------------------------------- the solver

/// One instruction in a path.
#[derive(Clone, Debug)]
pub enum Step {
    /// Fight whatever is standing in front of you, and expect to win.
    Fight,
    /// Fight until the run is standing on this rung index.
    FightTo(usize),
    /// Answer the door with this id by the choice with this label.
    Answer(&'static str, &'static str),
    /// Answer whatever door is standing, by the first choice that is open.
    AnswerAnything,
    /// Walk into the town standing here and take this door.
    Town(gearmaster_engine::town::Action),
    /// Walk past the town standing here.
    PastTheTown,
    /// Throw the points down the road with this label.
    Throw(&'static str),
    /// Feed a pedestal this orb.
    Feed(&'static str),
    /// Take what the fountain is offering.
    Drink,
    /// Buy the named word off the bar, paying with whatever it wants.
    Barter(&'static str),
}

/// What a walk saw.
#[derive(Default, Debug)]
pub struct Seen {
    pub doors: Vec<&'static str>,
    pub answered: Vec<&'static str>,
    pub fights: usize,
    pub losses: usize,
    pub stopped_at: usize,
    pub why: Option<String>,
}

/// A strong board, walking the road for real.
///
/// The board is `A_WINNING_RUN` - a finished run's, 48 of 50 rungs at Medium -
/// because the question is "can this be reached at all", and a build that
/// cannot clear the ladder cannot answer it. Difficulty is Medium, which is
/// gear as written and the setting the curve is defined at.
pub struct Walk {
    pub run: Run,
    pub seen: Seen,
}

impl Walk {
    pub fn new() -> Self {
        let mut run = common::run_from(gearmaster_engine::share::A_WINNING_RUN);
        run.mode = Mode::Grinder;
        run.difficulty = Difficulty::Medium;
        run.rung = 0;
        run.gold = 100_000;
        // A spare of every kind the bar can ask for.
        //
        // `A_WINNING_RUN` is a *finished* board: every piece it owns is worn,
        // so its tray is empty and `payment_for` finds nothing to hand over.
        // That is a true fact about that board and the wrong one to measure a
        // door by - a player who wants a word off the bar keeps a spare mold
        // for it, and the question here is whether the door can be reached at
        // all. So the walk starts with one loose piece of each kind the bar
        // prices anything in, and nothing else.
        let mut wants: Vec<gearmaster_engine::piece::PieceKind> = Vec::new();
        for r in gearmaster_engine::rumour::RUMOURS {
            if let gearmaster_engine::rumour::Barter::Kind(k) = r.price {
                if !wants.contains(&k) {
                    wants.push(k);
                }
            }
        }
        for k in wants {
            if let Some(d) = gearmaster_engine::piece::CATALOG
                .iter()
                .find(|d| d.kind == k && !gearmaster_engine::piece::is_event_only(d.name))
            {
                run.give(d.name);
            }
        }
        Walk { run, seen: Seen::default() }
    }

    fn note_doors(&mut self) {
        if let Some(e) = self.run.pending_event() {
            if !self.seen.doors.contains(&e.id) {
                self.seen.doors.push(e.id);
            }
        }
    }

    /// Fight the thing in front of you. Returns false if it was not a win.
    fn fight(&mut self) -> bool {
        if self.run.phase != Phase::Loadout {
            self.run.back_to_loadout();
        }
        self.run.pending_scene = None;
        let before = self.run.rung;
        let log = self.run.fight_next().clone();
        self.run.settle();
        self.run.take_receipt();
        self.run.pending_scene = None;
        self.run.back_to_loadout();
        self.seen.fights += 1;
        if log.outcome != Outcome::Victory {
            self.seen.losses += 1;
            self.seen.why = Some(format!(
                "lost to {} at rung {} after {:.1}s",
                log.enemy().name,
                before + 1,
                log.duration_ms as f32 / 1000.0
            ));
            return false;
        }
        true
    }

    pub fn step(&mut self, s: &Step) -> bool {
        self.note_doors();
        match s {
            Step::Fight => {
                if !self.clear_the_road() {
                    return false;
                }
                self.fight()
            }
            Step::FightTo(rung) => {
                let mut guard = 0;
                while self.run.rung < *rung {
                    guard += 1;
                    if guard > 200 {
                        self.seen.why = Some(format!(
                            "stuck at rung {} trying to reach {}",
                            self.run.rung + 1,
                            rung + 1
                        ));
                        return false;
                    }
                    self.note_doors();
                    if !self.clear_the_road() {
                        return false;
                    }
                    if self.run.rung >= *rung {
                        break;
                    }
                    if !self.fight() {
                        return false;
                    }
                }
                true
            }
            Step::Answer(id, label) => {
                let Some(e) = self.run.pending_event() else {
                    self.seen.why =
                        Some(format!("no door at rung {} to answer", self.run.rung + 1));
                    return false;
                };
                if e.id != *id {
                    self.seen.why = Some(format!(
                        "expected {id} at rung {}, found {}",
                        self.run.rung + 1,
                        e.id
                    ));
                    return false;
                }
                let Some(c) = e.choices.iter().find(|c| c.label == *label) else {
                    self.seen.why = Some(format!("{id} has no choice {label:?}"));
                    return false;
                };
                if !self.run.choice_open(c) {
                    self.seen.why = Some(format!(
                        "{id}/{label:?} was shut: {}",
                        c.requires.describe()
                    ));
                    return false;
                }
                self.run.take_choice(c);
                self.run.take_receipt();
                self.seen.answered.push(id);
                true
            }
            Step::AnswerAnything => {
                let Some(e) = self.run.pending_event() else { return true };
                let id = e.id;
                let Some(c) = e.choices.iter().find(|c| self.run.choice_open(c)) else {
                    self.seen.why = Some(format!("{id} has no choice anybody can take"));
                    return false;
                };
                self.run.take_choice(c);
                self.run.take_receipt();
                self.seen.answered.push(id);
                true
            }
            Step::Town(a) => {
                if self.run.pending_town().is_none() {
                    self.seen.why =
                        Some(format!("no town gate at rung {}", self.run.rung + 1));
                    return false;
                }
                self.run.visit_town(*a);
                self.run.take_receipt();
                true
            }
            Step::PastTheTown => {
                if self.run.town.is_some() {
                    self.run.skip_town();
                }
                true
            }
            Step::Throw(label) => {
                let Some((d, floor)) = self.run.dungeon else {
                    self.seen.why = Some("not in a dungeon".into());
                    return false;
                };
                let Some(i) = d.floors[floor].exits.iter().position(|e| e.label == *label) else {
                    self.seen.why = Some(format!("no road called {label:?}"));
                    return false;
                };
                let ok = self.run.throw_points(i);
                self.run.take_receipt();
                ok
            }
            Step::Feed(orb) => {
                let Some(id) = self
                    .run
                    .owned
                    .iter()
                    .copied()
                    .find(|&i| self.run.registry.def(i).name == *orb)
                else {
                    self.seen.why = Some(format!("{orb} is not held"));
                    return false;
                };
                if self.run.feed_pedestal(id).is_none() {
                    self.seen.why = Some(format!("the pedestal refused {orb}"));
                    return false;
                }
                self.run.take_receipt();
                true
            }
            Step::Barter(word) => {
                // A word can be priced in another word - the ledger is bought
                // with the crownwright - so the chain is walked back to
                // something the tray can pay for, then bought forwards.
                let mut chain: Vec<&'static str> = vec![*word];
                let mut guard = 0;
                while let Some(r) = gearmaster_engine::rumour::by_name(chain[chain.len() - 1]) {
                    guard += 1;
                    if guard > 8 {
                        self.seen.why = Some(format!("{word:?} is priced in a circle"));
                        return false;
                    }
                    match r.price {
                        gearmaster_engine::rumour::Barter::Rumour(other)
                            if !self.run.holds(other) =>
                        {
                            chain.push(other)
                        }
                        _ => break,
                    }
                }
                for want in chain.iter().rev() {
                    let Some(slot) = (0..gearmaster_engine::shop::SHOP_SIZE)
                        .find(|&i| self.run.rumour_on(i).is_some_and(|r| r.name == *want))
                    else {
                        self.seen.why = Some(format!("{want:?} is not on this bar"));
                        return false;
                    };
                    let Some(&pay) = self.run.payment_for(slot).first() else {
                        self.seen.why =
                            Some(format!("nothing in the tray pays for {want:?}"));
                        return false;
                    };
                    if self.run.barter(slot, pay).is_err() {
                        self.seen.why = Some(format!("the bar refused to trade for {want:?}"));
                        return false;
                    }
                }
                true
            }
            Step::Drink => {
                let offer: Vec<_> = self.run.fountain_offer().to_vec();
                let Some(c) = offer.first() else {
                    self.seen.why = Some("no fountain here".into());
                    return false;
                };
                self.run.drink_choosing(c);
                self.run.take_receipt();
                true
            }
        }
    }

    /// Answer or walk past anything standing between here and the fight.
    ///
    /// A gate, a fountain and a door all block a fight, and a walk that is
    /// trying to get *somewhere* has to get past the ones it did not come for.
    /// Takes the first open choice, which is deliberately dumb: a path that
    /// needs a particular answer says so with `Answer`.
    fn clear_the_road(&mut self) -> bool {
        let mut guard = 0;
        while let Some(what) = self.run.road_is_blocked() {
            guard += 1;
            if guard > 20 {
                self.seen.why = Some(format!("{what} would not clear at rung {}", self.run.rung + 1));
                return false;
            }
            self.note_doors();
            if self.run.pending_town().is_some() {
                self.run.skip_town();
                continue;
            }
            if self.run.at_fountain() || self.run.at_doubling_fountain() {
                if !self.step(&Step::Drink) {
                    return false;
                }
                continue;
            }
            if self.run.at_points {
                let (d, floor) = self.run.dungeon.expect("at points");
                let open = d.floors[floor]
                    .exits
                    .iter()
                    .position(|e| d.fights_ahead(e.to, &self.run.cleared_floors) > 0)
                    .unwrap_or(0);
                self.run.throw_points(open);
                self.run.take_receipt();
                continue;
            }
            if self.run.pending_event().is_some() {
                if !self.step(&Step::AnswerAnything) {
                    return false;
                }
                continue;
            }
            if let Some(specs) = self.run.pending_brawl() {
                self.run.fight_party(&specs);
                self.run.settle();
                self.run.take_receipt();
                continue;
            }
            self.seen.why = Some(format!("{what} blocks the road and nothing here can clear it"));
            return false;
        }
        true
    }

    /// Follow a path. Returns whether every step was taken.
    pub fn follow(&mut self, path: &[Step]) -> bool {
        for s in path {
            if !self.step(s) {
                self.seen.stopped_at = self.run.rung;
                return false;
            }
        }
        self.note_doors();
        self.seen.stopped_at = self.run.rung;
        true
    }
}

// ------------------------------------------------------------- the conditions

/// Every way a door can be reached, in words.
pub fn access(e: &'static gearmaster_engine::event::LadderEvent) -> String {
    let where_ = match e.trigger {
        Trigger::Rung => format!("rung {}", e.at + 1),
        Trigger::QuickKill { within_ms, from } => format!(
            "rungs {} to {}, after a win under {:.1}s",
            from + 1,
            e.at + 1,
            within_ms as f32 / 1000.0
        ),
        Trigger::SlowKill { over_ms, from } => format!(
            "rungs {} to {}, after a win over {:.1}s",
            from + 1,
            e.at + 1,
            over_ms as f32 / 1000.0
        ),
        Trigger::Whispered { rumour, from } => {
            format!("rungs {} to {}, carrying {rumour:?}", from + 1, e.at + 1)
        }
        Trigger::WhenFlagged { flag, from } => {
            format!("rungs {} to {}, having done {flag:?}", from + 1, e.at + 1)
        }
    };
    let shut = if e.blocked_by.is_empty() {
        String::new()
    } else {
        format!("; shut by {:?}", e.blocked_by)
    };
    format!("{where_}{shut}")
}

// ----------------------------------------------------------------- the audit

/// What has to be true for a choice to be takeable, in words.
fn asks(r: &Requirement) -> String {
    match r {
        Requirement::None => "-".into(),
        other => other.describe(),
    }
}

/// Every door in the game, with the conditions to reach it and to get through.
#[test]
#[ignore]
fn report_every_door_and_what_it_wants() {
    println!("\n## Every door, and how a run reaches it\n");
    for e in EVENTS {
        println!("\n{}  [{}]", e.title, e.id);
        println!("  stands: {}", access(e));
        println!("  expects: {} (rung {})", e.expects, e.at + 1);
        if let Some(w) = opens_it(e) {
            println!("  its key: {w}");
        }
        for c in e.choices {
            println!("    - {:<38} {}", c.label, asks(&c.requires));
        }
    }
}

/// What hands over the key a whispered or flagged door waits on.
fn opens_it(e: &'static gearmaster_engine::event::LadderEvent) -> Option<String> {
    match e.trigger {
        Trigger::Whispered { rumour, .. } => {
            let mut from: Vec<String> = Vec::new();
            for o in EVENTS {
                for c in o.choices {
                    for out in gearmaster_engine::event::every_outcome(&c.outcome) {
                        if matches!(out, gearmaster_engine::event::Outcome::Give(n) if *n == rumour)
                        {
                            from.push(format!("{} rung {}", o.id, o.at + 1));
                        }
                    }
                }
            }
            for t in gearmaster_engine::town::TOWNS {
                for a in t.actions {
                    if a.gives() == Some(rumour) {
                        from.push(format!("{} ({:?})", t.id, a));
                    }
                }
            }
            if gearmaster_engine::rumour::by_name(rumour).is_some_and(|r| r.on_the_bar) {
                from.push("the pub's bar".into());
            }
            Some(format!("{rumour:?} from {from:?}"))
        }
        Trigger::WhenFlagged { flag, .. } => {
            let mut from: Vec<String> = gearmaster_engine::event::set_by(flag)
                .iter()
                .map(|(id, label)| format!("{id}/{label:?}"))
                .collect();
            for d in gearmaster_engine::dungeon::DUNGEONS {
                if d.also.iter().any(|o| {
                    matches!(o, gearmaster_engine::event::Outcome::Flag(n) if *n == flag)
                }) {
                    from.push(format!("{} (on any way out)", d.id));
                }
                for f in d.floors {
                    if f.also.iter().any(|o| {
                        matches!(o, gearmaster_engine::event::Outcome::Flag(n) if *n == flag)
                    }) {
                        from.push(format!("{} floor {}", d.id, f.creature));
                    }
                }
            }
            Some(format!("{flag:?} from {from:?}"))
        }
        _ => None,
    }
}

/// Every door that waits on a key has somewhere the key comes from, and that
/// somewhere is inside the window.
///
/// `completable.rs` asks this of the shapes it knows. This asks it of every
/// door at once and prints the whole chain when it fails, because a door whose
/// key arrives one rung late is indistinguishable from a door nobody wrote.
#[test]
fn every_door_that_waits_on_a_key_can_be_handed_one_in_time() {
    let mut bad: Vec<String> = Vec::new();
    for e in EVENTS {
        // A door a pedestal pushes onto the stack stands on no rung at all.
        // `"never"` is the sentinel that says so, and asking when its key
        // arrives is asking the wrong question - the orb is the key, and
        // `pedestal.rs` lints that.
        if matches!(e.trigger, Trigger::WhenFlagged { flag: "never", .. }) {
            continue;
        }
        let (key, earliest) = match e.trigger {
            Trigger::Whispered { rumour, from: _ } => {
                let mut soonest: Option<usize> = None;
                for o in EVENTS {
                    for c in o.choices {
                        for out in gearmaster_engine::event::every_outcome(&c.outcome) {
                            if matches!(out, gearmaster_engine::event::Outcome::Give(n) if *n == rumour)
                            {
                                let at = o.trigger.from().min(o.at);
                                soonest = Some(soonest.map_or(at, |s: usize| s.min(at)));
                            }
                        }
                    }
                }
                // A town door or the bar can hand one over the moment a run
                // reaches either, and both are earlier than any window here.
                let by_town = gearmaster_engine::town::TOWNS.iter().any(|t| {
                    t.actions.iter().any(|a| a.gives() == Some(rumour))
                });
                let on_bar =
                    gearmaster_engine::rumour::by_name(rumour).is_some_and(|r| r.on_the_bar);
                if by_town || on_bar {
                    continue;
                }
                (rumour, soonest)
            }
            Trigger::WhenFlagged { flag, from: _ } => {
                let mut soonest: Option<usize> = gearmaster_engine::event::set_by(flag)
                    .iter()
                    .filter_map(|(id, _)| EVENTS.iter().find(|o| o.id == *id))
                    .map(|o| o.trigger.from().min(o.at))
                    .min();
                for d in gearmaster_engine::dungeon::DUNGEONS {
                    // A dungeon sets a flag two ways: on any way out (`also`)
                    // or at a particular buffer stop (`Floor::also`). The
                    // first is how THE THRESHOLD hands over `threshold-cleared`
                    // and the second is how the yard hands over
                    // `switchyard-cleared`; a lint that knew only the second
                    // called the Unwinding's whole back half unreachable.
                    let by_dungeon = d.also.iter().any(|o| {
                        matches!(o, gearmaster_engine::event::Outcome::Flag(n) if *n == flag)
                    });
                    let by_floor = d.floors.iter().any(|f| {
                        f.also.iter().any(|o| {
                            matches!(o, gearmaster_engine::event::Outcome::Flag(n) if *n == flag)
                        })
                    });
                    if by_dungeon || by_floor {
                        // A dungeon is entered three ways: an event's choice, a
                        // town door, or a pedestal. THE THRESHOLD is a town
                        // door - `Action::CellarDoor` - and a lint that knew
                        // only about events called the Unwinding's back half
                        // unreachable.
                        for t in gearmaster_engine::town::TOWNS {
                            if t.actions.iter().any(|a| a.opens() == Some(d.id)) {
                                let at = t.after + 1;
                                soonest = Some(soonest.map_or(at, |s: usize| s.min(at)));
                            }
                        }
                        for x in gearmaster_engine::pedestal::DESTINATIONS {
                            let goes_here = match x.kind {
                                gearmaster_engine::pedestal::Where::Dungeon(id) => id == d.id,
                                gearmaster_engine::pedestal::Where::Siding { dungeon, .. } => {
                                    dungeon == d.id
                                }
                                _ => false,
                            };
                            if goes_here {
                                // A pedestal stands in a town, so the earliest
                                // is that town's gate.
                                for t in gearmaster_engine::town::TOWNS {
                                    if t.actions.iter().any(|a| {
                                        matches!(a, gearmaster_engine::town::Action::Pedestal)
                                    }) {
                                        let at = t.after + 1;
                                        soonest = Some(soonest.map_or(at, |s: usize| s.min(at)));
                                    }
                                }
                            }
                        }
                        for o in EVENTS {
                            for c in o.choices {
                                for out in gearmaster_engine::event::every_outcome(&c.outcome) {
                                    if matches!(
                                        out,
                                        gearmaster_engine::event::Outcome::Enter(x)
                                            | gearmaster_engine::event::Outcome::StartDungeon(x)
                                            if *x == d.id
                                    ) {
                                        let at = o.trigger.from().min(o.at);
                                        soonest =
                                            Some(soonest.map_or(at, |s: usize| s.min(at)));
                                    }
                                }
                            }
                        }
                    }
                }
                (flag, soonest)
            }
            _ => continue,
        };
        match earliest {
            None => bad.push(format!("{}: nothing anywhere hands over {key:?}", e.id)),
            Some(when) if when > e.at => bad.push(format!(
                "{}: waits on {key:?} from rung {}, and the earliest anything hands one over is rung {}",
                e.id,
                e.trigger.from() + 1,
                when + 1
            )),
            _ => {}
        }
    }
    assert!(bad.is_empty(), "doors whose key cannot arrive in time:\n  {}", bad.join("\n  "));
}

/// No door is shut by something that stands after it.
///
/// `blocked_by` is "answering that one closes this one for good". A door
/// blocked by one that stands *later* is a door that can never be closed,
/// which is harmless - and one blocked by a door on the same rung is a
/// coin-toss nobody can see. Both are worth knowing about.
#[test]
fn nothing_is_shut_by_a_door_that_comes_after_it() {
    for e in EVENTS {
        for b in e.blocked_by {
            let other = EVENTS.iter().find(|o| o.id == *b).unwrap_or_else(|| {
                panic!("{} is shut by {b:?}, which is not a door", e.id)
            });
            assert!(
                other.trigger.from() <= e.at,
                "{} stands from rung {} and is shut by {}, which cannot stand before rung {}",
                e.id,
                e.trigger.from() + 1,
                b,
                other.trigger.from() + 1
            );
        }
    }
}

/// Every door has a way through that a build can actually satisfy.
///
/// Not "a free choice" - `every_event_has_a_way_through_that_costs_nothing`
/// already says that. This asks the harder half: of the choices that *are*
/// gated, is each one gated on something a run can come by? A door whose only
/// interesting answers want a component nothing sells is a door that is
/// decorative.
#[test]
fn every_gated_choice_wants_something_a_run_can_get() {
    use gearmaster_engine::piece::CATALOG;
    let mut bad: Vec<String> = Vec::new();
    for e in EVENTS {
        for c in e.choices {
            match c.requires {
                Requirement::Holding(name) => {
                    if !CATALOG.iter().any(|d| d.name == name) {
                        bad.push(format!("{}/{:?} wants {name:?}, which is not a component", e.id, c.label));
                    }
                }
                Requirement::Flag(f) => {
                    if gearmaster_engine::event::set_by(f).is_empty()
                        && !gearmaster_engine::dungeon::DUNGEONS.iter().any(|d| {
                            d.floors.iter().any(|fl| {
                                fl.also.iter().any(|o| {
                                    matches!(o, gearmaster_engine::event::Outcome::Flag(n) if *n == f)
                                })
                            })
                        })
                    {
                        bad.push(format!("{}/{:?} waits on {f:?}, which nothing sets", e.id, c.label));
                    }
                }
                Requirement::Took(label) => {
                    if !EVENTS.iter().any(|o| o.choices.iter().any(|k| k.label == label)) {
                        bad.push(format!("{}/{:?} wants {label:?} taken, and no door offers it", e.id, c.label));
                    }
                }
                _ => {}
            }
        }
    }
    assert!(bad.is_empty(), "gated on the unobtainable:\n  {}", bad.join("\n  "));
}

// ------------------------------------------------------------- the walks

/// Walk the road from rung one, fighting everything, and see what it meets.
///
/// The greedy walk: answer whatever is standing by the first open choice, take
/// whatever a fountain offers, walk past every town, and fight. No `force_win`
/// and no `skip_to` - every rung is a simulated fight against the creature
/// actually standing there.
fn greedy_walk(to: usize) -> Walk {
    let mut w = Walk::new();
    w.follow(&[Step::FightTo(to)]);
    w
}

#[test]
#[ignore]
fn report_what_a_strong_build_meets_on_the_way_down() {
    let w = greedy_walk(45);
    println!("\n## A greedy walk from rung 1, fighting every rung\n");
    println!("  reached rung {}", w.seen.stopped_at + 1);
    println!("  fights {}, losses {}", w.seen.fights, w.seen.losses);
    if let Some(why) = &w.seen.why {
        println!("  stopped because: {why}");
    }
    println!("\n  doors met ({}):", w.seen.doors.len());
    for d in &w.seen.doors {
        println!("    {d}");
    }
    let missed: Vec<&str> = EVENTS
        .iter()
        .filter(|e| !w.seen.doors.contains(&e.id))
        .map(|e| e.id)
        .collect();
    println!("\n  doors not met ({}):", missed.len());
    for d in &missed {
        let e = EVENTS.iter().find(|e| e.id == *d).expect("a door");
        println!("    {:<26} {}", d, access(e));
    }
}

/// A build good enough to clear the ladder can fight its way to the deep end.
///
/// The floor under every other walk in this file. If this fails, nothing below
/// it means anything: a door "unreachable" by a walk that could not get past
/// rung twelve is a statement about the walk.
#[test]
fn a_strong_build_can_fight_its_way_down_the_road() {
    let w = greedy_walk(45);
    assert!(
        w.seen.stopped_at >= 45,
        "stopped at rung {} after {} fights ({} lost): {:?}",
        w.seen.stopped_at + 1,
        w.seen.fights,
        w.seen.losses,
        w.seen.why
    );
    assert_eq!(w.seen.losses, 0, "the walk lost a fight: {:?}", w.seen.why);
}

/// Every door that stands on a rung is met by a run that walks past it.
///
/// `Trigger::Rung` is "stands on `at`, every run, no questions", and this is
/// that sentence measured rather than trusted: a greedy walk from rung one to
/// the deep end meets every scheduled door on the way, by fighting.
///
/// Not the earned ones - a `QuickKill` door needs a fast fight and a
/// `Whispered` one needs a word this walk may have sold - and not the two a
/// pedestal pushes, which stand on no rung. Those have walks of their own.
#[test]
fn every_scheduled_door_is_met_by_a_run_that_fights_past_it() {
    let w = greedy_walk(45);
    let mut missed: Vec<String> = Vec::new();
    for e in EVENTS.iter().filter(|e| matches!(e.trigger, Trigger::Rung)) {
        if e.at > 45 {
            continue;
        }
        if !w.seen.doors.contains(&e.id) {
            missed.push(format!("{} (rung {})", e.id, e.at + 1));
        }
    }
    assert!(
        missed.is_empty(),
        "a run that fought past their rungs never saw:\n  {}\n(reached rung {}, met {} doors)",
        missed.join("\n  "),
        w.seen.stopped_at + 1,
        w.seen.doors.len()
    );
}

/// A word bought at the bar opens the door it is a word about.
///
/// The rumour-gated half of the road, proved the same way as the scheduled
/// half: fight to the first town, buy the word, fight on, and find the door
/// standing. Every fight is simulated.
///
/// Sump Bottom stands after rung 7 and has the bar in it, so this is also the
/// earliest any of them can be reached - which is what
/// `every_door_that_waits_on_a_key_can_be_handed_one_in_time` argues from the
/// tables and this measures by walking.
#[test]
fn a_word_bought_at_the_bar_opens_the_door_it_is_about() {
    // Every word the pub sells, and the door each one opens.
    let sold: Vec<(&str, &str)> = gearmaster_engine::rumour::RUMOURS
        .iter()
        .filter(|r| r.on_the_bar)
        .map(|r| (r.name, r.opens))
        .collect();
    assert!(sold.len() >= 2, "the bar sells {} words", sold.len());

    let first_town = gearmaster_engine::town::TOWNS
        .iter()
        .filter(|t| matches!(t.unlock, gearmaster_engine::town::Unlock::Pinned))
        .map(|t| t.after)
        .min()
        .expect("a pinned town");

    let mut unreachable: Vec<String> = Vec::new();
    for (word, opens) in sold {
        let door = EVENTS.iter().find(|e| e.id == opens).expect("a real door");
        let mut w = Walk::new();
        // To the gate, into the pub, buy the word, then on to the door.
        let ok = w.follow(&[
            Step::FightTo(first_town + 1),
            Step::Town(gearmaster_engine::town::Action::Pub),
        ]);
        if !ok {
            unreachable.push(format!("{word}: could not reach the bar - {:?}", w.seen.why));
            continue;
        }
        if !w.step(&Step::Barter(word)) {
            // The bar is a rotating six and a seed may not stock this one on
            // the visit; that is a fact about the shelf and not about the door.
            unreachable.push(format!("{word}: {:?}", w.seen.why));
            continue;
        }
        assert!(w.run.holds(word), "{word} was bought and is not held");

        // Walk to the first rung its window covers and look for it.
        let target = door.trigger.from().max(w.run.rung);
        if !w.follow(&[Step::FightTo(target)]) {
            unreachable.push(format!("{word}: could not reach rung {} - {:?}", target + 1, w.seen.why));
            continue;
        }
        let standing = w.run.pending_event().map(|e| e.id);
        assert_eq!(
            standing,
            Some(opens),
            "{word} was in the tray at rung {} and {opens} did not stand; {:?} did",
            w.run.rung + 1,
            standing
        );
    }
    assert!(unreachable.is_empty(), "{}", unreachable.join("\n  "));
}

/// The Switchyard's four doors, reached by a build that fights for them.
///
/// The chain this mission added, walked without `force_win` for the first
/// time: every rung between the timetable and the last train is a simulated
/// fight, the yard's four floors are simulated fights, and the door at the end
/// reads a counter that only real clearings could have moved.
///
/// This is the test `post-unwinding.md` §10.6 says the repository did not
/// have. `switchyard::the_chain_can_be_walked_in_one_run_in_either_mode`
/// proves the road *graph* with fights won by fiat; this proves a board can
/// get there.
#[test]
fn the_switchyard_chain_is_walkable_by_a_build_that_fights_for_it() {
    let mut w = Walk::new();

    let ok = w.follow(&[
        // Rung 21, and Hesketh is standing there for every run.
        Step::FightTo(20),
        Step::Answer("the-timetable", "Buy a timetable"),
    ]);
    assert!(ok, "could not reach THE TIMETABLE: {:?}", w.seen.why);
    assert!(w.run.holds("A Word About the Sidings"), "the sheet bought nothing");

    // The box stands on the first rung of its window a run carrying the word
    // arrives at, which is 22.
    assert!(
        w.follow(&[Step::FightTo(21)]),
        "could not reach the signal box's window: {:?}",
        w.seen.why
    );
    assert!(
        w.follow(&[Step::Answer("the-signal-box", "Ask him to throw the points")]),
        "THE SIGNAL BOX did not stand at rung {}: {:?}",
        w.run.rung + 1,
        w.seen.why
    );
    assert!(w.run.holds("A Word About the Points"));

    assert!(
        w.follow(&[Step::FightTo(25), Step::Answer("the-turntable", "Step onto the turntable")]),
        "could not step onto the turntable: {:?}",
        w.seen.why
    );
    assert_eq!(w.run.dungeon.map(|(d, _)| d.id), Some("the-switchyard"));

    // Down the yard. Every floor is a real fight against a packed board.
    let before = w.seen.fights;
    assert!(
        w.follow(&[Step::Fight, Step::Throw("Down line"), Step::Fight, Step::Fight]),
        "the yard beat the board: {:?}",
        w.seen.why
    );
    assert!(w.run.at_points, "not at the pit points");
    assert!(
        w.follow(&[Step::Throw("The coal road"), Step::Fight]),
        "the coal stage beat the board: {:?}",
        w.seen.why
    );
    assert!(w.run.dungeon.is_none(), "still in the yard");
    assert_eq!(w.seen.fights - before, 4, "a line of the yard is four fights");

    assert!(w.run.holds("Ballast Bed"), "the coal stage paid no ground");
    assert!(w.run.holds("Shunter's Orb"), "the coal stage paid no ticket");
    assert_eq!(w.run.counted("sidings-cleared"), 1);

    // High Wick stands after rung 32 and its pedestal costs no visit.
    assert!(
        w.follow(&[Step::FightTo(31), Step::Feed("Shunter's Orb")]),
        "could not spend the ticket: {:?}",
        w.seen.why
    );
    assert_eq!(w.run.dungeon.map(|(d, _)| d.id), Some("the-switchyard"));
    assert_eq!(w.run.dungeon.map(|(_, f)| f), Some(5), "the siding lands on the Up line");

    assert!(
        w.follow(&[
            Step::Fight,
            Step::Fight,
            Step::Throw("The roundhouse road"),
            Step::Fight,
        ]),
        "the Up line beat the board: {:?}",
        w.seen.why
    );
    assert_eq!(w.run.counted("sidings-cleared"), 2, "both lines");
    assert!(w.run.holds("Signal Wire"));

    // And Ambrose, who reads the count.
    assert!(
        w.follow(&[Step::PastTheTown, Step::FightTo(33)]),
        "could not reach THE LAST TRAIN: {:?}",
        w.seen.why
    );
    assert!(
        w.follow(&[Step::Answer("the-last-train", "Tell him both lines")]),
        "the door that reads the count was shut: {:?}",
        w.seen.why
    );
    assert!(w.run.underwritten_until.is_some(), "the underwriter did not sign");
    assert_eq!(w.seen.losses, 0, "the walk lost a fight: {:?}", w.seen.why);
}

/// The yard's floors are fights a real board wins, one at a time.
///
/// M10 measured this against the four reference boards by calling the oracle
/// directly. This walks in through the door and out the other side, so the
/// board that fights each floor is the board the run actually has when it
/// gets there - carrying whatever the road handed it, at whatever the shop
/// let it build.
#[test]
fn every_floor_of_the_yard_is_won_on_the_way_through() {
    let d = gearmaster_engine::dungeon::by_id("the-switchyard").expect("the yard");
    for line in [("Down line", "The coal road"), ("Up line", "The roundhouse road")] {
        let mut w = Walk::new();
        assert!(
            w.follow(&[
                Step::FightTo(20),
                Step::Answer("the-timetable", "Buy a timetable"),
                Step::FightTo(21),
                Step::Answer("the-signal-box", "Ask him to throw the points"),
                Step::FightTo(25),
                Step::Answer("the-turntable", "Step onto the turntable"),
                Step::Fight,
                Step::Throw(line.0),
                Step::Fight,
                Step::Fight,
                Step::Throw(line.1),
                Step::Fight,
            ]),
            "{}: {:?}",
            line.0,
            w.seen.why
        );
        assert!(w.run.dungeon.is_none(), "{}: never came out", line.0);
        assert_eq!(w.seen.losses, 0, "{}: lost a fight", line.0);
        assert!(w.run.flags.contains(&"switchyard-cleared"));
    }
    let _ = d;
}

/// What the route map says about the yard.
#[test]
#[ignore]
fn report_the_map() {
    let mut run = Walk::new().run;
    run.rung = 27;
    for line in gearmaster_engine::route::ascii(&run) {
        println!("{line}");
    }
}

/// Every dungeon a door opens is drawn on the map, once.
///
/// The map draws a dungeon by scanning each door's outcomes for one that
/// enters it, and it used to scan `c.outcome` rather than `every_outcome` -
/// so a door that opens a dungeon *and* does something else drew nothing.
/// THE UNDER-MINE has been in the game since the Unwinding and had never once
/// been on the map, because both of the choices that open it buy you a shelf
/// on the way past.
///
/// That is the Unwinding's own most expensive lesson (`HANDOFF.md` §4: every
/// lint over `EVENTS` stopped at the top of an outcome) reaching the one place
/// it had not been applied.
#[test]
fn every_dungeon_a_door_opens_is_on_the_map() {
    use gearmaster_engine::event::{every_outcome, Outcome};
    use gearmaster_engine::route::{route, NodeKind};

    let mut want: Vec<&str> = Vec::new();
    for e in EVENTS {
        for c in e.choices {
            for o in every_outcome(&c.outcome) {
                if let Outcome::Enter(id) | Outcome::StartDungeon(id) = o {
                    if !want.contains(id) {
                        want.push(id);
                    }
                }
            }
        }
    }
    assert!(want.len() >= 3, "only {} dungeons are opened by a door", want.len());

    let mut run = Walk::new().run;
    run.rung = 45;
    let map = route(&run);
    let drawn: Vec<&str> = map
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Dungeon { .. }))
        .map(|n| n.id)
        .collect();

    for id in &want {
        assert!(drawn.contains(id), "{id} is opened by a door and is not on the map");
    }
    // And once each: two choices of one door that both open the same dungeon
    // are two ways through one door.
    let mut seen = drawn.clone();
    seen.sort_unstable();
    let n = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), n, "a dungeon is drawn twice: {drawn:?}");
}

/// The Switchyard's own content is on the map, and says how deep it goes.
#[test]
fn the_yards_content_is_on_the_map() {
    use gearmaster_engine::route::{ascii, route, NodeKind};

    let mut run = Walk::new().run;
    run.rung = 27;
    let map = route(&run);

    for id in ["the-timetable", "the-signal-box", "the-turntable", "the-last-train"] {
        assert!(
            map.nodes.iter().any(|n| n.id == id && n.kind == NodeKind::Event),
            "{id} is not on the map"
        );
    }
    let yard = map
        .nodes
        .iter()
        .find(|n| n.id == "the-switchyard")
        .expect("the yard is not on the map");
    assert_eq!(yard.kind, NodeKind::Dungeon { fights: 4, forks: 3 });

    // And the label says both numbers, which is the one thing the ascii map
    // gained: a straight line still says only its fights.
    let lines = ascii(&run).join("\n");
    assert!(
        lines.contains("THE SWITCHYARD (4 fights, 3 points)"),
        "the map does not say how deep the yard goes"
    );
    assert!(
        lines.contains("THE CREVICE IN THE ROCK (3 fights)"),
        "a straight line grew a points clause"
    );
}
