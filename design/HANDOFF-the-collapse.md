# Handoff — the quartermaster reaches rung 11 and then forgets how

Written against `9e9198b`. Suite: **engine 1075, GUI 90, CLI 12, trades 36,
agent 11, lab 11, 0 warnings.**

Read `CLAUDE.md` first, then `design/HANDOFF-two-agents.md` §8 for how the two
agents got here. This document is one open question and everything known about
it. It is deliberately narrow: **do not train anything new until the triage
below is done**, because four separate faults in this mission were each
mistaken for "it needs more episodes" and each cost hours.

---

## 1. The observation

`crates/lab/src/bin/qrow.rs` trains the quartermaster on **the row**: one
episode is one Rogue run, from rung one until it runs out of lives, and what
the episode is worth is how deep it got. 4,000 episodes, two hours two minutes.
The full log is `analysis/nets/qrow-r12.log`.

| episode | ε | deepest in block | spread |
|---:|---:|---:|---:|
| 2000 | 0.29 | 9 | 0.197 |
| 2300 | 0.18 | 5 | 0.203 |
| 2500 | 0.11 | 7 | 0.202 |
| **2600** | **0.07** | **11** | 0.210 |
| 2800 | 0.05 | 10 | 0.221 |
| 2900 | 0.05 | 9 | 0.231 |
| 3000 | 0.05 | 7 | 0.241 |
| 3200 | 0.05 | 8 | 0.244 |
| **3300** | 0.05 | **3** | **0.255** |
| 3400 | 0.05 | 3 | 0.250 |
| 3500 | 0.05 | 2 | 0.239 |
| 3700 | 0.05 | 3 | 0.241 |
| 3999 | 0.05 | 2 | 0.228 |

It climbed to **rung 11 with ε already at 0.07** - past the written control's
mean of 6.0 through the same loop, near its best of 13 - held between 7 and 10
for six hundred episodes at the exploration floor, and then fell to 2-3 and
stayed there for the last seven hundred.

**The interesting part is the spread column.** It kept *rising* through the
turn - 0.221 at the peak, **0.255 at the first bad block** - and only fell
after the behaviour had already gone. The values were separating further while
the policy got worse.

---

## 2. What that rules out, and it is most of what this mission has seen

Four faults were found and fixed in the sessions before this, and every one of
them presents as a **flat** network - a Q spread of hundredths against rewards
spread over units. `--bin qwhy` prints the values per action and `--bin qmind`
prints the weights against what they were initialised as; both are cheap and
both should be run first.

| already found, do not re-find | signature |
|---|---|
| the loss clipping the rewards out of the gradient (`CLAUDE.md` 53) | flat, spread ~0.003 |
| the road agent unable to tell its own verbs apart (50) | flat, one distinct action vector |
| a reward floor that paid for owning nothing | `clear` pressed 206 times in a 262-key run |
| a step charge a hundred times the objective | spread falling monotonically from the start |

**None of them fits.** The spread rose to 0.255 and the policy reached rung 11
before it went. This is a network that had learned something and then lost it,
which is a different animal from the four above and should be triaged as one.

---

## 3. The leading hypothesis, and it is only that

**Overestimation compounding through the bootstrap.** The target is
`r + γ·max_a Q(s',a)` with the *same* frozen net choosing and valuing the
action, so any upward error in `max` is selected for and fed back. Rising
spread with falling performance is the textbook signature, and three things
here make it more likely than usual:

* **γ = 0.999 over a whole run.** An episode is every packing at every rung, so
  a value has to integrate hundreds of decisions and errors have hundreds of
  steps to accumulate over.
* **`BOOTSTRAP_KEEP = 16`.** The max is taken over the sixteen candidates the
  *behaviour* policy liked at collection time (`qrow.rs:83`), not over all ~180.
  That biases the max **low**, which cuts the other way - but it also means the
  bootstrap set is chosen by an older policy, and what that does to the bias is
  not obvious and has not been measured.
* **The target net refreshes every 50 episodes** (`ep % 50 == 49`), which at
  24 updates an episode is 1,200 gradient steps between refreshes.

The standard answer is **Double-DQN**: select the action with the online net,
evaluate it with the frozen one. It is a few lines in the update loop.

