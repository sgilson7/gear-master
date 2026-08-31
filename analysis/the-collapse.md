# The Collapse — measurements

One block a milestone, each headed by the commit it was read off. The plan is
`design/the-collapse.md` and the brief it came from is
`design/HANDOFF-the-collapse.md`.

---

# M0 — The instruments, and what they said the moment they worked

Read off `c2241cd` plus the working tree of this milestone.

## M0.1 Every net in the repo had stopped loading, silently

The brief's §4 says to run `--bin qmind` first. At `c2241cd` it printed this,
and had been printing it for two commits:

```
================ rogue quartermaster    … did not load
================ grinder quartermaster  … did not load
================ rogue pathfinder       … did not load
================ grinder pathfinder     … did not load
```

`QNet::parse` gated on `w1.len() == feature::PAIR * hidden`. `cf91f65` took
`BOARD` from 30 to 270, so a packing pair is **315**; `f429dae` gave the road's
move description six more numbers, so a road pair is **64**. Every file in
`analysis/nets/` is **70** wide and matches neither.

`load` hands back an `Option` and every caller reads `None` as "no weights", so:

| what was reading nothing | what it looked like |
|---|---|
| `qmind` | four nets reported on, none of them loaded |
| `Packer::Learned(None)` | the floor, wearing a trained net's name |
| `qcross`, `qproof`, `qwhy` | the written control against itself |

Nine measurements in `analysis/the-two-trades.md` R6 and C7 cannot be repeated
at this tip for that reason. They are listed in
`crates/trades/tests/checkpoints.rs::STALE`, which is a ledger that may only get
shorter.

**A width is not a version, and a road net's width is not even its own.**
`qroad` pads a road pair up to `feature::PAIR` before feeding it in, so a road
net is *stored* at the packing width and the file's shape says nothing about
which road columns it read. That is why the two road nets on the shelf are 70
wide against a road pair of 64, and why the trainers stamp `pair <n>` now.

## M0.2 The provenance of the two nets the triage is about

`qrow` writes `runs/quartermaster_row.txt` for the best block and
`runs/quartermaster_row_last.txt` for the end. Played through the row itself -
`--bin qhand`, six runs, Rogue, Medium, `qrow`'s own seeds:

```
  packed by                                  mean    best    runs
  control                                     6.0      13       6   [2, 3, 3, 13, 2, 13]
  runs/quartermaster_row.txt                  2.0       2       6   [2, 2, 2, 2, 2, 2]
  runs/quartermaster_row_last.txt             2.0       2       6   [2, 2, 2, 2, 2, 2]
```

The control's line is the one `qrow` prints before it takes a gradient - **mean
rung 6.0, best 13** - reproduced exactly, so the loop is unchanged and
everything below is about the nets.

Over a hundred runs apiece:

```
  control                                     5.3      47     100
  runs/quartermaster_row.txt                  2.0       3     100
  runs/quartermaster_row_last.txt             2.2      13     100
```

**Neither saved net plays past rung 3 in the mean.** Whatever "deepest rung 11"
was, it is not in either file.

## M0.3 There is no collapse

The brief's table is `qrow`'s own printout, and its `deepest in block` column is
a **maximum over a hundred episodes**. Depth in this game is heavy-tailed and
seed-dominated - the written control's hundred runs are mean 5.3 and max 47 -
so that column can wander a long way while the policy behind it does not move.

`--bin qhand` with `QHAND_BLOCKS` holds one net completely still, explores at
the same 5% floor `qrow` decays to, and prints the same column:

```
  runs/quartermaster_row_last.txt, Rogue, 8 blocks of 100 at eps 0.05

    block  deepest     mean   past 6
        0        7     2.27        1
      100        7     2.21        2
      200        3     2.07        0
      300        7     2.18        3
      400       11     2.22        1
      500        9     2.10        1
      600        3     2.07        0
      700       11     2.12        1
```

