//! THE HUNDRED: seven by seven tiles under the road, generated from a seed.
//!
//! A subdivision of a shire, and a count. The county is the Switchyard's floor
//! graph in two dimensions with a step budget on it: a place you walk into
//! from a town, walk five tiles of, and walk out of, and which remembers what
//! you cleared for the rest of the run.
//!
//! **It is derived, never stored.** `generate` is a pure function of a seed
//! derived from the run's own, and the run keeps only where you are standing
//! and what you have cleared. Nothing here ever touches `Run::rng`: that
//! stream stocks shops, rolls drops and melts pieces, and a draw from it here
//! would move every one of them and break every replay in the suite.
//!
//! **Generation arranges authored content and never writes prose.** Every
//! string a player reads is authored elsewhere and linted there; the generator
//! decides only which authored tile goes where.

use crate::combat::Difficulty;
use crate::rng::Rng;
use crate::run::Mode;

/// Columns A to G, west to east.
pub const W: u8 = 7;
/// Rows 1 to 7, north to south.
pub const H: u8 = 7;
/// Forty-nine.
pub const TILES: usize = (W as usize) * (H as usize);

/// How many times `generate` re-derives before it gives up and hands back
/// [`FALLBACK`]. The count is part of the pure function, so a replay holds.
pub const ATTEMPTS: u8 = 32;

/// Five moves a trip, and arriving on the mouth's own tile is free.
pub const MOVES_A_TRIP: u8 = 5;

/// Event tiles the generator arranges from the pool. The pale is the twelfth.
pub const ARRANGED: usize = 11;

// --------------------------------------------------------------- the vocabulary

/// Rows 1-2, 3-5, 6-7. The pale's checklist counts cleared tiles in each.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Region {
    North,
    Middle,
    South,
}

impl Region {
    pub const ALL: [Region; 3] = [Region::North, Region::Middle, Region::South];

    /// The canonical word. The theme layer swaps it.
    pub const fn name(self) -> &'static str {
        match self {
            Region::North => "north",
            Region::Middle => "middle",
            Region::South => "south",
        }
    }

    /// North is fourteen tiles, Middle twenty-one, South fourteen.
    pub const fn of_row(y: u8) -> Region {
        match y {
            0 | 1 => Region::North,
            2 | 3 | 4 => Region::Middle,
            _ => Region::South,
        }
    }
}

/// The three claims on the same county.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Chain {
    /// Triangulate. Three trig points, and the lines they draw cross at a hill
    /// nothing marks.
    Ordnance,
    /// Pursue. Three sign tiles, and a drover walking a ring by the clock.
    Drove,
    /// Unseal. Three boundary stones, and a pale that opens a far corner.
    Enclosure,
}

impl Chain {
    pub const ALL: [Chain; 3] = [Chain::Ordnance, Chain::Drove, Chain::Enclosure];

    /// A stable key. Never shown raw.
    pub const fn key(self) -> &'static str {
        match self {
            Chain::Ordnance => "the-ordnance",
            Chain::Drove => "the-drove-roads",
            Chain::Enclosure => "the-enclosure",
        }
    }

    /// What it calls itself. Canonical; the theme layer swaps the noun.
    pub const fn name(self) -> &'static str {
        match self {
            Chain::Ordnance => "THE ORDNANCE",
            Chain::Drove => "THE DROVE ROADS",
            Chain::Enclosure => "THE ENCLOSURE",
        }
    }
}

/// What finishing a chain hands over.
///
/// One enchantment each, in the slot that chain taxed, plus the orb the two
/// that have one pay. The Ordnance also sets `THE_SHEET`, which is not a
/// component and so is not here.
pub const fn chain_pays(chain: Chain) -> &'static [&'static str] {
    match chain {
        Chain::Ordnance => &["Trig Pillar", "Surveyor's Orb"],
        Chain::Drove => &["Drove Way", "Drover's Orb"],
        Chain::Enclosure => &["The Common Ground"],
    }
}

/// The creature standing at a chain's end.
///
/// The Drove's is a brawl of two and this names the one the party is built
/// around; `pinnacle_party` is the pair.
pub const fn pinnacle_creature(chain: Chain) -> &'static str {
    match chain {
        Chain::Ordnance => "THE SURVEYOR",
        Chain::Drove => "THE DROVER",
        Chain::Enclosure => "THE COMMISSIONER",
    }
}

/// Everything that comes to a chain's ending.
///
/// A drover without a herd is a man on a walk, which is why the Drove's is two
/// and the other two are one.
pub const fn pinnacle_party(chain: Chain) -> &'static [&'static str] {
    match chain {
        Chain::Ordnance => &["THE SURVEYOR"],
        Chain::Drove => &["THE DROVER", "THE DRIVEN"],
        Chain::Enclosure => &["THE COMMISSIONER"],
    }
}

/// Which damage lane a ford asks for. A ford names its lane; a river does not,
/// because there is only one kind of mana.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Lane {
    Physical,
    Magic,
}

/// What a Feature tile asks of a board before it will let you across.
///
/// Every figure is derived from the **assembled** board and computed in
/// integers - milli-units a second, summed per item with the division done
/// per item - so that no float ever decides whether a tile is passable. The
/// figures themselves land at F4 (`loadout.rs`); this enum is the threshold
/// each tile carries, and F11 sets the numbers from F4's measured table.
///
/// Failing costs the move and leaves you where you were. Crossing is
/// permanent: a met Feature is a bridge you paid for once.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Toll {
    /// `flow` - milli-mana a second across the assembled board.
    River { milli_per_s: u32 },
    /// `dps(lane)` - milli-damage a second in one lane.
    Ford { lane: Lane, milli_per_s: u32 },
    /// `armour_ps` - milli-armour a second.
    Scarp { milli_per_s: u32 },
    /// The fastest assembled item, in milliseconds. Lower is harder.
    Drift { fastest_ms: u32 },
    /// Summed `curse_resist`.
    Hedge { curse_resist: i32 },
    /// Gold, in multiples of the rung's bounty. Crossing **spends** it.
    Gate { bounties: u32 },
}

impl Toll {
    /// Whether a board and a purse get across.
    ///
    /// `gold` and `bounty` are only read by the toll gate, and it is the only
    /// one that **spends** anything - the other five are a measurement of the
    /// board rather than a price, so crossing one costs nothing and crossing
    /// it again costs nothing twice.
    pub fn met(&self, f: &crate::loadout::Figures, gold: i32, bounty: i32) -> bool {
        match *self {
            Toll::River { milli_per_s } => f.flow >= milli_per_s as i64,
            Toll::Ford { lane, milli_per_s } => f.dps(lane) >= milli_per_s as i64,
            Toll::Scarp { milli_per_s } => f.armour_ps >= milli_per_s as i64,
            // A board with nothing assembled has no fastest item, which is not
            // the same as a slow one: a drift asks for a board that acts often
            // and an empty grid does not act at all.
            Toll::Drift { fastest_ms } => f.fastest_ms.is_some_and(|ms| ms <= fastest_ms),
            Toll::Hedge { curse_resist } => f.curse_resist >= curse_resist,
            Toll::Gate { bounties } => gold >= bounties as i32 * bounty,
        }
    }

