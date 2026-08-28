//! Every signature is a ratio over the fight, and this is what says so.
//!
//! `CLAUDE.md` §6 trap 29 in the form it takes for a number an agent will
//! optimise: **ask what the cheapest way to satisfy a check is before shipping
//! it.** "Carries a Searing curse" is satisfied by seating one piece.
//! "Most of the damage arrived as Searing ticks" cannot be satisfied except by
//! building the creature the theme describes.
//!
//! So the test is not that the meter gives high scores to the right creatures.
//! It is that a fight in which nothing happens scores nothing, that carrying
//! the vocabulary without using it scores nothing, and that the parts move
//! when the fight moves.

use gearmaster_engine::bestiary::MonsterTheme;
use gearmaster_engine::combat::{simulate_at, Difficulty, LADDER};
use gearmaster_oracle::fidelity::{self, Reading};
use gearmaster_oracle::gate::References;

fn read(board: usize, name: &str) -> Reading {
    let refs = References::standard();
    let (_, stats, items, _) = &refs.boards[board];
    let spec = LADDER.iter().find(|s| s.name == name).expect("on the ladder");
    Reading::of(&simulate_at(*stats, items, spec, Difficulty::Medium))
}

#[test]
fn a_fight_where_nothing_happened_scores_nothing() {
    let empty = Reading::default();
    for t in MonsterTheme::ALL {
        let f = fidelity::score(t, &empty);
        assert!(
            f.score < 0.35,
            "{} scored {:.2} on a fight in which nothing happened: {:?}",
            t.name(),
            f.score,
            f.parts
        );
    }
}

#[test]
fn carrying_the_vocabulary_is_not_using_it() {
    // Bone Cantor is a Burner and its gear says so four times over - two
    // Searing items, one of which applies the curse twice. Against a board
    // that can feel it, the curse lands. It still reads as a Burner only if
    // the *burn* carries the fight, and it does not: the blows do.
    let r = read(0, "Bone Cantor");
    assert!(r.curses > 0, "it does apply the curse - the gear is not the problem");
    assert!(
        r.burn_damage < r.blow_damage / 4,
        "burn {} against blows {}",
        r.burn_damage,
        r.blow_damage
    );
    let f = fidelity::score(MonsterTheme::Burner, &r);
    assert!(
        f.score < 0.2,
        "a creature that kills on the swing must not read as one that kills on the clock: {:.2}",
        f.score
    );
}

#[test]
fn a_board_that_cannot_feel_a_theme_measures_the_board() {
    // Curse resistance is clamped to 100 and scales a curse to nothing at that
    // point (`curse.rs:137`). The owner's finished build carries 145. So the
    // same creature, against two boards, gives two different readings - and
    // the one against the immune board is a fact about the board.
    let felt = read(0, "Ember Wisp");
    let curve = read(2, "Ember Wisp");
    assert!(felt.curses > 0, "a Slower curses a board that can be cursed");
    assert_eq!(curve.curses, 0, "and lands nothing at all on one that cannot");
    assert!(
        fidelity::score(MonsterTheme::Slower, &felt).score
            > fidelity::score(MonsterTheme::Slower, &curve).score,
        "which the meter has to show rather than average away"
    );
}

#[test]
fn the_parts_are_carried_rather_than_recomputed() {
    // One walk, one classification - `stats.rs`'s argument, and the reason
    // `Fidelity` holds its parts instead of offering a second function that
    // works them out again.
    let r = read(0, "Cog Priest");
    let f = fidelity::score(MonsterTheme::Slower, &r);
    assert!(!f.parts.is_empty());
    let mean = f.parts.iter().map(|(_, v)| v).sum::<f64>() / f.parts.len() as f64;
    assert!((mean - f.score).abs() < 1e-9, "the score is the mean of its own parts");
    assert_eq!(f.reading, r, "and it carries the reading it was scored from");
}

#[test]
fn every_part_stays_inside_zero_and_one() {
    // A ramp that runs off the end would let one half of a claim pay for the
    // other, and a score above one would say a creature is more than what its
    // theme describes - which is not a thing the meter can mean.
    let refs = References::standard();
    for spec in LADDER.iter() {
        for t in MonsterTheme::ALL {
            for board in 0..refs.boards.len() {
                let (_, stats, items, _) = &refs.boards[board];
                let log = simulate_at(*stats, items, spec, Difficulty::Medium);
                let f = fidelity::of(t, &log);
                for (what, v) in &f.parts {
                    assert!(
                        (0.0..=1.0).contains(v),
                        "{} / {} / board {}: `{}` read {}",
                        spec.name,
                        t.name(),
                        board,
                        what,
                        v
                    );
                }
                assert!((0.0..=1.0).contains(&f.score));
            }
        }
    }
}
