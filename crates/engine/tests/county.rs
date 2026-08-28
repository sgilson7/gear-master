//! THE HUNDRED, generated: the grid, the checks, and the county nobody rolled.
//!
//! F1 lands the generator and wires it to nothing. Every assertion here is
//! about a `County` in the abstract - a pure function's output, and the
//! authored county it falls back to - because until F2 the run does not know
//! the place exists. The exit criterion is elsewhere: the ladder replays
//! byte-identically, which `the_road` and `catalog_shape` say.

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::county::{
    self, Bearing, Chain, County, Region, Tile, TileKind, Toll, ATTEMPTS, CIRCUIT, FALLBACK,
    MOUTHS, TILES, W, H,
};
use gearmaster_engine::run::Mode;

/// Seeds that are not all the same shape: a zero, a small one, two with the
/// high bits busy, and the derived seeds of four real runs.
fn a_spread_of_seeds() -> Vec<u64> {
    let mut out = vec![0u64, 1, 0xFFFF_FFFF_FFFF_FFFF, 0x5EED_1234_ABCD_0001];
    for run_seed in [0x1_00Du64, 0xB0A7, 0xD0A9, 0x8001] {
        for mode in [Mode::Grinder, Mode::Rogue] {
            for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Insane] {
                out.push(county::seed_for(run_seed, mode, d));
            }
        }
    }
    out
}

// --------------------------------------------------------------- the fixture

/// The authored county passes every check the generated ones have to.
///
/// D-3, and the reason it is D-3: a generator whose only known-good output is
/// one it produced itself has checks nobody can falsify. If this goes red, the
/// bug is as likely to be in a check as in the fallback, and that is the point.
#[test]
fn the_fallback_passes_every_check() {
    let refused = county::refusals(&FALLBACK);
    assert!(refused.is_empty(), "the authored county is refused by its own checks:\n  {}", refused.join("\n  "));
}

/// And it is the county you get when nothing else works.
#[test]
fn the_fallback_says_it_is_the_fallback() {
    assert!(FALLBACK.is_fallback(), "the authored county has to announce itself");
    assert_eq!(FALLBACK.attempts(), ATTEMPTS);
    // A generated one never claims to be it.
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        if !c.is_fallback() {
            assert!(c.attempts() < ATTEMPTS, "seed {seed:#x} claims {} attempts", c.attempts());
        }
    }
}

/// Every tile knows where it is and which third of the county it is in.
#[test]
fn the_grid_agrees_with_itself() {
    for c in [FALLBACK.clone(), county::generate(0x1_00D)] {
        assert_eq!(c.tiles().len(), TILES);
        assert_eq!(TILES, 49);
        for (i, t) in c.tiles().iter().enumerate() {
            let want = ((i % W as usize) as u8, (i / W as usize) as u8);
            assert_eq!(t.at, want, "tile {i} is drawn out of order");
            assert_eq!(*c.at(want), *t, "`at` and the array disagree about {want:?}");
            assert_eq!(t.region, Region::of_row(t.at.1));
        }
    }
    // Fourteen, twenty-one, fourteen.
    let by_region = |r: Region| FALLBACK.tiles().iter().filter(|t| t.region == r).count();
    assert_eq!((by_region(Region::North), by_region(Region::Middle), by_region(Region::South)), (14, 21, 14));
}

// ---------------------------------------------------------------- purity

/// Same seed, same county. Three times, and again after generating others in
/// between, because a generator that carried state would pass the first check
/// and fail this one.
#[test]
fn the_same_seed_makes_the_same_county() {
    for seed in a_spread_of_seeds() {
        let a = county::generate(seed);
        let b = county::generate(seed);
        for other in [seed ^ 0xABCD, seed.wrapping_add(7), 0] {
            let _ = county::generate(other);
        }
        let c = county::generate(seed);
        assert_eq!(a, b, "seed {seed:#x} made two different counties");
        assert_eq!(a, c, "seed {seed:#x} drifted after other seeds were rolled");
    }
}

