# THE TWO TRADES — measurements

One block a milestone, headed by the commit it was read off. The spec is
`design/the-two-trades.md`; this file is what the machine said.

Same machine as THE APPRENTICE: **Apple M2 Max, 8 performance cores, 4
efficiency, 32 GB, rustc 1.95**, everything in `--release` on mains power, and
every throughput figure carries its thread count.

---

# Q0 — The ground, and the two menus

Read off **`523b4db`** plus this milestone. `crates/trades`, **6 tests**, no
engine change.

## Q0.1 The partition

Sixteen verbs apiece, exhaustive and disjoint.

| the quartermaster | the pathfinder |
|---|---|
| Place · PlaceLocked · Unequip · UnequipLocked | Answer · AnswerWith |
| Rotate · RotateLocked · Lock | Fight · FightParty |
| ClearSlot · ClearAll · Undo · Grow | Town · WalkOn |
| Buy · Sell · Barter · Reroll · Pin | ThrowPoints · Leave |
| | Walk · Out · Perambulate |
| | Drink · DrinkChoosing · Double |
| | Pedestal · Crush |

`tests/partition.rs` reads the variants **out of `verb.rs`'s source** rather
than out of a match, so adding a verb fails the lint until somebody decides
which trade owns it. That is the point: the decision is the deliverable.

Two calls worth arguing with, and both are in the module's own doc comment.
**The shop is the quartermaster's** - buying and placing are one decision, and
A6 measured it. **`Crush` and `Pedestal` are the pathfinder's** even though
they take a piece: crushing a relic buys a town door and feeding an orb sends
the run somewhere, so they are road decisions spelled with an item.

## Q0.2 The horizons, and the estimate that was wrong

Eight runs of the hand-written control, every press classified.

    presses, by trade
      pathfinder             839    0.1%
      quartermaster       941,965  99.9%
      of which trial pairs 761,840  80.8%

**Ninety-nine point nine percent of a run belongs to the quartermaster**, and
four fifths of that is place-then-undo. The pathfinder makes 839 presses
across eight runs.

The spec estimated 30-60 decisions a quartermaster episode. Measured:

| | min | median | 90th | max |
|---|---:|---:|---:|---:|
| the control's **presses** an episode | 7 | **492** | 798 | 40,133 |
| **decisions that stuck** | 2 | **13** | 26 | 47 |
| pathfinder **decisions a run** | 126 | **204** | - | 264 |

The estimate was outside on both counts and wrong in the useful direction. It
was measuring the control's **search**, not the decision. A packer that emits
placements rather than trying them makes **thirteen decisions** a typical
episode - shorter than the spec hoped for, and a horizon TD learning is
comfortable with.

The pathfinder's 204 is inside the estimate, and it is the number the whole
architecture rests on: 195,273 presses become **204 decisions plus 79 calls to
`pack`**.

## Q0.3 What Q0 hands Q1

- Both horizons are learnable. The macro-action buys what it was supposed to.
- **The quartermaster is the whole game by press count**, which is where the
  learning has to pay.
- `Rotate` is 171,364 presses across eight runs - the third commonest verb,
  and every one of them exploratory. A learned packer choosing a rotation
  rather than trying four is a large part of the saving on its own.

**Gate met:** the partition lint passes in three directions - disjoint,
exhaustive, and no list naming a verb that does not exist - and the horizons
are measured rather than assumed.