    /// What crossing costs, which is nothing except at the gate.
    pub fn toll_in_gold(&self, bounty: i32) -> i32 {
        match *self {
            Toll::Gate { bounties } => bounties as i32 * bounty,
            _ => 0,
        }
    }

    /// The figure this toll reads, and the figure it wants, for a receipt that
    /// says how far short a board fell rather than that it fell short.
    pub fn shortfall(&self, f: &crate::loadout::Figures, gold: i32, bounty: i32) -> String {
        match *self {
            Toll::River { milli_per_s } => {
                format!("mana a second {} against {}", milli(f.flow), milli(milli_per_s as i64))
            }
            Toll::Ford { lane, milli_per_s } => format!(
                "{} damage a second {} against {}",
                match lane {
                    Lane::Physical => "physical",
                    Lane::Magic => "magic",
                },
                milli(f.dps(lane)),
                milli(milli_per_s as i64)
            ),
            Toll::Scarp { milli_per_s } => {
                format!("armour a second {} against {}", milli(f.armour_ps), milli(milli_per_s as i64))
            }
            Toll::Drift { fastest_ms } => format!(
                "fastest item {} against {} ms",
                f.fastest_ms.map(|m| format!("{m} ms")).unwrap_or_else(|| "nothing assembled".into()),
                fastest_ms
            ),
            Toll::Hedge { curse_resist } => {
                format!("curse resistance {} against {}", f.curse_resist, curse_resist)
            }
            Toll::Gate { bounties } => {
                format!("{}g against {}g", gold, bounties as i32 * bounty)
            }
        }
    }

    /// What the tile says it wants, when a player is close enough to read it.
    pub fn threshold(&self) -> String {
        match *self {
            Toll::River { milli_per_s } => format!("~R{}", milli(milli_per_s as i64)),
            Toll::Ford { lane, milli_per_s } => format!(
                "~F{}{}",
                milli(milli_per_s as i64),
                match lane {
                    Lane::Physical => "p",
                    Lane::Magic => "m",
                }
            ),
            Toll::Scarp { milli_per_s } => format!("^S{}", milli(milli_per_s as i64)),
            Toll::Drift { fastest_ms } => format!("^D{}", milli(fastest_ms as i64)),
            Toll::Hedge { curse_resist } => format!("#H{curse_resist}"),
            Toll::Gate { bounties } => format!("#G{bounties}x"),
        }
    }

    /// The glyph the map draws, and the prefix its threshold is printed
    /// against. Canonical: the theme layer retells the *word*, never this.
    pub const fn glyph(&self) -> char {
        match self {
            Toll::River { .. } => '~',
            Toll::Ford { .. } => '~',
            Toll::Scarp { .. } => '^',
            Toll::Drift { .. } => '^',
            Toll::Hedge { .. } => '#',
            Toll::Gate { .. } => '#',
        }
    }

    /// One letter, so a seven-wide grid can name six tolls in three
    /// characters: `~R3`, `^D2`, `#H5`.
    pub const fn letter(&self) -> char {
        match self {
            Toll::River { .. } => 'R',
            Toll::Ford { .. } => 'F',
            Toll::Scarp { .. } => 'S',
            Toll::Drift { .. } => 'D',
            Toll::Hedge { .. } => 'H',
            Toll::Gate { .. } => 'G',
        }
    }
}

/// The id every unarranged event tile carries until F7 writes the pool.
///
/// The county's shape has to be measurable before its content exists - that
/// is the whole of the phase discipline - so the twelve event tiles are
/// arranged now and named later. `county::every_event_tile_names_an_event`
/// is exempted by [`TILES_AWAITING_THEIR_EVENTS`] and the mirror test goes
/// red the moment `COUNTY_EVENTS` has anything in it, so F7 cannot land the
/// pool without putting these tiles back under the lint.
pub const UNARRANGED: &str = "county-event-unarranged";

/// The pale's own tile is an event, and it is one of the twelve.
///
/// B3.1 asks the pale for a checklist read from one tile away and a single
/// gated choice answered by standing on it, which is an event and not a new
/// kind of tile. It costs `TileKind` no variant and the pale nothing it was
/// promised.
pub const PALE: &str = "the-pale";

/// The flag a chain's on-ramp sets: you have been told what to look for.
///
/// THE THEODOLITE, THE STOCKMAN and THE COMMONS each hand over one word and
/// teach one geometry, and this is what a run carries away from them. Read by
/// the map: a trig point you have not been told about is a stone in a field.
pub const fn chain_known(chain: Chain) -> &'static str {
    match chain {
        Chain::Ordnance => "knows-the-ordnance",
        Chain::Drove => "knows-the-drove-roads",
        Chain::Enclosure => "knows-the-enclosure",
    }
}

/// The flag a chain sets when its pinnacle goes down.
///
/// Authored at F8. Named here for the reason the other two are: the county is
/// what the flag is about, and one place asks.
pub const fn chain_done(chain: Chain) -> &'static str {
    match chain {
        Chain::Ordnance => "the-ordnance-is-done",
        Chain::Drove => "the-drove-roads-are-done",
        Chain::Enclosure => "the-enclosure-is-done",
    }
}

/// The flag the Ordnance's sheet sets: every threshold visible from anywhere.
///
/// Authored at F8. Named here because a threshold's visibility is the county's
/// business, and `Run::holds_the_surveyors_sheet` is the one place that asks.
pub const THE_SHEET: &str = "the-surveyors-sheet";

/// The flag the pale's own choice sets, and the far corner reads.
///
/// Authored at F8. Named here because the county is what the flag is *about*,
/// and because a fence has to be shut by something before anything can open
/// it - `Run::pale_is_open` is the one place that asks.
pub const PALE_OPEN: &str = "the-pale-is-open";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// An id into `COUNTY_EVENTS` (F7), which is the road's own `LadderEvent`
    /// with a dead `at`.
    Event(&'static str),
    Feature(Toll),
    Empty,
    Objective { chain: Chain, nth: u8 },
    Pinnacle { chain: Chain },
    Gaol,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub kind: TileKind,
    pub at: (u8, u8),
    pub region: Region,
}

/// Where a town lets you down into it.
///
/// **Fixed rather than generated**, one per town, in `TOWNS` order. A mouth
/// that moved with the seed would make "Sump Bottom comes in at A6" a thing
/// no player could ever learn, and the checks below all measure distance
/// *from* a mouth - a generated mouth would be tuning the ruler.
pub const MOUTHS: [(&str, (u8, u8)); 6] = [
    ("sump-bottom", (0, 5)),
    ("kettleworks", (2, 0)),
    ("high-wick", (6, 2)),
    ("extra-large", (0, 1)),
    ("the-manse", (5, 0)),
    ("the-slagworks", (4, 6)),
];

/// The ring of the inner five by five, clockwise from B2.
///
/// The Drover stands at `CIRCUIT[events_resolved % 16]` and has since the run
/// answered its first door; a sign tile is what teaches a player to look.
/// V11 keeps every one of the sixteen free of a toll, because a pursuit you
/// cannot follow onto the next tile is not a pursuit.
pub const CIRCUIT: [(u8, u8); 16] = [
    (1, 1),
    (2, 1),
    (3, 1),
    (4, 1),
    (5, 1),
    (5, 2),
    (5, 3),
    (5, 4),
    (5, 5),
    (4, 5),
    (3, 5),
    (2, 5),
    (1, 5),
    (1, 4),
    (1, 3),
    (1, 2),
];

