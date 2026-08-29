# Second-order effects, from the quest spec and the named pathfinders

Things milestones C6 and C7 turned up that are **not** C6 or C7's problem, and
that would be a mistake to fix in the middle of them. Each one is real, each one
has evidence in a test or a printer, and none of them blocks the mission.

Written down as they appear rather than at the end, because the useful detail is
what was happening when they surfaced. `analysis/second-order.md` is the base
game's version of this file; this one is specific to the two agents.

---

## 1. `chain.rs` proves the doors and not the road

`tests/chain.rs::the_chain_can_be_finished_in_one_run_in_either_mode` answers
THE LOCKED GATE at rung 26 and then sets `run.rung` **backwards** to 25 to meet
THE MANSE:

```rust
answer(&mut run, 25, "the-locked-gate", "Use the word");
run.rung = gearmaster_engine::town::by_id("the-manse").expect("authored").after;
```

Every station of the chain does open the next, which is what the test says it
proves and what it does prove. What no test in the suite asked until now is
whether the stations can be met **in the order a player meets them** - and for
a hidden town, whose gate is asked about on exactly one rung, that is a
different question with a different answer.

`tests/quest.rs` walks it forward and the chain survives, so this is not a bug
report. It is the observation that **a chain test that stands the run at each
door is measuring the table, and a chain test that lets the run arrive is
measuring the road**, and this repo had four of the first kind and none of the
second. Any future chain wants one of each.

Not fixed here because `chain.rs` is not wrong; it is answering the question it
says it is answering. The new file is the second question.

## 2. Three of the four reward tiers pay on a run that cannot finish

Measured, four rungs wide, `tests/quest.rs::a_run_past_the_deadline_still_answers_every_door_on_the_chain`.

A run that comes by `A Word About the Wrong Stars` on displayed rungs 26 to 29
is offered THE ASTRONOMER, hears him out, is offered THE LOCKED GATE, uses the
word, and puts THE MANSE on the map. The house stands after rung 25. The run
walks to HIGH WICK and the chain is over without anything saying so.

Under the tiers of `HANDOFF-two-agents.md` §3.6 that trajectory earns *door
offered* twice, *prerequisite held*, and *correct choice taken* twice, and
earns them honestly - every one of those things happened. It is the farm, and
it is four rungs of this road rather than a hypothetical.

The defence is in C6 (the tiers telescope to zero over a complete episode) and
the evidence is the trajectory pair in `crates/trades/tests/quest.rs`. What is
**not** fixed here is the game-side observation underneath it: the road gives a
player no signal at all that the chain has become unwinnable. The word is still
in the tray, the doors still stand, and the map still draws a town nobody can
reach. A player is in exactly the agent's position. That is a content decision
rather than a bug, and it belongs to whoever owns the chain.

## 3. Potential-based shaping leaks on truncation, and `qpack` leaks

`qpack.rs:409` zeroes the potential when the episode **terminates**:

```rust
let phi2 = if e.finished { 0.0 } else { potential(&c) };
```

`e.finished` is the agent pressing `Done`. An episode that ends by running out
of its press budget leaves the loop through `ms.is_empty()` at the top, and the
last transition pushed keeps its `γΦ(s_T)` - so a truncated episode banks the
shaping instead of giving it back, and the telescoping sum that makes
potential-based shaping provably policy-neutral is `γᵀΦ(s_T) − Φ(s_0)` rather
than `−Φ(s_0)`.

For the packer this is close to harmless: Φ counts items assembled, so the leak
pays for assembling items, which is the thing the reward wanted anyway. It is
worth writing down because **for a quest the same leak is exactly the farm** -
a farming trajectory is precisely the one that ends by running out of road with
three tiers ticked and nothing finished, which is to say by truncation.

Not fixed in `qpack` here: changing the packer's reward moves the `QPACK_PHI`
band, and the band is what C4 exists to re-derive. Fixing it now would land a
reward change in the middle of a milestone that is not about the packer, and
`HANDOFF-two-agents.md` §5.2 already names inheriting that number as the
likeliest silent mistake in the plan. The pathfinder's own shaping distinguishes
the two cases from the start and `crates/trades/tests/quest.rs` pins it.

## 4. `Goal::Town` compares a name and the other four compare ids