/// The derived seed is A1's formula, and it never touches `Run::rng`.
///
/// The second half is the one that matters and it cannot be asserted here -
/// `Run::rng` is private and F2 is the milestone that could break it. What
/// this pins is that mode and difficulty each move the county, so a run cannot
/// silently share one with a run set up differently.
#[test]
fn the_seed_is_derived_and_not_drawn() {
    let base = 0x5EED_1234_ABCD_0001u64;
    let g = county::seed_for(base, Mode::Grinder, Difficulty::Medium);
    assert_eq!(g, base ^ ((Mode::Grinder as u64) << 40) ^ ((Difficulty::Medium as u64) << 44));

    let mut seen = std::collections::BTreeSet::new();
    for mode in [Mode::Grinder, Mode::Rogue] {
        for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Insane] {
            assert!(seen.insert(county::seed_for(base, mode, d)), "{mode:?}/{d:?} shares a seed");
        }
    }
    assert_eq!(seen.len(), 8);
}

// ------------------------------------------------------------- the checks

/// Ten thousand seeds pass or fall back, and the retry bound is never
/// exceeded.
///
/// The deliverable F1 owes: a retry-rate histogram. Over 1% means a check is
/// too tight, and the histogram is what says which - a run of counties all
/// refused at the same attempt count is a check refusing a *shape*, not a
/// seed.
#[test]
fn ten_thousand_seeds_land_somewhere() {
    let mut histogram = [0usize; ATTEMPTS as usize + 1];
    for seed in 0..10_000u64 {
        let c = county::generate(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        assert!(c.attempts() <= ATTEMPTS, "seed {seed} got past the retry bound");
        histogram[c.attempts() as usize] += 1;
        // Whatever came back - derived or authored - is a county that passes.
        if !c.is_fallback() {
            let refused = county::refusals(&c);
            assert!(refused.is_empty(), "seed {seed} returned a refused county:\n  {}", refused.join("\n  "));
        }
    }
    let first_try = histogram[0];
    let fell_back = histogram[ATTEMPTS as usize];
    let retried: usize = histogram[1..ATTEMPTS as usize].iter().sum();
    println!("first try {first_try}  retried {retried}  fell back {fell_back}");
    println!("histogram {histogram:?}");
    assert_eq!(fell_back, 0, "{fell_back} seeds in ten thousand exhausted {ATTEMPTS} attempts");
    assert!(
        retried * 100 <= 10_000,
        "{retried} of 10,000 seeds retried, which is over 1% and means a check is too tight; \
         the histogram above says which attempt count they pile up at: {histogram:?}"
    );
}

/// Every check refuses something.
///
/// A check that cannot fail is a comment. Each of the twelve is handed a
/// county broken in exactly its own way and has to say so - and, just as
/// importantly, the *other* checks must not be what catches it, so the
/// assertion looks for the check's own prefix.
#[test]
fn every_check_refuses_the_thing_it_is_for() {
    let put = |c: &County, p: (u8, u8), k: TileKind| {
        let mut kinds = [TileKind::Empty; TILES];
        for (i, t) in c.tiles().iter().enumerate() {
            kinds[i] = t.kind;
        }
        kinds[p.1 as usize * W as usize + p.0 as usize] = k;
        County::of(kinds, c.hill(), *c.bearings(), c.pale(), *c.sealed(), 0)
    };
    let rebuilt = |c: &County, hill, bearings, pale, sealed| {
        let mut kinds = [TileKind::Empty; TILES];
        for (i, t) in c.tiles().iter().enumerate() {
            kinds[i] = t.kind;
        }
        County::of(kinds, hill, bearings, pale, sealed, 0)
    };
    let says = |c: &County, which: &str| {
        let r = county::refusals(c);
        assert!(
            r.iter().any(|s| s.starts_with(&format!("{which}:"))),
            "{which} did not refuse the county broken for it; the refusals were:\n  {}",
            r.join("\n  ")
        );
    };

    // V1 and V8 - wall the hill in with tolls on every approach.
    let mut walled = FALLBACK.clone();
    for n in county::neighbours(FALLBACK.hill()) {
        walled = put(&walled, n, TileKind::Feature(Toll::Hedge { curse_resist: 9 }));
    }
    // Two tolls deep, so five moves and one toll cannot get there and neither
    // can eight moves and one toll.
    for n in county::neighbours(FALLBACK.hill()) {
        for m in county::neighbours(n) {
            if m != FALLBACK.hill() {
                walled = put(&walled, m, TileKind::Feature(Toll::Hedge { curse_resist: 9 }));
            }
        }
    }
    says(&walled, "V1");
    says(&walled, "V8");

    // V2 - strand a chain along the southern edge, which is the thin part of
    // any county: the mouths are on the edge and a tile on the edge is
    // approached from one side. Three tiles that only sump-bottom and the
    // slagworks can reach cannot be given three different gates between them.
    //
    // Not adjacent and not in one corner, so V3 is not what catches it -
    // V4 is, because the southern edge is one region, and that is unavoidable
    // in a breaker for V2: the thin part of the county *is* a region.
    let mut stranded = FALLBACK.clone();
    for at in FALLBACK.objectives(Chain::Drove) {
        stranded = put(&stranded, at, TileKind::Empty);
    }
    for (i, at) in [(0u8, 6u8), (2, 6), (4, 6)].iter().enumerate() {
        stranded = put(&stranded, *at, TileKind::Objective { chain: Chain::Drove, nth: i as u8 + 1 });
    }
    says(&stranded, "V2");
    says(&stranded, "V4");

    // V3 - two of a chain's objectives beside each other.
    let mut huddled = FALLBACK.clone();
    for at in FALLBACK.objectives(Chain::Drove) {
        huddled = put(&huddled, at, TileKind::Empty);
    }
    for (i, at) in [(0u8, 0u8), (1, 0), (0, 1)].iter().enumerate() {
        huddled = put(&huddled, *at, TileKind::Objective { chain: Chain::Drove, nth: i as u8 + 1 });
    }
    says(&huddled, "V3");

    // V5 - the pale on the edge.
    says(&rebuilt(&FALLBACK, FALLBACK.hill(), *FALLBACK.bearings(), (0, 0), *FALLBACK.sealed()), "V5");

    // V6 - two pinnacles beside each other.
    let mut crowded = FALLBACK.clone();
    let drove = FALLBACK.pinnacle(Chain::Drove).unwrap();
    crowded = put(&crowded, drove, TileKind::Empty);
    crowded = put(&crowded, county::neighbours(FALLBACK.hill())[0], TileKind::Pinnacle { chain: Chain::Drove });
    says(&crowded, "V6");

    // V9 - the gaol at a corner.
    let mut exiled = FALLBACK.clone();
    exiled = put(&exiled, FALLBACK.gaol().unwrap(), TileKind::Empty);
    exiled = put(&exiled, (0, 0), TileKind::Gaol);
    says(&exiled, "V9");

    // V10 - three tiles of composition gone.
    let mut thinned = FALLBACK.clone();
    for at in FALLBACK.objectives(Chain::Drove) {
        thinned = put(&thinned, at, TileKind::Empty);
    }
    says(&thinned, "V10");

    // V11 - a toll on the ring.
    says(&put(&FALLBACK, CIRCUIT[0], TileKind::Feature(Toll::Gate { bounties: 1 })), "V11");

    // V12 - two of the three bearings the same line.
    says(
        &rebuilt(
            &FALLBACK,
            FALLBACK.hill(),
            [Bearing::Row, Bearing::Row, Bearing::Column],
            FALLBACK.pale(),
            *FALLBACK.sealed(),
        ),
        "V12",
    );
    // And the hill on the edge, which is V12's other half.
    says(&rebuilt(&FALLBACK, (0, 0), *FALLBACK.bearings(), FALLBACK.pale(), *FALLBACK.sealed()), "V12");
}

// ----------------------------------------------------------- the composition

/// Forty-nine tiles, and each kind within one of A1.2.
///
/// V10 is the check; this is the same arithmetic asserted **exactly** on the
/// counties the game will actually hand out, because a tolerance is for a
/// generator that has to place things under twelve constraints and not for a
/// promise about what a county is.
#[test]
fn the_composition_is_what_a1_2_says() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        let count = |f: fn(&TileKind) -> bool| c.count(f);
        let got = (
            count(|k| matches!(k, TileKind::Objective { .. })),
            count(|k| matches!(k, TileKind::Pinnacle { .. })),
            count(|k| matches!(k, TileKind::Gaol)),
            count(|k| matches!(k, TileKind::Event(_))),
            count(|k| matches!(k, TileKind::Feature(_))),
            count(|k| matches!(k, TileKind::Empty)),
        );
        assert_eq!(
            got,
            (9, 3, 1, 12, 12, 12),
            "seed {seed:#x}: objectives, pinnacles, gaol, events, features, empties"
        );
        assert_eq!(got.0 + got.1 + got.2 + got.3 + got.4 + got.5, TILES);
    }
}

