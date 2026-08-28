//! THE HUNDRED's six figures, and what a tile charges for getting them wrong.
//!
//! A toll reads what a board **does a second** rather than what it has. That
//! is one sentence and it is the whole design: two boards with the same mana
//! on them cross different rivers, because one of them spends it four times as
//! often. Every figure is computed in integers - the division done per item
//! and then summed, which is not the same as summing and then dividing - and
//! no float touches any of it anywhere.
//!
//! Each figure below is **hand-computed in its own assertion**. A test that
//! calls the function it is testing to work out what the answer should be is a
//! test that the function is self-consistent.

mod common;

use gearmaster_engine::county::{self, Lane, Step, Toll, TileKind, MOUTHS};
use gearmaster_engine::loadout::{Figures, ItemProfile};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::{Run, TripSource};
use gearmaster_engine::stats::Stats;

/// An item that acts every `cooldown_ms` and carries `stats`.
///
/// Built by hand rather than assembled, because what a figure reads is a
/// cooldown and a stat block, and putting a recipe between the test and the
/// arithmetic would mean the test moved whenever a recipe did.
fn item(cooldown_ms: u32, stats: Stats) -> ItemProfile {
    ItemProfile {
        sigil_seed: 0,
        pieces: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
        diagonal_items: Vec::new(),
        name: String::new(),
        full_name: String::new(),
        core: String::new(),
        slot: SlotKind::Weapon,
        cooldown_ms,
        stats,
        triggers: Vec::new(),
        adjacent_assembled_same_slot: 0,
        open_cells: 0,
        steady: false,
        overtakes: false,
        attracts_curses: false,
        power_bonus: 0,
        power: 100,
        casts: Vec::new(),
        rating: 0,
    }
}

// ------------------------------------------------------------- the six figures

/// The pair A3 is written around: the worse-looking piece crosses the deeper
/// river.
///
/// Eight mana on a four-second item is two mana a second. Three mana on a
/// one-second item is three. A river that reads *mana* takes the first board
/// and a river that reads *flow* takes the second, and flow is the one that
/// means anything - mana is fuel and what a board can do with fuel is a
/// function of how often it gets to spend it.
#[test]
fn flow_is_not_mana() {
    let slow = Figures::of(&[item(4000, Stats { mana: 8, ..Stats::ZERO })]);
    let fast = Figures::of(&[item(1000, Stats { mana: 3, ..Stats::ZERO })]);

    assert_eq!(slow.flow, 2000, "8 mana on a 4,000 ms item is 2 a second");
    assert_eq!(fast.flow, 3000, "3 mana on a 1,000 ms item is 3 a second");
    assert!(fast.flow > slow.flow, "the faster board has less mana and more flow");

    // And the river agrees with the arithmetic.
    let deep = Toll::River { milli_per_s: 3000 };
    assert!(!deep.met(&slow, 0, 0), "eight mana crossed a river three deep");
    assert!(deep.met(&fast, 0, 0), "three mana did not cross a river it pays for");
}

/// Every figure, hand-computed, on one board of three items.
///
/// ```text
///   1,000 ms  mana 3  phys 10  magic 0  armour 4   curse resist 2
///   2,000 ms  mana 0  phys  0  magic 7  armour 0   curse resist 5
///   5,000 ms  mana 1  phys  3  magic 0  armour 20  curse resist 0
/// ```
#[test]
fn the_six_figures_are_what_the_arithmetic_says() {
    let board = [
        item(1000, Stats { mana: 3, physical_damage: 10, armor: 4, curse_resist: 2, ..Stats::ZERO }),
        item(2000, Stats { magic_damage: 7, curse_resist: 5, ..Stats::ZERO }),
        item(5000, Stats { mana: 1, physical_damage: 3, armor: 20, ..Stats::ZERO }),
    ];
    let f = Figures::of(&board);

    // flow: 3/1s = 3000, 0, 1/5s = 200.
    assert_eq!(f.flow, 3000 + 200);
    // physical: 10/1s = 10000, 3/5s = 600.
    assert_eq!(f.physical_dps, 10_000 + 600);
    assert_eq!(f.dps(Lane::Physical), f.physical_dps);
    // magic: 7/2s = 3500, and the ford names its lane.
    assert_eq!(f.magic_dps, 3500);
    assert_eq!(f.dps(Lane::Magic), 3500);
    assert_ne!(f.dps(Lane::Physical), f.dps(Lane::Magic), "a ford that names no lane names nothing");
    // armour: 4/1s = 4000, 20/5s = 4000.
    assert_eq!(f.armour_ps, 8000);
    // the drift is the fastest item, not the average.
    assert_eq!(f.fastest_ms, Some(1000));
    // the hedge is a stat held, not a rate paid.
    assert_eq!(f.curse_resist, 7);
}

