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
exposed it: `avail::the_shelves_are_not_the_same_six_things_every_time` went
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