**Eleven, nine, seven, three, seven, eleven - from a network that is not
learning anything, because it is a file.** The mean underneath is flat at 2.07
to 2.27 the whole way.

The other saved net, whose greedy best over a hundred runs is **rung 3**, does
the same thing:

```
  runs/quartermaster_row.txt, Rogue, 8 blocks of 100 at eps 0.05

    block  deepest     mean   past 6
        0        6     2.07        0
      100        6     2.08        0
      200        3     1.99        0
      300        3     1.96        0
      400       13     2.19        1
      500        3     1.93        0
      600        5     2.03        0
      700        3     2.01        0
```

A net that cannot reach rung 4 on its own prints **13**, once, because one seed
in eight hundred carried it there. That is the column the brief's peak is drawn
from.

Set the two beside the eleven blocks the brief is written about:

```
  ep 2600   11        ep 3300    3
  ep 2800   10        ep 3400    3
  ep 2900    9        ep 3500    2
  ep 3000    7        ep 3700    3
  ep 3200    8        ep 3999    2
```

Same statistic, same range, same wander. The rise to rung 11 and the fall to 2
are the maximum of a hundred heavy-tailed draws, and the policy under them sat
at mean rung two throughout.

So the observation this mission was called for - *"it had learned something and
then lost it"* - does not survive its own instrument. Nothing was learned and
nothing was lost. The Q spread rising through the turn is a value function
drifting upward under a behaviour that never changed, which is unremarkable and
was the only part of the picture that did not fit overestimation anyway.

**What this costs.** M1 to M5 of `design/the-collapse.md` were a triage of a
peak-and-collapse, and there is no peak and no collapse to triage. The standing
question is the mission's older one, unchanged since Q3: **the quartermaster
does not learn to pack.** Rung 2.0 against the written control's 6.0.

**And what it is an instance of.** `CLAUDE.md` trap 51 - *a known failure shape
is a hypothesis, not a finding* - for the third time in this mission, and trap
54 exactly: `qroad` prints a per-block depth *because* a running maximum made a
block that collapsed read identically to one that did not. `qrow` was never
given the same column.

## M0.4 What was changed, and the two numbers that decided it

`qrow` prints a **mean** now, and picks its best weights on it.

The mean was necessary and it is free - `out.deepest` was already in hand, so it
is a running sum and a divide. What justifies it is the noise, over the same
eight blocks of a hundred with the policy held still:

| | how far it moves between blocks |
|---|---|
| the maximum, which is what was printed | 3 to 11, and 3 to 13 |
| the mean, which was not | 2.07 to 2.27, and 1.93 to 2.19 |

Ten rungs against two tenths of one, for a policy that is not changing. No
milestone in this mission has produced a ten-rung change, so the column that was
printed could not see any of them. The mean also separates packers where the
maximum does not: the written control is mean 5.3 against these nets' 2.0, and
max 47 against 13.

**And the maximum was choosing the weights**, which is the half that did damage:
the file r12 saved as "the best block, rung 11" plays at rung 2. A block is
judged on its mean now, and a block shorter than `BLOCK` is not eligible at all -
the first block is one episode, and one lucky run would otherwise pin the best
weights to the initialisation for the whole run.

**No greedy evaluation block**, and that was measured rather than assumed. On
identical seeds:

| net | greedy | at eps 0.05 |
|---|---|---|
| `quartermaster_row.txt` | 2.00 | 2.07 |
| `quartermaster_row_last.txt` | 2.20 | 2.27 |

The exploration floor is worth **+0.07 of a rung**, in the helpful direction and
smaller than the block mean's own noise. An evaluation block would cost runs per
checkpoint to correct something the measurement cannot see. Worth revisiting
only if a policy ever gets good enough that a random press in twenty can wreck a
board it built.

The change, in a 250-episode smoke run:

