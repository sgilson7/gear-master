//! Every verb belongs to exactly one agent.
//!
//! Both directions, because half a lint is not a lint (`CLAUDE.md` §6 trap 19):
//!
//! * **Nothing shared** - a verb both lists claim is a decision two learners
//!   fight over and neither is blamed for.
//! * **Nothing lost** - a verb neither claims is an action nobody can take, and
//!   it would go missing silently. THE APPRENTICE lost a whole fountain that
//!   way, at rung 47.
//!
//! The list is read from the source of `verb.rs` rather than from a match, so
//! adding a variant to `Verb` fails this test until somebody decides which
//! trade owns it. That is the point: the decision is the deliverable, not the
//! bookkeeping.

use gearmaster_trades::{PATHFINDER, QUARTERMASTER};

const VERBS: &str = include_str!("../../console/src/verb.rs");

/// Every variant of `Verb`, read off the enum.
fn variants() -> Vec<String> {
    let body = VERBS
        .split("pub enum Verb {")
        .nth(1)
        .expect("the enum is there")
        .split("\n}")
        .next()
        .expect("and it ends");
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('/') {
            continue;
        }
        // `Place { piece: PieceId, ... },` or `ClearAll,`
        let name: String = t.chars().take_while(|c| c.is_alphanumeric()).collect();
        if name.is_empty() || !name.starts_with(|c: char| c.is_uppercase()) {
            continue;
        }
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[test]
fn the_two_lists_are_disjoint() {
    let both: Vec<&&str> = QUARTERMASTER.iter().filter(|q| PATHFINDER.contains(q)).collect();
    assert!(both.is_empty(), "both trades claim {:?}", both);
}

#[test]
fn between_them_they_own_every_verb() {
    let all = variants();
    assert!(all.len() > 20, "only found {} variants - the reader is broken", all.len());
    let mut orphans = Vec::new();
    for v in &all {
        if !QUARTERMASTER.contains(&v.as_str()) && !PATHFINDER.contains(&v.as_str()) {
            orphans.push(v.clone());
        }
    }
    assert!(
        orphans.is_empty(),
        "no trade owns {:?}. A verb nobody owns is an action nobody can take - \
         decide which agent it belongs to rather than letting it go missing.",
        orphans
    );
}

#[test]
fn neither_list_names_a_verb_that_does_not_exist() {
    let all = variants();
    for named in QUARTERMASTER.iter().chain(PATHFINDER.iter()) {
        assert!(
            all.contains(&named.to_string()),
            "`{}` is claimed by a trade and is not a Verb",
            named
        );
    }
}

#[test]
fn the_partition_is_the_size_it_says_it_is() {
    // A number rather than a feeling: sixteen apiece at Q0, and a commit that
    // moves it says why in its own message.
    assert_eq!(QUARTERMASTER.len(), 16, "the quartermaster's half");
    assert_eq!(PATHFINDER.len(), 16, "the pathfinder's half");
    assert_eq!(variants().len(), 32, "the whole vocabulary");
}
