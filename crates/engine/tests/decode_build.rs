//! Read a shared run code and print the board it describes.
//!
//! `cargo test -p gearmaster-engine --test decode_build -- --ignored --nocapture`

use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_engine::share::import;

const CODE: &str = "1-1J-1J-2-72W-1-2-2-0-2B-1H400-1D831-7441-10W15-13036-GM0C-B03D-M81G-1GR3J-740Q-1GG2R-11C4R-A0G3-HCH0-144H4-18M4-14RK9-1CGC-1K4MD-1JGGG-148GM-A0KN-158MR-KMGY-1HN00-16D11-KD20-FD41-6104-1AD34-17D08-16129-8X48-1813F-250H-1A15G-494M-2X0R-992R-16X5S-1FS0W-1FN4W-111G2-85J1-185M1-10NN1-18HG4-119J8-15XM8-85GC-HNJH-KHMG-5SGN-HSMN-BDGS-1E9KR-DJ00-X611-DA20-Y233-YE40-XP51-1KY18-WP48-YJ1C-YT2C-1JT0G-K61G-XY1H-1H23H-YA3M-XP4Q-XT5N-X62R-GY0X";

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
