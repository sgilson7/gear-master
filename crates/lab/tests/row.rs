//! A run in the row says what it pressed, and what it pressed replays.
//!
//! `qproof`'s contract, in the loop that is going to be producing them: a
//! transcript is a proof only if a fresh console fed the same keys ends up in
//! the same place. The episode watcher hands `Ran::tape` to the window, so if
//! this is not true the window shows a run that never happened - which is the
//! class of fault this mission keeps finding and the reason `qproof` verifies
//! before it writes.
//!
//! `cargo test -p gearmaster-lab --test row`. Nothing runs the lab suite by
//! habit (`CLAUDE.md` trap 46), so this is in the run list in §1.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_lab::row;
use gearmaster_trades::env::Move;

/// Rogue, because that is what `qrow` trains and what the watcher will emit.
const MODE: Mode = Mode::Rogue;
const SEED: u64 = 0x0D0E_5EED;
const BUDGET: usize = 40;

/// A packer with no opinions: the first key on the menu, every time.
///
/// Not a policy and not meant to be. What is being tested is that a tape of
/// whatever was pressed replays, and a fixed chooser makes the run the same
/// every time this suite is run.
fn first_key(c: &mut Console) -> Vec<Verb> {
    let pressed = row::pack_with(c, BUDGET, |_c, ms| {
        ms.iter().position(|m| matches!(m, Move::Press(_))).unwrap_or(0)
    });
    row::keys(&pressed)
}

/// Replay a tape into a fresh console and say where it got and what it refused.
fn replay(tape: &[Verb]) -> (usize, usize) {
    let mut c = Console::start(SEED, MODE, Difficulty::Medium);
    let (mut best, mut refused) = (1usize, 0usize);
    for v in tape {
        if !c.apply(*v).ok {
            refused += 1;
        }
        best = best.max(c.view().rung_shown);
    }
    (best, refused)
}

#[test]
fn what_a_run_pressed_replays_to_the_rung_it_reached() {
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    let (best, refused) = replay(&out.tape);
    assert_eq!(refused, 0, "{refused} of {} keys were refused on replay", out.tape.len());
    assert_eq!(
        best, out.deepest,
        "the run reached rung {} and its own tape replays to {best}",
        out.deepest
    );
}

/// **And the run has to be worth replaying.**
///
/// Six proofs in `analysis/proofs/` claim rung 1, and a rung-1 run replays to
/// rung 1 under any mode, with any board, having pressed almost nothing - so it
/// passes the test above while proving nothing at all. That is how
/// `tests/proofs.rs` has stayed green while replaying every Rogue proof as
/// Grinder. A test whose subject is trivial is a test that cannot fail.
#[test]
fn the_run_being_replayed_is_not_a_trivial_one() {
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    assert!(
        out.deepest >= 2,
        "a run that never left rung 1 replays vacuously; this one reached {}",
        out.deepest
    );
    assert!(out.packs > 0, "nothing was packed");
    assert!(out.tape.len() > 10, "a tape of {} keys is not a run", out.tape.len());
}

/// The packing has to be *on* the tape, not merely the road and the fights.
///
/// A transcript missing the packing replays into a different board, which is
/// `qproof`'s own reason for recording all three halves. The road and the
/// fights are pressed inside `row::run`; the packing is pressed by a closure
/// the caller owns, and it reaches the tape only because the closure hands it
/// back. If that ever stops happening the tape still replays - into an empty
/// board - so this is the assertion that catches it.
#[test]
fn the_packing_is_on_the_tape_and_not_only_the_road() {
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    let packing = out
        .tape
        .iter()
        .filter(|v| {
            gearmaster_trades::partition::owner(**v) == gearmaster_trades::Trade::Quartermaster
        })
        .count();
    assert!(packing > 0, "the tape has {} keys and none of them pack", out.tape.len());
    let road = out.tape.len() - packing;
    assert!(road > 0, "the tape has no road keys, so `walk_on` taped nothing");
}

/// A control that does not report leaves an honest hole rather than a wrong tape.
#[test]
fn a_packer_that_does_not_report_leaves_the_packing_off_the_tape() {
    let mut control = |c: &mut Console| {
        gearmaster_lab::packers::control(c, BUDGET);
        Vec::new()
    };
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut control);
    let packing = out
        .tape
        .iter()
        .filter(|v| {
            gearmaster_trades::partition::owner(**v) == gearmaster_trades::Trade::Quartermaster
        })
        .count();
    assert_eq!(packing, 0, "the written control reports nothing, so it tapes nothing");
    assert!(!out.tape.is_empty(), "but the road and the fights are still taped");
}

