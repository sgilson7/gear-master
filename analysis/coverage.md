# Coverage — what a forward, player-legal play has reached

Written by `cargo run --release -p gearmaster-lab --bin cover`. 64 runs: 32 seeds in each of two modes, at Medium, 600000 presses each, with the coverage dial at maximum.

Nothing below is read out of a table. Every count is a place a run stood.

The deepest rung any run reached is **50**.

## The three columns

| | offered | answered | branched |
|---|---:|---:|---:|
| doors (53) | **38** (72%) | **38** (72%) | **27** (51%) |
| choices (120) | - | **74** (62%) | - |

## Every gap, classified

### Too few runs got there to say anything — **1** doors

Fewer than 8 of 64 runs ever stood on the rung these are pinned to, so their absence is a fact about the pilot's ceiling and not about them. Closing this class is a *player* problem and it belongs to A6.

  - `the-second-shadow` — rung 49, 5 runs stood there

### Offered and never answered — **0** doors

A run stood in front of these and took nothing. Either no choice was open, or the run ended there.


### Reached at a rung runs *did* get to, and still never offered — **11** doors

**This is the class worth reading.** Runs were on the rung and the door did not appear, which means its trigger asks for something no run had — a flag, a word, a fight fast enough. Each one is either content behind a condition nothing meets, or a condition that is harder than it reads.

  - `the-constable` — rung 40, 16 runs stood there · **wants a flag**
  - `the-crownwright` — rung 20, 19 runs stood there · **wants a rumour**
  - `the-green-ledger` — rung 23, 18 runs stood there · **wants a rumour**
  - `the-locked-gate` — rung 41, 16 runs stood there · **wants a rumour**
  - `the-glow-over-the-ridge` — rung 46, 11 runs stood there · **wants a rumour**
  - `the-sealed-bid` — rung 36, 16 runs stood there · **wants a flag**
  - `the-passenger` — rung 42, 16 runs stood there · **wants a flag**
  - `the-fork` — rung 37, 16 runs stood there · **wants a flag**
  - `the-foundry-remembers` — rung 47, 11 runs stood there · **wants a flag**
  - `the-signal-box` — rung 25, 18 runs stood there · **wants a rumour**
  - `the-turntable` — rung 28, 18 runs stood there · **wants a rumour**

### Delivered rather than walked to — **3** doors

`flag: "never"` is `event.rs`'s own sentinel for a door nothing on a rung can reach: something else pushes it through `forced_event`. Reporting these as gaps would be describing the design and calling it a fault - the mirror of the mistake the acquisition class exists to avoid.

  - `the-thrumbus-race` — nominally rung 41
  - `mole-town` — nominally rung 41
  - `the-unwound` — nominally rung 50

### Answered, but not every branch — **11** doors

  - `the-county-surveyed` — rung 38, 1 of 3 branches
  - `the-vip-area` — rung 30, 1 of 3 branches
  - `the-toads-offer` — rung 3, 1 of 2 branches
  - `the-shrine-fork` — rung 10, 2 of 3 branches
  - `the-dispenser` — rung 17, 2 of 3 branches
  - `the-buyer` — rung 32, 2 of 3 branches
  - `the-last-train` — rung 34, 1 of 3 branches
  - `the-pale` — in the county, 1 of 2 branches
  - `the-boundary-ditch` — in the county, 1 of 2 branches
  - `the-field-barn` — in the county, 1 of 2 branches
  - `the-milestone` — in the county, 1 of 2 branches

## Reachable only through a specific acquisition

THE ATLAS cut the switchyard into two islands. These floors cannot be walked to from any mouth — an Orb of Travel lands a run on a siding inside them — so a ledger that counted them as missed would be reporting a design decision as a bug.

| dungeon | floors | walked to | islands |
|---|---:|---:|---|
| `the-crevice` | 3 | 0 | - |
| `the-threshold` | 4 | 0 | - |
| `the-under-mine` | 2 | 0 | - |
| `the-undertow` | 2 | 0 | - |
| `den-rivals` | 2 | 0 | - |
| `wumpus-world` | 2 | 0 | - |
| `the-switchyard` | 9 | 0 | [5, 6, 7, 8] |

## Towns

| town | gate reached | doors gone through |
|---|---|---|
| SUMP BOTTOM | yes | chapel x10, county x9, factory x9, pub x10, shop x9 |
| KETTLEWORKS | yes | chapel x5, county x4, factory x5, pub x5, shop x5 |
| HIGH WICK | yes | chapel x5, county x4, factory x4, pedestal x4, pub x4, shop x4 |
| EXTRA LARGE | yes | aisle9 x2, county x1, manager x1, pedestal x1, returnsdesk x2, samplecounter x1 |
| THE MANSE | **no** | - |
| THE SLAGWORKS | **no** | - |

## Classes

14 of 31 drunk: Archmage, Avenged, Duelist, Geomancer, Longhauler, Piety, Recycler, Showstopper, Stormcaller, Tired, Trundle, Unionized, Warpriest, Wellspring

## THE HUNDRED

20 tiles stood on across 64 runs. 23 brawls walked into.

