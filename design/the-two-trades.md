# THE TWO TRADES — a quartermaster and a pathfinder, both learned
## Execution spec

Written against `a8f8d19` (2026-08-29). Successor to
`design/the-apprentice.md`, whose measurements are the baseline every gate
below is set against. Like every document in `design/`: **code follows this
document; when they disagree, this is the bug report.**

**What this is for.** THE APPRENTICE proved the game is walkable and produced a
pilot that beats Francis on 40.6% of seeds. Every line of that pilot's policy
is hand-written, and the last two milestones were spent hand-coding things a
learner should have found - which choice opens a dungeon, that a rumour is not
for selling. This mission replaces the policy with two Q-learning agents that
learn it, and makes the second one **goal-conditioned**, so that "reach this
door" is a thing you ask for rather than a rule somebody writes.

---

# 0. Why two agents, and why this split

The pilot's cost is one number: **a run is 195,000 presses and 77% of them are
trial-seat-and-undo.** No temporal-difference method can assign credit across
that. Splitting the problem is not a preference, it is what makes the horizon
learnable at all.

The split is **by screen**, which is how a person experiences it:

| | **The quartermaster** | **The pathfinder** |
|---|---|---|
| plays | the loadout and the shop | the road |
| owns | buy, sell, barter, reroll, pin, place, rotate, lock, unequip, clear, grow, undo, **done** | answer, town, walk-on, throw points, leave, walk, out, drink, pedestal, crush, fight, brawl, **pack** |
| episode | one shopping-and-packing session | one run |
| horizon | 30-60 decisions | 200-500 decisions |
| reward | did this board win the fight in front of it | rungs cleared, and the goal |
| asked | "make me a board that beats *that*" | "get me to *there*" |

`pack` is the pathfinder's one macro-action: it hands control to the
quartermaster, which plays its own episode and hands back a board. That is the
options framing, and it is what turns 195,000 steps into 400.

## 0.1 What the hand-written objective cannot see, measured

The reason to learn the packer rather than tune it, in one table
(`piece.rs`, counted at `a8f8d19`):

| | pieces | can `Sense::worth` see it? |
|---|---:|---|
| produce **mana** | 118 | yes - `Figures::flow` |
| produce **faith** | 33 | **no field exists** |
| produce **nature** | 29 | **no field exists** |
| produce **rage** | 26 | **no field exists** |
| **consume** a pool to act | 75 | **no** - it is a trigger, and `Figures` reads `stats.*` only |

So a tray of nature producers and a spell that spends nature for damage scores
**zero on both halves**: the producers have no field, and the spell's damage is
conditional and therefore not in `stats.physical_damage`. The current packer
reaches 48/50 by maximising `items × 400` - density - which
`rl-research.md` §3 already says is *"nearly irrelevant"*. The one board it
cannot recover is the perfect run's, 50/50 by hand and 46-48 by search, and
that is the board most likely to be built around a synergy.

**A learner rewarded by the fight sees all of it, because the chain shows up
as damage in the log.** That is the whole argument.

---

# 1. The numbers to beat

Every gate is set against a measurement THE APPRENTICE already took.

| | baseline, at `a8f8d19` |
|---|---|
| repack, owner's 75 pieces | **48/50**, all board-decided |
| repack, friend's 76 | **49/50** |
| repack, perfect's 62 | 46-48/50 (the human: **50/50**) |
| SCR(R10) / R15 / R25, 64 seeds | 100% / 81.2% / 79.7% |
| **SCR(FRANCIS)** | **40.6%** |
| median rung | 47 |
| door coverage, offered / branched | 70% / 58% |
| dungeons entered | **1 of 7** |
| wall-clock a run | 32 s |
| presses a run | 195,273 |

---

# 2. Where it lives

Two new crates, and the A1 boundary unmoved.

```
crates/console    the player's surface        (unchanged)
crates/oracle     privileged scoring          (unchanged)
crates/agent      the hand-written pilot      (unchanged - it is the control)
crates/lab        harnesses and training      (gains the trainers)

crates/trades     gearmaster-trades
                  The two agents. Depends on `gearmaster-console` and nothing
                  else from this workspace: the same guarantee the pilot has,
                  for the same reason. Q networks are read as plain weights.
```

**Training is privileged; acting is not.** The quartermaster's reward is a
fight, which the player cannot run - so the *trainer* lives in `lab` and holds
the oracle, and the *agent* reads only the `View`. This is the asymmetric
actor-critic, and the crate graph makes it checkable rather than promised.
`trades/tests/boundary.rs` asserts it exactly as `agent`'s does.

---

# 3. The two Markov decision processes, written down

## 3.1 The quartermaster

**State.** From the `View` alone:

