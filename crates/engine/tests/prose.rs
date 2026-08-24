//! What the game is allowed to sound like.
//!
//! Every scene in this game is written in one voice, taken from the book the
//! theme comes from: a grave, patiently-observed situation with a mundane
//! errand underneath it, and rules explained carefully for insane things. A
//! monastery at the top of a frozen mountain, three days of climbing, the
//! Master's grave question - and the answer is a delivery, and the complaint
//! is about pickles.
//!
//! What that voice is *not* is the thing this file exists to catch. Left alone,
//! the prose drifted into a register that withholds every noun ("something is
//! running", "the square thing", "what a cart becomes"), sets mood in place of
//! fact, and closes every paragraph on the same deflating half-sentence. It
//! reads like atmosphere and carries no information, and nine events in a row
//! of it reads like one event nine times.
//!
//! These are cheap mechanical proxies, not literary judgement: a sentence can
//! pass all of them and still be bad. Three of them - the hedging phrases, the
//! scene naming nothing, and the mood titles - fail outright on the prose that
//! was here before this file existed, which is what they were written from.
//! The rest are guards rather than detectors: they hold today and would catch
//! the drift coming back.

use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::EVENTS;
use gearmaster_engine::town::{Action, TOWNS};

/// Everything the player reads, with a label for the failure and whether it is
/// a paragraph of a scene or a line on a button. The rules differ: a paragraph
/// has to be about something, a button is allowed to be plain.
fn scenes() -> Vec<(String, &'static str, bool)> {
    let mut out: Vec<(String, &'static str, bool)> = Vec::new();
    for e in EVENTS {
        for p in e.prose {
            out.push((format!("{} prose", e.id), p, true));
        }
        for c in e.choices {
            out.push((format!("{} / {}", e.id, c.label), c.blurb, false));
            if !c.unmet.is_empty() {
                out.push((format!("{} / {} (shut)", e.id, c.label), c.unmet, false));
            }
        }
    }
    for t in TOWNS {
        for p in t.blurb {
            out.push((format!("{} blurb", t.id), p, true));
        }
    }
    for a in Action::ALL {
        out.push((format!("town action {:?}", a), a.blurb(), false));
    }
    for d in DUNGEONS {
        for p in d.blurb {
            out.push((format!("{} blurb", d.id), p, true));
        }
        for p in d.landings {
            out.push((format!("{} landing", d.id), p, true));
        }
    }
    out
}

#[test]
fn nothing_is_written_with_a_dash_the_font_cannot_draw() {
    // The bundled font has no glyph for an em or en dash, so one renders as a
    // hole in the middle of a sentence. This is the single most common way a
    // rewrite breaks the screen.
    for (where_, text, _) in scenes() {
        for bad in ['\u{2014}', '\u{2013}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}'] {
            assert!(
                !text.contains(bad),
                "{where_}: contains {bad:?}, which the font cannot draw: {text:?}"
            );
        }
    }
}

#[test]
fn no_scene_withholds_the_noun() {
    // The tell of the register this file exists to prevent. Every one of these
    // was in the prose before it was rewritten, and each is a sentence that
    // gestures at a thing rather than saying what the thing is.
    const HEDGES: &[&str] = &[
        "something is",
        "something else is",
        "you get the impression",
        "you get the strong impression",
        "the strong impression",
        "which is worse",
        "which is somehow worse",
        "in a way that is",
        "the unhurried business",
        "whatever it was going to",
        "not entirely sure",
        "seems to be watching",
        "you feel a",
        "a chill",
        "an air of",
        "there is a sense",
        "somehow both",
        "and yet",
    ];
    for (where_, text, _) in scenes() {
        let low = text.to_lowercase();
        for h in HEDGES {
            assert!(
                !low.contains(h),
                "{where_}: {h:?} is mood standing in for a fact. Say what the thing is.\n  {text}"
            );
        }
    }
}

#[test]
fn every_scene_names_something() {
    // A scene has to be about somebody or somewhere, and the cheap proof of
    // that is a proper noun: Merrik, Gerald, Kettleworks, the Bog Toad, Lord
    // Drabley Henpeck, Kolok Hold-Em. The register this file guards against
    // has none - the old versions of these nine events, between them, named
    // one creature and one lord and nothing else, which is why they read as
    // the same scene told nine times.
    //
    // Checked per scene rather than per paragraph. A middle paragraph is
    // allowed to run on pronouns once the first one has said who is talking,
    // and an earlier draft of this test that demanded a name in every single
    // paragraph ended up with a widening list of exceptions - which is a test
    // being fitted to its data rather than checking it.
    let named = |text: &str| -> bool {
        let mut fresh = true;
        for w in text.split_whitespace() {
            let bare = w.trim_matches(|c: char| !c.is_alphanumeric());
            let opener = std::mem::replace(
                &mut fresh,
                w.ends_with('.') || w.ends_with('!') || w.ends_with('?') || w.ends_with(','),
            );
            if bare.chars().any(|c| c.is_ascii_digit()) {
                return true;
            }
            if !opener && bare.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                return true;
            }
        }
        false
    };

    for e in EVENTS {
        assert!(
            named(&e.prose.join(" ")),
            "{}: three paragraphs and not one name, place, sign or number in them.\n  {:?}",
            e.id,
            e.prose
        );
    }
    for t in TOWNS {
        assert!(named(&t.blurb.join(" ")), "{}: the gate names nothing", t.id);
    }
    for d in DUNGEONS {
        assert!(named(&d.blurb.join(" ")), "{}: the door names nothing", d.id);
        assert!(named(&d.landings.join(" ")), "{}: the landings name nothing", d.id);
    }
}