/// Two of each of the six tolls, and none of them a surprise.
#[test]
fn twelve_tolls_are_two_of_each_of_six() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        let mut by_letter = std::collections::BTreeMap::new();
        for t in c.tiles() {
            if let TileKind::Feature(toll) = t.kind {
                *by_letter.entry(toll.letter()).or_insert(0usize) += 1;
            }
        }
        assert_eq!(
            by_letter,
            [('R', 2), ('F', 2), ('S', 2), ('D', 2), ('H', 2), ('G', 2)].into_iter().collect(),
            "seed {seed:#x} deals the tolls unevenly"
        );
    }
}

/// The pale is one of the twelve event tiles, and it is the only one of its id.
///
/// B3.1 asks the pale for a checklist and one gated choice, which is an event
/// and not a new kind of tile. Counting it among the twelve is what keeps
/// A1.2's arithmetic exact; the other eleven are arranged from the pool.
#[test]
fn the_pale_is_an_event_tile_and_there_is_one_of_it() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        let pales: Vec<&Tile> =
            c.tiles().iter().filter(|t| t.kind == TileKind::Event(county::PALE)).collect();
        assert_eq!(pales.len(), 1, "seed {seed:#x} has {} pales", pales.len());
        assert_eq!(pales[0].at, c.pale(), "the pale's tile is not where the county says it is");
    }
}

