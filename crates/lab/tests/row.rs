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

/// **A press that puts the board back where it has been costs something, and
/// nothing else does.**
///
/// `CLAUDE.md` trap 44 has been relocated five times - `Rotate`, `Pin`, `Undo`,
/// `Lock` half of it unlocking, and `Undo` again - by two verb removals and two
/// feature fixes. This is the rule that does not care which key it was, and
/// these are the four things it has to get right for that to be worth anything.
mod revisits {
    use gearmaster_console::{Console, Difficulty, Mode, SlotKind, Verb};
    use gearmaster_lab::row;

    /// A run with something in the tray, ready to be packed.
    fn a_tray() -> Console {
        Console::start(0x0D0E_5EED, Mode::Rogue, Difficulty::Medium)
    }

    fn seat(c: &mut Console) -> Verb {
        let v = c
            .menu()
            .into_iter()
            .find(|v| matches!(v, Verb::Place { .. }))
            .expect("something to place");
        assert!(c.apply(v).ok, "the placement sticks");
        v
    }

    #[test]
    fn placing_a_piece_and_undoing_it_is_charged_and_the_placement_is_not() {
        let mut c = a_tray();
        let start = row::fingerprint(&c);
        let placed = seat(&mut c);
        let after = row::fingerprint(&c);
        assert_ne!(start, after, "a placement moves the board");
        assert!(c.apply(Verb::Undo).ok, "and it can be taken back");
        assert_eq!(
            row::fingerprint(&c),
            start,
            "undoing a placement puts the board exactly back, which is what makes \
             place-then-undo a no-op pair rather than two decisions"
        );
        let _ = placed;
    }

    /// The one that keeps the rule honest.
    ///
    /// L measured a packer pressing `lock` on half its choices with half of
    /// those unlocking. A rule that charged for "a press that changed no
    /// figures" would charge the lock as well as the unlock, and locking is a
    /// thing a packer genuinely needs to do - `analysis/the-collapse.md` M1.1.
    /// The fingerprint reads `item.locked`, so only the unlock returns anywhere.
    #[test]
    fn locking_is_free_and_unlocking_again_is_not() {
        let mut c = a_tray();
        // Seat pieces until something assembles, so there is an item to lock.
        let mut item = None;
        for _ in 0..60 {
            let assembled: Vec<_> = c
                .view()
                .grids
                .iter()
                .flat_map(|g| g.items.iter())
                .filter(|i| i.assembled)
                .filter_map(|i| i.pieces.first().copied())
                .collect();
            if let Some(&first) = assembled.first() {
                item = Some(first);
                break;
            }
            if c.menu().iter().any(|v| matches!(v, Verb::Place { .. })) {
                seat(&mut c);
            } else if let Some(buy) =
                c.menu().into_iter().find(|v| matches!(v, Verb::Buy { .. }))
            {
                if !c.apply(buy).ok {
                    break;
                }
            } else {
                break;
            }
        }
        let Some(item) = item else {
            // No item on this seed within the budget; the claim is about the
            // rule, and `locking_and_unlocking_do_not_describe_identically` in
            // `trades` covers the same ground from the feature side.
            return;
        };
        let before = row::fingerprint(&c);
        assert!(c.apply(Verb::Lock { piece: item }).ok, "it locks");
        let locked = row::fingerprint(&c);
        assert_ne!(before, locked, "locking is a real change and must not read as a revisit");
        assert!(c.apply(Verb::Lock { piece: item }).ok, "and the same key unlocks it");
        assert_eq!(
            row::fingerprint(&c),
            before,
            "unlocking puts the board back, so the second press is the one that pays"
        );
    }

    /// Two different placements are two decisions, not a cycle.
    #[test]
    fn two_different_placements_are_not_a_revisit() {
        let mut c = a_tray();
        let mut seen = vec![row::fingerprint(&c)];
        for _ in 0..3 {
            if !c.menu().iter().any(|v| matches!(v, Verb::Place { .. })) {
                break;
            }
            seat(&mut c);
            let f = row::fingerprint(&c);
            assert!(!seen.contains(&f), "each placement is a board nobody has seen before");
            seen.push(f);
        }
        assert!(seen.len() > 1, "the fixture placed nothing and proves nothing");
    }

