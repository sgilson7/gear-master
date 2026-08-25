//! Pieces the test suite leans on, and what it leans on them for.
//!
//! Eleven tests across five files name a component as their example of a
//! mechanic: `effects` needs something that opens a fight, `drains` needs
//! something that takes a pool, `curses_in_combat` needs something that banks
//! empowerment. When the rewrite moves a mechanic to its proper slot, those
//! pieces stop carrying it and the tests fail — five times so far, and each
//! time the failure landed in a file with no obvious connection to the change.
//!
//! The obvious fix does not work. Those fixtures do not merely name a piece,
//! they *place* it: `equip(&mut run, "Braced Plating", SlotKind::Helmet, 0, 2)`
//! puts a specific polyomino at specific cells. Looking one up by mechanic
//! would hand back a shape the coordinates do not fit, so the coupling is real
//! and cannot be abstracted away.
//!
//! So it is declared instead. Each row below says "this test needs this piece
//! to keep doing this", and the sweep that takes the mechanic away fails
//! *here*, with the name of the test it is about to break, rather than
//! somewhere downstream. That turns a debugging session into a line of output.
//!
//! **This is not a reason to keep a mechanic in the wrong slot.** When a sweep
//! moves one of these, the answer is to fix the test it names — give the
//! fixture a piece that still carries what it needs, and re-pin the
//! coordinates for the new shape — and then delete the row. A shrinking table
//! is the point.

use gearmaster_engine::piece::{Action, PieceDef, Trigger, CATALOG};

/// What a fixture needs its piece to still do.
#[derive(Copy, Clone)]
enum Needs {
    OpensTheFight,
    BanksEmpowerment,
    TakesAPool,
    Grows,
    Forks,
    SpendsAWholePool,
    CarriesRealHealth,
}

impl Needs {
    fn holds(self, d: &PieceDef) -> bool {
        let does = |want: fn(&Action) -> bool| {
            d.triggers.iter().any(|t| match t {
                Trigger::OnActivate(a)
                | Trigger::OnBattleStart(a)
                | Trigger::OnAdjacentActivate(a)
                | Trigger::OnAlignedActivate(a)
                | Trigger::OnDiagonalActivate(a)
                | Trigger::OnOtherCast(a) => want(a),
                Trigger::Watch { then, .. } => want(then),
                Trigger::PerAdjacentItem { action, .. } => want(action),
                Trigger::Consume { per, .. } => want(per),
                Trigger::SpendGold { on_success, .. } => want(on_success),
                Trigger::SpendMana { on_success, on_failure, .. }
                | Trigger::Spend { on_success, on_failure, .. } => {
                    want(on_success) || want(on_failure)
                }
                Trigger::PerAdjacentEmpty(_) => false,
            })
        };
        match self {
            Needs::OpensTheFight => {
                d.triggers.iter().any(|t| matches!(t, Trigger::OnBattleStart(_)))
            }
            Needs::BanksEmpowerment => does(|a| matches!(a, Action::GainEmpowerment(_))),
            Needs::TakesAPool => does(|a| matches!(a, Action::Drain { .. })),
            Needs::Grows => does(|a| matches!(a, Action::Grow(_))),
            Needs::Forks => does(|a| matches!(a, Action::GainForking(_))),
            Needs::SpendsAWholePool => {
                d.triggers.iter().any(|t| matches!(t, Trigger::Consume { .. }))
            }
            Needs::CarriesRealHealth => d.base.health > 15,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Needs::OpensTheFight => "a trigger that fires before the first tick",
            Needs::BanksEmpowerment => "empowerment",
            Needs::TakesAPool => "a drain",
            Needs::Grows => "growth",
            Needs::Forks => "forking",
            Needs::SpendsAWholePool => "a whole-pool spend",
            Needs::CarriesRealHealth => "more than fifteen health",
        }
    }
}

/// Piece, the test that leans on it, and what it needs the piece to do.
///
/// Where the mechanic is one the rewrite is still moving, the slot it is
/// bound for is named too — those rows are the ones expected to go.
const LEANED_ON: &[(&str, &str, Needs, &str)] = &[
    ("Archmage's Primer", "curses_in_combat::a_stunning_caster", Needs::BanksEmpowerment, "helmet"),
    ("Empowering Focus", "curses_in_combat::a_stunning_caster", Needs::BanksEmpowerment, "helmet"),
    ("Sump Sole", "drains::a_draining_board", Needs::TakesAPool, "gloves"),
    ("Vigil Crest", "drains", Needs::OpensTheFight, "greaves"),
    ("Braced Plating", "effects::a_prepared_item_only_opens_once", Needs::OpensTheFight, "greaves"),
    ("Opening Grudge", "effects::a_prepared_item_only_opens_once", Needs::OpensTheFight, "greaves"),
    ("Twinning Mold", "effects", Needs::Forks, "weapon"),
    ("Tin Frame", "effects", Needs::CarriesRealHealth, "chest"),
    ("Hermit's Band", "effects", Needs::CarriesRealHealth, "chest"),
    ("Gluttonous Fang", "progression", Needs::Grows, "chest"),
    ("Slash and Burn", "slash_and_burn", Needs::SpendsAWholePool, "helmet"),
];

#[test]
fn every_fixture_still_does_what_its_test_needs() {
    let mut broken = Vec::new();
    for (piece, test, needs, bound_for) in LEANED_ON {
        let Some(d) = CATALOG.iter().find(|d| d.name == *piece) else {
            broken.push(format!("{piece} is gone from the catalogue; {test} names it"));
            continue;
        };
        if !needs.holds(d) {
            broken.push(format!(
                "{piece} no longer carries {} - {test} leans on it for exactly that. \
                 It was bound for the {bound_for}, so this is the sweep arriving: fix that \
                 test's fixture, re-pin its coordinates for the new shape, and delete this row.",
                needs.name()
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} fixture(s) lost the mechanic their test needs:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

#[test]
fn no_fixture_is_listed_twice_for_the_same_reason() {
    // Two tests may lean on one piece for different things; the same pair
    // twice is a copy that will rot.
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for (piece, test, ..) in LEANED_ON {
        assert!(!seen.contains(&(piece, test)), "{piece} is listed twice for {test}");
        seen.push((piece, test));
    }
}

#[test]
fn the_table_names_pieces_that_exist() {
    // A row naming a piece that has been renamed away is a row nobody is
    // checking, which is worse than no row at all.
    for (piece, ..) in LEANED_ON {
        assert!(
            CATALOG.iter().any(|d| d.name == *piece),
            "{piece} is in the fixture table and not in the catalogue"
        );
    }
}
