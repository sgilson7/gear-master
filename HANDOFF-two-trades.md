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
| Q1 The representation | **in progress** |
| Q2 Two environments | not started |
| Q3 The quartermaster learns | not started |
| Q4 Pools, proven | not started |
| Q5 The pathfinder learns | not started |
| Q6 The validity solver | not started |
| Q7 Generations | not started |
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