    /// And the charge itself: flat, per packing, and zero without a revisit.
    ///
    /// Flat because revisits are not sparse - 80.3% of presses, measured - and
    /// an increasing charge over that is quadratic in the packing's length.
    #[test]
    fn every_revisit_costs_the_same_and_nothing_else_costs_anything() {
        let same = |state: u64| row::Pressed {
            before: Default::default(),
            after: Default::default(),
            items_after: 0,
            verb: Some(Verb::Undo),
            stuck: true,
            state,
        };
        // Start at 1; press to 2, back to 1, to 2, to 1: three revisits.
        let packing = [same(2), same(1), same(2), same(1)];
        let pay = row::revisit_penalty(&packing, 1);
        let c = row::revisit();
        assert_eq!(pay[0], 0.0, "a board nobody has seen is not a revisit");
        for (i, p) in pay.iter().enumerate().skip(1) {
            assert!((p + c).abs() < 1e-6, "revisit {i} cost {p}, and every one costs {c}");
        }

        // A packing that never doubles back pays nothing at all.
        let forward = [same(2), same(3), same(4)];
        assert_eq!(row::revisit_penalty(&forward, 1), vec![0.0, 0.0, 0.0]);
    }
}

/// Two things the fingerprint has to get right about the shop, and it got one
/// of them wrong first.
///
/// A **pin** holds a shelf through a restock and changes nothing about what the
/// board is or what it can fight, so it must read as a press that left the
/// board where it was. Hashing the flag made every pin configuration novel -
/// sixty-four of them across six shelves, more than the forty-press budget can
/// spend - and `pin` went from 0.0% of a trained policy's choices to 22.7%
/// under the charge. A finer fingerprint is not a better one.
///
/// A **reroll** changes the stock and costs gold, so it must read as novel;
/// charging for it would be a step charge on the one key that buys new options.
#[test]
fn pinning_a_shelf_is_a_revisit_and_rerolling_it_is_not() {
    use gearmaster_console::{Console, Difficulty, Mode, Verb};
    use gearmaster_lab::row;
    let mut c = Console::start(0x0D0E_5EED, Mode::Rogue, Difficulty::Medium);
    let before = row::fingerprint(&c);

    let pin = c
        .menu()
        .into_iter()
        .find(|v| matches!(v, Verb::Pin { .. }))
        .expect("a shelf to hold");
    assert!(c.apply(pin).ok, "it pins");
    assert_eq!(
        row::fingerprint(&c),
        before,
        "a pin changes nothing the board does, so it has to read as a no-op"
    );

    if c.menu().contains(&Verb::Reroll) {
        assert!(c.apply(Verb::Reroll).ok, "it rerolls");
        assert_ne!(
            row::fingerprint(&c),
            before,
            "a reroll costs gold and changes the shelves, so it is a real decision"
        );
    }
}

/// **Saying "I am done" is not putting the board back where it was.**
///
/// `Move::Done` leaves the board exactly as it stands, so a fingerprint test
/// calls it a revisit - and charging it puts a price on the one action a
/// dithering packer is supposed to reach for. `Packing`'s own doc comment says
/// why it exists: *without it a packer dithers, and a step cost alone does not
/// teach it to stop; it teaches it to press the cheapest key.*
///
/// Measured over four trained nets, `Done` is offered at every single decision
/// and chosen on 0.0% to 0.9% of them. It does not need a reason to be chosen
/// less often.
#[test]
fn saying_done_is_not_charged_as_a_revisit() {
    use gearmaster_console::Verb;
    use gearmaster_lab::row;
    let at = |state: u64, verb: Option<Verb>| row::Pressed {
        before: Default::default(),
        after: Default::default(),
        items_after: 0,
        verb,
        stuck: true,
        state,
    };
    // A press back to the start, then `Done`, which changes nothing further.
    let packing = [at(2, Some(Verb::Undo)), at(1, Some(Verb::Undo)), at(1, None)];
    let pay = row::revisit_penalty(&packing, 1);
    assert_eq!(pay[0], 0.0);
    assert!(pay[1] < 0.0, "going back to the start is a revisit");
    assert_eq!(pay[2], 0.0, "`Done` sits on a board it has seen and pays nothing");
}

