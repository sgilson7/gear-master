//! What the catalogue does with the eight pools, and whether the features
//! carry it.
//!
//!     cargo run --release -p gearmaster-lab --bin pools
//!
//! Q1's two deliverables. The census says what exists; the **probe** says
//! whether a model over the board features could ever notice it - which is the
//! gate, because if the state cannot express "this makes nature and that
//! spends it", no amount of Q-learning discovers the build. It learns the
//! current blindness faster.

use gearmaster_console::view::{BoardPools, POOLS};
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::combat::{simulate_at, Event, Side, LADDER};
use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::run::Run;

/// A board standing in a run, so the console can draw it.
fn stand(defs: &[(usize, SlotKind, u8, u8, u8)]) -> Console {
    let board = gearmaster_oracle::Board {
        gear: defs.to_vec(),
        chunks: Vec::new(),
        rows: [0; 5],
    };
    let (reg, lo) = board.rebuild();
    let mut run = Run::new();
    run.clear_all();
    run.owned.clear();
    run.registry = reg;
    run.loadout = lo;
    run.owned = SlotKind::ALL.into_iter().flat_map(|k| run.loadout.slot(k).pieces()).collect();
    Console::standing_in(run, 0)
}

/// Random boards, drawn from the whole catalogue, seated wherever they fit.
fn sample(rng: &mut Rng, pieces: usize) -> Console {
    let mut gear: Vec<(usize, SlotKind, u8, u8, u8)> = Vec::new();
    for _ in 0..pieces {
        let def = (rng.next_u64() % CATALOG.len() as u64) as usize;
        let slot = CATALOG[def].slot;
        let rot = (rng.next_u64() % 4) as u8;
        // First seat that takes it, so the board is legal by construction.
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                let mut trial = gear.clone();
                trial.push((def, slot, x, y, rot));
                let b = gearmaster_oracle::Board { gear: trial.clone(), chunks: Vec::new(), rows: [0; 5] };
                let (_, lo) = b.rebuild();
                let before = gearmaster_oracle::Board { gear: gear.clone(), chunks: Vec::new(), rows: [0; 5] };
                let (_, lo0) = before.rebuild();
                if lo.slot(slot).pieces().len() > lo0.slot(slot).pieces().len() {
                    gear = trial;
                    break 'seat;
                }
            }
        }
    }
    stand(&gear)
}

fn main() {
    census();
    probe();
}

fn census() {
    println!("=== THE CENSUS ===\n");
    let c = Console::start(0, Mode::Grinder, Difficulty::Medium);
    let _ = c;
    // Every piece, through the console's own reader, by standing it alone.
    let mut produce = [0usize; 8];
    let mut consume = [0usize; 8];
    let mut both = 0usize;
    let mut conditional = 0usize;
    for def in 0..CATALOG.len() {
        let pools = piece_pools(def);
        for i in 0..8 {
            if pools.produces[i] > 0 {
                produce[i] += 1;
            }
            if pools.consumes[i] > 0 {
                consume[i] += 1;
            }
        }
        if pools.produces_any() && pools.consumes_any() {
            both += 1;
        }
        if pools.conditional > 0 {
            conditional += 1;
        }
    }
    println!("{:<16} {:>9} {:>9}", "pool", "produced", "consumed");
    for i in 0..8 {
        if produce[i] + consume[i] == 0 {
            continue;
        }
        println!("{:<16} {:>9} {:>9}", POOLS[i], produce[i], consume[i]);
    }
    println!(
        "\n  {} of {} pieces both make and spend something\n  {} carry at least one conditional",
        both,
        CATALOG.len(),
        conditional
    );
}

/// One piece's pools, read through the console.
fn piece_pools(def: usize) -> gearmaster_console::view::Pools {
    let slot = CATALOG[def].slot;
    let mut run = Run::new();
    run.clear_all();
    run.owned.clear();
    let id = run.registry.alloc(def);
    run.owned.push(id);
    let _ = slot;
    let c = Console::standing_in(run, 0);
    c.view().tray.first().map(|p| p.pools).unwrap_or_default()
}

