use crate::combat::{
    CombatLog, Difficulty, Event, MonsterSpec, Outcome, Side, LADDER, RUST_GOLEM,
};
use crate::loadout::{Loadout, LockedItem, SlotReport};
use crate::piece::{all_def_indices, PieceId, PieceRegistry, QuestTrack, SlotKind, CATALOG};

/// The one weapon a run is handed for free. Everything else is bought — this
/// exists so the very first decision is *where to place* a weapon rather than
/// whether the shop happened to offer you one.
pub const STARTER_KIT: &[&str] = &["Oak Handle", "Iron Blade"];


use crate::slot::{PlaceError, SLOT_H, SLOT_W};
use crate::rng::Rng;
use crate::shop::{Shop, REROLL_COST, STARTING_GOLD};
use crate::stats::Stats;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Phase {
    /// Arranging gear. The only phase in which the loadout can change.
    Loadout,
    /// A fight has been simulated; the GUI is replaying its log.
    Fighting,
}

/// What losing costs you. Either way a loss still pays the bounty - you need
/// income to buy your way past whatever just beat you - and never advances the
/// ladder, because you did not actually kill the thing.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Mode {
    /// Losing knocks you back down to the rung you last cleared, so there is
    /// always an easier fight to farm before trying again.
    Grinder,
    /// Losing costs a life. Three of them and the run is over.
    Rogue,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Grinder => "GRINDER",
            Mode::Rogue => "ROGUE",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Mode::Grinder => {
                "Lose and you slide back a rung. You still get paid, so grind \
                 the easy one until you can take the hard one."
            }
            Mode::Rogue => {
                "Three losses and it is over. Everything you own goes with it. \
                 You still get paid, so a loss buys you one more shot."
            }
        }
    }
}

/// How many losses a Rogue run survives.
pub const ROGUE_LIVES: u32 = 3;

/// How many board changes can be taken back.
pub const UNDO_DEPTH: usize = 40;

/// How many loose pieces you may carry.
///
/// A tray with no limit turns every shop into "buy it, decide later", and the
/// decision never comes: you end a run holding forty things you meant to look
/// at. Twelve is enough to hold a plan and not enough to hold every plan, so
/// buying something means either using it or selling something first.
pub const INVENTORY_CAP: usize = 12;

/// The board as it stood before a change. Rotations live on the registry
/// rather than the loadout, so both have to be kept or undoing a rotate would
/// put a piece back at the wrong footprint.
#[derive(Clone)]
struct BoardSnapshot {
    loadout: Loadout,
    registry: PieceRegistry,
    /// What you owned and what you had. Buying and selling are board changes
    /// too, and undo used to restore the grids without them: sell a piece and
    /// undo it and the piece came back to the board while the money stayed in
    /// your pocket and the component stayed out of your bag.
    owned: Vec<PieceId>,
    gold: i32,
    /// What the change was, so the interface can say what it undid.
    label: String,
}

/// A quest that came good in the fight just watched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestDone {
    pub from: String,
    pub into: &'static str,
}

/// What a settled fight did to the run, so the GUI can say so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    pub outcome: Outcome,
    /// Gold banked. Paid on a loss too.
    pub reward: i32,
    /// Rungs given back by a Grinder loss.
    pub knocked_back: bool,
    /// Quests that finished during the fight.
    pub quests_done: Vec<QuestDone>,
    /// Lives left, in Rogue. `None` in Grinder.
    pub lives_left: Option<u32>,
    /// The Rogue run ran out of lives and has been wiped back to the start.
    pub run_ended: bool,
    /// A trophy taken off a named creature - gear no shop will ever sell.
    /// `None` on an ordinary rung, on anything but a victory, or when there
    /// was no room in the tray to put it.
    pub dropped: Option<&'static str>,
    /// What the dungeon said on the landing, if a floor was just cleared.
    pub landing: Option<&'static str>,
    /// The class a finished dungeon handed over.
    pub class_won: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleError {
    Place(PlaceError),
    /// Tried to change the loadout mid-fight.
    LoadoutLocked,
    NotEquipped,
    /// Tried to buy something you can't afford.
    NotEnoughGold { need: i32, have: i32 },
    /// Tried to buy from an empty shelf.
    NothingThere,
    /// No room left in the tray for another loose piece.
    TrayFull,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::Place(e) => write!(f, "{}", e),
            RuleError::LoadoutLocked => write!(f, "can't change gear during a fight"),
            RuleError::NotEquipped => write!(f, "that piece isn't equipped"),
            RuleError::NotEnoughGold { need, have } => {
                write!(f, "costs {} gold, you have {}", need, have)
            }
            RuleError::NothingThere => write!(f, "nothing for sale there"),
            RuleError::TrayFull => write!(
                f,
                "your tray is full at {} pieces - wear something or sell something",
                INVENTORY_CAP
            ),
        }
    }
}

impl From<PlaceError> for RuleError {
    fn from(e: PlaceError) -> Self {
        RuleError::Place(e)
    }
}

pub struct Run {
    pub registry: PieceRegistry,
    /// Every component the player owns, in a stable display order. What is in
    /// the inventory is derived from this minus what is in the slots, so the
    /// two can never disagree.
    pub owned: Vec<PieceId>,
    pub loadout: Loadout,
    pub phase: Phase,
    /// Set by `begin_fight`, cleared by `back_to_loadout`.
    pub log: Option<CombatLog>,
    pub gold: i32,
    pub shop: Shop,
    /// How far up the monster ladder you are.
    pub rung: usize,
    pub wins: u32,
    pub losses: u32,
    pub mode: Mode,
    pub difficulty: Difficulty,
    /// The classes the fountains have given you, in the order taken. Every
    /// one of their powers applies at once.
    pub classes: Vec<&'static crate::class::ClassDef>,
    /// The class the third fountain doubled, by name. `None` until then.
    pub doubled: Option<&'static str>,
    /// Standing in for this rung's own creature, because an event put it
    /// there. Cleared when the rung is left.
    pub substitute: Option<&'static MonsterSpec>,
    /// Events already answered, by id, so one is never asked twice.
    pub answered: Vec<&'static str>,
    /// The quickest win this run has managed, in milliseconds. Events that are
    /// earned rather than scheduled read it - see `event::Trigger`.
    pub best_fight_ms: Option<u32>,
    /// Choices actually taken, by label, so a later event can ask what you did
    /// at an earlier one.
    pub took: Vec<&'static str>,
    /// The dungeon being walked and which floor, if any. A dungeon stands off
    /// the road: it never moves the rung, so coming out puts you back in front
    /// of the fight you had not got to.
    pub dungeon: Option<(&'static crate::dungeon::Dungeon, usize)>,
    /// Said on the landing between floors, once.
    pub pending_landing: Option<&'static str>,
    /// Losses this run may take beyond the mode's own allowance. Earned, not
    /// given: there is exactly one place to pick one up.
    pub extra_lives: u32,
    /// Rerolls bought since the last fight. Resets on settling.
    pub rerolls: u32,
    /// A scene the theme owes you for the fight just settled, waiting to be
    /// read. Cleared once it has been.
    pub pending_scene: Option<&'static [&'static str]>,
    /// Creatures whose scene has already been shown, so beating one twice does
    /// not tell you the same thing twice.
    seen_scenes: Vec<&'static str>,
    /// The words this run is played in. Purely a display layer - nothing the
    /// engine decides depends on it - so a run is the same run whichever theme
    /// it is wearing.
    pub theme: &'static crate::theme::Theme,
    /// Maximum health earned by gear that grows, kept for the whole run.
    ///
    /// This is the only number on a character that a fight can leave larger
    /// than it found it. It is what makes a growing piece worth its price: the
    /// health it banked in the last fight is health you start the next one
    /// with, and it goes on compounding for as long as the run does.
    pub grown_health: i32,
    /// Losses left before a Rogue run is wiped. Ignored in Grinder.
    pub lives: u32,
    /// The last settled fight, kept so the GUI can report what it cost.
    pub last_settlement: Option<Settlement>,
    /// The highest rung ever reached, which a Grinder knock-back does not
    /// take away. Only here so a run can say how far it actually got.
    pub best_rung: usize,
    /// Set once a fight's result has been banked, so the reward can't be
    /// claimed twice by replaying the same log.
    settled: bool,
    rng: Rng,
    /// Board states to step back through, oldest first.
    undo_stack: Vec<BoardSnapshot>,
    /// How far each piece's quest has come. Pieces without a quest never
    /// appear; a piece that finishes one is transformed and its entry dropped.
    quest_progress: std::collections::HashMap<PieceId, u32>,
}

