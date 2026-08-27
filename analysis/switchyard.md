# The Switchyard - measurements, milestone by milestone

Every block below is headed by the commit its numbers were read off. The
spec is `design/the-switchyard.md`; the record of decisions is
`design/HANDOFF-switchyard.md`. This file holds only what was measured, and
where the code turned out not to be where a citation said it was.

The rule this file exists for is `CLAUDE.md` §7: write down which commit a
number came from.

---

## M0 baseline at `e38d968`

`e38d968` "The guide catches up with the five fixes", 2026-08-26 18:06 -0400,
which is the tip `design/the-switchyard.md` was written against. The working
tree at measurement time carried exactly two changes, both M0's own:
`Cargo.toml`'s `rust-version` and the `gear_at` fixture with its printer and
gate test. Neither touches a rule, a piece or a creature.

### Toolchain

| | |
|---|---|
| Declared | `rust-version = "1.83"` (was `"1.75"`; **M0 changed it**) |
| Built with | rustc 1.95.0 (59807616e 2026-04-14), cargo 1.95.0 |
| Warnings | **0**, `cargo check --workspace --all-targets` from a touched `lib.rs` |

`post-unwinding.md` §10.1 and `CLAUDE.md` §6 trap 1 both say the declaration
is a lie: the code needs 1.83 for `Option::is_none_or` (1.82) and for `const`
items referring to `static` items (1.83). The declaration is now 1.83 and the
trap is retired. Nothing in the source changed to earn it - the floor was
already 1.83 and only the promise was wrong.

### The suite

| | |
|---|---|
| Engine | **801 passed, 0 failed, 40 ignored** before M0's own test; **802 / 41** after |
| Binaries | 49 integration binaries in `tests/` plus the lib's 158 |
| Wall | ~14 s warm, one core; `avail` 7.6 s and `insight` 2.1 s are the slowest |
| GUI | **62 passed, 0 failed** (`cargo test -p gearmaster-gui`) |

`CLAUDE.md` §5 says "801 green, 40 ignored" and "50 binaries + lib" at
`b30c80b`. The 801 and the 40 are confirmed at `e38d968`. **The binary count
is 49, not 50** - `ls crates/engine/tests/*.rs` is 49 files. Recorded as a
difference, not fixed here; §5's other counts were all confirmed except the
five below, which the five post-`b30c80b` fixes moved and §5 did not follow:

| Binary | `CLAUDE.md` §5 | At `e38d968` |
|---|---:|---:|
| `completable` | 4 | **7** (+ 1 ignored) |
| `primitives` | 17 | **21** |
| `road_stack` | 11 | **14** |
| `structures` | 24 | **25** |
| `the_road` | 6 | **10** |

M0's own additions are `catalog_shape::no_creature_changed_what_it_wears`
(green) and `catalog_shape::report_gear_at` (ignored).

### The four-board table, at Medium

`cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1`

| build | cleared | weapon % | median ttk | burn % |
|---|---:|---:|---:|---:|
| starter | 2/50 | 100.0% | 45.00 s | 0.0% |
| preset | 9/50 | 100.0% | 9.00 s | 0.0% |
| owner | **48/50** | **75.5%** | **9.00 s** | 0.0% |
| friend | **48/50** | **97.4%** | **8.15 s** | 0.0% |

Byte-identical to `post-unwinding.md` §5's tip column at `18d1b85`. This is
the table every Phase-1 milestone's exit criterion compares against.

Per-rung, the four probe fights:

| build | 1 Cave Rat | 10 Warded Idol | 25 Cog Priest | 40 The Rust Parliament |
|---|---|---|---|---|
| starter | win 4.50 s | loss 9.00 s | loss 5.90 s | loss 7.50 s |
| preset | win 1.50 s | win 19.50 s | loss 39.00 s | loss 44.00 s |
| owner | win 1.50 s | win 2.80 s | win 12.00 s | win 22.50 s |
| friend | win 2.60 s | win 4.75 s | win 10.30 s | win 15.45 s |

### Cadence and the mind lane

| build | items | activations/s | per item | helmet mind | greaves mind |
|---|---:|---:|---:|---:|---:|
| starter | 1 | 0.50 | 0.502 | 0 | 0 |
| preset | 8 | 2.06 | 0.258 | 0 | 0 |
| owner | 19 | **6.60** | 0.348 | 59 | 59 |
| friend | 17 | **3.43** | 0.202 | **698** | 0 |

Unmoved from `post-unwinding.md` §5, including the friend's 698 against the
707 M1 of the Unwinding recorded.

### The shallow ladder, rungs 1-14 at Medium

| rung | starter | preset | owner | friend |
|---|---|---|---|---|
| 1 Cave Rat | 4.50 s | 1.50 s | 1.50 s | 2.60 s |
| 2 Bog Toad | - | 6.00 s | 1.50 s | 2.60 s |
| 3 Bone Archer | - | 9.00 s | 2.00 s | 2.75 s |
| 4 Rust Golem | - | 9.00 s | 2.00 s | 2.75 s |
| 5 Frost Wisp | - | 6.00 s | 1.50 s | 2.60 s |
| 6 Plague Hound | - | 9.00 s | 1.50 s | 2.60 s |
| 7 The Iron Warden | - | 34.00 s | 2.60 s | 4.65 s |
| 8 Iron Sentinel | - | 31.50 s | 2.60 s | 2.75 s |
| 9 Whisperling | - | - | 2.00 s | 2.75 s |
| 10 Warded Idol | - | 19.50 s | 2.80 s | 4.75 s |
| 11 Mirror Fiend | - | - | 2.00 s | 2.75 s |
| 12 Rust Colossus | - | - | 3.10 s | 5.15 s |
| 13 Ashen Marshal | - | - | 4.00 s | 5.15 s |
| 14 Grave Chorus | - | - | 4.00 s | 5.15 s |

Acceptance criterion 2 asks these to stay within +/-10%; nothing in this
mission touches rungs 1-14.

### No-weapon viability

| build | rungs won | best rung | rung 15 | ttk | what carried it |
|---|---:|---:|---|---:|---|
| starter | 1/50 | 42 | Defeat | 2.8 s | nothing |
| preset | 0/50 | none | Defeat | 5.6 s | nothing |
| owner | 42/50 | 48 | Defeat | 47.0 s | the clock, not the gear |
| friend | 35/50 | 46 | Victory | 44.6 s | the clock, not the gear |

Unmoved. `post-unwinding.md` §5's warning stands: the one "clear" is sudden
death's.

### Census, at `e38d968`

Counted out of the tables directly, and cross-checked against
`baseline::report_catalog_census`.

| | |
|---|---:|
| `CATALOG` | **504** (helmet 96, chest 71, gloves 83, greaves 67, weapon 187) |
| inert (no trigger, effect or adjacency) | 120 (23.8%) |
| creatures | **69** - `LADDER` 50 (Rust Golem spliced at index 3), `ALTERNATES` 19, `CREVICE` 0 |
| `EVENTS` | **33** |
| `TOWNS` | **6** |
| `DUNGEONS` | **6** |
| rumours | **8** |
| `FRAMES` | **15**, all dressed |
| `DESTINATIONS` | **4** |

Rarity: 499 Common, 1 Rare, 2 Epic, 2 Legendary.

### The ratchet

`cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture`

**Every exclusivity row is at budget 0 with 0 away. Every quota is 0 away.**
`the_catalog_keeps_every_rule` (the targets, not the budgets) is green.
Identity mechanics on floating kinds: **0**.

Quota shares, which M5 moves by six pieces in 510:

| slot | own axis | bleed axis | filler | dearest third interacts | pool spend |
|---|---:|---:|---:|---:|---:|
| Helmet | 75.0% | 21.9% | 27.1% | 48.4% | - |
| Chest | 97.2% | 23.9% | 12.7% | 39.1% | 5.6% |
| Gloves | 67.5% | 21.7% | 15.7% | 85.2% | 10.8% |
| Greaves | 76.1% | 22.4% | 11.9% | 54.5% | 7.5% |
| Weapon | - | - | - | 39.3% | 13.4% |

### The `gear_at` fixture

`crates/engine/tests/fixtures/gear_at.txt`: **5,568 placements**, being every
creature in `LADDER`, `ALTERNATES` and `CREVICE` dressed at all four
difficulties, two lines each (the placement, then the component on its own
line so a diff names the piece).

This is the measurement behind A6's sentence "no creature re-gears on any
setting". `catalog_shape::no_creature_changed_what_it_wears` is the gate and
it is green from M0 on. Re-baselining is deliberate and takes an environment
variable:

```
REBASELINE_GEAR_AT=1 cargo test -p gearmaster-engine --test catalog_shape \
  -- --ignored --nocapture report_gear_at
```

The variable exists because `--ignored` on that binary is the ratchet's own
printer command in `CLAUDE.md` §5, and a printer that wrote a fixture as a
side effect would erase the evidence every time somebody measured the
catalogue.

### The packer's cost, for M9

`PACK_MONSTER="Cog Priest" cargo test --release -p gearmaster-engine --test pack_francis pack -- --ignored --nocapture --exact`

**38.62 s** for one creature at the default 300 trials, release, this
machine. `post-unwinding.md` §5 recorded 39.5 s in its container; the two
agree. Best board found: 71/240 cells (30%), 963/8 outcomes on target, 21
pieces. `gui/src/pack.rs:5`'s "about five minutes per creature per power
band" remains unreconciled with both figures.

### The road, read aloud

`cargo test -p gearmaster-engine --test prose -- --ignored --nocapture read`
prints 1,124 lines and asserts nothing. Read at M0 for the six dungeons'
entries and landings, which M1 moves. Nothing in it needs a fix; it is
recorded here as run because `CLAUDE.md` §5 says four fixes a batch have come
out of reading it.

---

## The citation audit

The kickoff asks that every `file.rs:line` in the spec be checked against the
tip before it is trusted. The spec was written against `e38d968`, which is
the tip, so the expectation was that they hold. **166 distinct citations were
extracted and resolved; every one lands on code that says what the spec says
it says.** Three findings, all small:

