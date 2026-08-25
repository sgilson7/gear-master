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

#[test]
fn the_perfect_run_reads_back_as_what_it_says_it_is() {
    // Transcribed off a screenshot, so it is checked rather than trusted: a
    // share code stores pieces by catalogue index, and a mistyped character
    // does not fail, it seats somebody else's gear.
    use gearmaster_engine::share;
    let r = share::import(share::A_PERFECT_RUN).expect("the perfect run reads");
    assert_eq!(r.rung, 50, "it finished the ladder");
    assert_eq!((r.wins, r.losses), (50, 0), "fifty fights and nothing lost");
    assert_eq!(r.placed.len(), 62, "sixty-two pieces");
    assert_eq!(r.classes.len(), 4, "four titles");
    // Every index it names is a real component, which is what catches a
    // transcription slip that happens to stay in range.
    for &(d, ..) in &r.placed {
        assert!(d < gearmaster_engine::piece::CATALOG.len(), "index {d} is not a component");
    }
}

#[test]
#[ignore]
fn probe_what_a_shared_board_loses_on_the_way_back() {
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::share;
    for (label, code) in [
        ("perfect", share::A_PERFECT_RUN),
        ("owner", share::A_WINNING_RUN),
        ("friend", share::A_FRIENDS_RUN),
    ] {
        let sh = share::import(code).expect("reads");
        let (reg, lo) = sh.loadout();
        let seated: usize = SlotKind::ALL.iter().map(|&k| lo.slot(k).pieces().len()).sum();
        println!("\n{label}: code says {} pieces, board seated {seated}", sh.placed.len());
        for k in SlotKind::ALL {
            let r = lo.report(&reg, k);
            let want = sh.placed.iter().filter(|(_, s, ..)| *s == k).count();
            println!(
                "  {:?}: {} of {} seated, {} items assembled, {} loose",
                k,
                lo.slot(k).pieces().len(),
                want,
                r.assembled_count(),
                r.loose_count()
            );
        }
    }
}

#[test]
fn a_shared_board_comes_back_as_the_items_it_was_built_from() {
    // The guard that was missing. `Shared::loadout` seated every piece
    // correctly and then derived the wrong *items* from them - a dense board
    // is one connected mass, so a single pass at the end merged whole grids
    // into one item. The owner's nineteen weapon pieces came back as one; the
    // perfect run's eleven came back as none. Nothing caught it, and every
    // measurement in the rewrite was taken against boards nobody had built.
    //
    // Pinned as a floor rather than an exact count: the number depends on the
    // catalogue, and the catalogue moves. What must not happen again is a board
    // quietly collapsing, and a floor says so loudly.
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::share;
    for (label, code, least) in [
        ("owner", share::A_WINNING_RUN, 17),
        ("friend", share::A_FRIENDS_RUN, 15),
        ("perfect", share::A_PERFECT_RUN, 13),
    ] {
        let sh = share::import(code).expect("reads");
        let (reg, lo) = sh.loadout();
        let seated: usize = SlotKind::ALL.iter().map(|&k| lo.slot(k).pieces().len()).sum();
        assert_eq!(seated, sh.placed.len(), "{label} lost pieces on the way back");
        let items: usize =
            SlotKind::ALL.iter().map(|&k| lo.report(&reg, k).assembled_count()).sum();
        assert!(
            items >= least,
            "{label} came back with {items} finished items, and a board somebody won with \
             should manage at least {least}. A dense board that assembles almost nothing is \
             the connectivity fault returning."
        );
        // And every grid a finished board fills should hold something that
        // acts - a whole empty slot is the same fault, seen per-grid.
        for k in SlotKind::ALL {
            if lo.slot(k).pieces().is_empty() {
                continue;
            }
            assert!(
                lo.report(&reg, k).assembled_count() > 0,
                "{label}'s {:?} holds {} pieces and not one finished item",
                k,
                lo.slot(k).pieces().len()
            );
        }
    }
}