```
  episode     0   eps 1.00   mean rung  1.00   deepest   1 (ever   1)
  episode   100   eps 0.43   mean rung  1.98   deepest   7 (ever   7)
  episode   200   eps 0.05   mean rung  2.05   deepest   6 (ever   7)
  episode   249   eps 0.05   mean rung  2.06   deepest   5 (ever   7)
  wrote runs/quartermaster_row.txt (best block, mean rung 2.05)
```

The old column reads 1, 7, 6, 5 - a policy that peaked and declined. The new one
reads 1.00, 1.98, 2.05, 2.06 - a policy that improved slightly and flattened.
The second is what happened, and the whole of `design/HANDOFF-the-collapse.md`
is the first reading at four thousand episodes instead of two hundred and fifty.

## M0.5 The same run again, with the mean, and it settles it from the source

M0.3 argued the peak was an artefact by reconstructing the column out of a file.
This is the argument from the training run itself.

Nothing in the collapse work touched the RNG, the reward or the update loop -
only what is printed and which checkpoint is kept - so `qrow` at defaults
retraces r12 exactly. It does: every `deepest`, `buffer` and `spread` in r13
matches r12 line for line. It is the same run with one more column.

The region the brief was written about:

| episode | eps | deepest (what the brief read) | mean (what was happening) |
|---:|---:|---:|---:|
| 2000 | 0.29 | 9 | 2.30 |
| 2100 | 0.25 | 7 | 2.26 |
| 2200 | 0.21 | 6 | 2.12 |
| 2300 | 0.18 | 5 | 2.18 |
| 2400 | 0.14 | 6 | 2.01 |
| 2500 | 0.11 | 7 | 2.21 |
| **2600** | **0.07** | **11** | **2.07** |
| 2700 | 0.05 | 7 | 2.26 |
| 2800 | 0.05 | 10 | 2.16 |
| 2900 | 0.05 | 9 | 2.02 |
| 3000 | 0.05 | 7 | 1.84 |

`design/HANDOFF-the-collapse.md` reads episode 2,600 as the policy reaching
**rung 11 with epsilon already at 0.07 - past the written control's mean of
6.0**. Its mean is **2.07**, and the eleven blocks around it never leave 1.8 to
2.3.

There was no climb. The maximum found a seed; the policy stood still.

r13 was stopped at episode 3,400 once it had been confirmed identical to r12 on
every shared column across all thirty-five blocks - the same `deepest`, the same
`buffer`, the same `spread`. It had nothing left to say that r12 had not already
said, and the six hundred episodes remaining were half an hour of a machine to
re-derive a log that is already on disk. `analysis/nets/qrow-r13.log` is that
run, truncated, and it is the only file in the repo carrying this loop's mean.

The brief's other half - the fall to rung 2 over the last seven hundred
episodes - is the same statistic returning to where the seeds put it, and needs
no explanation beyond that. Neither half is a fact about learning, and the Q
spread rising through both is a value function drifting under a behaviour that
never changed.

# M1 — What it presses, and why it never locks

Read off `ba59f86`. `--bin qhand` with `QHAND_KEYS`, forty episodes of Rogue
under `runs/quartermaster_row.txt`.

The collapse brief asked for a key histogram and said the cheapest diagnostic in
two missions had been one. Here it is, and it answers a question that was not
the one being asked.

```
  over 40 episodes, 7968 presses, deepest rung 3
  key               times    share
  buy                 194     2.4%
  clear                13     0.2%
  done                  8     0.1%
  pin                1653    20.7%
  place              3182    39.9%
  reroll                1     0.0%
  sell                 37     0.5%
  unequip            2880    36.1%
```

**It never locks.** Not once in seven thousand nine hundred and sixty-eight
presses. And it is not a preference:

```
  decisions where locking was offered:  6198
  ...and scored *identically* to pin:   6198  (100%)
```

`feature::mv` sorts a move into one of eight one-hot shapes and `_ => 8` for
everything else, and **bucket 8 is `PlaceLocked`, `Lock`, `Undo`, `Grow` and
`Pin`**. None of the five is in the `piece` match either, so fields 9-21 are
zero for all of them. All five are the same thirty-two numbers.

