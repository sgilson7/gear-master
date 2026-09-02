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
