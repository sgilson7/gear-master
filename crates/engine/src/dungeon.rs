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
    /// The class only this dungeon hands out, or empty for one that pays
    /// something that is not a class.
    ///
    /// Two of the four do. The Undertow pays a row on a board of your choice
    /// and DEN RIVALS pays a hide, and neither of those is a thing a
    /// `ClassDef` can say - which is what `also` is for.
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
            "It has a new name now. It had an older one when it was a fishing \
             village, before the company came and the boys were put on trains \
             and the Home for Immature Men was turned into a seed facility. \
             There is one old analyst left on the line. He still prays to the \
             old gods, on a floor that cuts his knees, which he says helps him \
             concentrate.",
            "He has noticed a purple glint down between the shell and the \
             rock that will not move. He has been noticing it for 6 years and \
             has told nobody, because nobody who works here has the shoulders \
             to widen a crack in a rock, and he has been very patient about \
             waiting for somebody who does.",
        ],
        entry: &[
            "The hole in the back wall is a hole in the back wall for about \
             four feet, and then it is a staircase somebody cut, and then it \
             is not a staircase.",
            "The old man is already ahead of you. He has been ahead of you \
             for 6 years.",
        ],
        floors: &["The Reciter", "The Long Haul", "The Watchers"],
        landings: &[
            "The recitation stops mid-verse. Behind the pulpit, the shell has \
             grown out over a crack in the rock the way a lip grows over a bad \
             tooth. The old man gets a bar under it. The old man is 71.",
            "The train goes over on the bend. Whatever was in the cars is out \
             in the dark now, and it does not appear to want anything from \
             you at all, and it does not appear to want anything from the old \
             man either, who keeps walking and does not look at it once.",
            "The Core is soup and light with a piece of somewhere else \
             sitting in the middle of it. The old man looks at it for a while, \
             and stops being an old man, and there is a moment there where he \
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
            "There is light at the bottom and the light is a person, or was, \
             and it is pleased to see you, which is the worst of it. You come \
             back up seeing with the wrong sense, and it does not stop.",
        ],
        reward: "Threshold-Sighted",
        also: &[
            crate::event::Outcome::UnlockInsight,
            crate::event::Outcome::Flag("threshold-cleared"),
        ],
    },
    // THE UNDER-MINE, under the seam the Sprocketmen were told was empty.
    Dungeon {
        id: "the-under-mine",
        name: "THE UNDER-MINE",
        blurb: &[
            "The mouth of it is boarded from the outside, and the boards are \
             stamped HENPECK, which is worth thinking about for a moment and \
             then worth thinking about again.",
            "Somebody sealed this in a hurry and somebody else has been \
             keeping the boards in repair for a very long time since, and the \
             two of them were not the same person and did not agree.",
        ],
        entry: &[
            "The seam was sealed from the outside, which is worth thinking about.",
            "Ossery said the foundry keeps melting down what keeps climbing \
             out of the melt. He did not say what climbs out of a seam.",
        ],
        floors: &["THE DIGGERS", "WHAT THE SEAM HID"],
        landings: &[
            "There are 14 of them and they put their tools down when you come \
             round the corner and pick them up again after, which is the only \
             part of it that is frightening.",
            "It was sealed for a reason and the reason is looking at you, and \
             behind the reason there is a vein of something the colour of a \
             very old bar of chocolate going down further than the lamp goes.",
        ],
        reward: "Prospector",
        also: &[],
    },
    // THE UNDERTOW, reached from a gallery by selling something good enough
    // that the buyer mentions where the last one was fished up.
    Dungeon {
        id: "the-undertow",
        name: "THE UNDERTOW",
        blurb: &[
            "The old man fished here for 60 years and the water goes down and \
             does not come back up, and both of those things were true the \
             whole time he was doing it.",
            "There is a boat pulled up on the shingle with BUNKO painted on \
             the transom, and somebody has left it there, and the paint is \
             not old.",
        ],
        entry: &[
            "The water goes down and does not come back up. Neither does the light.",
            "Sixty years is a long time to fish somewhere nothing swims.",
        ],
        floors: &["THE CURRENT", "THE THING ON THE HOOK"],
        landings: &[
            "The water decides how fast you are allowed to be. It decided that \
             about him too, for 60 years, and there is no arguing with a \
             decision made by a quantity.",
            "It comes up on the line the way a thing comes up when it has \
             chosen to. Underneath it the water is deeper than the world is, \
             and you understand, all at once, what he was patient about.",
        ],
        // No class at all. What the Undertow pays is room - one board of your
        // choice, one row taller for the rest of the run - and H3 says the
        // class it used to hand out is cut in favour of exactly that.
        reward: "",
        also: &[crate::event::Outcome::GrantRow],
    },
    // DEN RIVALS, which the Galapagos Emporium's exhibit promised and did not
    // deliver until now.
    Dungeon {
        id: "den-rivals",
        name: "DEN RIVALS",
        blurb: &[
            "The exhibit at the Emporium promised the fury of a thousand \
             bears and charged 4 gold for it and showed you a diorama.",
            "The museum never lied. It simply did not say where.",
        ],
        entry: &[
            "You counted the eyes. You stopped at forty.",
            "The exhibit promised the fury of a thousand bears. The museum \
             never lied.",
        ],
        floors: &["THE DEN MOUTH", "THE THOUSANDTH BEAR"],
        landings: &[
            "That was 100 of them and the den goes back further than a \
             hundred, and every one of them was in the way rather than in \
             front.",
            "The thousandth is the one the diorama was of, and the diorama \
             was to scale, and the placard did not say to what.",
        ],
        reward: "",
        also: &[crate::event::Outcome::Give("Bearhide")],
    },
    // WUMPUS WORLD. The classic hunt, and deterministic like everything else.
    Dungeon {
        id: "wumpus-world",
        name: "WUMPUS WORLD",
        blurb: &[
            "There are 20 rooms and one of them has a wumpus in it and the \
             wumpus does not stay in the room it is in.",
            "You will smell it before you see it. That is the good news and \
             it is also, on reflection, how it finds you.",
        ],
        entry: &[
            "Something in the dark already knows your footsteps.",
            "You smell it. Worse: that is how it finds you.",
        ],
        floors: &["DARK FLOOR", "THE WUMPUS"],
        landings: &[
            "Whatever lives near a wumpus lives there by being too quick and \
             too many to be worth the trouble. Neither is a defence against \
             somebody with a torch and 20 rooms to get through.",
            "It knew where you were the whole way in. What it did not know is \
             that you had stopped moving quietly a hundred yards back and had \
             been listening to it work that out.",
        ],
        reward: "Wumpus Hunter",
        also: &[],
    },];

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
    fn every_dungeon_pays_something() {
        for d in DUNGEONS {
            assert!(
                !d.reward.is_empty() || !d.also.is_empty(),
                "{} is three fights and a walk home",
                d.id
            );
        }
    }

    #[test]
    fn every_reward_is_a_real_class_and_only_from_here() {
        for d in DUNGEONS.iter().filter(|d| !d.reward.is_empty()) {
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
                if !a.reward.is_empty() {
                    assert_ne!(a.reward, b.reward);
                }
            }
        }
    }
}
