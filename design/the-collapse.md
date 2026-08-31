# The Collapse — the plan

Written against `c2241cd`. The brief is `design/HANDOFF-the-collapse.md` and
this is the plan executed from it; `analysis/the-collapse.md` is where the
measurements go, one block a milestone, each headed by the commit it was read
off.

The question, in one line: **the quartermaster trained on the row reaches rung
11 with exploration already at 0.07 and then degrades over thirteen hundred
episodes to rung 2, and the Q spread rises through the turn.** That last clause
is what makes it a new animal. Every fault this mission has already found and
fixed presents as a *flat* network, and this one had learned something and then
lost it.

## The rule

**No training run is justified until M3 returns a number.** Four separate
faults in this mission were each read as "it needs more episodes" and each cost
hours. M0 to M3 are minutes to hours of writing and seconds of running. M5 is
the only expensive thing in this document and it happens once, with a
hypothesis attached to it and an arm to compare against.

## What the brief could not know

Its §4 says to run `--bin qmind` and `--bin qwhy` first. Neither can see
anything, and nothing says so:

```
================ rogue quartermaster    … did not load
================ grinder quartermaster  … did not load
================ rogue pathfinder       … did not load
================ grinder pathfinder     … did not load
```

`QNet::parse` gates on `w1.len() == feature::PAIR * hidden`. `cf91f65` took
`BOARD` from 30 to 270, so this build's pair is **315** and every file in
`analysis/nets/` is **70** wide. `load` returns `Option` and every caller reads
`None` as "no weights", silently - so `Packer::Learned(None)` is the floor
wearing a trained net's name, and every pathfinder number in
`analysis/the-two-trades.md` R6 is unreproducible at this tip until its nets
are re-saved or the parse is made width-aware.

**A saved net is keyed to the feature width of the day it was saved.** That is
a trap and it goes in `CLAUDE.md` when M0 lands.

Second, smaller, and in the way of the same milestone: `runs/quartermaster_row.txt`
and `runs/quartermaster_row_last.txt` were both written at 02:35:57, one minute
before `9e9198b`, and `analysis/nets/qrow-r12.log` ends with the **pre**-`9e9198b`
single-file message. The two files differ. Nothing yet shows that one holds the
rung-11 policy and the other the collapse, and M1 to M3 are a comparison
between them.

---

## What M0 found, and what it does to the rest of this document

**There is no collapse.** `qrow`'s `deepest in block` column is a *maximum* over
a hundred episodes, and depth in this game is heavy-tailed and largely a
property of the seed - the written control's hundred runs are mean 5.3 and max
47. Held completely still at the same 5% exploration floor, a net that plays at
mean rung 2.1 prints this:

```
    block  deepest     mean
        0        7     2.27
      100        7     2.21
      200        3     2.07
      300        7     2.18
      400       11     2.22
      500        9     2.10
      600        3     2.07
      700       11     2.12
```

Eleven, nine, seven, three, eleven - out of a file, from a policy that cannot
change. That is the brief's whole observation, and the mean underneath it never
moves. `analysis/the-collapse.md` M0.3 sets it beside the eleven blocks the
brief was written about; they are the same statistic in the same range.

So the rise to rung 11 was never a policy reaching rung 11, and the fall to 2
was never a policy forgetting. The rising Q spread is a value function drifting
under a behaviour that never changed.

**M1 to M5 below are therefore a triage of something that does not exist**, and
they are left standing only as a record of what was planned. The question that
survives is the mission's older one, unchanged since Q3 and unmoved by the row:
the quartermaster plays at rung 2.0 against the written control's 6.0. What the
next milestone should be is the owner's call; what it should *not* be is
Double-DQN.

M6 was the exception and it is **done**: `qrow` prints a mean and chooses its
best weights on it, because a running maximum is exactly the fault trap 54
records `qroad` having had and nobody gave `qrow` the same column. The greedy
evaluation half was measured and refused - the exploration floor is worth +0.07
of a rung on identical seeds, which is smaller than the block mean's own noise.
`analysis/the-collapse.md` M0.4 has both numbers.

---

## M0 — The instruments can see the nets

The prerequisite the brief's §4 does not know it has.

