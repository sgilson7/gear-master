# The action gap — measurements

Read off `9fa6d28` plus the working tree of this triage. Two new instruments,
`--bin qcarry` and `--bin qcols`, and everything below is their output.

The question this answers is the owner's, in their words: *"you can make some
absurdly strong builds early that can carry you through 6-10 rungs at a time
with no changes, which makes all future random moves look good."*

It is right, and the shape of it is not quite what the sentence says. The
decisions are not worthless; their effect is **displaced**. Packing at a rung
changes that rung's outcome on 22% of visits and changes the run's final depth
by 1.8 rungs, and the gap between those two numbers is the whole finding.

---

# A — What a board does on its own

`--bin qcarry`, 40 Rogue runs on `qrow`'s own seeds, Medium, packed by the
written control. At every rung the run stands on, three clones of that exact
console: press nothing and fight; press the 40-decision budget out uniformly at
random and fight; pack with the written control and fight. Combat is a pure
function of two boards, so every cell is a fact and not a rate - the rate is
across the 350 rung visits.

```
  rung   visits |  cleared by: none  random  control |  carry from here
  -------------- |  ---------------------------------- |  mean   max
     1       54 |            0%     31%      74% |   0.8     3
     2      107 |            3%      2%      23% |   0.4     5
     3       44 |            7%      0%      20% |   1.0     6
     4       19 |           68%     37%      74% |   3.1     8
     5       14 |          100%     57%     100% |   4.1     8
     6       16 |           88%     31%      88% |   4.1     7
     7       20 |           40%     15%      60% |   2.5     6
     8       12 |          100%      8%     100% |   3.7     5
     9       14 |           57%      7%      71% |   2.8     4
    10       10 |          100%    100%     100% |   2.9     3
    11       10 |          100%     30%     100% |   1.9     2
    12       10 |           90%     40%     100% |   1.0     1
    13       20 |            0%      0%       0% |   0.0     0
```

**The rung came out the same packed and unpacked on 274 of 350 visits (78%).**
Packing changed it on 76, at rungs 1, 2, 3, 4, 7, 9 and 12 - and 68 of those 76
are at rungs 1 to 3.

So 78% of every episode's forty decisions are graded against a rung that was
already decided before the packing began. The carry column is the mechanism:
from rung five the standing board clears a mean of 4.1 further rungs untouched,
and the longest carry measured is **eight**.

## A.1 And random is not the same as nothing

The `random` column is the one that stops this being "moves do not matter". At
rungs 8 to 12 doing nothing clears the rung 90-100% of the time and pressing
forty random keys clears it 8-40%. A random packing takes the board apart:
`Unequip`, `ClearSlot` and `ClearAll` are on every menu.

So the advantage structure at a carried rung is one-sided. Every non-destructive
action is worth the same - the rung is won either way - and destructive ones are
worth less. **The uniquely safe action is the one that changes nothing**, and
`place` followed by `undo` is exactly that, at a cost of two presses out of
forty. `analysis/the-collapse.md` M4.3 measured the trained packer pressing
`undo` on 45.5% of its presses and read it as `CLAUDE.md` trap 44, a third free
verb. It is not a free verb. It is the best-estimated action in a state where
acting has mean advantage zero and negative variance.

## A.2 What the packing is actually worth

Four whole-run policies, the same 40 seeds:

```
  policy                                 mean    best vs control
  the written control, every rung        5.62      13    +0.00
  control to rung 3, then nothing        3.83       9    -1.80
  control to rung 1, then nothing        2.12       4    -3.50
  nothing at all, ever                   1.00       1    -4.62
  random presses, every rung             1.62       3    -4.00
```

Packing above rung three is worth **+1.8 rungs of final depth** and changes the
outcome of the rung it happens at on 8 of 87 visits. That is the displacement,
in two numbers: the signal is real at the scale of a run and nearly absent at
the scale of a decision.

## A.3 And the discount does not localise it

An episode is about 350 decisions. At `gamma = 0.999` the terminal reward -
which is `deepest^2`, and almost the whole of the return once a run is past rung
two - reaches the **first** decision at 0.70 of its face value and the last at
1.00. `gamma^k >= 0.9` for `k <= 105`, so the final hundred presses are credited
within a tenth of each other.

There is no temporal structure in the credit at all. Three hundred and fifty
state-action pairs, 78% of them causally inert at the step they were taken,
receive the same terminal number to within 30%.

---

