# Handoff — the gear-slot rewrite

Written for an agent starting with no context. Read `CLAUDE.md` first, then this,
then `design/monster-themes.md`. The spec is `design/gear-slot-basis-rewrite.md`
and it now carries its own dated amendments inline — read them where they sit
rather than trusting the line above each one.

## 1. Where the code is

- **`main`** is published and live. `eb5cc5d Publish web build`. GitHub Pages
  serves `docs/` from `main`.
- **`phase-2`** is the working branch and is not merged. Everything below lands
  there; `main` is touched once, at the end.
- Suite: **537 tests, green, no warnings.** `cargo test -p gearmaster-engine`.

## 2. What the rewrite was for

Five gear slots, of which only the weapon had mechanical identity. The other
four were one stat-pile wearing four shapes (helmet and chest at 0.93 cosine
similarity). The rewrite gives each slot a basis vector — Weapon **Conversion**,
Gloves **Reaction**, Greaves **Tempo**, Chest **Reserve**, Helmet **Economy** —
and moves the weapon's side-monopolies out to the slots that should have owned
them.

## 3. What is done

The engine half. The catalogue half is the larger one and is not.

- **`tests/baseline.rs`** — the measurement harness. Damage attributed by slot by
  pairing `Event::Hit` to the preceding `Event::Activate`; no engine change
  needed. Reports are `#[ignore]`d printers:
  `cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1`
- **`tests/catalog_shape.rs`** — the rules, shipped as a **ratchet**: budgets are
  today's distance, `no_budget_is_slack` forbids leaving slack, and the
  `#[ignore]`d `the_catalog_keeps_every_rule` asserts the targets and is **red at
  69 rules unmet** (from 79). Lower a budget in the commit that earns it; never
  raise one.
- **Four primitives**: `Trigger::Watch` + `Watched`, the diagonal relation, fused
  pools (`Resource` grew to 7), and `PieceKind::Terrain` (underlay, a second
  layer in `Slot`). Plus **reflection** (`Stats::reflect`), which the spec never
  asked for and the chest now owns exclusively.
- **All five slots swept once.** The two monopolies are gone: gloves hold 47
  reaction triggers to the weapon's 2; greaves hold 26 curse applications to the
  weapon's 20. Inert pieces are down from 44% of the catalogue to 21%.
- **`design/monster-themes.md`** — six themes, clustered rungs, hybrid
  mini-bosses, density curve, difficulty curve. Implemented in
  `tests/pack_francis.rs`, which is monster-agnostic (`PACK_MONSTER`,
  `PACK_TROPHY`, `PACK_BAND`, `PACK_ITEMS`).
- **`tests/fixtures.rs`** — a manifest of the tests that name a piece as their
  example of a mechanic, so a sweep fails there rather than downstream.
- **Documents reconciled** and every harness figure retaken at Medium on boards
  that assemble correctly (`analysis/baseline.md`, last entry). Underlays are
  town stock now: ground is bought where somebody has a floor to sell.

## 4. What is left, in order

Nine milestones. Each ends green, with its numbers written into
`analysis/baseline.md`.

**M1 — One way to rebuild a board.** `Shared::loadout` locks each item as it
assembles; three test-side reconstructions still hand-roll `run.equip` in a loop
and reproduce the fault it fixed — `towns.rs:36` (17 tests), `francis.rs:22`,
`pack_francis.rs:575`. Route them all through one helper. Then pin item
*membership* by name rather than counts, and add `debt_is_a_debt` to
`fixtures.rs`. Until this lands, every figure taken through those three is taken
on a board nobody built.

**M2 — Clear the road for the repack.** Six tests refuse a themed board by
construction and the packer cannot address `ALTERNATES` at all. Decide each once,
here, rather than discovering them three boards into a batch: the five-slot
requirement in `progression::the_named_fights_pack_their_boards`, the overkill
accounting in `brawl::the_aim_moves_along_so_they_come_down_together`, the casino
corridor in `two_runs.rs`, the three named drainers in `drains.rs`, and the
hard-coded rungs in `effects.rs`, `class_reaches_combat.rs`, `progression.rs` and
`taller_boards.rs`. Re-derive the density curve first — see §5.