/// The division is per item, and summing first would give a different answer.
///
/// Two items, one point of mana each, on 1,000 ms and 3,000 ms. Per item that
/// is 1000 + 333 = 1333 milli-mana a second. Summed first it would be two
/// points over a 4,000 ms total, or five hundred, which is not a figure about
/// anything: the two items do not take turns.
#[test]
fn the_division_is_done_per_item_and_then_summed() {
    let f = Figures::of(&[
        item(1000, Stats { mana: 1, ..Stats::ZERO }),
        item(3000, Stats { mana: 1, ..Stats::ZERO }),
    ]);
    assert_eq!(f.flow, 1000 + 333, "1/1s plus 1/3s, each rounded down where it is computed");
    assert_ne!(f.flow, 2 * 1_000_000 / 4000, "the items were summed and then divided");
}

/// A board with nothing assembled has no fastest item, which is not slow.
#[test]
fn an_empty_board_has_no_fastest_item() {
    let f = Figures::of(&[]);
    assert_eq!(f.fastest_ms, None);
    assert!(
        !Toll::Drift { fastest_ms: 9_999 }.met(&f, 0, 0),
        "a drift that would take any item at all took a board with none"
    );
    // And every rate is zero rather than absent.
    assert_eq!((f.flow, f.physical_dps, f.magic_dps, f.armour_ps, f.curse_resist), (0, 0, 0, 0, 0));
}

/// A zero cooldown contributes nothing rather than dividing by it.
#[test]
fn a_cooldown_of_zero_is_not_a_division() {
    let f = Figures::of(&[item(0, Stats { mana: 99, physical_damage: 99, ..Stats::ZERO })]);
    assert_eq!((f.flow, f.physical_dps), (0, 0));
    assert_eq!(f.fastest_ms, None, "an item that never acts is not the fastest one");
}

/// Loose pieces are not on the board a toll reads.
///
/// `Figures::of` is handed `combat_items`, which is assembled items only. A
/// loose piece contributes passive stats and does not act, and every one of
/// the six figures is about acting.
#[test]
fn a_toll_reads_the_items_and_not_the_tray() {
    let mut run = Run::seeded(0xF0117);
    let bare = run.county_figures();
    common::build_full_loadout(&mut run);
    let dressed = run.county_figures();
    assert!(
        dressed.flow > bare.flow || dressed.physical_dps > bare.physical_dps,
        "a preset board reads the same as an empty one: {bare:?} against {dressed:?}"
    );
    assert_eq!(dressed, Figures::of(&run.combat_items()));
}

// ------------------------------------------------------------------- the tax

/// A toll refused costs the move and leaves you where you were.
#[test]
fn a_failed_toll_costs_one_move_and_no_position() {
    let mut run = Run::seeded(0x1_00D);
    // Stand beside a toll this board cannot pay. Placed by hand: which mouth
    // has a river next to it is the seed's business and this is about the tax.
    let c = run.county();
    let f = run.county_figures();
    let (here, into, toll) = c
        .tiles()
        .iter()
        .find_map(|t| {
            let TileKind::Feature(toll) = t.kind else { return None };
            if toll.met(&f, run.gold, run.rung_bounty()) {
                return None;
            }
            let from = county::neighbours(t.at).into_iter().find(|n| !c.is_sealed(*n))?;
            let step = Step::ALL.into_iter().find(|s| s.from(from) == Some(t.at))?;
            Some((from, step, toll))
        })
        .expect("some toll on this county refuses a starter board");

    run.county_at = Some(here);
    run.county_moves_left = 5;
    assert!(!run.county_walk(into), "{toll:?} let a starter board across");
    assert_eq!(run.county_at, Some(here), "a refusal moved somebody");
    assert_eq!(run.county_moves_left, 4, "a refusal was free");
    assert!(!run.county_is_cleared(into.from(here).unwrap()), "a refusal cleared the tile");

    // And the receipt says which figure fell short and by how much, rather
    // than that something fell short.
    let receipt = run.last_receipt.clone().expect("a receipt");
    assert!(
        receipt.iter().any(|l| l.contains("against")),
        "the refusal did not say how far short: {receipt:?}"
    );
}

