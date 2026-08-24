//! The last thing on the ladder, and how hard it is.
//!
//! Francis was on thirty-six percent of his cells with one item a slot. That
//! is not a hard fight, it is four fifths of an empty board: the two finished
//! human boards in `share` pack ninety-seven and ninety-eight percent, and the
//! stronger of them took him on Hard in nine and a half seconds.
//!
//! He is packed by `tests/pack_francis.rs`, which is a generator rather than a
//! check. What this file does is hold the result: density, shape, and the
//! outcome against both boards, so a later change to gear or to the rating
//! curve cannot quietly hand him back.

use gearmaster_engine::combat::{simulate_at, Difficulty, Outcome, LADDER};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::share;

fn francis() -> &'static gearmaster_engine::combat::MonsterSpec {
    LADDER.iter().find(|m| m.name == "Francis").expect("the top of the ladder")
}

fn board(code: &str) -> Run {
    let sh = share::import(code).expect("reads");
    let mut r = Run::new();
    r.mode = Mode::Grinder;
    r.loadout.grow(sh.extra_rows);
    for (d, sl, x, y, rot) in &sh.placed {
        let id = r.registry.alloc(*d);
        r.owned.push(id);
        r.registry.set_rotation(id, *rot);
        if r.equip(id, *sl, *x, *y).is_err() {
            r.owned.pop();
        }
    }
    for c in &sh.classes {
        if let Some(k) = gearmaster_engine::class::CLASSES.iter().find(|k| k.name == *c) {
            r.classes.push(k);
        }
    }
    r.refresh_class_effects();
    r
}

fn wins(code: &str, d: Difficulty) -> bool {
    let run = board(code);
    simulate_at(run.player_stats(), &run.combat_items(), francis(), d).outcome == Outcome::Victory
}

#[test]
fn his_boards_are_packed_like_somebody_lives_in_them() {
    let (reg, lo) = francis().loadout_at(Difficulty::Medium);
    let mut used = 0;
    let mut items = 0;
    for slot in SlotKind::ALL {
        let s = lo.slot(slot);
        used += (0..s.rows())
            .flat_map(|y| (0..6u8).map(move |x| (x, y)))
            .filter(|&(x, y)| s.get(x, y).is_some())
            .count();
        items += lo.report(&reg, slot).items.iter().filter(|i| i.assembled).count();
    }
    assert!(used >= 150, "{used} of 240 cells - he is back to standing in an empty wardrobe");
    // And not so packed that he stops being a person. A player's finished
    // board carries twelve or thirteen items; twenty of them out-damages
    // anything the game can hand anybody, which the first attempt at this did:
    // it killed both finished boards in under three seconds at every setting,
    // and dropping his own health and strength by three quarters changed
    // nothing, because none of the damage was his.
    assert!(items <= 15, "{items} items is more than any player can carry");
}

#[test]
fn he_carries_one_sword() {
    let (reg, lo) = francis().loadout_at(Difficulty::Medium);
    let swings = lo.report(&reg, SlotKind::Weapon).items.iter().filter(|i| i.assembled).count();
    assert_eq!(swings, 1, "a creature with {swings} weapons swings {swings} times a cooldown");
}

#[test]
fn the_strongest_board_in_the_project_no_longer_walks_through_him() {
    // The whole point of the repack. The friend's build cleared the ladder and
    // used to take Francis on Hard in nine and a half seconds.
    assert!(wins(share::A_FRIENDS_RUN, Difficulty::Easy), "he is now unbeatable, which is not the ask");
    assert!(wins(share::A_FRIENDS_RUN, Difficulty::Medium));
    assert!(!wins(share::A_FRIENDS_RUN, Difficulty::Hard), "Hard is still a walk");
    assert!(!wins(share::A_FRIENDS_RUN, Difficulty::Insane));
}

#[test]
fn the_owners_board_gets_exactly_one_setting() {
    assert!(wins(share::A_WINNING_RUN, Difficulty::Easy));
    for d in [Difficulty::Medium, Difficulty::Hard, Difficulty::Insane] {
        assert!(!wins(share::A_WINNING_RUN, d), "{} should be past this board", d.name());
    }
}

#[test]
fn he_still_wears_his_own_coat_and_nobody_elses() {
    let names: Vec<&str> = francis().gear.iter().map(|&(n, ..)| n).collect();
    assert!(names.contains(&"The Money Jacket"), "the coat is the one strange thing he owns");
    for n in &names {
        if gearmaster_engine::piece::is_boss_only(n) {
            assert_eq!(*n, "The Money Jacket", "{n} belongs to another creature");
        }
    }
}
