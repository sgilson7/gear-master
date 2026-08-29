//! The player's surface, and the only door an agent gets.
//!
//! A `Console` wraps a `Run` and offers three things: the verbs that are legal
//! right now (`menu`), a way to press one (`apply`), and the screen
//! (`view`/`screen`). That is the whole interface. It hands out no `Run`, no
//! catalogue, no monster table and no way to simulate a fight that is not the
//! one in front of you - so an agent linked against this crate and nothing else
//! is playing the game rather than reading it.
//!
//! ## Why a crate and not a module
//!
//! `crates/agent/Cargo.toml` names this crate and no other. Rust will not
//! resolve a `use` for a crate that is not a dependency, so the pilot cannot
//! spell `gearmaster_engine::combat::simulate_party` however much it would
//! like to. `tests/boundary.rs` asserts the manifest, so a later convenience
//! cannot quietly add the edge back.
//!
//! ## What is re-exported, and why each is safe
//!
//! Leaf types only: `SlotKind`, `PieceKind`, `PieceId`, `Stats`, `Difficulty`,
//! `Mode`, `Step` and `town::Action`. None of them can reach a `Run`, a
//! `CATALOG` or a `MonsterSpec` - they are the vocabulary the screen is
//! written in. `Figures` is **mirrored** rather than re-exported, because the
//! engine's version comes with a constructor that scores arbitrary boards.

pub mod verb;
pub mod view;
mod screen;

pub use verb::Verb;
pub use view::View;

// The vocabulary the screen is written in. Nothing here reaches the tables.
pub use gearmaster_engine::combat::Difficulty;
pub use gearmaster_engine::county::Step;
pub use gearmaster_engine::piece::{PieceId, PieceKind, SlotKind};
pub use gearmaster_engine::run::{Mode, ROGUE_LIVES};
pub use gearmaster_engine::stats::Stats;
pub use gearmaster_engine::town::Action as Door;

use gearmaster_engine::run::{Phase, Run, RuleError};

/// What pressing a verb did.
///
/// Three states, not two, because a button that a player may press and that
/// does nothing is a different thing from a button that is not there:
///
/// * `ok: false` - not a legal thing to press here. If the menu offered it,
///   the menu has a bug, and `tests/legality.rs` is what says so.
/// * `ok: true, changed: false` - pressed, and nothing moved. Turning a piece
///   whose rotation would not fit is the case this exists for: the interface
///   lets you press it and the piece stays where it is.
/// * `ok: true, changed: true` - the run moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ok: bool,
    pub changed: bool,
    /// What the screen said about it, one line each. The receipt is in here.
    pub lines: Vec<String>,
}

impl Outcome {
    fn no(why: impl Into<String>) -> Outcome {
        Outcome { ok: false, changed: false, lines: vec![why.into()] }
    }
    fn yes(what: impl Into<String>) -> Outcome {
        Outcome { ok: true, changed: true, lines: vec![what.into()] }
    }
    /// Pressed, and nothing moved.
    fn inert(what: impl Into<String>) -> Outcome {
        Outcome { ok: true, changed: false, lines: vec![what.into()] }
    }
}

/// A run, seen from the player's chair.
pub struct Console {
    run: Run,
    /// Every verb pressed, in order. This is the proof.
    history: Vec<Verb>,
    seed: u64,
    /// The fight that just happened, kept because the run does not.
    ///
    /// `back_to_loadout` clears `Run::log`, and the console calls it as soon
    /// as a fight settles - so without this the result screen would be blank
    /// by the time anybody looked at it, and an agent would score every fight
    /// as "no fight happened". The window keeps the same thing on screen for
    /// the same reason: you get to read what just happened.
    last: Option<view::Fight>,
}

impl Console {
    pub fn start(seed: u64, mode: Mode, difficulty: Difficulty) -> Console {
        Console {
            run: Run::start(seed, mode, difficulty),
            history: Vec::new(),
            seed,
            last: None,
        }
    }

    /// A console standing in a run somebody else set up.
    ///
    /// **Privileged by its own signature.** It takes a `Run`, and `Run` is an
    /// engine type - so only a crate that depends on the engine can call this,
    /// and `gearmaster-agent` does not. A benchmark harness may stand a run in
    /// front of the pilot with a particular tray in it; the pilot cannot stand
    /// one in front of itself. That is the same guarantee as the rest of the
    /// boundary, made by the type rather than by a rule.
    pub fn standing_in(run: Run, seed: u64) -> Console {
        Console { run, history: Vec::new(), seed, last: None }
    }