impl Default for Run {
    fn default() -> Self {
        Self::new()
    }
}

impl Run {
    /// A fresh run: the basic weapon pair, some gold, and a stocked shop.
    /// Everything beyond that has to be bought.
    pub fn new() -> Self {
        Self::seeded(0x5EED_1234_ABCD_0001)
    }

    /// Same, with the shop's rolls pinned so a test can predict them.
    pub fn seeded(seed: u64) -> Self {
        let mut registry = PieceRegistry::new();
        let mut owned = Vec::new();
        for name in STARTER_KIT {
            if let Some(d) = CATALOG.iter().position(|p| &p.name == name) {
                owned.push(registry.alloc(d));
            }
        }
        let mut rng = Rng::new(seed);
        let shop = Shop::new(&mut rng);
        let mut loadout = Loadout::new();
        loadout.name_seed = seed;
        Run {
            registry,
            owned,
            loadout,
            phase: Phase::Loadout,
            log: None,
            gold: STARTING_GOLD,
            shop,
            rung: 0,
            wins: 0,
            losses: 0,
            mode: Mode::Grinder,
            difficulty: Difficulty::Easy,
            classes: Vec::new(),
            pending_scene: None,
            seen_scenes: Vec::new(),
            theme: crate::theme::THEMES[0],
            grown_health: 0,
            lives: ROGUE_LIVES,
            last_settlement: None,
            doubled: None,
            substitute: None,
            answered: Vec::new(),
            best_fight_ms: None,
            took: Vec::new(),
            dungeon: None,
            pending_landing: None,
            extra_lives: 0,
            rerolls: 0,
            best_rung: 0,
            settled: false,
            rng,
            undo_stack: Vec::new(),
            quest_progress: std::collections::HashMap::new(),
        }
    }

    /// Same, in a chosen mode and difficulty, from a chosen seed. The seed is
    /// what makes two runs stock different shops.
    pub fn start(seed: u64, mode: Mode, difficulty: Difficulty) -> Self {
        let mut run = Self::seeded(seed);
        run.mode = mode;
        run.difficulty = difficulty;
        run
    }

    /// Same, in a chosen set of words. The theme changes nothing the engine
    /// decides - it survives a Rogue wipe for that reason, being a property of
    /// the sitting rather than of the run.
    pub fn start_themed(
        seed: u64,
        mode: Mode,
        difficulty: Difficulty,
        theme: &'static crate::theme::Theme,
    ) -> Self {
        let mut run = Self::start(seed, mode, difficulty);
        run.set_theme(theme);
        run
    }

    /// Change the words. The name generator draws from the theme's corpora, so
    /// the loadout has to be told as well as the run.
    pub fn set_theme(&mut self, theme: &'static crate::theme::Theme) {
        self.theme = theme;
        self.loadout.naming = theme.naming;
    }

    /// Same, in a chosen mode.
    pub fn with_mode(mode: Mode) -> Self {
        let mut run = Self::new();
        run.mode = mode;
        run
    }

    /// Every component in the catalog, for the preset, the tests, and the
    /// AUTO-BUILD button. Bypasses the shop entirely.
    pub fn with_all_pieces() -> Self {
        let mut run = Self::new();
        run.owned.clear();
        run.registry = PieceRegistry::new();
        run.owned = all_def_indices().into_iter().map(|d| run.registry.alloc(d)).collect();
        run
    }

    /// The monster you are facing now.
    /// What the next reroll costs.
    ///
    /// Doubling, from one: 1, 2, 4, 8. A flat price meant a player with money
    /// could simply keep asking until the shelves said what they wanted, which
    /// made the shop a formality rather than a decision. It resets after every
    /// fight, so the pressure is inside a single visit and never carries.
    pub fn reroll_cost(&self) -> i32 {
        REROLL_COST << self.rerolls.min(16)
    }

    /// Is the player without an assembled weapon? The shop guarantees one can
    /// be built only when the answer is yes.
    pub fn needs_a_weapon(&self) -> bool {
        self.report(SlotKind::Weapon).items.iter().all(|i| !i.assembled)
    }

