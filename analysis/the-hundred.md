# THE HUNDRED - measurements

One block a milestone, each headed by the commit it was read off. The spec is
`design/the-hundred.md`; the decisions are `design/HANDOFF-hundred.md`. Where
the spec and the code disagree about an address or a number, **the code is the
news** and the difference is written down here.

---

## F0 baseline, at `150adf7`

Branch `the-hundred`, off `2f7f426` on `main`. Nothing of the mission has
landed; this block is what the mission is measured against.

### Counts

| | passed | ignored | failed |
|---|---:|---:|---:|
| engine (53 binaries + lib) | 919 | 49 | 0 |
| gui | 78 | 0 | 0 |
| cli | 5 | 0 | 0 |
| **workspace** | **1002** | **49** | **0** |

Counted after F0's own two additions - the road fixture test and its guarded
printer - and after the GUI fix below, which is why the engine reads 919/49
where `main` at `2f7f426` reads 918/48 and the GUI reads 78 for 78 rather than
78 for 77.

`cargo check --workspace --all-targets`: **no warnings** - after F0's own two
fixes, below. Engine source 39,030 lines; `CATALOG` 512 pieces; `EVENTS` 38;
rustc 1.95.0, MSRV declared 1.83.

### The four-board table, and the ladder

Byte-for-byte from `--test baseline -- --ignored`. This is the table every
milestone through F6 has to reproduce unchanged.

```
## Weapon share across the whole ladder

build          cleared    weapon %  median ttk    burn %
starter           2/50      100.0%      45.00s      0.0%
preset            9/50      100.0%       9.00s      0.0%
owner            48/50       75.5%       9.00s      0.0%
friend            48/50      97.4%       8.15s      0.0%

## Board cadence - friendly activations a second

build              items activations/s    per item
starter                1          0.50       0.499
preset                 8          2.06       0.257
owner                 19          6.58       0.346
friend                17          3.42       0.201

## Mind damage across the whole ladder (max health removed)

build          helmet    chest   gloves  greaves   weapon
starter             0        0        0        0        0
preset              0        0        0        0        0
owner              58        0        0       57        0
friend            698        0        0        0        0
```

Four reference rungs at Medium: starter win 4.50 / loss 9.00 / loss 5.90 /
loss 7.50; preset win 1.50 / win 19.50 / loss 38.90 / loss 44.00; owner win
1.50 / 2.60 / 14.00 / 22.50; friend win 2.60 / 4.75 / 10.30 / 15.45.

No-weapon viability unmoved: owner 42/50 best rung 48, friend 35/50 best 46,
and both rung-15 results are **the clock's** (47.0 s and 44.6 s past a sudden
death that starts at 30).

### The ratchets

`report_shape`: every exclusivity row **0 away** at budget 0, all 35 of them;
every quota **0 away**; identity mechanics on floating kinds **0**. Rarity per
slot: helmet 97/1/1/0, chest 71/0/0/1, gloves 84/0/0/0, greaves 67/0/0/1,
weapon 188/0/1/0.

### The fixtures F0 leaves behind

| Fixture | What it holds | Re-baselined by |
|---|---|---|
| `tests/fixtures/gear_at.txt` | every creature's gear at every difficulty - **6,216 placements** | `REBASELINE_GEAR_AT=1 --test catalog_shape -- --ignored report_gear_at` |
| `tests/fixtures/road-at-{5,20,40}.txt` | `route::ascii` for a bare run at three rungs, 96 lines each, **exact** | `REBASELINE_ROAD_AT=1 --test the_road -- --ignored report_road_at` |
| `analysis/replays/dungeons.txt` | the six straight-line dungeons walked from the top | (pre-existing, Switchyard M1) |

**6,216, not the 5,568 `CLAUDE.md` §5 quotes.** That number was read at the
Switchyard's tip; the assembly-bonuses mission grew `ALTERNATES` and the
fixture with it. The fixture is the measurement and 6,216 is what it says.

The three road fixtures are **exact** where `dungeons.rs`'s pre-existing
`route-ascii-m0.txt` is a *subsequence*. Different questions: that one asks
whether M1's graph moved any line the pre-graph road had, and must tolerate
the road getting longer; these ask whether the road changed at all. A8 has
`route::ascii` growing a county half at F9, and the test's own comment says
what F9 must do about it - assert a prefix, with the reason named in the
assertion - rather than re-baseline the fixture to include a county that could
not be read at F0.

### Two things found while taking the baseline, and fixed

**A GUI layout test had been dead, and the count hid it.** A `#[test]` was
separated from `no_two_buttons_share_a_pixel` by a test inserted between the
doc comment and the attribute, so the stray attribute landed on the *next*
function - which already had one. rustc said so twice ("duplicated attribute",
"function is never used") and the suite said nothing, because the duplicate
**registered the pedestal test twice**: 78 entries, 77 distinct tests, one of
them counted double and one dead. `cargo test -- --list` is what shows it;
`test result: ok. 78` does not. Reunited with its function; it passes armed.
The GUI's honest count went 77 to 78 while the printed number stayed 78.

**`assembly_bonuses.rs:199` dropped a `Result`.** The seat-hunt puts a partner
back when two pieces do not assemble, and `run.unequip(b)` returning `Err`
would have left it seated and the search wrong. Asserted rather than silenced.

Neither is this mission's; both are the kind of thing a baseline exists to
find. Nothing else in the workspace changed and no measurement moved.

### The spec's citations, resolved against the tip

All seventeen `file.rs:line` citations in `design/the-hundred.md` were read at
`2f7f426`. Every one lands on code that says what the spec says it says.
**Eight have moved** - the prose pass and the assembly bonuses are between the
spec's reading and this branch:

| Spec says | Actually | What is there |
|---|---|---|
| `run.rs:701` | **`run.rs:805`** | `rng: Rng` |
| `run.rs:796` | **`run.rs:88`** (`:857` uses it) | `ROGUE_LIVES`, and it is **4** |
| `run.rs:649-655` | **`run.rs:759`** | `forced_event` |
| `run.rs:887-892` | **`run.rs:993`** | `fn monster` |
| `run.rs:2884-2886` | **`run.rs:3369`** | `fn wildcard_seed` |
| `town.rs:329-465` | **`town.rs:327`** | `pub const TOWNS` |
| `town.rs:313-318` | **`town.rs:311-319`** | `Town::actions` and the paragraph on why doors belong to the town |
| `event.rs:14-175` | `event.rs:14` / **`:112`** | `Requirement` / `Outcome` |

