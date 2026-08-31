//! Every net on the shelf still loads, and the ones that no longer can are named.
//!
//! A checkpoint is keyed to the feature vector of the day it was written, and
//! this repo widens that vector. `BOARD` went from 30 to 270 in `cf91f65`, so a
//! packing pair went from 70 to 315; the road's move description gained six
//! numbers in `f429dae`, so a road pair is 64. **Every file in
//! `analysis/nets/` matches neither**, and nothing said so: `QNet::load` hands
//! back an `Option`, so `qmind` reported on four nets it had not loaded,
//! `Packer::Learned(None)` was the floor wearing a trained net's name, and
//! every road figure in `analysis/the-two-trades.md` R6 quietly became a number
//! nobody could reproduce.
//!
//! Two claims, then, and the second is the ledger:
//!
//! * anything on the shelf **reads** as a net, because a file that is not even
//!   well formed is a different and worse problem;
//! * anything that no longer matches either agent's pair is **on the list
//!   below**, and the list may only get shorter.
//!
//! The list is not an exemption. It is nine measurements that cannot currently
//! be repeated, held where somebody has to look at them, and the way to take a
//! line off it is to retrain the net and delete the file - not to widen the
//! test.

use gearmaster_trades::feature;
use gearmaster_trades::pathfinder;
use gearmaster_trades::QNet;

const SHELF: &str = "../../analysis/nets";

/// Checkpoints saved against a feature vector that no longer exists.
///
/// **This list may only get shorter - and when it does not, the commit says
/// which measurement it just cost.** Each of these is a net some measurement was
/// read off, and none of them can be fed today: seven are 70 wide against a packing pair of 315, and the two road
/// nets are 70 wide because `qroad` stores a road net at the *packing* width -
/// which is why trainers stamp `pair` now, and why these nine, written before
/// the stamp, cannot say which road columns they read.
const STALE: &[&str] = &[
    // **These two were added, and that is the cost of a feature change.**
    // `analysis/the-collapse.md` M1 widened `feature::MOVE` from 32 to 38 so
    // that `Lock`, `Grow`, `Undo` and `Pin` stop being one identical vector,
    // and a widened pair invalidates every net trained against the old one.
    // What can no longer be repeated is M0.2 and M0.3 - the provenance of r12's
    // two checkpoints and the reconstruction of its column out of a held-still
    // policy. Both are written down with their numbers; neither can be re-run
    // until something is retrained at 321.
    "qrow-r12-best.txt",
    "qrow-r12-last.txt",
    "pathfinder-grinder.txt",
    "pathfinder-rogue.txt",
    "pathfinder-threshold.txt",
    "pathfinder-unwound.txt",
    "quartermaster-briefed.txt",
    "quartermaster-phi15.txt",
    "quartermaster-unconditioned.txt",
    "quartermaster_grinder.txt",
    "quartermaster_rogue.txt",
];

/// Every `.txt` on the shelf, sorted, so a failure names files in one order.
fn shelf() -> Vec<(String, std::path::PathBuf)> {
    let mut out: Vec<_> = std::fs::read_dir(SHELF)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "txt"))
                .map(|p| (p.file_name().expect("a file").to_string_lossy().into_owned(), p))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Whether either agent could feed this file today, and why not when it cannot.
fn usable(path: &std::path::Path) -> Result<usize, String> {
    let p = path.to_str().expect("utf-8");
    match (QNet::load_at(p, feature::PAIR), QNet::load_at(p, pathfinder::PAIR)) {
        (Ok(_), _) => Ok(feature::PAIR),
        (_, Ok(_)) => Ok(pathfinder::PAIR),
        (Err(pack), Err(_)) => Err(pack),
    }
}

#[test]
fn every_checkpoint_on_the_shelf_reads_as_a_net() {
    for (name, path) in shelf() {
        let text = std::fs::read_to_string(&path).expect("a file the directory listed");
        if let Err(why) = QNet::read(&text) {
            panic!("{name} is not a net: {why}");
        }
    }
}

#[test]
fn only_the_nets_on_the_list_are_ones_nobody_can_feed() {
    let mut news: Vec<String> = Vec::new();
    for (name, path) in shelf() {
        if let Err(why) = usable(&path) {
            if !STALE.contains(&name.as_str()) {
                news.push(why);
            }
        }
    }
    assert!(
        news.is_empty(),
        "checkpoints saved against a feature vector that no longer exists, and \
         not on the list in this file:\n{}\n\n\
         A widening of `feature::PAIR` or `pathfinder::PAIR` invalidates every \
         net on the shelf. Retrain them, or add them to STALE and say in the \
         commit which measurement can no longer be repeated.",
        news.join("\n")
    );
}

/// And the other direction: a name comes off the list by being retrained.
#[test]
fn nothing_on_the_list_has_quietly_started_working_again() {
    let mut back: Vec<String> = Vec::new();
    for name in STALE {
        let path = std::path::Path::new(SHELF).join(name);
        if !path.exists() {
            back.push(format!("{name} is gone - take it off the list"));
        } else if let Ok(w) = usable(&path) {
            back.push(format!("{name} loads at {w} now - take it off the list"));
        }
    }
    assert!(
        back.is_empty(),
        "the list of unfeedable nets is out of date:\n{}\n\nIt may only get shorter.",
        back.join("\n")
    );
}

/// The two agents are not the same width, and that is what `q_pair` is for.
///
/// A road pair is fed to a net stored at the packing width, so the arithmetic
/// runs over the *net's* rows and takes the rest as zero. If these two ever
/// became equal, a road net and a packing net would be indistinguishable on
/// disk and `usable` above would stop being able to say which is which.
#[test]
fn a_road_pair_is_narrower_than_a_packing_one() {
    assert!(
        pathfinder::PAIR < feature::PAIR,
        "a road pair is {} and a packing pair is {}",
        pathfinder::PAIR,
        feature::PAIR
    );
}

/// A stamped net is refused by width, not merely by shape.
///
/// The fault this file exists for was a well-formed file that no caller could
/// use, so the thing worth pinning is that the *refusal* happens - and that it
/// happens on what the trainer said it fed the net rather than on how many
/// numbers ended up in the file.
#[test]
fn a_stamp_is_what_a_loader_checks() {
    let hidden = 2usize;
    let wide = 4usize;
    let body = format!(
        "w1{}\nb1 0 0\nw2 0 0 0 0\nb2 0 0\nw3 0 0\nb3 0\n",
        " 0".repeat(wide * hidden)
    );
    let unstamped = QNet::read(&body).expect("a well-formed net");
    assert_eq!(unstamped.width(), wide, "the width is read out of w1");
    assert_eq!(unstamped.declared(), None, "and nothing was stamped");

    let stamped = QNet::read(&format!("pair 3\n{body}")).expect("a well-formed net");
    assert_eq!(stamped.width(), wide, "the file is still four rows wide");
    assert_eq!(stamped.declared(), Some(3), "and it says three of them mean anything");

    assert!(
        QNet::read(&format!("pair 9\n{body}")).is_err(),
        "a stamp larger than the weights is a file that cannot be believed"
    );
}
