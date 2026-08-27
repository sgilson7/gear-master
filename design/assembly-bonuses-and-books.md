# Assembly bonuses, and the book

Written against commit `e38d968` (2026-08-27). Every figure below was read off
that tip. Code follows this document; where it records what the code *does
today*, it was read and cited, and there the code is the news.

Two pieces of work that share a milestone chain because both move balance and
neither should move it twice.

---

## 1. What is wrong

### 1.1 "Adjacency" is not adjacency

`piece.rs:303` declares `Adjacency { label, stats }` and `loadout.rs:534`
applies it:

```rust
if assembled[gi] {
    for &p in group {
        if let Some(adj) = reg.def(p).adjacency {
            item_stats += adj.stats.scaled(100 + self.adjacency_pct);
```

The only condition is **did this group satisfy a recipe**. There is no
neighbour test anywhere on that path. The struct's own doc says where the name
came from - *"Gear Master's version of a Backpack Battles adjacency bonus"* -
and in that game the bonus really is adjacency-based. Here the trigger was
changed to assembly and the name was not.

It collides with five places that mean *touching*: `Trigger::OnAdjacentActivate`,
`RunningItem::adjacent_items`, `aligned_items`, `diagonal_items`, and
`Slot::sets_touch_diagonally`. The interface mostly gets it right - the GUI
prints "when assembled:" and the CLI "on assembly:" - but the word escapes in
the one place least able to survive it, **Recycler**, whose power text
(`class.rs:562`, `:621`, and again hardcoded in `main.rs:4077`) says
*"+10% adjacency bonuses"*. A player reading that will go and try to make
their pieces touch.

### 1.2 The numbers are not on the screen

There are fifteen assembly bonuses and they were written by two different
hands:

| Slot | Label | What it actually is |
|---|---|---|
| Weapon | `Timeworn: +0.30x weapon power` | power 30 |
| Helmet | `Stonewall: +25% physical resistance` | physical_resist 25 |
| Chest | `Voidsilk: +20% magic resist` | magic_resist 20 |
| Chest | `Bulwark: +20% physical hardening` | physical_harden 20 |
| Chest | `Rimeguard: 20% magic hardening` | magic_harden 20 |
| Chest | `Heartwood: +4 regen a second` | regen 4 |
| Gloves | `Breaker: +6 strength` | strength 6 |
| Greaves | `the road knows you` | curse_resist 4, faith 1 |
| Greaves | `one stride ahead` | curse_resist 5 |
| Greaves | `planted` | curse_resist 10 |
| Greaves | `downhill all the way` | curse_resist 4, strength 2 |
| Greaves | `the cold gets into the works` | curse_resist 6 |
| Greaves | `sure-footed on ice` | curse_resist 8 |
| Greaves | `already moving` | strength 4 |
| Greaves | `set before they arrive` | armor 6 |

Every non-greaves label is a specification. Every greaves label is atmosphere
with no number in it. Both go through one renderer (`main.rs:4182`) which
prints `"when assembled: {label}"` verbatim - so a Deeprooted Sole card reads
**"when assembled: planted"**, which is the largest assembly bonus in the game
and says so nowhere. The figure is printed on no card, no tooltip and no CLI
line; the only trace is the keyword rail, which shows a curse-resist glyph and
no quantity.

**And six of the eight greaves bonuses are curse resist.** They are not merely
vague, they are interchangeable. That is the real fault: one slot's worth of
bonuses is one bonus with eight names.

### 1.3 The fusion pools are unreachable

`Resource` has eight members. Three are fusions - **DruidicMight**
(nature+rage), **Communion** (faith+nature), **Zealotry** (rage+faith) - and
`Combatant::held_bonus` (`combat.rs:3217`) pays each at **double both parents'
rates**, uncapped: a point of Zealotry is +2 physical damage *and* +4 to both
resistances.

`Action::Fuse` exists. `Resource::parents()` exists. **No entry in the
504-piece `CATALOG` uses either.** Every mention of a fusion in `piece.rs` is
in the enum and its impl blocks; there are none past `:1202`, where the
catalogue starts.

This is the same shape as `cursed_for_good` before the Unwinding found it, and
as the three counters `completable.rs` now records: machinery that works,
costs nothing to reach, and is reached by nothing.

### 1.4 The book is the worse ball

```
Book:  Book 1 + Ink 1 + Spell 1 + Accessory 0-1     (piece.rs:1044)
Orb:   Orb 1  + Spell 2-3 + Alignment 0-1
```

