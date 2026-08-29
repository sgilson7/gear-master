//! What the dungeon graphs look like now, and which floors are islands.
//!
//! THE ATLAS made THE THRESHOLD a T and cut an edge out of the yard, so a
//! floor can now be unreachable from the mouth by walking. That is content the
//! coverage ledger has to classify rather than count as a miss.
use gearmaster_engine::dungeon::DUNGEONS;

fn main() {
    for d in DUNGEONS {
        let n = d.floors.len();
        let mut seen = vec![false; n];
        let mut stack = vec![0usize];
        seen[0] = true;
        while let Some(at) = stack.pop() {
            for e in d.floors[at].exits {
                if e.to < n && !seen[e.to] {
                    seen[e.to] = true;
                    stack.push(e.to);
                }
            }
        }
        let islands: Vec<usize> = (0..n).filter(|&i| !seen[i]).collect();
        let forks: Vec<usize> = (0..n).filter(|&i| d.floors[i].exits.len() > 1).collect();
        let stops: Vec<usize> = (0..n).filter(|&i| d.floors[i].exits.is_empty()).collect();
        println!(
            "{:<18} {:>2} floors  forks {:<10} buffer stops {:<10} unreachable by walking {:?}",
            d.id,
            n,
            format!("{:?}", forks),
            format!("{:?}", stops),
            islands
        );
    }
}
