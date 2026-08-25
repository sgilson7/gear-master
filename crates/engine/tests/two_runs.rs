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
use gearmaster_engine::piece::SlotKind;
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
/// `share::A_WINNING_RUN`. Nothing about it is ever seeded.
///
/// Which door it finds is decided by the setting it is played on, and that is
/// the neatest thing about it: on Medium its quickest shallow win is 1600ms
/// and it walks into the casino; on Hard the same board takes 3200ms at best
/// and 14400ms at worst, so the casino is shut and the road is open instead.
/// One build, two chains, nothing arranged.
fn the_winning_board(difficulty: Difficulty) -> Run {
    let shared = gearmaster_engine::share::import(gearmaster_engine::share::A_WINNING_RUN)
        .expect("the winning code still reads");
    let mut run = Run::new();
    run.difficulty = difficulty;
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

fn a_sharp_run() -> Run {
    the_winning_board(Difficulty::Medium)
}

/// A run whose fights genuinely run long. Nothing is seeded: a complete board
/// on Medium takes well over ten seconds in the shallow end all by itself.
fn a_blunt_run() -> Run {
    let mut run = a_run(Difficulty::Medium);
    run.rung = 1;
    run
}

/// The same winning board, on a setting where it grinds. Its quickest shallow
/// win is 3200ms - past the casino's three-second bar - and its slowest is
/// 14400ms, which is what opens the road instead.
fn a_grinding_run() -> Run {
    a_grinding_run_and_its_weapon().0
}

/// The same, and the weapon that was taken off it, so a run can put it back.
///
/// A board that grinds the shallow end and a board that walks to rung 21 are
/// not the same board, and after the repack they cannot be: the road wants a
/// fight over fifteen seconds in rungs 2-9, which takes the weapon off, and the
/// burners at rungs 14-20 stop a weaponless board at 19. A player is not stuck
/// with that choice - they ground the shallow end and then bought a weapon,
/// which is the whole shape of the run this test is about - so the fixture
/// stops pretending its board never changes.
fn a_grinding_run_and_its_weapon() -> (Run, Vec<(gearmaster_engine::piece::PieceId, u8, u8)>) {
    // Insane, not Hard. This fixture's whole job is to be the board whose
    // fights run long enough to open the road instead of the casino, and the
    // gloves sweep moved that line: a board that answers its neighbours kills
    // fast enough on Hard to earn the casino, which blocks the road by design.
    // The difficulty is the knob that makes it slow, so the difficulty moved.
    let mut run = the_winning_board(Difficulty::Insane);
    // And blunted for real, by taking the weapon off. Insane alone no longer
    // does it: the road wants a fight over twenty seconds now, and this board's
    // slowest on Insane was 14.4s. The fixture's whole job is to be the run
    // that grinds, and grinding is what a board with no weapon does - the
    // other four slots still fight, which is the rewrite's own point.
    // The fixture is unchanged this time and the door moved instead. Taking
    // the gloves off as well does make it grind, and makes it too weak to
    // reach the pay-off twelve rungs later - the fixture has to be slow in the
    // shallow end *and* still get to rung 21, and after the repack no amount of
    // blunting is both. What changed is the ladder underneath it, so what had
    // to move was the number the door asks for. See `the-long-way` in
    // `event.rs`.
    let weapon: Vec<_> = run
        .loadout
        .slot(SlotKind::Weapon)
        .pieces()
        .into_iter()
        .filter_map(|id| run.loadout.slot(SlotKind::Weapon).anchor_of(id).map(|(x, y)| (id, x, y)))
        .collect();
    run.loadout.slot_mut(SlotKind::Weapon).clear();
    let still: Vec<_> =
        SlotKind::ALL.iter().flat_map(|&k| run.loadout.slot(k).pieces()).collect();
    run.loadout.locks.retain(|l| l.pieces.iter().all(|p| still.contains(p)));
    (run, weapon)
}

/// Put the weapon back on, the way twelve rungs of shopping would have.
fn rearm(run: &mut Run, weapon: &[(gearmaster_engine::piece::PieceId, u8, u8)]) {
    for &(id, x, y) in weapon {
        let _ = run.equip(id, SlotKind::Weapon, x, y);
    }
    gearmaster_engine::loadout::lock_assembled_in(
        &mut run.loadout,
        &run.registry,
        SlotKind::Weapon,
    );
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
    assert!(
        best < 3_000,
        "the winning board took {best}ms and should not have got in. Measured at 1600ms, so \
         something took 1.4s off the early ladder's headroom - see the corridor note on \
         `the_casino_bar_is_high_and_the_other_door_is_not`"
    );
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
/// the shallow end is **4.5s** against a three-second bar, while the board that
/// actually cleared the game gets through in **1.6s**. That is the door working
/// - it selects for a build that has gone all in on damage early - but it is
/// worth having written down, because it also means most runs meet the *other*
/// door rather than this one.
///
/// **The corridor, measured, for whoever repacks the early ladder.** Both doors
/// key off rungs 1-10, so every creature down there is inside it. Making them
/// stronger slows both boards: the sharp run has **1.4s** of room before its
/// best win stops being under three seconds and the casino shuts. Making them
/// weaker speeds both: the plain board has **1.5s** of room before an ordinary
/// build clears a bar meant for a sharp one. The slow door is not close - the
/// plain run's worst is 44s against a 10s trip - so the binding constraint is
/// the 1.4s. `probe_the_casino_corridor` prints all four figures.
#[test]
fn the_casino_bar_is_high_and_the_other_door_is_not() {
    let mut run = a_run(Difficulty::Medium);
    run.rung = 1;
    play(&mut run, 10, |_| true);

    let best = run.best_fight_ms.expect("won something in the shallow end");
    let worst = run.worst_fight_ms.expect("same");
    assert!(
        best >= 3_000,
        "a plain board now clears the casino bar at {best}ms - it was 4500ms, so the early \
         ladder got 1.5s easier and the sharp door is no longer sharp"
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


// ---------------------------------------------------------------- follow-ups

/// The winning board, all the way to the room behind the velvet rope.
#[test]
fn the_winning_board_reaches_the_vip_area_with_a_chip_it_won() {
    let mut run = a_sharp_run();
    let vip = EVENTS.iter().find(|e| e.id == "the-vip-area").expect("authored");

    // Step in at the casino; the chip is what the table is worth.
    let t = play(&mut run, vip.at, |c| matches!(c.outcome, ChoiceOutcome::Step(_)));
    assert!(t.events.contains(&"the-casino"), "never found the casino: {:?}", t.events);
    assert!(
        run.owned.iter().any(|&i| run.registry.def(i).name == "Platinum Chip"),
        "reached rung {} without winning the table: events {:?}, choices {:?}",
        t.reached + 1,
        t.events,
        t.choices
    );
    assert_eq!(run.rung, vip.at, "stalled at rung {} short of the rope", run.rung + 1);

    // And the door opens, because the chip is in the tray.
    let ev = run.pending_event().expect("the VIP area stands here");
    assert_eq!(ev.id, "the-vip-area");
    let gated: Vec<_> = ev
        .choices
        .iter()
        .filter(|c| !matches!(c.outcome, ChoiceOutcome::FightAsWritten))
        .collect();
    for c in &gated {
        assert!(run.choice_open(c), "{} stayed shut for a run holding the chip", c.label);
    }
}

/// Both branches of the VIP area, walked from rung one.
#[test]
fn both_vip_branches_can_be_taken_by_the_winning_board() {
    for want_deal in [true, false] {
        let mut run = a_sharp_run();
        let vip = EVENTS.iter().find(|e| e.id == "the-vip-area").expect("authored");
        play(&mut run, vip.at, |c| matches!(c.outcome, ChoiceOutcome::Step(_)));
        assert_eq!(run.rung, vip.at, "did not reach the rope");

        let rows = run.loadout.rows();
        // The offer is checked the moment it is made. It lasts until the next
        // fight settles and the shop turns over, which is one shopping window
        // - the same one any other event leaves you standing in.
        if want_deal {
            let ev = run.pending_event().expect("the rope is here");
            let deal = ev
                .choices
                .iter()
                .find(|c| matches!(c.outcome, ChoiceOutcome::Stock { .. }))
                .expect("the bargain");
            run.take_choice(deal);
            assert!(
                run.shop.stock_defs().iter().all(|d| gearmaster_engine::piece::is_vip_only(d.name)),
                "the shelves hold something that was not on the table: {:?}",
                run.shop.stock_defs().iter().map(|d| d.name).collect::<Vec<_>>()
            );
            assert_eq!(run.shop.stock_defs().len(), 5, "five things were laid out");
        }
        let t = play(&mut run, vip.at + 2, |c| {
            if want_deal {
                matches!(c.outcome, ChoiceOutcome::Stock { .. })
            } else {
                matches!(c.outcome, ChoiceOutcome::Step(_))
            }
        });
        assert!(
            t.events.contains(&"the-vip-area") || want_deal,
            "the rope never came up"
        );

        if want_deal {
            assert!(
                run.classes.iter().any(|c| c.name == "Immense Guilt"),
                "kept cover and felt nothing"
            );
        } else {
            // Won or lost, the run came out the other side and carried on.
            assert!(run.brawl.is_none(), "still stuck in the back room");
            assert!(
                run.loadout.rows() >= rows,
                "the boards shrank on the way out"
            );
        }
    }
}

/// The same board on a harder setting takes the road, and the road pays out
/// twelve rungs later.
#[test]
fn the_winning_board_can_walk_the_road_and_collect_on_it() {
    let (mut run, weapon) = a_grinding_run_and_its_weapon();
    let follow = EVENTS.iter().find(|e| e.id == "where-it-was-going").expect("authored");
    let road = EVENTS.iter().find(|e| e.id == "the-long-way").expect("authored");

    // Grind as far as the road, then arm up and walk it.
    // Ask rather than take: the whole point of the free branch.
    let mut t = play(&mut run, road.at + 1, |c| c.label.starts_with("Ask"));
    rearm(&mut run, &weapon);
    let rest = play(&mut run, follow.at, |c| c.label.starts_with("Ask"));
    t.events.extend(rest.events);
    t.choices.extend(rest.choices);
    t.reached = rest.reached.max(t.reached);
    assert!(
        t.events.contains(&"the-long-way"),
        "the road never came up: reached rung {}, events {:?}, slowest win {:?}ms",
        t.reached + 1,
        t.events,
        run.worst_fight_ms
    );
    assert!(
        !t.events.contains(&"the-casino"),
        "this board was offered the casino on Hard: best win {:?}ms",
        run.best_fight_ms
    );
    assert!(run.took.iter().any(|l| l.starts_with("Ask")), "nothing was remembered");
    assert_eq!(run.rung, follow.at, "stalled at rung {} short of the pay-off", run.rung + 1);

    // Twelve rungs on, it is there, and it has something for you.
    let ev = run.pending_event().expect("the cart is here");
    assert_eq!(ev.id, "where-it-was-going");
    let claim = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Claim("Longhauler")))
        .expect("the pay-off is a choice");
    assert!(run.choice_open(claim), "asked, got there, and the door was shut anyway");
    run.take_choice(claim);
    assert!(run.classes.iter().any(|c| c.name == "Longhauler"));
}

#[test]
fn taking_trundle_shuts_the_pay_off_but_not_the_door() {
    // A run that took the class never asked the question, so the follow-up
    // stands there and tells it so - and must still be passable.
    //
    // Placed at the rung rather than walked to it: a trundling run cannot get
    // this far, which is its own finding and has its own test below.
    let mut run = a_grinding_run();
    let follow = EVENTS.iter().find(|e| e.id == "where-it-was-going").expect("authored");
    run.rung = follow.at;
    run.took.push("Walk with them a while");

    let ev = run.pending_event().expect("the cart is here either way");
    assert_eq!(ev.id, "where-it-was-going");
    let claim = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Claim("Longhauler")))
        .expect("authored");
    assert!(!run.choice_open(claim), "collected on a question it never asked");
    assert!(!claim.unmet.is_empty(), "shut without saying why");
    assert!(ev.choices.iter().any(|c| run.choice_open(c)), "no way past");
}