// ---- what the tape is written as ------------------------------------------

/// A directory of this test's own, named for the test that uses it.
fn scratch(what: &str) -> String {
    let dir = std::env::temp_dir().join(format!("gearmaster-proof-{what}"));
    std::fs::remove_dir_all(&dir).ok();
    dir.to_string_lossy().into_owned()
}

#[test]
fn a_written_proof_replays_and_says_what_it_claims() {
    let dir = scratch("written");
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    let path = gearmaster_lab::proof::write(
        &dir,
        "ep-000000",
        SEED,
        MODE,
        Difficulty::Medium,
        &out.tape,
        &out.pack_ends,
        out.deepest,
        &[("episode", "0".into()), ("epsilon", "1.00".into())],
    )
    .expect("a tape that replays is a proof");

    // Parsed back the way `gui::watch` and `lab/tests/proofs.rs` parse it, by
    // column. If this drifts, the window stops being able to open what the
    // trainer writes and nothing else says so.
    let text = std::fs::read_to_string(&path).expect("readable");
    let seed = text
        .lines()
        .find_map(|l| l.strip_prefix("# seed        0x"))
        .and_then(|r| u64::from_str_radix(r.trim(), 16).ok())
        .expect("a seed in the header");
    let claimed: usize = text
        .lines()
        .find_map(|l| l.strip_prefix("# reached     rung "))
        .and_then(|r| r.split_whitespace().next())
        .and_then(|r| r.parse().ok())
        .expect("a reached in the header");
    assert_eq!(seed, SEED);
    assert_eq!(claimed, out.deepest);
    assert!(text.contains("# mode        Rogue"), "the mode has to be in there - see trap on proofs.rs");
    assert!(
        text.contains("# difficulty  Medium"),
        "one spelling of the difficulty, the one every other proof uses"
    );

    // And the keys survive the round trip through text.
    let keys: Vec<Verb> = text.lines().filter_map(Verb::parse).collect();
    assert_eq!(keys, out.tape, "every key written parses back to the key it was");
}

/// The refusal is the point of the exercise.
#[test]
fn a_tape_that_does_not_replay_is_refused_rather_than_written() {
    let dir = scratch("refused");
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    // A rung it never reached. Nothing else about the tape is wrong, which is
    // exactly the shape a stale claim takes.
    let err = gearmaster_lab::proof::write(
        &dir,
        "ep-000000",
        SEED,
        MODE,
        Difficulty::Medium,
        &out.tape,
        &out.pack_ends,
        out.deepest + 5,
        &[],
    )
    .expect_err("a claim the replay disagrees with is not a proof");
    assert!(err.contains("claims rung"), "the refusal says which number was wrong: {err}");
    assert!(
        gearmaster_lab::proof::listed(&dir).is_empty(),
        "and it wrote no file"
    );
}

#[test]
fn pruning_keeps_the_newest_and_drops_the_rest() {
    let dir = scratch("pruned");
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    for ep in [0usize, 25, 50, 75] {
        gearmaster_lab::proof::write(
            &dir,
            &format!("ep-{ep:06}"),
            SEED,
            MODE,
            Difficulty::Medium,
            &out.tape,
            &out.pack_ends,
            out.deepest,
            &[],
        )
        .expect("a proof");
    }
    assert_eq!(gearmaster_lab::proof::listed(&dir).len(), 4);
    assert_eq!(gearmaster_lab::proof::prune(&dir, 2), 2, "two of four go");
    let left: Vec<String> = gearmaster_lab::proof::listed(&dir)
        .iter()
        .map(|p| p.file_name().expect("a name").to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["ep-000050.proof", "ep-000075.proof"], "the newest by name");
}