# B — What the trained nets learned instead

`--bin qcols`. Four nets on the shelf load at this build's pair of 321:
`analysis/nets/qrow-r18-best.txt` (squared, the baseline of
`analysis/the-collapse.md` M5), `qrow-r23-best.txt` (cubed, M6), and
`runs/quartermaster_row{,_last}.txt` from the last run. The first and third are
byte-identical.

## B.1 Where the weight went

Row `i` of `w1` is every weight input `i` can reach. `init` draws uniform on
`+/- sqrt(2/321)`, so a band at 1.00x has the spread it was drawn with.

```
  r18                              cols        rms    vs init
  pools, the four totals              4    0.04567      1.00x
  pools, per resource                12    0.04533      0.99x
  what the board is                   8    0.04982      1.09x
  purse, tray, what is coming         4    0.10658      2.34x
  the rung, and the lives left        2    0.11726      2.57x
  the layout, 5 grids x 48 cells    240    0.04494      0.99x
  the brief                          13    0.04597      1.01x
  the move's kind, one-hot           13    0.04737      1.04x
  the piece it is about              13    0.04562      1.00x
  where it goes                      10    0.04675      1.03x
  what locking would fix              2    0.04742      1.04x
```

Five of 321 inputs grew: gold, tray fullness, what the coming creature brings,
the rung and the lives left. The single largest is **`f[29]`, lives left, at
4.05x**. The 240 layout cells - three quarters of the vector, and the whole
spatial description of the board - are at 0.99x. Every band of the move
description is between 1.00x and 1.04x.

**This is evidence and not proof**, because weights can move and keep their
spread. Differencing the two checkpoints of one run says what was written:

```
  band                             cols     mean |d|  as % of rms  sign kept
  purse, tray, what is coming         4     0.342198      454.67%        71%
  the rung, and the lives left        2     0.330770      425.68%        69%
  the layout, 5 grids x 48 cells    240     0.080258      208.16%        68%
  the brief                          13     0.000000        0.00%       100%
```

The layout **is** written to and does not accumulate. The brief reads exactly
0.00% and 100% sign-kept, which is the instrument checking itself: `qrow` feeds
`Brief::NONE`, thirteen zeros, so those columns provably cannot take a gradient
and the diagnostic agrees.

## B.2 What the answer depends on

Row norms say what an input can reach, not what the output uses. Zeroing one
band at a time over 224 real pairs from real runs, against an across-state Q
standard deviation of **0.720**:

```
  band zeroed                      cols    mean |dQ|   of that sd
  the rung, and the lives left        2       1.5119         210%
  the layout, 5 grids x 48 cells    240       0.8866         123%
  purse, tray, what is coming         4       0.5600          78%
  the move's kind, one-hot           13       0.2592          36%
  what the board is                   8       0.2091          29%
  where it goes                      10       0.1840          26%
  the piece it is about              13       0.0772          11%
  pools, per resource                12       0.0070           1%
  the brief                          13       0.0000           0%
```

**Two inputs move the answer more than the other 319 put together.** The whole
38-column move description - which is the only part of the vector that differs
between two candidate actions at one state - accounts for 0.52 against 3.18 for
the state side. Six to one.

Per column it is two hundred to one: 0.756 for each of rung and lives against
0.0037 for each layout cell.

## B.3 The action gap

The quantity the approximation-error argument is about is `Q(s,a*)` minus the
best of the rest, and the states have to be ones the ladder actually contains.
The trained packer dies at rung three, so the written control walks and the net
scores the menu it finds.

```
  r18                                                        within 1%
  rung    states     menu     best Q        gap   gap/best   of best
     1        18      137      5.654     0.0087      0.15%        23
     2        21      182      4.411     0.0325      0.74%        10
     3        12      105      4.920     0.0770      1.56%         3
     5         3      148      5.455     0.0719      1.32%         7
     8         3      161      7.494     0.0988      1.32%         5
    10         3      272      8.765     0.0177      0.20%        10
    13         8      246     11.563     0.0492      0.43%        10
```

The state value climbs monotonically with the rung - 5.65 to 11.56 - and the gap
does not. At rung one, **23 of 137 legal moves are within 1% of the best**; at
rung ten, 10 of 272. The greedy argmax is a near-tie broken by whichever
numerical residual happens to be largest, which is `CLAUDE.md` trap 50's fault
arriving by a different road: there it was two actions with identical features,
here it is twenty-three actions with indistinguishable values.

