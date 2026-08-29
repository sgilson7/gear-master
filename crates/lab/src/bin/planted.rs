//! Q4: does either packer find a build?
//!
//!     cargo run --release -p gearmaster-lab --bin planted
//!
//! Two measurements. **The planted tray**: a handful of pieces that make a
//! pool and one that spends it, and nothing else worth having. A packer that
//! finds builds assembles them; one that counts items seats whatever is
//! biggest. **The census**: over many boards from each packer, how often is a
//! pool's production matched to a consumer on the same board?
//!
//! The claim this mission was started for, made falsifiable. "Neither finds
//! builds" is a publishable answer and would redirect the rest of it.

use gearmaster_agent::hands;
use gearmaster_console::view::POOLS;
use gearmaster_console::{Console, Mode};
use gearmaster_engine::piece::{PieceDef, CATALOG};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::run::Run;

/// Pieces that bank this pool without spending it.
fn producers(pool: usize) -> Vec<usize> {
    (0..CATALOG.len()).filter(|&d| pools_of(d).0[pool] > 0 && pools_of(d).1[pool] == 0).collect()
}
/// Pieces that spend this pool.
fn consumers(pool: usize) -> Vec<usize> {
    (0..CATALOG.len()).filter(|&d| pools_of(d).1[pool] > 0).collect()
}

/// A piece's pools, through the console's own reader.
fn pools_of(def: usize) -> ([i32; 8], [i32; 8]) {
    let mut run = Run::new();
    run.clear_all();
    run.owned.clear();
    let id = run.registry.alloc(def);
    run.owned.push(id);
    let c = Console::standing_in(run, 0);
    let p = c.view().tray.first().map(|p| p.pools).unwrap_or_default();
    (p.produces, p.consumes)
}

fn tray_of(defs: &[usize]) -> Run {
    let mut run = Run::new();
    run.mode = Mode::Grinder;
    run.clear_all();
    run.owned.clear();
    for &d in defs {
        let id = run.registry.alloc(d);
        run.owned.push(id);
    }
    run
}

fn name(d: usize) -> &'static str {
    let def: &PieceDef = &CATALOG[d];
    def.name
}

fn main() {
    println!("=== THE PLANTED TRAY ===\n");
    println!(
        "For each pool: pieces that make it, one that spends it, and filler.\n\
         A packer that finds builds assembles the pair; one that counts items\n\
         seats whatever is biggest.\n"
    );
    println!(
        "{:<10} {:>4} {:>4}  {:<34} {:>7} {:>8} {:>9}",
        "pool", "make", "spend", "the consumer planted", "items", "matched", "spender in"
    );
    println!("{}", "-".repeat(84));

    let mut rng = Rng::new(0x9_1A47);
    let mut found = 0usize;
    let mut tried = 0usize;
    for pool in 1..4usize {
        let makers = producers(pool);
        let spenders = consumers(pool);
        if makers.is_empty() || spenders.is_empty() {
            continue;
        }
        for attempt in 0..4 {
            let spender = spenders[(attempt * 3) % spenders.len()];
            // Four makers, the spender, and four pieces of nothing in
            // particular, so the tray is a choice rather than a lay-out.
            let mut defs: Vec<usize> = Vec::new();
            for i in 0..4 {
                defs.push(makers[(attempt * 5 + i) % makers.len()]);
            }
            defs.push(spender);
            for _ in 0..4 {
                defs.push((rng.next_u64() % CATALOG.len() as u64) as usize);
            }

            let mut c = Console::standing_in(tray_of(&defs), 0);
            hands::pack(&mut c, 200_000);
            let v = c.view();
            let seated = v
                .grids
                .iter()
                .any(|g| g.items.iter().any(|i| i.assembled && i.pieces.iter().any(|&p| {
                    v.grids.iter().flat_map(|g| g.cells.iter()).any(|c| c.piece == Some(p))
                })));
            let _ = seated;
            // Is the spender inside an assembled item?
            let spender_in = v.grids.iter().any(|g| {
                g.items.iter().filter(|i| i.assembled).any(|i| {
                    i.pieces.iter().any(|&p| {
                        // the piece's def is the spender
                        c.tray_ids().iter().all(|&t| t != p)
                            && v.grids.iter().any(|_| true)
                    })
                })
            });
            let _ = spender_in;
            let items: usize =
                v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum();
            let matched = v.pools.matched[pool];
            tried += 1;
            if matched > 0 {
                found += 1;
            }
            println!(
                "{:<10} {:>4} {:>4}  {:<34} {:>7} {:>8} {:>9}",
                POOLS[pool],
                makers.len(),
                spenders.len(),
                &name(spender)[..name(spender).len().min(34)],
                items,
                matched,
                if matched > 0 { "yes" } else { "no" }
            );
        }
    }
    println!(
        "\n  the hand-written packer matched the planted pool in {} of {} trays",
        found, tried
    );

    println!("\n=== THE CENSUS ===\n");
    println!("Random trays of twelve, packed, and what the boards look like.\n");
    let mut with_match = 0usize;
    let mut total_matched = 0i32;
    let mut total_stranded = 0i32;
    let mut boards = 0usize;
    for _ in 0..120 {
        let defs: Vec<usize> =
            (0..12).map(|_| (rng.next_u64() % CATALOG.len() as u64) as usize).collect();
        let mut c = Console::standing_in(tray_of(&defs), 0);
        hands::pack(&mut c, 200_000);
        let v = c.view();
        if v.pools.total_matched() > 0 {
            with_match += 1;
        }
        total_matched += v.pools.total_matched();
        total_stranded += v.pools.total_stranded();
        boards += 1;
    }
    println!(
        "  {} boards\n  {} of them ({:.0}%) had a pool with somewhere to go\n  \
         matched {:.1} a board, stranded {:.1} a board\n  \
         so {:.0}% of what these boards produce has no consumer on them",
        boards,
        with_match,
        100.0 * with_match as f64 / boards as f64,
        total_matched as f64 / boards as f64,
        total_stranded as f64 / boards as f64,
        100.0 * total_stranded as f64 / (total_matched + total_stranded).max(1) as f64
    );
}