/// The three shared boards, item by item, by name.
///
/// The floors above say a dense board must not collapse. This says what it
/// comes back as. Counts and ladder results agreed while the reconstruction
/// was wrong - nineteen weapon pieces coming back as one item is still one
/// item, and one item still fights - so the only thing that could have caught
/// it was looking at *which pieces ended up in which item*, which nothing did.
///
/// Written out in full rather than derived, because a derived expectation
/// would be the same code twice and would agree with itself while both halves
/// were wrong. This is what the boards hold; a diff here is a board coming
/// back different, and the reason has to be found before the table is edited.
///
/// Regenerate with `probe_membership`.
#[allow(clippy::type_complexity)]
const MEMBERSHIP: &[(&str, &[(SlotKind, &str)])] = &[
    ("owner", &[
        (SlotKind::Helmet, "Aegis Crown + Warding Plate"),
        (SlotKind::Helmet, "Bone Frame + Crown of the Deep + Layered Plating"),
        (SlotKind::Helmet, "Eighth Ray Crown + Heartwood Crest + Reckoning Plate"),
        (SlotKind::Chest, "Adamant Base + Seedbed Layer"),
        (SlotKind::Chest, "Deep Roots Base + Emberplate + Runic Weave + Scale Layer"),
        (SlotKind::Chest, "Riveted Layer + Runed Lining + Wellspring Base"),
        (SlotKind::Gloves, "Bloomguard + Padded Mold"),
        (SlotKind::Gloves, "Breaker's Fist + Sovereign Mold"),
        (SlotKind::Gloves, "Channeling Mold + Henpeck's Cell Keys + Rootwoven Material"),
        (SlotKind::Gloves, "Gripping Mold + Plaguewalkers"),
        (SlotKind::Gloves, "Iron Band + Quickfinger Mold + Seal of Power + Spun Material"),
        (SlotKind::Gloves, "Ring of Embers + Thornweald Grip + Wrathful Talons"),
        (SlotKind::Greaves, "Anchor Material + Anchored Sole + Warded Plating"),
        (SlotKind::Greaves, "Anchor Material + Plain Sole + Scaled Plating"),
        (SlotKind::Greaves, "Mage's Sandals + Studded Sole"),
        (SlotKind::Greaves, "Scaled Material + Striding Mold + Tin Plating"),
        (SlotKind::Greaves, "Scrying Lens + Sevenleague Boots + Widow's Sole"),
        (SlotKind::Weapon, "Forking Bead + Gravebound Haft + Loaded Fob + Witchglass Shard + Worldsplitter"),
        (SlotKind::Weapon, "Iron Blade + Oak Handle + Sawtooth Edge"),
    ]),
    ("friend", &[
        (SlotKind::Helmet, "Asker's Monocle + Mage's Circlet + Runed Plating"),
        (SlotKind::Helmet, "Runed Plating + Scaled Plating + Tin Frame"),
        (SlotKind::Helmet, "Visor of Focus + Witch's Hat"),
        (SlotKind::Chest, "Adamant Base + Lightweave + Plate Layer + Wickstub"),
        (SlotKind::Chest, "Becalming Layer + Quilted Base + Sigil Layer"),
        (SlotKind::Chest, "Becalming Layer + Wellspring Base"),
        (SlotKind::Chest, "Rimeguard Base + Seedbed Layer"),
        (SlotKind::Gloves, "Boiled Leather + Bramble Mold"),
        (SlotKind::Gloves, "Braced Mold + Mage's Sandals + Seal of the Grove + Tithe Ring"),
        (SlotKind::Gloves, "Empowering Mold + Tallykeeper's Weave"),
        (SlotKind::Gloves, "Hexer's Reckoning + Scaled Material + Seal of the Grove"),
        (SlotKind::Greaves, "Anchor Material + Layered Plating + Tarpit Sole"),
        (SlotKind::Greaves, "Anchored Sole + Ironthread Material"),
        (SlotKind::Greaves, "Pilgrim's Sole + Spun Material"),
        (SlotKind::Greaves, "Thornweald Grip + Treadmill Sole"),
        (SlotKind::Weapon, "Blood Rite + Mirrorcast + The Seeker's Tears"),
        (SlotKind::Weapon, "Hollow Sphere + Last Rite + Mirror Ward + Tidal Alignment"),
    ]),
    ("perfect", &[
        (SlotKind::Helmet, "Bronze Frame + Runed Plating + Warlord's Crest"),
        (SlotKind::Helmet, "Deadweight Plating + Warded Frame"),
        (SlotKind::Chest, "Rimeguard Base + Starlit Mantle + Wrathbreaker"),
        (SlotKind::Chest, "Sackcloth Base + Woven Underlayer"),
        (SlotKind::Gloves, "Bloodring + Bulwark Material + Flaying Mold + Henpeck's Cell Keys"),
        (SlotKind::Gloves, "Coven Mold + Tallykeeper's Weave"),
        (SlotKind::Gloves, "Deft Mold + Mage's Sandals + Signet of Iron"),
        (SlotKind::Gloves, "Plaguewalkers + Spiked Vambrace"),
        (SlotKind::Greaves, "Greave Mold + Rootbound Material"),
        (SlotKind::Greaves, "Leather Material + Reckoning Plate + Stormstep Mold"),
        (SlotKind::Greaves, "Pilgrim's Sole + Witch's Claw"),
        (SlotKind::Greaves, "Rootbound Material + Stumblefoot Mold"),
        (SlotKind::Weapon, "Balance Weight + Bulwark Vial + Cull + Iron Blade + Oak Handle"),
        (SlotKind::Weapon, "Codex Interminable + Shatterbolt + Tidewrack Ink"),
        (SlotKind::Weapon, "Sawtooth Edge + Toolwright's Grip"),
    ]),
];