| Spec says | Code says |
|---|---|
| `rating.rs:677` for `BOND_POINTS = 45.0` | `:675`. `:677` is the doc comment two lines below it. `:724`, the other half of the same citation, is right |
| `bestiary.rs:525-537` for `unpacked()` and `is_unpacked()` | `:525` is `unpacked()`; `:537` is `#[cfg(test)]`, one line past `is_unpacked()` |
| `CLAUDE.md` §5: 50 test binaries | 49 |

Everything A0 asserts as a fact about the code was re-derived here and holds,
including the three findings A0 says contradict a shipped design document:

- **A dungeon floor does not drop its `drops`.** The victory arm
  (`run.rs:2208-2245`) never reads them, whatever `the-unwinding.md` Part B
  says about the named-drop rule. Confirmed by reading the arm.
- **There is no flee.** `grep -c flee crates/engine/src/run.rs` is **0**.
- **`town_shelf()` collects every enchantment by kind with no event-only
  filter** (`piece.rs:10283-10293`). Confirmed; this is the fact A3's
  decision turns on.

And the tables the content milestones stand on:

- `LADDER` is **50** with `RUST_GOLEM` spliced at index **3**. The four
  `expects` the spec names are right: **[18] Ruin Hound, [24] Cog Priest,
  [27] Obsidian Colossus, [34] Rimefather**.
- The free event indices between 18 and 35 are exactly **18, 20, 25, 27, 32,
  34**, which is what the spec claims and what the four doors need.
- `EVENTS` 33, `DUNGEONS` 6, rumours 8, `TOWNS` 6, `FRAMES` 15,
  `DESTINATIONS` 4, `ALTERNATES` 19 - all as cited.

---

## M1 the floor graph, at `7c2dcfe`+

`Dungeon.floors` is `&[Floor]`, `Dungeon.landings` is gone into
`Floor.landing`, and the six shipped dungeons are straight lines in the new
shape. Landed inert, and "inert" here is three diffs rather than three
sentences.

### The three inertness diffs

| What | Command | Result |
|---|---|---|
| The six dungeons, walked | `dungeons::the_six_shipped_dungeons_replay_word_for_word` | **byte-identical** |
| The four-board table and every other baseline figure | `--test baseline -- --ignored` | **byte-identical**, whole printer output |
| Every creature's gear at every difficulty | `catalog_shape::no_creature_changed_what_it_wears` | **green**, 5,568 placements unmoved |

The first was measured rather than argued. The transcript in
`analysis/replays/dungeons.txt` - banner, creature, fights ahead, landing,
receipt and map figures for all six dungeons walked from the top - was
captured **off the pre-M1 code**, by stashing M1 and running the same walk
against `Dungeon.floors: &[&str]`, then diffed against M1's. `diff` is empty.
83 lines.

The second: the whole of `baseline`'s six printers differ only in cargo's own
two timing lines. So the four-board table, the cadence figures, the mind
figures, rungs 1-14, no-weapon viability, slot-mattering and the census are
all where M0 left them.

The ratchet (`catalog_shape -- --ignored`) is identical apart from one status
line's position, which is thread scheduling. `prose::read_the_road_aloud` is
identical.

### The suite

| | M0 | M1 |
|---|---:|---:|
| Engine passed | 802 | **813** |
| Engine ignored | 41 | 41 |
| GUI | 62 | 62 |
| Warnings | 0 | 0 |

The eleven: eight graph lints in `dungeon.rs`'s own test module, and three in
`tests/dungeons.rs` (the map fixture, the points count, the replay).

### The size of it, counted

750 insertions, 123 deletions across 11 files. `dungeon.rs` is most of it
(542 changed lines, of which the six rewritten tables are about 300). Ten call
sites across five source files and twenty-two across six test binaries, which
is what A1.7 predicted; the three it missed are listed below.

---

### Findings - where the code was not what the spec said

**1. `route::ascii` changes by exactly one word, and the spec asks for both.**
A1.4 spells out the two strings the printer must produce - `(4 fights, 2
points)` for the yard and `(3 fights)` for THE THRESHOLD - and acceptance
criterion 3 says `route::ascii` is "byte-identical". Today's printer says
`(3 floors)`. The two cannot both hold.

Taken: **A1.4**, because it is the section that specifies the printer, in
detail, with both example strings written out, and because "floors" is a room
count, which stops being a thing a run experiences the moment a dungeon has
points in it. Criterion 3's number - which is what it exists to protect -
does not move: for a straight line `fights_ahead(0, &[])` is `floors.len()`.

Re-pinned with the reason in the assertion rather than in this file:
`dungeons::the_ascii_map_did_not_change_for_a_linear_dungeon` holds the real
pre-M1 bytes as a fixture (`tests/fixtures/route-ascii-m0.txt`, 89 lines,
captured off `e38d968`) and applies the one substitution in the test body, so
the word that moved is named in the assertion instead of hidden in a file.

**2. THE THRESHOLD is never on the route map.** Criterion 3 names it, and a
`NodeKind::Dungeon` is only ever pushed for an event choice whose outcome is
`Enter` or `StartDungeon` (`route.rs:266-283`). THE THRESHOLD is reached
through a town door, so it has no node. The one shipped dungeon the ascii map
draws is **THE CREVICE IN THE ROCK**. The fixture pins the whole 89-line map
rather than one line, so it would catch either.

**3. `Floor::along` cannot take a floor number.** A1.1 writes the six
conversions as `exits: &[Exit { to: i+1, .. }]`. A `const fn` cannot hand back
a reference to a slice built out of its own arguments - there is nowhere for
the temporary to live, and rvalue static promotion does not reach anything
argument-dependent (`E0716`). So the constructor takes the exits:
`Floor::along("The Reciter", "...", &[Exit::on(1)])`, and `Floor::last` is the
same call with `&[]`. No loss: a fork spells its levers out in the same place.

**4. The seventh graph lint is half a lint until M3.** A1.1's seventh asks
that every floor with a non-empty `entry` is some destination's
`Where::Siding` and every `Where::Siding` lands on a floor that has one.
`Where::Siding` is M3. The forward half is
`dungeon::no_floor_offers_a_way_in_that_nothing_uses`, asking
`pedestal::lands_on(id, floor)`, which today answers "floor 0 of a
`Where::Dungeon`" and nothing else - so the lint is **vacuous until a floor
has an entry**, which is M6. Said out loud in the test's own comment rather
than discovered, per `CLAUDE.md` §6 trap 22. The backward half lands in
`pedestal.rs` at M3.

**5. A1.7's table missed three call sites**, all mechanical:

| Site | What |
|---|---|
| `tests/dungeons.rs:102, :104` | `frame(f)` and `f.band` over `d.floors` - the frame lookups, not the name walk the table lists |
| `tests/two_voices.rs:222` | `PLAIN.landings(d.id, d.landings)` - the *signature* change, not the field |
| `tests/two_voices.rs:243` | reads `Retold.landings`, which does **not** move and needed nothing |

**6. `Theme::landing` falls back per floor, where `landings` fell back per
dungeon.** `landings(id, canonical)` handed back the whole themed list or the
whole canonical one, and `run.rs` then did `.get(floor)` - so a theme that
retold two floors of a three-floor dungeon produced **no landing at all** on
the third. `landing(id, floor, canonical)` falls through to that floor's
canonical line. Strictly better and the same fallback in spirit; nothing in
the tree exercised the old behaviour, because no `Retold` is short.

