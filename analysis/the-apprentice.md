# THE APPRENTICE — measurements

One block a milestone, headed by the commit it was read off. The spec is
`design/the-apprentice.md`; this file is what the machine said.

Every timing in this file was taken on the same machine, in `--release`, on
mains power, with nothing else running: **Apple M2 Max, 12 cores (8
performance, 4 efficiency), 32 GB unified, macOS 26.3, rustc 1.95.0**. Thread
counts are written beside every throughput figure, because four of this
machine's cores are three to four times slower than the other eight and a
number without a thread count is not a measurement.

---

# A0 — The ground

Read off **`020bc7c`** (2026-08-28), the tip that published THE HUNDRED.
No code entered the repo for this milestone. The harness that took the timings
is a scratch crate outside the workspace with a path dependency on the engine,
so that measuring did not add a fifty-eighth binary to a suite that relinks all
fifty-seven on every engine edit.

## A0.1 The suite, as it stands

| | Count |
|---|---|
| `cargo test -p gearmaster-engine` | **1,043 passed, 0 failed, 51 ignored**, 57 integration binaries + lib + doc-tests, 49.7 s |
| `cargo test -p gearmaster-gui` | **81 passed** |
| `cargo test -p gearmaster-cli` | **10 passed** (0 in the lib, 10 in `replay.rs`) |
| Workspace total | **1,134 passed** |
| `cargo build --workspace` | **no warnings**, rustc 1.95.0 |

`Cargo.toml` declares `rust-version = "1.83"` and the code needs 1.83.
**D1 is closed**: the declaration is honest and the workspace builds and tests
warning-free twelve minor versions later.

**Six counts in `CLAUDE.md` are stale.** None of them is a bug; all of them are
denominators this mission depends on, so they are corrected here and the
correction belongs in `CLAUDE.md` when this mission writes its record.