Wang et al.'s dueling paper quotes DDQN on Seaquest at an average action gap of
0.04 against an average state value of about 15 - **0.27%**. This packer is in
the same regime and often below it.

## B.4 Cubing made the ratio worse

`qrow-r23-best.txt`, `QROW_POW=3`, which `analysis/the-collapse.md` M6 measured
as +0.06 of a rung and called a wash. The gap table says what happened:

```
  r23                                                        within 1%
  rung    states     menu     best Q        gap   gap/best   of best
     1        18      137     19.634     0.0954      0.49%         3
     3        12      105     50.502     1.5907      3.15%         3
     6         4      205    203.320     5.0123      2.47%         1
     9         3      191    637.974     2.2268      0.35%        19
    12         3      242    990.597     0.7076      0.07%        24
    13         8      246   2008.507     2.5151      0.13%        33
```

The state value went up by a hundred and seventy fold at rung thirteen and the
gap went up by fifty. So the **relative** gap collapses with depth - 0.07% at
rung twelve, with 24 of 242 moves inside 1% of the best - and the deeper the run,
the less the policy can tell its options apart.

Steepening the depth reward scales `V(s)` and leaves `A(s,a)` behind. That is
the mechanism behind M6's wash, and it says the same about any further
steepening: `POW` is not the lever.

---

# C — What this rules in and out

Three milestones of `analysis/the-collapse.md` looked for the fault in the
optimiser - the bootstrap (M3, Double-DQN, measured not to be it), the loss knee
(M2, which was a real fault and fixed), the replay distribution (M5,
demonstrations, measured harmful). M4's summary is that everything worked and
the rung did not move.

The measurements above say why nothing in that list could have moved it. The
quantity a Q-learner needs is `A(s,a)`, and in this environment `A` is near zero
at 78% of the states the agent is graded on, is displaced by two to eight fights
from the decision that caused it, and is credited by a discount that spreads it
evenly over three hundred and fifty decisions. Every one of M2 to M6 improved
the fitting of `Q`, and `Q` here is very nearly `V`.

`CLAUDE.md` trap 52 says to print the values before diagnosing a policy. The
sequel is: **print the gap before diagnosing the values.** A network with a
healthy spread across states and a 0.2% gap across actions has learned exactly
what it was shown and has not learned a policy.

# D — What it does not say

* It does not say the task is unlearnable. Packing above rung three is worth 1.8
  rungs and that is a real signal at the scale of a run.
* It does not say the written control is good. It reaches mean 5.62 and the
  question of what a *strong* packer would reach is untouched.
* It does not measure any remedy. Nothing here has been run with a dueling head,
  an n-step return, an advantage-learning operator or a rung-local reward, and
  `CLAUDE.md` trap 51 is the standing reason not to write down what one would do
  before running it.
* The deep-rung rows are three to twenty visits. The rungs that carry the
  argument - 1 through 4, and the gap tables, which are per-state - are
  hundreds. The 100% cells at rungs 8, 10 and 11 are 12, 10 and 10 visits and
  should be read as "most" rather than "all".

---

# E — The first remedy, and it was refuted by its own output bias

Read off `b23cafe`. Two arms, 3,000 episodes each, same seed stream, same
reward as r18 (`QROW_CHURN=0`), run concurrently:
`analysis/nets/qrow-r24-control.log` and `qrow-r25-duel.log`.

The intervention is a **dueling head**, chosen because §B.3's measurement is
the measurement the dueling architecture was introduced for. The anchor is a
reference action rather than the mean over the menu, because the replay buffer
keeps the chosen pair and the *next* state's candidates and no menu at all:

```
  Q(s,a) = V(s) + A(s,a) - A(s, done)
```

## E.1 What the arms did

| | floor mean (ep 2100+, eps at 0.05) | range | items paid / held |
|---|---:|---|---|
| control, plain `Q` | **2.926** | 1.52-5.16 | 2.29 / 1.61 |
| dueling, reference anchor | **1.367** | 1.12-1.92 | 1.00 / 0.04 |

Thirty-seven blocks apiece and the ranges do not overlap. The control lands
where r18's floor did (3.02, and not identical because the value tower is drawn
from the same stream either way, so this pair of arms is not seed-identical to
r18 - only to each other). The dueling arm sits a third of a rung above
"nothing at all, ever", which §A.2 measures at 1.00, and it finishes a run
holding **0.04 items**.

