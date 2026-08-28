# THE HUNDRED - a county under the road, three claims on it, and the walk that settles them
## Design plan and execution spec - third draft, refined for execution

**Written against commit `2f7f426`** on `main`. The Switchyard has landed
(`Where::Siding` at `pedestal.rs:101`, `design/HANDOFF-switchyard.md` in the
tree, its chain at rung indices 20/24/27/33). Every `file.rs:line` was read at
this tip; nothing was compiled, so every difficulty claim names the replay that
would prove it. Where the code has moved by the time this is executed, **the
code is the news** and the difference goes in `analysis/the-hundred.md`.

**What changed in this draft.** Nothing in the design; everything in the
precision. The second draft settled seven decisions; this one specifies the
engine surface those decisions imply, because the gap between "the Drover walks
a circuit" and code is exactly the gap an agent fills with guesses. New:
movement and resolution order (A2.1), the run state field by field with the trip
census as an enum (A2.2), the six toll formulas in integer form with starting
thresholds (A3), the clock's increment points enumerated (A5), the bearing
geometry (B1.1), the circuit enumerated (B2.1), the pale's checklist as named
`Requirement`s (B3.1), the perambulation's exact rule (B5), the consolidated
engine-surface table (A9), and corrected frame counts throughout (five, not
ten). One design change, flagged where it happens: the Surveyor's Orb goes
through the pedestal system rather than a new use-from-tray verb (B1.2).

**The county is called THE HUNDRED** - a subdivision of a shire, and a count.

---

# PART A - THE PRIMITIVE

## A0. What the code does today, verified

- **Six towns**, three pinned, three hidden: Sump Bottom after rung index 6,
  Extra Large after 13 (hidden), Kettleworks after 17, The Manse after 24
  (hidden), High Wick after 31, The Slagworks after 33 (hidden)
  (`town.rs:329-465`). Doors belong to the town, not the idea of a town
  (`town.rs:313-318`): a door added to all six is six edits.
- **The PRNG is one stream.** `Rng` is xorshift64\* (`rng.rs:7-33`); `Run::rng`
  (`run.rs:701`) feeds shop stocks, drops, melts, lots. **Never draw county
  generation from it.** Copy `wildcard_seed` (`run.rs:2884-2886`): derive a
  seed, use a throwaway `Rng::new(that)`.
- **Mana is per activation** (`stats.rs:19-20`); `regen` is the only per-second
  stat (`combat.rs:915`). A3 is built on this.
- An assembled item exposes `cooldown_ms` (`loadout.rs:44`), `stats`, `slot`,
  `assembled` (`:138`) - a per-second figure is computable without a fight.
- **Rogue holds lives** (`run.rs:796`, `:3569-3573`); a death spends one and
  the run continues.
- **The map is one untabbed overlay**: `render_route` (`gui/main.rs:6925`),
  M/Escape (`:12279-12303`); the tab pattern to copy is the glossary's
  (`:9499-9520`); `route::ascii` prints the same map headless.
- **Event vocabulary** (`event.rs:14-175`): `Requirement::{None,
  LooseItemOfSize, Took, Holding, Flag, Counter, AssembledOfRarity,
  AlignedItems, Purse, ...}`; `Outcome::{FightAsWritten, FightInstead, BuyOff,
  Enter, Spare, Claim, Give, Step, Stock, Flag, Count, RevealTown, OpenShop,
  StartDungeon, GrantRow, GrantQuest, ClaimTicket, StandingOrder, Underwrite,
  UnlockInsight, Pay, Purse, All, ...}`. `Phase` is `Loadout | Fighting`
  (`run.rs:19-24`).
- **Rumours**: `every_rumour_can_be_come_by` knows two sources - an event's
  `Give`, a town door's `gives()` (`rumour.rs`). A county tile is a third and
  needs an arm (B6).
- **Free rung indices**: 0, 1, 4, 5, 6, 7, 11, 13, 14, 17, 18, 25, 32, 34, 37,
  39, 42, 43, 44; bosses at 14, 30, 46, so 14 is out. Thirty-eight events.
- **The Switchyard's verbs are reusable wholesale**: graph, cleared-node
  memory, partial traversal, re-entry, leaving. The county is that primitive in
  two dimensions with a step budget.

## A1. The grid, and how it is generated

**Seven by seven, forty-nine tiles**, columns `A-G` west to east, rows `1-7`
north to south. Three **regions** partition it by row: NORTH rows 1-2 (14
tiles), MIDDLE rows 3-5 (21), SOUTH rows 6-7 (14).

```
seed = run_seed ^ (mode as u64) << 40 ^ (difficulty as u64) << 44
```

`county::generate(seed) -> County` is a **pure function**: same inputs, same
county, re-derivable, never stored, never touching `Run::rng`.

**Generation arranges authored content and never writes prose.** Every string
is authored and linted; the generator decides only which authored tile goes
where.

### A1.1 Placement: validity checks instead of pins

Trig points, sign tiles, boundary stones, the pale, the pinnacles and the gaol
are placed by the generator, subject to checks it must satisfy before
returning. On failure it re-derives from `seed.wrapping_add(1)` and retries, at
most **32 attempts**, then returns the **hand-authored fallback county** - a
`const FALLBACK: County` checked into `county.rs`, itself required to pass every
check by a test. Attempt count is part of the pure function, so replay holds.