// ------------------------------------------------------------------ the county

/// A line through the hill: one of the four a tile can sit on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Bearing {
    Row,
    Column,
    /// North-west to south-east: `x - y` is constant.
    Falling,
    /// North-east to south-west: `x + y` is constant.
    Rising,
}

impl Bearing {
    pub const ALL: [Bearing; 4] = [Bearing::Row, Bearing::Column, Bearing::Falling, Bearing::Rising];

    /// Whether `p` sits on this line drawn through `hill`.
    pub const fn holds(&self, hill: (u8, u8), p: (u8, u8)) -> bool {
        match self {
            Bearing::Row => p.1 == hill.1,
            Bearing::Column => p.0 == hill.0,
            Bearing::Falling => p.0 as i8 - p.1 as i8 == hill.0 as i8 - hill.1 as i8,
            Bearing::Rising => p.0 + p.1 == hill.0 + hill.1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct County {
    tiles: [Tile; TILES],
    /// Where the three bearing lines cross, and where THE SURVEYOR stands.
    hill: (u8, u8),
    /// Which three of the four lines carry a trig point.
    bearings: [Bearing; 3],
    /// The sealed tile the Enclosure's checklist is read at.
    pale: (u8, u8),
    /// The far corner the pale opens: the corner tile and its two neighbours
    /// along the edges. Unenterable until the pale is answered.
    sealed: [(u8, u8); 3],
    /// How many seeds `generate` got through. `ATTEMPTS` means none of them
    /// worked and this is [`FALLBACK`].
    attempts: u8,
}

impl County {
    /// The county as a run sees it: the hill drawn as what it looks like.
    ///
    /// **B1.1's presentation, inverted.** The spec stores the hill as `Empty`
    /// and rewrites it to a Pinnacle when the third sighting is taken; the
    /// store here carries `Pinnacle { Ordnance }` and this hides it until
    /// then. The behaviour is identical - the game never marks the hill, the
    /// lines do - and storing it as a pinnacle is what makes A1.2's skeleton
    /// count and V6's spacing true as written.
    ///
    /// `sightings` is how many trig points have been cleared.
    pub fn as_seen(&self, sightings: usize) -> County {
        if sightings >= 3 {
            return self.clone();
        }
        let mut seen = self.clone();
        let i = self.hill.1 as usize * W as usize + self.hill.0 as usize;
        seen.tiles[i].kind = TileKind::Empty;
        seen
    }

    /// The full line through the hill that a trig point draws, once cleared.
    ///
    /// One line is a line. Two cross at exactly one tile, so **two sightings
    /// are knowledge** - a player who draws them knows where to walk. The
    /// third is not information, it is the key.
    pub fn sighting(&self, nth: u8) -> Vec<(u8, u8)> {
        let Some(b) = self.bearings.get(nth as usize - 1) else { return Vec::new() };
        (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|p| b.holds(self.hill, *p))
            .collect()
    }

    pub const fn at(&self, p: (u8, u8)) -> &Tile {
        &self.tiles[p.1 as usize * W as usize + p.0 as usize]
    }

    pub fn tiles(&self) -> &[Tile; TILES] {
        &self.tiles
    }

    pub const fn hill(&self) -> (u8, u8) {
        self.hill
    }

    pub const fn bearings(&self) -> &[Bearing; 3] {
        &self.bearings
    }

    pub const fn pale(&self) -> (u8, u8) {
        self.pale
    }

    pub const fn sealed(&self) -> &[(u8, u8); 3] {
        &self.sealed
    }

    pub const fn attempts(&self) -> u8 {
        self.attempts
    }

    /// True when every seed in the retry window was refused and this is the
    /// authored county rather than a derived one.
    pub const fn is_fallback(&self) -> bool {
        self.attempts == ATTEMPTS
    }

    pub fn is_sealed(&self, p: (u8, u8)) -> bool {
        self.sealed.contains(&p)
    }

    /// The tiles a chain's objectives stand on, in `nth` order.
    pub fn objectives(&self, chain: Chain) -> Vec<(u8, u8)> {
        let mut found: Vec<(u8, u8, u8)> = self
            .tiles
            .iter()
            .filter_map(|t| match t.kind {
                TileKind::Objective { chain: c, nth } if c == chain => Some((nth, t.at.0, t.at.1)),
                _ => None,
            })
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, x, y)| (x, y)).collect()
    }

    pub fn pinnacle(&self, chain: Chain) -> Option<(u8, u8)> {
        self.tiles
            .iter()
            .find(|t| t.kind == TileKind::Pinnacle { chain })
            .map(|t| t.at)
    }

    /// Where the gaol is, if the county has one.
    ///
    /// An `Option` because [`refusals`] is the thing that diagnoses a
    /// malformed county and must not be the thing that panics on one.
    pub fn gaol(&self) -> Option<(u8, u8)> {
        self.tiles.iter().find(|t| t.kind == TileKind::Gaol).map(|t| t.at)
    }

    pub fn count(&self, want: fn(&TileKind) -> bool) -> usize {
        self.tiles.iter().filter(|t| want(&t.kind)).count()
    }

    /// Assemble a county from forty-nine kinds in row-major order.
    ///
    /// A `const fn` so [`FALLBACK`] can be authored as a picture of the grid
    /// rather than as forty-nine `Tile` literals that each repeat their own
    /// coordinates back at the reader.
    pub const fn of(
        kinds: [TileKind; TILES],
        hill: (u8, u8),
        bearings: [Bearing; 3],
        pale: (u8, u8),
        sealed: [(u8, u8); 3],
        attempts: u8,
    ) -> County {
        let mut tiles =
            [Tile { kind: TileKind::Empty, at: (0, 0), region: Region::North }; TILES];
        let mut i = 0;
        while i < TILES {
            let x = (i % W as usize) as u8;
            let y = (i / W as usize) as u8;
            tiles[i] = Tile { kind: kinds[i], at: (x, y), region: Region::of_row(y) };
            i += 1;
        }
        County { tiles, hill, bearings, pale, sealed, attempts }
    }
}

/// A milli-unit figure, said the way a person reads it: 2000 is `2`, 2500 is
/// `2.5`. Integer arithmetic all the way down; the decimal point is printing.
pub fn milli(n: i64) -> String {
    let whole = n / 1000;
    match (n % 1000).abs() {
        0 => format!("{whole}"),
        r if r % 100 == 0 => format!("{whole}.{}", r / 100),
        r if r % 10 == 0 => format!("{whole}.{:02}", r / 10),
        r => format!("{whole}.{r:03}"),
    }
}

/// A grid reference: `A1` to `G7`, the way the map's own edges are labelled.
pub fn reference(p: (u8, u8)) -> String {
    format!("{}{}", (b'A' + p.0) as char, p.1 + 1)
}

impl TileKind {
    /// What the tile is, in canonical words.
    ///
    /// Terse on purpose: a banner has to name where you are standing and the
    /// prose is F7's and F10's. The theme layer swaps these the way it swaps a
    /// door's name, and nothing the engine decides reads one.
    pub const fn what(&self) -> &'static str {
        match self {
            TileKind::Event(_) => "a question",
            TileKind::Feature(Toll::River { .. }) => "a river",
            TileKind::Feature(Toll::Ford { .. }) => "a ford",
            TileKind::Feature(Toll::Scarp { .. }) => "a scarp",
            TileKind::Feature(Toll::Drift { .. }) => "a drift",
            TileKind::Feature(Toll::Hedge { .. }) => "a hedge",
            TileKind::Feature(Toll::Gate { .. }) => "a toll gate",
            TileKind::Empty => "open ground",
            TileKind::Objective { chain: Chain::Ordnance, .. } => "a trig point",
            TileKind::Objective { chain: Chain::Drove, .. } => "a sign",
            TileKind::Objective { chain: Chain::Enclosure, .. } => "a boundary stone",
            TileKind::Pinnacle { .. } => "the end of a chain",
            TileKind::Gaol => "the gaol",
        }
    }
}

