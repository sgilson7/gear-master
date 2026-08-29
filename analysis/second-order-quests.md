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

## 8. A budget on decisions and a budget on presses are not the same number

`Step::Pack` hands the console to a packer with a budget, and the number that
had always been passed was **forty** - the learned packer's, where a press is a
decision and Q0 measured thirteen of them in a typical episode.

`hands::pack` is greedy over pieces and **exhaustive over seats**, so its cost
is the number of anchors it tries: Q0 measured the same control at a median of
492 presses an episode and 798 at the ninetieth percentile. Given forty it
bought four pieces, seated none of them, assembled nothing, and lost rung one
for ever - and in a trace that reads exactly like a road policy that has decided
packing is worthless.

An afternoon, and the fix is one constant. What makes it worth writing down is
that **nothing was wrong**: `hands::pack` respected its budget, the road agent
pressed `Pack`, the console did what it was told. Two correct components with
one shared word meaning two things.

The general shape is `CLAUDE.md` trap 43 - a predicate with two readers is two
predicates that happen to agree - in units rather than in meaning. Any number
crossing between the two agents wants asking what it counts.

## 9. Half the shopping list is a hole in the partition, not a design choice

§3.3 argues for the shopping list from the top down: *why* to barter for a word
is a fact about a door seven rungs away, the packer cannot see it, so the
planner decides and the driver executes.

Building it found the argument from the bottom up, and it is harder. `Buy` and
`Barter` are the quartermaster's verbs (`partition.rs`), and the agent that can
see why the word matters is not allowed to press the key, while the agent
allowed to press it cannot see why. **Neither agent can buy the word.** Without a
driver doing it programmatically the first stop of the first chain is
unreachable by construction, and no amount of training reaches it.

That is worth knowing because it generalises: any chain whose first stop is a
purchase has the same hole, and the partition is exhaustive and disjoint, so
there is no verb to move. §3.3's second option - "the driver performs the
purchase outside both agents' action spaces" - is not the simpler of two
choices. It is the only one.

## 10. A written control has to remember one thing, and that thing is not obvious

`lab::roads` is a road policy with a priority order, and the first version had
no state at all: answer a door, else visit a town, else fight. It pressed
`Fight` three hundred and twenty times an episode and finished on rung one.

A run that fights on the board it was dealt loses rung one; a Grinder cannot
slide below rung one; the same fight is offered again. "Pack when the tray is
not empty" does not fix it either - `hands::pack` leaves in the tray whatever
it would not seat, so the tray is rarely empty and the policy packs for ever
instead.

The question a written road policy has to be able to answer is **"has this rung
had its packing yet"**, which is one field and is the only state in the file. It
is the same shape as `CLAUDE.md` trap 23 - a road-walking helper has to know how
to throw a lever - one rung earlier: a road-walking helper has to know how to
put a board together before it fights.

## 11. The road agent has never had an action representation

Not this milestone's to fix, and the largest thing it found.

`feature::mv` describes a candidate move for the network. Its one-hot has eight
shapes - `Place`, `Buy`, `Sell`, `Barter`, `Reroll`, `Rotate`, `Unequip`,
`ClearSlot` - and `_ => 8` for everything else, and every field after the
one-hot is about a **piece**: how many cells, what it costs, what it does with
the pools, where it is being seated. Those are the quartermaster's questions and
they are the right ones for the quartermaster.

Every verb the pathfinder owns falls into the eighth bucket with no piece
behind it. Measured (`--bin qmoves`), over four runs:

    1,341 road verbs offered
    four verb kinds among them: answer, drink, fight, town
    distinct feature vectors the network can see: 1

The road network is choosing between `Pack`, which is the all-zero vector by
convention, and "a road verb". Which road verb is the order the console listed
them in.

**This is why the road half has never worked**, and it is a better answer than
the one that has been on record since Q5.1: *"the Q network is not what is
deciding"*, attributed there to Q3's miss and the packer being the written one.
The packer was not the reason. A5, A6, Q5, Q6 and now C7 have all measured a
road policy that cannot distinguish its own actions, and every one of them
reached about the same rung as the policy that presses the first thing on the
list - which is exactly what you would expect.

Fixing it is its own milestone and its shape is clear: describe a road verb the
way `mv` describes a placement. Which kind it is, separately rather than in one
bucket; for an `Answer`, which choice and what it requires and what its outcome
does; for a `Town`, which door; for a `ThrowPoints`, which exit. All of it is on
the screen - `View::Question::Choice` carries `label`, `requires` and `open`,
and `View::Town::doors` carries the door list - so none of it costs the
boundary anything.

Left undone here because it is a new representation rather than a constant, and
because landing it inside a milestone about quests would mean the quest work and
the feature work were measured together and neither could be attributed.
`crates/trades/tests/quest.rs::the_pathfinder_can_tell_this_many_of_its_own_verbs_apart`
holds the figure at 1 so the day it is fixed there is a number saying by how
much.

## 12. A diagnosis that fits is not the diagnosis

The trace showed the trained model pressing `Pack` 320 times out of 320. This
repo has seen that exact shape twice - `Rotate` 400 times out of 420, then `Pin`
410 out of 420 - and it has a trap for it, number 44: *a free action is a scale
problem, not a verb problem*, and the fix is to charge for what the board does.

That reading fits perfectly and it is wrong. The charge was already there
(`NOTHING_HAPPENED = 0.35`, read off the board rather than the verb, exactly as
trap 44 says to). What the trace could not show is that the alternative to
`Pack` was not `Fight` - it was a vector identical to every other road verb, so
the policy was not choosing `Pack` over `Fight` at all. It was choosing the one
action it could see against a bucket it could not see into.

The general form is worth keeping: **a known failure shape is a hypothesis, not
a finding.** The cheapest way to tell them apart was to measure the inputs
rather than tune the reward, and the measurement took twenty minutes where the
tuning would have taken forty and produced another null.