| # | Check |
|---|---|
| V1 | Every objective, pinnacle, the pale and the hill (B1.1) reachable from at least one mouth by a path of length <= 5 crossing at most one toll |
| V2 | Each chain's three objectives reachable from three different mouths between them |
| V3 | No two objectives of one chain adjacent |
| V4 | Each chain's three objectives in three different regions |
| V5 | The pale not on an edge, at least 2 tiles (Manhattan) from every mouth, and **not on the circuit** (B2.1) |
| V6 | Pinnacles pairwise >= 3 apart (Manhattan), none adjacent to a mouth |
| V7 | County connected ignoring tolls; every tile within 8 moves of some mouth |
| V8 | No objective or pinnacle enclosed so every path to it crosses 2+ tolls |
| V9 | Gaol within 3 tiles of D4 |
| V10 | Composition within one tile of A1.2 |
| V11 | The circuit's sixteen tiles contain no Feature tile |
| V12 | The hill's three bearing lines are pairwise distinct and concurrent only at the hill (B1.1); the hill is an Empty non-edge tile |

V1, V8 and V12 will do the actual rejecting. A retry-rate histogram over ten
thousand seeds is an F1 deliverable; over 1% retries means a check is too tight.

### A1.2 Composition

| Kind | Count |
|---|---:|
| Event tiles (13 skeleton + 12 arranged from the county pool) | 25 |
| Feature tiles (tolls) | 12 |
| Empty | 12 |

Skeleton: nine objectives, three pinnacles, the gaol. The twelve arranged event
tiles are drawn from `COUNTY_EVENTS`, never from the road's `EVENTS`.

## A2. Tiles, movement, and resolution

```rust
pub enum TileKind {
    Event(&'static str),                       // id into COUNTY_EVENTS
    Feature(Toll),
    Empty,
    Objective { chain: Chain, nth: u8 },       // Chain::{Ordnance,Drove,Enclosure}
    Pinnacle { chain: Chain },
    Gaol,
}
pub struct Tile { pub kind: TileKind, pub at: (u8, u8), pub region: Region }
```

`COUNTY_EVENTS` is a table of the **same struct the road uses** - `LadderEvent`
with `at: usize::MAX` and a doc comment saying the field is dead here - so the
choice, requirement, outcome, prose-lint and theme machinery all apply
unchanged. A county event's outcomes are restricted: **no `FightAsWritten`,
`FightInstead`, `Step`, `Enter`, or `StartDungeon`** - the county's only fights
are pinnacles and THE PARISH - and a lint,
`county::county_events_never_fight`, enforces the restriction so it cannot rot.

### A2.1 A move, in order

Movement is **orthogonal only** - N, S, E, W. One move:

1. Refused if `moves_left == 0`, if a county event is pending, or if
   `phase != Loadout`.
2. The target tile is inspected. If it is a **Feature** whose toll you do not
   meet: `moves_left -= 1`, you do not move, receipt says which figure fell
   short and by how much. Done.
3. Otherwise `moves_left -= 1`, `at_tile` moves.
4. If the tile is **cleared already**: nothing resolves; receipt notes the walk.
5. Otherwise it resolves by kind:
   - **Empty** - marked cleared.
   - **Feature** (met) - marked cleared; crossing is permanent, a bridge you
     paid for once.
   - **Event** - the event is set pending exactly as `forced_event` works
     (`run.rs:649-655`); the player answers it on the event screen; answering
     marks the tile cleared and **increments the clock** (A5).
   - **Objective** - resolves as its chain says (B1-B3); marked cleared; if it
     carries an event, the event rule applies.
   - **Pinnacle** - if its chain's gate is met (B1-B3), the fight begins:
     `Run::monster()` returns the pinnacle's spec with priority **dungeon,
     then county pinnacle, then brawl, then ladder** (extending
     `run.rs:887-892`); victory marks it cleared and pays the chain; loss is
     A7. If the gate is unmet, the tile says so and is not cleared.
   - **Gaol** - marked cleared; prose only.
6. If `moves_left == 0` and no event is pending, the trip ends and you are
   returned to the town entryway you left, exactly as a pedestal dungeon
   returns you.

**The Drover check** runs after steps 3 and after any clock increment: if the
Drove chain is live (>= 1 sign cleared) and `at_tile == drover_tile()`, the
pursuit fight begins (B2.2).

While in the county, `road_stack()` shows `Interrupt::County { at, moves_left }`
on top; `blocks_a_rematch()` is true; the banner reads
`THE HUNDRED - <tile name> - <n> moves left`. The CLI verbs are `go` (enter,
from a town), `walk n|s|e|w`, `out`, and the existing `answer <n>` for county
events.

### A2.2 Run state, field by field

```rust
// --- on Run ---
/// Where you are, when you are down there.
pub county_at: Option<(u8, u8)>,
pub county_moves_left: u8,                      // 0..=5
/// Cleared for the whole run. Rogue keeps it across lives (A7). wipe() clears.
pub county_cleared: Vec<(u8, u8)>,
/// The census (A4). One entry per trip, at most one per variant except Town,
/// which is at most one per town id.
pub county_trips: Vec<TripSource>,
/// The clock (A5).
pub events_resolved: u32,
/// Sightings taken (B1), sign tiles read (B2), pale lines met (B3) are all
/// derivable from county_cleared and the tables; nothing extra is stored.
/// The Waste bet (C2): which grid, and the rung it must stay empty until.
pub waste_bet: Option<(SlotKind, usize)>,
/// C1: set when a trip ends with zero tiles cleared, or a toll was failed
/// and the run left the county without ever crossing it.
pub county_business: bool,

pub enum TripSource {
    Town(&'static str),      // one per town id; six towns
    SurveyorsOrb,            // B1.2
    WasteBet,                // C2
    Constable,               // C1
    Perambulation,           // B5, granted not taken
}
```

