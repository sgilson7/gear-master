//! Print the chains, derived rather than typed.
//!
//!     cargo run -p gearmaster-lab --bin qquest
//!
//! `gearmaster_engine::quest::chain_to` walks the tables backwards from a thing
//! at the end of a chain. This prints what it finds, one station a line, with
//! the window each one has after the whole chain has been tightened against
//! itself - which is where the deadline shows up.
//!
//! Then it prints the same chain as the pathfinder gets it, through
//! `lab::quests`, which is the translation across the boundary: marks the
//! screen can show, and nothing else. The two lists differing is not a fault -
//! a flag is real and is not on the screen - but the difference is worth
//! looking at, because a stop that vanishes here is a stop nothing pays for.

use gearmaster_engine::quest::chain_to;
use gearmaster_lab::quests;

fn main() {
    for (name, goal) in quests::NAMED {
        let q = chain_to(*goal);
        println!("\n{name}   {:?}", q.goal);
        println!("  {:<14} {:<46} {:<12} by", "tier", "mark", "rungs");
        for s in &q.stations {
            println!(
                "  {:<14} {:<46} {:<12} {}",
                format!("{:?}", s.tier),
                format!("{:?}", s.mark),
                format!("{}-{}", s.window.0 + 1, s.window.1 + 1),
                s.by
                    .iter()
                    .map(|a| format!("{a:?}"))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
        println!("  deadline: rung {:?}", q.deadline().map(|r| r + 1));
        // And the same chain as the agent gets it: the marks it can see, with
        // whatever the screen cannot show dropped rather than approximated.
        match quests::quest(name, *goal) {
            Ok(dressed) => {
                println!("  as the pathfinder reads it, {} stops:", dressed.stops.len());
                for s in &dressed.stops {
                    println!("    {:<14} {:?}", format!("{:?}", s.tier), s.mark);
                }
            }
            Err(why) => println!("  CANNOT be handed to an agent: {why:?}"),
        }
    }
}
