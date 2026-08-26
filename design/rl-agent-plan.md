# The Solver — making Gear Master playable by an agent that is not a person
## Execution spec for Claude Code (Opus)

Written against commit `18d1b85` (2026-08-26). Companion to
`design/rl-research.md` (the argument for the stack) and successor to
`design/the-unwinding.md` in sequence. Like every document in `design/`:
**code follows this document; when they disagree, this is the bug report** -
except where this document records what the code *does today*, which was
read off the tip and cited by line, and there the code is the news.

**What this is for.** The repo has no way to demonstrate that a specific path
through the game is completable by a build the game itself can produce. It
has a monster-board generator (`tests/pack_francis.rs`), three share codes
somebody built by hand, a preset, and a road-walker that wins fights by fiat
(`Run::force_win`, `run.rs:2377`). This mission builds a **validity solver**:
an agent, with no generative AI anywhere in it, that starts a run from a seed
with 28 gold and an oak handle and either reaches a named door or reports
that it could not - and whose successes are replayable proofs and whose
failures are balance findings. Its by-product is authored boards: the
reference builds the balance work currently hand-writes, and the creature
boards `pack_francis` samples.

**What this is not.** Not a player-facing feature. Not an engine change of
any size (§2 lists the four `pub` exposures it wants). Not a training run in
CI, ever.

---

# 0. Corrections to the brief, from the code

The brief stated six facts about the codebase and asked to be corrected where
the code says otherwise. It is right on five and a half.

1. **"Combat is deterministic and non-interactive."** True. `simulate_party`
   (`combat.rs:3850`) is a pure function of `(Stats, &[ItemProfile],
   &[MonsterSpec], Difficulty, &[ClassDef], purse)`. The half: the **purse**
   is an input - `SpendGold` actions reach into run gold during the fight and
   `Run::fight` (`run.rs:3360`) deducts `log.gold_spent` afterwards. So the
   oracle's answer depends on gold held, and a packer that scores boards
   without the run's purse (as `pack_francis` does, purse 0) scores a
   slightly different fight from the one the run will have. The environment
   scores with the real purse.
2. **"The action space is combinatorial."** True, and smaller at any state
   than it looks. Encoding and sizes in §3.
3. **"Determinism is the asset."** True, with two wrinkles. The run's PRNG is
   **private** (`run.rs:626`), so the agent cannot re-seed mid-run and every
   exploration decision is the agent's own randomness, which is the right
   discipline. And a Rogue wipe draws the *next* run's seed from the current
   run's PRNG (`run.rs:2409`), so "seed 17, Rogue" is a chain of runs, not
   one.
4. **"The engine must stay graphics-free and dependency-light."** It is
   dependency-*free* (`crates/engine/Cargo.toml`), and stays so. §2.
5. **"A search baseline must be beaten before a neural net is justified."**
   Agreed, and sharpened: the repo *already has* a search - a seeded
   stochastic sampler over themed recipes, 300 trials, oracle-scored
   (`pack_francis.rs:54`, `:197`, `:662`). The baseline of this plan has to
   beat that first, on its own job, at equal wall-clock (M2).
6. **"The existing solver is the thing to beat."** There are three, and the
   brief's framing ("decide a build is valid or a rung is clearable") hides
   the important one:
   - *Monster boards:* `pack_francis::pack`. Measured: Cog Priest in
     **39.5 s** release, 300 trials, 21 pieces / 9 items / 30% cells
     (`analysis/post-unwinding.md` §5).
   - *Player boards:* three share codes (`share.rs:159-180`) and
     `apply_preset` (`run.rs:3167`), replayed by `francis`,
     `reference_builds`, `baseline`, `progression`. Hand-built; the repo has
     no record of how their pieces were acquired.
   - *Paths:* `force_win` and `skip_to`. `chain.rs:275-308` proves the chain
     "completable in either mode" by assigning `run.rung` and winning by
     fiat; `progression.rs` has 25 `skip_to` sites. **No test in the repo
     demonstrates that a build reachable through a seed's own shop economy
     fights its way to any door.** The headline metric (§1) is defined so
     that today's value is zero by construction, because that is the truth.

One fact the brief did not state and the plan depends on: **`Run` is not
`Clone`** (`run.rs:408`; the only `derive(Clone)` in the file is
`BoardSnapshot`, `:77`). A branching search over shop and route needs to fork
a run. Either the engine gains one `#[derive(Clone)]` - every field already
is `Clone`: `Rng`, `Shop`, `Loadout`, the two `HashMap`s, `Option<CombatLog>`
- or the agent forks by replaying `(seed, action prefix)`, which is exact but
O(prefix) per branch. This is decision D5 in §11 and M1 is written for
either answer.