**The cap is the census**: `county_trips.len()` may never exceed
`3 + 3 + 1 + 1 + 1 + 1 = 10`, and the test that guards it counts the enum -
`TripSource` variants (with `Town` weighted by `TOWNS.len()`) must equal the
cap, so adding a source without raising the cap fails the suite. A town's
`Action::County` door refuses a second use with a line; it does **not** cost
the visit, the `Pedestal` precedent (`town.rs:112-114`).

`county: County` itself is **derived on demand** from the seed, never stored;
`wipe()` clears every field above.

## A3. Tolls: six derived figures, in integer form

A toll reads a figure derived from the assembled board - never a raw stat -
and taxes a move on failure. All figures are over `assembled` items only.

| Toll | Figure, exactly | Starting thresholds (F11 pins) |
|---|---|---|
| **River** | `flow = sum(stats.mana * 1000 / cooldown_ms)` milli-mana/s, display `/1000` | 2000, 3000, 4000, 5000, 6000 milli/s as `~R2..~R6` |
| **Ford** | `dps(lane) = sum(stats.physical_damage * 1000 / cooldown_ms)` or magic; the ford names its lane | 3000-6000 milli/s |
| **Scarp** | `armour_ps = sum(stats.armor * 1000 / cooldown_ms)` | 2000-5000 milli/s |
| **Drift** | `min(cooldown_ms)` over assembled items `<= n` | 2500, 2000, 1500 ms as `^D2.5..^D1.5` |
| **Hedge** | `sum(stats.curse_resist) >= n` | 3, 5, 8 |
| **Toll gate** | `gold >= n * bounty(rung)`; crossing **spends** it | 1x |

Integer division per item, summed - no floats anywhere. The worked pair that
justifies flow: 8 mana on 4,000 ms pays 2000 milli/s; 3 mana on 1,000 ms pays
3000; the worse-looking piece crosses the deeper river, and
`tests/tolls.rs::flow_is_not_mana` asserts exactly that pair.

**Failing costs the move and leaves you in place.** No damage, no loss. A met
Feature is cleared permanently. **A threshold is visible from one tile away and
not before**; the Surveyor's sheet (B1) shows all of them from anywhere.

## A4. Trips: ten is a census

Five moves a trip; arriving on the mouth's edge tile is free and resolves it.

| # | `TripSource` | Guaranteed? |
|---:|---|---|
| 1-3 | `Town` x pinned | yes |
| 4-6 | `Town` x hidden | if found |
| 7 | `SurveyorsOrb` | one chain deep |
| 8 | `WasteBet` | a build decision |
| 9 | `Constable` | unasked, punitive |
| 10 | `Perambulation` | all three chains; granted |

Arithmetic, from the A6 map: one chain costs 8-11 moves all in (three
objectives + 3-5 travel + 1-3 pinnacle + 0-2 failed tolls). So **four or five
trips finishes two chains; three chains needs about seven; ten is the run that
did everything** and is the only run that can also afford the perambulation.
The deeper limit is that the chains tax five basis vectors between them (Part
B), and a board that crosses rivers is not a board that climbs scarps.

## A5. The clock

`events_resolved` increments in exactly three places, and a test walks a
scripted run asserting the total at four checkpoints:

1. A road event answered - every path through `take_choice` / the event's
   answer step, including chain doors and dungeon-mouth events.
2. A county event answered.
3. Nothing else. Not fights, not tiles walked, not towns, not tolls.

```
drover_tile = CIRCUIT[ events_resolved as usize % 16 ]
```

The county is alive while you walk the road; a player one tile short of an
interception can go up, answer a door, and come back. Discovered, not
explained.

## A6. The sample map

One seed, illustrative, passing every check; regions marked at the left edge.

```
                                 THE HUNDRED
                         Kettleworks        The Manse
                          (after 17)        (after 24)
                              |                  |
              A       B       C       D       E       F       G
          +-------+-------+-------+-------+-------+-------+-------+
 N      1 |  E    |  ~R2  |  o T1 |  E    |   .   |  ^S2  |  O SUR|
 O        +-------+-------+-------+-------+-------+-------+-------+
 R      2 |  ^S2  |   *   |  E *  |  ~R3* |  E *  |  E *  |   .   |
 T Extra  +-------+-------+-------+-------+-------+-------+-------+
 H Large-3 |  E   |  o B1*|   .   |  E    |  #H5  |  o T2*|  E    | - High
   (a.13) +-------+-------+-------+-------+-------+-------+-------+   Wick
 M      4 |   .   |  E  * |  ~R3  |  J    |  E    |   .  *|  ~F4  | (a.31)
 I        +-------+-------+-------+-------+-------+-------+-------+
 D      5 |  E    |  E  * |  o S1 |  E    |  o B2 |  E  * |  ^D2  |
 D Sump   +-------+-------+-------+-------+-------+-------+-------+
 L Bottom-6 | o S2 |   *   |  E *  |  #G * |  E *  |  o T3*|  E    |
 E (a.6)  +-------+-------+-------+-------+-------+-------+-------+
 S      7 |  O DRO|  ^S3  |  o S3 |   .   |  ~R4  |  PALE |  O COM|
 O        +-------+-------+-------+-------+-------+-------+-------+
 U            (no mouth)                        |
 T                                          The Slagworks (a.33)
 H
  E event   . empty   J gaol   PALE the gate   O pinnacle   o objective
  * the Drover's circuit: B2 C2 D2 E2 F2 F3 F4 F5 F6 E6 D6 C6 B6 B5 B4 B3
  T trig points (Ordnance)   S sign tiles (Drove)   B boundary stones (Enclosure)
  ~Rn river flow>=n000   ~Fn ford dps>=n000   ^Sn scarp armour/s>=n000
  ^Dn drift fastest<=n*1000ms   #Hn hedge curse resist>=n   #G toll gate 1x bounty
  SUR the Surveyor   DRO the Drover (on the circuit)   COM the Commissioner
```

