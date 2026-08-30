# Handoff — the two agents, and how they play together

Written against `fac1495` (2026-08-29). Suite: **1059 engine green, 88 GUI,
12 CLI, 52 ignored, 0 warnings**.

> **C6 and C7 have shipped.** §8 at the bottom is the ledger: what landed, what
> the measurements say, what changed shape, and the three things this found that
> are somebody's decision rather than somebody's bug. The milestones in §4 are
> left exactly as the owner wrote them, because a brief that gets edited to
> match what happened stops being a brief.

Read `CLAUDE.md` first. Then `analysis/the-two-trades.md`, which is the
measurement record for everything summarised in §1 and is far more detailed
than this. This document is the state of the work plus the owner's decisions
about what comes next.

---

## 1. What exists, and what it is worth

A mission called **THE TWO TRADES** built two reinforcement-learning agents
over the same game, in ten milestones Q0 to Q9. Every number below is from
`analysis/the-two-trades.md`.

### The partition, which is the good part

`crates/trades/src/partition.rs` splits every `Verb` in the console into two
disjoint sets and neither agent is ever offered a verb it does not own.

| | verbs |
|---|---|
| **Quartermaster** | `Place`, `PlaceLocked`, `Unequip`, `Rotate`, `Lock`, `ClearSlot`, `ClearAll`, `Undo`, `Grow`, `Buy`, `Sell`, `Barter`, `Reroll`, `Pin` |
| **Pathfinder** | `Fight`, `Answer`, `Town`, `WalkOn`, `ThrowPoints`, `Leave`, `Walk`, `Out`, `Perambulate`, `Drink`, `Double`, `Pedestal`, `Crush`, … |

This is sound and should not be rebuilt. **One line of it is wrong for what
comes next** and §3.3 says which.

### The quartermaster (`crates/lab/src/bin/qpack.rs`)

DQN over `(board, move)` pairs — the menu changes shape every step, so there is
no head with a neuron per action; the network scores a pair and the agent takes
the argmax over what is legal.

**It does not work yet.** The gate is 48/50 on the repack benchmark and it is
nowhere near:

| tray | agent items | agent cleared | control (`A3`) |
|---|---:|---:|---|
| owner | 2 | 8/50 | 17 items, **48/50** |
| friend | 0 | 1/50 | 18 items, **49/50** |
| preset | 0 | 2/50 | 7 items, 12/50 |
| perfect | 0 | 1/50 | 15 items, 46/50 |

The overnight run tested three suspects on the same seed and 2,200 episodes:

| arm | items | won |
|---|---:|---:|
| baseline | 0.8 | 2/20 |
| `QPACK_PRIORITY=0.5` | **0.6** | 0/20 |
| `QPACK_BUDGET=120` | **1.0** | 3/20 |
| `QPACK_PHI=0.6` | **1.6** | 2/20 |
| `QPACK_PHI=1.5` | **2.8** | 3/20 |
| `QPACK_PHI=4.0` | 1.2 | 1/20 |

**The shaping weight was the whole plateau.** A completing placement was worth
`+0.119` shaped while the Q values were spread over 1.5, so the one event the
task is about was inaudible. Ten times the weight is three and a half times the
items. Priority replay made it *worse*; the press budget was nearly neutral.
**Do not spend a run re-testing either — both were measured and both were
negative.**

The reward as it stands is in §2.

### The pathfinder (`crates/trades/src/pathfinder.rs`)

Q-learning over the road, with `Step::Pack` as **one macro-action into a frozen
packer**, and a `Goal` in the state — which is what turns an agent that plays
well into a solver that can be asked whether a door is reachable. Goal is
one-hot in the features and `+50` in the reward.

Reachability, aimed rather than accidental:

| | A5 | A6 | **Q6** |
|---|---:|---:|---:|
| doors offered | 70% | 72% | **79%** |
| branches taken | 67% | 62% | **68%** |
| towns | 4 of 6 | 4 of 6 | **5 of 6** |
| dungeons | 1 of 7 | 1 of 7 | **5 of 7** |