    pub fn monster(&self) -> &'static MonsterSpec {
        // An event can put something else in front of you. It stands in for
        // the rung rather than adding one, so the road stays the same length
        // whichever way you answered.
        // A dungeon floor stands in front of everything else.
        if let Some((d, floor)) = self.dungeon {
            if let Some(spec) = d.floors.get(floor).and_then(|n| crate::combat::alternate(n)) {
                return spec;
            }
        }
        if let Some(m) = self.substitute {
            return m;
        }
        &LADDER[self.rung.min(LADDER.len() - 1)]
    }

    // ------------------------------------------------------------ events

    /// The event standing in front of this rung, if there is one and it has
    /// not been answered.
    pub fn pending_event(&self) -> Option<&'static crate::event::LadderEvent> {
        if self.phase != Phase::Loadout || self.at_fountain() || self.at_doubling_fountain() {
            return None;
        }
        crate::event::at(self.rung, self.best_fight_ms)
            .filter(|e| !self.answered.contains(&e.id))
    }

    /// Loose components that would satisfy `req`, as ids.
    ///
    /// Loose only: what you are wearing is not on the table. Handing something
    /// over has to cost you something you could have used.
    pub fn offerings(&self, req: crate::event::Requirement) -> Vec<PieceId> {
        use crate::event::Requirement;
        if req == Requirement::None {
            return Vec::new();
        }
        self.inventory()
            .into_iter()
            .filter(|&id| {
                let cells: Vec<(u8, u8)> = self
                    .registry
                    .shape(id)
                    .cells()
                    .iter()
                    .map(|&(x, y)| (x as u8, y as u8))
                    .collect();
                req.met_by_shape(&cells)
            })
            .collect()
    }

    /// Can this choice be taken right now?
    pub fn choice_open(&self, c: &crate::event::Choice) -> bool {
        use crate::event::Requirement;
        match c.requires {
            Requirement::None => true,
            Requirement::Took(label) => self.took.contains(&label),
            // Worn or loose, both count: a key you have built into a helmet is
            // still a key you have.
            Requirement::Holding(name) => {
                self.owned.iter().any(|&id| self.registry.def(id).name == name)
            }
            Requirement::LooseItemOfSize { .. } => !self.offerings(c.requires).is_empty(),
        }
    }

    /// Answer the event in front of you.
    ///
    /// Returns what it cost, if anything - the component handed over, by name -
    /// so the interface can say what just happened. Refuses a choice whose
    /// requirement is not met, so the offer cannot be widened by asking
    /// differently.
    pub fn take_choice(&mut self, c: &crate::event::Choice) -> Option<&'static str> {
        use crate::event::Outcome as ChoiceOutcome;
        let Some(ev) = self.pending_event() else { return None };
        if !self.choice_open(c) {
            return None;
        }
        self.answered.push(ev.id);
        self.took.push(c.label);
        let mut gave = None;
        match c.outcome {
            ChoiceOutcome::FightAsWritten => {}
            ChoiceOutcome::FightInstead(name) => {
                self.substitute = crate::combat::alternate(name);
            }
            ChoiceOutcome::Spare => self.grant_life(),
            ChoiceOutcome::Give(name) => {
                if let Some(d) = crate::piece::CATALOG.iter().position(|d| d.name == name) {
                    let id = self.registry.alloc(d);
                    self.owned.push(id);
                }
            }
            ChoiceOutcome::Claim(name) => {
                if let Some(c) = crate::class::CLASSES.iter().find(|c| c.name == name) {
                    if !self.classes.iter().any(|k| k.name == c.name) {
                        self.classes.push(c);
                    }
                }
            }
            ChoiceOutcome::Enter(id) => {
                self.dungeon = crate::dungeon::by_id(id).map(|d| (d, 0));
            }
            ChoiceOutcome::BuyOff { times } => {
                if let Some(&id) = self.offerings(c.requires).first() {
                    gave = Some(self.registry.def(id).name);
                    self.owned.retain(|&o| o != id);
                }
                self.gold += LADDER[self.rung.min(LADDER.len() - 1)].bounty * times;
                // Paid off rather than beaten: the rung is behind you, but it
                // was never fought, so it is not a win.
                self.rung += 1;
                self.best_rung = self.best_rung.max(self.rung);
                let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
            }
        }
        gave
    }

    /// True once the ladder has been cleared.
    pub fn ladder_complete(&self) -> bool {
        self.rung >= LADDER.len()
    }

    /// Buy the component on shelf `slot`.
    pub fn buy(&mut self, slot: usize) -> Result<PieceId, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let price = self.shop.price(slot).ok_or(RuleError::NothingThere)?;
        if self.gold < price {
            return Err(RuleError::NotEnoughGold { need: price, have: self.gold });
        }
        if self.inventory().len() >= INVENTORY_CAP {
            return Err(RuleError::TrayFull);
        }
        self.remember("buying");
        let def = self.shop.take(slot).ok_or(RuleError::NothingThere)?;
        self.gold -= price;
        let id = self.registry.alloc(def);
        self.owned.push(id);
        Ok(id)
    }

    /// Reroll the shelves. Cheap, but it is gold you are not spending on gear.
    pub fn reroll(&mut self) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let cost = self.reroll_cost();
        if self.gold < cost {
            return Err(RuleError::NotEnoughGold { need: cost, have: self.gold });
        }
        self.gold -= cost;
        self.rerolls += 1;
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
        Ok(())
    }

    /// Sell a component back for half its price, rounded down. Equipped pieces
    /// come off first.
    pub fn sell(&mut self, id: PieceId) -> Result<i32, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let refund = crate::rating::resale_price(self.registry.def(id));
        self.remember(format!("selling {}", self.registry.def(id).name));
        self.loadout.remove_anywhere(id);
        self.owned.retain(|&o| o != id);
        // Selling a piece out of a locked item ends the lock: what is left is
        // not that item any more, and a lock holding a sold piece would keep
        // reporting an item that no longer exists.
        self.loadout.locks.retain(|l| !l.pieces.contains(&id));
        self.gold += refund;
        Ok(refund)
    }

    /// Bank the result of the fight just watched: pay the bounty, move the
    /// ladder, and turn the shop over. Idempotent, so the GUI can call it when
    /// playback finishes without worrying about repeats.
    ///
    /// The bounty is paid whatever happened. Losing is meant to be a setback,
    /// not a dead end: a run with no income cannot buy its way past whatever
    /// just beat it, and would have nothing to do but replay a fight it
    /// already knows it loses. What losing actually costs is set by the mode.
    pub fn settle(&mut self) -> Option<i32> {
        if self.settled {
            return None;
        }
        let outcome = self.log.as_ref()?.outcome;
        self.settled = true;
        // A fresh shop is a fresh price. The escalation is meant to bite
        // inside one visit, not to follow you up the ladder.
        self.rerolls = 0;

        // Whatever your gear grew, you keep - win or lose. The work was done
        // either way, and a piece that only paid on a win would be worth
        // nothing in the fights where you actually need it.
        //
        // A stalemate is the exception, and it has to be. Nothing banks more
        // growth than surviving the full clock, so counting it would make
        // failing to finish the most profitable thing a growing build could
        // do - and the knock-back means it can be repeated for ever. A fight
        // you did not finish leaves you nothing.
        let grew: i32 = self
            .log
            .as_ref()
            .filter(|l| l.outcome != Outcome::Stalemate)
            .map(|l| {
                l.entries
                    .iter()
                    .filter_map(|e| match e.event {
                        Event::Grew { side: Side::Player, amount, .. } => Some(amount),
                        _ => None,
                    })
                    .sum()
            })
            .unwrap_or(0);
        self.grown_health += grew;

        let bounty = self.monster().bounty;
        self.gold += bounty;

        let mut settlement = Settlement {
            outcome,
            reward: bounty,
            knocked_back: false,
            quests_done: self.award_quests(),
            lives_left: None,
            run_ended: false,
            dropped: None,
            landing: None,
            class_won: None,
        };

        // The quickest win the run has had, which is what earns the casino.
        // Only a real win counts: a stalemate lasts the full clock by
        // definition, and a defeat that ended in half a second is not a fight
        // you won quickly.
        if outcome == Outcome::Victory {
            if let Some(ms) = self.log.as_ref().map(|l| l.duration_ms) {
                self.best_fight_ms = Some(self.best_fight_ms.map_or(ms, |b| b.min(ms)));
            }
        }

        match outcome {
            Outcome::Victory if self.dungeon.is_some() => {
                // A floor cleared moves you down, not along. The rung does not
                // change, so coming out of a dungeon puts you back in front of
                // the fight you had not got to.
                self.wins += 1;
                let (d, floor) = self.dungeon.expect("just checked");
                settlement.landing = d.landings.get(floor).copied();
                self.pending_landing = settlement.landing;
                if floor + 1 < d.floors.len() {
                    self.dungeon = Some((d, floor + 1));
                } else {
                    // Out the other side, with the thing you went in for.
                    self.dungeon = None;
                    if let Some(c) =
                        crate::class::CLASSES.iter().find(|c| c.name == d.reward)
                    {
                        if !self.classes.iter().any(|k| k.name == c.name) {
                            self.classes.push(c);
                            settlement.class_won = Some(c.name);
                        }
                    }
                }
                self.shop.restock(&mut self.rng, false);
            }
            Outcome::Victory => {
                self.wins += 1;
                // A scene is owed for beating this thing, if the theme has one
                // and has not already told it.
                let beaten = LADDER[self.rung.min(LADDER.len() - 1)].name;
                if !self.seen_scenes.contains(&beaten) {
                    if let Some(scene) = self.theme.cutscene(beaten) {
                        self.seen_scenes.push(beaten);
                        self.pending_scene = Some(scene);
                    }
                }
                // A named creature leaves something behind. It is the only
                // way any of this gear is ever obtainable: it is barred from
                // the shop, and it is off the scale for its slot on purpose.
                //
                // No room in the tray means no drop, and it says so rather
                // than silently binning it - twelve is the cap, and a player
                // who wants the trophy can make space and beat the thing
                // again.
                let spec = &LADDER[self.rung.min(LADDER.len() - 1)];
                if !spec.drops.is_empty() && self.inventory().len() < INVENTORY_CAP {
                    let pick = self.rng.below(spec.drops.len());
                    let name = spec.drops[pick];
                    if let Some(def) = crate::piece::CATALOG.iter().position(|d| d.name == name) {
                        let id = self.registry.alloc(def);
                        self.owned.push(id);
                        settlement.dropped = Some(name);
                    }
                }
                self.rung += 1;
                self.best_rung = self.best_rung.max(self.rung);
                // Whatever stood in for that rung is done standing in.
                self.substitute = None;
            }
            // A draw or a defeat both mean the thing is still standing, so
            // neither advances the ladder.
            _ => {
                self.losses += 1;
                // Losing in a dungeon puts you out of it. The door does not
                // reopen - you sold the thing that opened it.
                self.dungeon = None;
                match self.mode {
                    Mode::Grinder => {
                        // Back to the rung you last cleared, so there is
                        // always something easier to farm.
                        if self.rung > 0 {
                            self.rung -= 1;
                            settlement.knocked_back = true;
                        }
                    }
                    Mode::Rogue => {
                        self.lives = self.lives.saturating_sub(1);
                        settlement.lives_left = Some(self.lives);
                        if self.lives == 0 {
                            settlement.run_ended = true;
                        }
                    }
                }
            }
        }

        // New shelves after every battle, win or lose.
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);

        let ended = settlement.run_ended;
        self.last_settlement = Some(settlement);
        if ended {
            // Everything goes: gear, gold, ladder. The mode and the seed
            // survive so the player lands straight into a fresh run.
            self.wipe();
        }
        Some(bounty)
    }

    /// Take the next rung without fighting for it, paid as though you had won.
    ///
    /// Exists because the early rungs get played many times over - once to
    /// learn them and once for every later idea that has to start from the
    /// bottom - and because the numbers further up are much easier to test
    /// when reaching them is not itself the work. It pays the full bounty, so
    /// a skipped rung leaves the run exactly as beating it would have.
    ///
    /// Returns the bounty, or `None` if there is nothing left to skip.
    pub fn skip_fight(&mut self) -> Option<i32> {
        self.skip_to(self.rung + 1)
    }

    /// Walk up to `target` without fighting for any of it, paid as though every
    /// rung on the way had been won.
    ///
    /// Only ever upwards: going back down is what losing is for, and a ladder
    /// that can be walked in both directions is not a ladder. Every rung
    /// crossed pays its own bounty, so arriving at rung twenty by this road
    /// leaves the same purse as arriving by the long one.
    ///
    /// Returns the total paid, or `None` if there is nothing to walk to.
    /// Settle a win without simulating one.
    ///
    /// For tests and the ladder picker: what is under test is usually the
    /// settlement - which floor you move to, what drops - rather than whether
    /// a particular build could take the fight.
    pub fn force_win(&mut self) {
        self.log = Some(crate::combat::CombatLog::won_by_default(self.monster()));
        self.settled = false;
        self.settle();
    }

    pub fn skip_to(&mut self, target: usize) -> Option<i32> {
        if self.phase != Phase::Loadout || target <= self.rung || target >= LADDER.len() {
            return None;
        }
        let mut paid = 0;
        while self.rung < target {
            paid += self.monster().bounty;
            self.wins += 1;
            self.rung += 1;
        }
        self.gold += paid;
        self.best_rung = self.best_rung.max(self.rung);
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
        // Quests want a fight to have happened, so a skipped rung does not
        // advance them. Skipping past a quest is the cost of skipping.
        self.last_settlement = None;
        Some(paid)
    }

    /// Throw the run away and start over, keeping only the mode. What a Rogue
    /// run does when it runs out of lives.
    pub fn wipe(&mut self) {
        let mode = self.mode;
        let theme = self.theme;
        let settlement = self.last_settlement.take();
        let seed = self.rng.next_u64();
        let mut fresh = Run::seeded(seed);
        fresh.mode = mode;
        fresh.difficulty = self.difficulty;
        fresh.classes = Vec::new();
        fresh.grown_health = 0;
        fresh.set_theme(theme);
        fresh.pending_scene = None;
        fresh.last_settlement = settlement;
        // The fight just watched stays on screen; the GUI is still replaying
        // it and needs somewhere to go back to.
        fresh.log = self.log.take();
        fresh.phase = self.phase;
        fresh.settled = true;
        *self = fresh;
        self.forget_undo();
    }

    // ------------------------------------------------------------- locks

    /// Fix an assembled item in place, or release one. Returns whether it is
    /// locked afterwards.
    ///
    /// A locked item stops negotiating with its neighbours: nothing can join
    /// it and it cannot lose a piece. From then on it behaves like a single
    /// large component - it turns as one, and it comes off the board as one.
    pub fn toggle_lock_item(&mut self, piece: PieceId) -> bool {
        if let Some(at) = self.loadout.locks.iter().position(|l| l.pieces.contains(&piece)) {
            self.remember("releasing an item");
            self.loadout.locks.remove(at);
            return false;
        }
        let Some(kind) = self.loadout.slot_holding(piece) else { return false };
        let Some(item) = self
            .report(kind)
            .items
            .into_iter()
            .find(|i| i.assembled && i.pieces.contains(&piece))
        else {
            return false;
        };
        self.remember("locking an item");
        let offsets = self.shape_of(kind, &item.pieces);
        self.loadout.locks.push(LockedItem { pieces: item.pieces, offsets });
        true
    }

    /// Where each of `pieces` sits relative to the group's top-left corner.
    fn shape_of(&self, kind: SlotKind, pieces: &[PieceId]) -> Vec<(u8, u8)> {
        let slot = self.loadout.slot(kind);
        let anchors: Vec<(u8, u8)> =
            pieces.iter().map(|&p| slot.anchor_of(p).unwrap_or((0, 0))).collect();
        let minx = anchors.iter().map(|(x, _)| *x).min().unwrap_or(0);
        let miny = anchors.iter().map(|(_, y)| *y).min().unwrap_or(0);
        anchors.iter().map(|&(x, y)| (x - minx, y - miny)).collect()
    }

    pub fn locked_set(&self, piece: PieceId) -> Option<&[PieceId]> {
        self.loadout
            .locks
            .iter()
            .find(|l| l.pieces.contains(&piece))
            .map(|l| l.pieces.as_slice())
    }

    /// The pieces of a locked item and where each sits relative to the item's
    /// own top-left, so it can be carried and put back down as one shape.
    pub fn locked_shape(&self, piece: PieceId) -> Option<Vec<(PieceId, u8, u8)>> {
        let l = self.loadout.locks.iter().find(|l| l.pieces.contains(&piece))?;
        Some(
            l.pieces
                .iter()
                .zip(l.offsets.iter())
                .map(|(&p, &(dx, dy))| (p, dx, dy))
                .collect(),
        )
    }

    /// Put a locked item back on the board with its top-left at `(ax, ay)`.
    ///
    /// All of it or none of it: a locked item that lands half on the grid is
    /// not a locked item any more.
    pub fn equip_locked_at(
        &mut self,
        piece: PieceId,
        kind: SlotKind,
        ax: u8,
        ay: u8,
    ) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let Some(shape) = self.locked_shape(piece) else {
            return Err(RuleError::NotEquipped);
        };
        // Every piece has to fit before any of them is placed, or a rejected
        // drop would leave the item scattered across the grid.
        for &(p, dx, dy) in &shape {
            let (x, y) = (ax as u32 + dx as u32, ay as u32 + dy as u32);
            if x >= SLOT_W as u32 || y >= SLOT_H as u32 {
                return Err(RuleError::Place(PlaceError::OutOfBounds));
            }
            self.loadout.can_place(&self.registry, p, kind, x as u8, y as u8)?;
        }
        self.remember("placing a locked item");
        for &(p, dx, dy) in &shape {
            self.loadout.slot_mut(kind).place(&self.registry, p, ax + dx, ay + dy);
        }
        Ok(())
    }

    pub fn is_locked_item(&self, piece: PieceId) -> bool {
        self.locked_set(piece).is_some()
    }

    /// Turn a locked item a quarter turn, as though it were one component.
    ///
    /// Every piece turns, and the whole footprint turns with it: a cell at
    /// `(x, y)` in the item's bounding box lands at `(height - 1 - y, x)`.
    /// Refused, and rolled back, if the result would not fit.
    pub fn rotate_locked(&mut self, piece: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let Some(set) = self.locked_set(piece).map(|s| s.to_vec()) else {
            return Err(RuleError::NotEquipped);
        };
        let Some(kind) = self.loadout.slot_holding(piece) else {
            return Err(RuleError::NotEquipped);
        };

        let slot = self.loadout.slot(kind);
        let cells: Vec<(PieceId, Vec<(u8, u8)>)> =
            set.iter().map(|&p| (p, slot.cells_of(p))).collect();
        let minx = cells.iter().flat_map(|(_, c)| c.iter().map(|(x, _)| *x)).min().unwrap_or(0);
        let miny = cells.iter().flat_map(|(_, c)| c.iter().map(|(_, y)| *y)).min().unwrap_or(0);
        let maxy = cells.iter().flat_map(|(_, c)| c.iter().map(|(_, y)| *y)).max().unwrap_or(0);
        let height = maxy - miny + 1;

        // Where each piece's own footprint lands once the item has turned.
        let mut want: Vec<(PieceId, u8, u8)> = Vec::new();
        for (p, cs) in &cells {
            let turned: Vec<(u8, u8)> = cs
                .iter()
                .map(|&(x, y)| (minx + (height - 1 - (y - miny)), miny + (x - minx)))
                .collect();
            let ax = turned.iter().map(|(x, _)| *x).min().unwrap_or(0);
            let ay = turned.iter().map(|(_, y)| *y).min().unwrap_or(0);
            want.push((*p, ax, ay));
        }

        self.remember("turning a locked item");
        let before: Vec<(PieceId, u8, u8, u8)> = cells
            .iter()
            .map(|(p, _)| {
                let a = self.loadout.slot(kind).anchor_of(*p).unwrap_or((0, 0));
                (*p, a.0, a.1, self.registry.rotation(*p))
            })
            .collect();

        for &(p, ..) in &before {
            self.loadout.slot_mut(kind).remove(p);
            self.registry.rotate_cw(p);
        }
        let mut ok = true;
        for &(p, ax, ay) in &want {
            if self.loadout.can_place(&self.registry, p, kind, ax, ay).is_ok() {
                self.loadout.slot_mut(kind).place(&self.registry, p, ax, ay);
            } else {
                ok = false;
                break;
            }
        }
        if !ok {
            for &(p, ax, ay, rot) in &before {
                self.loadout.slot_mut(kind).remove(p);
                self.registry.set_rotation(p, rot);
                self.loadout.slot_mut(kind).place(&self.registry, p, ax, ay);
            }
            self.undo_stack.pop();
            return Err(RuleError::Place(PlaceError::OutOfBounds));
        }
        // The item has a new shape now, and the stored one is what puts it back
        // down if it is lifted into the inventory.
        let offsets = self.shape_of(kind, &set);
        if let Some(l) = self.loadout.locks.iter_mut().find(|l| l.pieces.contains(&piece)) {
            l.offsets = offsets;
        }
        Ok(())
    }

    /// Take a whole locked item off the board.
    pub fn unequip_locked(&mut self, piece: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let Some(set) = self.locked_set(piece).map(|s| s.to_vec()) else {
            return Err(RuleError::NotEquipped);
        };
        self.remember("removing a locked item");
        for p in set {
            self.loadout.remove_anywhere(p);
        }
        Ok(())
    }

    /// The inventory, with locked items kept together as one entry. A locked
    /// item off the board is carried around as a single thing.
    pub fn inventory_groups(&self) -> Vec<Vec<PieceId>> {
        let loose = self.inventory();
        let mut out: Vec<Vec<PieceId>> = Vec::new();
        let mut taken: Vec<PieceId> = Vec::new();
        for &id in &loose {
            if taken.contains(&id) {
                continue;
            }
            match self.locked_set(id) {
                Some(set) if set.iter().all(|p| loose.contains(p)) => {
                    taken.extend(set.iter().copied());
                    out.push(set.to_vec());
                }
                _ => out.push(vec![id]),
            }
        }
        out
    }

    // ----------------------------------------------------------- classes

    /// Which rung is the fairy fountain rather than a monster. You meet it
    /// after the fourth battle.
    /// Rungs where a fountain stands instead of a fight.
    ///
    /// The first sits past the Iron Sentinel, which is far enough in that a
    /// build has a shape worth reading; the second past the Hollow King, by
    /// which point that shape has usually changed enough to be worth reading
    /// again. Each hands over a class you do not already hold, so the second
    /// adds to the first rather than replacing it.
    pub const FOUNTAINS: &'static [usize] = &[7, 14];

    /// The rung the third fountain stands on - in front of the third boss.
    ///
    /// A different thing from the other two. Those hand over a class you do
    /// not hold; this one takes a class you already have and doubles it. By
    /// the third boss a build has stopped being a collection of ideas and
    /// become one idea, and this is where the game agrees with that.
    pub const DOUBLING_FOUNTAIN: usize = 46;

    /// Is the tray at its limit? Loose pieces only - what you are wearing does
    /// not count against it.
    pub fn tray_full(&self) -> bool {
        self.inventory().len() >= INVENTORY_CAP
    }

    /// How many fights away the next named creature is, and which kind.
    ///
    /// Whichever is closer. A boss two rungs off matters more than a mini-boss
    /// five off, and the player should be able to see one coming rather than
    /// walking into fifteen items of gear having spent their gold.
    pub fn next_named(&self) -> Option<(usize, crate::combat::Rank, &'static str)> {
        LADDER
            .iter()
            .enumerate()
            .skip(self.rung)
            .find(|(_, m)| m.rank != crate::combat::Rank::Ordinary)
            .map(|(i, m)| (i - self.rung, m.rank, m.name))
    }

    /// Is the third fountain standing here, and still owed?
    pub fn at_doubling_fountain(&self) -> bool {
        self.rung == Self::DOUBLING_FOUNTAIN
            && self.doubled.is_none()
            && !self.doubling_offer().is_empty()
    }

    /// Which of the classes you hold this fountain could double.
    ///
    /// Not all of them: a power that is a switch rather than a number has no
    /// second helping, and the fountain does not offer what it cannot give.
    pub fn doubling_offer(&self) -> Vec<&'static crate::class::ClassDef> {
        self.classes.iter().copied().filter(|c| c.power.doubled().is_some()).collect()
    }

    /// Drink from it. Refuses anything it is not offering.
    pub fn double_class(&mut self, choice: &'static crate::class::ClassDef) -> bool {
        if self.doubled.is_some() || !self.doubling_offer().iter().any(|c| c.name == choice.name) {
            return false;
        }
        self.doubled = Some(choice.name);
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
        true
    }

    /// Every class you hold, with the doubled one already doubled - what a
    /// fight actually runs on.
    pub fn effective_classes(&self) -> Vec<crate::class::ClassDef> {
        self.classes
            .iter()
            .map(|c| {
                let mut c = **c;
                if self.doubled == Some(c.name) {
                    if let Some(p) = c.power.doubled() {
                        c.power = p;
                    }
                }
                c
            })
            .collect()
    }

    /// Is the next thing on the ladder a fountain?
    ///
    /// A fountain stands *between* rungs rather than on one: drinking does not
    /// move you up, so the creature at that rung is still there to be fought
    /// afterwards. Advancing past it - which is what this used to do - quietly
    /// deleted a monster from every run.
    pub fn at_fountain(&self) -> bool {
        Self::FOUNTAINS.get(self.classes.len()) == Some(&self.rung)
    }

    /// The rung the next fountain stands on, if there is one left.
    pub fn next_fountain(&self) -> Option<usize> {
        Self::FOUNTAINS.get(self.classes.len()).copied()
    }

    /// Measure the build as it stands. What the fountain will read, and what
    /// the interface shows you beforehand so the outcome is never a surprise.
    pub fn fingerprint(&self) -> crate::class::Fingerprint {
        let filled: usize = SlotKind::ALL
            .iter()
            .map(|&k| {
                let slot = self.loadout.slot(k);
                slot.pieces().iter().map(|&p| slot.cells_of(p).len()).sum::<usize>()
            })
            .sum();
        crate::class::Fingerprint::of(&self.registry, &self.combat_items(), filled)
    }

    /// Every class ranked against the build right now, eligible ones first.
    pub fn class_outlook(&self) -> Vec<crate::class::Match> {
        crate::class::rank(&self.fingerprint())
    }

    /// Take the imbuement. Returns the class given.
    /// What the fountain is willing to hand over: the class your build earns,
    /// the two it comes nearest to, and one drawn out of the water.
    ///
    /// Never something you already hold - a second fountain that read you the
    /// same way as the first would be a rung of nothing.
    pub fn fountain_offer(&self) -> Vec<&'static crate::class::ClassDef> {
        let held: Vec<&str> = self.classes.iter().map(|c| c.name).collect();
        let ranked = crate::class::rank(&self.fingerprint());
        let mut out: Vec<&'static crate::class::ClassDef> = ranked
            .iter()
            .filter(|m| !held.contains(&m.class.name))
            .take(3)
            .map(|m| m.class)
            .collect();

        // And a wildcard, which is the only way to end up somewhere your gear
        // was not already pointing. Drawn from the run's own stream so it is
        // the same offer every time you look at this fountain.
        let pool: Vec<&'static crate::class::ClassDef> = crate::class::CLASSES
            .iter()
            .filter(|c| !held.contains(&c.name))
            .filter(|c| !out.iter().any(|o| o.name == c.name))
            .collect();
        if !pool.is_empty() {
            let mut rng = Rng::new(self.wildcard_seed());
            out.push(pool[(rng.next_u64() % pool.len() as u64) as usize]);
        }
        out
    }

    /// A seed fixed to this fountain, so the wildcard does not reshuffle every
    /// time the panel redraws.
    fn wildcard_seed(&self) -> u64 {
        0x9E37_79B9_7F4A_7C15 ^ (self.rung as u64) << 17 ^ (self.classes.len() as u64) << 3
    }

    /// Take a named class from the fountain. Refuses anything it is not
    /// offering, so the choice cannot be widened by asking differently.
    pub fn drink_choosing(
        &mut self,
        choice: &'static crate::class::ClassDef,
    ) -> Option<&'static crate::class::ClassDef> {
        if !self.fountain_offer().iter().any(|c| c.name == choice.name) {
            return None;
        }
        self.classes.push(choice);
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
        Some(choice)
    }

    pub fn drink(&mut self) -> &'static crate::class::ClassDef {
        // Never the same twice: a second fountain that read you the same way
        // as the first would be a rung of nothing.
        let held: Vec<&str> = self.classes.iter().map(|c| c.name).collect();
        let class = crate::class::rank(&self.fingerprint())
            .into_iter()
            .find(|m| m.eligible && !held.contains(&m.class.name))
            .map(|m| m.class)
            .unwrap_or_else(|| crate::class::classify(&self.fingerprint()));
        self.classes.push(class);
        // A fountain is not a fight and does not stand on a rung of its own,
        // so the ladder does not move. The shelves still turn over: drinking
        // is a moment between fights like any other.
        let need = self.needs_a_weapon();
        self.shop.restock(&mut self.rng, need);
        class
    }

    // ------------------------------------------------------------ quests

    /// How far along a piece's quest is.
    pub fn quest_progress(&self, id: PieceId) -> u32 {
        self.quest_progress.get(&id).copied().unwrap_or(0)
    }

    /// Tally every quest against the fight just watched, and transform any
    /// piece that finished one.
    ///
    /// Read off the log afterwards rather than tracked during the fight: the
    /// simulation stays a pure function of stats and gear, and quests become
    /// something the run does with the record of what happened.
    fn award_quests(&mut self) -> Vec<QuestDone> {
        let Some(log) = self.log.as_ref() else { return Vec::new() };
        let profiles = self.combat_items();

        // Only what the player's own gear did counts.
        let mut activations: Vec<usize> = Vec::new();
        let mut curses_landed = 0u32;
        for entry in &log.entries {
            match &entry.event {
                Event::Activate { side: Side::Player, index, .. } => activations.push(*index),
                Event::Cursed { on: Side::Enemy, .. } => curses_landed += 1,
                _ => {}
            }
        }

        let mut earned: Vec<(PieceId, u32)> = Vec::new();
        for (i, profile) in profiles.iter().enumerate() {
            for &piece in &profile.pieces {
                let Some(quest) = self.registry.def(piece).quest else { continue };
                let count = match quest.track {
                    // A piece is only on duty while its item is assembled, and
                    // `combat_items` only ever returns assembled items - so
                    // simply being in this loop is the check.
                    QuestTrack::SelfActivations => {
                        activations.iter().filter(|&&a| a == i).count() as u32
                    }
                    QuestTrack::AdjacentActivations => activations
                        .iter()
                        .filter(|&&a| profile.adjacent_items.contains(&a))
                        .count() as u32,
                    QuestTrack::AlignedActivations { word } => activations
                        .iter()
                        .filter(|&&a| profile.aligned_items.contains(&a))
                        .filter(|&&a| self.item_uses_word(&profiles, a, word))
                        .count() as u32,
                    QuestTrack::CursesLanded => curses_landed,
                };
                if count > 0 {
                    earned.push((piece, count));
                }
            }
        }

        let mut done = Vec::new();
        for (piece, count) in earned {
            let quest = match self.registry.def(piece).quest {
                Some(q) => q,
                None => continue,
            };
            let was = self.quest_progress(piece);
            let now = was + count;
            self.quest_progress.insert(piece, now);
            if now >= quest.goal {
                let from = self.registry.def(piece).name;
                if let Some(target) = CATALOG.iter().position(|d| d.name == quest.becomes) {
                    // The new component may not belong where the old one sat -
                    // a helmet frame can finish as a weapon piece - so take it
                    // off the board and hand it back to the inventory.
                    self.loadout.remove_anywhere(piece);
                    self.registry.transform(piece, target);
                    self.quest_progress.remove(&piece);
                    done.push(QuestDone { from: from.to_string(), into: quest.becomes });
                }
            }
        }
        // A transformation changes shapes on the board, so the history no
        // longer describes anything that can be put back.
        if !done.is_empty() {
            self.forget_undo();
        }
        done
    }

    /// Is item `idx` built from a component whose name contains `word`?
    fn item_uses_word(
        &self,
        profiles: &[crate::loadout::ItemProfile],
        idx: usize,
        word: &str,
    ) -> bool {
        profiles.get(idx).map(|p| {
            p.pieces.iter().any(|&q| self.registry.def(q).name.contains(word))
        }) == Some(true)
    }

    // ------------------------------------------------------------- undo

    /// Remember the board before a change. Called by every method that moves
    /// something, so `undo` can put it back.
    ///
    /// Only the board is kept. Gold and the shop deliberately are not: undo is
    /// for "that was the wrong square", not for taking a purchase back.
    fn remember(&mut self, what: impl Into<String>) {
        self.undo_stack.push(BoardSnapshot {
            loadout: self.loadout.clone(),
            registry: self.registry.clone(),
            owned: self.owned.clone(),
            gold: self.gold,
            label: what.into(),
        });
        if self.undo_stack.len() > UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
    }

    /// Step the board back one change, returning what was undone.
    pub fn undo(&mut self) -> Option<String> {
        if self.phase != Phase::Loadout {
            return None;
        }
        let snap = self.undo_stack.pop()?;
        self.loadout = snap.loadout;
        self.registry = snap.registry;
        self.owned = snap.owned;
        self.gold = snap.gold;
        Some(snap.label)
    }

    /// What the next undo would take back, if anything.
    pub fn undoable(&self) -> Option<&str> {
        self.undo_stack.last().map(|s| s.label.as_str())
    }

    /// Drop the history. Used when the board stops being the one the history
    /// describes - a fight ending, or a run being wiped.
    pub fn forget_undo(&mut self) {
        self.undo_stack.clear();
    }

    /// Losses this run may still take. `None` outside Rogue.
    pub fn lives_left(&self) -> Option<u32> {
        match self.mode {
            Mode::Grinder => None,
            Mode::Rogue => Some(self.lives),
        }
    }

    /// Grant one more loss before the run ends. Rogue counts them down; in
    /// Grinder there is nothing to count, and the choice says so.
    pub fn grant_life(&mut self) {
        self.extra_lives += 1;
        self.lives += 1;
    }

    /// Components not currently in a slot, in stable order.
    pub fn inventory(&self) -> Vec<PieceId> {
        self.owned
            .iter()
            .copied()
            .filter(|id| self.loadout.slot_holding(*id).is_none())
            .collect()
    }

    pub fn is_equipped(&self, id: PieceId) -> bool {
        self.loadout.slot_holding(id).is_some()
    }

    /// Can `id` be dropped into `kind` with its anchor at `(ax, ay)`? Pure
    /// query — the GUI calls this every frame while dragging so it can tint
    /// the preview, and must never work the answer out for itself.
    pub fn can_equip(
        &self,
        id: PieceId,
        kind: SlotKind,
        ax: u8,
        ay: u8,
    ) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        // A piece being moved within its own slot shouldn't collide with
        // itself; `Slot::can_place` already allows that. Moving between slots
        // is checked against the destination as it currently stands, which is
        // correct because the source slot is a different grid.
        Ok(self.loadout.can_place(&self.registry, id, kind, ax, ay)?)
    }

    /// Place `id` into `kind` at `(ax, ay)`, taking it out of wherever it was.
    /// Ordering:
    ///   1. reject if the loadout is locked or the destination doesn't fit
    ///   2. lift the piece out of any slot currently holding it
    ///   3. write it into the destination
    pub fn equip(&mut self, id: PieceId, kind: SlotKind, ax: u8, ay: u8) -> Result<(), RuleError> {
        self.can_equip(id, kind, ax, ay)?;
        let moving = self.is_equipped(id);
        self.remember(format!(
            "{} {}",
            if moving { "moving" } else { "placing" },
            self.registry.def(id).name
        ));
        self.loadout.remove_anywhere(id);
        self.loadout.slot_mut(kind).place(&self.registry, id, ax, ay);
        Ok(())
    }

    /// Take `id` off and return it to the inventory.
    pub fn unequip(&mut self, id: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        if !self.is_equipped(id) {
            return Err(RuleError::NotEquipped);
        }
        self.remember(format!("removing {}", self.registry.def(id).name));
        self.loadout.remove_anywhere(id);
        Ok(())
    }

    /// Rotate `id` a quarter turn clockwise. A piece already in a slot only
    /// turns if it still fits afterwards — otherwise the rotation is undone,
    /// so a rejected rotation leaves the world untouched.
    pub fn rotate(&mut self, id: PieceId) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let before = self.registry.rotation(id);
        // Recorded before the turn is attempted and dropped again if it is
        // refused, so a rotation that could not happen leaves no history.
        self.remember(format!("turning {}", self.registry.def(id).name));
        self.registry.rotate_cw(id);

        if let Some(kind) = self.loadout.slot_holding(id) {
            let anchor = self
                .loadout
                .slot(kind)
                .anchor_of(id)
                .expect("a held piece has an anchor");
            // Re-place from scratch: clear the old footprint, then test.
            self.loadout.remove_anywhere(id);
            match self.loadout.can_place(&self.registry, id, kind, anchor.0, anchor.1) {
                Ok(()) => {
                    self.loadout.slot_mut(kind).place(&self.registry, id, anchor.0, anchor.1);
                }
                Err(e) => {
                    self.registry.set_rotation(id, before);
                    self.loadout.slot_mut(kind).place(&self.registry, id, anchor.0, anchor.1);
                    // Nothing changed, so there is nothing to take back.
                    self.undo_stack.pop();
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    /// A complete, legal loadout. Used by the "auto-build" button and by the
    /// tests, so the two can never drift apart.
    ///
    /// Deliberately shows off the mechanics rather than maxing the numbers:
    /// chest, gloves and greaves each carry **two** separate finished items,
    /// the weapon's Runed Edge doubles the Ruby Inlay next to it, and the
    /// Hollow Weave sits out in open space where its empty-cell bonus counts.
    /// Fields are `(name, slot, anchor x, anchor y, quarter turns)`.
    pub fn apply_preset(&mut self) {
        for kind in SlotKind::ALL {
            self.loadout.slot_mut(kind).clear();
        }
        const PRESET: &[(&str, SlotKind, u8, u8, u8)] = &[
            // Helmet — one item: frame + two plating (one is the bonus piece)
            // + crest.
            ("Steel Frame", SlotKind::Helmet, 0, 0, 0),
            ("Iron Plating", SlotKind::Helmet, 0, 2, 0),
            ("Visor of Focus", SlotKind::Helmet, 0, 4, 0),
            ("Crest of Vigor", SlotKind::Helmet, 3, 0, 0),
            // Chest — two items. The first fills the top-left; the second
            // hangs off the right-hand column with a gap between them, so the
            // Hollow Weave keeps five empty cells against its flank.
            ("Padded Base", SlotKind::Chest, 0, 0, 0),
            ("Chain Layer", SlotKind::Chest, 0, 3, 0),
            ("Woven Underlayer", SlotKind::Chest, 0, 4, 0),
            ("Hollow Weave", SlotKind::Chest, 5, 2, 1),
            ("Hide Base", SlotKind::Chest, 3, 6, 0),
            // Gloves — two items.
            ("Leather Material", SlotKind::Gloves, 0, 0, 0),
            ("Gripping Mold", SlotKind::Gloves, 2, 0, 0),
            ("Steel Material", SlotKind::Gloves, 0, 4, 0),
            ("Gauntlet Mold", SlotKind::Gloves, 2, 4, 0),
            // Greaves — two items.
            ("Runed Material", SlotKind::Greaves, 0, 0, 0),
            ("Greave Mold", SlotKind::Greaves, 2, 0, 0),
            ("Boiled Leather", SlotKind::Greaves, 0, 4, 0),
            ("Runner's Mold", SlotKind::Greaves, 3, 4, 0),
            // Weapon — one item, built around the Runed Edge so both
            // accessories sit against it.
            ("Balanced Grip", SlotKind::Weapon, 0, 0, 0),
            ("Runed Edge", SlotKind::Weapon, 1, 0, 0),
            ("Ruby Inlay", SlotKind::Weapon, 2, 0, 0),
            ("Balance Weight", SlotKind::Weapon, 2, 2, 0),
        ];
        // The preset names specific components, so grant any the player has
        // not bought. It is a demo button, not a way to dodge the shop.
        for &(name, ..) in PRESET {
            if self.find_by_name(name).is_none() {
                if let Some(d) = CATALOG.iter().position(|p| p.name == name) {
                    let id = self.registry.alloc(d);
                    self.owned.push(id);
                }
            }
        }
        for &(name, kind, ax, ay, rot) in PRESET {
            let Some(id) = self.find_by_name(name) else { continue };
            self.registry.set_rotation(id, rot);
            self.loadout.remove_anywhere(id);
            if self.loadout.can_place(&self.registry, id, kind, ax, ay).is_ok() {
                self.loadout.slot_mut(kind).place(&self.registry, id, ax, ay);
            }
        }
    }

    /// First owned component with this catalog name.
    pub fn find_by_name(&self, name: &str) -> Option<PieceId> {
        self.owned
            .iter()
            .copied()
            .find(|&id| self.registry.def(id).name == name)
    }

    /// Strip every slot and reset rotations.
    pub fn clear_all(&mut self) {
        self.remember("clearing every slot");
        for kind in SlotKind::ALL {
            self.loadout.slot_mut(kind).clear();
        }
        for &id in &self.owned {
            self.registry.set_rotation(id, 0);
        }
    }

    pub fn clear_slot(&mut self, kind: SlotKind) -> Result<(), RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        self.remember(format!("clearing the {}", kind.name().to_lowercase()));
        self.loadout.slot_mut(kind).clear();
        Ok(())
    }

    pub fn reports(&self) -> Vec<SlotReport> {
        self.loadout.reports(&self.registry)
    }

    pub fn report(&self, kind: SlotKind) -> SlotReport {
        self.loadout.report(&self.registry, kind)
    }

    /// Base character stats plus every slot's contribution.
    pub fn player_stats(&self) -> Stats {
        let mut base = self.raw_player_stats();
        base.health += self.grown_health;
        // Effective, not held: a doubled Standing has to actually be double
        // on the character sheet, not only inside the fight.
        for c in self.effective_classes() {
            if let crate::class::ClassPower::Standing(bonus) = c.power {
                base += bonus;
            }
        }
        base
    }

    fn raw_player_stats(&self) -> Stats {
        self.loadout.total_stats(&self.registry)
    }

    /// Activation profiles for every assembled item — what combat runs on.
    pub fn combat_items(&self) -> Vec<crate::loadout::ItemProfile> {
        self.loadout.combat_items(&self.registry)
    }

    /// Simulate the whole fight against `spec` and enter the replay phase.
    pub fn fight(&mut self, spec: &MonsterSpec) -> &CombatLog {
        let log = crate::combat::simulate_with_purse(
            self.player_stats(),
            &self.combat_items(),
            spec,
            self.difficulty,
            &self.effective_classes(),
            self.gold,
        );
        // What the fight spent out of the purse is gone whichever way it went.
        // Charged here rather than inside the simulation, which never touches
        // the run - a replayed fight must not charge you twice.
        self.gold = (self.gold - log.gold_spent).max(0);
        self.phase = Phase::Fighting;
        self.settled = false;
        self.log = Some(log);
        self.log.as_ref().expect("just set")
    }

    /// Fight whatever is next on the ladder.
    pub fn fight_next(&mut self) -> &CombatLog {
        let spec = *self.monster();
        self.fight(&spec)
    }

    /// Simulate against the original opponent, ladder position ignored.
    pub fn begin_fight(&mut self) -> &CombatLog {
        self.forget_undo();
        self.fight(&RUST_GOLEM)
    }

    /// Return to gear-arranging and discard the fight.
    pub fn back_to_loadout(&mut self) {
        self.phase = Phase::Loadout;
        self.log = None;
    }
}
