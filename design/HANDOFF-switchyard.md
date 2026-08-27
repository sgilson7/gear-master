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
| **M1** The floor graph, landed inert | **done** | "M1 the floor graph" |
| **M2** Run state, the four transitions, the stack | **done** | "M2 run state, the four transitions, the stack" |
| **M3** Sidings, the CLI verbs, the interface | **done** | "M3 sidings, the CLI verbs, the interface" |
| **M4** Four actions, four weights, four rows - inert | **done** | "M4 four actions, four weights, four rows" |
| **M5** The catalogue lands once | **done** | "M5 the catalogue lands once" |
| **M6** The chain, the yard, the frames, the destinations | **done** | "M6 the chain, the yard, the frames, the destinations" |

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

### M1 - The floor graph

`Dungeon.floors` is a list of `Floor`s that know where they lead, and
`Dungeon.landings` is gone into `Floor.landing`. The six shipped dungeons are
straight lines in the new shape and `every_shipped_dungeon_is_a_straight_line`
is what holds them there.

**Landed inert, and inert was measured three ways.** The whole `baseline`
printer is byte-identical to M0, cargo's timing lines apart - so the
four-board table, the cadence, the mind figures, rungs 1-14, no-weapon
viability and the census have not moved. `no_creature_changed_what_it_wears`
is green on all 5,568 placements. And every one of the six dungeons was walked
from the top on the **pre-M1 code**, by stashing the milestone and running the
same walk against `&[&str]` floors, then diffed against M1's transcript:
`diff` is empty. That transcript is `analysis/replays/dungeons.txt` and it is
now a fixture with a test on it.

**Seven decisions worth reading before M2**, all in `analysis/switchyard.md`
under "M1 the floor graph" with the reasoning:

1. `route::ascii` prints `(3 fights)` where it printed `(3 floors)`. A1.4
   specifies that word and acceptance criterion 3 says "byte-identical"; they
   cannot both hold, and A1.4 wins because it is the section that specifies
   the printer. The **number** does not move. The fixture holds the real
   pre-M1 bytes and the test applies the one substitution in its own body, so
   the word that moved is named in the assertion.
2. THE THRESHOLD is not on the route map at all - criterion 3 names a dungeon
   that has no node, because a dungeon node hangs off an event choice and THE
   THRESHOLD is a town door. THE CREVICE IN THE ROCK is the one the map draws.
3. `Floor::along` takes the exits rather than a floor number: a `const fn`
   cannot return a reference into a temporary built from its own arguments.
4. The seventh graph lint is half-written until `Where::Siding` lands at M3,
   and its own comment says so rather than reading green and meaning nothing.
5. A1.7's call-site table missed three sites; all three were mechanical.
6. `Theme::landing` falls back per floor where `Theme::landings` fell back per
   dungeon, which fixes a latent hole nothing had stepped in.
7. M1's replay is the six-dungeon transcript rather than a CLI script, because
   no board the CLI can build from its own verbs clears rung 9, which is where
   THE CREVICE's door stands.

**The eight graph lints** (A1.1 asks for seven; the eighth is `fights_ahead`'s
own arithmetic): `every_exit_leads_somewhere_that_exists`,
`no_exit_points_at_the_mouth_or_at_itself`, `no_dungeon_doubles_back`,
`every_floor_is_reachable_from_the_mouth`,
`the_points_have_a_scene_and_nothing_else_does`,
`no_floor_offers_a_way_in_that_nothing_uses`,
`every_shipped_dungeon_is_a_straight_line`,
`fights_ahead_counts_the_road_out_and_not_the_rooms`.

Two existing lints changed rather than moved. `every_floor_names_a_creature_
that_exists` lost its `landings.len() == floors.len()` assertion, because the
type guarantees it now, and gained "a cleared floor says something" in its
place. `every_dungeon_pays_something` gained "or every buffer stop pays its
own way", which is the shape a graph wants and which the yard's four leaves
will be the first to use.

**Suite:** 813 passed, 41 ignored, 0 failed. GUI 62. No warnings.
750 insertions, 123 deletions across 11 files.

### M2 - Run state, the four transitions, the stack

`Run` carries `cleared_floors`, `at_points`, `took_exits` and
`entry_started_at`. `Interrupt` has a `Points` variant that blocks a rematch
and says `"the points"`. Clearing a floor, throwing the lever, leaving,
losing and coming back in are five transitions with a test each, and the
fixture they are tested against is `common::A_YARD` - four rooms, a fork at
the top, four creatures that already exist - which is **not** in `DUNGEONS`
and never will be.

**831 passed, 41 ignored, 0 failed. GUI 62. No warnings.** The `baseline`
printer is still byte-identical to M0 and `acceptance` is green.