So on six thousand one hundred and ninety-eight decisions the network scored
`Lock` and `Pin` exactly equal, and `Iterator::max_by` returns the **last**
maximum. `Lock` is pushed onto the menu inside the per-slot loop and `Pin` in
the shop section after it, so the last one is always `Pin`. The agent pressed
the bucket-8 vector 1,653 times and the console executed `Pin` every time -
**by menu order, not by policy**.

This is `CLAUDE.md` trap 50 inside the quartermaster's own features. Trap 50 was
written up as the *road* agent's fault, because `feature::mv` is the packer's
description and every road verb fell into its catch-all. Nobody looked at what
fell into the catch-all from the packing side.

## M1.1 What that costs, and what it does not

An unlocked item **negotiates with whatever it is touching**
(`loadout::lock_assembled_in`): pack two flush and the optional pieces drift to
whichever core is nearest, so seating the next piece can take apart the item
before it. That is the repo's own trap 4 in a different costume.

Measured over the same forty episodes:

```
  items the reward paid for:        54
  items still standing at the end:  48
  paid for and gone by the end:      6
  presses that took an item apart: 1618
```

Two readings, and only one of them holds up.

**The reward leak is real and it is small.** An item is paid for on the press
that finishes it and only on a new high for the run, so six items in forty
episodes were paid for and were gone by the end. The churn earns nothing,
because the high-water mark does not pay twice.

**The thrash is real and it is large.** 1,618 presses took an item apart, one
press in five, and the histogram underneath it is 3,182 placements against
2,880 unequips. The packer seats a piece and pulls it out again, and it cannot
fix what it builds because the verb for fixing it is indistinguishable from the
verb for pinning a shop shelf.

**And it is not the collapse.** There was no collapse (M0.3, M0.5): the mean
was flat at about 2.0 from episode 0 to episode 3,400. A mechanism that only
bites once a board is full enough for two items to touch cannot explain a curve
that never moved, and this policy's deepest rung over forty episodes is **3**.

What it is a candidate for is the *standing* question - Q3, Q7 and Q8's, that
the quartermaster does not learn to pack. A packer that cannot lock cannot hold
a multi-item board, and a board it cannot hold is a rung it cannot clear. That
is a claim about the ceiling rather than about a fall, and it is the first
named, measured candidate this mission has had for it.

## M1.2 The fix, and what it costs

`feature::mv` has to tell bucket 8 apart. `Lock` wants its own kind and
something about the item it would fix; `PlaceLocked` wants to be a *placement* -
it is one, and today it is described as nothing at all, which is dormant only
because nothing ever locks.

Widening `MOVE` changes `feature::PAIR`, which invalidates every saved net. That
is now a loud failure rather than a silent one (`QNet::load_at`, and the ledger
in `trades/tests/checkpoints.rs`), and every net on the shelf is already
unfeedable, so the cost is a retrain that was needed anyway.

# M2 — The knee was forty times too far out

Read off `30d6f27` plus this milestone. `CLAUDE.md` trap 53's own closing
instruction, which was written for the road and never carried out here: *any new
reward wants its target range printed once against the loss it is being fitted
with.*

`qrow` prints it every block now. Three hundred episodes:

```
  episode   299   eps 0.05   mean rung  2.07   spread  0.053
       targets -1.96..+3.04 mean +0.040   residual mean 0.049
       past the knee of 120: 0.0%   gradient 1/120 of a squared loss
```

**The targets occupy `[-1.96, +3.04]` and the knee is at 120.** Nothing reaches
it, in any block, ever.

`min(|d|,k)*|d|/k` is `d^2/k` below the knee, so the gradient is `2|d|/k` -
bounded by two *at* the knee, and proportional to **1/k** everywhere below it.
`qroad`'s comment says dividing by the knee "decouples where the loss is
proportional from how large the gradients are", and that is true at the top of
the range and false at the bottom, which is where `qrow` has always lived.

