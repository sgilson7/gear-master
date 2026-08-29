//! What this agent has met, across runs.
//!
//! Cross-episode **memory**, which is learning and is fair (`§2` of the spec):
//! a person who plays a seed twenty times knows its shops, and a person who
//! has met a door knows what it asks. Cross-episode **peeking** - reading the
//! tables to find out what exists - is not, and this cannot do it: everything
//! below is put here by a run that stood in front of the thing.
//!
//! That distinction is the whole reason the ledger can say anything. A count
//! of doors read out of `EVENTS` would be a census; a count of doors a run has
//! actually been offered is coverage.

use std::collections::{BTreeMap, BTreeSet};

/// Everything the agent has been offered and everything it has taken.
#[derive(Clone, Debug, Default)]
pub struct Seen {
    /// Door id -> the rungs it was offered on.
    pub doors_offered: BTreeMap<String, BTreeSet<usize>>,
    /// Door id -> how many times each of its choices has been taken.
    ///
    /// **Counts, not a set.** With a set, the first run to take a branch
    /// closes it for every later run, so a sweep is diverse and each run in it
    /// is monotonous - and content that needs *repetition* to reach, like a
    /// county trip out of a town gate, is visited once and never again. The
    /// dial picks the least-taken branch, which keeps coming back.
    pub choices_taken: BTreeMap<String, BTreeMap<usize, usize>>,
    /// Door id -> how many of its choices were open the last time it stood.
    pub choices_open: BTreeMap<String, BTreeSet<usize>>,
    /// Town name -> how many times each of its doors has been gone through.
    pub town_doors: BTreeMap<String, BTreeMap<String, usize>>,
    /// Town names whose gate was reached at all.
    pub gates: BTreeSet<String>,
    /// Dungeon id -> floors stood on.
    pub floors: BTreeMap<String, BTreeSet<usize>>,
    /// Choice labels that a shut choice somewhere asked for, by name.
    ///
    /// A greyed choice says what it wants, in the plain column under it -
    /// *"Requires: having chosen \"TAKE THE DEAL\" earlier"* - and the
    /// interface draws that sentence. So a pilot can read which **earlier
    /// choice** a door it could not open was waiting on, and take that one
    /// when it next meets it.
    ///
    /// That is the whole of how a run gets into the crevice: its door's second
    /// branch wants a deal taken two doors back, and nothing about the deal
    /// says so at the time. A person learns it by being refused once.
    pub wanted_labels: BTreeSet<String>,
    /// `(door id, choice)` pairs that put the run inside a dungeon, and which
    /// one.
    ///
    /// **The only way a run gets into a dungeon is by being put there** - by a
    /// door's answer, a town's cellar, or an orb (`console/src/verb.rs`, which
    /// says why there is no `Enter` verb). So a pilot that wants to see the
    /// inside of one cannot look for a way in; it has to remember where a
    /// choice took it. A person who answers a door and finds themselves down a
    /// mine remembers the same thing, and that is what makes this learning
    /// rather than a table.
    pub doors_into: BTreeMap<(String, usize), String>,
    /// Classes drunk.
    pub classes: BTreeSet<String>,
    /// County tiles stood on, by reference.
    pub county_tiles: BTreeSet<String>,
    /// The highest rung any run has stood on.
    pub deepest_rung: usize,
    /// Rung -> how many runs stood on it.
    ///
    /// The difference between "no run got that deep" and "runs were there and
    /// the door did not appear" - and without it every door past the wall
    /// reads as the second, which would turn a ceiling into twenty-six false
    /// findings.
    pub rungs_stood: BTreeMap<usize, usize>,
    /// Brawls walked into.
    pub brawls: usize,
    /// Runs played.
    pub runs: usize,
}

impl Seen {
    /// How many times this branch has been taken.
    pub fn times(&self, door: &str, choice: usize) -> usize {
        self.choices_taken.get(door).and_then(|m| m.get(&choice)).copied().unwrap_or(0)
    }

    /// Has some shut choice, somewhere, asked for this one by name?
    pub fn is_wanted(&self, label: &str) -> bool {
        self.wanted_labels.contains(label)
    }

    /// Which dungeon this choice leads into, if a run has found out.
    pub fn leads_into(&self, door: &str, choice: usize) -> Option<&str> {
        self.doors_into.get(&(door.to_string(), choice)).map(|s| s.as_str())
    }

    /// Has every floor of this dungeon been stood on?
    pub fn walked_out(&self, dungeon: &str, floors: usize) -> bool {
        self.floors.get(dungeon).is_some_and(|f| f.len() >= floors)
    }

    /// How many times this town's door has been gone through.
    pub fn town_times(&self, town: &str, door: &str) -> usize {
        self.town_doors.get(town).and_then(|m| m.get(door)).copied().unwrap_or(0)
    }

    /// How many distinct doors have been stood in front of.
    pub fn doors(&self) -> usize {
        self.doors_offered.len()
    }

    /// How many distinct branches have been taken.
    pub fn branches(&self) -> usize {
        self.choices_taken.values().map(|m| m.len()).sum()
    }
}
