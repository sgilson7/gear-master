# THE APPRENTICE — an agent that learns this game from outside it
## Execution spec

Written against `020bc7c` (2026-08-28), the tip that published THE HUNDRED.
Successor to `design/rl-agent-plan.md` ("The Solver") and to
`HANDOFF-solver.md`, both of which remain worth reading: the Solver's §3
action encoding, §8 budget shape and §10 failure modes are kept whole, and
this document says where it departs from them and why. Like every document in
`design/`: **code follows this document; when they disagree, this is the bug
report** - except where it records what the code *does today*, which was read
off `020bc7c` and cited by line, and there the code is the news.

**What this is for.** Three things the repo cannot do, in the order they are
worth:

1. **Prove that the game is walkable.** Nothing in this repo has ever played
   a run. `force_win` and `skip_to` assign outcomes; `completable.rs` proves a
   key *can* exist before its door shuts; `apply_preset` seats twenty-two
   pieces somebody chose by hand. No test demonstrates that a build a seed's
   own shop can produce fights its way to any door, and therefore no test can
   say whether every event in this game is reachable by a person.
2. **Author creature boards.** `pack_francis` is the tool and it is a sampler:
   three hundred independent draws, best kept, no local search anywhere
   (`pack_francis.rs:783`, `for trial in 0..trials()`). It costs 39.5 s a
   creature and lands in band by luck. THE HUNDRED's five county creatures
   wear boards **borrowed** from ladder creatures because hand-packing was
   moved past the deploy, and three of the five run past sudden death
   (`design/HANDOFF-hundred.md` §5). That is the open work this mission is
   meant to close.
3. **Say what a theme is worth.** `MonsterTheme::allows` (`bestiary.rs:149`)
   filters the *pool* a packer draws from. Nothing anywhere asks whether the
   resulting **fight** reads as the theme. A Burner that kills with blows and
   a Striker that kills on the clock both pass every test in the repo.

**What this is not.** Not a player-facing feature. Not an engine rule change:
§2 lists the two derives and zero rules it wants. Not a training run in CI,
ever. Not a generative model anywhere in the loop - no language model reads
this game, writes a board, or picks a door.

---

# 0. What this changes about The Solver

`design/rl-agent-plan.md` is a good plan for a *search over the engine*. The
owner's brief is narrower in one way and wider in four, and the narrowing is
the one that reorganises everything.

| | The Solver | THE APPRENTICE |
|---|---|---|
| **Access** | The agent calls the engine: `can_equip`, `legal_anchors`, `choice_open`, `simulate_party`. The forbidden list (`skip_to`, `force_win`, fight-and-rewind) is enforced by discipline in evaluation | The agent **cannot name the engine**. It links one crate that offers the player's menu and the player's screen and nothing else. The forbidden list is enforced by the dependency graph, and `cargo tree` is the proof |
| **Learning** | M5, and only if the search plateaus | A deliverable. The search is the expert the net learns from, not the alternative to it |
| **Headline metric** | SCR - seeds cleared to a target | **DCR** - what fraction of this game's doors a forward, player-legal play has actually stood in front of, answered, and branched through. SCR is kept and demoted to second |
| **The packer** | M2, a benchmark against `pack_francis`; M6, a by-product | A product. The bar is `pack_francis`'s 39.5 s a creature and its band-hit rate, and both are meant to be beaten by an order of magnitude |
| **Theme** | Not addressed; `pack_francis` filters its pool by theme and stops there | A first-class input: it gates the pool, it scores the **log**, and it conditions the policy. One net, ten themes, two held out to see whether it generalises |

Kept from the Solver, unamended: the factored action decomposition (piece,
then anchor-and-rotation), the oracle memoisation key, the proof-is-a-replay
doctrine, the board-clear versus game-clear split, the artifact policy, and
the rule that an artifact enters the engine as data or not at all.

**One thing that mission's documents can no longer be trusted about: line
numbers.** `run.rs` has grown 1,250 lines since `HANDOFF-solver.md` was
written, and every citation in `CLAUDE.md` §3, the Solver and the handoff is
now short by hundreds. Read at `020bc7c`:

| Name | The handoff says | It is at |
|---|---|---|
| `pub struct Run` (still **no** `Clone`) | `run.rs:465` | **`:611`** |
| `Run::new` | `:700` | **`:991`** |
| `Run::start` | `:787` | **`:1100`** |
| `Run::road_stack` | `:912` | **`:1268`** |
| `Run::choice_open` | `:1097` | **`:1540`** |
| `Run::holds` | `:1845` | **`:3285`** |
| `Run::price` / `payment_for` | `:1859, :1890` | **`:3299, :3330`** |
| `Run::settle` | `:2043` | **`:3483`** |
| `Run::force_win` / `skip_to` | `:2434, :2440` | **`:4065, :4071`** |
| `Run::can_equip` | `:3127` | **`:4758`** |
| `Run::apply_preset` | `:3224` | **`:4871`** |
| `Run::fight_next` | `:3437` | **`:5084`** |
| `simulate_party` | `combat.rs:3862` | **`combat.rs:4217`** |
| `recipes` | `piece.rs:1039` | **`piece.rs:1238`** |

Correct as written: `combat.rs:40` (`SUDDEN_DEATH_MS`), `slot.rs:5,:8,:289`,
`share.rs:218`. New and load-bearing here: `Figures` (`loadout.rs:185`),
`Figures::of` (`:222`), `Toll` (`county.rs:161`), `Run::county_figures`
(`run.rs:2486`), `Run::undo` / `undoable` (`run.rs:4666, :4679`),
`UNDO_DEPTH = 40` (`run.rs:193`), `INVENTORY_CAP = 12` (`:201`),
`ROGUE_LIVES = 4` (`:160`), `MonsterTheme` (`bestiary.rs:37`), `slots`
(`:119`), `allows` (`:149`), `SWARM_BLOW = 25` (`:315`), `theme_for` (`:349`),
`tally_items` (`combat.rs:7725`).

**A0's first job is to stop quoting and start measuring.** Every number in
this document that is not marked as read off `020bc7c` is an estimate.

---

# 1. The three products, and their metrics

Each metric is defined so that **today's value is zero by construction**,
because that is the truth.

## 1.1 DCR - Door Coverage Rate

Over the evaluation seed set, the fraction of the game's content that a
*forward, player-legal* play has reached. Three columns, not one, because the
cheapest way to satisfy a coverage metric is to stand in front of things:

- **Offered.** The item appeared on the road stack, the shelf, the town menu
  or the county tile - the agent was in a position to take it.
- **Answered.** The agent took it: a choice made, a door entered, a tile
  crossed, a class drunk.
