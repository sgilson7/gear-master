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

/// Where a trip into THE HUNDRED came from.
///
/// **The census.** Ten trips exist in a run that does everything, and the cap
/// is this enum rather than a number written down beside it: `Town` is worth
/// `TOWNS.len()` and the other four are worth one each, so a mission that adds
/// a way down cannot land without the suite making it raise the cap.
/// `county::trip_cap` is the arithmetic and
/// `the_census_is_the_enum_and_not_a_number` is the test.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TripSource {
    /// One per town, once each. Three pinned and three found.
    Town(&'static str),
    /// B1.2 - the Ordnance's ticket, spent at a pedestal, and the only way in
    /// that offers a choice of mouth.
    SurveyorsOrb,
    /// C2 - the bet on an empty grid, paid at the deadline.
    WasteBet,
    /// C1 - unasked, and the fastest ride into the middle there is.
    Constable,
    /// B5 - all three chains done. Granted rather than taken.
    Perambulation,
}

impl TripSource {
    /// Every variant, so a count of them is a count of the enum.
    pub const ALL: [TripSource; 5] = [
        TripSource::Town(""),
        TripSource::SurveyorsOrb,
        TripSource::WasteBet,
        TripSource::Constable,
        TripSource::Perambulation,
    ];

    /// How many trips this variant is worth to the cap. A town is worth as
    /// many as there are towns; everything else is worth one.
    pub fn seats(self) -> usize {
        match self {
            TripSource::Town(_) => crate::town::TOWNS.len(),
            _ => 1,
        }
    }

    /// A stable key. Never shown raw.
    pub fn key(self) -> &'static str {
        match self {
            TripSource::Town(id) => id,
            TripSource::SurveyorsOrb => "surveyors-orb",
            TripSource::WasteBet => "waste-bet",
            TripSource::Constable => "constable",
            TripSource::Perambulation => "perambulation",
        }
    }
}

/// The flag THE CONSTABLE reads: a trip that came back with nothing.
pub const COUNTY_BUSINESS: &str = "county-business";

/// How many moves are left, said the way a person would say it.
fn moves_left(n: u8) -> String {
    match n {
        0 => "no moves left".into(),
        1 => "1 move".into(),
        n => format!("{n} moves"),
    }
}

/// Ten: three pinned towns, three hidden, an orb, a bet, an arrest and a
/// perambulation. Counted off `TripSource`, so it cannot drift from it.
pub fn trip_cap() -> usize {
    TripSource::ALL.iter().map(|t| t.seats()).sum()
}

/// What losing costs you. Either way a loss still pays the bounty - you need
/// income to buy your way past whatever just beat you - and never advances the
/// ladder, because you did not actually kill the thing.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Mode {
    /// Losing knocks you back down to the rung you last cleared, so there is
    /// always an easier fight to farm before trying again.
    Grinder,
    /// Losing costs a life. Run out of them and the run is over.
    Rogue,
}

impl Mode {
    /// The line under the heading on the screen where a mode is picked.
    ///
    /// It lives here rather than in the interface for two reasons. The screen
    /// is not the only thing that has to be able to say this - the CLI picks a
    /// mode too - and a line of prose kept in a crate the prose lint cannot
    /// reach is a line of prose nothing checks. The one that was here said
    /// "Losing pays either way. It just does not get you past the thing that
    /// beat you", which is a verdict on both cards rather than a statement of
    /// what the screen is asking.
    ///
    /// What the screen is asking is which of one thing you want, and both
    /// cards below already say what their own answer costs.
    pub const WHAT_THE_CHOICE_IS: &'static str =
        "The two differ in one thing: what a loss takes off you.";

    pub fn name(self) -> &'static str {
        match self {
            Mode::Grinder => "GRINDER",
            Mode::Rogue => "ROGUE",
        }
    }

    /// One paragraph under the name on the mode card.
    ///
    /// A `String` rather than a literal because the Rogue one counts the
    /// lives, and the number is `ROGUE_LIVES`. It was written out as a word
    /// in three places across two crates and none of them was reading the
    /// constant, so raising it left the game telling the player a number that
    /// was no longer true in every one of them.
    pub fn blurb(self) -> String {
        match self {
            Mode::Grinder => "Lose and you slide back a rung. You still get paid, so grind \
                 the easy one until you can take the hard one."
                .into(),
            Mode::Rogue => format!(
                "{} losses and it is over. Everything you own goes with it. \
                 You still get paid, so a loss buys you one more shot.",
                capitalised(lives_in_words())
            ),
        }
    }
}

/// How many losses a Rogue run survives.
///
/// Four, at the owner's asking. Balancing around **five** is the eventual
/// intent and this is the number to raise when that work happens - nothing
/// else has to move with it, because everything that quotes the count now
/// reads it from here.
pub const ROGUE_LIVES: u32 = 4;

/// The life count as a word, for the prose that quotes it.
///
/// "4 losses and it is over" is not a sentence anybody writes, so the three
/// lines that say the number say it in words - and they all say it from here
/// rather than each spelling it out and drifting apart, which is what happened
/// the first time.
pub fn lives_in_words() -> &'static str {
    match ROGUE_LIVES {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        // Past six it stops being a number a player counts on their fingers,
        // and a mode card that says "seven lives" has stopped being a card
        // about scarcity anyway.
        _ => "several",
    }
}

/// The same word with its first letter up, for the head of a sentence.
fn capitalised(word: &'static str) -> String {
    let mut cs = word.chars();
    match cs.next() {
        Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
        None => String::new(),
    }
}

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

/// What Aisle 9 has out.
///
/// The only place on any plane that reliably stocks an Orb of Travel, and the
/// two relics that are shelved with the orbs because nobody working here could
/// think of a better aisle for them.
pub const AISLE_NINE: &[&str] =
    &["Wayfarer's Orb", "Pilgrim's Orb", "Ferry Orb", "Stray Orb", "The Odometer", "The Ledger"];

/// What the Slagworks' mold line lays out. Ground, and things that go under
/// gear - and the rod, always, because that is what the line is known for.
pub const MOLD_LINE: &[&str] =
    &["the Lightning Rod", "Keystone Base", "Woven Underlayer", "Bulwark Plating", "Scaled Plating"];

/// What the tempering adds to a piece.
pub const TEMPERING_GAIN: i32 = 10;
/// What the library's book adds, and what it costs is a curse for good.
pub const LIBRARY_GAIN: i32 = 25;
/// What the long table is worth, for the rest of the run.
pub const LONG_TABLE_HEALTH: i32 = 100;

/// How much slower everything runs while a contract is being honoured.
///
/// Fifty, which is what a stack of frost does. The point of the arrangement is
/// that the handicap is real and that you chose it - a difficulty setting
/// priced by the player, local and transactional, rather than a dial in a
/// menu.
pub const CONTRACT_SLOWER: i64 = 50;

/// How much slower an item carrying a permanently cursed piece runs.
///
/// Half a contract, and for the right reason: a contract is every slot for
/// three rungs and you asked for it, while this is one item for the rest of
/// the run and somebody did it to you. A price you can see is a price you can
/// decide about - the Manse library charges it, the thirsty wizard charges it
/// for being refused, and the mole with the tools is the only thing that lifts
/// it.
pub const CURSED_SLOWER: i64 = 25;

/// How much better a piece comes back from consignment.
pub const CONSIGNMENT_GAIN: i32 = 30;
/// How many shops it is away for.
pub const CONSIGNMENT_SHOPS: u32 = 3;

/// How far a melt may move a piece's rating, either way.
///
/// Fifteen: enough that the crucible is a gamble worth taking with something
/// mediocre and not enough that it is a way of turning a common into a
/// legendary. The bands are ninety, a hundred and thirty and a hundred and
/// seventy apart, so one melt never crosses two of them.
pub const MELT_SPREAD: i32 = 15;

/// The relic that opens the road past Francis.
///
/// Named here rather than in the chain because the plumbing has to know it
/// before the content does: share codes, scoring and the road all have to
/// accept a rung fifty-one before there is anything standing on one.
pub const MAINSPRING: &str = "An Unwound Mainspring";

/// How many rungs an underwritten loss stays good for.
///
/// Five, and the number is the whole of the arrangement: long enough to be
/// worth taking, short enough that it is a decision about the next stretch of
/// road rather than a life in your pocket for the rest of the run.
pub const UNDERWRITTEN_FOR: usize = 5;

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
    /// The fight an underwritten loss was spent on, if one was.
    pub underwrote: Option<&'static str>,
    /// Whether a passenger was riding, and is not any more.
    pub lost_passenger: bool,
    /// Gear pried off a named creature by a Prospector, beyond its trophy.
    pub pried_off: Vec<&'static str>,
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
    /// Tried to wear a quest item. They are carried and never worn.
    NotWearable,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::Place(e) => write!(f, "{}", e),
            RuleError::LoadoutLocked => write!(f, "can't change gear during a fight"),
            RuleError::NotEquipped => write!(f, "that piece isn't equipped"),
            RuleError::NotWearable => {
                write!(f, "that is a quest item - it is carried, not worn")
            }
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
    /// THE HUNDRED, being walked. Outermost of the outermost: a dungeon hangs
    /// off a rung and the county hangs off a town, and you cannot be in both.
    County { at: (u8, u8), moves_left: u8 },
    /// A mini dungeon being walked. Innermost: you are standing inside it, so
    /// everything else is underneath you.
    ///
    /// `nth` and `of` are the banner's two numbers, worked out by
    /// `road_stack` because only the run knows them - which fights this entry
    /// has already won, and which floors it walked past because it had beaten
    /// them before. They are a reading of the run at the moment the stack was
    /// built, and nothing holds an `Interrupt` across a transition; the stack
    /// is derived fresh every time it is asked for.
    Dungeon {
        at: &'static crate::dungeon::Dungeon,
        floor: usize,
        /// Which fight of this entry this is, counting from one.
        nth: usize,
        /// How many fights this entry is, as things stand.
        of: usize,
    },
    /// Standing at a set of points: the floor just cleared has more than one
    /// way on and nobody has said which. Above the dungeon, because the lever
    /// is in the dungeon and you are standing at the lever.
    Points(&'static crate::dungeon::Dungeon, usize),
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
            Interrupt::County { .. } => "county",
            Interrupt::Dungeon { .. } => "dungeon",
            Interrupt::Points(..) => "points",
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
            Interrupt::Dungeon { at, .. } => at.id,
            Interrupt::Points(d, _) => d.id,
            Interrupt::TownGate(t) => t.id,
            Interrupt::Event(e) => e.id,
            Interrupt::County { .. } => "the-hundred",
            Interrupt::Fountain(_) | Interrupt::Brawl(_) => "",
        }
    }

    /// What it calls itself. Canonical: the theme layer swaps the noun.
    pub fn name(self) -> &'static str {
        match self {
            Interrupt::Dungeon { at, .. } => at.name,
            // Not the dungeon's name. The stack shows a row per interrupt and
            // the points sit directly on top of the dungeon they are in, so
            // naming both after the building printed it twice and read as two
            // buildings. The same shape `Fountain` and `Brawl` already have:
            // an interrupt that is a *moment* says what the moment is.
            Interrupt::Points(..) => "THE POINTS",
            Interrupt::County { .. } => "THE HUNDRED",
            Interrupt::TownGate(t) => t.name,
            Interrupt::Fountain(_) => "A FOUNTAIN",
            Interrupt::Event(e) => e.title,
            Interrupt::Brawl(_) => "A FIGHT ARRANGED",
        }
    }

    /// One line for a hover: what it is, and where you are in it.
    pub fn describe(self) -> String {
        match self {
            // Three parts, and the middle one is new: the creature. `floor
            // {n} of {m}` used to be an index and a room count, and a graph
            // makes both of them lies - a floor's index says nothing about how
            // deep it is, and nine rooms with points in them are four fights.
            // So `n` is which fight of this entry this is and `m` is how many
            // this entry turns out to be, and for a straight line walked from
            // the top they are exactly the old two numbers.
            Interrupt::Dungeon { at, floor, nth, of } => format!(
                "{} - {} - floor {} of {}",
                at.name,
                at.floors.get(floor).map(|f| f.creature).unwrap_or(""),
                nth,
                of
            ),
            Interrupt::Points(d, floor) => format!(
                "{} - the points after {}: {}",
                d.name,
                d.floors.get(floor).map(|f| f.creature).unwrap_or(""),
                d.floors
                    .get(floor)
                    .map(|f| f.exits.iter().map(|e| e.label).collect::<Vec<_>>().join(" / "))
                    .unwrap_or_default()
            ),
            // The banner A2.1 asks for. The tile says what it is in canonical
            // words; the theme layer swaps them, the way it swaps a door's.
            Interrupt::County { at, moves_left } => format!(
                "THE HUNDRED - {} - {} move{} left",
                crate::county::reference(at),
                moves_left,
                if moves_left == 1 { "" } else { "s" }
            ),
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
    /// thing it stands for. The points are the exception inside the
    /// exception: standing at a lever is not standing in front of a fight,
    /// and which fight it will be is the thing that has not been decided.
    pub fn blocks_a_rematch(self) -> bool {
        !matches!(self, Interrupt::Dungeon { .. })
    }

    /// The word `road_is_blocked` has always answered with.
    pub fn blocking_name(self) -> &'static str {
        match self {
            Interrupt::TownGate(_) => "a town",
            Interrupt::Fountain(_) => "a fountain",
            Interrupt::Points(..) => "the points",
            Interrupt::County { .. } => "the county",
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
            Interrupt::Dungeon { floor, .. } => *floor,
            Interrupt::Points(_, f) => *f,
            Interrupt::Fountain(r) => *r,
            // Where you are standing, so two reads of one county at two tiles
            // do not compare equal. `moves_left` is deliberately not in it:
            // a trip with four left and one with three are the same place.
            Interrupt::County { at, .. } => at.0 as usize * 7 + at.1 as usize,
            _ => 0,
        };
        self.kind() == other.kind() && self.id() == other.id() && floor(self) == floor(other)
    }
}

impl Eq for Interrupt {}