The book spends a whole mandatory piece - inks run 4 to 5 cells - on a flat
multiplier for **one** spell. The orb spends nothing on a core tax and gets
two or three payloads.

| Build | Cells | Casts |
|---|---|---|
| Grand Grimoire + Kingsblood Ink + Kingsbane | 6+5+4 = **15** | **1** |
| Worldeye Orb + Kingsbane + Cometfall | 6+4+4 = **14** | **2** |

Fewer cells, more payloads, and an alignment that colours all of them. There is
no board on which the book is the right answer.

**The machinery is already general.** `loadout.rs:819` filters
`PieceKind::Alignment` out of `item.pieces` and folds its stats and triggers
into **every** `Cast`; it never asks whether the core is an orb. `casts` is
built from every `Spell` in the item. `power_bonus` is summed over every piece,
so zero inks is no bonus and two is double. Cast cycling (`combat.rs:4509`) is
`cast_index % casts.len()` and is not orb-gated either.

**So the book is weak because of the recipe table and nothing else.**

---

## 2. What it becomes

### 2.1 The type

```rust
pub struct AssemblyBonus {
    pub label: &'static str,
    pub stats: Stats,
    /// Triggers that exist only while the item is assembled.
    pub triggers: &'static [Trigger],
}
```

One field added. It reuses the entire `Trigger`/`Action` vocabulary, so most of
§2.3 costs no new combat code at all.

### 2.2 The book

```
Book:  Book 1 + Spell 1-2 + Ink 0-2 + Alignment 0-1
Orb:   Orb 1  + Spell 2-3 + Alignment 0-1            (unchanged)
```

Every bound is **relaxed**, never tightened, which is why this cannot break an
existing board: anything that assembled before still assembles. Four creature
boards wear book cores - Chained Codex, Leaden Tome, Apprentice's Primer, Grand
Grimoire - and all four keep working.

The identities separate properly:

- **The book is the focused caster.** One or two spells, and up to two inks
  multiplying them. Ink stacking is the book's whole argument and it is what
  makes a single big spell worth building around.
- **The orb is the broad one.** Two or three spells, no ink, one alignment
  colouring all of them. Breadth, not depth.

They overlap at two spells and that is fine - at two spells the book is paying
cells for multipliers and the orb is paying them for a third payload.

### 2.3 The eight greaves bonuses

Greaves own **tempo and what can stop you**. Each design is built to its own
name; the cost column is honest about what exists.

| Label | Becomes | Cost |
|---|---|---|
| **the road knows you** | Start holding 6 faith; every 12 faith banked becomes 1 **Communion** | free - `OnBattleStart(Gain)` + `Consume{Faith, 12, Gain{Communion}}` |
| **planted** | It never moves and never fires: every 8 nature it holds becomes 1 **Communion** | free - `Consume{Nature, 8, ..}` |
| **the cold gets into the works** | Every second curse you land **derails** an enemy item 400 ms | free - `Watch{CurseApplied, 2, Derail, repeats}` |
| **set before they arrive** | At the bell, armour equal to 3x the empty cells touching it | small - the battle-start scan (`combat.rs:4016`) matches the top-level trigger, so it must unwrap `PerAdjacentEmpty` |
| **sure-footed on ice** | This item cannot misfire and cannot be stunned | small - `RunningItem::steady` exists; stun immunity is one new flag |
| **downhill all the way** | Starts at half cooldown and adds 200 ms every time it fires | **new** - persistent cadence drift |
| **already moving** | Every item on the board starts at 40% of its cooldown | **new** - board-wide priming. `ReduceCooldown` is deliberately clamped to `cooldown_ms - 1` so it *cannot* do this |
| **one stride ahead** | When an **enemy** item activates, push this one forward 150 ms | **new** - `Trigger::OnEnemyActivate`; every relational trigger today looks only at your own board |

The seven specification labels keep their bonuses and lose the `Name: number`
half of their text once the renderer prints the stat block, or the card says
the number twice. `Stonewall`, `Breaker`, `Timeworn`.

Two of them are the natural homes for the remaining fusions: **Breaker**
(gloves, strength) becomes **Zealotry**, and **Heartwood** (chest, regen)
becomes **DruidicMight**.

---

## 3. Milestones

The two inert milestones come first on purpose. Both halves of this document
move balance, and the house rule from `design/HANDOFF.md` §5 is **land
primitives inert, arm them separately** - it is why every Phase-1 milestone of
the Unwinding shipped with the ladder byte-identical.

