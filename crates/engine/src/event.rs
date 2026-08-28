//! Things that happen on a rung instead of a fight starting.
//!
//! An event stands in front of a rung and asks a question. It never adds a
//! rung of its own - the road is fifty long whichever answers you give - and
//! it never resolves itself: every one of them is a choice the player makes,
//! because an event that decides for you is just a cutscene with extra steps.
//!
//! Adding one is adding an entry to `EVENTS`. The engine works out whether a
//! choice can be taken, `Run::take_choice` applies it, and the interface draws
//! whatever is there. Nothing else has to know.

/// What a choice needs before it can be taken.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Requirement {
    /// Always available.
    None,
    /// A loose component in the tray whose footprint is exactly `w` by `h`,
    /// at some rotation. Handing something over has to cost you something you
    /// could have used.
    LooseItemOfSize { w: u8, h: u8 },
    /// A choice taken at an earlier event, by its label. What you did three
    /// rungs ago is allowed to change what is on offer now.
    Took(&'static str),
    /// A named component, anywhere you own it - worn or loose. Unlike
    /// `LooseItemOfSize` this is not handed over: the door opens because you
    /// have the key, and you keep the key.
    Holding(&'static str),
    /// Something the run has done, by name.
    ///
    /// The chain's stations set these and later stations read them. A list of
    /// strings rather than a field per station, and that is a decision worth
    /// arguing: named booleans are checked by the compiler and a string is
    /// not. What a string buys is the reverse index - `set_by` walks `EVENTS`
    /// and finds which outcome sets a flag, so `no_flag_is_waited_on_forever`
    /// is one assertion. A field per station gives no such lint, and the fault
    /// this is guarding against is not a typo, it is a chain with a station
    /// nothing reaches.
    Flag(&'static str),
    /// Something the run has done `at_least` times, silently counted.
    ///
    /// The watcher pattern: revealing the Slagworks arms a counter nobody
    /// mentions, and forty rungs later the foundry says what it noticed. The
    /// arming leaves a receipt line and no explanation, which is the closest
    /// this game gets to being haunted.
    Counter { what: &'static str, at_least: u32 },
    /// Tiles of THE HUNDRED cleared in one region, at least this many.
    ///
    /// The pale's checklist reads three of these (B3.1). A region rather than
    /// a total because eighteen tiles spread across the county is two trips'
    /// work in three directions, and eighteen in one corner is one trip walked
    /// four times - and the Enclosure is the chain you finish by having been
    /// everywhere.
    CountyTiles { region: crate::county::Region, at_least: usize },
    /// A chain of THE HUNDRED finished: its pinnacle beaten.
    ///
    /// The perambulation waits on all three (B5), and the two words that cross
    /// back up onto the road wait on one.
    CountyCleared(crate::county::Chain),
    /// An assembled item of at least this rarity, anywhere on the board.
    ///
    /// Rarity is an *item's*, not a component's - `RARE_AT` is 90 on a scale
    /// where full marks is the best a whole item can do, so a single component
    /// almost never clears it. A door that asked for a loose Legendary would
    /// be asking for one of ten pieces in the catalogue.
    AssembledOfRarity(crate::rating::Rarity),
    /// At least this many assembled items sharing one alignment word.
    ///
    /// The inspector's question, and the reason building *for* an event is a
    /// strategy: it reads the live board rather than the tray.
    AlignedItems(usize),
    /// An Orb of Travel, worn or loose - and surrendering it is the price.
    ///
    /// Unlike `Holding`, which names one component, this asks for a *kind* of
    /// key, because the county has two orbs of its own and the road has four
    /// and any of the six will do.
    HoldingOrb,
    /// The pale's whole checklist: six tiles cleared in each of the three
    /// regions, two boundary stones read, and an orb in hand.
    ///
    /// **One requirement for five lines**, because a choice carries one and
    /// the gate is one decision. What a player *reads* is
    /// `Run::pale_checklist`, which is the same five questions asked
    /// separately so that each can be ticked - and asked through the same
    /// machinery, so the list and the gate cannot drift.
    ThePaleIsReady,
    /// A word in the tray - any of them.
    ///
    /// The Buyer's menu is meant to be generated from what you hold, and
    /// `Choice` is static data: generating choices would mean the event table
    /// stopped being a table. So the menu is **gated** instead - three doors,
    /// each shut unless you have the thing it wants - which reads the same
    /// from the player's side and keeps the table a table.
    HoldingRumour,
    /// At least this many titles.
    Classes(usize),
    /// Gold, priced in what the rung in front of you is worth.
    ///
    /// Every figure in this mission is a multiple of the standing rung's
    /// bounty rather than a constant, and this is the requirement half of
    /// that. A price written as a number is a price that means one thing at
    /// rung four and something else entirely at rung forty; the spec's own
    /// figures were two and a half times everything a run had ever seen at one
    /// end and one bounty at the other.
    ///
    /// Spent when the choice is taken, which is what makes it a price rather
    /// than a test of wealth.
    Purse { times: i32 },
    /// A number, named by the player, inside these bounds.
    ///
    /// Always open - anybody can say a figure - so `choice_open` is true and
    /// the refusal happens at `take_choice_with`, which is where the figure
    /// arrives. A choice asking for one cannot be taken by `take_choice`,
    /// because there is nothing to take it *with*.
    Figure { min: i32, max: i32 },
}

/// A fight an event sets up, against however many creatures it likes.
///
/// It is not a rung. The ladder does not move whichever way it goes, because
/// an event putting two creatures in front of you is a detour and not a step -
/// whatever the rung was going to hand you is still waiting afterwards.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Brawl {
    /// Everything across the table, by name.
    pub with: &'static [&'static str],
    /// The component you keep if you win. Empty for a fight worth nothing.
    pub win: &'static str,
    /// Rows added to every grid on a win, on top of the component.
    pub and_grow: u8,
    /// Whether losing costs you a life.
    ///
    /// The casino does not: a branch that punishes you for taking the
    /// interesting option is a branch nobody takes twice. What you lose by
    /// losing is the thing you would have won.
    pub forgiving: bool,
}

/// What taking a choice does.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Outcome {
    /// Fight the creature this rung was always going to hand you.
    FightAsWritten,
    /// Fight something else instead. The rung is still one rung.
    FightInstead(&'static str),
    /// Skip the fight. The bounty is paid `times` over, and whatever the
    /// requirement named is taken off you.
    BuyOff { times: i32 },
    /// Walk into a mini dungeon. The rung does not move, so coming out puts
    /// you back in front of the fight you had not got to.
    Enter(&'static str),
    /// One more loss before the run ends.
    Spare,
    /// A class handed over on the spot, which no fountain offers.
    Claim(&'static str),
    /// A component handed over on the spot. It arrives loose, in the tray,
    /// where it takes up room like anything else - a reward you have to find
    /// space for is a reward you have to think about.
    Give(&'static str),
    /// Step into a fight the event has arranged. See `Brawl`.
    Step(&'static Brawl),
    /// Put these on the shelves, and hand over what agreeing to them costs.
    ///
    /// The shop is emptied and restocked with exactly `shelves`, which is how
    /// a curated offer works without needing a screen of its own: you walk out
    /// and the shop is different. `class` is the price of the arrangement.
    Stock { shelves: &'static [&'static str], class: &'static str },
    /// Remember that this happened, by name. The chain is built out of these.
    Flag(&'static str),
    /// Count that this happened, silently. Nothing says a word; a door forty
    /// rungs later reads the tally and says what it noticed.
    Count(&'static str),
    /// Put a hidden town on the road. It stands at its own rung from here.
    RevealTown(&'static str),
    /// A curated shelf, one visit. Unlike `Stock` it costs nothing and grants
    /// nothing - it is a shop somebody laid out for you rather than a bargain.
    OpenShop { shelves: &'static [&'static str] },
    /// Walk into a mini dungeon from somewhere that is not a rung.
    ///
    /// The same thing `Enter` does, named for where it is used: `Enter` is an
    /// event's, and this is a town door's. Kept apart because a town is a rung
    /// of its own and an event stands in front of one, so what "coming out"
    /// means is different for each.
    StartDungeon(&'static str),
    /// A row, on one slot, chosen later. `Run::owed_rows` holds it until the
    /// player says which slot - "one board of your choice" is a decision and
    /// an outcome cannot make it for you.
    GrantRow,
    /// Hand a loose piece a quest it was not born with.
    GrantQuest(&'static crate::piece::Quest),
    /// The next named creature of your choice drops its **entire** board.
    ClaimTicket,
    /// A standing arrangement with the shop, for the rest of the run.
    StandingOrder(Standing),
    /// Your next loss within five rungs does not count. One fight, once.
    Underwrite,
    /// Take an Orb of Travel, whichever one comes first.
    ///
    /// The pale's price, and the only thing in the game that eats an orb
    /// without going anywhere. Worn or loose: an orb built into a weapon is
    /// still an orb, and the gatepost takes it out of the weapon.
    SurrenderOrb,
    /// Open the mind lane, for good.
    ///
    /// There is exactly one of these in the game and there should be: a pool
    /// you have to be given is a thing that happens once, and the road that
    /// gives it is the whole of what THE THRESHOLD is for.
    UnlockInsight,
    /// Several things at once.
    ///
    /// A choice used to be worth exactly one thing, and most of them still
    /// are. The ones that are not are the ones where the *trade* is the
    /// content - a hundred of your maximum health for the best piece off
    /// somebody's back - and writing that as one outcome would mean inventing
    /// a variant per bargain.
    All(&'static [Outcome]),
    /// Gold, priced in what the rung in front of you is worth.
    ///
    /// `BuyOff` has paid this way since the toad first counted fnorp onto a
    /// stone; this is the same arithmetic without the rung being bought.
    Pay { times: i32 },
    /// Maximum health, for the rest of the run. Negative takes it.
    ///
    /// Nothing else in the game trades this away, which is the whole reason
    /// the Teller is worth meeting - and the Manse's long table is its mirror.
    Health(i32),
    /// A seeded gamble: `wins` times in `outof`, from the run's own PRNG.
    ///
    /// The receipt shows what happened and never the odds. A machine at the
    /// roadside does not print its own probabilities on the front, and being
    /// told them afterwards would turn a story into a spreadsheet.
    Gamble { wins: u32, outof: u32, won: &'static Outcome, lost: &'static Outcome },
    /// The foundry's auction. One lot, against a reserve nobody has seen.
    ///
    /// The figure comes from `Requirement::Figure` and the reserve from the
    /// run's own seed. Over pays, under loses the lot - and the receipt shows
    /// the reserve either way, which is what makes losing teach rather than
    /// sting.
    SealedBid { lots: &'static [&'static str] },
    /// A shelf, laid out for you *after* whatever else this choice started.
    ///
    /// The Fork's other half. Shopping before a fight and shopping after one
    /// are two different decisions, and the only way to say the second is to
    /// hold the shelf until the fight is over.
    ShopAfter { shelves: &'static [&'static str] },
    /// Everything on every shelf costs this much more, for the rest of the run.
    Markup(i32),
    /// A fragile thing rides one of your boards for `rungs` rungs.
    Passenger { rungs: usize, pays: &'static str },
    /// Frost on all your gear for `rungs` rungs, taken on purpose.
    Contract { rungs: usize },
    /// Sell a word. The door it opened stays shut.
    SellWord,
    /// Sell a title. He does not ask which and neither does this.
    SellTitle,
    /// Frost on something of yours, chosen by the run's own seed.
    Chill,
    /// Take a curse off a piece, for good.
    ///
    /// The only thing that undoes the Manse library's price, and the only
    /// reason that price is a trade rather than a mistake.
    Uncurse,
    /// Leave it standing, and let it find you again `rungs` further on.
    ///
    /// The one outcome that does not close the door it was offered at.
    /// Everything else in this enum is a decision; this is declining to make
    /// one, and the price is that the thing comes back - which is the shape
    /// several of the chain's stations want and none of them could say.
    Defer { rungs: usize },
    /// See an upcoming boss's packed board from the loadout screen, for the
    /// rest of the run. Grants no stats whatsoever - the board view is the
    /// entire reward.
    Scout,
}

/// An arrangement with the shop that outlives one restock.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Standing {
    /// Every shelf from here on offers at least one piece of this kind.
    GuaranteedKind(crate::piece::PieceKind),
    /// The first reroll after every restock is free.
    FreeFirstReroll,
    /// A piece sold goes on consignment: it comes back three shops later,
    /// worth thirty more than it left.
    Consignment,
}

impl Standing {
    pub fn describe(self) -> String {
        match self {
            Standing::GuaranteedKind(k) => {
                format!("Standing order: every shelf offers a {}", k.name())
            }
            Standing::FreeFirstReroll => "Standing order: the first reroll is always free".into(),
            Standing::Consignment => {
                "Standing order: what you sell comes back three shops later, worth 30 more".into()
            }
        }
    }
}

impl Outcome {
    /// The concrete deltas this hands over, one line each.
    ///
    /// A `Vec` where `Requirement::describe` is a `String`, and the difference
    /// is the point: a requirement is one condition, and an outcome is
    /// however many things happen. The VIP area's bargain restocks a shop
    /// *and* costs you a class, and a receipt that mentioned one of those
    /// would be a receipt somebody had to check.
    ///
    /// Static: what this outcome *is*, for a tooltip before it is taken. What
    /// it *did*, with the run's own numbers in it, is `Run::receipt` - a
    /// bounty depends on the rung and a seeded gamble depends on the roll,
    /// and neither is knowable from here.
    pub fn describe(&self) -> Vec<String> {
        match self {
            Outcome::FightAsWritten => vec!["Fight the creature standing here".into()],
            Outcome::FightInstead(name) => vec![format!("Fight {} instead", name)],
            Outcome::BuyOff { times } => vec![
                "Hand over what was asked".into(),
                format!("The rung is bought off, and pays its bounty {} times over", times),
            ],
            Outcome::Enter(id) => {
                let name = crate::dungeon::by_id(id).map(|d| d.name).unwrap_or(id);
                vec![format!("Enter: {}", name)]
            }
            Outcome::Spare => vec!["One more loss before the run ends".into()],
            Outcome::Claim(name) => vec![format!("Class: {}", name)],
            Outcome::Give(name) => vec![format!("Gained: {}", name)],
            Outcome::Step(b) => {
                let mut out = vec![format!("Fight: {}", b.with.join(" and "))];
                if !b.win.is_empty() {
                    out.push(format!("If you win: {}", b.win));
                }
                if b.and_grow > 0 {
                    out.push(format!(
                        "If you win: +{} row{} on every board",
                        b.and_grow,
                        if b.and_grow == 1 { "" } else { "s" }
                    ));
                }
                out.push(
                    if b.forgiving {
                        "Losing costs no life".into()
                    } else {
                        "Losing costs what losing costs".into()
                    },
                );
                out
            }
            Outcome::Flag(what) => vec![format!("Noted: {}", what.replace('-', " "))],
            Outcome::SurrenderOrb => vec!["The gatepost takes the orb".into()],
            // A silent counter says nothing. That is the whole mechanic: the
            // receipt is where a player would look for an explanation, and
            // there is not one until the thing that was counting speaks.
            Outcome::Count(_) => vec!["Nothing you could point to".into()],
            Outcome::RevealTown(id) => {
                let t = crate::town::by_id(id);
                match t {
                    Some(t) => vec![format!("Revealed: {} (after rung {})", t.name, t.after + 1)],
                    None => vec![format!("Revealed: {}, which is nowhere", id)],
                }
            }
            Outcome::OpenShop { shelves } => {
                let mut out = vec![format!("A shelf of {}, this once", shelves.len())];
                for n in shelves.iter() {
                    out.push(format!("  {}", n));
                }
                out
            }
            Outcome::StartDungeon(id) => {
                let name = crate::dungeon::by_id(id).map(|d| d.name).unwrap_or(id);
                vec![format!("Enter: {}", name)]
            }
            Outcome::GrantRow => {
                vec!["+1 row on a board of your choice, for the rest of the run".into()]
            }
            Outcome::GrantQuest(q) => vec![format!("A task: {}", q.label)],
            Outcome::ClaimTicket => {
                vec!["A claim on one named creature's whole board".into()]
            }
            Outcome::StandingOrder(o) => vec![o.describe()],
            Outcome::Underwrite => {
                vec!["Your next loss within five rungs does not count".into()]
            }
            Outcome::Scout => vec!["You can read a boss's board before you fight it".into()],
            Outcome::UnlockInsight => vec!["Insight unlocked".into()],
            Outcome::Uncurse => vec!["One piece stops being cursed".into()],
            Outcome::SellWord => vec!["A word, sold. That door stays shut".into()],
            Outcome::SellTitle => vec!["A title, sold. You stop being it".into()],
            Outcome::Chill => vec!["Something of yours runs cold".into()],
            Outcome::SealedBid { lots } => {
                let mut out = vec!["One lot, against a reserve nobody has seen".into()];
                for l in lots.iter() {
                    out.push(format!("  {}", l));
                }
                out
            }
            Outcome::ShopAfter { shelves } => {
                vec![format!("A shelf of {}, once this is over", shelves.len())]
            }
            Outcome::Markup(pct) => vec![format!("Every shelf costs {}% more", pct)],
            Outcome::Passenger { rungs, pays } => vec![
                format!("Something fragile rides your board for {} rungs", rungs),
                format!("Deliver it and the courier hands over: {}", pays),
            ],
            Outcome::Contract { rungs } => {
                vec![format!("Frost on all your gear for {} rungs", rungs)]
            }
            Outcome::All(each) => each.iter().flat_map(|o| o.describe()).collect(),
            Outcome::Pay { times } => {
                vec![format!("{} times this rung's bounty, into your purse", times)]
            }
            Outcome::Health(n) if *n < 0 => {
                vec![format!("{} maximum health, for the rest of the run", n)]
            }
            Outcome::Health(n) => {
                vec![format!("+{} maximum health, for the rest of the run", n)]
            }
            // What it *might* do, for a tooltip before it is taken. What it
            // did is the receipt's, and the receipt never says the odds.
            Outcome::Gamble { won, lost, .. } => {
                let mut out = vec!["A gamble. Either:".into()];
                out.extend(won.describe().into_iter().map(|l| format!("  {}", l)));
                out.push("or:".into());
                out.extend(lost.describe().into_iter().map(|l| format!("  {}", l)));
                out
            }
            Outcome::Defer { rungs } => {
                vec![format!("It finds you again {} rungs further on", rungs)]
            }
            Outcome::Stock { shelves, class } => {
                let mut out = vec![format!("The shop is emptied and stocked with {}", shelves.len())];
                for name in shelves.iter() {
                    out.push(format!("  {}", name));
                }
                out.push(format!("Class: {}", class));
                out
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Choice {
    pub label: &'static str,
    /// One line under the label. What it costs, or what you are in for.
    pub blurb: &'static str,
    pub requires: Requirement,
    pub outcome: Outcome,
    /// Shown instead of the choice when the requirement is not met, so a
    /// greyed-out button always says why.
    pub unmet: &'static str,
}

/// The rungs the two shallow-end doors watch, as indices.
///
/// A fight outside this is not evidence about the early game, and letting one
/// count meant a Grinder knocked back from rung eleven could open a door with
/// a fight it won on the way up.
pub const SHALLOW: std::ops::RangeInclusive<usize> = 1..=9;

/// What has to be true before an event will stand in front of you.
///
/// Most events are pinned to a rung and that is the whole condition. Some are
/// earned: the casino opens because of something you did, not because of where
/// you are, and if you never do it the casino never happens.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Trigger {
    /// Stands on `at`, every run, no questions.
    Rung,
    /// Turns up once the run has won a fight in under `within_ms`, anywhere
    /// from rung `from` up to and including `at`. Miss the window and it never
    /// fires.
    QuickKill { within_ms: u32, from: usize },
    /// The other side of the same coin: a win that took *longer* than
    /// `over_ms`. The shallow end has two doors and they are the same
    /// question asked twice - how is this build actually going?
    SlowKill { over_ms: u32, from: usize },
    /// Stands anywhere from `from` to `at`, for somebody carrying the named
    /// rumour *and* answering whatever it is a rumour about.
    ///
    /// Unlike the others this cannot be decided from a rung and two
    /// stopwatches: the conditions are about the board and about the whole run
    /// so far. `event::at` refuses it and `Run::pending_event` answers it,
    /// because the run is the only thing that knows.
    ///
    /// A window rather than a rung, because a door priced in a rumour is a
    /// door you might arrive at holding nothing - and one that stands on
    /// exactly one rung is a door a run can walk past for reasons that have
    /// nothing to do with the bet it made. The two shipped ones set `from` to
    /// their own rung and behave exactly as they did.
    Whispered { rumour: &'static str, from: usize },
    /// Stands anywhere from `from` to `at`, for a run that has done something.
    ///
    /// The chain's own trigger. Everything else here is decided by where you
    /// are or how fast you got there; this is decided by what you did, which
    /// is what makes a chain a chain rather than four events on four rungs.
    WhenFlagged { flag: &'static str, from: usize },
}

impl Trigger {
    /// The first rung an earned event can appear on. Scheduled ones stand on
    /// exactly one rung, so it is that.
    pub fn from(self) -> usize {
        match self {
            Trigger::Rung => 0,
            Trigger::QuickKill { from, .. }
            | Trigger::SlowKill { from, .. }
            | Trigger::Whispered { from, .. }
            | Trigger::WhenFlagged { from, .. } => from,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LadderEvent {
    /// Stable id, so a run can remember it has answered this one.
    pub id: &'static str,
    /// Rung index it stands on - or, for an earned event, the last rung it
    /// will still stand on.
    pub at: usize,
    /// What has to be true for it to appear at all.
    pub trigger: Trigger,
    /// Ids that shut this one for good once they have been answered.
    ///
    /// The two shallow-end doors are alternatives, not a pair: taking the
    /// casino is a statement about the run, and having made it you do not also
    /// get asked the opposite question.
    pub blocked_by: &'static [&'static str],
    /// The creature whose rung this is - checked against the ladder so a
    /// renumbering cannot leave an event pointing at the wrong fight.
    pub expects: &'static str,
    pub title: &'static str,
    pub prose: &'static [&'static str],
    pub choices: &'static [Choice],
}

/// The two at the third table, and what stepping between them is worth.
///
/// Calibrated against a complete auto-built board, which beats this pair and
/// loses to the next one up. That is the line to hold: the chip is the key to
/// the whole VIP event, so a pair nobody can beat would quietly delete a later
/// event rather than making an early one exciting.
///
/// The casino can open as early as rung one, where a starter board loses this
/// badly - and that is the tension worth having. Step in early and you will
/// probably lose; wait and your build is better, but the door shuts at rung
/// nine. Losing costs nothing either way.
pub static TABLE_THREE: Brawl = Brawl {
    with: &["Bone Archer", "Frost Wisp"],
    win: "Platinum Chip",
    and_grow: 0,
    forgiving: true,
};

/// The two standing over the sprocketmen in the back room.
///
/// Not forgiving. The casino's table is a bet you can walk away from; this is
/// a decision about somebody else, and it costs what losing costs.
pub static THE_BACK_ROOM: Brawl = Brawl {
    with: &["Obsidian Colossus", "Vermin Sovereign"],
    win: "Sprocketman's Gratitude",
    and_grow: 1,
    forgiving: false,
};

/// What the foundry lays out at the fork.
pub const FOUNDRY_SHELF: &[&str] =
    &["Ironbound Haft", "Bonesaw", "Adamant Fang", "Serrated Edge", "Kingmaker Hilt"];

/// Two showfighters, at exhibition stakes.
///
/// Forgiving, because that is what "exhibition stakes" means: it costs this
/// rung's purse and never a life or a rung, and a bout that could end a run
/// would not be a demonstration of anything.
pub static THE_SHOWFIGHTERS: Brawl = Brawl {
    with: &["Bone Archer", "Frost Wisp"],
    win: "",
    and_grow: 0,
    forgiving: true,
};

/// What the little shop has out, at a permanent discount.
pub const MOLE_SHELF: &[&str] =
    &["Ring of Hours", "Signet of Iron", "Tin Band", "Iron Band", "Oathring"];

/// What the table sets a piece to become.
///
/// The only content anywhere that touches the quest system, which is why it is
/// one line rather than a table: a piece that has gone off thirty times has
/// been *used*, and the table's whole trick is telling a thing what it is for.
pub static TABLE_TASK: crate::piece::Quest = crate::piece::Quest {
    label: "the table said what it is for",
    goal: 30,
    track: crate::piece::QuestTrack::SelfActivations,
    becomes: "Kingmaker Hilt",
};

/// The birds, past the next ridge, arriving with the rung.
///
/// The only event that changes the shape of the *next* fight rather than
/// standing in front of it, and the first adversarial use of a party outside
/// the casino's table.
pub static THE_FLOCK: Brawl = Brawl {
    with: &["THE FLOCK"],
    win: "",
    and_grow: 0,
    forgiving: true,
};

/// Your shadow, and what your shadow carries.
///
/// The first party fight in the game outside the casino's table, and the only
/// one that pays. Not forgiving: this is the chain's own gate, and a gate that
/// costs nothing to fail is not one - but a Grinder knocked back meets it
/// again two rungs up, because failing forward is the rule everywhere else on
/// this chain and the Herald is not an exception to it, only a delay.
pub static THE_HERALD: Brawl = Brawl {
    with: &["THE SHADOW", "THE LANTERN"],
    win: "An Unwound Mainspring",
    and_grow: 0,
    forgiving: false,
};

/// What THE HUNDRED asks, tile by tile.
///
/// **The same struct the road uses**, so the choice, requirement, outcome,
/// receipt, prose-lint and theme machinery all apply unchanged. Two fields are
/// dead here and say so: `at` is `usize::MAX` because a county event stands on
/// a tile rather than a rung, and `expects` is empty because there is no
/// creature behind it.
///
/// **A county event never fights.** `FightAsWritten`, `FightInstead`, `Step`,
/// `Enter` and `StartDungeon` are all barred - the county's only fights are
/// its pinnacles and THE PARISH - and `county::county_events_never_fight` is
/// the lint that keeps it true. What is left is what a county is for: things
/// left in fields, and people who know the ground.
///
/// **Eight into eleven slots** (D-2). Three of them are arranged twice, which
/// is why the id is not enough to say a tile has been answered and why
/// `Run::county_event` is not filtered on `answered`.
pub const COUNTY_EVENTS: &[LadderEvent] = &[
    LadderEvent {
        id: "the-pale",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE PALE",
        prose: &[
            "A fence with no field behind it. It runs from nowhere to nowhere \
             and it is in better repair than anything else in the county, and \
             the gate in it is shut with three separate arrangements none of \
             which is a lock.",
            "There is a board on the gate and the board has a list on it. \
             Somebody wrote the list to be read from exactly here, one tile \
             out, and it has been read from here a great many times.",
        ],
        choices: &[
            Choice {
                label: "Hand over an orb and open it",
                blurb: "It goes into the gatepost. You do not get it back.",
                requires: Requirement::ThePaleIsReady,
                outcome: Outcome::All(&[
                    Outcome::SurrenderOrb,
                    Outcome::Flag("the-pale-is-open"),
                ]),
                unmet: "The list is not finished, or you have no orb to give",
            },
            Choice {
                label: "Read the list again",
                blurb: "It says the same thing. It will keep saying it.",
                requires: Requirement::None,
                outcome: Outcome::Flag("read-the-pale"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-boundary-ditch",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE BOUNDARY DITCH",
        prose: &[
            "A ditch, and beside it the spoil from the ditch, and standing in \
             the spoil a woman called Ordish with a spade she has not put down \
             since before you came over the rise.",
            "She is not digging a new ditch. She is digging the old one out, \
             which she explains is a different job and a worse one, and which \
             she has been paid for once, forty years ago, by somebody who has \
             since stopped existing.",
        ],
        choices: &[
            Choice {
                label: "Take the other spade",
                blurb: "Two hours. She talks the whole time and none of it is small.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Pay { times: 1 },
                    Outcome::Count("county-work"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Ask what she has turned up",
                blurb: "Forty years of ditch has forty years of things in it.",
                requires: Requirement::None,
                outcome: Outcome::Give("Whetstone"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-field-barn",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE FIELD BARN",
        prose: &[
            "One barn, in the middle of one field, a long way from any house \
             that would want a barn. The door is shut with a stone.",
            "Inside: hay from a year that was better than this one, a bicycle \
             with no chain, and a shelf at head height with things on it that \
             somebody put there on purpose and did not label.",
        ],
        choices: &[
            Choice {
                label: "Take something off the shelf",
                blurb: "It is not stealing if you leave the stone where it was.",
                requires: Requirement::None,
                outcome: Outcome::Give("Iron Band"),
                unmet: "",
            },
            Choice {
                label: "Sleep in the hay",
                blurb: "An hour, and the roof holds.",
                requires: Requirement::None,
                outcome: Outcome::Health(20),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-milestone",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE MILESTONE",
        prose: &[
            "A stone the height of your knee with a number cut into it. The \
             number is not a distance. It is not a year either, and it is not \
             the number of any road that has ever run past here.",
            "Somebody has been keeping it clear of the grass. The cut edges \
             are sharp, and the stone is older than sharp edges last.",
        ],
        choices: &[
            Choice {
                label: "Write the number down",
                blurb: "You will know what it is for or you will not.",
                requires: Requirement::None,
                outcome: Outcome::Count("county-work"),
                unmet: "",
            },
            Choice {
                label: "Clear the grass back yourself",
                blurb: "Somebody has been doing this. Now somebody has done it.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("kept-the-milestone"),
                    Outcome::Pay { times: 1 },
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-gleaners",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE GLEANERS",
        prose: &[
            "Four of them working a field that has already been cut, bent \
             over, going slowly, picking up what the cutting left. They have \
             the right to be here and they will tell you so before you ask.",
            "The oldest is called Rell and she does not straighten up to talk \
             to you. She says the field is worse every year and the right is \
             the same right, and both of those are somebody's doing.",
        ],
        choices: &[
            Choice {
                label: "Glean with them",
                blurb: "Slow, and split five ways at the end of it.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 1 },
                unmet: "",
            },
            Choice {
                label: "Ask who cut the field",
                blurb: "Rell straightens up for this one.",
                requires: Requirement::None,
                outcome: Outcome::Flag("asked-who-cuts-it"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-pound",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE POUND",
        prose: &[
            "A square of wall with a gate in it, built to hold anything found \
             wandering until whoever lost it pays to have it back. There is \
             nothing in it now except a great deal of nettle.",
            "The gate is locked and the wall is four feet high. It has been \
             that way, presumably, for as long as anybody has been paying.",
        ],
        choices: &[
            Choice {
                label: "Pay the fee anyway",
                blurb: "There is a slot in the gatepost and it is not rusted shut.",
                requires: Requirement::Purse { times: 1 },
                outcome: Outcome::All(&[
                    Outcome::Flag("paid-the-pound"),
                    Outcome::Give("Tin Band"),
                ]),
                unmet: "You have not got a bounty spare",
            },
            Choice {
                label: "Step over the wall",
                blurb: "It is four feet high. The nettles are the only guard it has.",
                requires: Requirement::None,
                outcome: Outcome::Health(-10),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-charcoal-burner",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE CHARCOAL BURNER",
        prose: &[
            "Smoke coming out of a heap of earth, and a man asleep beside it \
             who wakes the moment the smoke changes colour and not before. He \
             is called Sowerby and he has been in this county longer than the \
             roads have.",
            "He knows what is under every field for a mile. He says this the \
             way a man says what the weather is doing, and then he tells you \
             one of them, because you are going up and he is not.",
        ],
        choices: &[
            Choice {
                label: "Listen",
                blurb: "It takes as long as it takes. The heap does not wait.",
                requires: Requirement::None,
                outcome: Outcome::Give("A Word About the Hundred"),
                unmet: "",
            },
            Choice {
                label: "Watch the heap while he sleeps",
                blurb: "He has not slept properly in nine days and it shows.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Pay { times: 2 },
                    Outcome::Flag("watched-the-heap"),
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-drowned-lane",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE DROWNED LANE",
        prose: &[
            "The lane goes into the water and comes out the other side, and \
             the water in between is the lane as well, and has been since the \
             brook was straightened for somebody's convenience.",
            "There is a plank. The plank is somebody's idea of a bridge and it \
             is nobody's idea of a good one.",
        ],
        choices: &[
            Choice {
                label: "Take the plank",
                blurb: "Quickly, and looking at the far end rather than down.",
                requires: Requirement::None,
                outcome: Outcome::Flag("crossed-the-lane"),
                unmet: "",
            },
            Choice {
                label: "Wade, and mend the plank after",
                blurb: "Wet to the knee, and the next one across will not be.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Count("county-work"),
                    Outcome::Pay { times: 1 },
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-parish-chest",
        at: usize::MAX,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "",
        title: "THE PARISH CHEST",
        prose: &[
            "An iron-bound box the size of a coffin, standing on a floor that \
             has no building on it any more. Three locks, and the three keys \
             were given to three people so that no one of them could open it \
             alone.",
            "Two of those people are gone and the third is not from here. The \
             box has been standing open-lidded to the rain for a long time and \
             the locks still work perfectly.",
        ],
        choices: &[
            Choice {
                label: "Say what the third key was for",
                blurb: "A surveyor up on the road told you, and did not know they had.",
                requires: Requirement::Flag("knows-the-third-key"),
                outcome: Outcome::All(&[
                    Outcome::Flag("opened-the-chest"),
                    Outcome::GrantRow,
                ]),
                unmet: "You do not know what the third key was for",
            },
            Choice {
                label: "Look at what the rain left",
                blurb: "Paper does not last. What is under the paper does.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 1 },
                unmet: "",
            },
        ],
    },
];

/// One county event by id.
pub fn county_event(id: &str) -> Option<&'static LadderEvent> {
    COUNTY_EVENTS.iter().find(|e| e.id == id)
}

pub const EVENTS: &[LadderEvent] = &[
    // -------------------------------------------- THE HUNDRED's three on-ramps
    //
    // Rungs 11, 13 and 17. All three genuinely free - no event, no town gate,
    // no boss - which is thirteen rungs rather than A0's nineteen, because
    // that list counts events and six of its entries are gates.
    //
    // **THE STOCKMAN moved from 25**, where A0 put it: The Manse is `after:
    // 24`, so 25 is its gate rung and `switchyard::the_four_doors` has refused
    // an event there since the last mission. It is a `Trigger::Rung` here
    // rather than the one-rung `Whispered` window Part B drew, because the
    // word that window waited on would have had to come off a bar that is
    // exactly six names and full.
    LadderEvent {
        id: "the-theodolite",
        at: 11,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Rust Colossus",
        title: "THE THEODOLITE",
        prose: &[
            "Three legs and a brass head, standing in the middle of the road \
             with nobody near it. The head turns when you touch it and keeps \
             turning after you stop.",
            "Look through it and the road is not what is in the eyepiece. What \
             is in the eyepiece is a field with a stone in it, and a line \
             ruled from the stone to somewhere out of frame, and the line does \
             not move when you move the head.",
            "A hand-written card is tied to one leg. Ackworth, it says, and \
             then: THREE LINES CROSS SOMEWHERE. THE CROSSING IS NOT MARKED. \
             MARKING IT WAS NEVER THE POINT.",
        ],
        choices: &[
            Choice {
                label: "Take the card",
                blurb: "Ackworth's handwriting, and Ackworth's arithmetic on the back.",
                requires: Requirement::None,
                outcome: Outcome::Flag("knows-the-ordnance"),
                unmet: "",
            },
            Choice {
                label: "Take the theodolite apart",
                blurb: "Brass is brass, and you read the card on the way past.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("knows-the-ordnance"),
                    Outcome::Pay { times: 1 },
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-stockman",
        at: 13,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Grave Chorus",
        title: "THE STOCKMAN",
        prose: &[
            "A man sitting on a gate counting something that is not there. \
             Ketton, he says, when you ask, and then he goes back to it: he \
             gets to sixteen and starts again, and Ketton has been doing it \
             long enough that the gate has worn where he sits.",
            "Sixteen, he says, is how many places there are. Not sixteen \
             animals and not sixteen fields. Sixteen places, in a ring, and \
             the ring does not go anywhere - it goes round, and whatever is \
             walking it has been walking it since before you asked.",
            "He will not say what is walking. He says you would not see it \
             yet and that seeing it is a thing you have to be taught by a \
             sign, and the signs are down there, and he is not going down \
             there again.",
        ],
        choices: &[
            Choice {
                label: "Count with him",
                blurb: "Sixteen. Sixteen. It is not a long ring and it is not a fast walk.",
                requires: Requirement::None,
                outcome: Outcome::Flag("knows-the-drove-roads"),
                unmet: "",
            },
            Choice {
                label: "Ask what he lost",
                blurb: "Two hundred head, over one winter, and he counts them still.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("knows-the-drove-roads"),
                    Outcome::Health(15),
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-commons",
        at: 17,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Pale Twin",
        title: "THE COMMONS",
        prose: &[
            "Four posts in a line across open ground, and a fifth lying down, \
             and nothing between any of them. It is the beginning of a fence \
             and it has been the beginning of a fence for a long time.",
            "A woman named Yaxley is putting the fifth one back up on her own, \
             badly. She explains without being asked that this is the fourth \
             time and that the fence was never about keeping anything in.",
            "\"There is a proper one further down,\" Yaxley says. \"It goes all \
             the way round something. Nobody who put it up is still alive and \
             it has not fallen over once.\"",
        ],
        choices: &[
            Choice {
                label: "Help her with the post",
                blurb: "It takes two, which is why it has been down four times.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("knows-the-enclosure"),
                    Outcome::Count("county-work"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Ask what is inside the proper one",
                blurb: "Yaxley stops working for the first time since you arrived.",
                requires: Requirement::None,
                outcome: Outcome::Flag("knows-the-enclosure"),
                unmet: "",
            },
        ],
    },
    // C1. Standing only for a run that has been down there and got nothing
    // out of it - a trip that cleared no tile, or a toll failed and never
    // crossed. `WhenFlagged` rather than `Trigger::Rung`, so he finds you when
    // he finds you rather than on a rung a gate might already have.
    LadderEvent {
        id: "the-constable",
        at: 39,
        trigger: Trigger::WhenFlagged { flag: "county-business", from: 8 },
        blocked_by: &[],
        expects: "The Rust Parliament",
        title: "THE CONSTABLE",
        prose: &[
            "The constable is called Wragby and he is waiting at the side of \
             the road with his hands behind his back, and he has been waiting \
             a while. He knows your name and he says it the way a man says a \
             name he has written down.",
            "There is a matter of a county, Wragby says. There is a matter of \
             somebody going about in it and coming back up with nothing to \
             show, which is not against anything, and which is the sort of \
             thing that gets looked into.",
            "He is going to take you down and he is not asking. What he does \
             not mention, and what is perfectly obvious once the steps start, \
             is that the gaol he is taking you to is nowhere near an edge.",
        ],
        choices: &[
            Choice {
                label: "Go quietly",
                blurb: "Five moves, from the middle, and the middle is a long way in.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("arrested"),
                    Outcome::Flag("county-business-settled"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Explain yourself",
                blurb: "It works. He writes it down and it takes the afternoon.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("county-business-settled"),
                    Outcome::Pay { times: 1 },
                ]),
                unmet: "",
            },
        ],
    },
    // C2. Off-rung, pushed by `settle` when a grid has nothing assembled in
    // it. Its `at` and `expects` are a formality - `forced_event` puts it in
    // front of you wherever you are - and `Trigger::WhenFlagged` on a flag
    // nothing sets is how the road's own tables say "not by rung".
    LadderEvent {
        id: "the-waste",
        at: 42,
        trigger: Trigger::WhenFlagged { flag: "never", from: 16 },
        blocked_by: &[],
        expects: "Verdigris",
        title: "THE WASTE",
        prose: &[
            "Somebody has been looking at your gear. Not at the good parts - \
             at the grid with nothing in it, which he calls waste ground and \
             which he says is a word with a legal meaning.",
            "The man is called Vessey and improving it is his job. Waste \
             ground that stays waste for long enough stops being anybody's, \
             Vessey says, and then it is his, and then he improves it.",
            "He would like to improve yours. Or he would like to bet you that \
             you cannot leave it exactly as it is.",
        ],
        choices: &[
            Choice {
                label: "Let him improve it",
                blurb: "He fills it with something. You do not choose what.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Give("Tin Band"),
                    Outcome::Pay { times: 1 },
                    Outcome::Flag("waste-improved"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Take the bet",
                blurb: "Five rungs. Leave it empty and he pays; fill it and you do.",
                requires: Requirement::None,
                outcome: Outcome::Flag("waste-bet-taken"),
                unmet: "",
            },
            Choice {
                label: "Spoken for",
                blurb: "It is not waste. It is where the third layer goes.",
                requires: Requirement::None,
                outcome: Outcome::Flag("waste-declined"),
                unmet: "",
            },
        ],
    },
    // ------------------------------------------------ THE HUNDRED comes up
    //
    // The one road door the county opens, and the one thing on the road that
    // opens a county tile. B6's crossing, both directions, in one event: you
    // carry a word up out of a field and what you are told up here is what the
    // third lock down there was for.
    //
    // Rung 37, which is genuinely free - no event, no town gate, no boss. A0's
    // list of nineteen counts events only and six of its entries are gates;
    // thirteen are actually free and this is one of them.
    LadderEvent {
        id: "the-county-surveyed",
        at: 37,
        trigger: Trigger::Whispered { rumour: "A Word About the Hundred", from: 12 },
        blocked_by: &[],
        expects: "The Iron Choir",
        title: "THE COUNTY SURVEYED",
        prose: &[
            "There is a table set up at the side of the road with a map on it \
             and a woman holding the map down against the wind with both \
             elbows. The map is of a place that is not on the road.",
            "Her name is Tasker and she takes what the burner told you the way \
             somebody takes a parcel they have been waiting on, and then \
             Tasker is quiet for long enough that you think you have said the \
             wrong thing.",
            "\"That is a hundred,\" she says. \"A subdivision, and a count. \
             Somebody counted it. Somebody kept a box.\"",
        ],
        choices: &[
            Choice {
                label: "Ask about the box",
                blurb: "Three locks and three keys. She knows what the third one was for.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Flag("knows-the-third-key"),
                    Outcome::Flag("the-county-is-surveyed"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Ask what she is being paid",
                blurb: "The map is not hers and the table is not hers.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Pay { times: 2 },
                    Outcome::Flag("the-county-is-surveyed"),
                ]),
                unmet: "",
            },
            // The counter's one reader. Three tiles down there count the same
            // thing - a ditch dug out, a milestone kept clear, a plank mended -
            // and Tasker is the only person on the road who would know to ask.
            //
            // It exists because `completable::no_more_counters_go_unread_than_already_do`
            // refused three new counters with no door, which is `CLAUDE.md`
            // §6 trap 19 catching exactly what it was written for. The answer
            // was one counter and a reader, not three counters and a budget.
            Choice {
                label: "Tell her what you have been doing down there",
                blurb: "Ditches, milestones, planks. She writes every one of them down.",
                requires: Requirement::Counter { what: "county-work", at_least: 2 },
                outcome: Outcome::All(&[
                    Outcome::Pay { times: 3 },
                    Outcome::Flag("knows-the-third-key"),
                    Outcome::Flag("the-county-is-surveyed"),
                ]),
                unmet: "You have done nothing down there worth writing down",
            },
        ],
    },

    // ---- the two the pub sells ----
    //
    // Neither stands here for anybody who did not buy the rumour, and neither
    // stands here for somebody who bought it and then did not do the thing.
    // That is the shape of a rumour: it is a bet on the board you will have.
    LadderEvent {
        id: "the-crownwright",
        at: 19,
        trigger: Trigger::Whispered { rumour: "A Word About the Crownwright", from: 19 },
        blocked_by: &[],
        expects: "Bone Cantor",
        title: "THE CROWNWRIGHT",
        prose: &[
            "Padgett the crownwright works out of one room over a fish shop \
             and does not turn round when you come in, on the grounds that he \
             can hear how full your head is from where he is sitting.",
            "\"Full,\" he says. \"Good. Most of them come up those stairs \
             empty and want me to put something in it. I make hats. I am not \
             a philanthropist and I am very much not a doctor.\"",
            "He will not sell you a hat. He will take a measurement, for the \
             record. The record is a ledger four inches thick that lives \
             under the bench, and Padgett will not let you look in it.",
        ],
        choices: &[
            Choice {
                label: "Stand still for it",
                blurb: "It takes a minute and he hums the entire way round.",
                requires: Requirement::None,
                outcome: Outcome::Give("Crownwright's Measure"),
                unmet: "",
            },
            Choice {
                label: "Ask what he made last",
                blurb: "It is on the shelf behind him at head height. He has been waiting.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Piety"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-green-ledger",
        at: 22,
        trigger: Trigger::Whispered { rumour: "A Word About the Green Ledger", from: 22 },
        blocked_by: &[],
        expects: "The Gearwright",
        title: "THE GREEN LEDGER",
        prose: &[
            "Creel has had the same column open for eleven years. He turns \
             the ledger round so you can read the figure at the bottom. \
             It is a large number and it is in green ink, because everything \
             in this ledger is in green ink, including the corrections.",
            "The figure is roughly what you have put into the ground and \
             pulled back out of it since the Cave Rat. Creel has been keeping \
             count the whole way up, in fives, four strokes and a bar. He \
             will not say who asked him to.",
            "\"Sign it off,\" he says, \"and it closes, and I go home, and my \
             wife has been asking. Or put a line under it and it stays open, \
             and I do not.\"",
        ],
        choices: &[
            Choice {
                label: "Close the column",
                blurb: "He is out of the door before the ink dries. The drawer under it is yours.",
                requires: Requirement::None,
                outcome: Outcome::Give("The Green Ledger"),
                unmet: "",
            },
            Choice {
                label: "Add your own line",
                blurb: "Eleven years is not so long. Creel says so himself, twice.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Longhauler"),
                unmet: "",
            },
        ],
    },

    // The pay-off for having asked rather than taken. Always stands here, so a
    // player who took Trundle at the roadside sees what the other answer was
    // worth - and a player who never met the cart at all learns there was one.
    LadderEvent {
        id: "where-it-was-going",
        at: 21,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Slag Warden",
        title: "AHEAD OF SCHEDULE",
        prose: &[
            "Kettleworks, twelve rungs and some weeks on, and there is Gerald \
             in the yard with the harness off, eating.",
            "The four tons went in through the doors nine days ago. Rowe has \
             been paid, has bought a hat with some of it, and is wearing the \
             hat. He is extremely pleased that you asked.",
            "\"Ahead of schedule,\" he says, for the second time in your \
             acquaintance, and this time he has the docket for it, and makes \
             you look at the docket.",
        ],
        choices: &[
            Choice {
                label: "Ask him again",
                blurb: "Whatever he did, he did by not stopping. Nothing starts fast.",
                requires: Requirement::Took("Ask how he does it"),
                outcome: Outcome::Claim("Longhauler"),
                unmet: "You never asked Rowe anything on the road, so he has nothing for you at Kettleworks.",
            },
            Choice {
                label: "Let them eat",
                blurb: "Gerald has earned that yard more than you have earned this road.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // Always stands here, whether or not you can go in. A door you cannot
    // open still tells you there was a door, and a player who skipped the
    // casino learns the casino existed - which is the whole reason the chip
    // is worth carrying thirty rungs.
    LadderEvent {
        id: "the-vip-area",
        at: 29,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Silence",
        title: "MEMBERS AND GUESTS",
        prose: &[
            "The rope is velvet. The man with the clipboard behind it is \
             called Merrik, his badge says HOST, and Merrik would very much \
             like to see the chip.",
            "Down the corridor behind him, past a door stencilled LINE 3 - \
             AUTHORISED ONLY, there is a noise. It is the noise gear-folk \
             make when they have been at something a very long time and \
             nobody has told them when it stops. You were mined out of a \
             cave. You know the noise.",
            "Merrik says there are five items on a table down there that have \
             never been for sale, and that guests are always welcome, and \
             that he will need your voice down and your hands where he can \
             see them. He says the second half in precisely the tone he said \
             the first.",
        ],
        choices: &[
            Choice {
                label: "Keep your face still",
                blurb: "Look at the table. Do not look down the corridor. Merrik checks.",
                requires: Requirement::Holding("Platinum Chip"),
                outcome: Outcome::Stock {
                    shelves: &[
                        "Overseer's Circlet",
                        "Foreman's Harness",
                        "Tallykeeper's Weave",
                        "Treadmill Sole",
                        "Quota Edge",
                    ],
                    class: "Immense Guilt",
                },
                unmet: "Merrik does not move the rope. Merrik has not moved the rope in eleven years",
            },
            Choice {
                label: "Get them out",
                blurb: "Two of them are paid to stop exactly this. It costs what that costs.",
                requires: Requirement::Holding("Platinum Chip"),
                outcome: Outcome::Step(&THE_BACK_ROOM),
                unmet: "Merrik does not move the rope. Merrik has not moved the rope in eleven years",
            },
            Choice {
                label: "Walk on",
                blurb: "Merrik thanks you for coming and means it, and holds the door for you.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // Earned, not scheduled: it turns up the moment you have flattened
    // something inside two seconds, so long as you are still in the shallow
    // end. Build something sharp early and the door is there; do not, and you
    // will finish the run without ever knowing the casino was in the game.
    //
    // `at` is the deadline rather than the address - the last rung it will
    // still stand on.
    LadderEvent {
        id: "the-casino",
        at: 8,
        // Rung two at the earliest: flattening the Cave Rat is not a
        // demonstration of anything, and the door being open before you have
        // built anything makes the first real decision of the run a coin toss.
        trigger: Trigger::QuickKill { within_ms: 3_500, from: 1 },
        blocked_by: &[],
        expects: "Whisperling",
        title: "THE CASINO",
        prose: &[
            "The Parlour takes anybody who can walk in, which is how you got \
             in. There is a bowl of complimentary salted rice by the door and \
             a card over it reading ONE (1) HANDFUL - HONOUR SYSTEM - WE ARE \
             WATCHING YOU TAKE IT.",
            "You are here for Hold-Em, played the way this house plays it: one \
             card in the deck is a live wasp and no player may look at it. You \
             have the gold. You have taken your one handful.",
            "At the third table along, two players have stopped playing \
             Hold-Em and started on each other. The room has formed a ring \
             around it. Marlow is working through the ring with a book, \
             taking side bets in a very neat hand, and the dealer is standing \
             perfectly still with the wasp held out at arm's length.",
        ],
        choices: &[
            Choice {
                label: "Step in",
                blurb: "Both of them at once. Marlow will want your name for the book first.",
                requires: Requirement::None,
                outcome: Outcome::Step(&TABLE_THREE),
                unmet: "",
            },
            Choice {
                label: "Keep out of it",
                blurb: "Not your table. Cash out, and take whatever the window gives you.",
                requires: Requirement::None,
                outcome: Outcome::Give("Gold Chip"),
                unmet: "",
            },
        ],
    },
    // The other shallow-end door, and the opposite question. Shut for good if
    // you took the casino: that was already an answer about how this run is
    // going, and nobody gets asked both.
    LadderEvent {
        id: "the-long-way",
        at: 8,
        // Fifteen seconds, down from twenty.
        //
        // The number is a statement about the shallow ladder, and the shallow
        // ladder was repacked to a curve: rungs 2 to 9 are four to six themed
        // pieces now where they were hand-authored boards two and three times
        // that. A board blunted until it grinds - the winning build with its
        // weapon taken off, at 27x - takes 18.0s at its slowest down there,
        // and took well over twenty against the boards this threshold was set
        // against. Nothing that can still reach the pay-off twelve rungs later
        // is slower than that.
        //
        // A sharp board's slowest shallow fight is 8.0s, so the two doors stay
        // as far apart as they were; and the prose has always said "that last
        // one took eleven seconds", which is nearer fifteen than twenty.
        trigger: Trigger::SlowKill { over_ms: 15_000, from: 1 },
        blocked_by: &["the-casino"],
        expects: "Whisperling",
        title: "GERALD",
        prose: &[
            "That last one took eleven seconds. You know it took eleven \
             seconds because a man at the roadside was counting out loud, and \
             when you finished he wrote the number in a notebook, said his \
             name was Rowe, and said nothing else about it.",
            "Rowe's cart is ahead of you on the road, pulled by an animal \
             with a brass plate on its harness. The plate \
             gives the species, which is Draught Tortoise, and the name, which \
             is Gerald, and the top speed, which is given in metres per hour.",
            "Gerald is hauling four tons of ore to Kettleworks. \
             They set off in the spring. Rowe says they are ahead of \
             schedule, and shows you the notebook again at a different page, \
             as though that settles it.",
        ],
        choices: &[
            Choice {
                label: "Ask how he does it",
                blurb: "Rowe will not say on the road. He says catch them up when they get there.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
            Choice {
                label: "Walk with them a while",
                blurb: "Gerald's pace, from here on. Everything slower, every plate worth double.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Trundle"),
                unmet: "",
            },
        ],
    },
    // Stands on the rung *after* Henpeck, which is where you are once he is
    // down. The theme's cutscene has already played by then - he has told you
    // he sold them, and told you twice - so this is the moment after that,
    // with him still on the floor and still talking.
    LadderEvent {
        id: "what-to-do-with-henpeck",
        at: 15,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "The Curator",
        title: "HE IS STILL TALKING",
        prose: &[
            "The Hollow King is on the floor of his own counting house with a \
             broken hip and an excellent view of the ceiling, and he is \
             talking.",
            "He has been talking since he went down. He has names. He has \
             routes. He has the clearance order for the works, filed \
             correctly, in triplicate, because he is exactly the sort of man \
             who would. All three copies are available for the obvious \
             consideration.",
            "He is having a marvellous time. He has asked you twice now \
             whether you are getting all this.",
        ],
        choices: &[
            Choice {
                label: "LET HIM TALK",
                blurb: "He wants a witness and a promise. One more loss before the run ends.",
                requires: Requirement::None,
                outcome: Outcome::Spare,
                unmet: "",
            },
            Choice {
                label: "FINISH IT",
                blurb: "The triplicate burns with him. You walk on angry and arrive angrier.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Avenged"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-toads-offer",
        at: 2,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Bone Archer",
        title: "TWO BY TWO",
        prose: &[
            "The Bog Toad has been sitting in this road since before you got \
             up this morning and has clearly used the time.",
            "It does not want to fight you. It wants the square thing in your \
             bag. It says square, it says two by two, it will not be moved on \
             the shape and it will not say what it is for.",
            "It counts the gold out onto a flat stone while you decide. It \
             counts out twice what the thing is worth. Then it counts the \
             whole pile again, gets the same number, and seems mildly \
             disappointed by that.",
        ],
        choices: &[
            Choice {
                label: "TAKE THE DEAL",
                blurb: "Hand over a 2x2 component. No fight, and twice the bounty on the stone.",
                requires: Requirement::LooseItemOfSize { w: 2, h: 2 },
                outcome: Outcome::BuyOff { times: 2 },
                unmet: "Nothing two by two in the bag. It checks. It counts the bag.",
            },
            Choice {
                label: "FIGHT IT ANYWAY",
                blurb: "It was going to be a fight before it was a negotiation.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-shrine-fork",
        at: 9,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Warded Idol",
        title: "THREE THINGS IN THE SHRINE",
        prose: &[
            "The Warded Idol stands in the shrine the way the Warded Idol \
             always stands in the shrine: plated to the eyeballs, wound to \
             the last click, entirely ready for you.",
            "There is also a hole in the back wall, which was not in the back \
             wall when you came in. Down the hole is a seed line, three floors \
             of it, and on the bottom floor an old analyst called Wenlock \
             prays on stone that cuts his knees. He has been waiting a long \
             while for somebody with shoulders.",
            "And there is a third thing behind the altar, asleep, with a \
             shell on it like a walnut. Nobody who works here will look \
             straight at it. The idol does not look at it either, and the \
             idol has no eyes.",
        ],
        choices: &[
            Choice {
                label: "FIGHT THE IDOL",
                blurb: "The rung as written. It has been ready since before you were born.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
            Choice {
                label: "FOLLOW THE THING YOU SOLD",
                blurb: "Three floors down, and Wenlock at the bottom with something to hand over.",
                requires: Requirement::Took("TAKE THE DEAL"),
                outcome: Outcome::Enter("the-crevice"),
                unmet: "You never sold it, so it never came this way, so there is no hole in the wall.",
            },
            Choice {
                label: "GO ROUND THE BACK",
                blurb: "A boss, this early, and it leaves something behind when it stops.",
                requires: Requirement::None,
                outcome: Outcome::FightInstead("The Dreaming Idiot"),
                unmet: "",
            },
        ],
    },

    // ------------------------------------------------------- the Unwinding
    //
    // Four stations, and every one of them fails forward: a refused choice
    // costs the reward and never the chain. The only thing that can actually
    // stop it is losing to the Herald, and even that waits two rungs and
    // offers again.
    //
    // The order is the chain: a word bought or won, a man who trades it for
    // another word, a gate that trades that one for a house, a light on a
    // ridge that trades the third for a foundry, and then the thing that has
    // been walking behind you the whole time.
    LadderEvent {
        id: "the-astronomer",
        // Rungs eighteen to twenty-nine. A window rather than a rung, because
        // a door priced in a rumour is a door you might arrive at holding
        // nothing - and stopping one short of thirty because the VIP area
        // stands there and a rung with two things on it is a rung where one of
        // them is a surprise.
        at: 28,
        trigger: Trigger::Whispered { rumour: "A Word About the Wrong Stars", from: 17 },
        blocked_by: &[],
        expects: "Null Sentinel",
        title: "THE ASTRONOMER",
        prose: &[
            "His name is Halloway and he has been thrown out of every \
             observatory on this road, and thrown out of all of them for the \
             same sentence, which he will say to you inside ninety seconds \
             whether or not you ask him to.",
            "The sentence is that eleven stars this year have fallen against \
             their own arcs. Not moved. Fallen - the way a thing falls when \
             something with mass goes past it - and always in the same \
             direction, and the direction is *down*, which is not a direction \
             the sky has.",
            "His lens is cracked from the middle out. He says that happened \
             on the eighth one, and offers the crack as the proof.",
        ],
        choices: &[
            Choice {
                label: "Hear him out",
                blurb: "It takes an hour and he does not stop to breathe. He is right.",
                requires: Requirement::None,
                outcome: Outcome::Give("A Word About the Cellar"),
                unmet: "",
            },
            Choice {
                label: "Buy the lens",
                blurb: "Three times a rung's bounty. It is cracked and he wants that for it.",
                requires: Requirement::None,
                outcome: Outcome::Give("The Cracked Lens"),
                unmet: "",
            },
            Choice {
                label: "Turn him in",
                blurb: "Somebody up the road pays for madmen. The bounty again, and nothing else.",
                requires: Requirement::None,
                outcome: Outcome::BuyOff { times: 0 },
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-locked-gate",
        at: 40,
        trigger: Trigger::Whispered { rumour: "A Word About the Cellar", from: 22 },
        blocked_by: &[],
        expects: "Sootmother",
        title: "THE LOCKED GATE",
        prose: &[
            "A gate, in good repair, hung on two posts, with a lock on it \
             that somebody oils. There is no wall on either side of it and no \
             road behind it, and the grass behind it has not been walked on \
             by anything with feet.",
            "The word Halloway gave you is not a key. It is a thing to say, \
             and he said it to himself twice before he said it to you, to be \
             sure he had it in the right order.",
            "There is a brass plate screwed to the middle post. It says \
             HOLLIS and then a number that is longer than a house number \
             needs to be.",
        ],
        choices: &[
            Choice {
                label: "Use the word",
                blurb: "It is four syllables and one of them is not a sound. The gate opens.",
                requires: Requirement::None,
                outcome: Outcome::RevealTown("the-manse"),
                unmet: "",
            },
            Choice {
                label: "Walk on",
                blurb: "Keep the word. The gate is patient and the gate knows the road.",
                requires: Requirement::None,
                outcome: Outcome::Defer { rungs: 3 },
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-glow-over-the-ridge",
        at: 45,
        trigger: Trigger::Whispered { rumour: "A Word About the Glow", from: 30 },
        blocked_by: &[],
        expects: "The Salt Wedding",
        title: "THE GLOW OVER THE RIDGE",
        prose: &[
            "There is a light over the ridge and it does not go out. It is not \
             a fire: a fire moves and this does not, and a fire goes out and \
             this has been on since before anybody on this road was born.",
            "Whatever is under it has been melting things down for a very long \
             time. The interesting part is what it has been melting: not ore, \
             which comes out of the ground, but *finished things*, which have \
             to be carried in - and the road to it goes only one way.",
            "A carter called Gull, coming the other way, says it is the \
             Slagworks, says it the way you would say a word you had been told \
             not to, and does not slow down while saying it.",
        ],
        choices: &[
            Choice {
                label: "Follow it",
                blurb: "Over the ridge and down. It is further away than it looks and then it is not.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::RevealTown("the-slagworks"),
                    Outcome::Flag("slagworks-known"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Ignore it",
                blurb: "It is a light. You have a road. The rung pays again for the trouble.",
                requires: Requirement::None,
                outcome: Outcome::BuyOff { times: 0 },
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-second-shadow",
        at: 48,
        // Standing because of what the run has done rather than where it is:
        // both towns found, and the antechamber walked.
        trigger: Trigger::WhenFlagged { flag: "threshold-cleared", from: 42 },
        blocked_by: &[],
        expects: "Gilt",
        title: "THE SECOND SHADOW",
        prose: &[
            "Your shadow is ahead of you on the road, which happens when the \
             light is behind you, and the light is not behind you.",
            "It is carrying your build. Not gear like yours - yours, the same \
             pieces in the same corners with the same one crooked, and it is \
             holding a lantern you have never owned and the lantern is what \
             is casting it.",
            "It has been walking at your pace since the Manse, which is a long \
             way back to have been keeping step without once being in front. \
             It has been waiting for you to be worth meeting.",
            "It has stopped waiting.",
        ],
        choices: &[
            Choice {
                label: "Face it",
                blurb: "Both of them at once. It knows what you are going to do, and does it first.",
                requires: Requirement::None,
                outcome: Outcome::Step(&THE_HERALD),
                unmet: "",
            },
            Choice {
                label: "Refuse",
                blurb: "It follows. It is in no hurry at all and it has the light.",
                requires: Requirement::None,
                outcome: Outcome::Defer { rungs: 3 },
                unmet: "",
            },
        ],
    },

    // -------------------------------------------- five that always happen
    //
    // No requirement, no rumour, no chain. A run that touches nothing meets
    // all five, which is the guarantee they exist for: the road is never bare,
    // and F1 is how a blind run learns the chain is in the game at all.
    //
    // Every figure in them is a multiple of the rung's own bounty rather than
    // a number. The spec's constants were written against a milestone table
    // that does not exist, and measured against the real economy they were two
    // and a half times everything a run had ever seen at one end and one
    // bounty at the other. See RECONCILIATION II #16.
    LadderEvent {
        id: "back-in-a-minute",
        at: 3,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Rust Golem",
        title: "BACK IN A MINUTE",
        prose: &[
            "A man on the road hands you something to hold. He says his name \
             is Wint. He hands it over the way you hand a thing to somebody \
             you have known for years, and you have not known him for any \
             years at all.",
            "He says he is going to get a drink and asks whether you are \
             coming. You are not coming. Wint goes anyway, off the road and up \
             the bank, and does not come back, and after a while it becomes \
             clear that not coming back was always the plan.",
            "The wrapping is a page torn out of a star chart. Somebody has \
             drawn a ring round a pub two towns up - the one that trades in \
             words rather than money - and written HE IS RIGHT under it, and \
             underlined it once.",
        ],
        choices: &[
            Choice {
                label: "Keep it",
                blurb: "Whatever Wint left you, it is yours now. So is the page it came in.",
                requires: Requirement::None,
                outcome: Outcome::Give("The Stranger's Parcel"),
                unmet: "",
            },
            Choice {
                label: "Leave it on the milestone",
                blurb: "Somebody will take it. Somebody pays for the trouble of not having it.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 3 },
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-teller",
        at: 10,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Mirror Fiend",
        title: "THE TELLER",
        prose: &[
            "There is a windowless store here the size of a county and its \
             entire sign says LARGE. The man who owns it is Ollam. He has been \
             carrying a story he is not able to put down, and he will pay to \
             say it to somebody who is still standing afterwards.",
            "He has tried three people. Ollam describes what happened to all \
             three of them in a level voice and with a great deal of \
             sympathy, and then asks whether you are interested.",
            "He is not lying and he is not selling. What the story costs to \
             hear is maximum health, for the rest of the run, and he says the \
             figure out loud before you answer, because a man who has done \
             this three times has stopped being delicate about it.",
        ],
        choices: &[
            Choice {
                label: "Hear it all",
                blurb: "The whole thing. He takes the best piece off his own back for it.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::Health(-100),
                    Outcome::Give("The Cracked Lens"),
                ]),
                unmet: "",
            },
            Choice {
                label: "Hear the short version",
                blurb: "Half of it, and half of what half of it costs, and he pays cash.",
                requires: Requirement::None,
                outcome: Outcome::All(&[Outcome::Health(-50), Outcome::Pay { times: 10 }]),
                unmet: "",
            },
            Choice {
                label: "Plug your ears",
                blurb: "Ollam takes it well. You keep what a head can hold, which turns out to matter.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-dispenser",
        // Sixteen rather than fifteen: Henpeck is still talking on that one,
        // and a rung with two doors on it is a rung where one of them is a
        // surprise.
        at: 16,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Salt Idol",
        title: "THE DISPENSER",
        prose: &[
            "A machine at the roadside, humming, plugged into nothing anybody \
             can find. Every light on it works. The whole front is lit up and \
             every row is stocked and there is a bottle wedged sideways \
             between the glass and the coil where somebody's last coin went.",
            "The one behind the red panel costs ten times what anything else \
             in the machine costs. There is a small brass plate under it and \
             the plate says WORTH IT, which is either a claim or a price.",
        ],
        choices: &[
            Choice {
                label: "One coin",
                blurb: "The cheapest row. It has been known to wedge.",
                requires: Requirement::Purse { times: 1 },
                outcome: Outcome::Gamble {
                    wins: 2,
                    outof: 3,
                    won: &Outcome::Give("The Stranger's Parcel"),
                    lost: &Outcome::FightAsWritten,
                },
                unmet: "The slot wants a coin and you are counting lint.",
            },
            Choice {
                label: "The red one behind the glass",
                blurb: "Ten times the price and it comes out with ceremony.",
                requires: Requirement::Purse { times: 10 },
                outcome: Outcome::Give("The Cracked Lens"),
                unmet: "The red one is behind the glass and the glass has a price on it.",
            },
            Choice {
                label: "Shake it",
                blurb: "Free. Two might fall at once. Somebody might hear.",
                requires: Requirement::None,
                outcome: Outcome::Gamble {
                    wins: 1,
                    outof: 2,
                    won: &Outcome::Pay { times: 2 },
                    lost: &Outcome::Count("shook-the-machine"),
                },
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "what-the-table-said",
        at: 23,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Crowned Hollow",
        title: "WHAT THE TABLE SAID",
        prose: &[
            "Salter's inn at this crossroads has one long table in it and \
             nobody sits at the middle of it, and nobody has for as long as \
             Salter has held the licence. He will tell you why if you ask, in \
             a tone that says he has stopped finding it interesting.",
            "Set a thing down at the centre and the table says what the thing \
             is trying to become. Not what it is. What it is *for*, which is \
             a different question and one nothing else in the world has ever \
             asked out loud.",
            "It has been right every time. It has also, twice, been right \
             about things nobody wanted to be right about.",
        ],
        choices: &[
            Choice {
                label: "Set a piece on it",
                blurb: "It speaks the task aloud, in full, and then halves it.",
                requires: Requirement::None,
                outcome: Outcome::GrantQuest(&TABLE_TASK),
                unmet: "",
            },
            Choice {
                label: "Keep your gear to yourself",
                blurb: "The table says nothing. It says nothing very pointedly.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-bird-problem",
        at: 26,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Vermin Sovereign",
        title: "THE BIRD PROBLEM",
        prose: &[
            "A courier called Pether hands you a memo. It is four pages, it \
             is CC'd to three governing bodies, and its subject line is THE \
             COMING TERRITORIAL WAR WITH THE BIRDS.",
            "Page two lists three potential outcomes. Two of them are bad. \
             The third is described as 'bad, but ours', which is the sort of \
             sentence that takes a committee eleven weeks.",
            "Page four is about armament and recommends racquets. The memo is \
             absurd from end to end and it is also, past the next ridge and \
             getting closer, entirely correct.",
        ],
        choices: &[
            Choice {
                label: "Arm up",
                blurb: "Pether has one spare and is glad to be rid of it.",
                requires: Requirement::None,
                outcome: Outcome::Give("Vicegrip Mold"),
                unmet: "",
            },
            Choice {
                label: "Pay the toll",
                blurb: "One rung's bounty and the flock parts. Nothing follows.",
                requires: Requirement::Purse { times: 1 },
                outcome: Outcome::FightAsWritten,
                unmet: "The birds do not take promises and Pether has stopped listening.",
            },
            Choice {
                label: "Ignore the memo",
                blurb: "It is four pages about birds. The next one arrives with company.",
                requires: Requirement::None,
                outcome: Outcome::Step(&THE_FLOCK),
                unmet: "",
            },
        ],
    },

    // ---------------------------------------------- the sign behind the sign
    //
    // The one door in the game that opens because of something you *declined*.
    // Keeping your head whole is what lets you see a second sign further back
    // and taller, which retroactively makes the Teller's third choice the
    // secret best one.
    LadderEvent {
        // Rung thirteen, and the number is load-bearing.
        //
        // It stood on forty-one, which made the door it opens unreachable:
        // EXTRA LARGE stands in the gap after rung fourteen, so a sign offered
        // twenty-seven rungs later revealed a town the road had long since
        // walked past. Every test passed - something *did* reveal it, and the
        // town *was* on the road - because nothing was asking whether the
        // reveal could happen in time. `a_reveal_can_happen_before_its_town`
        // asks now.
        //
        // Thirteen is two clear of THE TELLER at eleven, where the ears get
        // plugged, and one clear of the gate.
        id: "the-bigger-sign",
        at: 12,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Ashen Marshal",
        title: "THE BIGGER SIGN",
        prose: &[
            "Ollam's store is behind you and has been for some rungs now. The \
             thing that has been wrong about it the whole way is the sign.",
            "The sign says LARGE and it is nailed to a hoarding, and the \
             hoarding is not the building. Behind the hoarding, further back \
             and a good deal taller, there is a second sign, and the second \
             sign says EXTRA LARGE.",
            "Nobody else on this road has looked up. Ollam's story takes the \
             part of you that would have.",
        ],
        choices: &[
            Choice {
                label: "Follow the sign",
                blurb: "Off the road, round the hoarding, and in. It stands after the next rung.",
                requires: Requirement::Took("Plug your ears"),
                outcome: Outcome::RevealTown("extra-large"),
                unmet: "You heard Ollam out. Whatever part of you looks up is not looking up.",
            },
            Choice {
                label: "Forget you saw it",
                blurb: "Some knowledge is for selling, and somebody up the road is buying.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 3 },
                unmet: "",
            },
        ],
    },
    // ------------------------------------------------------ the destinations
    //
    // Neither of these stands on a rung. They are pushed onto the stack by a
    // pedestal, which is why they need no window and no trigger anybody will
    // ever meet by climbing - `Run::forced_event` is what asks them.
    LadderEvent {
        id: "the-thrumbus-race",
        at: 40,
        trigger: Trigger::WhenFlagged { flag: "never", from: 40 },
        blocked_by: &[],
        expects: "Sootmother",
        title: "THE BOLTER RACE",
        prose: &[
            "The 45th running, and the paddock is nine deep in people who have \
             an opinion about a bolter. A bolter is the fastest thing that has \
             ever been bred and looks, standing still, like a mistake.",
            "There is a book taking bets and a rail you can lean on and a \
             steward called Cobb who will let anybody run who signs the form, \
             and the form is one line long and the line is about teeth.",
        ],
        choices: &[
            Choice {
                label: "Back a runner",
                blurb: "Three rungs' worth on the nose. The jackpot is a claim on a whole board.",
                requires: Requirement::Purse { times: 3 },
                outcome: Outcome::Gamble {
                    wins: 1,
                    outof: 4,
                    won: &Outcome::ClaimTicket,
                    lost: &Outcome::FightAsWritten,
                },
                unmet: "The book has seen your purse and has gone back to the book.",
            },
            Choice {
                label: "Ride",
                blurb: "No stake. Finish, and take something off the paddock rail.",
                requires: Requirement::None,
                outcome: Outcome::Give("Sevenleague Sole"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "mole-town",
        at: 40,
        trigger: Trigger::WhenFlagged { flag: "never", from: 40 },
        blocked_by: &[],
        expects: "Sootmother",
        title: "MOLE TOWN",
        prose: &[
            "The highway ends at a town built entirely at ankle height. Not \
             ruined and not small - finished, to plan, at that height, with \
             three storeys of it above ground and every storey under your \
             knee.",
            "Everybody here is perfectly polite about the size of you. One of \
             them is called Tibb, who is older than the rest, carries a case \
             of tools, and has been looking at what you are carrying since you \
             arrived.",
        ],
        choices: &[
            Choice {
                label: "The little shop",
                blurb: "A curated shelf, at a permanent discount, because nobody your size shops here.",
                requires: Requirement::None,
                outcome: Outcome::OpenShop { shelves: MOLE_SHELF },
                unmet: "",
            },
            Choice {
                label: "The mole with the tools",
                blurb: "Four rungs' worth, and he takes a curse off a piece and keeps it.",
                requires: Requirement::Purse { times: 4 },
                outcome: Outcome::All(&[Outcome::Uncurse, Outcome::Count("moles-paid")]),
                unmet: "Tibb looks at your purse, and then at you, and says nothing at all.",
            },
        ],
    },

    // ------------------------------------------------ the nine structures
    //
    // Doors that are not doors. Everything above this line is a question with
    // two or three answers; these are shapes - an inspection that reads your
    // live board, an auction against a number nobody has seen, a handicap you
    // ask for, a passenger, a menu made of what you are holding, and a
    // counter nobody mentioned until it spoke.
    LadderEvent {
        id: "the-inspection",
        at: 19,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Bone Cantor",
        title: "THE INSPECTION",
        prose: &[
            "Nance Twiss sets a folding stool down in the road in front of \
             you, sits on it, and asks to see what you are wearing. She gives \
             her name and nothing else about herself, and treats that as the \
             whole of an introduction, which it is.",
            "She grades things. Rice for the trade board, and ropes before \
             that, and she says the principle does not change between them \
             and that people who think it does are the reason she still has \
             work.",
            "What she is looking for is whether the things you have built \
             agree with each other. She is entirely uninterested in whether \
             any of them is good.",
        ],
        choices: &[
            Choice {
                label: "Show her everything",
                blurb: "Three of your items speaking with one voice. She has a word for that.",
                requires: Requirement::AlignedItems(3),
                outcome: Outcome::Give("The Tally"),
                unmet: "She looks along the row twice and writes one word down and it is short.",
            },
            Choice {
                label: "Show her the good half",
                blurb: "Two that agree. Twiss grades the pair, and a graded thing is worth more.",
                requires: Requirement::AlignedItems(2),
                outcome: Outcome::Pay { times: 2 },
                unmet: "Nothing you own agrees with anything else you own. She notes it.",
            },
            // Declining is not nothing, and it is the one door in the game
            // where refusing is what pays: she is a labour professional being
            // refused, and on the way out she mentions where else that is
            // happening this month.
            Choice {
                label: "Decline the inspection",
                blurb: "She folds the stool and says, packing it, where else people are refusing this month.",
                requires: Requirement::None,
                outcome: Outcome::Give("A Word About the Picket"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-sealed-bid",
        at: 35,
        // Foundry business, so it needs the foundry known. The window opens on
        // the rung the Slagworks itself stands after, which is the earliest a
        // run can have been down there at all.
        trigger: Trigger::WhenFlagged { flag: "slagworks-known", from: 33 },
        blocked_by: &[],
        expects: "The Tallow Saint",
        title: "THE SEALED BID",
        prose: &[
            "The foundry auctions one lot a month and does it the old way. \
             Sarn writes the reserve down before anybody arrives, takes one \
             figure from each bidder, and holds no second round.",
            "Over the reserve and the lot is yours at the reserve. Under it and \
             the lot is somebody else's, and Sarn reads your figure out anyway, \
             to the room, in the voice he reads the winning one in.",
        ],
        choices: &[
            Choice {
                label: "Name a figure",
                blurb: "One number, written once. Sarn reads the reserve out either way.",
                requires: Requirement::Figure { min: 0, max: 5_000 },
                outcome: Outcome::SealedBid {
                    lots: &["the Skip Stone", "the Second Key", "the Appeal", "The Odometer"],
                },
                unmet: "A bid is a number and you have not got one.",
            },
            Choice {
                label: "Watch",
                blurb: "Somebody else wins it. Sarn reads the reserve out to the room.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-contract",
        at: 24,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Cog Priest",
        title: "THE CONTRACT",
        prose: &[
            "A man called Braddock sets a document on a milestone and does not \
             hand you a pen, on the grounds that people who need a pen handed \
             to them do not sign. He underwrites. He says the word the way \
             another man would say farriery.",
            "The clause is one line. Your gear runs cold for three rungs - all \
             of it, every slot, no exceptions and no early exit - and if you \
             are still upright at the end of the third one, they honour their \
             side.",
            "Their side is four rungs' worth of gold and one loss underwritten: \
             fall inside five rungs of collecting and the house eats that \
             fall. Braddock gives both figures standing here, says you collect \
             at rung 29, and does not offer to write any of it down.",
        ],
        choices: &[
            Choice {
                label: "Sign it",
                blurb: "Everything of yours runs cold for three rungs. All of it.",
                requires: Requirement::None,
                outcome: Outcome::Contract { rungs: 3 },
                unmet: "",
            },
            Choice {
                label: "Walk past the milestone",
                blurb: "Braddock does not look up. He was not expecting you to.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-payout",
        at: 28,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Null Sentinel",
        title: "THE PAYOUT",
        prose: &[
            "The house keeps its word and keeps it in an office, and the \
             office is a table at the roadside with Braddock behind it and a \
             ledger open at your column.",
            "He has been reading the column while you walked. He knows to the \
             rung how cold you were and for how long, and he did not have to \
             ask anybody.",
        ],
        choices: &[
            Choice {
                label: "Collect",
                blurb: "Four rungs' worth, and a name they will honour once if you fall.",
                requires: Requirement::Took("Sign it"),
                outcome: Outcome::All(&[Outcome::Underwrite, Outcome::Pay { times: 4 }]),
                unmet: "Your column is empty. He turns the ledger round so you can see that.",
            },
            Choice {
                label: "Walk on",
                blurb: "The table folds up behind you very quickly indeed.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-passenger",
        at: 41,
        trigger: Trigger::WhenFlagged { flag: "threshold-cleared", from: 39 },
        blocked_by: &[],
        expects: "The Quiet Hour",
        title: "THE PASSENGER",
        prose: &[
            "A courier called Larkin is carrying something in both hands on \
             the road and has been carrying it in both hands for a long time, \
             judging by the arms.",
            "It is a calf. It is a sacred calf and it is the size of a loaf \
             and it has to be at the Last Oxen before the road gets there, and \
             Larkin is not going to make it and knows it.",
            "It will not travel in a bag. It rides wrapped in sacking, sitting \
             somewhere on you in the open and taking up room you were using - \
             which is exactly the rent, and Larkin says so without being \
             asked. Everybody who sees it will call it a parcel and nobody \
             will be told otherwise.",
        ],
        choices: &[
            Choice {
                label: "Take it aboard",
                blurb: "Five rungs of dead cells - put it on a board, because a parcel in the tray is riding for free and pays nothing. Lose one fight and it is lost.",
                requires: Requirement::None,
                outcome: Outcome::Passenger { rungs: 5, pays: "An Unwound Mainspring" },
                unmet: "",
            },
            Choice {
                label: "Wish him luck",
                blurb: "Larkin says thank you and means it, which does not help either of you.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-buyer",
        at: 31,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "The Long Mirror",
        title: "THE BUYER",
        prose: &[
            "The man who buys is called Vell and he works out of a hired room \
             with the door open and one chair set exactly far enough back that \
             you have to walk in to sit on it.",
            "He does not sell. He buys three things and only those three: a \
             word somebody told you, a title you are, and a hundred of your \
             maximum health. None of them is gear and none of them has a price \
             anywhere else.",
            "He is entirely honest about it. Vell says what each one is worth, \
             to the gold, and the number is correct every time - and that, \
             rather than the open door or the chair, is what makes the room \
             hard to be in.",
        ],
        choices: &[
            Choice {
                label: "Sell him a word",
                blurb: "Six rungs' worth. The door it opened stays shut for good.",
                requires: Requirement::HoldingRumour,
                outcome: Outcome::All(&[Outcome::SellWord, Outcome::Pay { times: 6 }]),
                unmet: "You are carrying nothing anybody has told you. He is sorry to hear it.",
            },
            Choice {
                label: "Sell him a title",
                blurb: "Eight rungs' worth for something you are. He does not ask which.",
                requires: Requirement::Classes(1),
                outcome: Outcome::All(&[Outcome::SellTitle, Outcome::Pay { times: 8 }]),
                unmet: "You are nobody in particular yet, which he says without unkindness.",
            },
            Choice {
                label: "Sell him a hundred of your maximum",
                blurb: "Ten rungs' worth. It does not come back and he says so twice.",
                requires: Requirement::None,
                outcome: Outcome::All(&[Outcome::Health(-100), Outcome::Pay { times: 10 }]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-fork",
        at: 36,
        trigger: Trigger::WhenFlagged { flag: "slagworks-known", from: 33 },
        blocked_by: &[],
        expects: "Hollowmarch",
        title: "THE FORK",
        prose: &[
            "The seam forks. One way goes down to a mouth that is boarded from \
             the outside; the other comes back up into a yard where the \
             Slagworks has laid a shelf out for whoever arrives next.",
            "Both are yours. Ossery says so and shrugs at being asked, because \
             from where he is standing there is no question here at all.",
            "There is a question. It is which one first, and it is the whole \
             question: a shelf before a fight is different gear, and a shelf \
             after one is different money.",
        ],
        choices: &[
            Choice {
                label: "The shelf, then the seam",
                blurb: "Buy first. Whatever you find down there, you find it carrying this.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::OpenShop { shelves: FOUNDRY_SHELF },
                    Outcome::StartDungeon("the-under-mine"),
                ]),
                unmet: "",
            },
            Choice {
                label: "The seam, then the shelf",
                blurb: "Fight first. Whatever you come back up with, you spend it here.",
                requires: Requirement::None,
                outcome: Outcome::All(&[
                    Outcome::StartDungeon("the-under-mine"),
                    Outcome::ShopAfter { shelves: FOUNDRY_SHELF },
                ]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-foundry-remembers",
        at: 46,
        trigger: Trigger::WhenFlagged { flag: "slagworks-known", from: 44 },
        blocked_by: &[],
        expects: "Nine of Ashes",
        title: "THE FOUNDRY REMEMBERS",
        prose: &[
            "The glow is behind you and a long way behind you, and there is a \
             man at the roadside in Slagworks overalls who has walked further \
             than that to be standing here. His name is Rusk and Ossery sent \
             him.",
            "He says the foundry keeps a book. Rusk says it the way you say a \
             thing you have been asked to pass on exactly, and then he passes \
             it on exactly.",
            "Nobody mentioned a book. Nobody mentioned that the crucible was \
             counting, either, and it was.",
        ],
        choices: &[
            Choice {
                label: "\"We kept your best\"",
                blurb: "You used the crucible once, and the foundry has been holding one back.",
                requires: Requirement::Counter { what: "crucible-melts", at_least: 1 },
                outcome: Outcome::Give("The Cracked Lens"),
                unmet: "He checks the page twice. Your column has nothing in it.",
            },
            Choice {
                label: "Say nothing",
                blurb: "Rusk nods, notes it, and walks back. Prices run ahead of you after that.",
                requires: Requirement::None,
                outcome: Outcome::Markup(10),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "through-the-cracked-lens",
        at: 47,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "The Last Light",
        title: "THROUGH THE CRACKED LENS",
        prose: &[
            "This high up the air stops arguing with light and the lens does \
             what Halloway said it would do, which is the thing he was thrown \
             out of every observatory on this road for describing.",
            "It does not magnify. It focuses - on a thing rather than at a \
             distance - and what comes into focus is whatever is standing \
             between you and the top, in the gear it is standing in, from \
             here.",
        ],
        choices: &[
            Choice {
                label: "Look through it",
                blurb: "Every board ahead of you, from the loadout screen, for the rest of it.",
                requires: Requirement::Holding("The Cracked Lens"),
                // Scouting, and a note that you looked.
                //
                // What you see through it is the thing past the top. A run
                // that never looked has no idea there is anything past
                // Francis, and the door at the end does not stand for it -
                // which is the difference between an ending you earned and one
                // that simply happened to you.
                outcome: Outcome::All(&[Outcome::Scout, Outcome::Flag("looked-through-the-lens")]),
                unmet: "You would need the lens. Halloway offered it to you once, at a price.",
            },
            Choice {
                label: "Keep your eyes on the road",
                blurb: "There is a strong argument for not knowing. It is not a good one.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },

    // ---------------------------------------------- the three standalone pairs
    LadderEvent {
        id: "the-wizards-thirst",
        at: 30,
        trigger: Trigger::Whispered { rumour: "A Word About the Thirsty Wizard", from: 7 },
        blocked_by: &[],
        expects: "Weeping Idol",
        title: "THE WIZARD'S THIRST",
        prose: &[
            "Sam the Wise wants vials. He wants cans as well and will take \
             either and he is extremely specific about which shapes and \
             entirely unwilling to say why.",
            "He has a hoard. He has shown three people the hoard and all three \
             of them describe the same thing, which is a great many empty \
             vessels stacked with real care in a room with the curtains shut.",
        ],
        choices: &[
            Choice {
                label: "Trade him one",
                blurb: "Triple what anybody pays, and he throws in a second chance at something.",
                requires: Requirement::LooseItemOfSize { w: 1, h: 2 },
                outcome: Outcome::All(&[Outcome::Pay { times: 3 }, Outcome::Give("the Appeal")]),
                unmet: "Nothing the right shape. He looks at your bag until you move it.",
            },
            Choice {
                label: "Refuse him",
                blurb: "He takes it well, and then something of yours runs cold.",
                requires: Requirement::None,
                outcome: Outcome::Chill,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-picket-line",
        at: 38,
        // From nineteen, not twelve: the word is handed over by THE INSPECTION,
        // which stands on rung twenty and nowhere else. A window that opens
        // seven rungs before its own key can exist is not wrong so much as
        // misleading - the route map draws it and the strip counts it, and a
        // player reads a door that is not there.
        trigger: Trigger::Whispered { rumour: "A Word About the Picket", from: 19 },
        blocked_by: &[],
        expects: "Gallowglass",
        title: "THE PICKET LINE",
        prose: &[
            "The arena workers have downed tools. There are six demands \
             chalked on the board at the gate, they are numbered, and every \
             one of them is about the sand.",
            "Demand four is about armour. Demand four is about *your* armour, \
             which is an odd thing to find on somebody else's picket, and it \
             is there because Nettle has been raking up after people like you \
             for eleven years and put it on the board herself.",
        ],
        choices: &[
            Choice {
                label: "Honor the line",
                blurb: "The next town's shop is shut to you. Demand four is honoured too.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Unionized"),
                unmet: "",
            },
            Choice {
                label: "Cross it",
                blurb: "Two rungs' worth in your hand, and your name on the board at the gate.",
                requires: Requirement::None,
                outcome: Outcome::All(&[Outcome::Pay { times: 2 }, Outcome::Count("crossed")]),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-exhibition",
        at: 33,
        trigger: Trigger::Whispered { rumour: "A Word About the Exhibition", from: 18 },
        blocked_by: &[],
        expects: "The Last Gearwright",
        title: "THE EXHIBITION",
        prose: &[
            "Dorn and Ilder were the two finest players the ring game ever \
             had. They are both retired now and are, between them, about as \
             bored as it is possible for two people to be.",
            "They want a demonstration bout. Two of them, one of you, in front \
             of whoever is passing, at exhibition stakes - which means nobody \
             is going to be seriously hurt and everybody is going to be seen.",
        ],
        choices: &[
            Choice {
                label: "Give them a bout",
                blurb: "Top of the bill, two on one. Losing costs this rung's purse and nothing else.",
                requires: Requirement::AssembledOfRarity(crate::rating::Rarity::Rare),
                outcome: Outcome::All(&[
                    Outcome::Claim("Showstopper"),
                    Outcome::Step(&THE_SHOWFIGHTERS),
                ]),
                unmet: "Ilder looks at what you have built and is far too polite about it.",
            },
            Choice {
                label: "Decline politely",
                blurb: "Dorn says that is fair. Ilder does not say that is fair.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // ---- THE SWITCHYARD ---------------------------------------------------
    //
    // Four doors across the rung-19-to-35 stretch, seeded by two words. Every
    // index is zero-based and the displayed rung is one more; every gold
    // figure is a multiple of the standing rung's bounty rather than a number,
    // which `acceptance::e6_7` lints.
    //
    // The free indices between Kettleworks (after 17) and the Slagworks (after
    // 33) are 18, 20, 25, 27, 32 and 34, and these four take four of them.
    LadderEvent {
        id: "the-timetable",
        at: 20,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Ember Wisp",
        title: "THE TIMETABLE",
        // Unconditional, for the reason the Unwinding's F1 is: a chain most
        // runs never see the start of is a chain nobody walks.
        //
        // **Index 20, not 18.** The spec puts it at 18 and argues that
        // Kettleworks' gate can share the rung because the stack pops the gate
        // first - which is true of a *fountain* at index 7 and is not allowed
        // of an event: `town::no_town_shares_a_rung_with_an_event` has refused
        // that since before this mission, on the grounds that both want the
        // screen and there is no sensible order for it. Kettleworks stands
        // after 17, so its gate is met on 18. Index 20 is the only other rung
        // in the stretch that is free of both a scheduled event and a town.
        prose: &[
            "Hesketh sells timetables off a folding table at the side of the \
             road, for a line that closed before the road was cut, and she \
             sells them at the printed price because she has never seen a \
             reason to change it.",
            "The times in them are being kept, and Hesketh has checked. Every \
             train on the sheet leaves the yard when the sheet says, and the \
             yard is under your feet, and you have not heard a train because \
             there are no trains, and the times are being kept anyway.",
            "She will sell you one. She would also, if you had something small \
             she could use, take that instead, because the money is not the \
             point and never has been.",
        ],
        choices: &[
            Choice {
                label: "Buy a timetable",
                blurb: "A rung's bounty. The printed price, which she is proud of.",
                requires: Requirement::Purse { times: 1 },
                outcome: Outcome::Give("A Word About the Sidings"),
                unmet: "Hesketh does not do credit, and says so kindly.",
            },
            Choice {
                label: "Trade her something small",
                blurb: "A loose one-by-one. She turns it over twice and puts it in her coat.",
                requires: Requirement::LooseItemOfSize { w: 1, h: 1 },
                outcome: Outcome::Give("A Word About the Sidings"),
                unmet: "You have nothing small enough to be worth her while.",
            },
            Choice {
                label: "Leave the table alone",
                blurb: "The times will go on being kept without you.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-signal-box",
        at: 24,
        // The window opens on the rung after the timetable, so the earliest a
        // run can carry the word is the earliest the box can stand. 21 to 24,
        // and a rumour door goes first on its rung in any case. The window
        // shuts before the Manse (after 24) for the reason the astronomer's
        // shuts before the VIP area: a rung with two doors on it is a rung
        // where one of them is a surprise, and a town gate is a third.
        trigger: Trigger::Whispered { rumour: "A Word About the Sidings", from: 21 },
        blocked_by: &[],
        expects: "Cog Priest",
        title: "THE SIGNAL BOX",
        prose: &[
            "The signal box stands on legs over the cutting and the man in it \
             is called Ambrose and he does not look up, because the 21:14 is \
             due and the 21:14 is more important than you are, whatever you \
             are.",
            "He throws the lever. Below you, in the dark, something heavy \
             moves a foot and stops, and Ambrose writes a time in a book, and \
             the time is 21:14, and it is the right time.",
            "He will set the points for you if you ask. He sets them one way. \
             He has always set them one way, and nobody has ever asked him \
             which, and he would like it noted that you did not ask either.",
        ],
        choices: &[
            Choice {
                label: "Ask him to throw the points",
                blurb: "He writes you into the book. The yard is open, and he will not say what is in it.",
                requires: Requirement::None,
                outcome: Outcome::Give("A Word About the Points"),
                unmet: "",
            },
            // Pays and closes the chain this run, which is the "turn him in"
            // shape: the word is spent on a bounty rather than on a door, and
            // that is a real offer for a run one component short.
            Choice {
                label: "Ask what runs on the 21:14",
                blurb: "Nothing. He knew that. The bounty again, for a question worth asking.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 1 },
                unmet: "",
            },
            Choice {
                label: "Leave him to it",
                blurb: "The 21:22 is due. He has already stopped seeing you.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-turntable",
        at: 27,
        // Index 25 is bare and 27 is bare; 26 carries THE BIRD PROBLEM, which
        // is scheduled and is asked after this one. The window shuts one short
        // of 28, where THE PAYOUT and the astronomer's deadline both stand.
        trigger: Trigger::Whispered { rumour: "A Word About the Points", from: 25 },
        blocked_by: &[],
        expects: "Obsidian Colossus",
        title: "THE TURNTABLE",
        prose: &[
            "The turntable is at the bottom of the cutting and it is turning. \
             Nobody is on it. It turns a quarter of the way round, and stops, \
             and a bell rings once in the dark, and it turns back.",
            "The yard goes off from it in two directions and both of them are \
             unlit, and on the wall of the turntable pit somebody has painted \
             DOWN LINE and UP LINE with an arrow each, and under the arrows, \
             smaller, the words BUFFER STOPS AT THE END OF BOTH.",
            "Ambrose has thrown the points. You can hear them, thrown, \
             somewhere out past the lamp. Which way he threw them is a thing \
             you find out by walking.",
        ],
        choices: &[
            Choice {
                label: "Step onto the turntable",
                blurb: "Four fights on either line, and the line is your choice at the first points. What is at the buffer stop stays there until somebody takes it.",
                requires: Requirement::None,
                outcome: Outcome::Enter("the-switchyard"),
                unmet: "",
            },
            Choice {
                label: "Sell the timetable to the man in the pit",
                blurb: "There is a man in the pit who collects them. The bounty three times, and the yard stays shut.",
                requires: Requirement::None,
                outcome: Outcome::Pay { times: 3 },
                unmet: "",
            },
            Choice {
                label: "Come back up",
                blurb: "The turntable turns a quarter of the way round, and stops, and turns back.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-last-train",
        at: 33,
        // Part E's E-1, taken as recommended: option (c). This was written as a
        // `WhenFlagged` door waiting on `switchyard-cleared`, and its third
        // choice is for the run that *sold* the sheet - which sets no flag and
        // would never have met the door. `Trigger` has no "either flag".
        //
        // So it stands for everybody and greys what a run did not earn, which
        // is the VIP area's shape ("the rope does not move") and teaches that
        // the yard existed. A chain nobody can tell they missed is a chain
        // they will not look for next run.
        //
        // **Index 33, not 32.** (c) says 32; High Wick stands after 31, so its
        // gate is met on 32 and `town::no_town_shares_a_rung_with_an_event`
        // refuses it. 33 is the next rung that is not a gate, it is still past
        // the pedestal - which is the whole reason the door stands late, so
        // the count is read after a run has had somewhere to spend its orbs -
        // and it shares the rung with THE EXHIBITION, which is a window rather
        // than an address and is a pairing the road already has three of.
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "The Last Gearwright",
        title: "THE LAST TRAIN",
        prose: &[
            "Ambrose is on the road. He has never been on the road. He has the \
             book under his arm and the lever is not in the box any more, \
             because the box is not there any more, because the last train ran \
             at 02:40 this morning and it took the box with it.",
            "He wants to know how far down the yard you went, and Ambrose \
             writes the answer in the book without looking at the page, in a \
             hand that has written the same eleven times a day for longer than \
             there has been a road.",
            "There was one more train on the sheet. There was always one more \
             train on the sheet. Ambrose says the sheet was right about that \
             too.",
        ],
        choices: &[
            Choice {
                label: "Tell him both lines",
                blurb: "You walked the yard twice. He closes the book. Three times the bounty, and your next loss inside five rungs does not count.",
                requires: Requirement::Counter { what: "sidings-cleared", at_least: 2 },
                outcome: Outcome::All(&[Outcome::Pay { times: 3 }, Outcome::Underwrite]),
                unmet: "You walked one line. Ambrose knows, because he threw the points.",
            },
            Choice {
                label: "Tell him one line",
                blurb: "He writes it down. The bounty again, and a nod.",
                requires: Requirement::Counter { what: "sidings-cleared", at_least: 1 },
                outcome: Outcome::Pay { times: 1 },
                unmet: "You did not go down. He knows that too.",
            },
            Choice {
                label: "Tell him you never found it",
                blurb: "He has heard that before and does not believe it, and writes it down anyway. There is no bounty for this and it costs you nothing.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // ---- past the top ---------------------------------------------------
    //
    // The one door that stands after the ladder ends. Off the road entirely:
    // `flag: "never"` is the sentinel for a door nothing on a rung can reach,
    // and `settle` pushes this one through `forced_event` the moment Francis
    // goes down for a run that looked through the lens.
    //
    // `at` and `expects` name Francis because that is the last rung there is
    // and `every_event_stands_where_it_thinks_it_does` has to be able to check
    // something. Where it actually stands is after him.
    LadderEvent {
        id: "the-unwound",
        at: 49,
        trigger: Trigger::WhenFlagged { flag: "never", from: 49 },
        blocked_by: &[],
        expects: "Francis",
        title: "THE ROAD PAST FRANCIS",
        prose: &[
            "Francis is down and the road does not stop. It goes on past him \
             for about forty yards and then it goes down, and the going down \
             is the part the lens showed you.",
            "Merrik said there was a road under the road and that it would \
             still be here when you got this far, and Merrik was right about \
             it the way he was right about everything, which is to say he \
             said it once and did not explain. Whatever is at the bottom has \
             been unwinding the whole time you were climbing.",
            "The mainspring in your hand is the wrong shape for any lock on \
             this road. It is the right shape for that.",
        ],
        choices: &[
            Choice {
                label: "Go down",
                blurb: "The thing at the bottom is harder than the man you just put down, and it does not stop when you do.",
                requires: Requirement::Holding("An Unwound Mainspring"),
                outcome: Outcome::FightInstead("THE UNWOUND"),
                unmet: "The way down wants a mainspring, and yours went somewhere else.",
            },
            Choice {
                label: "Turn round",
                blurb: "The road behind you is the whole game and you have just finished it.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
];

/// **Every** event standing on `rung`, given what the run has managed so far.
///
/// `best_fight_ms` is the quickest win the run has had, or `None` if it has
/// not won one yet. An earned event fires on the first rung after it qualifies
/// rather than on a fixed one, so it turns up when you have earned it.
///
/// This was a `find` and returned one. That is the whole of a bug the owner
/// hit at rung three: a quick kill in the shallow end opens THE CASINO, whose
/// window is rungs two to nine, and TWO BY TWO stands on rung three - so the
/// casino was returned, the toad was never asked, and answering the casino
/// left the rung empty. A scheduled event stands on exactly one rung and is
/// gone if it is not asked there, so an earned window passing over one used to
/// delete it.
///
/// **Scheduled first**, which is the ordering rule and not an accident. A
/// `Trigger::Rung` event has one rung and expires; an earned one roams a
/// window and will still be there next rung. Ask the one that is about to be
/// lost. Within each group the table's own order stands, which is what keeps
/// THE CASINO ahead of GERALD - they are alternatives, and answering the first
/// shuts the second through `blocked_by`.
pub fn standing_at(
    rung: usize,
    best_fight_ms: Option<u32>,
    worst_fight_ms: Option<u32>,
    answered: &[&'static str],
) -> Vec<&'static LadderEvent> {
    let mut out: Vec<&'static LadderEvent> = EVENTS.iter().filter(|e| {
        // Answered shuts a door as hard as `blocked_by` does. This parameter
        // used to gate only the second, which reads as a filter that does
        // nothing of the sort - the caller filtered its own id afterwards and
        // both halves of the question lived in different files.
        if answered.contains(&e.id) || e.blocked_by.iter().any(|id| answered.contains(id)) {
            return false;
        }
        match e.trigger {
            Trigger::Rung => e.at == rung,
            Trigger::QuickKill { within_ms, from } => {
                (from..=e.at).contains(&rung) && best_fight_ms.is_some_and(|ms| ms < within_ms)
            }
            Trigger::SlowKill { over_ms, from } => {
                (from..=e.at).contains(&rung) && worst_fight_ms.is_some_and(|ms| ms > over_ms)
            }
            // Not answerable from here: one is about the board and the run,
            // the other about what the run has done, and this knows about
            // neither. `Run::pending_event` answers both.
            Trigger::Whispered { .. } | Trigger::WhenFlagged { .. } => false,
        }
    }).collect();
    // A stable partition: scheduled, then earned, each in table order.
    out.sort_by_key(|e| !matches!(e.trigger, Trigger::Rung));
    out
}

/// Every choice whose outcome sets `flag`, as (event id, choice label).
///
/// The reverse index the string-keyed flags buy. A flag waited on by a door
/// and set by nothing is a chain with a station nothing reaches, which is the
/// failure a chain is most exposed to and the one hardest to see by reading.
pub fn set_by(flag: &str) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for e in EVENTS {
        for c in e.choices {
            // Through `every_outcome`: the glow sets its flag alongside a town
            // reveal, and a flag set inside an `All` is set just as hard as one
            // set on its own.
            if every_outcome(&c.outcome).iter().any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
            {
                out.push((e.id, c.label));
            }
        }
    }
    out
}

/// Everything a choice actually does, `All` and `Gamble` unpacked.
///
/// `All` is a list of outcomes and `Gamble` is two of them, so a lint that
/// matches on `c.outcome` alone sees a composite and nothing inside it. That
/// blind spot has cost this file twice - a class claimed inside an `All` read
/// as a class no door hands out, and a fountain nearly poured it. Everything
/// that asks "does any door do X" asks it through here.
pub fn every_outcome(o: &'static Outcome) -> Vec<&'static Outcome> {
    let mut out = vec![o];
    match o {
        Outcome::All(each) => out.extend(each.iter().flat_map(every_outcome)),
        Outcome::Gamble { won, lost, .. } => {
            out.extend(every_outcome(won));
            out.extend(every_outcome(lost));
        }
        _ => {}
    }
    out
}

/// Every flag any door in the game waits on.
pub fn flags_waited_on() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in EVENTS {
        for c in e.choices {
            if let Requirement::Flag(f) = c.requires {
                if !out.contains(&f) {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// Every event a rumour is the condition on, in table order.
///
/// The reverse of `Rumour::opens`, and worth having both ways round. Forwards
/// answers "what does this rumour do"; backwards answers "is this rumour for
/// anything at all", which is the question that catches dead content. Built by
/// walking `EVENTS` rather than kept in a table, so an event that moves takes
/// its rumour's description with it.
pub fn conditioned_by(rumour: &str) -> Vec<&'static LadderEvent> {
    EVENTS
        .iter()
        .filter(|e| matches!(e.trigger, Trigger::Whispered { rumour: r, .. } if r == rumour))
        .collect()
}

impl LadderEvent {
    /// Where this stands, in a phrase.
    ///
    /// A scheduled event stands on exactly one rung. An earned one roams a
    /// window and `at` is its deadline rather than its address, which is the
    /// one place the field's name lies and the one place it matters.
    pub fn where_it_stands(&self) -> String {
        match self.trigger {
            Trigger::Rung => format!("rung {}", self.at + 1),
            Trigger::QuickKill { from, .. }
            | Trigger::SlowKill { from, .. }
            | Trigger::Whispered { from, .. }
            | Trigger::WhenFlagged { from, .. } => {
                if from == self.at {
                    format!("rung {}", self.at + 1)
                } else {
                    format!("rungs {} to {}", from + 1, self.at + 1)
                }
            }
        }
    }
}

impl Requirement {
    /// What this asks for, in a plain sentence.
    ///
    /// Not the same thing as `Choice::unmet`, and both are needed. `unmet` is
    /// flavour written for the moment after you have tried - "Merrik does not
    /// move the rope" - and it is the right register for a door that has just
    /// refused you. This is the plain statement *before* an attempt: hover a
    /// greyed choice and it tells you what would open it.
    ///
    /// Two authored events once sat behind four gates with no feedback of any
    /// kind and the result was that nobody ever saw them. `Condition::describe`
    /// was the answer for rumours; this is the same answer for choices.
    ///
    /// The nouns are canonical. The theme layer swaps them on the way to a
    /// screen, which is why this returns the engine's words and not the
    /// player's - the CLI has to be able to print the same sentence.
    pub fn describe(&self) -> String {
        match self {
            Requirement::None => String::new(),
            Requirement::LooseItemOfSize { w, h } => {
                format!("Requires: a loose component {} by {}", w, h)
            }
            Requirement::Took(label) => format!("Requires: having chosen \"{}\" earlier", label),
            Requirement::Holding(name) => format!("Requires: {}", name),
            Requirement::Flag(what) => format!("Requires: {}", what.replace('-', " ")),
            Requirement::Counter { what, at_least } => {
                format!("Requires: {} at least {} times", what.replace('-', " "), at_least)
            }
            Requirement::CountyTiles { region, at_least } => format!(
                "Requires: {} tiles cleared in the {}",
                at_least,
                match region {
                    crate::county::Region::North => "north",
                    crate::county::Region::Middle => "middle",
                    crate::county::Region::South => "south",
                }
            ),
            Requirement::CountyCleared(chain) => {
                format!("Requires: {:?} finished", chain).to_lowercase().replace("requires:", "Requires:")
            }
            Requirement::AssembledOfRarity(r) => {
                format!("Requires: an assembled {}", r.name())
            }
            Requirement::AlignedItems(n) => {
                format!("Requires: {} assembled items sharing an alignment", n)
            }
            Requirement::Figure { min, max } => {
                format!("Name a figure between {} and {}", min, max)
            }
            Requirement::Purse { times } => {
                format!("Costs {} times this rung's bounty", times)
            }
            Requirement::HoldingRumour => "Requires: a word you have not spent".into(),
            Requirement::HoldingOrb => "Requires: an Orb of Travel, and it stays here".into(),
            Requirement::ThePaleIsReady => {
                "Requires: six tiles in each third of the county, two boundary stones, and an \
                 Orb of Travel"
                    .into()
            }
            Requirement::Classes(n) => format!("Requires: {} title(s)", n),
        }
    }

    /// Does `shape` - a component's footprint, in cells - satisfy this?
    pub fn met_by_shape(self, cells: &[(u8, u8)]) -> bool {
        match self {
            Requirement::None => true,
            // Everything else is answered by the run rather than by a shape.
            Requirement::Took(_)
            | Requirement::Holding(_)
            | Requirement::Flag(_)
            | Requirement::Counter { .. }
            | Requirement::CountyTiles { .. }
            | Requirement::CountyCleared(_)
            | Requirement::AssembledOfRarity(_)
            | Requirement::AlignedItems(_)
            | Requirement::Purse { .. }
            | Requirement::HoldingRumour
            | Requirement::HoldingOrb
            | Requirement::ThePaleIsReady
            | Requirement::Classes(_)
            | Requirement::Figure { .. } => true,
            Requirement::LooseItemOfSize { w, h } => {
                let (mut mx, mut my) = (0u8, 0u8);
                for &(x, y) in cells {
                    mx = mx.max(x);
                    my = my.max(y);
                }
                let (fw, fh) = (mx + 1, my + 1);
                cells.len() as u32 == w as u32 * h as u32
                    && ((fw == w && fh == h) || (fw == h && fh == w))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::LADDER;

    /// An event points at a rung by number and names the creature it expects
    /// to find there. Renumbering the ladder - which has happened twice - must
    /// not silently leave an event in front of the wrong fight.
    #[test]
    fn every_event_stands_where_it_thinks_it_does() {
        for e in EVENTS {
            let m = LADDER.get(e.at).unwrap_or_else(|| panic!("{}: rung {} is off the end", e.id, e.at));
            assert_eq!(m.name, e.expects, "{} expects {} at rung {}", e.id, e.expects, e.at + 1);
        }
    }

    /// A scheduled event has a rung to itself.
    ///
    /// `event::at` returns the *first* match, so two events that both stand on
    /// one rung means one of them silently never fires. Earned events are the
    /// exception and have to be: they roam a window rather than standing
    /// anywhere, several can be open at once, and which one is asked is
    /// settled by the order they are written in and by `blocked_by`. That is
    /// deliberate - the casino comes first, so a run that earned both doors is
    /// offered the casino and answering it shuts the other.
    #[test]
    fn no_two_scheduled_events_stand_on_the_same_rung() {
        let mut seen = Vec::new();
        for e in EVENTS.iter().filter(|e| matches!(e.trigger, Trigger::Rung)) {
            assert!(!seen.contains(&e.at), "two events on rung {}", e.at + 1);
            seen.push(e.at);
        }
    }

    /// A scheduled event inside an earned one's window is not eaten by it.
    ///
    /// This used to assert the opposite thing, for the opposite reason:
    /// `event::at` was a `find`, exactly one event came back, and the test
    /// pinned the *write order* so that the earned one won. Which meant a
    /// quick kill in the shallow end deleted TWO BY TWO - the casino's window
    /// is rungs two to nine, the toad stands on rung three, and the toad was
    /// simply never asked. The owner hit it on rung three.
    ///
    /// `standing_at` returns all of them now, so the question is no longer
    /// which one wins but which is asked first, and the answer is the one
    /// about to expire.
    #[test]
    fn an_earned_window_does_not_eat_a_scheduled_rung() {
        for earned in EVENTS.iter() {
            if !matches!(earned.trigger, Trigger::QuickKill { .. } | Trigger::SlowKill { .. }) {
                continue;
            }
            let window = earned.trigger.from()..=earned.at;
            for sched in EVENTS.iter() {
                if !matches!(sched.trigger, Trigger::Rung) || !window.contains(&sched.at) {
                    continue;
                }
                // Standing on the shared rung, with the earned one qualified.
                let both = standing_at(sched.at, Some(0), Some(u32::MAX), &[]);
                assert!(
                    both.iter().any(|e| e.id == sched.id),
                    "{} stands on rung {} and {}'s window swallowed it",
                    sched.id,
                    sched.at + 1,
                    earned.id
                );
                assert_eq!(
                    both.first().map(|e| e.id),
                    Some(sched.id),
                    "on rung {} the scheduled door is the one that expires and is asked first",
                    sched.at + 1
                );
            }
        }
    }

    /// The rung the owner reported, end to end.
    #[test]
    fn rung_three_offers_the_toad_and_the_casino_and_loses_neither() {
        // A quick kill anywhere in the shallow end opens the casino, whose
        // window covers rung three, where TWO BY TWO stands.
        let standing = standing_at(2, Some(1_000), None, &[]);
        let ids: Vec<&str> = standing.iter().map(|e| e.id).collect();
        assert_eq!(
            ids,
            vec!["the-toads-offer", "the-casino"],
            "both stand, and the one that expires is asked first"
        );

        // Answering one leaves the other exactly where it was.
        let after = standing_at(2, Some(1_000), None, &["the-toads-offer"]);
        assert_eq!(after.iter().map(|e| e.id).collect::<Vec<_>>(), vec!["the-casino"]);
        let other = standing_at(2, Some(1_000), None, &["the-casino"]);
        assert_eq!(other.iter().map(|e| e.id).collect::<Vec<_>>(), vec!["the-toads-offer"]);
    }

    /// The two shallow-end doors are still alternatives.
    ///
    /// They are both earned and both can qualify at once; the casino is
    /// written first and `blocked_by` shuts GERALD once it is answered. That
    /// survived the change from `find` to a list and is worth saying so.
    #[test]
    fn the_two_shallow_doors_are_still_one_question() {
        let both = standing_at(8, Some(1_000), Some(20_000), &[]);
        let ids: Vec<&str> = both.iter().map(|e| e.id).collect();
        assert!(ids.contains(&"the-casino"), "the casino qualified and is not there: {ids:?}");
        assert!(
            ids.iter().position(|i| *i == "the-casino")
                < ids.iter().position(|i| *i == "the-long-way").or(Some(usize::MAX)),
            "the casino is asked first: {ids:?}"
        );
        let after = standing_at(8, Some(1_000), Some(20_000), &["the-casino"]);
        assert!(
            !after.iter().any(|e| e.id == "the-long-way"),
            "answering the casino has to shut GERALD"
        );
    }

    /// Every event has to offer a way through that needs nothing, or a player
    /// with an empty tray is stuck in front of it forever.
    #[test]
    fn every_event_has_a_way_through_that_costs_nothing() {
        for e in EVENTS {
            assert!(
                e.choices.iter().any(|c| c.requires == Requirement::None),
                "{} can be locked shut",
                e.id
            );
            for c in e.choices {
                if c.requires != Requirement::None {
                    assert!(!c.unmet.is_empty(), "{}: {} never says why", e.id, c.label);
                }
            }
        }
    }

    /// Whatever an event puts in front of you has to exist.
    #[test]
    fn every_alternate_named_by_an_event_is_real() {
        for e in EVENTS {
            for c in e.choices {
                // Through `every_outcome`, so a composite is read all the way
                // down: half the mission's bargains are an `All`, and a lint
                // that stopped at the top of one would be a lint that stopped
                // working the day the content arrived.
                for o in every_outcome(&c.outcome) {
                    match *o {
                        Outcome::FightInstead(name) => assert!(
                            crate::combat::alternate(name).is_some(),
                            "{} names {}, which is not an alternate",
                            e.id,
                            name
                        ),
                        Outcome::Claim(name) => {
                            let class = crate::class::CLASSES
                                .iter()
                                .find(|k| k.name == name)
                                .unwrap_or_else(|| panic!("{} claims {}, no such class", e.id, name));
                            // Claimed, not qualified for - so nothing you build
                            // can reach it and a fountain must never offer it.
                            assert!(
                                class.requires.is_empty(),
                                "{} is claimable but also has requirements, so a fountain could pour it",
                                name
                            );
                        }
                        Outcome::Enter(id) | Outcome::StartDungeon(id) => assert!(
                            crate::dungeon::by_id(id).is_some(),
                            "{} opens {}, which is not a dungeon",
                            e.id,
                            id
                        ),
                        Outcome::RevealTown(id) => assert!(
                            crate::town::by_id(id).is_some(),
                            "{} reveals {}, which is not a town",
                            e.id,
                            id
                        ),
                        Outcome::Give(name) => assert!(
                            crate::piece::CATALOG.iter().any(|d| d.name == name),
                            "{} hands over {}, which is not a component",
                            e.id,
                            name
                        ),
                        Outcome::SealedBid { lots } => {
                            for lot in lots {
                                assert!(
                                    crate::piece::CATALOG.iter().any(|d| d.name == *lot),
                                    "{} auctions {}, which is not a component",
                                    e.id,
                                    lot
                                );
                            }
                        }
                        Outcome::Step(b) => {
                            for who in b.with {
                                assert!(
                                    crate::combat::creature(who).is_some(),
                                    "{} steps you in with {}, who is nobody",
                                    e.id,
                                    who
                                );
                            }
                            assert!(
                                b.win.is_empty()
                                    || crate::piece::CATALOG.iter().any(|d| d.name == b.win),
                                "{} pays {}, which is not a component",
                                e.id,
                                b.win
                            );
                        }
                        _ => {}
                    }
                }
                // A requirement naming an earlier choice has to name one that
                // exists, or the door is nailed shut and nothing says so.
                if let Requirement::Took(label) = c.requires {
                    assert!(
                        EVENTS.iter().any(|o| o.choices.iter().any(|k| k.label == label)),
                        "{} waits on {:?}, which no choice offers",
                        e.id,
                        label
                    );
                }
            }
        }
    }

    #[test]
    fn a_two_by_two_is_the_only_thing_that_satisfies_a_two_by_two() {
        let r = Requirement::LooseItemOfSize { w: 2, h: 2 };
        assert!(r.met_by_shape(&[(0, 0), (1, 0), (0, 1), (1, 1)]));
        assert!(!r.met_by_shape(&[(0, 0), (1, 0), (2, 0), (3, 0)]));
        assert!(!r.met_by_shape(&[(0, 0), (1, 0), (0, 1)]), "an L is not a square");
        assert!(!r.met_by_shape(&[(0, 0)]));
    }
}