Note the map obeys its own rules: the circuit's sixteen tiles carry no toll
(V11 - D2's river sits on the ring in this drawing only to show what V11
*rejects*; the generator would have retried this seed, and the caption says so
deliberately: **the checks are the spec, the picture is not**).

**A worked trip.** In from Sump Bottom: A6 is sign tile S2, resolved free.
B6 (1, circuit tile, empty), C6 event (2, clock +1, Drover advances), C5 sign
tile S1 (3), C4 river R3 - flow 3000 needed - crossed (4), C3 empty (5). Trip
over: two signs, one event, a bridge bought. Failing R3 twice instead ends the
trip at three tiles and sets nothing.

## A7. Losses, Rogue, leaving

Entering never advances the rung; you return to the gate you left by. A county
loss costs what a road loss costs - Grinder knock-back or a Rogue life - and
ends the trip. **County progress survives a Rogue death**: `county_cleared`,
the census, the clock, the bet - all kept when a life is spent, because the
county is a place, not an attempt, and it is where the endgame lives. Leaving
early is free and forfeits only the moves; moves never bank.

## A8. The map screen

Two tabs on the M overlay, glossary pattern (`gui/main.rs:9499-9520`):
**[ THE ROAD ] [ THE HUNDRED ]**. Road tab byte-identical to today, proven by
fixture. The Hundred tab draws the 7x7 in the road's own vocabulary - filled
cleared, ringed standing, hollow seen-but-uncleared, blank never-adjacent;
tolls show glyphs always and thresholds only when known (adjacency or the
sheet); the Drover drawn from the clock only after a sign is cleared; mouths
labelled with their towns and greyed until found; the pale's checklist inline
at one tile. Greyed with a line before the first visit. **`route::ascii` gains
the county half in the same milestone**, so the CLI prints the same map and a
fixture can diff it.

## A9. Engine surface, consolidated

Every new thing, in one table, so nothing is discovered mid-milestone:

| Kind | Name | Where | Milestone |
|---|---|---|---|
| module | `county.rs`: `County`, `Tile`, `TileKind`, `Toll`, `Region`, `Chain`, `CIRCUIT`, `FALLBACK`, `generate` | engine | F1 |
| fields | `county_at`, `county_moves_left`, `county_cleared`, `county_trips`, `events_resolved`, `waste_bet`, `county_business` | `Run` | F2, F3 |
| enum | `TripSource` (5 variants) | run.rs | F2 |
| variant | `Interrupt::County { at, moves_left }` | run.rs | F2 |
| fns | `enter_county(TripSource, mouth)`, `county_walk(dir)`, `leave_county()`, `drover_tile()` | Run | F2 |
| variant | `Action::County` on all six towns | town.rs | F2 |
| fns | six toll figures over `&Loadout` | loadout.rs | F4 |
| variants | `Requirement::CountyTiles { region, at_least }`, `Requirement::CountyCleared(Chain)` | event.rs | F4 |
| effects | `Bearing`, `Overtake`, `Commons` + weights, rows, tooltips | piece/rating/catalog_shape | F5 |
| pieces | 3 enchantments, 2 orbs, event-only, one block | CATALOG | F6 |
| variant | `Where::County` + destination via Surveyor's Orb | pedestal.rs | F8 |
| table | `COUNTY_EVENTS` (8 authored, `LadderEvent` shaped) | event.rs | F7 |
| lint arm | third rumour source: a county tile's `Give` | rumour.rs tests | F7 |
| frames | THE SURVEYOR, THE DROVER, THE DRIVEN, THE COMMISSIONER, THE PARISH | bestiary/combat | F8 |
| tab | the Hundred on M; `route::ascii` county half | gui, route.rs | F9 |
| verbs | `go`, `walk`, `out` | cli | F2, F9 |

Not touched: `share.rs` (the county is derived, never encoded), `slot.rs`,
`LADDER`, existing town doors, `CLASSES`, `dungeon.rs`.

---

# PART B - THREE CLAIMS ON THE SAME COUNTY

| | THE ORDNANCE | THE DROVE ROADS | THE ENCLOSURE |
|---|---|---|---|
| Verb | triangulate | pursue | unseal |
| Road on-ramp | THE THEODOLITE, index 11 | THE STOCKMAN, index 25 | THE COMMONS, index 17 |
| Objectives | 3 trig points | 3 sign tiles | 3 boundary stones |
| Tolls taxed | Scarp (Chest), Drift (Greaves) | River (Helmet), Ford (Weapon) | Hedge (Gloves), Toll gate (gold) |
| Pinnacle | THE SURVEYOR, band 35 | THE DROVER + THE DRIVEN, band 42 | THE COMMISSIONER, band 48 |
| Pays | **trips** | **moves** | **power** |
| Enchantment | greaves | gloves | chest |
| New effect | Bearing | Overtake | Commons |