**M3 — The repack.** 49 ladder boards, cluster by cluster, ascending. Francis is
excluded by design. Nothing has been repacked yet: the 11 specs carrying `items:`
got them from `packing::author_the_named_fights`, and every spec from index 6 up
still spans all five slots, which a themed board never does. Commit every three
with the suite as the verdict.

**M4 — Chest gets its attack.** Reflection is Wall-theme-exclusive, so rungs 7–13
are the only rungs where a creature can wear it: this runs *inside* M3's Wall
cluster, not after it. `rating.rs` has no weight for `reflect` at all — add one,
and re-tune so `a_slots_ceiling_is_full_marks` holds.

**M5 — The rating re-audit.** Reaction triggers are priced on the carrier item's
cadence rather than the neighbour's; frost is knowingly under-priced and is
greaves' now; `Grow` was tuned when it was a weapon mechanic; `Fuse` is a flat
constant; terrain has no arm. Watch `stepped_component` — it re-gears every
monster on three of the four difficulty settings.

**M6 — The shape sweep, 69 → 0.** 15 exclusivity lines + 7 quota lines + 43
floating carriers + 4 dull treasures. Cheapest first; the two large ones are
`health above 15` → chest (30) and greaves' bleed axis (22).

**M7 — Criterion 1.** 86.0% against 66–76%. M3 to M6 each move it; if it is still
short after them, the levers are the interaction fabric reaching real boards and
per-slot cadence at `piece.rs:860`.

**M8 — Step 9.** Shop pool routing and milestone pricing (neither exists),
`apply_preset` (which is also a reference build), GUI tooltips per primitive, one
identity line per slot. Theme and naming are already complete and must stay so.

**M9 — Final verification, then one merge.** Re-run the four printers, write the
§7 verification, rewrite this file as a record, merge `phase-2` into `main`,
publish.

**And nobody has played this.** Every claim here comes from the test suite.

## 5. The things that will bite you

**A dense board does not reconstruct into the items its owner built** unless each
item is locked as it assembles. A finished board packs to ~97% of its cells, so
nearly everything touches everything; deriving items in one pass at the end
merges whole grids. The owner's 19 weapon pieces came back as **one** item; the
perfect run's 11 came back as **none**.

Fixed in `share.rs` and in `pack_francis.rs`. **Not** fixed in the three
reconstructions M1 names, which is why M1 is first. Consequence: every drift
figure in `analysis/baseline.md` before the correction entry **understates the
weapon**, and anything measured through `towns.rs`, `francis.rs` or
`pack_francis.rs`'s reference boards still does.

**The repack's own gate is calibrated on that fault.** `design/monster-themes.md`
§6 sets `target(rung) = 2.8s + 0.4s × rung`, ±30%, read off the owner's board at
Medium and justified by "its median across the 46 rungs it currently clears is
14.4s". Corrected, that median is **9.00s**. Packing 49 boards against the old
line would bake the fault into the whole ladder.

**`CATALOG` is index-keyed by `share.rs`.** Append-only for ever: nothing moves,
nothing is deleted, a sweep rewrites in place under the existing name.

**Any `rating.rs` weight change re-gears every monster** on Easy, Hard and
Insane, through `stepped_component`. Almost nothing pins non-Medium outcomes.

## 6. Numbers that matter

| | |
|---|---|
| Weapon damage share, owner at Medium | **86.0%** (target 66–76%) |
| Baseline it started at | 96.1% |
| Criterion 2 | passes for gloves, helmet, greaves; **chest 0% everywhere** |
| Criterion 3 | passes — owner takes 43/50 rungs with no weapon, on greaves |
| Criterion 4 (early game) | holds — rung-1 TTK unmoved across every sweep |
| `catalog_shape` target | **69 rules unmet** |
| Board cadence, owner | 6.79 activations/s, against the 2.0 `rating.rs` assumes |

## 7. Habits that paid off

- **Measure before designing.** Two whole damage channels were invisible and the
  target band was arithmetic on a baseline nobody had taken.
- **Land primitives inert, arm them separately.** Watch, fusion and underlay all
  shipped with the ladder byte-identical.
- **Sweep a slot and repack the creatures wearing it in the same change.** The
  greaves sweep failed until Francis was repacked with it. Everything that arms a
  slot arms the monsters first.
- **When a guard refuses your change, it is usually right.** It caught four
  regressions that would otherwise have shipped, including the best board in the
  project losing to Francis on Easy.