---

# 1. The headline metric, and its value today

**Seed-Clear Rate, SCR(target, mode, budget).** Over the fixed evaluation
seed set S (§7), the fraction of seeds on which the agent, starting from
`Run::start(seed, mode, Medium)` (`run.rs:730`) and using only actions a
player has, reaches `target` within `budget` seconds of wall-clock, where
every fight on the way is a **board clear**: `Outcome::Victory` with
`duration_ms < SUDDEN_DEATH_MS` (`combat.rs:40`, 30,000). A fight the clock
decided is a game clear and not a board clear, and the two are reported
separately everywhere.

Targets, in the order the plan reaches them: **R10** (rung 10 cleared),
**R25**, **FRANCIS** (rung 50 cleared), **CHAIN** (`Run::holds(MAINSPRING)`,
`run.rs:1788`), **R51** (THE UNWOUND cleared), **NOWEAPON** (rung 15 cleared
with the weapon grid empty, board-decided), and **FRANCIS@d** for each
`Difficulty`.

| Metric | Today, at `18d1b85` |
|---|---|
| SCR(R10, Grinder) | **0 / \|S\|** - nothing in the repo plays a run |
| SCR(FRANCIS, Grinder) | 0 |
| SCR(CHAIN, either) | 0 (proved only by `force_win`) |
| SCR(R51, either) | 0 |
| SCR(NOWEAPON) | 0; both hand boards reach rung 15 weaponless **on the clock** (`report_no_weapon_viability`: owner Defeat 47.0 s, friend Victory 44.6 s) |
| Best fixed player board, no economy | owner's share code, 48/50 at Medium |
| Best automated player board, no economy | `apply_preset`, 9/50 |
| Repack-from-tray (§5, M2) | unmeasured; the human packed 48/50 from the owner's 75 pieces |
| Monster board authoring | `pack_francis`: 39.5 s a creature, one band hit per run |

Every milestone reports SCR against these rows and against M2's baseline,
in `analysis/rl-agent.md`, with the commit hash beside every number.

---

# 2. Where it lives, and what it may not touch

**A new crate, `crates/agent` (`gearmaster-agent`).** Depends on
`gearmaster-engine` and, optionally, `rayon`. Nothing depends on it. It is a
workspace member so `cargo check --workspace` keeps it compiling; it is **not
in `default-members`**, so `cargo test` at the root does not run its tests
unless asked (D6 decides whether that is wanted).

```
crates/agent/
  Cargo.toml          # [features] nn = ["burn"]   -- off by default; never in CI
  src/lib.rs
  src/env.rs          # Env: wraps Run; observe, legal(), step(), replay()
  src/action.rs       # Action enum, flat index, legality mask
  src/encode.rs       # state features (used from M5; a struct from M1)
  src/oracle.rs       # memoised, parallel scoring of boards against specs
  src/search/greedy.rs  nrpa.rs  cem.rs  beam.rs
  src/proof.rs        # (seed, mode, actions) <-> text; replay == identical logs
  src/bin/pack.rs     # author one board (player or creature)
  src/bin/play.rs     # play one seed to a target; emit a proof or a failure
  src/bin/eval.rs     # SCR over the seed set; writes a markdown table
  src/bin/train.rs    # M5 only, behind `nn`
  tests/              # millisecond tests: legality, replay, toy boards
```

**The engine does not change**, with four exceptions, each a visibility or
derive and none a rule, each taken only if M1 finds it needed and each listed
in the M1 commit:

1. `#[derive(Clone)]` on `Run` (D5).
2. `pub` on any field the observation needs and cannot derive - list them at
   M1; expected: none, since `Run` is nearly all `pub` already (`rerolls`
   and `best_rung` are, `rng` and the two quest maps are not and need not
   be).
3. A `pub fn Run::rng_state(&self) -> u64` **only if** a proof needs to assert
   the PRNG position; expected: no, `e6_1` shows seed + actions suffices.
4. Nothing in `combat.rs`, `piece.rs`, `rating.rs`. If the oracle needs a
   faster path (a `simulate` that does not build the `entries` log), that is
   a *proposal* recorded in `analysis/rl-agent.md`, not a change this mission
   makes - the log is what the GUI replays and every test reads.

**The test path.** `cargo test -p gearmaster-engine` is unaffected by
construction. `cargo test -p gearmaster-agent` runs in under a second and
touches no artifact. Training and search are `[[bin]]`s under
`--release`. CI, if it exists, runs `cargo test --workspace` **without**
`--features nn` and never `cargo run -p gearmaster-agent`. A `make solve`
and `make eval` target wrap the binaries so nobody types the flags.

