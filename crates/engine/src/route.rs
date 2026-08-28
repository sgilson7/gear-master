//! The road, drawn.
//!
//! A branching map of a whole run, and a pure function of the tables plus run
//! state - so it can never depict a road the game does not have. `LADDER`,
//! `TOWNS`, `EVENTS` and `DUNGEONS` are where the nodes come from; what is
//! filled in and which branches were taken come from the run; and every name
//! is canonical, so the theme layer swaps them on the way to a screen.
//!
//! Because it lives here rather than in the interface, the headless driver
//! prints the same map in ASCII for nothing, and the test for it is one
//! assertion per rule rather than a screenshot nobody can read a diff of.
//!
//! ## The grammar
//!
//! 1. **The spine is the ladder.** All fifty rungs are visible from rung one,
//!    with the pinned towns and the bosses already marked ahead. Cleared rungs
//!    are filled, the road ahead is hollow, the current rung is ringed.
//! 2. **Loops are events** - an out-and-back branch off the rung, which is
//!    literally a rendering of the road stack. A dungeon opened mid-event
//!    extends the loop deeper before it returns to the rung it left.
//! 3. **Exceptions draw as exceptions.** A branch that does not return home -
//!    a rung bought off, a stone that skips one - is a merge-ahead edge to
//!    wherever it actually lands.
//! 4. **Hidden towns sit off-spine**, because they were never on the road
//!    until something put them there. Pinned towns are on it.
//! 5. **Rung fifty-one appears only once the Mainspring is held.** The map
//!    growing a node past Francis *is* the reveal; nothing else announces it.
//! 6. Hover is the interface's, and reads the same `describe()`s everything
//!    else does.

use crate::combat::{Rank, LADDER};
use crate::run::Run;

/// What a node stands for.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum NodeKind {
    /// A creature on the ladder. `rank` is what makes a boss draw larger.
    Rung(Rank),
    /// A town. `pinned` is false for one that had to be found.
    Town { pinned: bool },
    /// An event standing in front of a rung.
    Event,
    /// A mini dungeon: how many fights the longest road through it is, and
    /// how many places it asks which way.
    ///
    /// Fights rather than floors, because a graph's room count is not a thing
    /// a run experiences - nine rooms with points in them are four fights
    /// whichever way you walk.
    Dungeon { fights: usize, forks: usize },
    /// A fountain owed at this rung.
    Fountain,
    /// The thing past Francis. Present only when the Mainspring is held.
    PastTheTop,
}

/// How far the run has got with this.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Fill {
    /// Behind you.
    Cleared,
    /// Where you are standing.
    Current,
    /// Hollow, and dashed.
    Ahead,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    /// The table id, where the thing has one. Empty for a plain rung.
    pub id: &'static str,
    /// Canonical. The theme layer swaps it.
    pub label: &'static str,
    /// The rung this hangs off, indexed from zero.
    pub at: usize,
    pub fill: Fill,
    /// Drawn beside the spine rather than on it.
    pub off_spine: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum EdgeKind {
    /// One rung to the next.
    Spine,
    /// Out to something beside the road, and back again.
    Branch,
    /// Out, and not back: it lands somewhere further along.
    MergeAhead,
}

#[derive(Copy, Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, Default)]
pub struct RouteMap {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl RouteMap {
    /// Nodes standing on, or hanging off, one rung.
    pub fn at(&self, rung: usize) -> Vec<usize> {
        self.nodes.iter().enumerate().filter(|(_, n)| n.at == rung).map(|(i, _)| i).collect()
    }

    /// The spine node for a rung, if the map has one.
    pub fn spine_of(&self, rung: usize) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| n.at == rung && matches!(n.kind, NodeKind::Rung(_)) && !n.off_spine)
    }
}

fn fill_for(run: &Run, rung: usize) -> Fill {
    if rung < run.rung {
        Fill::Cleared
    } else if rung == run.rung {
        Fill::Current
    } else {
        Fill::Ahead
    }
}

