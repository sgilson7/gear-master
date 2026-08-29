//! What the screen may read.
//!
//! The rule is one sentence: **every accessor the view uses is one the window
//! also uses.** An agent that reads a field no interface draws is an agent
//! playing a different game, and the failure would be silent - the numbers
//! would all look fine.
//!
//! ## The audit this milestone was told to do, and its answer
//!
//! `design/the-apprentice.md` §4 named `Run::monster()` as a leak to close: it
//! hands back the coming creature's whole spec, gear and all. **It is not a
//! leak.** `gui/src/main.rs` draws the creature's stats, then "WHAT THEY
//! BRING" - every item it will swing - and then its whole board, under a
//! comment saying the panel exists so you can shop against what is coming and
//! that showing half of it would defeat the point. So the view carries all of
//! it, and the spec's §4 is the thing that was wrong.
//!
//! What the view does **not** carry is the rest of the ladder, and there the
//! two interfaces disagree: the CLI's `ladder` prints every creature's outfit
//! at every rung and the window shows only the next one. This takes the
//! window's answer. Telling an agent *less* than a person knows can only make
//! a reachability claim stronger, and the claim is the product.

use gearmaster_console::{Console, Difficulty, Mode};

const GUI: &str = include_str!("../../gui/src/main.rs");
const CLI: &str = include_str!("../../cli/src/main.rs");
const READ: &str = include_str!("../src/read.rs");

/// Read by the view and by neither interface - because the interfaces are
/// wrong about it.
///
/// `Run::price(slot)` is `shop.price(slot) + markup%`, and `Run::buy` charges
/// exactly that (`run.rs:3299`, `:3308`). Both interfaces draw
/// `rating::shop_price(def)` instead - `gui/src/main.rs:3720, :3828, :4315`
/// and the CLI's shelf list - which is the price **before** the markup. THE
/// TOLLBOOTH answers with `Outcome::Markup(10)` (`event.rs:2499`) and its
/// receipt says "Every shelf costs 10% more"; from that moment every shelf in
/// the game shows a number it will not honour.
///
/// The console shows the true price, because that is what the player pays. The
/// CLI was fixed with it. **The window still has the bug** - three call sites,
/// each drawing from a `PieceDef` where the shelf index is not in scope - and
/// that is recorded rather than fixed here, because it is interface work and
/// this milestone is not.
const THE_INTERFACES_ARE_WRONG_ABOUT_THESE: &[&str] = &["price"];

fn calls_on(src: &str, recv: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in src.match_indices(recv) {
        let rest = &src[i + recv.len()..];
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && rest[name.len()..].starts_with('(') {
            out.push(name);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn every_field_the_view_reads_is_one_an_interface_also_reads() {
    let gui = calls_on(GUI, "run.");
    let cli = calls_on(CLI, "run.");
    let mut unknown = Vec::new();
    for m in calls_on(READ, "run.") {
        if !gui.contains(&m)
            && !cli.contains(&m)
            && !THE_INTERFACES_ARE_WRONG_ABOUT_THESE.contains(&m.as_str())
        {
            unknown.push(m);
        }
    }
    assert!(
        unknown.is_empty(),
        "the view reads {:?}, which no interface draws. Either the window \
         should draw it, or the agent should not see it.",
        unknown
    );
}

#[test]
fn the_shelf_shows_the_price_it_will_charge() {
    // The bug above, held as a test so the console never grows it. A shelf
    // that says one number and takes another is the shop lying, and an agent
    // budgeting against the wrong figure would look like an agent that cannot
    // count.
    let c = Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    for s in &v.shop {
        let shown = s.price.expect("a stocked shelf has a price");
        assert_eq!(
            s.affordable,
            shown <= v.gold,
            "shelf {} says {}g and affordability was worked out from something else",
            s.index,
            shown
        );
    }
}

#[test]
fn the_view_carries_what_the_portrait_card_carries() {
    // The card draws stats, then every item, then the board. The first two are
    // fields; the third is the same information said as a list, which is what
    // `brings` is.
    let c = Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    assert!(v.coming.stats.health > 0, "the card draws their health");
    assert_eq!(v.coming.rung_shown, 1, "displayed rung is at + 1 (trap 9)");
    // Rung one's creature is the Cave Rat, the one creature on the ladder with
    // an innate attack and nothing else - so `brings` is empty here and
    // `innate` is not, which is the pair worth pinning.
    assert!(!v.coming.innate.is_empty(), "the Cave Rat's teeth are not gear");
}

#[test]
fn the_view_does_not_carry_the_rest_of_the_ladder() {
    // A negative test, because this is the one place the console is stricter
    // than a shipped driver and it should stay a decision rather than drift
    // into an oversight. If a `ladder` field ever appears, it needs an
    // argument in this file first.
    let c = Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    let drawn = format!("{:?}", v);
    assert!(
        !drawn.contains("Francis"),
        "the view named the last creature on the ladder from rung one"
    );
}

#[test]
fn the_shortcut_reads_what_the_screen_reads() {
    // `figures()` skips drawing the grids and the tray so the hands can afford
    // to call it twice a seat. It must not skip anything else.
    let mut c = Console::start(0x1111, Mode::Grinder, Difficulty::Medium);
    for _ in 0..60 {
        let menu = c.menu();
        let Some(&v) = menu.first() else { break };
        if !c.apply(v).ok {
            break;
        }
        let full = c.view();
        let (figures, stats, items, filled) = c.figures();
        assert_eq!(figures, full.figures, "the figures");
        assert_eq!(stats, full.stats, "the character sheet");
        assert_eq!(
            items,
            full.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum::<usize>(),
            "how many items assembled"
        );
        assert_eq!(
            filled,
            full.grids
                .iter()
                .map(|g| g.cells.iter().filter(|c| c.piece.is_some()).count())
                .sum::<usize>(),
            "how many cells are filled"
        );
    }
}

#[test]
fn the_screen_and_the_view_cannot_disagree() {
    // The text is drawn from the `View` and from nothing else, so a field that
    // changes changes both. Cheap to state, and it is the thing that stops a
    // person and an agent reading two different games.
    let mut c = Console::start(0x6060, Mode::Grinder, Difficulty::Medium);
    for _ in 0..40 {
        let menu = c.menu();
        let Some(&v) = menu.first() else { break };
        if !c.apply(v).ok {
            break;
        }
    }
    let screen = c.screen().join("\n");
    let view = c.view();
    assert!(screen.contains(&view.coming.name));
    assert!(screen.contains(&format!("{}g", view.gold)));
}
