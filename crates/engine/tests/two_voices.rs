//! The base game does not speak turtle.
//!
//! Doctrine four says a theme cannot break the game, and it is usually read as
//! a rule about *logic*: `theme.rs` is a lookup and nothing routes decisions
//! through it. There is a second half, quieter and easier to break, and this
//! file is it - **the canonical column has to be canonical**. A base game with
//! Fnorp in its prose is not a base game with a theme on top; it is one game
//! wearing its own name twice, and the plain theme has nothing to fall back
//! *to*.
//!
//! It broke. Every milestone of this mission wrote canonical scenes in the
//! book's voice, because the book's voice is the fun one to write in, and by
//! the end of Phase 2 there were fourteen scenes naming people who exist only
//! in a theme. They are moved: the canonical column names the role and the
//! turtle column names the man, which is what the two columns are for.
//!
//! Shipped as a ratchet, the way `catalog_shape` is - a budget that can only
//! go down, and an `#[ignore]`d target that asserts zero. The remaining
//! entries are **piece names in `CATALOG`**, which is append-only forever and
//! therefore the one place a leak cannot be fixed, only recorded.

use gearmaster_engine::combat::{ALTERNATES, LADDER};
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::EVENTS;
use gearmaster_engine::rumour::RUMOURS;
use gearmaster_engine::theme::{PLAIN, THEMES, TURTLE_DICK};
use gearmaster_engine::town::TOWNS;

/// The book's proper nouns.
///
/// Taken from the turtle theme's own `attributives` - the words it names items
/// out of - plus the people and places the design document cites with a page
/// number. A word is on this list because it means nothing to somebody who has
/// not read the book.
const BOOK: &[&str] = &[
    "Treyway", "Kaplin", "Multicity", "Petonkle", "Dobira", "Sneel", "Fnorp", "fnorp", "Yonk",
    "Mansus", "Bambulon", "Kolok", "Wextreen", "Yodregar", "Songil", "Promte", "Thrumbus",
    "Gooster", "gooster", "Frong", "Brumpus", "Octarine", "Wimpler", "Skoogle", "Drambus",
    "Nibbalonius", "Eggbert", "Burnwarp", "Weirdeir", "Weirdeirs", "Boyetano", "Ghirbi", "Bunko",
    "Foreston", "Sprocketman", "Sprocketmen", "Henpeck", "Drabley", "Galapagos", "Ypytryktrium",
    "Francian", "Corrqk", "Hanglo", "Chiemstar", "Chonga", "gortball", "Spindrift", "Kaklon",
];

/// Every string the canonical game shows a player, with where it came from.
fn canonical_prose() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |what: String, text: &str| out.push((what, text.to_string()));

    for e in EVENTS {
        push(format!("event {} title", e.id), e.title);
        for p in e.prose {
            push(format!("event {} prose", e.id), p);
        }
        for c in e.choices {
            push(format!("event {} label", e.id), c.label);
            push(format!("event {} blurb", e.id), c.blurb);
            push(format!("event {} unmet", e.id), c.unmet);
        }
    }
    for t in TOWNS {
        push(format!("town {} name", t.id), t.name);
        for p in t.blurb {
            push(format!("town {} blurb", t.id), p);
        }
    }
    for d in DUNGEONS {
        push(format!("dungeon {} name", d.id), d.name);
        for p in d.blurb.iter().chain(d.entry).chain(d.landings) {
            push(format!("dungeon {} prose", d.id), p);
        }
    }
    for r in RUMOURS {
        push(format!("rumour {}", r.name), r.name);
        push(format!("rumour {} hint", r.name), r.hint);
    }
    for m in LADDER.iter().chain(ALTERNATES) {
        push(format!("creature {}", m.name), m.name);
        for g in m.gear {
            push(format!("creature {} gear", m.name), g.0);
        }
        for d in m.drops {
            push(format!("creature {} drop", m.name), d);
        }
    }
    for c in gearmaster_engine::class::CLASSES {
        push(format!("class {}", c.name), c.name);
        push(format!("class {} blurb", c.name), c.blurb);
    }
    for d in gearmaster_engine::piece::CATALOG {
        push(format!("piece {}", d.name), d.name);
    }
    out
}