## E.2 And the mechanism is arithmetic, not a hyperparameter

`--bin qmind`, the two nets, and the biases are the honest column because they
start at exactly zero:

```
  advantage tower          duel          control
  b1  sd                 0.0076           0.0508
  b2  sd                 0.0098           0.0400
  w3  against init          +11%            +117%
  b3                 0 (exact)  NO          0.6012
```

**`b3` is exactly zero after three thousand episodes**, and it could not have
been anything else. It is the advantage tower's output bias, the tower is
evaluated twice, and the two evaluations are subtracted:

```
  dQ/db3  =  dA(x)/db3 - dA(anchor)/db3  =  1 - 1  =  0
```

Identically, for ever. And the same cancellation starves the rest of that
tower to the degree that `x` resembles the anchor - they differ in 38 of 321
columns, and §B.2 already measured that band as the one the answer moves least
with. `b1` moved 0.15 as far as the control's and `w3` 0.09 as far.

With the advantage stream starved, everything else follows: `Q`'s across-state
standard deviation is **0.117** against the control's 2.089, the action gap is
0.00% to 1.32% with **13 to 19 keys inside 1% of the best** at rungs 7 to 13,
and at rungs 7 and 8 the gap is 0.0000 exactly. The split did form - mean `|V|`
0.416 against mean `|A|` 0.074, so `A` is 15% of the answer - and what it
formed was a value function with a decoration on it.

## E.3 What this refutes, and what it does not

It refutes **this form of the anchor** and nothing about dueling. The textbook
constraint subtracts the mean advantage over the menu, which does not cancel
because standard dueling reads the action as an *output* - one forward pass
gives every action's advantage from a different output unit. Here the action is
an **input**, so a differentiable menu mean is sixteen more towers a sample,
eighteen against three, which is about ten hours for three thousand episodes
against two. That is why the reference action was chosen and it is the thing
that has to be got right.

**The affordable correction is to detach the anchor.** `Q` is unchanged in
value, `Q(s, done)` is still exactly `V(s)`, and the advantage tower takes
`dA(x)/dtheta` rather than the difference of two of them - a semi-gradient,
which is what a TD target already is. One hundred episodes with
`QROW_DUEL_ANCHOR` at its default:

```
  b3   0.0193   yes
```

against exactly zero after three thousand. The gradient path is open, so the
remedy has not in fact been tested yet. `QROW_DUEL_ANCHOR=grad` is the arm
above, kept so the comparison can be repeated.

## E.4 The instrument that caught it

Nothing in the training curve says "your output bias cannot receive gradient".
The block line prints a mean rung and a Q spread, and both of them said what
they say for any bad policy. What named it was `qmind`'s bias column, which
exists because a weight that has moved is hard to see against its own spread
and **a bias starts at exactly zero** - so the one printed word that mattered
was `NO`.

That is the fourth time in this mission that the diagnostic which settled a
question was one that reports a *mechanism* rather than a behaviour, and the
third time the behaviour on its own supported a wrong reading: a policy at rung
1.37 with 0.04 items held reads exactly like a policy that has learned to do
nothing, which is `CLAUDE.md` trap 44's shape and was the first thing this
looked like.

---

# F — The corrected arm, and dueling does not transfer to a pair input

Read off `67cc9d7`. `analysis/nets/qrow-r26-duel-detached.log`, 3,000 episodes,
7,010 s, same seed stream and same control as E.

## F.1 The fix worked, mechanically and completely

`--bin qmind` on the advantage tower, the three arms side by side:

```
                     duel (grad)   duel (detach)   control
  b3                0 (exact) NO          0.8843    0.6012
  b1  sd                  0.0076          0.3924    0.0508
  w1  against init            +0%            +98%       +2%
  w3  against init           +11%           +425%     +117%
```

The output bias that could not receive gradient now receives it, and the tower
around it moved further than the control's did. Whatever else follows, E.2's
diagnosis was right and its cure does what it says.

## F.2 And the remedy is refuted on the metric it was chosen for

The whole point of a dueling head here was §B.3's gap. It got **narrower**:

```
  gap as a share of the state value, at the written control's states
  rung        1     2     3     4     5     6     7     8     9    10    11    12
  control  0.32  0.70  1.34  1.69  1.42  0.59  0.14  1.69  0.33  0.49  2.93  0.46
  duel     0.28  0.06  0.07  0.06  0.33  0.18  0.08  0.34  0.47  0.21  0.23  0.37
```

