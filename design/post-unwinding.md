# After the Unwinding — the record, checked against the code

**Commit:** `18d1b85` on `main` ("The glossary is a grid of words now, and every
shelf is one screen"), 2026-08-26. Eight commits past the merge (`21c0bb4 The
Unwinding`) and the publish (`b0a775c`). Everything below was read off this
tip, not off the documents. Where a document says otherwise, the document is
quoted so the disagreement is on the page.

Written as its own file rather than folded into `HANDOFF.md`, because
`HANDOFF.md` is the mission's summary of itself and this is the audit of it -
two documents with two authors' interests, and the second should not overwrite
the first. Where they disagree, this one is later and was measured.

## How this was measured, and the one condition on every number

The suite was built and run on **rustc 1.75.0**, which is what
`Cargo.toml:7` says the workspace needs. It does not compile there.

- `Option::is_none_or` (stable since 1.82) at `bestiary.rs:528`,
  `shop.rs:241`, and in `tests/structures.rs:337`, `tests/chain.rs:103`,
  `tests/pack_francis.rs:286,967`, `tests/packing.rs:1911`.
- `const` items referring to `static` items (stable since 1.83): `EVENTS` is
  `pub const` (`event.rs:590`) and holds `&TABLE_THREE`, `&THE_BACK_ROOM`,
  `&THE_HERALD`, `&TABLE_TASK`, `&THE_FLOCK`, `&THE_SHOWFIGHTERS`, all
  `pub static` (`event.rs:514-583`).

To measure anything, a thirteen-line shim was applied **in a scratch clone
only**: `x.is_none_or(f)` → `x.map_or(true, f)` (identical semantics), and the
six statics → `const` (identical semantics for every use in the tree; nothing
compares them by address - `grep ptr::eq` finds one hit, in the GUI, on a
theme). The shim is not committed anywhere and not part of any deliverable.
**Every figure in this file was taken under it.** On the owner's toolchain -
which is evidently ≥ 1.83 - the same commands should give the same numbers; if
they do not, the shim is the first suspect and this paragraph is the bug report.

The declared MSRV is therefore wrong, and that is a finding: `rust-version =
"1.75"` promises a floor the code does not honour.

## 1. Where the code is

| | |
|---|---|
| `main` | `18d1b85`, published; GitHub Pages serves `docs/` |
| Engine suite | **776 passed, 0 failed, 38 ignored**, 49 integration binaries + lib (`cargo test -p gearmaster-engine`, 98 s wall on one core; 91 s of that is tests executing) |
| Warnings | **2** under rustc 1.75 - `tests/packing.rs:1141` redundant import, `tests/primitives.rs:440` unused variable `and`. The second fires on every toolchain. `HANDOFF.md` says "no warnings" |
| GUI suite | 60 tests, per commit `18d1b85`; not run here (macroquad does not build in this container - **unverified**) |
| `the_catalog_keeps_every_rule` | green; every exclusivity row 0 away, every quota 0 away, 0 identity mechanics on floating kinds |
| Frame lint | green; `FRAMES` is 15 and every one has a board |
| Replays | `acceptance::e6_1` green; the two CLI replays `HANDOFF.md` cites were not re-run here (the CLI builds; nobody wrote the script down) - **unverified** |

The counts the documents carry are quotes from three different days.
`CLAUDE.md` says 764, `HANDOFF.md` says 764, the last post-merge commit says
774, the suite says 776 (the difference from 774 is presumably the two
`#[ignore]`d lib tests being counted differently; not chased). None of them is
wrong; all of them are old.

## 2. What of the spec landed

Read against `design/the-unwinding.md` Part E and its reconciliation blocks.

| Part | Spec | Landed | Where it lives |
|---|---|---|---|
| A1 | Empowerment and shield magic-only | yes | `combat.rs` (`take_typed`); `tests/typed_lanes.rs` (8) |
| A2 | Spellblade and Deflection | yes; `SPELLBLADE_POWER = 50` (`combat.rs:722`), `DEFLECTION_FLAT = 10` (`:729`) | catalog homes per `report_shape`: Spellblade 5 gloves + 2 weapon, Deflection 6 chest + 1 greaves |
| A3 | Insight and Dread | yes; `Resource` is 8, `DREAD_DIVISOR = 2` (`:736`); gate is `Run::insight_unlocked` and `Shop::insight_open` | `tests/insight.rs` (13) |
| A4 | Hidden towns, conditions, outcomes, melt, rung 51 | yes; `Requirement` has 12 variants, `Outcome` 35, town `Action` 17 (`event.rs`, `town.rs`) | `tests/road_machinery.rs` (23), `hidden_towns.rs` (8) |
| A5 | Chain state | **changed shape** - words in the tray plus one flag, see §3 | `run.rs` `flags`, `counters` |
| A6, A9 | Tooltips, receipts | yes; `describe()` on `Requirement`/`Outcome`, `Run::last_receipt` | `tests/tooltips.rs` (13) |
| A7 | Road stack | yes, **derived** not stored (`Run::road_stack`, `run.rs:855`); pop order amended (#12) | `tests/road_stack.rs` (11) |
| A8 | Dungeon presentation | yes; `Retold { entry, landings }` in `theme.rs` | `tests/dungeons.rs` (14) |
| A10 | Route map | yes; `route::route` (`route.rs:124`), `route::ascii` (`:265`) | 9 tests in `route.rs` |
| B | The chain | yes: four stations, three words, THE MANSE and THE SLAGWORKS hidden, THE HERALD brawl, THE UNWOUND at 51 | `tests/chain.rs` (13), `phase_two.rs` (7), `completable.rs` (4) |
| C, F6, G5, H5 | Turtle telling | yes, `theme.rs` only; `told: &[Retold]` keyed by id | `tests/two_voices.rs` (6 + 1 ignored) |
| D | Three pairs, two classes | yes; Unionized stacks, Showstopper | `tests/structures.rs` (24) |
| F | Five unconditional events | yes, at rung indices 3, 10, 16, 23, 26 | `tests/unconditional_events.rs` (12) |
| G | Extra Large, orbs, destinations | yes; 4 destinations (`pedestal.rs`), orbs are Orb-kind weapon cores first | `tests/pedestal.rs` (9) |
| H1 | Twelve payables | yes; 4 relics (`relic.rs`), crushables, the rod | `tests/relics.rs` (17) |
| H2 | Nine structures | **eight** - the Brain Farm slipped (#20) | `tests/structures.rs` |
| H4 | Frames and four themes | yes; `MonsterTheme` is 10 variants, `FRAMES` 15 (`bestiary.rs:37, :400`) | 9 tests in `bestiary.rs` |
| E1.8 | Engraving, Brain Farm | **slipped** at M8 (#20) | - |
| E4.2 | Boards by hand in `make pack` | **changed shape** - 13 of 15 by the generator, see §3 | `combat.rs` `ALTERNATES` |
| E6.5 | A fourth build that wins because of the mind lane | **not met** - see §4 | `tests/reference_builds.rs` |

Census at the tip, from `report_catalog_census` and the source:
**504** pieces (helmet 96, chest 71, gloves 83, greaves 67, weapon 187; 120
inert, 23.8%); **69** creatures (`LADDER` 50 with `RUST_GOLEM` spliced in at
`combat.rs:813`, `ALTERNATES` 19, `CREVICE` 0); **33** events; **6** towns (3
pinned, 3 hidden); **6** dungeons; **8** rumours; **31** classes; **15**
frames; **10** themes; **4** destinations; **4** relics.

## 3. What changed shape during execution, and why

Each of these is a place where the code is not what the body of the spec
describes, and the reconciliation blocks say so. Listed in the order they will
matter to somebody reading the spec cold.

1. **The chain's state is the words you carry.** A5 lists five flags;
   `run.rs` has `flags: Vec<&'static str>` and only `threshold-cleared`
   is one. The astronomer's word is in your tray and the towns are in
   `towns_revealed`; a second copy of a fact is a second thing to keep true
   (#21.1).
2. **The road stack is a function, not a field.** `Run::road_stack()`
   (`run.rs:855`) derives the interrupts from `dungeon`, `town`,
   `at_fountain`, `answered` and `brawl`. "Resolving an interrupt may push
   more" needed no code (#12, HANDOFF-unwinding M3).
3. **Pop order is gate, fountain, events** - the order the road already
   had, not the spec's fountain-first. `FOUNTAINS = &[7, 14]` (`run.rs:2652`)
   collides with Sump Bottom at index 7, so this was forced (#12).
4. **Gold is in bounties, not gold.** Every figure in Parts B-H is 1x/3x/10x
   the standing rung's bounty, resolved at resolution
   (`Outcome::Pay { times }`, `Requirement::Purse { times }`), because the
   milestone table the body priced against does not exist (#6, #16).
   `acceptance::e6_7` lints it.
5. **Rumour doors stand in windows** (`Trigger::Whispered { from }`,
   `event.rs`), not on a rung (#21.3). The Wrong Stars is sold at the pub
   rather than found by luck (#21.2). The Slagworks stands after 33, not 32
   (#21.5). `Outcome::Defer { rungs }` exists so a gate can find you again
   (#21.4).
6. **THE BUYER's menu is gated, not generated** - three static doors that
   open on holdings (#22.1). `Choice` is a table.
7. **THE CONTRACT is frost in `combat_items`** (`CONTRACT_SLOWER = 50`,
   `run.rs:171`), applied where every other speed is, not taught to
   `simulate` (#22.2). `CURSED_SLOWER = 25` (`:181`) closed a list nothing
   read.
8. **The passenger is a component** (The Stranger's Parcel) that costs cells
   (#22.3). Showstopper is claimed on agreeing to headline, not on winning
   (#22.4). A Word About the Picket comes from THE INSPECTION *refused*
   (#22.6). The sealed bid is capped at 5,000 (#22.7).
9. **Hollow fills Helmet and Chest** (`bestiary.rs:130`). `monster-themes.md`
   §7 says Helmet and Gloves, and commit `346d9df` says "Hollow is helmet
   and gloves" while fixing THE SHADOW's board. The code is the news; §7 and
   the commit message have the bug report. §1's amended table agrees with
   the code.
10. **The boards are the generator's.** E4.2 says "by hand, in `make pack`";
    the owner's instruction at M17 changed it to "generator first, rebuild
    by hand afterwards". 13 of 15 frames wear `pack_francis` output; DOORKEEP
    and THE STAIR THAT LISTENS are the owner's. The generator's guards were
    skipped for undressed frames only; the curve guard was not.
11. **Frames took the ladder's stats at their band.** They shipped with
    placeholders (THE LAST LANDING at 1,200 health where band 26 carries
    2,230) because nothing fights a frame. THE WUMPUS is 748 health at band
    32 on purpose - its board is heavy.
12. **THE UNWOUND was scaled by measurement**, x1.5 on health and strength
    (10,000/230 → 15,000/345), to land two-of-three losses and a 28.0 s win.

And three that happened **after** the merge, in no ledger:

13. **THE BIGGER SIGN moved from rung 41 to rung 13** (`c99c261`). It
    revealed EXTRA LARGE, which stands after rung 14; the reveal fired 27
    rungs after the gap. Every test asked whether *something* revealed the
    town; none asked whether it could in time.
14. **THE PICKET LINE's window opens at 20, not 13** (`e53a50b`) - the word
    it wants is first handed over at 20. **THE FOUNDRY REMEMBERS asks for
    one melt, not two** - no run can take two crucible actions.
15. **`Event::Watched` is logged on every sighting** (`346d9df`), carrying
    the relation (`b1b54fe`), because the interface replays a log, and the
    combatant a log carries is the pre-fight one.

## 4. What was cut, deferred, or not achieved

- **Engraving and the Brain Farm.** Slipped at M8 on a measurement:
  `rating::piece_rating` is `fn(&PieceDef) -> i32` and everything prices
  definitions, so an engraved *instance* would fight right and be priced,
  named and rated as the piece it was. Amendment #20 says what unblocks it
  (a rating over instances). Nothing depends on either.
- **The fourth reference build.** E6.5 wants a board that wins against THE
  UNWOUND *because of* Deflection and Insight. `reference_builds.rs` has
  `THE_FOURTH` as a fifteen-name list; the criterion that is actually
  asserted is "two of three shipped boards lose, and the third wins inside
  the window". The owner's board wins at 28.0 s. A board that wins by the
  mind lane has not been built. This is the single open acceptance item.
- **The hand-authored boards.** Thirteen creatures are wearing samples.
  `HANDOFF.md` §6 calls this "the owner is rebuilding them by hand"; nothing
  in the eight post-merge commits did, apart from THE SHADOW getting two
  pieces so it lands *something* (`346d9df`).
- **The ledger's last two entries.** `HANDOFF-unwinding.md` has no M18 or
  M19 section and its table marks M19 "next". The M18 work (the reference
  builds and acceptance sweep) is folded into M17's section under "The
  fourth build"; M19's is `HANDOFF.md` itself. The file says it was
  "rewritten into `HANDOFF.md` at M19", which is what happened, but the
  table was never updated.
- **Nobody has played it.** Still true, and still the biggest gap. Every
  claim in every handoff, and every claim here, is from the suite.

## 5. Which numbers moved

All from the printers, at the tip, at Medium. `cargo test -p
gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1`
and `--test catalog_shape -- --ignored --nocapture`.

### The four-board table

| build | Before the Unwinding (2026-08-25) | M17/M18 (baseline.md) | **tip `18d1b85`** |
|---|---|---|---|
| starter | 2/50, 100%, 45.00 s | 2/50, 100%, 45.00 s | **2/50, 100.0%, 45.00 s** |
| preset | 9/50, 100%, 9.00 s | 9/50, 100%, 9.00 s | **9/50, 100.0%, 9.00 s** |
| owner | 50/50, 75.2%, 10.50 s | 48/50, 75.5%, 9.00 s | **48/50, 75.5%, 9.00 s** |
| friend | 48/50, 97.6%, - | 48/50, 97.4%, 8.15 s | **48/50, 97.4%, 8.15 s** |

Unmoved since M16, which is what the phase discipline was for. The owner's two
lost rungs are Nine of Ashes and Francis, lost at M1 when the magic multiplier
came off iron (`HANDOFF-unwinding.md` M1); they were never recovered and the
spec never asked for them back.

### Cadence, mind damage, the shallow ladder

| | before | tip |
|---|---|---|
| Owner cadence | 6.69/s | **6.60/s**, 19 items |
| Friend cadence | - | **3.43/s**, 17 items |
| Preset cadence | - | **2.06/s**, 8 items |
| Friend mind damage, helmet | 595 → 707 (M1) | **698** |
| Owner mind damage | 62 helmet, 59 greaves | **59 helmet, 59 greaves** |
| Rungs 1-14, owner | - | 1.50-4.00 s, every rung won; rung 12 is 3.10 s |
| Rungs 1-14, preset | - | wins 1-8 and 10; 34.0 s and 31.5 s at 7 and 8 |

The friend's mind figure is 698 against the 707 M1 recorded - a small drift
whose commit was not chased. It is the channel Insight multiplies and it is
still a rounding error on three boards out of four.

### The casino corridor

Not re-measured (`two_runs::probe_the_casino_corridor` is a generator and
takes minutes). Last recorded: sharp 1,600 ms against < 3,000 needed, plain
6,000 ms against ≥ 3,000. `acceptance::e6_2_the_shallow_ladder_did_not_move`
is green, which is the constraint the corridor is behind - **corridor figures
themselves unverified at the tip.**

### No-weapon viability

| build | rungs won, weapon grid empty | rung 15 |
|---|---|---|
| owner | 42/50, best rung 48 | Defeat, 47.0 s, "the clock, not the gear" |
| friend | 35/50, best rung 46 | Victory, 44.6 s, the clock |
| preset | 0/50 | Defeat |

Criterion 3 of the gear-slot rewrite ("a build with no weapon clears rung
15") holds only by sudden death - both finished boards get there on the clock.
That is worth knowing for the RL plan: a "no-weapon clear" is a fight the
board did not decide.

### The ratchet

Every exclusivity row is at budget 0 with 0 away. Every quota inside its
band: helmet own-axis 75.0%, bleed 21.9%, filler 27.1%; chest 97.2 / 23.9 /
12.7; gloves 67.5 / 21.7 / 15.7; greaves 76.1 / 22.4 / 11.9; weapon dearest
third 39.3% interacting. Rarity per slot: one Rare, two Epic, two Legendary in
the entire catalogue; 499 of 504 are Common. (That is not a bug - names grow
with rarity and the thresholds are pinned - but an agent that reads rarity as
a signal will read almost nothing.)

### The oracle's cost, measured for the RL plan

Owner's board, release build, one core of this container (unremarkable):

| rung | fight | duration | log entries | ms per fight |
|---|---|---|---|---|
| 1 | Cave Rat | 1.5 s | 12 | **0.03** |
| 10 | Warded Idol | 2.8 s | 90 | 0.14 |
| 25 | Cog Priest | 12.0 s | 1,069 | 0.66 |
| 40 | The Rust Parliament | 22.5 s | 2,069 | 1.17 |
| 50 | Francis | 13.0 s (loss) | 1,441 | 1.37 |

Whole ladder, 50 fights: **30.6 ms** (owner), 21.9 ms (friend).
`Run::combat_items()` - loadout to profiles - is **0.44 ms** on the owner's
board, which is the same order as a mid-ladder fight and cannot be ignored in
a packer's inner loop. Debug is 10-20x slower (whole ladder 295 ms). The
existing packer, `pack_francis::pack`, packs Cog Priest in **39.5 s** release
at the default 300 trials (4,800 fights scored against four boards at four
settings, plus seating), which is ~130 ms a candidate - so most of a
candidate's cost is not the fight. `gui/src/pack.rs:5` says "about five
minutes per creature per power band", which is the owner's machine, or debug,
or both - **not reconciled**.

## 6. Which pins were re-pinned, and on what

| Pin | Was | Is | Justification (as recorded) |
|---|---|---|---|
| `ACTIVATIONS_PER_S` (`rating.rs:497`) | 2 | **5** | Boards report 2.06, 3.43, 6.60/s; 5 is the mean of the two finished human boards (M16) |
| `pool_weight(Insight)` | fuel | **held** | Nothing spends Insight; it multiplies Dread for the fight (M16) |
| Dearest piece | 250 g ceiling | **220 g** | Every rating is a fraction of its slot's ceiling and the ceilings rose; 252 → 227 measured (M16) |
| Stepped boards moved by the catalogue landing | 29 of 162 | **11** | `stepped_component` gained `is_event_only` and `touches_insight` filters (`combat.rs:316-332`); the eleven are the old `Gold Chip`/`Crownwright's Measure` leak closing (M9) |
| Cracked Lens | 20 mind | **12** | Out-rated boss gear (M9) |
| Owner's ladder | 50/50 | **48/50** | A1 took a magic multiplier off iron; the two rungs are Nine of Ashes and Francis (M1) |
| Weapon share | 75.2% | **75.5%** | Same commit (M1) |
| THE UNWOUND | 10,000 / 230 | **15,000 / 345** | Two of three boards lose; the third wins at 28.0 s, inside 16-29 s (M17) |
| Fifteen frames' stats | placeholders | ladder creature at band | "What a band means" (M17) |
| The Iron Warden, rung 7 | `items [3,2,2,1]` | `[3,2,2]` | **Unintended.** `make pack`'s save rewrote a creature nobody was editing; nothing pinned rung 7 (M15). Never re-pinned on purpose |
| THE SHADOW | no weapon, no mind | + Antechamber Crown, Third Eye | Landed nothing at Medium (`346d9df`, post-merge) |

Three pins that did **not** move and must not: `RARE_AT = 90`, `EPIC_AT =
130`, `LEGENDARY_AT = 170` (`rating.rs:230`); `SUDDEN_DEATH_MS = 30_000`
(`combat.rs:40`); the four-board table at Medium since M16.

## 7. Which spec amendments the code earned

Numbered 9-23 in `design/the-unwinding.md` and all still true at the tip.
Four more the eight post-merge commits earned and nobody has written into the
spec yet:

- **#24 (proposed).** THE BIGGER SIGN stands at rung 13. The body and #22's
  table put it at 41.
- **#25 (proposed).** THE PICKET LINE's window opens at 20. THE FOUNDRY
  REMEMBERS counts one melt. `tests/completable.rs` is the general rule:
  every door's key must be able to exist before the door's window shuts, and
  `Trigger::from` returning 0 for `Rung` is not "the earliest the door can be
  met".
- **#26 (proposed).** `Event::Watched` is emitted on every sighting with
  `seen`, `count`, `paid` and the relation; the interface counts from the log.
- **#27 (proposed).** Hollow is Helmet + Chest. `monster-themes.md` §7's table
  is wrong and §1's is right.

And one against `gear-slot-basis-rewrite.md`, unchanged since the sweep and
still unresolved: greaves are quota'd to bleed *reserve* into a chest they
cannot reach through any recipe (§2's amendment). It is a design question, not
a sweep, and it is still open.

## 8. The eight post-merge commits, since no ledger has them

| Commit | What | Suite |
|---|---|---|
| `b395fd6` | "M for the road" hint on the opponent panel; two themed slugs | 764 |
| `346d9df` | Watcher counter reads the log; **the GUI's `cfg(test)` module had not compiled since M7** and 54 tests were not running; THE SHADOW gets mind pressure | 765, +54 GUI |
| `c99c261` | THE BIGGER SIGN 41 → 13; `whispered_event` returns all words on a rung, not one; receipts say what a choice opened | 769 |
| `3d005d5` | `hidden_towns::a_run_that_plugs_its_ears_can_walk_into_extra_large` walks the road rather than checking a flag; `opened_by_taking` reverse index | 770 |
| `e53a50b` | `tests/completable.rs`: four audits of "can this key exist in time"; two more doors fixed | 774 |
| `b1b54fe` | Watcher prose says whose activations; `Watched::counted` | 774 |
| `fbafc19` | Hover highlights all five relations the catalogue speaks (`Reads`) | 774, 56 GUI |
| `18d1b85` | Glossary as chips; `chip_rects` tested without a font context | 774, 60 GUI |

Two of these found bugs that a green suite of 764 had been hiding, and both
were the same shape: **a test that checks a thing exists is not a test that
the thing can happen.** `completable.rs` is the generalisation and it is
worth extending before the next door is authored.

## 9. Where the documents and the code disagree

The bug reports, so the next reader does not rediscover them.

| Document says | Code says |
|---|---|
| `CLAUDE.md` intro: "Nothing in it has been executed" | §6 of the same file: "finished and merged". The intro is a fossil from M0 |
| `CLAUDE.md` §3: piece.rs ~9,600, combat.rs ~5,350, run.rs ~2,060, event.rs ~760 | 10,636 / 6,413 / 3,692 / 2,352. `bestiary.rs`, `route.rs`, `relic.rs`, `pedestal.rs` absent from the table |
| `CLAUDE.md` §6, `HANDOFF-unwinding.md`: 46 test binaries | 49 files in `tests/` plus `common/` |
| `HANDOFF.md` §4: "`EVENTS` is a static holding promoted arrays" | `pub const EVENTS` (`event.rs:590`). Which is exactly why a caller in another crate held a *copy* and `ptr::eq` failed |
| `Cargo.toml`: `rust-version = "1.75"` | Needs ≥ 1.83 |
| `HANDOFF.md`: 764 green, no warnings | 776 green, 2 warnings (1.75) |
| `HANDOFF-unwinding.md` table: M19 "next" | Merged; `HANDOFF.md` exists |
| `monster-themes.md` §7: Hollow leans on Helmet, Gloves | `bestiary.rs:130`: Helmet, Chest |
| `monster-themes.md` §5.5: "the four in `ALTERNATES`" | 19 |
| `the-unwinding.md` #22 table: THE BIGGER SIGN at 41 | 13 |
| `CLAUDE.md` §1 lists `begin_fight` among the things the engine does | `Run::begin_fight` (`run.rs:3415`) fights `RUST_GOLEM` regardless of rung - "ladder position ignored". The road's fight is `fight_next` (`:3380`) |
| `Makefile` `## test:` "59 tests" | 776 |
| `README.md` | not audited here |

## 10. Things that will bite you

1. **The toolchain.** `Cargo.toml` says 1.75; you need 1.83. Fix the
   declaration or the code, and say which.
2. **A green suite can hide a door that cannot open in time.** Three found
   post-merge, all after "the road holds everything the mission promised"
   passed. `completable.rs` catches the shapes it knows; a new requirement
   kind needs a new row there.
3. **`EVENTS` is a `const`.** Every reference is a fresh copy. Compare
   choices by value; never by address; never assume two `&Choice` from
   different crates point at one thing.
4. **The GUI's tests do not run under `cargo build`.** Run `cargo test -p
   gearmaster-gui` or a fixture will rot silently, as it did for eight
   milestones.
5. **`make pack`'s save rewrites `combat.rs` in place** and once rewrote a
   creature nobody was editing. Read the diff before committing a save.
6. **"Completable" is proved by `force_win`.**
   `chain::the_chain_can_be_finished_in_one_run_in_either_mode`
   (`tests/chain.rs:275-308`) assigns `run.rung` and wins fights by fiat. It
   proves the road graph, not that any reachable build can fight it. So does
   every walk in `progression.rs` that uses `skip_to` (25 call sites). That
   is the gap `design/rl-agent-plan.md` exists to close.
7. **Both no-weapon clears of rung 15 are the clock's.** Anything that calls
   a sudden-death victory a "clear" is measuring `SUDDEN_DEATH_MS`.
8. **`begin_fight` is not the next fight.** It is a fixture against Rust
   Golem.
9. Everything in `CLAUDE.md` §6's trap list, which is re-derived there from
   this tip.

## 11. Habits that paid

- **Walk the run in.** `3d005d5`'s first attempt at proving the sign fix
  failed on its own error, and that was the point: asserting a fix without
  walking the road is the mistake twice.
- **Ask "can it happen in time", not "does it exist".** `completable.rs`.
- **A log entry stores a relation, not a sentence.** `Event::Watched`
  carries `Watched`, and the wording is decided by whoever draws it.
- **Read the printers after every merge and write the numbers down beside
  the commit hash.** This file is the first one that says which commit its
  numbers came from. Keep doing that.
- Everything in `HANDOFF.md` §5.
