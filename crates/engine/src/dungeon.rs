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
    /// Shown on the way in.
    pub blurb: &'static [&'static str],
    /// Floors in order, each naming an alternate creature.
    pub floors: &'static [&'static str],
    /// Said between floors, one per floor cleared. The last is the ending.
    pub landings: &'static [&'static str],
    /// The class only this dungeon hands out.
    pub reward: &'static str,
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
            "The thing you sold turns up again three rungs later, in the hands of \
             somebody who should not have it, in a hamlet that is not on any map \
             you have seen.",
            "They call it Corrqk's Cavern now. It was Bunko's Cavern when it was \
             a fishing village, before the Cork came and the boys were put on \
             trains, and there is one old analyst left in the seed facility who \
             still prays to the old gods on a floor that cuts his knees.",
            "He has noticed a purple glint between the Cork and the Unmovable \
             Rock. He has been waiting for somebody with the shoulders to widen \
             it.",
        ],
        floors: &["The Reciter", "The Long Haul", "The Watchers"],
        landings: &[
            "The Anticipations stop mid-verse. Behind the pulpit the Cork has \
             been growing over a crack, the way a lip grows over a bad tooth.",
            "The train goes over. Whatever was in the cars is loose in the dark \
             now and does not seem to want anything from you, which is worse.",
            "The Core is soup and light and something in the middle of it that \
             is a piece of somewhere else. Boyetano looks at it, and stops \
             being Boyetano, and there is a moment where he could keep all of \
             it. He splits it instead, the way he always said he would, and \
             hands you a share on his way out. Somewhere above, fishing rods \
             are being cast.",
        ],
        reward: "Ascendant",
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