Smaller at eleven rungs of twelve, and by five to twenty fold over rungs 2 to 4.
The floor mean follows it down - **1.501 against the control's 2.926**, 37 blocks
apiece, and 0.13 items held against 1.61.

The ablation says where the capacity went instead, and it is the opposite of
what the architecture was for:

```
  band zeroed                      control    duel (detach)
  the rung, and the lives left         28%             149%
  the layout, 240 cells                81%              15%
  purse, tray, what is coming          45%              20%
```

The dueling net is **more** concentrated on the two run-progress scalars than
the plain one, not less, and it has largely stopped reading the board.

## F.3 Why, and it is the architecture rather than the tuning

Standard dueling reads the action as an **output**: `A(s, .)` is a vector with
one unit per action, and the constraint subtracts the mean *along the action
axis at a fixed state*. That subtraction is what makes an advantage an
advantage, and it is free because one forward pass produces the whole vector.

This repo reads the action as an **input**, because the menu is 100 to 545 keys
and changes shape every step - which is why `feature::pair` exists at all. There
is no action axis to subtract along without scoring the whole menu, and a
differentiable menu mean is eighteen towers a sample against three.

Both anchors fail, for opposite reasons, and between them they cover the
options:

* **anchored with gradient** - the tower is evaluated twice and subtracted, so
  the advantage's own parameters cancel and it learns nothing (E.2);
* **anchored without gradient** - nothing centres the advantage any more, and
  since `A` reads the state as well as the move, it is simply a second `Q`
  network summed with a state-only one. Nothing makes it an advantage.

The second reading has a tell that is hard to argue with. `b3` and `vb3` are
both **0.8843**, to every digit printed. They must be: the two output biases sit
either side of an addition, so each receives exactly `dLoss/dQ`, and starting
from zero at one learning rate they move identically for ever. The two towers
are not doing two jobs; one of them is a duplicated parameter and a handicapped
copy of the other.

And the behaviour matches. The key histogram is `place` 62.3%, **`clear` 25.4%**,
`undo` 10.8% - a quarter of every press is `ClearSlot` or `ClearAll`, which
§A.1 measures as the one thing a carried board cannot survive.

## F.4 What is refuted, precisely

**Dueling, in a `Q(state, action)` architecture with a variable menu.** Not
dueling in general, and not the diagnosis in §A and §B, which stands untouched -
both arms agree with it, and the detached one agrees with it harder.

What the two arms rule out is the cheap version. Making this work would mean
storing the menu in the buffer and paying for a differentiable mean over it, and
that is a factor of six on a two-hour run before anybody knows whether it helps.

That leaves the two families of §6 that do not need an action axis, and the
measurements in §A point at them rather than at this one:

* **return decomposition** - redistribute the episode's return onto the
  decisions that predicted it, which is the direct attack on A.2's displacement
  and A.3's broadcast;
* **temporal abstraction** - credit a whole packing rather than forty presses,
  which turns 350 smeared decisions an episode into about nine.

Neither has been run, and `CLAUDE.md` trap 51 is the reason this section does
not say which will work.

---

# G — Return decomposition: a wash on depth, and negative on the gap

Read off `ca770b1`. `analysis/nets/qrow-r27-redist.log`, 3,000 episodes, 5,981 s,
same seed stream and the same control as E and F. `QROW_REDIST=1` stops paying
`worth` on the last press and pays the telescoped increments at the packings
that won them, which is return-equivalent by construction and checked to a
residual of `+0.00e0` on the first episode.

## G.1 The four arms

| | floor mean (ep 2100+) | median | range | items held | Q spread |
|---|---:|---:|---|---:|---:|
| control, terminal reward | 2.926 | 2.80 | 1.52-5.16 | 1.61 | 0.314 |
| duel, gradient anchor | 1.367 | 1.32 | 1.12-1.92 | 0.04 | 0.200 |
| duel, detached anchor | 1.501 | 1.44 | 1.00-2.04 | 0.13 | 1.952 |
| **redistribution** | **2.939** | 2.92 | 1.76-4.12 | 0.99 | 2.025 |

Thirty-seven blocks apiece. **+0.013 of a rung**, against a block noise of about
0.4 - which is nothing, and is the first arm of the three that is not a
regression.

What did move is the *shape*: the range narrows at both ends, 1.76-4.12 against
1.52-5.16. Lower variance and the same mean is what a redistribution that
preserves the sum and removes the terminal spike should do, and it did it.