**Do not just apply it.** `CLAUDE.md` trap 51 exists because a familiar failure
shape fitted the evidence perfectly twice in this mission and was wrong both
times. Measure first - §4 says how.

---

## 4. What to run, in this order

Everything here is minutes, not hours. **No training run is justified until
these are done.**

1. **Are the values overestimates?** For a sample of states, compare
   `max_a Q(s,a)` at the collapsed net against the **return actually achieved**
   from that state in a rollout. If Q says 40 and the run yields 4, that is the
   answer. There is no binary for this yet and it is the one worth writing:
   `--bin qmind` already walks states and reports values, so extend it rather
   than starting a new one.
2. **Did the weights blow up?** `cargo run --release -p gearmaster-lab --bin qmind`
   reports every layer against its initialisation. Compare
   `runs/quartermaster_row.txt` (the best block, rung 11) against
   `runs/quartermaster_row_last.txt` (the collapsed one) - both are written now
   and that is what they are for. A large `w3` or `b3` in the second is
   diverging values; similar weights with different behaviour is something else.
3. **What does it press?** `--bin qwhy` takes a road net today. The packing
   equivalent is what is missing, and the single cheapest diagnostic in this
   mission turned out to be a **key histogram** - 206 `clear`, 16 `buy`, 0
   `place` - which no instrument here produced and a person reading a proof
   found in a minute. Write it. If the collapsed policy has fallen onto one
   verb, that is a different fault again.
4. **Is it the bootstrap set?** Re-run 1,200 episodes with `QROW_UPDATES=24`
   and `BOOTSTRAP_KEEP` raised to the full menu (it is a `const`, so this is a
   recompile), and see whether the collapse moves or goes. Expensive - about
   eleven times the update cost, measured - so do it last and only if 1-3 are
   inconclusive.

---

## 5. Things that will waste your time

* **Training longer.** The collapse is *after* the peak; more episodes at the
  floor produced seven hundred more episodes of rung 2-3.
* **Reading the spread as health.** The working Grinder pathfinder reaches rung
  17.5 with a mean across-action spread of **0.008**, *smaller* than the broken
  one's 0.083. Spread answers "has this learned anything at all" and not "is it
  any good" - `analysis/the-two-trades.md` R6.4.
* **Assuming the loop is fine because the numbers moved.** `qrow` prints what
  the **written control** does through the same loop before taking a gradient:
  *mean rung 6.0, best 13*. If a change makes that number move, the change is
  about the harness and not the learning. This line exists because a simplified
  curriculum walker once read as a fact about Rogue and was a fact about a
  hundred lines of harness - rung 1 against the pilot's 18.
* **Grinder.** Do not spend the machine on it. It is designed to always be
  possible, so the constrained mode is the interesting one and the uninteresting
  one costs about fifteen times as much per episode.

---

## 6. Where things are

| what | where |
|---|---|
| the trainer | `crates/lab/src/bin/qrow.rs` |
| the loop - one episode is one run | `crates/lab/src/row.rs` |
| what a run is worth, and what an item is worth | `row::worth`, `row::assembly_bonus` |
| the board judge, and `reach` | `crates/lab/src/scoring.rs` |
| the features - board 270, move 32 | `crates/trades/src/feature.rs` |
| the weights, layer by layer | `--bin qmind` |
| what a road net values per action | `--bin qwhy` |
| the run this document is about | `analysis/nets/qrow-r12.log` |
| best block / final weights | `runs/quartermaster_row.txt`, `..._last.txt` |

The reward, in full, per press:

```
  finishing an item, on the press that finishes it   +1 and up to +2 for quality,
                                                      only on a new high for the run
  the last press of the run                          deepest^2 / 25  −  0.5 a life lost
  every other press                                   0
```

`γ = 0.999`, `lr = 0.05`, 24 updates an episode, Huber knee 120, buffer 80,000,
target net every 50 episodes.

---

## 7. One thing that is not the collapse and is worth knowing

The **pair** still fails. `--bin qcross` puts the learned road policy at rung
6.6 in Rogue against the written pilot's 10.0 and a floor of 1.0 - a real
result - and the two learned agents *together* at 1.0. A pair is worth what its
packer is, and that is this document's question. Fixing the collapse is the
thing standing between a working road policy and a working pair.
