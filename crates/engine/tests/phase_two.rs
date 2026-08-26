//! The Phase-2 exit, said as assertions.
//!
//! Phase 2 is the mission's content phase and its rule is that content is
//! *data*: every event, town, dungeon, reward and word in the spec exists and
//! is reachable, and not one creature has been dressed. E6.8 is four claims and
//! this file is four claims.
//!
//! The fourth is the one worth saying out loud. Every creature the mission adds
//! is still a `MonsterFrame` - a name, a band, a theme and a note - and the
//! frame lint is still red on purpose. It goes green in M17, by hand, in
//! `make pack`, after M16 has re-pinned the rating that decides what
//! `stepped_component` hands every monster on three of the four settings.

mod common;

use gearmaster_engine::bestiary;
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::{every_outcome, Outcome, Trigger, EVENTS};
use gearmaster_engine::rumour::RUMOURS;
use gearmaster_engine::town::TOWNS;

/// Everything the mission promises to hand over, and what it says hands it.
///
/// A reward with no home is dead content that reads as finished, which is the
/// exact failure this file exists to catch.
const REWARDS: [&str; 12] = [
    "An Unwound Mainspring",
    "The Cracked Lens",
    "The Tally",
    "The Odometer",
    "The Ledger",
    "the Second Key",
    "the Appeal",
    "the Skip Stone",
    "Bearhide",
    "The Lightning Rod",
    "The Stranger's Parcel",
    "Pilgrim's Orb",
];

// ------------------------------------------------------ 1. every door stands

#[test]
fn every_door_in_the_game_can_be_arrived_at() {
    for e in EVENTS {
        match e.trigger {
            // A rung is a rung.
            Trigger::Rung | Trigger::QuickKill { .. } | Trigger::SlowKill { .. } => {}
            // A word has to be a word somebody can come by, which
            // `every_rumour_can_be_come_by` checks from the other end.
            Trigger::Whispered { rumour, .. } => {
                assert!(
                    RUMOURS.iter().any(|r| r.name == rumour),
                    "{} waits on {}, which is not a word",
                    e.id,
                    rumour
                );
            }
            // A flag has to be set by something. "never" is the one exception
            // and it is deliberate: a destination is pushed onto the stack by a
            // pedestal and waits on no rung at all.
            Trigger::WhenFlagged { flag, .. } => {
                if flag == "never" {
                    assert!(
                        gearmaster_engine::pedestal::DESTINATIONS
                            .iter()
                            .any(|d| matches!(d.kind, gearmaster_engine::pedestal::Where::Event(id) if id == e.id)),
                        "{} waits on a flag nothing sets and no pedestal pushes it",
                        e.id
                    );
                    continue;
                }
                let by_a_door = !gearmaster_engine::event::set_by(flag).is_empty();
                let by_a_floor = DUNGEONS.iter().any(|d| {
                    d.also
                        .iter()
                        .flat_map(every_outcome)
                        .any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
                });
                assert!(by_a_door || by_a_floor, "{} waits on {}, which nothing sets", e.id, flag);
            }
        }
    }
}

/// A reveal that arrives after its town's gate is a town nobody can enter.
///
/// `every_town_and_dungeon_has_a_way_in` asks whether *something* reveals a
/// hidden town, and that is half the question. THE BIGGER SIGN stood on rung
/// forty-one and revealed a town standing in the gap after rung fourteen: the
/// reveal happened, the town went on the map, and the road had walked past the
/// gap twenty-seven rungs earlier. Every test was green and the town was
/// unreachable in every run the game could produce.
///
/// So: the **earliest** rung a reveal can happen on must be at or before the
/// gate it opens. A door that roams a window is allowed to arrive too late -
/// hearing a rumour late and missing the turning is a bet the player made -
/// but it has to be able to arrive in time at all.
#[test]
fn a_reveal_can_happen_before_the_town_it_reveals() {
    for e in EVENTS {
        for c in e.choices {
            for o in every_outcome(&c.outcome) {
                let Outcome::RevealTown(id) = o else { continue };
                let town = TOWNS.iter().find(|t| t.id == *id).expect("a real town");
                let earliest = e.trigger.from();
                assert!(
                    earliest <= town.after,
                    "{} can first stand on rung {} and reveals {}, whose gate is after rung {} - \
                     nobody can ever reach it",
                    e.id,
                    earliest + 1,
                    id,
                    town.after + 1
                );
            }
        }
    }
}

