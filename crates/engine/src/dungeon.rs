//! Mini dungeons: a short chain of fights off the side of the road.
//!
//! A dungeon is a few floors that ends in a class you cannot get anywhere
//! else. It does not advance the ladder - when you come out you are standing
//! exactly where you went in, in front of the fight you had not got to yet.
//!
//! Floors are a **graph**, not a list. Each one names where it leads, so a
//! floor with one exit is the next room, a floor with none is a buffer stop,
//! and a floor with two is a set of points and a decision. All six dungeons
//! written before this were lists, and all six are straight lines in the new
//! shape - `every_shipped_dungeon_is_a_straight_line` is what holds them
//! there.
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
    /// The rooms, and what leads to what. Floor 0 is always the way in.
    ///
    /// An index into this list is a stable key - it is what the theme's
    /// landings are keyed on and what a siding names - so floors are appended
    /// rather than reordered, for the same reason `CATALOG` is.
    pub floors: &'static [Floor],
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


/// One way out of a floor.
///
/// A label and a line under it, because an exit is drawn as a choice and read
/// the way a `Choice` is read. Both are empty on a floor with one way on:
/// there is no decision to name, and `the_points_have_a_scene` is what stops
/// a real fork shipping without one.
#[derive(Copy, Clone, Debug)]
pub struct Exit {
    /// Index into the dungeon's `floors`.
    pub to: usize,
    /// What the lever is called. Shown as a choice; through the theme layer.
    pub label: &'static str,
    /// One line under it, in the register a `Choice::blurb` has.
    pub blurb: &'static str,
}

impl Exit {
    /// The only way on. A corridor rather than a decision.
    pub const fn on(to: usize) -> Exit {
        Exit { to, label: "", blurb: "" }
    }
}

/// One room of a dungeon: a fight, what is said after it, and where it goes.
#[derive(Copy, Clone, Debug)]
pub struct Floor {
    /// The creature. An alternate, as it always was.
    pub creature: &'static str,
    /// Said on the landing after this floor is cleared. For a leaf this is the
    /// ending.
    ///
    /// It lives here rather than in a list beside `floors` because the two
    /// were required to be the same length and nothing but a test said so.
    /// Now the type says it.
    pub landing: &'static str,
    /// Where this floor leads. Empty: a buffer stop, and the dungeon ends
    /// here. One: the next floor, which is every floor of every dungeon
    /// written before the yard. Two or more: points, and a decision.
    pub exits: &'static [Exit],
    /// Read when the exits are two or more: the scene at the points. Empty
    /// otherwise.
    pub fork: &'static [&'static str],
    /// Played when a dungeon is *entered at* this floor rather than walked to.
    /// Empty for every floor nothing can land you on.
    pub entry: &'static [&'static str],
    /// Applied on clearing this floor, before the landing.
    ///
    /// For a buffer stop this is what clearing the dungeon by this route pays;
    /// for a floor in the middle it is nearly always empty. `Dungeon::also`
    /// and `Dungeon::reward` go on meaning "on any way out", so a floor's own
    /// `also` is on top of them and the six shipped dungeons need no change of
    /// meaning.
    pub also: &'static [crate::event::Outcome],
}

impl Floor {
    /// A floor that leads on: one fight, one landing, and the ways out.
    ///
    /// The exits are a slice at the call site rather than a floor number here
    /// because a `const fn` cannot hand back a reference to something built
    /// out of its own arguments - there is nowhere for it to live. Which is
    /// no loss: `&[Exit::on(1)]` says "one way on, to floor 1" and a fork
    /// spells its levers out in the same place.
    pub const fn along(
        creature: &'static str,
        landing: &'static str,
        exits: &'static [Exit],
    ) -> Floor {
        Floor { creature, landing, exits, fork: &[], entry: &[], also: &[] }
    }

    /// A buffer stop. Nothing leads out of it and the dungeon ends here.
    pub const fn last(creature: &'static str, landing: &'static str) -> Floor {
        Floor::along(creature, landing, &[])
    }

    /// Is this a buffer stop - a floor the dungeon ends at?
    pub fn is_leaf(&self) -> bool {
        self.exits.is_empty()
    }

