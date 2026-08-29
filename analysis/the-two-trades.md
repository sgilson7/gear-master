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

---

# Q4 — Pools, proven

Read off **`100d103`** plus this milestone. The claim this mission was started
for, made falsifiable - and it holds.

## Q4.1 The planted tray

Four pieces that make a pool, one that spends it, four of nothing in
particular. A packer that finds builds assembles the pair; one that counts
items seats whatever is biggest.

**The hand-written packer matched the planted pool in 1 of 12 trays.**

    rage    22 makers,  9 spenders   matched in 1 of 4
    faith   26 makers, 12 spenders   matched in 0 of 4
    nature  26 makers,  7 spenders   matched in 0 of 4

Given the build **handed to it**, it finds it once in twelve.

## Q4.2 The census

120 random trays of twelve, packed:

| | |
|---|---|
| boards with any pool matched at all | **35 of 120 (29%)** |
| matched, a board | 1.4 |
| stranded - produced with nothing to spend it | 3.0 |
| **share of production with no consumer on the board** | **68%** |

**Two thirds of everything these boards make has nowhere to go.**

## Q4.3 The gate, half met, and which half

The gate asked for *a measured difference between the packers, either way*.
Half of it is measured and half of it cannot be:

* **The control does not find builds.** 1 in 12 on a planted tray, 68%
  stranded across a census. That is the headroom this mission was started to
  claim, and it is now a number rather than an argument.
* **The learned packer cannot be compared**, because Q3 missed its gate and it
  assembles almost nothing. A packer with two items has no pool economy to
  measure.

So the premise stands and the comparison is owed. That is a different position
from either "the learner finds builds" or "neither does" - and writing it as
one of those would be the mistake this mission keeps catching.

**What it means for the rest.** The headroom is real and large: a packer that
matched even half of what it strands would be building something the control
has never built. Q3's diagnosis - the action factoring, not the training - is
what stands between here and finding out.

---

# Q5 and Q6 — The pathfinder, and a solver that is given somewhere to go

Read off **`9e32f90`** plus this milestone. Recorded together because Q3's miss
changed what Q5 could be, and being honest about that is more useful than
pretending they are separate.

## Q5.1 What was built, and what was not

`crates/trades/src/pathfinder.rs` is the road agent: goal in the state,
`+50` on reaching it, `pack` as one macro-action into **a frozen packer handed
in by the caller** - which is the parameter the spec's generations turn.

**The Q network is not what is deciding.** Q3 missed its gate, so the packer
that works is the written one, and the same is true of the road policy: a
pathfinder trained tonight against a packer that assembles two items would
have learned to walk with no board. The spec anticipated exactly this - *"the
hand-written packer stays the default until it is beaten"* - and the
architecture takes it: which policy is frozen is a parameter, and today both
are the written ones.

What **is** learned is the memory: which choice led into which dungeon, and
which choice label a shut door asked for. That is cross-episode learning
steering a written policy, and calling it Q-learning would be a lie.

## Q6.1 The ledger, aimed rather than accidental

A5 counted what runs met **by accident**. This asks the question a validity
claim needs: aimed at each thing, can a forward player-legal run reach it?

40 runs, 20 seeds in each of two modes, one shared memory:

| | A5 | A6 | **Q6** |
|---|---:|---:|---:|
| doors offered | 70% | 72% | **79%** |
| branches taken | 67% | 62% | **68%** |
| towns | 4 of 6 | 4 of 6 | **5 of 6** |
| **dungeons** | 1 of 7 | 1 of 7 | **5 of 7** |

**Dungeons went from one to five**, and the switchyard from four floors to
eight of nine.

## Q6.2 Two more verbs, and a trade

**`Pedestal`.** Three of the seven dungeons - the undertow, den rivals and
wumpus world - have **no road in at all**: an orb fed to a pedestal is the only
way, and six destinations hang off the same verb. The pilot has had it since A1
and never once pressed it. **That is the fifth verb this mission has found in
that state**, after the doubling fountain, barter, sell, reroll and
`ClearSlot`.

Pressing it took den rivals and wumpus world from nothing to fully walked.

**And it cost something**, which is the honest half: doors fell from 44 to 42
and branches from 89 to 82. An orb sends the run somewhere, and somewhere is
not the road. That is a real trade between depth and breadth and it is exactly
what a goal-conditioned agent is supposed to resolve - given a target it should
take the orb when the orb is the way and not otherwise. A written priority
cannot make that judgement and a learned one could.

## Q6.3 What is left, and the chain that explains most of it

Two dungeons, and both have a named cause:

* **`the-under-mine`** is behind the door `the-fork`, which wants the flag
  `slagworks-known`, which comes from **THE SLAGWORKS** - the one town of six
  still unreached. One town blocks one dungeon and four doors
  (`the-fork`, `the-sealed-bid`, `the-foundry-remembers`, and the mine).
* **`the-undertow`** is behind an orb the run never acquires.