`env.rs:182` matches `Goal::Town(name)` against `v.town.name`, where every other
variant matches an id. The name is canonical today - `read.rs:194` puts
`Town::name` in the view and the theme touches only `blurb` - so nothing is
broken and no themed string is reaching game logic.

It is one edit away from being wrong, though, and it is the edit somebody would
make for good reasons: `View::Town` carries no `id`, so a town is the one thing
on the screen an agent identifies by what it is called. The four other `Goal`
variants would survive a theme that renamed everything and this one would not.

Left alone because adding `id` to `View::Town` is a console change in the middle
of a trades milestone, and because the quest steps built in C6 key on the same
field, so fixing one without the other would be worse than fixing neither.

## 5. A chain's finish can be off the screen, and one of the three named ones is

`design/HANDOFF-two-agents.md` §3.5 names three models. Two of them dress for
an agent and the third does not.

`pathfinder_drover` is aimed at a chain of THE HUNDRED being finished. The
engine records that as a flag - `county::chain_done(chain)`, checked by
`Run::county_chain_done` - and `View` carries no flags, deliberately: a flag is
bookkeeping and a player is shown no list of them. The GUI reads
`county_chain_done` in exactly one place, to decide whether to draw the drover's
marker, so a player learns the chain is finished by a glyph going away.

So the drover's chain derives correctly on the engine side and cannot be handed
across the boundary, and `lab::quests::quest` **refuses** it rather than dressing
it headless. A quest whose finish cannot be recognised is one an agent can never
be paid for finishing, so it would train on the cheap tiers and nothing else -
which is the failure the tiers exist to prevent, reached from the other side.

This is a console question rather than a training one and it is the owner's
call: either the county tab grows a line saying which of its three chains are
done - which a player arguably should be told anyway - or the drover model is
aimed at something that is already on the screen. `crates/lab/tests/quests.rs`
holds the refusal and names the day it stops being needed.

Not decided here because it is a change to what the game shows a person, and
this milestone is about what an agent is paid.

## 6. A flag cannot cross the boundary, and the fix is to walk one hop further back

The first version of the translation dropped every station marked by a flag,
which cost `pathfinder_unwound` its middle: `threshold-cleared` is what both
routes to the mainspring wait on, and dropping it left a three-stop chain with a
hole where the whole Manse detour should be.

Dropping was right and the derivation was wrong. A flag is not a thing a run
*has*; it is a note about something it **did**, and the thing it did is on the
screen. `threshold-cleared` is set by clearing THE THRESHOLD, and a dungeon is a
field of `View`. So `quest::resolve` now walks one hop further back at a flag
set by a dungeon's `also`, and the chain carries `Entered("the-threshold")`
beside the flag rather than instead of it.

The result is worth more than the fix: `pathfinder_unwound` is now a strict
superset of `pathfinder_threshold`, nine stops to seven, **and it is derived**.
Nobody wrote down that the road past Francis runs through the Manse's cellar.
Walking the tables backwards is what says it, and it is why §C7's choice of the
threshold as the first target is the right one for a reason stronger than the
one §C7 gives.

The same trick will not work for a flag set only by a *choice*, because taking a
choice leaves no mark on the screen except what it produced - which is the whole
of §3.6's outcome rule, met again one level down. Where a choice's flag has no
visible product, the station cannot cross, and the honest thing is #5's refusal.

## 7. Two suites in this repo cannot check the piece between them

The derivation is engine-side; the payment rule is in `gearmaster-trades`, which
cannot see the engine; the translation between them is in `gearmaster-lab`,
which is not one of the three suites the mission counts (`cargo test --workspace`
compiles it, but the counts quoted everywhere are engine, GUI and CLI).

So `crates/lab/tests/quests.rs` is real, green, and invisible to every number in
`CLAUDE.md` §5. It is run with `cargo test -p gearmaster-lab --test quests` and
nothing runs it by habit. That is not a fault of this milestone - it is where the
boundary put the code - but it is a test that can rot silently, which is the
shape this repo has been bitten by before (`cargo build` not compiling the GUI's
`cfg(test)` module, trap 14).

Left as it is because moving it means either giving the engine a dependency or
giving `trades` one, and both are worse. What is done instead is naming it here
and in `CLAUDE.md`'s run list.
