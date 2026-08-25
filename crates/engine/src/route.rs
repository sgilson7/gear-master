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
    /// A mini dungeon, and how deep it goes.
    Dungeon { floors: usize },
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
    for e in crate::event::EVENTS {
        let at = e.at.min(LADDER.len().saturating_sub(1));
        let answered = run.answered.contains(&e.id);
        map.nodes.push(Node {
            kind: NodeKind::Event,
            id: e.id,
            label: e.title,
            at,
            fill: if answered { Fill::Cleared } else { fill_for(run, at) },
            off_spine: true,
        });
        let me = map.nodes.len() - 1;
        if let Some(spine) = map.spine_of(at) {
            map.edges.push(Edge { from: spine, to: me, kind: EdgeKind::Branch });
        }
        for c in e.choices {
            match c.outcome {
                // ---- rule 2, deeper: a dungeon extends the loop.
                crate::event::Outcome::Enter(id)
                | crate::event::Outcome::StartDungeon(id) => {
                    let Some(d) = crate::dungeon::by_id(id) else { continue };
                    let inside = run.dungeon.is_some_and(|(x, _)| x.id == d.id);
                    map.nodes.push(Node {
                        kind: NodeKind::Dungeon { floors: d.floors.len() },
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
pub fn ascii(run: &Run) -> Vec<String> {
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
        for &i in &here {
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
                NodeKind::Event => out.push(format!("   \\_ {} (event)", n.label)),
                NodeKind::Dungeon { floors } => {
                    out.push(format!("     \\_ {} ({} floors)", n.label, floors))
                }
                NodeKind::Fountain => out.push(format!("   \\_ {} (fountain)", n.label)),
            }
        }
    }
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
        for t in crate::town::TOWNS {
            assert!(
                map.nodes.iter().any(|n| n.id == t.id),
                "{} is not on the map at rung one",
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
        // No hidden towns ship yet, so this is the machinery's assertion: a
        // pinned town is on the spine, and the predicate that would put a
        // hidden one beside it is the same one `town_between` reads.
        for t in crate::town::TOWNS {
            let n = route(&run).nodes.iter().find(|n| n.id == t.id).cloned().expect("on the map");
            assert!(!n.off_spine, "{} is pinned and was drawn beside the road", t.id);
        }
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
