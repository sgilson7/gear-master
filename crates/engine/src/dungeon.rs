//! Mini dungeons: a short chain of fights off the side of the road.
//!
//! A dungeon is a few floors, fought in order, that ends in a class you cannot
//! get anywhere else. It does not advance the ladder - when you come out you
//! are standing exactly where you went in, in front of the fight you had not
//! got to yet.
//!
//! Adding one is adding an entry to `DUNGEONS` and the alternates its floors
//! name. Nothing else has to know.

/// One short chain of fights, and what finishing it is worth.
#[derive(Copy, Clone, Debug)]
pub struct Dungeon {
    pub id: &'static str,
    /// What the door calls itself.
    pub name: &'static str,
    /// Shown on the way in - the door, and what is behind it.
    pub blurb: &'static [&'static str],
    /// One or two lines played as a cutscene the moment you step through.
    ///
    /// Not the same thing as `blurb`, and the difference is where you are
    /// standing. The blurb is read at the door while it is still a decision;
    /// this is said once the decision is made, on the same machinery a boss
    /// uses, and it is how you know you have gone somewhere. A dungeon you can
    /// walk into without noticing is a dungeon nobody knows they are in.
    pub entry: &'static [&'static str],
    /// Floors in order, each naming an alternate creature.
    pub floors: &'static [&'static str],
    /// Said between floors, one per floor cleared. The last is the ending.
    pub landings: &'static [&'static str],
    /// The class only this dungeon hands out.
    pub reward: &'static str,
    /// Anything else clearing it does.
    ///
    /// A class is one kind of reward and the road has several. THE THRESHOLD's
    /// prize is a *pool*, which no `ClassDef` can say - so a dungeon carries a
    /// list of outcomes as well, applied on the way out, and the receipt says
    /// what they were.
    pub also: &'static [crate::event::Outcome],
}

pub const DUNGEONS: &[Dungeon] = &[
    // Bunko's Cavern, pp. 84-85: a fishing hamlet swallowed by the Holy Cork
    // Empire and renamed Corrqk's Cavern, its Home for Immature Men turned
    // into a Drambus seed facility. Boyetano works it, prays to the old gods,
    // and one evening notices a purple glint between the Cork and the
    // Unmovable Rock. He reaches the Core, gazes on a piece of the Mansus, and
    // ascends - then splits the wisdom into pieces to be handed to the boys he
    // has left.
    Dungeon {
        id: "the-crevice",
        name: "THE CREVICE IN THE ROCK",
        blurb: &[
            "The thing you sold turns up three rungs later in the hands of \
             somebody who should not have it, in a hamlet that is not on any \
             map you have seen.",
            "They call it Corrqk's Cavern now. It was Bunko's Cavern when it \
             was a fishing village, before the Cork came and the boys were \
             put on trains and the Home for Immature Men was turned into a \
             Drambus seed facility. There is one old analyst left on the \
             line. His name is Boyetano and he still prays to the old gods, \
             on a floor that cuts his knees, which he says helps him \
             concentrate.",
            "Boyetano has noticed a purple glint down between the Cork and \
             the Unmovable Rock. He has been noticing it for six years and \
             has told nobody, because nobody who works here has the shoulders \
             to widen a crack in a rock, and he has been very patient about \
             waiting for somebody who does.",
        ],
        entry: &[
            "The hole in the back wall is a hole in the back wall for about \
             four feet, and then it is a staircase somebody cut, and then it \
             is not a staircase.",
            "Boyetano is already ahead of you. He has been ahead of you for \
             six years.",
        ],
        floors: &["The Reciter", "The Long Haul", "The Watchers"],
        landings: &[
            "The Anticipations stop mid-verse. Behind the pulpit, the Cork \
             has grown out over a crack in the rock the way a lip grows over \
             a bad tooth. Boyetano gets a bar under it. Boyetano is seventy- \
             one.",
            "The train goes over on the bend. Whatever was in the cars is out \
             in the dark now, and it does not appear to want anything from \
             you at all, and it does not appear to want anything from \
             Boyetano either, who keeps walking and does not look at it once.",
            "The Core is soup and light with a piece of somewhere else \
             sitting in the middle of it. Boyetano looks at it for a while, \
             and stops being Boyetano, and there is a moment there where he \
             could have kept the lot. He splits it instead, the way he always \
             said he would, and puts your share in your hand on his way past. \
             Somewhere above you, for the first time in a long time, somebody \
             is casting a line.",
        ],
        reward: "Ascendant",
        also: &[],
    },
    // The Mansus antechamber, behind a cellar door in a house that was not on
    // the road until somebody told you about it. Three floors of wardens, and
    // what you come out with is a sense you did not have going in.
    Dungeon {
        id: "the-threshold",
        name: "THE THRESHOLD",
        blurb: &[
            "The man behind the cellar door has been talking for a long time \
             and is not talking to you. He is describing a staircase. He is \
             describing it very accurately.",
            "Behind the door there is a staircase.",
            "Nobody in the Manse comes down here, and everybody in the Manse \
             knows exactly how many steps there are, which is the sort of \
             thing that stops being strange about four minutes after you \
             notice it.",
        ],
        entry: &[
            "The door was not locked. Doors like this never are.",
            "Behind you it is a cellar. Ahead of you it is not, and the \
             difference happened somewhere in the middle without a line.",
        ],
        floors: &["DOORKEEP", "THE STAIR THAT LISTENS", "THE LAST LANDING"],
        landings: &[
            "It stands aside. It was always going to stand aside. What it was \
             doing was making sure you went down rather than in.",
            "The stair has been counting. Not the steps - there are 402 steps \
             and it has known that since before there were steps - it has been \
             counting *you*, and the number it has reached is one.",
            "There is light at the bottom and the light is a person called \
             Ghirbi, and Ghirbi is pleased to see you, which is the worst of \
             it. You come back up seeing with the wrong sense, and it does \
             not stop.",
        ],
        reward: "Threshold-Sighted",
        also: &[
            crate::event::Outcome::UnlockInsight,
            crate::event::Outcome::Flag("threshold-cleared"),
        ],
    },
];