/// Ending the packing where it starts going in circles.
///
/// N closed the case for *pricing* a revisit: two verb removals, two feature
/// fixes and a charge, and the share of presses that put the board back where
/// it was finished at 76%. M.2 gives the arithmetic - no charge is above the
/// network's error floor and below the objective at once. Ending the packing is
/// the only quantity in this system that is, because the cost is the rest of
/// the budget.
mod ending {
    use gearmaster_console::{Console, Difficulty, Mode, Verb};
    use gearmaster_lab::row;
    use gearmaster_trades::env::Move;

    fn a_run() -> Console {
        Console::start(0x0D0E_5EED, Mode::Rogue, Difficulty::Medium)
    }

    /// A chooser that seats a piece and takes it straight back.
    fn cycling(n: &mut usize) -> impl FnMut(&Console, &[Move]) -> usize + '_ {
        move |_, ms| {
            *n += 1;
            if *n % 2 == 1 {
                ms.iter()
                    .position(|m| matches!(m, Move::Press(Verb::Place { .. })))
                    .unwrap_or(0)
            } else {
                ms.iter().position(|m| matches!(m, Move::Press(Verb::Undo))).unwrap_or(0)
            }
        }
    }

    /// A packer that seats a piece and takes it back gets two presses under the
    /// rule and the whole budget without it.
    #[test]
    fn a_packing_that_cycles_stops_and_one_that_does_not_runs_on() {
        let mut n = 0usize;
        let mut c = a_run();
        let stopped = row::pack_with_ending(&mut c, 12, Some(0), cycling(&mut n));
        assert_eq!(
            stopped.len(),
            2,
            "the undo puts the board back where the packing began, and that is where \
             it ends - two presses of twelve, which is the whole point: the cost of a \
             cycle is the budget it did not spend"
        );

        // Tolerating two lets the cycle run twice more before it bites.
        let mut n = 0usize;
        let mut c = a_run();
        let tolerant = row::pack_with_ending(&mut c, 12, Some(2), cycling(&mut n));
        assert!(
            tolerant.len() > stopped.len(),
            "a tolerance of two ran {} presses against {}",
            tolerant.len(),
            stopped.len()
        );

        // And off is exactly what it always was.
        let mut n = 0usize;
        let mut c = a_run();
        let mut m = 0usize;
        let mut c2 = a_run();
        assert_eq!(
            row::pack_with_ending(&mut c, 12, None, cycling(&mut n)).len(),
            row::pack_with(&mut c2, 12, cycling(&mut m)).len(),
            "`pack_with` is `pack_with_ending` with the switch read from the edge"
        );

        let mut c = a_run();
        let mut n = 0usize;
        let pressed = row::pack_with(&mut c, 12, |_, ms| {
            n += 1;
            // Alternate: the first placement, then undo, for ever.
            if n % 2 == 1 {
                ms.iter()
                    .position(|m| matches!(m, Move::Press(Verb::Place { .. })))
                    .unwrap_or(0)
            } else {
                ms.iter().position(|m| matches!(m, Move::Press(Verb::Undo))).unwrap_or(0)
            }
        });
        assert_eq!(
            pressed.len(),
            12,
            "with the switch off a cycle spends the whole budget, which is what N \
             measured at 76% of presses"
        );
    }

    /// And the fingerprints say the cycle is a cycle, which is what the switch
    /// keys on. Two presses in, the board is back where it started.
    #[test]
    fn the_fingerprints_of_a_cycle_repeat() {
        let mut c = a_run();
        let start = row::fingerprint(&c);
        let mut n = 0usize;
        let pressed = row::pack_with(&mut c, 6, |_, ms| {
            n += 1;
            if n % 2 == 1 {
                ms.iter()
                    .position(|m| matches!(m, Move::Press(Verb::Place { .. })))
                    .unwrap_or(0)
            } else {
                ms.iter().position(|m| matches!(m, Move::Press(Verb::Undo))).unwrap_or(0)
            }
        });
        let states: Vec<u64> = pressed.iter().map(|p| p.state).collect();
        assert!(states.len() >= 4, "the fixture pressed {} times", states.len());
        assert_eq!(states[1], start, "the undo puts it back where the packing began");
        assert_eq!(states[3], start, "and again");
        assert_eq!(states[0], states[2], "and the placement is the same board twice");
    }
}
