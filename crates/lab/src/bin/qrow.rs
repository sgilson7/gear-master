//! Train the quartermaster on **the row**: one episode is one run.
//!
//!     cargo run --release -p gearmaster-lab --features nn --bin qrow
//!
//! `qpack` draws a rung, stands a run there out of a pool the pilot walked, and
//! scores one packing. This starts at rung one with nothing, packs at every
//! rung, fights, and keeps going until the run runs out of lives - and what the
//! episode is worth is **how deep it got**, growing faster the deeper it is.
//!
//! Every transition in the run is credited with that, so a placement at rung
//! three is judged by the rung the run eventually reached and not by the fight
//! in front of it. That is the whole difference: the packer now meets the
//! consequence of its own board, and the economy at rung twenty is the gold its
//! own boards won.

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
    use gearmaster_lab::row;
    use gearmaster_trades::brief::Brief;
    use gearmaster_trades::env::Move;
    use gearmaster_trades::feature::{self, PAIR};
    use std::time::Instant;

    type B = Autodiff<NdArray>;
    const HIDDEN: usize = 96;

    /// How far a placement's credit reaches.
    ///
    /// A run is many packings and the thing being learned is which placement
    /// led to depth, so the discount has to span a whole run rather than a
    /// single packing. 0.999 over three hundred decisions is 0.74.
    const GAMMA: f32 = 0.999;

    /// Decisions the packer gets at each rung.
    const PACK_BUDGET: usize = 40;

    /// What a decision that changed nothing costs.
    ///
    /// **Nothing, and that is a decision.** It was 0.05, and a run reaching
    /// rung four makes about a hundred and twenty decisions - so the run paid
    /// six in step charges to earn 0.064 for the depth, and a deeper run is
    /// more packings and therefore a larger charge. The reward was punishing
    /// the thing it was for.
    ///
    /// A per-press charge is there to stop an agent dithering, and here there
    /// is nothing to dither into: the packing budget bounds each rung at forty
    /// presses and the episode ends when the *run* dies rather than when the
    /// agent stops pressing. There is no free action to defend against, so
    /// there is no charge.
    const NOTHING: f32 = 0.0;

    /// The seed the trainer's own draws come from.
    const ROW_SEED: u64 = 0x0D0E_5EED;

    /// How many of the next state's candidates the bootstrap looks at.
    ///
    /// `max_a Q(s',a)` over every candidate is correct and it is unaffordable
    /// here: a packing state offers about a hundred and eighty moves, so a
    /// batch of 128 costs twenty-three thousand forward passes an update and
    /// twenty-four updates an episode is half a million. Measured at three
    /// seconds an episode, which is four hours for four thousand.
    ///
    /// The best sixteen under the behaviour policy at the moment the
    /// transition was collected are kept instead. That is a low-biased
    /// estimate of the max - the true best is in the set unless the network has
    /// changed its mind about all sixteen since - and it is eleven times
    /// cheaper.
    const BOOTSTRAP_KEEP: usize = 16;

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
        fn text(&self) -> String {
            let rows: [(&str, &Tensor<B, 2>); 6] = [
                ("w1", &self.w1),
                ("b1", &self.b1),
                ("w2", &self.w2),
                ("b2", &self.b2),
                ("w3", &self.w3),
                ("b3", &self.b3),
            ];
            let mut out = String::new();
            for (n, t) in rows {
                out.push_str(n);
                for x in t.clone().inner().to_data().convert::<f32>().into_vec::<f32>().unwrap() {
                    out.push_str(&format!(" {x:.6}"));
                }
                out.push('\n');
            }
            out
        }
        fn frozen(&self) -> gearmaster_trades::QNet {
            gearmaster_trades::QNet::parse(&self.text()).expect("its own weights")
        }
    }

    struct Trans {
        x: [f32; PAIR],
        r: f32,
        next: Vec<[f32; PAIR]>,
    }

    pub fn run() {
        let episodes: usize =
            std::env::var("QROW_EPISODES").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
        let lr: f32 = std::env::var("QROW_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let updates: usize =
            std::env::var("QROW_UPDATES").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
        // The knee has to cover what a run can be worth: rung 47 squared over
        // twenty-five is 88.
        let knee: f32 =
            std::env::var("QROW_HUBER").ok().and_then(|v| v.parse().ok()).unwrap_or(120.0);
        let mode = if std::env::var("QROW_MODE").as_deref() == Ok("grinder") {
            Mode::Grinder
        } else {
            Mode::Rogue
        };
        println!(
            "  the row: one episode is one run, from rung one until it dies\n  \
             {mode:?}   lr {lr}   updates {updates}   huber knee {knee}   gamma {GAMMA}"
        );

        // **What the written control does through this same loop**, before a
        // gradient is taken. If the control only reaches rung two here then the
        // loop is the ceiling and nothing trained against it means anything -
        // which is the mistake the curriculum walker made once already, where a
        // simplified walk read as a fact about Rogue and was a fact about a
        // hundred lines of harness.
        let mut r = Rng::new(ROW_SEED);
        let (mut sum, mut best) = (0usize, 0usize);
        const CONTROLS: usize = 6;
        for _ in 0..CONTROLS {
            let mut pack = |c: &mut Console| {
                gearmaster_lab::packers::control(c, PACK_BUDGET);
            };
            let (_, out) = row::run(r.next_u64(), mode, Difficulty::Medium, &mut pack);
            sum += out.deepest;
            best = best.max(out.deepest);
        }
        println!(
            "  the written control through this loop: mean rung {:.1}, best {}",
            sum as f32 / CONTROLS as f32,
            best
        );

        let dev = Default::default();
        let mut rng = Rng::new(ROW_SEED);
        let mut net = Net::new(&mut rng, &dev);
        let mut frozen = net.frozen();
        let mut buffer: Vec<Trans> = Vec::with_capacity(80_000);
        let batch = 128usize;
        let t0 = Instant::now();
        let (mut deepest_block, mut deepest_ever, mut ran) = (0usize, 0usize, 0usize);
        let (mut spread, mut spreads) = (0.0f64, 0usize);

        for ep in 0..episodes {
            let eps = (1.0 - ep as f32 / (episodes as f32 * 0.7)).clamp(0.05, 1.0);
            let seed = rng.next_u64();
            // The chosen pair at each decision **and every pair that was on
            // offer there**, because the second is what the decision before it
            // bootstraps from. Storing only the chosen one leaves `next` empty,
            // and an empty `next` means `boot = 0` - no bootstrapping at all,
            // so the run's worth reaches the last press and nothing else.
            let mut trail: Vec<([f32; PAIR], Vec<[f32; PAIR]>)> = Vec::new();

            let mut pack = |c: &mut Console| {
                row::pack_with(c, PACK_BUDGET, |c, ms| {
                    let v = c.view();
                    let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                    let pairs: Vec<[f32; PAIR]> = ms
                        .iter()
                        .map(|m| match m {
                            Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                            Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                        })
                        .collect();
                    let qs: Vec<f32> = pairs.iter().map(|p| frozen.q(p)).collect();
                    let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
                    let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
                    spread += (hi - lo) as f64;
                    spreads += 1;
                    let at = if (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                        (rng.next_u64() % ms.len() as u64) as usize
                    } else {
                        qs.iter()
                            .enumerate()
                            .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                            .map(|(i, _)| i)
                            .expect("not empty")
                    };
                    // Ranked now, while the scores are already in hand, so the
                    // bootstrap can look at a few good candidates rather than
                    // every candidate. See `BOOTSTRAP_KEEP`.
                    let mut ranked: Vec<(usize, f32)> =
                        qs.iter().cloned().enumerate().collect();
                    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("real"));
                    let best: Vec<[f32; PAIR]> = ranked
                        .iter()
                        .take(BOOTSTRAP_KEEP)
                        .map(|(i, _)| pairs[*i])
                        .collect();
                    trail.push((pairs[at], best));
                    at
                });
            };

            let (_c, out) = row::run(seed, mode, Difficulty::Medium, &mut pack);
            deepest_block = deepest_block.max(out.deepest);
            deepest_ever = deepest_ever.max(out.deepest);
            ran += 1;

            // **What the run was worth, credited to every decision in it.**
            //
            // A placement at rung three is judged by the rung the run reached,
            // which is the whole reason this loop exists. The discount does the
            // apportioning: a decision near the end of the run is credited
            // almost in full and one at the start through `gamma^n`.
            let worth = row::worth(&out);
            let n = trail.len();
            for i in 0..n {
                let x = trail[i].0;
                // **What was on offer at the next decision.** The last one has
                // nothing after it, which is what makes it terminal and what
                // stops the run's worth being bootstrapped out of existence.
                let next = if i + 1 < n { trail[i + 1].1.clone() } else { Vec::new() };
                let r = if i + 1 == n { worth } else { -NOTHING };
                buffer.push(Trans { x, r, next });
            }
            if buffer.len() > 80_000 {
                buffer.drain(0..20_000);
            }

            for _ in 0..updates {
                if buffer.len() < batch {
                    break;
                }
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
                    ys.push(s.r + GAMMA * boot);
                }
                let x = Tensor::<B, 2>::from_data(TensorData::new(xs, [batch, PAIR]), &dev);
                let y = Tensor::<B, 2>::from_data(TensorData::new(ys, [batch, 1]), &dev);
                let out = net.forward(x);
                let d = out.sub(y);
                let loss = d.clone().abs().clamp(0.0, knee).mul(d.abs()).div_scalar(knee).mean();
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

            if ep % 50 == 49 {
                frozen = net.frozen();
            }
            if ep % 100 == 0 || ep + 1 == episodes {
                println!(
                    "  episode {:>5}   eps {:.2}   buffer {:>6}   deepest rung {:>3} \
                     (ever {:>3})   spread {:>6.3}",
                    ep,
                    eps,
                    buffer.len(),
                    deepest_block,
                    deepest_ever,
                    spread / spreads.max(1) as f64
                );
                deepest_block = 0;
                ran = 0;
                spread = 0.0;
                spreads = 0;
            }
        }
        let _ = ran;
        println!("trained in {:.1}s", t0.elapsed().as_secs_f64());
        std::fs::create_dir_all("runs").ok();
        let path = "runs/quartermaster_row.txt";
        std::fs::write(path, net.text()).unwrap();
        println!("wrote {path}");
    }
}