* `QNet` records the width it was **read at** rather than asserting the width
  this build happens to compile with. `parse` derives `hidden` from `b1` and
  the pair width from `w1`, and validates the file against itself.
* A load that fails says why: `w1 is 70 wide, this build's pair is 315`, not
  `did not load`.
* `q_pair` pads a road pair to the **net's** width rather than to
  `feature::PAIR`, so a 70-wide pathfinder net and a 315-wide one both work.
* `qmind` takes a path, and its default list gains
  `runs/quartermaster_row.txt` and `runs/quartermaster_row_last.txt` - the two
  nets this whole document is about, which it never listed.
* **Provenance.** Play both row nets greedily through `row::run` over a fixed
  seed set and print the deepest rung each reaches. If they are not about 11
  and about 2-3, the labels are wrong and the comparison M1 and M2 rest on is
  void; regenerate before going on.
* A ratchet in `crates/trades/tests/` - not `lab`, which nothing runs by habit
  (trap 46) - asserting every net in `analysis/nets/` parses, and naming each
  file's width in the failure.

**Deliverables.** A width-aware `QNet`; a diagnostic that says why it is empty;
`qmind` pointed at the row nets; the ratchet; one line of provenance saying
which file is the rung-11 policy; the trap written into `CLAUDE.md`.

## M1 — What the collapsed policy presses

The brief's §4.3, and the cheapest thing here. Its own note is that the
single cheapest diagnostic in this mission was a key histogram that no
instrument produced and a person reading a proof found in a minute.

* A packing counterpart to `qwhy`: takes a packing net, plays real `row::run`
  episodes, and prints per-verb counts - place, buy, sell, clear, done - with
  items assembled, presses to the first item, and the per-state spread.
* Run against both row nets, with the written control as the reference column.

**Deliverable.** A three-column histogram, best against collapsed against
control. **What it settles:** if the collapsed policy has fallen onto one verb
this is trap 44 and trap 52 again, and the overestimation hypothesis is dropped
without a run being spent on it.

## M2 — Did the weights blow up

The brief's §4.2, and free once M0 lands.

* `qmind` on both row nets. The biases are the honest column: they start at
  exactly zero, so anything there is learning and nothing else.

**Deliverable.** The layer table, best against collapsed, and a one-line
verdict. A large `w3` or `b3` in the collapsed net is diverging values; similar
weights with different behaviour is something else, and points at M3.

## M3 — Are the values overestimates

The brief's §4.1: the measurement the repo does not have, and the one
everything after it branches on.

* **First, single-source the reward.** `qrow.rs` assembles the per-press reward
  inline - the assembly bonus on a new high, `worth` on the last press. Lift it
  into `row::rewards(&[Pressed], &Ran) -> Vec<f32>` so the trainer and the
  diagnostic cannot drift. A predicate with two readers is two predicates that
  happen to agree, and this repo has learned that twice in units and once in
  meaning.
* **Then the calibration.** Play episodes under the collapsed net; at each
  decision record `max_a Q(s,a)`; afterwards compute the discounted return
  actually realised from that decision out of `row::rewards` at the same
  `gamma`. Print mean Q, mean return and the bias between them, banded by rung
  and by depth into the episode.
* Run it against the rung-11 net too. A bias small at the peak and large at the
  collapse is the answer. A bias large at both is not.

**Deliverables.** `row::rewards`, extracted and used by `qrow`; the calibration
binary; a table for both nets. This is the milestone the document exists for.

## M4 — The verdict, and only then a change

One arm, chosen by M1 to M3, written down beside the measurement that chose it.

* **Overestimation confirmed** - Double-DQN: select with the online net,
  evaluate with the frozen one. A few lines in the update loop, behind
  `QROW_DOUBLE=0|1` so one binary runs both arms of M5.
* **Weights diverged** - the target refresh cadence, which is 1,200 gradient
  steps between refreshes today, and Polyak averaging; `QROW_TARGET_EVERY`.
* **One verb** - the reward and the action space, per trap 44: charge for what
  the board does rather than for what the verb is called.
* **None of those** - the brief's §4.4. `BOOTSTRAP_KEEP` to the full menu, and
  an env var rather than a `const` so testing it is not a recompile. About
  eleven times the update cost, so it is last.

