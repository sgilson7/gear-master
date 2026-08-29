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

---

# Q1 — The representation

Read off **`194b8ce`** plus this milestone. The console learned to say what a
piece does with the eight pools; the probe says a learner could notice.

## Q1.1 The census

Every piece, through the console's own reader:

| pool | pieces producing | pieces consuming |
|---|---:|---:|
| mana | 132 | 31 |
| rage | 31 | 9 |
| faith | 36 | 12 |
| nature | 33 | 7 |
| **insight** | 11 | **0** |

**51 of 523 pieces both make and spend something**, and **190 carry at least
one conditional.**

Insight has eleven producers and no consumers, and that is not a fault: it is
fuel for Dread, which multiplies mind damage rather than spending a pool. It
is the one pool whose worth is a *rate on something else*, and a packer that
treats it like the other seven will bank a number.

`view::Pools` carries produce and consume per piece, and `view::BoardPools`
carries the board's economy: produced, consumed, **matched** - `min` of the
two, which is what actually flows - and what is stranded or starved. None of
it is privileged; the card prints *"on activation, spend 8 nature: if it
works, apply curse of searing"* in as many words, and this is the same
sentence as numbers.

## Q1.2 The probe, and the bug it caught

The gate: can a linear model over the board features predict whether a board
will **successfully spend a pool** in a fight? The label comes from the log -
a `ResourceCheck { paid: true }` - and never from the features, so the probe
cannot cheat.

Six hundred random boards, 314 of which assembled anything:

| | first attempt | after the fix |
|---|---:|---:|
| accuracy | 87.9% | **91.4%** |
| majority baseline | 85.7% | 85.7% |
| lift | +2.2 | **+5.7** |
| on boards that spent | - | 77.8% |
| on boards that did not | - | 93.7% |
| **balanced** | - | **85.7% against 50%** |

**The first attempt failed the gate, and the reason was a wrong
representation rather than a weak model.** `BoardPools` summed every *seated*
piece - and a loose piece pays its passive stats and never acts, so a nature
producer and a nature spender lying loose on one grid are two pieces that will
never meet. The board read as matched and the fight had no match in it.
Counting only the pieces inside **assembled items** moved the lift from +2.2
to +5.7 and made `any-match` the feature the probe leans on hardest.

That is `CLAUDE.md` §6 trap 36 in its general form - *only assembled items
act* - and it was caught by a gate rather than by a training run that quietly
learned nothing. **This is what Q1 is for**, and it justified itself on its
first execution.

## Q1.3 What Q1 hands Q2

The features the probe leaned on, in order: `any-match`, `matched-total`,
`prod-mana`, `pools-flowing`, `stranded-total`. Four of the five are about the
*relationship* between pieces rather than about any piece, which is the whole
claim - a build is a property of a board and not of a bag.

**Gate met**, and with the honest metric rather than the flattering one: 14%
of boards spend a pool, so plain accuracy rewards a model that always says no.
Balanced accuracy is 85.7% against a 50% floor.

---

# Q2 — Two environments

Read off **`3d761be`** plus this milestone. `crates/trades/src/env.rs`, **12
tests** in the crate, no engine change.

## Q2.1 What an episode is

**The quartermaster's** is `Packing`: the sixteen verbs it owns, masked to the
menu, plus one thing that is not a verb - **`Done`**. That action is what makes
this an episode rather than a loop, and it is not a nicety: a step cost alone
does not teach a packer to stop, it teaches it to press the cheapest key.

**The pathfinder's** is `Walking`: its own sixteen, plus **`Pack`**, which
hands the board to the quartermaster. And a **`Goal`** - a door, a dungeon, a
town, a rung or a county tile - which `met()` answers **off the screen**, so a
goal the pathfinder cannot recognise is a goal it cannot aim at.

## Q2.2 The numbers

One performance core, release, random policies:

| | |
|---|---|
| quartermaster, an episode | **1.40 ms** |
| its steps | 32 median, 60 at the cap |
| **10⁶ quartermaster steps** | **43 s of environment on one core** |
| pathfinder, a run with `pack` stubbed | 61 ms |

Forty-three seconds a million steps is far better than the spec's estimate and
it changes what is affordable: a Q3 training run is minutes of environment
rather than hours, and the cost will be the network rather than the game.

**One figure is not a measurement and says so.** The pathfinder's 600 steps is
the *budget*, not the horizon: a random policy never fights, so it never
progresses and never terminates - it reached rung 1. Q0's 204, measured off
the control, is the real number. A random walk is a throughput test and not a
horizon test, and reporting it as one would be the mistake this mission keeps
catching.

Six of three hundred random packing episodes won the fight they were packed
for. That is the floor Q3 has to climb from.

## Q2.3 The sampler

