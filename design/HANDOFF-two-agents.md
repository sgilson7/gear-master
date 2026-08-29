# Handoff — the two agents, and how they play together

Written against `fac1495` (2026-08-29). Suite: **1059 engine green, 88 GUI,
12 CLI, 52 ignored, 0 warnings**.

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

Five decisions. They are not negotiable defaults; they are the brief.

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

---

## 4. Milestones

Ordering matters here more than usual: **C1 exists so that every later failure
is attributable to one agent rather than to the pair.**

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

### C6 — Named pathfinders ▲

One harness, three runs, three artefacts: `pathfinder_unwound`,
`pathfinder_threshold`, `pathfinder_drover`.

* **Gate:** each reaches its own objective more often than the other two do,
  which is the only thing that makes them three models rather than one model
  copied. And each is measured with C5's packer plugged in.

### C7 — The record ▲

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