**7. M1's replay is an engine transcript, not a CLI one.** The spec's M1 exit
asks for "the CLI replay of a scripted run through THE CREVICE". THE CREVICE's
door is the shrine fork at event index 9, met on clearing rung 9, and **no
board the CLI can build from its own verbs clears rung 9**: `preset` loses
there (M0's shallow-ladder table), there is no `skip`, and `sandbox` plus
seventy-five `equip` lines is a script nobody will read a diff of.
`the_six_shipped_dungeons_replay_word_for_word` is what replaced it, and it
is stronger on every axis that matters here - it walks **all six** dungeons
rather than one, it pins the landing prose and the banner as well as the
outcome, and it runs in the suite instead of by hand. Acceptance criterion 1's
CLI transcript is M6's, by which time `throw` and `leave` exist and there is
something to say.

---

## M2 run state, the four transitions, the stack, at `87a391d`+

The points exist. `Run` carries what it has beaten, `Interrupt` has a variant
for standing at a lever, and clearing / throwing / leaving / losing /
re-entering are five transitions with a test each. No content: the fixture is
`common::A_YARD`, four rooms with a fork at the top, four creatures that
already exist in `ALTERNATES`, and it is deliberately **not** in `DUNGEONS`.

### The suite

| | M1 | M2 |
|---|---:|---:|
| Engine passed | 813 | **831** |
| Engine ignored | 41 | 41 |
| GUI | 62 | 62 |
| Warnings | 0 | 0 |

The eighteen are the new binary `tests/switchyard.rs`.

### Still inert where it has to be

| What | Result |
|---|---|
| `baseline` printer, all six | **byte-identical to M0** |
| `catalog_shape::no_creature_changed_what_it_wears` | green, 5,568 placements |
| `acceptance` (10, `e6_1` among them) | green |
| `route::ascii` fixture | green, unmoved |

### Two pins moved, both re-pinned with the reason in the assertion

**1. The dungeon banner gained the creature's name.** The replay fixture
`analysis/replays/dungeons.txt` was re-baselined. The diff is **fourteen
banner lines and nothing else** - `grep -vc 'banner:'` over the diff's changed
lines is **0**. Every one gained the creature between the dungeon's name and
the floor count:

```
- banner: THE THRESHOLD - floor 1 of 3
+ banner: THE THRESHOLD - DOORKEEP - floor 1 of 3
```

Every `floor {n} of {m}` pair came back the same, which is the half that had
to hold: the two numbers changed *meaning* at M2 (from floor index and room
count to which-fight-of-this-entry and how-many-this-entry-is) and a straight
line walked from the top is where the old and new readings agree. That is
acceptance criterion 3's "plus the creature's name", measured.

**2. `road_stack::a_dungeon_sits_on_top_of_whatever_it_was_entered_from`.**
It hand-assigns `run.dungeon = Some((d, 1))` and asserted `"floor 2 of 3"`.
Under the new reading a run *put* on floor 1 without fighting anything has won
no fights and has two ahead of it, so it is on **floor 1 of 2** - and a
hand-assignment into the middle of a dungeon is exactly what a siding does, so
the new answer is the right one. Re-pinned to 1 of 2, with a second run added
beside it that walks in properly and pins **2 of 3**, so the number a real
walk produces is still held down.

---

### Findings

**8. A1.3's walk-through rule skips rooms nobody chose to skip.** The spec
says: *"while `dungeon` is on a floor already in `cleared_floors`, follow it -
one exit, take it; several, take the single uncleared one if exactly one is
uncleared"*. "Uncleared" there means the exit's **next room**, and that is not
the same question as whether the road is finished.

The case, found by `a_road_half_walked_is_still_a_road` and reproduced in
`A_YARD`: a run walks one road as far as its first room and leaves. Coming
back to the fork, that road's next room *is* beaten, so the naive rule reads
"one road left open" and throws the lever down the **other** one - past every
room on the first road that nobody has fought.

Fixed in the engine, not in the test: a road is open when
`fights_ahead(to, cleared) > 0`, which is "there is still a fight somewhere
down it". The two readings agree on every walk the yard's own shape produces,
including A4's worked eight-floor example - which is why the spec's rule looks
right - and they disagree the moment a run leaves half way down a line, which
is a thing `leave_dungeon` exists to let it do.

**9. `enter_dungeon_at` takes the dungeon, not an id.** A1.3 writes
`Run::enter_dungeon_at(&mut self, id, floor)`. A dungeon that exists only in a
test binary cannot be found by `dungeon::by_id`, and M2's whole test list is
"prove the primitive before any content exists", so an id-keyed entry point
would have forced a fixture into `DUNGEONS`. It takes
`&'static Dungeon`; `enter_dungeon(id)` resolves and delegates, and
`feed_pedestal` will do the same at M3. `by_id` is public and every road-side
caller has already resolved one.

**10. `Interrupt::Dungeon` is a struct variant carrying the banner's two
numbers.** A1.4 requires `describe()` to produce `floor {n} of {m}` where both
numbers are readings of the *run* - fights won this entry, and floors walked
past because they were beaten before. `describe(self)` has no run. Putting the
two numbers in the variant keeps one formatter for the banner and the stack
strip, which is what A1.4 wanted; `road_stack()` computes them, and nothing
holds an `Interrupt` across a transition because the stack is derived fresh
every time it is asked for. Nine patterns changed, two of them outside
`run.rs`.

**11. A cleared road out of a cleared floor is an unreachable state the type
allows.** If a siding put a run on a floor whose every road is walked out, the
walk-through would have had nowhere to go and the run would have stood in a
room it had already emptied - and fought the thing it beat. It cannot happen
while a destination fires once a run, which is `pedestal.rs`'s rule, but the
type does not say so. `walk_through_cleared` ends the dungeon and says
`"Walked out of {name} - nothing left in it."` rather than guessing.

**12. `wipe` needed nothing.** It builds a fresh `Run::seeded` and copies four
things across, so the four new fields clear themselves.
`wiping_forgets_the_yard` pins it, because "it happens to work" and "it is
guaranteed" are different, and the next field added to `Run` will be added by
somebody reading that test.

---

## M3 sidings, the CLI verbs, the interface, at `afeff32`+

`Where::Siding` exists and nothing is one yet. The two dungeon verbs are on
the CLI. The GUI has a points screen, a way out, a pip row that counts fights,
and a map label that says how deep a dungeon goes.

### The suite

| | M2 | M3 |
|---|---:|---:|
| Engine passed | 831 | **834** |
| GUI passed | 62 | **65** |
| CLI passed | - | **3** |
| Warnings | 0 | 0 |

`crates/cli/tests/` did not exist before this milestone. The workspace is
**902 tests**.

### Still inert where it has to be

`baseline`'s whole printer output is **byte-identical to M0**, three
milestones in. `no_creature_changed_what_it_wears` green.
`an_orbless_run_meets_a_pedestal_and_nothing_happens` green, which is M3's
own exit criterion.

---

### Findings

**13. The CLI verb replay cannot walk a yard until M6, so M3 built the
harness instead.** The spec's M3 test is
`switchyard::the_cli_verbs_replay` - "a script using `throw` and `leave`
piped twice diffs clean". Neither verb can be exercised through the driver
today: no dungeon in `DUNGEONS` has points in it until M6, and no dungeon is
*reachable* by any board the driver can build from its own verbs, which is
M1's finding 7 again.

What landed is `crates/cli/tests/replay.rs`, **the first test the CLI crate
has ever had**:

- `a_scripted_run_replays_identically` - a fourteen-line script piped into the
  built binary twice, byte-compared. It walks the preset board up eight rungs
  to a town gate and prints the map.
- `throw_and_leave_say_no_from_the_road` - both verbs asked from where they
  are illegal, so their refusals are in a transcript rather than in nobody's
  memory.
- `help_lists_the_two_new_verbs` - a help line for a verb that does not exist
  is worse than no help line, and two verbs were added here.

This is worth more than the test it replaces. `post-unwinding.md` §1 records
that the two CLI replays `HANDOFF.md` cites "were not re-run here (the CLI
builds; nobody wrote the script down) - **unverified**". The script is written
down now and it runs in the suite. Acceptance criterion 1 extends this file at
M6 rather than inventing a harness under deadline.

**14. M1's half-lint is whole.** `pedestal::lands_on` answers for
`Where::Siding`, so `dungeon::no_floor_offers_a_way_in_that_nothing_uses` has
something to say the moment a floor carries an `entry`. The other direction -
every siding lands on a floor that has one - is asserted in `pedestal.rs`'s
own test module, beside the range check, because that is where `Where` lives.

**15. A lint the spec does not ask for: `no_two_sidings_land_on_the_same_
floor`.** Two orbs pointing into one dungeon is the design - each line's
buffer stops pay the ticket to the *other* line - so the existing "no two
destinations share an id or an orb" does not catch two sidings written onto
one floor. The second would be refused by the visited-set at the pedestal
while looking like a fresh ticket, which is the worst kind of dead content:
one that costs the player an orb to discover.

**16. The GUI's layout is four pure functions, for the reason `chip_rects`
is one.** `dungeon_banner`, `pip_row`, `ticked_pips` and `points_cells` take
no font context, so `cargo test -p gearmaster-gui` reads the geometry and the
words without macroquad. `the_pip_row_is_the_banner_read_as_circles` pins the
one that matters: the pips and the banner were two formatters over
`d.floors.len()` and they agreed because they were the same expression twice.
They are a graph's reading now, and the row has to be the banner's `m` or the
circles and the words over them describe different walks.

**17. Three places now read one banner.** `Interrupt::describe()` is the only
formatter: the GUI's opponent panel, the GUI's pip row and the CLI's
`show_road` all read it, which is what A1.4 asked for and the change M3 needed
anyway.

**18. `no_destination_is_a_siding_yet` is a test that says the plumbing is
inert.** M5 and M6 are where the content lands, and a green suite over an
unused variant is easy to mistake for a shipped feature. The test names the
milestone that should delete it.

### The leave button

`leave_strip` is one shape and one sentence in one function, offered on two
screens - the landing and the points. `LEAVE_BLURB` is the spec's wording
verbatim: *"What you cleared stays cleared. The door does not reopen."*
`the_way_out_says_what_leaving_costs` pins both halves, because the second one
is the cost and a blurb that loses it is a blurb that makes leaving look free.

---

## M4 four actions, four weights, four rows - inert, at `0386d48`+

`Shunt`, `Ballast`, `Derail` and `Accrue` are in the engine, priced, homed in
the basis, described in three interfaces and tested to the tick. **No
component speaks any of them**, which is the whole design of the milestone and
the thing its exit criterion measures.

### The exit criterion, measured

| | Result |
|---|---|
| The four-board table at Medium | **byte-identical to M0** |
| Every other `baseline` figure - cadence, mind, rungs 1-14, no-weapon, slot-mattering, census | **byte-identical to M0** |
| `gear_at`, every creature at every difficulty | **5,568 placements unmoved. 0 creatures re-geared.** |
| `acceptance` (10, `e6_2` among them) | green |

A2.5's argument was that four weights added for verbs no piece speaks price
six components and re-gear nobody, because `stepped_component` filters
event-only pieces out of every footprint family and nothing at all speaks the
verbs yet. That is now a fixture diff rather than a sentence.

### The ratchet

`report_shape` differs from M0 by **exactly four rows**, and every one reads
`0/0`, 0 away, budget 0:

```
Ballast                        0/0    0    0
Derail                         0/0    0    0
Shunt outside the weapon       0/0    0    0
Accrue                         0/0    0    0
```

Every other row and every quota is where M0 left it.

### The suite

| | M3 | M4 |
|---|---:|---:|
| Engine passed | 834 | **848** |
| GUI passed | 65 | 65 |
| CLI passed | 3 | 3 |
| Warnings | 0 | 0 |

### The numbers as landed

| Effect | Variant | Home | Rule | Weight |
|---|---|---|---|---|
| Shunt | `Shunt { ms }` | Greaves, tempo | Only, shared with Weapon | `SHUNT_PS = 3.0` /s |
| Ballast | `Ballast(n)` | Chest, reserve | Only | `HEALTH x TYPICAL_FIGHT_S x BALLAST_FUNDED (0.66)` |
| Derail | `Derail { window_ms, back_ms }` | Gloves, reaction | Mostly(70) | `DENIAL_S x AIMED x DERAIL_WINDOW (0.4)` |
| Accrue | `Accrue { what, pct }` | Helmet, economy | Mostly(70) | `ACCRUED_ASSUMED (30) x pct/100 x pool_weight` |

All four are starting points and M8 is where they are measured.

---

### Findings

**19. `assembly::every_action_is_well_formed` did not exist.** The spec's M4
scope extends it; `grep -rn well_formed` over the tree finds nothing but the
comment this milestone wrote. It exists now, and it carries half of what the
spec asked of it - see the next finding.

**20. `Derail`'s bad case is prevented by the type, not by a lint.** A2.3 asks
the well-formedness test to refuse `Derail { target: Yourself }`, "because
there is no reading of it that is not a stun on your own bar". If there is no
reading of it, the type should not be able to write it: `Action::Derail`
carries **no target** and always reads the front foe. A lint that can only
ever pass is a type that should have said so, which is `CLAUDE.md` §6 trap 22
read from the other end. The lint keeps the half that is genuinely
representable - `Accrue` on a fused pool - and says in its own doc comment why
the other half is missing.