`situation(seed, rung)` stands a run at a rung with a purse and a shop.
It uses **`skip_to`, which is privileged and training-only**: the pathfinder
cannot reach it because it is not a verb, and the quartermaster never learns
how it got there. That is the curriculum, and without it a packer trained only
on rung one has never seen a board worth packing.

**Gate met:** both episodes run and end, every move offered is accepted, a seed
replays, and the packing horizon is inside sixty.

---

# Q3 — The quartermaster learns, and does not learn enough

Read off **`7d8bfe2`** plus this milestone. **The gate is not met**, and this
block is the record of it rather than a note that it will be met later.

## Q3.1 What was built

DQN over `(board, move)` pairs, because the menu is 17 to 545 verbs and changes
shape every step, so a head with a neuron per action does not apply. Replay
buffer of 80,000, a target network on a 200-episode clock, ε from 1.0 to 0.05,
Huber loss, and a curriculum over rungs that widens as it trains.

The reward is **one fight**, computed by the trainer and never seen by the
agent - the asymmetric actor-critic the crate graph enforces. A win is worth
more the faster it is won; a loss is worth more the closer it came, which is
the gradient A6 found missing.

Shaping is potential-based on **item count and nothing about pools**,
deliberately: `F = γΦ(s') − Φ(s)` leaves the optimal policy alone, and if the
shaping told the agent that matched pools were good, Q4 could not claim it
discovered them.

## Q3.2 Three failures, each with a number

**It collapsed onto `Rotate`.** The first trained quartermaster pressed rotate
**400 times out of 420** and assembled nothing. Rotate is free - it changes no
item count, so the shaping neither pays nor charges it - and there is one per
tray piece, so a nearly-flat Q picks one by chance, the state barely moves, and
the same action wins again. A step cost was in the spec and had not been
implemented.

**The discount did not reach the reward.** γ = 0.97 over a 120-step budget
discounts the fight to **2.6%**. The fight is the only real reward there is, so
at that rate the agent was optimising the step cost. γ = 0.995 and a 40-step
budget put it back: 0.995⁴⁰ is 0.82.

**One gradient step an episode is 2,500 updates.** The Q spread - how far apart
the best and worst move look - sat at **0.09 from the first evaluation to the
last**. The network had nothing to say and never got the chance to. Twelve
updates an episode moved it to 1.90 and the eval boards went from 0.0 items to
0.9.

## Q3.3 The gate, missed

| tray | pieces | items | cleared | the control (A3) |
|---|---:|---:|---:|---:|
| owner | 75 | **2** | **8/50** | 17 items, **48/50** |
| friend | 76 | 0 | 1/50 | 18 items, **49/50** |
| preset | 24 | 0 | 2/50 | 7 items, 12/50 |
| perfect | 62 | 0 | 1/50 | 15 items, 46/50 |

And the learning curve is real, which is why this is a miss rather than a
dead end:

    episode     0   items 0.0   spread 0.15   won 0/20
    episode   800   items 0.3   spread 1.40   won 1/20
    episode  1600   items 0.9   spread 1.90   won 2/20
    episode  1999   items 0.6   spread 1.90   won 1/20

It is learning. It is nowhere near a greedy packer that tries every seat.

## Q3.4 What is actually wrong, and it is not the amount of training

The spec's stop condition 2 says a learned packer worse than a greedy one over
a fixed tray is a learner that has not learned, and that **the fault is the
reward or the action factoring**. It is the factoring.

`Rotate` is still a third to a half of what it presses. The control does not
*decide* to rotate - it rotates to look, and the looking is free because it
undoes it. A learner has no undo: every rotation is a real step against a real
budget, and it has to discover that rotate-then-place is a two-step composite
whose value is entirely in the second step. That is a hard credit assignment
problem invented by insisting the action space be exactly the player's.

Three things would move it, in order:

1. **A rotate-and-place composite**, so a placement carries its rotation. It
   is a departure from strict action-fidelity - a person presses twice - but
   the *board* it produces is identical, and a proof written from it still
   replays. This is the one worth doing first.
2. **A learned per-step value** rather than an item count, so the shaping
   carries more than "you finished something".
3. **More training.** 2,000 episodes is 24,000 updates; DQN normally wants
   10⁵ to 10⁶. This is real but it is third, because the first two change what
   is being learned rather than how long it takes.

## Q3.5 What this milestone hands the rest

**The hand-written packer stays the default**, which the spec says it should
until it is beaten. Q5 and Q6 freeze *it* as the `pack` skill rather than the
learned one, and that is not a workaround: the plan's own design freezes a
packer while the pathfinder trains, and which packer is frozen is a parameter.

The goal-conditioned pathfinder - **the validity solver, which is the
product** - does not depend on the quartermaster being learned. It depends on
`pack` being *good*, and the good one exists.