---

# 3. The action space, encoded

Enumerated from the run's own legality: `Run::can_equip` (`run.rs:3070`),
`Slot::legal_anchors` (`slot.rs:289`), `Run::price`/`payment_for`
(`:1802`, `:1833`), `choice_open` (`:1040`), `Town::actions` and
`Action::costs_the_visit`, `pending_town`, `pending_event`, `at_fountain`,
`fountain_offer`, `pending_brawl`. **Rule:** the agent enumerates, then
calls the engine, and the engine's `Result` is the truth; an enumeration the
engine refuses is a bug in the agent, and `tests/legality.rs` fuzzes 1,000
states to keep the two equal.

```
Place   { tray: u8 (0..12), slot: u8 (0..5), x: u8 (0..6), y: u8 (0..16), rot: u8 (0..4) }
Unequip { piece }                  # back to the tray
Lock    { piece }                  # toggle_lock_item - assembly locks, the reconstruction fault
Buy     { shelf: u8 (0..6) }
Barter  { shelf, paying: tray }
Sell    { tray }
Reroll
Pin     { shelf }                  # Shop::toggle_lock
Fight                              # fight_next, or fight_party if a brawl stands
Answer  { choice: u8 (0..8) }      # take_choice, or take_choice_with(figure) for the sealed bid
Figure  { choice, bid_bucket: u8 } # 0..16 buckets over Requirement::Figure's range
Town    { door: u8 (0..17) }       # visit_town(Action)
WalkOn                             # skip_town
Drink   { class: u8 (0..4) }       # drink_choosing at a fountain
Double  { class }                  # the third fountain
Enter                              # enter_dungeon
Melt    { tray }  Crush { tray }  Pedestal { tray }  Seat { tray, slot, x, y, rot }  # the passenger
Grow    { slot }                   # spend an owed row
Undo                               # only in search, never in a proof
```

**Flat index.** `Place` dominates: 12 × 5 × 16 × 6 × 4 = **23,040**; the
rest fit in 256. Total flat space **23,296**. `share.rs:182`'s packing
(`def<<12 | slot<<9 | x<<6 | y<<2 | rot`) is the same shape with `def` in
place of `tray`; use the tray index in the agent and the def index in the
proof, so proofs are readable by name.

**Legal at any state:** far fewer. A tray of six pieces against five grids
with a dozen legal anchors each at up to four rotations is ~300 placements;
the shop adds six buys and six sells; the road adds at most eight. Every
policy head (M5) is masked to this set; every search (M2-M4) enumerates it.

**Factored form**, which is what the search and any net actually use:
choose a *piece* (tray, ≤ 12), then an *anchor and rotation* for it (≤ 384),
which is the order-then-place decomposition the packing literature settled on
(`rl-research.md` §3) and what `pack_francis::seat_item` already does.

**State, for M5** (a struct from M1, floats from M5): five grids as 6×16
cells × (kind one-hot 16 + assembled + locked + enchant-under + rating
bucket 4) = 5 × 96 × 23 = **11,040**; tray 12 × piece features; shop 6 ×
(piece features + price + pinned); scalars: gold, rung, lives, mode, wins,
losses, `extra_rows`, `insight_unlocked`, classes (31 bits), flags and
counters (by name, ~40), `best_fight_ms`, the road stack's head kind, what
the next creature is (`monster()` index and its theme), difficulty. Piece
features: `piece_rating`, the 20 `Stats` fields, kind, cell count, trigger
count, curse kinds, `touches_insight`, slot. About **13,000 floats**; a
policy over the factored action needs two small heads. Nothing here needs a
GPU.

---

# 4. How the plan spends determinism

1. **A proof is `(seed, mode, difficulty, [action])`.** `proof.rs` writes it
   as text (one action a line, piece by def name), reads it back, replays it
   through the engine, and asserts the `CombatLog`s are identical. That is
   the artifact this mission produces and the one thing a test may ever
   replay. `acceptance::e6_1` is the precedent.
2. **Zero-variance rollouts.** A board's score against a spec is one fight,
   not an average. There is no N to choose.
3. **The oracle is memoised.** `(board hash, spec name, difficulty, purse
   bucket, classes)` → `(outcome, duration_ms, health_left)`. Local search
   revisits boards constantly and a `HashMap` turns revisits into lookups.
   The hash is over `(def index, slot, x, y, rot, locked)` tuples, sorted -
   the same tuple `share.rs` exports.
4. **Curriculum by seed and by rung.** `Run::skip_to(n)` (`run.rs:2383`)
   stands a run at rung n with every bounty paid; it is the honest way to
   train a packer on rung-30 fights without playing thirty rungs first, and
   it is forbidden in evaluation (§7). Curriculum by seed: order S by how far
   the greedy baseline got, easiest first.
