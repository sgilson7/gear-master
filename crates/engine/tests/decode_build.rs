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


#[test]
fn both_shared_runs_read_back_the_classes_they_were_played_with() {
    // The guard on the bug that made this necessary. These two codes are the
    // only record of two complete runs, and a class-order change decodes them
    // into somebody else's build without erroring.
    use gearmaster_engine::share;
    let owner = share::import(share::A_WINNING_RUN).expect("the owner's code reads");
    assert_eq!(owner.classes, vec!["Berserker", "Chronomancer"], "owner's titles");
    assert_eq!(owner.placed.len(), 75);
    assert_eq!(owner.rung, 50);

    let friend = share::import(share::A_FRIENDS_RUN).expect("the friend's code reads");
    assert_eq!(
        friend.classes,
        vec!["Trundle", "Tired", "Avenged", "Piety"],
        "the friend's titles"
    );
    assert_eq!(friend.placed.len(), 76);
    assert_eq!(friend.rung, 50);
    assert_eq!(friend.wins, 50);
    assert_eq!(friend.losses, 2);
}