/// Which way a step goes. Orthogonal only: the bearings are lines drawn on a
/// map, not roads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Step {
    North,
    South,
    East,
    West,
}

impl Step {
    pub const ALL: [Step; 4] = [Step::North, Step::South, Step::East, Step::West];

    pub const fn key(self) -> &'static str {
        match self {
            Step::North => "n",
            Step::South => "s",
            Step::East => "e",
            Step::West => "w",
        }
    }

    pub fn parse(word: &str) -> Option<Step> {
        Step::ALL.into_iter().find(|s| s.key() == word.trim().to_lowercase())
    }

    /// The tile this step lands on, or `None` at the county's edge.
    pub fn from(self, p: (u8, u8)) -> Option<(u8, u8)> {
        let d = match self {
            Step::North => (0i8, -1i8),
            Step::South => (0, 1),
            Step::East => (1, 0),
            Step::West => (-1, 0),
        };
        let q = (p.0 as i8 + d.0, p.1 as i8 + d.1);
        in_bounds(q).then(|| (q.0 as u8, q.1 as u8))
    }
}

// ------------------------------------------------------------------- geography

pub const fn in_bounds(p: (i8, i8)) -> bool {
    p.0 >= 0 && p.1 >= 0 && p.0 < W as i8 && p.1 < H as i8
}

pub const fn on_edge(p: (u8, u8)) -> bool {
    p.0 == 0 || p.1 == 0 || p.0 == W - 1 || p.1 == H - 1
}

pub const fn manhattan(a: (u8, u8), b: (u8, u8)) -> u8 {
    let dx = if a.0 > b.0 { a.0 - b.0 } else { b.0 - a.0 };
    let dy = if a.1 > b.1 { a.1 - b.1 } else { b.1 - a.1 };
    dx + dy
}

pub fn on_circuit(p: (u8, u8)) -> bool {
    CIRCUIT.contains(&p)
}

pub fn is_mouth(p: (u8, u8)) -> bool {
    MOUTHS.iter().any(|(_, m)| *m == p)
}

/// How many answered doors the Drover gains a point of strength for.
///
/// **D-4, taken as recommended: shipped behind its own constant so that F14's
/// replay can zero it in one line.** A run that dawdled meets a harder drover
/// - pursuit punishing patience, and the sudden-death budget enforced from the
/// other side.
///
/// It punishes a slow run twice, which is the argument against it: harder
/// drover *and* fewer events left. Set this to 0 and the pursuit is the same
/// pursuit whenever it is met, and nothing else in the game changes.
pub const DROVER_STRENGTH_PER: u32 = 8;

/// The fifth edge tile a perambulation reaches is where THE PARISH stands.
pub const PARISH_AT: u8 = 5;

/// The county's boundary, walked: every edge tile once, clockwise from A1.
///
/// Twenty-four tiles. A perambulation is a walk **round** the county rather
/// than across it, so what "the next one" means is a position in this ring
/// rather than a direction - and going the other way is the same ring read
/// backwards, which is what makes "always clockwise or always
/// counter-clockwise, chosen by the first move" one rule instead of two.
pub fn boundary() -> Vec<(u8, u8)> {
    let mut out = Vec::with_capacity(24);
    for x in 0..W {
        out.push((x, 0));
    }
    for y in 1..H - 1 {
        out.push((W - 1, y));
    }
    for x in (0..W).rev() {
        out.push((x, H - 1));
    }
    for y in (1..H - 1).rev() {
        out.push((0, y));
    }
    out
}

/// The next tile round the boundary from `p`, one way or the other.
///
/// `None` when `p` is not on the boundary at all, which is a move that breaks
/// the walk rather than one that continues it.
pub fn next_round(p: (u8, u8), clockwise: bool) -> Option<(u8, u8)> {
    let ring = boundary();
    let i = ring.iter().position(|q| *q == p)?;
    let n = ring.len();
    Some(if clockwise { ring[(i + 1) % n] } else { ring[(i + n - 1) % n] })
}

/// N, S, E, W. Movement is orthogonal and there is no diagonal anything - the
/// bearings are lines drawn on a map, not roads.
pub fn neighbours(p: (u8, u8)) -> Vec<(u8, u8)> {
    let mut out = Vec::with_capacity(4);
    for (dx, dy) in [(0i8, -1i8), (0, 1), (-1, 0), (1, 0)] {
        let q = (p.0 as i8 + dx, p.1 as i8 + dy);
        if in_bounds(q) {
            out.push((q.0 as u8, q.1 as u8));
        }
    }
    out
}

/// The corner tile and its two neighbours along the edges: three tiles.
///
/// A two-by-two block would be the obvious shape and it is the wrong one -
/// every two-by-two corner of a seven by seven contains exactly one tile of
/// the circuit, and a Drover walking into a region nobody can enter is a
/// pursuit that ends by arithmetic.
pub const fn corner_l(corner: (u8, u8)) -> [(u8, u8); 3] {
    let inx = if corner.0 == 0 { 1 } else { W - 2 };
    let iny = if corner.1 == 0 { 1 } else { H - 2 };
    [corner, (inx, corner.1), (corner.0, iny)]
}

/// The four corners. `max_by_key` keeps the **last** maximum, so the corner
/// a tie should go to is written last.
pub const CORNERS: [(u8, u8); 4] = [(0, 0), (W - 1, 0), (0, H - 1), (W - 1, H - 1)];

/// The corner the pale opens: the one furthest from it, holding no mouth.
pub fn far_corner(pale: (u8, u8)) -> Option<(u8, u8)> {
    CORNERS
        .iter()
        .copied()
        .filter(|c| !corner_l(*c).iter().any(|t| is_mouth(*t)))
        .max_by_key(|c| manhattan(*c, pale))
}

// ------------------------------------------------------------------- the seed

/// The county's seed, derived from the run's and never drawn from it.
///
/// A1's formula. `^` binds looser than `<<` in Rust as it does on the page,
/// so this is the run's seed exclusive-or'd with two small numbers parked
/// well above anything a seed's low bits are doing.
pub fn seed_for(run_seed: u64, mode: Mode, difficulty: Difficulty) -> u64 {
    run_seed ^ ((mode as u64) << 40) ^ ((difficulty as u64) << 44)
}