## G.2 The target spike is gone and the mean is untouched

```
                      max target ever   mean target   worst clipping
  control                        2702        +3.718             0.2%
  redistribution                   23        +3.738             0.2%
```

**A hundred and twenty-two fold smaller maximum at the same mean.** The mean
cannot move - the redistribution is return-equivalent, so it is the same total
by construction - and the maximum is the terminal spike being spread over the
episode. This is the cleanest confirmation available that the mechanism did what
it was written to do.

It also settles a plausible reason for expecting a gain, in the negative: the
worst clipping either arm ever saw is **0.2%**, so the knee of 5 was never
scaling the control's gradients away and there was no clipped gradient here to
rescue. `CLAUDE.md` trap 53 is about a knee too far *out*; this would have been
a knee too far in, and the column says it never happened. (The detached dueling
arm hit 12.3% in eight blocks, which is one more way that arm misbehaved.)

## G.3 And the gap - the thing it was aimed at - got worse

```
  gap as a share of the state value, at the written control's states
  rung        1     2     3     4     5     6     7     8     9    10    11    12
  control  0.32  0.70  1.34  1.69  1.42  0.59  0.14  1.69  0.33  0.49  2.93  0.46
  redist   0.07  0.53  0.60  0.20  0.04  0.40  0.00  0.00  0.00  0.00  0.00  0.00
```

**Exactly 0.0000 at six consecutive rungs.** Not small - zero: the network scores
its first and second choice identically from rung seven up, so the greedy policy
there is decided by menu order and nothing else. That is `CLAUDE.md` trap 50's
fault reached by a third road.

And the ablation says the capacity went the same way it went for the dueling
arms:

```
  band zeroed                      control    redist
  the rung, and the lives left         28%      104%
  the layout, 240 cells                81%       51%
```

## G.4 Why, and it is what the doc comment predicted

`row::spread`'s own comment says it in advance: *"What it does not buy is credit
at the decision: it moves the payment from the end of the run to the end of the
rung, and A.1 measures 78% of rungs as coming out the same whatever was pressed
at them."*

That is exactly what happened. The redistribution anchored the **value** function
locally - spread 2.025 against 0.314, targets 23 against 2702 - and gave the
**policy** nothing new to discriminate on, because the redistributed reward is
still not a function of the decision. It is a function of the rung, and the rung
is not a function of the decision at 78% of the states being graded.

The behaviour has a tell that is almost too on-the-nose. The trained policy's
key histogram is:

```
  pin      1261    58.2%          place     310    14.3%
  undo      193     8.9%          lock      169     7.8%
```

**It presses `Pin` for 58% of its presses** - a shop-shelf toggle that does
nothing whatever to the board - and reaches the same depth as the control, which
packs. A whole arm of this experiment is an accidental replication of §A: past
rung three, a policy that does almost nothing gets where a policy that packs
gets.

## G.5 What is and is not refuted

Refuted: **redistributing the return over rungs**. It is a genuine
return-equivalent redistribution and it is RUDDER's construction with the
regression replaced by a closed form - but RUDDER's power is in *where* it
redistributes, and it redistributes onto the state-action pairs a learned model
says predicted the return. Onto time-blocks is the cheap half, and the cheap half
is measured here as a wash.

Not refuted: RUDDER proper. A learned contribution analysis over the sequence
could in principle put the credit on the *press* that made the board that won
rung nine, which is the thing §A.2's +1.8 rungs is made of and the thing none of
the four arms has touched. It needs a sequence model over 350-step episodes,
which is a different order of work from any arm in this document.

## G.6 Four arms, one direction

Every intervention tried has moved the ablation the same way:

```
  zeroing the rung and the lives left, as a share of Q's across-state spread
    control                28%
    duel, detached        149%
    redistribution        104%
```

Three architectures and two reward schemes, and each one ends up *more*
dependent on the two numbers that say where the run is and less on the board.
That is not four failures of tuning. It is four measurements of the same
property of the environment, which §A states directly and which no change on the
learner's side of the boundary has moved.

---

# H — Full RUDDER, asked what it would redistribute onto

Read off `592fe4c`. `--bin qrudder`, 120 episodes under the control arm's best
net at eps 0.05, 31,497 presses, 74 s.