* five grids as occupancy + item identity + assembled/locked;
* the tray, and the six shelves, each piece described by **what it produces
  and what it consumes** (§4);
* gold, reroll cost, rows owed;
* the coming creature - the portrait card draws its stats, its items and its
  whole board, so all of it is fair (`A1.4`);
* a **conditioning vector**: theme, and how much a fight's speed is worth
  against surviving it.

**Actions.** The shop and the board, masked to `Console::menu()`, factored so
the space is small at any step: first *what kind of move*, then *which piece*
(≤ 12 or ≤ 6), then *where* (≤ 384 for a placement). Plus **`done`**.

**Reward.** One fight against the target creature, at the end:

* win: `+1 + margin`, where margin rewards a board-decided clear over a
  clock one and a fast clear over a slow one;
* loss: `-1 + closeness`, so a board that nearly won ranks above one that did
  nothing - the gradient A6 found missing;
* `-0.002` a press, so it stops rather than dithers;
* and gold left unspent is neither punished nor rewarded. A6 measured a run
  hoarding 15,000 gold; the fight is what says whether that was wrong.

**Terminal.** `done`, or the press budget.

## 3.2 The pathfinder

**State.** The road as the screen draws it: rung, gold, lives, mode, the road
stack's head, the standing question with its choices *and their requirement
text*, the town's doors, the dungeon graph the atlas draws, the county tile and
its neighbours, classes held, quest items and rumours in the tray, and the
board's summary (what the quartermaster just built).

Plus **the goal**, one-hot over the things a validity solver can be asked for:
a door id, a dungeon, a town, a chain, a rung, or *nothing*, which means
"climb".

**Actions.** Everything on the road, masked to the menu, plus **`pack`**.

**Reward.**

* `+1` a rung cleared, `+3` a rung never reached before in this episode;
* `-1` a fight lost, `-5` a life lost in Rogue;
* **`+50` on reaching the goal**, and the episode ends there;
* `-0.01` a step.

The large goal reward is what makes this a validity solver rather than a
climber: reaching a named door outweighs any amount of laddering.

---

# 4. The representation, which comes before the learner

**This is the milestone that decides whether any of it works**, and it is
independent of Q-learning. If the state cannot express *"this piece makes
nature and that one spends it"*, no amount of training discovers the build; it
learns the current blindness faster.

Every piece is described by, at minimum:

* **produces**: the eight `Resource` pools, from `Stats` and from any
  unconditional `OnActivate(Gain)`, per second where a cadence exists;
* **consumes**: the pools its triggers spend, and what it does if the spend
  works;
* **conditionality**: how much of its output is behind a spend, a neighbour, an
  alignment, or a battle-start;
* the four `When` groups the card draws (`Stats::parts_when`), which is the
  engine's own classification of rate against quantity;
* footprint, kind, slot, price, rarity.

And every **board** by the same, summed, plus the **match**: for each pool, how
much is produced against how much is consumed. That single derived number is
what a build is.

None of it is privileged - the card draws all of it, which is why the
quartermaster may read it.

---

# 5. How the work lands

**A branch, a commit a milestone, and one merge at the end.** There are no
deploy points in this mission until the last one, because nothing before it
changes the game - the agents are tooling and ship nothing.

* The work lives on **`the-two-trades`**, branched off `the-apprentice`, which
  it depends on for the console, the oracle and the control it is measured
  against.
* **Every milestone is a commit**, pushed when it is green, with its numbers in
  the message the way this repo's commits carry them. A milestone that cannot
  state its gate in its own commit message is a milestone that has not
  finished.
* **One merge, at Q9**, and it carries the wasm publish with it - because Q9 is
  the only milestone that changes the game, by splicing boards into
  `combat.rs`, and that diff is the owner's to read before it ships.
* A milestone that fails its gate is still committed, with the failure in the
  message. `design/` records what happened; a branch that only holds successes
  is a branch that has lost the findings.

---

# 6. Milestones

Ten. Each ends green, with its numbers in `analysis/the-two-trades.md` beside
the commit they were read off.

| | Milestone | Gate |
|---|---|---|
| Q0 | The ground, and the two menus | the verb set partitions exactly, nothing lost or shared |
| Q1 | **The representation** | a probe shows the features carry pool-matching; the census prints |
| Q2 | Two environments | an episode of each runs; horizons inside 60 and 600 |
| Q3 | The quartermaster learns | beats 48/50 and 49/50, or it is recorded |
| Q4 | **Pools, proven** | a measured difference in what it builds, or a written finding of none |
| Q5 | The pathfinder learns | SCR(FRANCIS) ≥ 40.6%, or it is recorded |
| Q6 | **The validity solver** | coverage above 70%/58%; every gap classified |
| Q7 | Generations | each generation ≥ the last on held-out seeds |
| Q8 | Themes | held-out themes packed better than unconditioned |
| Q9 | The harvest, and the record | five county creatures packed; the suite green |