// ------------------------------------------------------ the shape of a walk

/// The circuit is the ring of the inner five by five, once round, no repeats.
#[test]
fn the_circuit_is_a_ring_and_walks_itself() {
    assert_eq!(CIRCUIT.len(), 16);
    let unique: std::collections::BTreeSet<_> = CIRCUIT.iter().collect();
    assert_eq!(unique.len(), 16, "the ring visits a tile twice");
    for p in CIRCUIT.iter() {
        assert!(
            (1..=5).contains(&p.0) && (1..=5).contains(&p.1),
            "{p:?} is not in the inner five by five"
        );
        assert!(
            p.0 == 1 || p.0 == 5 || p.1 == 1 || p.1 == 5,
            "{p:?} is inside the ring rather than on it"
        );
    }
    // Consecutive, and closing.
    for i in 0..16 {
        let a = CIRCUIT[i];
        let b = CIRCUIT[(i + 1) % 16];
        assert_eq!(county::manhattan(a, b), 1, "the ring jumps from {a:?} to {b:?}");
    }
}

/// Six mouths, one per town, all on the edge and none on a toll.
///
/// A gate you cannot walk out of is a trip that ends before it starts, and
/// the checks all measure distance *from* a mouth - so a mouth on a Feature
/// would be tuning the ruler.
#[test]
fn every_town_has_a_mouth_and_every_mouth_is_a_way_in() {
    let towns: Vec<&str> = gearmaster_engine::town::TOWNS.iter().map(|t| t.id).collect();
    assert_eq!(MOUTHS.len(), towns.len(), "a town without a mouth, or a mouth without a town");
    for (id, _) in MOUTHS.iter() {
        assert!(towns.contains(id), "{id} is a mouth and not a town");
    }
    let places: std::collections::BTreeSet<_> = MOUTHS.iter().map(|(_, p)| *p).collect();
    assert_eq!(places.len(), MOUTHS.len(), "two towns share a mouth");

    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        for (id, m) in MOUTHS.iter() {
            assert!(county::on_edge(*m), "{id}'s mouth at {m:?} is not on the edge");
            assert!(
                !matches!(c.at(*m).kind, TileKind::Feature(_)),
                "seed {seed:#x}: {id}'s mouth is a toll"
            );
            assert!(
                !matches!(
                    c.at(*m).kind,
                    TileKind::Objective { .. } | TileKind::Pinnacle { .. } | TileKind::Gaol
                ),
                "seed {seed:#x}: {id}'s mouth is skeleton"
            );
        }
    }
}

