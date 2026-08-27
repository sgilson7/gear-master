//! Five short chains off the side of the road, and what each of them pays.
//!
//! A dungeon does not advance the ladder. It stands *beside* a rung, and
//! coming out puts you back in front of the fight you had not got to - which
//! is the whole reason a run can afford to take one.
//!
//! The four the mission adds are packed for their **entry** bands rather than
//! for the rung whose event opened them. A dungeon met by a formed build is a
//! dungeon that can be hard; packing one for the rung that unlocked it would
//! make the whole set trivial, and `design/monster-themes.md` §4 already
//! exempts anything standing beside the road from the curve.

mod common;

use gearmaster_engine::bestiary::{frame, is_unpacked, MonsterTheme};
use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::dungeon::{by_id, Dungeon, DUNGEONS};
use gearmaster_engine::event::Outcome;
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::{Mode, Run};

fn a_run() -> Run {
    let mut run = Run::seeded(0xD0A9);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

/// Walk one from the top, and hand back what came out of it.
fn walk(run: &mut Run, id: &'static str) {
    run.enter_dungeon(id);
    // Fights on the road out, not rooms in the building. The two are the same
    // number for every dungeon shipped before the floor graph, and the graph
    // lints are what keep them so.
    let fights = run.dungeon.expect("in it").0.fights_ahead(0, &[]);
    for _ in 0..fights {
        run.pending_scene = None;
        run.force_win();
        run.settle();
        run.back_to_loadout();
    }
    assert!(run.dungeon.is_none(), "{} did not end", id);
}

#[test]
fn the_mission_adds_four_and_they_all_stand_beside_the_road() {
    assert_eq!(DUNGEONS.len(), 7, "one shipped, five the Unwinding added, and the yard");
    for d in DUNGEONS {
        assert!(!d.floors.is_empty());
        // "One landing a floor" was two lists counted against each other. It
        // is a field on `Floor` now, so the type says it and the assertion is
        // retired rather than loosened.
        assert!(!d.entry.is_empty(), "{} lets you in without a word", d.id);
        // Nothing on the ladder: a dungeon is reached by an event, a town door
        // or a pedestal, and never by climbing.
        for f in d.floors {
            assert!(
                !gearmaster_engine::combat::LADDER.iter().any(|m| m.name == f.creature),
                "{} is on the road as well as beside it",
                f.creature
            );
        }
    }
}

#[test]
fn every_floor_is_a_frame_with_a_band_and_a_theme() {
    for d in DUNGEONS.iter().filter(|d| d.id != "the-crevice") {
        for f in d.floors {
            let c = f.creature;
            let fr = frame(c).unwrap_or_else(|| panic!("{c} has no frame"));
            assert!(fr.band >= 20, "{c} packs to rung {}", fr.band);
            assert!(!fr.note.is_empty());
        }
    }
}

/// A floor with no board yet is the Phase-2 state, not a rule.
///
/// This asserted every floor was still naked, which was true for exactly as
/// long as nobody had packed one. Packing is what Phase 4 is *for*, and the
/// count of what is left belongs in one place - `bestiary`'s own ratchet -
/// rather than in every test that happens to name a frame. So this checks the
/// two agree with each other instead.
#[test]
fn the_frame_lint_and_the_floors_agree_about_who_is_dressed() {
    let naked: Vec<&str> =
        gearmaster_engine::bestiary::unpacked().iter().map(|f| f.name).collect();
    for d in DUNGEONS.iter().filter(|d| d.id != "the-crevice") {
        for f in d.floors {
            let c = f.creature;
            assert_eq!(
                is_unpacked(c),
                naked.contains(&c),
                "{c} disagrees with the frame lint about whether it has a board"
            );
        }
    }
}

#[test]
fn a_dungeon_reads_as_one_creature_all_the_way_down() {
    // Two floors of the same idea, getting harder. A dungeon whose floors
    // disagree is two dungeons somebody stapled together.
    for d in DUNGEONS.iter().filter(|d| d.id != "the-crevice") {
        let themes: Vec<MonsterTheme> =
            d.floors.iter().filter_map(|f| frame(f.creature)).map(|f| f.theme).collect();
        assert!(!themes.is_empty());
        // Along every *road out*, not along the list. The list is a graph now
        // and its order is an index rather than a walk: THE SWITCHYARD's floor
        // 5 is band 28 after floor 4's 30 because floors 5 to 8 are the other
        // line, and a run walks one or the other. What has to hold is that
        // nothing gets easier as you go deeper down a road somebody takes.
        for (i, f) in d.floors.iter().enumerate() {
            let Some(here) = frame(f.creature).map(|x| x.band) else { continue };
            for e in f.exits {
                let Some(next) = frame(d.floors[e.to].creature).map(|x| x.band) else { continue };
                assert!(
                    next >= here,
                    "{}: floor {i} is band {here} and leads to band {next}",
                    d.id
                );
            }
        }
        // Two exceptions, both on purpose. WUMPUS WORLD changes because the
        // dark floor is what *lives near* a wumpus and the wumpus is not that.
        //
        // THE SWITCHYARD is not a creature at all - it is a place, and the
        // nine things in it are a shunter, a gang of platelayers, what came up
        // with the ballast, a coal stage, a water tower, a gantry, a lamp
        // room, a goods shed and an engine in steam. The two lines are meant
        // to read differently in the first three seconds: the Down line is
        // weight and the Up line is light, which is how a run that has walked
        // one of them knows the other is worth an orb.
        if !matches!(d.id, "wumpus-world" | "the-switchyard") {
            assert!(
                themes.windows(2).all(|w| w[0] == w[1]),
                "{}: {:?} is two dungeons stapled together",
                d.id,
                themes
            );
        }
    }
}

#[test]
fn every_dungeon_pays_something_and_two_of_them_pay_no_class_at_all() {
    let no_class: Vec<&str> =
        DUNGEONS.iter().filter(|d| d.reward.is_empty()).map(|d| d.id).collect();
    // THE SWITCHYARD joins them, and for a third reason: it pays neither a
    // class nor a dungeon-wide `also`, because every reward is a *buffer
    // stop's*. Which one you reached is the whole of what a graph asks, so a
    // payout on the way out would be a payout for having been there at all.
    assert_eq!(no_class, vec!["the-undertow", "den-rivals", "the-switchyard"]);
    for d in DUNGEONS {
        // On any way out, or at every buffer stop. The second is what a graph
        // wants: THE SWITCHYARD pays four different things depending on which
        // of its four ends you reached, and a dungeon-wide payout would be a
        // payout for having been there at all.
        let on_any_exit = !d.reward.is_empty() || !d.also.is_empty();
        let every_stop_pays =
            d.floors.iter().filter(|f| f.is_leaf()).all(|f| !f.also.is_empty());
        assert!(on_any_exit || every_stop_pays, "{} is a walk there and a walk back", d.id);
    }
}

#[test]
fn the_antechamber_pays_the_pool_and_the_class_is_only_the_marker() {
    let mut run = a_run();
    walk(&mut run, "the-threshold");
    assert!(run.insight_unlocked);
    assert!(run.classes.iter().any(|c| c.name == "Threshold-Sighted"));
}

#[test]
fn the_undertow_pays_a_row_on_a_board_of_your_choice() {
    // H3 cuts its class in favour of the Depth, and E6.10 asks that the row
    // move no placed piece and that its receipt name the slot.
    let mut run = a_run();
    let before: Vec<(SlotKind, u8)> =
        SlotKind::ALL.iter().map(|&k| (k, run.loadout.slot(k).rows())).collect();
    walk(&mut run, "the-undertow");
    assert_eq!(run.owed_rows, 1, "the Undertow paid nothing");
    for &(k, rows) in &before {
        assert_eq!(run.loadout.slot(k).rows(), rows, "a board grew before it was chosen");
    }
    assert!(run.grow_slot(SlotKind::Helmet));
    assert_eq!(run.loadout.slot(SlotKind::Helmet).rows(), before[SlotKind::Helmet.index()].1 + 1);
    let receipt = run.take_receipt().expect("a row is a resolution");
    assert!(receipt[0].contains("helmet"), "{:?}", receipt);
}

#[test]
fn den_rivals_pays_the_hide_the_exhibit_promised() {
    let mut run = a_run();
    walk(&mut run, "den-rivals");
    assert!(run.holds("Bearhide"), "the museum lied after all");
    assert!(
        gearmaster_engine::piece::is_event_only("Bearhide"),
        "the hide could be bought off a shelf"
    );
}

#[test]
fn the_mine_and_the_hunt_pay_classes_nothing_else_hands_out() {
    for (id, class) in [("the-under-mine", "Prospector"), ("wumpus-world", "Wumpus Hunter")] {
        let mut run = a_run();
        walk(&mut run, id);
        assert!(run.classes.iter().any(|c| c.name == class), "{} paid no {}", id, class);
        // Nothing you build points at one, so no fountain may pour it.
        let def = gearmaster_engine::class::CLASSES
            .iter()
            .find(|c| c.name == class)
            .expect("authored");
        assert!(def.requires.is_empty());
        assert!(gearmaster_engine::class::is_earned(class));
    }
}

#[test]
fn a_prospector_pries_gear_off_a_named_creature() {
    // The only thing in the game that changes what a corpse is worth. A
    // trophy is one piece off a creature carrying fifteen, and every one of
    // those fifteen is barred from every shelf there is.
    let mut run = a_run();
    let named = gearmaster_engine::combat::LADDER
        .iter()
        .position(|m| m.rank.is_named() && !m.gear.is_empty())
        .expect("the road is full of them");
    run.rung = named;
    run.force_win();
    run.settle();
    let without = run.last_settlement.clone().expect("settled").pried_off.len();
    assert_eq!(without, 0, "gear came off without the class");

    let mut run = a_run();
    run.classes
        .push(gearmaster_engine::class::CLASSES.iter().find(|c| c.name == "Prospector").unwrap());
    run.rung = named;
    run.force_win();
    run.settle();
    let with = run.last_settlement.clone().expect("settled").pried_off;
    assert_eq!(with.len(), 1, "a prospector took {:?}", with);
}

#[test]
fn a_hunter_lands_the_first_one_whatever_is_in_front_of_it() {
    use gearmaster_engine::combat::{Combatant, DamageType};
    use gearmaster_engine::stats::Stats;
    // Deflection turns a flat share off every physical blow. It does not
    // touch the first one.
    let mut target = Combatant::player(Stats::new(10_000, 0, 0, 100), &[]);
    target.deflection = 5;
    assert_eq!(target.take_typed(100, DamageType::Physical, 0).1, 50);
    assert_eq!(
        target.take_typed_with(100, DamageType::Physical, 0, true).1,
        100,
        "the first one was turned aside"
    );
}

#[test]
fn coming_out_of_one_puts_you_where_you_went_in() {
    let mut run = a_run();
    run.rung = 20;
    let rung = run.rung;
    walk(&mut run, "wumpus-world");
    assert_eq!(run.rung, rung, "a dungeon moved the ladder");
}

/// Losing a floor leaves you standing in front of it.
///
/// Re-pinned, and the rule is inverted. It used to put you out of the dungeon,
/// which meant a floor you could not beat cost you the line whether you liked
/// it or not - and `leave_dungeon`, the verb that exists so a set of points is
/// a decision rather than a trap, was only ever the polite version of
/// something the game did to you anyway.
///
/// Now a loss costs the mode's own price and leaves you where you were. Fight
/// it again, or retreat. **Retreating is how you survive**, and the door still
/// does not reopen once you have.
#[test]
fn losing_a_floor_leaves_you_in_the_dungeon_and_retreating_is_the_way_out() {
    let mut run = a_run();
    run.mode = Mode::Grinder;
    run.rung = 20;
    let rung = run.rung;
    run.enter_dungeon("the-under-mine");
    assert!(run.dungeon.is_some());

    run.fight(gearmaster_engine::combat::LADDER.last().expect("a hard one"));
    run.settle();
    assert!(run.dungeon.is_some(), "a loss carried you out without asking");
    assert_eq!(run.losses, 1, "it still counts as a loss");
    assert_eq!(
        run.rung,
        rung - 1,
        "a Grinder still pays for a lost floor; what changed is where it leaves you"
    );

    // The way out is the verb.
    run.back_to_loadout();
    assert!(run.leave_dungeon());
    assert!(run.dungeon.is_none());
}

/// A Rogue on its last life is carried out rather than left to die down there.
///
/// The one exception to the rule above. A run put out of the game inside a
/// side-room, four fights from a road it could have walked away down, was
/// never offered the choice the verb exists to offer - so the last life is
/// spent on the road.
#[test]
fn a_rogue_down_to_its_last_life_is_carried_out_of_the_dungeon() {
    let hard = gearmaster_engine::combat::LADDER.last().expect("a hard one");

    // Two lives left: the loss costs one and leaves you in it.
    let mut run = a_run();
    run.mode = Mode::Rogue;
    run.rung = 20;
    run.lives = 3;
    run.enter_dungeon("the-under-mine");
    run.fight(hard);
    run.settle();
    assert_eq!(run.lives, 2);
    assert!(run.dungeon.is_some(), "carried out with two lives still in hand");

    // Down to one: out onto the road, with what was cleared still cleared.
    run.back_to_loadout();
    run.fight(hard);
    run.settle();
    assert_eq!(run.lives, 1);
    assert!(run.dungeon.is_none(), "left on its last life inside a dungeon");
    let receipt = run.take_receipt().expect("it says what happened");
    assert!(receipt[0].contains("Carried out of"), "{receipt:?}");
}

#[test]
fn every_dungeon_can_be_reached_by_something() {
    // A dungeon nobody can open is content nobody sees. Three routes exist:
    // an event's choice, a town door, and a pedestal.
    // A ratchet, not an exemption: these are the ones whose opener has not
    // been authored yet, and the list only ever gets shorter. THE FORK is
    // M14's and the two destinations are M12's.
    const NOT_YET: &[&str] = &["the-under-mine"];
    for d in DUNGEONS {
        let by_event = gearmaster_engine::event::EVENTS.iter().any(|e| {
            e.choices.iter().any(|c| {
                matches!(c.outcome, Outcome::Enter(id) | Outcome::StartDungeon(id) if id == d.id)
            })
        });
        let by_door = gearmaster_engine::town::TOWNS.iter().any(|t| {
            t.actions.iter().any(|a| a.opens() == Some(d.id))
        });
        let by_orb = gearmaster_engine::pedestal::DESTINATIONS.iter().any(|x| {
            matches!(x.kind, gearmaster_engine::pedestal::Where::Dungeon(id) if id == d.id)
        });
        let opened = by_event || by_door || by_orb;
        if NOT_YET.contains(&d.id) {
            assert!(!opened, "{} has an opener now - take it off NOT_YET", d.id);
            continue;
        }
        assert!(opened, "{} is a dungeon nobody can open", d.id);
        assert!(by_id(d.id).is_some());
    }
}

// ------------------------------------------------- the map, before and after

/// The route map draws a straight-line dungeon exactly as it drew one before
/// floors became a graph, apart from one word.
///
/// The fixture is the real pre-M1 bytes: `route::ascii` for a seeded run,
/// captured off `e38d968` before `NodeKind::Dungeon` changed shape. What M1
/// promised is that a dungeon with no points in it reads the same, and what M1
/// actually changed is the noun - `floors` was the room count and `fights` is
/// the length of the road out, which for a straight line is the same number.
/// The substitution is applied here rather than baked into the fixture so that
/// the one word that moved is named in the assertion instead of hidden in a
/// file.
///
/// The spec's acceptance criterion 3 says "`route::ascii` for a run inside THE
/// THRESHOLD". THE THRESHOLD is never on the map: a `NodeKind::Dungeon` hangs
/// off an event choice whose outcome enters one (`route.rs`), and THE
/// THRESHOLD is reached through a town door. The one shipped dungeon the map
/// draws is THE CREVICE IN THE ROCK, and this pins the whole map rather than
/// its line, so it would catch either.
#[test]
fn the_ascii_map_did_not_change_for_a_linear_dungeon() {
    let mut run = Run::seeded(0x8001);
    run.difficulty = Difficulty::Easy;
    run.rung = 20;
    run.enter_dungeon("the-threshold");

    let before = include_str!("fixtures/route-ascii-m0.txt");
    let want: Vec<String> =
        before.lines().map(|l| l.replace(" floors)", " fights)")).collect();
    let got = gearmaster_engine::route::ascii(&run);

    assert!(
        before.contains(" floors)"),
        "the fixture is supposed to be the pre-M1 bytes, and those say floors"
    );
    // Every line of the pre-M1 road is still on the map, in order.
    //
    // It compared lengths until M6, which was right while the road was the
    // road the fixture was taken from. The Switchyard adds four doors and a
    // dungeon, so the map is longer - and what M1 promised is that the lines
    // that were there did not *move*, which is a subsequence check and not a
    // length check. A line that changed a character still fails, and so does
    // one that got reordered.
    let mut at = 0usize;
    for line in &want {
        match got[at..].iter().position(|g| g == line) {
            Some(k) => at += k + 1,
            None => panic!("the map lost or changed {line:?}"),
        }
    }
    assert!(
        got.len() >= want.len(),
        "the map is shorter than the road it was taken from"
    );
}

/// A dungeon with points says so on the map, and one without does not.
///
/// The `points` clause is dropped at zero on purpose: six shipped dungeons
/// have no forks and their line must not grow a `, 0 points` nobody needs.
#[test]
fn the_map_counts_points_only_where_there_are_some() {
    let map = gearmaster_engine::route::route(&Run::seeded(0x8001));
    let drawn: Vec<_> = map
        .nodes
        .iter()
        .filter_map(|n| match n.kind {
            gearmaster_engine::route::NodeKind::Dungeon { fights, forks } => {
                Some((n.id, fights, forks))
            }
            _ => None,
        })
        .collect();
    assert!(!drawn.is_empty(), "no dungeon is on the map at all");
    for (id, fights, forks) in drawn {
        let d = by_id(id).expect("a node names a dungeon");
        assert_eq!(fights, d.fights_ahead(0, &[]));
        assert_eq!(forks, d.forks());
        // One dungeon in the game asks which way, and the map says so by
        // naming it rather than by tolerating any number of points anywhere.
        let want = if id == "the-switchyard" { 3 } else { 0 };
        assert_eq!(forks, want, "{id} has {forks} sets of points and should have {want}");
    }
}

// ------------------------------------------------------------- the transcript

/// Walk every shipped dungeon from the top and write down everything a run is
/// told on the way through.
///
/// Banner, creature, fights ahead, landing, receipt and the map's line for it -
/// which between them are every string and every number M1 moved. Reading a
/// transcript rather than asserting a list of facts is deliberate: the failure
/// this guards against is a *word* changing, and a diff says which word.
fn transcript() -> String {
    // The six that predate the floor graph, and only those. THE SWITCHYARD has
    // points in it, and a walk that does not throw them stands at the lever for
    // ever - which is what this loop did for six minutes before anybody noticed
    // it was not a slow test. The yard's own walk is `tests/switchyard.rs`'s,
    // where there is something to decide.
    let mut out = String::new();
    let six: Vec<&Dungeon> = DUNGEONS.iter().filter(|d| d.id != "the-switchyard").collect();
    assert_eq!(six.len(), 6, "a dungeon arrived that this fixture does not know about");
    for d in six {
        let mut run = a_run();
        run.rung = 20;
        run.enter_dungeon(d.id);
        out.push_str(&format!("\n=== {} [{}] ===\n", d.name, d.id));
        for line in d.entry {
            out.push_str(&format!("  entry: {line}\n"));
        }
        // Bounded. A dungeon that cannot be walked out of is a hang, and a
        // hang is a worse bug than a wrong room.
        let mut guard = 0;
        while let Some((_, floor)) = run.dungeon {
            guard += 1;
            assert!(guard < 32, "{} never ended", d.id);
            let banner = run
                .road_stack()
                .first()
                .map(|i| i.describe())
                .unwrap_or_else(|| "<nothing on the stack>".into());
            out.push_str(&format!(
                "  banner: {banner}\n  at floor {floor}, fighting {}, {} fights ahead\n",
                run.monster().name,
                d.fights_ahead(floor, &[])
            ));
            run.pending_scene = None;
            run.force_win();
            run.settle();
            if let Some(l) = run.pending_landing {
                out.push_str(&format!("  landing: {l}\n"));
            }
            if let Some(r) = run.take_receipt() {
                for line in r {
                    out.push_str(&format!("  receipt: {line}\n"));
                }
            }
            run.back_to_loadout();
        }
        out.push_str(&format!(
            "  out the other side at rung {}, in front of {}\n",
            run.rung,
            run.monster().name
        ));
        out.push_str(&format!(
            "  map: {} fights, {} points\n",
            d.fights_ahead(0, &[]),
            d.forks()
        ));
    }
    out
}

/// Every shipped dungeon says and does exactly what it said and did at M0.
///
/// The fixture is `analysis/replays/dungeons.txt`. M1 rewrote the floor tables,
/// moved the landings into them, changed how a landing is looked up in the
/// theme, and rewrote the map's node - and none of it is allowed to change one
/// character of what a run walking a straight line is told. That is the whole
/// of what "landed inert" means, and it is a diff rather than an argument.
///
/// Re-baseline with `REBASELINE_DUNGEON_REPLAY=1`, and say in the commit which
/// dungeon started saying something else.
///
/// **Re-baselined once, at M2**, and the diff was fourteen banner lines and
/// nothing else: every one gained the creature's name between the dungeon's
/// and the floor count, which is what acceptance criterion 3 asks for in those
/// words. Every `floor {n} of {m}` pair came back the same, which is the half
/// of the banner that had to hold when its two numbers changed meaning.
#[test]
fn the_six_shipped_dungeons_replay_word_for_word() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/replays/dungeons.txt");
    let got = transcript();
    if std::env::var("REBASELINE_DUNGEON_REPLAY").as_deref() == Ok("1") {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/replays"))
            .unwrap();
        std::fs::write(path, &got).unwrap();
        return;
    }
    let want = include_str!("../../../analysis/replays/dungeons.txt");
    if want != got {
        let first = want
            .lines()
            .zip(got.lines())
            .find(|(a, b)| a != b)
            .map(|(a, b)| format!("was: {a}\nnow: {b}"))
            .unwrap_or_else(|| "the transcript changed length".into());
        panic!("a shipped dungeon replays differently:\n{first}\n(fixture: {path})");
    }
}