Correct as written: `rng.rs:7-33`, `stats.rs:19-20`, `combat.rs:915`,
`run.rs:19-24`, `loadout.rs:44`, `pedestal.rs:101`, `gui/main.rs:6925`, and
`gui/main.rs:9499-9520` to within the fifteen lines `render_glossary` moved
(it is at `:9484`).

### One thing the spec is wrong about, and it is not an address

**A0's free-rung list counts events and not town gates**, which is exactly
`CLAUDE.md` §6 trap 27 - and the Switchyard was moved twice by it. The rule is
`switchyard.rs:536`: no scheduled event may have `at == t.after + 1` for any
town. Town gates therefore occupy rung indices **7, 14, 18, 25, 32, 34**.

A0's nineteen free rungs are free of *events*: 0, 1, 4, 5, 6, 7, 11, 13, 14,
17, 18, 25, 32, 34, 37, 39, 42, 43, 44. Take out the six gate rungs and the
boss at 14, and **thirteen** are actually free:

```
0  1  4  5  6  11  13  17  37  39  42  43  44
```

Two of the mission's four road placements do not fit:

| Spec | Index | Verdict |
|---|---:|---|
| THE THEODOLITE (B1) | 11 | **free** |
| THE COMMONS (B3) | 17 | **free** - though its note "Kettleworks' gate rung; gate pops first" is wrong. Kettleworks is `after: 17`, so its gate rung is **18**, and 17 has nothing on it at all |
| THE STOCKMAN (B2) | 25 | **taken** - The Manse is `after: 24` |
| THE CONSTABLE (C1) | 18 | **taken** - Kettleworks is `after: 17` |

