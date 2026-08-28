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
| **F0** Baseline | **done** |
| **F1** The county, generated | **done** |
| **F2** Standing in it | **done** |
| **F3** The clock | **done** |
| **F4** Tolls, and the two Requirement variants | **done** - DEPLOY POINT 1 |
| **F5** Bearing, Overtake, Commons - inert | **done** |
| **F6** The catalogue, once | **done** |
| **F7** County events and the word crossing | **done** |
| **F8** The chains, as frames | **done** |
| **F9** The map's second tab | **done** - DEPLOY POINT 2 |
| **F10** Theme | **done** |
| **F11** Thresholds | **done** |
| **F12** Boards, **borrowed** | **done** - and the hand-packing is deferred, see §5 |
| **F13** Rating pins | **done** |
| **F14** Acceptance, by replay | **done** |
| **F15** The record | **done** - this file, `CLAUDE.md`, and the merge |

Every milestone's numbers are in `analysis/the-hundred.md`, one block apiece,
headed by the commit they were read off.

## 3. Open questions for the user

### THE ENCLOSURE finishes on 5% of simulated censuses, and that wants a decision

A validity pass over the county (`analysis/the-hundred.md`, "F16") walked a
hundred and twenty full censuses with each finished board. It found and fixed
one real bug - the pale was consumed on first contact - which took the gate
from opening on 19% of runs to 61%. What is left is not a bug:

- **THE ENCLOSURE finishes on 5%**, and THE PARISH on **none**, which follows
  since the perambulation wants all three chains.
- The cause is sequencing, and it is measured: **the checklist becomes ready on
  trip nine of ten in 81 of 120 runs.** After that the chain wants two more
  journeys - one to stand on the pale, one to reach the far corner where THE
  COMMISSIONER is - and it has one trip.
- The checklist itself is fine. The three region lines pass almost always; two
  stones fails 14 of 120 and the orb 5 of 120. It is **eighteen tiles across
  three regions** that necessarily completes late, which is B3.1's own design:
  "the chain you finish by having been everywhere."
- And the simulation is **generous** - it takes all ten trips without earning
  the Constable or the Waste bet, carries a finished board from rung one and
  never loses. Five percent is the optimistic figure.

Three dials, all of them yours:

1. **Fewer tiles per region.** Six to four is fifty-four tiles' worth of walking
   down to thirty-six, and it is one number in `Run::pale_checklist`.
2. **Opening the pale grants a trip.** A `TripSource` exists for exactly this
   shape - `Perambulation` is granted rather than taken - and it would make the
   gate pay for the journey it creates.
3. **THE COMMISSIONER stands where the pale can reach.** The far corner is
   B3.1's image and the distance is what costs; a sealed region adjacent to the
   pale would keep the image and lose the second journey.

Nothing is blocked. The chain is finishable - five percent is not zero - and
every tile of it is reachable.


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

---

## 5. What is not done

- **The five wear borrowed boards.** F12 gave each of them a ladder creature's
  whole board rather than packing one, and that is the mission's one deliberate
  half-measure: packing by hand wants somebody reading the diff, and it was
  moved past the deploy on the owner's instruction. `hundred::the_five_wear_a_board_borrowed_from_their_band`
  names every pair and is what has to change when they are packed - and it
  should change to something that measures the boards rather than compares them
  to somebody else's.
- **Three of the five run past sudden death on the owner's board** at Medium.
  That is the board and not the county: the same board needs 38 s for the
  ladder's own band-48 creature and loses to Francis outright. Recorded rather
  than fixed, and it is the first thing the hand-packing should look at.
- **Nobody has played it.** Still true and still the biggest gap. Every claim
  in this file is from the suite, from two CLI transcripts that diff clean, and
  from reading the road aloud.
- **The CLI has no theme switch**, which predates this mission and is why a
  themed county has never been printed by the driver. The GUI has one.
- **The Drover's strength scaling is on.** D-4 said to ship it behind its own
  constant so it could be turned off in one line. It is
  `county::DROVER_STRENGTH_PER`, it is 8, and at clock 300 the pursuit meets
  you a quarter stronger. Nobody has played it either.

## 6. The five things that cost the most

**A0's free-rung list counts events and not town gates**, which is `CLAUDE.md`
§6 trap 27 and which moved two of the mission's four road placements. Thirteen
rungs are genuinely free, not the nineteen A0 lists. Found at F0, before a line
of content was written, which is the cheapest possible place.

**`ALTERNATES` is append-only and nothing said so.** Five creature specs
inserted at the top of that table moved **2,592 lines** of `gear_at.txt`
without one creature changing what it wears, because the fixture keys every
line on `ALTERNATES[i]`. It reads exactly like a re-gearing. `CATALOG` has the
same property and everybody knows it; this one had never come up.

**Overtake had to repeat the activation and not the blow.** The first version
put it in `reps` beside `Echo` and `Fork` - the cheap place, and the wrong one
for exactly the slot the effect is for. `reps` repeats the swing, only weapons
swing, and Overtake is gloves-only, so the effect did nothing whatsoever. The
**negative** test found it: the control glove reported zero opening blows,
because a glove has none.

**A forced event goes to the front of the stack, and a door in front of another
is not a queue.** C2 was pushed through `forced_event` and broke the
Switchyard's chain walk in both modes, because
`road_stack::the_door_underneath_cannot_be_answered_over_the_top_of_the_one_in_front`
is deliberate. Making `take_choice` answer whichever standing door owns the
choice would also have fixed it, and was reverted: the rule is the game's and
the fix belonged on the other side. Vessey has his own field and waits.

**Reading the road aloud found the thing no lint could.** THE THEODOLITE, THE
STOCKMAN and THE COMMONS each set a flag and **nothing anywhere read one** -
the whole payload of three doors standing on three rungs. They have a reader
now: a chain nobody has explained to you is stones in fields, and the map says
so.

## 7. What shipped

| Part | What |
|---|---|
| **A1** | A seven by seven county, a pure function of a seed derived from the run's, with twelve checks and an authored `FALLBACK` that passes all of them |
| **A2** | Five moves a trip, orthogonal, resolved in order; ten trips a run and the cap is the enum |
| **A3** | Six tolls, all of them rates, in integers, over the assembled board - and a formula A3 got wrong by a factor of a thousand |
| **A4-A5** | The census, and a clock that is doors answered and nothing else |
| **A8** | A second tab on the M overlay, and `route::ascii` grown a county half without moving the road |
| **B1** | Three sightings, two of which are knowledge and the third a key, and a hill nothing marks |
| **B2** | A sixteen-tile ring, a pursuit that walks by the clock, and a door answered that can bring it to you |
| **B3** | A checklist read at one tile, one requirement for five lines, and a far corner that opens |
| **B5** | A perambulation: a route rather than a destination, broken by an illegal move or a failed toll |
| **B6** | One word, up out of a field and back down as an answer |
| **C** | A constable who takes you to the middle, and a man with a legal opinion about your empty grid |
| **D** | Sixteen milestones, every one measured against the one before it |
