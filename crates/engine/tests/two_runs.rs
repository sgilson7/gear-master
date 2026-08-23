//! Two runs, played rather than poked at.
//!
//! Every other test here sets a field and asks a question. These start at rung
//! one with a board and a purse, fight what the ladder puts in front of them,
//! answer whatever turns up, and carry on - which is the only way to catch the
//! faults that live between the parts rather than inside one.
//!
//! One build is sharp enough to earn the casino. The other is deliberately
//! blunt, so its fights run long and the other door opens instead.

use gearmaster_engine::combat::{Difficulty, Outcome};
use gearmaster_engine::event::{Outcome as ChoiceOutcome, EVENTS};
use gearmaster_engine::run::{Mode, Run};

/// A run with a complete, properly assembled board.
///
/// `apply_preset` rather than a list of component names at chosen cells.
/// Hand-seating does not produce the build you meant - pieces join their
/// nearest core and come out as something else - and three attempts at a
/// "sharp" list here produced weapons of nine, sixteen and twenty-five damage
/// a second while the auto-builder managed forty-five. The design document
/// says as much; this is the third time it has been true.
fn a_run(difficulty: Difficulty) -> Run {
    let mut run = Run::new();
    run.difficulty = difficulty;
    run.mode = Mode::Grinder;
    run.gold = 500;
    run.apply_preset();
    assert!(!run.combat_items().is_empty(), "the preset built nothing");
    run
}

/// What happened while walking a run up the ladder.
#[derive(Default, Debug)]
struct Trace {
    events: Vec<&'static str>,
    choices: Vec<&'static str>,
    reached: usize,
    wins: u32,
}

/// Play up to `until`, answering every event with the first choice `pick`
/// accepts, and stop when the run stops getting anywhere.
///
/// A Grinder that cannot beat the rung it is on loses, is knocked back, wins
/// the easier one, and comes straight back to lose again - for ever. That is
/// the game working, not a fault, so the walk gives up rather than asserting.
fn play(
    run: &mut Run,
    until: usize,
    pick: impl Fn(&gearmaster_engine::event::Choice) -> bool,
) -> Trace {
    let mut t = Trace::default();
    let (mut best, mut stuck) = (run.rung, 0);

    while run.rung < until && stuck < 40 {
        if run.rung > best {
            best = run.rung;
            stuck = 0;
        } else {
            stuck += 1;
        }

        if let Some(ev) = run.pending_event() {
            t.events.push(ev.id);
            let choice = ev
                .choices
                .iter()
                .find(|c| run.choice_open(c) && pick(c))
                .or_else(|| ev.choices.iter().find(|c| run.choice_open(c)))
                .unwrap_or_else(|| panic!("{}: no choice could be taken at all", ev.id));
            t.choices.push(choice.label);
            run.take_choice(choice);
            // Answering must leave it behind, or the run is in a loop nobody
            // can see from inside it.
            if let Some(next) = run.pending_event() {
                assert_ne!(next.id, ev.id, "{} asked itself again", ev.id);
            }
            continue;
        }

        // A fight an event arranged stands beside the rung, not on it.
        if let Some(specs) = run.pending_brawl() {
            let before = run.rung;
            run.fight_party(&specs);
            run.settle();
            assert_eq!(run.rung, before, "an arranged fight moved the ladder");
            assert!(run.brawl.is_none(), "the arranged fight is still pending");
            continue;
        }

        if run.fight_next().outcome == Outcome::Victory {
            t.wins += 1;
        }
        run.settle();
        // Back to the board. `settle` does not do this for an ordinary fight -
        // the interface holds you on the replay until you ask to leave - and
        // `pending_event` is gated on the phase, so a walk that forgets this
        // silently sees no events at all. Which is exactly what the first
        // version of this file did.
        run.back_to_loadout();
        assert!(run.gold >= 0, "the purse went negative at rung {}", run.rung + 1);
        assert!(run.loadout.rows() >= 8, "the boards shrank");
    }
    t.reached = run.rung;
    t
}

/// The board that cleared the game, worn from rung one.
///
/// Not a build assembled for this test - it is the owner's own winning run,
/// seventy-five pieces at ninety-seven percent of the cells, read back out of
/// `share::A_WINNING_RUN`. Nothing about it is seeded: it is quick enough to
/// open the casino because it is genuinely quick.
fn a_sharp_run() -> Run {
    let shared = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN)
        .expect("the winning code still reads");
    let mut run = Run::new();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.gold = 500;
    run.loadout.grow(shared.extra_rows);
    for (def, slot, x, y, rot) in &shared.placed {
        let Some(d) = gearmaster_engine::piece::CATALOG.get(*def) else { continue };
        let _ = d;
        let id = run.registry.alloc(*def);
        run.owned.push(id);
        run.registry.set_rotation(id, *rot);
        if run.equip(id, *slot, *x, *y).is_err() {
            // A piece that will not sit is a piece the board has outgrown;
            // the point is the shape of the build, not every last cell.
            run.owned.pop();
        }
    }
    assert!(
        run.combat_items().len() >= 8,
        "the winning board came back as {} items - it should be a full loadout",
        run.combat_items().len()
    );
    run.rung = 1;
    run
}