**No door in this game has been shown unreachable.** Eleven are unreached and
each has a named chain in front of it.

An honest caveat the record makes and this handoff repeats: **the Q network is
not what is deciding** in most of that improvement. Q3 missed its gate, so the
packer inside `Pack` is largely the control.

### The brief (`crates/trades/src/brief.rs`)

Thirteen floats — five grid weights, eight pool affinities. A theme reaches the
packer as a **description in the packer's own vocabulary** and never as a name:
`gearmaster-trades` cannot see `MonsterTheme`, deliberately, so whoever asks
for a board has to say what they want in terms the packer understands.

**This is the signalling channel the composition needs and it already exists.**

### Q9, which is about creatures rather than agents

The harvest found `subject()` defaulting every off-ladder creature to rung 40,
which made the first four harvests meaningless. Once fixed, **three of five
borrowed boards are outside the acceptance band** — THE DROVER at 0.541, THE
COMMISSIONER at 0.563, THE PARISH at 0.630 against a 0.300 ceiling. Dressing
THE DROVER from a run brought it to 0.009 and made it read as its theme at
1.00. That work is unfinished and is not what this handoff is about, but it is
the reason `analysis/dressed` and `analysis/nets` exist.

---

## 2. The reward, as it stands today

Per press (`qpack.rs`):

```
r_step = γ·Φ(s′) − Φ(s) − STEP_COST − (NOTHING_HAPPENED if the features did not move)
```

* `Φ = QPACK_PHI × items assembled`, default **1.5**. Potential-based, so it
  provably leaves the optimal policy alone. **Φ counts items and nothing about
  pools, deliberately** — if the shaping said matched pools were good, Q4 could
  not claim it discovered them.
* `STEP_COST = 0.03`. Without it the policy collapsed onto `Rotate`: 400
  presses out of 420, nothing assembled.
* `NOTHING_HAPPENED = 0.25`, charged when an action leaves the features exactly
  as it found them. Q7 removed `Rotate` from the action space and the policy
  moved straight onto `Pin` — 410 of 420 — so the charge is generic and reads
  the board rather than the verb. **Taking actions away one at a time is not a
  strategy.**

Once, at the end, added to the last step and credited backwards:

```
win   = 1.0 + quick·0.5 + decided        quick = 1 − ttk/30s, decided = 0.3 under sudden death
loss  = −1.0 + (1 − enemy_health_left)·0.8
empty = −1.5
total = win_or_loss + FIDELITY(0.5) · delivered(brief)
```

`γ = 0.995` over a 40-press budget. It was 0.97 over 120, which discounted the
fight to **2.6%** — the agent optimised the shaping and the step cost and
nothing else.

---

## 3. What the owner has decided

Six decisions. They are not negotiable defaults; they are the brief.

### 3.1 Train the quartermaster against every fight on the ladder, in order

Not the current curriculum, which draws a rung and stands a run there with
`skip_to`. **Every fight in the row.** The packer should see rung 1 through
Francis as a sequence, because that is how a run meets them and because a board
that clears rung 12 and dies at 13 is a different lesson from a board that was
dropped at 13 cold.

Consequences to think about before writing it:

* `skip_to` pays the bounties and leaves the tray holding a handle and a blade.
  Walking the row instead means the **shop economy is real** — the tray at rung
  20 is whatever the run bought on the way, which is the thing the current
  curriculum papers over.
* An episode is now much longer than 40 presses. `BUDGET` is per *packing*,
  not per run, so it probably stays; what grows is the number of packings.
* The terminal reward has to become per-rung — see 3.2.

### 3.2 The reward rises per rung cleared, and per pool per second

The owner wants two changes to `score()`:

* **Per rung cleared.** More reward the further up the row the board gets, not
  one fight scored once.
* **Pool income.** Rage per second, faith per second, and the rest — a board
  that banks faster is a better board.

Two things to hold while doing it:

1. **The `QPACK_PHI` band was measured against a reward capped near 1.8.** If
   the terminal reward can now reach 10, the band 0.6–1.5 does not hold and has
   to be re-derived. **Change the reward first, then re-run the arms.** Do not
   inherit the number.