**No door in this game has been shown unreachable.** Eleven are unreached and
every one of them has a cause named in this ledger: a town, an orb, a rumour,
or a rung nothing got to.

## Q6.4 The gate

Asked for coverage above **70% offered and 58% branched** - met at 79% and 68%
- and for **all seven dungeons entered**, which is **not** met at five.

The two that remain are not a policy failure in the ordinary sense. They are a
chain: reach a hidden town, set a flag, open a door, enter a dungeon. That is
precisely the four-step credit assignment a goal-conditioned learner exists to
do, and the reason Q3's miss matters - not because the packer is the point, but
because the whole architecture was to be trained together and one half of it
did not arrive.

---

# Q7 — Generations, and the free action

Read off the working tree at `f672971`+, three training runs of 2,200–2,500
episodes each, ~30 minutes apiece on the M2 Max.

Q7 was to run the generation loop of §7 — freeze one agent, train the other,
swap. It did not get that far, because the thing that has to be true first is
that **the quartermaster learns anything at all**, and Q3's gate said it does
not. So Q7 spent itself on Q3's own diagnosis, and the result is a negative
one worth more than the milestone would have been.

## Q7.1 What Q3 said to do, and what happened when it was done

Q3's record named the fix:

> **A rotate-and-place composite**, so a placement carries its rotation. It is
> a departure from strict action-fidelity — a person presses twice — but the
> *board* it produces is identical, and a proof written from it still replays.
> This is the one worth doing first.

Done: `Rotate` and `RotateLocked` filtered out of the learner's action space
in `qpack::decisions`. 2,500 episodes. The result:

| what it pressed | preset | owner | friend | perfect |
|---|---:|---:|---:|---:|
| **Pin** | **102/120** | **341/420** | **410/420** | **339/360** |
| Place | 13 | 46 | 7 | 8 |
| everything else | 5 | 33 | 3 | 13 |

The policy did not learn to place. It moved onto **`Pin`**, which is the next
free action — pinning a shop shelf changes no board — and pressed it 85% of
the time. Taking `Rotate` away moved the collapse one action along.

## Q7.2 The real fault, which is a scale and not a verb

A no-op cost `STEP_COST = 0.01`. The Q values at the end of training were
spread over **1.70**. So the ordering between a free action and a real one was
noise by a factor of a hundred and seventy, and *which* free action the policy
found was an accident of initialisation. Removing verbs one at a time cannot
fix that; there is always another cheapest key.

So the charge was made generic and made to read the board rather than the
verb: **an action whose feature vector comes back identical costs
`NOTHING_HAPPENED = 0.25` on top**, and `STEP_COST` went to 0.03. That catches
every free action there is, including ones a later mission adds.

| | ep 400 | ep 800 | ep 1200 | ep 1600 |
|---|---:|---:|---:|---:|
| Q3, as shipped | items 0.1 · won 0 | 0.3 · 1 | 0.6 · 0 | 0.9 · 2 |
| Q7, rotations out | 0.3 · 0 | 0.6 · 2 | 0.6 · 1 | 0.8 · 2 |
| Q7, + inert charge | 0.8 · 0 | 0.6 · 1 | 0.6 · 1 | 0.6 · 1 |

**Three runs, one trajectory.** Items assembled per episode plateaus below one
and the win rate sits at one or two in twenty in all three. The fix removed
the *symptom* — no run since presses one key 85% of the time — and did not
move the gate.

## Q7.3 The gate, and what it now means

Q3's gate — the learned packer at or above the control's 48/50 and 85.7%
balanced — is **missed**, and Q7 is the second milestone to miss it. What has
changed is that the diagnosis is no longer "it needs more training". Three
runs at different action spaces and different step costs produce the same
curve, which is the signature of a **representation** that cannot express the
answer rather than a search that has not found it.

The specific suspicion, stated so a later mission can test it: a placement is
scored as `(board, move)` where the move carries *where* and *what*, and the
thing that makes a placement good is whether the piece **completes a recipe**
with pieces already seated. `feature::mv` carries `feeds`/`fed` — whether the
piece answers a pool the board starves for — and carries nothing about
adjacency to an unfinished item. The control does not have this problem
because it enumerates seats and asks the engine whether an item assembled.

That is a feature, not a hyper-parameter, and it is the honest next thing.

## Q7.4 What was committed anyway

Both fixes are in `qpack.rs` and both are right independent of the gate:
`decisions` because a rotation genuinely is not a decision, and
`NOTHING_HAPPENED` because a free action genuinely should cost more. A run
that presses `Pin` four hundred times is uninformative about anything else,
and no run since does.

---

# Q8 — Themes

Read off the working tree after the completion feature landed. Two networks of
identical shape (`HIDDEN = 96`), identical budget (2,500 episodes), identical
seed. One saw a brief every episode drawn from eight themes; the other saw
thirteen zeros. Neither saw Hollow or Warden.

## Q8.1 The brief, and why it is not a one-hot