/// The whole road, as this run has it.
pub fn route(run: &Run) -> RouteMap {
    let mut map = RouteMap::default();

    // ---- rule 1: the spine.
    for (i, m) in LADDER.iter().enumerate() {
        map.nodes.push(Node {
            kind: NodeKind::Rung(m.rank),
            id: "",
            label: m.name,
            at: i,
            fill: fill_for(run, i),
            off_spine: false,
        });
    }
    for i in 1..LADDER.len() {
        let (Some(from), Some(to)) = (map.spine_of(i - 1), map.spine_of(i)) else { continue };
        map.edges.push(Edge { from, to, kind: EdgeKind::Spine });
    }

    // ---- rules 1 and 4: towns.
    //
    // A pinned town is furniture and stands on the spine; a hidden one was
    // never on the road until something put it there, so it hangs beside it.
    // Both are drawn only where they are: a hidden town nobody has heard of is
    // not a secret the map keeps, it is a place that does not exist yet.
    for t in crate::town::TOWNS {
        let pinned = matches!(t.unlock, crate::town::Unlock::Pinned);
        if !pinned && !run.towns_revealed.contains(&t.id) {
            continue;
        }
        let at = t.after + 1;
        map.nodes.push(Node {
            kind: NodeKind::Town { pinned },
            id: t.id,
            label: t.name,
            at,
            // Seen is behind you whichever rung you are on.
            fill: if run.towns_seen.contains(&t.id) {
                Fill::Cleared
            } else {
                fill_for(run, at)
            },
            off_spine: !pinned,
        });
        if !pinned {
            if let Some(spine) = map.spine_of(at) {
                let me = map.nodes.len() - 1;
                map.edges.push(Edge { from: spine, to: me, kind: EdgeKind::Branch });
            }
        }
    }

    // ---- fountains, which stand on a rung and are not one.
    for (i, &at) in Run::FOUNTAINS.iter().enumerate() {
        map.nodes.push(Node {
            kind: NodeKind::Fountain,
            id: "",
            label: "A FOUNTAIN",
            at,
            fill: if run.classes.iter().filter(|c| !crate::class::is_earned(c.name)).count() > i {
                Fill::Cleared
            } else {
                fill_for(run, at)
            },
            off_spine: true,
        });
        if let Some(spine) = map.spine_of(at) {
            let me = map.nodes.len() - 1;
            map.edges.push(Edge { from: spine, to: me, kind: EdgeKind::Branch });
        }
    }

    // ---- rule 2: loops are events.
    //
    // Two things here are not `fill_for` and not `e.at`, and both were bugs.
    //
    // **Where.** `LadderEvent::at` is a scheduled event's rung and an earned
    // one's *deadline*. THE CASINO's window is rungs two to nine, so drawing
    // it at `at` drew it at rung nine - a rung the run had not reached - for a
    // door that was answered on rung three. An earned event is drawn where it
    // is standing, or where it was answered, or at the first rung it could
    // turn up on, in that order. Never at the deadline.
    //
    // **Which one is ringed.** `fill_for` says `Current` for anything whose
    // rung is the rung you are on, so on rung three TWO BY TWO was ringed
    // whether or not it was the door being asked - and the casino, which was,
    // was drawn nine rungs away and hollow. `Current` means *standing*, which
    // is what `road_stack` already answers.
    let standing: Vec<&'static str> = run
        .road_stack()
        .iter()
        .filter_map(|i| match i {
            crate::run::Interrupt::Event(e) => Some(e.id),
            _ => None,
        })
        .collect();
    for e in crate::event::EVENTS {
        let answered_on = run.answered_on.iter().find(|(id, _)| *id == e.id).map(|(_, r)| *r);
        let here = standing.contains(&e.id);
        let at = if here {
            run.rung
        } else if let Some(r) = answered_on {
            r
        } else {
            // The earliest it can stand. `Trigger::from` is zero for a
            // scheduled event, whose `at` is its address, so this is `at` for
            // those and the window's opening for the rest.
            match e.trigger {
                crate::event::Trigger::Rung => e.at,
                _ => e.trigger.from(),
            }
        }
        .min(LADDER.len().saturating_sub(1));
        map.nodes.push(Node {
            kind: NodeKind::Event,
            id: e.id,
            label: e.title,
            at,
            fill: if here {
                Fill::Current
            } else if run.answered.contains(&e.id) {
                Fill::Cleared
            } else {
                // Not "behind you": an unanswered door did not happen, whether
                // or not its rung is behind you.
                Fill::Ahead
            },
            off_spine: true,
        });
        let me = map.nodes.len() - 1;
        if let Some(spine) = map.spine_of(at) {
            map.edges.push(Edge { from: spine, to: me, kind: EdgeKind::Branch });
        }
        // Through `every_outcome`, not `c.outcome`.
        //
        // It matched the top of the outcome, so a door that opens a dungeon
        // *and* does something else - `All[OpenShop, StartDungeon]` - drew no
        // dungeon at all. THE UNDER-MINE has been in the game since the
        // Unwinding and has never once been on this map, because the only two
        // choices that open it buy you a shelf on the way past.
        //
        // The Unwinding learned this exact lesson about `class::is_earned`,
        // `event::set_by` and its reachability lint, and wrote it down as the
        // most expensive thing that mission found (`HANDOFF.md` §4). This is
        // the one place it did not reach.
        // One node a dungeon, however many of a door's choices open it. THE
        // FOUNDRY offers the shelf then the seam and the seam then the shelf,
        // and they are two ways through one door rather than two dungeons.
        let mut drawn: Vec<&'static str> = Vec::new();
        for c in e.choices {
            for out in crate::event::every_outcome(&c.outcome) {
            match *out {
                // ---- rule 2, deeper: a dungeon extends the loop.
                crate::event::Outcome::Enter(id)
                | crate::event::Outcome::StartDungeon(id) => {
                    let Some(d) = crate::dungeon::by_id(id) else { continue };
                    if drawn.contains(&d.id) {
                        continue;
                    }
                    drawn.push(d.id);
                    let inside = run.dungeon.is_some_and(|(x, _)| x.id == d.id);
                    map.nodes.push(Node {
                        kind: NodeKind::Dungeon {
                            fights: d.fights_ahead(0, &[]),
                            forks: d.forks(),
                        },
                        id: d.id,
                        label: d.name,
                        at,
                        fill: if inside { Fill::Current } else { fill_for(run, at) },
                        off_spine: true,
                    });
                    let deep = map.nodes.len() - 1;
                    map.edges.push(Edge { from: me, to: deep, kind: EdgeKind::Branch });
                }
                // ---- rule 3: a branch that does not come home.
                crate::event::Outcome::BuyOff { .. } => {
                    if let Some(next) = map.spine_of(at + 1) {
                        map.edges.push(Edge { from: me, to: next, kind: EdgeKind::MergeAhead });
                    }
                }
                _ => {}
            }
            }
        }
    }

    // ---- rule 5: the map grows a node past Francis, and that is the reveal.
    if run.holds(crate::run::MAINSPRING) {
        map.nodes.push(Node {
            kind: NodeKind::PastTheTop,
            id: "the-unwound",
            label: "THE UNWOUND",
            at: LADDER.len(),
            fill: fill_for(run, LADDER.len()),
            off_spine: false,
        });
        let me = map.nodes.len() - 1;
        if let Some(last) = map.spine_of(LADDER.len() - 1) {
            map.edges.push(Edge { from: last, to: me, kind: EdgeKind::Spine });
        }
    }

    map
}