**21. A rule with no carriers is what `every_rule_names_a_mechanic_that_
exists` exists to catch, and M4 lands four.** The phase discipline requires
the rows before the pieces; the lint says "a rule matching nothing at all is a
typo that would sit here reading green forever". Both are right.

Resolved with a **two-way ratchet** rather than by loosening the lint:
`RULES_AWAITING_THEIR_PIECES` names the four and the milestone that empties it,
`every_rule_names_a_mechanic_that_exists` skips exactly those, and
**`no_rule_waits_for_a_piece_that_has_arrived` goes red the moment any of them
finds a carrier** - so M5 cannot land the components without taking the names
off the list, which puts the rows back under the lint they were exempted from.
An exemption that outlives its reason is a lint with a hole in it.

**22. Two guards where the spec asks for one.** `Accrue` on a fused pool is
refused by the catalogue lint *and* by `combat::apply`, which returns without
banking. A rule only a lint enforces is a rule a hand-built `ItemProfile`
walks straight through, and every test in this milestone is a hand-built
`ItemProfile`.

**23. A shunt owes only what it managed to give.** A2.1 caps the target's
`progress_ms` at `cooldown_ms - TICK_MS` and adds `ms` to the source's debt.
Read literally that charges the giver for time that went nowhere: a bar
already near the top takes less than was offered, and the difference would
vanish. The debt is the amount that actually landed, so time is conserved
against the cap as well as across it, and
`shunt_moves_time_and_conserves_it` measures total bar-fill on both sides of
the trade rather than trusting the arithmetic.

**24. `Combatant::player` starts every pool and the wall at zero, whatever
`Stats` says.** Which means a hand-built profile testing armour or mana has to
*bank* it in the fight, and - the part that cost twenty minutes - a player
built from `Stats::ZERO` has **zero maximum health and is dead on the first
tick**. Every count read off such a fight is zero, which reads exactly like
"the mechanic does nothing". `effects.rs` and `reactions.rs` now carry an
`ALIVE` constant with a doc comment saying so, because the next person to
hand-build a profile will hit it too.

**25. `Event::Grew` gained a field rather than a sibling.** `Run::settle` sums
`Event::Grew { amount, .. }` over the log into `grown_health`, so ballast
compounds across a run exactly as `Grow` does **with no new arm anywhere** -
which is the argument for the field, and `ballast_banks_as_growth` is the test
of it rather than a comment claiming it.

---

## M5 the catalogue lands once, at `d648df5`+

Eight components appended in one block at the end of `CATALOG`: four
enchantments, two orbs, two words. All eight event-only.

### The exit criterion, measured

**`gear_at` matches the M0 fixture. All 5,568 placements. 0 creatures
re-geared.** That is A6's whole claim - eight event-only components cannot
re-dress anybody, because `stepped_component` filters event-only out of every
footprint family - and it is now measured at the milestone that could have
broken it.

The four-board table at Medium is **unmoved**, and so is every other figure
`baseline` prints. The only diff in the whole printer is the census:

| slot | M0 | M5 |
|---|---:|---:|
| Helmet | 96 | **99** |
| Chest | 71 | **72** |
| Gloves | 83 | **84** |
| Greaves | 67 | **68** |
| Weapon | 187 | **189** |
| **total** | **504** | **512** |

### The ratchet

Every exclusivity row is **0 away at budget 0**, and every quota is **0 away**
- the shares moved by fractions of a percent on 512 pieces and every one
stayed inside its band. The four rows M4 landed empty now carry:

```
Ballast                        1/1    0    0
Derail                         1/2    0    0
Shunt outside the weapon       1/2    0    0
Accrue                         1/1    0    0
```

`RULES_AWAITING_THEIR_PIECES` is empty, which is what M4's second ratchet
forced: `no_rule_waits_for_a_piece_that_has_arrived` was red until the four
names came off, which put the rows back under the lint they were exempted
from.

### The suite

| | M4 | M5 |
|---|---:|---:|
| Engine passed | 848 | **853** |
| GUI | 65 | 65 |
| CLI | 3 | 3 |
| Warnings | 0 | 0 |

---

### Findings

**26. `PieceKind::Orb` is twenty-three pieces over eight footprints, not the
four Orbs of Travel.** A6 says "the four shipped orbs use the 2x2 square and
the plus", which is true of the four *Orbs of Travel* and not of the kind.
Shunter's Orb was drawn as a T-tetromino and the T is already Timeworn Orb's
and Spinning Orb's. It is an L now - `(0,0),(0,1),(0,2),(1,2)` - which no
other Orb carries, and `no_orb_in_the_catalogue_shares_a_footprint_with_
these_two` walks all twenty-three rather than the four.

The claim never had to hold, because both orbs are event-only and
`stepped_component` skips them either way. It is held anyway so the guarantee
does not *depend* on the exemption, which is the difference between a rule and
a coincidence.

**27. The shop's shelf tilt counted the catalogue rather than the pool it was
dealing from.** A real bug, and the eight unsellable components are what
exposed it: `avail::the_shelves_are_not_the_same_few_things_every_time` went
red at 4.1x spread against a 2.0x bound, because the shelves shifted when the
catalogue grew *in places no shelf can reach*.

`Shop::restock` builds a pool, filtering out boss gear, quest rewards,
event-only pieces, town stock and the mind lane while it is locked. It then
deals slots round-robin on `n.powf(SHELF_TILT)` tickets - and `n` was
`CATALOG.iter().filter(|d| d.slot == k).count()`, the **whole** catalogue. So
a slot was dealt in proportion to how much of it exists rather than to how
much of it is for sale, and the two have not been the same number since the
Unwinding appended thirty-one event-only rewards.

Fixed to count the pool. **The blast radius was zero**: the distribution test
went green and no other test in the suite moved, which is the measurement that
made it safe to take inside a catalogue-only milestone rather than record and
defer.

**28. The theme's piece names had to land at M5, not M7.**
`theme::the_turtle_theme_covers_the_catalogue` requires every component to
have a turtle name, so it went red the moment the block landed and would have
stayed red for two milestones. That is not a scheduling accident - it is the
gear skill's own rule (`.claude/skills/gearmaster-gear`: author the piece "and
give it its Turtle Dick name in the same change"). The eight `pieces` entries
are in. M7 still owns the scenes, the creatures and the effect vocabulary,
which is the part Part C is actually about.

**29. Two pins moved, both re-pinned with the reason in the assertion.**

- `avail::town_gear_is_reachable_and_only_in_a_town` read "every piece that is
  town stock is on the cart". A3 breaks exactly that: the four are town stock
  - they are ground, and `shop.rs` refuses them on the road for that reason -
  and are on no cart, because they are what a four-fight line pays and a shelf
  is a purchase. Narrowed to town stock that is not event-only, and **widened**
  in the other direction: everything on the cart must be town stock and must
  not be dug up, which the old loop could not see because it walked the
  catalogue rather than the cart.
- `primitives::every_rumour_and_the_trophy_trade_is_a_quest_item`: nine quest
  items became **eleven**. The chain is seeded by two words, both `Carried`
  and both bought from a door rather than sold at the bar, because `SHELVES`
  is six names and `SHOP_SIZE` is six - the pub is full (Part E, E-2).

---

## M6 the chain, the yard, the frames, the destinations, at `13ecad2`+

Two rumours, four doors, nine floors, nine frames, nine undressed creatures
and two sidings. The first content of the mission that a player can walk.

### The exit criterion, measured

**Zero authored `gear:` boards.** `grep` over the nine new `MonsterSpec`s
finds `gear: &[]` nine times and `gear: &[(` never. The four-board table is
byte-identical to M0 and `gear_at` still matches the M0 fixture, which is what
it should be: nine creatures with no boards cannot move anybody's gear.

The chain walks end to end in both modes
(`the_chain_can_be_walked_in_one_run_in_either_mode`): buy the sheet, ask for
the points, step onto the turntable, walk a line to its buffer stop, spend the
ticket it paid on the other line, walk that one, and tell Ambrose both -
`counted("sidings-cleared") == 2` and the underwriter signs.

**Nine rooms, and the most a run can fight is eight.**
`nine_floors_and_the_most_a_run_can_see_is_eight` walks the yard greedily -
in at the mouth, back in by every siding a ticket can pay for, taking whichever
road still has a fight down it - and counts **8 distinct floors**, with the one
left over always a buffer stop. That is the property the whole floor graph was
built for, and it is now a fact about the tables rather than a claim.

### The suite

| | M5 | M6 |
|---|---:|---:|
| Engine passed | 853 | **860** |
| GUI | 65 | 65 |
| CLI | 3 | **5** |
| Warnings | 0 | 0 |

### The census

| | M5 | M6 |
|---|---:|---:|
| `EVENTS` | 33 | **37** |
| `DUNGEONS` | 6 | **7** |
| `RUMOURS` | 8 | **10** |
| `DESTINATIONS` | 4 | **6** |
| `FRAMES` | 15 | **24** |
| `ALTERNATES` | 19 | **28** |
| creatures | 69 | **78** |

---

### Findings

**30. Two of the four doors could not stand where the spec drew them.** A0
lists the free indices between Kettleworks and the Slagworks as 18, 20, 25, 27,
32 and 34, which is true of *events* and takes no account of towns. Town gates
stand on rungs 7, 14, 18, 25, 32 and 34, and
`town::no_town_shares_a_rung_with_an_event` has refused a door on a gate's rung
since before this mission - "both would want the screen, and there is no
sensible order for that".

So the intersection that is free of both is **20 and 27**, and two of the four
doors had to move:

| Door | Spec | Landed | Why |
|---|---:|---:|---|
| THE TIMETABLE | 18 | **20** | Kettleworks stands after 17 |
| THE SIGNAL BOX | window from 20 | window from **21** | the timetable took 20 |
| THE TURNTABLE | 27 | 27 | unchanged |
| THE LAST TRAIN | 32 | **33** | High Wick stands after 31 |