2. **Pools in the reward retires Q4's claim.** Φ was kept blind to pools so the
   agent could be said to have *discovered* pool matching. Putting pool income
   in the terminal reward does not break the shaping argument, but it does mean
   Q4's finding no longer stands unqualified. Say so in the commit rather than
   letting somebody find it later.

### 3.3 The shopping list is forced, not learned

**The quartermaster does not learn to buy rumours or crests.** The pathfinder
learns *what the run needs*; when it has decided, the purchase is executed
**programmatically** and the packer is handed a tray that already contains it.

This is the right split and it resolves the tension §1 flagged: `Buy` is in the
quartermaster's verb list, but *why* to buy a Rumour is a fact about a door
three rungs away that the packer cannot see and should not be asked to.

What this means in practice:

* A **shopping list** channel beside the `Brief` — the planner names pieces,
  the driver buys them, the packer receives them as constraints rather than as
  reward.
* **Not in the `Brief`.** A brief is a *description*; "buy this piece" is an
  instruction, and folding it in would break what makes Q8's held-out-theme
  gate meaningful.
* `partition.rs` may need `Buy` to be splittable by argument, or the driver
  performs the purchase outside both agents' action spaces. The second is
  simpler and is what "programmatically" means.

### 3.4 The quartermaster is done when it can pack three sets that clear Francis

The training phase ends on this and not on an episode count: **three different
gear sets, each of which beats Francis.** Different is doing work in that
sentence — three copies of one build is one answer found three times.

Somebody has to decide what "different" means before the gate can be written.
The cheapest defensible reading: three boards whose **item sets differ by more
than half their pieces**, all three winning. `share.rs` can encode them and
`reference_builds.rs` is the file that already holds boards of this kind.

### 3.5 The pathfinder is trained per quest, and the models are named

One model per objective, each with its reward maximised on **completing that
event**:

```
pathfinder_unwound      the road past Francis
pathfinder_threshold    the stair, and the mind lane it opens
pathfinder_drover       the county chain
```

Each is an output artefact with a frozen packer plugged into it to execute the
plan. `Goal` already carries `Door / Dungeon / Town / Rung / County`, so the
enum is there; what is missing is the training harness that takes one goal,
trains to it, and writes a named model.

### 3.6 A quest is a spec the pathfinder can read, and it pays along the way

`Goal` today is one thing: a door, a dungeon, a town, a rung, a tile. Reaching
it is `+50` and everything before it is worth nothing. **A chain twelve
decisions long with one payout at the end is a reward the agent cannot climb** —
it is the same fault §1 found in the packer, one agent along, and it is why
eleven doors are unreached.

So a quest becomes a **spec**: an ordered thing with named steps, and the
pathfinder is paid at each one.

**The tiers the owner has asked for**, cheapest to dearest:

| what happened | why it pays |
|---|---|
| an event on the chain was **offered** | the run is in the right part of the road |
| a **rumour or item the chain requires** was bought | a prerequisite that is easy to walk past |
| the **correct choice** at a chain event was taken | the only irreversible decision in the chain |
| the chain **finished** | the objective |

Each tier is worth more than the one above it, and the last is worth more than
all of them together — or the agent will farm the cheap tiers and never
finish. That ordering is the whole design and it is the thing to get wrong.

**Derive the spec, do not hand-write it.** `event.rs` already carries the
reverse indexes this needs and they exist for exactly this kind of question:

* `set_by(flag)` — every `(event, choice)` that sets a flag, walked through
  `every_outcome` so a flag set inside an `All` counts as hard as one set
  alone.
* `every_outcome(o)` — flattens `All` and both arms of a `Gamble`.
* `flags_waited_on()` — every flag some door is waiting for.
* `conditioned_by(rumour)` — every event a rumour opens.
* `Requirement` — twelve variants, including `Took`, `Holding`, `Flag`,
  `Counter`, `HoldingRumour`, `CountyCleared`. **That enum is the vocabulary a
  step is written in**, and it is already what the game checks.