/// The map, in one column of characters.
///
/// The headless driver's version, and the reason `route` is in the engine at
/// all: two renderings of one function cannot disagree about which road the
/// game has.
/// The road half of the map, alone.
///
/// What `ascii` was until F9. The three fixtures in `the_road.rs` pin this,
/// and `ascii` is this plus the county when there is a county to draw.
pub fn ascii_road(run: &Run) -> Vec<String> {
    let map = route(run);
    let mut out = Vec::new();
    let mark = |f: Fill| match f {
        Fill::Cleared => '#',
        Fill::Current => 'O',
        Fill::Ahead => '.',
    };
    for rung in 0..=LADDER.len() {
        let here = map.at(rung);
        if here.is_empty() {
            continue;
        }
        // Anything that is not a rung stands *between* two of them and happens
        // between two fights - a gate after one and before the next, an event
        // in front of the fight it interrupts, a fountain owed on arrival. So
        // it prints above the rung's own line rather than under it, which is
        // the order the player meets them in. A dungeon keeps its place
        // directly under the event that opened it.
        let (between, spine): (Vec<usize>, Vec<usize>) = here
            .iter()
            .partition(|&&i| !matches!(map.nodes[i].kind, NodeKind::Rung(_) | NodeKind::PastTheTop));
        for &i in between.iter().chain(spine.iter()) {
            let n = &map.nodes[i];
            match n.kind {
                // A rung is a rung: a mark, a number and a name.
                NodeKind::Rung(r) => {
                    let tag = if r == Rank::Ordinary {
                        String::new()
                    } else {
                        format!(" [{}]", format!("{:?}", r).to_lowercase())
                    };
                    out.push(format!("{} {:>2} {}{}", mark(n.fill), rung + 1, n.label, tag));
                }
                // A town is a diamond, and it does not take a rung number,
                // because it does not stand on one - it stands between two.
                NodeKind::Town { pinned } => out.push(format!(
                    "{} <> {} (a town{}, between {} and {})",
                    mark(n.fill),
                    n.label,
                    if pinned { "" } else { ", found" },
                    rung,
                    rung + 1
                )),
                NodeKind::PastTheTop => {
                    out.push(format!("{} {:>2} {}", mark(n.fill), rung + 1, n.label))
                }
                NodeKind::Event => {
                    out.push(format!("{} -- {} (event, between {} and {})", mark(n.fill), n.label, rung, rung + 1))
                }
                // The word is "points" and it is dropped at zero, so a
                // straight line reads the way it always did apart from the one
                // word: `floors` was the room count and `fights` is what a run
                // walks, and for the six straight lines the number is the same.
                NodeKind::Dungeon { fights, forks } => out.push(if forks == 0 {
                    format!("     \\_ {} ({} fights)", n.label, fights)
                } else {
                    format!("     \\_ {} ({} fights, {} points)", n.label, fights, forks)
                }),
                NodeKind::Fountain => out.push(format!(
                    "{} -- {} (fountain, between {} and {})",
                    mark(n.fill),
                    n.label,
                    rung,
                    rung + 1
                )),
            }
        }
    }
    out
}