## Q0 — The ground, and the two menus

No learning. Partition `Verb` into the quartermaster's and the pathfinder's,
exhaustively and disjointly, and prove it: **every verb belongs to exactly one
agent, and the union is the whole vocabulary.** That is A1's parity lint in a
new shape and it is the thing that stops an action falling between the two.

Measure the decision counts a hand-written run actually makes at each level -
the horizon estimates in §3 are estimates until this milestone.

**Deliverable:** `analysis/the-two-trades.md` with the §1 table re-taken, the
partition, and the measured horizons.
**Gate:** the partition lint passes; horizons are inside the §3 figures or the
figures move here rather than later.

## Q1 — The representation

Feature encoders for both agents, and the diagnostic that says they work.

**Deliverable:** the encoders; a printer that lists every piece with what it
produces and consumes, and every reference board with its pool matches; and a
**probe** - a linear model over the board features predicting whether a board's
damage came from a conditional. If a linear probe can do it, the information is
there.

**Gate:** the probe beats chance by a margin written down. If it does not, the
features are wrong and nothing after this matters.

## Q2 — Two environments

`reset`, `step`, `reward`, `legal`, for both. The quartermaster's reset samples
a situation: a rung, a tray, a purse, a shop, a target creature. Sampling uses
`skip_to`, which is **privileged and training-only** - the pathfinder can never
call it because it is not a verb.

**Deliverable:** both environments, a random-policy smoke test, and the
throughput: episodes a second on eight cores.
**Gate:** a random quartermaster episode is under 60 decisions; a random
pathfinder episode under 600. Both replay identically from a seed.

## Q3 — The quartermaster learns

DQN over the factored action, masked. Replay buffer, target network,
ε-greedy annealed, Huber loss, Double-DQN. Trained against sampled situations
across the whole ladder, so it is not a rung-10 specialist.

**Deliverable:** a checkpoint, the repack benchmark, and the training curve.
**Gate:** **≥ 48/50 from the owner's tray and ≥ 49/50 from the friend's** -
the hand-written packer's own numbers. Missing it is a finding, not a failure,
and the hand-written packer stays the default until it is beaten.
**Commit:** `Q3: the quartermaster packs`, with the repack table in the
message. The learned packer is selectable by the existing pilot with one
environment variable, so every A-series measurement can be re-run against it.

## Q4 — Pools, proven

The claim this mission was started for, made falsifiable.

**Deliverable:** two measurements.

1. **The planted tray.** A tray of nature producers and one spell that spends
   nature for damage. Does the learned packer assemble it? Does the
   hand-written one? Repeat for each pool.
2. **The census.** Over a hundred boards from each packer, how often is a
   pool's production matched to a consumer on the same board, and how much of
   the damage dealt arrives through a conditional?

**Gate:** a measured difference, written down, either way. *"The learned packer
does not find builds either"* is a publishable result and would redirect the
rest of the mission.

## Q5 — The pathfinder learns

Goal-conditioned DQN over the road, calling the **frozen** Q3 checkpoint for
`pack`. Frozen because an agent learning against a moving environment is an
agent whose failures cannot be attributed.

Curriculum: goals from near to far - a rung, then a town, then a door, then a
dungeon, then Francis.

**Deliverable:** a checkpoint, SCR over the 64 training seeds in both modes,
and the failure histogram.
**Gate:** SCR(FRANCIS) **≥ 40.6%** on the training half, or a written finding.
**Commit:** `Q5: the pathfinder walks`, with the SCR table in the message.
`make play GOAL=...` runs the pair to a target.

## Q6 — The validity solver

The product. Give it a door and it plans to it.

**Deliverable:** `analysis/coverage.md` regenerated by the goal-seeking pair,
every door tried as an **explicit goal** rather than met by accident, and every
gap classified in A5's five classes.

**Gate:** door coverage above **70% offered and 58% branched**, and **every
one of the seven dungeons entered at least once** - which is the specific
failure this mission was asked to fix.
**Commit:** `Q6: a solver that is given somewhere to go`, with the coverage
table in the message. `make validity` regenerates the ledger.

## Q7 — Generations

Unfreeze. Retrain the quartermaster against the trays the pathfinder actually
produces, then the pathfinder against the new quartermaster. Two or three
generations, each one measured against the last on the **held-out** 64 seeds.

**Gate:** each generation is at least the last, or the mission stops at the
best one and says so.

## Q8 — Themes

The conditioning vector earns its place: train on eight `MonsterTheme`s, hold
two out, and measure whether the held-out two are packed better than by an
unconditioned packer. This is what makes the quartermaster usable as the enemy
packer.