Working backwards from a goal through `Requirement` → `set_by` → the choice
that sets it gives the chain without anybody typing it twice. A hand-written
spec is a second copy of `EVENTS` and goes stale the first time a door moves —
which `CLAUDE.md` trap 20 is about and which this repo has paid for four times.

`tests/completable.rs` is the closest thing that already does this walk. It
asks whether a key can exist before its door shuts; a quest spec asks the same
graph a different question, and the two should share the traversal rather than
each have one.

**Three things to decide while building it:**

1. **Whether a step can be reached more than once.** Paying for the same event
   twice is a farm. Steps are one-shot per episode and the spec has to say so.
2. **What "the correct choice" is.** For a chain derived backwards it is the
   choice that sets the flag the next step waits on — which is well defined and
   falls out of `set_by`. For a choice with two acceptable answers it is not,
   and the spec should carry a set rather than a label.
3. **Whether the tiers are potential-based.** `F = γΦ(s′) − Φ(s)` provably
   leaves the optimal policy alone, and the packer's Φ is the worked example in
   this repo. A chain has a natural potential — **how many steps of the spec are
   done** — and using it means the granular rewards cannot change what the best
   plan *is*, only how fast it is found. That is worth more than it costs.

**Repeatable actions, and why a step must key on the outcome.**

The question "is repeatability an issue" was asked and measured, and the
answer is no for the case that prompted it — but the survey is worth carrying
because it is the sharpest argument for the one-shot rule above.

Doors that appear at more than one town, which is the "same name, different
place" set:

| door | towns | costs the visit | scope |
|---|---:|---|---|
| `town-county` | **6** | no | per town — six trips down |
| `town-chapel` | 3 | yes | per town |
| `town-pub` | 3 | yes | per town |
| `town-factory` | 3 | yes | per town |
| `town-shop` | 3 | yes | per town |
| `town-pedestal` | 2 | no | **door per town, destination once a run** |

The other twelve doors belong to one town each; the three hidden towns carry
their own.

**The county is already repeatable and already free.** `TripSource::Town(id)`
is keyed by town id, `seats()` returns `TOWNS.len()`, and `costs_the_visit()`
is false for `County` — so six trips are available and none of them spends a
town's one action. Nothing needs changing for it.

**The binding constraint is the visit, not the door.** A town is one action and
`towns_seen` means one visit, so chapel, pub, factory and shop are repeatable
across three towns while each costs that town's only action. County and
pedestal are the two exceptions, which is exactly why they are the two that
feel repeatable.

**The pedestal is the asymmetry to watch.** The door is at two towns and costs
nothing; `destinations_visited` is shared across both, deliberately — *"the
second exists so a run whose orbs arrived late can still spend them, not so a
patient run spends them twice."* A repeatable action with non-repeatable
outcomes.

So: **a step keys on the outcome, never on the action.** "Visit the chapel" is
satisfiable three times and "go down into the county" six, and the county one
is free, which makes it the cheapest farm in the game. Piety is the case that
proves this is not pedantry: stacking Piety across three chapels is *meant* to
be repeatable and is worth doing, so a step reading "chapel visited" would both
farm and mislead, while one reading "Piety at least n" is honest about what the
run actually needs. `Requirement::Counter { what, at_least }` already says that
second thing.

**And the named models get their argument back.** §3.5 wants
`pathfinder_unwound`, `pathfinder_threshold` and `pathfinder_drover` to be
three models rather than one copied three times. Three *specs* with different
steps is what makes that true — without granular steps the only difference
between them is which state gets `+50`, and a goal one-hot is a thin thing to
hang three models on.

---

## 4. Milestones

Ordering matters here more than usual: **C1 exists so that every later failure
is attributable to one agent rather than to the pair**, and C6 comes before
C7 because a quest spec is what makes three named models three models.

### C1 — The driver, with the control packer ▲

Compose pathfinder + the **A3 control packer**, not the learned one. A run
plays end to end.

* **Deliverable:** a run that walks the road, packs at each rung, fights, and
  answers doors — and a log that says which agent made each decision.