Q8's gate asks that a **held-out** theme be packed better with conditioning
than without. A one-hot cannot pass it, and not because the training is bad: a
class the network never saw is a coordinate that was zero in every gradient it
ever took, so the conditioning contributes exactly nothing and the two packers
are the same packer. The gate would be unpassable by construction.

So a brief is a *description* in the packer's own vocabulary - which grids to
fill, and which pools the pieces the theme allows tend to move
(`lab/src/themes.rs`). Checked before training, because a brief that does not
separate two themes cannot condition anything:

| held out | nearest trained themes, by cosine |
|---|---|
| **Hollow** | Wall 0.99, Caster 0.87, Drainer 0.76 |
| **Warden** | Burner 0.85, Wall 0.77, Slower 0.75 |

Every theme has a distinct row and both held-out themes have close trained
neighbours, so generalisation was a question worth asking rather than a
foregone answer.

## Q8.2 What the two packers did

Twelve situations each, packed greedily, judged by A2's fidelity meter on a
real fight with the packed board taking the field as the creature.

| theme | items, briefed | items, control | fidelity, briefed | fidelity, control |
|---|---:|---:|---:|---:|
| Striker | 0.6 | 0.2 | **0.336** | 0.174 |
| Wall | 0.7 | 0.2 | **0.062** | 0.021 |
| Burner | 0.5 | 0.2 | 0.000 | 0.000 |
| Slower | 1.1 | 0.2 | 0.023 | **0.051** |
| Drainer | 0.7 | 0.2 | 0.000 | 0.000 |
| Caster | 0.3 | 0.2 | 0.125 | 0.125 |
| Swarm | 0.6 | 0.2 | **0.149** | 0.116 |
| Beast | 0.5 | 0.2 | **0.500** | 0.250 |
| **Hollow** *(held out)* | 0.8 | 0.2 | 0.000 | 0.000 |
| **Warden** *(held out)* | 0.2 | 0.2 | 0.020 | **0.063** |

Two things are true at once and both are worth having.

**The conditioning did something.** The briefed packer assembles two to five
times as many items, and the control's row is *identical for every theme* -
0.2 items, the same board every time - which is what "unconditioned" means and
is a good sign the harness is measuring what it says.

**It did not carry.** On the eight themes it trained on, the brief is worth
more fidelity in four, the same in three and less in one. On the two it never
saw it is worth **+0.000** and **−0.043**. The brief is being used as a label
to memorise against, not as a description to interpolate from.

## Q8.3 The gate

**Missed.** Asked that both held-out themes be packed better with the brief
than without; Hollow ties at zero and Warden is worse by 0.043.

The honest reading is that this is Q3's miss again rather than a separate
failure. A network that assembles 0.8 items in forty presses has not learned
to pack, and conditioning something that cannot pack yet on *what kind* of
board to pack is asking the second question before the first is answered. The
brief is built, it is checked, it separates the themes, and it is wired
through the state - what it is waiting for is a packer worth conditioning.

---

# Q9 — The harvest

## Q9.1 The bug that made the first four harvests meaningless

`harvest`'s `subject()` defaulted every off-ladder creature to rung 40 and
took its theme from `theme_for(40)`, which is Drainer. So THE SURVEYOR — band
**35**, theme **Warden** — was dressed as a band-41 Drainer, judged against a
17.2 s line it was never meant to meet, and reported as reading 0.00 as a
Drainer, which was true and beside the point.

`FRAMES` carries the band and the theme for exactly this reason
(`bestiary.rs:393`). Both harnesses read it now.

## Q9.2 What the five borrowed boards actually score

Nothing in the repo had asked. `pack_francis` judges a *proposal* against the
creature it would replace, and both sides of that comparison can be off the
line together — which is what was happening: every candidate board for THE
SURVEYOR came back at exactly 0.302, the same figure as the board it ships
with, because the `owner` reference was winning at 12.0 s no matter what the
creature wore.

`cargo run --release -p gearmaster-lab --bin asworn`, at each creature's own
band:

| creature | band | line | as it ships |
|---|---:|---:|---|
| THE SURVEYOR | 36 | 14.7s | accepted, 0.186 off |
| **THE DROVER** | 43 | 18.2s | **off-curve, 0.541 against 0.300** |
| THE DRIVEN | 43 | 18.2s | accepted, 0.233 off |
| **THE COMMISSIONER** | 49 | 21.1s | **off-curve, 0.563 against 0.300** |
| **THE PARISH** | 51 | 22.1s | **off-curve, 0.630 against 0.300** |

**Three of the five borrowed boards are outside the acceptance band.** That is
not a criticism of F12's decision — borrowing bought five real fights at
roughly the right weight on the day it shipped, and said so — but "roughly"
turns out to be three creatures fighting at half again the length the curve
asks for, and nothing was measuring it.

## Q9.3 Boards from runs that were played

`dress` plays sixteen seeds in both modes with the frozen A-series packer,
replays each run's transcript to the board it finished holding, cuts that
board to the creature's own theme's grids, drops gear a creature may not wear,
and keeps whichever run's board lands nearest the line.