| | `CLAUDE.md` says | Read off `020bc7c` |
|---|---|---|
| engine tests | 1,035 | **1,043** |
| CLI tests | 9 | **10** |
| `ALTERNATES` | 28 | **33** (THE HUNDRED's five county creatures) |
| creatures, all tables | 78 | **83** (50 + 33 + 0) |
| `relic::RELICS` | 4 | **3** |
| town door kinds | 17 | **18** (`Action::EVERY`; `Action::ALL` is the original four) |
| `Brawl` statics | 6 | **5** (`TABLE_THREE`, `THE_BACK_ROOM`, `THE_SHOWFIGHTERS`, `THE_FLOCK`, `THE_HERALD`) |

## A0.2 Oracle throughput

The board under test is the owner's, decoded from `share::A_WINNING_RUN`: 75
pieces placed, **19 assembled items**. Reconstruction verified against the
repo's own claim — the board clears **48/50** at Medium, which is the number
`CLAUDE.md` §5 quotes, so the trap-4 reconstruction fault did not happen here.
Of those 48 victories, **44 are board-decided** (under `SUDDEN_DEATH_MS`) and
four are the clock's.

One core, median of 200 calls:

| Fight | Cost | Rate | The fight itself |
|---|---|---|---|
| rung 1, Cave Rat | **0.014 ms** | 70,175 /s | 1.50 s |
| rung 25, Cog Priest | **0.453 ms** | 2,208 /s | 14.00 s |
| rung 50, Francis | **0.768 ms** | 1,302 /s | 12.85 s |

The pieces of a candidate score, one core:

| | Cost | Rate |
|---|---|---|
| `Figures::of()` — **S0**, no fight | **0.000042 ms** | **23,809,523 /s** |
| `total_stats()` | 0.118 ms | 8,477 /s |
| `combat_items()` on 19 items | 0.271 ms | 3,694 /s |
| A whole 50-rung ladder | 18.9 ms | 52.8 ladders/s |
| 4 fights (one board × four difficulties) | 1.283 ms | 779 /s |
| 16 fights (the four-board gate), extrapolated | 5.133 ms | 194 /s |

Many threads, whole ladders:

| Threads | Ladders/s | Fights/s |
|---|---|---|
| 1 | 52.8 | 2,640 |
| 8 (performance only) | **337.0** | **16,848** |
| 12 (all cores) | 391.0 | 19,551 |

**Four efficiency cores buy 16%.** Eight threads is 6.4× one thread; twelve is
7.4×. Every throughput number in this mission is therefore reported at eight
unless it says otherwise, and a "per-core" figure taken across twelve is a
lie by about a fifth.

**A0's gate is met, in the good direction.** The plan required fight throughput
within 5× of the container's figures (`design/post-unwinding.md` §5). This
machine is **1.6× to 2.1× faster** on every one of them: 0.014 ms against 0.03
at rung 1, 0.768 against 1.4 at rung 50, 18.9 ms against 31 for a ladder,
0.271 ms against 0.44 for `combat_items`.

**S0 is free, and that is the finding the packer's redesign rests on.** A
board's six figures cost **42 nanoseconds** — about one seventeen-thousandth of
a rung-50 fight, and one hundred-and-twenty-thousandth of the four-board gate.
A search that rejects a candidate on `Figures` pays nothing for the rejection.
The spec's estimate was ~2 µs; the truth is fifty times better than that.

## A0.3 The packer, as it stands

`pack_francis`, `--release`, 300 trials, single-threaded, this machine.
The command is `PACK_MONSTER="..." cargo test --release -p gearmaster-engine
--test pack_francis pack -- --ignored --nocapture --exact`.

| Creature | Rung | Pieces packed | Wall-clock | Result |
|---|---|---|---|---|
| Bone Archer | 3 | 4 | **8.0 s** | landed, 1.5 s against a 2.0 s target — **25% off, in a 30% band** |
| Cog Priest | 25 | 21 | **59.4 s** | landed, 9.0 s against a 9.35 s target — 3.7% off |
| Francis | 50 | — | **242.5 s** | **failed**: *"nothing landed on the curve for rung 50: wanted 21.6s within 30%, best was a loss"* |

Three things this says, and the third is the mission's justification.

1. **The quoted 39.5 s is neither the best nor the worst case.** Cost scales
   with the piece budget — four pieces cost 8 s, twenty-one cost 59 s,
   forty-four cost four minutes — so "seconds a creature" is only meaningful
   beside a rung. A3 reports the same three creatures.
2. **This machine is faster per fight and slower per creature than the
   container was.** 59.4 s for Cog Priest against a quoted 39.5 s, on hardware
   that fights 1.7× faster. The candidate count did not change; what changed is
   the catalogue and the boards it draws from — `CATALOG` is 518 pieces now.
   The sampler pays for content growth linearly and gets nothing back for it.
3. **The sampler cannot author the final boss.** Three hundred candidates, four
   minutes, and not one of them produced a Francis the owner's reference board
   beats inside a 30% band around 21.6 s. The best of three hundred was a
   *loss*. This is the measured form of "does not work well", and it is the
   number A3 has to beat: not a better time on a creature the sampler can
   already do, but a board on the one it cannot.

The early-rung result is worth its own line. Bone Archer landed at 25% off a
2.0 s target inside a 30% band — it is the edge of the band, not the middle,
and `pack_francis`'s own doc comment says the early rungs are the hard ones
because four pieces of themed gear is a narrow target. A3's gate should read
*closer to `target_ms`*, not merely *inside the band*.

## A0.4 The census — the DCR denominator

Counted off the tables at `020bc7c`. This is what "every event in the game is
accessible" has to be measured against, and it is written down here so that a
later coverage figure cannot quietly change its own denominator.

| Content | Count | Choices / parts |
|---|---|---|
| `EVENTS` | **44** doors | **102** choices |
| `COUNTY_EVENTS` | **9** doors | **18** choices |
| `Brawl` statics | **5** | — |
| `TOWNS` | **6** towns | **32** doors offered, out of **18** door kinds |
| `DUNGEONS` | **7** | **23** floors, **16** exits (floors are a graph, not a list) |
| Pedestal destinations | **7** | — |
| `RUMOURS` | **11** | — |
| `CLASSES` | **31** | — |
| `RELICS` | **3** | — |
| County `TOLLS` | **12** thresholds | 6 toll kinds |
| County `MOUTHS` / `CIRCUIT` | **6** / **16** | — |
| `LADDER` / `ALTERNATES` / `CREVICE` | **50** / **33** / **0** | 83 creatures |
| `MonsterTheme::ALL` / `FRAMES` | **10** / **29** | — |
| `CATALOG` | **518** pieces | — |

**Offered / answered / branched** are counted against these numbers as follows:
a door is *offered* when it reaches the road stack, *answered* when one of its
choices is taken, and *branched* only when every one of its choices has been
taken by some run somewhere in the evaluation. The branched denominator for the
road is therefore **120 choices** across **53 doors**, not 53.

Three of the game's four `Outcome::Count` counters are written by a choice and
read by no door at all (`completable.rs`'s `COUNTERS_NOBODY_READS = 3`:
`shook-the-machine`, `moles-paid`, `crossed`). They are content with no door
rather than content the agent failed to reach, and the ledger must say so
rather than counting them as misses.

## A0.5 The seed set

128 seeds, fixed. The four the repo already uses go in the training half so the
held-out half is clean; the remaining 124 are successive draws from
`Rng::new(0x501_7E5)`, taken in order. **Written out here so nobody
regenerates them, and no seed ever moves across the split.**

Training half (64) — the four the repo already uses:

    0x5EED1234ABCD0001   (Run::new)
    0x0000000000006060   (acceptance::a_run)
    0x0000000000001111   (acceptance::a_run)
    0x0000000000001212   (acceptance::a_run)

and the first 60 draws:

    0x8F0290779940D189 0x461F77D8B7E9EC1A 0x5CB1FFC46DE1C3D5 0xC434E4A68C5906EE
    0xE5D530F73DE5618D 0x15AD0469B38FEC99 0x81B52DE9967736C4 0x4D7B4200CADAE7F6
    0x6C68B99BC7BBE570 0xFF5B87E43F02FCFB 0xCD20ACA2E114D294 0xD16A863E0F07B343
    0xC631FB380931F96F 0xDDE36F5B2BE01EDA 0x218BDDB920FF47C1 0xAB57EAD39B41C9F5
    0x745C9DC1269274A6 0xAD1E289B7C058A2C 0x870BD396A455CDA7 0xB952310BE79F48AD
    0x6C41960722EFA025 0x533DB2FF156E63DF 0x6EAA8C8936E880C5 0x4C61A66F5A606D9C
    0xAA821F8EAF8B9D0F 0x51763883117845D6 0x5A6238406532A973 0xAC22B617D0E20CB0
    0x5FE20A55CC91BD7B 0x6511EC93D994087F 0x88C62CEBD04F1B9F 0x9C10EFB79FA32CD4
    0x88DD53F763E45EF0 0xF539B1772C4A921E 0xD066DCB8A09CA34F 0x67375586989FAC13
    0x11C0027CF5A1B281 0x77AFD28C2B0E77BF 0x70217B9484D5D160 0x6A25F2E67E33DC67
    0xD45F3D225899BFA6 0x4C369AEC363A535D 0x7BB9FD011EBFAB9C 0x7C35AA266B3D8B74
    0x65EF795FF701AEF1 0x735ABF3E011958AE 0xB0121BCA8D695D29 0x7AD5947440A9517E
    0x6F41FE3F8265EF76 0x096CF6A5BCAC3798 0xA302AFA7EEF652A3 0x5ACF9F35118F4DE0
    0x77D4ED1D083BF903 0x98440774A38248C2 0x41DC15AABF550377 0xF2C27BCACCB9AEED
    0x6F29541386DCC988 0x7E8EA5C0CDF81751 0xD1F5A73E55A31693 0xDFE9AEADB48FFE58

Held-out half (64) — draws 61 to 124:

    0x83813B848606060E 0xDE383E5E89FE1370 0xD56302770B9289A3 0x7A7AAC28E776C575
    0x167B82145397C297 0x49CD4333544808A8 0x998A1AB80D462356 0x241EAE43EB8EDA72
    0xCBBC613A9483E803 0x17D487C4379072DC 0xAE0D9FA32C20EBD6 0x7F310E71D4D90D02
    0xAD6E1CC5B1B9F242 0x77E92FF913F5F29A 0x564BB6E31BA1CDA0 0x08146AC27F2325A9
    0xBF32E8F543643CD0 0x51034F8D679CFC08 0x78D892F0C95D8F9C 0x166FD767E28DEFEC
    0xABC7427FAACD6DDA 0x72D227794E3C7A7E 0x67C5EF67815E8057 0x233DF3CFAFA758F0
    0x63AB6A550999E2EB 0x860306184F9349B6 0x63F6655D3AD244CB 0x15DCAFDFC2B880FB
    0xD6277939A416F7E8 0x6C39E8F524C70318 0x49FA34E8513A2948 0x86E1D2B234E6F197
    0x98BC6DB3931FD9BE 0xE355E253F0A73BD2 0x174BE8A6C669AF88 0xA28B6DDB4B7555A6
    0x4BC9640CE6241092 0x3197624F20CE8DD1 0xF186A1EA3093A401 0x738BC3CFAE530133
    0x0331DAF586D2C69A 0x500077366122F8FC 0xF020D1708F9F30C5 0x48873AEB61F8E7FA
    0xC0D142E72628358D 0xC8826F3F99A466BE 0x01F7F634F26976F3 0x5F6102DCB3BA693E
    0x2FB05FE5D3E31C4D 0x343B4FD37A7A3878 0x0411BFA6070BD05E 0xAD40C358358E9F66
    0x1AE9C76C704BCC9E 0x3F8334857DEDBE47 0xC3705D1013310E7C 0xACD82AC771E3B474
    0xE28C8B8375F7D136 0x8B125D231C405924 0xC84866AB6A6E5331 0x3D2CCE9A5D2CF3B8
    0xAE2B77AAFE0678B0 0xF16FC5F9471D9E8F 0x562C66F2648E1A99 0x581BB69E5C0F1A7C

## A0.6 What A0 changed, and what it did not

Nothing entered the repo but this file and the spec beside it. No engine
change, no new test binary, no new dependency. `Run` still has no `Clone`
derive (`run.rs:611`) and E5 is still open.

**What A0 hands the next milestone:**

- The oracle is fast enough that nothing in this plan is priced wrongly, and
  S0 is so nearly free that the tiering in §5 of the spec is not an
  optimisation, it is the only sensible shape.
- The packer's bar is three numbers and one failure, not one number.
- The coverage denominator exists, and six of `CLAUDE.md`'s counts were wrong
  before anybody counted them for a metric.


---

# A1 — The console, and the CLI made whole

Read off **`020bc7c`** plus this milestone's own changes. Two new crates, one
extended driver, no engine change of any kind - `cargo test -p
gearmaster-engine` is **1,043 passed, 0 failed, 51 ignored**, byte-identical to
A0's run, and the workspace still builds with **no warnings**.

## A1.1 What landed

| Crate | What it is |
|---|---|
| `crates/console` | `gearmaster-console`. `Verb` (35 variants), `Console::menu`, `Console::apply`, `Console::view`, `Console::screen`, transcripts. Depends on the engine |
| `crates/agent` | `gearmaster-agent`. The pilot. **Depends on `gearmaster-console` and nothing else** |
| `crates/cli` | Sixteen new verbs, so that every console verb is one a person can type |

The boundary is a fact about the dependency graph rather than a promise:
`cargo tree -p gearmaster-agent` names the console and, under it, the engine -
and no path from the pilot's own manifest can reach `simulate_party`,
`CATALOG`, `LADDER` or `skip_to`, because Rust will not resolve a `use` for a
crate that is not a dependency. `crates/agent/tests/boundary.rs` asserts the
manifest, the source and the forbidden list.

| Suite | Tests | Time |
|---|---|---|
| `gearmaster-console` | **16** (legality 2, transcript 4, parity 5, view 5) | 0.5 s |
| `gearmaster-agent` | **5** (boundary 3, starter 2) | 0.6 s |
| `gearmaster-cli` | **12**, up from 10 | 3.2 s |

## A1.2 The numbers

Legal verbs over **1,080 fuzzed reachable states**, debug: **min 17, median
57, max 419**. Over four seeds in release, sampling 1,600 presses: min 18,
median 53, max 545.

Cost of one step on this machine, release, one core, medians:

| | Cost |
|---|---|
| `menu()` - enumerate every legal verb | **28.9 µs** |
| `view()` - build the whole screen | **50.9 µs** |
| `apply()`, no fight | **1.6 µs** |
| `apply()`, a fight | **220 µs** |

A press is not the cost; enumerating and reading the screen is, by about
fifty to one. That matters for A4: an agent that calls `menu()` once per
decision and `view()` once per decision spends 80 µs a step, so a whole
fifty-rung run costs well under a second of *interface* time and the fights
are the budget. The plan's estimate of "≤ 50 minutes a seed" is priced for a
search on top of this, not for the console.

## A1.3 Three findings, all from lints, none from a failing feature

### Four player-facing verbs had no interface at all

`tests/parity.rs` walks the two shipped drivers' source for the `Run` methods
they call, walks `run.rs` for the methods that take `&mut self`, and compares.
Four mutators are tested, documented, and reachable from **nothing a person
can drive**:

| | What it does | Who had it |
|---|---|---|
| `clear_slot` | empty one grid | nobody - the CLI has `clear` (all five) and the window has no button |
| `crush` | break a relic for what is inside it | **nobody at all**, in either driver, ever |
| `grow_slot` | spend an owed row on one board | nobody - only `grow_boards` (all five) is reachable, from the window's debug menu |
| `walk_the_perambulation` | **THE PARISH's tenth trip** | nobody - `tests/hundred.rs` is the only thing that has ever walked one |

The fourth is the one that matters. B5's perambulation is a route rather than
a destination and it is the last journey of THE PARISH;
`design/HANDOFF-hundred.md` §3 records that the chain finishes on no simulated
census and gives three dials for it. There is a fourth reason, and it is not a
dial: **no interface could walk it.** The CLI has `perambulate <x> <y>` now,
and `mouths` to find out where to start.

The window still has no button for any of the four. That is recorded, not
fixed - it is interface work with a layout question attached to each - and
`the_window_still_has_no_button_for_the_four` fails the day one arrives.

### The shop shows a price it will not charge

`tests/view.rs` holds one rule: every accessor the view reads is one an
interface also reads. It refused exactly one, `Run::price`, and the reason is a
bug rather than an overreach.

- `Run::price(slot)` is `shop.price(slot) + markup%` (`run.rs:3299`) and
  `Run::buy` charges precisely that (`:3308`).
- Both interfaces draw `rating::shop_price(def)` instead - the catalogue's
  number, before the markup (`gui/src/main.rs:3720, :3828, :4315`, and the
  CLI's shelf list).
- THE TOLLBOOTH answers with `Outcome::Markup(10)` (`event.rs:2499`) and its
  receipt says *"Every shelf costs 10% more"*.

So from the moment that door is answered, **every shelf in the game displays a
number it will not honour**, and the only place the true price appears is the
gold that leaves your purse. The console shows what will be charged; the CLI
was fixed with it; the window's three call sites draw from a `PieceDef` with no
shelf index in scope and are recorded for the owner.

### A verb the game does not have

The console was written with an `Enter` verb for walking into a dungeon.
`every_verb_presses_something_an_interface_also_presses` refused it: nothing in
either interface calls `Run::enter_dungeon`, because **a run never walks into a
dungeon** - it is put in one by answering a door, by a town's cellar, or by
feeding an orb to a pedestal. The verb is gone and the reason is in
`verb.rs`. This is the lint working in the direction that matters most for
this mission: it stops an agent from having an action a person does not.

## A1.4 The leak audit, and its answer

`design/the-apprentice.md` §4 named `Run::monster()` as the one known leak: it
hands back the coming creature's whole spec, gear included, and the spec said a
player standing on rung 30 does not see rung 30's board before the fight.

**The spec was wrong.** `gui/src/main.rs:4768` draws "WHAT THEY BRING" - every
item the creature will swing - and `:4803` draws its whole board, under a
comment saying the panel is a preview that exists so you can shop against what
is coming and that showing half of it would defeat the point. The `View`
carries the creature's stats, its items and its innate attacks, because that is
what is on the screen.

What the view does **not** carry is the rest of the ladder, and there the two
interfaces disagree: the CLI's `ladder` verb prints every creature's outfit at
every rung and the window shows only the next one. The console takes the
window's answer, on the ground that telling an agent *less* than a player knows
can only make a reachability claim stronger. It is a decision rather than an
oversight, and `the_view_does_not_carry_the_rest_of_the_ladder` is the negative
test that keeps it one.

## A1.5 Two design calls worth knowing about

**Rotation is a verb, not a coordinate.** The plan's action encoding is
`{tray, slot, x, y, rot}`. A player has no such action: they turn the piece in
their hand, then put it down, which is two presses. `Place` carries no
rotation. The search may still reach all four - it costs one extra step,
exactly as it costs a person one.

**A press has three outcomes, not two.** `ok: false` is "not a legal thing to
press here" and means the menu has a bug. `ok: true, changed: false` is
"pressed, and nothing moved" - turning a piece with nowhere to turn to, which
the interface lets you do. Without the third state the legality fuzz failed on
a rotation that would not fit, which is not an illegal action.

## A1.6 The control, played

`starter` is the first thing in this repo's history to play a run forward
through a player's own actions. It seats what will sit - trying each anchor,
keeping the one that finishes an item, taking the others back with the
player's own `Undo` - and then fights until it stops climbing.

    seed 0x5EED1234ABCD0001, Grinder, Medium:
      rung 2, 6 board clears, 6 game clears, 145 presses, "stuck below its ceiling"

Two pieces of gear beat the Cave Rat in 4.5 s and lose to the Bog Toad in
7.2 s, over and over, because a Grinder knocks back on a loss and there is
always an easier fight to farm. That is the game working exactly as designed
and the control working exactly as designed: it is the zero every later
milestone is measured against.

**SCR(R10, starter) = 0**, as the plan predicted, and now for the first time
the number was produced by playing rather than by argument.

---

# Interlude — the card mission landed on top of this one

Read off **`1cda004`**. Twelve commits (T0 to T6 plus the deploy) arrived
between A1 and A2: the county freeze, the item card grouped by *when* its
figures happen, one spelling for a pool grant, and a glossary shelf. They
touched `stats.rs`, `piece.rs`, `rating.rs`, the GUI's cards and the CLI's
shop listing - all of which A0 measured and A1 reads.

**What survived unchanged**, re-taken at the new tip:

| | A0, at `020bc7c` | Now, at `1cda004` |
|---|---|---|
| a rung-50 fight | 0.768 ms | **0.768 ms** |
| `Figures::of()` (S0) | 42 ns | **42 ns** |
| `combat_items()` on 19 items | 0.271 ms | **0.271 ms** |
| a whole ladder, one core | 18.9 ms | **18.8 ms** |
| eight threads | 16,848 fights/s | **17,130 fights/s** |
| owner's board up the ladder | 48/50, 44 board-decided | **48/50, 44 board-decided** |
| the census, every row | - | **identical** |
| `pack_francis`, Bone Archer | 8.0 s, 21/240 cells | **9.0 s, 21/240 cells** |
| `pack_francis`, Cog Priest | 59.4 s, 69/240 cells | **60.8 s, 69/240 cells** |
| `pack_francis`, Francis | 242.5 s, **failed** | **243.3 s, failed identically** |

Ratings moved - `gear_at.txt` was re-baselined at 6,744 placements - and not
one of these did. That is worth having: it says the packer's bar is a
property of the search rather than of the curve underneath it, so A3's target
does not move when the curve does.

**What moved:** the suite. Engine **1,053 passed, 52 ignored** (was 1,043 and
51); GUI **83** (was 81); CLI **12** (A1's own). Everything else in A0 stands.

## Two amendments to A1, both from the new code

### Any verb can start a fight, and the console did not know it

`f4354ec` fixed a freeze **reported from play**: walking onto a pinnacle in
THE HUNDRED calls `begin_county_fight`, which simulates the whole bout and
leaves `Phase::Fighting` with a log waiting - and `county_walk` and
`leave_county` both refuse outside `Loadout`, so every control died at once,
including the way out.

A1's console had the same fault in its own shape: `apply(Walk)` returned
happily, `menu()` then returned empty because the phase was `Fighting`, and
the pilot would have stopped with "nothing left to press" on the one tile
where a chain ends. It settles any fight wherever one appears now, rather
than inside the two verbs that were known to start one.

The assertion is written as the general fault, which is what that commit's own
gate does: **after any press, there is something to press, or the run is
over.** It runs on every one of the fuzz's 1,080 states.

### The card's four groups are player-visible now, so the view carries them

`Stats::parts_when` classifies every stat field as `Passive`, `OnActivation`
or `Damage`, checked against the fight rather than hand-written. Eight of the
twenty are handed over on every activation. The card draws them in groups; the
CLI's shop listing uses `summary_by_when`; and the `View`'s tray and shelf
pieces carry the same grouping, so an agent does not have to guess which of
`+2 nature` and `+8 curse res` is a rate.

This is not cosmetic for this mission. **S0 - the fight-free surrogate the
whole packer redesign rests on - has to weight a per-activation figure by the
item's cadence and a passive figure once**, and until T3 there was no engine
answer to which was which. `design/the-apprentice.md` §5 is amended to say so.

The same commit fixed a fault in the figures themselves: `Figures::of` reads
`stats.mana` and nothing else, and eighteen pieces granted mana through a
trigger - invisible to every county toll asking what a board makes a second.
The preset crosses six of the twelve thresholds now rather than five. **S0
measures more of what a board does than it did when A0 measured it**, at the
same 42 ns.

## What did not need updating

A2 to A9 stand as written. The theme meter, the tiered objective, the
expert-iteration design and the coverage ledger are all untouched by a mission
about how a card is laid out - and the one thing that could have moved them,
the rating curve, moved without moving a single number the packer is measured
by.

---

# A2 — The oracle and the theme meter

Read off **`1cda004`** plus this milestone. `crates/oracle`, **19 tests**, and
no engine change. The workspace: engine 1,053 / 0 / 52, GUI 83, CLI 12,
console 16, oracle 19, agent 5 - **1,188 green, no warnings**.

## A2.1 What the three tiers actually cost

A0 timed the engine's primitives. This times what a search pays, which is not
the same thing. One performance core, release, medians:

| Tier | | Cost | Against a fight |
|---|---|---|---|
| **S0** | the surrogate, board already built | **584 ns** | **411× cheaper** |
| **S0** | the surrogate, **rebuilding the board first** | **1,444,750 ns** | **6× dearer** |
| **S1** | one fight | 240,458 ns | - |
| **S1** | one fight, cached | 1,292 ns | 186× cheaper |
| **S2** | the sixteen-fight acceptance gate | 3,898,875 ns | 16× dearer |

Eight threads run **1,503 gates a second** - 24,054 fights - and twelve run
1,760, which is the efficiency cores buying 17% again.

**The tiering is real, and it has a condition nobody had written down: the
board has to stay built.** Rebuilding a seventy-five-piece board from its
placements costs 1.44 ms, which is two and a half thousand S0 reads and six
whole fights. A search that scores a candidate by rebuilding it has already
spent more than the fight it was avoiding.

So A3's local search mutates a live board - place one piece, re-lock the
touched slot, read the figures - and never reconstructs from a placement list
except at the boundaries. That is now a requirement rather than an
optimisation, and it is the second structural difference from the incumbent
packer: the first is that it improves rather than resamples.

**S0 was twenty-four times slower before it was fixed**, and the reason is
worth keeping. `Stats::parts_when` is the engine's own classification of which
figures are handed over on every activation - but it returns each one
*formatted into a `String`*, so summing a group by parsing them back cost
13,959 ns a board. The classification is constant per field, so it is asked
once: twenty-one probes at startup, each setting one field in an empty block
and reading back which group the engine puts it in. A table of *fields* is
kept; a table of classifications is not, which is the discipline T3 built
`parts_when` with in the first place. `tests/s0.rs` fails if a
twenty-second field is ever added to `Stats` and the list does not hear
about it.

## A2.2 The cache

**10,880 lookups, 272 misses, 97.5% hit rate**, and every one of the 10,880
compared against what the engine says uncached. 272 distinct fights - twenty
creatures by four reference boards by four difficulties - each asked for forty
times, which is the shape a local search has rather than a shape chosen to
flatter the number.

The key is `(player board, creature, difficulty, purse bucket)`. A board's key
is over its **sorted** placements, so an ordering is not a board and a moved
piece is: `two_boards_that_differ_are_not_one_key` holds both halves.

## A2.3 The gate, ported

Every constant is read out of `tests/pack_francis.rs` at test time and
compared - `FLOOR_MS`, `FLAT_UNTIL`, `CASINO_BAR_MS`, `BAND` - so a port that
drifts from its original fails rather than diverging quietly. The curve is
checked rung by rung, the flat window's wider band at the boundary, the preset
corridor in all four of its cases, and a loss as infinitely far from the line.

Then the port is pointed at a board the original has an opinion about. The
original's own output for Cog Priest reads

    board want W8.0s W14.0s W14.0s W14.0s got W8.0s W9.0s W9.0s W9.0s

and this port reads the same eight figures: the owner's board beats the
**shipped** Cog Priest in 14.0 s against a 9.35 s line - **0.497 off the
curve, against a band of 0.30** - and the candidate the search was proposing
sits at 9.0 s. So the shipped creature is half a band outside the line it is
supposed to sit on, which is why the packer was proposing a replacement, and
a port that *accepted* it would be a port that had lost the curve.

## A2.4 The theme meter, and what it found

Ten signatures, each computed from a `CombatLog`, each a ratio over the fight.
The table is `cargo run --release -p gearmaster-oracle --bin themes`.

### The yardstick cannot feel three of the ten themes

`CurseKind::landing_ms` clamps curse resistance to 100 and scales a curse's
duration by `(100 - resist)/100` (`curse.rs:137`). **At 100 it is total
immunity.** The owner's finished build carries **145** and the friend's
**135**.

So every curse either of them meets lands for zero milliseconds. Measured, on
six creatures across the two halves:

| Creature | four-piece board | preset | owner | friend |
|---|---|---|---|---|
| Salt Idol | 6 curses, 115 burn | 6, 155 | **0, 0** | **0, 0** |
| Ruin Hound | 10 curses, 228 burn | 10, 196 | **0, 0** | **0, 0** |
| Bone Cantor | 7 curses, 2 stuns | 7, 2 | **0, 0** | **0, 0** |
| Ember Wisp | 48 curses | 52 | **0** | **0** |
| Cog Priest | 42 curses | 74 | **0** | **0** |
| Obsidian Colossus | 99 curses | 123 | **0** | **0** |

The creatures are not the problem: they curse constantly against a board that
can be cursed. **The difficulty curve is read off a board that is immune to
them.** Three of the ten themes - Burner, Slower, Warden - speak mostly in
curses, and against the yardstick they are silent.

That is why the table is printed in two columns. **FELT** is scored against
the two boards a player might actually have at that rung; **CURVE** against
the two finished builds the line is read off.

| theme | n | felt | worst | best | curve | the claim |
|---|---:|---:|---:|---:|---:|---|
| Striker | 6 | 0.55 | 0.37 | 0.78 | 0.17 | fast and fragile |
| Wall | 7 | 0.54 | 0.18 | 0.67 | **0.07** | slow, heavy, hits back |
| Burner | 7 | **0.20** | 0.01 | 0.46 | **0.00** | kills on the clock |
| Slower | 8 | 0.63 | 0.25 | 0.94 | **0.24** | denies tempo |
| Drainer | 8 | 0.41 | 0.25 | 0.72 | 0.51 | starves a banked build |
| Caster | 8 | 0.53 | 0.50 | 0.61 | 0.50 | bursty and mana-gated |

Drainer and Caster barely move between the columns, because mind damage and
magic are not resisted to nothing. Wall falls from 0.54 to 0.07.

### The Burner cluster does not kill on the clock

Even where it is felt, Burner is the worst theme by a factor of two and a half
- 0.20 against the next-lowest 0.41. The ten that read least like themselves
are four Burners at the top:

    Bone Cantor      0.01   most of it burns 0.02 · rather than lands 0.00
    The Hollow King  0.03   most of it burns 0.06 · rather than lands 0.00
    Salt Idol        0.11   most of it burns 0.17 · rather than lands 0.00
    Pale Twin        0.15   most of it burns 0.24 · rather than lands 0.08

Their gear is not the problem either. Bone Cantor's Searing Cleaver applies the
curse twice and Ruin Hound carries four searing items. The blows simply dwarf
the burn: rungs 14 to 20 are packed as strikers that happen to carry a searing
word, and `MonsterTheme::allows` cannot tell the difference because it filters
the pool and never reads the fight.

### A meter reading is a property of a pair, not of a creature

Four Drainers score 0.25 with *"it takes what you banked 0.00"* - because the
four-piece board has banked nothing to take. A drain against an empty pool
reads as zero, and that is a fact about the board.

So fidelity is a property of a **(creature, board)** pair. Both columns are
printed and neither is the answer on its own; a search using this as its λ
term (A3) scores against a board that can both express and receive the theme,
and says which board it used.

### The meter's own trap, found by its own test

`a_fight_where_nothing_happened_scores_nothing` failed on the first run:
**Burner scored 0.50 on an empty fight.** Every theme has a negative half -
"rather than lands", "and does little else", "and does nothing clever" - and a
creature that does nothing at all satisfies every one of them for free. The
cheapest way to read as a Burner was to be inert.

That is `CLAUDE.md` §6 trap 29 in the exact form the handoff predicted this
mission would meet it, and it was in the meter before the meter had scored
anything. Two fixes: a negative claim about a share reads **zero** when there
is no damage to have a share of, and an empty fight bears out nothing at all.
Striker's curve column fell from 0.42 to 0.17 when it landed, and Burner's
from 0.04 to 0.00 - both had been inflated by free negatives.

## A2.5 What A2 hands A3

1. **Keep the board built.** The rebuild is the cost, not the fight.
2. **The gate is portable and pinned**, so a search can be judged by the same
   rule the incumbent is judged by, at the same numbers.
3. **λ has something to multiply.** Theme fidelity is a number now, and it
   comes with the warning that it is a number about a pair.
4. **Two balance findings the owner may want before A3 packs anything**: the
   curve is read off curse-immune boards, and the Burner cluster kills on the
   swing. Packing rungs 14-20 against the present yardstick would author seven
   more creatures whose theme the measurement cannot see.

---

# A3 — The pilot's hands

**Re-scoped by the owner.** A3 was "the packer: greedy plus local search,
benchmarked against `pack_francis`". The instruction is that the packer runs
only once there is a trained agent, and that its builds are then the source of
creature boards - so the oracle-scored creature packer is not built at all.
`pack_francis` stays the incumbent and its 243-second failure on Francis
stands as a recorded fact rather than something this milestone races.

What A3 is instead: **the board sense the pilot needs to play at all**, which
A4 wanted regardless. The A1 control seated the first thing that fitted and
reached rung 2.

The benchmark is unchanged, and it is the one that decides whether anything
above it means anything: *a packer that cannot recover what a person did with
the same pieces cannot be trusted to do better with different ones.*

## A3.1 Blind hands, privileged eyes

The pilot builds with two presses and no oracle: **put a piece down, read what
the slot says, take it back if it did not help.** `Undo` is a button in the
game and that is what makes it legal rather than clever.

What it reads is `Sense` - the same arithmetic the oracle's S0 does, computed
from the `View` rather than from a `Loadout`, because every figure is on a
screen: the six county figures, the character sheet, how many items assembled,
and - since the card rewrite - **which of a piece's figures are rates and which
are quantities**. Before T3 said *when* each figure happens, an agent scoring a
board off the screen would have had to guess, and guessing wrong prices a rate
as a quantity.

Two implementations of one idea drift, so `lab/tests/one_score.rs` holds them
to the same answer on all three reference boards, figure by figure. It is the
only place in the workspace that can see both.

The harness that runs the benchmark is privileged and the pilot in it is not,
and the split is made by **type**: `Console::standing_in` takes a `Run`, and
`Run` is an engine type, so only a crate that depends on the engine can stand a
run in front of the pilot. The pilot cannot stand one in front of itself.
`crates/lab` exists for exactly this meeting and is the only place the two
halves are linked together.

## A3.2 Repack-from-tray

Every board below was built blind, from a tray, by pressing keys. The ladder
walk afterwards is the harness's and the pilot never sees it.

| tray | pieces | seated | items | cells | presses | cleared | board-decided | median TTK |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| starter | 2 | 2 | 1 | 7 | 742 | **2/50** | 1 | 45.0 s |
| preset | 24 | 24 | 7 | 99 | 48,979 | **12/50** | 12 | 6.0 s |
| owner | 75 | 65 | 19 | 203 | 91,739 | **48/50** | **48** | 4.1 s |
| friend | 76 | 68 | 20 | 211 | 89,038 | **49/50** | **49** | 4.0 s |
| perfect | 62 | 58 | 15 | 179 | 84,601 | **48/50** | 48 | 2.3 s |

Eleven to twelve seconds a tray, on one core.

**The gate was ≥ 48/50 from the owner's tray and ≥ 48/50 from the friend's.
Met: 48 and 49.**

Against the humans, piece for piece:

| tray | the human | the pilot |
|---|---|---|
| owner's 75 | 48/50, **44 board-decided** | 48/50, **48 board-decided** |
| friend's 76 | 48/50 | **49/50**, all board-decided |
| preset's 24 | `apply_preset` gets 9/50 | **12/50**, all board-decided |
| starter's 2 | the printer's starter row: 2/50 | 2/50 |

It matches the owner's count and beats it on the mission's own stricter
standard - **every one of its forty-eight clears was decided by the board
rather than by the clock, where four of the human's were the clock's.** It
beats the friend's board by a rung and the auto-builder by three.

It loses one: the perfect run's sixty-two pieces cleared **50/50** in the hands
that assembled them and 48/50 in these. That board is the only one in the
project that never gave a rung back, and the pilot cannot yet recover it.
Recorded rather than explained; A6's prior is where that gap is either closed
or written off.

The starter row is worth its own line: two pieces clear **2/50**, which is
exactly what the baseline printer has always said the starter board clears.
Two independent routes to one number, and the second one played it.

## A3.3 A flaw in the first harness, and the number it produced

The first run of this benchmark reported the owner's tray as **seventy-seven**
pieces and 49/50. `Run::new` deals the starter kit - an oak handle and an iron
blade (`run.rs:10`) - so every tray built on top of one was two pieces bigger
than it claimed, and the comparison was against a board the owner did not
build. The harness empties the tray first now and asserts it, which is why the
owner's row reads 75 and 48.

A pleasant number obtained by measuring the wrong thing is the failure this
mission is most exposed to, because every gate here is a number an agent is
being asked to move.

## A3.4 What A3 hands A4

The pilot can now build a board that clears forty-eight rungs from the right
pieces. What it cannot do is **get** those pieces: everything above starts with
a tray somebody handed it. A4 is the economy - shop, road, doors, towns - and
the first honest SCR.

The costs, for pricing A4: about 90,000 presses and eleven seconds to pack a
seventy-five-piece tray, which is exhaustive over seats and greedy over pieces.
That is affordable once a rung and not affordable once a decision, so A4's shop
layer re-packs only when the tray has changed.

---

# Interlude — THE ATLAS

Read off **`a8f8d19`**. Nine commits between A3 and A4: a map for every place a
run has been, THE THRESHOLD grown a crossbar with a shop on it, and the yard
cut into two islands. It touched the dungeon graphs, the catalogue and the
interface, all of which this mission reads.

**Everything of this mission's still passes**, unchanged and unedited:
console 16, oracle 19, agent 10, lab 1. The parity lint passing is the useful
part - it means THE ATLAS added no player action the pilot lacks, and nothing
had to be checked by hand to know that.

## What moved

| | A3, at `1cda004` | Now, at `a8f8d19` |
|---|---|---|
| engine suite | 1,053 / 52 | **1,059 / 52** |
| GUI suite | 83 | **88** |
| `CATALOG` | 518 | **523** (a five-piece shelf sold in one room) |
| dungeon floors | 23 | **24** |
| dungeon exits | 16 | **16** - one added to the Threshold, one cut from the yard |
| repack-from-tray | 48 / 49 / 48 | **48 / 49 / 48**, byte for byte |

## The graphs, as they stand

    the-crevice        3 floors  forks []      stops [2]           islands []
    the-threshold      4 floors  forks [1]     stops [2, 3]        islands []
    the-under-mine     2 floors  forks []      stops [1]           islands []
    the-undertow       2 floors  forks []      stops [1]           islands []
    den-rivals         2 floors  forks []      stops [1]           islands []
    wumpus-world       2 floors  forks []      stops [1]           islands []
    the-switchyard     9 floors  forks [2, 6]  stops [3, 4, 7, 8]  islands [5, 6, 7, 8]

**Four of the switchyard's nine floors cannot be walked to.** They are reached
only by feeding a particular Orb of Travel to a pedestal, which lands a run on
a siding inside them. That is a sixth of every dungeon floor in the game
behind a purchase.

For the coverage ledger this is a **fourth class**, and it has to exist before
A5 counts anything: *offered · answered · branched* were the three, and these
floors are **reachable only through a specific acquisition**. Counting them as
"never offered" would report a design decision as a bug, and counting them as
reached would be worse.

## Two amendments

**The `View` carries the dungeon's graph now.** The atlas lays a dungeon out
from its exits and names what stands on each floor once the run has entered -
`gui/src/main.rs:7203`, *"a floor you have not reached does not name what is on
it"*. That is routing information a player has and the pilot did not, and A4
needs it: a run standing at the points on floor 1 of THE THRESHOLD is choosing
between the way down and a shop, and until now the console could see the two
labels but not where either went.

**A5's ledger gains the fourth class**, above.

## What did not move

The three reference trays repack to exactly the same boards - 48/50, 49/50,
48/50, identical cells and presses. Five new pieces entered the catalogue and
none of them is in any of those trays, which is the answer one would want:
this benchmark measures board sense against a fixed tray, so it is insensitive
to content growth by construction, and now that is measured rather than
assumed.

---

# A4 — The pilot plays

Read off **`a8f8d19`**. `crates/agent/src/pilot.rs`, and three binaries in
`crates/lab`. Workspace: **1,206 green, no warnings**.

**Every number below was produced by playing.** A run starts with 28 gold and
an oak handle, and everything after that is the pilot reading a screen and
pressing keys.

## A4.1 The headline

64 seeds - the training half, exactly as A0 wrote it down - at Medium.

| | Grinder | Rogue |
|---|---:|---:|
| **SCR(R5)** | **81.2%** (52/64) | 45.3% (29/64) |
| **SCR(R10)** | **76.6%** (49/64) | 28.1% (18/64) |
| **SCR(R15)** | 10.9% (7/64) | 6.2% (4/64) |
| **SCR(R25)** | **6.2%** (4/64) | 4.7% (3/64) |
| **SCR(R50)** | **3.1%** (2/64) | 1.6% (1/64) |
| deepest run | rung 53 | rung 53 |
| wall-clock | 0.8 s a seed | 0.5 s a seed |

**A4's gate was SCR(R10), SCR(R25) and SCR(FRANCIS) above zero in both modes.
All six are.** Every one of them was zero by construction until this
milestone, because nothing in this repo had ever played a run.

The wall-clock is worth its own line. The plan priced a FRANCIS attempt at up
to fifty minutes a seed and budgeted two days for a sweep of sixty-four. The
whole sweep takes **fifty seconds**, because this pilot does no search over
the road at all - it answers what is in front of it and fights. That is
headroom A6 can spend, and it means the plateau test A7 wants is minutes
rather than an overnight.

## A4.2 A proof, replayed

`analysis/proofs/0000000000001212-grinder-medium.proof` - 12,008 lines, 348 KB.
Seed `0x1212`, Grinder, Medium: **rung 51, 62 board clears, 62 game clears,
10 losses, 11,996 presses.**

Rung 51 is past Francis. The pilot beat the game's final boss from a seed's
own economy, and the file is the whole run written out as keys a person could
press. It replays to rung 51 with **zero refusals**, and
`lab/tests/proofs.rs` is the `#[ignore]`d test that says so for every proof
in the directory.

**A replay that looked like a divergence and was not.** The first check
compared the pilot's *highest* rung against the replay's *final* rung and
reported 51 against 49. A Grinder is knocked back on a loss, so a run that
touched 51 can finish standing on 49. The transcript was right and the
comparison was wrong - which is the same shape as A3's starter-kit flaw, one
week apart: a number that looks like a finding, produced by measuring a
slightly different thing.

## A4.3 A blind spot in the pilot, found by a run

Seed `0x1212` first stopped at **rung 47** with *"a fountain offering
nothing"*. The third fountain does not pour - it **doubles** a class you
already hold, and offers `Double` alone with no `Drink` at all. The pilot
looked for a drink, found none, and stopped in front of it.

The console was right and the pilot was wrong, and the run that found it was
the deepest one. With the third verb added, the same seed goes to 51. A
mission about whether a game is walkable is exactly as good as the walker.

## A4.4 The wall at rung 13

Nine of the first sixteen seeds stopped on exactly the same rung. That is not
a property of nine economies.

| seed | rung | items | cells | gold | ladder creatures its board beats |
|---|---:|---:|---:|---:|---:|
| 0x6060 | 13 | 9 | 182 | 718 | **23** |
| 0xCD20… | 13 | 9 | 175 | 682 | **30** |
| 0x6C68… | 13 | 9 | 183 | 632 | **26** |
| 0xFF5B… | 13 | 6 | 197 | 662 | **24** |
| 0x8F02… | 13 | 11 | 191 | 564 | 21 |
| 0x461F… | 13 | 7 | 154 | 663 | 12 |

**The boards are not too weak.** A board that beats thirty of the fifty
creatures on the ladder cannot pass the thirteenth. Nor is it money: these
runs are sitting on five to eight hundred gold, farming a rung they have
already cleared, unable to buy their way through.

The creature is **Ashen Marshal**, and A2's theme meter had already named it.
It is in the ten that read least like what they say they are: *Wall 0.18 - it
lasts 0.00 · it answers being hit 0.00 · and it keeps putting armour on 0.60*.
A Wall that does not last and does not answer being hit, but stacks armour -
and armour absorbs before anything else does. That is not a wall, it is a
damage check with a gate on it.

**Two instruments built for different jobs, converging on one creature.** The
fidelity meter said the fight does not read as its theme; the pilot says it is
where nine of sixteen runs die. Neither could have said it alone.

The other cluster is the early economy: five of sixteen stop at rungs 2-3 with
three to five items and fourteen to eighteen gold - a shop that never offered
the piece that would have finished a second item. That is the shallow end
being a shop problem rather than a board problem, and it is what A6's prior
would have to learn first.

## A4.5 How it decides

Not a policy - a **priority**, which is what a control is. Whatever stands on
the rung is answered first, because the road stack pops in an order and a door
in front of another is not a queue. Underneath: keep the board packed, buy for
the grid with the least finished, and fight. Patience is eight fights on a
rung it is not getting past, because a Grinder may farm for ever.

Every one of those rules is a thing A6 replaces with something learned. They
are written down so that there is a number to beat.

## A4.6 What A4 hands A5

- A run reaches a **door** 0 to 24 times and a **town** 0 to 3 times in the
  runs above. That is the raw material of the coverage ledger and it is thin:
  the median run answers six doors out of fifty-three.
- The rung-13 wall caps coverage before it starts. Most of the road's content
  stands past rung 13, and 89% of runs never see it.
- **The four county mouths and the switchyard's islands have not been touched
  at all** by any run so far, which A5 has to be able to say out loud rather
  than report as a zero.

---

# A5 — Coverage as an objective

Read off **`a8f8d19`**. `crates/agent/src/seen.rs`, the coverage dial on
`Doctrine`, and `crates/lab/src/bin/cover.rs`, which writes
**`analysis/coverage.md`**. Workspace: **1,209 green, no warnings**.

## A5.1 The ledger

128 runs - 64 seeds in each of two modes, at Medium, with the dial at maximum.
**Nothing in it is read out of a table. Every count is a place a run stood.**

| | offered | answered | branched |
|---|---:|---:|---:|
| doors (53) | **37** (70%) | **37** (70%) | **31** (58%) |
| choices (120) | - | **80** (67%) | - |

Every gap is classified rather than counted, and there are **five** classes
rather than the three the spec asked for. The two extra ones both exist to
stop the ledger reporting a design decision as a fault:

| class | doors | what it means |
|---|---:|---|
| too few runs got there | **1** | the ceiling, not the content |
| offered and never answered | **0** | - |
| **runs were there and it did not appear** | **12** | the class worth reading |
| **delivered rather than walked to** | **3** | `flag: "never"`, the engine's own sentinel |
| answered, not every branch | **6** | 1 or 2 of 3 |
| reachable only by acquisition | **4 floors** | the switchyard's islands |

## A5.2 The class worth reading

Twelve doors that runs stood on the rung for and never saw. Every one is
conditional, and the ledger says which condition:

    the-crownwright          rung 20, 13 runs stood there · wants a rumour
    the-green-ledger         rung 23, 12 runs stood there · wants a rumour
    the-wizards-thirst       rung 31, 10 runs stood there · wants a rumour
    the-exhibition           rung 34,  9 runs stood there · wants a rumour
    the-locked-gate          rung 41,  9 runs stood there · wants a rumour
    the-glow-over-the-ridge  rung 46,  8 runs stood there · wants a rumour
    the-sealed-bid           rung 36,  9 runs stood there · wants a flag
    the-fork                 rung 37,  9 runs stood there · wants a flag
    the-constable            rung 40,  9 runs stood there · wants a flag
    the-passenger            rung 42,  9 runs stood there · wants a flag
    the-foundry-remembers    rung 47,  8 runs stood there · wants a flag
    the-pale                 in the county, 128 runs stood there

**Six of the twelve want a rumour, and the pilot has been to a pub nineteen
times.** A rumour is not bought with money - it is **bartered** for, with
something you are carrying - and `Barter` is a verb the pilot has and never
presses. So six road doors are shut behind a verb the pilot owns and does not
use. That is a finding about the walker, precisely located, and it is A6's
first job.

Five want a flag, and three of those want the same one - `slagworks-known`.
THE SLAGWORKS is one of the two towns no run has reached.

`the-pale` is the county's, and the answer is arithmetic: 20 tiles stood on
across 128 runs, out of forty-nine. A trip is five moves and the pale is one
tile.

## A5.3 Two classes that had to exist first

**Delivered rather than walked to.** `event.rs:2877` says it plainly:
`flag: "never"` is *"the sentinel for a door nothing on a rung can reach"* -
something else pushes it through `forced_event`. THE UNWOUND is one, and so
are THE THRUMBUS RACE and MOLE TOWN. A ledger that reported those as gaps
would be describing the design and calling it a fault.

**Reachable only by acquisition.** THE ATLAS's islands: four of the
switchyard's nine floors cannot be walked to from any mouth.

Both are the same mistake in two directions, and the ledger's whole value is
that it does not make it. `offered / answered / branched` were three columns;
the game needed five classes to be described honestly.

## A5.4 The dial, and the change that made it work

The first sweep covered **51%** of doors and stood on **zero** county tiles in
128 runs. Two faults, both in the pilot:

**The dial did not reach the town gate.** The pilot always wanted the shop, so
it never went down the steps - and the county was not unreachable, it was
unasked for. At coverage the gate is a door-shaped opportunity rather than a
shop.

**And the memory was a set.** `Seen` recorded *that* a branch had been taken.
With a set, the first run to take a branch closes it for every later run: the
sweep is diverse and each run in it is monotonous, and content that needs
**repetition** - a county trip out of a town gate, ten trips a run - is
visited once and never again. Counting instead, and taking the least-visited
branch, is a four-line change:

| | untried-first | least-visited-first |
|---|---:|---:|
| doors offered | 27 (51%) | **37 (70%)** |
| choices taken | 61 (51%) | **80 (67%)** |
| doors fully branched | 22 (42%) | **31 (58%)** |
| county tiles | 8 | **20** |
| towns reached | 3 of 6 | **4 of 6** - a hidden one appeared |
| deepest rung | 47 | **51** |

It got *deeper* as well as wider, which is not what a coverage dial is
supposed to do to a clear rate and is worth not over-reading: one seed does
the deep running either way.

## A5.5 What A5 hands A6

Three things, in the order they are worth:

1. **`Barter` is a verb the pilot owns and never presses**, and six doors are
   behind it. A learned policy would find this; the point of writing it down
   is that a hand-written one did not.
2. **The county is walked at 20 tiles out of 49 across 128 runs.** Ten trips a
   run exist and the pilot takes one or two.
3. **Two towns and five dungeons are still untouched** - THE MANSE, THE
   SLAGWORKS, and every dungeon but the crevice and the yard. The dungeons are
   entered by answering a door, so they are downstream of the same problem.

**No door in this game has been shown unreachable.** Every gap the ledger
holds is a gap in the walker, and that is the honest state of the validity
claim after A5: the instrument works, and the thing it is measuring is not yet
good enough to make the measurement interesting.

---

# A6 — The analysis that decides whether a net is justified

Read off **`a8f8d19`**. The spec opens A6 *"only if A4's failure class is
exploration"*, and the whole value of that gate is that it is a measurement
rather than a preference. It took three measurements to answer, two of which
refuted a hypothesis I had already acted on. **No network was built, and the
gate is the reason.**

## A6.1 The plateau

Patience - fights spent on a rung a run is not passing - and not the press
budget, which was never binding.

| patience | budget | R10 | R15 | R25 | R50 | median rung |
|---:|---:|---:|---:|---:|---:|---:|
| 8 | 200k | 76.6% | 10.9% | 6.2% | 3.1% | 13 |
| 24 | 600k | **90.6%** | 12.5% | 9.4% | 6.2% | 13 |
| 80 | 2,000k | 90.6% | 12.5% | 9.4% | 7.8% | 13 |

**Three times the budget buys a great deal; ten times buys nothing.** It had
plateaued, and the median run stopped on rung 13 at every budget.

## A6.2 Two hypotheses, both refuted by measurement

**The classifier said exploration**, and it said so from one number: 53 of 56
failed runs lost with 80-100% of the creature still standing. Nothing within
10%. On the spec's own rule that is "the tray never held the family the fight
wanted", which is what a learned prior is for.

It was wrong twice, and both times the check was cheap.

**Hypothesis one: armour.** Ashen Marshal stacks armour, armour absorbs before
anything else, and a board of many small hits would do nothing to it. Measured
off the log: **11% of everything thrown was eaten before it landed.** One
stuck board dealt 1,374 damage - *more than the owner's winning board's 1,167*
- and lost. Not armour.

**Hypothesis two: glass cannons.** The `your hp` column is stark:

| | health | dealt | taken | outcome |
|---|---:|---:|---:|---|
| owner's board | **2,346** | 1,167 | 455 | victory in 4.0 s |
| friend's board | **1,755** | 1,360 | 439 | victory in 3.8 s |
| eleven stuck boards | 280-974 | 228-1,374 | 646-2,129 | defeat |

So the objective was rewritten: `Sense::worth` weighted damage four to one
against health, and a board is only worth the damage it lives long enough to
deliver, so the two halves **multiply** rather than add - a geometric mean,
maximised where offence and defence are equal.

**It changed nothing.** The boards at the wall came back byte-identical: same
health, same damage, same fight, to the point. Clear rates moved inside noise
and one repack benchmark got worse. The hypothesis that generated the change
was refuted by the change's own outcome, which is the cheapest possible way to
find out.

The rewrite is kept anyway, on its own merits and not on the argument that
produced it: it is the right shape for a board score, and A6's features are
what a prior would learn on. **The argument for it is gone and the change is
not.** That is worth saying out loud.

## A6.3 What it actually was: three verbs the pilot never pressed

The boards were identical because the pilot's *seating* was never the
constraint. **What it owns is.** At the wall a run carries six to eleven items
and **five to eight hundred gold**, and buys one piece a rung by a crude rule.

A5 had already found the first of these and I had not connected it:

- **`Barter`** - six road doors want a rumour, and a rumour is not bought with
  money. Nineteen pub visits, no barters.
- **`Sell`** - a piece the hands will not seat sits in the tray paying nothing,
  and the tray is twelve. Nothing was ever sold.
- **`Reroll`** - six new shelves for a coin, and a run standing on six hundred
  gold has no better use for one. Nothing was ever rerolled.

All three are verbs the console has offered since A1. The pilot pressed none
of them.

| | before | after |
|---|---:|---:|
| SCR(R5) | 81.2% | **100%** |
| SCR(R10) | 76.6% | **95.3%** |
| SCR(R15) | 10.9% | **65.6%** |
| SCR(R25) | 6.2% | **56.2%** |
| SCR(R50) | 3.1% | 3.1% |
| median rung | **13** | **34** |
| deepest | 51 | 53 |

R25 went up **nine times**, R15 six, and the wall at rung 13 - nine of sixteen
runs, two instruments pointing at it - is gone. The median run now stops
where the last quarter of the ladder begins. **Not one line of the
board-packing changed.**

R50 did not move, and that is the honest shape of the result: the shop was the
whole of the middle of the game and none of the end of it.

## A6.3a Two faults the verbs brought with them

**An infinite loop.** A reroll is a coin against six shelves and a run with
six hundred gold can afford six hundred of them; the first version had no
bound and ate a two-million-press budget without fighting a single rung.
Shopping is capped at twenty-four presses between fights now.

**And a threshold that read the finding backwards.** The reroll fired on
`gold > reroll_cost * 8`, which is true at rung two with thirty coins - so a
run rerolled six times before it had anything to spend on and arrived at its
first real fight poorer than it started. Seed `0x1212` went from **rung 51 to
rung 2** on that alone, and an agent test caught it rather than a sweep. The
finding this verb came from was a run sitting on six hundred gold; that is the
condition it belongs under, and gating it on hoarding took the median run from
20 to **34**.

## A6.4 Coverage, again

| | A5 | A6 |
|---|---:|---:|
| doors offered | 37 of 53 (70%) | **38 of 53 (72%)**, on half the runs |
| choices taken | 80 (67%) | 74 (62%), on half the runs |
| the class worth reading | 12 doors | **11 doors** |

Two rumour-gated doors opened - THE WIZARD'S THIRST and THE EXHIBITION - and
two new ones appeared, THE SIGNAL BOX and THE TURNTABLE, because runs now
reach rungs 25 and 28 to walk past them. The remaining eleven still want a
rumour the pub did not stock or a flag from a town no run has reached.

## A6.5 The decision about the net

**Not built, and the gate is why.** The spec says a prior is the answer to
exploration and a better objective is the answer to evaluation. It was
neither: it was a **policy** failure - three actions in the vocabulary that
the hand-written control never chose - and the fix was forty lines.

That is the most useful thing this milestone produced, and it is an argument
about method rather than about this game: **before a net, spend a day asking
what the control does not do at all.** A learned policy would have found the
barter, eventually, at the cost of a training run. A measurement found it in
an afternoon, and the measurement is still there to catch the next one.

What A7 has to answer, with the same discipline: after this, is the remaining
gap - **R25 at 56%, R50 at 3%** - exploration, evaluation, or another verb
nobody presses? The classifier says exploration and the classifier has now
been wrong twice.

One thing points at the answer already. The shop fixed the middle of the
ladder and did nothing at all to the end of it, which is what one would expect
if the last fifteen rungs are a *board* problem rather than an economy one -
and A3 measured a board that clears 48 of them from the right pieces. The gap
between 48/50 handed the pieces and 3% getting there from a seed is the whole
of what is left.

---

# A7 — The loop, which turned out to be two more verbs

Read off **`a8f8d19`**. A6 ended by predicting that the last fifteen rungs
were a board problem rather than an economy one. **They were a board problem,
and the board's problem was that it was full.**

## A7.1 What a deep run looks like

The same probe A4 used, re-run after A6:

| seed | rung | items | cells | health | gold | ladder creatures its board beats |
|---|---:|---:|---:|---:|---:|---:|
| 0x1111 | 45 | 13 | 222 | 2,429 | **14,901** | 43 |
| 0xC434… | 50 | 16 | 228 | 1,010 | **18,778** | 48 |
| 0xCD20… | 45 | 10 | 235 | 506 | **13,944** | 43 |
| 0x8F02… | 34 | 12 | 229 | 2,066 | **11,391** | 42 |

Two hundred and twenty-eight cells of two hundred and forty. **The board is
full and the purse holds fifteen thousand gold.** A deep run buys seven
hundred pieces and sells six hundred and ninety-nine of them, because
`hands::pack` only ever *adds* - it has no move that removes - so buying is
pointless when there is nowhere to put anything.

## A7.2 The verb that unblocks it is the one no interface had

`ClearSlot` empties one grid in a single press. Re-packing it afterwards gives
the greedy strictly more choice than it had when it filled that grid one piece
at a time, because now it chooses from everything the run owns rather than
from whatever happened to be in the tray at the time.

It is one of **the four verbs `console/tests/parity.rs` found had no interface
at all** at A1 - the engine could empty one grid and no person could ask it to.
The verb that unblocks the deep game is the verb nobody could reach.

And `Grow`, another of the four, is pressed now too.

## A7.3 Two costs, both measured the hard way

**Rebuilding every rung is a search, not a player.** A rebuild is a whole
re-pack - fifteen thousand presses on a full board - and doing it whenever the
tray had leftovers took the median run from 34 to **10**, with every seed out
of budget. Gated on a *loss*, it is what a person does after a defeat.

**And once a loss is still twenty-four times a rung**, because patience is
twenty-four. Capped at once a rung, reset when the run gets past one.

## A7.4 The result

The whole training half, 64 seeds, Grinder, Medium.

| | A4 | A6 | **A7** |
|---|---:|---:|---:|
| SCR(R5) | 81.2% | 100% | **100%** |
| SCR(R10) | 76.6% | 95.3% | **100%** |
| SCR(R15) | 10.9% | 65.6% | **81.2%** |
| SCR(R25) | 6.2% | 56.2% | **79.7%** |
| **SCR(FRANCIS)** | **3.1%** | **3.1%** | **40.6%** |
| median rung | 13 | 34 | **47** |
| deepest | 51 | 53 | **54** |
| wall-clock | 0.8 s | 7 s | 32 s a seed |

**Francis falls on twenty-six seeds in sixty-four**, from each seed's own
economy, with no oracle anywhere in the loop and no action a person does not
have. Every seed clears rung 10; four in five clear rung 25.

For the row this began from: `design/the-apprentice.md` §1's table says
SCR(FRANCIS) is *"0, and nothing in the repo plays a run"*. It is 40.6%.

The cost is real: thirty-four seconds a seed against A4's 0.8. The whole of it
is the rebuild, and the whole of the rebuild is `hands::pack` re-reading the
board for every trial seat. That is the thing to make cheaper if A8 wants
more sweeps, and it is an engineering cost rather than a design one.

## A7.5 A proof of a fifty-three-rung run

`analysis/proofs/C434E4A68C5906EE-grinder-medium.proof`. Seed
`0xC434E4A68C5906EE`, Grinder, Medium: **rung 53, 73 board clears, 25 losses.**
It replays to rung 53 with zero refusals.

The run pressed **195,273** keys. The proof is **44,967** of them, because the
hands try a seat, read the board and take it back - so a transcript is mostly
`place`/`undo` pairs, which is a faithful record of what the pilot *did* and a
poor record of what it *played*. An undo cancels the press before it exactly,
so cancelling them out leaves the keys a person would press if they already
knew what the pilot found. Six megabytes becomes half a one, and **the
reduction is only valid because the reduced file still replays** - which is
checked rather than argued.

The spec's artifact policy says a proof is "a few KB each". That was written
when nothing in the repo had played a run. A proof is as long as the run it
proves; one is committed as the mission's evidence and the rest are made on
demand, which `.gitignore` now says and why.

## A7.6 The pattern, three times now

A4 found the pilot stopped at a fountain it had no verb for. A6 found three
verbs it owned and never pressed. A7 found two more. Every one of them was
worth more than a training run would have been, and every one was found by
**asking what the control does not do at all** rather than by asking it to do
what it does better.

The mission's own instrument keeps saying the same thing: the vocabulary was
complete at A1, and the policy over it is where everything has been.

---

# A8 — The learning, at last, and exactly what it bought

Read off **`a8f8d19`**. burn 0.20.1 behind `--features nn` in `crates/lab`;
`cargo test --workspace` compiles not one line of it.

## A8.0 What had and had not happened

Nothing had been learned. A0 to A7 took SCR(FRANCIS) from 0% to 40.6% with no
network, no gradient and no training run - four times the gate said the
control had a hole in it, and four times it was right. That is a defensible
sequence and it is **not** what the brief asked for, and the record should say
so as plainly as it says the clear rates.

Two things changed that made this the moment:

1. **There is an expert worth imitating.** Before A6 the control was stuck at
   rung 13; a net trained on it would have learned to be stuck at rung 13.
2. **There is a cost only learning can pay.** A run takes thirty-two seconds
   because the hands try *every* seat and take it back - 195,273 presses, 77%
   of them trial-and-revert. That is a search doing by brute force what a
   prior is for.

## A8.1 The labels were already being thrown away

Every trial seat is `(what the piece is, where it went, what the board was)
→ what it was worth`. The hands make a hundred and fifty thousand of them a
run and discard every one.

`lesson.rs` describes a seat in **24 numbers**, all of them read off the
`View`: the piece's size, shape, price, triggers, stats, and - since the card
rewrite - whether any of its figures are handed over per activation rather
than held. Then the grid it is going into: how full, how many items finished,
how many neighbours the seat touches. A model over these learns *"a two-cell
accessory in a nearly-full weapon grid is worth trying and a five-cell base is
not"* without knowing anything about the run.

**129,206 lessons** from eight played seeds and the three finished trays.

## A8.2 The model

Three layers, 24 → 64 → 64 → 1, ReLU, MSE, plain gradient descent. Written
against burn's tensor API rather than its `Module` derive and `Learner` - six
tensors and a manual step is less to go wrong across versions and short enough
to read in one sitting. `Autodiff<NdArray>` on the M2 Max's CPU.

    250 epochs, lr 0.1, 41.8 s      train 0.267   held out 0.176

**Longer training makes it worse on held-out data**, and the reason is worth
having: the held-out fifth is the *tail* of the file, which is the
finished-tray packing rather than the early-run packing. At 800 epochs the
training loss falls to 0.188 and held-out rises to 0.214. The split is a
distribution shift, not a shuffle, and it is measuring the thing that matters
- whether a prior learned on early boards is any use on deep ones.

**Training is privileged and inference is not.** The trainer writes six
matrices of plain floats; `agent/src/prior.rs` reads them and multiplies them
out in forty lines. The pilot links no framework and the boundary is unmoved.

## A8.3 What it bought, measured

The prior does not decide anything. It **orders** the seats the hands were
going to try, so they try sixteen instead of ninety and check each against the
real board exactly as before. Every board below was measured, not predicted.

Repack-from-tray, the A3 benchmark, same trays, same objective:

| | exhaustive | prior, top 8 | prior, top 16 |
|---|---:|---:|---:|
| owner's 75 | **48**/50 · 88,940 presses · 18.0 s | 47/50 · 25,872 · 5.4 s | **48**/50 · 47,557 · **11.9 s** |
| friend's 76 | **49**/50 · 83,659 · 18.2 s | 48/50 · 23,475 · 5.6 s | **49**/50 · 40,847 · **8.4 s** |
| preset's 24 | 12/50 · 47,550 | 12/50 · 10,489 | 12/50 · 19,825 |

**At top 16 the prior matches the exhaustive search's board quality exactly -
48 and 49, which is A3's gate - for 1.9× fewer presses and 1.5× less
wall-clock.** At top 8 it is 3.4× cheaper for one rung.

That is a clean, controlled result: same trays, same objective, same
reconstruction, one variable.

## A8.4 What it did not buy

At the run level, on sixteen seeds:

| | unguided | guided, top 16 |
|---|---:|---:|
| SCR(R10) | 100% | 100% |
| SCR(R15) | 75.0% | 75.0% |
| SCR(R25) | 68.8% | **75.0%** |
| SCR(R50) | 31.2% | 31.2% |
| median rung | 47 | 47 |
| wall-clock | 34 s a seed | **51.6 s a seed** |

One extra seed at R25 out of sixteen is noise, and the wall-clock went **up**,
because a run that packs faster gets further and a run that gets further packs
more. The plan's gate for a prior is *"strictly higher at equal wall-clock
including inference"*, and on this evidence **it is not met at the run
level**.

So the honest verdict is split, and both halves are worth having:

- **The prior works where it was aimed.** Halving a search at equal quality is
  what a learned prior is for and it is measured cleanly.
- **It does not move the headline metric**, which is exactly the outcome
  `rl-research.md` §5 called likeliest: on an exactly-scored constructive
  problem, search is hard to beat per unit of compute, and the learning's real
  contribution is making the search cheaper rather than making it smarter.

## A8.5 Where the learning goes next, if it goes

The prior is trained on **`Sense::worth`**, which is the pilot's own crude
objective - so at best it can only be as right as that. Two things would move
it, in order of expected value:

1. **Train on outcomes rather than on the surrogate.** A seat's real label is
   whether the run that took it cleared the next rung. That is a value
   function rather than a regression on a proxy, and it is what the plan's
   Ranked Reward was for. The data exists: every proof is a labelled
   trajectory.
2. **Learn the shop.** Every gain in this mission came from acquisition, and
   `want_to_buy` is still six lines of hand-written preference. The features
   are the same shelf cards the pilot already reads.

Both are the same architecture with a different target, and neither needs
anything the workspace does not now have.

---

# A8.6 Three questions, answered by measurement

## Is it picking up rumours?

**It was bartering for them and then selling them, every time.**

`--bin report` over six runs, before the fix: **40 barters made, 9 pub doors
gone through, and not one rumour still held at the end of any run.** Three of
the eleven rumour-opened doors had ever been offered.

The cause, confirmed rather than guessed: **every rumour in the game costs one
gold**, which is the cheapest price in five hundred and twenty-three pieces.
A6's `Sell` branch picks the cheapest thing in the tray when the tray fills.
So the pilot bartered for a key and sold it for a coin, reliably, at the next
shop.

`PieceKind::Quest` is what a rumour's card says it is, so the fix is one
filter on a thing the pilot can see:

| | before | after |
|---|---:|---:|
| rumours held at the end of a run | **none, ever** | **seven kinds** |
| rumour-opened doors ever offered | 3 of 11 | **7 of 11** |
| dungeons ever entered | **0 of 7** | **1 of 7** |

`the-locked-gate` was on A5's *"runs were there and it did not appear"* list.
It appears now. So did `the-signal-box` and `the-turntable`, and those two are
switchyard doors - **which is how the pilot got into a dungeon for the first
time.** One filter closed three coverage gaps and opened a dungeon.

Four rumours are still never acquired: the Crownwright's, the Glow, the
Picket, and the Hundred. Three of those four are `told` rather than
`on_the_bar` - they come from somewhere other than a pub - which is the next
thing to find.

## How is it doing in the mini dungeons?

**Badly, and now measurably.** Before the rumour fix: **zero of seven,
never, in any run, at any depth** - while clearing rung 51. After it: one of
seven, the switchyard, four of its nine floors.

The reason is structural and A1's parity lint already said it: **a run never
walks into a dungeon.** There is no `Enter` verb because there is no such
player action - you are *put* in one, by answering a door whose outcome is
`Enter`, by a town's cellar, or by feeding an orb to a pedestal. So dungeon
entry is entirely downstream of the door-choice policy, and the pilot's is
"take the first open choice".

That makes the dungeons the sharpest remaining target: **24 floors, six
dungeons and four unreachable-by-walking islands, behind a policy that is one
line long.**

## How well is it doing overall?

64 seeds, Grinder, Medium, the training half:

| | value |
|---|---|
| SCR(R10) | **100%** |
| SCR(R15) | 81.2% |
| SCR(R25) | 79.7% |
| SCR(FRANCIS) | **40.6%** |
| median rung reached | 47 |
| deepest | 54 |
| doors answered a run | ~20 of 53 |
| towns visited a run | 2.5 of 6 |
| dungeons entered | **1 of 7** |
| county tiles walked | 20 of 49, across 128 runs |

The board it builds is as good as a person's from the same pieces (48/50 and
49/50 at A3). What it is still bad at is **everything that is not the ladder**:
one dungeon in seven, two and a half towns in six, a fifth of the county.

---

# Watching it play

    GEARMASTER_WATCH=analysis/proofs/C434E4A68C5906EE-grinder-medium.proof \
        cargo run -p gearmaster-gui

The window plays the proof: 44,967 presses, 98 of them fights, reaching rung
53. **Space** pauses, **right arrow** steps one press while paused, **up** and
**down** change the pace, and `GEARMASTER_WATCH_MS=20` starts it fast.

Two things make it honest rather than a re-enactment:

1. **Every non-fight press goes through the same `Console` the agent uses.**
   There is no second implementation of what a verb does, so what the window
   shows is what the agent did rather than a story about it.
2. **Every fight goes through the window's own `begin_next_fight`**, so the
   battle screen plays out exactly as it does for a person. Watching a run
   means watching the fights.

`--bin watchcheck` walks the same path headlessly and reports the presses, the
fights and the rung reached, so a proof can be checked without a window.