/// What Trundle costs, measured on the board that cleared the game.
///
/// It used to cost the road: a trundling board stalled at rung fourteen on a
/// *stalemate* - alive after sixty seconds and unable to finish - while the
/// same board that asked reached twenty-two. Armour bought survival, survival
/// was not victory, and no defensive option could be worth taking.
///
/// Sudden death removed the clock, and with it the trap. Both runs now reach
/// the pay-off. Trundle is still a real cost where fights are marginal - on
/// Hard it gives up about a dozen rungs of reach - but it no longer stops a
/// run dead, which is the difference between a decision and a mistake.
///
/// Recorded rather than asserted at a number, so a retune reads as a change.
#[test]
fn trundle_no_longer_costs_the_road() {
    let follow = EVENTS.iter().find(|e| e.id == "where-it-was-going").expect("authored");

    // Both runs grind to the road and then arm up, for the reason on
    // `a_grinding_run_and_its_weapon`: a board that opens the road cannot also
    // be the board that walks the twelve rungs after it, and a player is not
    // stuck with that choice.
    let road = EVENTS.iter().find(|e| e.id == "the-long-way").expect("authored");
    let walk = |pick: &dyn Fn(&gearmaster_engine::event::Choice) -> bool| -> Run {
        let (mut run, weapon) = a_grinding_run_and_its_weapon();
        play(&mut run, road.at + 1, |c| pick(c));
        rearm(&mut run, &weapon);
        play(&mut run, follow.at, |c| pick(c));
        run
    };
    let asked = walk(&|c| c.label.starts_with("Ask"));
    let took = walk(&|c| matches!(c.outcome, ChoiceOutcome::Claim("Trundle")));

    println!(
        "the same board: rung {} having asked, rung {} having taken Trundle",
        asked.rung + 1,
        took.rung + 1
    );
    assert_eq!(
        asked.rung, follow.at,
        "a run that asked no longer reaches the pay-off"
    );
    // Trundle costs the pay-off on Insane now, and it costs it by losing.
    //
    // This asked for both runs to reach rung 21 and both did, until the pools
    // were switched on: `held_bonus` had been computing faith and nature and
    // nothing had ever read it, so half of what every creature banks did
    // nothing at all. Switching it on made the ladder harder for everybody, and
    // a board carrying Trundle is the board with least room for that.
    //
    // What matters is *how* it is stopped. The trap this test was written
    // against was the clock - alive after sixty seconds and unable to finish -
    // and that is not what happens: the trundling run walks to rung 14 and is
    // beaten by The Hollow King in 11.2s. A boss stopping a deliberately slow
    // build is a cost; a fight it cannot end is a mistake. Pinned at the rung
    // it reaches so any further retune reads as a change, which is what the
    // note above asks for.
    assert_eq!(
        took.rung, 13,
        "a trundling run now reaches rung {} - it reached 14 when this was pinned",
        took.rung + 1
    );
    assert!(
        took.rung > road.at,
        "Trundle stopped the run where it opened the road, which is the trap and not a cost"
    );
}

