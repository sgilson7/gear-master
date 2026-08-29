//! Train the quartermaster.
//!
//!     cargo run --release -p gearmaster-lab --features nn --bin qpack
//!
//! DQN over `(board, move)` pairs: the menu changes shape every step, so there
//! is no head with a neuron per action - the network scores a pair and the
//! agent takes the argmax over what is legal.
//!
//! **The reward is a fight**, which is privileged, so it is computed here and
//! never seen by the agent. That is the asymmetric actor-critic: the trainer
//! knows what a board is worth and the packer only ever knows what it can see.
//!
//! Written against burn's tensor API rather than its `Module` derive: six
//! tensors and a manual step is less to go wrong across versions.

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
    use burn::tensor::{backend::AutodiffBackend, Tensor, TensorData};
    use gearmaster_console::{Console, Difficulty, Mode, Verb};
    use gearmaster_engine::combat::{simulate_at, Outcome, LADDER, SUDDEN_DEATH_MS};
    use gearmaster_engine::rng::Rng;
    use gearmaster_engine::run::Run;
    use gearmaster_trades::env::{Move, Packing};
    use gearmaster_trades::feature::{self, PAIR};
use gearmaster_lab::themes;
use gearmaster_trades::brief::Brief;
    use std::time::Instant;

    type B = Autodiff<NdArray>;
    const HIDDEN: usize = 96;
    /// How much a step into the future is worth.
    ///
    /// **0.97 over a 120-step budget discounts the fight to 2.6%.** The fight
    /// is the only real reward there is, so at that rate the agent was
    /// optimising the shaping and the step cost and nothing else - and it
    /// dithered for the whole budget, every episode, assembling nothing. The
    /// horizon has to reach the end of the episode or the reward is not in the
    /// problem.
    const GAMMA: f32 = 0.995;

    /// The most presses an episode may take.
    ///
    /// Q0 measured the control at thirteen decisions and forty-seven at the
    /// worst. A hundred and twenty was generous to the point of being an
    /// invitation to dither, and 0.995^40 is 0.82 - the fight is still worth
    /// most of itself at the first decision.
    const BUDGET: usize = 40;

    /// **Suspect 3, from the handoff.** `qcheck`'s control column spends 120 to
    /// 420 presses packing the same trays this agent gets forty for. A budget
    /// that cannot reach the answer makes every episode a failure the reward
    /// cannot distinguish from a bad policy.
    fn budget() -> usize {
        std::env::var("QPACK_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(BUDGET)
    }

    /// Gradient steps a episode.
    const UPDATES: usize = 12;

    /// What a press costs.
    ///
    /// **Without this the policy collapses onto `Rotate`.** It is free - it
    /// changes no item count, so the shaping pays it nothing and costs it
    /// nothing - and there is one of it per tray piece, so a nearly-flat Q
    /// picks one by chance, the state barely moves, and the same action wins
    /// again. The first trained quartermaster pressed rotate 400 times out of
    /// 420 and assembled nothing at all.
    ///
    /// One item is worth fifteen steps at this rate, which is the trade the
    /// number encodes.
    const STEP_COST: f32 = 0.03;

    /// What an action that changed nothing costs on top.
    ///
    /// Q7 took `Rotate` out of the action space on Q3's own diagnosis and the
    /// policy moved straight onto `Pin` - 410 presses out of 420 - which is
    /// the same pathology one action along. Taking actions away one at a time
    /// is not a strategy. The real fault is that a no-op cost 0.01 while the Q
    /// values were spread over 1.7, so the ordering between a no-op and a real
    /// move was noise.
    ///
    /// So the charge is generic and it reads the board rather than the verb:
    /// **an action that leaves the features exactly as it found them costs
    /// more.** That catches every free action there is, including the ones a
    /// later mission adds.
    const NOTHING_HAPPENED: f32 = 0.25;

    /// One remembered decision.
    struct Step {
        x: [f32; PAIR],
        r: f32,
        /// Every legal pair at the state it landed in, for the max in the
        /// bootstrap. Empty when the episode ended there.
        next: Vec<[f32; PAIR]>,
    }

    /// Stand a run at a rung with a purse and a shop. `skip_to` is privileged
    /// and training-only; the agent never learns how it got here.
    ///
    /// **A curriculum, not a uniform sample.** The first version drew a rung
    /// uniformly from fifty and the win rate sat at 1-2% for four thousand
    /// episodes, because `skip_to` pays the bounties and leaves the tray
    /// holding a handle and a blade: the agent was being asked to beat a
    /// rung-forty creature out of one shop, and it never once did. A reward
    /// that is always -1 is not a reward.
    fn situation(seed: u64, rung: usize) -> (Console, usize) {
        let mut run = Run::start(seed, Mode::Grinder, Difficulty::Medium);
        if rung > 0 {
            run.skip_to(rung);
        }
        (Console::standing_in(run, seed), rung)
    }

    /// What an episode was worth: one fight against what is coming.
    ///
    /// A win is worth more the faster and the more decisively it is won; a
    /// loss is worth more the closer it came. That second half is the gradient
    /// A6 found missing - without it every losing board scores the same and
    /// the search has nothing to climb.
    /// How much of the reward is "is this the board that was asked for".
    ///
    /// Half. The packer is still being asked to build something that wins - a
    /// board reading perfectly as a Drainer and losing every fight is not an
    /// enemy board anybody wants - but half is enough that two briefs have
    /// visibly different best answers, which is the thing Q8 tests.
    const FIDELITY: f32 = 0.5;

    /// How much of the brief the board actually delivered.
    ///
    /// **Not A2's fidelity meter, and deliberately.** That meter reads a
    /// *fight*, from the creature's side, which means turning the packed board
    /// into a `MonsterSpec` - and `as_creature` leaks its gear on purpose
    /// (`Box::leak`, oracle/src/lib.rs:197), which is correct for a harvest
    /// that runs once and ruinous inside a loop that runs a hundred thousand
    /// times. So training shapes on what the *board* is and the gate judges
    /// with the meter, which is the right way round anyway: a shaping term
    /// should be cheap and a gate should be the real thing.
    fn delivered(c: &Console, w: &Brief) -> f32 {
        if w.is_none() {
            return 0.0;
        }
        let v = c.view();
        let mut got = [0.0f32; gearmaster_trades::brief::BRIEF];
        for (i, g) in v.grids.iter().enumerate().take(5) {
            if g.items.iter().any(|it| it.assembled) {
                got[i] = 1.0;
            }
        }
        let peak = (0..8)
            .map(|j| (v.pools.produces[j] + v.pools.consumes[j]) as f32)
            .fold(0.0f32, f32::max)
            .max(1.0);
        for j in 0..8 {
            got[5 + j] = (v.pools.produces[j] + v.pools.consumes[j]) as f32 / peak;
        }
        Brief(got).likeness(w)
    }

    fn score(c: &Console, rung: usize, w: &Brief) -> f32 {
        let (stats, items) = c.board_for_scoring();
        if items.is_empty() {
            return -1.5;
        }
        let spec = &LADDER[rung.min(LADDER.len() - 1)];
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        let enemy_max = spec.health.max(1) as f32;
        let won = if log.outcome == Outcome::Victory {
            let quick = 1.0 - (log.duration_ms as f32 / SUDDEN_DEATH_MS as f32).min(1.0);
            let decided = if log.duration_ms < SUDDEN_DEATH_MS { 0.3 } else { 0.0 };
            1.0 + quick * 0.5 + decided
        } else {
            let left = log.enemy().health.max(0) as f32 / enemy_max;
            -1.0 + (1.0 - left) * 0.8
        };
        won + FIDELITY * delivered(c, w)
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
                w1: mat(init(rng, PAIR, HIDDEN), PAIR, HIDDEN, d),
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
        /// The weights as plain numbers, which is what the agent reads.
        fn plain(&self) -> Vec<(&'static str, Vec<f32>)> {
            [("w1", &self.w1), ("b1", &self.b1), ("w2", &self.w2), ("b2", &self.b2), ("w3", &self.w3), ("b3", &self.b3)]
                .into_iter()
                .map(|(n, t)| {
                    (n, t.clone().inner().to_data().convert::<f32>().into_vec().unwrap())
                })
                .collect()
        }
        fn snapshot(&self) -> Frozen {
            Frozen { rows: self.plain() }
        }
    }

    /// A target network: the same weights, held still.
    ///
    /// Bootstrapping off the network being trained is what makes DQN chase its
    /// own tail; the target is updated on a slow clock instead.
    struct Frozen {
        rows: Vec<(&'static str, Vec<f32>)>,
    }
    impl Frozen {
        fn net(&self) -> gearmaster_trades::QNet {
            let mut text = String::new();
            for (n, v) in &self.rows {
                text.push_str(n);
                for x in v {
                    text.push(' ');
                    text.push_str(&format!("{:.6}", x));
                }
                text.push('\n');
            }
            gearmaster_trades::QNet::parse(&text).expect("its own weights")
        }
    }

    pub fn run() {
        let dev: Dev = Default::default();
        let mut rng = Rng::new(0x01_EA_4_1E);
        let episodes: usize =
            std::env::var("QPACK_EPISODES").ok().and_then(|v| v.parse().ok()).unwrap_or(4_000);
        let batch: usize = 128;
        let lr: f32 = std::env::var("QPACK_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);

        let mut net = Net::new(&mut rng, &dev);
        let mut target = net.snapshot();
        let mut frozen = target.net();
        let mut buffer: Vec<Step> = Vec::with_capacity(80_000);
        // Indices into `buffer` where something assembled. Kept alongside
        // rather than inside because the buffer is a ring and an index into it
        // has to be dropped when the slot it names is overwritten.
        let mut good: Vec<usize> = Vec::new();
        // **Whether this run is conditioned at all.** `QPACK_BRIEFS=0` trains
        // the control for Q8's comparison: the same network, the same shape,
        // the same episodes, and thirteen zeros where the brief goes.
        let want_briefs = std::env::var("QPACK_BRIEFS").as_deref() != Ok("0");
        // The learning curve is read against one fixed brief so the figure
        // printed at episode 400 and the one at 2400 are comparable.
        let eval_brief =
            if want_briefs { themes::brief(themes::trained()[0]) } else { Brief::NONE };
        println!(
            "  briefs: {}",
            if want_briefs { "on, eight themes, Hollow and Warden held out" } else { "off" }
        );
        let phi_weight: f32 =
            std::env::var("QPACK_PHI").ok().and_then(|v| v.parse().ok()).unwrap_or(1.5);
        // **Suspect 1, from the handoff.** Roughly five hundred legal placements
        // a step, forty steps, 2,500 episodes: a completing placement is about
        // one in five hundred at random, so the buffer holds a couple of hundred
        // of them among a hundred thousand transitions and a uniform batch of
        // 128 sees one every four batches. This oversamples the transitions
        // whose *shaped reward was positive* - which, since Phi counts items and
        // nothing else, is exactly the transitions where something assembled.
        //
        // It changes which transitions are **sampled**, not which are rewarded,
        // so the optimal policy is untouched. That is the whole reason to reach
        // for this before touching the reward again.
        let priority: f64 =
            std::env::var("QPACK_PRIORITY").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        println!(
            "  phi {:.2}   budget {}   priority {:.2}",
            phi_weight,
            budget(),
            priority
        );
        let t0 = Instant::now();
        let mut won = 0usize;
        let mut seen = 0usize;

        for ep in 0..episodes {
            // ε from 1.0 down to 0.05 over the first two thirds.
            let eps = (1.0 - ep as f32 / (episodes as f32 * 0.66)).clamp(0.05, 1.0);
            // Rungs the agent can actually win at first, widening as it
            // learns: at the start almost everything is in the shallow end,
            // and by the end the whole ladder is in the sample.
            let reach = 3.0 + 47.0 * (ep as f32 / episodes as f32).powf(1.5);
            let rung = (rng.next_u64() % reach as u64) as usize;
            let (mut c, rung) = situation(rng.next_u64(), rung);
            let mut e = Packing::new(budget());
            let mut trail: Vec<([f32; PAIR], Vec<[f32; PAIR]>, f32)> = Vec::new();
            // Potential-based shaping, on the crudest possible signal:
            // **how many items are finished**. `F = γΦ(s') - Φ(s)` provably
            // leaves the optimal policy alone, so the fight is still what is
            // being optimised - this only stops the first thousand episodes
            // being a random walk through a reward of -1.
            //
            // Φ counts items and **nothing about pools**, deliberately. If the
            // shaping told it that matched pools were good, Q4 could not
            // claim it discovered them.
            // **The one dial that moved anything.** At the original 0.15 a
            // completing placement was worth `+0.119` shaped against `-0.03`
            // for any other change, while the Q values were spread over 1.5 -
            // so assembly was a whisper against the noise and the network
            // could not hear the one event the whole task is about.
            //
            // Ten times the weight is three and a half times the items
            // assembled, and 8/50 to 14/50 on the repack benchmark. It does
            // not scale further: at 4.0 the spread blows out to 5.4 and the
            // policy gets worse, which is the hint starting to dominate the
            // fight it was a hint about. The band is four to ten times.
            let potential = |c: &Console| -> f32 {
                let (_, _, items, _) = c.figures();
                phi_weight * items as f32
            };
            let mut phi = potential(&c);

            // **What this episode was asked for.** One theme a episode, drawn
            // from the eight; the two held out are never seen here and are the
            // whole of the Q8 measurement.
            let pool = themes::trained();
            let w = if want_briefs {
                themes::brief(pool[(rng.next_u64() % pool.len() as u64) as usize])
            } else {
                Brief::NONE
            };

            loop {
                let ms = decisions(e.moves(&c));
                if ms.is_empty() {
                    break;
                }
                let v = c.view();
                let b = feature::briefed(&feature::board(&v), &w);
                let pairs: Vec<[f32; PAIR]> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                        // `Done` is a move with no piece and no destination -
                        // an all-zero action, which the network can learn to
                        // value like any other.
                        Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                    })
                    .collect();

                let at = if (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                    (rng.next_u64() % ms.len() as u64) as usize
                } else {
                    argmax(&pairs, &frozen)
                };
                let chosen = pairs[at].clone();
                let m = ms[at];
                e.step(&mut c, m);

                // What comes next, for the bootstrap.
                let next: Vec<[f32; PAIR]> = {
                    let ms2 = decisions(e.moves(&c));
                    if ms2.is_empty() {
                        Vec::new()
                    } else {
                        let v2 = c.view();
                        let b2 = feature::briefed(&feature::board(&v2), &w);
                        ms2.iter()
                            .map(|m| match m {
                                Move::Press(verb) => feature::pair(&b2, &feature::mv(&v2, *verb)),
                                Move::Done => feature::pair(&b2, &[0.0; feature::MOVE]),
                            })
                            .collect()
                    }
                };
                let after = feature::board(&c.view());
                let inert = after == b[..feature::BOARD] && !e.finished;
                let phi2 = if e.finished { 0.0 } else { potential(&c) };
                let shaped = GAMMA * phi2
                    - phi
                    - STEP_COST
                    - if inert { NOTHING_HAPPENED } else { 0.0 };
                phi = phi2;
                trail.push((chosen, next, shaped));
                if e.finished {
                    break;
                }
            }

            // The fight, once, at the end - and every step in the episode is
            // credited with it through the discount.
            let r = score(&c, rung, &w);
            seen += 1;
            if r > 0.0 {
                won += 1;
            }
            for (x, next, shaped) in trail {
                // Phi counts items and nothing else, so a positive shaped
                // reward is exactly "an item assembled on this press".
                if shaped > 0.0 {
                    good.push(buffer.len());
                }
                buffer.push(Step { x, r: shaped, next });
            }
            if let Some(last) = buffer.last_mut() {
                last.r += r;
                last.next.clear();
            }
            if buffer.len() > 80_000 {
                buffer.drain(0..20_000);
                // The indices moved. Anything naming a drained slot is gone and
                // everything else shifts down - an index kept past this is an
                // index into somebody else's transition, which trains the
                // network on a label that belongs to a different board.
                good.retain(|&i| i >= 20_000);
                for i in good.iter_mut() {
                    *i -= 20_000;
                }
            }

            // ---- gradient steps, plural ----
            //
            // One a episode was 2,500 updates over a whole run, and the Q
            // spread stayed at 0.09 from the first evaluation to the last: the
            // network had nothing to say and never got the chance to. Only one
            // transition in forty carries the fight, so a batch holds about
            // three samples with any real reward in it - the signal needs
            // repetition to arrive at all.
            for _ in 0..UPDATES {
            if buffer.len() >= batch {
                let mut xs = Vec::with_capacity(batch * PAIR);
                let mut ys = Vec::with_capacity(batch);
                // Which transitions this batch is drawn from. With priority at
                // zero this is the whole buffer and the sampling is uniform.
                for _ in 0..batch {
                    let pick = if priority > 0.0
                        && !good.is_empty()
                        && (rng.next_u64() % 1000) as f64 / 1000.0 < priority
                    {
                        good[(rng.next_u64() % good.len() as u64) as usize]
                    } else {
                        (rng.next_u64() % buffer.len() as u64) as usize
                    };
                    let s = &buffer[pick];
                    xs.extend_from_slice(&s.x);
                    let bootstrap = if s.next.is_empty() {
                        0.0
                    } else {
                        s.next.iter().map(|p| frozen.q(p)).fold(f32::MIN, f32::max)
                    };
                    ys.push(s.r + GAMMA * bootstrap);
                }
                let x = Tensor::<B, 2>::from_data(TensorData::new(xs, [batch, PAIR]), &dev);
                let y = Tensor::<B, 2>::from_data(TensorData::new(ys, [batch, 1]), &dev);
                let out = net.forward(x);
                // Huber, so one wild target cannot dominate a batch.
                let d = out.sub(y);
                let loss = d.clone().abs().clamp(0.0, 1.0).mul(d.abs()).mean();
                let grads = loss.backward();
                let step = |p: &mut Tensor<B, 2>, g: &<B as AutodiffBackend>::Gradients| {
                    if let Some(gr) = p.grad(g) {
                        *p = Tensor::from_inner(p.clone().inner().sub(gr.mul_scalar(lr)))
                            .require_grad();
                    }
                };
                step(&mut net.w1, &grads);
                step(&mut net.b1, &grads);
                step(&mut net.w2, &grads);
                step(&mut net.b2, &grads);
                step(&mut net.w3, &grads);
                step(&mut net.b3, &grads);
            }
            }

            // The target follows on a slow clock.
            if ep % 200 == 199 {
                target = net.snapshot();
                frozen = target.net();
            }
            if ep % 400 == 0 || ep + 1 == episodes {
                // **On a fixed set**, greedily. The training win rate is not a
                // learning curve: the curriculum widens as it trains, so the
                // task gets harder underneath the number and a flat curve can
                // be real progress or none at all. This is the same twenty
                // situations every time.
                let (ewon, eitems, esteps, espread) = evaluate(&frozen, &eval_brief);
                println!(
                    "  episode {:>5}   eps {:.2}   buffer {:>6}   training {:>3}/{:<4}   \
                     EVAL won {:>2}/20  items {:>4.1}  steps {:>5.1}  spread {:>6.3}",
                    ep,
                    eps,
                    buffer.len(),
                    won,
                    seen,
                    ewon,
                    eitems,
                    esteps,
                    espread
                );
                won = 0;
                seen = 0;
            }
        }

        println!("trained in {:.1}s", t0.elapsed().as_secs_f64());
        std::fs::create_dir_all("runs").ok();
        let mut out = String::new();
        for (n, v) in net.plain() {
            out.push_str(n);
            for x in v {
                out.push(' ');
                out.push_str(&format!("{:.6}", x));
            }
            out.push('\n');
        }
        std::fs::write("runs/quartermaster.txt", out).unwrap();
        println!("wrote runs/quartermaster.txt");
    }

    /// The moves a learner actually chooses between.
    ///
    /// **Rotations are not decisions here.** Q3's diagnosis: the control does
    /// not choose to rotate, it rotates to *look*, and looking is free because
    /// it undoes it. A learner has no undo, so every rotation is a real step
    /// against a real budget, and it must discover that rotate-then-place is a
    /// composite whose value is entirely in the second half. It is a departure
    /// from strict action-fidelity - a person presses twice - but the board it
    /// produces is identical and a proof written from it still replays.
    fn decisions(ms: Vec<Move>) -> Vec<Move> {
        let kept: Vec<Move> = ms
            .iter()
            .copied()
            .filter(|m| !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. })))
            .collect();
        if kept.is_empty() {
            ms
        } else {
            kept
        }
    }

    /// Twenty fixed situations, played greedily. The learning curve.
    fn evaluate(net: &gearmaster_trades::QNet, w: &Brief) -> (usize, f64, f64, f64) {
        let (mut won, mut items, mut steps) = (0usize, 0usize, 0usize);
        // How far apart the best and worst moves look. A network with nothing
        // to say scores every move the same, and that is a different failure
        // from one that has learned the wrong thing.
        let mut spread = 0.0f64;
        let mut spreads = 0usize;
        for i in 0..20u64 {
            let rung = (i as usize * 2) % 24;
            let (mut c, rung) = situation(0xE_A100 + i * 7919, rung);
            let mut e = Packing::new(budget());
            loop {
                let ms = decisions(e.moves(&c));
                if ms.is_empty() {
                    break;
                }
                let v = c.view();
                let b = feature::briefed(&feature::board(&v), w);
                let pairs: Vec<[f32; PAIR]> = ms
                    .iter()
                    .map(|m| match m {
                        Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                        Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                    })
                    .collect();
                let qs: Vec<f32> = pairs.iter().map(|p| net.q(p)).collect();
                let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
                let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
                spread += (hi - lo) as f64;
                spreads += 1;
                let at = argmax(&pairs, net);
                e.step(&mut c, ms[at]);
                steps += 1;
                if e.finished {
                    break;
                }
            }
            let (_, _, n, _) = c.figures();
            items += n;
            if score(&c, rung, w) > 0.0 {
                won += 1;
            }
        }
        (won, items as f64 / 20.0, steps as f64 / 20.0, spread / spreads.max(1) as f64)
    }

    fn argmax(pairs: &[[f32; PAIR]], net: &gearmaster_trades::QNet) -> usize {
        let mut best = (0usize, f32::MIN);
        for (i, p) in pairs.iter().enumerate() {
            let q = net.q(p);
            if q > best.1 {
                best = (i, q);
            }
        }
        best.0
    }
}
