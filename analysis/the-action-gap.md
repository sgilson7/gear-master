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

---

# I — Temporal abstraction, and the prediction attached to it was wrong

Read off `d83429b`. `analysis/nets/qrow-r28-abstract.log`, 3,000 episodes,
5,606 s, same seed stream and the same control. `QROW_ABSTRACT=1
QROW_REDIST=1`: the target for every press in a packing is the reward that
actually accrued to the end of that packing plus `gamma^k` times the value at
the start of the next one. An episode goes from about 350 chained bootstraps to
about nine.

**This is the first arm that moved anything**, and the prediction written into
its commit message said it would not.

## I.1 Depth

| | reward | backup | floor mean | sd | min block |
|---|---|---|---:|---:|---:|
| control | terminal | press | 2.926 | 0.933 | 1.52 |
| G, redistribution | per-rung | press | 2.939 | 0.456 | 1.76 |
| **I, abstraction** | per-rung | **packing** | **3.265** | 0.375 | **2.72** |

Thirty-seven blocks apiece. **+0.339 of a rung, which is 2.0 standard errors**
of the difference - not overwhelming, and the first thing in this document that
is outside the noise at all. **It did not survive a second seed stream; see J.** The distribution is the better evidence: **33 of 37
blocks are above the control's median**, against the control's own 16 of 37, and
the arm's *worst* block is 2.72 against a control median of 2.80.

## I.2 The gap, which is what it was measured against

```
  rung        1     2     3     4     5     6     7     8     9    10    11    12    13
  control  0.32  0.70  1.34  1.69  1.42  0.59  0.14  1.69  0.33  0.49  2.93  0.46  0.38
  abstract 5.34  2.74  3.12  0.25  4.56  0.28  0.99  0.69  2.63  2.15  1.37  1.81  7.54

  keys within 1% of the best
  control     8    10     6     2     6     6    10     7     8     3     3    19    23
  abstract    1     3     3     4     3     5     3     2     2     2     2     2     1
```

> **Corrected by J.2.** "Mean gap 2.57% against 0.96%" is one normalisation -
> the gap over the state's own value - and the other one available does not
> agree: over `Q`'s across-state spread the gap is 1.44% against 1.79%, which is
> flat to slightly *down*. The tie count below replicated on a second stream and
> the ratio below did not survive being divided differently. Read the ties.

**Mean gap 2.57% against 0.96%**, and the tie-breaking problem is largely gone:
a mean of **2.5 keys inside one percent of the best against 8.5**, and a unique
best at rungs 1 and 13 where the control had eight and twenty-three.

## I.3 And the ablation reverses, for the first time

G.6 said four arms had all moved the same way and that nothing on the learner's
side of the boundary had moved what A measures. This moved it:

```
  band zeroed                    control   duel detach   redist   abstraction
  the rung, and the lives left       28%          149%     104%           18%
  what the board is                  38%           26%      18%           73%
  the layout, 240 cells               81%           15%      51%           77%
```

The dependence on the two run-progress scalars is **lower than the control's**,
and the two bands that describe the *board* are the highest they have been in
any arm. This is the first network in five that is looking at the thing it is
supposed to be packing.

## I.4 What it presses

```
  lock       912    38.6%          place     574    24.3%
  pin        308    13.0%          unequip   218     9.2%
  undo       166     7.0%          buy        76     3.2%
```

> **Corrected by K.2.** `Verb::Lock` is a toggle and half of these presses are
> unlocks, measured. The thrash did not stop; it moved from place-then-undo to
> lock-then-unlock, and the board shows it - 1.50 items against the plain arm's
> 4.32. Read K.2 rather than the paragraph below.

**`Lock` is 38.6% of its choices**, against 0.0% in every arm before the feature
fix and 0.4% after it (`analysis/the-collapse.md` M4.3). And `undo` is **7.0%**
against 22.9% in the control and 45.5% at M4.3. The place-then-undo thrash that
three milestones of this mission read as a free-action problem is not there.