The knee was chosen to cover what a run *can* be worth - rung 47 squared over
twenty-five is 88 - rather than what a run has ever been worth. The agent has
never passed rung 11.

## M2.1 The same three hundred episodes, with the knee where the targets are

Same seed, same everything, one constant:

| | knee 120 | knee 3 |
|---|---|---|
| Q spread, episode 0 -> 299 | 0.051 -> **0.053** | 0.051 -> **0.118** |
| mean target | -0.024 -> +0.040 | -0.024 -> **+0.225** |
| residual mean | 0.026 -> 0.049 | 0.026 -> 0.070 |
| mean rung at 299 | 2.07 | 1.79 |

**The value function was not stuck, it was idling.** At 120 the spread did not
move in three hundred episodes - 0.051 to 0.053, which is the flat network this
mission has diagnosed four times and never once traced to the loss. At 3 it
more than doubled, and the mean target climbed toward the returns instead of
sitting on zero.

**And read that for exactly what it is.** A spread answers "has this learned
anything at all" and not "is it any good" - `analysis/the-two-trades.md` R6.4,
and the Grinder pathfinder walks a road at a spread of 0.008. Depth has not
moved: 1.79 against 2.07, inside the block noise of 0.2, over three hundred
episodes with epsilon at the floor for barely a hundred of them. **Nothing here
says the packer is fixed.** What it says is that until now the optimiser was not
in a position to fix it, and that every null result this mission has recorded
was measured through it.

## M2.2 What this does to the four faults before it

Every one of them was read off a network that was barely being fitted:

* Q3, Q7, Q8 - the quartermaster does not learn to pack. Measured at 1/120.
* R6.1 to R6.3 - the flat road network. `qroad` found its own version of this
  and fixed it by scaling the reward, then set its knee to 12; `qrow` inherited
  the shape and none of the reasoning.
* M1 - the packer never locks. That is a *feature* fault and stands on its own:
  no gradient size makes two identical vectors distinguishable.

The three suspects `HANDOFF-two-trades.md` names for the packer were each "one
constant or one sampler rather than a rewrite". This was a third constant nobody
had looked at, and it sat under all of them.

# M3 — Double-DQN was not it, and the reward was

Read off `a478d0f`. Two arms, same seed, same knee of 3, same schedule:
`analysis/nets/qrow-r16-single.log` and `qrow-r17-double.log`.

## M3.1 What the A/B says

| episode | single: mean / target | double: mean / target |
|---:|---|---|
| 500 | 1.61 / +1.080 | 1.68 / +0.957 |
| 1000 | 1.61 / +2.364 | 1.84 / +2.111 |
| 1400 | 1.51 / +3.025 | 1.72 / +2.832 |
| 1800 | 1.49 / +3.761 | 1.45 / +3.368 |

Mean rung from episode 1000 on: **single 1.55, double 1.64** - against a
block-to-block noise of 0.2 (M0.4), which is nothing.

**Double-DQN damps the target growth by about a tenth and does not arrest it.**
Both arms climb at the same slope with no plateau, and the mean rung declines in
both. It is kept because it is theoretically right and costs only CPU, and it is
written down here as *measured not to be the fix* so nobody later reads it as
one.

## M3.2 And the failure to move the curve is the finding

If the values were inflating through the bootstrap, decoupling the choice from
the valuation should bend that curve. It barely moved it. So the value growth is
**genuine**: the agent really is earning more, and the depth is falling while it
does.

Which the reward explains without any help from the optimiser:

* an assembled item pays `ASSEMBLED + quality`, **1 to 3**, on the press that
  finishes it;
* reaching rung 2 pays `4/25 = 0.16`;
* and `worth` subtracts `0.5` a life, so a Rogue run - which always ends with
  its four lives spent - carries a terminal term of about **-1.84** whatever it
  does. Depth moves that by 1.28 across four whole rungs.

The return is almost entirely assembly at the place this policy lives. Rising
value with falling depth is not a bug in the optimiser. It is an agent doing
exactly what it was paid to do, and `RUNG = 1/25` is the constant that pays it.