| creature | as it ships | dressed from a run | reads as its theme |
|---|---|---|---:|
| THE SURVEYOR | accepted, 0.186 | **accepted, 0.050** | 0.00 |
| THE DROVER | off-curve, 0.541 | **accepted, 0.009** | **1.00** |
| THE DRIVEN | accepted, 0.233 | **accepted, 0.119** | 0.19 |
| THE COMMISSIONER | off-curve, 0.563 | **accepted, 0.042** | 0.10 |
| THE PARISH | off-curve, 0.630 | off-curve, 0.358 | 0.50 |

**Four of five accepted, and all five nearer the line than the board they
wear today.** THE DROVER lands at 0.009 off a line of 18.2 s and reads as a
Striker at 1.00, which is as good an answer as this gate gives.

The five blocks are in `analysis/dressed/`. **Nothing was written into
`combat.rs`** — the owner reads every diff, and `make pack`'s save once
rewrote a creature nobody was editing.

## Q9.4 What the fidelity column says

Three of the five read poorly as their own theme: Warden 0.00, Wall 0.10,
Swarm 0.19. This is the exact hole Q8 was meant to fill. A board cut to a
theme's *grids* is not a board that fights like the theme, and choosing pieces
by what they *do* is the packing problem — so the fidelity column is a direct
measurement of what a trained quartermaster would be worth, and it is worth
between 0.00 and 0.50 per creature.

---

# Post-merge: the three suspects, tested

`HANDOFF-two-trades.md` closed the mission by naming three suspects for the
quartermaster's plateau, in the order the measurements supported, and said each
was one constant or one sampler rather than a rewrite. All three were then
tested, which is cheaper than arguing about them. **One of the three is real
and the other two are worse than doing nothing.**

Each is an environment variable on the same binary, so all three ran against
the same code, the same seed and the same 2,200 episodes.

| arm | what it changes | items assembled | won |
|---|---|---:|---:|
| baseline | — | 0.8 | 2/20 |
| `QPACK_PRIORITY=0.5` | oversamples transitions where something assembled | **0.6** | 0/20 |
| `QPACK_BUDGET=120` | forty presses becomes a hundred and twenty | **1.0** | 3/20 |
| `QPACK_PHI=0.6` | the shaping weight, 4x | **1.6** | 2/20 |
| `QPACK_PHI=1.5` | the shaping weight, 10x | **2.8** | 3/20 |
| `QPACK_PHI=4.0` | the shaping weight, 27x | 1.2 | 1/20 |

**Suspect 2 was right and the reasoning behind it was right.** A completing
placement was worth `+0.119` shaped against `-0.03` for any other change, while
the Q values were spread over 1.5 — so assembly was a whisper against the
noise, and the network could not hear the one event the whole task is about.
Ten times the weight is **three and a half times the items**, which is the
largest single move any change has produced in this mission.

It does not scale forever: at 4.0 the Q spread blows out to 5.4 and the policy
gets worse, which is the shaping starting to dominate the fight it is supposed
to be a hint about. The band is roughly four to ten times the original.

**Suspect 1 was wrong, and interestingly so.** Oversampling the transitions
where an item assembled made the packer *worse* — 0.6 items against 0.8. The
buffer was not short of completions; it was short of a reason to prefer them.
Feeding the same network more copies of a signal it cannot hear does not make
it louder, and it costs the diversity that the rest of the batch was carrying.

**Suspect 3 was nearly neutral.** Three times the press budget bought 1.0 items
against 0.8, and cost three times the wall clock. Forty presses was not the
binding constraint.

## What this does and does not change

It does **not** move Q3's gate. The gate is 48/50 on the repack benchmark and a
packer assembling 2.8 items an episode is not close to a packer that clears
forty-eight rungs. Q3, Q7 and Q8 all still miss.

What it changes is the diagnosis, which had been "a representation that cannot
express the answer" since Q7. That is no longer the best reading: the
representation could express it and the reward could not *weight* it. Whoever
picks this up should start from `QPACK_PHI` between 0.6 and 1.5 and should not
spend a run re-testing priority replay or the press budget — both were measured
and both were negative.

---

# C6 — The quest spec

Read off **`727fae4`** plus this milestone. `crates/engine/quest.rs` (new, and
landed inert), `crates/trades/quest.rs`, `crates/lab/quests.rs`. Engine 1074
(+15), trades 26 (+7), lab 4 (new). No warnings.

## C6.1 The chain, walked forward, which nobody had done

`tests/chain.rs` proves each station of the Manse chain opens the next, and it
proves it by standing the run at each door in turn - so between two of them it
sets `run.rung` **backwards**: THE LOCKED GATE is answered at rung 26 and the
house stands after rung 25. That is a true claim about the tables and it is not
a claim about the road.

`tests/quest.rs` walks it forward. Rung one upwards, every fight won by fiat,
every chain door answered the moment it stands, nothing ever set back:

```
  rung  8  word    A Word About the Wrong Stars      (sump-bottom's bar)
  rung 18  door    the-astronomer   -> "Hear him out"
  rung 23  door    the-locked-gate  -> "Use the word"
  rung 26  town    the-manse        -> cellar door
  rung 26  class   Threshold-Sighted   insight true
```

The chain survives, and the deadline comes out as **three bands** rather than
one. `lab/src/bin/qchain.rs` is the printer.

| the first word arrives | doors answered | town revealed | gate stands | class |
|---|---:|---|---|---|
| up to rung 25 | 2 | yes | rung 26 | **won** |
| rungs 26-29 | 2 | yes | never | - |
| rung 30 onward | 0 | no | never | - |

**The middle band is four rungs of this road where every cheap tier pays and
nothing can be finished.** A run there is offered both doors, hears the
astronomer out, uses the word at the gate, puts THE MANSE on the map - and the
house stood twenty-five rungs ago. Three of the four tiers of §3.6 pay in full
on a run that cannot finish, which is what makes the ordering rule a rule about
this game rather than a general worry.

## C6.2 The derivation, and the two walkers agreeing

`engine::quest::chain_to` walks `EVENTS`, `TOWNS`, `DUNGEONS` and `RUMOURS`
backwards from a thing at the end of a chain. `completable.rs` shares the
earliest-rung arithmetic rather than keeping its own copy - six functions moved
and none changed what they say.

```
pathfinder_threshold   Class("Threshold-Sighted")
  Prerequisite   Holding("A Word About the Wrong Stars")   rungs  8-25   Bar
  Offered        Offered("the-astronomer")                 rungs 18-25   Road
  Chose          Holding("A Word About the Cellar")        rungs 18-25   the-astronomer / the-slagworks
  Offered        Offered("the-locked-gate")                rungs 23-25   Road
  Chose          Gate("the-manse")                         rung  26      the-locked-gate
  Chose          Entered("the-threshold")                  rungs 26-50   the-manse, CellarDoor
  Finish         Wearing("Threshold-Sighted")              rungs 26-50   Road
  deadline: rung 25
```

**The deadline is derived and it is the rung the walk measured.** The two share
no code but the tables: the walk found rung 25 by playing to it, and the
derivation found it by tightening every window against its neighbours through
`Station::by_when` - which subtracts a rung at a town gate, because
`town::between` matches `after + 1` exactly and `Run::settle` asks for it the
instant a rung is cleared, so a reveal landing on the gate's own rung lands
after the question was asked.

## C6.3 Two things the derivation says that nobody had written down

**The road past Francis runs through the Manse's cellar.** Both routes to the
mainspring - THE PASSENGER and THE SECOND SHADOW - wait on `threshold-cleared`,
so a run that never went down the stair cannot open the fifty-first rung.
`pathfinder_unwound` derives as a **strict superset** of `pathfinder_threshold`,
nine stops to seven, and the two share a deadline: **rung 25 is the rung by
which a run has either bought a word at a bar or lost the end of the game.**

That is a stronger reason for §C7's ordering than §C7 gives.

**One of the three named models cannot be trained as written.**
`pathfinder_drover` is aimed at a chain of THE HUNDRED being finished, which the
engine records as a flag and `View` carries no flags. `lab::quests` refuses it
rather than dressing it headless. `analysis/second-order-quests.md` §5 has the
argument and the two ways out; it is a console question and it is the owner's.

## C6.4 The gates

| gate | result |
|---|---|
| the three chains derive without anybody typing them | **met** - and the third is refused for a reason, which is the fourth thing derived |
| the derived steps match what `completable.rs` believes | **met** - they share the traversal, so they cannot disagree |
| an agent that farms a cheap tier scores less than one that finishes | **met, and stronger than asked** |

The second gate is not a weighting. `Φ` is potential-based and is zeroed at the
end of every episode **however it ended**, so over any complete trajectory the
tiers telescope to `γᵀΦ(s_T) − Φ(s_0)` = 0. There is no sum for a finish to have
to beat.

    farming four tiers and running out of road   banked  0.00
    finishing the chain                          banked 50 · γ⁶ = 49.11

`crates/trades/tests/quest.rs` plays both, and every prefix of the chain under
both endings, and asserts the tiers add to nothing each time.

**Zeroing on truncation is the decision the farm makes for us.** `qpack.rs:409`
zeroes on `e.finished` only, so a packing that runs out of budget banks its
shaping - nearly harmless for items assembled, and exactly the farm for a chain,
because a farming episode is precisely one that ends by running out of road with
the cheap tiers ticked. The cost is a truncated bootstrap biased low by `Φ(s_T)`,
deliberately: a chain not finished was not progress worth anything.

---

# C7 — The named pathfinders

Read off **`973e54f`** plus this milestone. `crates/lab/{packers,shopping,roads}.rs`
(new), `qroad` quest-conditioned, `qaim` (new, the measurement), `qproof`
carrying all three halves of a run.

