//! Train the pathfinder.
//!
//!     cargo run --release -p gearmaster-lab --features nn --bin qroad
//!
//! The milestone Q5 specified and never ran. It was blocked on Q3: the
//! pathfinder walks with a **frozen** packer, and the trained packer assembled
//! about one item in forty presses, so there was nothing worth freezing. The
//! post-merge finding fixed enough of that - `QPACK_PHI=1.5` took the repack
//! benchmark from 8/50 to 14/50 - that a frozen checkpoint is now a real
//! packer rather than a way of guaranteeing every episode loses.
//!
//! Same architecture as `qpack` and for the same reason: the road menu changes
//! shape every step, so there is no head with a neuron per action. The network
//! scores a `(road, step)` pair and the agent takes the argmax over what is
//! legal.
//!
//! **`pack` is one action.** A run is 195,273 key-presses and 204 decisions,
//! and the difference is entirely the packing - so the packer is a macro-action
//! the pathfinder spends one decision on. That is the temporal abstraction the
//! whole two-agent split exists to buy.
//!
//! ## Trained to a chain, and named after it
//!
//!     QROAD_QUEST=pathfinder_threshold cargo run --release -p gearmaster-lab \
//!         --features nn --bin qroad
//!
//! With `QROAD_QUEST` set the episode is paid along a chain rather than at a
//! rung: `lab::quests` derives the stops, `trades::quest` pays for them, and the
//! model is written under the chain's own name. Without it the episode is what
//! it was - climb, and sometimes be sent to a rung.
//!
//! **The packer defaults to the written control**, and §C1 is why: composing
//! with the learned one first makes every failure ambiguous, and the learned
//! one assembles 2.8 items where the control assembles seventeen. A road policy
//! trained against a packer that cannot clear rung ten will never see the end of
//! a chain that starts at rung eighteen.

#[cfg(not(feature = "nn"))]
fn main() {
    eprintln!("built without --features nn");
}

#[cfg(feature = "nn")]
fn main() {
    q::run();
}

#[cfg(feature = "nn")]
mod q {
    use burn::backend::{Autodiff, NdArray};
    use burn::tensor::activation::relu;
    use burn::tensor::{Tensor, TensorData};
    use gearmaster_console::{Console, Difficulty, Mode};
    use gearmaster_engine::rng::Rng;
    use gearmaster_lab::packers::Packer;
    use gearmaster_lab::quests;
    use gearmaster_trades::env::{Goal, Step as RoadStep, Walking};
    use gearmaster_trades::feature;
    use gearmaster_trades::pathfinder::{self, PAIR};
    use gearmaster_trades::quest::{End, Progress, Quest};
    // The width the weights are actually stored at.
    //
    // `QNet::q_pair` zero-pads a 51-wide road pair into a `feature::PAIR`-wide
    // buffer, so a road network *is* a packing-shaped network with nineteen
    // columns that are always zero. Building it any narrower writes a file the
    // agent cannot load, which is what the first version did.
    use gearmaster_trades::feature::PAIR as WIDE;
    use std::time::Instant;

    type B = Autodiff<NdArray>;
    const HIDDEN: usize = 96;

    /// How much a step into the future is worth.
    ///
    /// Higher than the packer's, and it has to be: a door forty rungs away is
    /// forty decisions of credit, where a placement is worth something inside
    /// the same episode. 0.997^200 is 0.55.
    const GAMMA: f32 = 0.997;

    /// The most decisions a run may take.
    ///
    /// `horizons` measured a run at 204 pathfinder decisions with `pack` as one
    /// action, so 320 is generous rather than binding.
    const BUDGET: usize = 320;