/// A watcher reads this directory while the trainer writes to it.
///
/// `fs::write` is not atomic, so a proof has to appear whole or not at all -
/// otherwise a window opens a truncated episode under a header claiming a rung
/// it never reaches, and nothing anywhere says so. The temporary is not a
/// `.proof`, so it is invisible to the watcher's own listing until the rename.
#[test]
fn a_proof_appears_whole_or_not_at_all() {
    let dir = scratch("atomic");
    let (_, out) = row::run(SEED, MODE, Difficulty::Medium, &mut first_key);
    gearmaster_lab::proof::write(
        &dir,
        "ep-000000",
        SEED,
        MODE,
        Difficulty::Medium,
        &out.tape,
        &out.pack_ends,
        out.deepest,
        &[],
    )
    .expect("a proof");
    let left: Vec<String> = std::fs::read_dir(&dir)
        .expect("a directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["ep-000000.proof"], "no temporary is left behind");
    assert_eq!(gearmaster_lab::proof::listed(&dir).len(), 1);
}

/// **The redistributed run is worth exactly what the run was worth.**
///
/// That identity is the whole licence for `row::spread`: a reward redistribution
/// preserves the optimal policy when it is *return-equivalent*, which means the
/// sum over the episode is unchanged. If it drifts, the agent is being trained
/// on a different objective than the one being reported, and nothing in a
/// training curve would say so.
mod decomposition {
    use gearmaster_lab::row;

    fn total(v: &[f32]) -> f32 {
        v.iter().sum()
    }

    /// A run that climbed 1 -> 2 -> 3 -> 4 without losing a fight.
    #[test]
    fn a_clean_climb_pays_the_square_of_the_rung_it_reached() {
        let rungs = vec![1, 2, 3];
        let worth = 4.0f32.powf(row::pow()) * row::RUNG;
        let pay = row::spread(&rungs, 4, worth);
        assert_eq!(pay.len(), 3);
        assert!((total(&pay) - worth).abs() < 1e-4, "paid {} for a worth of {worth}", total(&pay));
        // And it telescopes: each packing is paid the increment its fight won.
        assert!((pay[0] - (1.0 + (4.0 - 1.0))).abs() < 1e-4, "the first packing got {}", pay[0]);
        assert!((pay[1] - (9.0 - 4.0)).abs() < 1e-4, "the second got {}", pay[1]);
        assert!((pay[2] - (16.0 - 9.0)).abs() < 1e-4, "the third got {}", pay[2]);
    }

    /// A fight that does not move the rung is a fight that was lost, which is
    /// the rule `row::run` counts `losses` by - so the life comes off at the
    /// packing that cost it and not at the end.
    #[test]
    fn a_lost_fight_costs_a_life_where_it_was_lost() {
        // 1 -> 1 (lost) -> 2 -> 2 (lost), ending on 2.
        let rungs = vec![1, 1, 2];
        let worth = 2.0f32.powf(row::pow()) * row::RUNG - 2.0 * row::LIFE;
        let pay = row::spread(&rungs, 2, worth);
        assert!((total(&pay) - worth).abs() < 1e-4, "paid {} for a worth of {worth}", total(&pay));
        assert!(pay[0] < pay[1], "the packing whose fight was lost paid worse");
    }

    /// Knocked back and climbing again pays nothing for ground already bought.
    ///
    /// The depth term is a **high-water mark**, so a run that reaches rung five,
    /// falls to three and climbs back is paid once for the five. Anything else
    /// would make being knocked back a source of income.
    #[test]
    fn ground_won_twice_is_paid_for_once() {
        let rungs = vec![1, 3, 5, 3, 4];
        let worth = 5.0f32.powf(row::pow()) * row::RUNG - 2.0 * row::LIFE;
        let pay = row::spread(&rungs, 5, worth);
        assert!((total(&pay) - worth).abs() < 1e-4, "paid {} for a worth of {worth}", total(&pay));
        // The climb back from 3 to 4 to 5 buys no depth: it has been paid for.
        assert!(pay[3] <= 0.0, "re-treading rung 4 paid {}", pay[3]);
    }

    /// The identity holds over a run the harness actually played, which is the
    /// case the hand-written ones above are a model of.
    #[test]
    fn it_holds_over_a_run_that_was_really_played() {
        use gearmaster_console::{Console, Difficulty, Mode};
        let mut rungs: Vec<usize> = Vec::new();
        let mut pack = |c: &mut Console| {
            rungs.push(c.view().rung_shown);
            gearmaster_lab::packers::control(c, 40);
            Vec::new()
        };
        let (c, ran) = row::run(0x0D0E_5EED, Mode::Rogue, Difficulty::Medium, &mut pack);
        let worth = row::worth(&ran);
        let pay = row::spread(&rungs, c.view().rung_shown, worth);
        assert_eq!(pay.len(), rungs.len(), "one payment a packing");
        assert!(
            (total(&pay) - worth).abs() < 1e-3,
            "a real run reaching rung {} was worth {worth} and paid {}",
            ran.deepest,
            total(&pay)
        );
    }
}