/// The far corner the pale opens is three tiles, none of them on the ring.
///
/// A two-by-two block would be the obvious shape and every one of the four
/// contains exactly one circuit tile, which would walk the Drover into a
/// region nobody can enter.
#[test]
fn the_sealed_corner_is_an_l_and_never_touches_the_ring() {
    for corner in county::CORNERS {
        let l = county::corner_l(corner);
        assert_eq!(l.len(), 3);
        assert!(l.contains(&corner));
        for p in l {
            assert!(county::on_edge(p), "{p:?} of {corner:?}'s L is not on the edge");
            assert!(!county::on_circuit(p), "{p:?} of {corner:?}'s L is on the ring");
        }
        // The two-by-two the L is not.
        let block = [corner, (if corner.0 == 0 { 1 } else { W - 2 }, corner.1),
                     (corner.0, if corner.1 == 0 { 1 } else { H - 2 }),
                     (if corner.0 == 0 { 1 } else { W - 2 }, if corner.1 == 0 { 1 } else { H - 2 })];
        assert_eq!(
            block.iter().filter(|p| county::on_circuit(**p)).count(),
            1,
            "the reason the sealed region is an L and not a block has stopped being true at {corner:?}"
        );
    }

    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        assert_eq!(*c.sealed(), county::corner_l(c.sealed()[0]));
        // The Enclosure's ending is behind it, which is the chain's own joke.
        assert!(c.is_sealed(c.pinnacle(Chain::Enclosure).unwrap()), "seed {seed:#x}: the Commissioner is not behind the pale");
        assert!(c.is_sealed(c.objectives(Chain::Enclosure)[2]), "seed {seed:#x}: the third stone is not behind the pale");
        assert!(!c.is_sealed(c.objectives(Chain::Enclosure)[0]));
        assert!(!c.is_sealed(c.objectives(Chain::Enclosure)[1]));
    }
}

/// Two sightings are knowledge and the third is the key.
///
/// The geometry half of B1.1, which is all F1 owns: three lines through one
/// tile, pairwise distinct, so any two of them cross at the hill and nowhere
/// else. A player who draws two knows where to walk. Taking the third is what
/// makes the tile a pinnacle, and that is F8's.
#[test]
fn any_two_bearings_cross_only_at_the_hill() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        let hill = c.hill();
        assert!(!county::on_edge(hill), "seed {seed:#x}: the hill is on the edge");
        let b = c.bearings();
        for i in 0..3 {
            for j in i + 1..3 {
                let both: Vec<(u8, u8)> = (0..H)
                    .flat_map(|y| (0..W).map(move |x| (x, y)))
                    .filter(|p| b[i].holds(hill, *p) && b[j].holds(hill, *p))
                    .collect();
                assert_eq!(both, vec![hill], "seed {seed:#x}: {:?} and {:?} meet at {both:?}", b[i], b[j]);
            }
        }
        // And each line carries exactly one trig point.
        let trigs = c.objectives(Chain::Ordnance);
        assert_eq!(trigs.len(), 3);
        for line in b {
            assert_eq!(
                trigs.iter().filter(|t| line.holds(hill, **t)).count(),
                1,
                "seed {seed:#x}: {line:?} does not carry exactly one trig point"
            );
        }
    }
}

