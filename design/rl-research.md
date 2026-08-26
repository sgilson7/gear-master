# Reinforcement learning in Rust, for this game — a survey with a recommendation

Written against commit `18d1b85` (2026-08-26). Sources are dated where the
date could be read; anything marked *(unverified)* is from memory or from a
page whose date was not visible. The ecosystem moves; re-check crates.io
before adding a dependency.

This is the argument behind `design/rl-agent-plan.md`. Read that for the
milestones; read this for why the milestones use what they use.

## 0. What the repo says the problem is

Four facts, each of which rules out most of the RL literature and rules in a
narrow slice of it. All verified at the tip.

1. **Combat is a pure, cheap, exact function.** `simulate_party`
   (`combat.rs:3850`) consults no RNG. In release it costs 0.03 ms at rung 1
   and 1.4 ms at rung 50 on one ordinary core; the whole fifty-rung ladder is
   31 ms (`analysis/post-unwinding.md` §5). Nobody acts during a fight. So
   combat is not an environment the agent is inside; it is a **reward
   oracle** over a board, and any algorithm that needs millions of cheap
   evaluations can have them.
2. **The decision problem is packing, shopping and routing**, in that order
   of difficulty. Placement: a piece from a tray of at most 12
   (`INVENTORY_CAP`, `run.rs:72`) into one of five 6×8 grids (`slot.rs:5,8`,
   growable) at an anchor and one of four rotations, subject to `can_place`
   and the recipes. Shopping: six shelves (`SHOP_SIZE`), buy/sell/reroll at
   1 g, starting purse 28 g. Routing: 33 events with up to a handful of
   choices each, six towns with 17 door kinds, six dungeons, four
   destinations.
3. **Everything else is seeded** through one private xorshift64* on `Run`
   (`rng.rs`, `run.rs:626`). Two runs from one seed making the same choices
   see the same shelves, the same reserve at the sealed bid, the same melt.
   `acceptance::e6_1` pins it.
4. **The engine has zero dependencies** (`crates/engine/Cargo.toml`) and the
   repo's doctrine is that it stays that way. Whatever trains cannot live in
   the engine, and whatever the engine's tests depend on cannot be a trained
   artifact.

Fact 1 is the one that matters most. The standard deep-RL setting - a slow,
stochastic, partially-observed environment where sample efficiency is the
whole game - is not this. This is closer to **combinatorial optimisation with
an exact objective**, which is where search methods and search-plus-learned-
prior methods live, and where the honest question is whether learning beats
search at all.

## 1. The Rust crates

### Summary table

| Crate | Maturity (2026-08) | Trains? | GPU on a laptop | Non-Rust build dep | Fit here |
|---|---|---|---|---|---|
| **burn** | Active; 0.20 released 2026-01-15, later releases since | Yes, first-class | Yes via `wgpu` (Metal/Vulkan/DX) and CUDA/ROCm; CPU via `ndarray` and its own SIMD backend | **None** with `ndarray`/`wgpu` backends | Primary, if a net is ever justified |
| **candle** | Active (HF; candle-core 0.10.x, 2026) | Yes, but inference-first | CUDA, Metal; no Vulkan/wgpu | None for CPU/Metal; CUDA needs the toolkit | Fallback #2 |
| **tch-rs** | Active; 0.24.0 2026-03-26 | Yes, everything PyTorch has | CUDA, MPS via a Python install | **LibTorch v2.11.0** (a ~GB C++ download or system install) | Fallback #1 only if burn fails on this problem |
| **dfdx** | Dormant; last commit 2024-07-23, last release 2023 | Yes | CUDA, wgpu (old) | None | No - unmaintained |
| **ort** | Active; 2.0.0-rc.12, wraps ONNX Runtime 1.24, MSRV 1.88 | Yes (on-device training API) | Any EP | **ONNX Runtime** binary | No - inference-deployment tool for models trained elsewhere |
| **rurel** | Alive but tiny; 0.6.0, tabular Q-learning only | Tabular | - | None | No - state space is far beyond a table |
| **rsrl** | Dead since 2021 | Classical | - | BLAS | No |
| **border** (laboroai) | Active 2025-05; DQN/IQN/SAC on tch *and* candle; async multi-actor trainer | Yes | Via backend | tch variant: LibTorch; candle variant: none | Reference for loop shape; heavier than needed |
| **rl4burn** | New; RL on burn, backend-generic | Yes | Via burn | None | Watch; too young to build on *(unverified maturity)* |
| **luminal** | Active to mid-2025; static-graph compiler, CUDA/Metal | Yes | CUDA/Metal | None | No - research-grade |
| **tract**, **rten** | Active pure-Rust ONNX *inference* *(unverified versions)* | No | CPU | None | Only if a trained net must be loaded by the GUI without burn |

