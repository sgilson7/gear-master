# THE ATLAS — every place you have been, drawn

Written against `7443edc6a2c2` (2026-08-28), suite at **1056 engine green, 84
GUI, 12 CLI, 52 ignored, 0 warnings**. Every count below was measured off that
tip. This document is an execution spec for an agent picking the work up cold.

Read `CLAUDE.md` first. Then this.

---

## 1. What this is

Five changes, and four of them are one change wearing different hats. The map
knows about the road and about THE HUNDRED. It knows nothing about the seven
dungeons, the six orb destinations or the yard — which between them are most
of the places a run actually goes.

At the same time three of those places are thinner than the screen they would
be drawn on: a dungeon is a **line of two or three floors**, an orb
destination is **one event or one dungeon**, and the yard is nine floors that
read as a corridor rather than a yard.

So: give every place a map, and give three of them something worth mapping.
The fifth change is the shop that pays for the trip.

### The ground, measured

| Dungeon | Floors | Exits | Shape today |
|---|---:|---:|---|
| `the-crevice` | 3 | 2 | a line |
| `the-threshold` | 3 | 2 | a line |
| `the-under-mine` | 2 | 1 | a line |
| `the-undertow` | 2 | 1 | a line |
| `den-rivals` | 2 | 1 | a line |
| `wumpus-world` | 2 | 1 | a line |
| `the-switchyard` | 9 | 2 | a graph, one fork |

Six orb destinations (`pedestal.rs:53`): two events (`the-thrumbus-race`,
`mole-town`), two dungeons (`den-rivals`, `wumpus-world`), two sidings into
the yard (`the-up-line` → floor 5, `the-down-line` → floor 1).

### What already exists and must be used rather than rebuilt

- **`Floor` is a graph node.** `exits: &[Exit]` — empty is a buffer stop, one
  is the next room, two or more is a set of points with a `fork` scene. The
  Switchyard proved the primitive; nothing needs inventing to branch a
  dungeon.
- **`Floor.also: &[event::Outcome]`** applies outcomes on clearing a floor.
- **`Outcome::ShopAfter { shelves: &'static [&'static str] }`** already
  exists (`event.rs:244`), sets `run.shop_owed`, and is drained at
  `run.rs:4028`. **The threshold shop needs no new plumbing** — it needs a
  shelf constant and one `also`.
- **`map_tabs()`** (`main.rs:7067`) returns a fixed `[Rect; 2]` and
  `map_tab: usize` selects between `render_route` and `render_county_map`.
  Both halves are already pure-layout-plus-paint.
- **`county_cells()`** is the model for a grid map: pure, testable, and drawn
  from a table rather than from a picture.
- The multiplier precedent for §5's crest is `empowerment` (magic) and
  `spellblade` (physical) on `Combatant` — a third of the same shape is the
  cheap way to build a mind multiplier.

---

## 2. The five changes

### 2.1 A tab per place, once you have been

The map gains a tab for every dungeon, every orb destination and the yard —
**greyed and unclickable until the run has entered it**, the way THE HUNDRED's
tab is greyed until `county_trips` is non-empty.

Thirteen more tabs will not fit one row of `map_tabs()`'s 170px chips. The
tabs become **two rows, or a row with a left/right pager**, and that is a
layout decision the executing agent should make against the measured widths
rather than by guessing.

**Discovery is already recorded** and does not need a new flag:
`run.dungeons_cleared` / the dungeon's own visited state, and
`run.destinations_visited` for orbs. Find the existing predicate before adding
one — `CLAUDE.md` trap 19 is about counters nobody reads, and its mirror is
flags somebody writes twice.

### 2.2 The Threshold becomes a T

Three floors in a line becomes four in a T: the stem is what is there now, and
the crossbar is a floor that hangs off the middle one with the shop on it.

```
        today                    after
      0 — 1 — 2              0 — 1 — 2
                                 |
                                 3   <- the shop
```

Floor 1 gains a second exit, which makes it a **set of points** and therefore
needs a `fork` scene. Floor 3 is a buffer stop carrying
`also: &[Outcome::ShopAfter { shelves: THRESHOLD_SHELF }]`.

**Why the Threshold and not somewhere else:** insight is locked until THE
THRESHOLD is cleared, and insight is what Dread multiplies. The one dungeon
that opens the mind lane is the right place to sell the mind lane its gear.

### 2.3 The threshold shelf

Exclusive, sold nowhere else, mostly **helmets and crests** — which is where
the mind lane already lives (`bestiary.rs` says mind damage is the helmet's,
and `item.mind` is handled outside the weapon branch so a helmet can reach
you).

The shelf is a `&'static [&'static str]` beside `TOWN_ONLY`, and every name on
it must be added to a new `THRESHOLD_ONLY` list so `is_town_stock`'s siblings
keep the shop honest — a piece sold in one place and found in another is not
exclusive.