    /// Gradient steps per episode.
    ///
    /// **They are nearly free and this was taking eight.** An episode is three
    /// hundred and twenty road decisions and about half of them call the
    /// control packer, which is two thousand presses of exhaustive seat search;
    /// a gradient step on a batch of 128 through a three-layer 96-wide net is
    /// microseconds. So the wall clock is the environment and the learning was
    /// rationed against a cost it does not have.
    ///
    /// Measured: 300 episodes at eight updates left the network with a Q spread
    /// of 0.163 against rewards spread over four to fifty - climbing, and
    /// nowhere near able to order two actions. `QROAD_UPDATES` moves it.
    fn updates() -> usize {
        std::env::var("QROAD_UPDATES").ok().and_then(|v| v.parse().ok()).unwrap_or(64)
    }

    /// What a road decision that changed nothing costs.
    ///
    /// Sized against the reward it competes with: a rung is `+1`, so a third of
    /// a rung is enough to make three wasted decisions worse than one rung of
    /// progress, and not so much that exploring is punished into never
    /// happening.
    const NOTHING_HAPPENED: f32 = 0.35;

    /// What the whole per-step reward is multiplied by before it is learned.
    ///
    /// **The loss is Huber with a knee at one.** `min(|d|,1)·|d|` is quadratic
    /// for residuals under one and linear above, so above one the gradient is
    /// `±1` whatever the residual is - and the targets here run to **+54**
    /// (measured, `target y: min -1.462 mean 0.140 max 54.100`). A state worth
    /// fifty therefore pulls on the weights exactly as hard as one worth one
    /// and a half, and the network settles on the **median** target, which is
    /// about `-0.5` because most decisions pay a step cost and nothing else.
    ///
    /// That is what three milestones read as a policy collapsing onto a free
    /// action. It was a network with a Q spread of 0.003 between its best and
    /// worst action, fitting the middle of a distribution whose tails had been
    /// clipped out of the gradient.
    ///
    /// Scaling the reward is the standard answer and it is why DQN clips
    /// rewards to `[-1,1]` when it uses this loss. Every term moves together,
    /// so nothing about the ordering changes - `GOAL` still dominates a rung,
    /// a rung still dominates a wasted press - and the targets land where the
    /// loss is proportional.
    ///
    /// **A twenty-fifth was too much, and `--bin qmind` said so.** It put the
    /// targets in `[-0.28, 2.27]` and the value function came out at about
    /// 0.2 - and the gradients shrank with it, so the hidden layers ended 4%
    /// from where they were initialised while the packer's, whose reward is
    /// not scaled, moved 15%. Only the output layer learned: `w3` at +177% of
    /// its initial spread on top of `w1` and `w2` at +4%, which is a linear
    /// readout on random features.
    ///
    /// So the knee moves instead. `HUBER` below decides where the loss stops
    /// being proportional, the loss is divided by it so the gradients stay the
    /// same size whatever it is, and the reward only has to be scaled far
    /// enough to keep the value function somewhere a network can reach.
    fn reward_scale() -> f32 {
        std::env::var("QROAD_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0 / 5.0)
    }

    /// Where the loss stops being proportional to the residual.
    ///
    /// `min(|d|,k)·|d| / k` is quadratic below `k` and linear above it, and
    /// **dividing by `k` is the point**: the gradient is `2|d|/k` below and `1`
    /// above, so it is bounded by two whatever `k` is. That decouples *where
    /// the loss is proportional* from *how large the gradients are*, which the
    /// original `min(|d|,1)·|d|` welded together - and welding them together
    /// is what clipped a target of fifty down to the pull of a target of one
    /// and a half.
    fn huber() -> f32 {
        std::env::var("QROAD_HUBER").ok().and_then(|v| v.parse().ok()).unwrap_or(12.0)
    }

    /// What finishing a chain pays.
    ///
    /// The same as reaching a goal, deliberately: a chain *is* a goal with the
    /// road to it written down, and paying more for one would make the two
    /// incomparable. Everything the chain pays before this telescopes to
    /// nothing (`trades::quest`), so this is the whole of what an episode on a
    /// chain can earn beyond the ladder.
    const FINISH: f32 = 50.0;

    struct Trans {
        x: [f32; WIDE],
        r: f32,
        next: Vec<[f32; WIDE]>,
    }

    /// A road pair in the width the weights are stored at.
    fn wide(p: &[f32; PAIR]) -> [f32; WIDE] {
        let mut out = [0.0f32; WIDE];
        out[..PAIR].copy_from_slice(p);
        out
    }

    fn init(rng: &mut Rng, r: usize, c: usize) -> Vec<f32> {
        let scale = (2.0 / r as f32).sqrt();
        (0..r * c)
            .map(|_| ((rng.next_u64() >> 11) as f32 / (1u64 << 53) as f32 - 0.5) * 2.0 * scale)
            .collect()
    }

    type Dev = <B as burn::tensor::backend::Backend>::Device;
    fn mat(v: Vec<f32>, r: usize, c: usize, d: &Dev) -> Tensor<B, 2> {
        Tensor::<B, 2>::from_data(TensorData::new(v, [r, c]), d).require_grad()
    }

    struct Net {
        w1: Tensor<B, 2>,
        b1: Tensor<B, 2>,
        w2: Tensor<B, 2>,
        b2: Tensor<B, 2>,
        w3: Tensor<B, 2>,
        b3: Tensor<B, 2>,
    }

    impl Net {
        fn new(rng: &mut Rng, d: &Dev) -> Net {
            Net {
                w1: mat(init(rng, WIDE, HIDDEN), WIDE, HIDDEN, d),
                b1: mat(vec![0.0; HIDDEN], 1, HIDDEN, d),
                w2: mat(init(rng, HIDDEN, HIDDEN), HIDDEN, HIDDEN, d),
                b2: mat(vec![0.0; HIDDEN], 1, HIDDEN, d),
                w3: mat(init(rng, HIDDEN, 1), HIDDEN, 1, d),
                b3: mat(vec![0.0; 1], 1, 1, d),
            }
        }
        fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
            let rows = x.dims()[0];
            let h = relu(x.matmul(self.w1.clone()).add(self.b1.clone().repeat_dim(0, rows)));
            let h = relu(h.matmul(self.w2.clone()).add(self.b2.clone().repeat_dim(0, rows)));
            h.matmul(self.w3.clone()).add(self.b3.clone().repeat_dim(0, rows))
        }
        fn plain(&self) -> Vec<(&'static str, Vec<f32>)> {
            [
                ("w1", &self.w1),
                ("b1", &self.b1),
                ("w2", &self.w2),
                ("b2", &self.b2),
                ("w3", &self.w3),
                ("b3", &self.b3),
            ]
            .into_iter()
            .map(|(n, t)| (n, t.clone().inner().to_data().convert::<f32>().into_vec().unwrap()))
            .collect()
        }
        fn text(&self) -> String {
            // **The road pair, not the file's width.** This net is stored
            // `WIDE` because `wide()` pads a road pair up to the packing width
            // before it is fed in, so the shape of the file is a fact about the
            // quartermaster's vector and says nothing about which road columns
            // were read. Two nets on the shelf are 70 wide for exactly that
            // reason and the road pair is 64.
            let mut out = format!("pair {}\n", PAIR);
            for (n, v) in self.plain() {
                out.push_str(n);
                for x in v {
                    out.push(' ');
                    out.push_str(&format!("{:.6}", x));
                }
                out.push('\n');
            }
            out
        }
        fn frozen(&self) -> gearmaster_trades::QNet {
            gearmaster_trades::QNet::parse(&self.text()).expect("its own weights")
        }
    }