The spec's argument for 18 - "the stack pops the gate first, which is the shape
Sump Bottom and the first fountain already share at index 7" - is true of a
*fountain* and does not transfer: the lint only forbids events, and a fountain
is not one. THE LAST TRAIN at 33 shares its rung with THE EXHIBITION, which is
a window rather than an address, and two doors on a rung is a pairing the road
already has three of.

**31. `every_scene_names_something` caught a fourth instance of the blind spot
Part B warns about.** Part B audits its own strings against a reimplementation
of the lints and reports three failures, all `names_something`, all repaired.
It missed one: THE YARD THROAT's fork scene named Ambrose only at a full stop
("...it has been pulled. Ambrose pulled it. He did not say which way."), and a
name that only ever opens a sentence is invisible to the predicate.

Repaired the way Part B repairs the other three - one clause putting the name
past the first word - and **repaired in the spec as well**, so the document and
the engine do not hold two versions of a scene, which is what Part B asks for
by name.

**32. A generic road-walker has to know how to throw a lever.**
`two_runs::play` walks the road answering whatever door is open, and three of
its walks stalled at rung 26. Not a balance problem: the walk answered THE
TURNTABLE by taking its first open choice, stepped into the yard, cleared the
throat, and then stood at the points for forty iterations because nothing in
the helper could decide which way. It throws the first road with a fight down
it now. Any other walker that reaches a dungeon will need the same.

**33. The M1 replay fixture hung the suite for six minutes.**
`the_six_shipped_dungeons_replay_word_for_word` walks every dungeon in
`DUNGEONS` with `while let Some(_) = run.dungeon`, and THE SWITCHYARD never
ends without somebody throwing the points. It is filtered to the six it is a
fixture for - the yard's walk is `switchyard.rs`'s, where there is something to
decide - and the loop carries a bound, because a dungeon that cannot be walked
out of is a hang and a hang is a worse bug than a wrong room.

**34. Five linear-dungeon assumptions surfaced at once**, all in tests that
were correct about a list and wrong about a graph:

- Bands had to rise **along the list**; they now have to rise **along every
  road out**, because the yard's floor 5 is band 28 after floor 4's 30 and the
  two are different lines.
- Theme uniformity: the yard is exempted beside WUMPUS WORLD, and for a
  different reason - it is not a creature at all but a place, and the two lines
  are meant to read differently in the first three seconds.
- `every_dungeon_pays_something` needed "or every buffer stop pays its own
  way", which M1 had already taught `dungeon.rs`'s copy and not this one.
- The ascii-map fixture compared **lengths**; it is a subsequence check now,
  because the road is longer and what M1 promised is that the old lines did not
  move.
- `the_shipped_banner_did_not_change` walked all of `DUNGEONS`; it walks the
  six and pins the yard's `floor 1 of 4` separately.

**35. `switchyard.rs`'s own fixture run stood on rung 20**, which THE TIMETABLE
now occupies, so half the file failed on `road_is_blocked` finding a door
instead of the points. It stands on rung 43 - no scheduled event, no gate, no
fountain - because every test in that file is about what a dungeon does to the
road, and the road underneath has to be empty or the measurement is of
something else.

**36. Four phase-discipline budgets went red together**, and all four are
tied to `bestiary::unpacked()` rather than to a copied list of nine names, so
M9 clears them by packing rather than by editing:

- `bestiary::UNDRESSED` 0 → **9**. **The only budget in the repository that is
  allowed to go up**, and it says so in its own doc comment.
- `acceptance::e6_8` now asserts the undressed set *equals* the yard's nine.
- `phase_two`'s copy asserts nothing of *the Unwinding's* is undressed.
- `gui::pack`'s toothless lint skips what has not been packed - a creature with
  no board lands nothing because it has nothing to land it with.

---

## M7 the turtle telling, at `37a176f`+

Part C into `theme.rs` and nowhere else. Five creature names, four event
titles, the dungeon's title, blurb and entry, and four effect words. The eight
component names went in at M5, because the gear skill says a piece gets its
themed name in the same change that writes the piece.

### The suite

| | M6 | M7 |
|---|---:|---:|
| Engine passed | 860 | **861** |
| GUI | 65 | 65 |
| CLI | 5 | 5 |
| Warnings | 0 | 0 |

`two_voices` green with **the budget still 5** - this mission spends none of
it, because every canonical string it wrote was checked against
`two_voices::BOOK` before it was written. `no_road_id_is_told_twice` green.

### What the theme says, and what it deliberately does not

| Canonical | Turtle |
|---|---|
| THE SWITCHYARD | THE CORK TRAIN YARDS |
| THE TIMETABLE | THE CORK TIMETABLE |
| THE SIGNAL BOX | THE SPROCKETMAN IN THE BOX |
| THE TURNTABLE | THE TURNTABLE ON THE YONK STANDARD |
| THE LAST TRAIN | THE LAST CORK TRAIN |
| THE SHUNTER | THE CORK SHUNTER |
| THE PLATELAYERS | THE SPROCKETMEN WHO KEPT THE LINE |
| THE BALLAST | WHAT THE EMPIRE LEFT IN THE PIT |
| THE GANTRY | THE ELEVEN CORK SIGNALS |
| THE LAMP ROOM | THE ROOM WITH EVERY LAMP LIT |
| Ballast, Derail, Accrue *(effect words)* | cork-ballast, skoogle, fnorp-interest |
| Shunt *(effect word)* | **kept** - a railway word is a railway word on any plane |
| THE COAL STAGE, THE WATER TOWER, THE GOODS SHED, THE ROUNDHOUSE | **kept** |

`the_turtle_theme_retells_the_yard` pins the split both ways: the five that are
renamed must be, and the four that are kept must stay kept. A half-finished
table is a failure there rather than a run in two voices.

The road was read aloud (`prose::read_the_road_aloud`) and the yard reads
correctly: blurb, entry, nine landings, three fork scenes and six exit blurbs
in the order a player meets them.

---

### Findings

**37. E-6 was never answered, and M7 shipped the proposed names.** Part C was
written without the book PDF or the titles CSV, and six of its rows are marked
*proposed*: the chain name, the dungeon name, Hesketh's themed role, the two
orbs' names and the effect words `skoogle` and `fnorp-interest`. The kickoff
said not to stop for input, so they are in as written.

**Nothing is blocked by this and nothing downstream depends on it.** Every one
is display-only, `theme.rs` is a lookup with a fall-through, and replacing any
of them is one line and no test. The rows to replace first, in order, are the
ones Part C names: the chain name, the dungeon name, the two orbs, then the two
effect words.

**38. Hesketh and Ambrose stay roles in the canonical column, so the theme
spends no paragraphs on the four doors.** `Retold.prose` is empty for all four
and only the titles are themed. That is the rule `retell` exists for - a common
noun is fixed in place and the vocabulary puts the themed word back; a theme
spends paragraphs only where a *proper noun* is carrying a scene - and the
prose pass already moved this game's proper nouns out of the canonical column.
The dungeon gets prose because the Empire is carrying it.

**39. Per-floor siding entries are left canonical**, which is Part E's E-3
taken as recommended. `Retold` carries one `entry` per dungeon and the two
siding lines are per floor; both name no proper noun, so `two_voices` has
nothing to catch and a missing entry falls through by design. `Retold.sidings`
was not added. If the book ever supplies a line worth the code, that is the
field to add.

---

## M8 rating pins, at `ab264a7`+

Two of the four weights were starting points and are measurements now. Both
moved, and both moved because a fight said so rather than because a price
needed hitting.

### The two that moved

| Weight | Was | Is | What measured it |
|---|---:|---:|---|
| `DERAIL_WINDOW` | 0.4 | **0.79** | 65 activations against the four creatures at bands 27-30 |
| `BALLAST_FUNDED` | 0.66 | **0.87** | 24 activations across nine armour-income configurations |

**`DERAIL_WINDOW` was the right arithmetic for the wrong question.** 0.4 is
the share of a *single* item's duty cycle that a 1,000 ms window covers on a
2,500 ms board. A creature at these bands wears fourteen to twenty-six items,
and the chance that one of them is within a second of firing is nearly one:

| creature | items | activations | caught |
|---|---:|---:|---:|
| Obsidian Colossus | 23 | 16 | 16 (100%) |
| Null Sentinel | 14 | 17 | 10 (59%) |
| Silence | 14 | 16 | 12 (75%) |
| Weeping Idol | 26 | 4 | 4 (100%) |
| **overall** | | **53** | **42 (79%)** |

Still a discount rather than 1.0, and the discount is the thin boards - which
is the honest shape, because a creature with three items is exactly the one a
denial is worth least against.

**`BALLAST_FUNDED`'s first measurement was zero.** A wall granted once at the
bell is gone before a five-second chest item comes round, because the creature
is hitting you and armour absorbs first. Every one of nine configurations read
0.00. That is a true fact about a build with no armour *income*, and the wrong
build to price against: the discount is for "what a build that wanted it will
manage", and a build that wants Ballast wants income. Re-probed with income:

| asked | wall 10 | wall 30 | wall 60 |
|---:|---:|---:|---:|
| 10 x8 | 1.00 | 1.00 | 1.00 |
| 20 x8 | 0.50 | 1.00 | 1.00 |
| 30 x8 | 0.33 | 1.00 | 1.00 |

Mean **0.87**. The two shortfalls are where the income cannot keep up, which
is the condition the discount is for.

`SHUNT_PS = 3.0` and `ACCRUED_ASSUMED = 30` were checked and **not moved**:
Points Rodding rates 44 and Booking Hall 33, both inside the shipped
enchantments' spread, and neither weight had a measurable claim behind it that
a fight contradicts.

### The six, priced

| Component | Rating | Price | Shipped band |
|---|---:|---:|---|
| Ballast Bed | 59 | 58 | 34-60 |
| Points Rodding | 44 | 54 | 34-60 |
| Booking Hall | 33 | 60 | 34-60 |
| Signal Wire | 26 | **60** (was 62) | 34-60 |
| Shunter's Orb | 7 | 24 | 20-26 |
| Signalman's Orb | 9 | 22 | 20-26 |

All six inside. Signal Wire came down two gold, which is the only price that
moved: 62 is outside a band the shipped six have held since the Unwinding, and
60 is Chalked Circle's, the dearest ground in the game.