    /// The board as a fight would run it.
    ///
    /// Privileged by its return type - `ItemProfile` is an engine type, so a
    /// crate that cannot name the engine cannot call this. It exists for a
    /// harness that wants to ask what a board the pilot built can do, which is
    /// a question the pilot itself must never be able to ask.
    pub fn board_for_scoring(&self) -> (Stats, Vec<gearmaster_engine::loadout::ItemProfile>) {
        (self.run.player_stats(), self.run.combat_items())
    }

    /// Hand the run back. Privileged for the same reason `standing_in` is:
    /// it is an engine type, so the pilot cannot ask for one.
    pub fn into_run(self) -> Run {
        self.run
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn history(&self) -> &[Verb] {
        &self.history
    }

    /// The run is finished: out of lives, or past the top.
    pub fn over(&self) -> bool {
        self.run.lives_left() == Some(0) || self.run.past_the_top()
    }

    /// The ids in the tray, in the order the tray draws them.
    pub fn tray_ids(&self) -> Vec<PieceId> {
        self.run.inventory()
    }

    /// Where this piece would sit in this grid, at the rotation it is in now.
    ///
    /// Player-visible: the interface highlights exactly these cells while a
    /// piece is being dragged, which is what makes "where does this fit" a
    /// question a person answers by looking rather than by trying.
    pub fn anchors_for(&self, piece: PieceId, slot: SlotKind) -> Vec<(u8, u8)> {
        self.run
            .loadout
            .slot(slot)
            .legal_anchors(&self.run.registry, piece)
            .into_iter()
            .filter(|&(x, y)| self.run.can_equip(piece, slot, x, y).is_ok())
            .collect()
    }

    // ---- the menu --------------------------------------------------------

    /// Every verb that is legal right now.
    ///
    /// Enumerated the way the screen enumerates it, then checked against the
    /// engine: `tests/legality.rs` fuzzes reachable states and asserts that
    /// everything here is accepted and that nothing refused was offered.
    ///
    /// **One thing is deliberately not enumerated.** Where a whole assembled
    /// item may be dropped cannot be worked out without lifting it first
    /// (`equip_locked_at` tests every cell of the group against a board the
    /// group is still standing on), and `menu` does not mutate. `PlaceLocked`
    /// is therefore a verb the console accepts and the menu does not offer;
    /// the same board is reachable by unlocking and re-placing the pieces.
    pub fn menu(&self) -> Vec<Verb> {
        let mut out = Vec::new();
        let r = &self.run;

        if r.phase == Phase::Fighting {
            // Nothing but reading the log; the interface's own button is
            // "back to the loadout", which `apply` does for you after a fight.
            return out;
        }

        // ---- the board ---------------------------------------------------
        let inv = r.inventory();
        for &id in &inv {
            for kind in SlotKind::ALL {
                for (x, y) in r.loadout.slot(kind).legal_anchors(&r.registry, id) {
                    if r.can_equip(id, kind, x, y).is_ok() {
                        out.push(Verb::Place { piece: id, slot: kind, x, y });
                    }
                }
            }
            out.push(Verb::Rotate { piece: id });
        }
        for kind in SlotKind::ALL {
            let slot = r.loadout.slot(kind);
            if !slot.pieces().is_empty() {
                out.push(Verb::ClearSlot { slot: kind });
            }
            for id in slot.pieces() {
                if r.is_locked_item(id) {
                    out.push(Verb::UnequipLocked { piece: id });
                    out.push(Verb::RotateLocked { piece: id });
                } else {
                    out.push(Verb::Unequip { piece: id });
                    out.push(Verb::Rotate { piece: id });
                }
            }
            // Locking is offered on an assembled item's first piece only, so
            // one item is one verb rather than five identical ones.
            for item in r.report(kind).items.iter().filter(|i| i.assembled) {
                if let Some(&first) = item.pieces.first() {
                    out.push(Verb::Lock { piece: first });
                }
            }
            if r.owed_rows > 0 {
                out.push(Verb::Grow { slot: kind });
            }
        }
        if !inv.is_empty() || r.loadout.reports(&r.registry).iter().any(|s| !s.items.is_empty()) {
            out.push(Verb::ClearAll);
        }
        if r.undoable().is_some() {
            out.push(Verb::Undo);
        }

        // ---- the shop ----------------------------------------------------
        // Shelves are the shop's, and the shop stands wherever the run is;
        // buying is refused mid-town by the engine, not by the menu.
        for (i, _) in r.shop.stock.iter().enumerate() {
            if r.shop.def(i).is_none() {
                continue;
            }
            if r.price(i).is_some_and(|p| p <= r.gold) && inv.len() < gearmaster_engine::run::INVENTORY_CAP {
                out.push(Verb::Buy { shelf: i });
            }
            for paying in r.payment_for(i) {
                out.push(Verb::Barter { shelf: i, paying });
            }
            out.push(Verb::Pin { shelf: i });
        }
        for &id in &inv {
            out.push(Verb::Sell { piece: id });
            // These two carry conditions the screen can read and the menu
            // therefore has to read too. Duplicating an engine guard is a
            // drift risk and `tests/legality.rs` is the ratchet against it:
            // offer something the engine refuses and the fuzz fails.
            if can_crush(r, id) {
                out.push(Verb::Crush { piece: id });
            }
            if can_pedestal(r, id) {
                out.push(Verb::Pedestal { piece: id });
            }
        }
        if r.gold >= r.reroll_cost() {
            out.push(Verb::Reroll);
        }

        // ---- the road ----------------------------------------------------
        if let Some(e) = r.pending_event() {
            for (i, c) in e.choices.iter().enumerate() {
                if !r.choice_open(c) {
                    continue;
                }
                match c.requires {
                    gearmaster_engine::event::Requirement::Figure { min, max } => {
                        // Sixteen buckets over the range, which is the plan's
                        // encoding and enough resolution for a sealed bid.
                        for b in 0..16 {
                            let f = min + (max - min) * b / 15;
                            out.push(Verb::AnswerWith { choice: i, figure: f });
                        }
                    }
                    _ => out.push(Verb::Answer { choice: i }),
                }
            }
            // A question is the only thing you can do while one stands.
            return out;
        }

        if r.pending_town().is_some() {
            for d in r.pending_town().unwrap().actions.iter().copied() {
                out.push(Verb::Town { door: d });
            }
            out.push(Verb::WalkOn);
            return out;
        }

        if r.at_fountain() {
            out.push(Verb::Drink);
            for i in 0..r.fountain_offer().len() {
                out.push(Verb::DrinkChoosing { class: i });
            }
            return out;
        }
        if r.at_doubling_fountain() {
            for i in 0..r.doubling_offer().len() {
                out.push(Verb::Double { class: i });
            }
            return out;
        }

        if let Some(at) = r.county_at {
            if r.county_moves_left > 0 {
                for step in Step::ALL {
                    // Walking into the pale is a legal move that costs you the
                    // move; walking off the edge of the county is not a move at
                    // all, and the interface does not offer it.
                    if step.from(at).is_some() {
                        out.push(Verb::Walk { step });
                    }
                }
            }
            out.push(Verb::Out);
            return out;
        }

        if let Some((d, floor)) = r.dungeon {
            if r.at_points {
                for i in 0..d.floors[floor].exits.len() {
                    out.push(Verb::ThrowPoints { exit: i });
                }
            }
            out.push(Verb::Leave);
        }

        if r.pending_brawl().is_some() {
            out.push(Verb::FightParty);
        }
        if r.road_is_blocked().is_none() && r.pending_event().is_none() {
            out.push(Verb::Fight);
        }
        out
    }

    // ---- pressing one ----------------------------------------------------

    /// Press a verb. The engine's answer is the truth.
    pub fn apply(&mut self, v: Verb) -> Outcome {
        let mut out = self.press(v);
        // **Any verb may start a fight, not only `Fight`.**
        //
        // Walking onto a pinnacle in THE HUNDRED calls `begin_county_fight`,
        // which simulates the whole bout and leaves `Phase::Fighting` with a
        // log waiting - and `county_walk` and `leave_county` both refuse
        // outside `Loadout`, so every control dies at once. That is the freeze
        // `f4354ec` fixed in the window, reported from play: found the Drover,
        // and the map stopped answering.
        //
        // A headless driver has no bar to advance, so it settles here rather
        // than special-casing the verbs that can do it. Written as the general
        // fault for the reason that commit gives: the question "what else can
        // leave a fight unsettled" is the one nobody asked the first time.
        if out.ok {
            out.lines.extend(self.settle_any_fight());
        }
        if out.ok {
            // Inert presses go in the history too: a transcript is what was
            // pressed, and a person replaying it presses the same keys.
            self.history.push(v);
            if let Some(lines) = self.run.take_receipt() {
                out.lines.extend(lines);
            }
        }
        out
    }

    /// Read and settle a fight some verb started, if one is standing.
    fn settle_any_fight(&mut self) -> Vec<String> {
        if self.run.phase != Phase::Fighting {
            return Vec::new();
        }
        let Some(log) = self.run.log.clone() else {
            // Fighting with no log is a state nothing should be able to reach.
            self.run.back_to_loadout();
            return Vec::new();
        };
        self.last = Some(read::fight_of(&log));
        let mut lines = vec![format!(
            "{} after {:.1}s against {}",
            log.outcome.label(),
            log.duration_ms as f32 / 1000.0,
            log.enemy().name
        )];
        if let Some(g) = self.run.settle() {
            lines.push(format!("+{}g", g));
        }
        self.run.back_to_loadout();
        lines
    }

    fn press(&mut self, v: Verb) -> Outcome {
        let r = &mut self.run;
        match v {
            Verb::Place { piece, slot, x, y } => match r.equip(piece, slot, x, y) {
                Ok(()) => Outcome::yes(format!("placed {}", name_of(r, piece))),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::PlaceLocked { piece, slot, x, y } => match r.equip_locked_at(piece, slot, x, y) {
                Ok(()) => Outcome::yes("placed the item"),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::Unequip { piece } => match r.unequip(piece) {
                Ok(()) => Outcome::yes(format!("took off {}", name_of(r, piece))),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::UnequipLocked { piece } => match r.unequip_locked(piece) {
                Ok(()) => Outcome::yes("lifted the item"),
                Err(e) => Outcome::no(e.to_string()),
            },
            // A piece already on the board may have nowhere to turn to. The
            // interface lets you press it and the piece stays put, so that is
            // an inert press and not an illegal one - and the two are told
            // apart by which `RuleError` came back, because only a placement
            // error means "it would not fit".
            Verb::Rotate { piece } => match r.rotate(piece) {
                Ok(()) => Outcome::yes("turned it"),
                Err(RuleError::Place(_)) => Outcome::inert("it will not turn there"),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::RotateLocked { piece } => match r.rotate_locked(piece) {
                Ok(()) => Outcome::yes("turned the item"),
                Err(RuleError::Place(_)) => Outcome::inert("it will not turn there"),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::Lock { piece } => {
                if r.toggle_lock_item(piece) {
                    Outcome::yes("locked")
                } else {
                    Outcome::yes("unlocked")
                }
            }
            Verb::ClearSlot { slot } => match r.clear_slot(slot) {
                Ok(()) => Outcome::yes("emptied the grid"),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::ClearAll => {
                if r.phase != Phase::Loadout {
                    return Outcome::no("can't change gear during a fight");
                }
                r.clear_all();
                Outcome::yes("emptied every grid")
            }
            Verb::Undo => match r.undo() {
                Some(what) => Outcome::yes(format!("took back {}", what)),
                None => Outcome::no("nothing to take back"),
            },
            Verb::Grow { slot } => {
                if r.grow_slot(slot) {
                    Outcome::yes("a row")
                } else {
                    Outcome::no("no row owed")
                }
            }
            Verb::Buy { shelf } => match r.buy(shelf) {
                Ok(id) => Outcome::yes(format!("bought {}", name_of(r, id))),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::Sell { piece } => {
                let n = name_of(r, piece);
                match r.sell(piece) {
                    Ok(g) => Outcome::yes(format!("sold {} for {}g", n, g)),
                    Err(e) => Outcome::no(e.to_string()),
                }
            }
            Verb::Barter { shelf, paying } => match r.barter(shelf, paying) {
                Ok(id) => Outcome::yes(format!("traded for {}", name_of(r, id))),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::Reroll => match r.reroll() {
                Ok(()) => Outcome::yes("new shelves"),
                Err(e) => Outcome::no(e.to_string()),
            },
            Verb::Pin { shelf } => {
                if r.shop.toggle_lock(shelf) {
                    Outcome::yes("held")
                } else {
                    Outcome::yes("let go")
                }
            }
            Verb::Answer { choice } => self.answer(choice, None),
            Verb::AnswerWith { choice, figure } => self.answer(choice, Some(figure)),
            Verb::Fight => {
                if r.phase != Phase::Loadout {
                    return Outcome::no("a fight is already running");
                }
                if r.road_is_blocked().is_some() {
                    return Outcome::no("something is in the way");
                }
                if r.pending_event().is_some() {
                    return Outcome::no("something is asking you a question");
                }
                r.fight_next();
                // Settled by `apply`, with everything else that can start one.
                Outcome { ok: true, changed: true, lines: Vec::new() }
            }
            Verb::FightParty => {
                let Some(specs) = r.pending_brawl() else {
                    return Outcome::no("no brawl stands here");
                };
                r.fight_party(&specs);
                Outcome { ok: true, changed: true, lines: Vec::new() }
            }
            Verb::Town { door } => {
                let Some(t) = r.pending_town() else {
                    return Outcome::no("no gate here");
                };
                if !t.actions.contains(&door) {
                    return Outcome::no("that door is not at this gate");
                }
                let visit = r.visit_town(door);
                if visit.did.is_none() {
                    return Outcome::no("that door would not open");
                }
                Outcome { ok: true, changed: true, lines: visit.receipt() }
            }
            Verb::WalkOn => {
                if r.pending_town().is_none() {
                    return Outcome::no("no gate here");
                }
                let paid = r.skip_town();
                Outcome::yes(format!("walked on, +{}g", paid))
            }
            Verb::ThrowPoints { exit } => {
                if !r.at_points {
                    return Outcome::no("not at the points");
                }
                if r.throw_points(exit) {
                    Outcome::yes("thrown")
                } else {
                    Outcome::no("no road that way")
                }
            }
            Verb::Leave => {
                if r.leave_dungeon() {
                    Outcome::yes("out")
                } else if r.dungeon.is_none() {
                    Outcome::no("not in one")
                } else {
                    Outcome::no("not from here")
                }
            }
            Verb::Walk { step } => {
                if r.county_at.is_none() {
                    return Outcome::no("not in the county");
                }
                // `county_walk` says false both for "that did not happen" and
                // for "you went, you looked, and it cost you the move" - the
                // pale is the second. The receipt is what tells them apart,
                // which is `take_choice`'s trap (CLAUDE.md 21) in another verb.
                let moved = r.county_walk(step);
                if moved || r.last_receipt.is_some() {
                    Outcome::yes(if moved { "walked" } else { "went and looked" })
                } else {
                    Outcome::no("no move that way")
                }
            }
            Verb::Out => {
                if r.leave_county() {
                    Outcome::yes("back on the road")
                } else {
                    Outcome::no("not in the county")
                }
            }
            Verb::Perambulate { mouth } => {
                if r.walk_the_perambulation(mouth) {
                    Outcome::yes("set out")
                } else {
                    Outcome::no("not granted, or not from here")
                }
            }
            Verb::Drink => {
                if r.at_fountain() {
                    let c = r.drink();
                    Outcome::yes(format!("named you {}", c.name))
                } else if r.at_doubling_fountain() {
                    match r.doubling_offer().first().copied() {
                        Some(c) => {
                            r.double_class(c);
                            Outcome::yes(format!("doubled {}", c.name))
                        }
                        None => Outcome::no("nothing of yours to double"),
                    }
                } else {
                    Outcome::no("no fountain here")
                }
            }
            Verb::DrinkChoosing { class } => {
                let Some(c) = r.fountain_offer().get(class).copied() else {
                    return Outcome::no("it is not offering that");
                };
                match r.drink_choosing(c) {
                    Some(got) => Outcome::yes(format!("named you {}", got.name)),
                    None => Outcome::no("it will not"),
                }
            }
            Verb::Double { class } => {
                let Some(c) = r.doubling_offer().get(class).copied() else {
                    return Outcome::no("it is not offering that");
                };
                if r.double_class(c) {
                    Outcome::yes(format!("doubled {}", c.name))
                } else {
                    Outcome::no("it will not")
                }
            }
            Verb::Pedestal { piece } => match r.feed_pedestal(piece) {
                Some(d) => Outcome::yes(format!("it goes to {}", d.name)),
                None => Outcome::no("the socket does not want that"),
            },
            Verb::Crush { piece } => {
                let n = name_of(r, piece);
                match r.crush(piece) {
                    Some(_) => Outcome::yes(format!("crushed {}", n)),
                    None => Outcome::no("that does not crush"),
                }
            }
        }
    }

    fn answer(&mut self, i: usize, figure: Option<i32>) -> Outcome {
        let r = &mut self.run;
        let Some(e) = r.pending_event() else {
            return Outcome::no("nothing is asking you anything");
        };
        let Some(c) = e.choices.get(i) else {
            return Outcome::no("no such choice");
        };
        let label = c.label;
        let unmet = c.unmet;
        let took = match figure {
            Some(f) => r.take_choice_with(c, f),
            None => r.take_choice(c),
        };
        // `take_choice` hands back the component it handed over, and most
        // choices hand over nothing - so `is_some()` is not "the door was
        // answered". `answered` is (CLAUDE.md trap 21).
        if took.is_none() && r.last_receipt.is_none() {
            return Outcome::no(unmet.to_string());
        }
        Outcome::yes(label.to_string())
    }

    // ---- the screen ------------------------------------------------------

    /// The four figures a board is judged by, without drawing the rest of the
    /// screen.
    ///
    /// `view()` builds every grid, every tray entry and every shelf, which is
    /// 51 µs - and the hands read a board's worth twice for every seat they
    /// try, several hundred times a rung. This is the same numbers off the
    /// same accessors, and it is on the screen for the same reason they are:
    /// the county tab draws the figures, every slot draws whether it
    /// assembled, and the character sheet is the character sheet.
    ///
    /// It is a **shortcut through the drawing**, not a shortcut past it, and
    /// `tests/view.rs` holds the two to the same answer.
    pub fn figures(&self) -> (view::Figures, Stats, usize, usize) {
        let f = self.run.county_figures();
        let mut items = 0;
        let mut filled = 0;
        for k in SlotKind::ALL {
            let slot = self.run.loadout.slot(k);
            // Cells, not pieces: a piece is a polyomino and the grid is the
            // thing that runs out. Counting pieces here read 1 where the
            // screen read 3, which the test caught on its first run.
            filled += slot
                .pieces()
                .iter()
                .map(|&id| self.run.registry.def(id).cells.len())
                .sum::<usize>();
            items += self.run.report(k).items.iter().filter(|i| i.assembled).count();
        }
        (
            view::Figures {
                flow: f.flow,
                physical_dps: f.physical_dps,
                magic_dps: f.magic_dps,
                armour_ps: f.armour_ps,
                fastest_ms: f.fastest_ms,
                curse_resist: f.curse_resist,
            },
            self.run.player_stats(),
            items,
            filled,
        )
    }

    pub fn view(&self) -> View {
        self.build_view()
    }

    pub fn screen(&self) -> Vec<String> {
        screen::draw(&self.view())
    }

    /// A transcript line with the piece's name written after it, so the file
    /// reads like something a person wrote.
    pub fn annotate(&self, v: Verb) -> String {
        let line = v.line();
        let named = match v {
            Verb::Place { piece, .. }
            | Verb::PlaceLocked { piece, .. }
            | Verb::Unequip { piece }
            | Verb::UnequipLocked { piece }
            | Verb::Rotate { piece }
            | Verb::RotateLocked { piece }
            | Verb::Lock { piece }
            | Verb::Sell { piece }
            | Verb::Barter { paying: piece, .. }
            | Verb::Pedestal { piece }
            | Verb::Crush { piece } => Some(name_of(&self.run, piece)),
            _ => None,
        };
        match named {
            Some(n) => format!("{:<32} ; {}", line, n),
            None => line,
        }
    }
}

/// Whether the socket would take this piece.
///
/// An Orb of Travel that has not been spent - which is four pieces of the
/// twenty-three that are Orb-kind (`CLAUDE.md` trap 39), and a destination
/// each, once a run.
fn can_pedestal(run: &Run, id: PieceId) -> bool {
    let name = run.registry.def(id).name;
    match gearmaster_engine::pedestal::by_orb(name) {
        Some(d) => !run.destinations_visited.contains(&d.id),
        None => false,
    }
}

/// Whether this relic would break, and whether what is inside it has anywhere
/// to go. Each variant carries its own condition and the engine checks them
/// after taking the piece, so the menu checks them before offering it.
fn can_crush(run: &Run, id: PieceId) -> bool {
    use gearmaster_engine::relic::Crush;
    let name = run.registry.def(id).name;
    let Some(c) = gearmaster_engine::relic::crushable(name) else { return false };
    match c.what {
        Crush::SecondKey => run.town.is_some(),
        Crush::Appeal => !run.answered.is_empty(),
        Crush::SkipStone => run.rung + 1 <= gearmaster_engine::combat::LADDER.len(),
    }
}

fn name_of(run: &Run, id: PieceId) -> String {
    if run.owned.contains(&id) {
        run.registry.def(id).name.to_string()
    } else {
        format!("#{}", id.0)
    }
}

pub mod read;