/// Can a linear model over the board features tell whether a board will
/// successfully spend a pool in a fight?
///
/// The label comes from the **log**, not from the features: a
/// `ResourceCheck { paid: true }` or a `ManaCheck { paid: true }` is a spend
/// that actually happened. If a linear probe over the features beats the base
/// rate by a margin, the information is in the representation and a learner
/// can find it. If it does not, the features are wrong and Q3 would learn
/// nothing.
fn probe() {
    println!("\n=== THE PROBE ===\n");
    let mut rng = Rng::new(0x9_0B_E5);
    let mut xs: Vec<[f64; 18]> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    let n: usize = std::env::var("PROBE_N").ok().and_then(|v| v.parse().ok()).unwrap_or(600);

    for i in 0..n {
        let pieces = 3 + (rng.next_u64() % 14) as usize;
        let c = sample(&mut rng, pieces);
        let v = c.view();
        let (stats, items) = c.board_for_scoring();
        if items.is_empty() {
            continue;
        }
        let spec = &LADDER[(i * 7) % LADDER.len()];
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        let paid = log.entries.iter().any(|e| {
            matches!(
                e.event,
                Event::ResourceCheck { side: Side::Player, paid: true, .. }
                    | Event::ManaCheck { side: Side::Player, paid: true, .. }
            )
        });
        xs.push(features(&v.pools, items.len()));
        ys.push(if paid { 1.0 } else { 0.0 });
    }

    let base = ys.iter().sum::<f64>() / ys.len() as f64;
    println!("{} boards, {:.1}% of them spent a pool in the fight", ys.len(), base * 100.0);

    // Logistic regression by plain gradient descent. No dependency, and the
    // point is whether the information is there rather than how well a model
    // can be tuned.
    let mut w = [0.0f64; 18];
    let mut b = 0.0f64;
    let lr = 0.5;
    for _ in 0..4000 {
        let (mut gw, mut gb) = ([0.0f64; 18], 0.0f64);
        for (x, &y) in xs.iter().zip(&ys) {
            let z: f64 = w.iter().zip(x).map(|(a, b)| a * b).sum::<f64>() + b;
            let p = 1.0 / (1.0 + (-z).exp());
            let d = p - y;
            for j in 0..18 {
                gw[j] += d * x[j];
            }
            gb += d;
        }
        let m = xs.len() as f64;
        for j in 0..18 {
            w[j] -= lr * gw[j] / m;
        }
        b -= lr * gb / m;
    }
    let acc = xs
        .iter()
        .zip(&ys)
        .filter(|(x, &y)| {
            let z: f64 = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum::<f64>() + b;
            ((1.0 / (1.0 + (-z).exp())) > 0.5) == (y > 0.5)
        })
        .count() as f64
        / xs.len() as f64;
    let majority = base.max(1.0 - base);
    // Accuracy on a 14% base rate flatters a model that says "no" - so the
    // honest figure is how it does on each class separately.
    let hit = |want: f64| {
        let of: Vec<_> = xs.iter().zip(&ys).filter(|(_, &y)| y == want).collect();
        let right = of
            .iter()
            .filter(|(x, _)| {
                let z: f64 = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum::<f64>() + b;
                ((1.0 / (1.0 + (-z).exp())) > 0.5) == (want > 0.5)
            })
            .count() as f64;
        if of.is_empty() { 0.0 } else { right / of.len() as f64 }
    };
    let (pos, neg) = (hit(1.0), hit(0.0));
    println!(
        "  linear probe: {:.1}% accurate against a {:.1}% majority baseline\n  \
         lift: {:+.1} points\n  \
         on the boards that DID spend: {:.1}%   on the ones that did not: {:.1}%\n  \
         balanced: {:.1}% against 50%",
        acc * 100.0,
        majority * 100.0,
        (acc - majority) * 100.0,
        pos * 100.0,
        neg * 100.0,
        (pos + neg) / 2.0 * 100.0
    );
    println!(
        "\n  {}",
        if (pos + neg) / 2.0 > 0.65 {
            "GATE MET: the features carry pool-matching. A learner can find it."
        } else {
            "GATE FAILED: the features do not carry it. Fix them before Q2."
        }
    );

    // Which features the probe leaned on, as a sanity check on the encoding.
    let names = [
        "matched-total", "stranded-total", "starved-total", "pools-flowing",
        "prod-mana", "prod-rage", "prod-faith", "prod-nature",
        "cons-mana", "cons-rage", "cons-faith", "cons-nature",
        "match-mana", "match-rage", "match-faith", "match-nature",
        "items", "any-match",
    ];
    let mut order: Vec<(usize, f64)> = w.iter().copied().enumerate().map(|(i, v)| (i, v.abs())).collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("\n  the five it leaned on:");
    for (i, mag) in order.into_iter().take(5) {
        println!("    {:<16} {:+.3}", names[i], w[i] * mag.signum());
    }
}

fn features(p: &BoardPools, items: usize) -> [f64; 18] {
    let s = |v: i32| v as f64 / 20.0;
    [
        s(p.total_matched()),
        s(p.total_stranded()),
        s(p.total_starved()),
        p.flowing() as f64 / 4.0,
        s(p.produces[0]), s(p.produces[1]), s(p.produces[2]), s(p.produces[3]),
        s(p.consumes[0]), s(p.consumes[1]), s(p.consumes[2]), s(p.consumes[3]),
        s(p.matched[0]), s(p.matched[1]), s(p.matched[2]), s(p.matched[3]),
        items as f64 / 10.0,
        if p.total_matched() > 0 { 1.0 } else { 0.0 },
    ]
}