### 2.4 The crest that trades everything for the mind lane

The centrepiece, and the only genuinely new mechanic in this document.

> mind damage, and multiplicative power to mind damage equal to some factor of
> your dps — but you do 0 damage any more, just mind damage.

Read as a rule:

- Every point of physical and magic damage the board would deal is **not
  dealt**.
- The mind damage it does deal is multiplied by a factor derived from the
  damage it gave up.

That is a **conversion**, and conversions are the sharpest thing to get wrong
in this engine. Two properties the executing agent must hold:

1. **It cannot be a free multiplier.** If the factor is `dps / k` and the
   board's dps is unchanged by wearing the crest, the crest is strictly better
   than not wearing it for any mind board, and every board becomes a mind
   board. The physical and magic damage must actually stop.
2. **Sudden death owns everything past 30 s** (trap 5). Mind damage lowers
   maximum health and never heals, so a mind board that is slower than a
   damage board does not lose — it wins on a different clock. Measure the TTK
   curve, not the damage figure.

The shape to build it in is `Combatant`'s existing multipliers: `empowerment`
multiplies magic, `spellblade` multiplies physical, and this is the third.
A `mind_conversion` field plus a flag, set once at the bell like every other
passive, is the cheap version. **Do not** put it in `Stats` unless it is
classified in `parts_when` — see §4's trap.

### 2.5 The Switchyard becomes islands, and the orb destinations grow

**The yard**: nine floors in one graph becomes several small clusters with no
walking route between them. The Up Line and the Down Line stop being sidings
that drop you in and become **the way across** — the orbs are the only
connection between islands, which is what makes them tickets rather than
shortcuts.

This changes `Where::Siding { dungeon, floor }` from "a way in you have
already half-walked" to "a way in you cannot otherwise reach", and the
`fights_ahead` banner and the seven graph lints in `dungeon.rs` all read the
floor graph. **Expect every one of them to have an opinion.**

**The orb destinations**: each becomes a **2×2 zone with a self-contained
event chain** rather than one event or one dungeon. Four floors in a square —
which in `Floor` terms is a node with two exits leading to two nodes that both
lead to a fourth — and a chain of events that starts and finishes inside it.

---

## 3. Milestones

Each ends green on all three suites with no warnings. ▲ marks a deploy.
A1–A3 are the map and are independent of A4–A7, so they can be done in either
order; A4 must precede A5.

### A1 — The tab strip grows ▲

- `map_tabs()` stops returning `[Rect; 2]` and returns a `Vec<Rect>` from a
  count, in two rows or with a pager.
- A tab is greyed and inert until the place is known; THE HUNDRED's existing
  `!run.county_trips.is_empty()` is the pattern.
- **No new maps yet.** A dungeon's tab draws a placeholder that names the
  dungeon and says how many floors it has.
- **Gate:** a pure layout test in the house style — every tab inside the
  panel, none overlapping, at every count from 2 to 20, off a function that
  takes the count. Trap 32, and `mode_select_rects` and `pedestal_rects` are
  the two worked examples.
- **Deliverable:** the strip can hold what §3's later milestones will put in
  it, and nothing is drawn that a run has not earned.

### A2 — A dungeon draws its own graph ▲

- One renderer for every dungeon, taking the `Dungeon` and the run. Floors as
  nodes, `exits` as edges, cleared floors marked, the floor you are on marked
  differently.
- Laid out by a **pure function** from the graph — not by hand-placed
  coordinates per dungeon, which would be a second copy of the table.
- **Gate:** every dungeon in `DUNGEONS` lays out without overlapping nodes and
  inside the panel, including the nine-floor yard; every edge starts and ends
  on the node it names. `main.rs` already has
  `every_edge_on_the_road_map_starts_and_ends_on_the_node_it_names` for the
  road — copy its shape.

### A3 — The orb destinations get their tab ▲

- A destination that is an event draws the event and its chain; one that is a
  dungeon draws A2's graph; one that is a siding draws the island it lands on.
- Greyed until `destinations_visited` contains it.
- **Gate:** every `Destination` in `DESTINATIONS` draws something, which is
  the `every_destination_can_be_reached` shape aimed at the map.

### A4 — The Threshold grows its crossbar ▲

- Floor 1 gains a second exit and a `fork` scene; floor 3 is a buffer stop.
- **No shop yet** — the floor is a fight and a landing. Landing the shape
  before the content is what lets the graph lints be read on their own.
- **Gate:** the seven `dungeon.rs` graph lints, and a bounded walk that
  reaches floor 3 (trap 24: bound every walk).

### A5 — The threshold shelf, minus the crest ▲