/// A met Feature is a bridge you paid for once.
#[test]
fn a_crossed_toll_stays_crossed() {
    // The owner's board rather than a starter one: a starter board pays no
    // toll on any county, which is the whole point of a toll and makes it
    // useless for asking what happens after one is paid.
    let mut run = common::board_from(gearmaster_engine::share::A_WINNING_RUN);
    run.run_seed = 0x1_00D;
    let c = run.county();
    let f = run.county_figures();
    let (here, into) = c
        .tiles()
        .iter()
        .find_map(|t| {
            let TileKind::Feature(toll) = t.kind else { return None };
            if !toll.met(&f, run.gold, run.rung_bounty()) || matches!(toll, Toll::Gate { .. }) {
                return None;
            }
            let from = county::neighbours(t.at).into_iter().find(|n| !c.is_sealed(*n))?;
            let step = Step::ALL.into_iter().find(|s| s.from(from) == Some(t.at))?;
            Some((from, step))
        })
        .expect("some toll on this county takes a starter board");
    let tile = into.from(here).unwrap();

    run.county_at = Some(here);
    run.county_moves_left = 5;
    assert!(run.county_walk(into));
    assert!(run.county_is_cleared(tile));

    // Empty the board. The bridge is still there.
    run.loadout = gearmaster_engine::loadout::Loadout::new();
    assert_eq!(run.county_figures(), gearmaster_engine::loadout::Figures::default());
    let back = Step::ALL.into_iter().find(|s| s.from(tile) == Some(here)).unwrap();
    run.county_moves_left = 5;
    assert!(run.county_walk(back));
    run.county_moves_left = 5;
    assert!(run.county_walk(into), "a bridge that was paid for asked again");
}

/// The gate is the only toll that takes anything, and it takes it once.
#[test]
fn only_the_gate_spends_gold() {
    let mut run = Run::seeded(0x1_00D);
    run.rung = 12;
    let bounty = run.rung_bounty();
    run.gold = bounty * 4;
    let f = run.county_figures();

    for toll in county::TOLLS {
        let cost = toll.toll_in_gold(bounty);
        match toll {
            Toll::Gate { bounties } => assert_eq!(cost, bounties as i32 * bounty),
            _ => assert_eq!(cost, 0, "{toll:?} charges gold"),
        }
    }
    // And a purse one coin short does not cross.
    let gate = Toll::Gate { bounties: 1 };
    assert!(gate.met(&f, bounty, bounty));
    assert!(!gate.met(&f, bounty - 1, bounty), "a gate took a purse one coin short");
}

// -------------------------------------------------------------- what you see

/// A threshold is visible from one tile away and not before.
///
/// A county you can read from the mouth is a county you plan on paper; a
/// county you can read one tile at a time is one you walk. That is why the
/// Ordnance's sheet - which shows every threshold from anywhere - is a reward
/// rather than a setting.
#[test]
fn a_threshold_is_visible_at_exactly_one_tile() {
    let mut run = Run::seeded(0x1_00D);
    let c = run.county();
    let mouth = MOUTHS[0].1;
    assert!(run.enter_county(TripSource::Town("sump-bottom"), mouth));

    let mut near = 0;
    let mut far = 0;
    for t in c.tiles() {
        if !matches!(t.kind, TileKind::Feature(_)) || run.county_is_cleared(t.at) {
            continue;
        }
        let d = county::manhattan(mouth, t.at);
        let known = run.county_threshold_known(t.at);
        assert_eq!(
            known,
            d <= 1,
            "{:?} is {d} tiles away and its threshold is {}",
            t.at,
            if known { "readable" } else { "not readable" }
        );
        if d <= 1 {
            near += 1;
        } else {
            far += 1;
        }
    }
    assert!(far > 0, "every toll on this county is next to one mouth, so this proves nothing");
    let _ = near;

    // The sheet turns all of them on from anywhere, and nothing else does.
    assert!(!run.holds_the_surveyors_sheet());
    run.flags.push(county::THE_SHEET);
    assert!(run.holds_the_surveyors_sheet());
    for t in c.tiles() {
        assert!(run.county_threshold_known(t.at), "the sheet missed {:?}", t.at);
    }
}