Equal in cost and magnitude, different in currency; the order is a real
decision. Bands take the ladder's stats at band, the Switchyard precedent.

## B1. THE ORDNANCE - the county is measured

**Road.** THE THEODOLITE, rung index 11, `Trigger::Rung`, unconditional, before
the first hidden gate at 14. Hands the word, teaches the geometry.

### B1.1 The bearing geometry, exactly

At generation the county picks **the hill** - an Empty, non-edge tile - and
places the three trig points so that each **shares a line with it**: same row,
same column, or same exact diagonal. The three lines must be pairwise distinct,
so their only common tile is the hill (V12), and the generator verifies it by
construction.

Clearing trig point *n* takes its **sighting**: the full line through that trig
point is drawn on the map tab from then on. One line is a line. Two lines
cross at exactly one tile - **so two sightings are knowledge**: a player who
draws them knows where the hill is and can route toward it. **The third is the
key, not information**: only when all three are taken does the tile at the
crossing *become* THE UNMARKED HILL - `TileKind` rewritten from `Empty` to
`Pinnacle { Ordnance }` in the derived county view - and the Surveyor is on it.
The game never marks the hill; the lines do.

Stepping on the hill early resolves it as the empty tile it still is, and that
tile being cleared does not stop it becoming the hill - a cleared tile that
becomes a pinnacle is uncleared by the becoming, and a test says so, because
that is exactly the sort of edge an agent otherwise decides silently.

### B1.2 The reward: trips

The **sheet** (every threshold visible from anywhere, permanent, a `Run` flag);
a **greaves enchantment**; and the **Surveyor's Orb**. *Design change from
draft two:* the orb is spent at a **pedestal**, like every orb - a new
`Destination { via_orb: "Surveyor's Orb", kind: Where::County }` - and entering
by it offers **a choice of any of the six mouths, found or not**. The draft-two
version ("opens a mouth at your current rung") needed a new use-from-tray verb;
this version is one `Where` arm on a system with two precedents, and mouth
choice is the value that survives the translation. Pedestals stand after 13 and
after 31; the Ordnance finishes mid-run; High Wick is the natural spender.

**Bearing** (the effect): *this item's stats count double while it is the only
assembled item in its slot.* Greaves-only; integer; checked at loadout
recompute, not per tick.

## B2. THE DROVE ROADS - the county is walked

**Road.** THE STOCKMAN, rung index 25, `Whispered` from 25, deadline 25 - a
one-rung window on a bare rung, one clear of High Wick's gate at 32; its word
is sold at the Kettleworks pub or given by a county tile (B6), whichever the
run finds first.

### B2.1 The circuit, exactly

`CIRCUIT: [(u8,u8); 16]` - the ring of the inner 5x5, clockwise from B2:

```
B2 C2 D2 E2 F2  F3 F4 F5  F6 E6 D6 C6  B6 B5 B4 B3
```

V11: none of the sixteen is a Feature tile. V5: the pale is not on it. The
Drover stands at `CIRCUIT[events_resolved % 16]` from the run's first event -
it was always walking; you just could not see it before a sign tile taught you
to look.

Each **sign tile**, cleared, prints: what came through, its direction, and
`events_resolved` *at the time of printing* - so the player has the clock, the
ring, and the modulus, and interception is a subtraction. The map tab draws the
Drover once one sign is cleared.

### B2.2 The interception

After any move, and after any clock increment while standing still (answering a
county event can bring it to *you*, which is the best thing in the chain), if
at least one sign is cleared and `county_at == drover_tile()`: the fight. It is
a **brawl** - THE DROVER and THE DRIVEN together, `Brawl` machinery as-is -
because a drover without a herd is a man on a walk. Victory clears the chain;
the Drover leaves the circuit (drawn no more; interception check off).

**The Drover gets stronger with the clock**: its spec at fight time gains
`+strength per 8 events_resolved`, stated in its note, so a run that dawdled
meets a harder drover - pursuit punishing patience, and the sudden-death budget
enforced from the other side.

**Reward: moves.** A **gloves enchantment**, and the **Drover's Orb** - *not* a
pedestal orb: held, it adds a passive to county walking - **the first move of
every trip is free**. (Draft two's "move three tiles" was an activated verb
with UI; a passive per-trip discount is the same currency with no new surface,
worth up to 6 moves across a full census.) It is still an Orb-kind weapon core
first, worth building around.

**Overtake**: *the first time this item fires in a fight, it fires again
immediately.* Gloves-only; the echo activation cannot itself Overtake; resolves
inside the activation walk.

## B3. THE ENCLOSURE - the county is fenced

**Road.** THE COMMONS, rung index 17 (Kettleworks' gate rung; gate pops
first). It explains the fence and gives nothing; the fence was always there.

### B3.1 The pale, its checklist, and its opening

The pale is a sealed tile. Standing **one tile away** shows the checklist -
five lines, each a `Requirement`, ticked live by the same machinery a choice's
`requires` uses:

| Line | Requirement |
|---|---|
| Six tiles cleared in the NORTH | `CountyTiles { region: North, at_least: 6 }` (new variant, F4) |
| Six tiles cleared in the MIDDLE | `CountyTiles { region: Middle, at_least: 6 }` |
| Six tiles cleared in the SOUTH | `CountyTiles { region: South, at_least: 6 }` |
| The two boundary stones read | `Counter { what: "boundary-stones", at_least: 2 }` - each stone's tile `Count`s it |
| An orb surrendered at the gate | met at the gate itself: stepping on the pale with the four lines ticked offers one choice, `requires: Holding` any Orb-kind piece, and answering consumes it |

Eighteen cleared tiles across three regions *is* most of two trips' work by
itself, which is why the Enclosure pays power: it is the chain you finish by
having been everywhere. The third boundary stone is behind the pale - the
chain's own joke - and `Count`s to 3 for THE LAST of its prose, not for the
gate.

Opening the pale rewrites the county's far corner: the region behind it (the
pale's own row segment to the corner) flips from unenterable to normal, and
THE COMMISSIONER stands at its end. The county visibly grows.

**Commons** (the effect): *this item's stats count as if it were adjacent to
every assembled item on its board.* Chest-only; loadout-recompute, not per
tick; the rating prices it as the adjacency it claims, which is the test of
whether `rating.rs` can price adjacency honestly.

## B4. How the three hint at each other

Facts about the map, never quest markers: one of the Surveyor's three lines
passes through a circuit tile and its sighting prose says something stood on
the line when he took it; the circuit bends past the pale's row and a player
plotting it learns the pale is there before THE COMMONS says so; the boundary
stones are cut by the same hand as the trig points and the prose lets a player
who did the Ordnance read them faster.

## B5. THE PERAMBULATION

All three chains done: the tenth trip is **granted** - `TripSource::
Perambulation` - and it is a route, not a destination. Enter at any mouth;
every move must land on an **edge tile** (row 1, row 7, column A or column G),
always clockwise or always counter-clockwise, chosen by the first move; tolls
on the boundary must be paid; any illegal move, or a failed toll, breaks the
walk and the trip is spent. The **fifth** edge tile reached is where THE PARISH
stands - band 50-plus, the hardest authored thing in the game, testing all
five basis vectors, because the county has spent thirty tiles proving whoever
got here has all five.

## B6. Words that cross both ways

Three shapes, all mostly-existing machinery: a county tile's `Give` opening a
road door (the good one - a run comes back up with something to do); a road or
pub word opening a county tile that is inert without it (`Rumour.opens` already
takes an id; a county tile keyed by it needs one lookup); a hidden town's
`gives()` opening a county tile (a third reason to find the Manse). All
`Condition::Carried`. **The `every_rumour_can_be_come_by` third arm - a county
tile's `Give` - lands in the same milestone as the first county word**, or the
lint fails the day the word exists.

---

# PART C - THE TWO STANDALONE EVENTS

## C1. THE CONSTABLE

Rung index 18 (Kettleworks' gate rung; gate first), `Trigger::Rung`, standing
only when `county_business` is true - set when a trip ends with zero tiles
cleared, or a toll was failed and the run left without ever crossing it, and
cleared when he collects you. He takes you down: `TripSource::Constable`,
placed on the gaol tile, five moves, no choice about it.

**The gaol is within three tiles of centre (V9), and the mouths are on the
edges - so being arrested is the fastest ride into the middle that exists.** A
player will fail tolls on purpose to get sent down, and that is allowed to
work: a punishment a clever player farms beats one a careful player avoids. It
spends census slot nine, which is the real price. The doc comment says all of
this so the next mission does not patch it.

## C2. THE WASTE

Checked in `settle` after any won fight past rung index 15: if any of the five
grids has **zero assembled items**, and `waste_bet` is `None`, and the event
has not been declined-forever, it fires (an off-rung event via `forced_event`).

- **Let them improve it** - they fill the slot with a cheap piece you did not
  choose, small gold, never fires again.
- **Take the bet** - `waste_bet = Some((that_grid, rung + 5))`. Settle checks
  it each fight: still empty at the deadline pays `TripSource::WasteBet`;
  filled early owes the gold instead. Either way the bet clears.
- **Spoken for** - nothing; may fire again.

The bet is census slot eight - the one that makes three chains arithmetically
possible - and it is paid for by fighting the back half on four grids. Bearing
(B1) and the Switchyard's ground both pay the same build; three systems
rewarding an empty grid is what makes a build feel discovered.

---

# PART D - DECISIONS SETTLED AND STILL OPEN

**Settled:** clock = events resolved (A5); generator places everything under
V1-V12 (A1.1); Rogue keeps the county (A7); the checklist at one tile (B3.1);
ten trips as a census enum (A2.2, A4); words cross both ways (B6); the second
map tab (A8); Surveyor's Orb through the pedestal (B1.2); Drover's Orb as a
per-trip passive (B2.2); county events never fight (A2); frames are five.

**Open:**

**D-1. Two-grid bonding stays out.** The Enclosure pays a single-grid chest
enchantment; a cross-grid bond is a change to the bond layer and is its own
mission. *Recommendation: confirmed out.*

**D-2. Eight county events into twelve slots** (four repeats) rather than
twelve thin ones. *Recommendation: yes.*

**D-3. The fallback county is authored in F1**, first, as the fixture
everything else tests against. *Recommendation: yes, and the sample map in A6
is its starting sketch - minus the deliberate V11 violation.*

**D-4. Should the Drover's strength scale (`+1 per 8 events`) exist at all?**
It punishes slow runs twice - harder drover *and* fewer events left. Cheap to
cut. *Recommendation: ship it behind its own constant so F14's replay can zero
it in one line if the pursuit is already hard enough.*

**D-5. Nothing here has been compiled or played.** Bands, thresholds, the
five-move budget, 8-11 moves per chain: arithmetic off a paper map, measured in
F14, and the grid is the easier dial if the arithmetic is wrong.

---

# PART E - MILESTONES, DELIVERABLES, TEST GATES, DEPLOY POINTS

`HANDOFF.md` idiom. **All engine work lands first and inert; no creature gets a
board until the end** (`the-unwinding.md` E0). Branch `the-hundred`, merged
once. Every milestone writes its numbers to `analysis/the-hundred.md` under the
commit hash. Full suite once per milestone; iterate with `--lib` or one
`--test`; never two cargos at once.

### F0. Baseline
Printers + `report_shape`; fixtures: every creature's `gear_at` at every
difficulty; `route::ascii` at rungs 5, 20, 40. **Gate:** suite green, counts
recorded. **Exit:** the baseline block names its commit.

### F1. The county, generated
`county.rs` per A9 row one; V1-V12 as the module's own tests; the 32-attempt
retry; **the authored `FALLBACK` first** (D-3). **Gate:** purity (same seed
thrice, identical); 10,000 seeds pass or fall back, retry bound never exceeded;
`FALLBACK` passes V1-V12; composition within one tile; `county_events_never_
fight` (vacuous until F7, present now). **Exit:** ladder replays
byte-identically - nothing is wired. **Writes:** retry-rate histogram; >1%
means a check is too tight and the histogram says which.

### F2. Standing in it
A2.1 movement, A2.2 state, `TripSource`, `Interrupt::County`, `Action::County`
on six towns, mouths, CLI `go`/`walk`/`out`, `wipe`. **Gate:** five moves and a
free arrival; cleared tiles walk over silently; leaving forfeits moves only;
second use of a town's door refused with a line; **the census test** (variant
count, `Town` weighted by `TOWNS.len()`, equals the cap); Rogue life lost keeps
`county_cleared`; Grinder knock-back keeps it; two scripted trips diff clean.
**Exit:** a CLI script enters from all three pinned towns, walks fifteen moves,
twice, byte-identical.