fn leaks() -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for (what, text) in canonical_prose() {
        for w in BOOK {
            // Whole words only: "Cork" is a substance in one voice and a stop
            // in a bottle in the other, and the ones that are ordinary English
            // are not on the list at all. Possessives count - "Henpeck's Cell
            // Keys" is Henpeck, and a lint that missed that would have passed
            // on the day it was written for.
            if text
                .split(|c: char| !c.is_alphanumeric() && c != '\'')
                .map(|t| t.trim_end_matches("'s"))
                .any(|t| t == *w)
            {
                out.push((what.clone(), *w));
            }
        }
    }
    out
}

/// Today's distance. It goes down or it does not move.
///
/// Five, and every one of them is the same fault: a component named after
/// somebody in the book, shipped before this rule existed. `CATALOG` is
/// index-keyed by `share.rs` and append-only forever, so a piece cannot be
/// renamed - only recorded here and translated by the theme, which it is.
/// Three are the pieces themselves and two are the creatures that drop them,
/// which is the same name counted where it is written down.
const BUDGET: usize = 5;

#[test]
fn the_base_game_speaks_no_turtle_it_has_not_already_shipped() {
    let found = leaks();
    assert!(
        found.len() <= BUDGET,
        "the canonical column gained {} book word(s) over its budget of {}:\n{:#?}",
        found.len(),
        BUDGET,
        found
    );
}

#[test]
fn no_budget_is_slack() {
    let found = leaks();
    assert_eq!(
        found.len(),
        BUDGET,
        "the leak list shrank to {} - lower BUDGET in the commit that earned it",
        found.len()
    );
}

/// The target, for the day `CATALOG` stops being append-only, which is never.
#[test]
#[ignore]
fn the_base_game_speaks_no_turtle_at_all() {
    assert_eq!(leaks(), Vec::new());
}

// ------------------------------------------------------ and the other half

#[test]
fn every_road_id_the_theme_retells_is_a_real_one() {
    for t in THEMES {
        for r in t.told {
            let real = EVENTS.iter().any(|e| e.id == r.id)
                || TOWNS.iter().any(|x| x.id == r.id)
                || DUNGEONS.iter().any(|d| d.id == r.id);
            assert!(real, "{} retells {}, which is nothing on the road", t.id, r.id);
        }
    }
}

#[test]
fn no_road_id_is_told_twice() {
    for t in THEMES {
        let mut seen: Vec<&str> = Vec::new();
        for r in t.told {
            assert!(!seen.contains(&r.id), "{} tells {} twice", t.id, r.id);
            seen.push(r.id);
        }
    }
    // And the three tables really are one namespace, which is the assumption
    // that lets `told` be one table rather than three.
    let mut all: Vec<&str> = Vec::new();
    for id in EVENTS
        .iter()
        .map(|e| e.id)
        .chain(TOWNS.iter().map(|t| t.id))
        .chain(DUNGEONS.iter().map(|d| d.id))
    {
        assert!(!all.contains(&id), "{} names two different places", id);
        all.push(id);
    }
}

#[test]
fn a_theme_with_nothing_to_say_says_the_canonical_thing() {
    // The whole safety argument for a half-written theme.
    for e in EVENTS {
        assert_eq!(PLAIN.place(e.id, e.title), e.title);
        assert_eq!(PLAIN.scene(e.id, e.prose), e.prose);
    }
    for d in DUNGEONS {
        assert_eq!(PLAIN.entry(d.id, d.entry), d.entry);
        assert_eq!(PLAIN.landings(d.id, d.landings), d.landings);
    }
}

#[test]
fn the_turtle_theme_retells_the_scenes_it_took_the_nouns_out_of() {
    // Every scene de-turtled above has to have its turtle text somewhere, or
    // the migration lost content instead of moving it.
    for id in [
        "the-crownwright",
        "the-casino",
        "the-long-way",
        "the-undertow",
        "the-threshold",
    ] {
        let r = TURTLE_DICK
            .told
            .iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("{} lost its turtle text", id));
        assert!(
            !r.prose.is_empty() || !r.entry.is_empty() || !r.landings.is_empty(),
            "{} is retitled but not retold",
            id
        );
    }
}