    /// Where an episode is sent. Mostly nowhere; sometimes somewhere.
    ///
    /// A goal-conditioned network needs episodes with goals in them, and it
    /// needs *reachable* ones early or the +50 never fires and the conditioning
    /// is noise. So: a rung it can plausibly get to, drawn near where the last
    /// episodes ended.
    fn goal_for(rng: &mut Rng, best: usize, on_a_chain: bool) -> Option<Goal> {
        // A chain owns the payout when there is one. Two large rewards in one
        // episode is two tasks, and the agent would learn whichever is easier.
        if on_a_chain {
            return None;
        }
        match rng.next_u64() % 4 {
            0 => None,
            _ => {
                let reach = best.max(3);
                let want = 2 + (rng.next_u64() as usize % reach);
                Some(Goal::Rung(want))
            }
        }
    }

    pub fn run() {
        let episodes: usize =
            std::env::var("QROAD_EPISODES").ok().and_then(|v| v.parse().ok()).unwrap_or(1200);
        // **One mode a model.** Alternating them trains one policy to play two
        // games whose only difference is what a loss costs - which is the
        // difference the policy would most need to condition on, and it is in
        // the features, but two models is what §3.5's gate asks for and it is
        // the cheaper claim to check.
        let pinned_mode = match std::env::var("QROAD_MODE").as_deref() {
            Ok("grinder") => Some(Mode::Grinder),
            Ok("rogue") => Some(Mode::Rogue),
            _ => None,
        };
        println!(
            "  mode: {}",
            match pinned_mode {
                Some(m) => format!("{m:?} only"),
                None => "both, alternating".into(),
            }
        );
        let pack_path = std::env::var("QROAD_PACKER").unwrap_or_else(|_| "control".into());
        let packer = Packer::named(&pack_path);
        let pack_budget: usize =
            std::env::var("QROAD_PACK_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
        println!("  packer: {}   budget {}", packer.describe(&pack_path), pack_budget);

        // The chain this model is being trained to finish, if there is one.
        //
        // Derived rather than typed: `quests::by_name` walks the tables
        // backwards. A name that does not dress is refused out loud, because a
        // chain with no finish on it trains an agent on the cheap tiers and
        // never pays it for the objective.
        let quest: Option<Quest> = match std::env::var("QROAD_QUEST") {
            Ok(name) => match quests::by_name(&name) {
                Ok(q) => {
                    println!("  quest: {} - {} stops", q.name, q.stops.len());
                    for s in &q.stops {
                        println!(
                            "    {:<14} {:?}   rungs {}-{}",
                            format!("{:?}", s.tier),
                            s.mark,
                            s.window.0 + 1,
                            s.window.1 + 1
                        );
                    }
                    Some(q)
                }
                Err(why) => {
                    eprintln!("QROAD_QUEST={name}: {why:?}");
                    return;
                }
            },
            Err(_) => {
                println!("  quest: none - climbing, and sometimes sent to a rung");
                None
            }
        };
        let suffix = match pinned_mode {
            Some(Mode::Grinder) => "_grinder",
            Some(Mode::Rogue) => "_rogue",
            None => "",
        };
        let out_path = quest
            .as_ref()
            .map(|q| format!("runs/{}{suffix}.txt", q.name))
            .unwrap_or_else(|| format!("runs/pathfinder{suffix}.txt"));

        let dev = Default::default();
        let mut rng = Rng::new(0x0AD_BEEF);
        let mut net = Net::new(&mut rng, &dev);
        let mut frozen = net.frozen();
        let mut buffer: Vec<Trans> = Vec::with_capacity(60_000);
        let batch = 128usize;
        // **The road net was learning at a thirty-third of the packer's rate.**
        //
        // `qpack` runs at 0.05 and this ran at 0.0015, with eight updates an
        // episode against twelve and three hundred episodes against thousands -
        // something like a hundred-fold less learning in total. The result was
        // not a policy that had learned the wrong thing. It was a network with
        // nothing to say: Q(fight) -0.496 against Q(pack) -0.499, a spread of
        // three thousandths, against rewards spread over four to fifty.
        //
        // Three milestones read that as an agent collapsing onto a free action.
        // It was an untrained net whose arbitrary tie-break happened to be
        // consistent, and the column that would have said so is below.
        let lr: f32 =
            std::env::var("QROAD_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let updates = updates();
        let scale = reward_scale();
        let knee = huber();
        println!(
            "  learning: lr {lr}   updates an episode {updates}   reward x{scale}   huber knee {knee}"
        );
        let t0 = Instant::now();
        let mut best_seen = 3usize;
        let (mut reached, mut tried) = (0usize, 0usize);
        // How far along the chain the best episode of this block got, and how
        // many finished it. The two numbers the training is actually about.
        let (mut deepest_stop, mut finished) = (0usize, 0usize);
        // **How far apart the best and worst action look, per decision.**
        //
        // The packer has had this column since Q7 and the road trainer never
        // did, which is why three milestones of road results were read as
        // policy failures. A network with nothing to say scores every move the
        // same, and that is a different failure from one that has learned the
        // wrong thing - and only this number tells them apart.
        let (mut spread, mut spreads) = (0.0f64, 0usize);
        // **What the network is being asked to fit.** If these are small the
        // rewards are not reaching the target and the fault is in the data; if
        // they are large and the output is not, the fault is in the fitting.
        // Two different problems, and nothing here could tell them apart.
        let (mut ylo, mut yhi, mut ysum, mut yn) = (f32::MAX, f32::MIN, 0.0f64, 0usize);
        // **The loss itself**, which nothing here reported. A loss that is
        // falling is a network learning slowly; a loss that is flat is a
        // network not learning at all, and the two want different fixes.
        let (mut lsum, mut ln) = (0.0f64, 0usize);
        // Per block rather than cumulative. `best_seen` is a running maximum
        // over the whole run, so it cannot go down and a block that collapsed
        // reads exactly like one that did not.
        let mut block_deepest = 1usize;

        for ep in 0..episodes {
            let eps = (1.0 - ep as f32 / (episodes as f32 * 0.7)).clamp(0.05, 1.0);
            let seed = rng.next_u64();
            let mode = match pinned_mode {
                Some(m) => m,
                None => {
                    if ep % 2 == 0 {
                        Mode::Grinder
                    } else {
                        Mode::Rogue
                    }
                }
            };
            let mut c = Console::start(seed, mode, Difficulty::Medium);
            let goal = goal_for(&mut rng, best_seen, quest.is_some());
            let mut w = Walking::new(goal.clone(), BUDGET);
            let mut progress = quest.as_ref().map(Progress::new);
            // One reward per episode, because it remembers the highest rung
            // this episode has stood on and pays for nothing below it.
            let mut reward = pathfinder::Reward::new(mode == Mode::Rogue);
            let mut trail: Vec<([f32; WIDE], Vec<[f32; WIDE]>, f32)> = Vec::new();
            let mut best_rung = 1usize;

            loop {
                let ms = w.moves(&c);
                if ms.is_empty() || w.steps >= BUDGET {
                    break;
                }
                let v = c.view();
                let along = match (&quest, &progress) {
                    (Some(q), Some(p)) => q.features(p),
                    _ => [0.0; 2],
                };
                let r = pathfinder::road_on_quest(&v, goal.as_ref(), along);
                let pairs: Vec<[f32; PAIR]> = ms
                    .iter()
                    .map(|s| pathfinder::pair(&r, &pathfinder::describe(&v, s)))
                    .collect();
                let qs: Vec<f32> = pairs.iter().map(|p| frozen.q_pair(p)).collect();
                let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
                let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
                spread += (hi - lo) as f64;
                spreads += 1;
                let at = if (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                    (rng.next_u64() % ms.len() as u64) as usize
                } else {
                    qs.iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .map(|(i, _)| i)
                        .unwrap()
                };
                let chosen = wide(&pairs[at]);
                let losses_before = v.losses;

                // **`Pack` is the road's free action**, and it is the same
                // trap `Rotate` and `Pin` were on the packing side: it is
                // always legal, it never advances the run, and a network that
                // has not yet learned that fighting pays will press it until
                // the budget runs out. The thirty-episode checkpoint pressed it
                // 320 times out of 320 and finished on rung 1.
                //
                // So the charge is the one that worked there: read the board,
                // not the verb. A `Pack` that leaves the board exactly as it
                // found it costs what a wasted decision is worth.
                let board_before = feature::board(&v);
                match &ms[at] {
                    RoadStep::Pack => {
                        // The shopping list, first. §3.3: the purchase happens
                        // outside both agents' action spaces, so a chain that
                        // wants a word gets one the moment the run is standing
                        // somewhere that sells it - and *being* there in time is
                        // what the agent is being paid to work out.
                        let before = c.clone();
                        if let (Some(q), Some(p)) = (&quest, &progress) {
                            gearmaster_lab::shopping::fetch(q, p, &mut c);
                        }
                        packer.pack(&mut c, pack_budget);
                        w.packed(&before, &c);
                    }
                    RoadStep::Press(verb) => {
                        if !c.apply(*verb).ok {
                            break;
                        }
                    }
                }
                let inert = feature::board(&c.view()) == board_before;
                w.steps += 1;

                let after = c.view();
                best_rung = best_rung.max(after.rung_shown);
                let lost = after.losses > losses_before;

                // Whether this step ended the episode, and how - which is the
                // argument that decides whether a chain hands its tiers back.
                // Worked out before the reward, because the reward needs it.
                // **A Rogue episode is a run, and a wipe ends it.** The engine
                // is kind to a player: it replaces a dead run in place, at rung
                // one with the lives back, so `Console::over` never sees the
                // zero. That is a convenience at the screen and a lie to a
                // trainer - the run being trained is gone, and everything after
                // it belongs to a different one.
                //
                // Left running, an episode banked one wipe penalty per death
                // and there were forty-three of them: the road reward came to
                // -866 against +34 for the ground actually covered, and an
                // agent that pressed one key and never fought would have beaten
                // every agent that tried.
                let wiped = c.view().wiped;
                let ms2 = w.moves(&c);
                let over = wiped || ms2.is_empty() || w.steps >= BUDGET;
                let end = match (over, wiped, w.steps >= BUDGET) {
                    (false, _, _) => End::Running,
                    (true, true, _) => End::Terminated,
                    (true, _, true) => End::Truncated,
                    (true, _, false) => End::Terminated,
                };
                let mut rw = reward.of(&c, &w, lost)
                    - if inert { NOTHING_HAPPENED } else { 0.0 };
                let mut done_here = false;
                if let (Some(q), Some(p)) = (&quest, &mut progress) {
                    // A finish ends the episode, so the ending handed to `pay`
                    // is `Terminated` on the step that finishes it whatever the
                    // budget says.
                    let would_finish = !after.wiped && {
                        let mut peek = p.clone();
                        q.observe(&mut peek, &after);
                        q.done(&peek)
                    };
                    let e = if would_finish { End::Terminated } else { end };
                    let paid = q.pay(p, &after, GAMMA, e, FINISH);
                    rw += paid.total();
                    deepest_stop = deepest_stop.max(p.passed());
                    done_here = q.done(p);
                }

                let next: Vec<[f32; WIDE]> = if over || done_here {
                    Vec::new()
                } else {
                    let v2 = c.view();
                    let along2 = match (&quest, &progress) {
                        (Some(q), Some(p)) => q.features(p),
                        _ => [0.0; 2],
                    };
                    let r2 = pathfinder::road_on_quest(&v2, goal.as_ref(), along2);
                    ms2.iter()
                        .map(|s| wide(&pathfinder::pair(&r2, &pathfinder::describe(&v2, s))))
                        .collect()
                };
                // Scaled once, at the end, so every term of the road's reward
                // and every tier of a chain's moves together. See REWARD_SCALE.
                let rw = rw * scale;
                trail.push((chosen, next, rw));
                if wiped {
                    break;
                }
                if done_here {
                    finished += 1;
                    break;
                }
                if w.met(&c) {
                    w.reached = true;
                    break;
                }
            }

            best_seen = best_seen.max(best_rung);
            block_deepest = block_deepest.max(best_rung);
            if goal.is_some() {
                tried += 1;
                if w.reached {
                    reached += 1;
                }
            }
            for (x, next, r) in trail {
                buffer.push(Trans { x, r, next });
            }
            if buffer.len() > 60_000 {
                buffer.drain(0..15_000);
            }

            for _ in 0..updates {
                if buffer.len() >= batch {
                    let mut xs = Vec::with_capacity(batch * PAIR);
                    let mut ys = Vec::with_capacity(batch);
                    for _ in 0..batch {
                        let s = &buffer[(rng.next_u64() % buffer.len() as u64) as usize];
                        xs.extend_from_slice(&s.x);
                        let boot = if s.next.is_empty() {
                            0.0
                        } else {
                            s.next.iter().map(|p| frozen.q(p)).fold(f32::MIN, f32::max)
                        };
                        let y = s.r + GAMMA * boot;
                        ylo = ylo.min(y);
                        yhi = yhi.max(y);
                        ysum += y as f64;
                        yn += 1;
                        ys.push(y);
                    }
                    let x = Tensor::<B, 2>::from_data(TensorData::new(xs, [batch, WIDE]), &dev);
                    let y = Tensor::<B, 2>::from_data(TensorData::new(ys, [batch, 1]), &dev);
                    let out = net.forward(x);
                    let d = out.sub(y);
                    // Huber, normalised by its own knee - see `huber`.
                    let loss =
                        d.clone().abs().clamp(0.0, knee).mul(d.abs()).div_scalar(knee).mean();
                    lsum += loss.clone().into_scalar() as f64;
                    ln += 1;
                    let grads = loss.backward();
                    let step = |p: &mut Tensor<B, 2>| {
                        if let Some(gr) = p.grad(&grads) {
                            *p = Tensor::from_inner(p.clone().inner().sub(gr.mul_scalar(lr)))
                                .require_grad();
                        }
                    };
                    step(&mut net.w1);
                    step(&mut net.b1);
                    step(&mut net.w2);
                    step(&mut net.b2);
                    step(&mut net.w3);
                    step(&mut net.b3);
                }
            }

            if ep % 100 == 99 {
                frozen = net.frozen();
            }
            if ep % 100 == 0 || ep + 1 == episodes {
                match &quest {
                    Some(q) => println!(
                        "  episode {:>5}   eps {:.2}   buffer {:>6}   deepest {:>3} \
                         (ever {:>3})   stops {:>2}/{:<2}   finished {:>3}  spread {:>6.3}",
                        ep,
                        eps,
                        buffer.len(),
                        block_deepest,
                        best_seen,
                        deepest_stop,
                        q.stops.len(),
                        finished,
                        spread / spreads.max(1) as f64
                    ),
                    None => println!(
                        "  episode {:>5}   eps {:.2}   buffer {:>6}   deepest {:>3} \
                         (ever {:>3})   goals {:>3}/{:<4}  spread {:>6.3}",
                        ep,
                        eps,
                        buffer.len(),
                        block_deepest,
                        best_seen,
                        reached,
                        tried,
                        spread / spreads.max(1) as f64
                    ),
                }
                println!(
                    "                 target y: min {:>7.3}  mean {:>7.3}  max {:>7.3}   \
                     loss {:>8.5}   over {} batched",
                    ylo,
                    ysum / yn.max(1) as f64,
                    yhi,
                    lsum / ln.max(1) as f64,
                    yn
                );
                reached = 0;
                tried = 0;
                deepest_stop = 0;
                finished = 0;
                spread = 0.0;
                spreads = 0;
                block_deepest = 1;
                ylo = f32::MAX;
                yhi = f32::MIN;
                ysum = 0.0;
                yn = 0;
                lsum = 0.0;
                ln = 0;
            }
        }

        println!("trained in {:.1}s", t0.elapsed().as_secs_f64());
        std::fs::create_dir_all("runs").ok();
        std::fs::write(&out_path, net.text()).unwrap();
        println!("wrote {out_path}");
    }
}