**The walk-through rule in A1.3 is wrong, and a test found it.** The spec says
to follow a cleared floor by taking "the single uncleared exit" - meaning the
exit whose *next room* is unbeaten. A run that walked one road as far as its
first room and left has beaten that road's first room, so the rule declares
that road finished and quietly sends the player down the other one, past every
room on the first that nobody fought. The engine asks
`fights_ahead(to, cleared) > 0` instead - "is there still a fight down there" -
which agrees with the spec everywhere the yard's own shape can produce,
including A4's worked eight-floor example, and disagrees exactly where
`leave_dungeon` lets a run stop half way. `a_road_half_walked_is_still_a_road`
is the case.

**Two other deviations, both recorded in `analysis/switchyard.md`:**

- `enter_dungeon_at` takes `&'static Dungeon` rather than an id, because M2's
  whole job is proving the primitive before content exists and a test-local
  dungeon has no id to be found by. `enter_dungeon(id)` resolves and delegates.
- `Interrupt::Dungeon` became a struct variant carrying the banner's two
  numbers, because A1.4 asks `describe()` to say things only the run knows and
  `describe(self)` has no run. One formatter, which is what A1.4 wanted.

**Two pins moved, both re-pinned with the reason in the assertion.** The
dungeon replay fixture gained the creature's name on fourteen banner lines and
changed nothing else - `grep -vc banner:` over the diff is 0 - which is
acceptance criterion 3 in its own words. And
`road_stack::a_dungeon_sits_on_top_of_whatever_it_was_entered_from` now reads
"floor 1 of 2" for a run hand-placed on floor 1, because a run put there has
won no fights and has two ahead; a second run was added beside it that walks
in properly and pins "2 of 3".

### M3 - Sidings, the CLI verbs, the interface

`Where::Siding` lands and nothing is one yet (`no_destination_is_a_siding_yet`
says so out loud, and names M6 as the milestone that should delete it). The
CLI has `throw <n>` and `leave`, and `show_road` prints the roads out under
the banner. The GUI has a points screen built on the event screen's layout, a
way out on two screens, a pip row that counts fights rather than rooms, and a
map hover that says how deep a dungeon goes and how many times it asks which
way.

**834 engine, 65 GUI, 3 CLI. 902 in the workspace. No warnings.** `baseline`
is still byte-identical to M0, three milestones in.

**The CLI verb replay could not be written as specified, so the harness was
written instead.** Neither verb can be walked through the driver yet: nothing
in `DUNGEONS` has points until M6, and no dungeon is reachable by any board
the driver can build from its own verbs. What landed is
`crates/cli/tests/replay.rs` - the first test the CLI crate has ever had - a
scripted run piped in twice and byte-compared, plus both verbs' refusal paths
and a check that `help` does not advertise a verb that is not there. This ends
a standing gap: `post-unwinding.md` §1 marks the two CLI replays **unverified**
because "nobody wrote the script down". It is written down and it runs in the
suite, and acceptance criterion 1 extends this file at M6 rather than
inventing a harness under deadline.

**One lint the spec does not ask for.** `no_two_sidings_land_on_the_same_floor`
- two orbs may point into one dungeon, which is the whole design, so the
existing "no two destinations share an id or an orb" does not catch two
sidings written onto one floor. The second would be refused by the visited-set
while looking like a fresh ticket, which costs a player an orb to find out.

**Four pure functions carry the GUI's layout** - `dungeon_banner`, `pip_row`,
`ticked_pips`, `points_cells` - for the reason `chip_rects` is one: the
geometry and the words are testable without a font context, and
`cargo test -p gearmaster-gui` is the only thing that compiles that module
(`CLAUDE.md` §6 trap 14).

### M4 - Four actions, four weights, four rows, and nothing carrying any of them

`Shunt`, `Ballast`, `Derail` and `Accrue` are in `piece.rs`, resolved in
`combat::apply`, priced in `rating`, homed in the basis, described in the CLI,
the GUI's glossary and the tooltip layer, and tested to the tick. **No
component speaks one.**

**The exit criterion is the measurement of A2.5's argument.** Four weights for
verbs no piece speaks price nothing, so no creature re-gears on any setting:
`gear_at` is unmoved across all **5,568 placements**, the whole `baseline`
printer is byte-identical to M0, and `report_shape` differs by exactly four
rows, all reading `0/0` at budget 0.

**848 engine, 65 GUI, 3 CLI. No warnings.**

**Two lints found the milestone before it found them.**

`every_rule_names_a_mechanic_that_exists` refuses a rule that carries nothing,
which is precisely what a phase-disciplined M4 lands four of. Rather than
loosen it, `RULES_AWAITING_THEIR_PIECES` names the four and the milestone that
empties it, and `no_rule_waits_for_a_piece_that_has_arrived` goes red the
moment any of them finds a carrier - so M5 cannot land the components without
putting the rows back under the lint. An exemption that outlives its reason is
a lint with a hole in it.