* **Gate:** the composed run clears at least as many rungs as the control
  packer does alone. If composing makes it worse, the driver is wrong and
  nothing after this milestone means anything.
* **Why first:** the learned packer assembles 2.8 items against a control that
  assembles 17. Composing now makes every failure ambiguous.

### C2 — Goal to brief ▲

One pure function: goal + the creature at the next rung → `Brief`.

* Lives on the **planner's** side of the boundary. `gearmaster-trades` still
  cannot see `MonsterTheme`; `bestiary::theme_for(rung)` is the source.
* **Gate:** two different goals produce visibly different briefs, and the
  packer's boards differ accordingly. Q8's `delivered()` is the meter.

### C3 — The shopping list ▲

The second channel of 3.3.

* **Gate:** a plan that wants an Orb of Travel gets one bought and finds it in
  the tray; a plan that does not, does not. And the packer's own reward is
  unchanged by the purchase — it is a constraint, not a bonus.

### C4 — The row, and the new reward ▲

3.1 and 3.2 together, because the curriculum and the terminal reward are the
same change seen twice.

* Walk the ladder in order; reward rises per rung cleared and with pool income.
* **Re-run the three arms** on the new reward. `QPACK_PHI` is the only one that
  ever mattered; re-derive its band and write the new table into
  `analysis/the-two-trades.md` beside the old one rather than over it.
* **Gate:** items assembled beats 2.8 at some setting of `PHI`, and the table
  says which.

### C5 — Three sets that beat Francis ▲

The end of the quartermaster's training phase, per 3.4.

* **Gate:** three boards, each beating Francis, differing by more than half
  their pieces. Encoded as share codes so they are reproducible and so
  `reference_builds.rs` can hold them.
* **Deliverable:** a frozen packer model, and the three boards as evidence.

### C6 — The quest spec ▲

3.6, and it comes before the named models because it is what makes them
different from one another.

* A `Quest` type: ordered steps, each in `Requirement`'s vocabulary, **derived**
  from `EVENTS` by walking `set_by` and `every_outcome` backwards from a goal.
* The four tiers, each dearer than the last and the finish dearer than all of
  them together.
* Steps are one-shot per episode.
* **Gate:** the three chains of §3.5 derive without anybody typing them, and
  the derived steps match what `completable.rs` already believes about the same
  doors. Two walkers over one graph that disagree is a bug in whichever is
  newer.
* **Second gate:** an agent that farms a cheap tier scores less than one that
  finishes. Write that as a test with two hand-made trajectories rather than
  hoping training finds it.

### C7 — Named pathfinders ▲

One harness, three specs, three artefacts. **`pathfinder_threshold` is first**
and is the owner's chosen target: the class at the end of the Manse chain.

The chain, endpoints confirmed against the tables — **derive the middle rather
than trusting this list**:

1. the rumour `A Word About the Wrong Stars` opens **THE ASTRONOMER**
   (`at: 28`, window from 17)
2. a choice there hands over `A Word About the Cellar`
3. that word opens **THE LOCKED GATE** (`at: 40`, window from **22**), whose
   `Use the word` choice is `Outcome::RevealTown("the-manse")`
4. **THE MANSE** is `Unlock::Hidden` and stands `after: 24`
5. `Action::CellarDoor` enters `the-threshold`
6. clearing it pays `reward: "Threshold-Sighted"`, plus `UnlockInsight` and
   the flag `threshold-cleared`

**There is a deadline in the middle of that**, and it is why this is a good
first target rather than an easy one: the reveal window opens at rung 22 and
the Manse gate stands after rung 24, so steps 1-3 have to be finished inside a
narrow band or the town is never offered. `town_between` filters a hidden town
by `towns_revealed` *and* by the rung gap, so a late reveal is a town you walk
past. **Verify that against `completable.rs` before training anything** — it is
the file that audits exactly this, and if the window is tighter than it looks
the spec has to say so.

The class itself is `ClassPower::WrongSense(60)`, which is the same trade THE
WRONG SENSE crest makes — worth knowing, because a pathfinder that wins this
chain hands the quartermaster a board that no longer deals ordinary damage.