### F3. The clock
`events_resolved` at exactly the A5 increment points. **Gate:** a scripted run
asserts the counter at four checkpoints; a county-free run's counter equals its
answered ids. **Exit:** green; nothing reads it yet.

### F4. Tolls, and the two Requirement variants
Six figures over `Loadout`; the tax; one-tile visibility;
`Requirement::CountyTiles` and `::CountyCleared` (inert). **Gate:**
`tests/tolls.rs` - each figure hand-computed in the assertion;
`flow_is_not_mana` on the A3 pair; a failed toll costs one move and no
position; visibility flips at exactly one tile. **Exit:** four-board table and
`gear_at` byte-identical to F0. **Writes:** the six figures for the owner's
board at rungs 10, 20, 30, 40 - **the table F11 sets thresholds from, which
must exist before any threshold is chosen.**

> **DEPLOY POINT 1 - after F4.** Walkable, taxing, empty. Deploy anyway: it is
> coherent, and it is the cheapest moment to learn that five moves feels wrong.

### F5. Bearing, Overtake, Commons - inert
Semantics per B1/B2/B3 with same-tick ordering stated; slot rows; weights;
tooltips; `describe`/`scaled`/`action_points` arms; naming words; glossary
chips. No carrier exists. **Gate:** each effect's positive and negative test
(Bearing un-doubles when a second item lands; Overtake's echo cannot Overtake;
Commons counts its own board only); `catalog_shape` green, three rows at
budget 0. **Exit:** four-board table and `gear_at` identical to F0;
`acceptance::e6_2` green.

### F6. The catalogue, once
Three enchantments, two orbs, one append-only block, all `EVENT_ONLY`.
**Gate:** `avail` (400 runs, never shelved); uniqueness in `enchantment`;
share codes round-trip; `catalog_shape` budgets 0 or lowered. **Exit:**
`gear_at` matches F0 - the measured form of "event-only re-gears nobody."

### F7. County events and the word crossing
Eight authored events (D-2), the pool, the arrangement; first crossing words
both directions; **the rumour lint's third arm in this same milestone** (B6).
**Gate:** `prose` green on every string; `rumour` green with the new arm;
`completable` +1 row for a county `Give`; a scripted trip resolves four county
events and the clock reads +4; `county_events_never_fight` now non-vacuous.
**Exit:** a trip is worth taking with no chain running.

### F8. The chains, as frames
Three on-ramps (11, 17, 25); nine objectives; B1.1 hill and sightings; B2.1-2
circuit, brawl, interception, the D-4 constant; B3.1 checklist, pale, the
region that appears; C1 and C2; B5's walk; `Where::County` and the Surveyor's
destination; **five frames** (SURVEYOR, DROVER, DRIVEN, COMMISSIONER, PARISH)
undressed at band stats. **Gate:** `tests/hundred.rs` - each chain completes in
isolation under `force_win`; the hill lands where hand-arithmetic says on three
fixed seeds, and a cleared tile un-clears when it becomes the hill; the Drover
is at `CIRCUIT[clock % 16]` at six checkpoints and the interception fires when
an answered county event brings it to you; every checklist line ticks exactly
when its Requirement is met; the perambulation refuses an illegal move; the
gaol reaches an objective in three moves; **frame lint red by five**, budget
re-pinned with the reason; **zero authored `gear:` boards**, grep-asserted.
**Exit:** a scripted run finishes two chains in five trips and prints moves
left over.