/// The whole map: the road, and THE HUNDRED under it.
///
/// The road half is unchanged and always first, which is what the three
/// fixtures in `the_road.rs` pin - they hold `ascii_road`'s output and this
/// function's first ninety-six lines are it.
pub fn ascii(run: &Run) -> Vec<String> {
    let mut out = ascii_road(run);
    out.push(String::new());
    out.extend(ascii_county(run));
    out
}

/// THE HUNDRED, seven by seven, in the road's own vocabulary.
///
/// A8's drawing rules. Marks first, because they are the whole grammar:
///
/// ```text
///   #  cleared          a tile this run has finished with
///   O  where you are
///   o  seen             adjacent to somewhere you have been
///   .  known of         a mouth, or a line a sighting drew
///   (blank)             never been near it
/// ```
///
/// A toll shows its glyph always and its **threshold only when known** - one
/// tile away, or anywhere at all once the Ordnance has paid out its sheet.
/// That is the whole of why the sheet is a reward: a county you can read from
/// the road is a county you plan on paper.
pub fn ascii_county(run: &Run) -> Vec<String> {
    use crate::county::{self, Chain, TileKind, H, W};
    let mut out = Vec::new();
    let been = !run.county_trips.is_empty();
    if !been {
        // Greyed with a line, which is what a map says about a place you have
        // heard of and not gone to.
        out.push("THE HUNDRED".into());
        out.push("  a county, under the road. Every town has steps down.".into());
        return out;
    }

    let c = run.county();
    let seen = |p: (u8, u8)| -> bool {
        run.county_is_cleared(p)
            || county::neighbours(p).iter().any(|n| run.county_is_cleared(*n))
            || county::is_mouth(p)
            || run.county_at == Some(p)
    };
    // A sighting draws its whole line on the map from the moment it is taken.
    let drawn: Vec<(u8, u8)> = {
        let written = run.county_written();
        let mut lines = Vec::new();
        for n in 1..=run.sightings() as u8 {
            lines.extend(written.sighting(n));
        }
        lines
    };

    out.push("THE HUNDRED".into());
    // The column letters sit over the cells rather than beside them: a row is
    // two characters of number, two of gap, then six a tile.
    let mut head = "    ".to_string();
    for x in 0..W {
        head.push_str(&format!("{:<6}", (b'A' + x) as char));
    }
    out.push(head.trim_end().to_string());
    for y in 0..H {
        let mut row = format!("{:>2}  ", y + 1);
        for x in 0..W {
            let p = (x, y);
            let t = c.at(p);
            let mark = if run.county_at == Some(p) {
                'O'
            } else if run.county_is_cleared(p) {
                '#'
            } else if seen(p) {
                'o'
            } else if drawn.contains(&p) {
                '.'
            } else {
                ' '
            };
            // A toll's glyph is always drawn; its figure is not.
            let body = match t.kind {
                TileKind::Feature(toll) if run.county_threshold_known(p) => toll.threshold(),
                TileKind::Feature(toll) => format!("{}{}?", toll.glyph(), toll.letter()),
                _ if !seen(p) && !drawn.contains(&p) => String::new(),
                // A chain nobody has explained to you is stones in fields.
                // The on-ramps are what turn them into a numbered thing, and
                // this is what those three doors actually buy.
                TileKind::Objective { chain, nth } if run.knows_the_chain(chain) => format!(
                    "{}{nth}",
                    match chain {
                        Chain::Ordnance => 'T',
                        Chain::Drove => 'S',
                        Chain::Enclosure => 'B',
                    }
                ),
                TileKind::Objective { .. } => "stone".into(),
                TileKind::Pinnacle { .. } => "***".into(),
                TileKind::Gaol => "gaol".into(),
                TileKind::Event(id) if id == county::PALE => "PALE".into(),
                TileKind::Event(_) => "?".into(),
                TileKind::Empty => String::new(),
            };
            row.push_str(&format!("{mark}{body:<5}"));
        }
        out.push(row.trim_end().to_string());
    }

    // The gates, and which of them have been found.
    let mut mouths: Vec<String> = Vec::new();
    for (id, m) in county::MOUTHS.iter() {
        let town = crate::town::by_id(id);
        let found = town.is_some_and(|t| {
            matches!(t.unlock, crate::town::Unlock::Pinned) || run.towns_revealed.contains(id)
        });
        mouths.push(format!(
            "{} {}",
            county::reference(*m),
            if found { town.map(|t| t.name).unwrap_or(id) } else { "a town you have not found" }
        ));
    }
    out.push(format!("  gates: {}", mouths.join(" · ")));

    // The Drover, once a sign has taught you to look.
    if run.signs_read() >= 1 && !run.county_chain_done(Chain::Drove) {
        out.push(format!(
            "  the drover: {} (clock {})",
            county::reference(run.drover_tile()),
            run.events_resolved
        ));
    }

    // The pale's checklist, at one tile.
    let at_the_pale = run
        .county_at
        .is_some_and(|here| county::manhattan(here, c.pale()) <= 1);
    if at_the_pale {
        out.push(format!("  {} - the pale:", county::reference(c.pale())));
        for (r, met) in run.pale_checklist() {
            out.push(format!("    [{}] {}", if met { 'x' } else { ' ' }, r.describe()));
        }
    }

    out.push(format!(
        "  {} of 49 cleared · {} of {} trips spent",
        run.county_cleared.len(),
        run.county_trips.len(),
        crate::run::trip_cap()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_run() -> Run {
        let mut run = Run::seeded(0x8001);
        run.difficulty = crate::combat::Difficulty::Easy;
        run
    }

    /// Rule 1.
    #[test]
    fn the_spine_is_the_whole_ladder_and_fills_as_the_run_climbs() {
        let mut run = a_run();
        run.rung = 12;
        let map = route(&run);
        for i in 0..LADDER.len() {
            let n = map.spine_of(i).map(|x| &map.nodes[x]).expect("every rung is on the map");
            assert_eq!(n.label, LADDER[i].name);
            let want = if i < 12 {
                Fill::Cleared
            } else if i == 12 {
                Fill::Current
            } else {
                Fill::Ahead
            };
            assert_eq!(n.fill, want, "rung {}", i + 1);
        }
        let spine = map.edges.iter().filter(|e| e.kind == EdgeKind::Spine).count();
        assert_eq!(spine, LADDER.len() - 1, "the road has a gap in it");
    }

    /// Rule 1, the other half: what is ahead is already marked.
    #[test]
    fn the_bosses_and_the_pinned_towns_are_on_the_map_from_rung_one() {
        let run = a_run();
        let map = route(&run);
        let bosses = map
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Rung(r) if r != Rank::Ordinary))
            .count();
        assert!(bosses > 0, "nothing on the road is named");
        for t in crate::town::TOWNS.iter().filter(|t| t.unlock == crate::town::Unlock::Pinned) {
            assert!(
                map.nodes.iter().any(|n| n.id == t.id),
                "{} is not on the map at rung one",
                t.id
            );
        }
        // And a hidden one is not, which is the other half of the rule.
        for t in crate::town::TOWNS.iter().filter(|t| t.unlock == crate::town::Unlock::Hidden) {
            assert!(
                !map.nodes.iter().any(|n| n.id == t.id),
                "{} is on the map before anybody found it",
                t.id
            );
        }
    }

    /// Rule 2.
    #[test]
    fn every_event_hangs_off_the_rung_it_stands_on() {
        let run = a_run();
        let map = route(&run);
        for e in crate::event::EVENTS {
            let i = map.nodes.iter().position(|n| n.id == e.id).expect("on the map");
            assert!(map.nodes[i].off_spine, "{} was drawn on the spine", e.id);
            let home = map.spine_of(map.nodes[i].at).expect("a rung to hang off");
            assert!(
                map.edges
                    .iter()
                    .any(|x| x.from == home && x.to == i && x.kind == EdgeKind::Branch),
                "{} has no way back to the road",
                e.id
            );
        }
    }

    /// Rule 2, deeper.
    #[test]
    fn a_dungeon_extends_the_loop_the_event_opened() {
        let run = a_run();
        let map = route(&run);
        let d = map
            .nodes
            .iter()
            .position(|n| matches!(n.kind, NodeKind::Dungeon { .. }))
            .expect("the shipped dungeon");
        let from = map
            .edges
            .iter()
            .find(|e| e.to == d && e.kind == EdgeKind::Branch)
            .map(|e| e.from)
            .expect("a dungeon nobody can reach");
        assert_eq!(
            map.nodes[from].kind,
            NodeKind::Event,
            "the dungeon hangs off the road rather than off the door that opens it"
        );
    }

    /// Rule 3.
    #[test]
    fn a_branch_that_does_not_come_home_lands_where_its_outcome_says() {
        let run = a_run();
        let map = route(&run);
        let toad = map.nodes.iter().position(|n| n.id == "the-toads-offer").expect("authored");
        let merge = map
            .edges
            .iter()
            .find(|e| e.from == toad && e.kind == EdgeKind::MergeAhead)
            .expect("buying a rung off does not return to it");
        let at = map.nodes[toad].at;
        assert_eq!(map.spine_of(at + 1), Some(merge.to), "it merged into the wrong rung");
    }

    /// Rule 4.
    #[test]
    fn a_hidden_town_is_not_on_the_map_until_it_is_and_then_it_is_off_the_spine() {
        let mut run = a_run();
        for t in crate::town::TOWNS {
            let found = route(&run).nodes.iter().find(|n| n.id == t.id).cloned();
            match t.unlock {
                crate::town::Unlock::Pinned => {
                    let n = found.expect("a pinned town is on the map from rung one");
                    assert!(!n.off_spine, "{} is pinned and was drawn beside the road", t.id);
                }
                crate::town::Unlock::Hidden => {
                    assert!(found.is_none(), "{} was on the map before anybody found it", t.id)
                }
            }
        }
        // Found, and it is beside the road rather than on it - because it was
        // never on the road until something put it there.
        let hidden = crate::town::TOWNS
            .iter()
            .find(|t| t.unlock == crate::town::Unlock::Hidden)
            .expect("the chain finds two");
        run.towns_revealed.push(hidden.id);
        let n = route(&run)
            .nodes
            .iter()
            .find(|n| n.id == hidden.id)
            .cloned()
            .expect("revealed, and on the map");
        assert!(n.off_spine, "{} was drawn on the spine", hidden.id);

        run.towns_revealed.push("nowhere");
        assert!(
            !route(&run).nodes.iter().any(|n| n.id == "nowhere"),
            "revealing a town that does not exist put it on the map"
        );
    }

    /// Rule 5.
    #[test]
    fn nothing_stands_past_francis_until_the_mainspring_is_held() {
        let mut run = a_run();
        run.rung = LADDER.len() - 1;
        assert!(
            !route(&run).nodes.iter().any(|n| n.kind == NodeKind::PastTheTop),
            "the road past the top was on the map before anybody had earned it"
        );
        // Held, and the map grows a node. That *is* the reveal.
        let d = crate::piece::CATALOG.iter().position(|d| d.name == crate::run::MAINSPRING);
        let Some(d) = d else { return };
        let id = run.registry.alloc(d);
        run.owned.push(id);
        let map = route(&run);
        let past = map.nodes.iter().find(|n| n.kind == NodeKind::PastTheTop).expect("revealed");
        assert_eq!(past.at, LADDER.len());
        assert!(!past.off_spine, "the road past the top is the road");
    }

    #[test]
    fn the_map_reads_the_same_way_twice() {
        let run = a_run();
        let (a, b) = (ascii(&run), ascii(&run));
        assert_eq!(a, b);
        assert!(a.len() > LADDER.len(), "the map is shorter than the road");
    }

    #[test]
    fn every_edge_points_at_a_node_that_exists() {
        let mut run = a_run();
        run.rung = 20;
        let map = route(&run);
        for e in &map.edges {
            assert!(e.from < map.nodes.len() && e.to < map.nodes.len());
            assert_ne!(e.from, e.to, "an edge from a node to itself");
        }
    }
}
