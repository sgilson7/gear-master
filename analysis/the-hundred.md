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