M1.1 is why that matters and it was written two missions ago: an unlocked item
negotiates with whatever it touches, so a packer that cannot hold a multi-item
board cannot clear a rung that needs one. This is the first policy to lock.

## I.5 The prediction was wrong, and why

`QROW_ABSTRACT`'s doc comment and its commit both said: the recorded state at
the start of the next packing is the same for every press of this one, so the
bootstrap stops varying with what was pressed - and since the reward is zero on
99% of presses, that successor state is *the last place action-dependence
survives in this target*. The gap should therefore fall.

It rose, threefold. The error was in the last clause, and the measurements in B
are what should have caught it.

One-step TD's action-dependence flows entirely through `max_a Q(s', a)`. B
measured that `Q` here is very nearly `V`: two of 321 inputs move it more than
the other 319 together. **A bootstrap through a value function that is almost a
state-value launders the action out of the target.** So the successor state was
not where action-dependence survived; it was where it was being destroyed.

Extending the horizon to forty presses of *realised* reward moves the target
toward Monte Carlo, which is unbiased about the action taken - and across a
replay buffer, different actions at similar states are followed by different
realised futures, which is exactly the signal a bootstrapped `V` cannot carry.

That is the mechanism, it is consistent with all five arms, and it says the
right way to read A is not "the environment has no signal" but **"the signal is
there and one-step bootstrapping cannot reach it."** A.2's +1.8 rungs was always
the evidence for that and it took five arms to read it properly.

## I.6 What this does not say

* **+0.339 at 2.0 standard errors is one run.** It wants a repeat on a second
  seed stream before it is a fact, and this document has been wrong at two
  standard errors before (`analysis/the-collapse.md` M6.1).
* It is still far short of the written control's 5.62.
* Abstraction and redistribution moved together, and G measures redistribution
  alone at +0.012, so the abstraction is carrying it - but an abstraction-only
  arm was not run, because with a terminal reward there is nothing for an
  option's return to accumulate.
* Nothing here says the packing is the right option boundary. It is the obvious
  one and it is the only one tried.

---

# J — The confirmation: the depth did not survive, three other things did

Read off `684039a`. `analysis/nets/qrow-r29-s2-control.log` and
`qrow-r30-s2-abstract.log`, 3,000 episodes each, `QROW_SEED=0xFACEB00C` -
chosen out of eight candidates because the written control reads **5.8** through
that stream against 6.0 through the original, and the others read anywhere from
3.5 to 13.0. Both arms move, because a number from one stream is comparable only
to a number from the same one.

## J.1 The depth did not confirm

| | floor mean | sd | min | max | items held |
|---|---:|---:|---:|---:|---:|
| stream 1, control | 2.926 | 0.933 | 1.52 | 5.16 | 1.61 |
| stream 1, abstraction | 3.265 | 0.375 | 2.72 | 4.52 | 1.15 |
| stream 2, control | 2.704 | 0.790 | 1.36 | 4.58 | 1.55 |
| stream 2, abstraction | 2.774 | 0.372 | 2.24 | 3.84 | 0.89 |

```
  stream 1   abstraction - control  = +0.339 +/- 0.165  = +2.0 se   (33/37 blocks above the control's median)
  stream 2   abstraction - control  = +0.069 +/- 0.144  = +0.5 se   (22/37)
  pooled                              +0.186 +/- 0.108  = +1.7 se
```

**I.1's headline was a seed.** The pooled estimate is +0.19 of a rung at 1.7
standard errors, which is not a result, and `analysis/the-collapse.md` M6.1 is
the record of this document making exactly this mistake before at exactly this
threshold. The confirmation run is what it is for.

## J.2 And the gap claim was one normalisation of two

I.2 reported the action gap as a share of the state's own value and called it
threefold. Divided by `Q`'s across-state spread instead - which is the better
proxy for the estimation noise the gap has to survive, because a network whose
values are six times larger has six times larger errors too - it does not move:

```
  net             mean gap   gap/bestQ   gap/Qsd   keys within 1% of best
  s1 control        0.0373       0.96%     1.79%                     8.5
  s1 abstract       0.1963       2.16%     1.44%                     2.5
  s2 control        0.0505       1.08%     5.14%                     5.7
  s2 abstract       0.2549       6.03%     4.88%                     2.2
```

The abstraction net's values are five to six times wider across states and its
gaps scale with them. **Relative to the value's magnitude the gap improved on
both streams; relative to the value's spread it did not.** I.2 quoted the first
and did not check the second, and the strong form of that claim is withdrawn.

What is left of it is the tie count, which fell on both streams - 8.5 to 2.5 and
5.7 to 2.2 - and which is the measure with a behavioural meaning: how many keys
the greedy argmax cannot separate from its own choice. It shares the `bestQ`
normalisation and should be read with that in mind.

## J.3 Three things did replicate, and cleanly

**The variance halves.** On both streams the abstraction arm's block standard
deviation is under half the control's - 0.375 against 0.933, and 0.372 against
0.790 - and its worst block is far better while its best is worse. It is a
policy that never has a bad block, and the control's higher mean on stream 1 was
partly carried by occasional very good ones.

**The ablation reverses, and by more the worse the control is.**

```
  zeroing the rung and the lives left, as a share of Q's across-state spread
                       control    abstraction
    stream 1               28%            18%
    stream 2               84%            39%
```

Halved on both. On stream 2 the control is a far worse case - 84% of its answer
is those two inputs - and the abstraction still cuts it to 39%, with the board
bands rising to 48% and 55%.

**It locks, and it stops thrashing** - *and K.2 measures half of that locking as
unlocking, so this row is the thrash moving rather than stopping.*

```
  stream 2       lock     undo
    control      1.1%    27.4%
    abstraction 45.3%     6.8%
```

Against stream 1's 38.6% and 7.0%. `analysis/the-collapse.md` M4.3 measured
`undo` at 45.5% and three milestones of this mission read that as a free-action
problem; M1.1 measured `lock` at 0.0% and named it as the reason a packer cannot
hold a multi-item board. **Both reverse under the semi-MDP backup, on two
independent seed streams**, and neither the action space nor the verb costs were
touched to do it.

## J.4 What the pair of runs actually establishes

* The semi-MDP backup changes what the network *looks at* and what it *presses*,
  reproducibly, and in the direction A says it should.
* It does **not** reliably reach a deeper rung. +0.19 pooled at 1.7 se.
* It makes the policy markedly more consistent, which is a real result and not
  the one that was claimed.

Those are compatible, and the compatible reading is the one this whole document
keeps arriving at from new directions: a better packer is worth about a fifth of
a rung, because A.1 measured 78% of rungs as coming out the same however they
are packed. **Depth was never going to be a sensitive instrument for packing
quality, and every arm here has been graded on it.** The thing to fix next is
probably the measurement rather than the learner - an evaluation that scores the
board instead of the ladder would have separated these five arms in minutes
rather than in fourteen hours of training.

---

# K — Grading the board instead of the run, and what it caught in ten minutes

Read off `b15e604`. `--bin qgrade`. J.4 ended on the claim that depth is not a
sensitive instrument for packing quality and that the measurement was the thing
to fix. This is the fix, and it is not a new idea: `scoring::reach` has been in
the repo since THE APPRENTICE and asks the question directly - **how many
consecutive rungs does this board clear from where it stands?**

Depth is not being replaced by a proxy. `reach` *is* depth, asked of every board
rather than once of a whole run: it walks the ladder ahead of the board and
stops at the first fight it loses, which is where a Rogue run stops. You do not
reach a deeper rung unless the board is good enough, and that implication is
what it evaluates.

## K.1 The instrument

Twenty runs, greedy, Rogue at Medium:

```
  packer                       boards      reach       se       gain    items    depth
  the written control             173      1.179    0.120     0.0562     3.42     5.50
  r24, plain (the control arm)    185      1.676    0.176     0.2179     4.32     6.10
  r28, abstraction                131      0.779    0.119     0.0258     1.50     3.30
  r27, redistribution             114      0.430    0.074     0.0060     0.60     2.55
```

Twenty runs give twenty depths. The same twenty give **173 to 185 boards**, each
a fact rather than a draw because the fight is deterministic. And the depth
column shows why that matters: `qhand` on the same seeds gives r24's runs as

```
  [2, 3, 3, 45, 7, 2, 13, 2, 3, 9, 3, 3, 2, 3, 7, 5, 2, 3, 2, 3]
```

**A mean of 6.1 with a median of 3, carried by one seed.** Every arm in E to J
was graded on that statistic, over three thousand episodes and two hours apiece.

## K.2 What it caught: `lock` is the fourth cheapest key

I.4 and J.3 both reported the abstraction arm pressing `lock` on 38-45% of its
choices and `undo` on 7% against the control's 23-27%, and read it as the first
policy to hold a multi-item board. That reading was wrong.

`Verb::Lock` is a **toggle**. `Console::apply` calls `toggle_lock_item` and
answers "locked" or "unlocked", and `menu` offers it for every assembled item
whether it is locked or not. Pressing it twice on the same item returns the
board to where it was, for two presses of the forty.

Counted directly, ten runs apiece, by watching the locked-item count either side
of every `lock` press:

```
  packer              locked   unlocked   unlock share
  r24, plain              35         34            49%
  r27, redistribution     86         83            49%
  r28, abstraction       458        454            50%
```

**Half of every lock press is an unlock.** The abstraction arm does it thirteen
times as often as the plain one - 912 presses against 69 - and its board shows
it: 1.50 items against 4.32, and 0.78 rungs of reach against 1.68.

So the semi-MDP backup did not stop the thrash. It **moved** it, from
place-then-undo to lock-then-unlock, and `CLAUDE.md` trap 44's own words are the
description: *there is always another cheapest key.* What is new is that nobody
took a verb away this time - the backup simply made a different no-op the
cheapest, which means the trap is not about the action space at all. It is about
an environment where doing nothing is never punished (A.1).

I.4 and J.3 are corrected in place.

## K.3 And the ordering by board quality is not the ordering by depth

```
  by reach     r24 1.676  >  written 1.179  >  r28 0.779  >  r27 0.430
  by floor     r28 3.265  >  r24 2.926  >  r27 2.939        (stream 1, exploring)
```

The arm that won on the training floor builds the **worst** boards of the two
trained ones, and the plain arm builds the best - better than the written
control on these seeds. Two things are behind that and both are worth keeping:

* `runs/duel-control.txt` is a **best block** out of thirty windows, played
  greedily. The floor mean is the average policy through training and these are
  not the same statistic - `analysis/the-collapse.md` M0 is the record of that
  distinction costing a mission a wrong label.
* The floor is measured with 5% exploration, and A.1 measured random presses as
  the one thing a carried board does not survive. A policy that tolerates being
  jostled scores better there than one that does not.

## K.4 What this changes about how to run the next arm

* **Grade on `reach`, not depth.** Ten minutes and 180 observations against two
  hours and twenty, and it separated four packers at four to nine standard
  errors where depth separated none of them reliably.
* **Audit the no-ops before reading a key histogram as behaviour.** Three
  milestones read `undo` at 45% as a policy; one read `lock` at 45% as a virtue.
  Both were the same key press with a different name on it. A histogram wants a
  column saying how many of those presses changed the board.
* The caveat on K.1: 185 boards come from 20 runs, so boards within a run are
  correlated and the true standard errors are wider than printed. The orderings
  are large enough to survive that; the exact figures are not.

---

# L — The missing bit was real, and it moved the thrash rather than stopping it

Read off `e4a53dd`. `qrow-r31-w39-control.log` and `qrow-r32-w39-abstract.log`,
3,000 episodes each at pair 322, same seed stream, same settings as r24 and r28
in every other respect. The only change is `feature::MOVE` gaining one number:
**whether a lock press would lock or unlock.**

## L.1 It worked, exactly where the backup left room to learn

```
  what the `lock` key does when it is pressed, 10 runs apiece
  packer                    locked   unlocked   unlock share
  r28, abstraction (321)       458        454            50%
  r31, control (322)            73          0             0%
  r32, abstraction (322)       574        536            48%
```

**The plain arm never unlocks again.** Seventy-three locks, none of them undone.
Given the bit, it learned in one run that one face of the toggle is worthless -
which is what K.2 said would happen and is the whole case for the widening.

**The abstraction arm still toggles**, at 48%, and does it more than before -
1,110 presses against 912. The feature let it *tell* the two apart and gave it
no reason to prefer either, because the semi-MDP backup builds every press's
target from the same next-packing state (I's own prediction, which was wrong
about the gap and right about this). Where the target does not vary with the
action, a distinguishable no-op is still a free no-op.

## L.2 And the plain arm's thrash went straight back to `undo`

```
  r31, control (322)      place 49.8%   undo 31.0%   clear 8.9%   buy 6.4%   lock 2.6%
  r24, control (321)      place 44.8%   undo 22.9%   pin  8.1%    buy 5.2%   lock 0.0%
  r32, abstraction (322)  lock  51.1%   pin  16.4%   place 13.8%  buy 5.7%   undo 4.7%
```

Undo is **31.0%**, up from 22.9%. The presses freed from lock-toggling did not
go into packing; they went into the next cheapest key.

That is `CLAUDE.md` trap 44 for the **fifth** time, and the list is now long
enough to be a proof rather than a pattern:

```
  Rotate 400/420   ->  removed the verb        ->  Pin 410/420
  Pin (via M1)     ->  fixed the features      ->  Undo 45.5%
  Undo             ->  the semi-MDP backup     ->  Lock 45%, half of it unlocking
  Lock (via L)     ->  fixed the features      ->  Undo 31.0%
```

Two verb removals and two feature fixes, and each one relocated it. The trap's
own sentence - *there is always another cheapest key* - has been read as advice
about action spaces for three missions. It is not. **A.1 is why**: 78% of rungs
come out the same however they are packed, so the environment never charges for
doing nothing, and the cheapest key is whatever the menu happens to offer.

`NOTHING` is 0.0 and its comment says *"there is nothing to dither into: the
packing budget bounds each rung at forty presses"*. That is falsified five
times.

## L.3 What the widening did buy

**Buying roughly doubled**, which was the other half of the complaint:

```
  buy, as a share of choices     321      322
    control                     5.2%     6.4%
    abstraction                 1.8%     5.7%
```

Depth did not move, and by now that is the expected answer rather than a
disappointment:

```
                                floor mean      the extra column
  control      321 -> 322    2.926 -> 3.024     +0.098 +/- 0.229
  abstraction  321 -> 322    3.265 -> 3.368     +0.102 +/- 0.115
```

Both inside noise, and the abstraction-minus-control gap is unchanged at +0.343
+/- 0.195 against 321's +0.339 +/- 0.165 - which is at least a consistent
replication of I.1 on a third pair of runs, if not of its significance.

Board quality is flat to slightly down against the written control:

```
  packer                boards    reach       se    items    depth
  the written control      173    1.179    0.120     3.42     5.50
  r31, control (322)       136    0.971    0.142     1.96     3.75
  r32, abstraction (322)   145    1.034    0.149     1.68     4.15
```

## L.4 The change the evidence now supports, and nobody has made

Every fix so far has been about *which verbs exist* or *how they are described*.
The measurement says the fault is neither: it is that **a press which leaves the
board where it was costs nothing**, and there are always more ways to spend
forty presses doing that than there are ways to spend them well.

The general form is verb-agnostic and the plumbing is already there.
`row::Pressed` carries the board's `Figures` either side of every press and the
item count after it, so *"this press put the board back where the one before it
found it"* is computable today with nothing new recorded. Charging for that
catches place-then-undo, lock-then-unlock, pin, rotate and whatever the sixth
one turns out to be, in one rule - and it does not punish a single legitimate
lock, which a flat "changed nothing" charge would.

That is trap 44's own prescription - *charge for what the board does, not for
what the verb is called* - and it has never been implemented, because the
constant that would carry it is 0.0 behind a comment saying it is not needed.

---

# M — The charge against the gap it has to close

Read off `c9e4202`. `--bin qcharge`, six greedy runs a net, whether a candidate
revisits decided by **pressing it on a clone** and fingerprinting the result
rather than inferred from the verb.

`row::revisit` was sized in the previous section against the **episode return** -
the total came to about one assembled item, which is the right way to size
something that must not swamp the objective and no way at all to size something
that must flip an `argmax`. What it has to overcome is how far the chosen
revisit out-scores the best press that is not one.

```
                                        rv-control     rv-abstract
  decisions                                   1240            1412
  menu                                     47 keys         72 keys
  ...of which revisit                           18%             11%
  it chose a revisit on                         86%             80%
  a clean press was available on               100%            100%

  best - second (the ordinary gap)   median 0.0055   median 0.0112
  best - best clean, where it chose  median 0.0056   median 0.0776
                                       90th 0.0328     90th 0.3954
  best - done                        median 0.1786   median 0.6596

  the charge, 0.02, as a multiple of
    the median gap it must close                3.58x           0.26x
    the 90th percentile of it                   0.61x           0.05x
```

## M.1 Two arms, two different answers

**For the plain arm the charge is already three and a half times the gap it has
to close, and the policy revisits anyway** - 86% of decisions, with a clean
press available at every one of them. So the charge is not the binding
constraint there. It is arithmetically sufficient against the *fitted* gap and
behaviourally ineffective, which puts the fault below it: a systematic 0.02 is
buried under the network's own approximation error, and `CLAUDE.md` trap 44's
original sentence is the same observation - *a no-op cost 0.01 while the value
estimates were spread over 1.70*.

**For the abstraction arm it is four times too small** - 0.26x the median gap,
0.05x the 90th. There the arithmetic says plainly that it cannot work.

## M.2 And there is no size that works for both

To clear the 90th percentile of the gap wants 0.033 for the plain arm and
**0.40** for the abstraction arm. At 0.40 a press against an 80% revisit rate
and forty presses, one packing costs 12.8 and an episode of five costs about
**64** - against episode returns of four to nine at the rungs these policies
reach. That is the "step charge a hundred times the objective" that
`design/HANDOFF-the-collapse.md` lists as a known way to kill a run, arrived at
from the requirement rather than by accident.

So the window is narrow for the plain arm and empty for the abstraction one.
**A per-press charge cannot be simultaneously above the network's error floor
and below the objective**, and that is an argument about arithmetic rather than
about tuning. What it argues for is making a revisit *unavailable* rather than
expensive - masking it out of the menu, or ending the packing on one - which is
the kind of intervention the action-space literature says is decisive where
shaping is not, and which changes the rules the agent plays under.

## M.3 `Done` is a long way behind and not out of reach

`Move::Done` is offered at every decision and chosen on 0.0% to 0.9% of them,
and the policy rates it **0.18 and 0.66 below** its own choice. A per-press
charge accumulated over a whole packing is 0.02 x 30 = 0.6, which is the same
order - so for the plain arm `Done` is reachable in principle by the charge it
already has, and for the abstraction arm it is not.

That is worth knowing because `Done` is the one press guaranteed to cost nothing
further, and `Packing`'s own doc comment predicted this whole section:
*without it a packer dithers, and a step cost alone does not teach it to stop;
it teaches it to press the cheapest key.*

## M.4 Two of my own bugs, because both were instructive

`Move::Done` was being **charged as a revisit**. It leaves the board exactly
where it stands, so the fingerprint test called it a cycle and put a price on
the one action a dithering packer is supposed to reach for. Fixed, pinned in
`row.rs::saying_done_is_not_charged_as_a_revisit`, and the two arms running at
the time were restarted because they exist to test precisely that.

And `qcharge`'s first answer was that 1% of the menu revisits, against the
trainer's 74%. Its `seen` set was never pushed to, so "revisit" meant "back to
where this packing started". **`rustc` printed `variable does not need to be
mutable` about that exact line**, which is the whole diagnosis, and I read past
it to the numbers. A warning on a variable a diagnostic is built around is not
housekeeping.

---

# N — The charge does not work, and M said why before the run

Read off `14c2339`. `qrow-r35-revisit2-control.log` and
`qrow-r36-revisit2-abstract.log`, 3,000 episodes each at pair 322 with
`QROW_REVISIT=0.02`, the fingerprint corrected (no pin flag) and `Done` no
longer charged. Against r31 and r32, one variable.

## N.1 It moved nothing

```
                              floor mean          the charge      revisits   done
  control, no charge     3.024 +/- 0.169                    -             -      -
  control, charged       3.102 +/- 0.115    +0.078 +/- 0.205         76.3%   0.0%
  abstraction, no charge 3.368 +/- 0.097                    -             -      -
  abstraction, charged   3.338 +/- 0.091    -0.030 +/- 0.133         75.0%   0.4%
```

Depth is unchanged in both. **The revisit share at the floor is 76.3% and
75.0%, against 74.5% and 72.3% when the charge was first tried and 80.3%
uncharged.** Three quarters of every press puts the board somewhere it has
already been, with a charge on it, a corrected fingerprint, and `Done` free.

And `Done` is chosen on **0.0% and 0.4%** of the decisions it is offered at -
which is every one of them.

M.2 gave the arithmetic before this ran: the charge has to exceed the gap by
which a revisit out-scores the best clean press, that gap's 90th percentile
wants 0.033 and 0.40, and at 0.40 an episode costs about 64 against returns of
four to nine. **There is no size that is simultaneously above the network's
error floor and below the objective**, and the run is what that looks like.

## N.2 The lock fix works in one arm and cannot work in the other

```
  what the `lock` key does when pressed          locked   unlocked   share
  the plain arm, at 322, charged                     17          0      0%
  the abstraction arm, at 322, charged              713        705     50%
```

The plain arm has not unlocked once in any run since the feature landed. The
abstraction arm toggles at 50% in **every** run - before the feature, after it,
and now with a charge on top. Its `lock` share is 55.9%, the highest yet.

That is I's structural point holding: the semi-MDP backup builds every press's
target from the same next-packing state, so nothing within a packing varies with
what was pressed. **A feature the network cannot get a gradient on is not a
feature**, and a charge it cannot attribute is not a charge.

## N.3 And the plain arm's thrash is at its highest

```
  the plain arm's keys, at 322      place   undo   buy   pin   lock
    no charge                       49.8%  31.0%  6.4%  0.0%   2.6%
    charged (this run)              45.2%  38.7%  6.8%  1.9%   1.0%
```

`undo` at **38.7%**, up from 31.0%. Six relocations now: `Rotate`, `Pin`, `Undo`,
`Lock`, `Undo` again, and `Undo` higher still under a charge meant to stop it.

## N.4 What did move, and it is not the charge

```
  packer                       boards    reach       se    items    depth
  the written control             148    1.291    0.135     3.68     6.00
  the plain arm, charged          109    0.771    0.118     2.27     3.81
  the abstraction arm, charged    124    1.210    0.160     1.63     4.31
```

**The abstraction arm's board quality is 1.210 against the written control's
1.291** - the closest a trained packer has come, on the instrument that has a
hundred and twenty observations rather than sixteen. It gets there holding 1.63
items against the control's 3.68, which is its own puzzle and a better one than
any of the last five arms produced.

## N.5 The case for charging is closed

Six relocations, two verb removals, two feature fixes and one charge, and the
measured share of presses that put the board back where it was is where it
started. The remaining interventions are structural rather than economic, and
both change the rules the agent plays under:

* **mask a revisit out of the menu.** Exact, and `qcharge` already computes it -
  press each candidate on a clone and drop the ones that land somewhere seen.
  Costs a console clone per candidate per decision, which that binary measures
  as affordable for a diagnostic and would need timing for a trainer.
* **end the packing on a revisit.** Free to detect, and makes a cycle cost the
  rest of the budget rather than 0.02 - which is the only quantity in this
  system large enough to matter and small enough not to swamp the return.

Neither is a reward. That is the point: A.1 measured an environment in which
doing nothing is never punished, and six attempts to price it have now
established that pricing is the wrong lever.

---

# O — Ending the packing helps when it is played and hurts when it is trained

Read off `9abc1f1`. `qrow-r37-stop-control.log` and `qrow-r38-stop-abstract.log`,
3,000 episodes each at pair 322 with `QROW_STOP_REVISITS=0`, against r31 and r32.

## O.1 Trained under the rule, it is worse

```
                        floor mean          the rule       items
  control, plain   3.024 +/- 0.169                 -        1.79
  control, ending  2.409 +/- 0.119  -0.616 +/- 0.207        0.82
  abstract, plain  3.368 +/- 0.097                 -        1.15
  abstract, ending 3.254 +/- 0.073  -0.114 +/- 0.122        0.96
```

Three standard errors down for the plain arm and flat for the other, and **items
held fell in both** - 1.79 to 0.82. The mechanism is not subtle: an item takes
several placements, and if any of them recreates a board this packing has seen,
the packing is over. Early in training almost every press is random, so cycles
arrive at once: the replay buffer holds **480 transitions at episode 25 against
about 3,000** without the rule. It is cut off before it can build anything, and
it never sees what a long productive packing looks like.

That is a curriculum fault rather than an objection to the rule. The rule is
hardest exactly when the policy is least able to avoid it.

## O.2 Played under the rule, it is better - for nets that did not train on it

Thirty runs a net, the same nets played both ways:

```
  net                              played off   played on
  the written control                   1.172       1.172
  w39-control   (trained without)       0.772       1.124
  w39-abstract  (trained without)       0.896       0.910
  stop-control  (trained with)          0.943       0.873
  stop-abstract (trained with)          1.010       1.299
```

A net that never trained under it gains most: `w39-control` goes 0.772 to 1.124,
which is nearly the written control's 1.172, for nothing. Stopping a policy
before it churns a board it has already built is worth more than anything six
reward changes managed.

## O.3 And the highest number in this table is the banding confound

`stop-abstract` played under the rule reads **1.299**, above the written
control's 1.172 and the best any trained packer has scored. Banded by the rung
it was measured at, it is not:

```
  packer                   r1+    r2+    r3+    r4+    r6+    r9+
  the written control     0.77   0.34   0.86   3.04   2.33   1.81
  w39-control             0.37   0.73   1.26   2.56   4.00   1.79
  stop-abstract           0.96   0.98   1.75   2.46   1.00   2.12
```

It builds **better shallow boards and worse deep ones**, and the aggregate
favours it because it spends more of its life at rungs one to three where it is
strong. K's own warning about this confound is why the banded table is printed
beside the mean, and it is the second time in this document that a headline has
needed it.

## O.4 What to carry forward

* **The rule belongs at play time, not in training.** The play-time gain is
  free and measured on four nets; the training loss is three standard errors on
  the arm that matters.
* If it is to be trained under, it wants a **schedule**: tolerate many revisits
  while epsilon is high and tighten as it falls, so the packing is only cut off
  once the policy has some chance of avoiding it. That is one number in
  `pack_with_ending` and it has not been tried.
* Nothing here changes A. Depth still moves by tenths whatever is done to the
  packer, and `reach` still separates packers that depth cannot.