### Nothing re-geared

**`gear_at` matches the M0 fixture. The four-board table is byte-identical.**
`only_the_yards_own_six_speak_the_verbs_the_new_weights_price` is why that is
allowed to be true: the four weights price four verbs, exactly six components
speak them, and all six are event-only - so `stepped_component` cannot reach
one and a weight moving at M8 can only move those six ratings.

### The suite

**863 engine, 65 GUI, 5 CLI, 43 ignored, 0 warnings.**

---

### Findings

**40. The spec's band is a price band, not a rating band.** M8 asks that each
enchantment "rate within its slot's existing enchantments' band (Chalked
Circle 60 is the dearest; the Lightning Rod 34 the cheapest)". Those two
figures are **prices**: Chalked Circle rates 32 and is priced 60; the Lightning
Rod rates 30 and is priced 34. Read as a price band it is 34-60 and all four
land inside it, which is what
`the_yards_six_are_priced_like_the_things_they_are` pins.

Read as a *rating* band it is 30-50, and two of the four sit outside it:
Ballast Bed at 59 and Signal Wire at 26. Left there rather than tuned back,
because the weights behind both are measurements now and a measured weight
bent to hit a band is not a measurement. Both readings are recorded so the
next person does not have to re-derive which one the spec meant.

Nothing downstream cares: `RARE_AT` is 90, so 26 and 59 are both Common like
499 of the other 511, and rarity is the only thing a rating feeds that a
player sees.

---

## M9 boards, by hand, at `a2f87ed`+

All nine floors of THE SWITCHYARD are dressed. The frame lint is back to zero
for the first time since M6.

### What the packer wanted and what it got

`PACK_MONSTER=<name> PACK_RUNG=<band+1> cargo test --release --test pack_francis pack`,
one creature at a time, ~103 s each on this machine. The gate is the owner's
board at Medium against the curve for the floor's band.

| Floor | Band | Owner @ Medium | Curve target (+/-30%) | Cells | Pieces |
|---|---:|---:|---|---:|---:|
| THE SHUNTER | 27 | **10.5 s** | 14.0 s (9.8-18.2) | 68/240 | 24 |
| THE PLATELAYERS | 28 | **12.0 s** | 14.4 s (10.1-18.7) | 67/240 | 16 |
| THE BALLAST | 29 | **12.0 s** | 14.8 s (10.4-19.2) | 65/240 | 16 |
| THE COAL STAGE | 30 | **12.0 s** | 15.2 s (10.6-19.8) | 66/240 | 15 |
| THE WATER TOWER | 30 | **12.0 s** | 15.2 s (10.6-19.8) | 64/240 | 16 |
| THE GANTRY | 28 | **12.0 s** | 14.4 s (10.1-18.7) | 65/240 | 16 |
| THE LAMP ROOM | 29 | **12.0 s** | 14.8 s (10.4-19.2) | 66/240 | 17 |
| THE GOODS SHED | 30 | **12.0 s** | 15.2 s (10.6-19.8) | 67/240 | 17 |
| THE ROUNDHOUSE | 30 | **12.0 s** | 15.2 s (10.6-19.8) | 66/240 | 16 |

Every one inside its band, and the packer's own guards passed on all nine at
the default 300 trials - no creature needed a second run or a scaled dial.

### The diff, read

`CLAUDE.md` §6 trap 15: `make pack`'s save rewrites `combat.rs` in place and
once rewrote a creature nobody was editing. This milestone did not use it. A
targeted splice found each spec by its `name:` line, walked to that spec's own
closing brace, and replaced only the two fields inside it. The whole
milestone's deletions from `combat.rs`, in full:

```
   9 -        gear: &[],
   9 -        items: &[],
```

Eighteen lines, nine creatures, nothing else touched.

### The one re-baseline

`gear_at` was re-baselined - the first time since M0, and the only legitimate
occasion for it: nine creatures went from no board to a packed one. **Every
changed line in the diff names one of those nine.** No `LADDER` creature moved
and no other alternate moved, which is what the fixture exists to prove: the
catalogue grew by eight components between M0 and M9 and had eight chances to
re-sort a footprint family underneath somebody nobody was editing.

**The four-board table at Medium is still byte-identical to M0**, nine
milestones in.

### The four budgets, closed

| | M6 | M9 |
|---|---:|---:|
| `bestiary::UNDRESSED` | 9 | **0** |
| `acceptance::UNDRESSED_UNTIL_THE_YARD_IS_PACKED` | 9 names | **empty** |
| `phase_two`'s dressed check | yard-shaped hole | **whole again** |
| `gui::pack`'s toothless lint | skipped the naked | **skips nothing** |

None of the four needed editing to *close* except by lowering a number: all
four read `bestiary::unpacked()` rather than a copied list, which is why
packing the ninth creature closed three of them and made the fourth go red
until its number came down.

### The suite

**863 engine, 65 GUI, 5 CLI, 43 ignored, 0 warnings.**

---

### Findings

**41. `floors.last()` is an index, not an ending.**
`progression::every_alternate_is_a_finished_creature` exempts dungeon floors
from the "a named creature leaves a trophy" rule, except the *last* floor,
which is where a linear dungeon's reward fires. A graph has as many endings as
it has buffer stops - the yard has four - and `floors.last()` is simply floor
8, which THE ROUNDHOUSE happens to hold. It failed with "THE ROUNDHOUSE leaves
nothing behind" while THE COAL STAGE, THE WATER TOWER and THE GOODS SHED, its
three equals, went unasked.

Re-pinned on what the rule is actually about: a floor that pays on its own
through `Floor::also` has already left something behind, whatever `drops`
says. The yard's four buffer stops each pay ground and a ticket, which is more
than a trophy and is the whole of what a graph asks.

**42. The packer's diagnostics read alarming and are not.** Every one of the
nine prints two lines of `board want W42.0s ... got L4.3s`: the starter and
preset reference boards *lose* to these creatures in four to nineteen seconds.
That is correct and shipped behaviour - starter clears 2 of 50 rungs and preset
9 of 50, and the yard stands at displayed rung 26-28 - and the packer says so
itself on its first line, `FRAME: no board to regress, so the preset guards do
not apply`. The gate is the owner's board, and the owner wins all nine inside
the band.

Three of the nine run past sudden death on **Insane** (THE COAL STAGE 32.0 s,
THE ROUNDHOUSE 35.0 s, THE GANTRY 35.0 s against the owner). Those are the
clock's, not the board's, and Insane is not what the curve is measured at.
Recorded rather than tuned: the gate is Medium, and pulling Insane back inside
30 s would mean weakening a board that is correct at the setting the game is
built around.

---

## M10 balance, measured, at `1cb611a`+

> **The mission, in one block.** Eleven milestones, M0 to M11, on branch
> `switchyard` off `e38d968`. **866 engine tests, 65 GUI, 5 CLI - 936 in the
> workspace, 44 ignored, 0 failed, no warnings.** All twelve acceptance
> criteria met.
>
> The four-board table at Medium is **byte-identical to M0** - owner 48/50 at
> 75.5% and 9.00 s median, friend 48/50 at 97.4% and 8.15 s, preset 9/50,
> starter 2/50 - and `gear_at` moved exactly once, at M9, when nine creatures
> went from no board to a packed one, and every changed line named one of the
> nine.
>
> Nothing is blocked. Four things are recorded as open rather than done: nobody
> has played it; E-6's six turtle names are guesses without the book; the
> owner's board cannot tell the yard's two lines apart at Medium; and five
> floors run past sudden death on Insane. All four are in
> `design/HANDOFF-switchyard.md` §8.
>
> **`main` is untouched and `docs/` is unpublished.** Merging is the owner's.

### The yard, floor by floor, at Medium

| # | Floor | Band | perfect | owner | friend |
|---:|---|---:|---:|---:|---:|
| 0 | THE SHUNTER | 27 | W5.1 s | **W10.5 s** | W6.0 s |
| 1 | THE PLATELAYERS | 28 | W5.1 s | **W12.0 s** | W7.6 s |
| 2 | THE BALLAST | 29 | W5.5 s | **W12.0 s** | W7.6 s |
| 3 | THE COAL STAGE | 30 | W5.5 s | **W12.0 s** | W7.6 s |
| 4 | THE WATER TOWER | 30 | W5.5 s | **W12.0 s** | W6.0 s |
| 5 | THE GANTRY | 28 | W5.1 s | **W12.0 s** | W7.6 s |
| 6 | THE LAMP ROOM | 29 | W5.5 s | **W12.0 s** | W6.0 s |
| 7 | THE GOODS SHED | 30 | W5.5 s | **W12.0 s** | W6.0 s |
| 8 | THE ROUNDHOUSE | 30 | W5.5 s | **W12.0 s** | W6.0 s |

Every board wins every floor and **nothing is decided by the clock**: the
slowest fight in the yard is 12.0 s against a sudden death that starts at 30.

`a_full_yard_at_medium_finishes_inside_sudden_death` is the assertion, and it
walks all nine rather than the five the criterion names.

### The owner's board at every setting

| Floor | Easy | Medium | Hard | Insane |
|---|---:|---:|---:|---:|
| THE SHUNTER | W9.0 | W10.5 | W14.0 | W19.5 |
| THE PLATELAYERS | W10.0 | W12.0 | W24.0 | **W38.0** |
| THE BALLAST | W10.5 | W12.0 | W22.5 | **W37.5** |
| THE COAL STAGE | W10.4 | W12.0 | W18.2 | **W32.0** |
| THE WATER TOWER | W10.0 | W12.0 | W20.0 | **W33.0** |
| THE GANTRY | W10.0 | W12.0 | W18.0 | **W35.0** |
| THE LAMP ROOM | W9.0 | W12.0 | W18.0 | **W34.5** |
| THE GOODS SHED | W9.0 | W12.0 | W16.5 | W26.0 |
| THE ROUNDHOUSE | W10.5 | W12.0 | W20.0 | **W35.0** |

### The twelve acceptance criteria

