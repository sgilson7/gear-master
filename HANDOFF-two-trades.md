# Handoff — THE TWO TRADES

**Branch `the-two-trades`, off `the-apprentice`.** Read `CLAUDE.md`, then
`design/the-two-trades.md` (the mission), then `design/the-apprentice.md`
(the predecessor, whose measurements are every gate). `analysis/the-two-trades.md`
is the running record: one block a milestone, headed by the commit it was read off.

This file exists so a fresh agent can pick the work up cold. It is updated at
every milestone commit and it is the difference between the plan and where the
code actually is.

## Where the work is

| | state |
|---|---|
| Q0 The ground, and the two menus | **done** - 16 verbs apiece, horizons 13 and 204 |
| Q1 The representation | **done** - probe balanced 85.7% vs 50%; pools in the View |
| Q2 Two environments | **done** - 1.40 ms an episode; 10^6 steps in 43 s |
| Q3 The quartermaster learns | **GATE MISSED** - 8/50, then **14/50** post-merge with `QPACK_PHI=1.5`, against 48/50. The action factoring was blamed and was not the cause; the shaping weight was. See analysis Q3.4 and the post-merge block |
| Q4 Pools, proven | **premise confirmed** - control matches a planted pool 1 in 12; 68% of production stranded. Comparison to the learned packer owed (Q3) |
| Q5 The pathfinder learns | **built, not trained** - the agent and its features exist; the frozen packer is the written one because Q3 missed |
| Q6 The validity solver | **partly met** - doors 79%, branches 68%, towns 5/6, **dungeons 5 of 7** (gate wanted 7). Two left, both with named causes |
| Q7 Generations | **GATE MISSED** - Q3's own fix applied (rotations out) and the policy moved onto `Pin`, 410 presses of 420. The fault is a scale: a no-op cost 0.01 against a Q spread of 1.70. Charge made generic (`NOTHING_HAPPENED`). Three runs, one curve. See analysis Q7 |
| Q8 Themes | **GATE MISSED** - the brief is built, checked and wired; it is worth fidelity on four of eight trained themes and **+0.000 / -0.043** on the two held out. Memorisation, not generalisation. See analysis Q8 |
| Q9 The harvest, and the record | **done** - four of five county creatures dressed and accepted, all five nearer the line than the boards they wear. Found that three of the five borrowed boards are off-curve. See analysis Q9 |

## The one thing to pick up first

Q3, Q7 and Q8 are one miss, not three: **the quartermaster does not learn to
pack.** Everything downstream of it was built anyway and works - the
pathfinder, the goal conditioning, the brief, the harvest - because each was
specified against a *frozen* packer and the written control is a legitimate
generation zero.

Q7.3 named the suspect and Q8's groundwork **removed it**: a placement's value
is whether it finishes an item, and `feature::mv` could not say so. It can now,
and the numbers are checked against the engine rather than argued for -
`cargo run --release -p gearmaster-lab --bin completes` reports **100% recall
at 87% precision**. The briefed arm trained after that change hit `items 1.1`,
the highest any run has produced, and still lands at 0.8.

Three suspects were named here and **all three have now been tested**, because
that is cheaper than arguing about them. Each is an environment variable on
`qpack`, so all three ran against the same code, seed and episode count.

| arm | items | won |
|---|---:|---:|
| baseline | 0.8 | 2/20 |
| `QPACK_PRIORITY=0.5` — oversample transitions that assembled | **0.6** | 0/20 |
| `QPACK_BUDGET=120` — three times the presses | **1.0** | 3/20 |
| `QPACK_PHI=0.6` — four times the shaping weight | **1.6** | 2/20 |
| **`QPACK_PHI=1.5`** — ten times | **2.8** | 3/20 |
| `QPACK_PHI=4.0` — twenty-seven times | 1.2 | 1/20 |

**Start from `QPACK_PHI` between 0.6 and 1.5.** Ten times the shaping weight is
three and a half times the items, the largest single move anything in this
mission produced. Above that the Q spread blows out to 5.4 and it gets worse -
the shaping starts to dominate the fight it was meant to be a hint about.

**Do not spend a run re-testing the other two.** Prioritised replay made it
*worse*: the buffer was not short of completions, it was short of a reason to
prefer them, and more copies of a signal the network cannot hear do not make it
louder. The press budget bought 0.2 items for three times the wall clock.

On `qcheck`'s real repack benchmark this is **8/50 -> 14/50**, six or seven
items assembled where Q7's arm assembled none, and `Place` the commonest verb
in every column for the first time in the mission. **Q3 still misses** - the
gate is 48/50 - by less than half of what it did.

The new pathology is legible: **199 unequips against 213 placements**. The
packer seats a piece and takes it straight back out, which is what a policy
does when placing is worth something and it has not learned *which seat*. That
is the next thing to look at, and it is a far more productive failure than
pressing `Pin`.

The diagnosis had been "a representation that cannot express the answer" since
Q7. That is no longer the best reading: the representation could express it and
the reward could not weight it.

## How to run anything

    cargo test -p gearmaster-engine          # 1,059, the safety net, unchanged by this mission
    cargo test -p gearmaster-trades          # this mission's own, 21
    cargo run --release -p gearmaster-lab --bin briefs    # what each theme asks for
    cargo run --release -p gearmaster-lab --bin completes # does the completion feature mean anything
    cargo run --release -p gearmaster-lab --bin asworn    # the five borrowed boards vs the gate
    DRESS_FOR="THE DROVER" cargo run --release -p gearmaster-lab --bin dress
    cargo run --release -p gearmaster-lab --bin q8        # briefed vs unconditioned
    cargo run --release -p gearmaster-lab --bin play      # the hand-written control
    cargo run --release -p gearmaster-lab --bin repack    # the packing benchmark
    GEARMASTER_WATCH=<proof> cargo run -p gearmaster-gui  # watch a run

## The rules that are not negotiable

1. **`crates/trades` depends on `gearmaster-console` and nothing else** from
   this workspace. Training is privileged and lives in `lab`; acting is not.
   `trades/tests/boundary.rs` is the ratchet.
2. **A commit a milestone**, with the gate's numbers in the message, pushed.
   A milestone that fails its gate is still committed, with the failure in the
   message.
3. **One merge, at Q9**, and it carries the wasm publish.
4. Never loosen a gate to make it pass. Re-pin with the reason, or record the
   failure.