5. **Reproducible evaluation.** S is fixed; the agent's own `Rng` is seeded
   from `(run seed, search seed)`; parallel reduces are sorted before they
   are folded. The same command on the same commit prints the same table.
6. **Exact diffs.** When an engine commit moves a proof from pass to fail,
   the proof names the rung and the fight, and the diff of the two
   `CombatLog`s says what changed. That is a balance instrument the repo does
   not currently have.

---

# 5. Milestones

Six, in dependency order, plus a phase zero. Each ends green, with its
numbers in `analysis/rl-agent.md` beside the commit hash. Two ordering rules:
**no framework dependency before M5's gate opens**, and **no artifact enters
the engine except as data**.

| | Milestone | Gate |
|---|---|---|
| M0 | The ground, written down | numbers at the tip in `analysis/rl-agent.md` |
| M1 | The crate, the env, the proof | replay is byte-identical; legality fuzz is clean |
| M2 | The packer: greedy + local search | beats the human tray repack and `pack_francis` at equal wall-clock |
| M3 | The run: shop and road | SCR(R10), SCR(R25), SCR(FRANCIS) > 0 with proofs |
| M4 | The paths that matter, and the plateau | SCR for CHAIN, R51, NOWEAPON, FRANCIS@d; 1× vs 10× compute |
| M5 | A learned prior, **only if M4 says so** | beats M4 at equal wall-clock or is recorded and stopped |
| M6 | Used by the balance work | the fourth reference build; a creature packed; `make solve` |

## M0 - The ground, written down

No code. Fix or record the MSRV (D1). Write `analysis/rl-agent.md` with: the
§1 table; the oracle throughput on **the target laptop** (the numbers in
`post-unwinding.md` §5 are one container core and must be re-taken); the
`pack_francis` timing on the same machine; the seed set S written out; the
compute budget from §8 re-derived from the measured throughput.

**Gate:** the file exists, every number has a commit hash, and the laptop's
oracle throughput is within 5× of the container's. If it is not, stop and
find out why before anything else - the whole plan is priced in fights per
second.

## M1 - The crate, the environment, the proof

`crates/agent` with `env.rs`, `action.rs`, `proof.rs`, `oracle.rs`, and two
binaries that do nothing clever: `play` takes a seed and a proof file and
replays it; `eval` takes the seed set and a strategy name and prints a table.
The first strategy is **`starter`**: seat the oak handle and iron blade,
fight, repeat - which is the `starter` row of the baseline printer played
through the economy for the first time. Expect SCR(R10, starter) ≈ 0.

Decide D5 here: either the one-line `Clone` on `Run` or replay-forking.
Measure both if in doubt - replay-forking a 200-action prefix costs about
200 engine calls, which is microseconds without a fight and milliseconds
with one.

**Tests (all sub-second):** `replay_is_byte_identical` over ten seeds and
three strategies; `every_enumerated_action_is_accepted` and
`every_refused_action_was_not_enumerated`, fuzzed over 1,000 states reached
by random legal play; `a_proof_survives_a_round_trip_through_text`;
`the_oracle_cache_never_lies` (cached == recomputed over 10,000 lookups).

**Gate:** all green; the agent crate builds with `cargo test -p
gearmaster-agent` in under a second of test time; `cargo test -p
gearmaster-engine` is byte-identical in output to M0's run.

**Numbers written:** SCR(R10, starter); legal-action counts (min, median,
max) over the fuzzed states; replay cost per action.

## M2 - The packer: greedy plus local search

The board-level solver, oracle-scored, no run in the loop yet. Input: a
tray (a multiset of pieces, any size), five grids with their rows, a target
(a `MonsterSpec` at a `Difficulty`, or a list of them), a purse and classes.
Output: a board and its score.

**Greedy:** for each slot, for each recipe (`piece::recipes`, `:1039`), seat
the highest-`piece_rating` legal set touching, lock it as it assembles
(`toggle_lock_item` - the reconstruction fault, `HANDOFF.md` §5), repeat
until nothing fits; loose pieces fill remaining cells for flat stats. This
is `pack_francis::seat_item` generalised to a player's tray.

**Local search:** two implementations, compared: **NRPA** (level 2-3, the
playout policy a table over `(piece, slot, anchor, rot)` weights, adapted
toward the best board each level - `rl-research.md` §2) and **CEM** over
the greedy's tie-breaking noise. Moves: remove-and-reseat one item, swap two
items across slots, rotate, replace a loose piece. Score: the oracle, with
TTK as a tiebreak on a win and health-left as a tiebreak on a loss, so the
landscape has a gradient where win/loss alone is flat.