/// A tile you have crossed is a tile you have read.
#[test]
fn a_cleared_toll_is_readable_from_anywhere() {
    let mut run = Run::seeded(0x1_00D);
    let c = run.county();
    let some_toll = c
        .tiles()
        .iter()
        .find(|t| matches!(t.kind, TileKind::Feature(_)))
        .map(|t| t.at)
        .expect("a county has twelve");
    assert!(!run.county_threshold_known(some_toll), "readable from the road");
    run.county_cleared.push(some_toll);
    assert!(run.county_threshold_known(some_toll), "a bridge you paid for forgot its own price");
}

/// Each toll says what it wants in three characters and a number.
#[test]
fn every_toll_can_say_what_it_wants() {
    for toll in county::TOLLS {
        let said = toll.threshold();
        assert!(said.len() >= 3, "{toll:?} says {said:?}, which is not a threshold");
        assert_eq!(said.chars().next(), Some(toll.glyph()), "{toll:?} draws the wrong glyph");
        assert!(said.contains(toll.letter()), "{toll:?} does not name itself in {said:?}");
    }
    // The milli-unit reader, which is the only place a decimal point appears.
    assert_eq!(county::milli(2000), "2");
    assert_eq!(county::milli(2500), "2.5");
    assert_eq!(county::milli(2050), "2.05");
    assert_eq!(county::milli(2005), "2.005");
    assert_eq!(county::milli(0), "0");
}

// ------------------------------------------------------------ F11's table

/// What the owner's board pays, rung by rung: the table F11 sets thresholds
/// from.
///
/// **This must exist before any threshold is chosen.** A3's numbers are
/// starting points off a paper map, and F11's job is to replace them with
/// figures read off a board that actually plays the game. Printed rather than
/// asserted, because a measurement asserted against itself is not one.
#[test]
#[ignore]
fn report_what_a_board_pays() {
    let boards: [(&str, fn() -> Run); 4] = [
        ("starter", || Run::seeded(0x5EED_1234_ABCD_0001)),
        ("preset", || {
            let mut r = Run::seeded(0x5EED_1234_ABCD_0001);
            r.apply_preset();
            r
        }),
        ("owner", || common::board_from(gearmaster_engine::share::A_WINNING_RUN)),
        ("friend", || common::board_from(gearmaster_engine::share::A_FRIENDS_RUN)),
    ];
    // **Five of the six figures do not move with the rung**, and that is a
    // fact about the reference boards rather than about the tolls: a share
    // code is one board and it does not grow. The rung is read by the toll
    // gate alone, through the bounty, so it is printed separately below.
    // F11 calibrates against the *progression* the four boards stand for -
    // starter, a first assembled board, and two finished runs.
    println!("\n## What a board pays THE HUNDRED's six tolls\n");
    println!(
        "{:<9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}",
        "build", "flow", "phys/s", "magic/s", "armour/s", "fastest", "hedge"
    );
    for (name, make) in boards {
        let f = make().county_figures();
        println!(
            "{:<9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}",
            name,
            county::milli(f.flow),
            county::milli(f.physical_dps),
            county::milli(f.magic_dps),
            county::milli(f.armour_ps),
            f.fastest_ms.map(|m| format!("{m}ms")).unwrap_or_else(|| "-".into()),
            f.curse_resist
        );
    }
    println!("\n## What one bounty is, rung by rung - the toll gate's only dial\n");
    let mut run = boards[0].1();
    for rung in [10usize, 20, 30, 40] {
        run.rung = rung;
        println!("rung {:>2}   1x bounty = {}g", rung + 1, run.rung_bounty());
    }
    println!("\n## And what each of the twelve shipped thresholds would take\n");
    for toll in county::TOLLS {
        let mut takers = Vec::new();
        for (name, make) in boards {
            let mut run = make();
            run.rung = 20;
            run.gold = run.rung_bounty() * 2;
            if toll.met(&run.county_figures(), run.gold, run.rung_bounty()) {
                takers.push(name);
            }
        }
        println!("{:<10} {:>8}   crossed by: {}", toll.threshold(), format!("{:?}", toll.letter()), if takers.is_empty() { "nobody".to_string() } else { takers.join(", ") });
    }
}