#[test]
fn every_town_and_dungeon_has_a_way_in() {
    for t in TOWNS {
        match t.unlock {
            gearmaster_engine::town::Unlock::Pinned => {}
            gearmaster_engine::town::Unlock::Hidden => {
                let revealed = EVENTS.iter().any(|e| {
                    e.choices.iter().any(|c| {
                        every_outcome(&c.outcome)
                            .iter()
                            .any(|o| matches!(o, Outcome::RevealTown(id) if *id == t.id))
                    })
                });
                assert!(revealed, "{} is hidden and nothing reveals it", t.id);
            }
        }
    }
    for d in DUNGEONS {
        let by_a_door = EVENTS.iter().any(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(
                    |o| matches!(o, Outcome::Enter(id) | Outcome::StartDungeon(id) if *id == d.id),
                )
            })
        });
        let by_a_town = TOWNS.iter().any(|t| t.actions.iter().any(|a| a.opens() == Some(d.id)));
        let by_a_pedestal = gearmaster_engine::pedestal::DESTINATIONS
            .iter()
            .any(|p| matches!(p.kind, gearmaster_engine::pedestal::Where::Dungeon(id) if id == d.id));
        assert!(by_a_door || by_a_town || by_a_pedestal, "{} has no mouth", d.id);
    }
}

// -------------------------------------------------- 2. every reward has a home

#[test]
fn every_reward_the_mission_promises_is_handed_over_by_something() {
    for want in REWARDS {
        let by_a_door = EVENTS.iter().any(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(|o| match o {
                    Outcome::Give(n) => *n == want,
                    Outcome::SealedBid { lots } => lots.contains(&want),
                    Outcome::Step(b) => b.win == want,
                    _ => false,
                })
            })
        });
        let by_a_town = TOWNS.iter().any(|t| t.actions.iter().any(|a| a.gives() == Some(want)));
        let by_a_floor = DUNGEONS.iter().any(|d| {
            d.also.iter().flat_map(every_outcome).any(|o| matches!(o, Outcome::Give(n) if *n == want))
        });
        // A curated shelf is a fourth way, and the only way two of the
        // run-relics are come by: Aisle 9 is where a Multicity store keeps the
        // things nobody else stocks.
        let on_a_curated_shelf = gearmaster_engine::run::AISLE_NINE.contains(&want)
            || EVENTS.iter().any(|e| {
                e.choices.iter().any(|c| {
                    every_outcome(&c.outcome).iter().any(
                        |o| matches!(o, Outcome::OpenShop { shelves } | Outcome::ShopAfter { shelves } if shelves.contains(&want)),
                    )
                })
            });
        let on_a_shelf = !gearmaster_engine::piece::is_event_only(want)
            && !gearmaster_engine::piece::is_boss_only(want);
        assert!(
            by_a_door || by_a_town || by_a_floor || on_a_curated_shelf || on_a_shelf,
            "{} is in the catalogue and in nobody's gift",
            want
        );
    }
}

#[test]
fn both_routes_to_the_mainspring_are_walkable() {
    // The chain's own ending is a fight, and the mission's alternative is a
    // courier. Two roads to one component, which is what stops a run that
    // refused the Herald from being a run that cannot finish.
    const IT: &str = "An Unwound Mainspring";
    let by_the_herald = EVENTS.iter().any(|e| {
        e.choices.iter().any(|c| {
            every_outcome(&c.outcome).iter().any(|o| matches!(o, Outcome::Step(b) if b.win == IT))
        })
    });
    let by_the_courier = EVENTS.iter().any(|e| {
        e.choices.iter().any(|c| {
            every_outcome(&c.outcome)
                .iter()
                .any(|o| matches!(o, Outcome::Passenger { pays, .. } if *pays == IT))
        })
    });
    assert!(by_the_herald, "the Herald pays something else now");
    assert!(by_the_courier, "the courier pays something else now");
}