### burn

Tracel AI's framework is the one pure-Rust option that is built for training
and has a maintained GPU story that does not go through a C++ toolchain.
Burn 0.20 (2026-01-15) introduced CubeK kernels on CubeCL, its own GPU
language, targeting CUDA, ROCm, Metal, WebGPU and Vulkan, plus SIMD CPU
execution (Phoronix, 2026-01-15). The README's claims that matter for a
laptop: dynamic shapes with JIT kernel fusion, and incremental compilation
that recompiles a changed model in under five seconds in release
(tracel-ai/burn README, 2026). The `wgpu` backend auto-detects Metal on macOS
and Vulkan on Linux/Windows, so a MacBook or a Linux laptop with an integrated
GPU trains without installing anything. Backends are swappable behind a
`Backend` trait; `Autodiff<Wgpu>` and `Autodiff<NdArray>` are the same model
code.

Two known rough edges. The wgpu dependency chain needs
`#![recursion_limit = "256"]` (burn README warning, 2026). And burn's `tch`
backend, if you reach for it, pins LibTorch 2.9.0 (lib.rs burn-tch, 2026-08) -
so mixing burn and tch-rs 0.24 (LibTorch 2.11) in one tree is a version fight.
Don't.

RL on burn exists as examples rather than as a library: `bhansconnect/burn-
ppo` is a PPO with vectorised envs, checkpointing with best/latest symlinks,
JSONL metrics and TOML config on the wgpu backend; `yunjhongwu/burn-rl-
examples` is another set. `rl4burn` on crates.io is a backend-generic RL
library on burn *(maturity unverified)*. None of these should be a dependency;
they are worth reading for loop shape.

### candle