pub fn by_id(id: &str) -> Option<&'static Dungeon> {
    DUNGEONS.iter().find(|d| d.id == id)
}

/// Classes that exist only at the end of a dungeon.
pub fn is_dungeon_only(class: &str) -> bool {
    DUNGEONS.iter().any(|d| d.reward == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_floor_names_a_creature_that_exists() {
        for d in DUNGEONS {
            assert!(!d.floors.is_empty(), "{} has no floors", d.id);
            for f in d.floors {
                assert!(
                    crate::combat::alternate(f).is_some(),
                    "{}: no such creature as {}",
                    d.id,
                    f
                );
            }
            assert_eq!(
                d.landings.len(),
                d.floors.len(),
                "{}: one landing per floor, the last being the ending",
                d.id
            );
        }
    }

    #[test]
    fn every_reward_is_a_real_class_and_only_from_here() {
        for d in DUNGEONS {
            let c = crate::class::CLASSES
                .iter()
                .find(|c| c.name == d.reward)
                .unwrap_or_else(|| panic!("{} rewards {}, which is not a class", d.id, d.reward));
            // A dungeon class must not also be something a fountain can pour,
            // or the dungeon is not the only way to it.
            assert!(
                c.requires.is_empty(),
                "{} is a dungeon reward but also has axis requirements, so a \
                 fountain could hand it over",
                c.name
            );
        }
    }

    /// You always know you are inside one.
    ///
    /// A door that hands you three fights and says nothing is a door you can
    /// walk through by accident, and a fight you did not know you had chosen
    /// is the one kind of fight this game should never hand out.
    #[test]
    fn every_dungeon_says_something_the_moment_you_are_in_it() {
        for d in DUNGEONS {
            assert!(!d.entry.is_empty(), "{} lets you in without a word", d.id);
            for line in d.entry {
                assert!(line.len() > 20, "{}: an entry line worth reading", d.id);
            }
            assert!(d.entry.len() <= 3, "{}: an entry is a line or two, not a scene", d.id);
        }
    }

    #[test]
    fn no_two_dungeons_share_an_id_or_a_reward() {
        for (i, a) in DUNGEONS.iter().enumerate() {
            for b in &DUNGEONS[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.reward, b.reward);
            }
        }
    }
}