// ------------------------------------------------ F11: the numbers, measured

/// What each of the four reference boards gets across, tile by tile.
///
/// **F11's pin.** A3's starting thresholds were arithmetic off a paper map and
/// eleven of the twelve were crossed by the auto-builder's board; these are
/// chosen off F4's measured table so that each kind has one tier a formed
/// board takes and one it has to be built for.
///
/// The spec's own target - "the owner's board pays 3 of 6 at rung 12, 4 of 6
/// at 18 and 26" - **cannot be written**, and F4 measured why: a reference
/// board is a share code, so it does not grow, and five of the six figures are
/// identical at every rung. Only the toll gate reads the rung at all. What is
/// pinned instead is the thing that target was reaching for: a **spread**
/// across the four boards, and no board that takes everything.
#[test]
fn what_the_reference_boards_cross() {
    let boards: [(&str, fn() -> Run, usize); 4] = [
        ("starter", || Run::seeded(0x5EED_1234_ABCD_0001), 2),
        ("preset", || {
            let mut r = Run::seeded(0x5EED_1234_ABCD_0001);
            r.apply_preset();
            r
        }, 6),
        ("owner", || common::board_from(gearmaster_engine::share::A_WINNING_RUN), 10),
        ("friend", || common::board_from(gearmaster_engine::share::A_FRIENDS_RUN), 8),
    ];
    // The preset went 5 -> 6 at T2, and it is a fault being fixed rather than
    // a figure drifting. `Figures::of` reads `stats.mana` for flow, so a piece
    // that granted mana as `OnActivate(GainMana)` contributed **nothing** to
    // the toll that asks how much mana a second a board makes - eighteen
    // pieces' worth, invisible to every threshold in the county. Folding the
    // two spellings into one made it visible.
    for (name, make, want) in boards {
        let mut run = make();
        run.rung = 20;
        run.gold = run.rung_bounty() * 2;
        let f = run.county_figures();
        let crossed = county::TOLLS
            .iter()
            .filter(|t| t.met(&f, run.gold, run.rung_bounty()))
            .count();
        assert_eq!(
            crossed, want,
            "{name} crosses {crossed} of the twelve and the measurement said {want}. \
             Re-pin with the new figures, from `--test tolls -- --ignored report_what_a_board_pays`"
        );
    }
}

/// No board takes everything, and each fails what it did not build for.
///
/// The sentence the numbers exist to make true: a board that crosses rivers is
/// not a board that climbs scarps. Asserted by naming the tiles each of the
/// two finished boards is refused by, because a count alone would be satisfied
/// by two boards failing the same two.
#[test]
fn the_two_finished_boards_fail_different_things() {
    let refused = |code: &str| -> Vec<String> {
        let mut run = common::board_from(code);
        run.rung = 20;
        run.gold = run.rung_bounty() * 2;
        let f = run.county_figures();
        county::TOLLS
            .iter()
            .filter(|t| !t.met(&f, run.gold, run.rung_bounty()))
            .map(|t| t.threshold())
            .collect()
    };
    let owner = refused(gearmaster_engine::share::A_WINNING_RUN);
    let friend = refused(gearmaster_engine::share::A_FRIENDS_RUN);

    assert!(!owner.is_empty(), "the owner's board crosses every toll in the county");
    assert!(!friend.is_empty(), "the friend's board crosses every toll in the county");
    assert_ne!(owner, friend, "two very different boards are refused by the same tiles");

    // The owner is iron and the friend is magic, and the tolls know it.
    assert!(owner.contains(&"~F20m".to_string()), "the iron board crossed a deep magic ford");
    assert!(friend.contains(&"~F10p".to_string()), "the magic board crossed a deep iron ford");
    // And the friend's slower board is refused by the fast drift.
    assert!(friend.contains(&"^D1.6".to_string()), "a 1,900 ms board crossed a 1,600 ms drift");
}