* **Gate:** each reaches its own objective more often than the other two do,
  which is the only thing that makes them three models rather than one model
  copied. And each is measured with C5's packer plugged in.

### C8 — The record ▲

`analysis/the-two-trades.md` gains the composition's numbers;
`design/HANDOFF-two-agents.md` gains a "what shipped" ledger; `CLAUDE.md` gains
its counts and whatever traps this earned.

---

## 5. Risks, named now

1. **Composing before the packer works hides which agent is failing.** C1's
   control is the whole defence and it should not be skipped to save a day.
2. **The `PHI` band will not survive the reward change** and inheriting the
   number is the likeliest silent mistake in this plan.
3. **A frozen packer is load-bearing.** The pathfinder's `Pack` assumes the
   packer does not move underneath it; training both at once makes the
   planner's reward non-stationary. If joint training is ever wanted, it is its
   own milestone with its own argument.
4. **"Three different sets" is undefined until somebody defines it.** Write the
   definition into the gate's assertion, not into a commit message.
5. **Pool income double-counts.** A board that banks faster usually wins
   faster, and winning faster is already worth `quick·0.5`. Measure whether the
   new term changes the ordering of boards or only their scale.
6. **The row is slow.** Fifty fights an episode against forty presses a packing
   is a different order of wall clock. Budget it before C4 rather than
   discovering it at hour six.
7. **Granular rewards are farmable.** Four tiers paying along a chain is four
   ways to earn without finishing. Potential-based shaping is the defence and
   the ordering is the other one - the finish must be worth more than every
   step combined, and there should be a test that says so rather than a hope.
8. **A hand-written quest spec is a second copy of `EVENTS`.** It will go stale
   the first time a door moves, silently, and the agent will train against a
   road the game does not have. Derive it.

---

## 6. Where things live

| What | Where |
|---|---|
| The verb partition | `crates/trades/src/partition.rs` |
| The packing episode, `Move`, `Goal`, `Step` | `crates/trades/src/env.rs` |
| The brief | `crates/trades/src/brief.rs` |
| The pathfinder, `ROAD` features | `crates/trades/src/pathfinder.rs` |
| The packer's trainer and reward | `crates/lab/src/bin/qpack.rs` |
| The control packer and the pilot | `crates/agent/src/` |
| The console both agents play through | `crates/console/` |
| The oracle, `as_creature` (leaks — see below) | `crates/oracle/src/lib.rs` |
| Every measurement | `analysis/the-two-trades.md` |

**One landmine worth carrying forward:** `oracle::as_creature` leaks its gear
on purpose (`Box::leak`, `oracle/src/lib.rs:197`). That is correct for a
harvest that runs once and ruinous inside a training loop. `qpack`'s
`delivered()` avoids it deliberately and says so; anything new that reads a
board *as a creature* inside a loop has the same problem.

---

## 7. How to run things

```
cargo run --release -p gearmaster-lab --features nn --bin qpack
QPACK_PHI=1.5 QPACK_BUDGET=40 cargo run --release -p gearmaster-lab --features nn --bin qpack
cargo test -p gearmaster-engine          # 1059 green, 58 binaries + lib
cargo test -p gearmaster-gui             # 88; cargo build does NOT compile them
cargo test -p gearmaster-cli             # 12, the replay contract
```

Never start a second cargo while one is running. The engine has **zero
dependencies** and that is deliberate — the learning crates are where `burn`
lives, and it stays that way.


---

## 8. What shipped — C6 and C7

Written against `973e54f`. Suite: **1074 engine, 88 GUI, 12 CLI, 26 trades,
11 agent, 4 lab, 52 ignored, 0 warnings.** Every figure below is from
`analysis/the-two-trades.md`, blocks C6 and C7.

### 8.1 The chain was verified before anything was trained, and it needed to be

