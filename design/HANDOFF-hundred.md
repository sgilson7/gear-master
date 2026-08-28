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
| **F1** The county, generated | **done** - `analysis/the-hundred.md` "F1 the county, generated" |
| **F2** Standing in it | **done** - `analysis/the-hundred.md` "F2 standing in it" |
| **F3** The clock | **done** - `analysis/the-hundred.md` "F3 the clock" |
| **F4** Tolls, and the two Requirement variants | **done** - `analysis/the-hundred.md` "F4 tolls" — **DEPLOY POINT 1** |
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

### F1 - The county, generated

`county.rs` and `tests/county.rs`: a 7x7 as a pure function of a derived seed,
twelve checks, a 32-seed retry and an authored `FALLBACK` that passes all of
them. **937 engine, 78 GUI, 5 CLI. No warnings.** The `baseline` printer is
byte-identical to F0, `gear_at` is unmoved on all 6,216 placements and so are
the three road fixtures. Nothing is wired to a run.

**Ten thousand seeds, zero retries.** The first version retried 54% of the
time and one tally said why: V6 spaces the three pinnacles, and the generator
was asking it of one of them. The hill *is* the Ordnance's pinnacle and the
Commissioner is one of the sealed three, and both are knowable when they are
placed - so both are filtered rather than refused.

**Nine decisions the spec left to be made**, all in `analysis/the-hundred.md`
under F1 with what each one cost. The two worth knowing before F2: the hill is
stored as a pinnacle and *drawn* as empty, which inverts B1.1's presentation
and leaves its behaviour intact; and the pale is an Event tile rather than a
new `TileKind`, which is what keeps A1.2's composition exact.

**Two findings that were the spec's rather than the code's.** V9 did not make
C1's argument true - a gaol one tile from a mouth is not a shortcut - and it
has a second half now. And V2 read as a union is satisfied by the county it
exists to refuse; it is a matching.

### F2 - Standing in it

A2.1's movement, A2.2's state, `TripSource` and its census, `Interrupt::County`
on top of the stack, `Action::County` on all six towns, the CLI's `go`, `walk`
and `out`. **953 engine, 78 GUI, 8 CLI. No warnings.** The `baseline` printer
is still byte-identical to F0.

**The cap is the enum.** `trip_cap()` sums `TripSource::seats()`, where a town
is worth `TOWNS.len()`, so a mission that adds a way down cannot land without
the suite making it raise the cap. Ten, and every figure in A4 was costed
against ten.

**One bug introduced and caught in the same hour**, by a test whose doc comment
named the two towns that have a pedestal. Sump Bottom and Kettleworks do not,
and a shared door constant handed them one.

**Two pins moved.** The pedestal is no longer the only thing outside the
one-action rule. Both assertions say why in the message rather than in a commit.

**The exit criterion could not be met through the driver**, and it is the
Switchyard's M3 wall unmoved: no board the CLI can build from its own verbs
clears rung 9, and Kettleworks' gate is after rung 17. Split in two, both in
the suite - the driver proves a trip replays byte-identically, and the engine
walks three towns and fifteen moves. `every_town_lets_you_down_at_its_own_mouth`
does all six.

### F3 - The clock

`events_resolved`, moving. **957 engine, 78 GUI, 8 CLI. No warnings.**

**A5's three increment points are one.** Every event in the game is answered
in `take_choice_unchecked` and nowhere else, which is the strongest form of
"nothing else" there is. A fight won, a fight lost, a town door, five tiles of
county and a shop reroll are all walked past the counter to prove it.

**And one place it comes back down.** `Outcome::Defer` takes a door back off
`answered` - declining is not answering - and it has to come off the clock too,
or a run advances the Drover by saying "not yet" to one door repeatedly. That
is an interception bought rather than intercepted.

### F4 - Tolls, and the two Requirement variants

Six figures, the tax, one-tile visibility, and two inert requirements.
**969 engine, 78 GUI, 8 CLI. No warnings.** `baseline` byte-identical to F0
and `gear_at` unmoved on all 6,216 placements.

**A3's formula is out by a factor of a thousand and its worked example is
right.** A stat is per activation; a rate is `stat * 1_000_000 / cooldown_ms`,
which is what `ItemProfile::dps_milli` has computed since the gear-slot
rewrite. `flow_is_not_mana` pins A3's pair: 2000 and 3000, and the board with
less mana crosses the deeper river.

**Every threshold A3 ships is trivially met.** Eleven of the twelve are
crossed by the auto-builder's board and the twelfth by the starter. The owner
pays 11.77 flow into a river asking 2 to 6 and holds 131 curse resistance
against a hedge asking 3 to 8. D-5 said this would happen; F11 is where it is
fixed, and F4 deliberately does not move a number - a threshold bent before the
measurement is a threshold bent to a guess.

**A minimal county screen landed with F4 rather than with F9**, and it is
scope taken on purpose. Taking the way down set `county_at` and left the town
gate up, so the town screen re-rendered with no verb on it - the pedestal's old
bug, with a trip spent on the way in. Deploy Point 1 asks a person to find out
whether five moves feels wrong, which needs a screen. It is not the map; A8's
second tab is still F9's. Its layout is pure and tested, and the test caught
the way out hanging eighteen pixels past the border on its first run.

**The rung column in F4's own deliverable says the same thing four times.** A
share code is one board and it does not grow, so five of the six figures do not
move with the rung; only the toll gate reads it, through the bounty. The table
is one row a board, and F11 calibrates against the progression the four boards
stand for.

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
