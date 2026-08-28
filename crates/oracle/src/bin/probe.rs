//! Which half of a zero is which.
use gearmaster_engine::combat::{simulate_at, Difficulty, LADDER};
use gearmaster_oracle::fidelity::Reading;
use gearmaster_oracle::gate::References;

fn main() {
    let refs = References::standard();
    println!("{:<20} {:<8} {:>7} {:>7} {:>7} {:>7} {:>6}", "creature", "board", "curses", "stuns", "burn", "drain", "res");
    for name in ["Salt Idol", "Bone Cantor", "Ruin Hound", "Ember Wisp", "Cog Priest", "Obsidian Colossus"] {
        let Some(spec) = LADDER.iter().find(|s| s.name == name) else { continue };
        for (label, stats, items, _) in &refs.boards {
            let log = simulate_at(*stats, items, spec, Difficulty::Medium);
            let r = Reading::of(&log);
            println!(
                "{:<20} {:<8} {:>7} {:>7} {:>7} {:>7} {:>6}",
                name, label, r.curses, r.stuns, r.burn_damage, r.drained, stats.curse_resist
            );
        }
    }
    // And what the creatures are actually carrying.
    println!("\nwhat a Burner's gear says it does:");
    for name in ["Salt Idol", "Bone Cantor", "Ruin Hound"] {
        let Some(spec) = LADDER.iter().find(|s| s.name == name) else { continue };
        let (_, items) = spec.outfit();
        for it in &items {
            let says: Vec<String> = it.triggers.iter().map(|t| t.describe()).collect();
            println!("  {:<18} {:<28} {}", name, it.name, says.join(" | "));
        }
    }
}