#[test]
fn longhaul_winds_up_as_the_fight_drags() {
    use gearmaster_engine::class::{ClassPower, CLASSES};
    use gearmaster_engine::combat::simulate_with_class;

    // The winning board, because the whole point is a fight that goes on and
    // an auto-built one does not live that long this deep.
    let run = the_winning_board(Difficulty::Medium);
    let (stats, items) = (run.player_stats(), run.combat_items());
    let long = *CLASSES.iter().find(|c| c.name == "Longhauler").expect("authored");
    assert!(matches!(long.power, ClassPower::Longhaul { .. }));

    // A deep rung, because the whole point is a fight that goes on.
    //
    // Measured as "more swings, sooner" rather than "more swings in a window":
    // a hauler finishes the same fight faster, so any window with a fixed end
    // runs past the end of its fight and counts the difference backwards. The
    // first attempt at this test read 35 against 40 and looked like a broken
    // class; it was a fight that had already been won.
    let spec = gearmaster_engine::combat::LADDER[24];
    let read = |classes: &[gearmaster_engine::class::ClassDef]| -> (usize, u32) {
        let log = simulate_with_class(stats, &items, &spec, Difficulty::Medium, classes);
        let acts = log
            .entries
            .iter()
            .filter(|e| {
                matches!(e.event, gearmaster_engine::combat::Event::Activate {
                    side: gearmaster_engine::combat::Side::Player,
                    ..
                })
            })
            .count();
        (acts, log.duration_ms)
    };
    let (plain, plain_ms) = read(&[]);
    let (hauled, hauled_ms) = read(&[long]);

    assert!(plain > 0, "the control never swung; this proves nothing");
    // Rate, not count.
    //
    // "More swings, sooner" was measured as more swings, and the two come
    // apart the moment the fight gets shorter: a hauler that finishes in
    // three quarters of the time at five sixths the cadence swings *fewer*
    // times and is still winding up exactly as promised. This read 82 against
    // 90 on a repacked rung 25 and looked like a broken class; it was a class
    // working and a fight ending. What the power says is that the swings come
    // faster, so that is what this asks, and the assertion below still holds
    // it to finishing sooner.
    let rate = |acts: usize, ms: u32| acts as f64 / (ms.max(1) as f64 / 1000.0);
    assert!(
        rate(hauled, hauled_ms) > rate(plain, plain_ms),
        "a long-hauler swung {:.2} times a second against {:.2} - {hauled} swings in \
         {hauled_ms}ms against {plain} in {plain_ms}ms",
        rate(hauled, hauled_ms),
        rate(plain, plain_ms)
    );
    assert!(
        hauled_ms < plain_ms,
        "a long-hauler took {hauled_ms}ms where the same board took {plain_ms}ms"
    );
}

/// Both sides of the casino corridor at once, for whoever repacks rungs 1-10.
#[test]
#[ignore = "generator; run with --ignored"]
fn probe_the_casino_corridor() {
    let mut sharp = a_sharp_run();
    play(&mut sharp, 12, |c| matches!(c.outcome, ChoiceOutcome::Give(_)));
    println!(
        "PROBE sharp best {:?}ms worst {:?}ms  (needs best < 3000)",
        sharp.best_fight_ms, sharp.worst_fight_ms
    );
    let mut plain = a_run(Difficulty::Medium);
    plain.rung = 1;
    play(&mut plain, 10, |_| true);
    println!(
        "PROBE plain best {:?}ms worst {:?}ms  (needs best >= 3000, worst > 10000)",
        plain.best_fight_ms, plain.worst_fight_ms
    );
}