| # | Criterion | State |
|---:|---|---|
| 1 | Determinism - a script replays identically | **met at engine level, not through the CLI** - see finding 46. `switchyard::the_full_walk_replays_identically` over `analysis/replays/switchyard-full.txt`; `cli::a_scripted_run_replays_identically`; `acceptance::e6_1` green |
| 2 | No regression - four-board table, `gear_at`, rungs 1-14 | **met**; the table is byte-identical to M0 ten milestones on |
| 3 | The primitive is inert for six dungeons | **met with one word re-pinned** (M1 finding 1) |
| 4 | The chain is completable at Medium in both modes | **met**, `the_chain_can_be_walked_in_one_run_in_either_mode` |
| 5 | Eight of nine, and the ninth named | **met**, `nine_floors_and_the_most_a_run_can_see_is_eight` |
| 6 | Every leaf finishes inside the measurable region | **met**; slowest fight 12.0 s against 30 |
| 7 | The four effects do what their sentences say; on no creature's board | **met**, eight effect tests + `only_the_yards_own_six_speak_the_verbs...` |
| 8 | The ground is dug up and never sold | **met**, `the_yards_ground_is_dug_up_and_never_sold`, `avail` over 400 runs |
| 9 | Leaving costs what the blurb says | **met**, both modes |
| 10 | Phase discipline auditable | **met**; frame lint red M6 to M9, zero after |
| 11 | Every gold figure a `Pay`/`Purse` multiple | **met**, `every_figure_the_chain_deals_in_is_a_multiple_of_a_bounty` |
| 12 | Suite green, no warnings, every re-pin justified | **met** |

### The suite

**866 engine, 65 GUI, 5 CLI - 936 in the workspace. 44 ignored, 0 failed, 0
warnings.**

---

### Findings

**43. Five of nine floors run past sudden death on Insane**, at 32.0 to 38.0 s
against the owner's board. Those fights are the clock's, and they are left
there. The curve is defined at Medium, the packer's gate is the owner's board
at Medium, and every floor is 12.0 s or better there. Pulling Insane back
inside 30 s would mean weakening nine boards that are correct at the setting
the game is built around, to fix a setting whose own doctrine is that it steps
the gear before it touches any number.

Worth knowing rather than worth fixing, and it is not new: `post-unwinding.md`
§5 records the same shape on the no-weapon clears of rung 15.

**44. The owner's board kills eight of the nine in exactly 12.0 s.** Not a
bug and not a coincidence: the nine carry the same four resistances at four
adjacent bands, and the owner's board is a 6.60/s cadence that lands its
killing blow on the same activation of the same cycle against all of them. It
is a sign the nine are more alike than the two lines' themes suggest, and it is
the first thing a balance pass after this one should look at - the Down line is
supposed to be weight and the Up line light, and at Medium the owner cannot
tell them apart.

**45. `A_PERFECT_RUN` is not the preset board.** It is a finished run's, and
the first draft of `report_the_yard` labelled its column "preset" and would
have published a table saying the auto-builder clears the yard in five
seconds. `baseline.rs` builds the preset with `apply_preset` and keeps no
share code for it. Caught before the numbers went into this file; the column
says "perfect" now.

---

## After M11 - the merge review

Written after the mission was recorded, going back over it for what a merge
would carry that nobody asked for.

### The one criterion that was overclaimed

**46. Criterion 1's transcript did not exist, and it is an engine transcript
now.** M10's table marked it met on the strength of
`cli::a_scripted_run_replays_identically` and `acceptance::e6_1`. Neither is
the artifact the criterion names: `analysis/replays/switchyard-full.txt`, a
transcript of one specific walk - buy the sheet, ask for the points, Down line
and the coal road, feed the Shunter's, roundhouse road, feed the Signalman's,
be walked through to the water road, tell Ambrose both.

It exists now, 109 lines, generated twice inside one test and compared before
being compared against the committed file. The walk it records is the walk the
criterion names: **8 floors cleared `[0, 1, 2, 3, 5, 6, 8, 4]`, three levers
thrown, `sidings-cleared` 3, and three of the four pieces of ground** - the
Booking Hall stays at the goods shed, which is the ninth room.

**It is not a CLI transcript, and the reason is the driver.** The chain's first
door stands at rung 21 and no board the CLI can build from its own verbs clears
twenty rungs - `preset` wins nine of fifty, there is no `skip`, and there is no
way to read a share code in. That is a limitation the driver has had since
long before this mission (M1 finding 7, M3 finding 13) and closing it means
giving the CLI an import verb, which is a change to the driver rather than to
the yard.

### Second-order effects a merge carries

**47. The shop change is the most player-visible thing on this branch, and it
is not content.** M5 fixed the shelf tilt to count the pool rather than the
catalogue. Zero tests moved - but no test pins a specific shelf for a specific
seed, so "zero tests moved" means "nothing was watching", not "nothing
changed". **Every seed now deals different shelves from every seed before it.**
A player mid-run on the deployed build, or anybody who remembers what a
particular seed stocks, sees a different shop.

It is a fix rather than a regression: a slot was dealt in proportion to how
much of it exists rather than how much is for sale, and it had been wrong since
the Unwinding appended thirty-one event-only rewards. But it is a live
behaviour change riding into `main` inside a content mission, and it is the one
thing on this branch worth reverting separately if the owner would rather ship
the yard alone.

**48. Every run now meets a new door at rung 21, whether or not it wants the
yard.** THE TIMETABLE is `Trigger::Rung` and unconditional - by design, so the
chain has an on-ramp - which means rungs 21-25 got busier for every run in the
game. Two of `two_runs`'s walks had to be re-pointed because a whispered door
now goes first on rung 21. Expected, and worth knowing before somebody asks why
the mid-road feels fuller.

**49. Four public signatures changed**, none with a caller outside the
workspace: `Dungeon.floors` is `&[Floor]`, `Dungeon.landings` is gone,
`Theme::landings` is `Theme::landing(id, floor, canonical)`, and
`Interrupt::Dungeon` is a struct variant. Anything outside this repository that
read them breaks; nothing does.

**50. `make pack` still works and was not used.** Its save path writes `gear:`
and `items:`, both unchanged, and `gui::pack`'s own tests are green. The nine
boards went in through a targeted splice instead (M9).

### What a merge still needs

1. **`docs/` is stale** - built at `edcd9fc`, before any of this. `make
   publish` rebuilds the wasm and pushes it; the wasm target is installed.
2. **The GUI's points screen has never been rendered.** It compiles, its
   layout is unit-tested without a font context, and the main loop draws it
   after the landing scene and the receipt and before any road event - which
   is the right order and was checked by reading. Nobody has looked at it.
3. **Nobody has played any of it**, which is what `post-unwinding.md` §4 says
   of the Unwinding too.

---

## The shop, after the merge review (2026-08-27)

Two changes asked for on top of the mission, and one measurement that came out
better than expected.

### The shop was already the seed's, and is now pinned as such

`avail::two_seeds_are_two_shops`: sixteen seeds produce **sixteen distinct
opening shops**, no two of them share every shelf, and one seed twice is the
same shop. It was always true - the shop's rolls come off the run's own
xorshift and `Run::seeded` is the only way in - and nothing in the suite would
have noticed a refactor that started every shop from a constant. Now something
would.

### A seventh shelf, and what it did to the mix

`SHOP_SIZE` 6 -> **7**. It fits the strip the interface already draws: a card
is 126 wide with a 10 gap and the band is 1,186, so seven need 1,088 and eight
would not fit.

The interesting part is not the extra card. **The weapon's share of every
shelf fell from 53.2% to 48.7%**, measured over 400 opening shops at each
size:

| slot | at six | at seven |
|---|---:|---:|
| Weapon | **53.2%** | **48.7%** |
| Gloves | 13.3% | 14.6% |
| Helmet | 12.0% | 13.4% |
| Greaves | 11.2% | 12.1% |
| Chest | 10.2% | 11.2% |

The shelf is dealt round-robin over slot tickets, so a seventh card is one more
pass, and the pass after the weapon's tickets are spent is an armour slot's.
Every one of the four gains about a point. `avail::report_shelf_mix` is the
measurement and it says both numbers.

That is the second thing this branch does to the shop, and it pulls the same
way as the first: M5 stopped the tilt counting pieces no shelf can reach, and
this gives the round-robin one more turn. The weapon is still half the shelf,
which is the thing to look at next.

`stock_exactly` ignores `SHOP_SIZE`, so the pub's six shelves and a town's
curated five are unmoved. The bar is full because `SHELVES` is six names, and
three comments that said "because `SHOP_SIZE` is six" have been corrected -
that was true and coincidental, and it stopped being either.

**The four-board table is still byte-identical to M0.** 868 engine, 65 GUI, 5
CLI. No warnings.

---

## The validity solver (2026-08-27)

Asked for before the merge: spell out every event's access conditions, check
they are actually satisfiable, and prove it by running a strong build through
the game rather than by asserting the tables.

`crates/engine/tests/validity.rs`. **Nothing in it calls `force_win` or
`skip_to`** - every rung it passes is a fight the oracle simulated against the
creature actually standing there. That is the gap `post-unwinding.md` §10.6
names: "'completable' is proved by `force_win`", and this is the first thing in
the repository that proves any of it by fighting.

### The audit

`analysis/every-door.txt` is the table, generated by
`validity::report_every_door_and_what_it_wants`: all **37** doors with where
each one stands, what key it waits on, where that key comes from, and what
every choice asks for.

Four lints over it, all green:

| Lint | What it refuses |
|---|---|
| `every_door_that_waits_on_a_key_can_be_handed_one_in_time` | A whispered or flagged door whose key nothing hands over, or hands over after the window shuts |
| `nothing_is_shut_by_a_door_that_comes_after_it` | A `blocked_by` pointing at a door that cannot stand first |
| `every_gated_choice_wants_something_a_run_can_get` | A choice gated on a component that is not in `CATALOG`, a flag nothing sets, or a `Took` nobody offers |
| `every_scheduled_door_is_met_by_a_run_that_fights_past_it` | A `Trigger::Rung` door a run does not meet **while fighting past its rung** |