    /// Is this a set of points - a floor that asks which way?
    pub fn is_fork(&self) -> bool {
        self.exits.len() > 1
    }
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
            "The thing you sold turns up three rungs later in a hamlet that is \
             not on any map you have seen, in the hands of a line foreman who \
             cannot possibly have paid for it.",
            "The hamlet has a new name now. It was Fenmouth when it was a \
             fishing village, before the company came and the boys were put on \
             trains and the Home for Immature Men was turned into a seed \
             facility. There is one old analyst left on the line, and Wenlock \
             still prays to the old gods, on a floor that cuts his knees, \
             which he says helps him concentrate.",
            "He has noticed a purple glint down between the shell and the \
             rock that will not move. He has been noticing it for six years \
             and has told nobody, because nobody who works here has the \
             shoulders to widen a crack in a rock, and he has been very \
             patient about waiting for somebody who does.",
        ],
        entry: &[
            "The hole in the back wall is a hole in the back wall for about \
             four feet, and then it is a staircase somebody cut, and then it \
             is not a staircase.",
            "Wenlock is already ahead of you. He has been ahead of you for \
             six years.",
        ],
        floors: &[
            Floor::along(
                "The Reciter",
                "The recitation stops mid-verse. Behind the pulpit, the shell has \
                 grown out over a crack in the rock the way a lip grows over a bad \
                 tooth. Wenlock gets a bar under it. Wenlock is seventy-one.",
                &[Exit::on(1)],
            ),
            Floor::along(
                "The Long Haul",
                "The train goes over on the bend. Whatever was in the cars is out \
                 in the dark now, and it does not appear to want anything from \
                 you at all, and it does not appear to want anything from Wenlock \
                 either, who keeps walking and does not look at it once.",
                &[Exit::on(2)],
            ),
            Floor::last(
                "The Watchers",
                "The Core is soup and light with a piece of somewhere else \
                 sitting in the middle of it. Wenlock looks at it for a while, \
                 and stops being an old man, and there is a moment there where he \
                 could have kept the lot. He splits it instead, the way he always \
                 said he would, and puts your share in your hand on his way past. \
                 Somewhere above you, for the first time in a long time, somebody \
                 is casting a line.",
            ),
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
            "The man behind the cellar door is called Corvin. He has been \
             talking for a long time and he is not talking to you. He is \
             describing a staircase. He is describing it very accurately.",
            "Behind the door there is a staircase.",
            "Nobody in the Manse comes down here, and everybody in the Manse \
             knows exactly how many steps there are, and Corvin is the only \
             one who will say the number out loud.",
        ],
        entry: &[
            "The door was not locked. Doors like this never are.",
            "Behind you it is a cellar. Ahead of you it is not, and the \
             difference happened somewhere in the middle without a line.",
        ],
        floors: &[
            Floor::along(
                "DOORKEEP",
                "The thing at the top is called DOORKEEP and it stands aside. It \
                 was always going to stand aside. What it was doing was making \
                 sure you went down rather than in.",
                &[Exit::on(1)],
            ),
            Floor {
                creature: "THE STAIR THAT LISTENS",
                landing: "The stair has been counting. Not the steps - there are 402 \
                          steps and it has known that since before there were steps - \
                          it has been counting *you*, and the number it has reached \
                          is one.",
                exits: &[
                    Exit {
                        to: 2,
                        label: "Keep going down",
                        blurb: "The bottom is the bottom, and the thing at the bottom \
                                is pleased to see you.",
                    },
                    Exit {
                        to: 3,
                        label: "The landing with the light in it",
                        blurb: "A door in the side of the rock, at the count of two \
                                hundred and one, and somebody behind it keeping stock.",
                    },
                ],
                fork: &[
                    "The stair goes on down and it does not go on down alone. At the \
                     count of two hundred and one there is a landing off it, and a \
                     door in the side of the rock, and a light behind the door that \
                     does not flicker when the air moves.",
                    "The bottom is what Corvin told you about, and he did not \
                     mention this. Down is the bottom, and whatever is pleased to \
                     see you is down there waiting. Sideways is the room with the \
                     light in it, and whoever keeps that room has been keeping it a \
                     long time, for a sense nobody upstairs has yet.",
                ],
                entry: &[],
                also: &[],
            },
            Floor::last(
                "THE LAST LANDING",
                "There is light at the bottom and the light is a person, or was, \
                 and it is pleased to see you, which is the worst of it. You come \
                 back up seeing with the wrong sense, and it does not stop.",
            ),
            // The crossbar of the T. A room off the stair rather than under
            // it, and the only place in the game that sells to the mind lane -
            // which is the lane this dungeon unlocks, so the gear and the
            // sense that reads it are behind the same three fights.
            Floor {
                creature: "THE SHADOW",
                landing: "They were not guarding the door. They were keeping the \
                          stock, and they have been keeping it a long time - \
                          everything on these shelves is for a sense nobody \
                          upstairs has yet, priced by somebody who knew you \
                          would come down eventually.",
                exits: &[],
                fork: &[],
                entry: &[],
                also: &[crate::event::Outcome::ShopAfter {
                    shelves: crate::piece::THRESHOLD_SHELF,
                }],
            },
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
             stamped HOLLOW KING. He sealed it, and he sealed it from out \
             here, and those are two separate things to have found out.",
            "Somebody sealed this in a hurry and somebody else has been \
             keeping the boards in repair for a very long time since, and the \
             two of them were not the same person and did not agree.",
        ],
        entry: &[
            "The seam was sealed from the outside. Whatever the boards are \
             for, they are not for keeping people out.",
            "Ossery said the foundry keeps melting down what keeps climbing \
             out of the melt. He did not say what climbs out of a seam.",
        ],
        floors: &[
            Floor::along(
                "THE DIGGERS",
                "The diggers put their tools down when you come round the corner \
                 and pick them up again after, which is the only part of it that \
                 is frightening. There are fourteen of them, which is the number \
                 Ossery gave, and Ossery has never been down here.",
                &[Exit::on(1)],
            ),
            Floor::last(
                "WHAT THE SEAM HID",
                "It was sealed for a reason and the reason is looking at you, and \
                 behind the reason there is a vein of something the colour of a \
                 very old bar of chocolate going down further than the lamp goes.",
            ),
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
            "Fenn fished here for sixty years and the water goes down and does \
             not come back up, and both of those things were true the whole \
             time he was doing it.",
            "There is a boat pulled up on the shingle with PATIENCE painted on \
             the transom, and somebody has left it there, and the paint is \
             not old.",
        ],
        entry: &[
            "The water goes down and does not come back up. Neither does the light.",
            "Sixty years is a long time to fish somewhere nothing swims.",
        ],
        floors: &[
            Floor::along(
                "THE CURRENT",
                "The water decides how fast you are allowed to be. It decided that \
                 about Fenn too, for sixty years, and there is no arguing with a \
                 decision made by a quantity.",
                &[Exit::on(1)],
            ),
            Floor::last(
                "THE THING ON THE HOOK",
                "It comes up on the line the way a thing comes up when it has \
                 chosen to. Underneath it the water is deeper than the world is, \
                 and you understand, all at once, what Fenn was patient about.",
            ),
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
            "The exhibit was called THE FURY OF A THOUSAND BEARS, it charged \
             four gold, and what it showed you was a diorama.",
            "The museum never lied. It simply did not say where.",
        ],
        entry: &[
            "You counted the eyes. You stopped at forty.",
            "The exhibit promised the fury of a thousand bears. The museum \
             never lied.",
        ],
        floors: &[
            Floor::along(
                "THE DEN MOUTH",
                "That was a hundred of them and the den goes back further than a \
                 hundred, and every one of them was in the way rather than in \
                 front.",
                &[Exit::on(1)],
            ),
            Floor::last(
                "THE THOUSANDTH BEAR",
                "The thousandth is the one the diorama was of. The diorama was to \
                 scale. The placard said A THOUSAND BEARS and did not say to what \
                 scale.",
            ),
        ],
        reward: "",
        also: &[crate::event::Outcome::Give("Bearhide")],
    },
    // WUMPUS WORLD. The classic hunt, and deterministic like everything else.
    Dungeon {
        id: "wumpus-world",
        name: "WUMPUS WORLD",
        blurb: &[
            "There are twenty rooms and one of them has a wumpus in it and the \
             wumpus does not stay in the room it is in. A card nailed up at \
             the mouth of the first says ROOMS 20, HAZARDS SOME, WUMPUS ONE, \
             and somebody has crossed out ONE and written it again.",
            "You will smell it before you see it. That is the good news and \
             it is also, on reflection, how it finds you.",
        ],
        entry: &[
            "Something in the dark already knows your footsteps.",
            "You smell it. Worse: that is how it finds you.",
        ],
        floors: &[
            Floor::along(
                "DARK FLOOR",
                "Whatever lives near a wumpus lives there by being too quick and \
                 too many to be worth the trouble. Neither is a defence against \
                 somebody with a torch and twenty rooms to get through.",
                &[Exit::on(1)],
            ),
            Floor::last(
                "THE WUMPUS",
                "It knew where you were the whole way in. What it did not know is \
                 that you had stopped moving quietly a hundred yards back and had \
                 been listening to it work that out. The card at the mouth said \
                 WUMPUS ONE, and the card was right about the number.",
            ),
        ],
        reward: "Wumpus Hunter",
        also: &[],
    },

    // ---- THE SWITCHYARD -------------------------------------------------
    //
    // Nine rooms under the cutting, and four fights whichever way you walk.
    // The first dungeon in the game with points in it, which is what the floor
    // graph was built for.
    //
    // One entry sees four floors. One orb sees seven, two orbs see eight, and
    // nothing sees nine - each line's buffer stops pay the ticket to the
    // *other* line, so the ninth room is always behind an orb that has been
    // spent. `switchyard::nine_floors_and_the_most_a_run_can_see_is_eight`
    // walks it and counts.
    //
    // Nothing on the dungeon's own `also`: every reward is a buffer stop's,
    // because which buffer stop you reached is the whole of what the yard asks.
    Dungeon {
        id: "the-switchyard",
        name: "THE SWITCHYARD",
        blurb: &[
            "The yard is nine rooms under the cutting, and the turntable is \
             the first, and from it two lines go off into the dark with points \
             on each of them, and a buffer stop at the end of every road.",
            "The timetable Hesketh sells lists eleven trains a day out of \
             here, and Ambrose keeps the times. There are no trains. Something \
             has to be moving for a time to be kept, and whatever it is, it is \
             moving to the sheet.",
            "Four fights down either line. What is at the buffer stop was left \
             there on purpose. Nobody who left it expected to be back for it.",
        ],
        entry: &[
            "The turntable takes you a quarter of the way round and stops, and \
             the bell rings once, and when it turns back you are facing the \
             other way, down the yard.",
            "Somewhere past the lamp the points are already thrown. Ambrose \
             was here first. Ambrose is always here first.",
        ],
        floors: &[
            // [0] The mouth, and the first set of points.
            Floor {
                creature: "THE SHUNTER",
                landing: "The shunter goes back to the turntable pit when it \
                          is done with you and lies down on it, which is what \
                          it does between trains, and the turntable turns a \
                          quarter of the way round under it and stops. Only \
                          the down line leaves this pit. Ambrose pulled the \
                          lever for the up line a long time ago and the up \
                          line has been a mile of nothing ever since.",
                // **One way on, and the other line is not walkable.** The yard
                // was one graph with a fork at the mouth; A7 cuts it into two
                // islands with no track between them, and the Up Line is the
                // only crossing. That is what the orbs are for - a ticket to
                // somewhere you cannot otherwise get to, rather than a
                // shortcut to somewhere you could have walked.
                //
                // Ambrose pulled the lever and it is still pulled. What
                // changed is that the other line is no longer a few yards off
                // in the dark; it is a mile off, and you need a ticket.
                exits: &[Exit::on(1)],
                fork: &[],
                entry: &[],
                also: &[],
            },
            // [1] Down line, and where the Signalman's Orb puts you down.
            Floor {
                creature: "THE PLATELAYERS",
                landing: "The platelayers put the rail back where it was. They \
                          were only ever going to put it back where it was. \
                          Ahead the ballast dips, and the sleepers stop being \
                          level.",
                exits: &[Exit::on(2)],
                fork: &[],
                entry: &[
                    "The orb goes into the socket and the socket is a set of \
                     points, and the points throw, and you are standing on the \
                     Down line a hundred yards past the pit, and the turntable \
                     is behind you and already turning.",
                ],
                also: &[],
            },
            // [2] The pit points.
            Floor {
                creature: "THE BALLAST",
                landing: "The pit is where the ballast came from, and what came \
                          up out of it with the ballast is still down here, and \
                          it goes back into the pit when it has finished, and \
                          the lamp on the post beyond it is lit.",
                exits: &[
                    Exit {
                        to: 3,
                        label: "The coal road",
                        blurb: "It ends at the coal stage. There is still coal in it.",
                    },
                    Exit {
                        to: 4,
                        label: "The water road",
                        blurb: "It ends at the tower. The tank is full and nothing has drunk from it.",
                    },
                ],
                fork: &[
                    "Past the ballast pit the Down line splits again, and there \
                     is a lamp on a post here that says COAL one way and WATER \
                     the other, and the lamp is lit, and there is nobody to \
                     have lit it.",
                    "Both roads end. That was painted on the wall at the top. \
                     What they end at is the question.",
                ],
                entry: &[],
                also: &[],
            },
            // [3] Buffer stop: the coal stage.
            Floor {
                creature: "THE COAL STAGE",
                landing: "The coal stage is a wooden platform with a heap on it \
                          and a shovel, and the heap is warm, and under the \
                          shovel there is a ledger with a row of times in it, \
                          and the last time is this morning's. Whoever was \
                          shovelling was here today. What they laid under the \
                          boards, they laid for somebody with a chest to put \
                          over it.",
                exits: &[],
                fork: &[],
                entry: &[],
                also: &[
                    crate::event::Outcome::Give("Ballast Bed"),
                    crate::event::Outcome::Give("Shunter's Orb"),
                    crate::event::Outcome::Flag("switchyard-cleared"),
                    crate::event::Outcome::Count("sidings-cleared"),
                ],
            },
            // [4] Buffer stop: the water tower.
            Floor {
                creature: "THE WATER TOWER",
                landing: "The tank is full. It has been full for as long as the \
                          yard has been shut, because nothing here has drunk. \
                          Under the tower there is a length of rodding laid out \
                          straight, oiled, and a note pinned to it in Ambrose's \
                          hand that says FOR THE FEET, which is either a joke or \
                          the only instruction you are going to get.",
                exits: &[],
                fork: &[],
                entry: &[],
                also: &[
                    crate::event::Outcome::Give("Points Rodding"),
                    crate::event::Outcome::Give("Shunter's Orb"),
                    crate::event::Outcome::Flag("switchyard-cleared"),
                    crate::event::Outcome::Count("sidings-cleared"),
                ],
            },
            // [5] Up line, and where the Shunter's Orb puts you down.
            Floor {
                creature: "THE GANTRY",
                landing: "The gantry carries eleven signal arms and all eleven \
                          are lowered, which is clear, and something up there \
                          was pulling them one at a time, and now nothing is. \
                          Ahead the lamp room door is open and the room is lit.",
                exits: &[Exit::on(6)],
                fork: &[],
                entry: &[
                    "The orb goes into the socket and the socket is a signal, \
                     and the arm drops, and you are under the gantry on the Up \
                     line with eleven lamps lit above you, and the turntable is \
                     behind you and already turning.",
                ],
                also: &[],
            },
            // [6] The shed points.
            Floor {
                creature: "THE LAMP ROOM",
                landing: "Every lamp in the room is trimmed and filled and \
                          burning, and the lamp room keeper is on the floor, and \
                          the lamps go on burning, because a lamp does not know. \
                          Beyond the room the roads part under the last one.",
                exits: &[
                    Exit {
                        to: 7,
                        label: "The shed road",
                        blurb: "It ends at the goods shed, and the shed is locked from the inside.",
                    },
                    Exit {
                        to: 8,
                        label: "The roundhouse road",
                        blurb: "It ends at the roundhouse, and something in the roundhouse is in steam.",
                    },
                ],
                fork: &[
                    "The Up line splits under the last lamp, and the two roads \
                     run side by side for a while before one bends off to the \
                     shed and the other straight on to the roundhouse, and from \
                     the points you can see both ends and reach one.",
                    "Ambrose has thrown these too. He throws them every day at \
                     14:05, for a train that is not coming, and today they are \
                     thrown for you.",
                ],
                entry: &[],
                also: &[],
            },
            // [7] Buffer stop: the goods shed.
            Floor {
                creature: "THE GOODS SHED",
                landing: "The goods shed was locked from the inside because the \
                          clerk was inside, and the clerk is a very careful \
                          person and has kept the ledger up to the minute, and \
                          the ledger is what is worth having: it is enchanted \
                          into a hat-shaped plate on the counter, because the \
                          clerk had a head and wanted somewhere to keep the \
                          accounts.",
                exits: &[],
                fork: &[],
                entry: &[],
                also: &[
                    crate::event::Outcome::Give("Booking Hall"),
                    crate::event::Outcome::Give("Signalman's Orb"),
                    crate::event::Outcome::Flag("switchyard-cleared"),
                    crate::event::Outcome::Count("sidings-cleared"),
                ],
            },
            // [8] Buffer stop: the roundhouse. The ninth room, and the one
            // nothing can reach twice.
            Floor {
                creature: "THE ROUNDHOUSE",
                landing: "It was in steam. It is still in steam. It is on the \
                          turntable in the roundhouse and it will be on it \
                          tomorrow, and the roundhouse is the end of the yard \
                          in every sense there is. On the driver's seat there is \
                          a coil of signal wire, wound neat, warm from the \
                          boiler, and a ball of glass in the firebox that has \
                          not melted and is not going to.",
                exits: &[],
                fork: &[],
                entry: &[],
                also: &[
                    crate::event::Outcome::Give("Signal Wire"),
                    crate::event::Outcome::Give("Signalman's Orb"),
                    crate::event::Outcome::Flag("switchyard-cleared"),
                    crate::event::Outcome::Count("sidings-cleared"),
                ],
            },
        ],
        reward: "",
        also: &[],
    },
];

