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
