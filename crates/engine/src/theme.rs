//! Words, swapped wholesale.
//!
//! # Why this is a layer rather than a rewrite
//!
//! Every name the engine works with - `"Oak Handle"`, `"Cave Rat"` - is a
//! **key**, not a label. Recipes, monster loadouts, quest targets and the whole
//! test suite are string-keyed on those names, and renaming them in place
//! would mean editing all of it at once and hoping. So nothing here changes
//! what anything is *called* in the code; a theme is a lookup from the
//! canonical name to the one on screen.
//!
//! The consequence worth stating: **a theme cannot break the game.** A missing
//! entry falls through to the canonical name, so a half-finished theme is a
//! game with some untranslated words in it rather than a game that does not
//! start. The engine never reads a themed string back.
//!
//! Adding a theme is adding one `Theme` to `THEMES`. Nothing else has to know.

use std::collections::HashMap;
use std::sync::OnceLock;

/// One complete set of words for the game.
pub struct Theme {
    /// Stable identifier, for save data and debug hooks.
    pub id: &'static str,
    /// What the selection screen calls it.
    pub label: &'static str,
    /// One line under the label.
    pub blurb: &'static str,
    /// The opening screen: who you are and what you are doing. One entry per
    /// paragraph.
    pub story: &'static [&'static str],
    /// Canonical component name -> the name to show.
    pub pieces: &'static [(&'static str, &'static str)],
    /// Canonical monster name -> the name to show.
    pub monsters: &'static [(&'static str, &'static str)],
    /// Any other string in the interface, keyed by a short slug. See `word`.
    pub words: &'static [(&'static str, &'static str)],
}

impl Theme {
    /// The themed name for a component, or the canonical one if this theme has
    /// nothing to say about it.
    ///
    /// Takes a `&'static str` because every name in the game is a literal in
    /// `CATALOG` or `LADDER`. That is what lets the fallback simply hand the
    /// key back, with no allocation and no lifetime sleight of hand.
    pub fn piece(&'static self, canonical: &'static str) -> &'static str {
        lookup(self, Table::Pieces, canonical).unwrap_or(canonical)
    }

    /// The same for a creature on the ladder.
    pub fn monster(&'static self, canonical: &'static str) -> &'static str {
        lookup(self, Table::Monsters, canonical).unwrap_or(canonical)
    }

    /// An interface string by slug - "gold", "shop", "mana" and so on. Falls
    /// back to `default`, so a call site always has something to draw and an
    /// unfinished theme shows plain English rather than a slug.
    pub fn word(&'static self, slug: &str, default: &'static str) -> &'static str {
        lookup(self, Table::Words, slug).unwrap_or(default)
    }
}

#[derive(Copy, Clone)]
enum Table {
    Pieces,
    Monsters,
    Words,
}

