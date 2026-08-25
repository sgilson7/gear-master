use crate::combat::{
    CombatLog, Difficulty, Event, MonsterSpec, Outcome, Side, LADDER, RUST_GOLEM,
};
use crate::loadout::{Loadout, LockedItem, SlotReport};
use crate::piece::{all_def_indices, PieceId, PieceRegistry, QuestTrack, SlotKind, CATALOG};

/// The one weapon a run is handed for free. Everything else is bought — this
/// exists so the very first decision is *where to place* a weapon rather than
/// whether the shop happened to offer you one.
pub const STARTER_KIT: &[&str] = &["Oak Handle", "Iron Blade"];


use crate::slot::{PlaceError, SLOT_W};
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

/// What one town visit came to, for the screen to read back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TownVisit {
    pub at: Option<&'static str>,
    pub did: Option<crate::town::Action>,
    /// Gold the visit paid.
    pub paid: i32,
    /// The class walked out with, if any.
    pub gained_class: Option<&'static str>,
    /// How many of it is held now.
    pub stacks: usize,
    /// Set when five stacks converted into something else.
    pub became: Option<&'static str>,
    /// Shelves the visit put in the shop.
    pub stocked: usize,
}

impl TownVisit {
    /// The receipt: what one visit actually did, one line each.
    ///
    /// The struct already carries every number; this is those numbers said in
    /// the same voice as an event's receipt, so the panel does not need to know
    /// which sort of thing it is drawing.
    pub fn receipt(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let (Some(at), Some(did)) = (self.at, self.did) {
            out.push(format!("{}: {}", at, did.name()));
        }
        if self.paid != 0 {
            out.push(format!("+{}g", self.paid));
        }
        if let Some(became) = self.became {
            out.push(format!("Five became one: {}", became));
        } else if let Some(c) = self.gained_class {
            out.push(if self.stacks > 1 {
                format!("Class: {} x{}", c, self.stacks)
            } else {
                format!("Class: {}", c)
            });
        }
        if self.stocked > 0 {
            out.push(format!("The shelves are {} things you will not see again", self.stocked));
        }
        out
    }
}

/// Prayers it takes before the chapel gives you the other thing.
pub const PIETY_FOR_A_TICKET: usize = 5;

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
    /// A town this win has brought you to the gate of.
    pub town: Option<&'static str>,
    /// The component an event's fight handed over, on a win. Separate from
    /// `dropped`, which is a trophy off a named creature.
    pub won_item: Option<&'static str>,
    /// Rows added to every grid by that win. Nothing else in the game hands
    /// out room.
    pub rows_won: u8,
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