The key lint has to know **three** routes a flag can arrive by, and knowing
only one made it call the Unwinding's whole back half unreachable on its first
run: a dungeon's own `also` (THE THRESHOLD's `threshold-cleared`), a floor's
`also` (the yard's `switchyard-cleared`), and the dungeon being reached by a
**town door** rather than an event (`Action::CellarDoor`). `"never"` is the
sentinel for a door a pedestal pushes onto the stack, which stands on no rung
and is skipped.

### What a strong build meets by fighting

A greedy walk from rung 1: fight everything, answer whatever is standing by
the first open choice, drink at every fountain, walk past every town.

**Reached rung 46. 49 fights, 0 losses. Met 20 of the 37 doors.**

The seventeen it missed are all explained by what a greedy walk does not do,
and every one of them has a walk of its own or a reason:

| Missed | Why |
|---|---|
| the-crownwright, the-green-ledger, the-astronomer, the-locked-gate, the-glow-over-the-ridge, the-wizards-thirst, the-picket-line, the-exhibition | need a **word**, and the walk walked past every town |
| the-long-way | needs a win **over 15 s**; this board kills faster, and it is shut by the casino, which the walk took |
| the-second-shadow, the-passenger | need `threshold-cleared` - a dungeon behind a hidden town |
| the-sealed-bid, the-fork, the-foundry-remembers | need `slagworks-known` |
| the-thrumbus-race, mole-town | off-road; a pedestal pushes them |
| through-the-cracked-lens | rung 48, past where the walk stopped |

### The walks that fight for their door

| Test | What it proves |
|---|---|
| `a_strong_build_can_fight_its_way_down_the_road` | 45 rungs, 0 losses. The floor under everything else - a door "unreachable" by a walk that died at rung 12 is a statement about the walk |
| `every_scheduled_door_is_met_by_a_run_that_fights_past_it` | every `Trigger::Rung` door up to rung 46, met by fighting |
| `a_word_bought_at_the_bar_opens_the_door_it_is_about` | all **five** bar words: fight to Sump Bottom, buy the word, fight on, find the door standing |
| `the_switchyard_chain_is_walkable_by_a_build_that_fights_for_it` | the whole chain by fighting - timetable, box, turntable, four floors of the yard, the pedestal, four more floors, and Ambrose reading a counter only real clearings moved |
| `every_floor_of_the_yard_is_won_on_the_way_through` | both lines, in through the door and out the other side, with the board the run actually has when it arrives |

---

### Findings

**51. A finished board has an empty tray, and cannot buy a word.**
`A_WINNING_RUN` wears everything it owns, so `payment_for` finds nothing loose
and every bar trade was refused. A true fact about that board and the wrong one
to measure a door by - a player who wants a word keeps a spare mold. The solver
starts with one loose piece of each kind the bar prices anything in, and
nothing else.

**52. The bar prices one word in another.** A Word About the Green Ledger costs
A Word About the Crownwright, so a solver that buys one word at a time can
never buy the second. `Step::Barter` walks the price chain back to something
the tray can pay for and buys forwards.

**53. Nothing was found wrong with the road.** All 37 doors have a reachable
key, no `blocked_by` points forwards, no choice is gated on the unobtainable,
and every scheduled door is met by a build that fights past it. The Switchyard's
four are reachable by fighting, and so is every floor of its yard on both
lines.

---

## Is the new content on the map? (2026-08-27)

**Yes, all of it** - and asking found a piece of *old* content that was not.

### The yard, drawn

```
# 20 Bone Cantor [mini]
. -- THE TIMETABLE (event, between 20 and 21)
# 21 Ember Wisp
. -- AHEAD OF SCHEDULE (event, between 21 and 22)
. -- THE SIGNAL BOX (event, between 21 and 22)
...
# 25 Cog Priest
. -- THE TURNTABLE (event, between 25 and 26)
     \_ THE SWITCHYARD (4 fights, 3 points)
...
. 33 Iron Abbot
. -- THE LAST TRAIN (event, between 33 and 34)
```

All four doors, the dungeon hanging off the door that opens it, and the depth
label saying **4 fights, 3 points** - while THE CREVICE IN THE ROCK still reads
`(3 fights)` with no points clause, which is what A1.4 asked for.
`the_yards_content_is_on_the_map` pins all of it.

The two **sidings** are not drawn, and neither are the Unwinding's four
destinations: a pedestal's ticket is not a place on the road, and the yard it
returns you to is already a node. That is unchanged behaviour rather than a
gap.

### Finding 54: THE UNDER-MINE has never been on the map

`route.rs` scanned `c.outcome` for the outcome that opens a dungeon. THE
FOUNDRY's two choices open THE UNDER-MINE inside an `All` - the shelf then the
seam, or the seam then the shelf, and both buy you a shelf on the way past - so
the match fell through and **a whole dungeon the Unwinding shipped has never
been drawn**.

This is the Unwinding's own most expensive lesson arriving one mission late.
`HANDOFF.md` §4: *"Every lint over `EVENTS` stopped at the top of an outcome.
Half this mission's bargains are an `Outcome::All` ... `event::every_outcome`
unpacks `All` and everything asks through it now."* Everything except this.

Fixed, and the fix immediately drew it **twice**, because both of that door's
choices open it - so the scan keeps one node a dungeon per door. Both halves
are pinned by `every_dungeon_a_door_opens_is_on_the_map`, which walks every
door's outcomes through `every_outcome` and asserts each dungeon is drawn, once.

`analysis/every-door.txt` is regenerated.

**878 engine, 65 GUI, 5 CLI. No warnings.** The M0 ascii fixture still passes:
the lines that were on the map before are still on it, in order.

---

## The four caveats, closed (2026-08-27)

### 1. Classes are chips, and a stack is a number

The panel drew a list of names with a paragraph under each and a `+ N more`
holding the overflow - which answers "how many" and not "what". The band under
it is pinned, so a run wearing six titles saw two and a number.

They are chips now, laid out by `chip_rects`, the same shelf the glossary uses.
The whole set fits above the fold and the power description arrives on hover,
one at a time, which is what buys the room.

**A stacking class is one chip with a count.** `Run::classes` holds a stacking
class once per stack - Piety, Tired, Recycler and Unionized all stack - so the
list drew the same word three times and read as three classes. `class_stacks`
collapses them in earned order and `class_chip_label` puts `x3` on the chip.
The number is drawn only where there is one to draw, so the twenty-seven
classes that cannot stack say nothing.

Verified by rendering: nine class entries came out as six chips reading
`Piety x3 · Berserker · Chronomancer · Unionized x2 · Prospector · Geomancer`,
wrapped over three rows inside the panel.

### 2. The GUI was rendered, and it found two defects

`GEARMASTER_SKIP_INTRO=1 GEARMASTER_DUNGEON=the-switchyard:points
GEARMASTER_SHOT=<path>`. Two debug hooks were added to do it, in the style of
the four already there (`_TOOLS`, `_PRESET`, `_STUN`, `_PASTE`):
`GEARMASTER_DUNGEON=<id>[:points]` drops a run into a dungeon, and
`GEARMASTER_CLASSES=<names>` wears a set of titles. Both do nothing unless
asked.

The points screen renders correctly - title, the fork's two paragraphs, a card
a road, and the way out. Two things were wrong and are fixed:

- **The way out hung eight pixels below the panel.** `points_cells` put the
  roads at `r.h - 150`, copied from the event screen, which has no strip under
  its choices. The roads sit at `r.h - 186` and the panel is 36 taller.
- **The stack strip said THE SWITCHYARD twice.** `Interrupt::Points` took its
  name from the dungeon, and the points sit directly on top of the dungeon
  they are in, so the strip read as two buildings. It is `"THE POINTS"` now,
  which is the shape `Fountain` and `Brawl` already have: an interrupt that is
  a *moment* says what the moment is.

And one that was neither the yard's nor this mission's: **`WHAT THE WORDS
MEAN` and `TOOLS` have read as `WHAT THE WORDS MEANTOOLS`** for as long as
they have shared a row. The boxes never overlapped - the label did. `button`
draws centred and does not clip, so a label wider than its box spills out of
both ends. It is sized to the box now, which is also what a theme needs: a
themed word is free to be longer than the plain one.

### 3. The shop change ships

Both of them, and they pull the same way. M5 stopped the shelf tilt counting
pieces no shelf can reach; the seventh shelf gives the round-robin one more
turn. Together the weapon's share of every shelf falls from **54.8% before the
tilt existed, to 53.2%, to 48.7%**. Recorded in full under "The shop, after
the merge review", including that no test pins a seed's shelves - so the
change is real and was not being watched.

### 4. `docs/` rebuilt at publish

`make publish` rebuilds the wasm into `docs/` and pushes it. It was built at
`edcd9fc`, before any of this.

**878 engine, 68 GUI, 5 CLI - 951 in the workspace. No warnings.**

---

## The map's fraying edges (2026-08-27)

Reported after the deploy: three yellow diagonals on the road map, coming out
of certain bubbles and connecting to nothing.

**They were merge-ahead edges - a door that buys a rung off, drawn to the rung
it lands on - and they started in the wrong place.** The edge was drawn from
`(place(at).x, place(at).y - 26.0)`: the *spine* dot of the source rung, with a
constant guess at how far above it the bubble sits. An off-spine node does not
sit on the spine at all. It hangs **half a step to the left** and stacks
**upward by however many others share its rung**, so `-26` was right for
nothing and the line began in mid-air beside the bubble it was supposed to
leave.

Three doors buy a rung off, so three lines frayed.

The fix is not a nudge. `MapGrid` now works out where every node lands
*before* anything is drawn, and the renderer and the edges read the same list -
which is the only way an edge can know where its own node ended up. Pulled out
of the renderer for the reason `chip_rects` is: geometry computed inline is
geometry nobody can check.

Two lints, neither of which needs a window:

- `every_map_edge_touches_both_of_its_nodes` - a merge-ahead leaves a node that
  is genuinely off-spine, at a height above the spine, and lands exactly on the
  rung it names. A spine edge runs dot to dot.
- `nothing_on_the_map_is_drawn_over_something_else` - two things hanging off one
  rung stack rather than overlap.

Merge-ahead edges are also skipped across a row break now, the way spine edges
already were: fifty rungs wrap onto two rows, and a line from the right-hand
end of one to the left-hand end of the next crosses the whole map and means
nothing.

`GEARMASTER_MAP=1` opens the road on the first frame, which is how this was
looked at. A third debug hook beside `_DUNGEON` and `_CLASSES`.

**878 engine, 70 GUI, 5 CLI. No warnings.**
