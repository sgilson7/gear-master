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