- `THRESHOLD_SHELF` and `THRESHOLD_ONLY`, the helmets and crests, and
  `Outcome::ShopAfter` on floor 3's `also`.
- **Everything except the conversion crest**, so the shelf can be measured
  before the mechanic that will dominate it exists.
- **Gate:** nothing on the shelf is for sale anywhere else; the `catalog_shape`
  ratchet holds; `baseline` diffed and read.

### A6 — The conversion crest ▲

- The mechanic of §2.4, and the measurement that says what the factor is.
- **Land the primitive inert first** if it is at all possible — the house rule
  from `HANDOFF.md` §5, and it is why every claim in this repo can be
  attributed.
- **Gate:** a fight where a board wearing it deals **zero** physical and magic
  damage — asserted off the log, not off the stat block. A TTK curve against
  the reference boards at all four settings, written into `analysis/`. And
  `e6_5`, which is the criterion a mind board was always meant to answer:
  `reference_builds.rs`'s `THE_FOURTH` is "written at the mind lane" and has
  never beaten THE UNWOUND. **If this crest is the thing that makes THE_FOURTH
  win, say so in the commit — it closes a criterion that has been open since
  the Unwinding.**

### A7 — The yard becomes islands ▲

- The nine floors split into clusters with no walking route between them.
- The Up Line and the Down Line become the crossing.
- **Gate:** every floor is reachable by *some* route including orbs — the
  reachability shape, aimed at the yard; the seven graph lints; and
  `fights_ahead` still counts what a banner promises. `switchyard.rs` is
  thirty tests and it will have opinions about all of it.

### A8 — The orb destinations become zones ▲

- Four floors each, and an event chain that starts and finishes inside.
- **Gate:** `completable.rs` for each chain — a chain whose key arrives after
  its door shuts is this repo's oldest bug, and a self-contained chain makes
  it easy to write one by accident.

### A9 — The record ▲

- `analysis/` gets the TTK curves and the shelf's measurements;
  `design/the-atlas.md` gets a "what shipped" ledger; `CLAUDE.md` gets its
  counts and whatever traps this earned.
- Both printers diffed and **read**, not just run.

---

## 4. Traps specific to this work

1. **`Stats` is not passive** (trap 40). If the crest's conversion lands in
   `Stats`, it must be classified in `parts_when` or the card will file it
   wrongly and the `every_figure_a_stat_block_prints_says_when_it_happens`
   lint will fail. Prefer a `Combatant` field.
2. **A dungeon's floors are a graph and half a suite assumed a list** (trap
   22). Five lints were found wrong in one afternoon the last time this
   changed. Ask of any `d.floors` code whether it means "the list" or "the
   walk".
3. **Bound every walk** (trap 24). A test that walks until it runs out is a
   hang the day a floor refuses, and A7 makes floors that refuse.
4. **A page laid out inside `draw_*` cannot be tested** (trap 32). Three
   worked examples now: `fight_diagram_layout`, `mode_select_rects`,
   `pedestal_rects`.
5. **`CATALOG` is index-keyed by `share.rs` and append-only for ever** (trap
   2). The threshold shelf's pieces go on the end.
6. **A rating weight or a footprint sibling re-gears every creature** (trap
   3). The shelf appends footprint siblings. Settle the weights before
   measuring anything against them, and expect `gear_at.txt` to move.
7. **Only a weapon swings** (trap 36, and `second-order.md` §10). The crest is
   a helmet; its mind damage lands from any slot and its physical damage would
   not have. That is the whole reason the mind lane is the helmet's.
8. **The tab strip is the fourth thing to grow past its row.** The glossary
   shelf, the mode screen and the pedestal tray all did. The fix each time was
   a second column or a pure layout function, never a smaller font.

---

## 5. What could go wrong

1. **Thirteen tabs is a worse map than two.** If a run has entered one
   dungeon, it should see two tabs and not thirteen greyed ones. Greyed-until-
   known is doing more work than it looks like it is.
2. **The crest makes every board a mind board.** §2.4's first property is the
   guard, and the measurement is the only way to know. If the factor has to be
   small enough that the crest is never worth it, the mechanic is wrong rather
   than the number.
3. **Islands make the yard unfinishable without orbs.** A7's gate is
   reachability *including* orbs; if an island can only be reached with an orb
   the run cannot get, that is `completable.rs`'s family in a new room.
4. **Four-floor zones are four more creatures each.** Six destinations × three
   new floors is eighteen creature boards, and `pack_francis` is the tool the
   owner has recorded as too slow. Budget the authoring before starting A8, and
   consider whether a zone can reuse `ALTERNATES` that already exist.