Not decided here: F8 owns the two moves, and a `Whispered` window (THE
STOCKMAN's) has a deadline to move with it. Recorded now because the spec
argues *from* the collision - both notes say "gate pops first" as though
sharing were the design - and the argument does not survive the rule.

---

## F1 the county, generated

`county.rs`, 1 module, 1 test binary, **18 tests**. Wired to nothing: the run
does not know the place exists until F2.

**Inert, and inert was measured three ways.** The whole `baseline` printer is
byte-identical to F0 - the four-board table, the cadence, the mind figures,
rungs 1-14, no-weapon viability and the census. `gear_at` is unmoved on all
6,216 placements. The three `route::ascii` fixtures are unmoved.

**Engine 937 passed, 49 ignored, 0 failed.** GUI 78, CLI 5. No warnings.

### The retry histogram, which is F1's deliverable

Ten thousand seeds, spread by the golden-ratio multiplier so they are not
consecutive:

```
first try 10000   retried 0   fell back 0
```

The spec's bar is "over 1% retries means a check is too tight". It is 0%, and
the reason is that the generator satisfies the geometric checks **by
construction** rather than by rolling until it gets away with it. That was not
true of the first version, which retried **54%** of the time:

```
first try 4580  retried 5420  fell back 0
histogram [4580, 2470, 1362, 738, 372, 211, 121, 77, 31, 19, 11, 3, 3, 1, 0, 1, ...]
```

A smooth geometric decay, which is a check failing about half the time rather
than a check refusing a *shape* - and the diagnosis was one tally: **V6, and
nothing else, on 2,202 of 4,000 attempts.** V6 spaces the three pinnacles and
keeps them off a gate, and the generator was enforcing it on the Drove's
pinnacle only. The hill *is* the Ordnance's pinnacle and the Commissioner is
one of the three sealed tiles, and neither was being asked. Both are knowable
at the moment they are placed - the pale fixes the sealed corner before the
hill is picked - so both are filtered rather than refused.

**Nothing exercises the retry or the fallback.** Zero of ten thousand seeds
reach either. The retry bound is held by the histogram's own assertion and the
fallback by `the_fallback_passes_every_check`, and that is the whole of the
coverage those two paths have. Named here rather than left to be discovered.

### Nine decisions, and what each one cost

**F1-1. The hill is the Ordnance's pinnacle, stored as one.** A1.2's skeleton
is "nine objectives, three pinnacles, the gaol" and V6 spaces three pinnacles;
B1.1 says the hill is `Empty` and becomes `Pinnacle { Ordnance }` in the
derived view when the third sighting is taken. Both cannot hold. The store
carries `Pinnacle { Ordnance }` at the hill, which is what makes A1.2's
arithmetic exact and V6 meaningful; **presentation is inverted** and F8 draws
it as Empty until three sightings. B1.1's stated behaviour survives unchanged -
stepping on it early resolves it as the empty tile it looks like, and the
clear is dropped when it becomes a pinnacle - because A2.1 resolves against
the derived view.

**F1-2. The pale is an Event tile, and one of the twelve.** `TileKind` has no
`Pale` variant in A2 and B3.1 wants a checklist read from one tile away and a
single gated choice answered by standing on it, which is an event. Eleven are
arranged from the pool and the pale is the twelfth, so A1.2's composition is
exact rather than one over.

**F1-3. The sealed region is an L of three tiles, not a 2x2 block.** B3.1's
"the county's far corner". Every 2x2 corner block of a 7x7 contains exactly
one circuit tile - `the_sealed_corner_is_an_l_and_never_touches_the_ring`
asserts that, so the reason cannot quietly stop being true - and a Drover
walking into a region nobody can enter is a pursuit that ends by arithmetic.
The L is the corner and its two edge neighbours: three tiles, for the third
boundary stone, THE COMMISSIONER, and one more.

**F1-4. The pale is in the inner 3x3, and V5 forces it there.** Not on an edge
and not on the circuit leaves nine tiles on a 7x7. So the pale is a *shrine*
rather than a door: you stand on it with the checklist ticked and the far
corner opens somewhere else, which is what B3.1 describes anyway.

**F1-5. The far corner is the one furthest from the pale that holds no mouth.**
`CORNERS` is written so that the last entry wins a tie, because `max_by_key`
keeps the last maximum. Where the pale stands therefore decides which corner
opens.

**F1-6. Mouths are fixed, one per town, not generated.** A1.1's list of what
the generator places does not include them, and every check measures distance
*from* a mouth - a generated mouth would be tuning the ruler. Fixed also makes
"Sump Bottom comes in at A6" something a player can learn.

**F1-7. V1, V2 and V8 are evaluated with the pale open.** The Enclosure's
third stone and its pinnacle stand behind it by design; a reachability check
that refused them would refuse every county the spec describes. The pale is a
door and not a wall.

**F1-8. V2 is a matching, not a union.** "Reachable from three different
mouths between them" read weakly - the union of the three sets has three
mouths in it - is satisfied by three objectives huddled in one corner, which
is the shape the check exists to refuse. Read as a system of distinct
representatives it refuses that county. It is also the reading that means what
the sentence means: three objectives are three gates' work.

**F1-9. V9 grew a second half.** C1's argument is that being arrested is the
fastest ride into the middle there is, and "within three of D4" does not get
there on its own - a gaol at (2,1) is one tile from Kettleworks' mouth, which
`the_gaol_is_deeper_in_than_any_mouth` found on the first run. The gaol is now
also at least two from every mouth, and the assertion says out loud that the
shortcut is intended: a punishment a clever player farms beats one a careful
player avoids.

### One check cannot refuse anything, and the figure is in the assertion

**V7** - every tile within eight moves of some mouth, ignoring tolls. The
furthest any tile ever gets is **D4 at four moves**. It is kept rather than
deleted because it is the invariant the five-move budget is chosen against,
and `the_check_that_can_only_pass_is_v7_and_here_is_the_figure` asserts the
*measured figure* rather than the check, so the day the grid grows or the
mouth table shrinks it fails loudly.

**V2 was in that test until the measurement threw it out.** The claim was that
six mouths on the edge of a 7x7 cover the board several times over, so every
tile is inside a trip of at least three of them. The measurement said the
least-reachable tile is inside a trip of **one**: the southern edge of a
county is approached from one side, and three objectives stranded along it
cannot be given three gates between them. `CLAUDE.md` §6 trap 29 the other way
round - not "what is the cheapest way to satisfy this lint" but "is there any
way at all to fail it" - and the answer was not the one the argument gave.

Eleven of the twelve checks are handed a county broken in exactly their own
way and have to say so, and the assertion looks for that check's own prefix so
another check catching it does not count.

### What F1 leaves for F2

`generate` is ~0.9 ms in debug including `refusals`, which is fine for a test
and **not** fine on a frame. F2 derives the county on demand per A2.2 and
wants a memo, not a call per draw.

---

## F2 standing in it

A2.1's movement, A2.2's state, `TripSource`, `Interrupt::County`,
`Action::County` on all six towns, the CLI's `go`/`walk`/`out`, and a wipe
that takes it all. **Engine 953, GUI 78, CLI 8. No warnings.** The `baseline`
printer is byte-identical to F0, `gear_at` unmoved, the three road fixtures
unmoved.

### The census, which is the enum

`trip_cap()` is `TripSource::ALL.iter().map(seats).sum()` - `Town` is worth
`TOWNS.len()` and the other four are worth one each - so **ten** is counted off
the enum rather than written beside it. `the_census_is_the_enum_and_not_a_number`
asserts the arithmetic three ways: the variant count, `TOWNS.len() + 4`, and
the literal 10 with the reason in the message, because every figure in A4 (four
or five trips finishes two chains, seven finishes three) was costed against ten.

### One bug introduced and caught in the same hour

Sump Bottom and Kettleworks are the two pinned towns **without** a pedestal.
Replacing their `actions: &Action::ALL` with a shared constant that included
`Action::Pedestal` handed both of them a socket.
`pedestal::the_pedestal_costs_no_visit_and_is_the_only_thing_that_does_not`
refused it on the first run, and its own doc comment names the two towns that
have one - which is why it took a minute rather than a milestone. `PINNED_DOORS`
is the four doors and the way down, and its comment says which test caught it.

### Two pins moved, both re-pinned with the reason

`acceptance::e6_9` and `pedestal::the_pedestal_costs_no_visit_...` both held
"the pedestal is the only thing outside the one-action rule". There are two
now. Both assertions say why in the message: the way down is the pedestal's
exception rather than a new one - it is not a door, the county is *under* the
town rather than in it, and it is one trip per town for the whole run.
Charging a visit for it would make six towns six decisions the county always
loses.

### Five decisions

**F2-1. `run_seed` is a field.** `Loadout::name_seed` already held the run's
seed and is not it: a name generator's seed and a run's seed being the same
value is a coincidence, not a fact to build on.

**F2-2. The county is derived per call, not cached.** `Run::county()` is
`generate(county_seed())` every time: **77 us in release, 538 us in debug**.
A cache would be a second copy of a fact one line of arithmetic away, and
`Run::seeded` is called some thousands of times across the suite. A caller
drawing it every frame should hold its own; nothing in the engine does, and F9
is the milestone that will want one.
`the_run_derives_its_county_and_never_stores_one` pins the equality.

**F2-3. Three ways a move ends without moving you, and two of them cost the
move.** A fence you have not opened and (at F4) a toll you cannot pay both
cost it, because a move is a thing you spend *trying* rather than a thing you
spend arriving. The edge of the county is free, because walking into the edge
of a map is not an attempt at anything.

**F2-4. The sealed corner is shut from F2, not from F8.** `Run::pale_is_open`
reads a flag F8 authors the choice for. Landing the fence now means F8 opens
something rather than taking something away, and the method exists so F8
changes one place rather than a `flags.contains` at a call site.

**F2-5. Every tile clears on arrival, and the kind-specific arms are named
rather than stubbed.** `resolve_county_tile` is one arm and a comment: an
Event tile sets a pending county event at **F7**, an Objective pays its chain
and a Pinnacle asks whether its gate is met at **F8**. Written as one arm
rather than six identical ones, so the milestone that arms them finds a place
to put code rather than a shape to imitate.

### The exit criterion, and where it could not be met

F2 asks for "a CLI script that enters from all three pinned towns, walks
fifteen moves, twice, byte-identical". **The driver reaches one town.** The
wall is the Switchyard's M3 wall and it has not moved: `sandbox` grants every
component, `preset` is still the auto-builder, and the best board the driver
can build from its own verbs wins eight fights and then oscillates on the
Whisperling at rung 9. Kettleworks' gate is after rung 17.

Split, and both halves are in the suite:

- `cli/tests/replay.rs::a_county_trip_replays_identically` - seven fights to
  Sump Bottom's gate, down the steps, and **every way a move can end** in one
  transcript: an edge, fresh ground, a tile walked over twice, the last move
  and one asked after the trip is spent. Piped in twice, byte-compared. This
  is the half only a driver can prove.
- `county::three_towns_and_fifteen_moves` - the walk itself, three pinned
  towns and five moves each, asserting the census, that the gate survives each
  trip, that no tile cleared twice, and that three gates on three edges reach
  more than one region.

And `every_town_lets_you_down_at_its_own_mouth` does all **six** towns, which
is more than the criterion asked for and is the assertion that actually says
the door works everywhere.

### What F2 leaves for F3

`events_resolved` is a field on `Run`, initialised to zero and incremented
nowhere. `drover_tile()` reads it and is correct about a clock that has not
started. F3 is the three increment points.

`waste_bet` and `county_business` are **not** landed. A9's table puts them at
F2/F3 with the rest; they are C1's and C2's and nothing reads them until F8,
and a field with no reader is `CLAUDE.md` §6 traps 19 and 30 in a struct
instead of a table.

---

## F3 the clock

`events_resolved` moves, and nothing reads it yet. **Engine 957, GUI 78, CLI
8. No warnings.** `baseline` byte-identical to F0.

### A5's three places are one place

A5 says the clock increments in exactly three places: a road event answered, a
county event answered, and nothing else. In this engine **every event in the
game is answered in one function** - `take_choice_unchecked` - and that
includes a rung's door, a chain's, a dungeon mouth's, a forced one off a
pedestal, and, from F7, a county tile's. One increment point is the strongest
form of "nothing else" available, and `the_clock_counts_doors_and_nothing_else`
walks a fight won, a fight lost, a town door, five tiles of county and a shop
reroll past it to say so.

### And one place it has to come back down

**`Outcome::Defer` takes the door back off `answered`** - "declining is not
answering", which is a rule that predates this mission - and it has to take it
off the clock with it. A run that could advance the Drover by saying "not yet"
to one door over and over could walk it round the sixteen-tile ring for
nothing, which is an interception bought rather than intercepted. The
decrement is one line under the pop so the two cannot drift, and
`deferring_a_door_does_not_move_the_clock` is the test.

Both deferrable doors in the game are `Trigger::Whispered`, so neither stands
for a fresh run and the test hands over the word first - what is being
measured is the clock and not the road to the door.

### Why it is a counter and not `answered.len()`

They are equal for a county-free run and `the_clock_reads_the_same_as_the_run_at_four_checkpoints`
asserts exactly that at rungs 9, 19, 29 and 39, plus that the walk answered at
least eight doors, because four checkpoints that are all zero would pass the
equality and prove nothing.

They stop being equal at F7: a county event id can be **arranged onto more
than one tile** (D-2 puts eight events into twelve slots), and an id on
`answered` is an id that never asks again. So the clock has to be its own
number, which is what A2.2 asked for and this is the reason.

---

## F4 tolls, and the two Requirement variants

Six figures over `Loadout`, the tax, one-tile visibility,
`Requirement::CountyTiles` and `::CountyCleared`. **Engine 969, GUI 78, CLI 8.
No warnings.** `baseline` byte-identical to F0, `gear_at` **6,216 placements
unmoved**, the three road fixtures unmoved. `tests/tolls.rs` is 12 tests and a
printer.

### A3's formula is out by a factor of a thousand, and its worked example is right

A3 writes `flow = sum(stats.mana * 1000 / cooldown_ms)` and calls it
milli-mana a second. Eight mana on a 4,000 ms item gives **2** under that
formula, and A3's own worked pair says it should pay **2000**. The factor is
`1_000_000`, not `1000`: a stat is per *activation*, and turning it into a rate
is `stat x (1000 ms/s) / cooldown_ms`, which in thousandths is
`stat * 1_000_000 / cooldown_ms`.

The house already computes per-second figures this way -
`ItemProfile::dps_milli` is `hit * 1000 * 1000 / cooldown_ms` and has been
since the gear-slot rewrite. The worked pair is the spec, the table's
arithmetic was a slip, and `flow_is_not_mana` asserts both halves of the pair:
2000 and 3000, and that the board with **less mana** crosses the deeper river.

### The division is per item, and a test says what the other way would give

`the_division_is_done_per_item_and_then_summed`: one mana on a 1,000 ms item
and one on a 3,000 ms item pays `1000 + 333`, and it asserts *against*
`2 * 1_000_000 / 4000` - five hundred - because the two items do not take
turns. Rounding down happens where the division does, which is per item.

### The table F11 sets thresholds from

Read at this commit off `--test tolls -- --ignored report_what_a_board_pays`.

```
build          flow    phys/s   magic/s  armour/s   fastest   hedge
starter           0         0         0         0         -       0
preset        2.544         6         0    32.829    1500ms      10
owner         11.77    58.255     9.828    85.971    1500ms     131
friend        23.23     2.083    29.868    80.569    1900ms      63
```

**Five of the six figures do not move with the rung**, and that is a fact about
the reference boards rather than about the tolls: a share code is one board and
it does not grow. F4's deliverable asked for the owner's figures "at rungs 10,
20, 30 and 40" and those four rows are identical. The rung is read by the toll
gate alone, through the bounty - 34g, 134g, 224g, 328g at rungs 11, 21, 31, 41 -
so it is printed separately. **F11 calibrates against the progression the four
boards stand for**, not against a rung column that says the same thing four
times.

### Every threshold A3 ships is trivially met, and that is what F11 is for

```
~R2    crossed by: preset, owner, friend      ^D2.5  crossed by: preset, owner, friend
~R4    crossed by: owner, friend              ^D2    crossed by: preset, owner, friend
~F3p   crossed by: preset, owner              #H3    crossed by: preset, owner, friend
~F5m   crossed by: owner, friend              #H5    crossed by: preset, owner, friend
^S2    crossed by: preset, owner, friend      #G1x   crossed by: starter, preset, owner, friend
^S4    crossed by: preset, owner, friend      #G1x   crossed by: starter, preset, owner, friend
```

Eleven of the twelve are crossed by the auto-builder's board and the twelfth by
the starter. The owner pays **11.77** flow into a river asking 2 to 6, **58** physical
a second into a ford asking 3 to 6, **86** armour a second into a scarp asking 2
to 5, and holds **131** curse resistance against a hedge asking 3 to 8. Every
number in A3 is roughly an order of magnitude low, and the drift is worse than
low - the preset board's fastest item is 1,500 ms, so both drifts are free.

D-5 said this would happen ("arithmetic off a paper map") and F11 is where it
is fixed. The numbers are not moved here: F4's job is to build the ruler and a
threshold bent before the measurement is a threshold bent to a guess.

### Four decisions

**F4-1. `Figures` lives in `loadout.rs` and takes `&[ItemProfile]`.** A9 says
"six toll figures over `&Loadout`"; what the figures are actually over is
`combat_items`, which is assembled items only, and taking the slice makes the
one thing that matters testable without a recipe between the test and the
arithmetic.

**F4-2. `fastest_ms` is an `Option`.** A board with nothing assembled has no
fastest item, which is not the same as a slow one - a drift asks for a board
that acts *often*, and an empty grid does not act. `Drift { 9_999 }` refuses an
empty board and `an_empty_board_has_no_fastest_item` says so.

**F4-3. The hedge is the one figure that is a stat rather than a rate.**
Curse resistance is a percentage held rather than a thing paid out, and a hedge
is a fence you are proof against rather than one you outrun.

**F4-4. A crossed Feature is only free while it stays cleared.** The toll is
asked of a tile that is not yet cleared, so crossing is permanent and a run
that strips its board keeps its bridges. `a_crossed_toll_stays_crossed` empties
the loadout to zero and walks back over.

### One thing a starter board cannot do, and a test that had to be told

`a_crossed_toll_stays_crossed` was written against `Run::seeded` and failed:
**a starter board pays no toll on any county**. That is the tolls working, not
the test failing, and it now uses the owner's board with the reason in a
comment. Its sibling `a_failed_toll_costs_one_move_and_no_position` keeps the
starter board, because a board that fails everything is exactly what a test of
the tax wants.

### The two Requirement variants, inert

`CountyTiles { region, at_least }` is answered by `Run::county_cleared_in` and
works today; `CountyCleared(chain)` reads a flag no outcome sets, through
`Run::county_chain_done`, and is false for all three until F8 can beat a
pinnacle. Three flag names - `chain_done`, `THE_SHEET`, `PALE_OPEN` - are
declared in `county.rs` with one accessor each on `Run`, so that F8 changes one
place per flag rather than a `flags.contains` at a call site.

### F2's walking tests all moved, and the reason is in one helper

Five tests picked "any direction that is not sealed", which was true until this
milestone made a direction a question about the board. They share
`somewhere_to_go(&run)` now - not sealed, not the edge, and either not a toll
or one this board pays - and it prefers somewhere uncleared so a walk covers
ground rather than pacing between two tiles.

### The GUI would have stranded a player, and Deploy Point 1 is why it matters

Taking the way down set `county_at` and left `pending_town()` still `Some`, so
the town screen re-rendered with no verb on it - **the pedestal's old bug**,
which `render_town`'s own comment describes ("the town re-rendered unchanged
and the player clicked again. For ever."), except that this one has spent a
trip on the way in.

Deploy Point 1 says "walkable" and asks a person to find out whether five moves
feels wrong, so a minimal county screen landed with F4 rather than with F9:
`render_county` - where you are standing, a compass of four ways off it, and
the way out. It is **not** the map; A8's second tab on the M overlay is still
F9's. It sits above the town in the driver for the reason the points sit above
the dungeon: you are standing in it, and the gate is what you come back to.

The two pure halves are testable and are tested. `compass_cells` returns
rectangles rather than drawing them, and `the_compass_fits_and_nothing_sits_on_anything`
caught the way out hanging **eighteen pixels** past the bottom border on its
first run - which is the same number, found the same way, that `points_cells`
records in its own comment. `step_label` says *which* of the two noes a
direction is, because "behind the pale" and "your board is 4 short of this
river" are different refusals and a screen that says only "no" to both teaches
a player that the county is arbitrary.

`step_label` takes the county rather than deriving one: at 77 us a derive, four
directions a frame would have cost a third of a millisecond a frame to rebuild
the same forty-nine tiles.

---

## F5 Bearing, Overtake, Commons - inert

Three effects, three weights, three rows at budget 0, three glossary entries
and three more for the county itself. **Engine 981, GUI 80, CLI 8. No
warnings.** `baseline` byte-identical to F0, `gear_at` unmoved on 6,216
placements, `acceptance::e6_2` green.

### Overtake had to repeat the activation, not the blow, and a test found it

The first version put Overtake into `reps` alongside `Echo` and `Fork`, which
is the cheap place and the wrong one for **exactly the slot the effect is
for**: `reps` repeats the swing, only weapons swing, and Overtake is
gloves-only. A gloves item's whole output is its triggers. The effect did
nothing at all, and the test that found it was the negative one -
`an_ordinary_item_does_not_overtake` reported *zero* opening blows for the
control, because a glove has none.

`activate` returns whether to run again and the caller re-runs it, so
`check_down` sits between the two and an opening that kills does not get a
second one. `has_fired` is set at the top of the first run, so the repeat
cannot qualify for the effect that produced it - one repeat, not a loop.
`overtake_runs_the_triggers_again_and_not_a_blow` is the assertion that would
have caught the first version: an armour-banking glove that overtakes has
banked **14** at the bell where an ordinary one banks 7.

### Bearing is not a solitude, and the difference is why it exists

`SoleIf { Solitude::StackedWith(Greaves) }` asks about **overlap** with the
grids laid on top of one another. Bearing **counts**. Two greaves items that
never touch and never overlap are both alone under the first and neither is
alone under the second, and `bearing_is_not_a_solitude` is the case that
separates them - with a note saying that if the two ever mean the same thing,
one of them should be deleted.

### Commons is a relation, and a relation runs both ways

`join_the_commons` folds it in with `commons[i] || commons[j]`, not `&&` and
not `commons[i]` alone: an item that read its neighbours but was invisible to
theirs would be a different mechanic wearing this one's name. Two things go
wrong there and both have a test: a real neighbour counted twice for also
being a commons neighbour, and an item left in `diagonal_items` after Commons
turned its corner into an edge - `diagonal_items` is documented as "never also
adjacent" and this is what keeps the promise.

### Two of the three cannot be proved on a board until F6, and that is named

A board's effects come off its pieces, and no piece carries these until F6. So
Bearing and Commons are tested through the pure functions the recompute calls -
`bearing_doubles` and `join_the_commons`, split out for the reason `chip_rects`
takes a measure - and Overtake is tested in full, because combat reads it off
an `ItemProfile` field a test can hand it.

`RULES_AWAITING_THEIR_PIECES` holds all three, and
`no_rule_waits_for_a_piece_that_has_arrived` goes red the day any of them finds
a carrier. F6 cannot land the components without putting the rows back under
the lint. `the_three_new_effects_have_no_carrier_at_f5` is the milestone's own
exit criterion asserted from outside it.

### The weights, and the one name that was already taken

`BEARING 26.0`, `OVERTAKE 14.0`, `COMMONS 24.0` - starting points, settled at
F13. Rated against `SoleIf`'s 22 a multiple and `DoubleAdjacentItemStat`'s 20.

`naming.rs` already had a word "Bearing", for `PerOverlappingItem`, and has had
since the gear-slot rewrite. The greaves effect that shares its name does not
share its word: it names items **Sole**. A name is a word a player reads and
two mechanics answering to one is a name that says nothing.

---

## F6 the catalogue, once

Five components appended and never inserted - `share.rs` is index-keyed into
`CATALOG` and that format is append-only for ever. **Engine 982, GUI 80, CLI 8.
No warnings.**

| Piece | Slot | Chain | Carries |
|---|---|---|---|
| Trig Pillar | Greaves | Ordnance | `Bearing`, +10% speed |
| Drove Way | Gloves | Drove | `Overtake`, `OnAdjacentActivate` |
| The Common Ground | Chest | Enclosure | `Commons`, 26 health |
| Surveyor's Orb | Weapon | Ordnance | `SpendMana` → `GainForking` |
| Drover's Orb | Weapon | Drove | `OnOtherCast` → `Shunt` |

**One per chain, in the slot that chain taxes.** The Ordnance charges the
greaves through drifts and pays out in the greaves, so the board that got
through is the board the reward is for.

**`gear_at` is unmoved across all 6,216 placements**, which is the measured
form of "event-only re-gears nobody", and the whole `baseline` printer is
unchanged except the census: **512 pieces to 517**, plus the rows those five
are counted in. The four-board table and the cadence table diff clean.

### Two things refused it on the first run, and both were right

**`the_turtle_theme_covers_the_catalogue`**, immediately: five pieces in plain
words. The gear skill's rule is that `piece.rs` and the theme's table change in
the same commit, and the lint is what makes it true rather than remembered.
They are named for places the book has - `("Trig Pillar", "The Petonkle Trig
Stone")`, `("Drove Way", "The Kolok Drove Road")` - with the chest one taking a
substance, because a chestpiece is read off the defence ladder and sneel is the
rung twenty-six health sits on.

**`catalog_shape`**, on the Surveyor's Orb. Its first draft carried `Derail`,
which is Gloves-**majority** at 70% and whose entire weapon minority share is
already the Signalman's Orb. A second weapon carrier does not put one piece out
of place, it moves the *balance* - which is the difference between an exclusive
rule and a majority one, and the reason the majorities are written as
majorities. The orb carries `SpendMana → GainForking` instead: weapon-exclusive,
and a theodolite drawing one sighting to two places is what forking is.

### Three pins moved, all three re-pinned with the reason

- `RULES_AWAITING_THEIR_PIECES` emptied. `no_rule_waits_for_a_piece_that_has_arrived`
  was red until it was, which is the mechanism doing exactly its job: F6 could
  not land the components without putting the three rows back under the lint
  they were exempted from.
- `effects::the_three_new_effects_have_no_carrier_at_f5` **turned over** rather
  than deleted, into `each_of_the_three_has_exactly_one_carrier`. The list is
  the same list; what changed is which side of it is right.
- `enchantment`'s count 10 to **13**, and `prices::only_the_yards_own_six_speak_the_verbs`
  6 to **7** - the Drover's Orb speaks `Shunt`, which is the weapon's legal
  minority share of a greaves verb. The count is a ratchet on the sentence
  under it and the sentence has not moved: everything that speaks one of the
  four is event-only, so none of them can reach a creature.

### One law, applied twice

The Switchyard's E-4 settled "ground is bought in a town, **or dug up**, and
never for sale on the road" by filtering `town_shelf()` on `is_event_only`.
THE HUNDRED's three needed no code at all: they are enchantments, so
`is_town_stock` is true and `shop.rs`'s three road filters refuse them; they
are event-only, so the cart refuses them too. `the_countys_ground_is_dug_up_and_never_sold`
is the assertion, and it is deliberately a **second** test rather than three
more names on `THE_YARDS_GROUND` - a list that grows every mission is a list
that stops naming anything.

---

## F7 county events and the word crossing

Eight authored county events, dealt as a deck into eleven slots; one road door;
one word that makes the round trip; and four lints that learned about county
tiles. **Engine 987, GUI 80, CLI 8. No warnings.** The four-board table is
unchanged and `gear_at` is unmoved on 6,216 placements.

### The word goes up and comes back down as an answer

B6 asks for words crossing both ways. **One word does both.** A charcoal
burner called Sowerby, who has been in the county longer than the roads have,
hands over *A Word About the Hundred*; carried up, it opens THE COUNTY
SURVEYED at rung 37; and what Tasker tells you at her table is what the parish
chest's third lock was for, which is a county tile that is inert without it.

Two words rather than one was the first draft, and `SHELVES` refused it before
it was written: the bar is **exactly six names and full**, so neither direction
could have gone on it. One word that goes up and comes back down as an answer
is a better shape than two words passing each other.

### Eight into eleven, dealt as a deck and not a die

D-2. The generator shuffles the pool and deals it, then shuffles and deals
again for the remainder, so **every event is on the county once before any is
on it twice**. A per-tile draw satisfies "eleven tiles carry an event" and also
permits one event four times and three not at all, which is eight events
written and five read. `the_pool_is_dealt_as_a_deck_and_not_a_die` pins it.

That property is also load-bearing for a lint:
`completable::every_counter_can_reach_the_number_it_is_asked_for` counts each
county event **once**, because only the first deal is certain. Counting the
repeat would be counting on a shuffle.

### Five lints learned about county tiles, and four of them refused first

| Lint | What it said |
|---|---|
| `theme::the_turtle_theme_covers_the_catalogue` | the word was in plain words |
| `rumour::every_rumour_can_be_come_by` | "on nobody's bar and in nobody's gift" - **B6's third arm, landed in the milestone that needed it** |
| `rumour::every_rumour_opens_a_real_event` | the door did not exist yet |
| `completable::every_door_can_be_reached_before_its_window_shuts` | "nothing hands over A Word About the Hundred" |
| `validity::every_door_that_waits_on_a_key_can_be_handed_one_in_time` | the same, from the other end |

All four arms answer the same question - a county tile is a fourth way to come
by a word - and all four date it at **the first town's own gate** rather than
one rung later, because the way down is not a door and does not cost the visit.

### Three counters nobody read, and the answer was not a budget

`ditches-dug`, `stones-read` and `lanes-mended` went in as three `Count`
outcomes and `no_more_counters_go_unread_than_already_do` refused all three
against a budget of 3 - `CLAUDE.md` §6 trap 19 catching exactly what it was
written for.

The fix is **one** counter, `county-work`, and a reader: Tasker's third choice
wants two of it and pays three bounties and the third key. Three tiles down
there count the same thing - a ditch dug out, a milestone kept clear, a plank
mended - and the surveyor is the only person on the road who would know to ask.
The budget did not move.

### An event tile asks; answering is what clears it

A2.1. A run that walked onto a question and walked away has not answered it, so
`resolve_county_tile` sets `Run::county_event` and `take_choice_unchecked`
clears the tile. Two consequences worth knowing:

- **A tile with no open choice clears like any other.** A gated county event
  facing a run that has not got the key would otherwise stand there for ever,
  spending a move and giving nothing back. The parish chest always has one open
  choice, and `a_tile_with_nothing_to_ask_clears_like_any_other` pins both
  halves - that it asks, and that its gated choice is *shut* to a run that has
  not been up the road.
- **`county_event` is not filtered on `answered`**, unlike `forced_event`. An id
  on `answered` is an id the road never asks again, which is the opposite of
  what a repeat needs - and this is the reason the clock is its own counter
  rather than `answered.len()`, written down at F3 and now true.

### Two pins moved and one fixture was re-baselined

`acceptance::e6`'s census: `EVENTS` 38 to **39** and `RUMOURS` 10 to **11**,
with `COUNTY_EVENTS.len() == 8` added as its own row - a tile is not a rung and
adding them together would say the road had grown by nine.
`primitives`' quest-item count 14 to **15**.

And the three `route::ascii` fixtures, for the first time. **One line added to
each, and it is the same line**: `. -- THE COUNTY SURVEYED (event, between 12
and 13)`, which is where a door nobody has earned is drawn. Nothing else in
ninety-six lines moved on any of the three, and the reason is written into
`ROAD_AT`'s own doc comment rather than into a commit message.

### The scenes are held to the road's standard

`prose.rs` walks `EVENTS.iter().chain(COUNTY_EVENTS.iter())`: they are the same
struct and a player reads them on the same screen, so a county scene that could
not pass the road's lints would be a second standard nobody agreed to. It
refused THE COUNTY SURVEYED immediately - a woman with a map and no name - and
she is Tasker now, named **mid-sentence**, because `names_something` cannot
tell a name at a sentence start from an article (`CLAUDE.md`'s blind-spot note).

---

## F8 the chains, as frames

Three chains, three on-ramps, five creatures, the pale, the constable, the
waste and the perambulation. **Engine 1011, GUI 80, CLI 8. No warnings.**
`tests/hundred.rs` is 24 tests. The four-board table is unchanged and `gear_at`
is unmoved on 6,216 placements - which is the exit criterion, because F8 lands
the chains and **F12 dresses the creatures**.

### Where the three on-ramps actually stand

A0 drew them at 11, 17 and 25. **25 is The Manse's gate rung** (`after: 24`),
which the suite has refused since the Switchyard. THE STOCKMAN is at **13**
instead - free of events, gates and bosses - and it is a `Trigger::Rung` rather
than the one-rung `Whispered` window Part B drew, because the word that window
waited on would have had to come off a bar that is exactly six names and full.

THE CONSTABLE was drawn at 18, which is Kettleworks' gate rung. He is
`WhenFlagged` now, on a flag the **engine** raises, so he finds you when he
finds you rather than on a rung a gate might already have.

### `ALTERNATES` is append-only, and nothing said so

Five creatures inserted at the top of the table moved **2,592 placements** in
`gear_at.txt` without one creature changing what it wears: the fixture keys
every line on `ALTERNATES[i]`, so an insertion renumbers every creature after
it. It reads exactly like a re-gearing and is not one.

`CATALOG` has been append-only since `share.rs` was written and everybody knows
it. `ALTERNATES` has the same property for a different reason and this is the
first mission to find out. The comment is in `combat.rs` above the five.

### B1.1's presentation, inverted, and what that costs

The spec stores the hill as `Empty` and rewrites it to a `Pinnacle` when the
third sighting is taken. The store carries `Pinnacle { Ordnance }` and
`County::as_seen` hides it - **F1-1's decision, and this is the milestone that
had to make it work.** What it buys: A1.2's skeleton count and V6's spacing are
true as written. What it costs: `Run::county()` is the *seen* view and
`county_written()` is the table's, and the two things that need the truth - the
sighting lines, and the check that the hill is where the arithmetic says - ask
for the second.

B1.1's stated behaviour survives exactly. `a_cleared_tile_unclears_when_it_becomes_the_hill`
is the edge the spec names: a run that walked over the hill while it still
looked empty cleared an empty tile, and `resolve_county_tile` drops that
clearing when the tile becomes a pinnacle. The check is made **before** the
cleared check, because the hill is the one tile in the game that can be cleared
and then stop being cleared.

### The pale asks for an Orb-kind piece, not an Orb of Travel

`is_orb_of_travel` is the four pedestal keys, and the first draft of
`Requirement::HoldingOrb` used it - which would have refused the county's own
two, since the Drover's Orb is deliberately held rather than spent. B3.1's
wording is "any Orb-kind piece" and that is what it asks now: `PieceKind::Orb`,
twenty-three pieces over eight footprints (`CLAUDE.md` §6 trap 26).

It is also the right price. An orb is a weapon core somebody built around, so
surrendering one costs a **board** rather than a ticket - and `SurrenderOrb`
takes it out of whatever it was built into, which is what makes that true.

### One requirement for five lines, and they cannot drift

`Requirement::ThePaleIsReady` is the gate; `Run::pale_checklist` is the same
five questions asked separately so each can be ticked. Both go through
`Run::requirement_met`, which is `choice_open`'s body split out - a checklist
that computed its own answers would be a second implementation of every
requirement in the game, kept in step by hand.
`the_gate_cannot_disagree_with_the_list_above_it` is the assertion.

### Vessey waits his turn, and three Switchyard tests found out why

C2 was pushed through `forced_event`, which goes to the **front** of the road
stack - and `road_stack::the_door_underneath_cannot_be_answered_over_the_top_of_the_one_in_front`
is deliberate, so a door pushed in front of another is not a queue, it is a
door the player cannot answer until the first is answered. Vessey arriving
mid-chain broke `the_chain_can_be_walked_in_one_run_in_either_mode` in both
modes and the full-walk transcript with it.

`waste_offered` is its own field and `standing_events` appends it **last**. The
distinction is real and worth the field: a forced event is a place you have
just been sent, and this is a man at the roadside with an opinion about your
greaves. He waits.

**One thing was tried and reverted.** Making `take_choice` answer whichever
standing door owns the choice rather than the first one - which would also have
fixed it - was refused by that same road-stack test on the first run. The rule
is deliberate and the fix belonged on the other side.

### B5's failed toll, which the first version missed

"Any illegal move, **or a failed toll**, breaks the walk." The first version
broke the walk only on a move off the boundary, and
`the_first_move_chooses_the_way_round_and_the_rest_obey_it` found it: a toll
refused the move, the walk was not broken, and the run was left standing on the
boundary with moves in hand. A perambulation is a route rather than a
destination, and a route you cannot finish is not one you retry from where you
stopped. A fence does the same thing.

### The census is closed, and every way down is walked

`every_way_down_exists_and_the_tenth_is_the_perambulation` takes all ten in one
run: six towns, the orb, the arrest, the bet, and the perambulation last - and
then asks for an eleventh and is refused.

### Nine pins moved, all re-pinned with the reason

`bestiary::UNDRESSED` 0 to **5** - the one budget in the repository allowed to
go up, for the third time. `acceptance`'s census: `EVENTS` 39 to **44**,
`FRAMES` 24 to **29**, `DESTINATIONS` 6 to **7**, `COUNTY_EVENTS` 8 to **9**
(the pale is written in the same table and is not dealt from the pool).
`acceptance::e6_8` and `phase_two`'s frame list carry the five names.
`pedestal`'s orb count 6 to 7, and its "a siding ticket is never for sale" rule
became "an **earned** ticket is never for sale" - which is what it was standing
in for, and puts THE HUNDRED's on the right side.

`switchyard::every_floor_of_the_yard_is_dressed` was asking about **every frame
in the game** rather than about the yard's floors. It should have said so from
the start: a creature standing beside another mission's road is not a floor of
this dungeon, and a test named for the yard that failed on one was measuring
the wrong thing.

And the three road fixtures, a second time: **five lines added to each**, every
one a door this milestone wrote, with nothing that was there before moved. The
reason is in `ROAD_AT`'s doc comment.

### Three lints learned that the engine sets flags

`county-business` is raised by `Run::close_the_trip` when a trip clears
nothing. No walk of `EVENTS` can see that, so `completable`, `phase_two` and
`validity` each gained one named exception rather than a loosened assertion -
`completable::ENGINE_SETS` is the list, it has one entry, and the entry names
the function that raises it.

---

## F9 the map's second tab

`route::ascii_county`, `route::ascii` = the road and then the county, a
fixture for a known walk, and the M overlay's second tab. **Engine 1016, GUI
81, CLI 9. No warnings.**

### The road fixtures were not re-baselined, which is what F0 planned

`route::ascii` grew a county half and the three road fixtures did **not**
change. They hold `route::ascii_road` - what `ascii` was until this milestone -
and the test asserts them as a **prefix** of the whole map, which is exactly
what F0's own comment said to do when the day came:

> A8 has `route::ascii` growing a county half at F9. When it does, this fixture
> is the road half and this test must say so in its own assertion -
> `&got[..want.len()]`, with the reason named - rather than be re-baselined to
> include a county nobody could read at F0.

Five milestones later, that is what happened.

### What the county map says, and what it does not

```text
    A     B     C     D     E     F     G
 1   #G?        o     .~R?  o?    o      ^D?
 2  o           O     oB1   #T1   o
 3   ^S?  o?          .     o           o?
 4  o^D?  #S1   o~F?  .~F?               #H?
 5   ~R?  o           .?                 #H?
 6  o                 ogaol
 7         ^S?  o#G?  #T2   o?
  gates: A6 SUMP BOTTOM · C1 KETTLEWORKS · G3 HIGH WICK · A2 a town you have not found · ...
  the drover: F4 (clock 6)
  C3 - the pale: [ ] six in the north  [ ] six in the middle  ...
  3 of 49 cleared · 1 of 10 trips spent
```

`#` cleared, `O` where you are, `o` seen, `.` a line a sighting drew, blank
never been near. **A toll shows its glyph always and its figure only when
known** - `~R?` is a river whose depth you cannot read from here and `~R2` is
one you can. That is the whole of why the Ordnance's sheet is a reward rather
than a setting: a county you can read from the road is a county you plan on
paper.

The Drover is drawn only between the first sign read and the last fight, the
gates name the towns that have been found and say "a town you have not found"
for the rest, and the pale's checklist appears at one tile.

### The GUI's half, tested where it can be

`map_tabs` and `county_cells` return rectangles rather than drawing them
(`CLAUDE.md` §6 trap 32), so `the_second_tab_and_the_grid_it_opens_both_fit`
checks the whole geometry without a font context: both tabs on screen and not
overlapping, forty-nine cells on screen and not overlapping, every cell **under**
the tab strip rather than through it, and at least 130 pixels left under the
grid because the pale's five lines need them.

The two drawings read the same rules off the same run - `county_threshold_known`,
`sightings`, `signs_read`, `pale_checklist` - which is what "the CLI and the GUI
draw the same county" has to mean when one is characters and the other is
rectangles.
