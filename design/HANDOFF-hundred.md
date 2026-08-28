# Handoff — The Hundred

Written as the mission runs, not after it. Read `CLAUDE.md` first, then
`design/HANDOFF-switchyard.md` (the last mission's ledger, and the primitive
this one generalises), then the spec, `design/the-hundred.md`. The
measurements live in `analysis/the-hundred.md`, one block a milestone headed
by the commit it was read off; this file is the decisions and the surprises.

**Branch:** `the-hundred`, off `2f7f426`.
**Not merged and not published.**

## 1. The decisions taken from Part D

The kickoff says to take Part D's recommendations unless told otherwise. Every
one below is the recommendation, taken as written, recorded at F0 so a later
milestone that wants to reopen one has something to argue against.

| | Question | Taken | Why the recommendation, restated |
|---|---|---|---|
| **D-1** | Two-grid bonding | **confirmed out** | The Enclosure pays a single-grid chest enchantment. A cross-grid bond changes the bond layer, which is a mission and not a reward |
| **D-2** | Eight county events into twelve slots (four repeats) | **yes** | Eight written properly beats twelve written thin, and the arrangement already has to choose which tile gets what |
| **D-3** | The `FALLBACK` county authored first, in F1 | **yes**, from A6's sketch minus its deliberate V11 violation | It is the fixture everything else tests against, and a generator with no known-good output is a generator whose checks are unfalsifiable |
| **D-4** | Should the Drover's `+1 strength per 8 events` exist | **ship it behind its own constant** | So F14's replay can zero it in one line if the pursuit is already hard enough. The constant is the deliverable; the number is a guess until F14 measures it |
| **D-5** | Nothing has been compiled or played | acknowledged, not a decision | Bands, thresholds, the five-move budget and 8-11 moves a chain are arithmetic off a paper map. F14 measures them and the grid is the easier dial |

## 2. Where the code is

| Milestone | State |
|---|---|
| **F0** Baseline | **done** - `analysis/the-hundred.md` "F0 baseline" |
| **F1** The county, generated | not started |
| **F2** Standing in it | not started |
| **F3** The clock | not started |
| **F4** Tolls, and the two Requirement variants | not started |
| **F5** Bearing, Overtake, Commons - inert | not started |
| **F6** The catalogue, once | not started |
| **F7** County events and the word crossing | not started |
| **F8** The chains, as frames | not started |
| **F9** The map's second tab | not started |
| **F10** Theme | not started |
| **F11** Thresholds | not started |
| **F12** Boards, by hand | **held** - packed with the owner in the loop |
| **F13** Rating pins | not started |
| **F14** Acceptance, by replay | not started |
| **F15** The record | not started |

## 3. Open questions for the user

- **The two road placements that do not fit** (`analysis/the-hundred.md` F0,
  last section). THE STOCKMAN at index 25 and THE CONSTABLE at index 18 each
  land on a town gate, which the suite refuses. F8 owns the moves; the
  candidates are the thirteen genuinely free rungs. Not blocking anything
  before F8.

## 4. What each milestone did

### F0 - Baseline

Four printers, `report_shape`, and three new fixtures: `route::ascii` for a
bare run at rungs 5, 20 and 40, held **exactly**. The `gear_at` fixture
already existed (Switchyard M0) and holds **6,216** placements, not the 5,568
`CLAUDE.md` quotes - that number was read at the Switchyard's tip and the
assembly bonuses grew it.

**1002 passed, 49 ignored, 0 failed** across the workspace; no warnings.

Two pre-existing faults fixed, both about measurement rather than about the
game: a GUI layout test that had lost its `#[test]` to the function below it -
where the duplicate registered *that* test twice, so the count read 78 while
77 distinct tests ran - and a dropped `Result` in the assembly-bonus seat-hunt.
Neither moved a measurement.

All seventeen of the spec's `file.rs:line` citations resolved against the tip.
Every one lands on code that says what the spec says; eight have moved and the
table in `analysis/the-hundred.md` gives the true addresses.

**One thing the spec is wrong about**, and it is the trap the Switchyard was
moved twice by: A0's free-rung list counts events and not town gates. Thirteen
rungs are genuinely free, not nineteen, and two of the mission's four road
placements are on gates.