§C7 said to check the window against `tests/completable.rs` before training. The
check found something else: **nothing in the suite had ever walked the chain
forward.** `chain.rs::the_chain_can_be_finished_in_one_run_in_either_mode`
answers THE LOCKED GATE at rung 26 and then sets `run.rung` *backwards* to 25 to
meet THE MANSE. It proves the doors and not the road.

`tests/quest.rs` walks it forward and **the window is not tighter than §C7
says**. What it is, is three bands rather than one:

| the first word arrives | doors answered | town revealed | class |
|---|---:|---|---|
| up to rung 25 | 2 | yes | **won** |
| rungs 26-29 | 2 | yes | never |
| rung 30 on | 0 | no | never |

The middle band is four rungs where **three of the four tiers pay in full on a
run that cannot finish**. §3.6's ordering rule is not a general worry; it is
about these four rungs.

### 8.2 The spec derives, and it says two things nobody had written down

`engine::quest::chain_to` walks the tables backwards; `completable.rs` shares
the traversal rather than keeping a second copy. The sharp edge is
`Station::by_when`, which takes a rung off everything behind a town gate because
`town::between` matches `after + 1` exactly and `Run::settle` asks the instant a
rung is cleared. Tightening every window through it gives **rung 25**, which is
the rung the forward walk measured by playing to it. Two walkers, no shared code
but the tables, one answer.

- **The road past Francis runs through the Manse's cellar.** Both routes to the
  mainspring wait on `threshold-cleared`. `pathfinder_unwound` derives as a
  strict superset of `pathfinder_threshold`, nine stops to seven, sharing a
  deadline. That is a better argument for §C7's ordering than §C7 gives.
- **`pathfinder_drover` cannot be trained as written.** A county chain finishing
  is a flag and `View` carries none. `lab::quests` refuses it rather than
  dressing it headless. §8.5 is the decision.

### 8.3 The farm is defended structurally, not by weights

§3.6 asked for the finish to be worth more than every step combined. What
landed is stronger: `Φ` is potential-based and is **zeroed at the end of every
episode however it ended**, so the tiers telescope to nothing and there is no
sum for a finish to beat.

    farming four tiers and running out of road   banked  0.00
    finishing the chain                          banked 49.11  (50 · γ⁶)

Zeroing on *truncation* as well as termination is the decision the farm makes
for us, and it is the one thing `qpack` gets wrong on its own side
(`qpack.rs:409` zeroes on `e.finished` only). For items assembled that leak is
nearly harmless; for a chain it is exactly the farm.

### 8.4 Three things had to be fixed before training meant anything

Each was a day and each is one constant or one small module.

- **The packer had to become the written control**, per §C1. The learned one
  assembles 2.8 items; the chain's first door stands at rung 18.
- **The control needed a budget on presses rather than decisions.** It had
  always been handed forty, which is the learned packer's budget on *decisions*;
  `hands::pack` is exhaustive over anchors and Q0 measured its median at 492
  presses. With forty it bought four pieces, seated none, and lost rung one for
  ever.
- **Neither agent can buy the word the chain starts with.** `Barter` is the
  quartermaster's and *why* to barter is the pathfinder's. §3.3's "the driver
  performs the purchase outside both agents' action spaces" is not the simpler
  of two options — it is the only one. `lab::shopping` derives the list from the
  quest's own unpassed stops.

With those, the composition reaches the chain: **the word in 7 of 24 seeds, the
chain finished in 1**, with the plan-follower and the control packer.
`analysis/proofs/AA8D95DE31880461-grinder-medium-pathfinder_threshold.proof` is
one such run, 7/7 stops to rung 26, replayed with zero refusals and watchable
in the window.

### 8.5 Three decisions that are the owner's

1. **The drover's finish is not on the screen.** Either the county tab grows a
   line saying which of its three chains are done — which a player arguably
   should be told anyway — or `pathfinder_drover` is aimed at something already
   visible. `crates/lab/tests/quests.rs` holds the refusal and names the day it
   stops being needed.
2. **The game gives a player no signal that the chain has become unwinnable.**
   Past rung 25 the word is still in the tray, the doors still stand, the map
   still draws a town nobody can reach. A player is in exactly the agent's
   position. That is content, not a bug.