Two jobs, two benchmarks:

1. **Repack-from-tray.** The owner's 75 pieces (`A_WINNING_RUN` decoded,
   pieces stripped of placement), the friend's 76, the perfect run's, and the
   preset's 22. Pack each against `LADDER` at Medium; report rungs cleared.
   The human's number is 48/50 for the owner's pieces. **The gate is
   ≥ 48/50 from the owner's tray and ≥ 48/50 from the friend's.** A packer
   that cannot recover what a person did with the same pieces cannot be
   trusted to do better with different ones.
2. **Monster boards.** For the fifteen frames and a stratified ten from
   `LADDER` (three per theme cluster), run `pack_francis::pack` and the M2
   packer with the *same* acceptance gate (`target_ms` band, four boards,
   four settings - port the gate, do not reinvent it) at the same wall-clock
   (the `pack_francis` time on that machine, measured at M0). **Gate:** M2
   hits the band on at least as many creatures, and on the ones both hit,
   lands closer to `target_ms`.

**Gate:** both benchmarks, written down with the search seeds. If M2 loses
benchmark 2, that is a finding (`rl-research.md` §5's last bullet): record
it, keep the packer for benchmark 1's job, and do not proceed to M5 on the
strength of monster boards.

**Numbers written:** repack table (four trays × cleared, median TTK, weapon
share - the same columns as `report_damage_share_and_ttk`); the monster
table; candidates per second; cache hit rate; NRPA vs CEM at equal budget.

## M3 - The run: shop and road

The agent plays a seed. Per rung: observe the shop, decide buys/sells/
rerolls, call M2 to pack, fight; at a door, choose; at a town, choose a door
or walk on; at a fountain, drink.

**Shop:** value of information, computed rather than learned. For each
shelf, "best board score with this piece minus without", from M2 at a small
budget (M2 must expose a budget knob), minus price; buy the best positive
one, sell the tray's worst if the tray is full (`INVENTORY_CAP` 12), reroll
if nothing is positive and gold allows and `reroll_cost` (`run.rs:791`) is
still low. This is a one-step lookahead with an exact simulator, which is a
lot.

**Road:** a **beam** over route prefixes, width 4-8, each node a forked run
(D5), scored by "rungs cleared so far, then gold, then board score against
the next creature". Doors are enumerated from `choice_open`; a door whose
`describe()` names a reward the board lacks (a class, a row, a word) is
opened first. Towns: one action a visit; take the door whose `receipt`
would pay most by the same value-of-information. The route graph is small
enough that this is exhaustive for the doors and greedy only for the shop.

**Grinder vs Rogue:** a Grinder can farm; the agent's budget is wall-clock,
so farming is permitted and costs time. Rogue has three lives; the beam
carries `lives_left`. Report both.

**Gate:** SCR(R10), SCR(R25), SCR(FRANCIS) at Medium in both modes, each
**> 0**, with a proof file for every clear and a replay test over the
proofs (`#[ignore]`d in the agent crate, promoted if < 1 s - D6). And the
first honest number for the fixed boards: the M3 agent's *own* board at rung
50, measured by `report_damage_share_and_ttk`'s columns, beside the owner's.

**Numbers written:** the SCR table by target × mode; wall-clock per seed
(min/median/max); where the failures stop (rung histogram); gold curve
against #16's economy figures (61 g by rung 4, 223 by 11, 604 by 16, 2,177
by 27).

## M4 - The paths that matter, and the plateau

Four paths, each a target for `play`:

- **CHAIN.** The Wrong Stars at the pub, the astronomer, the locked gate,
  the Manse's cellar, THE THRESHOLD's floors, the second shadow, the Herald,
  the Mainspring. `completable.rs` says every key can exist in time; this
  says whether a board that can *fight* the floors can also hold the words.
- **R51.** After FRANCIS and CHAIN: THE UNWOUND, 15,000 health, authored to
  28.0 s against the owner's board. A board that wins it in under 30 s from
  a seed's economy is the fourth reference build E6.5 wanted, and the agent
  reports whether it won *because of* the mind lane (`Event::MindHit` share,
  Deflection absorbed) or in spite of it.
- **NOWEAPON.** Weapon grid empty from the start; target rung 15,
  board-decided. Today's answer is "only on the clock".
- **FRANCIS@d.** Easy, Hard, Insane. `stepped_component` re-gears him;
  `francis.rs` pins only "not walked through" and "never easier as the
  setting rises". The agent's clear rate by setting is a monotonicity check
  the suite does not have.

