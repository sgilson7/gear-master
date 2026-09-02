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

    /// The stream actually used, which `QROW_SEED` may replace.
    ///
    /// **For confirming a result on a second stream and nothing else.** Every
    /// arm in `analysis/the-action-gap.md` E to I shares this one, which is what
    /// makes them comparable; a number measured on a different stream is
    /// comparable only to another number measured on the same one. So a
    /// confirmation run moves *both* arms, and the written control's line at
    /// the top of the run says what the new stream deals - if that moves a long
    /// way, the streams are not equally hard and the two are not comparable
    /// either.
    fn row_seed() -> u64 {
        static S: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
        *S.get_or_init(|| {
            std::env::var("QROW_SEED")
                .ok()
                .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
                .unwrap_or(ROW_SEED)
        })
    }

    /// Episodes to a block: the unit this trainer **reports** on.
    ///
    /// Twenty-five, for a curve with four times the resolution. A block mean
    /// over twenty-five runs is about twice as noisy as one over a hundred -
    /// roughly 0.4 of a rung against 0.2, measured over a policy held
    /// completely still (`--bin qhand`, `QHAND_BLOCKS`) - which is a fine
    /// trade for a picture and a bad one for a decision.
    ///
    /// So it is no longer what the weights are chosen on. See `KEEP_OVER`.
    const BLOCK: usize = 25;

    /// Episodes the **best weights** are chosen over.
    ///
    /// A hundred, whatever the reporting block is. Keeping the best net is a
    /// decision and a decision wants the quieter statistic: `qrow` used to pick
    /// on a block *maximum* and saved a rung-2 net labelled "rung 11"
    /// (`analysis/the-collapse.md` M0), and picking on a noisy mean is the same
    /// mistake with a smaller error bar. Reporting can be as fine as it likes;
    /// choosing may not.
    const KEEP_OVER: usize = 100;

    /// Whether the bootstrap chooses and values with the same network.
    ///
    /// **It did, and that is what `max_a Q(s',a)` means.** One network picks
    /// the action *and* says what it is worth, so any upward error in an
    /// estimate is exactly what the max selects for, and the inflated value is
    /// fed back in as the next target. With `GAMMA` at 0.999 over a whole run
    /// there are hundreds of steps for it to compound over.
    ///
    /// Measured, once the loss stopped scaling the gradients away (M2): the
    /// mean target climbed +0.17 -> +2.83 -> +3.34 over sixteen hundred
    /// episodes and had not stopped, against a plausible return of about 2 -
    /// while the mean rung fell 1.85 -> 1.66 -> 1.36. Values rising without
    /// plateau and performance falling is the textbook signature, and it is
    /// what `design/HANDOFF-the-collapse.md` guessed at before there were any
    /// gradients for it to happen to.
    ///
    /// Double-DQN: the **online** net picks the action, the **frozen** one says
    /// what it is worth. An error has to be shared by two networks a target
    /// refresh apart to survive, which is what breaks the feedback.
    ///
    /// The selector is a snapshot taken once an episode rather than the live
    /// weights - at most twenty-four gradient steps stale, and a hundredth of
    /// the cost of running the training graph over every candidate.
    fn double() -> bool {
        std::env::var("QROW_DOUBLE").map(|v| v != "0").unwrap_or(true)
    }

    /// Whether the run's worth is paid where it was earned.
    ///
    /// **Return decomposition.** `row::spread` is the whole argument; what this
    /// switch does is stop paying `worth` on the last press of the episode and
    /// pay the telescoped increments at the packings that won them instead. The
    /// sum is identical, so the process is return-equivalent and its optimal
    /// policy is unchanged - and the trainer checks that identity on its first
    /// episode and prints the residual rather than trusting it.
    ///
    /// Off by default, because it is an arm.
    fn redistribute() -> bool {
        std::env::var("QROW_REDIST").map(|v| v != "0").unwrap_or(false)
    }

    /// Whether the bootstrap happens at the packing rather than at the press.
    ///
    /// **Temporal abstraction, in the credit and not in the action space.** A
    /// packing is a natural option: forty decisions, a termination condition,
    /// and an outcome the environment scores. So the target for every press in
    /// a packing is the reward that actually accrued to the end of that packing
    /// plus `gamma^k` times the value at the start of the **next** one - which
    /// is the semi-MDP backup, and it turns an episode from 350 chained
    /// bootstraps into about nine.
    ///
    /// **It is paired with `QROW_REDIST` on purpose.** An option's reward in an
    /// SMDP is what accrued during it, and with the reward left terminal that
    /// is exactly zero for every packing but the last - so there would be
    /// nothing for the accumulation to accumulate.
    ///
    /// And there is a prediction attached, written before the run: the recorded
    /// state at the start of the next packing is the **same for every press of
    /// this one**, so the bootstrap term no longer varies with what was
    /// pressed. Today it does - `s_{i+1}` is the board after press `i`, which
    /// is the last place action-dependence survives in this target. If that is
    /// the operative effect, the action gap should fall further, and the arm is
    /// a test of that rather than a hope.
    fn abstracted() -> bool {
        std::env::var("QROW_ABSTRACT").map(|v| v != "0").unwrap_or(false)
    }

    /// Where the state stops and the move starts, in the pair.
    ///
    /// The board and the brief are the situation; the last `feature::MOVE`
    /// numbers are the candidate key. A dueling net reads the first part with
    /// one tower and the whole thing with another.
    const SPLIT: usize = PAIR - feature::MOVE;

    /// Whether `Q` is split into a state value and an advantage over stopping.
    ///
    /// **Off by default, because this is an arm and not a decision.**
    ///
    /// `analysis/the-action-gap.md` measures what this is for: at the written
    /// packer's own states the gap between the best key and the next best is
    /// 0.15% to 2.1% of the state's value, and zeroing two of the 321 inputs -
    /// the rung and the lives left - moves `Q` by more than zeroing the other
    /// 319 together. So the network spends its capacity re-deriving where the
    /// run is inside every key's estimate, and what is left over to say which
    /// key is a fifth of a percent of the answer. That is the situation the
    /// dueling architecture was introduced for, and Seaquest's own quoted
    /// figure - an action gap of 0.04 against a state value of 15 - is 0.27%.
    ///
    /// The identifiability anchor is a **reference action** rather than the
    /// mean over the menu, because the buffer keeps the chosen pair and the
    /// next state's candidates and no menu at all. `Q(s,a) = V(s) + A(s,a) -
    /// A(s, done)`, where "done" is the all-zero move `Move::Done` already
    /// encodes - so `Q(s, done)` is exactly `V(s)` and every other key is
    /// priced as what it is worth over stopping here. See `QNet`'s `Duel`.
    fn dueling() -> bool {
        std::env::var("QROW_DUEL").map(|v| v != "0").unwrap_or(false)
    }

    /// Whether the anchor carries a gradient, and it may not.
    ///
    /// **The first dueling arm was refuted by its own output bias.** `Q = V(s)
    /// + A(x) - A(anchor)` evaluates one tower twice, so the advantage tower's
    /// output bias appears in both terms with opposite sign and its gradient is
    /// `1 - 1 = 0` - identically, for ever. `--bin qmind` printed `b3 ... 0
    /// (exact) NO` after three thousand episodes beside the control's 0.6012,
    /// and the rest of that tower is starved for the same reason: `x` differs
    /// from the anchor in 38 of 321 columns, so every other parameter receives
    /// a difference of two nearly identical gradients. Measured against the
    /// control's same tower, `b1` moved 0.15 as far and `w3` 0.09 as far.
    ///
    /// The textbook cure is to subtract the **mean over the menu** instead, and
    /// it is not affordable in this architecture: standard dueling gets every
    /// action's advantage out of one forward pass because the action is an
    /// *output*, and here it is an *input*, so a differentiable menu mean is
    /// sixteen more towers a sample - eighteen against three, which is ten
    /// hours for three thousand episodes rather than two.
    ///
    /// So the anchor keeps its job and loses its gradient. `Q` is unchanged in
    /// value, `Q(s, done)` is still exactly `V(s)`, and the advantage tower now
    /// takes `dA(x)/dtheta` rather than the difference of two of them. A
    /// semi-gradient, which is what a TD target already is.
    ///
    /// `QROW_DUEL_ANCHOR=grad` is the arm that was measured and refuted, kept
    /// so the comparison can be repeated.
    fn anchor_grad() -> bool {
        std::env::var("QROW_DUEL_ANCHOR").as_deref() == Ok("grad")
    }

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

    /// Three layers and two rectifiers. Both towers are this.
    struct Tower {
        w1: Tensor<B, 2>,
        b1: Tensor<B, 2>,
        w2: Tensor<B, 2>,
        b2: Tensor<B, 2>,
        w3: Tensor<B, 2>,
        b3: Tensor<B, 2>,
    }

    impl Tower {
        fn new(rng: &mut Rng, d: &Dev, wide: usize) -> Tower {
            Tower {
                w1: mat(init(rng, wide, HIDDEN), wide, HIDDEN, d),
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
        fn each(&mut self) -> [&mut Tensor<B, 2>; 6] {
            [
                &mut self.w1,
                &mut self.b1,
                &mut self.w2,
                &mut self.b2,
                &mut self.w3,
                &mut self.b3,
            ]
        }
    }

    struct Net {
        /// The whole pair. On a plain net this *is* `Q`.
        adv: Tower,
        /// The state alone, when this is a dueling net. See `dueling`.
        val: Option<Tower>,
        /// Ones over the state band and noughts over the move band, for
        /// building the reference-action pair without a slice-assign.
        mask: Tensor<B, 2>,
    }

    impl Net {
        fn new(rng: &mut Rng, d: &Dev) -> Net {
            let mut mask = vec![0.0f32; PAIR];
            mask[..SPLIT].fill(1.0);
            let adv = Tower::new(rng, d, PAIR);
            // **Drawn either way**, so the seed stream is identical with and
            // without the value tower and the two arms of the A/B stay
            // comparable. One `rng` here feeds the initialisation, the episode
            // seeds, the exploration and the replay sampling, so a tower drawn
            // only in one arm would change every episode that arm ever plays -
            // and the comparison would be of two different curricula. The
            // demonstration arm above carries the same line for the same
            // reason.
            let val = Tower::new(rng, d, SPLIT);
            Net { adv, val: dueling().then_some(val), mask: Tensor::<B, 2>::from_data(TensorData::new(mask, [1, PAIR]), d) }
        }
        fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
            let a = self.adv.forward(x.clone());
            let Some(v) = &self.val else { return a };
            let rows = x.dims()[0];
            // The same board with the move struck out, which is `Move::Done`.
            let anchor = x.clone().mul(self.mask.clone().repeat_dim(0, rows));
            let base = v.forward(anchor.clone().slice([0..rows, 0..SPLIT]));
            // See `anchor_grad`. Detached, the advantage tower takes its own
            // gradient instead of the difference of two nearly identical ones.
            let off = self.adv.forward(anchor);
            let off = if anchor_grad() { off } else { Tensor::from_inner(off.inner()) };
            base.add(a).sub(off)
        }
        fn text(&self) -> String {
            // **What the pair meant**, written down beside the weights.
            //
            // A width is not a version: the feature vector this repo trains
            // against has widened twice, and a checkpoint saved before a
            // widening reads its columns in the wrong places afterwards while
            // still being a perfectly well-formed file. The stamp is what lets
            // `QNet::load_at` refuse it in a sentence.
            let mut out = format!("pair {}\n", PAIR);
            let mut rows: Vec<(&str, &Tensor<B, 2>)> = vec![
                ("w1", &self.adv.w1),
                ("b1", &self.adv.b1),
                ("w2", &self.adv.w2),
                ("b2", &self.adv.b2),
                ("w3", &self.adv.w3),
                ("b3", &self.adv.b3),
            ];
            if let Some(v) = &self.val {
                // The split goes in the file for the same reason the pair does:
                // a reader may not assume this build's constants.
                out.push_str(&format!("split {}\n", SPLIT));
                rows.extend([
                    ("v1", &v.w1),
                    ("vb1", &v.b1),
                    ("v2", &v.w2),
                    ("vb2", &v.b2),
                    ("v3", &v.w3),
                    ("vb3", &v.b3),
                ]);
            }
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

    /// The candidates the bootstrap will look at: the best `BOOTSTRAP_KEEP`
    /// under the behaviour policy, ranked while the scores are already in hand.
    fn best_of(pairs: &[[f32; PAIR]], qs: &[f32]) -> Vec<[f32; PAIR]> {
        let mut ranked: Vec<(usize, f32)> = qs.iter().cloned().enumerate().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("real"));
        ranked.iter().take(BOOTSTRAP_KEEP).map(|(i, _)| pairs[*i]).collect()
    }

    struct Trans {
        x: [f32; PAIR],
        r: f32,
        /// What the bootstrap is multiplied by. `GAMMA` for a one-press
        /// transition; `gamma^k` when the backup spans a whole packing.
        g: f32,
        next: Vec<[f32; PAIR]>,
    }

    pub fn run() {
        let episodes: usize =
            std::env::var("QROW_EPISODES").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
        let lr: f32 = std::env::var("QROW_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let updates: usize =
            std::env::var("QROW_UPDATES").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
        // **Where the loss stops being proportional, and it was forty times
        // too far out.**
        //
        // It was 120, sized to cover what a run *can* be worth - rung 47
        // squared over twenty-five is 88. But `min(|d|,k)*|d|/k` is `d^2/k`
        // below the knee, so its gradient is `2|d|/k`: bounded by two at the
        // knee, and proportional to **1/k** everywhere below it. Sizing the
        // knee for a rung nothing has ever reached scaled every gradient in
        // three missions of training down by two orders of magnitude.
        //
        // Measured, which is trap 53's own closing instruction finally carried
        // out: the targets run `[-1.96, +3.04]` and **0.0%** of residuals ever
        // reached the knee. Three hundred episodes at 120 moved the Q spread
        // from 0.051 to 0.053 and the mean target from -0.024 to +0.040; the
        // same three hundred at a knee of 3 moved them to **0.118** and
        // **+0.225**. The optimiser was not stuck, it was idling.
        //
        // Three, because that is the top of the range the targets actually
        // occupy. As the value function fits, the targets grow - so read the
        // `past the knee` figure the block line prints: it is 0% now, and when
        // it is not, this number is the one to raise.
        //
        // **Five, chosen by measurement rather than by argument.**
        //
        // `row::RUNG` became one, so a rung pays its whole square and the
        // targets span 1 to 121 - and no single quadratic region covers that.
        // Too high and every ordinary residual has its gradient scaled away
        // (M2); too low and a run worth a hundred and nineteen pulls exactly as
        // hard as one worth two (trap 53). Three hundred episodes apiece, same
        // seed:
        //
        //     knee 20   spread 0.071   past the knee 0.0%
        //     knee  5   spread 0.119   past the knee 0.1%
        //     knee  2   spread 0.133   past the knee 0.1%
        //
        // Five, because two buys a tenth more gradient for two and a half times
        // less headroom, and the targets grow as the value function fits. The
        // `past the knee` figure is the check: a few percent is Huber doing its
        // job on the tail and forty percent is trap 53.
        let knee: f32 =
            std::env::var("QROW_HUBER").ok().and_then(|v| v.parse().ok()).unwrap_or(5.0);
        let mode = if std::env::var("QROW_MODE").as_deref() == Ok("grinder") {
            Mode::Grinder
        } else {
            Mode::Rogue
        };
        // **Proofs, for a window to watch.** `QROW_WATCH=<dir>` writes an
        // episode's tape every `QROW_WATCH_EVERY` episodes and keeps the last
        // `QROW_WATCH_KEEP`. Not every episode: an episode is about 1.8 s of
        // training and about 18 s to replay at the window's pace, so the
        // watcher is ten times slower than the trainer by construction and can
        // only ever sample. See `design/the-episode-watcher.md`.
        let watch = std::env::var("QROW_WATCH").ok();
        let watch_every: usize =
            std::env::var("QROW_WATCH_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(25);
        let watch_keep: usize =
            std::env::var("QROW_WATCH_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
        // **And every episode that gets somewhere, whether or not it is a
        // sample.** One in twenty-five is a fine cadence for "what is it doing
        // now" and useless for "what does it do when it gets deep": a policy
        // whose mean is rung two puts a rung-seven episode in the sample about
        // never, and twenty proofs off a live run were rung 1 and 2 without
        // exception. Those go in a directory of their own, because `prune`
        // drops by name and would eat them first.
        // **One run, handed to a fresh network as its first experience.**
        //
        // `QROW_DEMO_NET=<net> QROW_DEMO_SEED=<hex>` plays that seed with that
        // network, greedily, as episode zero - and its transitions go into the
        // buffer like any other episode's.
        //
        // **Not by replaying a proof, and the reason is worth keeping.** A tape
        // is keys, and keys are not the whole decision: the first attempt
        // followed a rung-13 tape through eighty presses exactly and then
        // diverged, because `row::run` is a function of the *packer* and a tape
        // only records what the packer pressed. Rebuilding an episode from its
        // output means reconstructing every hidden thing the output does not
        // carry. Re-deriving it from the packer that made it is exact by
        // construction, and `row::run(seed, mode, difficulty, packer)` is
        // already that function.
        //
        // What to expect, so the result can be read either way: a thousand
        // transitions against a buffer of eighty thousand, sampled uniformly,
        // is about one percent of the first draw and less after that. If it
        // does nothing, that is dilution rather than a verdict on
        // demonstrations - the next thing to try is repeating it or protecting
        // it from eviction, not dropping the idea.
        let teacher = std::env::var("QROW_DEMO_NET")
            .ok()
            .and_then(|p| match gearmaster_trades::QNet::load_at(&p, PAIR) {
                Ok(n) => {
                    println!("  demonstration: episode 0 played by {p}");
                    Some(n)
                }
                Err(why) => {
                    eprintln!("  {why}");
                    None
                }
            });
        let demo_seed: Option<u64> = std::env::var("QROW_DEMO_SEED")
            .ok()
            .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
        // **How often the teacher plays.**
        //
        // Once was measured and did nothing: 840 transitions against a buffer
        // of 80,000 is about one percent of the first draws and none at all by
        // episode two thousand, because the buffer drains twenty thousand at a
        // time. One good trajectory cannot outvote a hundred thousand mediocre
        // ones. Every tenth episode gives it a standing share instead - a
        // demonstration is 840 transitions where an ordinary episode is about
        // 120, so it is roughly two fifths of everything the buffer sees.
        let demo_every: usize =
            std::env::var("QROW_DEMO_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(10);

        let watch_deep: usize =
            std::env::var("QROW_WATCH_DEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
        if let Some(dir) = &watch {
            println!(
                "  watching: a proof every {watch_every} episodes into {dir}, keeping \
                 {watch_keep}\n  ...and every episode reaching rung {watch_deep} into {dir}/deep, kept"
            );
        }
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
        let mut r = Rng::new(row_seed());
        let (mut sum, mut best) = (0usize, 0usize);
        const CONTROLS: usize = 6;
        for _ in 0..CONTROLS {
            let mut pack = |c: &mut Console| {
                gearmaster_lab::packers::control(c, PACK_BUDGET);
                // The control does not report its presses; this loop wants the
                // rung it reached and nothing else.
                Vec::new()
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

        let double = double();
        if teacher.is_some() {
            println!("  the teacher plays every {demo_every} episodes, and is not counted in the mean");
        }
        println!("  bootstrap: {}", if double {
            "double - the online net picks, the frozen net values"
        } else {
            "single - one net picks and values, which is what overestimates"
        });

        // Where the two checkpoints go. `QROW_OUT=<prefix>` moves them, so an
        // A/B can run both arms at once without each clobbering the other's
        // weights - which is how two hours of one arm becomes two hours of
        // both.
        let out = std::env::var("QROW_OUT").unwrap_or_else(|_| "runs/quartermaster_row".into());
        let (out_best, out_last) = (format!("{out}.txt"), format!("{out}_last.txt"));

        let dev = Default::default();
        let mut rng = Rng::new(row_seed());
        let mut net = Net::new(&mut rng, &dev);
        // **The two halves of this trainer are two implementations of one
        // function**, and a dueling net has three towers' worth of composition
        // to get wrong. `forward` is the burn graph the gradient flows through;
        // `QNet` is the hand-rolled arithmetic that chooses every key and every
        // bootstrap. If they disagree the run trains one function and plays
        // another, and nothing anywhere would go red.
        //
        // So: score the same random pairs both ways, once, and print the worst
        // disagreement. `CLAUDE.md` trap 42 - "the engine treats these
        // identically" is a claim about one code path.
        {
            let check = net.frozen();
            // Its own stream, for the same reason: a diagnostic may not move
            // the run it is diagnosing.
            let mut rng = Rng::new(row_seed() ^ 0x0000_C4EC);
            let n = 8usize;
            let mut xs = Vec::with_capacity(n * PAIR);
            let mut pairs: Vec<[f32; PAIR]> = Vec::new();
            for _ in 0..n {
                let mut p = [0.0f32; PAIR];
                for v in p.iter_mut() {
                    *v = ((rng.next_u64() >> 11) as f32 / (1u64 << 53) as f32 - 0.5) * 2.0;
                }
                xs.extend_from_slice(&p);
                pairs.push(p);
            }
            let want = net
                .forward(Tensor::<B, 2>::from_data(TensorData::new(xs, [n, PAIR]), &dev))
                .inner()
                .to_data()
                .convert::<f32>()
                .into_vec::<f32>()
                .expect("its own output");
            let got: Vec<f32> = pairs.iter().map(|p| check.q(p)).collect();
            let set = check.q_set(&pairs);
            let worst = want
                .iter()
                .zip(got.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            // `q_set` shares the state terms across a menu, so it may only be
            // asked about pairs that share a state - these do not. It is
            // checked on a set that does.
            let mut shared = pairs.clone();
            for p in shared.iter_mut() {
                p[..SPLIT].copy_from_slice(&pairs[0][..SPLIT]);
            }
            let one_by_one: Vec<f32> = shared.iter().map(|p| check.q(p)).collect();
            let together = check.q_set(&shared);
            let worst_set = one_by_one
                .iter()
                .zip(together.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            let _ = set;
            println!(
                "  graph against evaluator: worst {worst:.2e}   q_set against q: worst {worst_set:.2e}   \
                 dueling {}",
                check.is_dueling()
            );
            assert!(worst < 1e-3, "the training graph and the acting evaluator disagree by {worst}");
            assert!(worst_set < 1e-3, "q_set and q disagree by {worst_set}");
        }

        // Whether the return-equivalence check below has been run.
        let mut redist_checked = false;
        // Whether the abstraction has said how deep it made the episode.
        let mut abstract_said = false;

        let mut frozen = net.frozen();
        // The selector, refreshed every episode. See `double`.
        let mut online = net.frozen();
        let mut buffer: Vec<Trans> = Vec::with_capacity(80_000);
        let batch = 128usize;
        let t0 = Instant::now();
        let (mut deepest_block, mut deepest_ever, mut ran) = (0usize, 0usize, 0usize);
        let mut depth_sum = 0usize;
        // **The best weights, not the last - and best means the deepest mean.**
        //
        // This wrote only at the end, and the end is not where the policy is
        // necessarily best, so a block is kept as it goes.
        //
        // What a block is judged on is the thing that took a mission to learn.
        // It was the block's **maximum**, and a maximum over a hundred episodes
        // is nearly all seed: depth in this game is heavy-tailed, so a policy
        // that cannot reach rung four prints a 13 whenever one seed in eight
        // hundred carries it there. Run r12 was written up as climbing to rung
        // 11 and forgetting; `--bin qhand` prints the same column out of a
        // *file*, which cannot learn or forget, and the mean beneath it never
        // moves. The file that milestone saved as "the best block, rung 11"
        // plays at rung 2.
        //
        // So the mean, which over the same blocks moves by two tenths of a rung
        // rather than by ten, and which separates the written control from a
        // rung-two net where the maximum does not.
        //
        // Not a greedy evaluation, and that was measured rather than assumed:
        // on identical seeds the exploration floor is worth **+0.07 of a rung**
        // against playing greedily, which is smaller than the block mean's own
        // noise and not worth the runs it would cost.
        let mut best_text: Option<(f32, String)> = None;
        // The selection window, which is not the reporting block.
        let (mut keep_sum, mut keep_n) = (0usize, 0usize);
        // Proofs written, and proofs that would not replay. The second is the
        // number worth printing: a tape that does not replay would put a run in
        // the window that never happened, and nothing else would say so.
        let (mut proofs, mut unreplayable, mut deep) = (0usize, 0usize, 0usize);
        // The deepest episode seen, for the `best.*` trio.
        let mut best_deep = 0usize;
        let (mut spread, mut spreads) = (0.0f64, 0usize);
        // **What the net is being asked to fit, against the loss fitting it.**
        //
        // `CLAUDE.md` trap 53's own closing instruction, which was written for
        // the road and never carried out here: *any new reward wants its target
        // range printed once against the loss it is being fitted with.*
        //
        // The loss is `|d| * min(|d|, knee) / knee`, which for every residual
        // under the knee is **d^2 / knee** - a squared loss divided by 120. Its
        // gradient is `2d/120`, where a squared loss gives `2d`. So if nothing
        // ever reaches the knee, the knee is not clipping anything; it is
        // quietly scaling every gradient in the run down by a factor of 120,
        // and that is a very different failure from the one trap 53 records.
        //
        // Cheap: the targets are already in hand as plain floats, and the
        // residuals are read back once an episode rather than once an update.
        // **What it built, beside how deep it got.**
        //
        // The third time this mission has needed a column to tell "learning
        // nothing" from "learning the wrong thing" - trap 54 for the road, M2
        // for the loss, and this. An item pays 1 to 3 on the press that
        // finishes it and reaching rung 2 pays `4/25 = 0.16`, so the return is
        // mostly assembly and hardly at all depth at the place this policy
        // actually lives. If items climb while the rung falls, the agent is
        // doing exactly what it was paid to do and the optimiser is innocent.
        let (mut items_paid, mut items_held) = (0usize, 0usize);
        let (mut tlo, mut thi) = (f32::MAX, f32::MIN);
        let (mut tsum, mut tn) = (0.0f64, 0usize);
        let (mut dsum, mut dn, mut dover) = (0.0f64, 0usize, 0usize);

        for ep in 0..episodes {
            let eps = (1.0 - ep as f32 / (episodes as f32 * 0.7)).clamp(0.05, 1.0);
            // Drawn either way, so the seed stream is identical with and
            // without a demonstration and the two runs stay comparable.
            let drawn = rng.next_u64();
            let following = teacher.is_some() && ep % demo_every == 0;
            let seed = match (demo_seed, following) {
                (Some(s), true) => s,
                _ => drawn,
            };
            // The chosen pair at each decision **and every pair that was on
            // offer there**, because the second is what the decision before it
            // bootstraps from. Storing only the chosen one leaves `next` empty,
            // and an empty `next` means `boot = 0` - no bootstrapping at all,
            // so the run's worth reaches the last press and nothing else.
            let mut trail: Vec<([f32; PAIR], Vec<[f32; PAIR]>)> = Vec::new();
            // What each press did, in the same order as `trail`, so a press that
            // finished an item can be paid for on the press that finished it.
            let mut presses: Vec<row::Pressed> = Vec::new();
            // One `(from, to)` a packing, into `presses`.
            let mut press_ends: Vec<(usize, usize)> = Vec::new();
            // The rung standing at each packing, which is what a redistributed
            // return is telescoped over. See `row::spread`.
            let mut rungs: Vec<usize> = Vec::new();

            let mut pack = |c: &mut Console| {
                rungs.push(c.view().rung_shown);
                let done = row::pack_with(c, PACK_BUDGET, |c, ms| {
                    let v = c.view();
                    let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                    let pairs: Vec<[f32; PAIR]> = ms
                        .iter()
                        .map(|m| match m {
                            Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                            Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                        })
                        .collect();
                    // `q_set` and not `q` a key: a menu is one board and many
                    // moves, and a dueling net's two state terms are the same
                    // for every one of them.
                    let qs: Vec<f32> = frozen.q_set(&pairs);
                    let hi = qs.iter().cloned().fold(f32::MIN, f32::max);
                    let lo = qs.iter().cloned().fold(f32::MAX, f32::min);
                    spread += (hi - lo) as f64;
                    spreads += 1;
                    // Following the demonstration: whatever the teacher would
                    // press here, with no exploration. Same function, same
                    // seed, same answer as the run this came from.
                    if let (true, Some(t)) = (following, &teacher) {
                        let i = t
                            .q_set(&pairs)
                            .into_iter()
                            .enumerate()
                            .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                            .map(|(i, _)| i)
                            .expect("not empty");
                        trail.push((pairs[i], best_of(&pairs, &qs)));
                        return i;
                    }
                    let at = if (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                        (rng.next_u64() % ms.len() as u64) as usize
                    } else {
                        qs.iter()
                            .enumerate()
                            .max_by(|a, b| a.1.partial_cmp(b.1).expect("real"))
                            .map(|(i, _)| i)
                            .expect("not empty")
                    };
                    trail.push((pairs[at], best_of(&pairs, &qs)));
                    at
                });
                // The keys, for the run's tape, before `done` is consumed by
                // the reward. A tape without the packing replays into an empty
                // board, so this is the half that makes an episode watchable.
                let keys = row::keys(&done);
                // Where this packing's presses start and end, because the churn
                // charge resets at every packing and `presses` is the whole run.
                press_ends.push((presses.len(), presses.len() + done.len()));
                presses.extend(done);
                keys
            };

            let (_c, out) = row::run(seed, mode, Difficulty::Medium, &mut pack);
            // The rung it finished standing on, which the last packing's fight
            // decided and `rungs` therefore has no entry for.
            let ended_at = _c.view().rung_shown;
            // **A demonstration is not a measurement of the policy.**
            //
            // The teacher reaches rung 18 and it plays every tenth episode, so
            // counting those would put about 1.8 of a rung into the reported
            // mean and the curve would show progress that belongs to a file.
            // Its transitions go into the buffer, which is the point of it, and
            // nothing it does is counted as something the learner did - not the
            // mean, not the items, not the deepest, and not the window the best
            // weights are chosen over.
            if !following {
                deepest_block = deepest_block.max(out.deepest);
                deepest_ever = deepest_ever.max(out.deepest);
                depth_sum += out.deepest;
                keep_sum += out.deepest;
                keep_n += 1;
                ran += 1;
            }

            // **Nothing the teacher played is captured either.**
            //
            // The statistics excluded it and the artefacts did not, so a run
            // with a demonstration every tenth episode wrote 449 deep proofs of
            // which **401 were the teacher's rung-18 run**, and `best.proof` -
            // the file that is supposed to be the run's deepest episode - was
            // the teacher at episode zero with epsilon 1.00. A demonstration
            // presented as something the learner did is the same fault as
            // counting it in the mean, one directory along.
            if let Some(dir) = &watch {
              if !following {
                // **The deepest episode of the run, in one place.**
                //
                // Every deep episode is kept below, which is a hundred and
                // seventy-seven files to sift by the end. The best one is the
                // one anybody actually asks for, so it is also written to a
                // fixed pair of names and overwritten whenever it is beaten -
                // `best.proof` to watch and `best.net` to re-derive it from,
                // because an episode is a function of its packer and the tape
                // alone cannot be replayed (see `QROW_DEMO_NET`).
                if out.deepest > best_deep {
                    best_deep = out.deepest;
                    let notes = [
                        ("episode", ep.to_string()),
                        ("epsilon", format!("{eps:.2}")),
                        ("packer", format!("{dir}/deep/best.net")),
                    ];
                    if gearmaster_lab::proof::write(
                        &format!("{dir}/deep"),
                        "best",
                        seed,
                        mode,
                        Difficulty::Medium,
                        &out.tape,
                        &out.pack_ends,
                        out.deepest,
                        &notes,
                    )
                    .is_ok()
                    {
                        std::fs::write(format!("{dir}/deep/best.net"), net.text()).ok();
                        std::fs::write(
                            format!("{dir}/deep/best.seed"),
                            format!("{seed:#018X}\n"),
                        )
                        .ok();
                    }
                }
                // The deep ones first, and they are the point of the exercise.
                if out.deepest >= watch_deep {
                    let notes = [
                        ("episode", ep.to_string()),
                        ("epsilon", format!("{eps:.2}")),
                        ("block mean", format!("{:.2}", depth_sum as f32 / ran as f32)),
                        ("packer", "learned, mid-training".to_string()),
                    ];
                    match gearmaster_lab::proof::write(
                        &format!("{dir}/deep"),
                        &format!("rung{:02}-ep{:06}", out.deepest, ep),
                        seed,
                        mode,
                        Difficulty::Medium,
                        &out.tape,
                        &out.pack_ends,
                        out.deepest,
                        &notes,
                    ) {
                        Ok(_) => {
                            deep += 1;
                            // **And the network that played it.**
                            //
                            // This run recorded a hundred and seventy-seven
                            // deep episodes and kept none of the nets that
                            // produced them, so its best - a rung 20 - could be
                            // watched and never re-derived. An episode is a
                            // function of its packer; keeping the tape without
                            // the packer keeps the output and throws away the
                            // thing that made it.
                            std::fs::write(
                                format!("{dir}/deep/rung{:02}-ep{:06}.net", out.deepest, ep),
                                net.text(),
                            )
                            .ok();
                        }
                        Err(why) => {
                            unreplayable += 1;
                            eprintln!("  deep proof refused: {why}");
                        }
                    }
                }
                if ep % watch_every == 0 {
                    // The epsilon goes in the header because a proof without
                    // one cannot be read: at 0.29 a third of what the window
                    // shows is a coin, and that is not the policy's opinion.
                    let notes = [
                        ("episode", ep.to_string()),
                        ("epsilon", format!("{eps:.2}")),
                        ("block mean", format!("{:.2}", depth_sum as f32 / ran as f32)),
                        ("packer", "learned, mid-training".to_string()),
                    ];
                    match gearmaster_lab::proof::write(
                        dir,
                        &format!("ep-{ep:06}"),
                        seed,
                        mode,
                        Difficulty::Medium,
                        &out.tape,
                        &out.pack_ends,
                        out.deepest,
                        &notes,
                    ) {
                        Ok(_) => {
                            proofs += 1;
                            gearmaster_lab::proof::prune(dir, watch_keep);
                        }
                        Err(why) => {
                            unreplayable += 1;
                            eprintln!("  proof refused: {why}");
                        }
                    }
                }
              }
            }

            // **What the run was worth, credited to every decision in it.**
            //
            // A placement at rung three is judged by the rung the run reached,
            // which is the whole reason this loop exists. The discount does the
            // apportioning: a decision near the end of the run is credited
            // almost in full and one at the start through `gamma^n`.
            let worth = row::worth(&out);
            let n = trail.len();
            // **Finishing an item pays on the spot, and only for a new high.**
            //
            // Depth is the objective and assembling is the means, and a reward
            // for the end alone is one an agent cannot climb from the bottom: a
            // run that builds its first item and still dies at rung three has
            // done something right and the depth term barely notices.
            //
            // A one-off reward is not potential-based, so it *can* change which
            // policy is best - and the obvious exploit is to sweep the board and
            // rebuild, which is exactly what a trained packer did two hundred
            // times when the empty-board floor paid better than a mediocre one.
            // Paying only when the count passes its own high makes that worth
            // nothing.
            let mut best_items = 0usize;
            let bonuses: Vec<f32> = presses
                .iter()
                .map(|p| {
                    if p.items_after > best_items {
                        best_items = p.items_after;
                        row::assembly_bonus(&p.before, &p.after)
                    } else {
                        0.0
                    }
                })
                .collect();
            // Counted here rather than above, because `best_items` is the
            // reward's own high-water mark and is not known until the bonuses
            // have been walked. Guarded the same way: the teacher's items are
            // the teacher's.
            if !following {
                items_paid += best_items;
                items_held += presses.last().map(|p| p.items_after).unwrap_or(0);
            }
            // The churn charge, packing by packing, laid back over the run.
            let mut churn = vec![0.0f32; presses.len()];
            for &(from, to) in &press_ends {
                for (k, c) in row::churn_penalty(&presses[from..to]).into_iter().enumerate() {
                    churn[from + k] = c;
                }
            }
            // **The run's worth, paid where it was earned.** One payment a
            // packing out of `row::spread`, laid onto the last transition of
            // that packing - the decision that produced the board its fight was
            // won with. A packing that pressed nothing carries its payment
            // forward, and whatever is left lands on the final transition, so
            // the episode's total is `worth` however the packings fell.
            let mut paid = vec![0.0f32; n];
            if redistribute() && n > 0 {
                let per = row::spread(&rungs, ended_at, worth);
                let mut owed = 0.0f32;
                for (p, amount) in per.iter().enumerate() {
                    owed += amount;
                    let (from, to) = press_ends.get(p).copied().unwrap_or((0, 0));
                    if to > from && to <= n {
                        paid[to - 1] += owed;
                        owed = 0.0;
                    }
                }
                paid[n - 1] += owed;
                // **Return-equivalence, checked rather than assumed.** The
                // licence for redistributing at all is that the episode's total
                // is unchanged; if it drifts, the agent is being trained on a
                // different objective than the one being reported and no curve
                // would say so. Once, on the first episode that has one.
                if !redist_checked {
                    redist_checked = true;
                    let got: f32 = paid.iter().sum();
                    println!(
                        "  redistribution: {} packings, worth {worth:+.4}, paid {got:+.4}, residual {:+.2e}",
                        rungs.len(),
                        got - worth
                    );
                    assert!(
                        (got - worth).abs() < 1e-2,
                        "the redistribution is not return-equivalent: {got} against {worth}"
                    );
                }
            }
            // Every press's own reward, before the backup decides how far it
            // reaches.
            let rew: Vec<f32> = (0..n)
                .map(|i| {
                    bonuses.get(i).copied().unwrap_or(0.0)
                        + churn.get(i).copied().unwrap_or(0.0)
                        + if redistribute() {
                            paid[i]
                        } else if i + 1 == n {
                            worth
                        } else {
                            -NOTHING
                        }
                })
                .collect();
            // Which packing each press belongs to, and where that packing ends.
            let mut ends_at = vec![n; n];
            if abstracted() {
                for &(from, to) in &press_ends {
                    for e in ends_at.iter_mut().take(to.min(n)).skip(from) {
                        *e = to.min(n);
                    }
                }
            }
            for i in 0..n {
                let x = trail[i].0;
                if abstracted() {
                    // **The semi-MDP backup.** Everything this packing pays
                    // from here to its end, actually realised, and then the
                    // value at the start of the next packing discounted by how
                    // many presses that took. The last packing has nothing
                    // after it, which is what makes it terminal.
                    let to = ends_at[i];
                    let mut r = 0.0f32;
                    let mut g = 1.0f32;
                    for j in i..to {
                        r += g * rew[j];
                        g *= GAMMA;
                    }
                    let next = if to < n { trail[to].1.clone() } else { Vec::new() };
                    buffer.push(Trans { x, r, g, next });
                    continue;
                }
                // **What was on offer at the next decision.** The last one has
                // nothing after it, which is what makes it terminal and what
                // stops the run's worth being bootstrapped out of existence.
                let next = if i + 1 < n { trail[i + 1].1.clone() } else { Vec::new() };
                buffer.push(Trans { x, r: rew[i], g: GAMMA, next });
            }
            if abstracted() && !abstract_said {
                abstract_said = true;
                let lens: Vec<usize> = press_ends.iter().map(|&(f, t)| t - f).collect();
                let mean = lens.iter().sum::<usize>() as f32 / lens.len().max(1) as f32;
                println!(
                    "  abstraction: {} packings, {mean:.1} presses each, so the bootstrap \
                     carries gamma^{mean:.0} = {:.3} and the episode is {} backups deep \
                     rather than {n}",
                    lens.len(),
                    GAMMA.powf(mean),
                    lens.len()
                );
            }
            if buffer.len() > 80_000 {
                buffer.drain(0..20_000);
            }

            if double {
                online = net.frozen();
            }
            for u in 0..updates {
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
                    } else if double {
                        // Chosen by one, valued by the other.
                        //
                        // Scored once each, by hand. `max_by` calls its
                        // comparator O(n) times and a closure that scores both
                        // sides evaluates the network **twice per comparison** -
                        // thirty forward passes over sixteen candidates instead
                        // of sixteen, which measured as 4.3 s an episode against
                        // 1.9. A network is not a cheap key.
                        let (mut pick, mut best) = (0usize, f32::MIN);
                        for (i, q) in online.q_set(&s.next).into_iter().enumerate() {
                            if q > best {
                                best = q;
                                pick = i;
                            }
                        }
                        frozen.q(&s.next[pick])
                    } else {
                        s.next.iter().map(|p| frozen.q(p)).fold(f32::MIN, f32::max)
                    };
                    ys.push(s.r + s.g * boot);
                }
                for &t in &ys {
                    tlo = tlo.min(t);
                    thi = thi.max(t);
                    tsum += t as f64;
                    tn += 1;
                }
                let x = Tensor::<B, 2>::from_data(TensorData::new(xs, [batch, PAIR]), &dev);
                let y = Tensor::<B, 2>::from_data(TensorData::new(ys, [batch, 1]), &dev);
                let out = net.forward(x);
                let d = out.sub(y);
                // Once an episode, not once an update: reading a tensor back
                // costs a synchronisation and this is a measurement, not a step.
                if u == 0 {
                    for r in d
                        .clone()
                        .inner()
                        .to_data()
                        .convert::<f32>()
                        .into_vec::<f32>()
                        .expect("its own residuals")
                    {
                        dsum += r.abs() as f64;
                        dn += 1;
                        if r.abs() > knee {
                            dover += 1;
                        }
                    }
                }
                let loss = d.clone().abs().clamp(0.0, knee).mul(d.abs()).div_scalar(knee).mean();
                let grads = loss.backward();
                let step = |p: &mut Tensor<B, 2>| {
                    if let Some(gr) = p.grad(&grads) {
                        *p = Tensor::from_inner(p.clone().inner().sub(gr.mul_scalar(lr)))
                            .require_grad();
                    }
                };
                for p in net.adv.each() {
                    step(p);
                }
                if let Some(v) = net.val.as_mut() {
                    for p in v.each() {
                        step(p);
                    }
                }
            }

            if ep % 50 == 49 {
                frozen = net.frozen();
            }
            // The weights are chosen on their own window, so the reporting
            // block can be made as fine as anybody wants without moving what
            // gets saved.
            if keep_n >= KEEP_OVER {
                let over = keep_sum as f32 / keep_n as f32;
                if best_text.as_ref().is_none_or(|(m, _)| over > *m) {
                    best_text = Some((over, net.text()));
                }
                keep_sum = 0;
                keep_n = 0;
            }

            if ep % BLOCK == 0 || ep + 1 == episodes {
                let mean = depth_sum as f32 / ran.max(1) as f32;
                println!(
                    "  episode {:>5}   eps {:.2}   buffer {:>6}   mean rung {:>5.2}   \
                     items {:>4.2} paid {:>4.2} held   deepest {:>3} (ever {:>3})   spread {:>6.3}",
                    ep,
                    eps,
                    buffer.len(),
                    mean,
                    items_paid as f32 / ran.max(1) as f32,
                    items_held as f32 / ran.max(1) as f32,
                    deepest_block,
                    deepest_ever,
                    spread / spreads.max(1) as f64
                );
                // The second line: what it is fitting, and what the loss does
                // with the difference.
                let over = 100.0 * dover as f64 / dn.max(1) as f64;
                println!(
                    "       targets {:+.2}..{:+.2} mean {:+.3}   residual mean {:.3}   \
                     past the knee of {:.0}: {:.1}%   gradient 1/{:.0} of a squared loss",
                    if tn == 0 { 0.0 } else { tlo },
                    if tn == 0 { 0.0 } else { thi },
                    tsum / tn.max(1) as f64,
                    dsum / dn.max(1) as f64,
                    knee,
                    over,
                    knee
                );
                deepest_block = 0;
                depth_sum = 0;
                items_paid = 0;
                items_held = 0;
                ran = 0;
                spread = 0.0;
                spreads = 0;
                tlo = f32::MAX;
                thi = f32::MIN;
                tsum = 0.0;
                tn = 0;
                dsum = 0.0;
                dn = 0;
                dover = 0;
            }
        }
        println!("trained in {:.1}s", t0.elapsed().as_secs_f64());
        if watch.is_some() {
            println!(
                "  {proofs} sampled proofs and {deep} deep ones written, \
                 {unreplayable} refused for not replaying\n  \
                 deepest episode kept as best.proof / best.net / best.seed \
                 (rung {best_deep})"
            );
        }
        std::fs::create_dir_all("runs").ok();
        // The last weights, and the best ones. A collapse at the exploration
        // floor is a real thing this loop does, so the run keeps both and says
        // which is which rather than quietly handing over whichever it ended on.
        std::fs::write(&out_last, net.text()).unwrap();
        match &best_text {
            Some((m, t)) => {
                std::fs::write(&out_best, t).unwrap();
                println!(
                    "wrote {out_best} (best {KEEP_OVER} episodes, \
                     mean rung {m:.2}) and {out_last} (final)"
                );
            }
            None => {
                std::fs::write(&out_best, net.text()).unwrap();
                println!("wrote {out_best}");
            }
        }
    }
}