3. **`lab`'s tests are green and invisible.** The translation between the two
   sides of the boundary can only be tested from `lab`, which is not one of the
   three suites every count in `CLAUDE.md` quotes. Run it with
   `cargo test -p gearmaster-lab --test quests`.

### 8.6 C7's gate is missed, and the reason is one measurement

Two models, 500 episodes each. **Neither beats its own absence.**

| chain | road policy | deepest rung | finished |
|---|---|---:|---:|
| threshold | no weights | 1 | 0/24 |
| threshold | `pathfinder_threshold` | 1 | 0/24 |
| threshold | written, following the plan | **26** | **1/24** |
| unwound | `pathfinder_unwound` | 1 | 0/24 |
| unwound | written, following the plan | **47** | 0/24 |

The trace shows the trained model packing twice and then pressing `Pack` for
the remaining 318 decisions, which is trap 44's shape exactly - a free action -
and trap 44's fix was already in (`NOTHING_HAPPENED`, read off the board rather
than the verb). So the shape fit and the diagnosis did not.

**`feature::mv` cannot tell one road verb from another.** It is the
quartermaster's move description: eight one-hot shapes for placements,
purchases and rotations, `_ => 8` for everything else, and every field after
that about a *piece*. Measured over four runs:

    1,341 road verbs offered - answer, drink, fight, town
    distinct feature vectors the network can see: 1

The road network's action space, as it sees it, is `Pack` and "a road verb".
Which road verb is the order the console listed them in.

**That retires a claim in the record.** Q5.1 wrote "the Q network is not what is
deciding" and put it down to the packer being the written one. The packer was
not the reason. A5, A6, Q5, Q6 and C7 have every one of them measured a road
policy that cannot distinguish its own actions, and every one of them reached
about the rung a policy that presses the first thing on the list reaches.

`crates/trades/tests/quest.rs::the_pathfinder_can_tell_this_many_of_its_own_verbs_apart`
holds the figure at **1** and is a ratchet that goes up. `--bin qmoves` is the
measurement.

**The next milestone is the road's action features, and it is not this one.**
Its shape is clear and all of it is on the screen: which kind of verb,
separately rather than in one bucket; for an `Answer`, which choice and what it
requires and what it does; for a `Town`, which door; for `ThrowPoints`, which
exit. Landing it inside a milestone about quests would have meant the quest work
and the feature work were measured together and neither could be attributed.

### 8.7 And the reason it was missed was not the one in 8.6 either

R0 gave the road agent action features and the policy still pressed one verb.
`--bin qwhy` finally asked what it thought they were worth: `fight -0.496`
against `pack -0.499`. A flat network, three thousandths apart, against rewards
spread over four to fifty.

The loss was Huber with a knee at one and the targets ran to **+54**, so a state
worth fifty pulled exactly as hard as one worth one and a half and the network
fitted the median target. Scaling the reward by 1/25 put the targets where the
loss is proportional, and the Grinder pathfinder went from **1.0 to 17.5** on
the cross table, against the written pilot's 32.8.

Both halves of R5's gate are met now. The two *pairs* are still 1.0, because a
pair is worth what its packer is and the learned packer cannot clear rungs -
which is C5's, not this.

`analysis/the-two-trades.md` R6 has it, and `CLAUDE.md` traps 52 to 54.

### 8.8 What C6 and C7 did not do

- **C1 through C5 are still open**, and C7 was reached without them by freezing
  the written control rather than a learned packer. That is exactly what §5.3
  says a frozen packer is for, but it means the quartermaster's half of the
  mission is untouched: `QPACK_PHI`'s band has not been re-derived against a new
  reward, the row has not been walked in order, and nothing packs three sets
  that beat Francis.
- **§3.5's gate — each model reaching its own objective more often than the
  other two — is measurable now and `qaim` is the harness**, but with
  `pathfinder_drover` refused there are two models rather than three, and with
  §8.6's finding the comparison is between two policies that cannot see their
  own actions. The harness is right and there is nothing yet to put in it.
