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
| Q3 The quartermaster learns | **GATE MISSED** - 8/50 against 48/50. Learning is real (spread 0.15->1.90) but the action factoring is wrong: rotate-then-place is a composite the agent has to discover with no undo. See analysis Q3.4 |
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

So the next suspect is not the features. In order of what the measurements
support:

1. **The exploration cannot find a completion.** Roughly five hundred legal
   placements a step, ε-greedy, forty steps, 2,500 episodes. A completing
   placement is perhaps one in five hundred at random, so the buffer holds a
   couple of hundred of them among a hundred thousand transitions. Prioritised
   replay keyed on `feature::mv`'s f[27] is the cheap test and it is honest -
   it changes which transitions are *sampled*, not which are rewarded.
2. **Φ is too quiet.** A completing placement is worth `+0.119` shaped against
   `-0.03` for any other change, and the Q values are spread over 1.5. Raising
   `0.15 * items` is one line and one run.
3. **A budget of forty presses may not be enough** to assemble from a tray the
   control needs a hundred for. `qcheck`'s control column spends 120-420.

None of these is a rewrite. All three are one constant or one sampler.

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