/// Built once per theme per table. The tables are static and never change, so
/// the maps outlive everything that reads them.
fn lookup(theme: &'static Theme, table: Table, key: &str) -> Option<&'static str> {
    static MAPS: OnceLock<HashMap<(&'static str, usize), HashMap<&'static str, &'static str>>> =
        OnceLock::new();
    let maps = MAPS.get_or_init(|| {
        let mut all = HashMap::new();
        for t in THEMES {
            for (i, pairs) in [t.pieces, t.monsters, t.words].iter().enumerate() {
                all.insert((t.id, i), pairs.iter().copied().collect());
            }
        }
        all
    });
    let i = match table {
        Table::Pieces => 0,
        Table::Monsters => 1,
        Table::Words => 2,
    };
    // Nothing in the table means the caller's own string is the answer, which
    // is what makes a half-written theme safe to ship.
    maps.get(&(theme.id, i)).and_then(|m| m.get(key).copied())
}

/// Every theme the game ships with. The first is the default.
pub static THEMES: &[&Theme] = &[&PLAIN, &TURTLE_DICK];

pub static PLAIN: Theme = Theme {
    id: "plain",
    label: "GEAR MASTER",
    blurb: "The game as it is written.",
    story: &[
        "You are an aspiring Gear Master.",
        "Nobody is born one. The title is given to whoever can take a heap of \
         loose parts and make something out of it that works - and then prove \
         it, against everything on the ladder, all the way up.",
        "You have five frames, a handful of scrap, and twenty gold.",
        "Build.",
    ],
    pieces: &[],
    monsters: &[],
    words: &[],
};

pub static TURTLE_DICK: Theme = Theme {
    id: "td",
    label: "TALES FROM THE CRYPT",
    blurb: "The same game, told in the language of the book. It's about a turtle.",
    story: &[
        "You are a Sprocketman.",
        "Your people were gear-folk of the Great Gear Cave in west Bambulon, \
         until Lord Drabley Henpeck found the Deep Chocolate you had been \
         quietly mining under it. He had the caves cleared and marched you all \
         to the pit the locals now call The End of All Gears.",
        "A Sprocketman's whole craft is making working gear out of loose \
         pieces. That is what the five frames are. Piece by scavenged piece, \
         you build yourself out of the hole.",
        "Above the pit are the planes, and above those is Mount Dobira, and at \
         the top of it is a gambler in a coat made of money who flattens worlds \
         when he loses.",
        "Climb anyway.",
    ],
    pieces: &[],
    monsters: &[
        // The ladder, re-cast from the book. Each is matched to the kit the
        // rung already has, not to its position: the wall bosses get the
        // book's bouncers and wardens, the mind-damage rung gets the riddler
        // who consumed those who could not answer, and the sovereign of vermin
        // gets the Worm who is Death.
        ("Cave Rat", "A. Rat"),
        ("Bog Toad", "Bengulon Jungle Toad"),
        ("Bone Archer", "Wallspider Swarm"),
        ("Rust Golem", "The Crimper"),
        ("Frost Wisp", "Frosty Kev"),
        ("Plague Hound", "The Brumpus"),
        ("The Iron Warden", "Gronkkos the Bouncer"),
        ("Iron Sentinel", "Velothi High Guard"),
        ("Whisperling", "Nesbit the Asker"),
        ("Warded Idol", "Idol of Marbulon"),
        ("Mirror Fiend", "The Yodregar Archive"),
        ("Rust Colossus", "Ponkey Dong"),
        ("Ashen Marshal", "Boucherian Commander"),
        ("Grave Chorus", "The Rice Criers"),
        // Your jailer. Beating him is the end of the first act.
        ("The Hollow King", "Lord Drabley Henpeck"),
        ("Salt Idol", "C O R K"),
        ("Pale Twin", "The Gamer Grandparents"),
        ("Ruin Hound", "Death-Leopard"),
        ("Bone Cantor", "Skeleton Tool Wizard"),
        ("Ember Wisp", "Lxirp Strangler Beast"),
        ("Slag Warden", "Warden of the Centrifuge"),
        ("The Gearwright", "Spike Kaklon"),
        ("Crowned Hollow", "Lord Kumeka of the Eighth Ray"),
        ("Cog Priest", "High Cork Priest"),
        ("Mire Behemoth", "Titan Megalodon"),
        // Death itself, and deliberately not at the top: the book is clear
        // that Francis out-escalates Death.
        ("Vermin Sovereign", "LETO, the Worm"),
        ("Obsidian Colossus", "The Unmovable Rock"),
        ("Null Sentinel", "Warden of Sneel"),
        ("Silence", "The Glacier of Dobira"),
        ("Weeping Idol", "The Weeping Seeker"),
        ("The Long Mirror", "The Perfect Crime"),
        ("Iron Abbot", "Time Order Bishop"),
        ("The Last Gearwright", "Nikka Mista"),
        ("Rimefather", "Emperor of Dobira"),
        ("The Tallow Saint", "Stink Sandwich"),
        ("Hollowmarch", "The Morning Rush"),
        ("The Iron Choir", "The Eight Hymns"),
        ("Gallowglass", "Mumu Lelonde"),
        ("The Rust Parliament", "The Shareholders"),
        ("Sootmother", "Marbulon"),
        ("The Quiet Hour", "The Grand Calculation"),
        ("Verdigris", "The Spreading Cork"),
        ("The Drowned Court", "The Sea of Cleveland"),
        ("Anvilheart", "The Comedian's Anvil"),
        ("The Salt Wedding", "The Jester's Wedding"),
        ("Nine of Ashes", "Nibbalonius the Wise"),
        // The last three read as one story: the final holy beast, the coat
        // made from one, and the man wearing it.
        ("The Last Light", "The Last Wimpler Oxen"),
        ("Gilt", "The Money Coat"),
        ("Francis", "Francis the Gambler"),
    ],
    words: &[],
};

/// The theme with this id, or the default.
pub fn by_id(id: &str) -> &'static Theme {
    THEMES.iter().copied().find(|t| t.id == id).unwrap_or(THEMES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A theme is a lookup with a fallback, so an entry it has never heard of
    /// comes back unchanged. This is the property that lets a theme be filled
    /// in one piece at a time without ever breaking the game.
    #[test]
    fn an_unthemed_name_falls_through_unchanged() {
        for t in THEMES {
            assert_eq!(t.piece("Oak Handle"), t.pieces.iter()
                .find(|(k, _)| *k == "Oak Handle")
                .map(|(_, v)| *v)
                .unwrap_or("Oak Handle"));
            assert_eq!(t.monster("A Creature That Does Not Exist"),
                       "A Creature That Does Not Exist");
        }
    }

    /// Ids have to be unique: they key the lookup tables and identify a theme
    /// in save data.
    #[test]
    fn theme_ids_are_distinct() {
        let mut seen = Vec::new();
        for t in THEMES {
            assert!(!seen.contains(&t.id), "two themes both call themselves {}", t.id);
            seen.push(t.id);
        }
    }

    /// Every theme owes the player an opening. A theme with no story would
    /// drop them onto the board with no idea what they are doing there.
    #[test]
    fn every_theme_tells_you_who_you_are() {
        for t in THEMES {
            assert!(!t.story.is_empty(), "{} has no opening", t.id);
            assert!(!t.label.is_empty() && !t.blurb.is_empty(), "{} is unlabelled", t.id);
        }
    }

    /// A theme names creatures by their canonical name, so a typo in the
    /// table is a rung that quietly keeps its old name. This catches that.
    #[test]
    fn every_themed_monster_names_a_real_one() {
        use crate::combat::LADDER;
        for t in THEMES {
            for (canonical, themed) in t.monsters {
                assert!(
                    LADDER.iter().any(|m| m.name == *canonical),
                    "{} renames {:?} -> {:?}, but no such creature is on the ladder",
                    t.id,
                    canonical,
                    themed
                );
            }
        }
    }

    /// And the other direction: a theme that claims to re-tell the ladder
    /// should not leave half of it in the old words.
    #[test]
    fn the_turtle_theme_renames_the_whole_ladder() {
        use crate::combat::LADDER;
        let missed: Vec<&str> = LADDER
            .iter()
            .map(|m| m.name)
            .filter(|n| TURTLE_DICK.monster(n) == *n)
            .collect();
        assert!(missed.is_empty(), "still in plain words: {:?}", missed);
    }

    /// Two creatures sharing a themed name would be two rungs the player
    /// cannot tell apart.
    #[test]
    fn no_two_creatures_get_the_same_new_name() {
        for t in THEMES {
            let mut seen: Vec<&str> = Vec::new();
            for (_, themed) in t.monsters {
                assert!(!seen.contains(themed), "{} uses {:?} twice", t.id, themed);
                seen.push(themed);
            }
        }
    }

    #[test]
    fn an_unknown_id_falls_back_to_the_default() {
        assert_eq!(by_id("nonsense").id, THEMES[0].id);
        assert_eq!(by_id("td").id, "td");
    }
}