- **Branched.** *Every* choice on that item has been answered by some seed,
  in some run, in the whole evaluation. This is the column that says the
  content is reachable rather than merely present.

The denominator is a census taken at A0 and written out in full. It is
roughly: 44 `EVENTS` and 9 `COUNTY_EVENTS` with their choices
(`event.rs:642, :959`), 6 brawls, 6 towns × 17 doors (`town.rs`), 7 dungeons
and every floor of each (`dungeon.rs`, and floors are a **graph** - trap 22),
7 pedestal destinations (`pedestal.rs:62`), 3 county chains, 6 toll kinds ×
12 thresholds (`county.rs:161`), 11 rumours, 31 classes, 50 ladder rungs, the
51st, and the three bosses.

The product is `analysis/coverage.md`: the ledger, plus a named list of every
item in the *offered but never answered* and *answered but never branched*
sets, each with the reason the agent gives - **never offered** (no seed put it
on the road), **offered and refused** (a requirement never met), **answered
and unpaid** (the outcome never landed). Those three reasons are three
different bug reports and the ledger must not blur them.

## 1.2 PQ - Packer Quality

For creature boards, against `pack_francis`'s own acceptance gate ported
rather than reinvented (`target_ms` band, `BAND = 0.30`, `FLOOR_MS = 2_000`,
the four reference boards at four difficulties):

- **Creatures in band**, out of a stratified twenty-five.
- **Seconds a creature**, at equal quality. The bar is **39.5 s** on a
  container core (`design/post-unwinding.md` §5), to be re-taken on this
  machine at A0 - that re-take is the honest baseline and the 39.5 is a
  quote.
- **Theme fidelity** (§6), which nothing measures today.
- **Repack-from-tray**: the owner's 75 pieces, decoded from `A_WINNING_RUN`
  and stripped of their placements, repacked by the agent and walked up the
  ladder. The human's number is 48/50. A packer that cannot recover what a
  person did with the same pieces has no business packing different ones.

## 1.3 SCR - Seed-Clear Rate

Kept from the Solver §1 exactly, including its targets (R10, R25, FRANCIS,
CHAIN, R51, NOWEAPON, FRANCIS@d) and its board-clear versus game-clear split:
a fight is a **board clear** if `Victory && duration_ms < SUDDEN_DEATH_MS`
(`combat.rs:40`), and a **game clear** if it is a `Victory` at all. A fight
past 30 s was decided by the clock, and the two are counted in separate
columns everywhere. R51 and CHAIN are reported in both modes; Rogue carries
**four** lives (`run.rs:160`), not three.

| Metric | At `020bc7c` |
|---|---|
| DCR, all three columns | **0** - nothing plays a run |
| SCR, every target and mode | **0** |
| PQ, seconds a creature | 39.5 s (quoted, one container core, no local search) |
| PQ, theme fidelity | **unmeasurable** - no meter exists |
| Repack-from-tray | unmeasured; the human packed 48/50 |
| County creatures packed rather than borrowed | **0 of 5** |

---

# 2. The boundary: what "no access to the code base" is made of

The brief's central constraint is that the agent plays like a person: headless,
but exposed to exactly the menu and the screen a person is exposed to, with no
reach into the tables underneath. Discipline does not enforce that. **The
dependency graph does.**

```
crates/engine    gearmaster-engine    (unchanged; zero dependencies, for ever)
        |
        +-- crates/console   gearmaster-console   the player's surface
        |         Verb, Menu, View, Screen, Console.  Re-exports leaf enums
        |         only (SlotKind, PieceKind, MonsterTheme, CurseKind,
        |         DamageType, Difficulty, Mode).  Never re-exports Run,
        |         CATALOG, LADDER, MonsterSpec, simulate_party, Figures.
        |
        +-- crates/oracle    gearmaster-oracle    privileged, developer-side
                  Fights arbitrary boards.  Reads CATALOG, LADDER, ALTERNATES,
                  rating, Figures, MonsterTheme::allows.  Memoised, parallel,
                  incremental.  The theme meter lives here.

crates/agent     gearmaster-agent
        depends on: gearmaster-console.  Nothing else from this workspace.
        The blind pilot: the policy, the encoder, the search over the menu.

crates/lab       gearmaster-lab
        depends on: agent, console, oracle, and burn behind `--features nn`.
        The trainer, the packer, the evaluator, and every binary.
```

**The proof is one command.** `cargo tree -p gearmaster-agent` names
`gearmaster-console` and, beneath it, the engine - and does **not** name
`gearmaster-oracle`. Rust's 2018 name resolution does the rest: a crate that
is not in `Cargo.toml` cannot be named in a `use`, so the pilot cannot call
`simulate_party`, cannot read `CATALOG`, cannot ask `theme_for` what is
coming, and cannot `skip_to`. It is not that it must not. It is that it
cannot, and `tests/boundary.rs` asserts the crate graph so that a future
convenience cannot quietly add the dependency.

**Privileged critic, blind actor.** The trainer may use the oracle - to score
the pilot's boards, to shape reward, to build a value target. That is normal
practice and it is honest as long as the actor cannot consult it at inference.
So: `lab` trains, `agent` plays, and the thing evaluated is `agent` alone.
A0 writes down which signals were privileged in training; A6's gate is
measured with the oracle switched off.

**What a player may legitimately know.** Three lines, and they are the design:

- **Anything on the screen** is fair - it is in the `View`.
- **Anything the agent learned by playing** is fair, across episodes and
  across seeds. A person who plays seed 17 twenty times knows its shops;
  a person who has fought a Burner knows what a Burner does. Cross-episode
  *memory* is learning.
- **Anything read out of the tables** is not, ever. Cross-episode *peeking*
  is cheating, and the crate graph makes it impossible rather than forbidden.

**Replaying a seed is allowed in training and not in evaluation.** A player
can restart. The evaluation is first sight: one forward play, on a held-out
seed the agent has never been trained on, no restarts, no forks, no rewinds.
That is a cleaner answer than the Solver's D4 debate about fight-and-rewind,
and it does not need a rule about undo - because `Run::undo` (`run.rs:4666`,
depth 40, Loadout phase only, restores loadout, registry, owned and gold) is a
**button in the game**. The player's own board editor is already a search
environment: place, read the figures, undo, place something else. The agent
gets that too, because a person gets it.

**Two engine changes, both derives, neither a rule.**

1. `#[derive(Clone)]` on `Run` (`run.rs:611`; every field is already `Clone`).
   Used by privileged code only - the packer's local search and the trainer's
   checkpointing. In its own commit, with the reason in the message.
2. `#[derive(Clone)]`, if missing, on whatever the `View` must copy out.
   Expected: nothing; the `View` is built from accessors.