**Gate:** held-out themes beat unconditioned, with the fidelity meter (A2) as
the measure and the acceptance gate as the constraint.

## Q9 — The harvest, and the record

Creature boards from trained play: the five county creatures that still wear
borrowed boards, then whatever else the owner names. A9's harvest path.

**Gate:** `cargo test -p gearmaster-engine` green with no new dependency;
`analysis/the-two-trades.md` finished; `CLAUDE.md` §6 rewritten.
**Merge:** `the-two-trades` into `main`, and the wasm publish with it. The
only point in this mission where the game changes, and the only one that needs
a diff the owner has read.

---

# 7. Training the two in parallel

They are **not** independent, and pretending otherwise is the fastest way to
an unattributable failure. What is true is weaker and still useful: **their
rewards do not share a term.** The quartermaster is scored by a fight; the
pathfinder by rungs and a goal.

So: **parallel with generations.**

* Both train at once, in separate processes, on separate cores.
* The pathfinder always trains against a **frozen, versioned** quartermaster
  checkpoint - never the one being written.
* At a generation boundary, the pathfinder's checkpoint is swapped in for the
  quartermaster's situation-sampler, and the quartermaster's for the
  pathfinder's `pack`.
* Every checkpoint carries the version it trained against. A result that
  cannot name both versions does not replay.

This is AlphaZero's generation loop with two players who are not opponents.
It costs a little optimality against joint training and buys the only thing
that matters early: when a number moves, you know which agent moved it.

---

# 8. Compute, on this machine

M2 Max, 8 performance cores, 32 GB, Metal. Measured at A0/A8: a fight is
0.24 ms, the surrogate 584 ns, and burn trains a small MLP at ~6,000
epochs a minute on `Autodiff<NdArray>`.

| | estimate |
|---|---|
| quartermaster episode | 30-60 decisions, one fight - **~2 ms** |
| 10⁶ quartermaster steps | ~20k episodes ≈ **40 s of environment**, plus training |
| pathfinder episode | 200-500 decisions, ~50 `pack` calls at ~2 ms - **~3 s** |
| 10⁶ pathfinder steps | ~2,500 episodes ≈ **2 hours on 8 cores** |
| a generation | one of each, plus evaluation - **half a day** |
| the whole mission | **three to five overnights of training**, plus the building |

The pathfinder is the expensive one and `pack` is why. If Q3's packer is fast
- a forward pass rather than a search - the estimate holds; if it needs
search at inference, this doubles and Q7 is where it hurts.

---

# 9. What would make me stop

1. **Q1's probe fails.** The features cannot express a build; fix them before
   anything else.
2. **Q3 cannot reach 48/50.** A learned packer worse than a greedy one over a
   fixed tray is a learner that has not learned; the fault is the reward or
   the action factoring, not the amount of training.
3. **Q4 finds no difference.** The learned packer builds what the heuristic
   builds. That retires the premise of this mission and the honest response is
   to write it down and stop, keeping the hand-written packer.
4. **Q5 is far below 40.6%.** Expected early; a problem if it persists past a
   generation. The hand-written pilot is a strong control and the mission's
   value does not depend on beating it - **the goal-conditioning is the
   product**, and a goal-seeker at 30% that reaches every door is worth more
   than a climber at 41% that reaches one dungeon.
5. **An engine rule needs to change.** Stop, write the proposal, ask.

---

# 10. Decisions for the owner

Each has a default so work can start.

* **T1 - Who owns the shop.** Taken as specified: **the quartermaster**. It is
  the right call - buying and packing are one decision, and A6 proved it by
  showing the shop was the whole of the middle game.
* **T2 - Does the pathfinder ever place a piece?** Default **no**. One agent
  owns the board. The cost is that it cannot make a tiny adjustment without a
  whole packing episode.
* **T3 - The goal vocabulary.** Default: door id, dungeon id, town, chain,
  rung, or none. Adding "hold this item" or "reach this county tile" later is
  a wider one-hot and no new machinery.
* **T4 - Q-learning specifically.** Taken as specified. Recorded for honesty:
  the environment is deterministic and cheap, so **search with a learned value
  is likely to beat model-free Q in the end** (`rl-research.md` §2). Q is the
  simpler thing that works and the right first implementation; if Q7 plateaus,
  the escalation is MCTS over the pathfinder's actions using the same value
  head, and no work is wasted.
* **T5 - burn's backend.** Default `Autodiff<NdArray>`, with `wgpu`/Metal
  measured once at Q3 on the real net. These are small networks and CPU may
  win; the measurement decides and goes in the record.
* **T6 - Checkpoints in git.** Default **no**, per the artifact policy. What
  is committed is the numbers, the ledger, and the boards.