### M1 - The name, and the numbers. No rules change. **▲**

- `Adjacency` -> `AssemblyBonus`, `adjacency_pct` -> `assembly_pct`. Internal
  type; no string key, no `CATALOG` index, no save format.
- Recycler stops saying "adjacency" in all three places, including the
  hardcoded copy in `main.rs:4077`.
- The card, the tooltip and the CLI print the bonus's **stat block** beside its
  label, from `Stats::summary`, the way every other stat block is printed.
- The seven specification labels lose their now-duplicated numbers.
- **Deliverable:** no assembly bonus can ship without its numbers on screen,
  because the numbers come from the stat block and not from whoever wrote the
  label.
- **Gate:** both suites green; the ladder byte-identical; a test that every
  bonus's rendered text contains its own figures.

### M2 - The bonus can act. Inert.

- `AssemblyBonus.triggers`, wired in `loadout.rs` so they are live only while
  assembled and dead the moment the item comes apart.
- Every existing bonus ships `triggers: &[]`.
- **Deliverable:** the field and its wiring, arming nothing.
- **Gate:** the ladder is **byte-identical** to M1's. If it is not, the wiring
  is wrong and nothing above this is meaningful.

### M3 - The book, rebuilt. **▲**

- The recipe of §2.2. Nothing else.
- Re-pin the five `assembly.rs` tests that encode the old rule - including
  `a_book_will_not_take_a_second_spell_but_an_orb_wants_one`, whose *name* is
  the old rule, and the recipe listing at `:1023` that reads
  `vec!["1 book", "1 ink", "1 spell"]`. Re-pinned with the reason in the
  assertion.
- **The risk to watch:** relaxing a recipe cannot stop a board assembling, but
  it can make a loose pile *start* assembling - which changes fights, and moves
  `progression` pins. Every moved pin is inspected rather than re-blessed.
- **Deliverable:** a book build that is worth packing, and a measured statement
  of what it is worth.
- **Gate:** both suites green; `baseline` and `catalog_shape` printers re-run
  and their diffs read; a book board fought against the ladder and its rungs
  cleared written into `analysis/`.

### M4 - The four free bonuses, and two fusions. **▲**

- `the road knows you`, `planted`, `the cold gets into the works`,
  `set before they arrive`.
- **Communion becomes reachable** - the first fusion pool any board can make.
- `Watched::CurseApplied` gets something to pay for.
- **Gate:** a test that fights a board holding each new bonus and asserts the
  pool actually arrives, because a bonus that grants a fusion nothing reads is
  the fault this whole document is about.

### M5 - Three new primitives. **▲**

- `sure-footed on ice` - one flag on `RunningItem`.
- `downhill all the way` - persistent cadence drift. Nothing in the game
  changes an item's cadence permanently; this is the dial.
- `already moving` - board-wide priming at the bell.
- `one stride ahead` - `Trigger::OnEnemyActivate`, the reactive relation the
  trigger list has never had.
- **Re-gearing:** these change what a greaves item is worth, so
  `stepped_component` re-gears every creature on Easy, Hard and Insane. Weights
  settle **before** anything is measured against them.
- **Gate:** each primitive has a test that fails without it; the ladder's
  movement is measured and written down rather than discovered later.

### M6 - Zealotry, DruidicMight, and the record. **▲**

- Breaker -> Zealotry, Heartwood -> DruidicMight. All three fusions reachable.
- The printers, `analysis/`, the ledger, and `CLAUDE.md`'s counts and traps.
- **Gate:** a test asserting every `Resource` variant can be created by some
  board - the lint that would have caught this in the first place.

---

## 4. What could go wrong

1. **M2's ladder is not byte-identical.** Then the trigger wiring is firing
   when it should not, and every measurement above it is against the wrong
   baseline. Stop and fix before M3.
2. **M3 makes piles assemble that were never meant to.** Relaxing bounds is
   backward-compatible for assembly but not for *intent*: a player's loose
   spells may now bind into a weak item they did not ask for. Watch the preset
   and the three share codes specifically.
3. **The book overshoots.** Two inks on one spell is a large multiplier and the
   dial is the ink bound, not the spell count.
4. **M5's primitives are stronger than the ratings know.** `rating.rs` prices
   stats, and none of cadence drift, priming or enemy-reaction is a stat. If
   the ratings cannot see them, the shop misprices them and
   `stepped_component` puts them on creatures at the wrong rung. This is the
   milestone most likely to need a `rating.rs` weight, which is the one thing
   that must be settled before anything is authored against it.