Hugging Face's "minimalist ML framework for Rust". It has autodiff and
`candle-nn` optimisers, so it trains, but the project's centre of gravity is
running transformer inference (the examples are Whisper, Llama, Stable
Diffusion; the README's model list is entirely inference). GPU is CUDA and
Metal; there is no Vulkan or wgpu path, so a Linux laptop with an AMD or Intel
GPU trains on CPU. The README also notes `-Ctarget-cpu=native` matters a lot
for CPU speed. Laurent Mazare is the primary author of both candle and tch-rs
(aiwiki, 2026-06). Reasonable fallback; less convenient than burn for the
train-small-net-on-any-laptop case.

### tch-rs

Thin bindings over LibTorch. Complete and fast and it needs the C++ PyTorch
library on the machine: v2.11.0 as of tch 0.24.0 (crates.io, 2026-03-26),
found via `LIBTORCH`, or `LIBTORCH_USE_PYTORCH=1` against a Python install,
or downloaded by a build-script feature. That is a gigabyte-class non-Rust
dependency, a Python-or-not decision, and on macOS MPS acceleration requires
a Python PyTorch install because there is no official LibTorch MPS build
(lib.rs burn-tch, 2026-08). It is the obvious hazard the brief names. The
case *for* it is that if a real network is ever the bottleneck, LibTorch's
kernels and optimisers are the most battle-tested thing available.

### dfdx

Compile-time shape-checked tensors; a lovely design, and the repository's
last commit is 2024-07-23 with the last crates.io release in 2023
(arewelearningyet, 2025). Not a foundation.

### ort

pykeio's wrapper for ONNX Runtime, now at 2.0.0-rc.12 against ORT 1.24 with
an on-device training API and a `train-clm` example (GitHub, 2026). It is the
right tool for shipping a model trained in Python to a Rust binary. That is
not this project: the brief forbids a non-Rust training path, and the
artifact this project produces is a **board**, not a model. ORT is also a
non-Rust binary dependency. Not needed.

### The RL-specific crates

`rurel` is tabular Q-learning over a hashable state (docs.rs); the state here
is a five-grid board plus a tray plus a purse and cannot be a table key.
`rsrl` has not been touched since 2021. `border` is the one serious Rust RL
library: `border-core` traits, a replay buffer, an async multi-actor trainer,
DQN/IQN/SAC agents on either tch or candle (crates.io, 2025-05). It is built
for the continuous-control, image-observation setting and brings tensorboard
and MLflow along. Read its `border-core` traits for how to shape an `Env`
and `Agent`; do not depend on it.

### What that adds up to

For the networks this problem could plausibly need - a policy prior over a
few hundred placements, a value head over a 6×8×5 grid plus a few dozen
scalars - CPU training in burn's `ndarray` backend is enough, wgpu is a free
upgrade on any laptop with a GPU, and nothing non-Rust is required. The
question is not which framework; it is whether a network is needed at all,
and §2 says probably not for the first three milestones.

## 2. Which algorithm families fit

### The setting, stated as an RL problem

Single agent. Episode = one run (or one rung, in the curriculum). Actions are
discrete and combinatorial (§0.2) with a legality mask the engine already
computes (`Slot::legal_anchors`, `Run::can_equip`, `choice_open`). Transition
is deterministic given the seed. Reward is terminal per fight (win/loss, TTK,
health left) and the fight is free to evaluate. The **state space** is
enormous; the **action space at any state** is at most a few hundred legal
placements plus a dozen shop/road moves. Horizon per rung is short (tens of
placements); per run it is long (fifty rungs, each with a shop).

### PPO (and policy-gradient generally)

The workhorse of game-playing RL, and `burn-ppo` shows it runs on burn.
Fits the run-level problem if you accept its costs: it needs a differentiable
policy over a masked combinatorial action, it is sample-hungry by the
standards of exact search, and it learns a *distribution* when what the
balance work wants is a *specific board*. Its real strength here is the
shop/route layer, where the decision is small and the consequences are
delayed. Not the first thing to build.

### DQN and variants

Value-based, off-policy, replay-buffer-friendly. Poor fit for a placement
action space that changes shape every step (a piece removed from the tray
removes hundreds of actions). Fine for the shop's six shelves. `border` has
DQN and IQN if it is ever wanted. Not the first thing to build.

### MCTS / AlphaZero-style search with a learned prior

The natural family for a deterministic, single-player, exactly-scored
problem with a cheap simulator - which is what packing is. Two adaptations
matter because AlphaZero was built for two-player games:

- **Ranked Reward (R2)** - Laterre et al., arXiv 1807.01672 (2018): rank the
  agent's own recent episode scores and train the value head on "did this
  beat the running percentile", which manufactures a self-play curriculum for
  a single-player problem. Applied to 2D and 3D bin packing and reported to
  beat plain MCTS. Wang et al. did the same for Morpion Solitaire, arXiv
  2006.07970 (2020).
- **Gumbel AlphaZero** (Danihelka et al. 2022) cuts the simulation budget
  per move, and **GAZ Play-to-Plan** (Pirnay et al. 2023) plans against the
  agent's past self; both are cited as the current single-player CO line by a
  2025 vehicle-routing paper (arXiv 2502.15777).

These are the methods to reach for *if* a search baseline is not enough and
the failure looks like "the search does not know where to look".

### Monte Carlo search without a network: NMCS and NRPA

The family the brief does not name and the one most likely to win. Nested
Rollout Policy Adaptation (Rosin 2011) learns a *tabular* softmax playout
policy online, per problem instance, by nesting levels of search and
adapting weights toward the best sequence found; it holds records on Morpion
Solitaire and crosswords and has been applied to **3D packing with object
orientation** and TSPTW (Cazenave, GNRPA, arXiv 2003.10024, 2020). It needs
no gradient, no framework, no GPU, and a few hundred lines of Rust. Its
weakness - it cannot carry knowledge between instances - is exactly what the
"learned prior" of §2.3 fixes later, and Cazenave's own line of work
(NeuralNRPA, expert iteration off NRPA) is the bridge. Put differently:
**NRPA is the search baseline, and AlphaZero-with-R2 is what you build if
the baseline plateaus.** A Semantic Scholar abstract of the same group's
2023-24 work notes Monte Carlo search outperforming a "state-of-the-art
neural approach" (deep CEM) on 68 automated-conjecture problems - evidence,
not proof, that on exactly-scored constructive problems search often beats
learning per unit of compute.

### Evolutionary and cross-entropy methods

The deep cross-entropy method (Wagner, "Constructions in combinatorics via
neural networks", arXiv 2104.14516, 2021 *(date from memory)*) trains a
small net on the top-k of a batch of sampled constructions and iterates. It
is simple, embarrassingly parallel, and fits a board-as-sequence-of-
placements encoding. Plain (network-free) CEM over the packer's existing
`choose()` jitter is a one-afternoon upgrade to `pack_francis` and belongs in
milestone 1's local search. Population methods over whole boards (mutate a
placement, re-score) are the "local search" half of milestone 1.

### Contextual bandits for the shop and route layer

Buy/sell/reroll on six shelves and yes/no at a door are small, repeated
decisions whose payoff arrives a few rungs later. A contextual bandit
(LinUCB or a tiny Thompson-sampling model over features: gold, rung, what
the board lacks) is the cheapest thing that could learn them and is
diagnosable in a way a policy net is not. Fits milestone 4's shop layer; the
route layer is better handled by **exhaustive enumeration** - the road graph
is small (33 events × a few choices, gated), `completable.rs` already
enumerates it, and a beam over route prefixes scored by the packer is more
honest than a bandit that has to discover the same graph.

### Verdict by layer

| Layer | Action space | First choice | Escalation |
|---|---|---|---|
| Packing a slot | ~10-400 legal placements | Greedy + NRPA/CEM local search, scored by the oracle | AlphaZero-R2 prior over placements |
| Packing five slots | product of the above | Slot-at-a-time with the oracle on the whole board | Same |
| Shop | ≤ 6 buys, sells, reroll | Value-of-information heuristic: score each shelf by "best board with it minus without" | Contextual bandit |
| Route | a few dozen gated choices | Beam over route prefixes | PPO over the run, last |

## 3. Prior art

### Packing (the closer literature)

- Laterre et al. 2018 (above): AlphaZero-style with ranked reward on 2D/3D
  BPP - the direct ancestor of milestone 5.
- A 2025 systematic review maps 231 RL-for-bin-packing studies from
  2019-2024 and calls hybrid RL-plus-heuristic the emerging state of the art
  while naming scalability, compute and generalisation as the limits
  (ScienceDirect, 2025-11-29). Read as: the field's own conclusion is that
  learning helps most when it steers a heuristic, which is the §2 verdict.
- Crescitelli & Oshima 2023 (IEEE TransAI): 2D *irregular* packing with a
  dense per-step reward - relevant because polyominoes are irregular and
  because "reward after every placement" is what makes a placement policy
  learnable rather than a needle-in-a-haystack terminal reward.
- Pointer-network order-then-place with height maps for regular 2D/3D BPP
  (arXiv 2403.12420, 2024) - the "choose the order, then place
  bottom-left" decomposition is exactly `pack_francis::seat_item`'s shape
  and the decomposition to keep.
- NRPA on 3D packing with orientation (cited in GNRPA, 2020) - rotations as
  part of the action, as here.

What none of them have: an *objective that is itself a simulation*. Packing
papers maximise fill. Here fill is nearly irrelevant (the friend's board is
half loose on purpose and clears 48/50) and the objective is a fight. That is
why the oracle's speed is the whole design.

### Deck, inventory and autobattler agents

- **Slay the Spire**: `sts-lightspeed` is an RNG-accurate C++ simulator and
  `benmuth/AutoClad` trains supervised agents against it (GitHub, 2026);
  `aiplaytesting.github.io` trains DRL agents for card balance. Bateni &
  Whitehead 2024 used an LLM as the player (FDG 2024) - the thing this brief
  rules out. The lesson from the simulator projects is the one already
  learned here: **a fast exact simulator is the asset, and the agent is a
  consumer of it.**
- **Super Auto Pets**: `andreped/super-ml-pets` (RL in Python against a
  simulated environment, deployed by screen capture) and
  `esterRozen/SuperAutoPetsAI` - hobby-scale, no published results found.
- **Magic: the Gathering draft**: Bertram, Fürnkranz & Müller, contextual
  preference ranking for card selection (IEEE CoG 2021, 2024) - the
  "learn a preference over items in context" framing is the shop layer.
- **Backpack Battles / grid-inventory autobattlers**: no published agent
  found in this search. *(Absence unverified - a targeted search for
  "Backpack Battles" agents returned nothing.)* `DESIGN-NOTES.md` calls the
  genre "an offline optimisation puzzle with a stochastic grader"; this
  game removed the stochastic grader.

## 4. Training-loop engineering in Rust

### Parallel rollouts without a gym

There is no Rust Gymnasium and there does not need to be. A `Run` carries
its own `Rng`, so a vector of runs from a vector of seeds is a vector of
independent environments and `rayon::par_iter` over seeds is the whole
vectorisation story. (`Run` is **not** `Clone` at the tip - `run.rs:408` has
no derive - so *forking* one mid-run is either a one-line engine change or a
replay from the seed; `rl-agent-plan.md` D5.) One core did 30 ms per full ladder; a laptop's eight cores do a
thousand full-ladder evaluations in four seconds. The engine's determinism
means results are reproducible under any thread schedule as long as
aggregation is order-independent (sum, max, sorted-then-reduced) - make the
reduce deterministic and the training run is replayable.

### Replay buffers, checkpoints, artifacts

For search (milestones 1-3) the "checkpoint" is a board: encode it as a
share code (`share::export`) or a `gear:`/`items:` tuple list, both of which
the repo already reads. For learned components: burn's `Recorder` writes
model files in its own or safetensors-ish formats; keep them **out of git**
(`.gitignore` gets `/artifacts` and `*.mpk`), and commit only the *board*
the agent found plus a one-line JSON of the config and seed that found it.
Replay buffers are a `Vec<Transition>` with a ring index; `border-core`'s
is a reasonable reference. Metrics go to JSONL, one line a step, so any tool
can plot them; burn ships a TUI dashboard if wanted.

### Seeding

Three seeds, kept separate and written into every artifact's header: the
**run seed** (`Run::start(seed, ..)`), the **search seed** (the agent's own
`Rng`, reusing `rng.rs`'s xorshift so the whole thing stays dependency-free),
and, if a net exists, the **init seed**. A result that names all three
replays; one that names two does not.

### Keeping training out of the test path

Four mechanisms, use all of them:

1. **A separate crate**, `crates/agent`, depending on `gearmaster-engine`
   and nothing the engine depends on. The engine never depends on it.
2. **Binaries, not tests.** Training and search are `[[bin]]` targets
   (`cargo run --release -p gearmaster-agent --bin pack -- ...`). They are
   not `#[test]`s, not even `#[ignore]`d ones - the repo has learned that
   `#[ignore]`d generators still get relinked on every engine edit.
3. **Feature-gated frameworks.** `burn` sits behind a non-default `nn`
   feature. `cargo test --workspace` never compiles it. The agent crate's
   own tests cover the encoders, the legality mask and the search on a toy
   board, in milliseconds.
4. **Artifacts enter the engine as data, never as a dependency.** A board
   the agent found is pasted into `share.rs` or `combat.rs` by the same
   splice path `pack_francis` uses, and from then on the suite fights it
   exactly as it fights the owner's share code. No test loads a model.

If CI runs, it runs `cargo test --workspace` without the `nn` feature and
never `cargo run -p gearmaster-agent`.

## 5. Recommendation

**Primary stack.** Milestones 1-4: pure Rust, no framework. `crates/agent`
with `rayon` as its only dependency (optional even then; `std::thread::scope`
is enough for eight cores). Greedy recipe packer, NRPA/CEM local search, beam
over routes, the engine as the oracle. Boards out as share codes. This is
where the headline metric gets beaten or not.

Milestone 5, only if the gate in the plan says the baseline plateaued:
**burn** with `Autodiff<NdArray>` for training a small policy prior and value
head (AlphaZero with Ranked Reward), `wgpu` as a free upgrade on a laptop
with a GPU. Pure Rust, no LibTorch, no ONNX Runtime, no Python.

**Fallback.** `tch-rs` + LibTorch, accepting the non-Rust dependency, and
only for one of two reasons: burn cannot train the network at all (a missing
op, a wgpu driver problem on the actual laptop), or the network is large
enough that CPU training is the wall-clock bottleneck *and* wgpu is not
available. `candle` is the second fallback if the machine is Apple silicon
(Metal) and the problem with burn was wgpu-specific.

**What would make me switch.**

- From search to a network: milestone 4's gate - the baseline's clear rate
  stops improving with 10× compute, and the failure analysis shows it is
  *exploration* (never trying the right family) rather than *evaluation*
  (the oracle scoring something wrongly).
- From burn to tch: a concrete unsupported op or a training-throughput
  measurement, written into `analysis/`, not a preference.
- From pure-Rust to anything with a Python path: never, per the brief.
- From this whole plan back to the packer: if milestone 1's baseline does
  not beat `pack_francis` at equal wall-clock on monster boards. That would
  say the search space is smaller than it looks and the right investment is
  a better `pack_francis`, not an agent.