/// A run whose fights genuinely run long. Nothing is seeded: a complete board
/// on Medium takes well over ten seconds in the shallow end all by itself.
fn a_blunt_run() -> Run {
    let mut run = a_run(Difficulty::Medium);
    run.rung = 1;
    run
}

#[test]
fn a_sharp_run_finds_the_casino_and_walks_out_with_a_chip() {
    let mut run = a_sharp_run();
    let t = play(&mut run, 12, |c| matches!(c.outcome, ChoiceOutcome::Give(_)));

    assert!(
        t.events.contains(&"the-casino"),
        "reached rung {}, events {:?}, best win {:?}ms",
        t.reached + 1,
        t.events,
        run.best_fight_ms
    );
    assert!(
        run.owned.iter().any(|&i| run.registry.def(i).name == "Gold Chip"),
        "answered the casino and came away with nothing"
    );
    assert!(!t.events.contains(&"the-long-way"), "both doors opened: {:?}", t.events);
    // Earned, not arranged: a board that cleared the ladder is quick enough
    // for the door on its own.
    let best = run.best_fight_ms.expect("won something in the shallow end");
    assert!(best < 3_000, "the winning board took {best}ms and should not have got in");
    println!("the winning board's quickest shallow win: {best}ms");
}

#[test]
fn a_sharp_run_can_step_in_and_the_ladder_does_not_move() {
    let mut run = a_sharp_run();
    let rung = run.rung;
    let t = play(&mut run, 12, |c| matches!(c.outcome, ChoiceOutcome::Step(_)));
    assert!(t.events.contains(&"the-casino"));
    assert!(t.choices.contains(&"Step in"), "never stepped in: {:?}", t.choices);
    // Win or lose at the table, the run came back out and carried on.
    assert!(run.brawl.is_none());
    assert!(run.rung >= rung, "the table cost the run ground it had made");
}

#[test]
fn a_blunt_run_finds_the_other_door_all_by_itself() {
    let mut run = a_blunt_run();
    let t = play(&mut run, 12, |c| matches!(c.outcome, ChoiceOutcome::Claim("Trundle")));

    assert!(
        t.events.contains(&"the-long-way"),
        "reached rung {}, events {:?}, slowest win {:?}ms",
        t.reached + 1,
        t.events,
        run.worst_fight_ms
    );
    assert!(
        !t.events.contains(&"the-casino"),
        "a build this slow was offered the casino: best win {:?}ms",
        run.best_fight_ms
    );
    assert!(run.classes.iter().any(|c| c.name == "Trundle"), "walked past the pace");
}

#[test]
fn the_free_branch_of_the_long_way_leaves_a_note_for_later() {
    let mut run = a_blunt_run();
    let t = play(&mut run, 12, |c| c.label.starts_with("Ask"));
    assert!(t.events.contains(&"the-long-way"));
    assert!(
        run.took.iter().any(|l| l.starts_with("Ask")),
        "nothing was remembered, so the follow-up can never fire: {:?}",
        run.took
    );
    assert!(
        !run.classes.iter().any(|c| c.name == "Trundle"),
        "the free branch handed over the class anyway"
    );
}

/// What it actually takes to get through the casino door.
///
/// A complete auto-built board on Medium is nowhere near: its quickest win in
/// the shallow end is around seven seconds against a three-second bar, while
/// the board that actually cleared the game gets through comfortably. That is
/// the door working - it selects for a build that has gone all in on damage
/// early - but it is worth having written down, because it also means most
/// runs meet the *other* door rather than this one.
#[test]
fn the_casino_bar_is_high_and_the_other_door_is_not() {
    let mut run = a_run(Difficulty::Medium);
    run.rung = 1;
    play(&mut run, 10, |_| true);

    let best = run.best_fight_ms.expect("won something in the shallow end");
    let worst = run.worst_fight_ms.expect("same");
    assert!(
        best >= 3_000,
        "a plain board now clears the casino bar at {best}ms - retune or reword this"
    );
    assert!(
        worst > 10_000,
        "a plain board no longer trips the slow door at {worst}ms - the long way may be \
         unreachable for ordinary runs"
    );
}

#[test]
fn every_event_on_the_road_can_be_answered_and_left_behind() {
    // Walk taking the first open choice every time. Nothing may be asked
    // twice, and everything answered must be recorded.
    for name in ["sharp", "blunt"] {
        let mut run = if name == "sharp" { a_sharp_run() } else { a_blunt_run() };
        let t = play(&mut run, gearmaster_engine::combat::LADDER.len(), |_| true);
        let mut seen = t.events.clone();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "{name}: an event was asked twice: {:?}", t.events);
        for id in &t.events {
            assert!(run.answered.contains(id), "{name}: {id} was answered but not recorded");
        }
    }
}

#[test]
fn no_event_can_strand_a_run() {
    // Every event needs a choice open to a run carrying nothing at all, or a
    // player without the right component has nowhere to go.
    let run = Run::new();
    for e in EVENTS {
        assert!(
            e.choices.iter().any(|c| run.choice_open(c)),
            "{}: a run carrying nothing has no way past it",
            e.id
        );
    }
}