/// V7 is the one check that cannot refuse anything, and this is the figure.
///
/// "Every tile within eight moves of some mouth" on a seven by seven with six
/// mouths on its edge: the furthest any tile ever gets is measured below and
/// it is nowhere near eight. V7 is kept rather than deleted because it is the
/// invariant the five-move budget is chosen against, and because the day the
/// grid grows or the mouth table shrinks it is what will fail - **loudly**,
/// since the assertion is on the measured figure rather than on the check.
///
/// `CLAUDE.md` §6 trap 29 the other way round: not "what is the cheapest way
/// to satisfy this lint" but "is there any way at all to fail it". V2 was in
/// this test until the measurement refused it - the southern edge of a county
/// is reached by two mouths and no more, so three objectives can be stranded
/// there, and `every_check_refuses_the_thing_it_is_for` now does exactly that.
#[test]
fn the_check_that_can_only_pass_is_v7_and_here_is_the_figure() {
    let mut worst = 0u8;
    let mut worst_at = (0u8, 0u8);
    for t in FALLBACK.tiles() {
        // Plain breadth-first, ignoring tolls, which is what V7 asks.
        let mut seen = vec![vec![false; W as usize]; H as usize];
        let mut queue: Vec<((u8, u8), u8)> = MOUTHS.iter().map(|(_, m)| (*m, 0u8)).collect();
        for (_, m) in MOUTHS.iter() {
            seen[m.1 as usize][m.0 as usize] = true;
        }
        let mut head = 0;
        let mut found = None;
        while head < queue.len() {
            let (p, d) = queue[head];
            head += 1;
            if p == t.at {
                found = Some(d);
                break;
            }
            for q in county::neighbours(p) {
                if !seen[q.1 as usize][q.0 as usize] {
                    seen[q.1 as usize][q.0 as usize] = true;
                    queue.push((q, d + 1));
                }
            }
        }
        let d = found.expect("a seven by seven of orthogonal steps is connected");
        if d > worst {
            worst = d;
            worst_at = t.at;
        }
    }
    println!("the furthest tile from every mouth is {worst_at:?}, at {worst} moves");
    assert!(
        worst < 8,
        "{worst_at:?} is {worst} moves from every mouth, so V7 has become a check that can \
         refuse - which is news, and it wants a county test of its own"
    );
}

/// Being arrested is the fastest ride into the middle there is.
///
/// V9 puts the gaol within three of D4 and every mouth is on an edge, so C1's
/// punishment is a shortcut. It is allowed to work - a punishment a clever
/// player farms beats one a careful player avoids - and this is the assertion
/// that says so out loud rather than a doc comment nobody reads.
#[test]
fn the_gaol_is_deeper_in_than_any_mouth() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        let gaol = c.gaol().expect("a generated county has a gaol");
        assert!(county::manhattan(gaol, (3, 3)) <= 3, "seed {seed:#x}: the gaol is not near the middle");
        let nearest = MOUTHS.iter().map(|(_, m)| county::manhattan(gaol, *m)).min().unwrap();
        assert!(
            nearest >= 2,
            "seed {seed:#x}: the gaol at {gaol:?} is {nearest} from a mouth, so being arrested \
             saves nothing and C1 is a punishment rather than a shortcut"
        );
    }
}

// ---------------------------------------------------- the events, not yet

/// No county tile names an event that does not exist.
///
/// Vacuous while `COUNTY_EVENTS` is empty and it is not vacuous *quietly*:
/// every arranged tile carries [`county::UNARRANGED`], which is the whole of
/// the exemption, and the mirror below goes red the day the pool has anything
/// in it. F7 cannot land the events without putting these tiles back under
/// this lint.
#[test]
fn every_event_tile_names_an_event_or_says_it_is_waiting() {
    for seed in a_spread_of_seeds() {
        let c = county::generate(seed);
        for t in c.tiles() {
            if let TileKind::Event(id) = t.kind {
                assert!(
                    id == county::UNARRANGED || id == county::PALE,
                    "seed {seed:#x}: {:?} names {id}, which is neither the pale nor a tile \
                     waiting for F7",
                    t.at
                );
            }
        }
    }
}

/// County events never fight.
///
/// The county's only fights are its pinnacles and THE PARISH. Vacuous until
/// F7 authors the pool - present now so that the milestone which writes the
/// first county event finds the lint rather than remembering it.
#[test]
fn county_events_never_fight() {
    // `COUNTY_EVENTS` lands at F7. Until it does, the restriction has nothing
    // to check and this test is the placeholder that says which outcomes are
    // barred: FightAsWritten, FightInstead, Step, Enter, StartDungeon.
    let pool: &[gearmaster_engine::event::LadderEvent] = &[];
    for e in pool {
        for ch in e.choices {
            for o in gearmaster_engine::event::every_outcome(&ch.outcome) {
                assert!(
                    !matches!(
                        o,
                        gearmaster_engine::event::Outcome::FightAsWritten
                            | gearmaster_engine::event::Outcome::FightInstead(_)
                            | gearmaster_engine::event::Outcome::StartDungeon(_)
                    ),
                    "{} fights, and the county's only fights are its pinnacles",
                    e.id
                );
            }
        }
    }
}