**The plateau.** Run M3's agent at 1× and 10× the wall-clock budget on the
same seeds. If SCR moves by less than the seed-to-seed noise (bootstrap over
S), the search has plateaued. Then the **failure analysis**: for the seeds
that fail, is the best board found losing narrowly (health-left within 10%)
- an *evaluation* plateau, fix the packer - or is the tray never holding a
viable family at all - an *exploration* plateau, which is M5's opening.

**Gate:** every path has either a proof or a written failure with the
failure class; the plateau table exists. **A path that fails at 10× with the
economy as it is, is a balance finding**, written into
`analysis/rl-agent.md` and proposed as an amendment to the relevant design
doc, not a reason to build a network.

**Numbers written:** SCR per path per mode; the plateau table; the
first-failure histogram per path; THE UNWOUND's mind-lane share on any
winning board.

## M5 - A learned prior, only if M4's failure class is exploration

Behind `--features nn`. Burn, `Autodiff<NdArray>` first, `wgpu` if the
laptop has a GPU (`rl-research.md` §1). One of two designs, chosen by what
M4 found:

- **AlphaZero with Ranked Reward** over the factored placement action
  (Laterre 2018): policy and value heads on the §3 state; MCTS with the
  oracle at the leaves (no rollouts needed - the oracle *is* the terminal
  value); training targets from search visit counts; the R2 percentile
  curriculum over the agent's own recent scores.
- **Deep CEM** (Wagner 2021) if the problem is "which family to try": a
  policy over piece-choice only, trained on the top-k of a batch, with M2's
  local search doing placement.

Both train against the seed set's *training half* and are evaluated on the
*held-out half* (§7). Checkpoints are gitignored (§9).

**Gate:** on the held-out seeds, at equal wall-clock including inference,
SCR strictly higher than M4's on the path that motivated it, or the trained
policy *used as a prior inside M2's search* makes M2 faster to the same
score. If neither: write down the numbers, the architecture, the seeds and
the checkpoint hash, and **stop**. That is a finding, not a failure, and it
is the one the brief predicted was likeliest.

**Numbers written:** training curve (JSONL), SCR held-out vs M4, inference
cost per action, total training wall-clock.

## M6 - Used by the balance work

Whatever won - M4's search or M5's prior - becomes a tool, not a dependency:

1. **The fourth reference build.** If M4/M5 found a board that beats THE
   UNWOUND by the mind lane, export it as a share code, add it beside
   `A_FRIENDS_RUN` in `share.rs` as `A_MIND_LANE_RUN` **as a string literal**,
   and have `reference_builds.rs` fight it exactly as it fights the other
   three. The test never knows an agent existed. If no such board was found,
   E6.5 stays open and the finding says why.