/// Every kind has one tier a formed board takes and one it has to build for.
#[test]
fn each_kind_has_an_easy_tier_and_a_hard_one() {
    let mut run = common::board_from(gearmaster_engine::share::A_WINNING_RUN);
    run.rung = 20;
    run.gold = run.rung_bounty() * 2;
    let f = run.county_figures();
    let mut by_kind: std::collections::BTreeMap<char, Vec<bool>> = Default::default();
    for t in county::TOLLS {
        by_kind.entry(t.letter()).or_default().push(t.met(&f, run.gold, run.rung_bounty()));
    }
    assert_eq!(by_kind.len(), 6, "six kinds");
    for (letter, met) in &by_kind {
        assert_eq!(met.len(), 2, "{letter} has {} tiers", met.len());
        assert!(
            met[0] || met[1],
            "{letter}: the owner's board is refused by both tiers, so this kind is a wall \
             rather than a question"
        );
    }
    // And at least two kinds refuse the owner's board somewhere, or the tiers
    // are not tiers.
    let has_a_hard_tier = by_kind.values().filter(|m| !m[0] || !m[1]).count();
    assert!(
        has_a_hard_tier >= 2,
        "only {has_a_hard_tier} of the six kinds refuses the owner's board at either tier"
    );
}

// ------------------------------------------------------- F13: the weights

/// The three chains pay the same, in three currencies.
///
/// Part B's claim - "equal in cost and magnitude, different in currency" -
/// checked against what the ratings actually say. They are **not** three equal
/// numbers and should not be: the rating can see a component and cannot see a
/// run-long passive, so reading the three combat pieces side by side and
/// calling one chain thin would be reading half the payout.
///
/// ```text
///   THE ORDNANCE    Trig Pillar 64  + Surveyor's Orb 10  + the sheet
///   THE DROVE ROADS Drove Way   29  + Drover's Orb    8  + a free move a trip
///   THE ENCLOSURE   The Common Ground 47
/// ```
///
/// The sheet is every threshold in the county from anywhere; the free move is
/// up to nine moves across a full census, which is nearly two trips. Neither
/// is a stat and neither is priced.
#[test]
fn each_chain_pays_the_top_of_the_slot_it_taxed() {
    use gearmaster_engine::piece::{PieceKind, CATALOG};
    use gearmaster_engine::rating::piece_rating;
    let rating = |n: &str| {
        piece_rating(CATALOG.iter().find(|d| d.name == n).unwrap_or_else(|| panic!("{n}")))
    };
    // Each is dear for its slot: at or above the median of the enchantments
    // that slot has, which is the family it is meant to sit in.
    for (piece, slot) in [
        ("Trig Pillar", gearmaster_engine::piece::SlotKind::Greaves),
        ("Drove Way", gearmaster_engine::piece::SlotKind::Gloves),
        ("The Common Ground", gearmaster_engine::piece::SlotKind::Chest),
    ] {
        let mut family: Vec<i32> = CATALOG
            .iter()
            .filter(|d| d.slot == slot && d.kind == PieceKind::Enchantment)
            .map(piece_rating)
            .collect();
        family.sort_unstable();
        let median = family[family.len() / 2];
        assert!(
            rating(piece) >= median,
            "{piece} rates {} and the median {slot:?} enchantment is {median} - a chain's \
             whole reward should not be the cheap half of its own family",
            rating(piece)
        );
    }
    // And the ordering is the ordering of the effects, which is what F13
    // measured: a conditional doubling, then an adjacency, then one extra
    // activation.
    assert!(
        rating("Trig Pillar") > rating("The Common Ground"),
        "a doubling is worth less than an adjacency"
    );
    assert!(
        rating("The Common Ground") > rating("Drove Way"),
        "an adjacency is worth less than one extra activation"
    );
}

/// The three weights are the measured ones.
///
/// A ratchet on the figures F13 chose, so that moving one is a decision
/// somebody makes rather than a thing that drifts. The reasons are in
/// `rating.rs` beside the constants.
#[test]
fn the_three_weights_are_what_f13_measured() {
    use gearmaster_engine::rating::{BEARING, COMMONS, OVERTAKE};
    assert_eq!(BEARING, 26.0, "unmoved at F13: +22 on Trig Pillar, which lands it at 64");
    assert_eq!(
        OVERTAKE, 10.5,
        "measured at F13 and moved DOWN from 14: one extra activation is +7.1% over a whole \
         fight and 33% at the four-board table's nine-second median"
    );
    assert_eq!(
        COMMONS, 30.0,
        "measured at F13 and moved up from 24: an item has 2.2 neighbours on a finished board \
         and a commons item would have eighteen"
    );
}