/// Everything that stands on a rung besides the fight.
///
/// The road has always had this order and it has always been a discipline
/// rather than a thing: `road_is_blocked` knew it, the interface knew it
/// again in its own words, and the two agreed because somebody kept them
/// agreeing. This is that order written down once.
///
/// **Derived, not stored.** The spec asks for `road_stack: Vec<Interrupt>` on
/// `Run`, pushed on arrival and popped on resolution. It is a function here
/// instead, and the reason is the same one that cost this project two
/// milestones already: a schedule kept in a second place is a schedule that
/// will one day disagree with the first. Every entry below is already decided
/// by run state - `dungeon`, `pending_town`, `at_fountain`, `answered`,
/// `brawl` - so a stored copy would have two sources of truth for one
/// question. Derived, "resolving an interrupt may push more" needs no code at
/// all: an event whose outcome sets `dungeon` simply appears with the dungeon
/// on top of it next time the stack is read, and a dungeon exit resumes the
/// pop where it left off because the rest of the stack never went anywhere.
#[derive(Copy, Clone, Debug)]
pub enum Interrupt {
    /// A mini dungeon being walked, and which floor. Innermost: you are
    /// standing inside it, so everything else is underneath you.
    Dungeon(&'static crate::dungeon::Dungeon, usize),
    /// A town's gate, standing between the rung just cleared and the next.
    TownGate(&'static crate::town::Town),
    /// A fountain owed at this rung. Carries the rung it stands on.
    Fountain(usize),
    /// An event standing in front of the fight.
    Event(&'static crate::event::LadderEvent),
    /// A fight an event arranged, waiting to be walked into.
    Brawl(&'static crate::event::Brawl),
}

impl Interrupt {
    /// What sort of thing this is, as a stable key. Never shown raw - the
    /// theme layer looks the word up.
    pub fn kind(self) -> &'static str {
        match self {
            Interrupt::Dungeon(..) => "dungeon",
            Interrupt::TownGate(_) => "town",
            Interrupt::Fountain(_) => "fountain",
            Interrupt::Event(_) => "event",
            Interrupt::Brawl(_) => "brawl",
        }
    }

    /// The id of the thing, where it has one. Empty for the two that are not
    /// table entries.
    pub fn id(self) -> &'static str {
        match self {
            Interrupt::Dungeon(d, _) => d.id,
            Interrupt::TownGate(t) => t.id,
            Interrupt::Event(e) => e.id,
            Interrupt::Fountain(_) | Interrupt::Brawl(_) => "",
        }
    }

    /// What it calls itself. Canonical: the theme layer swaps the noun.
    pub fn name(self) -> &'static str {
        match self {
            Interrupt::Dungeon(d, _) => d.name,
            Interrupt::TownGate(t) => t.name,
            Interrupt::Fountain(_) => "A FOUNTAIN",
            Interrupt::Event(e) => e.title,
            Interrupt::Brawl(_) => "A FIGHT ARRANGED",
        }
    }

    /// One line for a hover: what it is, and where you are in it.
    pub fn describe(self) -> String {
        match self {
            Interrupt::Dungeon(d, floor) => {
                format!("{} - floor {} of {}", d.name, floor + 1, d.floors.len())
            }
            Interrupt::TownGate(t) => format!("{} - a town, and one thing to do in it", t.name),
            Interrupt::Fountain(_) => {
                "A fountain, which reads your build and names you something".into()
            }
            Interrupt::Event(e) => format!("{} - a question, and it will wait", e.title),
            Interrupt::Brawl(b) => format!("{} - both of them at once", b.with.join(" and ")),
        }
    }

    /// Does this stop a replay from walking straight into the next fight?
    ///
    /// Everything except a dungeon. A dungeon *is* where the fighting happens
    /// while you are in one, so treating it as a blockage would stop the
    /// thing it stands for.
    pub fn blocks_a_rematch(self) -> bool {
        !matches!(self, Interrupt::Dungeon(..))
    }

    /// The word `road_is_blocked` has always answered with.
    pub fn blocking_name(self) -> &'static str {
        match self {
            Interrupt::TownGate(_) => "a town",
            Interrupt::Fountain(_) => "a fountain",
            _ => "something on the road",
        }
    }
}

impl PartialEq for Interrupt {
    /// By what it is and which one, so two reads of the same road compare
    /// equal without `LadderEvent` having to carry `PartialEq` down through
    /// its prose.
    fn eq(&self, other: &Self) -> bool {
        let floor = |i: &Interrupt| match i {
            Interrupt::Dungeon(_, f) => *f,
            Interrupt::Fountain(r) => *r,
            _ => 0,
        };
        self.kind() == other.kind() && self.id() == other.id() && floor(self) == floor(other)
    }
}