If any milestone finds it needs a third, that is the Solver's failure mode 8:
stop, write the proposal into `analysis/the-apprentice.md`, and ask.

---

# 3. The player's surface, enumerated

**The GUI is the reference surface.** `crates/gui/src/main.rs` (15,148 lines)
calls **90 distinct `Run` methods**. The CLI is a subset and is eleven verbs
short of the game: it has no `reroll`, `barter`, `melt`, `crush`, `grow`,
`lock`, `undo`, `give`, passenger seating, relic payment or shelf pinning.

So A1 has two halves that pay for each other:

- `Verb` is the union of what the GUI's buttons do.
- **The CLI gains the missing eleven**, which makes every `Verb` typeable, and
  makes a proof a transcript a person can paste into `cargo run -p
  gearmaster-cli` and watch. `crates/cli/tests/replay.rs` already pipes a
  script in twice and byte-compares; that harness becomes the proof checker
  and needs no new machinery.

Draft verb table, grouped as the screen groups them. A1 owns the authoritative
enumeration; this is what the reading found.

| Group | Verbs |
|---|---|
| Board | `Place {piece, slot, x, y, rot}` · `Unequip` · `Rotate` · `Lock` (`toggle_lock_item`) · `PlaceLocked` / `RotateLocked` / `UnequipLocked` · `ClearSlot` · `ClearAll` · `Undo` · `Grow {slot}` |
| Shop | `Buy {shelf}` · `Sell {piece}` · `Barter {shelf, paying}` · `Reroll` · `Pin {shelf}` (`run.shop.toggle_lock`, `shop.rs:369`) |
| Road | `Answer {choice}` · `AnswerWith {choice, figure}` · `TakeReceipt` · `Fight` · `FightParty` · `Settle` · `BackToLoadout` |
| Town | `Town {door}` (17 kinds) · `WalkOn` (`skip_town`) |
| Dungeon | `Enter` · `EnterAt {floor}` · `ThrowPoints` · `Leave` |
| County | `EnterCounty` · `Walk {n\|s\|e\|w}` · `LeaveCounty` · `Perambulate` |
| Fountain | `Drink` · `DrinkChoosing {class}` · `Double {class}` |
| Workshop | `Melt {piece}` · `Crush {piece}` · `Pedestal {piece}` · `Give` · `SeatPassenger` · `DeliverPassenger` · `RelicPay` |

**The cheat list**, which is the other half of the lint (trap 19 - half a lint
is not a lint). These are `Run` methods the GUI calls from its debug menu or
its tests and which no `Verb` may ever wrap: `force_win`, `skip_to`,
`with_all_pieces`, `apply_preset`, `wipe`, `skip_fight`, `grant_life`,
`grant_quest`, `begin_fight`, `stock_exactly`, `set_theme`. A1's lint asserts
`Verb`-reachable ∪ cheats = the GUI's mutator set, **both directions**, by
`include_str!`ing `crates/gui/src/main.rs` and extracting `run.<name>(` - the
same shape as `assembly_bonuses::which_pools_a_board_can_actually_make`
(trap 30), and idiomatic here: four tests already `include_str!` across crate
boundaries.

`Console::menu()` returns the legal `Verb`s at the current state, enumerated
the way the screen enumerates them, and the engine's `Result` is the truth: an
enumeration the engine refuses is a bug in the console, and `tests/legality.rs`
fuzzes a thousand reachable states to keep the two equal in both directions.

---

# 4. What the agent may see

`View` is the screen, in fields. **Every field must name the GUI element it is
read off**, in a table in `console/src/view.rs`'s doc comment, and the lint
asserts the console calls only accessors that appear in the GUI's source.

Sketch, by panel:

| Panel | Fields |
|---|---|
| Grids | five 6×`rows` cell maps: occupied-by, piece kind, assembled, locked, enchanted-under; owed rows |
| Tray | up to 12 pieces: name, kind, slot, cells, `Stats`, trigger `describe()` strings, price, rarity |
| Stats | `player_stats()` and `parts()` - the glyphs and figures the tooltip draws |
| Items | `combat_items()`: what assembled, its cooldown, what it does |
| Figures | the county's six (`Figures`, `loadout.rs:185`) - flow, two fords, scarp, drift, hedge |
| Shop | six shelves: piece, price, pinned; gold; `reroll_cost()` |
| Road | the road stack's head and kind; the standing event's title, body and choices with `choice_open` for each; the town's doors and their receipts; the fountain's offer |
| Next | the creature's **name and health**, its curse if shown, and nothing else |
| County | the tab: tiles walked, thresholds known (`county_threshold_known` - one tile away and not before), trips left, the clock, the checklist as far as read |
| Run | rung (displayed, `at + 1` - traps 9 and 20), gold, lives, mode, difficulty, classes held, flags **as the screen names them**, quest progress, the last `CombatLog` |
| Memory | what this agent has learned across episodes: per-creature outcomes it has fought, per-piece outcomes it has used. Not an engine read |

**The leak audit was A1's gate, and it answered the other way.**
This section used to say `Run::monster()` was a hazard: it returns a whole
`MonsterSpec`, gear included, and a player standing on rung 30 supposedly does
not see rung 30's board before the fight. **They do.**
`gui/src/main.rs:4768` draws "WHAT THEY BRING" - every item the creature will
swing - and `:4803` draws its whole board, under a comment saying the panel is
a preview that exists so you can shop against what is coming and that showing
half of it would defeat the point. The `View` carries all of it.

What it withholds is the rest of the ladder, and there the two interfaces
disagree: the CLI's `ladder` prints every creature's outfit at every rung and
the window shows only the next one. The console takes the window's answer,
because telling an agent *less* than a player knows can only make a
reachability claim stronger. `console/tests/view.rs` holds both halves.

---

# 5. The oracle, and why the current packer is slow

`pack_francis` draws three hundred boards from a fixed distribution, pays the
full acceptance gate on each - four reference boards at four difficulties,
sixteen fights - and keeps the best. There is no local search, no restart from
the best, no adaptation of the distribution. 39.5 s divided by 300 is about
130 ms a trial, and the gate is nearly all of it.

Two changes, and they are separable:

**Tier the objective.** Three scores, cheapest first, and each one filters for
the next:

| Tier | What | Cost | Player-legal? |
|---|---|---|---|
| **S0** | `Figures::of(&combat_items())` plus `player_stats`, assembled-item count and summed `piece_rating`, weighted by `Stats::parts_when` - see below | **42 ns** (A0) | **yes** - the county tab draws it |
| **S1** | One fight against one spec at one difficulty | 0.03 ms at rung 1, 1.4 ms at rung 50 (quoted; re-take at A0) | no |
| **S2** | The ported `pack_francis` gate: four boards × four settings | ~15 ms | no |

