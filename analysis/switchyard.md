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