2. **Creature boards.** `make pack` stays the hand tool. `make solve
   CREATURE=...` runs the M2 packer with `pack_francis`'s gate and emits the
   same `gear:`/`items:` tuples through the same splice script (M17's lesson:
   never the GUI's save). The owner decides which thirteen samples to
   replace.
3. **Proofs as balance instruments.** The proofs for FRANCIS and CHAIN live
   in `analysis/proofs/` (a few KB each). An `#[ignore]`d test in the agent
   crate replays them; `make eval` runs it. When an engine commit breaks a
   proof, the diff of the two logs is the report.
4. **`analysis/rl-agent.md`** is finished with the same sections as
   `post-unwinding.md`: what landed, what changed shape, what was cut, the
   numbers, the pins, the amendments earned.

**Gate:** `cargo test -p gearmaster-engine` green with no new dependency and
no new non-data file; the fourth build's status stated as a fact either way;
`CLAUDE.md` §6 rewritten to point at what comes next.

---

# 6. Consolidated test inventory

All in `crates/agent/tests/`, all sub-second, none touching an artifact:
`replay` (byte-identical logs; proofs round-trip) · `legality` (fuzzed
enumerate-vs-engine, both directions) · `oracle` (cache honesty; purse and
classes reach the score) · `greedy` (packs the preset's 22 pieces into ≥ the
preset's 9/50; packs a two-piece tray; refuses nothing legal) · `search`
(NRPA on a 10-piece toy tray finds the known optimum; deterministic under
`rayon`) · `beam` (a hand-authored three-door road finds the reward) ·
`proofs` (`#[ignore]`d: every file in `analysis/proofs/` still replays).
Engine suite: unchanged except the `Clone` derive if D5 says so.

---

# 7. Evaluation protocol

**Seed set S.** 64 seeds: the three the repo already uses
(`0x5EED_1234_ABCD_0001` from `Run::new`, `run.rs:644`, plus the two that
`acceptance.rs::a_run` and `two_runs.rs::a_run` are called with - read them
off at M0) plus 61 drawn as `Rng::new(0x501_7E5).next_u64()`
in sequence, written out in full in `analysis/rl-agent.md` so nobody
regenerates them. Split 32/32 into training and held-out **only for M5**;
M1-M4 report on all 64. Never add a seed because the agent does well on it.

**Settings.** Medium is the ladder and the gated setting; Easy/Hard/Insane
reported for FRANCIS@d only. Both modes reported; Grinder is the gated one
for R10/R25/FRANCIS because that is the road the tests walk; CHAIN and R51
are gated in both (E6.4's own criterion).

**What counts as a clear.** A fight is a *board clear* if `outcome ==
Victory && duration_ms < 30_000`; a *game clear* if `Victory`. A target is
reached only through board clears on every rung. Both counted; the headline
is board clears (D3 may relax this).

**What the agent may not do in evaluation.** `skip_to`, `force_win`,
`with_all_pieces`, `apply_preset`, `Undo` in a proof, reading the seed's
future shelves (it cannot - the PRNG is private - but a fork-and-peek beam
can: a forked run that *buys* to see what restocks is legitimate play; a
fork that *fights* to see the outcome and then rewinds is exactly what a
player cannot do and is **forbidden in evaluation**, permitted in search
only where D4 allows).

**Budget.** Wall-clock per seed, on the M0 laptop, at 1× = 60 s per rung
reached (so a FRANCIS attempt has 50 min) and 10× for the plateau. Report
median seconds per seed alongside SCR; a metric without its cost is half a
number.

**Noise.** There is none in the environment. The only variance is the
search seed; report SCR at three search seeds and give the spread.

---

# 8. Compute budget

From the container's numbers (`post-unwinding.md` §5), to be re-derived at
M0 on the real machine. Assumed: 8 cores, release, no GPU.

| Quantity | One core | 8 cores |
|---|---|---|
| A fight, mid-ladder | ~0.7 ms | - |
| `combat_items()` on a 19-item board | 0.44 ms | - |
| A scored candidate board (profiles + one fight) | ~1.2 ms | ~6,500 /s |
| A scored candidate against the four-board gate (16 fights) | ~15 ms | ~500 /s |
| Whole ladder for one board | 31 ms | 250 /s |

| Job | Budget | Wall-clock estimate |
|---|---|---|
| M1 legality fuzz, 1,000 states | - | seconds |
| M2 repack-from-tray, one tray, 20,000 candidates | one ladder per accepted improvement | 5-15 min |
| M2 monster board, 300 candidates at the `pack_francis` gate | equal to `pack_francis` | ~40 s each; 25 creatures ≈ 20 min |
| M3 one seed to FRANCIS at 1× (60 s/rung) | 50 rungs | ≤ 50 min; typically far less since most shops need < 5 s |
| M3 SCR over 64 seeds at 1× | | ≤ 2 days worst case, ~8 h typical; run overnight, parallel across seeds |
| M4 plateau: 16 failing seeds at 10× | | ~1 day |
| M5 training, small net, CPU | 10⁵ search-labelled positions | hours, not days; wgpu halves it |

If M3's typical seed is over 20 minutes at 1×, the shop's value-of-
information budget is too generous; lower M2's per-shelf budget before
anything else. The oracle is not the cost; the number of times it is called
per decision is.

---

# 9. Checkpoint and artifact policy

**Committed:**
- `analysis/rl-agent.md` - every number with its commit hash.
- `analysis/proofs/*.proof` - text, a few KB each: header (`commit`, `seed`,
  `mode`, `difficulty`, `search seed`, `agent version`, `wall-clock`), then
  one action a line by def name.
- Share codes promoted into `share.rs`, as string literals, and `gear:`
  tuples spliced into `combat.rs`, both by hand or splice script, both
  reviewed as diffs.
- The agent crate's source and its sub-second tests.

**Gitignored (`.gitignore` gains these lines):** `/artifacts`, `/runs`,
`*.mpk`, `*.safetensors`, `*.bin` under `crates/agent`, and any file over
100 KB the agent writes. A checkpoint is reproducible from its header
(seeds, config, commit) and is therefore not source.

**Never:** a model file in git; a test that loads a model; a test that runs
a search longer than a second unless `#[ignore]`d; anything the agent wrote
into `crates/engine` that is not a string literal or a tuple list.

---

# 10. Failure modes that should stop the project early

1. **The laptop's oracle is 20× slower than the container's.** (M0.) Find
   out why before writing the env - probably debug vs release, or
   `combat_items` allocating more than it should on a bigger board.
2. **Replay is not byte-identical across processes.** (M1.) The engine's
   two `HashMap`s (`run.rs:524, :631`) are keyed lookups and never iterated
   - checked at the tip - but anything new that iterates one, or any
   `HashSet` order reaching a decision (`loadout.rs:466` is a set of
   `StatKind`s used for membership only), would break it. If M1's replay test
   fails, nothing above it is meaningful.
3. **The env's legality disagrees with the GUI's.** (M1.) The engine is the
   referee, but if the GUI permits something `can_equip` refuses (or the
   reverse), the proofs describe a game nobody can play. Compare against
   `gui/src/main.rs`'s drag path once, by reading.
4. **M2 cannot recover 48/50 from the owner's own pieces.** Then the packer
   is not good enough to draw any conclusion about the economy, and M3's
   zeros would be the packer's, not the game's. Fix M2 or stop.
5. **M2 loses to `pack_francis` at equal wall-clock on monster boards.**
   Not fatal to the mission - M3 does not need it - but it retires the "author
   creature boards" goal and says the sampler was better than it looked.
6. **SCR(FRANCIS) is 0 at 10× on every seed with no narrow losses.** A
   balance finding about the economy, to be written up as an amendment
   (probably to #16's tiers), and the point at which to stop building agents
   and start reading the gold curve.
7. **M5 does not beat M4 at equal wall-clock.** Expected; recorded; stopped.
8. **Any milestone needs an engine rule to change.** Stop, write the
   proposal into `analysis/rl-agent.md`, and ask. The engine is what is being
   measured.

---

# 11. Decisions for the user

Things the repo could not settle. Each has a default the plan assumes so
Opus can start; each is one line to overrule.

- **D1 - The toolchain.** `Cargo.toml` declares 1.75 and the code needs
  1.83. Default: raise `rust-version` to `"1.83"` at M0 and say so in
  `CLAUDE.md`. Alternative: rewrite the seven `is_none_or` sites and six
  statics, which is what this audit did in a scratch clone to run anything.
- **D2 - Hardware and time.** The plan is priced for one laptop, 8 cores,
  no GPU, and about a week of wall-clock spread across M2-M4 plus overnight
  evaluations. If there is a GPU, burn's `wgpu` is free at M5. If there is a
  second machine, seeds shard across it trivially. What is the machine, and
  how many overnights?
- **D3 - What "solved" means for a path.** Default: *board-decided* clears
  only (under 30 s), on at least one seed, with a committed proof - "this
  path is completable" - and the SCR reported as the strength of the claim.
  Alternatives: game clears count (sudden-death wins); or a threshold
  ("completable on ≥ 25% of seeds"). The default is the strict one because
  the repo's own doctrine says a fight past 30 s was not decided by the
  board.
- **D4 - How much determinism may be relaxed for exploration.** The
  environment is never randomised. The question is whether the *search* may
  fork a run, fight, and rewind - a player cannot. Default: yes in M2-M4's
  search (it is what makes the beam exact), **never in a proof** (a proof is
  a forward play with no undo), and the evaluation's forbidden list in §7
  stands. Alternative: forbid fight-and-rewind everywhere, which turns the
  road into a partially observed problem and is where PPO would come back in.
- **D5 - `#[derive(Clone)]` on `Run`.** One line in the engine, every field
  already `Clone`; the alternative is replay-forking, exact but O(prefix).
  Default: take the derive, in its own commit, with the reason in the
  message. It changes no rule.
- **D6 - Whether proofs are replayed by a normal test.** Default: an
  `#[ignore]`d test in the agent crate, run by `make eval`, promoted to
  normal only if the whole file replays in under a second. A proof in the
  *engine's* suite is a test dependency on an artifact and the answer to that
  is no.
- **D7 - Non-Rust dependencies.** Default: **none** - burn on `ndarray`/
  `wgpu` if M5 happens. LibTorch only on a measurement written into
  `analysis/rl-agent.md` (`rl-research.md` §5). Is LibTorch acceptable at
  all, if that measurement arrives?
- **D8 - Grinder farming.** A Grinder can farm a rung for gold indefinitely.
  Default: permitted, bounded by the wall-clock budget, and the gold curve is
  reported so a farm-to-win looks like what it is. Alternative: cap fights
  per rung.
- **D9 - Which boards the agent may replace.** Default: none without the
  owner's say - the thirteen generator samples are the owner's to rebuild
  (`HANDOFF.md` §6), and `make solve` writes to a file, not to `combat.rs`.
- **D10 - The seed set.** 64 fixed seeds by the §7 rule. Fewer is faster and
  noisier; more is slower. Is 64 right for the overnights available?