/// Cloneable so a harness can keep a **pool of situations**.
///
/// A run stood at a rung by walking there is expensive - a whole control run of
/// shops and fights - and a trainer wants thousands of them. Walking a pool
/// once and cloning out of it is the difference between a curriculum and a
/// wait. Nothing in the game clones a run; this is for `crates/lab`.
#[derive(Clone)]
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
    /// Which rung each of those was answered on.
    ///
    /// A scheduled door is answered on its own rung and the map could always
    /// work that out. An **earned** one roams a window - THE CASINO's is rungs
    /// two to nine - so `LadderEvent::at` is its deadline and not its address,
    /// and a map drawing it at `at` puts a door you answered on rung three up
    /// at rung nine. Nothing recorded where it actually happened, so nothing
    /// could draw it there.
    ///
    /// A `Vec` of pairs rather than a `HashMap`: thirty-three entries at the
    /// very most, looked up by key, and iteration order that cannot surprise
    /// anybody reading a replay.
    pub answered_on: Vec<(&'static str, usize)>,
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
    /// What this run has done, by name.
    ///
    /// The chain's stations set these and later stations read them. Strings
    /// rather than a field each, because that is what buys the reverse index -
    /// `event::set_by` finds which door sets a flag, so "a station nothing
    /// reaches" is one assertion rather than a thing somebody has to notice.
    pub flags: Vec<&'static str>,
    /// Things counted without anybody being told they were being counted.
    ///
    /// The watcher pattern. Arming leaves a receipt line that explains
    /// nothing; the door that reads the tally is thirty rungs later.
    pub counters: Vec<(&'static str, u32)>,
    /// How many times the man at the top has been put down this run.
    ///
    /// The road does not end at Francis: `monster` clamps to the last rung,
    /// so every rung past fifty is him again. This is what makes that mean
    /// something - rung `50 + n` is `2^n` Francis. See `MonsterSpec::doubled`
    /// for what actually doubles and why it is not the resistances.
    pub francis_beaten: u32,
    /// The last figure the player named, for a door that asked for one.
    pub last_figure: Option<i32>,
    /// Rows granted but not yet spent. "One board of your choice" is a
    /// decision, and an outcome cannot make it for you.
    pub owed_rows: u8,
    /// Claims on a named creature's whole board, unspent.
    pub claim_tickets: u32,
    /// Arrangements with the shop that outlive a restock.
    pub standing_orders: Vec<crate::event::Standing>,
    /// The last rung an underwritten loss will still be forgiven on.
    ///
    /// Set five rungs ahead when it is taken, and taken away by the loss it
    /// eats. One fight, once - the receipt says which fight it was.
    pub underwritten_until: Option<usize>,
    /// Whether a boss's packed board can be read before it is fought.
    ///
    /// Grants no stats whatsoever. The board view is the entire reward, which
    /// is a thing worth pinning rather than trusting.
    pub scouting: bool,
    /// Quests handed to pieces that were not born with one.
    granted_quests: std::collections::HashMap<PieceId, &'static crate::piece::Quest>,
    /// Pieces carrying a curse for the rest of the run, whatever they were.
    ///
    /// The library's price. A curse in this game lasts a few seconds and lives
    /// on a fighter; this is the only one that lives on a *piece* and outlasts
    /// the fight, which is why it is a list on the run rather than anything
    /// combat knows about.
    pub cursed_for_good: Vec<PieceId>,
    /// Doors declined rather than answered, and the rung they come back on.
    ///
    /// The one thing an outcome can do that is not a decision: `Defer` leaves
    /// the door standing and lets it find you again. Kept apart from
    /// `answered` because an answered door is finished with and a deferred one
    /// is not - and because the two lists are read by different questions.
    pub deferred: Vec<(&'static str, usize)>,
    /// A fragile thing riding on one of your boards, and until when.
    ///
    /// The rent is paid in dead cells: it occupies a cell that could have held
    /// gear, for five rungs, and it is lost the moment you lose a fight. That
    /// is the whole of it - a courier's job, priced in floor space rather than
    /// in gold.
    pub passenger: Option<(PieceId, usize)>,
    /// A second town action, bought by crushing something. Cleared by the door
    /// it pays for.
    pub second_key_ready: bool,
    /// Pieces sold on consignment, and how many shops until they come back.
    pub consigned: Vec<(usize, u32)>,
    /// A shelf owed once whatever is in front of you is over.
    ///
    /// The Fork's other half: shopping before a fight and shopping after one
    /// are two different decisions, and the only way to offer the second is to
    /// hold the shelf back.
    pub shop_owed: Option<&'static [&'static str]>,
    /// Percentage every shelf costs above the list price, for the rest of the
    /// run. The foundry noticing it was snubbed.
    pub markup: i32,
    /// The rung a taken contract runs to, if one is running.
    ///
    /// Frost on all your gear, accepted deliberately, and the only handicap in
    /// the game a player asks for. `THE PAYOUT` verifies it was honoured by
    /// reading this rather than by trusting a flag.
    pub contract_until: Option<usize>,
    /// Whether a contract ever ran to its end.
    pub contract_honoured: bool,
    /// What the courier pays for a passenger delivered.
    pub passenger_pays: &'static str,
    /// Dungeons entered and not yet finished, outermost first.
    ///
    /// `dungeon` is the one you are standing in; this is what you come back to
    /// when you finish it or walk out of it. Nearly always empty - one dungeon
    /// at a time is the ordinary case - and not always, because a door that
    /// opens a dungeon can stand on the rung you walked into a dungeon from:
    /// THE MANSE is after rung 24 and THE TURNTABLE's window opens at 25, so a
    /// run in the cellar carrying A Word About the Points has a second door in
    /// front of it.
    ///
    /// It was one slot, and entering the second dungeon **erased the first** -
    /// the staircase and everything cleared in it, gone, with no way back to a
    /// door that was already answered. `route.rs`'s rule 2 has said "a dungeon
    /// opened mid-event extends the loop deeper before it returns to the rung
    /// it left" since the Unwinding drew the map; the run state could not say
    /// it.
    pub outer_dungeons: Vec<(&'static crate::dungeon::Dungeon, usize)>,
    /// Floors of dungeons this run has cleared, by dungeon id and floor index.
    ///
    /// Kept for the rest of the run rather than for the visit, because a floor
    /// cleared is a floor cleared: coming back in by another door walks past it
    /// rather than fighting it again. That is the whole reward of a siding -
    /// an orb that took you somewhere you had already been would be a worse
    /// version of the door you already used. It survives a loss, too, which
    /// matters only if something brings you back, and then it matters
    /// completely.
    pub cleared_floors: Vec<(&'static str, usize)>,
    /// Dungeons this run has set foot in, whether or not it won anything.
    ///
    /// `cleared_floors` records wins, and a run that walks into a dungeon and
    /// loses on the first floor has still *been there* - which is the question
    /// the map asks before it draws a place. Kept separate rather than derived
    /// for that reason: the two are different questions and they were the same
    /// answer only by luck.
    pub dungeons_entered: Vec<&'static str>,
    /// Standing at the points: the floor just cleared has more than one way on
    /// and the player has not said which.
    ///
    /// Not derived, because it is genuinely new information. `dungeon` says
    /// where you are and `cleared_floors` says you have beaten it, and neither
    /// says whether you have chosen.
    pub at_points: bool,
    /// Which lever was thrown where, in the order they were thrown: dungeon
    /// id, the floor it was thrown at, and which exit was taken.
    ///
    /// For the map, which draws the road walked and not the ones that were
    /// there, and for the receipt.
    pub took_exits: Vec<(&'static str, usize, usize)>,
    /// Where `cleared_floors` stood when this entry into a dungeon began.
    ///
    /// The banner counts fights won *this entry*, and a run that came back in
    /// by a siding has cleared floors it did not fight today. One index is
    /// enough to tell the two apart, and it is one index rather than a second
    /// counter because a second copy of a fact is a second thing to keep true.
    pub entry_started_at: usize,
    /// The seed this run was made from.
    ///
    /// Kept because THE HUNDRED derives its own from it and must be able to do
    /// so without drawing from `rng` - that stream stocks shops, rolls drops
    /// and melts pieces, and one draw from it here would move every one of
    /// them. `Loadout::name_seed` already held this number and is not it: a
    /// name generator's seed and a run's seed being the same value is a
    /// coincidence, not a fact to build on.
    pub run_seed: u64,
    /// Which tile of THE HUNDRED you are standing on, when you are down there.
    ///
    /// `None` on the road. The county itself is **derived** from the seed
    /// every time it is asked for and never stored - see `Run::county`.
    pub county_at: Option<(u8, u8)>,
    /// Moves left in this trip. Five, and arriving on the mouth is free.
    pub county_moves_left: u8,
    /// Tiles cleared, for the whole run.
    ///
    /// The county is a **place, not an attempt**: a Rogue keeps this when a
    /// life is spent and a Grinder keeps it through a knock-back, because it
    /// is where the endgame lives and re-walking it would be the same five
    /// moves again rather than a second chance at them.
    pub county_cleared: Vec<(u8, u8)>,
    /// One entry a trip. The census: ten, and no more, ever.
    pub county_trips: Vec<TripSource>,
    /// C2's bet: which grid must stay empty, and the rung it must last to.
    pub waste_bet: Option<(SlotKind, usize)>,
    /// Whether the fight in front of you is THE PARISH.
    pub walking_the_parish: bool,
    /// Which way round the perambulation is going, once the first move said.
    ///
    /// `None` until the first move, which is what "chosen by the first move"
    /// means. Cleared with the trip.
    pub perambulation_way: Option<bool>,
    /// How many edge tiles the perambulation has reached.
    ///
    /// The **fifth** is where THE PARISH stands (B5).
    pub perambulation_reached: u8,
    /// Which mouth the Surveyor's Orb should put you down at.
    ///
    /// Set by whatever is drawing the pedestal screen before the orb is fed,
    /// because the choice is the whole value of B1.2's translation: the orb
    /// offers **any** of the six, found or not, which is the only way into a
    /// hidden town's steps for a run that never found the town.
    pub county_mouth_wanted: Option<(u8, u8)>,
    /// Whether Vessey is waiting to be talked to.
    ///
    /// **Not `forced_event`**, and the difference is the whole reason it is a
    /// second field. A forced event is a place you have just been sent and
    /// goes to the front of the stack; this is a man at the roadside with an
    /// opinion about your greaves, and he waits. Answered last, after
    /// everything the road itself has standing.
    pub waste_offered: bool,
    /// How many tiles were cleared when this trip started.
    ///
    /// C1's condition is "a trip that ended with nothing cleared", which is a
    /// question about the difference rather than about the total - and one
    /// index is enough to answer it, the way `entry_started_at` answers the
    /// dungeon banner's.
    pub county_entry_cleared: usize,
    /// The chain whose pinnacle is being fought, if one is.
    ///
    /// Set the moment the fight starts and read by `written_monster`, so that
    /// the creature in front of you is the chain's rather than the rung's -
    /// the priority A2.1 asks for is dungeon, then county pinnacle, then
    /// brawl, then ladder, and this is the county's place in it.
    pub county_pinnacle: Option<crate::county::Chain>,
    /// A county event waiting to be answered, by id into `COUNTY_EVENTS`.
    ///
    /// Kept apart from `forced_event` because the two tables are apart: a
    /// county event id can be arranged onto more than one tile, so it is asked
    /// by the tile you are standing on rather than found by anything, and it
    /// is **not** filtered on `answered` - an id on that list is an id the
    /// road never asks again, which is the opposite of what a repeat needs.
    pub county_event: Option<&'static str>,
    /// Road and county events answered, which is the clock the Drover walks
    /// by. Wired to its increment points at F3; nothing moves it yet.
    pub events_resolved: u32,
    /// Destinations a pedestal has already sent this run to, by id.
    ///
    /// One set for both pedestals. Each destination fires once a run.
    pub destinations_visited: Vec<&'static str>,
    /// An event the road must ask next, wherever the road happens to be.
    ///
    /// Every other event is found by rung. This is for the ones pushed onto
    /// the stack from somewhere that is not a rung at all - a pedestal in a
    /// shop the size of a weather system, or a fork that puts two futures in
    /// front of you and asks only for the order.
    pub forced_event: Option<&'static str>,
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
            answered_on: Vec::new(),
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
            outer_dungeons: Vec::new(),
            cleared_floors: Vec::new(),
            dungeons_entered: Vec::new(),
            at_points: false,
            took_exits: Vec::new(),
            entry_started_at: 0,
            run_seed: seed,
            county_at: None,
            county_moves_left: 0,
            county_cleared: Vec::new(),
            county_trips: Vec::new(),
            county_event: None,
            county_entry_cleared: 0,
            county_pinnacle: None,
            waste_bet: None,
            waste_offered: false,
            county_mouth_wanted: None,
            perambulation_way: None,
            perambulation_reached: 0,
            walking_the_parish: false,
            events_resolved: 0,
            destinations_visited: Vec::new(),
            cursed_for_good: Vec::new(),
            deferred: Vec::new(),
            passenger: None,
            second_key_ready: false,
            consigned: Vec::new(),
            shop_owed: None,
            markup: 0,
            contract_until: None,
            contract_honoured: false,
            passenger_pays: "",
            forced_event: None,
            flags: Vec::new(),
            counters: Vec::new(),
            francis_beaten: 0,
            last_figure: None,
            owed_rows: 0,
            claim_tickets: 0,
            standing_orders: Vec::new(),
            underwritten_until: None,
            scouting: false,
            granted_quests: std::collections::HashMap::new(),
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
        run.owned = all_def_indices()
            .into_iter()
            // Every piece of *gear*. A rumour is a key rather than a
            // component, and a fixture holding all of them opens every rumour
            // door in the game at once - so a test about the VIP area found
            // itself answering a locked gate instead, because a rumour door
            // goes first and the chain's windows are wide.
            .filter(|&d| !crate::rumour::is_rumour(CATALOG[d].name))
            .map(|d| run.registry.alloc(d))
            .collect();
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
        // A standing order makes the *first* one free and leaves the rest
        // where they were, so it is a nudge to look again rather than a
        // licence to roll the shop until it says what you want.
        if self.rerolls == 0 && self.shop.free_first_reroll {
            return 0;
        }
        REROLL_COST << self.rerolls.min(16)
    }

    /// Is the player without an assembled weapon? The shop guarantees one can
    /// be built only when the answer is yes.
    pub fn needs_a_weapon(&self) -> bool {
        self.report(SlotKind::Weapon).items.iter().all(|i| !i.assembled)
    }

    pub fn monster(&self) -> MonsterSpec {
        self.written_monster().doubled(self.doublings())
    }

    /// The same creature, exactly as the table writes it.
    ///
    /// Split from `monster` because the doubling is a property of the *run*,
    /// not of the creature, and one or two things want the figure that was
    /// authored rather than the figure being fought.
    pub fn written_monster(&self) -> &'static MonsterSpec {
        // An event can put something else in front of you. It stands in for
        // the rung rather than adding one, so the road stays the same length
        // whichever way you answered.
        // A dungeon floor stands in front of everything else.
        if let Some((d, floor)) = self.dungeon {
            if let Some(spec) =
                d.floors.get(floor).and_then(|f| crate::combat::alternate(f.creature))
            {
                return spec;
            }
        }
        // THE HUNDRED's endings. Under a dungeon - you cannot be in both -
        // and over everything the road has, because a run standing on a
        // pinnacle is not standing on a rung.
        if self.walking_the_parish {
            if let Some(spec) = crate::combat::alternate("THE PARISH") {
                return spec;
            }
        }
        if let Some(chain) = self.county_pinnacle {
            if let Some(spec) = crate::combat::alternate(crate::county::pinnacle_creature(chain)) {
                return spec;
            }
        }
        if let Some(m) = self.substitute {
            return m;
        }
        &LADDER[self.rung.min(LADDER.len() - 1)]
    }

    /// How many times the thing in front of you has been doubled.
    ///
    /// Only the man at the top doubles, and only for a run that has already
    /// put him down. `Run::monster` clamps to the last rung, so past the
    /// ladder every rung is Francis again - this is what stops that being a
    /// treadmill and makes it a wall that moves.
    ///
    /// Counted in Francises beaten rather than in rungs past fifty. The two
    /// agree on every run except one that took the road past him, where rung
    /// 51 is not a Francis at all, and a run that walked down there should
    /// not find him twice as hard for having done it.
    pub fn doublings(&self) -> u32 {
        if self.written_monster().name == "Francis" {
            self.francis_beaten
        } else {
            0
        }
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
        // Being in the county is not something waiting for you, it is where
        // you are - the same reason a dungeon is on top of the rung. It is on
        // top of the *dungeon* too, and that is a statement about the two
        // never overlapping rather than about their order: a town gate and a
        // dungeon mouth are not the same door.
        if let Some(at) = self.county_at {
            out.push(Interrupt::County { at, moves_left: self.county_moves_left });
        }
        if let Some((d, floor)) = self.dungeon {
            // The lever is in the dungeon and you are standing at the lever.
            if self.at_points {
                out.push(Interrupt::Points(d, floor));
            }
            let won = self.fights_this_entry();
            out.push(Interrupt::Dungeon {
                at: d,
                floor,
                nth: won + 1,
                of: won + d.fights_ahead(floor, &self.cleared_floors),
            });
        }
        // And whatever is underneath it. Innermost first, so the strip reads
        // downwards the way the run walked: the yard, then the staircase you
        // opened it from, then the rung.
        //
        // Their banners count the fights of the entry that is paused, which is
        // the number that will be true again when the run comes back up to
        // them.
        for &(d, floor) in self.outer_dungeons.iter().rev() {
            out.push(Interrupt::Dungeon {
                at: d,
                floor,
                nth: 1,
                of: d.fights_ahead(floor, &self.cleared_floors),
            });
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

    /// Is this door standing off until a later rung?
    fn deferred_past(&self, id: &str) -> bool {
        self.deferred.iter().any(|(d, until)| *d == id && self.rung < *until)
    }

    /// Every event standing on this rung, in the order they will be asked.
    ///
    /// Rumour doors first - having gone to the trouble of earning one you
    /// should get to see it - then whatever `event::at` finds, which is one at
    /// most because it takes the first match. So this is usually a list of one
    /// and occasionally a list of two, and the second is the reason it is a
    /// list at all.
    fn standing_events(&self) -> Vec<&'static crate::event::LadderEvent> {
        // Anything pushed here from off the road goes first. It is not waiting
        // for a rung, so a rung is not what decides when it is asked.
        // Standing on a county tile that asked something. First, and ahead of
        // even a forced event: you are down a hole and the road is not where
        // you are.
        let mut out: Vec<&'static crate::event::LadderEvent> = self
            .county_event
            .and_then(crate::event::county_event)
            .into_iter()
            .collect();
        out.extend(
            self.forced_event
                .and_then(|id| crate::event::EVENTS.iter().find(|e| e.id == id))
                .filter(|e| !self.answered.contains(&e.id)),
        );
        for e in self.whispered_events() {
            if !out.iter().any(|o| o.id == e.id) {
                out.push(e);
            }
        }
        // The chain's own doors: standing because of something the run did
        // rather than because of where it is.
        for e in crate::event::EVENTS.iter() {
            let crate::event::Trigger::WhenFlagged { flag, from } = e.trigger else { continue };
            if !(from..=e.at).contains(&self.rung)
                || self.answered.contains(&e.id)
                || self.deferred_past(e.id)
                || !self.flags.contains(&flag)
                || e.blocked_by.iter().any(|id| self.answered.contains(id))
            {
                continue;
            }
            if !out.iter().any(|o| o.id == e.id) {
                out.push(e);
            }
        }
        // Every one of them, not the first. Two can stand on one rung - an
        // earned window passing over a scheduled rung - and the scheduled one
        // is the one that expires, so `standing_at` puts it first and both are
        // asked in turn.
        for e in
            crate::event::standing_at(self.rung, self.best_fight_ms, self.worst_fight_ms, &self.answered)
                .into_iter()
                .filter(|e| !self.answered.contains(&e.id))
        {
            if !out.iter().any(|o| o.id == e.id) {
                out.push(e);
            }
        }
        // And Vessey, **last**. C2 is a question about your board rather than
        // about where you are standing, so it waits behind everything the road
        // itself has - a chain's door met on the same rung is met first, and
        // `switchyard::the_chain_can_be_walked_in_one_run` is the test that
        // found what happens when it is not.
        if self.waste_offered && !self.answered.contains(&"the-waste") {
            if let Some(e) = crate::event::EVENTS.iter().find(|e| e.id == "the-waste") {
                if !out.iter().any(|o| o.id == e.id) {
                    out.push(e);
                }
            }
        }
        out
    }

    /// The event standing in front of this rung, if there is one and it has
    /// not been answered.
    pub fn pending_event(&self) -> Option<&'static crate::event::LadderEvent> {
        if self.phase != Phase::Loadout {
            return None;
        }
        // **A county tile's question is not behind the fountain.** The
        // fountain is owed on a rung and the county is not a rung: a run that
        // walked down the steps with one due would otherwise set the tile's
        // question, see nothing, keep walking - and be asked it on the road
        // one town later, which is what playing it looked like.
        //
        // Asked before the two gates rather than after, because those gates
        // are about the *road* being ready to ask, and down here the road is
        // not what is asking.
        if let Some(ev) = self.county_event.and_then(crate::event::county_event) {
            return Some(ev);
        }
        // A rumour door first: it stands on the same rung as whatever else is
        // there, and having gone to the trouble of earning it you should get
        // to see it. `standing_event` is that question; the two gates above
        // are the difference between "there" and "askable now".
        if self.at_fountain() || self.at_doubling_fountain() {
            return None;
        }
        self.standing_event()
    }

    /// Every rumour door standing on this rung: ones you are carrying the word
    /// about, whose condition you have actually met.
    ///
    /// Separate from `event::at` because neither half can be answered from a
    /// rung and two stopwatches - one is about the board and one is about the
    /// whole run so far, and the run is the only thing that knows either.
    ///
    /// **All of them, not the first.** This was a `find`, and the effect was
    /// not that the second door went unasked - `standing_events` is called
    /// again after each answer, so it did get asked - but that nothing could
    /// *see* it coming. The road stack strip is built from this, so a rung
    /// carrying two words showed one, the player answered it, and a second
    /// door appeared out of nowhere. Two doors resolving back to back reads as
    /// a bug when the strip promised one.
    fn whispered_events(&self) -> Vec<&'static crate::event::LadderEvent> {
        crate::event::EVENTS
            .iter()
            .filter(|e| {
                let crate::event::Trigger::Whispered { rumour, from } = e.trigger else {
                    return false;
                };
                (from..=e.at).contains(&self.rung)
                    && !self.deferred_past(e.id)
                    && !self.answered.contains(&e.id)
                    && self.owned.iter().any(|&i| self.registry.def(i).name == rumour)
                    && crate::rumour::by_name(rumour).is_some_and(|r| self.meets(r.needs))
            })
            .collect()
    }

    /// Is a rumour's condition true right now?
    pub fn meets(&self, c: crate::rumour::Condition) -> bool {
        use crate::rumour::Condition;
        match c {
            Condition::Crowded { slot, under } => self.empty_cells(slot) < under,
            Condition::BankedAllRun { what, at_least } => {
                self.banked_all_run[what.index()] >= at_least
            }
            Condition::Carried => true,
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
            // Never a key, and never the cargo.
            //
            // This filtered on *shape* alone, and every quest item in the game
            // is one cell - so "hand her a loose one-by-one" would take a
            // rumour word, the Platinum Chip, or An Unwound Mainspring, which
            // is the key to the only rung past Francis. A door that asks for
            // something small could end a chain, shut a door forty rungs away,
            // or quietly sell the ending, and say nothing about it either way.
            //
            // The same reasoning `melt` already applies: "quest pieces and
            // rumours refuse the pot - one is the far side of a task and the
            // other is a key, and neither is gear in the sense a crucible
            // understands". A courier is not a crucible, but it is not owed
            // your keys either.
            .filter(|&id| self.registry.def(id).kind != crate::piece::PieceKind::Quest)
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
        self.requirement_met(c.requires)
    }

    /// Whether one requirement is met by this run.
    ///
    /// Split out of `choice_open` so the pale's checklist can ask the same
    /// question a choice asks. A checklist that computed its own answers would
    /// be a second implementation of every requirement in the game, kept in
    /// step by hand.
    pub fn requirement_met(&self, requires: crate::event::Requirement) -> bool {
        use crate::event::Requirement;
        match requires {
            Requirement::None => true,
            Requirement::Took(label) => self.took.contains(&label),
            // Worn or loose, both count: a key you have built into a helmet is
            // still a key you have.
            Requirement::Holding(name) => {
                self.owned.iter().any(|&id| self.registry.def(id).name == name)
            }
            Requirement::LooseItemOfSize { .. } => !self.offerings(requires).is_empty(),
            Requirement::Flag(what) => self.flags.contains(&what),
            Requirement::Counter { what, at_least } => self.counted(what) >= at_least,
            Requirement::CountyTiles { region, at_least } => {
                self.county_cleared_in(region) >= at_least
            }
            // Answered at F8, when a pinnacle can be beaten. Until then no
            // chain is finished and nothing asks - the two variants land inert
            // so that the milestone which authors the pale's checklist finds
            // the requirement rather than inventing it under deadline.
            Requirement::CountyCleared(chain) => self.county_chain_done(chain),
            Requirement::AssembledOfRarity(want) => self
                .combat_items()
                .iter()
                .any(|i| crate::rating::Rarity::of(i.rating) >= want),
            Requirement::AlignedItems(n) => self.most_aligned() >= n,
            // Anybody can say a figure. What happens to the figure is the
            // door's business, and it happens in `take_choice_with`.
            Requirement::Figure { .. } => true,
            Requirement::Purse { times } => self.gold >= self.rung_bounty() * times,
            Requirement::HoldingRumour => self
                .inventory()
                .iter()
                .any(|&id| crate::rumour::is_rumour(self.registry.def(id).name)),
            // `PieceKind::Orb`, which is B3.1's own wording - "any Orb-kind
            // piece" - and **not** `is_orb_of_travel`, which is the four
            // pedestal keys and would have refused the county's own two.
            //
            // `CLAUDE.md` §6 trap 26: the kind is twenty-three pieces over
            // eight footprints. That is the right price for this gate anyway:
            // an orb is a weapon core somebody built around, so surrendering
            // one costs a board rather than a ticket.
            Requirement::HoldingOrb => self
                .owned
                .iter()
                .any(|&id| self.registry.def(id).kind == crate::piece::PieceKind::Orb),
            Requirement::ThePaleIsReady => {
                self.pale_is_ready() && self.requirement_met(Requirement::HoldingOrb)
            }
            Requirement::Classes(n) => self.classes.len() >= n,
        }
    }

    /// What the rung in front of you is worth.
    ///
    /// The unit every price in this mission is quoted in. A figure written as
    /// a constant means one thing at rung four and something else entirely at
    /// rung forty - thirty gold is three fights down there and a rounding
    /// error up here - and the road is forty-six rungs long.
    pub fn rung_bounty(&self) -> i32 {
        LADDER[self.rung.min(LADDER.len() - 1)].bounty
    }

    /// How many times something silently counted has happened.
    pub fn counted(&self, what: &str) -> u32 {
        self.counters.iter().find(|(k, _)| *k == what).map(|(_, v)| *v).unwrap_or(0)
    }

    /// Count one more of something, silently.
    pub fn count(&mut self, what: &'static str) {
        match self.counters.iter_mut().find(|(k, _)| *k == what) {
            Some(e) => e.1 += 1,
            None => self.counters.push((what, 1)),
        }
    }

    /// The largest group of assembled items sharing one earned qualifier.
    ///
    /// The inspector's question, and "alignment" in the sense the naming layer
    /// already uses it: a qualifier is the adjective an item earns from what
    /// its pieces *do* - Searing, Warded, Hastening - so items sharing one are
    /// items built around the same idea. Not `PieceKind::Alignment`, which is
    /// a crystal ball's colour and belongs to one recipe out of eight.
    ///
    /// Read off the live board rather than the tray, which is what makes
    /// building *for* an event a strategy rather than a thing you happen to
    /// have in a pocket.
    pub fn most_aligned(&self) -> usize {
        let items = self.combat_items();
        let words: Vec<Vec<&'static str>> = items
            .iter()
            .map(|i| crate::naming::qualifiers(&self.registry, &i.pieces))
            .collect();
        let mut best = 0usize;
        for mine in &words {
            for w in mine {
                best = best.max(words.iter().filter(|other| other.contains(w)).count());
            }
        }
        best
    }

    /// Answer a door that asked for a number.
    ///
    /// A choice whose requirement is a `Figure` cannot go through
    /// `take_choice`, because there is nothing there to take it with. The
    /// figure is remembered on the run so the outcome can read it and the
    /// receipt can say what it was.
    pub fn take_choice_with(
        &mut self,
        c: &crate::event::Choice,
        figure: i32,
    ) -> Option<&'static str> {
        let crate::event::Requirement::Figure { min, max } = c.requires else { return None };
        if figure < min || figure > max {
            return None;
        }
        self.last_figure = Some(figure);
        self.take_choice_unchecked(c)
    }

    /// Answer the event in front of you.
    ///
    /// Returns what it cost, if anything - the component handed over, by name -
    /// so the interface can say what just happened. Refuses a choice whose
    /// requirement is not met, so the offer cannot be widened by asking
    /// differently.
    pub fn take_choice(&mut self, c: &crate::event::Choice) -> Option<&'static str> {
        // A door that wants a figure is answered with one. Refused here rather
        // than resolved with a default, because a default bid is a bid nobody
        // made.
        if matches!(c.requires, crate::event::Requirement::Figure { .. }) {
            return None;
        }
        self.take_choice_unchecked(c)
    }

    fn take_choice_unchecked(&mut self, c: &crate::event::Choice) -> Option<&'static str> {
        let Some(ev) = self.pending_event() else { return None };
        // The choice has to belong to the door that is actually standing here.
        //
        // It did not have to before, because one door stood on a rung and the
        // interface only ever offered that door's choices. The chain's windows
        // are wide enough that two can be open at once, and answering one with
        // the other's choice marked the wrong event answered.
        //
        // By value rather than by address. `EVENTS` is a static holding
        // promoted arrays, and a caller in another crate can hold a reference
        // to a *copy* of the same choice - the address test passes inside the
        // engine, passes in the interface, and fails in a test binary, which
        // is the worst of the three places to find out. What "belongs to this
        // door" means is that the door has this choice on it, and that is what
        // this asks.
        // Kept as the *table's* copy rather than the caller's. They are equal
        // by value - that is what the check above establishes - but only the
        // table's is `'static`, and the reverse indexes that say what a choice
        // opened are all over static data.
        let Some(chosen) = ev.choices.iter().find(|k| *k == c) else {
            return None;
        };
        if !self.choice_open(c) {
            return None;
        }
        self.answered.push(ev.id);
        self.answered_on.push((ev.id, self.rung));
        // The clock THE HUNDRED's Drover walks by (A5). **One place, not the
        // three the spec counts**, because every event in the game - a rung's
        // door, a chain's, a dungeon mouth's, a forced one off a pedestal, and
        // from F7 a county tile's - is answered here and nowhere else.
        //
        // It is a separate counter and not `answered.len()` for a reason F7
        // needs: a county event id can be arranged onto more than one tile,
        // and an id on `answered` is an id that never asks again.
        self.events_resolved += 1;
        if self.forced_event == Some(ev.id) {
            self.forced_event = None;
        }
        if ev.id == "the-waste" {
            self.waste_offered = false;
        }
        // A county event is answered where it stands, and answering is what
        // clears the tile - a run that walked onto a question and walked away
        // has not answered it.
        if self.county_event == Some(ev.id) {
            self.county_event = None;
            // Answering a county event moves the clock, and the clock is what
            // the Drover walks by - so a question can bring the pursuit to
            // *you*, standing still, which is the best thing in the chain.
            // Checked after the outcome is applied, at the bottom of this
            // function, so the answer pays out before the fight starts.
            // **The pale is a gate and not a question.** Every other county
            // event is finished with you once you have answered it; this one
            // is finished with you once it *opens*, and reading the list is
            // not opening it.
            //
            // Without this the pale is consumed on first contact - "read the
            // list again" is open to anybody, answering it clears the tile,
            // and nothing that walks to uncleared tiles ever comes back. It is
            // the one county event whose tile is a door rather than a scene,
            // and `the_pale_is_not_consumed_by_reading_its_own_list` is what
            // holds it open.
            let a_gate_still_shut = ev.id == crate::county::PALE && !self.pale_is_open();
            if let Some(at) = self.county_at {
                if !a_gate_still_shut && !self.county_cleared.contains(&at) {
                    self.county_cleared.push(at);
                }
            }
            // A door answered on the last move ends the trip, which
            // `county_walk` could not do while the question was open.
            self.end_county_trip_if_spent();
        }
        self.took.push(c.label);
        // A price is paid when the choice is *taken*, which is what makes it a
        // price rather than a test of wealth - and here rather than inside
        // `apply_outcome`, because two outcomes recurse into it and a price
        // charged once per nested outcome is a price charged twice.
        let cost = match c.requires {
            crate::event::Requirement::Purse { times } => self.rung_bounty() * times,
            _ => 0,
        };
        self.gold -= cost;
        let (gave, mut receipt) = self.apply_outcome(&c.outcome, c.requires);
        if cost > 0 {
            receipt.insert(0, format!("-{}g", cost));
        }
        // And what it opened, if it opened anything. A receipt that says what
        // changed on your board and nothing about the door you just unlocked
        // sends the player back to the road thinking the answer was "fight the
        // next thing", which is the one reading that is wrong.
        receipt.extend(self.opened_by(&chosen.outcome));
        receipt.extend(self.opened_by_taking(chosen.label));
        self.last_receipt = Some(receipt);
        // The clock has moved, and the Drover walks by the clock. A door
        // answered on the tile the pursuit is about to reach brings it to you.
        if self.drover_is_here() {
            self.intercept_the_drover();
        }
        gave
    }

    /// What taking this outcome has opened further up the road.
    ///
    /// Built from the reverse indexes the tables already carry rather than
    /// from a second list that could drift: `event::set_by` walks the flags,
    /// `rumour::conditions_line` walks the words, and a town knows where it
    /// stands. Nothing here names a *condition* - a rumour's hint is vague on
    /// purpose and this does not undo that. It says a door exists and roughly
    /// where, which is the difference between a choice that felt like it did
    /// something and one that felt like it did nothing.
    fn opened_by(&self, outcome: &'static crate::event::Outcome) -> Vec<String> {
        use crate::event::Outcome;
        let mut out = Vec::new();
        for o in crate::event::every_outcome(outcome) {
            match o {
                // A flag is read by doors that wait on it. Name them: the
                // player set the flag deliberately and the door is the reason.
                Outcome::Flag(f) => {
                    for e in crate::event::EVENTS.iter() {
                        let crate::event::Trigger::WhenFlagged { flag, from } = e.trigger else {
                            continue;
                        };
                        if flag == *f && from >= self.rung {
                            out.push(format!("Opened: {} (rung {} or later)", e.title, from + 1));
                        }
                    }
                }
                // A word is a key, and the tray already says what it is for in
                // exactly these words. Saying it here too is the same fact in
                // the place the player is looking.
                Outcome::Give(name) if crate::rumour::is_rumour(name) => {
                    if let Some(line) = crate::rumour::conditions_line(name) {
                        out.push(line);
                    }
                }
                // `describe` already says a town was revealed and where it
                // stands. The only thing it cannot say is that the gate is
                // *behind* you, which reads as good news and is not.
                Outcome::RevealTown(id) => {
                    if let Some(t) = crate::town::by_id(id) {
                        if t.after < self.rung {
                            out.push(format!(
                                "{} stands behind you, and the road does not go back",
                                t.name
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// What *choosing this* has opened, as opposed to what its outcome did.
    ///
    /// A door can wait on a label rather than on a flag - `Requirement::Took`
    /// - and that is the quietest unlock in the game: the choice's own outcome
    /// is often `FightAsWritten`, so the receipt said "Fight the creature
    /// standing here" and the player had no way to know they had just opened
    /// something twenty rungs up. Plugging your ears at the Teller is exactly
    /// this, and it is the one that gets you into EXTRA LARGE.
    fn opened_by_taking(&self, label: &'static str) -> Vec<String> {
        let mut out = Vec::new();
        for e in crate::event::EVENTS.iter() {
            if self.answered.contains(&e.id) {
                continue;
            }
            for c in e.choices {
                if matches!(c.requires, crate::event::Requirement::Took(l) if l == label) {
                    out.push(format!("Opened: {} (rung {})", e.title, e.at + 1));
                }
            }
        }
        out
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
            ChoiceOutcome::Enter(id) => self.enter_dungeon(id),
            ChoiceOutcome::Flag(what) => {
                if !self.flags.contains(&what) {
                    self.flags.push(what);
                }
                // C2. The bet is on a *grid*, and which grid is a fact about
                // the board rather than about the choice - so the outcome
                // raises a flag and the rule reads the board, which is the
                // same shape `Underwrite` and the contract already have.
                // C1. He is not asking, so the ride is the outcome rather
                // than a thing offered by it.
                if what == "arrested" {
                    self.arrested_into_the_county();
                }
                if what == "waste-bet-taken" {
                    if let Some(k) = SlotKind::ALL
                        .into_iter()
                        .find(|k| self.report(*k).items.iter().all(|i| !i.assembled))
                    {
                        self.waste_bet = Some((k, self.rung + 5));
                    }
                }
            }
            ChoiceOutcome::Count(what) => self.count(what),
            ChoiceOutcome::RevealTown(id) => {
                if !self.reveal_town(id) {
                    receipt.clear();
                    receipt.push("Nothing: there is no such place".into());
                }
            }
            ChoiceOutcome::OpenShop { shelves } => self.shop.stock_exactly(shelves),
            ChoiceOutcome::StartDungeon(id) => self.enter_dungeon(id),
            ChoiceOutcome::GrantRow => self.owed_rows += 1,
            ChoiceOutcome::GrantQuest(q) => match self.inventory().first().copied() {
                Some(id) => {
                    self.grant_quest(id, q);
                    receipt.push(format!("Set on the table: {}", self.registry.def(id).name));
                }
                None => {
                    receipt.clear();
                    receipt.push("Nothing to put on the table".into());
                }
            },
            ChoiceOutcome::ClaimTicket => self.claim_tickets += 1,
            ChoiceOutcome::StandingOrder(o) => {
                if !self.standing_orders.contains(&o) {
                    self.standing_orders.push(o);
                }
                match o {
                    crate::event::Standing::GuaranteedKind(k) => {
                        if !self.shop.guaranteed.contains(&k) {
                            self.shop.guaranteed.push(k);
                        }
                        let need = self.needs_a_weapon();
                        self.restock(need);
                    }
                    crate::event::Standing::FreeFirstReroll => {
                        self.shop.free_first_reroll = true;
                    }
                    // Nothing to arm: the shelf does not change, the sell does.
                    crate::event::Standing::Consignment => {}
                }
            }
            ChoiceOutcome::Underwrite => {
                self.underwritten_until = Some(self.rung + UNDERWRITTEN_FOR);
                receipt.push(format!("Good until rung {}", self.rung + UNDERWRITTEN_FOR + 1));
            }
            ChoiceOutcome::Scout => self.scouting = true,
            ChoiceOutcome::UnlockInsight => self.unlock_insight(),
            ChoiceOutcome::SealedBid { lots } => {
                // The reserve is the run's own, and it is drawn *here* rather
                // than held on the event, so two replays of a seed bid against
                // the same number and a reload cannot re-roll it.
                let lot = lots[self.rng.below(lots.len().max(1)).min(lots.len() - 1)];
                let reserve = self.rung_bounty() * (1 + self.rng.below(6) as i32);
                let bid = self.last_figure.unwrap_or(0);
                receipt = vec![format!("The reserve was {}g", reserve)];
                if bid >= reserve {
                    self.gold -= reserve;
                    receipt.push(format!("-{}g, and the lot is yours", reserve));
                    let (_, lines) = self.apply_outcome(&ChoiceOutcome::Give(lot), req);
                    receipt.extend(lines);
                } else {
                    receipt.push("Under. The lot goes to somebody who guessed better".into());
                }
            }
            ChoiceOutcome::ShopAfter { shelves } => self.shop_owed = Some(shelves),
            ChoiceOutcome::Markup(pct) => self.markup += pct,
            ChoiceOutcome::Passenger { rungs, pays } => {
                // The passenger arrives as a component, because the rent is
                // cells and cells are what components cost. It goes in the
                // tray like anything else; seating it is the player's job and
                // `passenger_is_seated` is what checks they did it.
                match self.give("The Stranger's Parcel") {
                    Some(id) => {
                        self.take_passenger(id, rungs);
                        self.passenger_pays = pays;
                        receipt.push(format!("Riding: {}", self.registry.def(id).name));
                    }
                    None => {
                        receipt.clear();
                        receipt.push("Somebody else is already riding on you".into());
                    }
                }
            }
            ChoiceOutcome::Contract { rungs } => {
                self.contract_until = Some(self.rung + rungs);
                receipt.push(format!("It runs out after rung {}", self.rung + rungs + 1));
            }
            ChoiceOutcome::Uncurse => match self.cursed_for_good.pop() {
                Some(id) => receipt = vec![format!("Lifted: {}", self.registry.def(id).name)],
                None => receipt = vec!["Nothing of yours is cursed".into()],
            },
            // The Multicity buyer's three purchases. Each one takes something
            // away that the run cannot buy back, which is the only reason the
            // numbers next to them are as large as they are.
            ChoiceOutcome::SellWord => {
                let held = self
                    .owned
                    .iter()
                    .copied()
                    .find(|&id| crate::rumour::is_rumour(self.registry.def(id).name));
                match held {
                    // Selling the word *is* shutting the door: a rumour door
                    // opens on the piece being in hand, so handing the piece
                    // over is the whole of the mechanism and no second flag is
                    // needed to remember it.
                    Some(id) => {
                        let name = self.registry.def(id).name;
                        self.loadout.remove_anywhere(id);
                        self.owned.retain(|&o| o != id);
                        self.forget_undo();
                        receipt = vec![format!("Sold: {}", name)];
                    }
                    None => receipt = vec!["You are carrying nothing anybody told you".into()],
                }
            }
            ChoiceOutcome::SellTitle => match self.classes.pop() {
                Some(c) => {
                    self.refresh_class_effects();
                    receipt = vec![format!("Sold: {}", c.name)];
                }
                None => receipt = vec!["You are nobody in particular".into()],
            },
            // The library's curse, laid by a wizard instead. Permanent, on a
            // piece rather than a fighter, and chosen by the run's own PRNG so
            // that two replays of a seed chill the same thing.
            ChoiceOutcome::Chill => {
                // Something that *acts*, not something that merely sits there.
                // Frost slows gear, and a loose component has no cooldown to
                // slow - freezing one would be a receipt line and nothing
                // else, which is the shape of the bug this arm was written
                // after.
                let out: Vec<PieceId> = self
                    .combat_items()
                    .iter()
                    .flat_map(|i| i.pieces.clone())
                    .filter(|id| !self.cursed_for_good.contains(id))
                    .collect();
                match out.len() {
                    0 => receipt = vec!["Nothing of yours is out where it could freeze".into()],
                    n => {
                        let id = out[self.rng.below(n)];
                        self.cursed_for_good.push(id);
                        receipt = vec![format!("Chilled: {}", self.registry.def(id).name)];
                    }
                }
            }
            ChoiceOutcome::All(each) => {
                receipt.clear();
                for o in each {
                    let (g, lines) = self.apply_outcome(o, req);
                    gave = gave.or(g);
                    receipt.extend(lines);
                }
            }
            ChoiceOutcome::Pay { times } => {
                let paid = self.rung_bounty() * times;
                self.gold += paid;
                receipt = vec![format!("+{}g", paid)];
            }
            ChoiceOutcome::Health(n) => {
                self.grown_health += n;
                receipt = vec![if n < 0 {
                    format!("{} maximum health, and it does not come back", n)
                } else {
                    format!("+{} maximum health", n)
                }];
            }
            ChoiceOutcome::Gamble { wins, outof, won, lost } => {
                // Out of the run's own PRNG, never combat. Two replays of a
                // seed gamble the same way, which is the whole of E6.1.
                let roll = self.rng.below(outof.max(1) as usize) as u32;
                let (_, lines) = self.apply_outcome(if roll < wins { won } else { lost }, req);
                receipt = lines;
            }
            ChoiceOutcome::SurrenderOrb => {
                // Worn or loose. `remove_anywhere` takes it out of whatever it
                // was built into, which is what makes this a real price on a
                // board rather than a tax on the tray.
                if let Some(id) = self
                    .owned
                    .iter()
                    .copied()
                    .find(|&id| self.registry.def(id).kind == crate::piece::PieceKind::Orb)
                {
                    let name = self.registry.def(id).name;
                    self.loadout.remove_anywhere(id);
                    self.owned.retain(|&o| o != id);
                    self.forget_undo();
                    receipt = vec![format!("The gatepost takes {name}")];
                }
            }
            ChoiceOutcome::Defer { rungs } => {
                // Declining is not answering. The door comes off `answered`
                // and goes onto the list of things that will find you again,
                // which is the whole difference between "no" and "not yet".
                //
                // And off the clock with it, one line later so the two cannot
                // drift: a run that could advance the Drover by saying "not
                // yet" to the same door could walk it round the ring for
                // nothing, which is an interception bought rather than
                // intercepted.
                let id = self.answered.pop();
                self.events_resolved = self.events_resolved.saturating_sub(1);
                if let Some(id) = id {
                    match self.deferred.iter_mut().find(|(d, _)| *d == id) {
                        Some(e) => e.1 = self.rung + rungs,
                        None => self.deferred.push((id, self.rung + rungs)),
                    }
                }
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
                self.restock(need);
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

    /// Step into a dungeon, and say so.
    ///
    /// The cutscene is the whole of the addition: a door that hands you three
    /// fights and says nothing is a door you can walk through by accident,
    /// and a fight you did not know you had chosen is the one kind this game
    /// should never hand out. Played on the machinery the bosses use, so it
    /// skips the same way everything else does.
    pub fn enter_dungeon(&mut self, id: &'static str) {
        let Some(d) = crate::dungeon::by_id(id) else { return };
        self.enter_dungeon_at(d, 0);
    }

    /// Step into a dungeon at a particular floor.
    ///
    /// The way in a siding uses, and what `enter_dungeon` is now written on
    /// top of. Takes the dungeon rather than its id because that is what the
    /// caller has - `by_id` is public and every road-side caller has already
    /// resolved one - and because a graph is worth proving against a dungeon
    /// that exists only in a test binary, where an id would find nothing.
    ///
    /// The floor's own entry scene is played if it has one, and the dungeon's
    /// otherwise. Then the walk-through runs, because the whole point of a
    /// siding is that you may have been here.
    pub fn enter_dungeon_at(&mut self, d: &'static crate::dungeon::Dungeon, floor: usize) {
        if floor >= d.floors.len() {
            return;
        }
        // Whatever you were in, you are still in - underneath this one.
        if let Some(outer) = self.dungeon {
            if outer.0.id != d.id {
                self.outer_dungeons.push(outer);
            }
        }
        self.dungeon = Some((d, floor));
        if !self.dungeons_entered.contains(&d.id) {
            self.dungeons_entered.push(d.id);
        }
        self.at_points = false;
        self.entry_started_at = self.cleared_floors.len();
        let own = d.floors[floor].entry;
        let entry = if own.is_empty() { self.theme.entry(d.id, d.entry) } else { own };
        if !entry.is_empty() {
            self.pending_scene = Some(entry);
        }
        let walked = self.walk_through_cleared();
        if !walked.is_empty() {
            self.last_receipt = Some(walked);
        }
    }

    /// Come back up: out of the dungeon you were in, into the one under it.
    ///
    /// The one place `dungeon` is cleared, so that finishing, walking out and
    /// being carried out all leave a run in the same place - which is the one
    /// under it, or the road.
    fn back_up_a_dungeon(&mut self) {
        self.dungeon = self.outer_dungeons.pop();
        self.at_points = false;
        // The entry counter belongs to whatever you are standing in now, and
        // the floors of the outer one were cleared before this entry began.
        self.entry_started_at = self.entry_started_at.min(self.cleared_floors.len());
    }

    /// Have you beaten this floor of this dungeon, at any point this run?
    pub fn has_cleared(&self, dungeon: &str, floor: usize) -> bool {
        self.cleared_floors.iter().any(|&(id, f)| id == dungeon && f == floor)
    }

    /// Fights won since this entry into a dungeon began.
    ///
    /// Floors walked through do not count, because they were not fought
    /// today - which is exactly the difference the banner exists to show.
    pub fn fights_this_entry(&self) -> usize {
        self.cleared_floors.len().saturating_sub(self.entry_started_at)
    }

    /// Follow the road as far as floors this run has already beaten allow.
    ///
    /// A floor cleared is a floor cleared, so coming back in by a siding walks
    /// past the yard you know rather than fighting it again. Stops at the
    /// first floor with a fight still in it, or at a set of points where more
    /// than one road still has one - which is a decision and has to be handed
    /// back.
    ///
    /// **A road is open when there is still a fight somewhere down it**, which
    /// is `fights_ahead(to) > 0` and not "the next room is unbeaten". The two
    /// differ, and the difference is a bug: a run that walked one road as far
    /// as its second room and left has beaten that road's *first* room, so
    /// asking about the next room alone says that road is finished and quietly
    /// sends the player down the other one, past two rooms nobody has fought.
    /// `a_cleared_floor_is_walked_through_on_re_entry` is the case.
    ///
    /// Returns the receipt lines, one a floor, so the player who came in by a
    /// siding watches the part they already walked go past instead of seeing a
    /// banner that jumped.
    fn walk_through_cleared(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        // The graph is acyclic (`dungeon::no_dungeon_doubles_back`) so this
        // terminates on its own. The bound stands anyway: a lint that has not
        // run yet is not a proof, and a hang is a worse bug than a wrong room.
        for _ in 0..64 {
            let Some((d, here)) = self.dungeon else { return lines };
            if self.at_points || !self.has_cleared(d.id, here) {
                return lines;
            }
            let open: Vec<usize> = d.floors[here]
                .exits
                .iter()
                .map(|e| e.to)
                .filter(|&to| d.fights_ahead(to, &self.cleared_floors) > 0)
                .collect();
            match open.len() {
                // Every road out of here is walked out. Reaching a buffer stop
                // is what ends a dungeon, so this is unreachable while a
                // destination fires once a run - but the type allows it and a
                // run standing in a room it has already emptied would fight
                // the thing it beat.
                0 => {
                    self.dungeon = None;
                    lines.push(format!("Walked out of {} - nothing left in it.", d.name));
                    return lines;
                }
                // One road with a fight left throws itself: there is no
                // decision in a lever with one position.
                1 => {
                    lines.push(format!("Walked through: {} - cleared", d.floors[here].creature));
                    self.dungeon = Some((d, open[0]));
                }
                // Two roads still worth walking. Not this function's to decide.
                _ => {
                    self.at_points = true;
                    return lines;
                }
            }
        }
        lines
    }

    /// Throw the points, and go the way you said.
    ///
    /// Refused unless you are standing at them. Recorded in `took_exits`, so a
    /// replay of the same script makes the same walk and the map can draw the
    /// road taken rather than the roads that were there.
    pub fn throw_points(&mut self, exit: usize) -> bool {
        let Some((d, here)) = self.dungeon else { return false };
        if !self.at_points {
            return false;
        }
        let Some(e) = d.floors[here].exits.get(exit) else { return false };
        self.at_points = false;
        self.took_exits.push((d.id, here, exit));
        self.dungeon = Some((d, e.to));
        let mut lines = vec![format!("The points are thrown: {}", e.label)];
        // A thrown lever can land you on a road you have already walked.
        lines.extend(self.walk_through_cleared());
        self.last_receipt = Some(lines);
        true
    }

    /// Walk out of a dungeon without finishing it.
    ///
    /// The flee this game did not have, and it is deliberately not a flee from
    /// a *fight*: legal at a landing or at the points, never mid-fight, because
    /// a fight you can stop is a fight whose outcome depends on when you
    /// stopped it and the oracle would stop being one.
    ///
    /// It costs no life, no knock-back and nothing against `losses`, and what
    /// was cleared stays cleared. What it costs is the line: the door does not
    /// reopen, because the event that opened it is answered. Leaving that could
    /// be undone by walking back in would make a set of points free to sample,
    /// and a free sample is not a decision.
    ///
    /// Why it exists at all, when six dungeons lived without it: a three-floor
    /// dungeon whose last floor you cannot beat costs one life to learn. A
    /// four-deep one with points costs a life to learn *per branch*, and a
    /// branch you cannot see before you throw the lever is a fight you did not
    /// choose - which `dungeon.rs` says is the one kind this game must never
    /// hand out.
    pub fn leave_dungeon(&mut self) -> bool {
        if self.phase != Phase::Loadout {
            return false;
        }
        let Some((d, _)) = self.dungeon else { return false };
        self.back_up_a_dungeon();
        let mut lines = vec![format!("Left {}. What you cleared stays cleared.", d.name)];
        if let Some((outer, _)) = self.dungeon {
            lines.push(format!("You are still in {}.", outer.name));
        }
        self.last_receipt = Some(lines);
        true
    }

    // --------------------------------------------------------- THE HUNDRED

    /// The seed the county is derived from.
    ///
    /// A1's formula, and the whole of the reason `run_seed` is kept: the
    /// county must be re-derivable without a draw from `rng`, which stocks
    /// shops, rolls drops and melts pieces. One draw from that stream here
    /// would move every one of them and break every replay in the suite.
    pub fn county_seed(&self) -> u64 {
        crate::county::seed_for(self.run_seed, self.mode, self.difficulty)
    }

    /// The county, derived. Never stored, and the same one every time.
    ///
    /// Roughly 80 microseconds in release and half a millisecond in debug, so
    /// this is a call and not a field: a cache would be a second copy of a
    /// fact and the fact is one line of arithmetic away. A caller drawing it
    /// every frame should hold its own; nothing in the engine does.
    pub fn county(&self) -> crate::county::County {
        self.county_written().as_seen(self.sightings())
    }

    /// The county as the tables wrote it, hill and all.
    ///
    /// Everything that draws, walks or resolves wants `county()`, which hides
    /// the hill until three sightings are taken. This is for the two things
    /// that need the truth: the sighting lines themselves, and the check that
    /// the hill is where the arithmetic says.
    pub fn county_written(&self) -> crate::county::County {
        crate::county::generate(self.county_seed())
    }

    /// How many trig points have been cleared: nought to three.
    ///
    /// Derived from `county_cleared` and the table, per A2.2 - "sightings
    /// taken, sign tiles read and pale lines met are all derivable, and
    /// nothing extra is stored".
    pub fn sightings(&self) -> usize {
        let c = self.county_written();
        c.objectives(crate::county::Chain::Ordnance)
            .iter()
            .filter(|p| self.county_cleared.contains(p))
            .count()
    }

    /// How many sign tiles have been read, which is what teaches the Drover.
    pub fn signs_read(&self) -> usize {
        let c = self.county_written();
        c.objectives(crate::county::Chain::Drove)
            .iter()
            .filter(|p| self.county_cleared.contains(p))
            .count()
    }

    /// How many boundary stones have been read.
    pub fn stones_read(&self) -> usize {
        let c = self.county_written();
        c.objectives(crate::county::Chain::Enclosure)
            .iter()
            .filter(|p| self.county_cleared.contains(p))
            .count()
    }

    /// Whether a chain's pinnacle may be fought.
    ///
    /// B1-B3's gates, in one place. The Ordnance wants all three sightings -
    /// which is what makes the hill exist at all. The Drove wants a sign, so
    /// that a run that has never been taught to look cannot intercept by
    /// accident. The Enclosure wants the pale answered, because its pinnacle
    /// is behind it.
    pub fn county_gate_met(&self, chain: crate::county::Chain) -> bool {
        match chain {
            crate::county::Chain::Ordnance => self.sightings() >= 3,
            crate::county::Chain::Drove => self.signs_read() >= 1,
            crate::county::Chain::Enclosure => self.pale_is_open(),
        }
    }

    /// Whether the pale has been answered and the far corner is open.
    ///
    /// Shut for the whole of F2 through F7, which is what makes the three
    /// sealed tiles unenterable. A method rather than a flag test at its one
    /// call site, so that F8 - which authors the choice that sets it - has one
    /// place to change and not two.
    pub fn pale_is_open(&self) -> bool {
        self.flags.contains(&crate::county::PALE_OPEN)
    }

    /// The six figures this board pays a toll with.
    ///
    /// Over assembled items only, and read fresh: a player who walks up to a
    /// river, goes back to the loadout screen and builds a mana item has
    /// changed the answer, which is the whole point of a toll being a
    /// measurement rather than a key.
    pub fn county_figures(&self) -> crate::loadout::Figures {
        crate::loadout::Figures::of(&self.combat_items())
    }

    /// Whether this tile's threshold can be read from where you are standing.
    ///
    /// **One tile away and not before.** A county you can read from the mouth
    /// is a county you plan on paper; a county you can read one tile at a time
    /// is one you walk. The Surveyor's sheet (B1, F8) is the thing that turns
    /// the first into the second, and it is a reward for that reason.
    pub fn county_threshold_known(&self, at: (u8, u8)) -> bool {
        if self.county_is_cleared(at) || self.holds_the_surveyors_sheet() {
            return true;
        }
        self.county_at.is_some_and(|here| crate::county::manhattan(here, at) <= 1)
    }

    /// Whether the road has told this run what a chain is looking for.
    ///
    /// The on-ramps' whole payload. A run that has not met THE THEODOLITE can
    /// walk over a trig point and clear it - the chain is not gated on
    /// knowing - but the map draws it as a stone in a field, because that is
    /// what it is to somebody nobody has explained it to.
    pub fn knows_the_chain(&self, chain: crate::county::Chain) -> bool {
        self.flags.contains(&crate::county::chain_known(chain))
    }

    /// Whether a chain of THE HUNDRED has been finished: its pinnacle beaten.
    ///
    /// False for all three until F8, which is the milestone that can beat one.
    /// A method rather than three flag tests, for `pale_is_open`'s reason.
    pub fn county_chain_done(&self, chain: crate::county::Chain) -> bool {
        self.flags.contains(&crate::county::chain_done(chain))
    }

    /// Whether the Ordnance has paid out its sheet, which shows every
    /// threshold from anywhere.
    ///
    /// Always false until F8 authors the chain that sets the flag. A method
    /// rather than a flag test at its call site, for `pale_is_open`'s reason.
    pub fn holds_the_surveyors_sheet(&self) -> bool {
        self.flags.contains(&crate::county::THE_SHEET)
    }

    /// Whether a tile has been cleared this run.
    pub fn county_is_cleared(&self, at: (u8, u8)) -> bool {
        self.county_cleared.contains(&at)
    }

    /// How many tiles of a region have been cleared. The pale's checklist.
    pub fn county_cleared_in(&self, region: crate::county::Region) -> usize {
        self.county_cleared
            .iter()
            .filter(|p| crate::county::Region::of_row(p.1) == region)
            .count()
    }

    /// Which tile the Drover is standing on.
    ///
    /// It was always walking; a sign tile is what teaches a player to look.
    /// The clock is `events_resolved`, so a run one tile short of an
    /// interception can go up to the road, answer a door and come back.
    pub fn drover_tile(&self) -> (u8, u8) {
        crate::county::CIRCUIT[self.events_resolved as usize % crate::county::CIRCUIT.len()]
    }

    /// B5. All three chains done, and the tenth trip is granted.
    ///
    /// A **route, not a destination**: every move must land on an edge tile,
    /// always the same way round, and the fifth edge tile reached is where THE
    /// PARISH stands. Any illegal move breaks the walk and the trip is spent -
    /// which is the whole of what makes it a perambulation rather than a
    /// tenth ordinary trip.
    pub fn perambulation_is_granted(&self) -> bool {
        crate::county::Chain::ALL.iter().all(|c| self.county_chain_done(*c))
            && !self.county_trips.contains(&TripSource::Perambulation)
            && self.county_trips.len() < trip_cap()
    }

    /// Whether this move is legal on a perambulation.
    ///
    /// Three rules and they are all about the boundary: the tile must be on
    /// the edge, it must be the next one round, and "round" is whichever way
    /// the first move went.
    pub fn perambulation_allows(&self, to: (u8, u8)) -> bool {
        if !self.on_a_perambulation() {
            return true;
        }
        if !crate::county::on_edge(to) {
            return false;
        }
        let Some(here) = self.county_at else { return false };
        match self.perambulation_way {
            None => true,
            Some(clockwise) => crate::county::next_round(here, clockwise) == Some(to),
        }
    }

    /// Whether the trip being walked is the perambulation.
    pub fn on_a_perambulation(&self) -> bool {
        self.county_at.is_some()
            && self.county_trips.last() == Some(&TripSource::Perambulation)
    }

    /// The pale's checklist, ticked live (B3.1).
    ///
    /// Five lines, each a `Requirement` answered by the same machinery a
    /// choice's `requires` uses - so the checklist a player reads at one tile
    /// and the gate that opens are the same question asked twice rather than
    /// two questions kept in step.
    pub fn pale_checklist(&self) -> Vec<(crate::event::Requirement, bool)> {
        use crate::county::Region;
        use crate::event::Requirement;
        let mut out: Vec<Requirement> = Region::ALL
            .iter()
            .map(|r| Requirement::CountyTiles { region: *r, at_least: 6 })
            .collect();
        out.push(Requirement::Counter { what: "boundary-stones", at_least: 2 });
        out.push(Requirement::HoldingOrb);
        out.into_iter()
            .map(|r| {
                let met = self.requirement_met(r);
                (r, met)
            })
            .collect()
    }

    /// Whether the four lines that are read *before* the gate are all ticked.
    ///
    /// The fifth - an orb surrendered - is met at the gate itself, because
    /// what it asks for is not a state of the run but a thing handed over.
    pub fn pale_is_ready(&self) -> bool {
        self.pale_checklist().iter().take(4).all(|(_, met)| *met)
    }

    /// Whether the Drover is standing where you are.
    ///
    /// Only once a sign has been read: it was always walking, and a sign tile
    /// is what teaches a player to look. A run that has never been taught
    /// cannot intercept by accident.
    pub fn drover_is_here(&self) -> bool {
        self.signs_read() >= 1
            && !self.county_chain_done(crate::county::Chain::Drove)
            && self.county_at == Some(self.drover_tile())
    }

    /// The pursuit, ended.
    ///
    /// A brawl, because a drover without a herd is a man on a walk. It is the
    /// Drove's pinnacle by another road - the same party, the same settling -
    /// so it goes through `county_pinnacle` rather than through `brawl`.
    fn intercept_the_drover(&mut self) {
        self.county_pinnacle = Some(crate::county::Chain::Drove);
        self.begin_county_fight();
    }

    /// Whether this source has already been spent.
    pub fn county_trip_taken(&self, from: TripSource) -> bool {
        self.county_trips.contains(&from)
    }

    /// C1. Taken down, and put on the gaol rather than at a gate.
    ///
    /// **The fastest ride into the middle there is**, and that is deliberate:
    /// V9 keeps the gaol within three of the centre and every mouth is on an
    /// edge, so a player will fail tolls on purpose to be sent down. That is
    /// allowed to work - a punishment a clever player farms beats one a
    /// careful player avoids - and what it actually costs is census slot nine.
    ///
    /// The one entry that does not start at a mouth, which is why it is its
    /// own function rather than a `TripSource` handed to `enter_county`.
    pub fn arrested_into_the_county(&mut self) -> bool {
        if self.phase != Phase::Loadout || self.county_at.is_some() {
            return false;
        }
        if self.county_trips.contains(&TripSource::Constable)
            || self.county_trips.len() >= trip_cap()
        {
            return false;
        }
        let Some(gaol) = self.county_written().gaol() else { return false };
        self.county_trips.push(TripSource::Constable);
        self.county_at = Some(gaol);
        self.county_moves_left = crate::county::MOVES_A_TRIP;
        self.county_entry_cleared = self.county_cleared.len();
        self.perambulation_way = None;
        self.perambulation_reached = 0;
        self.flags.retain(|f| *f != COUNTY_BUSINESS);
        let mut lines = vec![format!("Taken down to {}", crate::county::reference(gaol))];
        if let Some(said) = self.resolve_county_tile(gaol) {
            lines.push(said);
        }
        lines.push(moves_left(self.county_moves_left));
        self.last_receipt = Some(lines);
        true
    }

    /// B5. The tenth trip, granted rather than taken.
    ///
    /// Enter at any mouth. Every move must land on an edge tile, always the
    /// same way round; the fifth is THE PARISH.
    pub fn walk_the_perambulation(&mut self, mouth: (u8, u8)) -> bool {
        if !self.perambulation_is_granted() || !crate::county::on_edge(mouth) {
            return false;
        }
        self.enter_county(TripSource::Perambulation, mouth)
    }

    /// Down into THE HUNDRED, five moves, from a mouth.
    ///
    /// Refused when the source is already spent, when the census is full, when
    /// a trip is already running, or when the fight screen is up. Arriving on
    /// the mouth's own tile is **free** and resolves it, which is what makes
    /// five moves five tiles of county rather than four.
    pub fn enter_county(&mut self, from: TripSource, mouth: (u8, u8)) -> bool {
        if self.phase != Phase::Loadout || self.county_at.is_some() {
            return false;
        }
        if self.county_trips.contains(&from) || self.county_trips.len() >= trip_cap() {
            return false;
        }
        if !crate::county::is_mouth(mouth) {
            return false;
        }
        self.county_trips.push(from);
        self.county_at = Some(mouth);
        self.county_moves_left = crate::county::MOVES_A_TRIP;
        self.county_entry_cleared = self.county_cleared.len();
        self.perambulation_way = None;
        self.perambulation_reached = 0;
        let mut lines = vec![format!("Down into THE HUNDRED at {}", crate::county::reference(mouth))];
        if let Some(said) = self.resolve_county_tile(mouth) {
            lines.push(said);
        }
        lines.push(moves_left(self.county_moves_left));
        self.last_receipt = Some(lines);
        true
    }

    /// One move: north, south, east or west.
    ///
    /// A2.1 in order. The three ways a move ends without moving you - the edge
    /// of the county, a fence you have not opened, a toll you cannot pay -
    /// **still cost the move**, because a move is a thing you spend trying
    /// rather than a thing you spend arriving. Only the edge is free, and that
    /// is because walking into the edge of a map is not an attempt at
    /// anything.
    pub fn county_walk(&mut self, dir: crate::county::Step) -> bool {
        if self.phase != Phase::Loadout || self.pending_event().is_some() {
            return false;
        }
        let Some(here) = self.county_at else { return false };
        if self.county_moves_left == 0 {
            return false;
        }
        let Some(to) = dir.from(here) else {
            self.last_receipt = Some(vec!["That is the edge of the county".into()]);
            return false;
        };
        let county = self.county();
        // The far corner is behind a pale nobody has opened. It costs the move
        // for the reason a failed toll does: you went and looked.
        if county.is_sealed(to) && !self.pale_is_open() {
            self.county_moves_left -= 1;
            let mut lines = vec![format!("{} is behind the pale", crate::county::reference(to))];
            if self.on_a_perambulation() {
                self.county_moves_left = 0;
                lines.push("The perambulation is broken on a fence".into());
                self.last_receipt = Some(lines);
                self.close_the_trip();
                return false;
            }
            lines.push(moves_left(self.county_moves_left));
            self.last_receipt = Some(lines);
            self.end_county_trip_if_spent();
            return false;
        }
        // B5. On a perambulation the boundary is the road, and stepping off
        // it - or the wrong way round it - **breaks the walk**. The trip is
        // spent, which is the price of a route rather than a destination.
        if self.on_a_perambulation() && !self.perambulation_allows(to) {
            self.county_moves_left = 0;
            self.last_receipt = Some(vec![
                format!("{} is not on the boundary", crate::county::reference(to)),
                "The perambulation is broken, and a broken one is walked".into(),
            ]);
            self.close_the_trip();
            return false;
        }

        // A2.1 step 2. A Feature you have already crossed is a bridge you paid
        // for once, so this asks only of a tile that is not yet cleared.
        if let crate::county::TileKind::Feature(toll) = county.at(to).kind {
            if !self.county_is_cleared(to) {
                let figures = self.county_figures();
                let bounty = self.rung_bounty();
                if !toll.met(&figures, self.gold, bounty) {
                    self.county_moves_left -= 1;
                    let mut lines = vec![
                        format!(
                            "{} - {} - no",
                            crate::county::reference(to),
                            county.at(to).kind.what()
                        ),
                        toll.shortfall(&figures, self.gold, bounty),
                    ];
                    // B5: "tolls on the boundary must be paid; any illegal
                    // move, **or a failed toll**, breaks the walk". A
                    // perambulation is a route rather than a destination, and
                    // a route you cannot finish is not one you get to retry
                    // from where you stopped.
                    if self.on_a_perambulation() {
                        self.county_moves_left = 0;
                        lines.push("The perambulation is broken on a toll it could not pay".into());
                        self.last_receipt = Some(lines);
                        self.close_the_trip();
                        return false;
                    }
                    lines.push(moves_left(self.county_moves_left));
                    self.last_receipt = Some(lines);
                    self.end_county_trip_if_spent();
                    return false;
                }
                // The gate is the only one that takes anything, and it takes
                // it once - the tile is cleared by the crossing.
                self.gold -= toll.toll_in_gold(bounty);
            }
        }
        self.county_moves_left -= 1;
        let was = self.county_at;
        self.county_at = Some(to);
        if self.on_a_perambulation() {
            if self.perambulation_way.is_none() {
                self.perambulation_way =
                    was.and_then(|h| crate::county::next_round(h, true).map(|n| n == to));
            }
            self.perambulation_reached += 1;
            if self.perambulation_reached >= crate::county::PARISH_AT {
                self.county_pinnacle = None;
                self.last_receipt = Some(vec![
                    format!("{} - and it is the fifth", crate::county::reference(to)),
                    "THE PARISH".into(),
                ]);
                self.begin_parish();
                return true;
            }
        }
        let mut lines = vec![format!(
            "{} - {}",
            crate::county::reference(to),
            county.at(to).kind.what()
        )];
        if let Some(said) = self.resolve_county_tile(to) {
            lines.push(said);
        }
        lines.push(moves_left(self.county_moves_left));
        self.last_receipt = Some(lines);
        if self.drover_is_here() {
            self.intercept_the_drover();
            return true;
        }
        self.end_county_trip_if_spent();
        true
    }

    /// Resolve the tile you have just arrived on, and say what happened.
    ///
    /// A cleared tile resolves nothing and says so - crossing one is a walk
    /// and not a second visit. Everything else is marked cleared.
    ///
    /// **Every kind clears at F2.** The kind-specific arms are the milestones
    /// that own them: an Event tile sets a pending county event (F7), an
    /// Objective pays its chain and a Pinnacle asks whether its gate is met
    /// before it will start a fight (F8). Written as one arm rather than six
    /// identical ones so that the milestone which arms them finds a place to
    /// put the code rather than a shape to imitate.
    fn resolve_county_tile(&mut self, at: (u8, u8)) -> Option<String> {
        use crate::county::{Chain, TileKind};
        let kind = self.county().at(at).kind;

        // A pinnacle whose gate is unmet says so and is not cleared. This is
        // asked **before** the cleared check, because the hill is the one tile
        // in the game that can be cleared and then stop being cleared: a run
        // that walked over it while it still looked empty cleared an empty
        // tile, and the third sighting makes that tile a pinnacle. A cleared
        // tile that becomes a pinnacle is uncleared by the becoming (B1.1).
        if let TileKind::Pinnacle { chain } = kind {
            if self.county_cleared.contains(&at) {
                self.county_cleared.retain(|p| *p != at);
            }
            if !self.county_gate_met(chain) {
                return Some(match chain {
                    Chain::Ordnance => "the hill, and you have not taken every sighting".into(),
                    Chain::Drove => "nothing here has taught you to look yet".into(),
                    Chain::Enclosure => "the pale is not open".into(),
                });
            }
            self.county_pinnacle = Some(chain);
            self.begin_county_fight();
            return Some(format!("{:?} - the end of it", chain).to_uppercase());
        }

        if self.county_cleared.contains(&at) {
            return Some("walked over, and already yours".into());
        }

        // An objective pays its chain. The Enclosure's stones count, because
        // the pale's fourth line reads a tally rather than a flag - two of
        // three, and the third is behind the gate the tally opens, which is
        // the chain's own joke.
        if let TileKind::Objective { chain, nth } = kind {
            self.county_cleared.push(at);
            if chain == Chain::Enclosure {
                self.count("boundary-stones");
            }
            return Some(match chain {
                Chain::Ordnance => {
                    let taken = self.sightings();
                    match taken {
                        3 => "the third sighting. Two lines were knowledge; this one is a key"
                            .to_string(),
                        n => format!("sighting {n} of 3, and the line is drawn"),
                    }
                }
                Chain::Drove => format!(
                    "a sign, and it says what came through: {} events ago the herd was here",
                    self.events_resolved
                ),
                Chain::Enclosure => format!("boundary stone {nth}, and it is cut by the same hand"),
            });
        }

        // An Event tile asks its question instead of clearing. Answering is
        // what clears it, in `take_choice_unchecked`, and a tile whose event
        // has nothing to ask - a word it needs and you have not got - clears
        // like any other rather than standing there refusing.
        if let crate::county::TileKind::Event(id) = self.county().at(at).kind {
            if let Some(ev) = crate::event::county_event(id) {
                // Through the theme, at the source. A receipt is prose the
                // run hands to whatever is drawing, and two interfaces would
                // otherwise have to remember to translate it twice - which is
                // the reason `Settlement::landing` is themed here too.
                let title = self.theme.place(ev.id, ev.title);
                if ev.choices.iter().any(|c| self.choice_open(c)) {
                    self.county_event = Some(id);
                    return Some(title.to_string());
                }
                self.county_cleared.push(at);
                return Some(format!("{title} - and nothing you have to say to it"));
            }
        }
        self.county_cleared.push(at);
        None
    }

    /// Out of the county when the moves run out.
    ///
    /// Moves never bank. You are put back at the town entryway you came in by,
    /// which is where `self.town` still is, because the county door does not
    /// cost the visit.
    fn end_county_trip_if_spent(&mut self) {
        if self.county_moves_left == 0 && self.pending_event().is_none() {
            self.close_the_trip();
        }
    }

    /// A trip is over. Whether it was worth taking is a question C1 asks.
    ///
    /// **The flag the constable reads.** A trip that cleared nothing is a run
    /// that went down there and came back with nothing to show, which is not
    /// against anything and is the sort of thing that gets looked into. Set by
    /// the engine rather than by a choice, which is why
    /// `completable::ENGINE_SETS` names it: a lint that walks `EVENTS` looking
    /// for the outcome that sets a flag cannot see a flag the rules set.
    fn close_the_trip(&mut self) {
        if self.county_cleared.len() == self.county_entry_cleared
            && !self.flags.contains(&COUNTY_BUSINESS)
        {
            self.flags.push(COUNTY_BUSINESS);
        }
        self.county_at = None;
    }

    /// C2. An empty grid past rung sixteen is somebody's business.
    ///
    /// Checked after a won fight. Fires **once** - either it is declined for
    /// ever, or the bet is taken and settles itself - and it is pushed through
    /// `forced_event` rather than standing on a rung, because what it is about
    /// is a board rather than a place.
    ///
    /// `phase_two::every_door_in_the_game_can_be_arrived_at` knows about three
    /// ways a door gets pushed now: a pedestal, the end of the road, and this.
    pub const WASTE_FROM: usize = 15;

    fn look_at_the_waste(&mut self) {
        // On the road, and only on the road. Vessey stands at the roadside
        // with a legal opinion about your greaves; he is not in a dungeon and
        // he is not down a county. Without this he arrives on the landing
        // between two floors of the Switchyard and blocks the points, which is
        // how three of that mission's tests found him.
        if self.rung <= Self::WASTE_FROM
            || self.dungeon.is_some()
            || self.county_at.is_some()
            || self.waste_bet.is_some()
            || self.flags.contains(&"waste-improved")
            || self.flags.contains(&"waste-declined")
            || self.waste_offered
            || self.answered.contains(&"the-waste")
        {
            return;
        }
        let empty = SlotKind::ALL
            .into_iter()
            .find(|k| self.report(*k).items.iter().all(|i| !i.assembled));
        if empty.is_some() {
            self.waste_offered = true;
        }
    }

    /// The bet, settled. Empty at the deadline pays a trip; filled owes gold.
    fn settle_the_waste(&mut self) {
        let Some((grid, deadline)) = self.waste_bet else { return };
        let still_empty = self.report(grid).items.iter().all(|i| !i.assembled);
        if !still_empty {
            let owed = self.rung_bounty();
            self.gold = (self.gold - owed).max(0);
            self.waste_bet = None;
            self.last_receipt = Some(vec![
                format!("{} is not waste any more, and Vessey was watching", grid.name()),
                format!("-{owed}g"),
            ]);
            return;
        }
        if self.rung >= deadline {
            self.waste_bet = None;
            if self.county_trips.contains(&TripSource::WasteBet)
                || self.county_trips.len() >= trip_cap()
            {
                self.gold += self.rung_bounty() * 2;
                self.last_receipt =
                    Some(vec!["Vessey pays up, and in coin".into()]);
            } else {
                self.flags.push("waste-bet-won");
                self.last_receipt = Some(vec![
                    format!("{} stayed waste, and Vessey pays what he said", grid.name()),
                    "A way down, whenever you want it".into(),
                ]);
            }
        }
    }

    /// What a county loss costs: a Rogue life, or a Grinder rung.
    ///
    /// A7 - "a county loss costs what a road loss costs". Written out here
    /// rather than falling through `settle`'s own arm because that arm also
    /// moves the rung, and a pinnacle is not a rung: a Grinder knocked back
    /// off a hill would lose ladder progress for something that never
    /// advanced it.
    fn spend_a_life_for_the_county(&mut self) {
        match self.mode {
            Mode::Grinder => {
                if self.rung > 0 {
                    self.rung -= 1;
                }
            }
            Mode::Rogue => {
                self.lives = self.lives.saturating_sub(1);
            }
        }
    }

    /// Walk out early. Free, and it forfeits the moves.
    ///
    /// Leaving is a decision with a price on one side only: what you cleared
    /// stays cleared and the trip is spent either way, so the price is the
    /// moves you did not take.
    pub fn leave_county(&mut self) -> bool {
        if self.phase != Phase::Loadout || self.county_at.is_none() {
            return false;
        }
        let left = self.county_moves_left;
        self.close_the_trip();
        self.county_moves_left = 0;
        self.last_receipt = Some(vec![
            "Back up out of THE HUNDRED. What you cleared stays cleared.".into(),
            format!("{left} move{} forfeited", if left == 1 { "" } else { "s" }),
        ]);
        true
    }

    /// Feed a pedestal an orb, and go where the orb goes.
    ///
    /// The orb is consumed and the destination fires **once per run**, however
    /// many pedestals a run finds - there are two of them and they share one
    /// visited-set, because the second exists so that a player whose orbs
    /// arrived late still gets to spend them, not so that a patient one gets
    /// to spend them twice.
    ///
    /// A duplicate orb is refused and stays what it was, which is a working
    /// weapon: an orb is a piece first and a ticket second.
    pub fn feed_pedestal(&mut self, id: PieceId) -> Option<&'static crate::pedestal::Destination> {
        if !self.owned.contains(&id) {
            return None;
        }
        let name = self.registry.def(id).name;
        let dest = crate::pedestal::by_orb(name)?;
        if self.destinations_visited.contains(&dest.id) {
            return None;
        }
        self.destinations_visited.push(dest.id);
        self.loadout.remove_anywhere(id);
        self.owned.retain(|&o| o != id);
        self.forget_undo();
        match dest.kind {
            crate::pedestal::Where::Dungeon(d) => self.enter_dungeon(d),
            crate::pedestal::Where::Siding { dungeon, floor } => {
                if let Some(d) = crate::dungeon::by_id(dungeon) {
                    self.enter_dungeon_at(d, floor);
                }
            }
            crate::pedestal::Where::Event(e) => self.forced_event = Some(e),
            // Any mouth, found or not. `enter_county` refuses a mouth that is
            // not one and refuses a source already spent, and the interface
            // picks which - `county::MOUTHS[0]` is the fallback for a caller
            // that does not ask, which is the CLI and nothing else.
            crate::pedestal::Where::County => {
                let mouth = self.county_mouth_wanted.take().unwrap_or(crate::county::MOUTHS[0].1);
                self.enter_county(TripSource::SurveyorsOrb, mouth);
            }
        }
        self.last_receipt = Some(vec![
            format!("Fed the pedestal: {}", name),
            format!("It takes you to {}", dest.name),
        ]);
        Some(dest)
    }

    /// Break a one-cell unique, and get the one thing it is for.
    ///
    /// The only things in this game that are *spent*. Everything else you own
    /// is either worn or sold; a crushable is destroyed by using it, which is
    /// what makes carrying one a decision about when rather than whether.
    ///
    /// Returns what it did, or `None` if that piece is not one of them.
    pub fn crush(&mut self, id: PieceId) -> Option<crate::relic::Crush> {
        use crate::relic::Crush;
        if !self.owned.contains(&id) {
            return None;
        }
        let name = self.registry.def(id).name;
        let what = crate::relic::crushable(name)?.what;
        let mut receipt = vec![format!("Crushed: {}", name)];
        match what {
            // The one legal breach of the one-action rule, and it is legal
            // because it costs you the key.
            Crush::SecondKey => {
                if self.town.is_none() {
                    return None;
                }
                self.second_key_ready = true;
                receipt.push("One more door, in this town".into());
            }
            // A door you walked away from, standing open again. The most
            // recent one, because that is the one you are still thinking
            // about.
            Crush::Appeal => {
                let Some(back) = self.answered.pop() else { return None };
                self.took.retain(|_| true);
                receipt.push(format!("They will hear you again: {}", back));
            }
            Crush::SkipStone => {
                if self.rung + 1 > LADDER.len() {
                    return None;
                }
                receipt.push(format!("Stepped over {}", LADDER[self.rung].name));
                receipt.push("It pays nothing, and it is behind you".into());
                self.rung += 1;
                self.best_rung = self.best_rung.max(self.rung);
            }
        }
        self.loadout.remove_anywhere(id);
        self.owned.retain(|&o| o != id);
        self.forget_undo();
        self.last_receipt = Some(receipt);
        Some(what)
    }

    /// Put a named component in the tray, if there is such a thing.
    pub fn give(&mut self, name: &str) -> Option<PieceId> {
        let d = CATALOG.iter().position(|d| d.name == name)?;
        let id = self.registry.alloc(d);
        self.owned.push(id);
        Some(id)
    }

    /// Take a passenger aboard, for `rungs` rungs.
    ///
    /// It has to be somewhere on a board: the rent is dead cells, and a
    /// passenger riding in the tray is paying nothing.
    pub fn take_passenger(&mut self, id: PieceId, rungs: usize) -> bool {
        if !self.owned.contains(&id) || self.passenger.is_some() {
            return false;
        }
        self.passenger = Some((id, self.rung + rungs));
        true
    }

    /// Is the passenger where it is supposed to be - on a board, not in a bag?
    pub fn passenger_is_seated(&self) -> bool {
        let Some((id, _)) = self.passenger else { return false };
        SlotKind::ALL.iter().any(|&k| self.loadout.slot(k).pieces().contains(&id))
    }

    /// Deliver it, if the road has gone far enough.
    pub fn deliver_passenger(&mut self) -> bool {
        let Some((id, until)) = self.passenger else { return false };
        if self.rung < until {
            return false;
        }
        self.passenger = None;
        self.loadout.remove_anywhere(id);
        self.owned.retain(|&o| o != id);
        self.forget_undo();
        self.last_receipt = Some(vec!["Delivered, and in one piece".into()]);
        true
    }

    /// Throw a piece into the melt and take back what comes out.
    ///
    /// A same-slot piece within `MELT_SPREAD` of what went in, drawn from the
    /// run's own PRNG. Never combat, so determinism holds: two replays of a
    /// seed melt the same thing into the same thing. Quest pieces and rumours
    /// refuse the pot - one is the far side of a task and the other is a key,
    /// and neither is gear in the sense a crucible understands.
    ///
    /// Returns the new piece, or `None` if it refused. Counts the melt
    /// whichever way it goes, because the foundry is counting visits and not
    /// successes.
    pub fn melt(&mut self, id: PieceId) -> Option<PieceId> {
        if self.phase != Phase::Loadout || !self.owned.contains(&id) {
            return None;
        }
        let def = self.registry.def(id);
        if crate::rumour::is_rumour(def.name)
            || self.quest_of(id).is_some()
            || crate::piece::is_quest_reward(def.name)
            || crate::piece::is_boss_only(def.name)
            || crate::piece::is_event_only(def.name)
        {
            return None;
        }
        let was = crate::rating::piece_rating(def);
        let slot = def.slot;
        let pool: Vec<usize> = crate::piece::all_def_indices()
            .into_iter()
            .filter(|&i| {
                let d = &CATALOG[i];
                d.slot == slot
                    && d.name != def.name
                    && (crate::rating::piece_rating(d) - was).abs() <= MELT_SPREAD
                    && !crate::piece::is_boss_only(d.name)
                    && !crate::piece::is_quest_reward(d.name)
                    && !crate::piece::is_event_only(d.name)
                    && (self.insight_unlocked || !crate::piece::touches_insight(d))
            })
            .collect();
        self.count("crucible-melts");
        let &pick = pool.get(self.rng.below(pool.len().max(1)))?;
        self.loadout.remove_anywhere(id);
        self.registry.transform(id, pick);
        self.quest_progress.remove(&id);
        self.granted_quests.remove(&id);
        self.forget_undo();
        self.last_receipt = Some(vec![
            format!("Into the melt: {}", def.name),
            format!("Out of it: {}", CATALOG[pick].name),
        ]);
        Some(id)
    }

    /// Is the road past Francis open?
    ///
    /// Rung 51 is not on the ladder and never will be: it appears when a run
    /// has finished the chain and put the man at the top down, and it is the
    /// only rung in the game that has to be earned twice.
    ///
    /// Dark until the chain exists. `THE_UNWOUND` names nothing yet, so this
    /// is false for every run that can currently be played, and the plumbing
    /// underneath it - share codes, scoring, the road stack - is being asked
    /// to accept a rung that does not arrive.
    pub fn past_the_top(&self) -> bool {
        // Exactly at the end of the ladder, not past it. Beating THE UNWOUND
        // moves the rung on again, and a run that has done that is not still
        // standing in front of it - it is finished, which is what
        // `ladder_complete` is for.
        self.holds(MAINSPRING) && self.rung == LADDER.len()
    }

    /// Does this run own a named component, worn or loose?
    pub fn holds(&self, name: &str) -> bool {
        self.owned.iter().any(|&id| self.registry.def(id).name == name)
    }

    /// True once the ladder has been cleared.
    pub fn ladder_complete(&self) -> bool {
        self.rung >= LADDER.len() && !self.past_the_top()
    }

    /// What a shelf costs this run, which is not always what it is worth.
    ///
    /// The foundry noticing it was snubbed is the only thing that moves it,
    /// and it moves everything at once - which is what "prices run ten percent
    /// ahead" means when nobody will say why.
    pub fn price(&self, slot: usize) -> Option<i32> {
        self.shop.price(slot).map(|p| p + p * self.markup / 100)
    }

    /// Buy the component on shelf `slot`.
    pub fn buy(&mut self, slot: usize) -> Result<PieceId, RuleError> {
        if self.phase != Phase::Loadout {
            return Err(RuleError::LoadoutLocked);
        }
        let price = self.price(slot).ok_or(RuleError::NothingThere)?;
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
        self.restock(need);
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
        // On consignment: it does not leave, it goes away for a while and
        // comes back worth more. The order is a standing arrangement, so this
        // happens to everything sold while it is held.
        if self.standing_orders.contains(&crate::event::Standing::Consignment) {
            let def = self.registry.def_index(id);
            if let Some(better) = crate::piece::dearer_than(def, CONSIGNMENT_GAIN) {
                self.consigned.push((better, CONSIGNMENT_SHOPS));
            }
        }
        Ok(refund)
    }

    /// Turn the shelves over.
    ///
    /// One place, rather than the nine `shop.restock` used to be called from,
    /// because a shelf now has two things to do when it turns over and the
    /// second one - handing back what was left on consignment - is exactly the
    /// kind of thing that gets added to eight of nine call sites.
    fn restock(&mut self, ensure_weapon: bool) {
        self.shop.restock(&mut self.rng, ensure_weapon);
        self.tick_consignments();
    }

    /// Move every consignment one shop closer, and put back whatever has come.
    ///
    /// Called on the restock rather than on the sale, because "three shops
    /// later" is a fact about shops.
    fn tick_consignments(&mut self) {
        let mut arrived: Vec<usize> = Vec::new();
        for (def, left) in self.consigned.iter_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                arrived.push(*def);
            }
        }
        self.consigned.retain(|(_, left)| *left > 0);
        for def in arrived {
            self.shop.put_on_a_shelf(def);
        }
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

        // THE HUNDRED's endings settle before anything else, because a
        // pinnacle is not a rung: winning one must not advance the ladder and
        // losing one must not knock you back off it. The trip ends either way
        // and what the chain pays is paid here.
        if self.walking_the_parish {
            self.walking_the_parish = false;
            if outcome == Outcome::Victory {
                self.flags.push("the-parish-is-walked");
                self.last_receipt =
                    Some(vec!["THE PARISH is walked, and the county is finished".into()]);
            } else {
                self.last_receipt = Some(vec!["THE PARISH stands".into()]);
                self.spend_a_life_for_the_county();
            }
            self.county_at = None;
            self.county_moves_left = 0;
            self.phase = Phase::Loadout;
            return Some(0);
        }
        if let Some(chain) = self.county_pinnacle.take() {
            let mut lines: Vec<String> = Vec::new();
            if outcome == Outcome::Victory {
                self.flags.push(crate::county::chain_done(chain));
                if let Some(at) = self.county_at {
                    if !self.county_cleared.contains(&at) {
                        self.county_cleared.push(at);
                    }
                }
                lines.push(format!("{} is finished", chain.name()));
                for reward in crate::county::chain_pays(chain) {
                    self.give(reward);
                    lines.push(format!("Gained: {reward}"));
                }
                if chain == crate::county::Chain::Ordnance {
                    self.flags.push(crate::county::THE_SHEET);
                    lines.push("Every threshold in the county, from anywhere".into());
                }
                if crate::county::Chain::ALL.iter().all(|c| self.county_chain_done(*c))
                    && !self.county_trips.contains(&TripSource::Perambulation)
                    && self.county_trips.len() < trip_cap()
                {
                    lines.push("All three. The perambulation is yours to walk".into());
                }
            } else {
                lines.push(format!("{} stands", chain.name()));
            }
            // **Winning puts you back on the map.** It used to end the trip
            // either way, which meant a run that banked ten moves and spent
            // one reaching a chain forfeited the other nine for finishing it.
            //
            // Losing still ends it, because A7 says a loss costs what a road
            // loss costs and that is deliberate - the asymmetry is the point,
            // not an oversight. The tile is marked cleared above, so there is
            // nothing here to walk into twice.
            if outcome == Outcome::Victory {
                lines.push(moves_left(self.county_moves_left));
            } else {
                self.county_at = None;
                self.county_moves_left = 0;
            }
            self.last_receipt = Some(lines);
            // A loss still costs what a road loss costs, which the rest of
            // this function does - but the rung must not move either way, so
            // the ladder half is skipped by returning the bounty and nothing
            // else.
            if outcome != Outcome::Victory {
                self.spend_a_life_for_the_county();
            }
            self.phase = Phase::Loadout;
            return Some(0);
        }
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
                underwrote: None,
                lost_passenger: false,
                pried_off: Vec::new(),
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
            self.restock(need);
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
            underwrote: None,
            lost_passenger: false,
            pried_off: Vec::new(),
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
                self.cleared_floors.push((d.id, floor));
                // What this floor pays, before what the dungeon pays, so the
                // receipt reads buffer stop and then dungeon. Nearly always
                // empty in the middle of a road; a buffer stop is where a
                // graph puts its rewards, because which one you reached is the
                // whole of what a graph asks.
                let mut lines = Vec::new();
                for o in d.floors[floor].also {
                    lines.extend(self.apply_outcome(o, crate::event::Requirement::None).1);
                }
                // Through the theme, at the source. A landing is prose the
                // run hands to whatever is drawing, and the two interfaces
                // would otherwise have to remember to translate it twice.
                settlement.landing =
                    d.floors.get(floor).map(|f| self.theme.landing(d.id, floor, f.landing));
                self.pending_landing = settlement.landing;
                match d.floors[floor].exits.len() {
                    // A buffer stop: out the other side, with the thing you
                    // went in for - and with whatever else it pays, which for
                    // THE THRESHOLD is the pool and not the class.
                    0 => {
                        self.back_up_a_dungeon();
                        for o in d.also {
                            lines.extend(self.apply_outcome(o, crate::event::Requirement::None).1);
                        }
                        if let Some(c) = crate::class::CLASSES
                            .iter()
                            .filter(|_| !d.reward.is_empty())
                            .find(|c| c.name == d.reward)
                        {
                            if !self.classes.iter().any(|k| k.name == c.name) {
                                self.classes.push(c);
                                self.refresh_class_effects();
                                settlement.class_won = Some(c.name);
                            }
                        }
                    }
                    // One way on: the next room, which is every floor of every
                    // dungeon written before the yard.
                    1 => self.dungeon = Some((d, d.floors[floor].exits[0].to)),
                    // Points. Stay standing on the floor just cleared - the
                    // lever is out past it and nobody has pulled it.
                    _ => self.at_points = true,
                }
                lines.extend(self.walk_through_cleared());
                if !lines.is_empty() {
                    self.last_receipt = Some(lines);
                }
                self.restock(false);
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
                // Prospector: one more piece off it, and the only thing in
                // the game that changes what a corpse is worth. Off the
                // *board* rather than off `drops`, because `drops` is the one
                // trophy a creature owns and a boss is standing there wearing
                // fifteen items nobody can buy.
                let extra: usize = self
                    .effective_classes()
                    .iter()
                    .filter_map(|c| match c.power {
                        crate::class::ClassPower::Prospector(n) => Some(n),
                        _ => None,
                    })
                    .sum();
                if extra > 0 && spec.rank.is_named() {
                    let mut taken = 0;
                    for &(name, ..) in spec.gear {
                        if taken >= extra || self.inventory().len() >= INVENTORY_CAP {
                            break;
                        }
                        if crate::piece::CATALOG.iter().any(|d| d.name == name)
                            && self.give(name).is_some()
                        {
                            taken += 1;
                            settlement.pried_off.push(name);
                        }
                    }
                }
                if !spec.drops.is_empty() && self.inventory().len() < INVENTORY_CAP {
                    let pick = self.rng.below(spec.drops.len());
                    let name = spec.drops[pick];
                    if let Some(def) = crate::piece::CATALOG.iter().position(|d| d.name == name) {
                        let id = self.registry.alloc(def);
                        self.owned.push(id);
                        settlement.dropped = Some(name);
                    }
                }
                // The man at the top, counted rather than the rung, so the
                // road past him does not accidentally make him harder for a
                // run that walked down it. Counted before the rung moves,
                // because `spec` is what was actually fought.
                if spec.name == "Francis" {
                    self.francis_beaten = self.francis_beaten.saturating_add(1);
                }
                self.rung += 1;
                self.best_rung = self.best_rung.max(self.rung);
                // Whatever stood in for that rung is done standing in.
                self.substitute = None;
                // The road past the top, for a run that earned it.
                //
                // Rung 51 is not on the ladder and cannot be: the door is
                // pushed on rather than stood on, the way a pedestal's is.
                //
                // This used to ask for `looked-through-the-lens` instead of
                // the mainspring, on the reasoning that having *looked* is
                // what makes the door appear and the mainspring is what opens
                // it - you cannot miss what you never saw. That is a good
                // line about a hint and the wrong line about an ending. The
                // only thing that sets that flag is one choice in THROUGH THE
                // CRACKED LENS, which stands on exactly one rung and needs a
                // second collectible to take, so a run could finish the whole
                // chain, hold the mainspring, put Francis down and be told
                // nothing at all. Reported from play.
                //
                // The item is the key. The lens keeps what it is good at,
                // which is seeing the boards ahead - and it keeps one more
                // thing, because the old condition was carrying a second
                // idea worth keeping: a run that *looked* and then spent the
                // mainspring is still shown the door, shut, so it learns
                // what it missed. That is the VIP area's shape at the end of
                // the road and it is why this is an `||` rather than a
                // replacement. Either having earned it or having seen it
                // makes the door stand; only the mainspring opens it, which
                // the choice's own `Requirement` has always said.
                if self.rung == LADDER.len()
                    && (self.holds(MAINSPRING)
                        || self.flags.contains(&"looked-through-the-lens"))
                    && !self.answered.contains(&"the-unwound")
                {
                    self.forced_event = Some("the-unwound");
                }
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
                // Whatever stood in for that rung stops standing in.
                //
                // A substitute is a detour you chose - GO ROUND THE BACK puts
                // The Dreaming Idiot in front of you instead of the rung's own
                // creature - and it used to be cleared on a win and left alone
                // on a loss. So a run that lost to it came back to find it
                // still there, and still there after the next loss, with no
                // way past but through: the rung's own fight was unreachable
                // for the rest of the run. A detour you cannot leave is not a
                // detour. Losing puts you back on the ladder.
                self.substitute = None;

                // Losing a dungeon floor **does not put you out of the
                // dungeon**, and that is the whole shape of the decision.
                //
                // It used to. Which meant a floor you could not beat cost you
                // the line whether you liked it or not, and `leave_dungeon` -
                // the verb that exists so the points can be a decision rather
                // than a trap - was only ever the polite version of something
                // the game would do to you anyway. Now a loss costs the mode's
                // own price and leaves you standing where you were: fight it
                // again, or retreat. **Retreating is how you survive.**
                //
                // The one exception is a Rogue on the edge of dying. A run put
                // out of the game inside a side-room, four fights from a road
                // it could have walked away down, is a run that was never
                // offered the choice this verb exists to offer - so the last
                // life is spent on the road. See `out_of_the_dungeon` below.
                //
                // `cleared_floors` is deliberately *not* rolled back either
                // way. The floors you beat before the one that beat you stay
                // beaten, which matters only if a siding brings you back, and
                // then it matters completely.
                let in_a_dungeon = self.dungeon.is_some();
                self.at_points = false;
                if !in_a_dungeon {
                    self.dungeon = None;
                }
                // And a passenger is a fragile thing that was riding on you.
                // It goes whatever the mode does about the loss, including a
                // loss the underwriter eats: the underwriter buys back a rung,
                // not a life somebody else was carrying.
                if let Some((id, _)) = self.passenger.take() {
                    self.loadout.remove_anywhere(id);
                    self.owned.retain(|&o| o != id);
                    settlement.lost_passenger = true;
                }
                // Underwritten: one fight, once, and it says which one.
                //
                // Read before the mode's own answer rather than after, because
                // "this loss did not happen" has to mean the knock-back did
                // not happen either. Taken away by the loss it eats, so a
                // second one inside the five rungs costs what it costs.
                if self.underwritten_until.is_some_and(|until| self.rung <= until) {
                    self.underwritten_until = None;
                    settlement.underwrote = Some(LADDER[self.rung.min(LADDER.len() - 1)].name);
                    self.last_receipt = Some(vec![
                        format!(
                            "The underwriter eats it: {}",
                            LADDER[self.rung.min(LADDER.len() - 1)].name
                        ),
                        "That loss did not happen".into(),
                    ]);
                } else {
                match self.mode {
                    Mode::Grinder => {
                        // Back to the rung you last cleared, so there is
                        // always something easier to farm.
                        //
                        // Inside a dungeon too. What changed about losing in
                        // one is where it leaves you standing, not what it
                        // costs: a Grinder that could retry a floor for
                        // nothing would be a Grinder for whom the way out is
                        // never worth taking, and the whole point of leaving
                        // is that it is a decision with a price on both sides.
                        // The rung is the one you walked in on, so the cost is
                        // paid when you walk back out.
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
                        // Down to the last one, and standing in a dungeon: out
                        // onto the road, where the next fight is one the run
                        // chose rather than one it had walked four floors
                        // into. Everything cleared stays cleared, and the door
                        // does not reopen - which is what leaving has always
                        // cost.
                        if in_a_dungeon && self.lives <= 1 {
                            let name = self.dungeon.map(|(d, _)| d.name).unwrap_or("");
                            // Out of this one and into whatever is under it.
                            // A run carried out of the innermost of two is
                            // still standing in the other, which is a place it
                            // can walk out of on its own terms.
                            self.back_up_a_dungeon();
                            self.last_receipt = Some(vec![
                                format!("Carried out of {name}"),
                                if self.lives == 0 {
                                    "There was nothing left to carry".into()
                                } else {
                                    "One life left, and the road is safer".into()
                                },
                            ]);
                        }
                    }
                }
                }
            }
        }

        // Showstopper: a win inside the window pays more. Read off the log's
        // own clock rather than a stopwatch anybody kept, which is the same
        // number the casino's door is decided by.
        if outcome == Outcome::Victory {
            let ms = self.log.as_ref().map(|l| l.duration_ms).unwrap_or(u32::MAX);
            let pct: i32 = self
                .effective_classes()
                .iter()
                .filter_map(|c| match c.power {
                    crate::class::ClassPower::Showstopper { pct, under_ms } if ms < under_ms => {
                        Some(pct)
                    }
                    _ => None,
                })
                .sum();
            if pct > 0 {
                let extra = bounty * pct / 100;
                self.gold += extra;
                settlement.reward += extra;
            }
        }

        // A contract that has run its course is a contract honoured, and the
        // Payout reads this rather than trusting a flag somebody set.
        if self.contract_until.is_some_and(|until| self.rung > until) {
            self.contract_until = None;
            self.contract_honoured = true;
        }
        // A passenger that has gone far enough is a passenger delivered, and
        // the courier is waiting wherever you got to.
        if self.passenger.is_some_and(|(_, until)| self.rung >= until) && self.passenger_is_seated()
        {
            let pays = self.passenger_pays;
            if self.deliver_passenger() && !pays.is_empty() {
                self.give(pays);
                self.last_receipt = Some(vec![
                    "Delivered, and in one piece".into(),
                    format!("The courier hands over: {}", pays),
                ]);
            }
        }

        // New shelves after every battle, win or lose.
        let need = self.needs_a_weapon();
        self.restock(need);
        // And a shelf somebody promised for afterwards.
        if let Some(shelves) = self.shop_owed.take() {
            self.shop.stock_exactly(shelves);
        }

        // C2, both halves: a bet already taken is settled here, and a board
        // with an empty grid is noticed here. In that order, so the fight that
        // wins a bet cannot also be the one that offers it.
        if outcome == Outcome::Victory {
            self.settle_the_waste();
            self.look_at_the_waste();
        }

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
        self.log = Some(crate::combat::CombatLog::won_by_default(&self.monster()));
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
        self.restock(need);
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
            // The slot's own height, not the tallest board's. Same shape of
            // fault as the one `branching-events.md` records: "anything
            // comparing against the constant is asking the wrong question",
            // and the constant grew into a per-board number that has now grown
            // into a per-slot one.
            if x >= SLOT_W as u32 || y >= self.loadout.slot(kind).rows() as u32 {
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
        self.restock(need);
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
        self.restock(need);
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
        self.restock(need);
        class
    }

    // ------------------------------------------------------------ quests

    /// How far along a piece's quest is.
    /// The quest a piece is carrying, born with or handed to it.
    ///
    /// A `PieceDef`'s quest is a property of the component; a granted one is a
    /// property of *this* piece in *this* run, which is why it lives on the
    /// run. Granted wins where both exist: the table said something out loud
    /// about a piece somebody was holding, and that is the more recent fact.
    pub fn quest_of(&self, id: PieceId) -> Option<&'static crate::piece::Quest> {
        if let Some(q) = self.granted_quests.get(&id) {
            return Some(q);
        }
        self.registry.def(id).quest.as_ref()
    }

    /// Hand a quest to a piece that was not born with one.
    pub fn grant_quest(&mut self, id: PieceId, q: &'static crate::piece::Quest) {
        self.granted_quests.insert(id, q);
    }

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
                let Some(quest) = self.quest_of(piece) else { continue };
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
            let quest = match self.quest_of(piece) {
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

    /// Spend a row that has been granted, on the board you choose.
    ///
    /// `owed_rows` holds the grant until the choice is made, because "one
    /// board of your choice" is a decision and an outcome cannot make it for
    /// you. Refuses when nothing is owed, so the receipt cannot claim a row
    /// that was not there.
    pub fn grow_slot(&mut self, kind: SlotKind) -> bool {
        if self.owed_rows == 0 {
            return false;
        }
        self.owed_rows -= 1;
        self.loadout.grow_one(kind, 1);
        self.last_receipt = Some(vec![format!("+1 row on the {}", kind.name().to_lowercase())]);
        true
    }

    /// Extra rows each grid has beyond the eight it started with.
    ///
    /// Indexed by `SlotKind::index`. Read off the boards rather than tracked,
    /// so it cannot disagree with them.
    pub fn slot_rows(&self) -> [u8; 5] {
        let mut out = [0u8; 5];
        for k in SlotKind::ALL {
            out[k.index()] = self.loadout.slot(k).rows().saturating_sub(crate::slot::SLOT_H);
        }
        out
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
        // A quest item is carried, never worn. It used to be refused by not
        // being worth seating - one cell, no stats, no triggers - which is a
        // rule nothing enforces and everything has to remember. This is the
        // one place that has to know.
        //
        // **Except the one riding on you.** A passenger is the one quest item
        // whose whole cost is the cells it sits in: `passenger_is_seated`
        // refuses to deliver a parcel that spent the trip in the tray, so a
        // parcel that cannot be seated is a parcel that can never be
        // delivered. Written as "the piece that is currently the passenger"
        // rather than as a name, because a rule with a name in it is a rule
        // with a list in it.
        let riding = self.passenger.map(|(p, _)| p) == Some(id);
        if self.registry.def(id).kind == crate::piece::PieceKind::Quest && !riding {
            return Err(RuleError::NotWearable);
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
        base += self.relic_pay().stats;
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

    /// What every relic on the board is paying right now.
    ///
    /// On the board, not in the tray: a relic costs a cell like anything else,
    /// and a reward that pays from a pocket is a reward with no decision in
    /// it. Summed rather than taken from the best, because two relics are two
    /// cells.
    pub fn relic_pay(&self) -> crate::relic::Payout {
        let mut out = crate::relic::Payout::default();
        for k in SlotKind::ALL {
            for id in self.loadout.slot(k).pieces() {
                let Some(r) = crate::relic::relic(self.registry.def(id).name) else { continue };
                let p = (r.pays)(self);
                out.stats += p.stats;
                out.speed_pct += p.speed_pct;
            }
        }
        out
    }

    /// Activation profiles for every assembled item — what combat runs on.
    pub fn combat_items(&self) -> Vec<crate::loadout::ItemProfile> {
        let mut items = self.loadout.combat_items(&self.registry);
        // A relic's speed is not a `Stats` field, because no speed in this
        // game is: every other one is a percentage on an item's cooldown, and
        // this is applied in the same place and the same way.
        let pct = self.relic_pay().speed_pct;
        if pct > 0 {
            for it in &mut items {
                it.cooldown_ms =
                    ((it.cooldown_ms as i64 * (100 - pct.min(75)) as i64) / 100).max(100) as u32;
            }
        }
        // A curse that outlasts the fight is still a curse, and the only kind
        // of curse a *piece* can carry. It is frost, because frost is the one
        // curse that means anything to a thing rather than to a fighter, and
        // it is applied here for the same reason the contract is: this is
        // where every speed in this game is applied.
        //
        // Until this existed, `cursed_for_good` was a list nothing read - the
        // library's price was a word in a receipt, and `Outcome::Uncurse`
        // undid nothing.
        if !self.cursed_for_good.is_empty() {
            for it in &mut items {
                if it.pieces.iter().any(|p| self.cursed_for_good.contains(p)) {
                    it.cooldown_ms =
                        ((it.cooldown_ms as i64 * (100 + CURSED_SLOWER) as i64) / 100) as u32;
                }
            }
        }
        // A contract is frost you asked for. Frost slows gear, so this slows
        // gear - in the one place every other speed in this game is applied,
        // rather than by teaching `simulate` about a piece of paper.
        if self.under_contract() {
            for it in &mut items {
                it.cooldown_ms =
                    ((it.cooldown_ms as i64 * (100 + CONTRACT_SLOWER) as i64) / 100) as u32;
            }
        }
        items
    }

    /// Is a contract running right now?
    pub fn under_contract(&self) -> bool {
        self.contract_until.is_some_and(|until| self.rung <= until)
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
        let spec = self.monster();
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

    /// B5's ending. The hardest authored thing in the game.
    ///
    /// Not a `county_pinnacle`: it belongs to no chain, and settling it must
    /// not mark one done. It ends the trip the way a pinnacle does.
    pub fn begin_parish(&mut self) -> &CombatLog {
        self.forget_undo();
        self.walking_the_parish = true;
        let party: Vec<crate::combat::MonsterSpec> =
            crate::combat::creature("THE PARISH").into_iter().copied().collect();
        self.fight_party(&party)
    }

    /// The fight at the end of a chain of THE HUNDRED.
    ///
    /// A **party**, always, even where the party is one: the Drove's ending is
    /// a drover and the herd he is driving, and building one code path for
    /// "one creature" and another for "two" would mean the Ordnance and the
    /// Drove settled differently. `simulate_party` handles a party of one.
    ///
    /// Losing costs what a road loss costs and ends the trip (A7); winning is
    /// settled by `settle`, which is where the chain is marked done.
    pub fn begin_county_fight(&mut self) -> &CombatLog {
        let Some(chain) = self.county_pinnacle else { return self.begin_fight() };
        self.forget_undo();
        // `simulate_party` steps each spec for the difficulty itself, the way
        // it does for a brawl.
        let mut party: Vec<crate::combat::MonsterSpec> = crate::county::pinnacle_party(chain)
            .iter()
            .filter_map(|n| crate::combat::creature(n))
            .copied()
            .collect();
        // D-4. The Drover gets stronger with the clock: a run that dawdled
        // meets a harder drover. Behind its own constant so that zeroing it is
        // one line and nothing else moves - which is what taking D-4's
        // recommendation meant.
        if chain == crate::county::Chain::Drove {
            let gained =
                (self.events_resolved / crate::county::DROVER_STRENGTH_PER.max(1)) as i32;
            if crate::county::DROVER_STRENGTH_PER > 0 {
                for m in party.iter_mut() {
                    m.strength += gained;
                }
            }
        }
        self.fight_party(&party)
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
        // Two things do not spend the visit. The pedestal is not a door - it
        // stands in the entryway and takes its own key - and the Second Key
        // is the one *thing* that ever breaks the rule, which is legal
        // because it costs you the key. Both exceptions live here, which is
        // what keeps them to two.
        let second = self.second_key_ready || !what.costs_the_visit();
        let Some(t) = (if second { self.town } else { self.town.take() }) else {
            return TownVisit::default();
        };
        if second {
            if what.costs_the_visit() {
                self.second_key_ready = false;
            }
        } else {
            self.towns_seen.push(t.id);
        }
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
                // This town's own seven of the eleven, so the shelf fits the
                // screen and no two towns stock the same shop.
                let shelf = crate::piece::town_shelf_for(self.run_seed, t.id);
                self.shop.stock_exactly(&shelf);
                out.stocked = shelf.len();
            }
            Action::Pub => {
                self.shop.stock_exactly(crate::rumour::on_offer());
                out.stocked = crate::rumour::on_offer().len();
            }

            // ---- the Slagworks -------------------------------------------
            //
            // Every door here changes something you already own. What the
            // crucible and the tempering want is a *piece*, and the one they
            // get is the first thing loose in the tray - the same rule
            // `BuyOff` has used since the toad first asked for a two-by-two.
            // Choosing which piece is the interface's job and not the rule's.
            Action::Crucible => {
                if let Some(id) = self.inventory().first().copied() {
                    self.melt(id);
                }
            }
            Action::MoldLine => {
                self.shop.stock_exactly(MOLD_LINE);
                out.stocked = MOLD_LINE.len();
            }
            Action::Tempering => {
                let cost = self.last_bounty / 2;
                if self.gold >= cost {
                    if let Some(id) = self.inventory().first().copied() {
                        let def = self.registry.def_index(id);
                        if let Some(better) = crate::piece::dearer_than(def, TEMPERING_GAIN) {
                            self.gold -= cost;
                            out.paid = -cost;
                            self.registry.transform(id, better);
                            self.forget_undo();
                        }
                    }
                }
            }
            Action::Foreman => {
                // He has heard something below. If you already know what, he
                // pays you not to say it back.
                if self.holds("A Word About the Cellar") || self.towns_revealed.contains(&"the-manse")
                {
                    let paid = self.last_bounty;
                    self.gold += paid;
                    out.paid = paid;
                } else {
                    self.give("A Word About the Cellar");
                }
            }

            // ---- the Manse -----------------------------------------------
            Action::CellarDoor => self.enter_dungeon("the-threshold"),
            Action::Gallery => {
                if let Some(id) = self.inventory().first().copied() {
                    let def = self.registry.def(id);
                    let worth = crate::rating::Rarity::of(crate::rating::piece_rating(def));
                    let paid = crate::rating::resale_price(def) * 2;
                    self.loadout.remove_anywhere(id);
                    self.owned.retain(|&o| o != id);
                    self.gold += paid;
                    out.paid = paid;
                    // Double for anything. A good piece gets you mentioned to,
                    // and a very good one gets you told where the last one
                    // like it was fished up - which is a place, and the place
                    // is at the bottom of some water.
                    if worth > crate::rating::Rarity::Common {
                        self.give("A Word About the Glow");
                    }
                    if worth >= crate::rating::Rarity::Legendary {
                        self.enter_dungeon("the-undertow");
                    }
                }
            }
            Action::LongTable => {
                self.grown_health += LONG_TABLE_HEALTH;
            }
            // ---- Extra Large ---------------------------------------------
            Action::Aisle9 => {
                self.shop.stock_exactly(AISLE_NINE);
                out.stocked = AISLE_NINE.len();
            }
            Action::ReturnsDesk => {
                // Full price, which nobody else pays - or consignment, which
                // is the standing arrangement rather than this one sale.
                if let Some(id) = self.inventory().first().copied() {
                    let def = self.registry.def(id);
                    let paid = crate::rating::shop_price(def);
                    self.loadout.remove_anywhere(id);
                    self.owned.retain(|&o| o != id);
                    self.gold += paid;
                    out.paid = paid;
                    if !self.standing_orders.contains(&crate::event::Standing::Consignment) {
                        self.standing_orders.push(crate::event::Standing::Consignment);
                    }
                }
            }
            Action::SampleCounter => {
                // Free, seeded, and genuinely a common. A sample counter that
                // handed out anything better would be the shop.
                let pool: Vec<usize> = crate::piece::all_def_indices()
                    .into_iter()
                    .filter(|&i| {
                        let d = &CATALOG[i];
                        crate::rating::Rarity::of(crate::rating::piece_rating(d))
                            == crate::rating::Rarity::Common
                            && !crate::piece::is_boss_only(d.name)
                            && !crate::piece::is_quest_reward(d.name)
                            && !crate::piece::is_event_only(d.name)
                            && !crate::piece::is_town_stock(d)
                            && (self.insight_unlocked || !crate::piece::touches_insight(d))
                    })
                    .collect();
                if let Some(&pick) = pool.get(self.rng.below(pool.len().max(1))) {
                    let id = self.registry.alloc(pick);
                    self.owned.push(id);
                }
            }
            Action::Manager => {
                if !self.holds("A Word About the Wrong Stars") {
                    self.give("A Word About the Wrong Stars");
                } else {
                    let paid = self.last_bounty;
                    self.gold += paid;
                    out.paid = paid;
                }
            }
            // The pedestal is answered by `feed_pedestal` with an orb in hand;
            // walking up to it without one is looking at furniture.
            Action::Pedestal => {}
            // The way down is answered by `enter_county` at this town's mouth,
            // which the interface picks the moment it knows which town it is
            // standing in. Refused once this town has been used, and the
            // refusal costs nothing - the door is not a door.
            Action::County => {
                let mouth = crate::county::MOUTHS.iter().find(|(id, _)| *id == t.id).map(|(_, m)| *m);
                match mouth {
                    Some(m) if self.enter_county(TripSource::Town(t.id), m) => {
                        // `enter_county` wrote the receipt, which says where
                        // you came down and what was standing there.
                        return TownVisit { at: Some(t.name), did: Some(what), ..out };
                    }
                    _ => {
                        self.last_receipt = Some(vec![
                            format!("The steps under {} have been walked once already", t.name),
                            "A town is one trip down, and this one is spent".into(),
                        ]);
                        return TownVisit { at: Some(t.name), did: Some(what), ..out };
                    }
                }
            }
            Action::Library => {
                if let Some(id) = self.inventory().first().copied() {
                    let def = self.registry.def_index(id);
                    if let Some(better) = crate::piece::dearer_than(def, LIBRARY_GAIN) {
                        self.registry.transform(id, better);
                        self.cursed_for_good.push(id);
                        self.forget_undo();
                    }
                }
            }
        }
        self.last_receipt = Some(out.receipt());
        out
    }

    /// Push the class rules that live on the board back onto the board.
    ///
    /// Recycler scales assembly bonuses, and `Loadout::report` is the single
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
        self.loadout.assembly_pct = pct;
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
