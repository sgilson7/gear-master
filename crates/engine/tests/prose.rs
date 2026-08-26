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

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::EVENTS;
use gearmaster_engine::run::Mode;
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
    // The two lines under the headings on the setup screen. They are the only
    // prose in the game a player reads before the road starts, and until this
    // file could see them they were the only prose nothing checked - which is
    // how both of them came to be knowing epigrams restating the cards
    // underneath. Neither is a paragraph: a line on a screen is allowed to be
    // plain, and these two are supposed to be.
    out.push(("mode screen subtitle".into(), Mode::WHAT_THE_CHOICE_IS, false));
    out.push(("difficulty screen subtitle".into(), Difficulty::WHAT_THE_CHOICE_IS, false));
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
        // The setup screen's own register, which is not the scenes' register
        // and went unchecked for as long as this file could not see it. Both
        // of these are the game standing outside itself and passing comment:
        // "Medium is the fight the game was built around", "It just does not
        // get you past the thing that beat you". A game that names itself in
        // copy a player reads has stopped being the thing they are in.
        "the game",
        "it just does not",
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

/// A subtitle says what its screen is asking. It does not grade the cards.
///
/// Both of the two the setup screen ships failed this, and neither was
/// checkable until `scenes()` could reach them. "Bigger numbers mean tougher,
/// meaner monsters. Medium is the fight the game was built around" singles out
/// an option standing directly underneath it - in a card that already says
/// "the intended fight" on its own face - so the subtitle is spending its one
/// line saying a thing the screen was going to say anyway.
///
/// The proxy: a subtitle may not name any of the options it sits above. It is
/// cheap and it is not literary judgement, but a line that has to single one
/// out is a line doing the cards' job instead of the heading's.
#[test]
fn a_subtitle_does_not_name_the_options_under_it() {
    let screens: [(&str, &str, Vec<&str>); 2] = [
        (
            "the mode screen",
            Mode::WHAT_THE_CHOICE_IS,
            vec![Mode::Grinder.name(), Mode::Rogue.name()],
        ),
        (
            "the difficulty screen",
            Difficulty::WHAT_THE_CHOICE_IS,
            Difficulty::ALL.iter().map(|d| d.name()).collect(),
        ),
    ];
    for (screen, subtitle, options) in screens {
        assert!(!subtitle.is_empty(), "{screen}: no line under the heading");
        let low = subtitle.to_lowercase();
        for o in options {
            assert!(
                !low.contains(&o.to_lowercase()),
                "{screen}: the line under the heading names {o}, which is a card \
                 directly below it.\n  {subtitle}"
            );
        }
    }
}

/// Does this text contain a proper noun?
///
/// The cheap proof that a scene is about somebody or somewhere: Merrik,
/// Gerald, Kettleworks, the Bog Toad, EGGBERT on a brass plate. The register
/// this file guards against has none - the old versions of nine of these
/// events, between them, named one creature and one lord and nothing else,
/// which is why they read as the same scene told nine times.
///
/// `on_a_digit` is the loophole, kept as a parameter so the two tests below
/// can measure it. A number satisfied the original test as cheaply as a name
/// did, and M14 and M15 duly bolted numbers onto the scenes that had no names
/// in them - "rice for the trade board for 19 years", "the 3 chairs" - which
/// left the lint green and the scenes exactly as anonymous as they were.
///
/// Checked per scene rather than per paragraph. A middle paragraph is allowed
/// to run on pronouns once the first one has said who is talking, and an
/// earlier draft that demanded a name in every single paragraph ended up with
/// a widening list of exceptions - which is a test being fitted to its data
/// rather than checking it.
fn names_something(text: &str, on_a_digit: bool) -> bool {
    let mut fresh = true;
    for w in text.split_whitespace() {
        let bare = w.trim_matches(|c: char| !c.is_alphanumeric());
        let opener = std::mem::replace(
            &mut fresh,
            w.ends_with('.') || w.ends_with('!') || w.ends_with('?') || w.ends_with(','),
        );
        if on_a_digit && bare.chars().any(|c| c.is_ascii_digit()) {
            return true;
        }
        // "I" is a capital in the middle of a sentence and it is nobody.
        if !opener && bare != "I" && bare.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            return true;
        }
    }
    false
}

/// Every scene, with the id to blame, for the two tests that walk them.
fn every_scene() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for e in EVENTS {
        out.push((format!("event {}", e.id), e.prose.join(" ")));
    }
    for t in TOWNS {
        out.push((format!("town {}", t.id), t.blurb.join(" ")));
    }
    for d in DUNGEONS {
        out.push((format!("dungeon {} blurb", d.id), d.blurb.join(" ")));
        out.push((format!("dungeon {} landings", d.id), d.landings.join(" ")));
    }
    out
}

#[test]
fn every_scene_names_something() {
    for (where_, text) in every_scene() {
        assert!(
            names_something(&text, true),
            "{where_}: not one name, place, sign or number in it.\n  {text}"
        );
    }
}

// ----------------------------------------------- and the ratchet under it
//
// The test above is the one that has always been here, and it is satisfied by
// a digit. These three are the same question asked without the loophole, shipped
// as a budget the way `catalog_shape` and `two_voices` are: it goes down, or it
// does not move.
//
// Eighteen, and every one of them is a scene that calls its people by their job
// - "the tally man", "a woman with a clipboard", "The buyer" - and satisfies
// the lint on a number bolted on for exactly that purpose. The number is the
// symptom. The fix is a name.