// ------------------------------------------------------------------ generation

/// Same seed, same county, for ever. Never stored, never drawn from `Run::rng`.
///
/// Up to [`ATTEMPTS`] seeds are tried in order - `seed`, `seed+1`, ... - and
/// the first arrangement that satisfies every check in [`refusals`] is
/// returned. If none does, [`FALLBACK`] is, which is the reason `FALLBACK` is
/// authored and checked rather than sampled.
pub fn generate(seed: u64) -> County {
    for n in 0..ATTEMPTS {
        if let Some(mut c) = arrange(seed.wrapping_add(n as u64)) {
            if refusals(&c).is_empty() {
                c.attempts = n;
                return c;
            }
        }
    }
    let mut c = FALLBACK.clone();
    c.attempts = ATTEMPTS;
    c
}

/// One attempt: place the skeleton, then the tolls, then fill.
///
/// Returns `None` when the arrangement painted itself into a corner - no hill
/// with three usable lines, no room for a trig point on one of them. Those are
/// cheaper to refuse here than to build and then check.
fn arrange(seed: u64) -> Option<County> {
    let mut rng = Rng::new(seed);
    let mut kinds = [TileKind::Empty; TILES];
    let put = |kinds: &mut [TileKind; TILES], p: (u8, u8), k: TileKind| {
        kinds[p.1 as usize * W as usize + p.0 as usize] = k;
    };
    let taken = |kinds: &[TileKind; TILES], p: (u8, u8)| {
        kinds[p.1 as usize * W as usize + p.0 as usize] != TileKind::Empty
    };

    // The pale is forced into the inner three by three: V5 puts it off the
    // edge and off the circuit, and on a seven by seven those are the only
    // nine tiles left.
    let mut inner: Vec<(u8, u8)> = (2..=4)
        .flat_map(|y| (2..=4u8).map(move |x| (x, y)))
        .filter(|p| MOUTHS.iter().all(|(_, m)| manhattan(*p, *m) >= 2))
        .filter(|p| far_corner(*p).is_some())
        .collect();
    rng.shuffle(&mut inner);
    let pale = *inner.first()?;
    let sealed = corner_l(far_corner(pale)?);
    put(&mut kinds, pale, TileKind::Event(PALE));

    // The hill is any tile off the edge that is not already spoken for. Its
    // three lines have to be pairwise distinct, which they are by construction
    // - four directions through one tile meet only at that tile - and each has
    // to have somewhere to put a trig point.
    //
    // And it is a pinnacle, so it owes V6 what the other two owe: three tiles
    // from every other pinnacle, and never beside a gate. The Enclosure's is
    // somewhere in the sealed L, which the pale already fixed, so both halves
    // are knowable here - and filtering here rather than refusing afterwards
    // is the difference between a 55% retry rate and none.
    let mut candidates: Vec<(u8, u8)> = (1..H - 1)
        .flat_map(|y| (1..W - 1).map(move |x| (x, y)))
        .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
        .filter(|p| !neighbours(*p).iter().any(|n| is_mouth(*n)))
        .filter(|p| sealed.iter().all(|s| manhattan(*s, *p) >= 3))
        .collect();
    rng.shuffle(&mut candidates);

    for hill in candidates {
        let mut lines = Bearing::ALL;
        rng.shuffle(&mut lines);
        let bearings = [lines[0], lines[1], lines[2]];
        let mut chosen = [(0u8, 0u8); 3];
        let mut ok = true;
        for (i, b) in bearings.iter().enumerate() {
            let mut on: Vec<(u8, u8)> = (0..H)
                .flat_map(|y| (0..W).map(move |x| (x, y)))
                .filter(|p| *p != hill && b.holds(hill, *p))
                .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
                // Three different regions between them, and no two adjacent.
                .filter(|p| chosen[..i].iter().all(|c| manhattan(*c, *p) > 1))
                .filter(|p| {
                    chosen[..i]
                        .iter()
                        .all(|c| Region::of_row(c.1) != Region::of_row(p.1))
                })
                .collect();
            rng.shuffle(&mut on);
            match on.first() {
                Some(p) => chosen[i] = *p,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        for (i, p) in chosen.iter().enumerate() {
            put(&mut kinds, *p, TileKind::Objective { chain: Chain::Ordnance, nth: i as u8 + 1 });
        }
        put(&mut kinds, hill, TileKind::Pinnacle { chain: Chain::Ordnance });

        // The Enclosure's third stone and its pinnacle are behind the pale.
        // Which of the L's three tiles is which is the seed's business; the
        // third is left to the fill.
        let mut sealed_order = sealed;
        rng.shuffle(&mut sealed_order);
        // V6 again: the Commissioner may not stand beside a gate. The L is
        // three edge tiles and a mouth is an edge tile, so this can bite.
        if sealed_order
            .iter()
            .position(|p| !neighbours(*p).iter().any(|n| is_mouth(*n)))
            .map(|k| sealed_order.swap(0, k))
            .is_none()
        {
            continue;
        }
        put(&mut kinds, sealed_order[0], TileKind::Pinnacle { chain: Chain::Enclosure });
        put(
            &mut kinds,
            sealed_order[1],
            TileKind::Objective { chain: Chain::Enclosure, nth: 3 },
        );

        // The gaol within three of the middle (V9), which is what makes being
        // arrested the fastest ride into the county there is.
        let mut gaols: Vec<(u8, u8)> = (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|p| manhattan(*p, (3, 3)) <= 3)
            .filter(|p| MOUTHS.iter().all(|(_, m)| manhattan(*p, *m) >= 2))
            .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
            .collect();
        rng.shuffle(&mut gaols);
        let gaol = *gaols.first()?;
        put(&mut kinds, gaol, TileKind::Gaol);

        // The two remaining chains: three objectives each in three regions, no
        // two of one chain adjacent, and the Enclosure already has its third.
        let mut placed_ok = true;
        for (chain, from) in [(Chain::Drove, 1u8), (Chain::Enclosure, 1u8)] {
            let already: Vec<(u8, u8)> = if chain == Chain::Enclosure {
                vec![sealed_order[1]]
            } else {
                vec![]
            };
            let mut mine = already.clone();
            for nth in from..=(if chain == Chain::Enclosure { 2 } else { 3 }) {
                let mut room: Vec<(u8, u8)> = (0..H)
                    .flat_map(|y| (0..W).map(move |x| (x, y)))
                    .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
                    .filter(|p| mine.iter().all(|c| manhattan(*c, *p) > 1))
                    .filter(|p| {
                        mine.iter().all(|c| Region::of_row(c.1) != Region::of_row(p.1))
                    })
                    .collect();
                rng.shuffle(&mut room);
                match room.first() {
                    Some(p) => {
                        put(&mut kinds, *p, TileKind::Objective { chain, nth });
                        mine.push(*p);
                    }
                    None => {
                        placed_ok = false;
                        break;
                    }
                }
            }
            if !placed_ok {
                break;
            }
        }
        if !placed_ok {
            continue;
        }

        // The Drove's pinnacle: three apart from the other two and never
        // beside a mouth, so no chain's ending is two steps from a gate.
        let others: Vec<(u8, u8)> = vec![hill, sealed_order[0]];
        let mut lairs: Vec<(u8, u8)> = (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
            .filter(|p| others.iter().all(|o| manhattan(*o, *p) >= 3))
            .filter(|p| !neighbours(*p).iter().any(|n| is_mouth(*n)) && !is_mouth(*p))
            .collect();
        rng.shuffle(&mut lairs);
        let lair = match lairs.first() {
            Some(p) => *p,
            None => continue,
        };
        put(&mut kinds, lair, TileKind::Pinnacle { chain: Chain::Drove });

        // Twelve tolls, two of each kind, none of them on the circuit (V11)
        // and none of them on a mouth - a gate you cannot walk out of is a
        // trip that ends before it starts.
        let mut room: Vec<(u8, u8)> = (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|p| !taken(&kinds, *p) && !sealed.contains(p) && !is_mouth(*p))
            .filter(|p| !on_circuit(*p))
            .collect();
        if room.len() < TOLLS.len() {
            continue;
        }
        rng.shuffle(&mut room);
        for (p, t) in room.iter().zip(TOLLS.iter()) {
            put(&mut kinds, *p, TileKind::Feature(*t));
        }

        // Eleven arranged events, and everything left is empty. The pale is
        // the twelfth.
        //
        // Eight authored into eleven slots (D-2), dealt as a shuffled deck and
        // then dealt again: **every** event is on the county once before any
        // is on it twice, which a per-tile draw would not promise and which is
        // the difference between "eight events, three repeated" and "eight
        // events, one of them four times".
        let mut rest: Vec<(u8, u8)> = (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|p| !taken(&kinds, *p))
            .collect();
        rng.shuffle(&mut rest);
        // The pale is in the same table and is not in the deck: the generator
        // placed it before the hill was picked, and dealing it again would put
        // two gates on one county.
        let mut deck: Vec<&'static str> = crate::event::COUNTY_EVENTS
            .iter()
            .map(|e| e.id)
            .filter(|id| *id != PALE)
            .collect();
        if deck.is_empty() {
            deck.push(UNARRANGED);
        }
        rng.shuffle(&mut deck);
        let mut hand: Vec<&'static str> = Vec::new();
        while hand.len() < ARRANGED {
            let mut again = deck.clone();
            rng.shuffle(&mut again);
            hand.extend(again);
        }
        for (p, id) in rest.iter().take(ARRANGED).zip(hand) {
            put(&mut kinds, *p, TileKind::Event(id));
        }

        return Some(County::of(kinds, hill, bearings, pale, sealed, 0));
    }
    None
}

/// The twelve tolls a county carries: two of each of the six kinds.
///
/// **Measured at F11**, off F4's table of what the four reference boards
/// actually pay. A3's starting numbers were arithmetic off a paper map and
/// every one of them was crossed by the auto-builder's board; these are chosen
/// so that each kind has one tier a formed board takes and one it has to be
/// built for.
///
/// The figures they were chosen against, read at F4:
///
/// ```text
/// build          flow    phys/s   magic/s  armour/s   fastest   hedge
/// starter           0         0         0         0         -       0
/// preset        2.544         6         0    32.829    1500ms      10
/// owner         11.77    58.255     9.828    85.971    1500ms     131
/// friend        23.23     2.083    29.868    80.569    1900ms      63
/// ```
///
/// **A board that crosses rivers is not a board that climbs scarps**, and
/// these numbers are what makes that a fact rather than a sentence: the owner
/// fails the deep river and the magic ford, the friend fails the physical
/// ford, the steep scarp, the fast drift and the high hedge, and neither of
/// them crosses all twelve. `tolls::what_the_reference_boards_cross` is the
/// pin, and moving a number here moves no gear and re-gears nobody - a
/// threshold is read at a tile and priced nowhere.
pub const TOLLS: [Toll; 12] = [
    // One a formed board has, and one it has to build for.
    Toll::River { milli_per_s: 3_000 },
    Toll::River { milli_per_s: 15_000 },
    Toll::Ford { lane: Lane::Physical, milli_per_s: 10_000 },
    Toll::Ford { lane: Lane::Magic, milli_per_s: 20_000 },
    Toll::Scarp { milli_per_s: 30_000 },
    Toll::Scarp { milli_per_s: 84_000 },
    // Lower is harder: a drift asks how *often* the board acts.
    Toll::Drift { fastest_ms: 2_000 },
    Toll::Drift { fastest_ms: 1_600 },
    Toll::Hedge { curse_resist: 12 },
    Toll::Hedge { curse_resist: 80 },
    Toll::Gate { bounties: 1 },
    Toll::Gate { bounties: 2 },
];

// --------------------------------------------------------------- the checks

/// How far a trip reaches, and how many tolls it may cross getting there.
const REACH: u8 = MOVES_A_TRIP;
const TOLL_BUDGET: u8 = 1;

/// Every tile reachable from some mouth in `moves` moves crossing at most
/// `budget` tolls, with the pale treated as **open**.
///
/// The pale is a door and not a wall: the Enclosure's third stone and its
/// pinnacle stand behind it by design, and a reachability check that refused
/// them would refuse every county the spec describes.
fn reachable_within(c: &County, moves: u8, budget: u8) -> Vec<Vec<Option<u8>>> {
    // best[y][x] = fewest tolls crossed on any path of <= `moves` moves.
    let mut best = vec![vec![None::<u8>; W as usize]; H as usize];
    let mut queue: Vec<((u8, u8), u8, u8)> = Vec::new();
    for (_, m) in MOUTHS.iter() {
        best[m.1 as usize][m.0 as usize] = Some(0);
        queue.push((*m, 0, 0));
    }
    let mut head = 0;
    while head < queue.len() {
        let (p, used, tolls) = queue[head];
        head += 1;
        if used == moves {
            continue;
        }
        for q in neighbours(p) {
            let cost = u8::from(matches!(c.at(q).kind, TileKind::Feature(_)));
            let next = tolls + cost;
            if next > budget {
                continue;
            }
            let slot = &mut best[q.1 as usize][q.0 as usize];
            if slot.is_none_or(|had| next < had) {
                *slot = Some(next);
                queue.push((q, used + 1, next));
            }
        }
    }
    best
}

/// Which mouths each tile can be reached from, within the trip budget.
pub fn mouths_reaching(c: &County, p: (u8, u8)) -> Vec<&'static str> {
    MOUTHS
        .iter()
        .filter(|(_, m)| {
            let mut best = vec![vec![None::<u8>; W as usize]; H as usize];
            let mut queue = vec![(*m, 0u8, 0u8)];
            best[m.1 as usize][m.0 as usize] = Some(0);
            let mut head = 0;
            let mut found = *m == p;
            while head < queue.len() {
                let (q, used, tolls) = queue[head];
                head += 1;
                if used == REACH {
                    continue;
                }
                for r in neighbours(q) {
                    let cost = u8::from(matches!(c.at(r).kind, TileKind::Feature(_)));
                    let next = tolls + cost;
                    if next > TOLL_BUDGET {
                        continue;
                    }
                    let slot = &mut best[r.1 as usize][r.0 as usize];
                    if slot.is_none_or(|had| next < had) {
                        *slot = Some(next);
                        if r == p {
                            found = true;
                        }
                        queue.push((r, used + 1, next));
                    }
                }
            }
            found
        })
        .map(|(id, _)| *id)
        .collect()
}

/// V1 to V12, in order, each returning the tiles that refused it.
///
/// One function rather than twelve, because a generator that has to satisfy
/// them all wants one answer to "why not", and a test that names one wants to
/// read it off the same list the generator did.
pub fn refusals(c: &County) -> Vec<String> {
    let mut out = Vec::new();
    let mut objectives: Vec<(Chain, u8, (u8, u8))> = Vec::new();
    let mut pinnacles: Vec<(Chain, (u8, u8))> = Vec::new();
    for t in c.tiles.iter() {
        match t.kind {
            TileKind::Objective { chain, nth } => objectives.push((chain, nth, t.at)),
            TileKind::Pinnacle { chain } => pinnacles.push((chain, t.at)),
            _ => {}
        }
    }

    // V1 - everything that matters is one trip from a gate.
    let near = reachable_within(c, REACH, TOLL_BUDGET);
    let mut must: Vec<(String, (u8, u8))> =
        vec![("the pale".into(), c.pale), ("the hill".into(), c.hill)];
    for (chain, nth, at) in &objectives {
        must.push((format!("{chain:?} objective {nth}"), *at));
    }
    for (chain, at) in &pinnacles {
        must.push((format!("{chain:?} pinnacle"), *at));
    }
    for (what, at) in &must {
        if near[at.1 as usize][at.0 as usize].is_none() {
            out.push(format!("V1: {what} at {at:?} is more than {REACH} moves or {TOLL_BUDGET} toll from every mouth"));
        }
    }

    // V2 - a chain's three objectives are three gates' work, not one's.
    //
    // "Reachable from three different mouths between them" read as a
    // **matching**: each objective can be given a mouth of its own. The weaker
    // reading - the union of the three sets has three mouths in it - is
    // satisfied by three objectives huddled in one corner, which is the shape
    // the check exists to refuse.
    for chain in Chain::ALL {
        let mine: Vec<(u8, u8)> =
            objectives.iter().filter(|(c, _, _)| *c == chain).map(|(_, _, a)| *a).collect();
        let sets: Vec<Vec<&'static str>> = mine.iter().map(|at| mouths_reaching(c, *at)).collect();
        let matched = sets.len() == 3
            && sets[0].iter().any(|a| {
                sets[1].iter().any(|b| b != a && sets[2].iter().any(|z| z != a && z != b))
            });
        if !matched {
            out.push(format!(
                "V2: {chain:?}'s three objectives cannot be given three different mouths                  between them ({sets:?})"
            ));
        }
    }

    // V3 - no two objectives of one chain adjacent.
    for chain in Chain::ALL {
        let mine: Vec<(u8, u8)> = objectives.iter().filter(|(c, _, _)| *c == chain).map(|(_, _, a)| *a).collect();
        for (i, a) in mine.iter().enumerate() {
            for b in mine.iter().skip(i + 1) {
                if manhattan(*a, *b) <= 1 {
                    out.push(format!("V3: {chain:?} has {a:?} beside {b:?}"));
                }
            }
        }
    }

    // V4 - and they are spread across the three regions.
    for chain in Chain::ALL {
        let mut regions: Vec<Region> = Vec::new();
        for (_, _, a) in objectives.iter().filter(|(c, _, _)| *c == chain) {
            let r = Region::of_row(a.1);
            if !regions.contains(&r) {
                regions.push(r);
            }
        }
        if regions.len() != 3 {
            out.push(format!("V4: {chain:?} covers {} regions", regions.len()));
        }
    }

    // V5 - the pale.
    if on_edge(c.pale) {
        out.push(format!("V5: the pale is on the edge at {:?}", c.pale));
    }
    if on_circuit(c.pale) {
        out.push(format!("V5: the pale is on the circuit at {:?}", c.pale));
    }
    for (id, m) in MOUTHS.iter() {
        if manhattan(c.pale, *m) < 2 {
            out.push(format!("V5: the pale is {} from {id}", manhattan(c.pale, *m)));
        }
    }
    if c.sealed.iter().any(|t| is_mouth(*t)) {
        out.push("V5: the sealed corner holds a mouth".into());
    }

    // V6 - the pinnacles.
    for (i, (ca, a)) in pinnacles.iter().enumerate() {
        for (cb, b) in pinnacles.iter().skip(i + 1) {
            if manhattan(*a, *b) < 3 {
                out.push(format!("V6: {ca:?} at {a:?} is {} from {cb:?}", manhattan(*a, *b)));
            }
        }
        if neighbours(*a).iter().any(|n| is_mouth(*n)) || is_mouth(*a) {
            out.push(format!("V6: {ca:?}'s pinnacle at {a:?} is beside a mouth"));
        }
    }

    // V7 - the county is one place, and all of it is walkable in eight.
    let far = reachable_within(c, 8, TOLL_BUDGET.max(12));
    for t in c.tiles.iter() {
        if far[t.at.1 as usize][t.at.0 as usize].is_none() {
            out.push(format!("V7: {:?} is more than 8 moves from every mouth", t.at));
        }
    }

    // V8 - nothing that matters is walled in by two tolls.
    let two = reachable_within(c, 8, 1);
    for (what, at) in &must {
        if two[at.1 as usize][at.0 as usize].is_none() {
            out.push(format!("V8: every path to {what} at {at:?} crosses two tolls or more"));
        }
    }

    // V9 - the gaol is near the middle, and not beside a gate.
    //
    // C1's whole argument is that being arrested is the fastest ride into the
    // middle there is, and "within three of D4" does not get there on its own:
    // a mouth two tiles in from a corner is itself within three of some tile
    // that is within three of D4. The second half of this check is what makes
    // the argument true, and `the_gaol_is_deeper_in_than_any_mouth` is what
    // found that it was not.
    match c.gaol() {
        None => out.push("V9: the county has no gaol".into()),
        Some(gaol) => {
            if manhattan(gaol, (3, 3)) > 3 {
                out.push(format!("V9: the gaol at {gaol:?} is {} from D4", manhattan(gaol, (3, 3))));
            }
            for (id, m) in MOUTHS.iter() {
                if manhattan(gaol, *m) < 2 {
                    out.push(format!("V9: the gaol at {gaol:?} is beside {id}'s mouth"));
                }
            }
        }
    }

    // V10 - the composition, within one tile of A1.2 in each kind.
    for (what, want, got) in [
        ("objectives", 9, c.count(|k| matches!(k, TileKind::Objective { .. }))),
        ("pinnacles", 3, c.count(|k| matches!(k, TileKind::Pinnacle { .. }))),
        ("gaols", 1, c.count(|k| matches!(k, TileKind::Gaol))),
        ("events", 12, c.count(|k| matches!(k, TileKind::Event(_)))),
        ("features", 12, c.count(|k| matches!(k, TileKind::Feature(_)))),
        ("empties", 12, c.count(|k| matches!(k, TileKind::Empty))),
    ] {
        if got.abs_diff(want) > 1 {
            out.push(format!("V10: {got} {what}, wanted {want}"));
        }
    }

    // V11 - the ring the Drover walks carries no toll.
    for p in CIRCUIT.iter() {
        if matches!(c.at(*p).kind, TileKind::Feature(_)) {
            out.push(format!("V11: a toll stands on the circuit at {p:?}"));
        }
    }

    // V12 - the bearings.
    if on_edge(c.hill) {
        out.push(format!("V12: the hill is on the edge at {:?}", c.hill));
    }
    for (i, a) in c.bearings.iter().enumerate() {
        for b in c.bearings.iter().skip(i + 1) {
            if a == b {
                out.push(format!("V12: two bearings are both {a:?}"));
            }
            // Two distinct lines through one tile meet only at that tile, so
            // this is a check on the *drawing* rather than on the geometry:
            // every tile either line holds, other than the hill, is on one of
            // them and not both.
            for y in 0..H {
                for x in 0..W {
                    let p = (x, y);
                    if p != c.hill && a.holds(c.hill, p) && b.holds(c.hill, p) {
                        out.push(format!("V12: {a:?} and {b:?} meet again at {p:?}"));
                    }
                }
            }
        }
    }
    let trigs = c.objectives(Chain::Ordnance);
    if trigs.len() == 3 {
        for (i, t) in trigs.iter().enumerate() {
            if !c.bearings.iter().any(|b| b.holds(c.hill, *t)) {
                out.push(format!("V12: trig point {} at {t:?} is on no bearing", i + 1));
            }
        }
        for b in c.bearings.iter() {
            if !trigs.iter().any(|t| b.holds(c.hill, *t)) {
                out.push(format!("V12: {b:?} carries no trig point"));
            }
        }
    }

    out
}

// ---------------------------------------------------------------- the fallback

/// The county the generator hands back when thirty-two seeds in a row are
/// refused, and the fixture everything else in the mission is tested against.
///
/// Authored rather than sampled, and authored **first**, from the sample map
/// in `design/the-hundred.md` A6 - minus that drawing's deliberate V11
/// violation, and minus the four other places the picture disagrees with the
/// checks it is drawn to illustrate. The spec's own caption says which of the
/// two is the spec: "the checks are the spec, the picture is not."
///
/// It has to pass every check in [`refusals`], and
/// `county::the_fallback_passes_every_check` is what holds it there. A
/// generator whose only known-good output is one it produced itself has
/// checks nobody can falsify.
///
/// ```text
///           A       B       C       D       E       F       G
///        +-------+-------+-------+-------+-------+-------+-------+
///  N   1 |   .   |  ~R2  |  mKET |  E    |  E    |  mMAN |  ^S2  |
///      2 |  mXL  |  o B2 |  E *  |  o T2*|  E *  |  o S2*|  ~F5m |
///        +-------+-------+-------+-------+-------+-------+-------+
///  M   3 |  ~R4  |  E *  |  o S1 |  PALE |  o B1 |  E *  |  mHW  |
///      4 |  #H3  |   . * |  ^D2.5|   J   |  ^S4  |   . * |  ~F3p |
///      5 |  #G   |  o T1*|  #H5  |  O ORD|  E    |  E  * |  ^D2  |
///        +-------+-------+-------+-------+-------+-------+-------+
///  S   6 |  mSUM |  o S3*|  o T3*|   . * |  E  * |   . * |   .   |
///      7 |  E    |  O DRO|  #G   |  E    |  mSLA |  o B3 |  O ENC|
///        +-------+-------+-------+-------+-------+-------+-------+
///
///  E event   . empty   J gaol   PALE the gate   O pinnacle   o objective
///  m<town> a mouth      * the Drover's circuit
///  T trig point (Ordnance)  S sign tile (Drove)  B boundary stone (Enclosure)
///  F7 and G7 and G6 are behind the pale.
/// ```
const FALLBACK_KINDS: [TileKind; TILES] = [
    // Row 1
    TileKind::Empty,
    TileKind::Feature(Toll::River { milli_per_s: 2000 }),
    TileKind::Empty,
    TileKind::Event(UNARRANGED),
    TileKind::Event(UNARRANGED),
    TileKind::Empty,
    TileKind::Feature(Toll::Scarp { milli_per_s: 2000 }),
    // Row 2
    TileKind::Empty,
    TileKind::Objective { chain: Chain::Enclosure, nth: 2 },
    TileKind::Event(UNARRANGED),
    TileKind::Objective { chain: Chain::Ordnance, nth: 2 },
    TileKind::Event(UNARRANGED),
    TileKind::Objective { chain: Chain::Drove, nth: 2 },
    TileKind::Feature(Toll::Ford { lane: Lane::Magic, milli_per_s: 5000 }),
    // Row 3
    TileKind::Feature(Toll::River { milli_per_s: 4000 }),
    TileKind::Event(UNARRANGED),
    TileKind::Objective { chain: Chain::Drove, nth: 1 },
    TileKind::Event(PALE),
    TileKind::Objective { chain: Chain::Enclosure, nth: 1 },
    TileKind::Event(UNARRANGED),
    TileKind::Empty,
    // Row 4
    TileKind::Feature(Toll::Hedge { curse_resist: 3 }),
    TileKind::Empty,
    TileKind::Feature(Toll::Drift { fastest_ms: 2500 }),
    TileKind::Gaol,
    TileKind::Feature(Toll::Scarp { milli_per_s: 4000 }),
    TileKind::Empty,
    TileKind::Feature(Toll::Ford { lane: Lane::Physical, milli_per_s: 3000 }),
    // Row 5
    TileKind::Feature(Toll::Gate { bounties: 1 }),
    TileKind::Objective { chain: Chain::Ordnance, nth: 1 },
    TileKind::Feature(Toll::Hedge { curse_resist: 5 }),
    TileKind::Pinnacle { chain: Chain::Ordnance },
    TileKind::Event(UNARRANGED),
    TileKind::Event(UNARRANGED),
    TileKind::Feature(Toll::Drift { fastest_ms: 2000 }),
    // Row 6
    TileKind::Empty,
    TileKind::Objective { chain: Chain::Drove, nth: 3 },
    TileKind::Objective { chain: Chain::Ordnance, nth: 3 },
    TileKind::Empty,
    TileKind::Event(UNARRANGED),
    TileKind::Empty,
    TileKind::Empty,
    // Row 7
    TileKind::Event(UNARRANGED),
    TileKind::Pinnacle { chain: Chain::Drove },
    TileKind::Feature(Toll::Gate { bounties: 1 }),
    TileKind::Event(UNARRANGED),
    TileKind::Empty,
    TileKind::Objective { chain: Chain::Enclosure, nth: 3 },
    TileKind::Pinnacle { chain: Chain::Enclosure },
];

pub const FALLBACK: County = County::of(
    FALLBACK_KINDS,
    (3, 4),
    [Bearing::Row, Bearing::Column, Bearing::Rising],
    (3, 2),
    [(6, 6), (5, 6), (6, 5)],
    ATTEMPTS,
);
