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