impl Dungeon {
    /// How many fights are left from `floor`, counting the one standing on it,
    /// down the longest road out.
    ///
    /// This is the number a banner wants and `floors.len()` is not. Nine rooms
    /// in a graph with points in it are four fights whichever way you walk, and
    /// a run that came back in by a siding and found two rooms already beaten
    /// has fewer than that. For a straight line entered at floor 0 with
    /// nothing cleared it is exactly `floors.len()`, which is why the six
    /// dungeons written before the graph read what they always read.
    ///
    /// `cleared` is the run's whole list, across every dungeon; the ones that
    /// are not this dungeon's are ignored, so a caller hands over
    /// `&run.cleared_floors` and thinks about nothing.
    pub fn fights_ahead(&self, floor: usize, cleared: &[(&'static str, usize)]) -> usize {
        let beaten = |i: usize| cleared.iter().any(|&(id, f)| id == self.id && f == i);
        // The graph is acyclic - `no_dungeon_doubles_back` is the guard - so
        // this terminates without a visited set. A depth cap stands anyway,
        // because a lint that has not run yet is not a proof.
        fn walk(d: &Dungeon, at: usize, beaten: &dyn Fn(usize) -> bool, depth: usize) -> usize {
            if depth == 0 {
                return 0;
            }
            let Some(f) = d.floors.get(at) else { return 0 };
            let here = usize::from(!beaten(at));
            here + f.exits.iter().map(|e| walk(d, e.to, beaten, depth - 1)).max().unwrap_or(0)
        }
        walk(self, floor, &beaten, self.floors.len() + 1)
    }

    /// How many floors in this one ask which way.
    pub fn forks(&self) -> usize {
        self.floors.iter().filter(|f| f.is_fork()).count()
    }
}

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
                    crate::combat::alternate(f.creature).is_some(),
                    "{}: no such creature as {}",
                    d.id,
                    f.creature
                );
                // The landing used to be checked by counting two lists against
                // each other. `Floor` carries its own, so the count cannot be
                // wrong; what can still be wrong is a floor that says nothing.
                assert!(
                    !f.landing.is_empty(),
                    "{}: {} is cleared and nobody says anything",
                    d.id,
                    f.creature
                );
            }
        }
    }

    /// Nothing here is three fights and a walk home.
    ///
    /// A dungeon pays on any way out (`reward`, `also`), or every buffer stop
    /// pays its own way - which is the shape a graph wants, because which
    /// buffer stop you reached is the whole of what a graph asks.
    #[test]
    fn every_dungeon_pays_something() {
        for d in DUNGEONS {
            let on_any_exit = !d.reward.is_empty() || !d.also.is_empty();
            let every_leaf_pays =
                d.floors.iter().filter(|f| f.is_leaf()).all(|f| !f.also.is_empty());
            assert!(
                on_any_exit || every_leaf_pays,
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

    // ------------------------------------------------------- the graph lints
    //
    // Six of the seven A1.1 asks for. The seventh - that every floor with an
    // `entry` is the landing point of some destination - needs
    // `pedestal::Where::Siding`, which is M3; its forward half is
    // `no_floor_offers_a_way_in_that_nothing_uses` below and it is vacuous
    // until a floor has an entry, which is stated here rather than discovered.

    #[test]
    fn every_exit_leads_somewhere_that_exists() {
        for d in DUNGEONS {
            for (i, f) in d.floors.iter().enumerate() {
                for e in f.exits {
                    assert!(
                        e.to < d.floors.len(),
                        "{}: floor {i} leads to {}, and there are {} floors",
                        d.id,
                        e.to,
                        d.floors.len()
                    );
                }
            }
        }
    }

    /// Nothing leads back to the way in, and nothing leads to itself.
    ///
    /// Floor 0 is the mouth, and a mouth you can be sent back to is a dungeon
    /// you can walk twice by accident.
    #[test]
    fn no_exit_points_at_the_mouth_or_at_itself() {
        for d in DUNGEONS {
            for (i, f) in d.floors.iter().enumerate() {
                for e in f.exits {
                    assert_ne!(e.to, 0, "{}: floor {i} leads back to the mouth", d.id);
                    assert_ne!(e.to, i, "{}: floor {i} leads to itself", d.id);
                }
            }
        }
    }

    /// A dungeon goes one way.
    ///
    /// `fights_ahead` walks the graph without a visited set, and the interface
    /// draws a path rather than a loop. Both of those are true because of this
    /// test and not because of anything in the type.
    #[test]
    fn no_dungeon_doubles_back() {
        for d in DUNGEONS {
            // Grey while a floor is on the stack, black once it is finished.
            // A grey floor met again is a road that comes back on itself.
            let mut colour = vec![0u8; d.floors.len()];
            fn walk(d: &Dungeon, at: usize, colour: &mut Vec<u8>, path: &mut Vec<usize>) {
                assert_ne!(
                    colour[at], 1,
                    "{}: {:?} comes back to floor {at}",
                    d.id, path
                );
                if colour[at] == 2 {
                    return;
                }
                colour[at] = 1;
                path.push(at);
                for e in d.floors[at].exits {
                    walk(d, e.to, colour, path);
                }
                path.pop();
                colour[at] = 2;
            }
            walk(d, 0, &mut colour, &mut Vec::new());
        }
    }

    /// Every room can be got to - by walking, or by a ticket.
    ///
    /// A floor nothing leads to is a fight, a landing and a reward that no run
    /// can ever meet - the dead content `completable.rs` exists to catch one
    /// rung over.
    ///
    /// **Walking is no longer the only way in.** A7 cut THE SWITCHYARD into
    /// islands with no track between them, and the Up Line orb is the crossing.
    /// So a siding counts as a mouth: a floor a `Where::Siding` lands on is
    /// reachable, and everything it leads to is reachable through it.
    ///
    /// The rule that matters is unchanged and is the reason this test exists -
    /// no room may be unreachable by *any* route. What changed is that the set
    /// of routes now includes the ones you buy.
    #[test]
    fn every_floor_is_reachable_from_the_mouth() {
        for d in DUNGEONS {
            let mut seen = vec![false; d.floors.len()];
            let mut stack = vec![0usize];
            // Every siding into this dungeon is another way in.
            for dest in crate::pedestal::DESTINATIONS {
                if let crate::pedestal::Where::Siding { dungeon, floor } = dest.kind {
                    if dungeon == d.id && floor < d.floors.len() {
                        stack.push(floor);
                    }
                }
            }
            while let Some(at) = stack.pop() {
                if std::mem::replace(&mut seen[at], true) {
                    continue;
                }
                stack.extend(d.floors[at].exits.iter().map(|e| e.to));
            }
            for (i, ok) in seen.iter().enumerate() {
                assert!(
                    ok,
                    "{}: nothing leads to floor {i} ({}), and no orb lands on it either",
                    d.id, d.floors[i].creature
                );
            }
        }
    }

    /// A decision is a scene, and only a decision is.
    ///
    /// Points with nothing written at them are two unlabelled buttons; a scene
    /// on a corridor is a paragraph nobody will ever be shown.
    #[test]
    fn the_points_have_a_scene_and_nothing_else_does() {
        for d in DUNGEONS {
            for (i, f) in d.floors.iter().enumerate() {
                if f.is_fork() {
                    assert!(
                        !f.fork.is_empty(),
                        "{}: floor {i} asks which way and says nothing",
                        d.id
                    );
                    for e in f.exits {
                        assert!(!e.label.is_empty(), "{}: floor {i} has an unnamed lever", d.id);
                        assert!(
                            e.blurb.len() > 20,
                            "{}: floor {i}'s lever {:?} needs a line under it",
                            d.id,
                            e.label
                        );
                    }
                } else {
                    assert!(
                        f.fork.is_empty(),
                        "{}: floor {i} has one way on and a scene at the points",
                        d.id
                    );
                }
            }
        }
    }

    /// A way in that nothing can use is a paragraph nobody will be shown.
    ///
    /// The other half - that every siding lands on a floor which has one - is
    /// in `pedestal.rs`, where `Where::Siding` lives.
    #[test]
    fn no_floor_offers_a_way_in_that_nothing_uses() {
        for d in DUNGEONS {
            for (i, f) in d.floors.iter().enumerate() {
                if f.entry.is_empty() {
                    continue;
                }
                assert!(
                    crate::pedestal::lands_on(d.id, i),
                    "{}: floor {i} has an entry cutscene and nothing lands on it",
                    d.id
                );
                for line in f.entry {
                    assert!(line.len() > 20, "{}: floor {i}'s entry is worth reading", d.id);
                }
            }
        }
    }

    /// Every dungeon written before the yard is a straight line, and stays one.
    ///
    /// This is what "landed inert" means for M1: the graph is a new shape and
    /// the six things standing in it are the same six things. A dungeon that
    /// grows points is a dungeon whose banner, map label and pip row all
    /// change, and it should have to say so here first.
    #[test]
    fn every_shipped_dungeon_is_a_straight_line() {
        // Re-pinned twice, never loosened. At M6 the claim was about the six
        // that predate the floor graph, and THE SWITCHYARD was named rather
        // than skipped so a *seventh* growing points would still fail.
        //
        // At A4 a seventh grew points, deliberately: THE THRESHOLD is a T now,
        // with the shop on its crossbar. So it is named too, and the five that
        // are still lines are still checked line by line - an eighth would
        // fail this exactly as the seventh did.
        const STRAIGHT: usize = 5;
        let lines: Vec<&Dungeon> = DUNGEONS
            .iter()
            .filter(|d| d.id != "the-switchyard" && d.id != "the-threshold")
            .collect();
        assert_eq!(lines.len(), STRAIGHT, "a dungeon appeared that nothing here knows about");
        for d in lines {
            assert_eq!(d.forks(), 0, "{} has points in it", d.id);
            for (i, f) in d.floors.iter().enumerate() {
                let want = if i + 1 == d.floors.len() { 0 } else { 1 };
                assert_eq!(f.exits.len(), want, "{}: floor {i} is not a straight line", d.id);
                if want == 1 {
                    assert_eq!(f.exits[0].to, i + 1, "{}: floor {i} skips a room", d.id);
                }
                assert!(f.fork.is_empty(), "{}: floor {i} is a corridor with a scene", d.id);
                assert!(f.entry.is_empty(), "{}: floor {i} is not a siding", d.id);
                assert!(f.also.is_empty(), "{}: floor {i} pays on its own", d.id);
            }
            // And the number the banner has always printed is the number
            // `fights_ahead` gives back for a straight line walked from the top.
            assert_eq!(
                d.fights_ahead(0, &[]),
                d.floors.len(),
                "{}: a straight line is as many fights as it has rooms",
                d.id
            );
        }
    }

    #[test]
    fn fights_ahead_counts_the_road_out_and_not_the_rooms() {
        let d = by_id("the-threshold").expect("shipped");
        assert_eq!(d.fights_ahead(0, &[]), 3, "three floors, three fights");
        assert_eq!(d.fights_ahead(1, &[]), 2);
        assert_eq!(d.fights_ahead(2, &[]), 1, "the last one is still a fight");
        // Cleared floors are walked through rather than fought, so they do not
        // count - which is the whole reason a siding can read "floor 1 of 1".
        assert_eq!(d.fights_ahead(0, &[("the-threshold", 0), ("the-threshold", 1)]), 1);
        assert_eq!(
            d.fights_ahead(0, &[("the-crevice", 0)]),
            3,
            "another dungeon's cleared floors are not this one's"
        );
    }
}
