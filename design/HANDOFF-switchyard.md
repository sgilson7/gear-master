# Handoff — The Switchyard

Written as the mission runs, not after it. Read `CLAUDE.md` first, then
`design/post-unwinding.md` (the most recent audit of what the code does), then
the spec, `design/the-switchyard.md`. The measurements live in
`analysis/switchyard.md`, one block per milestone headed by a commit hash;
this file is the decisions and the surprises.

**Branch:** `switchyard`, off `e38d968`, to be merged once at the end.

## 1. The decisions taken from Part E

The kickoff says to take Part E's recommendations unless told otherwise.
Every one below is the recommendation, taken as written. Recorded here at M0
so that a later milestone that finds a reason to reopen one has something to
argue against.

| | Question | Taken | Why the recommendation, restated |
|---|---|---|---|
| **E-1** | THE LAST TRAIN's third door is unreachable, because the event waits on `switchyard-cleared` and the door is for the run that sold the sheet | **(c)** - THE LAST TRAIN becomes a `Trigger::Rung` event at index 32 that always stands, with all three doors gated and greyed for a run that never went down | A chain nobody can tell they missed is a chain they will not look for next run. The VIP area is the precedent (`event.rs:711`, "the rope does not move"). No engine work; one more unconditional door on a bare rung. Part B is written as (a)-with-a-bug so the bug stays on the page |
| **E-2** | The chain's first word: a seventh pub shelf, or a door | **the door** | The bar's six is a pinned number the GUI draws against (`rumour.rs:241`, `shop.rs:7`); a seventh shelf is an interface change for one word; the other chain's second and third words already come from doors |
| **E-3** | Per-floor siding entries in the theme | **leave them canonical at M7** | `Retold` carries one `entry` per dungeon. Both siding lines name no proper noun, so `two_voices` has nothing to catch and a missing theme entry falls through by design. Add `Retold.sidings` only if the book supplies a line worth the code |
| **E-4** | The enchantment law's wording | **A3 as written** - route around `town_shelf()`, leave `is_town_stock` alone | The law becomes "ground is bought in a town, or dug up; never for sale on the road". The alternative arrives at rung 34 behind the other chain and makes the reward purchasable twice |
| **E-5** | Is leaving allowed in the six shipped dungeons | **allow it everywhere** | A rule that applies to one dungeon is a rule with a list in it. `leaving_costs_no_life_and_keeps_what_was_cleared` is asserted on a shipped dungeon as well as on the yard |
| **E-6** | The turtle names marked *proposed* in Part C | **asked, not assumed** - see §3 | Nothing before M7 depends on them, so the question is open until M7. The rows to replace first are the chain name, the dungeon name, the two orbs and `skoogle` / `fnorp-interest` |
| **E-7** | Derail is not a curse and `curse_resist` does not answer it | **ship it unanswerable**, measure at M10 | It is the answer to a creature whose whole board is curse resist, which is the point. If a Signal-Wire board trivialises a leaf, the dial is `back_ms` and never a new resist |
| **E-8** | Nobody has played it | acknowledged, not a decision | Every claim in this file is from the suite and from transcripts that diff clean |

## 2. Where the code is

| Milestone | State | Block in `analysis/switchyard.md` |
|---|---|---|
| **M0** Baseline, and the MSRV told the truth | **done** | "M0 baseline at `e38d968`" |

## 3. Open questions for the user

- **E-6, the turtle names.** Part C was written without the book PDF or the
  titles CSV, and six rows are marked *proposed*: the chain name, the dungeon
  name, Hesketh's themed role, the two orbs' names, and the effect words
  `skoogle` and `fnorp-interest`. Nothing before M7 depends on any of them.
  If the book is available before then, those six are the rows to replace.

## 4. What each milestone did

### M0 - Baseline, and the MSRV told the truth

`Cargo.toml:7` now says `rust-version = "1.83"`. Nothing in the source
changed to earn it: the floor was already 1.83 and only the promise was
wrong (`Option::is_none_or` is 1.82, `const` items referring to `static`
items is 1.83). `CLAUDE.md` §6 trap 1 is retired.

Two tests were added, both in `catalog_shape.rs`:

- **`no_creature_changed_what_it_wears`** compares every creature's `gear_at`
  at every difficulty against `tests/fixtures/gear_at.txt` - 5,568 placements.
  This is the measurement behind the spec's A6 claim that six appended
  event-only components re-gear nobody, and it is the exit criterion of M1,
  M4 and M5. `stepped_component` sorts a footprint family by `monster_value`
  and steps along it, so a single appended sibling can re-dress every
  creature in that family without a line of any monster's own table changing
  (`the-unwinding.md` #19). An argument is not a measurement; this is.
- **`report_gear_at`** re-baselines it, and only under `REBASELINE_GEAR_AT=1`.
  The env var is not fussiness: `--ignored` on that binary is the ratchet's
  own printer command in `CLAUDE.md` §5, so without the guard, measuring the
  catalogue would silently overwrite the evidence that the catalogue had not
  moved. **This is the shape of `CLAUDE.md` §6 trap 22** - ask what the
  cheapest way to satisfy a new test is before shipping it - applied to a
  fixture rather than a lint.

The four printers and `report_shape` were run and their output is in the M0
block. Everything is where `post-unwinding.md` §5 left it: owner 48/50 at
75.5% and 9.00 s median, friend 48/50 at 97.4% and 8.15 s, preset 9/50,
starter 2/50, every ratchet row 0 away.

All 166 of the spec's `file.rs:line` citations were extracted and resolved
against the tip. Every one lands on code that says what the spec says it
says; three are off by a line or two and are listed in the M0 block. The
three findings A0 offers as contradicting a shipped design document - no
flee, floors do not drop their `drops`, `town_shelf()` has no event-only
filter - were all re-derived and all hold.

**Suite:** 802 passed, 41 ignored, 0 failed (801/40 before M0's own two).
GUI 62 passed. No warnings, `cargo check --workspace --all-targets`.