**Deliverable.** The chosen change behind a flag that defaults to today's
behaviour, so the A/B is one binary and one seed.

## M5 — The A/B, run once

The only expensive step. r12 measured 1.83 s an episode, so 1,200 episodes is
about thirty-seven minutes an arm.

* Two 1,200-episode **Rogue** arms, same `ROW_SEED`, control and treatment.
  Grinder is designed to always be possible and costs about fifteen times as
  much an episode; it is not spent here.
* The written control's `mean rung 6.0, best 13` line is the harness check. If
  a change moves that number, the change is about the harness and not about the
  learning.
* The read-out is the peak **and whether it holds**, not the final block.

**Deliverables.** The episode, epsilon, deepest and spread table for both arms;
M3's calibration bias re-measured on the treated net; nets saved into
`analysis/nets/` at a width the M0 ratchet records.

## M6 — So that it cannot happen silently again

* **A held-out evaluation block.** Every so many episodes, a few greedy
  episodes at epsilon zero, printed as their own column, and the best weights
  kept on *that* rather than on the behaviour block's depth. The brief kept the
  block because an evaluation is more runs; this makes that cost explicit and
  measured rather than assumed.
* **A bias column in `qrow`'s printout.** Trap 54 one turn on: `qpack` prints a
  spread so it can tell "learned nothing" from "learned the wrong thing", and
  this run needs a column that tells learning from over-valuing. M3 is what
  makes it cheap enough to print.
* The trap entries: the stale net width, and whatever M3 turns out to say about
  a rising spread with falling depth.

## M7 — The pair, re-measured

The brief's §7, and answerable only after M0, because the packer `qcross` loads
is `None` today.

* `qcross` re-run with nets that load: written pilot, learned road, learned
  packer, and the pair.

**Deliverable.** The cross table at this tip, and a plain statement of whether
fixing the collapse moved the pair off 1.0 - including "it did not", if it did
not.

---

## Ordering, and where it can end early

M0 blocks everything. M1, M2 and M3 are independent of one another and all
three block M4. M5 follows M4; M6 and M7 follow M5.

M1 and M2 are the two most likely to end this early and cheaply. If either says
"one verb" or "`b3` is enormous", M3 becomes confirmation rather than
discovery.

## Kept for later: batches on one seed

The owner's proposal, recorded because it is a real technique and because the
reason it is *not* next is a measurement rather than an opinion.

**The proposal.** Batch the episodes, take the deepest run of a batch, and start
the next batch from it, so the value function is led back to the best thing the
policy has done.

That is self-imitation learning with a little Go-Explore in it, and both work on
problems shaped like this one. Two things stop the form as first stated:

* **Best-of-batch is a maximum statistic, and M0.3 measured this loop's maxima
  as mostly seed.** A fixed net that cannot learn prints block maxima of 3 to
  13. Selecting on that selects on noise, which is the fault that produced the
  phantom collapse and saved a rung-2 net labelled "rung 11".
* **A tape does not transfer across seeds.** A verb is
  `Place { piece: PieceId, .. }` or `Buy { shelf }`, bound to one run's registry
  and shop - which is why a proof is `(seed, mode, difficulty, [verb])` and why
  `proof::write` refuses one that will not replay. The best run's actions
  applied to another seed are not a worse plan, they are not a plan.

**The form that works is a batch on one seed.** Then depth differences inside a
batch are policy rather than luck, the best tape replays exactly, and "start
from the best run and carry on from there" is well defined. The open question
becomes generalisation: seeds would have to be cycled across batches, and a
policy that is excellent on the seeds it batched is the thing to check for.

**Why it is not next.** M2 below. The value function is not being misled, it is
barely being fitted: a target range of `[-1.96, +3.04]` against a Huber knee of
**120**, so nothing ever reaches the knee and every gradient in the run is
`2|d|/120`. Better data through an optimiser that cannot use it is a null result
nobody can interpret, and this mission has already lost three milestones to
exactly that.

## What is not in this document

Training longer. Grinder. Reading the spread as health - the working Grinder
pathfinder reaches rung 17.5 with a spread of 0.008, smaller than the broken
one's 0.083, and spread answers "has this learned anything at all" rather than
"is it any good".