The inner loop runs S0 and never touches a fight. S1 scores the top slice.
S2 is paid only on acceptance. A candidate whose flat damage a second is a
third of the incumbent's does not need a fight to be rejected, and today it
gets sixteen.

**Improve rather than resample.** Local search from the best board found -
remove-and-reseat one item, swap two across slots, rotate, replace a loose
piece - with the oracle memoised on the board hash `(def, slot, x, y, rot,
locked)` sorted, which is `share.rs:218`'s own tuple. NRPA (Rosin 2011)
adapts a tabular playout policy over `(piece, slot, anchor, rot)` toward the
best sequence found and needs no gradient, no framework and no GPU; CEM over
the sampler's tie-breaking noise is the cheaper comparison. Both are in
`rl-research.md` §2 and both are a few hundred lines.

**S0 knows which figures are rates.** `Stats::parts_when` (`stats.rs:370`,
landed by T3 after this document was written) classifies every one of the
twenty stat fields as `Passive`, `OnActivation` or `Damage`: eight of them are
handed over *on every activation* and the rest are true while the piece is
worn. A surrogate that sums a stat block without that split prices `+2 nature`
on a 2.8-second item and `+175 hp` as the same kind of number, which on a
thirty-second fight they are not remotely. **S0 weights the activation group by
the item's cadence and the passive group once**, and the classification is the
engine's own rather than a table this crate keeps up to date - which is the
whole reason T3 built it against the fight instead of by hand.

The same commit fixed a fault in the figures themselves: `Figures::of` reads
`stats.mana` and nothing else, and eighteen pieces were granting mana through a
trigger instead, invisible to every toll asking how much mana a second a board
makes. The preset crosses six of the twelve thresholds now rather than five.
S0 therefore measures more of what a board does than it did when this section
was written.

**Incremental evaluation.** `combat_items()` costs 0.271 ms on a 19-item board
and a move touches one slot. Cache per-slot `ItemProfile` lists and rebuild
one. This alone is most of an order of magnitude in the inner loop.

**The target**, to be stated as a number at A3: **under 5 s a creature at
equal band-hit rate**, on eight performance cores, and a band-hit rate that is
better rather than equal. If A3 cannot beat the sampler at equal wall-clock,
that is the Solver's failure mode 5 and it is a finding: the space is smaller
than it looks and the right investment is a better sampler.

---

# 6. Theme, as an input to learning and to scoring

The brief asks for an agent that takes a theme and lets it change how it
learns and behaves. The engine already has the noun: `MonsterTheme`
(`bestiary.rs:37`), ten of them, each with the grids it fills (`slots`,
`:119`) and the vocabulary it speaks (`allows`, `:149`). Three uses, and only
the first exists today.

## 6.1 The pool gate - exists

`allows` says whether a piece speaks the theme's language, and `plain`
(`:328`) lets cores and filler through everywhere. `slots` says which two or
three grids the creature fills. The packer inherits both unchanged.

## 6.2 The fidelity meter - new, and cheap, and useful on its own

A theme is a claim about **the fight**, and the repo checks it against the
*pool*. A Burner packed out of burning words that kills with one big blow
passes every test there is. So: ten signatures, each computed from a
`CombatLog` - the entries, the durations, `tally_items` (`combat.rs:7725`) -
and each a number between zero and one.

| Theme | Reads as | Signature over the log |
|---|---|---|
| Striker | fast and fragile | short TTK; damage concentrated in weapon and gloves; few, large blows |
| Wall | slow, heavy, hits back | long fight; high share of incoming absorbed by armour; `Reflected` entries present |
| Burner | kills on the clock | majority of damage dealt from `Searing` ticks rather than `Hit` |
| Slower | denies tempo | the player's activation count falls measurably against the same board unslowed |
| Drainer | starves a banked build | pool drained above a floor; mind damage present |
| Caster | bursty, mana-gated | magic share high; activations clustered rather than even |
| Hollow | takes the maximum away | maximum-health reduction present, and **no damage share at all** - the theme's own doc says its damage never appears in one |
| Swarm | everywhere, briefly | activation count high and mean blow ≤ `SWARM_BLOW` (25, `:315`) |
| Beast | no trick | physical share dominant; no curses, no drains |
| Warden | out-waits you | long fight; curses applied above a floor; damage low |

Two deliverables fall straight out, both at A2, both before any learning:

1. **A printer** that scores every one of the 78 creatures against its own
   theme and prints the table. The expectation is that several do not read as
   what they claim, and that list is a balance finding the repo has never been
   able to produce.
2. **A term in the packer's objective**: `score = acceptance + λ · fidelity`.
   λ = 0 recovers today's packer exactly, which is what makes the comparison
   at A3 fair.

**The trap, stated before it is shipped** (trap 21 - a lint satisfied by the
wrong thing): each signature must be a *ratio over the fight*, never a
property of the gear. "Carries a Searing curse" is satisfied by seating one
piece; "more than half the damage arrived as Searing ticks" is not. Write
every signature against the log or do not write it.

## 6.3 The conditioning vector - what makes the agent take a theme

The policy's input carries a **doctrine**, and it is what the brief asks for:

```
Doctrine {
    theme: Option<MonsterTheme>,   // ten one-hot, or none
    fidelity: f32,                 // how much the theme is worth against winning
    coverage: f32,                 // door-seeking against rung-seeking
    difficulty, mode,
}
```

Sampled during training, set at inference. One net serves all ten themes and
both jobs, and the dial is a parameter rather than a retrain. For the packer
the theme is the creature's; for the pilot it is a playstyle - and the pilot
also *observes* the coming creature's theme once it has met one, because a
person who has fought a Burner knows what a Burner does.

**A8 makes this falsifiable**: train on eight themes, hold two out, and
measure whether the held-out two are packed better than by the λ = 0 packer.
A conditioning vector that does not generalise is a lookup table with extra
steps, and the test says which one was built.

---

# 7. The learning design

The brief wants an agent that learns. The research document's honest
conclusion is that on a deterministic, exactly-scored, cheap-simulator problem
the search often wins. Both are satisfied by the same architecture, and it is
not a compromise: **the search is the expert and the net learns from it.**

**Expert iteration**, in three parts:

1. **The expert.** The A3/A4 search - greedy recipe packing, NRPA/CEM local
   search, a beam over the menu at the run level. Slow, strong, and it
   produces labelled decisions: at this `View`, with this `Doctrine`, the
   search chose this action and the episode reached this rung.