5. **The Threshold's T changes what clearing it means.** Insight unlocks on
   clearing it; if the shop floor is a second buffer stop, decide whether
   reaching *either* stop unlocks insight or only the original one, and write
   the answer where the unlock is read rather than in this document.

---

## 6. Where things live

| What | Where |
|---|---|
| Dungeons, `Floor`, `Exit`, the seven graph lints | `crates/engine/src/dungeon.rs` |
| Orb destinations, `Where`, `by_orb` | `crates/engine/src/pedestal.rs` |
| `Outcome::ShopAfter`, the event tables | `crates/engine/src/event.rs` (`:244`) |
| `shop_owed`, drained after a fight | `crates/engine/src/run.rs` (`:4028`) |
| Town/threshold exclusivity lists | `crates/engine/src/piece.rs` (`TOWN_ONLY`) |
| The mind lane: `dread`, `insight`, `mind_pierce` | `crates/engine/src/combat.rs` |
| The multiplier precedent: `empowerment`, `spellblade` | `crates/engine/src/combat.rs` (`:3156`, `:3182`) |
| Map tabs, the county map, the road map | `crates/gui/src/main.rs` (`map_tabs` `:7067`) |
| The yard's thirty tests | `crates/engine/tests/switchyard.rs` |
| Door reachability | `crates/engine/tests/completable.rs` |

## 7. How to run things here

```
cargo test -p gearmaster-engine          # 1056 green, 58 binaries + lib
cargo test -p gearmaster-gui             # 84; cargo build does NOT compile them
cargo test -p gearmaster-cli             # 12, the replay contract
cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1
cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture
REBASELINE_GEAR_AT=1 cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture report_gear_at
```

Never start a second cargo while one is running. Deploy with
`make web && cp dist/web/* docs/ && touch docs/.nojekyll`, then commit and
push; `index.html` carries the wasm's hash so a deploy busts its own cache.

**And look at the screens.** Every map in this document is a thing a person
has to read, and the suite cannot tell you whether it reads.


---

## 8. What shipped

Written at the end of the run. Seven of nine landed; A8 did not, and the
reason is worth more than the milestone was.

| | Milestone | State |
|---|---|---|
| A1 | The tab strip grows | **in** — 15 tabs, wrapping, greyed until known |
| A2 | A dungeon draws its own graph | **in** — laid out from the exits |
| A3 | The orb destinations get their tab | **in** |
| A4 | The Threshold grows its crossbar | **in** — a T, with points at floor 1 |
| A5 | The threshold shelf | **in** — five helmets and crests, sold nowhere else |
| A6 | The conversion crest | **in** — THE WRONG SENSE |
| A7 | The yard becomes islands | **in** — the throat's fork is gone |
| A8 | The orb destinations become zones | **not done** — see below |
| A9 | The record | **in** |

### A8 is blocked by two lints that are right

The plan said to reuse existing `ALTERNATES` rather than author eighteen
creature boards, and the owner confirmed it. Reuse does not work, and the
reason is a rule rather than an accident.

`dungeons.rs::a_dungeon_reads_as_one_creature_all_the_way_down` holds two
things at once: **bands may not fall along any road out**, and **a dungeon's
creatures share one theme** (with two named exceptions). Between them they
leave almost nothing to borrow:

| Dungeon | Needs | Available in `ALTERNATES` |
|---|---|---|
| `den-rivals` | Beast, band 30-32 | `THE ROUNDHOUSE` (30), `THE WUMPUS` (32) |
| `wumpus-world` | band 30-32, theme exempt | `THE ROUNDHOUSE`, `THE COAL STAGE` (both 30) |

Every candidate is a **switchyard** creature. A bear den with a roundhouse in
it and a wumpus cave with a coal stage in it would pass both lints and be
exactly what the theme lint's own message calls *"two dungeons stapled
together"* - the lint would be satisfied by the wrong thing, which is trap 29.

So the shape was built, measured against the lints, and reverted. **A8 wants
four authored creatures - two per zone - and that is its real cost.** The
alternative is to widen `FRAMES` with cave-and-den creatures at bands 30 to
32, which is the same authoring under a different name.

### And two things that came out differently

**The threshold shelf could not be a `Plating`.** `Plating` floats between the
helmet and the greaves and a floating kind may not carry an identity mechanic,
which the mind lane is. The lane moved onto crests and the plating became
plain filler - which is the catalogue's own rule correcting the design, and it
made the shelf better: the identity is in the crests, where a player reads it.

**The trade could not be a trigger.** `OnBattleStart` is the greaves'
identity mechanic and a helmet may not borrow it, so THE WRONG SENSE is an
`EffectKind` read off the board at the bell. That is more correct than the
trigger would have been: it is a standing state, and a trigger firing on the
item's first activation would have let the opening blows land - a free
multiplier for the start of the fight and a trade for the rest of it.
