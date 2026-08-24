//! Read a shared run code and print the board it describes.
//!
//! `cargo test -p gearmaster-engine --test decode_build -- --ignored --nocapture`

use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_engine::share::import;

/// The owner's own winning run. Lives in `share` now, so the walks in
/// `two_runs` can wear it too.
use gearmaster_engine::share::A_WINNING_RUN as CODE;

#[test]
#[ignore]
fn decode_the_winning_build() {
    let Some(s) = import(CODE) else {
        panic!("the code did not decode - check the transcription");
    };
    println!(
        "\nrung {}  -  {} won, {} lost  -  {}g  -  theme {}",
        s.rung, s.wins, s.losses, s.gold, s.theme
    );
    println!("classes: {}\n", s.classes.join(", "));

    for slot in SlotKind::ALL {
        let mine: Vec<_> = s.placed.iter().filter(|(_, sl, ..)| *sl == slot).collect();
        let cells: usize = mine
            .iter()
            .map(|(d, ..)| CATALOG.get(*d).map(|c| c.cells.len()).unwrap_or(0))
            .sum();
        println!("{:?}  {} pieces, {}/48 cells", slot, mine.len(), cells);
        for (d, _, x, y, rot) in &mine {
            match CATALOG.get(*d) {
                Some(c) => println!(
                    "    ({},{}) rot {}  {:<24} {:?}",
                    x, y, rot, c.name, c.kind
                ),
                None => println!("    ({},{}) rot {}  <unknown index {}>", x, y, rot, d),
            }
        }
    }
    println!("\n{} pieces placed in total", s.placed.len());
}

