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
- Suite: **538 tests, green, no warnings.** `cargo test -p gearmaster-engine`.

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
  that assemble correctly (`analysis/baseline.md`). Underlays are town stock now:
  ground is bought where somebody has a floor to sell.
- **M0 and M1 of §4 are done.** The spec lives in `design/` with ten dated
  amendments, `CLAUDE.md` matches the code, and there is one way to rebuild a
  shared board.

## 4. What is left, in order

Nine milestones. Each ends green, with its numbers written into
`analysis/baseline.md`.

**M1 — One way to rebuild a board. Done.** All three reconstructions go through
`common::board_from`, which is `Shared::loadout` and nothing else.
`decode_build::the_boards_come_back_holding_exactly_these_items` pins all
fifty-one items across the three shared boards by member name, and
`debt_is_a_debt_and_takes_real_time_to_pay_off` no longer depends on two runs
firing the same items.

**Open question it raised.** The friend's board beats Francis on **Hard** once it
assembles properly — in 17.1s, against the 9.5s the repack was written to fix.
`francis.rs` pinned that setting as a defeat, measured on a board holding twelve
items instead of seventeen, and is now pinned by the clock instead. Whether the
final boss should stop the best board in the project at Hard rather than at
Insane is a design decision. Settling it means repacking him against the
corrected curve, deliberately, and `design/monster-themes.md` §3 otherwise says
to leave him alone.

**M2 — Clear the road for the repack. Done.** Six tests refuse a themed board by
construction and the packer cannot address `ALTERNATES` at all. Decide each once,
here, rather than discovering them three boards into a batch: the five-slot
requirement in `progression::the_named_fights_pack_their_boards`, the overkill
accounting in `brawl::the_aim_moves_along_so_they_come_down_together`, the casino
corridor in `two_runs.rs`, the three named drainers in `drains.rs`, and the
hard-coded rungs in `effects.rs`, `class_reaches_combat.rs`, `progression.rs` and
`taller_boards.rs`. Re-derive the density curve first — see §5.

All of it landed. The curve keeps its line for a better reason: its band's top
edge at rung 50 is 29.1s, just inside the 30s where **sudden death** takes the
fight over, so any steeper and the packer would be authoring the top of the
ladder into a region it cannot measure. Its old justification was false — only
**13 of 37** gear-decided rungs are inside the band, because the ladder is a
scatter rather than a ramp, which is what the repack is for.

**M3 — The repack.** *In progress — 26 of 49.* Strikers (2-6), Walls (7-13),
Burners (14-20) and Slowers (21-28) are packed; Caster, Drainer and the unthemed
run-in remain. One skip so far: Rust Colossus, whose weakest buildable wall at
rung 12 still takes 4.5s against a 3.0s target.

The packer needed thirteen fixes to get through the first two clusters and none
at all for the two after, which is the shape to expect: the defects were
assumptions about what a creature *is*, and stating them correctly once holds
for every theme after. They are listed in the commits; the ones that will matter
again are that a theme's slot list is a permission and must not double as a
priority, that a cap on items is not a cap on pieces, and that a share of the
board must never be the reason a slot holds nothing.

49 ladder boards, cluster by cluster, ascending. Francis is
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

Fixed in `share.rs`, in `pack_francis.rs`, and — as of M1 — in the three
reconstructions that hand-rolled it. What survives: every drift figure in
`analysis/baseline.md` before the correction entry **understates the weapon**,
and any figure quoted from `towns.rs` or `francis.rs` before M1 was taken on a
board nobody built.

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