// ------------------------------------------------ 3. nothing has been dressed

/// Every creature the mission added is a frame, and every frame is dressed.
///
/// This asserted the opposite until Phase 4, and that was the point: the frame
/// lint is red before it and green after, which is E6.8's own wording. What
/// survives the inversion is the half that was always the real claim - a frame
/// carries what its packer needs, whether or not it has been packed yet.
#[test]
fn every_creature_the_mission_added_is_a_frame_with_a_brief() {
    for f in bestiary::FRAMES {
        assert!(f.band >= 1, "{} has no band, so nothing can pack it", f.name);
        assert!(!f.note.is_empty(), "{} tells its packer nothing", f.name);
    }
    assert!(
        bestiary::unpacked().is_empty(),
        "Phase 4 is finished and {:?} still has no board",
        bestiary::unpacked().iter().map(|f| f.name).collect::<Vec<_>>()
    );
}

// ------------------------------------------------------ 4. a run reaches them

#[test]
fn a_run_that_answers_everything_meets_the_whole_mission() {
    // Not a replay - a sweep. Every rung, every flag set, every word in the
    // tray, answering whatever stands there with whatever it will take. What
    // it measures is that the doors *stand*, which is the half of reachability
    // that a table cannot tell you.
    // A packed board rather than the starter preset. Two of these doors are
    // bets on what you have built - a helmet with no room left in it, and a
    // hundred nature banked - and a sweep against a thin board would report
    // them unreachable when what is thin is the fixture.
    let mut run = common::run_from(gearmaster_engine::share::A_WINNING_RUN);
    run.mode = gearmaster_engine::run::Mode::Grinder;
    run.difficulty = gearmaster_engine::combat::Difficulty::Easy;
    run.gold = 1_000_000;
    run.unlock_insight();
    for r in RUMOURS {
        run.give(r.name);
    }
    for f in ["threshold-cleared", "slagworks-known"] {
        run.flags.push(f);
    }
    // The two doors the bar sells words for are bets on the *board*, not on
    // the road, so a sweep has to have made the bet as well as bought the
    // word: a crowded helmet and a hundred nature banked across the run.
    run.banked_all_run[gearmaster_engine::piece::Resource::Nature.index()] = 1_000;

    let mut met: Vec<&str> = Vec::new();
    for rung in 0..gearmaster_engine::combat::LADDER.len() {
        run.rung = rung;
        // Several doors can stand on one rung. Answer until the rung is quiet.
        for _ in 0..4 {
            let Some(e) = run.pending_event() else { break };
            met.push(e.id);
            let Some(c) = e.choices.iter().find(|c| run.choice_open(c)) else { break };
            if matches!(c.requires, gearmaster_engine::event::Requirement::Figure { .. }) {
                run.take_choice_with(c, 0);
            } else {
                run.take_choice(c);
            }
            run.take_receipt();
            run.brawl = None;
            run.substitute = None;
            run.dungeon = None;
            run.forced_event = None;
        }
        run.back_to_loadout();
    }

    let missed: Vec<&str> = EVENTS
        .iter()
        .filter(|e| !matches!(e.trigger, Trigger::QuickKill { .. } | Trigger::SlowKill { .. }))
        // The destinations do not stand on rungs at all: a pedestal pushes
        // them, and `pedestal.rs` is where that is checked.
        .filter(|e| !matches!(e.trigger, Trigger::WhenFlagged { flag: "never", .. }))
        .map(|e| e.id)
        .filter(|id| !met.contains(id))
        .collect();
    assert!(missed.is_empty(), "a run that answered everything never met {:?}", missed);
}