`assembly::every_action_is_well_formed`, which the spec extends, **did not
exist**. It does now, carrying the half of its job that is representable.
The other half - refusing `Derail { target: Yourself }` - is done by the type
instead: `Action::Derail` carries no target at all. A lint that can only ever
pass is a type that should have said so.

**Two things the spec did not say, both worth knowing.** A shunt owes only
what actually landed, because the target's bar is capped and charging the
giver for time that went nowhere would make a shunt a net loss. And
`Combatant::player` starts every pool and the wall at zero whatever `Stats`
says - so a player built from `Stats::ZERO` has no maximum health and is dead
on the first tick, and every measurement off that fight reads as "the
mechanic does nothing". Both are written into the code that would otherwise
teach them again.

### M5 - The catalogue lands once

Eight components appended at the end of `CATALOG` and never inserted: Ballast
Bed, Points Rodding, Booking Hall and Signal Wire; the Shunter's and
Signalman's Orbs; A Word About the Sidings and A Word About the Points. All
eight event-only, which is doing four jobs at once - off the road shelves, out
of the crucible both ways, out of `dearer_than`, and out of every footprint
family `stepped_component` walks.

**`gear_at` is unmoved across all 5,568 placements**, which is A6's claim
measured at the milestone that could have broken it, and the four-board table
is byte-identical. The only thing in the whole `baseline` printer that moved
is the census: 504 pieces to **512**.

**853 engine, 65 GUI, 3 CLI. No warnings. Every ratchet row and every quota
still 0 away.**

**M4's second ratchet did its job.** `no_rule_waits_for_a_piece_that_has_
arrived` was red until the four verb names came off
`RULES_AWAITING_THEIR_PIECES`, which put the four exclusivity rows back under
the lint they had been exempted from. The list is kept but empty, so the next
mission that wants to settle a weight before its carriers exist finds the
mechanism rather than reinventing it.

**One real bug found, and it was not ours.** `Shop::restock` deals slots on
`n.powf(SHELF_TILT)` tickets where `n` counted the **whole catalogue**, not
the pool it had just finished filtering - so a slot was dealt in proportion to
how much of it exists rather than to how much of it is for sale. Appending
eight unsellable components moved the shelves and
`avail::the_shelves_are_not_the_same_six_things_every_time` refused them at
4.1x. It counts the pool now. The blast radius was measured before the fix was
kept: **zero other tests moved.**

**The theme's piece names came in at M5 rather than M7**, because
`the_turtle_theme_covers_the_catalogue` demands a name for every component and
the gear skill says to write it in the same change as the piece. M7 keeps the
scenes, the creatures and the effect vocabulary.

### M6 - The chain, the yard, the frames, the destinations

Two rumours, four doors, THE SWITCHYARD's nine floors with three sets of
points and four buffer stops, nine frames, nine undressed creatures and two
sidings. **860 engine, 65 GUI, 5 CLI. No warnings.** Zero authored `gear:`
boards, the four-board table byte-identical, `gear_at` unmoved.

**The property the graph exists for is now measured.** A run that walks the
yard greedily - in at the mouth, back by every siding a ticket can pay for,
always taking a road with a fight left down it - fights **eight distinct
floors of nine**, and the one it never reaches is always a buffer stop. Each
line's ends pay the ticket to the *other* line, so the ninth room is behind an
orb that has been spent.

**Two of the four doors could not stand where the spec drew them.** A0's list
of free rungs counts events and not towns, and a town gate has refused to share
a rung with an event since long before this mission. THE TIMETABLE moved 18 to
**20** (Kettleworks) and THE LAST TRAIN 32 to **33** (High Wick); the signal
box's window opens at 21 rather than 20 in consequence. The spec's argument for
18 - that the stack pops a gate first - is true of a *fountain* at index 7 and
does not transfer.

**Part B's own audit missed a fourth instance of the blind spot it warns
about.** THE YARD THROAT named Ambrose only at a full stop, which
`names_something` cannot tell from an article. Repaired in the engine **and in
the spec**, which is what Part B asks for so the two never hold different
versions of a scene.

**The M1 replay fixture hung the suite for six minutes** by walking a dungeon
with points and never throwing them. It walks the six it is a fixture for, and
it is bounded now.

**Five linear-dungeon assumptions surfaced at once** - bands rising along the
list rather than along a road, theme uniformity, the pays rule, the map
fixture's length check, the banner walk - and all five are re-pinned in
`analysis/switchyard.md` §M6 with the reason each one was right about a list
and wrong about a graph.

**Four phase-discipline budgets went red together** and all four now read
`bestiary::unpacked()` rather than a copied list, so M9 clears them by packing
a creature rather than by editing a test. `bestiary::UNDRESSED` went 0 to 9,
which is the only budget in the repository allowed to rise, and its doc comment
says so.
