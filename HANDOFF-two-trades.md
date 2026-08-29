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
| Q7 Generations | **in progress** |
| Q8 Themes | not started |
| Q9 The harvest, and the record | not started |

## How to run anything

    cargo test -p gearmaster-engine          # 1,059, the safety net, unchanged by this mission
    cargo test -p gearmaster-trades          # this mission's own
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