### F9. The map's second tab
`route::ascii` county half; the tabs; A8's drawing rules. **Gate:** road-tab
ASCII byte-identical to F0's three fixtures; a county ASCII fixture for a known
seed; tab greyed before first visit. **Exit:** CLI and GUI draw the same
county.

> **DEPLOY POINT 2 - after F9.** Playable, unbalanced: frames undressed,
> thresholds provisional. Deploy to learn whether two-chains-in-five-trips is a
> decision or a chore, while boards are still cheap.

### F10. Theme
`theme.rs` for the county, chains, five creatures, five components, six tolls'
vocabulary. **Gate:** `two_voices` at budget; `no_road_id_is_told_twice`;
`read_the_road_aloud` run **and read**. **Exit:** a themed run prints no
covered canonical noun.

### F11. Thresholds, chosen from F4's table
So the owner's board pays 3 of 6 at rung 12, 4 of 6 at 18 and 26. **Gate:**
that assertion, in `tolls.rs`. **Exit:** each number and its reason in
`analysis/`.

### F12. Boards, by hand
Five frames dressed in `make pack`, one at a time, **diff read after every
save** (trap 15). **Gate:** `pack` per creature inside the curve at Medium on
the owner's board; frame lint green at zero, budget retired. **Exit:**
transcripts in `analysis/`.

### F13. Rating pins
Three weights re-measured against five ratings and slot ceilings; **weights,
never thresholds**. **Gate:** `prices`; `catalog_shape`; `gear_at` fixture.
**Exit:** the numbers and reasons.

### F14. Acceptance, by replay
1. A two-chain five-trip script, piped twice, byte-identical; `e6_1` green.
2. F0 fixtures all clean (four-board table, `gear_at`, three `route::ascii`).
3. The two-chain script reports 0-4 moves left; negative shrinks the grid.
4. A maximal script finishes three chains and reports the trip - under seven
   means tolls are too cheap.
5. The census: cap == weighted variant count; the eleventh door refused.
6. Every pinnacle and THE PARISH finish inside 29 s at Medium on the owner's
   board, never by the clock; the Drover also at clock 300 with D-4's constant
   on and off.
7. `events_resolved` asserted at four checkpoints of a full run.
8. A Rogue script loses a life mid-county; tiles still cleared.
9. The gaol script reaches an objective in three moves; the assertion message
   says this is intended.
10. Words cross: one out of the county spent on the road, one down and spent
    below.
11. Suite green, no warnings, every re-pin justified.

### F15. The record
`design/HANDOFF-hundred.md` in the house shape; `CLAUDE.md` §3 counts and any
new §6 trap; merge; publish `docs/`.

> **DEPLOY POINT 3 - after F15.** The whole thing. The one to announce.

## Test inventory
New binaries: `county` (generation, checks, walking, trips, census, memory),
`tolls` (figures, tax, visibility, thresholds), `hundred` (chains, hill,
circuit, checklist, perambulation, gaol, clock, words). Extended: `effects` +3
pairs · `reactions` +1 · `catalog_shape` +3 rows · `rumour` +arm +2 ·
`completable` +1 row · `prose` + county scenes · `two_voices` + tellings ·
`avail` +1 · `enchantment` +1 · `route` +2 fixtures · `dungeons`/`chain`/
`progression` asserted unchanged.

---

## Kickoff prompt for Claude Code

> Read `CLAUDE.md`, then `design/HANDOFF.md`, then
> `design/HANDOFF-switchyard.md` (the last mission's ledger and the primitive
> this generalises), then `design/the-hundred.md` in full - it is the spec, and
> Part A9 is your map of every new engine surface. It was written against
> commit `2f7f426` without a toolchain: verify its `file.rs:line` citations
> against the tip before trusting them; where the code has moved, the code is
> the news, recorded in `analysis/the-hundred.md` under the commit hash.
>
> Create branch `the-hundred`. **F0 first**: printers, `report_shape`, and
> three fixture sets - `gear_at` for every creature at every difficulty,
> `route::ascii` at rungs 5, 20 and 40 - then the baseline block.
>
> Then **F1**: the generator as a pure function of the derived seed in A1,
> never touching `Run::rng` (that stream feeds shops, drops and melts; drawing
> from it breaks every replay in the suite). Author the `FALLBACK` county
> first, from A6's sketch minus its deliberate V11 violation; it is the fixture
> everything else tests against. Wire nothing to the run until F2.
>
> Rules for the mission: engine before content, and inert - every milestone
> through F6 exits on the four-board table and the `gear_at` fixture being
> byte-identical to F0. No content string before F7. No authored `gear:` board
> before F12, and F8 greps to prove it. The trip census is an enum and its test
> counts variants; if you add a trip source, the suite must force you to raise
> the cap. When a pinned number moves, re-pin with the reason in the assertion,
> never loosen. One cargo at a time; full suite once per milestone.
>
> Part D has five decisions with recommendations; take them unless I say
> otherwise and record which in `design/HANDOFF-hundred.md` as you go. Stop and
> report at the three deploy points, and stop before F12 - boards are packed by
> hand with me in the loop.
