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

#[test]
fn probe_boss_prices() {
    use gearmaster_engine::piece::{BOSS_ONLY, CATALOG};
    use gearmaster_engine::rating::{resale_price, shop_price};
    let mut worst = 0;
    for name in BOSS_ONLY {
        let d = CATALOG.iter().find(|d| d.name == *name).unwrap();
        println!("{:>24}  shop {:>5}  resale {:>5}", name, shop_price(d), resale_price(d));
        worst = worst.max(resale_price(d));
    }
    println!("worst boss resale: {worst}");
    let ordinary: i32 = CATALOG.iter()
        .filter(|d| !gearmaster_engine::piece::is_off_the_scale(d.name))
        .map(resale_price).max().unwrap();
    println!("best ordinary resale: {ordinary}");
}

/// A fingerprint of what a code actually seats, so a catalogue shift is loud.
fn worn(code: &str) -> Vec<&'static str> {
    let sh = gearmaster_engine::share::import(code).expect("reads");
    let mut names: Vec<&'static str> = sh
        .placed
        .iter()
        .map(|&(d, ..)| gearmaster_engine::piece::CATALOG[d].name)
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn both_shared_runs_still_seat_the_gear_they_were_built_from() {
    // `CATALOG` is a wire format: a share code stores a component as its
    // position in it. Inserting a piece anywhere but the end re-points every
    // saved board, and does it quietly - the code still reads, the board is
    // still full, it is simply somebody else's gear.
    //
    // That happened. One spell went into the middle of the catalogue and both
    // of these decoded into different boards; the owner's lost six hundred
    // health and the friend's helmet went from four items to two. Nothing
    // failed, because nothing was checking.
    //
    // A count is not enough - the wrong board has the same number of pieces.
    // These are what the two runs are actually wearing.
    let owner = worn(gearmaster_engine::share::A_WINNING_RUN);
    assert_eq!(owner.len(), 75);
    // Four trophies off four named creatures, which is the part of a board
    // nobody could have got any other way.
    for trophy in ["Asker's Monocle", "Eighth Ray Crown", "Henpeck's Cell Keys", "Kaklon's Patent"]
    {
        assert!(owner.contains(&trophy), "the owner's board lost its {trophy}");
    }
    assert_eq!(owner.iter().filter(|n| **n == "Riveted Layer").count(), 2);
    assert_eq!(owner.iter().filter(|n| **n == "Sawtooth Edge").count(), 2);
    assert_eq!(owner.iter().filter(|n| **n == "Witchglass Shard").count(), 2);
    assert!(owner.contains(&"Worldsplitter"));

    let friend = worn(gearmaster_engine::share::A_FRIENDS_RUN);
    assert_eq!(friend.len(), 76);
    // This run went through the VIP area and through a town, and the board
    // says so: two pieces off the table behind the rope, and three off a cart
    // in Sump Bottom. Nothing at those indices by accident would.
    assert_eq!(friend.iter().filter(|n| **n == "Tallykeeper's Weave").count(), 2);
    assert!(friend.contains(&"Treadmill Sole"), "lost the VIP sole");
    assert_eq!(friend.iter().filter(|n| **n == "Wickstub").count(), 3);
    assert_eq!(friend.iter().filter(|n| **n == "Runed Plating").count(), 3);
    assert!(friend.contains(&"The Seeker's Tears"));
}
