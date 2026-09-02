//! RUDDER's contribution analysis, asked what it would redistribute onto.
//!
//!     cargo run --release -p gearmaster-lab --features nn --bin qrudder
//!
//! `analysis/the-action-gap.md` G redistributed the episode's return over
//! **rungs** - exactly, in closed form, because `deepest^2` telescopes - and it
//! was a wash on depth and negative on the action gap. The doc comment on
//! `row::spread` predicted that before the run: paying at the rung is not
//! paying at the decision, and A.1 measures 78% of rungs as coming out the same
//! whatever was pressed at them.
//!
//! RUDDER proper redistributes onto the state-action pairs a **learned** model
//! says predicted the return, which could in principle put the credit on the
//! press that built the item that won rung nine. That is the thing none of the
//! four arms has touched.
//!
//! ## Why this is a diagnostic and not a training run
//!
//! A contribution analysis cannot manufacture signal that is not in the data.
//! If the learned redistribution turns out to be concentrated at the presses
//! where the rung changes, then it has rediscovered G's closed form and G has
//! already measured what that is worth - and two more hours of training would
//! produce a number this binary can predict in ninety seconds.
//!
//! So the order is: fit the predictor, ask where its credit lands, and only
//! then decide whether an arm is justified. `CLAUDE.md` trap 51 and
//! `analysis/the-collapse.md` M3 are both this rule, learned the expensive way.
//!
//! ## And why the predictor is Markov rather than an LSTM
//!
//! **Measured, not assumed.** RUDDER uses an LSTM because Atari is partially
//! observed and a prefix carries information the current frame does not. This
//! game is fully observed - the board, the purse, the rung and the lives *are*
//! the state - so a prefix should tell a forecaster nothing the last state does
//! not already say. The `R2` this prints is that claim's test: a Markov
//! predictor that explains the realised return is a sequence model with nothing
//! left to do. If it comes back low, the LSTM is justified and this binary says
//! so.
//!
//! With a Markov predictor the contribution analysis is
//! `R_t = r_t + V(s_t) - V(s_{t-1})`, which telescopes to the episode's return
//! less a constant `V(s_0)` - return-equivalent, and potential-based, so it
//! cannot change which policy is best (`CLAUDE.md` trap 49 is the same
//! machinery and the pitfall it carries).

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
    use gearmaster_console::{Console, Difficulty, Mode, Verb};
    use gearmaster_engine::rng::Rng;
    use gearmaster_lab::row;
    use gearmaster_trades::brief::{Brief, BRIEF};
    use gearmaster_trades::env::Move;
    use gearmaster_trades::feature::{self, BOARD};
    use gearmaster_trades::QNet;

    type B = Autodiff<NdArray>;
    type Dev = <B as burn::tensor::backend::Backend>::Device;

    /// What the forecaster reads: the state, without the move.
    ///
    /// The return is a property of the situation and not of the key about to be
    /// pressed - `V` and not `Q` - so the move band is left off and the width is
    /// the state half of a pair.
    const WIDE: usize = BOARD + BRIEF;
    const HIDDEN: usize = 64;
    const GAMMA: f32 = 0.999;
    const ROW_SEED: u64 = 0x0D0E_5EED;
    const PACK_BUDGET: usize = 40;

    fn init(rng: &mut Rng, r: usize, c: usize) -> Vec<f32> {
        let scale = (2.0 / r as f32).sqrt();
        (0..r * c)
            .map(|_| ((rng.next_u64() >> 11) as f32 / (1u64 << 53) as f32 - 0.5) * 2.0 * scale)
            .collect()
    }
    fn mat(v: Vec<f32>, r: usize, c: usize, d: &Dev) -> Tensor<B, 2> {
        Tensor::<B, 2>::from_data(TensorData::new(v, [r, c]), d).require_grad()
    }

    struct Fore {
        w1: Tensor<B, 2>,
        b1: Tensor<B, 2>,
        w2: Tensor<B, 2>,
        b2: Tensor<B, 2>,
        w3: Tensor<B, 2>,
        b3: Tensor<B, 2>,
    }

    impl Fore {
        fn new(rng: &mut Rng, d: &Dev) -> Fore {
            Fore {
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
        fn each(&mut self) -> [&mut Tensor<B, 2>; 6] {
            [&mut self.w1, &mut self.b1, &mut self.w2, &mut self.b2, &mut self.w3, &mut self.b3]
        }
        /// A whole episode's states at once, back in the return's own units.
        ///
        /// **One forward pass an episode and not one a press.** The first
        /// version called `at` thirty-one thousand times and spent an hour in
        /// it: this backend is an autodiff one, so every call builds a graph
        /// over the parameters whether or not anybody differentiates it. A
        /// diagnostic that costs more than the training run it is diagnosing is
        /// a diagnostic nobody runs.
        fn over(&self, xs: &[[f32; WIDE]], d: &Dev, n: &Norm) -> Vec<f32> {
            if xs.is_empty() {
                return Vec::new();
            }
            let rows = xs.len();
            let mut flat = Vec::with_capacity(rows * WIDE);
            for x in xs {
                flat.extend_from_slice(x);
            }
            self.forward(Tensor::<B, 2>::from_data(TensorData::new(flat, [rows, WIDE]), d))
                .inner()
                .to_data()
                .convert::<f32>()
                .into_vec::<f32>()
                .expect("a number a row")
                .into_iter()
                .map(|v| n.back(v))
                .collect()
        }

    }

    /// One press of one episode, with everything the analysis needs about it.
    struct Step {
        x: [f32; WIDE],
        /// The reward the environment actually paid here.
        r: f32,
        /// Discounted return from here to the end of the episode, realised.
        to_go: f32,
        verb: Option<Verb>,
        /// Whether this press was the last of its packing - the one the fight
        /// that follows is fought with.
        last_of_packing: bool,
        /// Whether the fight after this packing moved the rung.
        won_its_rung: bool,
        /// Whether this press finished an item that had not stood before.
        built: bool,
        rung: usize,
    }

    /// The scale the regression is done on.
    ///
    /// **The first fit diverged to `NaN` and this is why.** A return-to-go here
    /// runs to `deepest^2` - four hundred for a run that reached rung twenty -
    /// and a plain squared loss on a target of four hundred is a gradient of
    /// eight hundred. `analysis/the-collapse.md` M2 is the same lesson from the
    /// other side, where a knee sized for a rung nobody reaches scaled every
    /// gradient away; this is the version where nothing bounds it at all.
    ///
    /// Standardising is safe here in a way it would not be for a TD target: the
    /// forecaster is a **supervised regression** on a fixed set of labels, so
    /// centring and scaling them is a change of units and not a change of
    /// objective. The contribution analysis reads predictions back in the
    /// return's own units, so nothing downstream sees the scaling.
    struct Norm {
        mean: f32,
        sd: f32,
    }

    impl Norm {
        fn of(v: &[f32]) -> Norm {
            let n = v.len().max(1) as f32;
            let mean = v.iter().sum::<f32>() / n;
            let sd = (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n).sqrt();
            Norm { mean, sd: if sd > 1e-6 { sd } else { 1.0 } }
        }
        fn to(&self, y: f32) -> f32 {
            (y - self.mean) / self.sd
        }
        fn back(&self, y: f32) -> f32 {
            y * self.sd + self.mean
        }
    }

    fn kind(v: Option<Verb>) -> &'static str {
        match v {
            Some(Verb::Place { .. }) => "place",
            Some(Verb::Buy { .. }) => "buy",
            Some(Verb::Sell { .. }) => "sell",
            Some(Verb::Unequip { .. }) | Some(Verb::UnequipLocked { .. }) => "unequip",
            Some(Verb::ClearSlot { .. }) | Some(Verb::ClearAll) => "clear",
            Some(Verb::Lock { .. }) => "lock",
            Some(Verb::Undo) => "undo",
            Some(Verb::Pin { .. }) => "pin",
            Some(Verb::Reroll) => "reroll",
            Some(_) => "other",
            None => "done",
        }
    }

    /// Play one episode and record every press.
    fn episode(seed: u64, net: Option<&QNet>, eps: f32, rng: &mut Rng) -> Vec<Step> {
        let mut xs: Vec<[f32; WIDE]> = Vec::new();
        let mut presses: Vec<row::Pressed> = Vec::new();
        let mut ends: Vec<(usize, usize)> = Vec::new();
        let mut rungs: Vec<usize> = Vec::new();
        {
            let (xs, presses, ends, rungs) = (&mut xs, &mut presses, &mut ends, &mut rungs);
            let mut pack = |c: &mut Console| {
                rungs.push(c.view().rung_shown);
                let done = row::pack_with(c, PACK_BUDGET, |c, ms| {
                    let v = c.view();
                    let b = feature::briefed(&feature::board(&v), &Brief::NONE);
                    xs.push(b);
                    if eps > 0.0 && (rng.next_u64() % 1000) as f32 / 1000.0 < eps {
                        return (rng.next_u64() % ms.len() as u64) as usize;
                    }
                    let Some(net) = net else {
                        return (rng.next_u64() % ms.len() as u64) as usize;
                    };
                    let pairs: Vec<[f32; feature::PAIR]> = ms
                        .iter()
                        .map(|m| match m {
                            Move::Press(verb) => feature::pair(&b, &feature::mv(&v, *verb)),
                            Move::Done => feature::pair(&b, &[0.0; feature::MOVE]),
                        })
                        .collect();
                    net.q_set(&pairs)
                        .into_iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).expect("real"))
                        .map(|(i, _)| i)
                        .expect("not empty")
                });
                ends.push((presses.len(), presses.len() + done.len()));
                presses.extend(done);
                // The tape is not used here - this binary reads the presses,
                // not a proof - so nothing is handed back for one.
                Vec::new()
            };
            let _ = row::run(seed, Mode::Rogue, Difficulty::Medium, &mut pack);
        }
        let n = presses.len().min(xs.len());
        if n == 0 {
            return Vec::new();
        }

        // The reward the environment paid, press by press: the assembly bonus on
        // a new high, and the run's worth on the last press. The same two terms
        // `qrow` assembles, because a redistribution measured against a
        // different reward than the one being trained is measuring nothing.
        let mut best_items = 0usize;
        let mut built = vec![false; n];
        let mut r = vec![0.0f32; n];
        for i in 0..n {
            let p = &presses[i];
            if p.items_after > best_items {
                best_items = p.items_after;
                built[i] = true;
                r[i] += row::assembly_bonus(&p.before, &p.after);
            }
        }
        // `row::worth` wants a `Ran`; the depth is the deepest rung stood on.
        let deepest = rungs.iter().copied().max().unwrap_or(1);
        let losses = (0..rungs.len())
            .filter(|&p| {
                let after = rungs.get(p + 1).copied().unwrap_or(deepest);
                after <= rungs[p]
            })
            .count();
        r[n - 1] += (deepest as f32).powf(row::pow()) * row::RUNG - losses as f32 * row::LIFE;

        // Realised discounted return from each press to the end.
        let mut to_go = vec![0.0f32; n];
        let mut acc = 0.0f32;
        for i in (0..n).rev() {
            acc = r[i] + GAMMA * acc;
            to_go[i] = acc;
        }

        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let p = ends.iter().position(|&(f, t)| i >= f && i < t).unwrap_or(0);
            let last = ends.get(p).map(|&(_, t)| i + 1 == t).unwrap_or(false);
            let after = rungs.get(p + 1).copied().unwrap_or(deepest);
            out.push(Step {
                x: xs[i],
                r: r[i],
                to_go: to_go[i],
                verb: presses[i].verb,
                last_of_packing: last,
                won_its_rung: after > rungs.get(p).copied().unwrap_or(1),
                built: built[i],
                rung: rungs.get(p).copied().unwrap_or(1),
            });
        }
        out
    }

    pub fn run() {
        let episodes: usize =
            std::env::var("QRUDDER_EPISODES").ok().and_then(|v| v.parse().ok()).unwrap_or(120);
        let epochs: usize =
            std::env::var("QRUDDER_EPOCHS").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
        let lr: f32 =
            std::env::var("QRUDDER_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.02);
        let path = std::env::var("QRUDDER_NET")
            .unwrap_or_else(|_| "runs/duel-control.txt".to_string());
        let net = match QNet::load_at(&path, feature::PAIR) {
            Ok(n) => Some(n),
            Err(why) => {
                println!("  {why}\n  collecting under a uniform-random packer instead");
                None
            }
        };

        println!(
            "\n  RUDDER's contribution analysis, asked where it would put the credit.\n  \
             {episodes} episodes under {path}, eps 0.05."
        );

        let mut rng = Rng::new(ROW_SEED ^ 0x0000_5DDE);
        let mut seeds = Rng::new(ROW_SEED);
        let mut eps_out: Vec<Vec<Step>> = Vec::new();
        for _ in 0..episodes {
            let e = episode(seeds.next_u64(), net.as_ref(), 0.05, &mut rng);
            if !e.is_empty() {
                eps_out.push(e);
            }
        }
        let total: usize = eps_out.iter().map(|e| e.len()).sum();
        println!("  {} episodes, {total} presses", eps_out.len());

        // A held-out fifth, so `R2` is a forecast rather than a memory.
        let cut = eps_out.len() * 4 / 5;
        let (train, test) = eps_out.split_at(cut);
        let flat: Vec<&Step> = train.iter().flatten().collect();
        let held: Vec<&Step> = test.iter().flatten().collect();

        let dev: Dev = Default::default();
        let mut f = Fore::new(&mut rng, &dev);
        let norm = Norm::of(&flat.iter().map(|s| s.to_go).collect::<Vec<_>>());
        println!("  returns to go: mean {:+.2}, sd {:.2}", norm.mean, norm.sd);
        let batch = 256usize;
        for e in 0..epochs {
            for _ in 0..(flat.len() / batch).max(1) {
                let mut xs = Vec::with_capacity(batch * WIDE);
                let mut ys = Vec::with_capacity(batch);
                for _ in 0..batch {
                    let s = flat[(rng.next_u64() % flat.len() as u64) as usize];
                    xs.extend_from_slice(&s.x);
                    ys.push(norm.to(s.to_go));
                }
                let x = Tensor::<B, 2>::from_data(TensorData::new(xs, [batch, WIDE]), &dev);
                let y = Tensor::<B, 2>::from_data(TensorData::new(ys, [batch, 1]), &dev);
                let d = f.forward(x).sub(y);
                let loss = d.clone().mul(d).mean();
                let grads = loss.backward();
                for p in f.each() {
                    if let Some(g) = p.grad(&grads) {
                        *p = Tensor::from_inner(p.clone().inner().sub(g.mul_scalar(lr)))
                            .require_grad();
                    }
                }
            }
            if e + 1 == epochs || e == 0 {
                let r2 = score(&f, &held, &dev, &norm);
                println!("  epoch {:>3}   held-out R2 {r2:+.3}", e + 1);
            }
        }

        // **The control.** `R_t` is a difference of two forecasts on states one
        // press apart, so most of what it measures could be the difference of
        // two prediction *errors* rather than any contribution at all. An
        // untrained forecaster has only error, so if it concentrates the credit
        // the same way the trained one does, this whole analysis is reporting
        // noise and says nothing about RUDDER.
        //
        // The same shape as the brief band reading exactly 0.00% in B.1: a row
        // whose answer is known in advance is what makes the other rows mean
        // something.
        let untrained = Fore::new(&mut rng, &dev);
        report("an untrained forecaster (the control)", &untrained, &eps_out, &dev, &norm, total);
        report("RUDDER's contribution analysis", &f, &eps_out, &dev, &norm, total);
    }

    /// Where a forecaster's credit lands.
    fn report(
        what: &str,
        f: &Fore,
        eps_out: &[Vec<Step>],
        dev: &Dev,
        norm: &Norm,
        total: usize,
    ) {
        // `R_t = r_t + V(s_t) - V(s_{t-1})`, which is the contribution analysis
        // with a Markov forecaster. Where does the mass land?
        let mut by_kind: std::collections::BTreeMap<&'static str, (f64, usize)> =
            Default::default();
        let (mut at_rung_change, mut elsewhere) = (0.0f64, 0.0f64);
        let (mut on_built, mut not_built) = (0.0f64, 0.0f64);
        let (mut last_press, mut mid_press) = (0.0f64, 0.0f64);
        let mut by_rung: Vec<(f64, usize)> = vec![(0.0, 0); 24];
        let mut all: Vec<f64> = Vec::new();
        for ep in eps_out {
            let xs: Vec<[f32; WIDE]> = ep.iter().map(|s| s.x).collect();
            let vs = f.over(&xs, dev, norm);
            let mut prev = 0.0f32;
            for (i, s) in ep.iter().enumerate() {
                let v = if i + 1 == ep.len() { 0.0 } else { vs[i] };
                let contribution = (s.r + v - prev) as f64;
                prev = v;
                let m = contribution.abs();
                all.push(m);
                let e = by_kind.entry(kind(s.verb)).or_insert((0.0, 0));
                e.0 += m;
                e.1 += 1;
                if s.last_of_packing && s.won_its_rung {
                    at_rung_change += m;
                } else {
                    elsewhere += m;
                }
                if s.built {
                    on_built += m;
                } else {
                    not_built += m;
                }
                if s.last_of_packing {
                    last_press += m;
                } else {
                    mid_press += m;
                }
                let r = s.rung.min(by_rung.len() - 1);
                by_rung[r].0 += m;
                by_rung[r].1 += 1;
            }
        }
        let sum: f64 = all.iter().sum();
        let built_n: usize = eps_out.iter().flatten().filter(|s| s.built).count();
        let change_n: usize =
            eps_out.iter().flatten().filter(|s| s.last_of_packing && s.won_its_rung).count();
        let last_n: usize = eps_out.iter().flatten().filter(|s| s.last_of_packing).count();

        println!("\n================ {what}");
        println!("  mean |R| over every press: {:.4}", sum / total.max(1) as f64);
        println!("  where the credit lands, as a share of all |R_t| over {total} presses");
        let line = |what: &str, mass: f64, n: usize| {
            println!(
                "    {what:<38} {:>6.1}% of the mass on {:>5.1}% of the presses   ({:.1}x)",
                100.0 * mass / sum.max(1e-9),
                100.0 * n as f32 / total as f32,
                (mass / sum.max(1e-9)) / (n as f64 / total as f64).max(1e-9)
            );
        };
        line("presses that won their rung", at_rung_change, change_n);
        line("the last press of any packing", last_press, last_n);
        line("presses that finished an item", on_built, built_n);
        let _ = (elsewhere, not_built, mid_press);

        println!("\n    {:<12} {:>9} {:>12} {:>10}", "key", "presses", "mean |R|", "share");
        for (k, (mass, n)) in &by_kind {
            println!(
                "    {k:<12} {n:>9} {:>12.4} {:>9.1}%",
                mass / *n as f64,
                100.0 * mass / sum.max(1e-9)
            );
        }
        println!("\n    {:<6} {:>9} {:>12}", "rung", "presses", "mean |R|");
        for (r, (mass, n)) in by_rung.iter().enumerate() {
            if *n == 0 {
                continue;
            }
            println!("    {r:<6} {n:>9} {:>12.4}", mass / *n as f64);
        }
    }

    /// One minus the residual sum of squares over the total sum of squares.
    fn score(f: &Fore, held: &[&Step], dev: &Dev, n: &Norm) -> f32 {
        if held.is_empty() {
            return f32::NAN;
        }
        let mean = held.iter().map(|s| s.to_go).sum::<f32>() / held.len() as f32;
        let take: Vec<&&Step> = held.iter().take(4000).collect();
        let xs: Vec<[f32; WIDE]> = take.iter().map(|s| s.x).collect();
        let ps = f.over(&xs, dev, n);
        let (mut ss_res, mut ss_tot) = (0.0f64, 0.0f64);
        for (s, p) in take.iter().zip(ps) {
            ss_res += ((p - s.to_go) * (p - s.to_go)) as f64;
            ss_tot += ((s.to_go - mean) * (s.to_go - mean)) as f64;
        }
        (1.0 - ss_res / ss_tot.max(1e-9)) as f32
    }
}