impl Eq for Interrupt {}

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
    /// A fight an event has arranged, waiting to be walked into. It stands
    /// beside the rung rather than on it: whichever way it goes, the rung's
    /// own creature is still there afterwards.
    pub brawl: Option<&'static crate::event::Brawl>,
    /// What the last thing you answered actually did, one line each.
    ///
    /// The receipt. Flavour prose stays in the event; this is the plain
    /// accounting underneath it, and it sits between a resolution and the next
    /// pop of the road stack. Engine-side, so the CLI prints the same lines the
    /// interface draws and the theme layer swaps the nouns in both.
    ///
    /// A seeded gamble reveals its **result** here and never its odds: the
    /// dispenser's receipt is where you learn "It wedged. Nothing."
    pub last_receipt: Option<Vec<String>>,
    /// Whether the mind lane's pool has been earned.
    ///
    /// False until THE THRESHOLD is cleared. While it is false nothing that
    /// banks Insight or stacks Dread reaches a shelf and the pool draws
    /// nothing, because there is never anything in it to draw. There is
    /// exactly one way to set it - see `unlock_insight` - and it is never
    /// unset: a run does not un-learn a thing.
    pub insight_unlocked: bool,
    /// Extra rows this run has been given, on top of the eight every grid
    /// starts with. Only ever goes up: what grants them cannot be sold, so
    /// there is no way to end up with pieces sitting in a row that is about
    /// to stop existing.
    pub extra_rows: u8,
    /// The quickest and slowest wins this run has managed in the shallow end,
    /// in milliseconds. The two earned doors of the early game read them - see
    /// `event::Trigger`.
    pub best_fight_ms: Option<u32>,
    pub worst_fight_ms: Option<u32>,
    /// Choices actually taken, by label, so a later event can ask what you did
    /// at an earlier one.
    pub took: Vec<&'static str>,
    /// Every point of each resource this run has ever banked, across every
    /// fight it has fought.
    ///
    /// Nothing else in the game asks a question about a whole playthrough - a
    /// fight is the unit everything is measured in - so this is counted at
    /// settle time and kept nowhere else. Indexed by `Resource::index`.
    /// Eight, not four: `Resource::index` runs to seven and a fused pool or
    /// an Insight gain arriving through `GainResource` would have indexed off
    /// the end. Nothing does today - a fusion has an event of its own - which
    /// is exactly why it was worth widening before something did.
    pub banked_all_run: [i32; 8],
    /// What the last fight paid. The factory doubles it, and nothing else has
    /// ever needed to look back at a bounty after banking it.
    pub last_bounty: i32,
    /// The town you are standing in, if any. Unlike everything else that
    /// interrupts the road, this *is* a rung: it is set on arriving and stays
    /// set until you go in or walk on, and the ladder does not move meanwhile.
    pub town: Option<&'static crate::town::Town>,
    /// Towns already answered, by id, so a Grinder knocked back through one
    /// does not get a second visit.
    pub towns_seen: Vec<&'static str>,
    /// Hidden towns something has put on the road, by id.
    ///
    /// A pinned town is on the map before the run starts; a hidden one is not
    /// on it until an event says so, and after that it is a town like any
    /// other. Kept as a list rather than a flag per town because which towns
    /// exist is a table and this is a fact about a run.
    pub towns_revealed: Vec<&'static str>,
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
            brawl: None,
            extra_rows: 0,
            best_fight_ms: None,
            worst_fight_ms: None,
            took: Vec::new(),
            banked_all_run: [0; 8],
            insight_unlocked: false,
            last_receipt: None,
            last_bounty: 0,
            town: None,
            towns_seen: Vec::new(),
            towns_revealed: Vec::new(),
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

    /// What is standing in the road before the next fight, if anything.
    ///
    /// Deliberately not phase-gated, unlike `pending_town` and `pending_event`.
    /// Those answer "should this screen be drawn", and the answer is no while a
    /// fight is being replayed. This answers "may a fight start", which has to
    /// be answerable *from* the battle screen - because that is where the bug
    /// was: REMATCH called `fight_next` straight from the replay, the rung had
    /// already moved on, and the run walked past its town, its events and its
    /// fountain without any of them being drawn. A board good enough to keep
    /// pressing it reached rung ten with no class at all.
    pub fn road_is_blocked(&self) -> Option<&'static str> {
        self.road_stack().into_iter().find(|i| i.blocks_a_rematch()).map(|i| i.blocking_name())
    }

    /// Everything standing on this rung, in the order it will be answered.
    ///
    /// The rung's own fight is not in it - the fight is the floor the stack
    /// stands on, and it begins when the stack is empty. That is the whole
    /// doctrine of `the_road.rs` said once in a data structure instead of in
    /// four places that have to be kept agreeing.
    ///
    /// The order is the order the game has always resolved in: the town gate
    /// first, then the fountain, then the events in table order, then a fight
    /// an event arranged. A dungeon sits on top of all of it, because being
    /// inside one is not something waiting for you - it is where you are.
    ///
    /// The spec asks for fountain before gate. It is amended: the two collide
    /// for real (`FOUNTAINS` is 7 and 14, Sump Bottom's gate stands at rung 7)
    /// and the shipped towns' tests read the gate first. Changing the order to
    /// match a document would have been changing the game to match a document.
    pub fn road_stack(&self) -> Vec<Interrupt> {
        let mut out = Vec::new();
        if let Some((d, floor)) = self.dungeon {
            out.push(Interrupt::Dungeon(d, floor));
        }
        // `self.town`, not `pending_town`: the phase gate on that one asks
        // "should this screen be drawn", which is no during a fight. This asks
        // what is standing on the rung, and a town does not stop standing
        // there because a replay is up. `road_is_blocked` has always had to be
        // answerable from the battle screen, and reading the gated question
        // here made `a_town_gate_blocks_the_road_even_mid_replay` pass on the
        // fountain that happens to share rung seven with Sump Bottom.
        if let Some(t) = self.town {
            out.push(Interrupt::TownGate(t));
        }
        if self.at_fountain() || self.at_doubling_fountain() {
            out.push(Interrupt::Fountain(self.rung));
        }
        // Read without the fountain gate `pending_event` applies, so the strip
        // can show what is standing underneath the fountain rather than
        // pretending the rung is otherwise empty.
        // Every event standing here, not just the one that would be asked
        // next. The strip's whole job is to say what is underneath, and an
        // event that is going to be asked the moment this one is answered is
        // exactly that.
        for e in self.standing_events() {
            out.push(Interrupt::Event(e));
        }
        if let Some(b) = self.brawl {
            out.push(Interrupt::Brawl(b));
        }
        out
    }

    /// The event standing on this rung, whatever else is also standing here.
    ///
    /// `pending_event` is this plus two gates - the loadout phase, and a
    /// fountain taking precedence - which are about whether it is *askable*
    /// now. This is about whether it is *there*.
    fn standing_event(&self) -> Option<&'static crate::event::LadderEvent> {
        self.standing_events().into_iter().next()
    }

    /// Every event standing on this rung, in the order they will be asked.
    ///
    /// Rumour doors first - having gone to the trouble of earning one you
    /// should get to see it - then whatever `event::at` finds, which is one at
    /// most because it takes the first match. So this is usually a list of one
    /// and occasionally a list of two, and the second is the reason it is a
    /// list at all.
    fn standing_events(&self) -> Vec<&'static crate::event::LadderEvent> {
        let mut out: Vec<&'static crate::event::LadderEvent> =
            self.whispered_event().into_iter().collect();
        if let Some(e) =
            crate::event::at(self.rung, self.best_fight_ms, self.worst_fight_ms, &self.answered)
                .filter(|e| !self.answered.contains(&e.id))
        {
            if !out.iter().any(|o| o.id == e.id) {
                out.push(e);
            }
        }
        out
    }

    /// The event standing in front of this rung, if there is one and it has
    /// not been answered.
    pub fn pending_event(&self) -> Option<&'static crate::event::LadderEvent> {
        if self.phase != Phase::Loadout || self.at_fountain() || self.at_doubling_fountain() {
            return None;
        }
        // A rumour door first: it stands on the same rung as whatever else is
        // there, and having gone to the trouble of earning it you should get
        // to see it. `standing_event` is that question; the two gates above
        // are the difference between "there" and "askable now".
        self.standing_event()
    }

    /// A rumour door standing on this rung: one you are carrying the word
    /// about, whose condition you have actually met.
    ///
    /// Separate from `event::at` because neither half can be answered from a
    /// rung and two stopwatches - one is about the board and one is about the
    /// whole run so far, and the run is the only thing that knows either.
    fn whispered_event(&self) -> Option<&'static crate::event::LadderEvent> {
        crate::event::EVENTS.iter().find(|e| {
            let crate::event::Trigger::Whispered { rumour } = e.trigger else { return false };
            e.at == self.rung
                && !self.answered.contains(&e.id)
                && self.owned.iter().any(|&i| self.registry.def(i).name == rumour)
                && crate::rumour::by_name(rumour).is_some_and(|r| self.meets(r.needs))
        })
    }

    /// Is a rumour's condition true right now?
    pub fn meets(&self, c: crate::rumour::Condition) -> bool {
        use crate::rumour::Condition;
        match c {
            Condition::Crowded { slot, under } => self.empty_cells(slot) < under,
            Condition::BankedAllRun { what, at_least } => {
                self.banked_all_run[what.index()] >= at_least
            }
        }
    }

    /// Cells in a slot with nothing on them.
    pub fn empty_cells(&self, slot: crate::piece::SlotKind) -> usize {
        let s = self.loadout.slot(slot);
        (0..s.rows())
            .flat_map(|y| (0..crate::slot::SLOT_W).map(move |x| (x, y)))
            .filter(|&(x, y)| s.get(x, y).is_none())
            .count()
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
        let Some(ev) = self.pending_event() else { return None };
        if !self.choice_open(c) {
            return None;
        }
        self.answered.push(ev.id);
        self.took.push(c.label);
        let (gave, receipt) = self.apply_outcome(&c.outcome, c.requires);
        self.last_receipt = Some(receipt);
        gave
    }

    /// Do what an outcome says, and say what it did.
    ///
    /// Split out of `take_choice` because an outcome is not only an event's.
    /// A town door hands one over too, and the two would otherwise be the same
    /// twelve arms written twice - which is the shape of every "and then
    /// somebody forgot to update the other one" bug in this file's history.
    ///
    /// `req` is the choice's requirement, and it is here for exactly one arm:
    /// `BuyOff` takes the component the requirement named. A door with no
    /// requirement passes `Requirement::None` and nothing is taken.
    ///
    /// Returns what was handed over, if anything, and the receipt.
    pub fn apply_outcome(
        &mut self,
        outcome: &crate::event::Outcome,
        req: crate::event::Requirement,
    ) -> (Option<&'static str>, Vec<String>) {
        use crate::event::Outcome as ChoiceOutcome;
        // The receipt starts as what the outcome *is* and gains what it *did*
        // as the arms below work out their numbers. A bounty depends on the
        // rung and a life depends on the mode, and neither is knowable from a
        // table.
        let mut receipt = outcome.describe();
        let mut gave = None;
        match *outcome {
            ChoiceOutcome::FightAsWritten => {}
            ChoiceOutcome::FightInstead(name) => {
                self.substitute = crate::combat::alternate(name);
            }
            ChoiceOutcome::Spare => {
                self.grant_life();
                if let Some(left) = self.lives_left() {
                    receipt.push(format!("Lives left: {}", left));
                }
            }
            ChoiceOutcome::Step(b) => {
                self.brawl = Some(b);
            }
            ChoiceOutcome::Stock { shelves, class } => {
                self.shop.stock_exactly(shelves);
                self.claim_class(class);
            }
            ChoiceOutcome::Give(name) => {
                if let Some(d) = crate::piece::CATALOG.iter().position(|d| d.name == name) {
                    let id = self.registry.alloc(d);
                    self.owned.push(id);
                    receipt.push("It arrives loose, and takes up room".into());
                } else {
                    receipt.clear();
                    receipt.push(format!("Nothing: {} is not a component", name));
                }
            }
            ChoiceOutcome::Claim(name) => {
                self.claim_class(name);
            }
            ChoiceOutcome::Enter(id) => {
                self.dungeon = crate::dungeon::by_id(id).map(|d| (d, 0));
            }
            ChoiceOutcome::BuyOff { times } => {
                if let Some(&id) = self.offerings(req).first() {
                    gave = Some(self.registry.def(id).name);
                    self.owned.retain(|&o| o != id);
                    receipt[0] = format!("Handed over: {}", self.registry.def(id).name);
                }
                let paid = LADDER[self.rung.min(LADDER.len() - 1)].bounty * times;
                if receipt.len() > 1 {
                    receipt[1] = format!("+{}g, and the rung is behind you", paid);
                }
                self.gold += paid;
                // Paid off rather than beaten: the rung is behind you, but it
                // was never fought, so it is not a win.
                self.rung += 1;
                self.best_rung = self.best_rung.max(self.rung);
                let need = self.needs_a_weapon();
                self.shop.restock(&mut self.rng, need);
            }
        }
        (gave, receipt)
    }

    /// Take a class handed over rather than poured, if it is not already held.
    fn claim_class(&mut self, name: &str) {
        let Some(k) = crate::class::CLASSES.iter().find(|k| k.name == name) else { return };
        if self.classes.iter().any(|held| held.name == k.name) {
            return;
        }
        self.classes.push(k);
        self.refresh_class_effects();
    }

    /// Take the receipt, so the road can move on.
    ///
    /// Read once. The panel that shows it dismisses it, and the next pop of
    /// the stack happens after that - which is the whole of A9's ordering.
    pub fn take_receipt(&mut self) -> Option<Vec<String>> {
        self.last_receipt.take()
    }

    /// Open the mind lane. Once, and never closed again.
    ///
    /// The shelf is told at the same moment, because a flag on the run that
    /// the shop has to be reminded of separately is a flag that will one day
    /// be set without the reminder.
    pub fn unlock_insight(&mut self) {
        self.insight_unlocked = true;
        self.shop.insight_open = true;
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

    // ------------------------------------------------------------ bartering

    /// Loose components that would pay for the rumour on `slot`.
    ///
    /// Loose only, like every other trade in the game: what you are wearing is
    /// not on the table. Handing something over has to cost you something you
    /// could have used.
    pub fn payment_for(&self, slot: usize) -> Vec<PieceId> {
        if self.trophy_shelf(slot) {
            // Any trophy. There is no scale between them - what the bar is
            // buying is that you went and took one off something.
            return self
                .inventory()
                .into_iter()
                .filter(|&id| crate::piece::is_boss_only(self.registry.def(id).name))
                .collect();
        }
        let Some(r) = self.rumour_on(slot) else { return Vec::new() };
        self.inventory()
            .into_iter()
            .filter(|&id| {
                let d = self.registry.def(id);
                match r.price {
                    crate::rumour::Barter::Kind(k) => d.kind == k,
                    crate::rumour::Barter::Rumour(n) => d.name == n,
                }
            })
            .collect()
    }

    /// The rumour on a shelf, if that shelf holds one.
    pub fn rumour_on(&self, slot: usize) -> Option<&'static crate::rumour::Rumour> {
        let def = self.shop.def(slot)?;
        crate::rumour::by_name(def.name)
    }

    /// Is this shelf the bar's standing offer on boss trophies?
    ///
    /// The counter pays nothing for one, so this is the only thing in the game
    /// that will take one at all.
    pub fn trophy_shelf(&self, slot: usize) -> bool {
        self.shop.def(slot).is_some_and(|d| d.name == crate::rumour::TROPHY_SHELF)
    }

    /// Buy a rumour by handing something over.
    ///
    /// A separate door from `buy` on purpose: the pub does not take money, and
    /// a shelf that quietly accepted either would make the one thing the pub
    /// is for - what you are carrying being worth more than what you have
    /// banked - into a footnote.
    pub fn barter(&mut self, slot: usize, paying: PieceId) -> Result<PieceId, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        if self.rumour_on(slot).is_none() && !self.trophy_shelf(slot) {
            return Err(RuleError::NothingThere);
        }
        if !self.payment_for(slot).contains(&paying) {
            return Err(RuleError::NothingThere);
        }
        // The trophy trade hands over a class, not a component. The shelf
        // restocks, because a run that took two bosses may spend two.
        if self.trophy_shelf(slot) {
            self.remember("trading a trophy");
            self.owned.retain(|&i| i != paying);
            self.loadout.remove_anywhere(paying);
            self.gain_class("Recycler");
            self.refresh_class_effects();
            return Ok(paying);
        }
        self.remember("bartering");
        let def = self.shop.take(slot).ok_or(RuleError::NothingThere)?;
        // Handed over, not sold: no gold changes hands in either direction.
        self.owned.retain(|&i| i != paying);
        self.loadout.remove_anywhere(paying);
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

        // Everything the fight banked, added to the run's running total. Read
        // from the events rather than from the end state: a pool that was
        // banked and then spent still happened, and the only question anything
        // asks of this is how much has passed through your hands.
        if let Some(l) = self.log.as_ref() {
            for e in &l.entries {
                match &e.event {
                    Event::GainResource { side: Side::Player, what, amount, .. } => {
                        if let Some(r) = crate::piece::Resource::by_name(what) {
                            self.banked_all_run[r.index()] += amount;
                        }
                    }
                    // Mana has an event of its own - most of the mana in the
                    // game arrives through it rather than through a named
                    // resource gain, so leaving it out would make the mana
                    // total permanently zero.
                    Event::GainMana { side: Side::Player, amount, .. } => {
                        self.banked_all_run[crate::piece::Resource::Mana.index()] += amount;
                    }
                    _ => {}
                }
            }
        }

        // A fight an event arranged is settled on its own terms and never
        // touches the ladder: it is a detour, so whatever the rung was going
        // to hand you is still waiting when it is over - including its bounty,
        // which is why this one pays nothing.
        if let Some(b) = self.brawl.take() {
            let mut settlement = Settlement {
                outcome,
                reward: 0,
                knocked_back: false,
                quests_done: self.award_quests(),
                lives_left: None,
                run_ended: false,
                dropped: None,
                landing: None,
                class_won: None,
                town: None,
                won_item: None,
                rows_won: 0,
            };
            if outcome == Outcome::Victory {
                self.wins += 1;
                if let Some(d) = crate::piece::CATALOG.iter().position(|d| d.name == b.win) {
                    let id = self.registry.alloc(d);
                    self.owned.push(id);
                    settlement.won_item = Some(b.win);
                }
                if b.and_grow > 0 {
                    self.grow_boards(b.and_grow);
                    settlement.rows_won = b.and_grow;
                }
            } else if !b.forgiving {
                self.losses += 1;
            }
            self.phase = Phase::Loadout;
            self.log = None;
            self.last_settlement = Some(settlement);
            let need = self.needs_a_weapon();
            self.shop.restock(&mut self.rng, need);
            return Some(0);
        }

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
            town: None,
            won_item: None,
            rows_won: 0,
        };

        // How fast, and how slow, the shallow end went. The two doors of the
        // early game are decided by these.
        //
        // Only a real win counts: a stalemate lasts the full clock by
        // definition, so counting one would hand out the slow door for free,
        // and a defeat that ended in half a second is not a fight you won
        // quickly. Only the shallow end counts either - a fight further up is
        // not evidence about the early game.
        if outcome == Outcome::Victory && crate::event::SHALLOW.contains(&self.rung) {
            if let Some(ms) = self.log.as_ref().map(|l| l.duration_ms) {
                self.best_fight_ms = Some(self.best_fight_ms.map_or(ms, |b| b.min(ms)));
                self.worst_fight_ms = Some(self.worst_fight_ms.map_or(ms, |w| w.max(ms)));
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
                            self.refresh_class_effects();
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
                // And there may be somewhere between here and the next one.
                // Once only: a Grinder knocked back through a town does not
                // get to work the same shift twice.
                if let Some(t) = self.town_between(self.rung) {
                    if !self.towns_seen.contains(&t.id) {
                        self.town = Some(t);
                        self.last_bounty = bounty;
                        settlement.town = Some(t.name);
                    }
                }
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
            if x >= SLOT_W as u32 || y >= self.loadout.rows() as u32 {
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
    /// How many classes a fountain has actually given you.
    ///
    /// Not `classes.len()`: a dungeon reward and the bargain in the back room
    /// are classes too, and counting them advanced the fountain schedule past
    /// a fountain the player had not been to. A run that cleared the crevice
    /// before rung fourteen simply never saw the second one, and nothing said
    /// why - the same shape of bug as the third fountain not appearing.
    fn poured(&self) -> usize {
        self.classes.iter().filter(|c| !crate::class::is_earned(c.name)).count()
    }

    pub fn at_fountain(&self) -> bool {
        Self::FOUNTAINS.get(self.poured()) == Some(&self.rung)
    }

    /// The rung the next fountain stands on, if there is one left.
    pub fn next_fountain(&self) -> Option<usize> {
        Self::FOUNTAINS.get(self.poured()).copied()
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
        self.refresh_class_effects();
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
        self.refresh_class_effects();
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
    /// Give every grid another row, for good.
    ///
    /// Thirty more cells across the five boards, which is the largest thing
    /// any one reward hands out - and it hands out *room*, which is worth
    /// whatever the player is clever enough to put in it.
    pub fn grow_boards(&mut self, by: u8) {
        self.extra_rows += by;
        self.loadout.grow(by);
    }

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
    ///
    /// And every grid stands on a bonded enchantment, because this is the
    /// button somebody presses to find out what the game is, and a demo that
    /// leaves out the newest layer is a demo of the game before it.
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
            // And one enchantment, bonded: every cell of it covered by one
            // finished item, which doubles that item and hands it a trigger.
            // Last in the list because it goes in the layer underneath and the
            // gear above has to be seated first for the bond to mean anything.
            //
            // One rather than five, and the chest rather than the weapon. This
            // is the demo button and it should show the newest layer, but it is
            // also the deliberately blunt reference build - `two_runs` walks it
            // up the ladder to prove the *other* door opens for a build that
            // cannot earn the casino. Five bonded items took its median kill
            // from nine seconds to four and a half and shut that door. The body
            // is the one grid where doubling an item makes the build tougher
            // rather than faster, so it is the one that can carry the
            // demonstration without changing what the build is for.
            ("Keystone Base", SlotKind::Chest, 0, 0, 0),
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

    /// The creatures an event has put in front of you, if any.
    pub fn pending_brawl(&self) -> Option<Vec<crate::combat::MonsterSpec>> {
        let b = self.brawl?;
        let specs: Vec<_> =
            b.with.iter().filter_map(|n| crate::combat::creature(n)).copied().collect();
        (specs.len() == b.with.len()).then_some(specs)
    }

    /// Fight several things at once, on the rung you are standing on.
    ///
    /// The rung does not move and the bounty is the rung's, not the sum: a
    /// brawl is an event putting two creatures in front of you, not two rungs
    /// collapsed into one.
    pub fn fight_party(&mut self, specs: &[crate::combat::MonsterSpec]) -> &CombatLog {
        let log = crate::combat::simulate_party(
            self.player_stats(),
            &self.combat_items(),
            specs,
            self.difficulty,
            &self.effective_classes(),
            self.gold,
        );
        self.gold = (self.gold - log.gold_spent).max(0);
        self.phase = Phase::Fighting;
        self.settled = false;
        self.log = Some(log);
        self.log.as_ref().expect("just set")
    }

    /// Simulate against the original opponent, ladder position ignored.
    pub fn begin_fight(&mut self) -> &CombatLog {
        self.forget_undo();
        self.fight(&RUST_GOLEM)
    }

    // -------------------------------------------------------------- towns

    /// The town standing in this gap, if there is one this run can see.
    ///
    /// A pinned town is always there. A hidden one is there only once
    /// something has put it there, which is the whole of what "hidden" means -
    /// after that it is a town like any other, at its own rung, with its own
    /// doors, subject to the same one-visit rule.
    pub fn town_between(&self, rung: usize) -> Option<&'static crate::town::Town> {
        crate::town::between(rung).filter(|t| match t.unlock {
            crate::town::Unlock::Pinned => true,
            crate::town::Unlock::Hidden => self.towns_revealed.contains(&t.id),
        })
    }

    /// Put a hidden town on the road. Idempotent, and never undone.
    pub fn reveal_town(&mut self, id: &'static str) -> bool {
        if crate::town::by_id(id).is_none() || self.towns_revealed.contains(&id) {
            return false;
        }
        self.towns_revealed.push(id);
        true
    }

    /// The town you are standing at the gate of, if any.
    pub fn pending_town(&self) -> Option<&'static crate::town::Town> {
        self.town.filter(|_| self.phase == Phase::Loadout)
    }

    /// Walk on. The bounty is paid a second time and the town is done with.
    ///
    /// A real offer, not a courtesy: a build one component short of an item
    /// wants gold more than it wants a class, and the town should lose that
    /// argument sometimes.
    pub fn skip_town(&mut self) -> i32 {
        let Some(t) = self.town.take() else { return 0 };
        self.towns_seen.push(t.id);
        let paid = self.last_bounty;
        self.gold += paid;
        self.last_receipt =
            Some(vec![format!("Walked past {}", t.name), format!("+{}g, the bounty again", paid)]);
        paid
    }

    /// Go in, and do the one thing you have time for.
    ///
    /// One action a visit. Four doors and one key makes a town a decision
    /// rather than a shopping trip.
    pub fn visit_town(&mut self, what: crate::town::Action) -> TownVisit {
        use crate::town::Action;
        let Some(t) = self.town.take() else { return TownVisit::default() };
        self.towns_seen.push(t.id);
        let mut out = TownVisit { at: Some(t.name), did: Some(what), ..TownVisit::default() };
        match what {
            Action::Chapel => {
                self.gain_class("Piety");
                // Five of them are taken away and handed back as one thing
                // that is worth more than five of anything.
                if self.stacks_of("Piety") >= PIETY_FOR_A_TICKET {
                    self.classes.retain(|c| c.name != "Piety");
                    self.gain_class("Ticket to Ride");
                    out.became = Some("Ticket to Ride");
                }
                out.gained_class = Some(out.became.unwrap_or("Piety"));
                out.stacks = self.stacks_of(out.gained_class.unwrap_or(""));
            }
            Action::Factory => {
                let paid = self.last_bounty * 2;
                self.gold += paid;
                out.paid = paid;
                self.gain_class("Tired");
                out.gained_class = Some("Tired");
                out.stacks = self.stacks_of("Tired");
            }
            Action::Shop => {
                self.shop.stock_exactly(crate::piece::town_shelf());
                out.stocked = crate::piece::town_shelf().len();
            }
            Action::Pub => {
                self.shop.stock_exactly(crate::rumour::on_offer());
                out.stocked = crate::rumour::on_offer().len();
            }
        }
        self.last_receipt = Some(out.receipt());
        out
    }

    /// Push the class rules that live on the board back onto the board.
    ///
    /// Recycler scales adjacency bonuses, and `Loadout::report` is the single
    /// place that maths happens - so the loadout has to be told. Every path
    /// that changes `self.classes` calls this; `a_class_gained_any_way_reaches_the_board`
    /// is the test that says so.
    pub fn refresh_class_effects(&mut self) {
        let pct = self
            .effective_classes()
            .iter()
            .filter_map(|c| match c.power {
                crate::class::ClassPower::Recycler { pct } => Some(pct),
                _ => None,
            })
            .sum();
        self.loadout.adjacency_pct = pct;
    }

    /// Add one to a stacking class, or the class itself if it does not stack.
    fn gain_class(&mut self, name: &'static str) {
        let Some(c) = crate::class::CLASSES.iter().find(|c| c.name == name) else { return };
        if !crate::class::stacks(name) && self.classes.iter().any(|k| k.name == name) {
            return;
        }
        self.classes.push(c);
        self.refresh_class_effects();
    }

    /// How many of a class is held. One for anything that does not stack.
    pub fn stacks_of(&self, name: &str) -> usize {
        self.classes.iter().filter(|c| c.name == name).count()
    }

    /// Return to gear-arranging and discard the fight.
    pub fn back_to_loadout(&mut self) {
        self.phase = Phase::Loadout;
        self.log = None;
    }
}