2. **The student.** Policy and value heads over the `View`. The policy is
   **factored** exactly as the Solver §3 has it: first *what kind of move*,
   then *which piece* (≤ 12), then *which anchor and rotation* (≤ 384),
   masked by `Console::menu()`. The value head predicts the episode's outcome
   under Ranked Reward (Laterre 2018): the target is "did this beat my own
   recent percentile", which manufactures a curriculum for a single-player
   problem that has no opponent to self-play against.
3. **The loop.** The student's policy becomes the prior inside the search
   (PUCT, or NRPA's initial weights), which makes the expert faster, which
   produces better labels. The gate at each turn is wall-clock: net-guided
   search must beat unguided search at equal seconds, or the net is recorded
   and switched off.

**Why not PPO from scratch.** It would work eventually and it wastes the
asset. The environment is deterministic, the reward is exact and free, an
expert exists, and the horizon per decision is short. PPO's strength - credit
assignment through a stochastic, expensive environment - is the one thing this
problem does not need. It stays on the shelf for the run-level layer if the
beam turns out to be the weak part, which A7 will say.

**Reward shaping**, because a terminal win/loss over fifty rungs is a needle:

- Per placement: Δ S0 - the change in the player-visible figures. Dense,
  free, and player-legal.
- Per fight: outcome, then margin - `duration_ms` against `SUDDEN_DEATH_MS`
  on a win, health left on a loss. A win at 29 s and a win at 9 s must not
  score the same, or the landscape is flat where it matters most.
- Per rung: +1, with the gold curve as a shaped term.
- Per **first sight**: a novelty bonus the first time a run stands in front of
  a door, enters a town door, lands on a dungeon floor or crosses a county
  tile. This is what makes the agent a validity explorer rather than a
  win-maximiser, and its weight is `Doctrine::coverage`.

**Curriculum.** Seeds ordered by how far the greedy baseline got, easiest
first. Rungs by `skip_to` **in training only** - and note that this needs no
rule, because `skip_to` is not a `Verb` and the pilot cannot reach it. The
trainer can, because the trainer is privileged. Evaluation constructs its own
runs and never skips.

**The framework, on this machine.** An M2 Max: 12 cores (8 performance, 4
efficiency), 32 GB unified, Metal. Burn with `Autodiff<NdArray>` first,
`Autodiff<Wgpu>` (which auto-detects Metal) measured against it on the actual
net at A6. The net is small - the `View` is on the order of 13,000 floats and
the heads are a few hundred outputs - and for nets this size kernel-launch
overhead often makes the CPU backend faster on Apple silicon. **Measure, then
choose, and write the measurement down.** Candle is the fallback here rather
than tch: it has a first-class Metal path and needs no LibTorch. Nothing
Python, ever.

---

# 8. Milestones

Ten. Each ends green, with its numbers in `analysis/the-apprentice.md` beside
the commit hash they were read off. Two ordering rules, both inherited: **no
framework dependency before A6's gate opens**, and **no artifact enters the
engine except as data**.

| | Milestone | Gate |
|---|---|---|
| A0 | The ground, on this machine | every number re-taken, with a hash |
| A1 | The console, and the CLI made whole | the parity lints pass in both directions; a transcript round-trips |
| A2 | The oracle and the theme meter | 78 creatures scored; cached == recomputed |
| A3 | The packer | beats `pack_francis` on seconds **and** band |
| A4 | The pilot | SCR and DCR first non-zero, with transcripts |
| A5 | Coverage as an objective | the ledger, with every gap named and classified |
| A6 | The prior | net-guided beats unguided at equal wall-clock, or it is written down and stopped |
| A7 | The loop | held-out SCR/DCR/PQ above A4/A3 |
| A8 | The theme dial | held-out themes packed better than λ = 0 |
| A9 | In service | five county boards packed; the ledger published; `make` targets |

## A0 - The ground, on this machine

No agent code. Re-take every number the plan is priced in, on this M2 Max,
in release, on mains power:

- A fight at rung 1, rung 25, rung 50. `combat_items()` on a 19-item board.
  A whole ladder for one board. Candidates a second on one P-core and on
  eight.
- `pack_francis` on Cog Priest, release, 300 trials - the real bar, replacing
  the quoted 39.5 s.
- The content census: the DCR denominator, counted and written out.
- The seed set (§10), written out in full so nobody regenerates it.
- The suite as it stands: `cargo test -p gearmaster-engine`, `-p
  gearmaster-gui`, `-p gearmaster-cli`, and the workspace warning count.

**Apple-silicon measurement hygiene, and it is not optional.** Four cores of
this machine are efficiency cores and they are three to four times slower.
`rayon` will schedule on them, so a "per-core" number is meaningless unless the
pool is pinned: measure with `RAYON_NUM_THREADS=8` for throughput numbers and
report the 12-thread number separately as the real one. Never compare a number
taken on battery with one taken on mains. Report medians over a fixed
workload, and re-take anything that took longer than a minute to be sure
thermals did not eat it. Build with `-C target-cpu=native`.

**Gate:** `analysis/the-apprentice.md` exists, every number has a commit hash
and a thread count, and the fight throughput is within 5× of the container's.
If it is not, find out why before writing anything - the whole plan is priced
in fights a second.

## A1 - The console, and the CLI made whole

`crates/console`: `Verb`, `Console::menu()`, `Console::do_()`,
`Console::view()`, `Console::screen()`. The `View` table, field by field,
against the GUI element each is read off. The leak audit on `Run::monster()`.
The CLI gains the eleven missing verbs and `replay.rs` grows a script that
uses them.

**Tests:** `boundary` (the crate graph: `agent` names neither `oracle` nor
`engine`) · `parity` (every `Verb` has a CLI spelling and a GUI affordance;
every GUI mutator is a `Verb` or a named cheat - **both directions**) ·
`legality` (a thousand fuzzed reachable states, enumerate-versus-engine, both
directions) · `transcript` (a play round-trips to text and back and replays
byte-identically, in-process and through the CLI binary).

**Gate:** all green, sub-second except the CLI subprocess test; `cargo test -p
gearmaster-engine` byte-identical to A0's run.

**Numbers:** legal-action counts (min, median, max) over the fuzzed states;
cost per action with and without a fight.

## A2 - The oracle and the theme meter

`crates/oracle`: memoised scoring, incremental per-slot rebuild, S0/S1/S2, the
ported `pack_francis` gate, and the ten fidelity signatures.

**Deliverable that stands alone:** the printer that scores all 78 creatures
against their own themes, and the list of the ones that do not read as what
they claim.

**Gate:** cached equals recomputed over 10,000 lookups; the gate ported from
`pack_francis` reproduces `pack_francis`'s verdict on a board it already
accepted; the fidelity table exists and is discussed.

**Numbers:** S0, S1, S2 throughput at 1 and 8 threads; cache hit rate; the
fidelity table.

## A3 - The packer

Greedy recipe packing plus NRPA and CEM, theme-conditioned, two-tier
objective, parallel restarts.

**Two benchmarks, both against a human or against the incumbent:**

1. **Repack-from-tray.** The owner's 75 pieces, the friend's 76, the perfect
   run's, and the preset's 22, each stripped of placement and repacked, then
   walked up `LADDER` at Medium. **Gate: ≥ 48/50 from the owner's tray and
   ≥ 48/50 from the friend's.**
2. **Creature boards.** Fifteen frames and a stratified ten from `LADDER`,
   against `pack_francis`'s own gate at the wall-clock A0 measured. **Gate:
   at least as many in band, closer to `target_ms` on the ones both hit, and
   materially faster - the target is under 5 s a creature.**

Reconstruct every board through `common::board_from` and lock each item as it
assembles (trap 4 - a name list is not a board; learned four times).

**Deliverable:** `make solve CREATURE="..." THEME=...` emitting `gear:` and
`items:` tuples through the same splice path `pack_francis` uses. Never the
GUI's save (trap 15).

## A4 - The pilot

The blind agent plays a run through the console and nothing else. Three
strategies in order: `starter` (seat the oak handle and iron blade, fight,
repeat - the baseline printer's `starter` row played through the economy for
the first time), `greedy` (S0-guided packing plus a value-of-information
shop), and `beam` (a beam over the menu, no fight-and-rewind, because the
pilot has no oracle to rewind into).

Every clear writes a transcript to `analysis/proofs/`, and every transcript is
a script the CLI will replay.

**Gate:** SCR(R10), SCR(R25), SCR(FRANCIS) > 0 in both modes with proofs; the
first DCR reading; the pilot's own rung-50 board reported in
`report_damage_share_and_ttk`'s columns beside the owner's.

**Numbers:** SCR by target × mode; wall-clock per seed; where failures stop
(a rung histogram); the gold curve against the economy's own figures.

## A5 - Coverage as an objective

The novelty reward, a *targeted* mode (given a door, plan to it using only
player knowledge), and the ledger. Doors the pilot never reaches get a second
pass with `Doctrine::coverage` at maximum and ten times the budget before they
are declared unreachable.

**Gate:** `analysis/coverage.md` exists with all three columns, and every gap
is classified as *never offered*, *offered and refused*, or *answered and
unpaid*. A gap in the third class is a bug in the game and gets an issue; a
gap in the second is a balance finding; a gap in the first may be either, and
the ledger says which after the second pass.

Expect this milestone to find things. `completable.rs` knows four shapes of
"a key that arrives after its door shuts" and its `COUNTERS_NOBODY_READS` is
3; a forward play knows all of them by construction.

## A6 - The prior

Behind `--features nn`, in `crates/lab`. Burn, `Autodiff<NdArray>` measured
against `Autodiff<Wgpu>` on Metal, on the real net. Expert iteration from
A3/A4's search: policy over the factored action masked by `menu()`, value
under Ranked Reward, both conditioned on the `Doctrine`.

**Gate:** on held-out seeds, at equal wall-clock **including inference**,
net-guided search strictly beats unguided, or the prior alone strictly beats
the greedy baseline. If neither: write the numbers, the architecture, the
three seeds and the checkpoint hash, and stop. That is a finding, and it is
the one the research predicted.

## A7 - The loop

Iterate: search with the prior, relabel, retrain. Curriculum by seed and by
rung. Report SCR, DCR and PQ on held-out seeds against A3 and A4. Include the
plateau test the Solver §5 asks for: 1× against 10× wall-clock, and a failure
classification - narrow losses are an *evaluation* plateau and belong to the
packer; trays that never hold a viable family are an *exploration* plateau and
belong to the policy.

## A8 - The theme dial

Train on eight themes, hold out two. Measure: does conditioning change
behaviour, does fidelity rise without band-hit falling, and do the two
held-out themes come out better than λ = 0? Report the confusion matrix -
which themes the packer conflates is a statement about the themes, not only
about the net, and `design/monster-themes.md` gets the amendment if two of
them are the same creature.

## A9 - In service

1. **The five county creatures**, packed rather than borrowed, which is THE
   HUNDRED's one open piece of work. `hundred::the_five_wear_a_board_borrowed_from_their_band`
   changes to something that measures the boards instead of comparing them to
   somebody else's. The owner reads the diff.
2. **The validity ledger**, published, with `make validity` re-running it.
3. **The fourth reference build**, if A7 found a board that beats THE UNWOUND
   by the mind lane - exported as a share code, added beside `A_FRIENDS_RUN`
   as a **string literal**, fought by `reference_builds.rs` exactly as the
   others are. The test never knows an agent existed. If no such board was
   found, E6.5 stays open and the finding says why.
4. **The record**: `analysis/the-apprentice.md` finished in the shape
   `post-unwinding.md` uses - what landed, what changed shape, what was cut,
   the numbers, the pins, the amendments earned. `CLAUDE.md` §6 rewritten.

**Gate:** `cargo test -p gearmaster-engine` green, no new engine dependency,
no new non-data file in the engine.

---

# 9. Test inventory

All in the new crates, all sub-second except where marked, none touching an
artifact. The engine's 57 binaries are unaffected by construction - which is
the point of §2's crate graph, and it is also why `cargo test -p
gearmaster-engine` is the regression check after every milestone.

| Crate | Binary | Holds |
|---|---|---|
| console | `boundary` | `agent`'s manifest names `console` and nothing else; `console` re-exports no type that can reach a `Run` |
| console | `parity` | `Verb` ∪ cheats = the GUI's mutator set, **both directions**, by `include_str!` over `gui/src/main.rs` |
| console | `legality` | 1,000 fuzzed reachable states: every enumerated action is accepted; every refused action was not enumerated |
| console | `view` | every `View` field is produced by an accessor the GUI also calls; `monster()` does not leak the coming board |
| console | `transcript` | a play round-trips through text; replay is byte-identical in-process |
| cli | `replay` (extended) | the same transcript, piped through the binary twice, byte-compared |
| oracle | `cache` | cached == recomputed over 10,000 lookups; purse and classes reach the score |
| oracle | `gate` | the ported acceptance gate agrees with `pack_francis` on a board it accepted |
| oracle | `fidelity` | each signature is a ratio over a log, and a board that merely *carries* the vocabulary scores low |
| agent | `greedy` | packs the preset's 22 pieces to at least the preset's 9/50; packs a two-piece tray; refuses nothing legal |
| agent | `search` | NRPA on a ten-piece toy tray finds the known optimum; deterministic under `rayon` |
| agent | `menu_beam` | a hand-authored three-door road finds the reward |
| lab | `proofs` (`#[ignore]`) | every file in `analysis/proofs/` still replays |

The `#[ignore]`d generators live in `lab` as `[[bin]]`s rather than as ignored
tests, because this repo has learned that an ignored test still relinks on
every engine edit (`Cargo.toml`'s `[profile.test]` comment, and the reason it
exists).

---

# 10. Evaluation protocol

**Seed set.** 128 seeds, split **64 training / 64 held-out**, written out in
full in `analysis/the-apprentice.md` at A0 so nobody regenerates them. The
four the repo already uses go in the training half so the held-out half is
clean: `0x5EED_1234_ABCD_0001` (`run.rs:991`) and `acceptance.rs::a_run`'s
`0x60_60`, `0x11_11`, `0x12_12`. The rest are drawn as successive
`Rng::new(0x501_7E5).next_u64()`. **Never add a seed because the agent does
well on it**, and never move one across the split.

**What evaluation is.** One forward play per held-out seed. First sight. No
restart, no fork, no rewind, no `skip_to`, no `apply_preset`, no
`with_all_pieces`, no `force_win` - and the pilot cannot reach any of them,
which is the difference between this list and the Solver's. `Run::undo` is
permitted because it is a button in the game.

**Settings.** Medium is the gated setting and the ladder's. Both modes
reported; Grinder is gated for R10/R25/FRANCIS, both modes for CHAIN and R51.
FRANCIS@d reported for all four difficulties, and its monotonicity - never
easier as the setting rises - is a check the suite does not have.

**Budget.** Wall-clock per seed, on the A0 machine, at 1× = 60 s per rung
reached, and 10× for the plateau test. Report median seconds a seed beside
every rate: a metric without its cost is half a number.

**Noise.** The environment has none. The only variance is the search seed and
the net's init seed. Report at three search seeds and give the spread. Every
artifact header names all three seeds - run, search, init - because a result
that names two does not replay.

**Grinder farming.** Permitted, bounded by wall-clock, and the gold curve is
reported beside the clear rate so that a farm-to-win looks like what it is.

---

# 11. Compute budget

The machine, read off at `020bc7c`: **Apple M2 Max, 12 cores (8 performance,
4 efficiency), 32 GB unified memory, macOS 26.3, rustc 1.95.0**, Metal
available. No CUDA, ever.

Estimates below are scaled from the container's numbers and are the thing A0
replaces. Every one assumes `--release` and `RAYON_NUM_THREADS=8`.

| Quantity | Estimate, one P-core | Eight P-cores |
|---|---|---|
| S0, a board's six figures | ~2 µs | ~4,000,000 /s |
| S1, one fight mid-ladder | ~0.5 ms | ~16,000 /s |
| S2, the four-board gate (16 fights) | ~10 ms | ~800 /s |
| A whole ladder for one board | ~25 ms | ~320 /s |

| Job | Shape | Wall-clock |
|---|---|---|
| A1 legality fuzz, 1,000 states | no fights | seconds |
| A2 fidelity over 78 creatures | 78 × S2 | under a minute |
| A3 one creature, 20,000 S0 candidates + 500 S1 + 50 S2 | tiered | target **< 5 s** |
| A3 repack-from-tray, one tray | ladder per accepted improvement | 2-10 min |
| A4 one seed to FRANCIS at 1× | 50 rungs | ≤ 50 min, typically far less |
| A4 SCR over 64 seeds | parallel across seeds, 8 at a time | an overnight |
| A5 coverage pass, 128 seeds × 2 doctrines | as above | an overnight |
| A6 training, small net | 10⁵ search-labelled positions | hours, not days |
| A7 the loop, three rounds | search + train + relabel | a week of overnights |

If a typical A4 seed costs more than twenty minutes at 1×, the shop's
value-of-information budget is too generous: lower A3's per-shelf budget
before anything else. **The oracle is not the cost; the number of times it is
called per decision is.**

---

# 12. Artifact policy

**Committed:** `analysis/the-apprentice.md` (every number with its commit hash
and thread count) · `analysis/coverage.md` (the ledger) ·
`analysis/proofs/*.txt` (a header naming commit, seed, mode, difficulty,
search seed, agent version and wall-clock, then the transcript - a script the
CLI replays) · share codes promoted into `share.rs` as string literals ·
`gear:` tuples spliced into `combat.rs` by the splice script and reviewed as a
diff · the four crates' source and their sub-second tests.

**Gitignored:** `/artifacts`, `/runs`, `*.mpk`, `*.safetensors`, `*.bin`
under the new crates, and any file over 100 KB an agent writes. A checkpoint
is reproducible from its header and is therefore not source.

**Never:** a model file in git; a test that loads a model; a search longer
than a second inside `cargo test`; anything an agent wrote into
`crates/engine` that is not a string literal or a tuple list.

---

# 13. Traps this mission will meet

Numbered in `CLAUDE.md` §6's style, and the first is the one that decides
whether any of the rest matter.

1. **An agent optimises the metric, so ask what the cheapest way to satisfy
   each gate is before you ship the gate.** This is trap 29 with a machine
   pushing on it. Coverage's cheapest satisfaction is standing in front of
   doors, which is why DCR has three columns. Fidelity's cheapest satisfaction
   is carrying the vocabulary, which is why every signature is a ratio over a
   log. SCR's is winning on the clock, which is why board clears and game
   clears never share a column.
2. **A proof no person can type is not a proof of playability.** The transcript
   goes through the CLI binary, and the CLI must therefore be complete. That
   is why eleven verbs are A1's problem and not a later convenience.
3. **The `View` can leak the game silently.** `Run::monster()` returns the
   coming creature's gear. One field copied without thinking makes every
   number in this document a measurement of a game nobody plays.
4. **The efficiency cores make every throughput number a lie** unless the
   thread count is written beside it. Four of this machine's twelve cores are
   three to four times slower and `rayon` will use them.
5. **`Run` is still not `Clone`** (`run.rs:611`), and the derive belongs to
   privileged code only. A pilot that can clone itself is a pilot that can
   fight and rewind.
6. **`skip_to` and `force_win` are not verbs**, and that is load-bearing
   rather than tidy: the training curriculum may use them because the trainer
   is privileged, and the evaluation cannot because the pilot has no name for
   them.
7. **`EVENTS` is a `const`** (trap 7). Every `&EVENTS[i].choices[j]` in
   another crate is a reference to a copy; compare by value. The coverage
   ledger keys on ids and choice indices for exactly this reason.
8. **Rungs are zero-based inside and one-based on the sign** (traps 9, 20).
   Proofs print rungs. Print `at + 1` and say which you mean, in the header.
9. **A dungeon's floors are a graph** (trap 22) and the coverage denominator
   must count floors as nodes, not as a list. Bound every walk (trap 24) and
   teach the pilot to throw a lever (trap 23) or it will stand at the points
   for forty iterations and the log will read as a stalled board.
10. **A forced event goes to the front of the road stack** (trap 35), so a
    beam that scores "what this door leads to" must know that the door in
    front blocks everything scheduled underneath it.
11. **Three counters are written and read by nothing**
    (`COUNTERS_NOBODY_READS = 3`: `shook-the-machine`, `moles-paid`,
    `crossed`). A beam must not learn to value them, and the coverage ledger
    must list them as content with no door rather than as content the agent
    failed to reach.
12. **The county's pale must not clear itself for being read** (trap 41) and
    THE ENCLOSURE finishes on 5% of simulated censuses today
    (`design/HANDOFF-hundred.md` §3). A pilot that never finishes it is
    probably right, and A5's ledger is the instrument that says whether the
    three dials in that section need turning.
13. **`ALTERNATES` and `CATALOG` are append-only** (traps 2, 34). A packed
    county board is a *replacement* of five specs' gear, not an insertion, and
    `gear_at.txt` will move by thousands of lines if that is got wrong.
14. **Borrowing a creature's board copies its faults** (trap 37). The five
    county creatures wear ladder boards, and three of them run past sudden
    death. Packing them is not a re-dressing, it is the first time anybody has
    asked what those five are *for*.
15. **Any verb can start a fight, not only the fight verb.** Walking onto a
    pinnacle in THE HUNDRED calls `begin_county_fight`, which simulates the
    bout and leaves `Phase::Fighting` with a log waiting - and `county_walk`
    and `leave_county` both refuse outside `Loadout`, so every control dies at
    once. That is the freeze `f4354ec` fixed in the window, **reported from
    play**. A driver settles it wherever it happens rather than special-casing
    the verbs that can do it, and the assertion to write is the general one:
    after any press, there is something to press or the run is over.
16. **`make pack`'s save rewrites `combat.rs` in place** and once rewrote a
    creature nobody was editing (trap 15). `make solve` writes to a file. The
    owner splices.

---

# 14. Decisions for the owner

Each has a default so work can start without an answer. Each is one line to
overrule. The first four are new; the rest carry the Solver's D-list forward
with what has been settled since.

- **E1 - Four crates or two.** Default: **four** (`console`, `oracle`,
  `agent`, `lab`), because the boundary is then a fact about the dependency
  graph rather than a promise in a doc comment, and "the agent has no access
  to the code base" becomes one `cargo tree`. The cheaper alternative is two
  (`console`, `agent`) with a source lint; it is weaker and it is the thing
  most likely to erode.
- **E2 - The CLI gains eleven verbs.** Default: **yes**, at A1. It is what
  makes a proof typeable, and it makes the whole game scriptable for the first
  time - useful well beyond this mission.
- **E3 - What the coverage denominator counts.** Default: events and their
  choices, brawls, town doors, dungeon floors as graph nodes, pedestal
  destinations, county chains and toll thresholds, rumours, classes, rungs.
  Not: individual county *tiles* (a 7×7 grid regenerated per run is a
  distribution, not a checklist) - those get their own coverage number.
- **E4 - The theme's weight.** Default: λ tuned so that a board losing more
  than 10% of its band accuracy for fidelity is rejected. Winning the fight is
  the first-order goal; reading as itself is the second. Say so now or the
  packer will quietly trade one for the other.
- **E5 - `#[derive(Clone)]` on `Run`.** Default: **take it**, in its own
  commit, used by privileged code only. Every field is already `Clone` and it
  changes no rule.
- **E6 - What "solved" means for a path.** Default: **board-decided clears
  only** (under 30 s), on at least one held-out seed, with a committed
  transcript, and the rate reported as the strength of the claim.
- **E7 - Replaying a seed.** Default: unlimited in training (a player can
  restart), never in evaluation. This replaces the Solver's D4 and makes the
  fight-and-rewind question moot for the pilot, since it has no oracle to
  rewind into.
- **E8 - Overnights available.** The plan is priced for one M2 Max and about
  two weeks of wall-clock across A3-A7, most of it unattended. How many
  overnights are there, and is there a second machine to shard seeds across?
- **E9 - Which boards the agent may replace.** Default: **none without the
  owner's say**. The five county creatures are the first candidates because
  they are borrowed rather than authored; the thirteen generator samples stay
  the owner's.
- **E10 - LibTorch.** Default: **no**. Burn on `ndarray`/`wgpu`, candle-metal
  as the fallback on this machine. LibTorch only against a training-throughput
  measurement written into `analysis/the-apprentice.md`.
- **E11 - 128 seeds.** Default: 64 training, 64 held-out. Fewer is faster and
  noisier. Is that the right trade for the overnights in E8?
- **E12 - Grinder farming.** Default: permitted, bounded by wall-clock, gold
  curve reported.

---

# 15. What stops this project early, and what each zero means

Inherited from the Solver §10 and re-pointed at this mission's shape.

1. **A0's fight throughput is far off the container's.** Find out why before
   writing the console. Probably debug versus release, or `rayon` on the
   efficiency cores.
2. **A1's replay is not byte-identical.** Nothing above it means anything.
   The engine's two `HashMap`s are keyed lookups and never iterated - checked
   at the tip - but anything new that iterates one breaks this.
3. **The console's legality disagrees with the GUI's.** Then the proofs
   describe a game nobody can play. This is A1's whole gate and it is worth
   reading the GUI's drag path by hand once.
4. **A3 cannot recover 48/50 from the owner's own tray.** Then the packer is
   not good enough to draw any conclusion, and A4's zeros would be the
   packer's rather than the game's. Fix A3 or stop.
5. **A3 loses to `pack_francis` at equal wall-clock.** Not fatal - A4 does not
   need it - but it retires the creature-board goal and says the sampler was
   better than it looked.
6. **A4's SCR is 0 at 10× on every seed with no narrow losses.** That is a
   balance finding about the economy, to be written up as an amendment, and
   the point at which to stop building agents and start reading the gold
   curve.
7. **A5 finds doors nothing can reach.** That is the mission succeeding, not
   failing. Classify, write up, and let the owner decide which are bugs.
8. **A6 does not beat A4 at equal wall-clock.** Expected, recorded, stopped -
   and the search remains the product. The brief predicted this and the
   research agrees; the reason to build A6 anyway is A7's loop, where the
   prior pays for itself by making the expert faster rather than by playing
   better alone.
9. **Any milestone needs an engine rule to change.** Stop, write the proposal,
   and ask. The engine is what is being measured.