That is the third time in this mission that a known failure shape fitted the
evidence and was wrong (trap 51), and the second time in this document.

## M3.3 The column that would have said so

`qrow` prints **items paid and items held** an episode beside the mean rung now.
The third time a trainer here has needed a column to tell "learning nothing"
from "learning the wrong thing": trap 54 gave the road one, M2 gave the loss
one, and the objective had none. If items climb while the rung falls, the
optimiser is innocent and the lever is `RUNG`, `ASSEMBLED` or `QUALITY`.

# M4 — Everything worked, and the rung did not move

Read off `b5be2d7`. `qrow-r17-double.log`, 4,000 episodes, 8,441 s. The net is
`runs/quartermaster_row.txt` (best block, mean rung 2.36) and `--bin qhand`'s
`QHAND_KEYS` over forty episodes is the reading.

## M4.1 The two fixes did what they were supposed to

**M1, the features.** Locking was offered on 5,982 decisions and scored
identically to pinning on **0 of them**, against 6,198 of 6,198 before. And the
agent presses it: `lock` 32 times, against never.

**M2, the loss.** The value function learns now: spread **2.037** against 0.051,
mean target **+5.45** against +0.04. Before, four thousand episodes moved the
weights 0% from their initialisation.

## M4.2 And the policy is exactly where it started

| | mean rung, epsilon at the floor |
|---|---|
| r12/r13 - old features, old knee, single | ~2.07 |
| r17 - new features, knee 3, Double-DQN | **2.10** |
| the written control | 6.0 |

Three fixes, one of which took the network from learning nothing to learning a
great deal, and **the thing being learned is worth 0.03 of a rung**.

## M4.3 What it spends its presses on now

```
  place              4257    49.5%        buy                 282     3.3%
  undo               3911    45.5%        unequip              56     0.7%
  lock                 32     0.4%        pin                   0     0.0%
```

Against the same measurement before the feature change: place 39.9%, unequip
36.1%, pin 20.7%, lock 0.0%.

**`Undo` is 45.5% of every press, and place-then-undo is a no-op pair.** Four
thousand two hundred placements and three thousand nine hundred undos: the
packer seats a piece and takes the move back, over and over, and spends its
forty decisions a rung doing it.

That is `CLAUDE.md` trap 44 for the **third** time in this repo. It was `Rotate`
400 times out of 420; `Rotate` was removed and it was `Pin` 410 out of 420; the
features were fixed so that `Pin` and `Lock` stopped being one key, and it is
`Undo`. The trap's own words: *taking verbs away one at a time cannot work,
because there is always another cheapest key.*

`NOTHING` is **0.0**, and its comment says why: *"there is nothing to dither
into: the packing budget bounds each rung at forty presses."* There is. A
place-and-undo pair is two presses of the forty and leaves the board where it
was.

## M4.4 Which is the same finding as M3, from the other end

A wasted budget should be expensive - a worse board is a shallower run - and it
is not, because **depth is worth almost nothing at the margin**. Rung 2 pays
0.16; the four lives a Rogue run always spends cost 1.84; an assembled item pays
1 to 3. Nothing in that arithmetic makes forty good decisions worth more than
forty wasted ones.

So M3's reading and M4's converge on one constant. `RUNG = 1/25` is what pays
for depth, and it does not pay enough for the objective to be the objective.

## M4.5 What this run cannot say

The Double-DQN comparison is honest only to episode 1,800, where the control was
stopped. r17 fell to 1.25 by episode 2,000 and **recovered to 2.1-2.3 once
epsilon reached the floor**, so the "monotone decline" M3 reported was partly the
exploration schedule and not only the policy. Whether the single-net arm would
have recovered too is not known, and M3.1's table should be read as a comparison
of the exploring phase alone.

The knee is also starting to bite: `past the knee of 3: 0.9%` at the end, with
targets reaching +13.61. Still small, and the column is there to be watched.