/// How many scenes still name nothing but a figure. It goes down.
///
/// 18 at `a46294a`. 15 once P4's second batch named the Hollow King on the
/// floor of his own counting house, Nance Twiss on her folding stool, and
/// Salter behind the bar of the inn with the long table - three scenes that
/// had been standing on "All 3 copies", "19 years" and "40 years".
///
/// 10 once the third batch named Braddock at the milestone and again at the
/// payout table, Vell in the hired room, Dorn and Ilder at the ring, and Sarn
/// reading the reserve out - which took "the 4th one down", "the 3 chairs",
/// "6 years each" and "1 lot a month" with them.
///
/// A note for whoever lowers this next: the proxy cannot see a name that only
/// ever *opens* a sentence, because at a sentence start it cannot tell "Vell"
/// from "The". THE BUYER named its man twice and failed anyway. The answer is
/// to write the name into the middle of a sentence somewhere, which is better
/// prose in any case - not to widen the proxy, which would mean keeping a list
/// of the cast in a test file and fitting the test to its data.
const DIGIT_PROPS: usize = 10;

fn leaning_on_a_number() -> Vec<String> {
    every_scene()
        .into_iter()
        .filter(|(_, text)| !names_something(text, false))
        .map(|(where_, _)| where_)
        .collect()
}

#[test]
fn no_more_scenes_lean_on_a_number_than_already_did() {
    let found = leaning_on_a_number();
    assert!(
        found.len() <= DIGIT_PROPS,
        "{} scenes name nothing but a figure, over a budget of {}:\n{:#?}",
        found.len(),
        DIGIT_PROPS,
        found
    );
}

#[test]
fn the_digit_budget_is_not_slack() {
    let found = leaning_on_a_number();
    assert_eq!(
        found.len(),
        DIGIT_PROPS,
        "the list shrank to {} - lower DIGIT_PROPS in the commit that earned it",
        found.len()
    );
}

/// The target: every scene names somebody or somewhere, and no scene needs a
/// number to prove it is about anything.
#[test]
#[ignore]
fn every_scene_names_a_person_or_a_place() {
    assert_eq!(leaning_on_a_number(), Vec::<String>::new());
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

// ------------------------------------------------------------- the printer
//
// Every lint in this file is a cheap mechanical proxy and the file says so at
// the top. The thing none of them can do is tell you whether a scene reads,
// and the only way to find that out is to read it - in the order a player
// meets it, with the choices under it, the way the screen has it.
//
//   cargo test -p gearmaster-engine --test prose -- --ignored --nocapture read
//
// Ignored, like the printers in `baseline`: it asserts nothing and it is not
// part of the suite. It is here because four bugs in the last mission survived
// a fully green suite, and every one of them was a thing no test was looking
// at.

/// The whole road, in the order it is walked, out loud.
#[test]
#[ignore]
fn read_the_road_aloud() {
    let mut stops: Vec<(usize, String)> = Vec::new();

    for e in EVENTS {
        let mut out = format!("\n{}  [{}]  {}\n", e.title, e.id, e.where_it_stands());
        for p in e.prose {
            out.push_str(&format!("\n    {}\n", wrapped(p)));
        }
        for c in e.choices {
            out.push_str(&format!("\n  > {}\n      {}\n", c.label, wrapped(c.blurb)));
            if !c.unmet.is_empty() {
                out.push_str(&format!("      (shut) {}\n", wrapped(c.unmet)));
            }
        }
        stops.push((e.at, out));
    }
    for t in TOWNS {
        let mut out = format!("\n{}  [town, after rung {}]\n", t.name, t.after + 1);
        for p in t.blurb {
            out.push_str(&format!("\n    {}\n", wrapped(p)));
        }
        for a in t.actions {
            out.push_str(&format!("\n  > {}\n      {}\n", a.name(), wrapped(a.blurb())));
        }
        stops.push((t.after, out));
    }
    for d in DUNGEONS {
        let mut out = format!("\n{}  [dungeon, {}]\n", d.name, d.id);
        for p in d.blurb.iter().chain(d.entry) {
            out.push_str(&format!("\n    {}\n", wrapped(p)));
        }
        for (f, l) in d.floors.iter().zip(d.landings) {
            out.push_str(&format!("\n  -- {} --\n    {}\n", f, wrapped(l)));
        }
        // A dungeon stands beside the road rather than on it; printed last so
        // the rung order above stays the walk.
        stops.push((usize::MAX, out));
    }

    stops.sort_by_key(|(at, _)| *at);
    println!("\n================ THE ROAD, IN ORDER ================");
    for (_, text) in stops {
        println!("{}", text);
    }
}

/// Hard-wrapped the way the screen wraps it, so a paragraph reads as a shape
/// rather than as one line off the side of a terminal.
fn wrapped(text: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for w in text.split_whitespace() {
        if col > 0 && col + 1 + w.len() > 72 {
            out.push_str("\n    ");
            col = 0;
        } else if col > 0 {
            out.push(' ');
            col += 1;
        }
        out.push_str(w);
        col += w.len();
    }
    out
}