G redistributed the return over **rungs**, in closed form. RUDDER proper
redistributes onto the state-action pairs a *learned* model says predicted the
return, which could in principle put the credit on the press that built the item
that won rung nine. This asks it to, and reads the answer before any arm is run:
**a contribution analysis cannot manufacture signal that is not in the data**,
and if its credit lands where G's did then G has already measured what that is
worth.

## H.1 The forecaster, and why it is Markov

RUDDER uses an LSTM because Atari is partially observed. This game is fully
observed - the board, the purse, the rung and the lives *are* the state - so a
prefix should tell a forecaster nothing the last state does not. That claim is
tested rather than assumed:

```
  returns to go: mean +34.81, sd 59.48
  epoch   1   held-out R2 +0.537
  epoch  40   held-out R2 +0.511
```

**A Markov forecaster explains about half the variance of the realised return**,
and it does so in one epoch. It does not improve with training, which is what a
model that has already extracted what its inputs carry looks like.

Half is not all, and this does not establish that a sequence model would add
nothing - the unexplained half is at least partly *future* stochasticity, the
shop's next deal and which creature is coming, which no predictor of the past
can reach. What H.2 shows is that the half it does explain is not the half that
would help.

## H.2 The credit lands nowhere in particular - less selectively than chance

`R_t = r_t + V(s_t) - V(s_{t-1})`. Where does the mass go? Beside it, the same
analysis run with an **untrained** forecaster, because `R_t` is a difference of
two forecasts one press apart and most of what it measures could be the
difference of two prediction *errors*:

```
  concentration of |R_t|             untrained (control)     trained
  presses that won their rung                       0.7x        1.2x
  the last press of any packing                     4.6x        1.1x
  presses that finished an item                     1.6x        1.0x
```

A ratio of 1.0 is credit spread exactly in proportion to how many presses there
are. **The trained analysis is at 1.0 to 1.2 everywhere**, and the *untrained*
one concentrates four and a half times harder at the presses a fight is fought
with - because those are simply the presses where the state changes most, and a
random projection notices that.

So training does not sharpen the redistribution. It **flattens** it.

By key, the same thing:

```
  key         presses   mean |R|    share      share of presses
  place        15194      4.3297    52.0%               48.2%
  undo          8242      4.5119    29.4%               26.2%
  buy           1875      4.7024     7.0%                6.0%
  unequip       1689      4.6358     6.2%                5.4%
  pin           2121      0.4928     0.8%                6.7%
```

Every row is its own share of the presses, to within a point or two - except
`pin`, which the forecaster prices at a ninth of everything else. That one row
is worth keeping: **it knows perfectly well that pinning a shelf does nothing**,
so the flatness elsewhere is not blindness. It is an accurate report that a
place and its undo, a buy and a sell, all move the forecast by about the same
amount.

## H.3 Which is A, arrived at from the last available direction

What a return predictor learns here is what B measured four times over: the
return is a function of where the run is on the ladder. That function is
**constant within a packing**, so its differences across the presses of a
packing carry no signal, and what is left is noise distributed evenly.

RUDDER finds the state-action pairs that *predict* the return. In this
environment the thing that predicts the return is the rung, and the rung is not
a function of the press - at 78% of the states being graded (A.1), by direct
counterfactual.

**This is the negative result the method is entitled to**, and it is worth being
precise about what it is not. It is not "RUDDER does not work". It is: a
contribution analysis over these episodes, with a forecaster that genuinely
predicts their returns, distributes credit uniformly over presses - so the arm
it would justify is one whose reward is a flatter version of the reward G
already measured as a wash. Two hours of training would produce a number this
binary predicted in seventy-four seconds, and that ordering was the point.

## H.4 The five arms

```
  arm                          floor mean   gap at rungs 7-12      verdict
  control, terminal reward          2.926   0.14-2.93%             the baseline
  duel, gradient anchor             1.367   0.00-0.34%             refuted (E.2)
  duel, detached anchor             1.501   0.00-0.47%             refuted (F.2)
  redistribution over rungs         2.939   0.00%                  a wash (G.1)
  RUDDER, learned redistribution        -   -                      not run (H.2)
```

Four trained arms across three architectures and two reward schemes, and the
ablation moved the same way in every one: zeroing the rung and the lives left
moves `Q` by 28% of its across-state spread in the control, 104% under
redistribution and 149% under the detached duel. Nothing on the learner's side
of the boundary has moved the thing A measures, and the fifth arm's own
diagnostic says in advance that it would not either.