## C7.1 Three things had to be true before any training meant anything

**The packer must be the written control.** §C1's argument, and the arithmetic
behind it: the learned packer assembles 2.8 items and clears 8/50 rungs, and the
threshold chain's first door stands at rung 18. A road policy trained against a
packer that cannot clear rung ten never sees a chain that starts at rung
eighteen, and every failure is then ambiguous between the two agents.

**The control needed its own press budget.** `Step::Pack` had always passed
forty, which is the learned packer's budget on *decisions*. `hands::pack` is
exhaustive over anchors and Q0 measured it at a median of **492 presses**. Given
forty it bought four pieces, seated none, and lost rung one for ever.

    with 40 presses:    items 0, rung 1, 320 decisions, every one a defeat
    with 2,000:         items assembled, rung 26 reachable

**Neither agent can buy the word.** `Buy` and `Barter` are the quartermaster's
verbs and *why* to barter for a word is a fact about a door seven rungs away.
`lab::shopping` derives the list from the quest's own unpassed `Holding` stops
and the driver presses the key, which is §3.3's second option and, it turns out,
its only one. **The word is automatic; being at a bar before rung 25 is not**,
and that is the part left to be learned.

## C7.2 Is the chain reachable at all by this composition

Asked before training, because if the answer is no no amount of training makes
it yes. `lab::roads` is a **written plan-follower** - it is handed the chain and
told which choice at each door passes which stop, so it is an upper bound rather
than a baseline. 24 seeds, both modes, Medium, control packer:

| stop | rungs | reached |
|---|---|---:|
| `Holding("A Word About the Wrong Stars")` | 8-25 | **7/24** |
| `Offered("the-astronomer")` | 18-25 | 2/24 |
| `Holding("A Word About the Cellar")` | 18-25 | 2/24 |
| `Offered("the-locked-gate")` | 23-25 | 2/24 |
| `Gate("THE MANSE")` | 26 | 2/24 |
| `Entered("the-threshold")` | 26-50 | 2/24 |
| `Wearing("Threshold-Sighted")` | 26-50 | **1/24** |

The wall is the first stop, and it is a wall about *time* rather than about
choice: seven runs in twenty-four are alive and at a bar before rung 25.

## C7.3 The artefact, and it replays

`analysis/proofs/AA8D95DE31880461-grinder-medium-pathfinder_threshold.proof`:
the whole chain, **7/7 stops, rung 26, 220 presses across 166 road decisions and
66 packings**, and replayed from its own header into a fresh console **to the
same rung with zero refusals**.

All three halves are in it - the road decisions, the shopping the chain asked
for, and the packing - because a transcript missing any of them replays into a
different run. Only the presses that *stuck* are written: Q0 measured 761,840
trial pairs in 941,965 presses, and a transcript carrying the trials would be
four times longer and replay identically.

It is watchable in the window, and the header says how.

## C7.4 The training, and what it says

Two models, 500 episodes each, ~40 minutes apiece on the M2 Max, written
control packer, shopping list on. `QROAD_QUEST` names the chain and the model
is written under it.

| block | ε | `pathfinder_threshold` stops | finished | `pathfinder_unwound` stops | finished |
|---|---:|---:|---:|---:|---:|
| ep 100 | 0.71 | 6/7 | 0 | 6/9 | 1 |
| ep 200 | 0.43 | 5/7 | 0 | 6/9 | 2 |
| ep 300 | 0.14 | 5/7 | 0 | 3/9 | 0 |
| ep 400 | 0.05 | **1/7** | 0 | **0/9** | 0 |
| ep 499 | 0.05 | 1/7 | 0 | 0/9 | 0 |

**Both curves go the wrong way as exploration decays.** The ε-greedy noise
walks the chain and the greedy policy does not, which is the shape of a policy
that has learned nothing about the chain and has learned something about
something else.

The evaluation, 24 seeds a row, both modes, Medium, control packer throughout:

| chain | road policy | deepest rung | finished |
|---|---|---:|---:|
| threshold | no weights - first legal step | 1 | 0/24 |
| threshold | `pathfinder_threshold` | 1 | 0/24 |
| threshold | `pathfinder_unwound` | 1 | 0/24 |
| threshold | **written, following the plan** | **26** | **1/24** |
| unwound | no weights | 1 | 0/24 |
| unwound | `pathfinder_threshold` | 1 | 0/24 |
| unwound | `pathfinder_unwound` | 1 | 0/24 |
| unwound | **written, following the plan** | **47** | 0/24 |

**Neither model beats its own absence**, so §3.5's gate - each reaching its own
objective more often than the other two - is not met and is not close. Every
figure in this milestone that is above zero belongs to the written half.

## C7.5 Why, and it is not the reward

The trace says it in ten lines. The trained model packs twice and then presses
`Pack` for the remaining three hundred and eighteen decisions:

```
step 0  rung 1  tray 2  items 0   taking Pack
step 1  rung 1  tray 3  items 1   taking Pack
step 2  rung 1  tray 0  items 2   taking Pack
step 3  rung 1  tray 0  items 2   taking Pack        ... and so on to 320
```

The obvious reading is `CLAUDE.md` trap 44 - a free action is a scale problem -
and the obvious fix is a bigger charge for a decision that changes nothing. That
reading is wrong, and `--bin qmoves` is why.

**`feature::mv` cannot tell one road verb from another.** It was written for the
quartermaster: its one-hot has eight shapes for placements, purchases, sells,
barters, rerolls, rotations, unequips and clears, and `_ => 8` for everything
else. Every verb the pathfinder owns lands in that eighth bucket, and every
other field in the vector is about a *piece*, which a road verb does not have.

    1,341 road verbs offered across four runs
    four verb kinds among them: answer, drink, fight, town
    distinct feature vectors the network can see: 1

So the road network's action space, as it can see it, is **two**: `Pack`, which
is all zeros by convention, and "a road verb". It cannot tell `Fight` from
`Answer 0` from `Town chapel`, and which of them gets pressed is the order the
console listed them in.

That is not a reward problem, a horizon problem or an exploration problem. A
network that cannot distinguish two actions cannot prefer one, and no amount of
shaping teaches a preference between things that are the same input.

**It also retires a claim.** Q5.1 wrote that "the Q network is not what is
deciding" and put it down to Q3's miss - the packer that works being the written
one. The packer was not the reason. The road agent has never had an action
representation, in this milestone or in Q5, Q6 or A6, and every road policy this
repo has called learned reached the same rung as the one that presses the first
thing on the list.

`crates/trades/tests/quest.rs::the_pathfinder_can_tell_this_many_of_its_own_verbs_apart`
holds the number at **1**, and it is a ratchet that goes up.

## C7.6 The gates

| gate | result |
|---|---|
| `pathfinder_threshold` first, as an artefact | **met** - trained, named, written to `analysis/nets/` |
| each model reaches its own objective more often than the other two | **missed, and not close** - neither beats no weights |
| each measured with a packer plugged in | met - the written control throughout, per §C1 |

What C7 delivered that stands: the harness (`qroad` quest-conditioned, `qaim`,
`qproof`), the shopping list, the control packer as a macro-action, the
measurement that the chain is walkable, and **the reason the learning half has
never worked**, which is one measurement and had been mis-attributed for three
milestones.

## C7.7 Where the artefacts are

| what | where |
|---|---|
| the chain, walked and watchable | `analysis/proofs/AA8D95DE31880461-grinder-medium-pathfinder_threshold.proof` |
| the two trained models | `analysis/nets/pathfinder-{threshold,unwound}.txt` |
| how to run and watch any of it | `analysis/the-threshold-run.html` |
| the chain, derived | `cargo run -p gearmaster-lab --bin qquest` |
| the chain, walked forward | `cargo run -p gearmaster-lab --bin qchain` |
| how far a composition gets | `cargo run --release -p gearmaster-lab --bin qaim` |
| what the road agent can see | `cargo run --release -p gearmaster-lab --bin qmoves` |
| training | `QROAD_QUEST=<name> cargo run --release -p gearmaster-lab --features nn --bin qroad` |
| a proof | `QPROOF_QUEST=<name> cargo run --release -p gearmaster-lab --bin qproof` |
| watching one | `GEARMASTER_WATCH=<proof> cargo run -p gearmaster-gui` |

---

# R0-R5 — The Rogue pair

Read off **`b298999`** plus this milestone. The plan is
`analysis/the-rogue-pair.html`; the measurements are `--bin qrogue`,
`--bin qjudge`, `--bin qmoves` and `--bin qcross`.

## R.1 What Rogue changes, measured

Four questions the training loops depended on and none of them had an answer.

**A dead Rogue run wipes in place.** Lives 1 → 4, gold back to 28, rung back to
1, and `Console::over` is never true, because `Run::settle` replaces the run
before anything outside can observe a zero. An episode driven by it never ends
in Rogue: it keeps going, in a different run, at the bottom of the ladder.

**The road reward paid for the same rung repeatedly** - 6.37 payments per rung
actually reached in Grinder and 5.04 in Rogue, for opposite reasons: a Grinder
oscillates against a wall and a Rogue is reset to the bottom.

**Neither agent could see the mode.** `View::grinder` was in no feature vector,
and the road's `f[2]` separated the two only by accident, through
`lives_left.unwrap_or(9)` putting Grinder on 1.80 where Rogue cannot go.

**And `combat.rs` never names `Mode`.** Three doors are priced by it -
`Outcome::Spare` once and `Outcome::Underwrite` twice - and nothing else in the
rules is.

## R.2 Why a Rogue quartermaster is a reward and not a flag

The packer's reward was one simulated fight. A fight is a pure function of two
boards, so **passing `Mode::Rogue` where `qpack` passed `Mode::Grinder` would
have produced the same packer.**