#[test]
fn the_boards_come_back_holding_exactly_these_items() {
    use gearmaster_engine::share;
    let code_for = |label: &str| match label {
        "owner" => share::A_WINNING_RUN,
        "friend" => share::A_FRIENDS_RUN,
        "perfect" => share::A_PERFECT_RUN,
        other => panic!("no code called {other}"),
    };
    for &(label, want) in MEMBERSHIP {
        let sh = share::import(code_for(label)).expect("reads");
        let (reg, lo) = sh.loadout();
        let mut got: Vec<(SlotKind, String)> = Vec::new();
        for k in SlotKind::ALL {
            for i in lo.report(&reg, k).items.iter().filter(|i| i.assembled) {
                let mut n: Vec<&str> = i.pieces.iter().map(|&p| reg.def(p).name).collect();
                n.sort_unstable();
                got.push((k, n.join(" + ")));
            }
        }
        got.sort_by(|a, b| {
            let ai = SlotKind::ALL.iter().position(|k| *k == a.0);
            let bi = SlotKind::ALL.iter().position(|k| *k == b.0);
            ai.cmp(&bi).then_with(|| a.1.cmp(&b.1))
        });
        let want: Vec<(SlotKind, String)> =
            want.iter().map(|&(k, s)| (k, s.to_string())).collect();
        for (g, w) in got.iter().zip(want.iter()) {
            assert_eq!(g, w, "{label} came back holding a different item");
        }
        assert_eq!(
            got.len(),
            want.len(),
            "{label} came back with {} items and was built from {}",
            got.len(),
            want.len()
        );
    }
}

/// Print `MEMBERSHIP` as it stands, for pasting back over it.
///
/// Regenerating is the last step of understanding why it moved, never the
/// first.
#[test]
#[ignore = "generator; run with --ignored"]
fn probe_membership() {
    use gearmaster_engine::share;
    for (label, code) in [
        ("owner", share::A_WINNING_RUN),
        ("friend", share::A_FRIENDS_RUN),
        ("perfect", share::A_PERFECT_RUN),
    ] {
        println!("    (\"{label}\", &[");
        let sh = share::import(code).expect("reads");
        let (reg, lo) = sh.loadout();
        for k in SlotKind::ALL {
            let mut items: Vec<String> = lo
                .report(&reg, k)
                .items
                .iter()
                .filter(|i| i.assembled)
                .map(|i| {
                    let mut n: Vec<&str> = i.pieces.iter().map(|&p| reg.def(p).name).collect();
                    n.sort_unstable();
                    format!("\"{}\"", n.join(" + "))
                })
                .collect();
            items.sort();
            for it in items {
                println!("        (SlotKind::{:?}, {}),", k, it);
            }
        }
        println!("    ]),");
    }
}

/// Print a shared board as a creature's `gear` and `items`, for pasting into
/// `combat.rs`.
///
/// The boards at the top of the ladder are the ones people actually built, and
/// they are far stronger than anything the packer will author: seventy-odd
/// pieces packed to within a cell or two of full, nineteen items on one of
/// them. A search that has to land on a curve cannot produce that, and the last
/// few fights are the place where it should not have to.
///
/// Assembled items only. A player may seat loose pieces for their flat stats -
/// the friend's board does it twelve times - but `MonsterSpec::unassembled`
/// forbids a creature a chunk that does not come together, and loose gear
/// would be exactly that. Each item's pieces are emitted contiguously, which
/// is what `items` partitions.
///
///     BOARD=owner cargo test -p gearmaster-engine --test decode_build \
///         -- --ignored --nocapture as_a_creature_board
#[test]
#[ignore = "generator; run with --ignored"]
fn as_a_creature_board() {
    use gearmaster_engine::share;
    let which = std::env::var("BOARD").unwrap_or_else(|_| "owner".into());
    let skip: Vec<String> = std::env::var("BOARD_SKIP_SLOT")
        .map(|v| v.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();
    let code = match which.as_str() {
        "owner" => share::A_WINNING_RUN,
        "friend" => share::A_FRIENDS_RUN,
        "perfect" => share::A_PERFECT_RUN,
        other => panic!("no board called {other}"),
    };
    let sh = share::import(code).expect("reads");
    let (reg, lo) = sh.loadout();
    let mut chunks: Vec<usize> = Vec::new();
    println!("GEAR");
    for k in SlotKind::ALL {
        if skip.iter().any(|s| s == &k.name().to_lowercase()) {
            continue;
        }
        for item in lo.report(&reg, k).items.iter().filter(|i| i.assembled) {
            chunks.push(item.pieces.len());
            for &p in &item.pieces {
                let (x, y) = lo.slot(k).anchor_of(p).expect("a seated piece has an anchor");
                println!(
                    "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
                    reg.def(p).name,
                    k,
                    x,
                    y,
                    reg.rotation(p)
                );
            }
        }
    }
    println!("ITEMS &{chunks:?}");
    println!("pieces: {}, items: {}", chunks.iter().sum::<usize>(), chunks.len());
}
