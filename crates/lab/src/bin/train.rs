//! Train a prior over seats.
//!
//!     cargo run --release -p gearmaster-lab --features nn --bin train
//!
//! Behind `--features nn`, so `cargo test --workspace` never compiles a line
//! of it. The model is a three-layer perceptron over the twenty-four numbers
//! that describe a candidate seat, trained on what the hands measured -
//! **supervised on the pilot's own objective**, which is what makes it a prior
//! over the search rather than a second opinion about the game.
//!
//! Written against burn's tensor API rather than its `Module` derive and
//! `Learner`: the model is six tensors and a manual step, which is less to go
//! wrong across versions and short enough to read in one sitting.

#[cfg(not(feature = "nn"))]
fn main() {
    eprintln!("built without --features nn");
}

#[cfg(feature = "nn")]
fn main() {
    nn::run();
}

#[cfg(feature = "nn")]
mod nn {
    use burn::backend::{Autodiff, NdArray};
    use burn::tensor::activation::relu;
    use burn::tensor::{backend::AutodiffBackend, Tensor, TensorData};
    use gearmaster_agent::lesson::FEATURES;
    use std::time::Instant;

    type B = Autodiff<NdArray>;
    const HIDDEN: usize = 64;

    fn read() -> (Vec<f32>, Vec<f32>, usize) {
        let bytes = std::fs::read("runs/lessons.bin").expect("run --bin lessons first");
        let row = (FEATURES + 1) * 4;
        let n = bytes.len() / row;
        let mut xs = Vec::with_capacity(n * FEATURES);
        let mut ys = Vec::with_capacity(n);
        for i in 0..n {
            let at = i * row;
            for j in 0..FEATURES {
                let b = &bytes[at + j * 4..at + j * 4 + 4];
                xs.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            }
            let b = &bytes[at + FEATURES * 4..at + FEATURES * 4 + 4];
            ys.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        }
        (xs, ys, n)
    }

    /// Xavier-ish, from the engine's own PRNG so a training run replays.
    fn init(rng: &mut gearmaster_engine::rng::Rng, r: usize, c: usize) -> Vec<f32> {
        let scale = (2.0 / r as f32).sqrt();
        (0..r * c)
            .map(|_| ((rng.next_u64() >> 11) as f32 / (1u64 << 53) as f32 - 0.5) * 2.0 * scale)
            .collect()
    }

    fn mat(v: Vec<f32>, r: usize, c: usize, dev: &<B as burn::tensor::backend::Backend>::Device)
        -> Tensor<B, 2>
    {
        Tensor::<B, 2>::from_data(TensorData::new(v, [r, c]), dev).require_grad()
    }

    pub fn run() {
        let dev = Default::default();
        let (xs, ys, n) = read();
        println!("{} lessons, {} features", n, FEATURES);

        // Hold out a fifth, by position - the lessons are written in the order
        // they were learned, so the tail is boards the model has not seen.
        let split = n * 4 / 5;
        let x_tr = Tensor::<B, 2>::from_data(
            TensorData::new(xs[..split * FEATURES].to_vec(), [split, FEATURES]),
            &dev,
        );
        let y_tr = Tensor::<B, 2>::from_data(
            TensorData::new(ys[..split].to_vec(), [split, 1]),
            &dev,
        );
        let x_te = Tensor::<B, 2>::from_data(
            TensorData::new(xs[split * FEATURES..].to_vec(), [n - split, FEATURES]),
            &dev,
        );
        let y_te = Tensor::<B, 2>::from_data(
            TensorData::new(ys[split..].to_vec(), [n - split, 1]),
            &dev,
        );

        let mut rng = gearmaster_engine::rng::Rng::new(0x5EA7_5EED);
        let mut w1 = mat(init(&mut rng, FEATURES, HIDDEN), FEATURES, HIDDEN, &dev);
        let mut b1 = mat(vec![0.0; HIDDEN], 1, HIDDEN, &dev);
        let mut w2 = mat(init(&mut rng, HIDDEN, HIDDEN), HIDDEN, HIDDEN, &dev);
        let mut b2 = mat(vec![0.0; HIDDEN], 1, HIDDEN, &dev);
        let mut w3 = mat(init(&mut rng, HIDDEN, 1), HIDDEN, 1, &dev);
        let mut b3 = mat(vec![0.0; 1], 1, 1, &dev);

        let forward = |x: Tensor<B, 2>,
                       w1: &Tensor<B, 2>,
                       b1: &Tensor<B, 2>,
                       w2: &Tensor<B, 2>,
                       b2: &Tensor<B, 2>,
                       w3: &Tensor<B, 2>,
                       b3: &Tensor<B, 2>| {
            let rows = x.dims()[0];
            let h = relu(x.matmul(w1.clone()).add(b1.clone().repeat_dim(0, rows)));
            let h = relu(h.matmul(w2.clone()).add(b2.clone().repeat_dim(0, rows)));
            h.matmul(w3.clone()).add(b3.clone().repeat_dim(0, rows))
        };

        let epochs: usize =
            std::env::var("TRAIN_EPOCHS").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
        let lr: f32 = std::env::var("TRAIN_LR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let t0 = Instant::now();
        for epoch in 0..epochs {
            let out = forward(x_tr.clone(), &w1, &b1, &w2, &b2, &w3, &b3);
            let diff = out.sub(y_tr.clone());
            let loss = diff.clone().powf_scalar(2.0).mean();
            let grads = loss.backward();

            let step = |p: &mut Tensor<B, 2>, g: &<B as AutodiffBackend>::Gradients| {
                if let Some(grad) = p.grad(g) {
                    let updated = p.clone().inner().sub(grad.mul_scalar(lr));
                    *p = Tensor::from_inner(updated).require_grad();
                }
            };
            step(&mut w1, &grads);
            step(&mut b1, &grads);
            step(&mut w2, &grads);
            step(&mut b2, &grads);
            step(&mut w3, &grads);
            step(&mut b3, &grads);

            if epoch % 10 == 0 || epoch + 1 == epochs {
                let held = forward(x_te.clone(), &w1, &b1, &w2, &b2, &w3, &b3)
                    .sub(y_te.clone())
                    .powf_scalar(2.0)
                    .mean()
                    .into_scalar();
                println!(
                    "  epoch {:>3}   train {:.5}   held out {:.5}",
                    epoch,
                    loss.into_scalar(),
                    held
                );
            }
        }
        println!("trained in {:.1}s on {}", t0.elapsed().as_secs_f64(), "Autodiff<NdArray>");

        // The weights, as plain floats, so the pilot can read them without
        // linking a framework. That is the point: training is privileged and
        // inference is not.
        std::fs::create_dir_all("runs").ok();
        let mut out = String::new();
        for (name, t) in [
            ("w1", &w1), ("b1", &b1), ("w2", &w2), ("b2", &b2), ("w3", &w3), ("b3", &b3),
        ] {
            let d = t.clone().inner().to_data();
            let v: Vec<f32> = d.convert::<f32>().into_vec().unwrap();
            out.push_str(name);
            for x in v {
                out.push(' ');
                out.push_str(&format!("{:.6}", x));
            }
            out.push('\n');
        }
        std::fs::write("runs/prior.txt", out).unwrap();
        println!("wrote runs/prior.txt");
    }
}