`lab::scoring` prices a board against a **window** of the rungs ahead. Grinder
averages it; Rogue averages it and then pays again for the worst thing in it,
because in Rogue the worst thing in the window is the thing that happens and
there are four of those in a run. The risk is not a dice roll - combat is
deterministic - it is that the screen shows the *next* creature and nothing
beyond it.

The strong claim is a property: a board that wins its whole window is priced
**identically** by both judges, and one that loses anywhere in it is worth
strictly less to a Rogue. The gate is that they also *order* boards
differently, and `--bin qjudge` finds **4 inversions in 406 pairs**. Small, and
concentrated exactly where a run would die.

## R.3 The road agent gets action features, and it was worth 1 to 18

C7 closed on the finding that `feature::mv` is the *quartermaster's* move
description and every road verb fell into its one leftover bucket:
**1,341 road verbs across four runs, one distinct vector.**
`pathfinder::describe` is the road's own now - seventeen kinds, then fifteen
columns about the particular one - and the same walk produces **18**.

## R.4 The curriculum had to be walked, and by the pilot

`skip_to` pays the bounties and not the shops: a run stood at rung 25 that way
carries 1,516 gold, seven pieces, one shelf and **no assembled item at all**, so
the window has no board to look at.

Walking needed the control's own walk rather than a simplified one. A walker
that packs once a rung and presses the first road verb reaches rung 13 in
Grinder and **rung 1** in Rogue; the pilot, which barters, sells, rerolls, grows
and rearranges after a defeat, reaches 28 and **18**. That had looked like a
fact about the mode and was a fact about a hundred lines of harness.

And a walked situation arrives **already packed**, so the packing episode has to
take the board apart first or it scores the pilot: the first campaign evaluated
at 10 wins in 20 on episode zero, before a gradient.

## R.5 Four models, and the cross table

Quartermasters 800 episodes each; pathfinders 300 each behind the **written
control** packer, per §C1.

| | items | eval wins | training wins |
|---|---:|---:|---:|
| `quartermaster_grinder` | 2.9 | 4/20 | 172/399 |
| `quartermaster_rogue` | **3.5** | 0/20 | 22/399 |

Mean best rung over 12 seeds, each row in each mode:

| row | grinder | rogue |
|---|---:|---:|
| the written pilot | **32.8** | **6.7** |
| no weights + control packer | 1.0 | 1.0 |
| grinder road + control packer | 1.0 | 1.0 |
| **rogue road + control packer** | 1.0 | **2.3** |
| grinder pair (both learned) | 1.0 | 1.0 |
| rogue pair (both learned) | 1.0 | 1.0 |

**The gate is half met.** In Rogue the Rogue policy is ahead, 2.3 against 1.0.
In Grinder the Grinder policy is level with having no weights at all.

## R.6 Why, and it is the owner's rule doing the work

The traces say it in two lines. The Grinder policy presses `Fight` on an empty
board, for ever:

```
step 0  rung 1  items 0  W0 L0   taking Press(Fight)
step 1  rung 1  items 0  W0 L1   taking Press(Fight)     ... to 320
```

The Rogue policy packs first:

```
step 0  rung 1  items 0  W0 L0   taking Pack
step 1  rung 1  items 1  W0 L0   taking Press(Fight)
step 2  rung 2  items 1  W1 L0   taking Press(Fight)
step 3  rung 2  items 1  W1 L1   taking Pack
```

Both collapse onto one verb eventually - the Rogue one stalls on `Pack` from
step 5 - and both reached rung 53 and 47 under exploration before the greedy
policy settled. So the ε-collapse C7 found is not fixed and the action features
did not fix it.

**What did change is which verb they collapse onto, and that is the reward.** In
Grinder a loss costs `-1.0` and pays a bounty, so fighting an unbeatable rung is
nearly free and the policy never learns that packing comes first. In Rogue a
loss costs `-2.5` and four of them cost `-10` and end the episode, so the same
architecture, the same features and the same packer produce a policy that packs
before it fights and clears a rung.

That is the owner's rule - *losing must provide negative or no value* - showing
up as a behaviour rather than as a number, and it is the clearest evidence in
this mission that a reward change reaches a policy.

## R.7 The gates

| gate | result |
|---|---|
| R0: the ratchet rises from 1 | **met** - 18 |
| R0: a trained pathfinder beats the first legal step | **missed in Grinder, met in Rogue** (2.3 against 1.0) |
| R1: a wipe is an event and per-run notes reset | met |
| R2: losing is never worth anything | met, as a property (`trades/tests/losing.rs`) |
| R3: the two judges order boards differently | met - 4 in 406 |
| R4: the tray at a rung is what the run bought | met |
| R5: each pair ahead in its own mode | **half met** |

Everything above 1.0 in the cross table that is not the pilot belongs to the
Rogue road policy. The written pilot at **32.8 against 6.7** remains the widest
statement of how much harder the mode is, and no learned pair is near it.