#[test]
fn the_events_do_not_all_end_the_same_way() {
    // Nine scenes that all close on a short deflating fragment read as one
    // scene nine times, whatever they say in the middle. Measured on the last
    // paragraph of each event, which is where the tic lands.
    let mut endings: Vec<&str> = EVENTS
        .iter()
        .filter_map(|e| e.prose.last().copied())
        .map(|p| {
            // The final sentence, roughly.
            p.rsplit_once(". ").map(|(_, last)| last).unwrap_or(p)
        })
        .collect();
    let n = endings.len();
    assert!(n >= 6, "only {n} events; this proves nothing");

    // No two events may close on the same words.
    endings.sort_unstable();
    let mut dedup = endings.clone();
    dedup.dedup();
    assert_eq!(dedup.len(), n, "two events end on the same sentence");

    // And they must not all be the same shape. A closing fragment under about
    // forty characters is the tic; some are fine, all of them is not.
    let curt = endings.iter().filter(|e| e.len() < 40).count();
    assert!(
        curt * 2 <= n,
        "{curt} of {n} events close on a fragment under forty characters, which is the \
         same beat every time"
    );
}

#[test]
fn a_scene_reads_like_somebody_is_in_it() {
    // The book's scenes have people doing things in them: Merrik with a
    // clipboard, a man counting out loud, a tally man turning a ledger round.
    // A scene with no verb of a person acting is a description of a place.
    const PEOPLE: &[&str] = &[
        " he ", " she ", " they ", " him ", " her ", " them ", "\"", " man ", " woman ",
        " somebody ", " nobody ", " everybody ", " it ", " you ",
    ];
    for e in EVENTS {
        let all = e.prose.join(" ").to_lowercase();
        assert!(
            PEOPLE.iter().any(|p| all.contains(p)),
            "{}: three paragraphs and nobody in them",
            e.id
        );
    }
}

#[test]
fn a_title_is_a_thing_and_not_a_mood() {
    // "A ROOM WITH NO CLOCKS" is a mood. "THE GALAPAGOS EMPORIUM" is a place
    // you can be thrown out of. The proxy: a title has to be short, and it may
    // not open with the hedging article-plus-abstraction shape that reads as
    // atmosphere.
    for e in EVENTS {
        assert!(!e.title.is_empty(), "{}: no title", e.id);
        assert!(
            e.title.len() <= 30,
            "{}: {:?} is a sentence, not a title",
            e.id,
            e.title
        );
        assert_eq!(
            e.title,
            e.title.to_uppercase(),
            "{}: titles are set in capitals",
            e.id
        );
        assert!(
            !e.title.starts_with("SOMETHING"),
            "{}: {:?} is the withheld noun again, in the title",
            e.id,
            e.title
        );
    }
}

#[test]
fn a_shut_door_says_why_in_words_somebody_would_use() {
    use gearmaster_engine::event::Requirement;
    for e in EVENTS {
        for c in e.choices {
            if matches!(c.requires, Requirement::None) {
                continue;
            }
            assert!(
                !c.unmet.is_empty(),
                "{} / {}: shuts without saying why",
                e.id,
                c.label
            );
            assert!(
                c.unmet.len() > 20,
                "{} / {}: {:?} is a label, not a reason",
                e.id,
                c.label,
                c.unmet
            );
        }
    }
}
