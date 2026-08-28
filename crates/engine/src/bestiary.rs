//! What each creature is *for*.
//!
//! `design/monster-themes.md` is the argument; this is the table. A theme
//! names the grids a creature fills and the vocabulary it draws from, and a
//! packer considers nothing outside them - which is what makes a creature
//! legible in the first three seconds of a fight, and what cuts a search's
//! candidate pool by roughly sixty percent.
//!
//! **This lived in `tests/pack_francis.rs`.** It was a test-local enum for as
//! long as the only thing that needed it was the search that authors boards.
//! A `MonsterFrame` carries a theme, and a frame is engine data - a creature
//! that exists before its board does - so the table comes home. The packer and
//! the interface read it from here now, which also means there is one of it.
//!
//! ## Frames
//!
//! A frame is a creature with a name, a band, a theme and a note, and no board
//! at all. That is not a placeholder: it is the order the mission is built in.
//! Content lands as frames, all of it, and then every board is authored by hand
//! in one pass against a settled rating curve - because a board authored before
//! the curve under it stops moving is a board that will be authored twice.
//!
//! The repo already had two of these before anybody called them that: the four
//! in `ALTERNATES` stood beside the road for a long time without anybody saying
//! how hard they were meant to be, and `CREVICE` is an empty list of specs.

use crate::combat::{ALTERNATES, LADDER};
use crate::curse::CurseKind;
use crate::piece::{Action, PieceDef, PieceKind, SlotKind, Trigger};

/// What a creature is built around.
///
/// Six of these came out of the gear-slot rewrite and describe the ladder as
/// it stands. Four are new and describe things that stand beside it - the
/// dungeons, the destinations and the thing after Francis.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MonsterTheme {
    Striker,
    Wall,
    Burner,
    Slower,
    Drainer,
    Caster,
    /// Kills by shrinking you: mind damage, drains, and a great deal of curse
    /// resistance so none of your answers land. The eldritch lane's face, and
    /// the only theme whose damage never appears in a damage share.
    Hollow,
    /// Arrives everywhere and dies immediately. Many small quick activations
    /// and almost no health - two of something is worse than one of something
    /// twice the size, and this is the theme that says so.
    Swarm,
    /// The honest fight. Strength, rage, health, nothing clever, and no answer
    /// to it except being better.
    Beast,
    /// Makes you pay for time. Armour, hardening and curses applied - it does
    /// not out-damage you, it out-waits you.
    Warden,
}

impl MonsterTheme {
    pub const ALL: [MonsterTheme; 10] = [
        MonsterTheme::Striker,
        MonsterTheme::Wall,
        MonsterTheme::Burner,
        MonsterTheme::Slower,
        MonsterTheme::Drainer,
        MonsterTheme::Caster,
        MonsterTheme::Hollow,
        MonsterTheme::Swarm,
        MonsterTheme::Beast,
        MonsterTheme::Warden,
    ];

    /// The canonical key. Never shown raw - the theme layer looks it up.
    pub fn name(self) -> &'static str {
        match self {
            MonsterTheme::Striker => "Striker",
            MonsterTheme::Wall => "Wall",
            MonsterTheme::Burner => "Burner",
            MonsterTheme::Slower => "Slower",
            MonsterTheme::Drainer => "Drainer",
            MonsterTheme::Caster => "Caster",
            MonsterTheme::Hollow => "Hollow",
            MonsterTheme::Swarm => "Swarm",
            MonsterTheme::Beast => "Beast",
            MonsterTheme::Warden => "Warden",
        }
    }

    /// What it is for, in one line, in the house register.
    pub fn reads_as(self) -> &'static str {
        match self {
            MonsterTheme::Striker => "fast and fragile; punishes a slow board",
            MonsterTheme::Wall => "slow; heavy; hits back harder when hit",
            MonsterTheme::Burner => "kills on the clock, not the swing",
            MonsterTheme::Slower => "denies tempo; deals little itself",
            MonsterTheme::Drainer => "starves a build that banks pools",
            MonsterTheme::Caster => "bursty and mana-gated",
            MonsterTheme::Hollow => "takes your maximum away, and none of it comes back",
            MonsterTheme::Swarm => "everywhere at once, and nowhere for long",
            MonsterTheme::Beast => "no trick at all, and enough of everything else",
            MonsterTheme::Warden => "out-waits you rather than out-hitting you",
        }
    }

    /// The grids this creature fills.
    ///
    /// Two rather than five: two is what a player can read at a glance, and
    /// five is what made every creature on the ladder the same creature. The
    /// Wall is the exception and has three, because the two it wants deal no
    /// damage at all - see `allows`.
    ///
    /// *Amended for the four new themes.* The six had the property that every
    /// slot appeared in exactly two of them, which is a nice property of six
    /// and not a rule. Ten themes cannot have it. Swarm and Slower share a
    /// pair of grids and are not remotely the same creature, because a theme
    /// is a pair of grids **and** a vocabulary, and theirs have nothing in
    /// common.
    pub fn slots(self) -> &'static [SlotKind] {
        match self {
            MonsterTheme::Striker => &[SlotKind::Weapon, SlotKind::Gloves],
            MonsterTheme::Wall => &[SlotKind::Chest, SlotKind::Helmet, SlotKind::Weapon],
            MonsterTheme::Burner => &[SlotKind::Weapon, SlotKind::Greaves],
            MonsterTheme::Slower => &[SlotKind::Greaves, SlotKind::Gloves, SlotKind::Weapon],
            MonsterTheme::Drainer => &[SlotKind::Gloves, SlotKind::Helmet, SlotKind::Weapon],
            MonsterTheme::Caster => &[SlotKind::Weapon, SlotKind::Helmet],
            // The head takes your maximum health away and the body refuses to
            // let go of its own. Mind damage is the helmet's, so unlike the
            // Wall this one could always reach you - which is why it had no
            // weapon for two missions.
            //
            // It has one at M15 anyway. Every theme carries a weapon now, and
            // the rule it answers is blunter than this theme's argument: a
            // creature with nothing that swings is a creature a player can
            // stand in front of and out-wait, and five of the ten were.
            MonsterTheme::Hollow => &[SlotKind::Helmet, SlotKind::Chest, SlotKind::Weapon],
            MonsterTheme::Swarm => &[SlotKind::Gloves, SlotKind::Greaves, SlotKind::Weapon],
            MonsterTheme::Beast => &[SlotKind::Weapon, SlotKind::Chest],
            MonsterTheme::Warden => &[SlotKind::Chest, SlotKind::Greaves, SlotKind::Weapon],
        }
    }

    /// Does this piece speak the theme's language?
    ///
    /// A piece that carries nothing either way - a plain frame, a bare
    /// material - is allowed everywhere: a board needs cores and filler to
    /// assemble at all, and refusing them would leave most themes unable to
    /// finish a single item.
    pub fn allows(self, d: &PieceDef) -> bool {
        let b = &d.base;
        let speaks = match self {
            MonsterTheme::Striker => {
                b.physical_damage != 0
                    || b.magic_damage != 0
                    || b.strength != 0
                    || says(d, |a| {
                        matches!(a, Action::Damage { .. } | Action::GainSpellblade(_))
                    })
            }
            MonsterTheme::Wall => {
                b.armor != 0
                    || b.health != 0
                    || b.physical_harden != 0
                    || b.magic_harden != 0
                    || b.reflect != 0
                    || says(d, |a| {
                        matches!(
                            a,
                            Action::GainArmor(_) | Action::Grow(_) | Action::GainDeflection(_)
                        )
                    })
                    // And something to swing.
                    //
                    // Wall is the one theme whose grids deal no damage, and a
                    // creature fights entirely through its gear - exactly one
                    // creature on the ladder has an innate attack, and it is
                    // the Cave Rat. So a chest-and-helmet wall lands nothing,
                    // ever: The Iron Warden packed into one slow chest item and
                    // could not hurt anybody, two of them were no harder than
                    // one, and nine tests said so in nine vocabularies.
                    //
                    // Reflection was meant to be the answer and structurally
                    // cannot be. It needs the player to swing first and the
                    // armour to soak it, it is reported as `Reflected` rather
                    // than `Hit`, and it can never threaten somebody who
                    // out-damages it.
                    //
                    // So a wall carries a weapon - one item of it, which is all
                    // any creature may carry.
                    || d.slots().contains(&SlotKind::Weapon)
            }
            MonsterTheme::Burner => {
                b.physical_damage != 0
                    || b.magic_damage != 0
                    || says(d, |a| {
                        matches!(a, Action::Damage { .. })
                            || matches!(a, Action::Curse { kind: CurseKind::Searing, .. })
                    })
            }
            MonsterTheme::Slower => {
                d.speed_bonus != 0
                    || b.curse_resist != 0
                    || d.triggers.iter().any(|t| matches!(t, Trigger::OnBattleStart(_)))
                    || says(d, |a| {
                        matches!(
                            a,
                            Action::Curse {
                                kind: CurseKind::Frost | CurseKind::Stun | CurseKind::Misfire,
                                ..
                            }
                        ) || matches!(a, Action::ReduceCooldown(_))
                    })
            }
            MonsterTheme::Drainer => {
                b.mind != 0
                    || b.mind_resist != 0
                    || says(d, |a| {
                        matches!(
                            a,
                            Action::Drain { .. }
                                | Action::MindDamage { .. }
                                | Action::StunStrongest { .. }
                        )
                    })
            }
            MonsterTheme::Caster => {
                d.power_bonus != 0
                    || b.mana != 0
                    || matches!(
                        d.kind,
                        PieceKind::Spell
                            | PieceKind::Ink
                            | PieceKind::Orb
                            | PieceKind::Book
                            | PieceKind::Alignment
                    )
                    || says(d, |a| {
                        matches!(a, Action::GainMana(_) | Action::GainForking(_))
                            || matches!(a, Action::GainEmpowerment(_) | Action::GainShield(_))
                    })
            }
            // The Drainer's words plus the third lane's, and the resistance
            // that stops you answering: what makes a Hollow different from a
            // Drainer is that a Drainer wants what you banked and this one
            // wants the bar itself.
            MonsterTheme::Hollow => {
                b.mind != 0
                    || b.mind_resist != 0
                    || b.curse_resist != 0
                    || says(d, |a| {
                        matches!(
                            a,
                            Action::MindDamage { .. }
                                | Action::Drain { .. }
                                | Action::GainDread(_)
                                | Action::Gain { what: crate::piece::Resource::Insight, .. }
                        )
                    })
            }
            // Quick and small. Not curses - a Swarm and a Slower fill the same
            // two grids and the vocabulary is the whole difference between
            // them.
            MonsterTheme::Swarm => {
                d.speed_bonus != 0
                    || says(d, |a| {
                        matches!(a, Action::ReduceCooldown(_))
                            || matches!(a, Action::Damage { amount, .. } if *amount <= SWARM_BLOW)
                    })
                    || d.triggers.iter().any(|t| {
                        matches!(
                            t,
                            Trigger::OnAdjacentActivate(_)
                                | Trigger::OnAlignedActivate(_)
                                | Trigger::PerAdjacentItem { .. }
                        )
                    })
            }
            MonsterTheme::Beast => {
                b.strength != 0
                    || b.rage != 0
                    || b.health != 0
                    || b.physical_damage != 0
                    || says(d, |a| {
                        matches!(a, Action::Damage { kind: crate::combat::DamageType::Physical, .. })
                            || matches!(a, Action::Gain { what: crate::piece::Resource::Rage, .. })
                    })
            }
            MonsterTheme::Warden => {
                b.armor != 0
                    || b.physical_harden != 0
                    || b.magic_harden != 0
                    || b.curse_resist != 0
                    || says(d, |a| {
                        matches!(a, Action::GainArmor(_) | Action::GainDeflection(_))
                            || matches!(
                                a,
                                Action::Curse {
                                    kind: CurseKind::Frost
                                        | CurseKind::Stun
                                        | CurseKind::Misfire,
                                    ..
                                }
                            )
                    })
            }
        };
        speaks || plain(d)
    }
}

/// The most a single blow can be and still be a Swarm's.
///
/// Twenty-five. The point of a swarm is that no one of them is the problem, so
/// a piece that lands a real hit is a Striker's piece wearing a small name.
pub const SWARM_BLOW: i32 = 25;

/// Does any of this piece's triggers do something the predicate recognises?
fn says(d: &PieceDef, want: fn(&Action) -> bool) -> bool {
    d.triggers.iter().any(|t| {
        let mut found = false;
        crate::piece::walk_actions(t, &mut |a| found |= want(a));
        found
    })
}

/// Carries nothing any theme would recognise. Cores and filler, which every
/// board needs whatever it is for.
pub fn plain(d: &PieceDef) -> bool {
    // "Says nothing when it fires." It used to count triggers and four stat
    // fields, which meant a piece banking two nature every activation was
    // plain filler as long as it spelled that in `Stats` - and a hundred and
    // fifty-eight of them did. T2 moved thirty-six more into that spelling
    // and the predicate would have called them filler too.
    //
    // So it asks the classification instead: anything a piece hands over on
    // activation, in any spelling, is a piece with something to say.
    let acts = d.base.parts_when().iter().any(|(_, _, w)| {
        matches!(w, crate::stats::When::OnActivation | crate::stats::When::Damage)
    });
    d.triggers.is_empty() && d.effect.is_none() && !acts && d.base.health == 0
}

/// The clusters, from `design/monster-themes.md`.
///
/// Stretches rather than a rotation, so a player has time to work out what is
/// in front of them - and ordered to teach: hit first, then that hitting is
/// not enough. Rungs 45 and beyond are deliberately unthemed, because by then
/// a build has answers to everything and the interest is in the specific
/// creature rather than the category.
///
/// The four new themes are not on this table on purpose. They belong to things
/// standing *beside* the road - dungeon floors, destinations, and the thing
/// after Francis - and `design/monster-themes.md` §4 already exempts those
/// from the curve and the clusters both.
pub fn theme_for(rung: usize) -> Option<MonsterTheme> {
    Some(match rung + 1 {
        1..=6 => MonsterTheme::Striker,
        7..=13 => MonsterTheme::Wall,
        14..=20 => MonsterTheme::Burner,
        21..=28 => MonsterTheme::Slower,
        29..=36 => MonsterTheme::Caster,
        37..=44 => MonsterTheme::Drainer,
        _ => return None,
    })
}

/// What this creature draws from.
///
/// A mini-boss is a hybrid: its own cluster's theme and the one the next
/// cluster introduces, so it is both a harder version of what you have learned
/// and the first sight of what is coming.
pub fn themes_of(rung: usize, ordinary: bool) -> Vec<MonsterTheme> {
    let mut out: Vec<MonsterTheme> = theme_for(rung).into_iter().collect();
    if !ordinary {
        if let Some(next) =
            (rung + 1..LADDER.len()).find_map(|r| theme_for(r).filter(|t| Some(*t) != theme_for(rung)))
        {
            out.push(next);
        }
    }
    out
}

// ------------------------------------------------------------------ frames

/// A creature that exists before its board does.
///
/// Name, band, theme, note - and nothing else, because nothing else can be
/// decided honestly until the rating curve the board is packed against has
/// stopped moving. The note is one line for whoever authors the board, and it
/// is the only place the *intent* of a creature is written down.
#[derive(Copy, Clone, Debug)]
pub struct MonsterFrame {
    pub name: &'static str,
    /// The rung whose difficulty it packs to.
    ///
    /// A creature off the ladder has to be told its rung: the curve, the
    /// density target and the theme are all functions of one, and nothing else
    /// in the game says how hard a thing beside the road is meant to be.
    pub band: usize,
    /// The packer draws from exactly one.
    pub theme: MonsterTheme,
    /// One line for the Phase-4 packer-author.
    pub note: &'static str,
}

/// Every creature the mission adds, before any of them has a board.
///
/// Empty in Phase 1 and filled in Phase 2, which is why the lint below reads
/// green today and is still worth having: it goes red on the first frame and
/// stays red until the last board is authored.
pub const FRAMES: &[MonsterFrame] = &[
    // THE THRESHOLD, behind the Manse's cellar door. Hollow all the way down,
    // and the reason the chain reaches it in the mid-twenties rather than at
    // the top: what it unlocks is a pool, and a pool earned at rung forty is a
    // pool nobody gets to use.
    MonsterFrame {
        name: "DOORKEEP",
        band: 24,
        theme: MonsterTheme::Hollow,
        note: "teaches Drain before it hurts",
    },
    MonsterFrame {
        name: "THE STAIR THAT LISTENS",
        band: 25,
        theme: MonsterTheme::Hollow,
        note: "mind pressure, little else",
    },
    MonsterFrame {
        name: "THE LAST LANDING",
        band: 26,
        theme: MonsterTheme::Hollow,
        note: "the gate before the light",
    },
    // THE HERALD: two at once, and the first party fight outside the casino.
    MonsterFrame {
        name: "THE SHADOW",
        band: 43,
        theme: MonsterTheme::Hollow,
        note: "your build, hollowed",
    },
    MonsterFrame {
        name: "THE LANTERN",
        band: 43,
        theme: MonsterTheme::Striker,
        note: "what the shadow carries",
    },
    // The four beside the road. Bands are their *entry* bands rather than
    // their unlock events': a dungeon met by a formed build is a dungeon that
    // can be hard, and packing one for the rung whose event opened it would
    // make the whole set trivial.
    MonsterFrame {
        name: "THE DIGGERS",
        band: 33,
        theme: MonsterTheme::Warden,
        note: "armor that digs in",
    },
    MonsterFrame {
        name: "WHAT THE SEAM HID",
        band: 34,
        theme: MonsterTheme::Warden,
        note: "sealed for a reason",
    },
    MonsterFrame {
        name: "THE CURRENT",
        band: 33,
        theme: MonsterTheme::Slower,
        note: "the water sets the pace",
    },
    MonsterFrame {
        name: "THE THING ON THE HOOK",
        band: 35,
        theme: MonsterTheme::Slower,
        note: "patient, like its fisherman",
    },
    MonsterFrame {
        name: "THE DEN MOUTH",
        band: 30,
        theme: MonsterTheme::Beast,
        note: "the first hundred bears",
    },
    MonsterFrame {
        name: "THE THOUSANDTH BEAR",
        band: 32,
        theme: MonsterTheme::Beast,
        note: "the exhibit's promise, kept",
    },
    MonsterFrame {
        name: "DARK FLOOR",
        band: 30,
        theme: MonsterTheme::Swarm,
        note: "what lives near a wumpus",
    },
    MonsterFrame {
        name: "THE WUMPUS",
        band: 32,
        theme: MonsterTheme::Beast,
        note: "it already knows your footsteps",
    },
    // The only creature in the mission that arrives *with* a rung rather than
    // instead of one. Its band is the rung the memo is handed over on.
    MonsterFrame {
        name: "THE FLOCK",
        band: 27,
        theme: MonsterTheme::Swarm,
        note: "annoying before deadly",
    },
    // The thing after Francis. Hollow because it is a projection of something
    // still in transit, and the note is the whole packing brief: Wall and
    // Drainer vocabulary, dense past the curve, high reflect, heavy mind
    // damage fed by its own Dread, Drain on every glove, curse resists near
    // the cap - and a time-to-kill inside 16-29s at Medium, because past 30
    // the clock decides the fight rather than the board.
    MonsterFrame {
        name: "THE UNWOUND",
        band: 51,
        theme: MonsterTheme::Hollow,
        note: "harder than Francis, and it must be over before 30s",
    },
    // ---- THE SWITCHYARD -------------------------------------------------
    //
    // Nine floors, entry bands rather than the rung the mouth stands on: the
    // yard is met at displayed rung 26-28 by a build that has had twenty-five
    // rungs to form, and a dungeon packed for the rung that unlocked it is a
    // dungeon nobody notices. The bands step with depth, so the fourth fight
    // down either line is the hardest thing in it.
    //
    // The two lines read differently in the first three seconds on purpose:
    // the Down line is weight and the Up line is light.
    MonsterFrame {
        name: "THE SHUNTER",
        band: 27,
        theme: MonsterTheme::Warden,
        note: "makes you pay for the turntable's time; teaches the yard is slow",
    },
    MonsterFrame {
        name: "THE PLATELAYERS",
        band: 28,
        theme: MonsterTheme::Swarm,
        note: "many small blows, the rail put back as fast as it is lifted",
    },
    MonsterFrame {
        name: "THE BALLAST",
        band: 29,
        theme: MonsterTheme::Wall,
        note: "what came up with the ballast; reflect, and the one weapon a wall carries",
    },
    MonsterFrame {
        name: "THE COAL STAGE",
        band: 30,
        theme: MonsterTheme::Burner,
        note: "the heap is warm; searing on the clock",
    },
    MonsterFrame {
        name: "THE WATER TOWER",
        band: 30,
        theme: MonsterTheme::Slower,
        note: "the tank sets the pace; frost, and nothing much of its own",
    },
    MonsterFrame {
        name: "THE GANTRY",
        band: 28,
        theme: MonsterTheme::Caster,
        note: "eleven arms, eleven casts; bursty and mana-gated",
    },
    MonsterFrame {
        name: "THE LAMP ROOM",
        band: 29,
        theme: MonsterTheme::Burner,
        note: "every lamp lit; kills on the clock, not the swing",
    },
    MonsterFrame {
        name: "THE GOODS SHED",
        band: 30,
        theme: MonsterTheme::Drainer,
        note: "the clerk keeps the accounts, yours included",
    },
    MonsterFrame {
        name: "THE ROUNDHOUSE",
        band: 30,
        theme: MonsterTheme::Beast,
        note: "it is in steam; strength, health, no trick at all",
    },

    // ------------------------------------------------- THE HUNDRED's five
    //
    // Three chain endings, the herd one of them drives, and the thing at the
    // end of the perambulation. Each is met by a run that has spent five moves
    // at a time proving something about its board, so each asks for the thing
    // its chain taxed: the Ordnance charged drifts and scarps, the Drove
    // charged rivers and fords, the Enclosure charged hedges and a purse.
    MonsterFrame {
        name: "THE SURVEYOR",
        band: 35,
        theme: MonsterTheme::Warden,
        note: "he has measured your board already; make the fight short",
    },
    MonsterFrame {
        name: "THE DROVER",
        band: 42,
        theme: MonsterTheme::Striker,
        note: "pursuit; he has been walking since your first door",
    },
    MonsterFrame {
        name: "THE DRIVEN",
        band: 42,
        theme: MonsterTheme::Swarm,
        note: "the herd, and the reason the interception is a brawl",
    },
    MonsterFrame {
        name: "THE COMMISSIONER",
        band: 48,
        theme: MonsterTheme::Wall,
        note: "the fence made a person; he outlasts, and he is meant to",
    },
    MonsterFrame {
        name: "THE PARISH",
        band: 50,
        theme: MonsterTheme::Caster,
        note: "all five basis vectors at once, and the hardest thing authored",
    },
];

pub fn frame(name: &str) -> Option<&'static MonsterFrame> {
    FRAMES.iter().find(|f| f.name == name)
}

/// The spec a frame names, if one has been written for it yet.
pub fn spec_of(name: &str) -> Option<&'static crate::combat::MonsterSpec> {
    LADDER.iter().chain(ALTERNATES.iter()).find(|m| m.name == name)
}

/// Frames standing on the road with nothing on.
///
/// A frame with no spec at all, or a spec with an empty `gear` list. The
/// second is the shape `CREVICE` and the four in `ALTERNATES` were in for a
/// long time before anybody named the pattern - a creature the game knows the
/// name of and has not dressed.
pub fn unpacked() -> Vec<&'static MonsterFrame> {
    FRAMES
        .iter()
        .filter(|f| spec_of(f.name).is_none_or(|s| s.gear.is_empty()))
        .collect()
}

/// Is this creature standing there with nothing on?
pub fn is_unpacked(name: &str) -> bool {
    unpacked().iter().any(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_names_two_grids_and_says_what_it_is_for() {
        for t in MonsterTheme::ALL {
            assert!(!t.name().is_empty());
            assert!(t.reads_as().len() > 20, "{} does not say what it is for", t.name());
            let n = t.slots().len();
            assert!((2..=3).contains(&n), "{} fills {} grids", t.name(), n);
        }
    }

    /// Every theme reaches the weapon grid, and nothing fills more than three.
    ///
    /// This was `only_the_wall_fills_three`, and the Wall's reason was written
    /// where it was enforced: its two grids deal no damage and a creature
    /// fights entirely through its gear. M15 makes that everybody's reason.
    /// Five themes had no weapon slot - Slower, Drainer, Hollow, Swarm and
    /// Warden - and a creature with nothing that swings is one a player can
    /// stand in front of and out-wait.
    #[test]
    fn every_theme_can_reach_you_and_none_fills_more_than_three() {
        for t in MonsterTheme::ALL {
            assert!(
                t.slots().contains(&SlotKind::Weapon),
                "{} has no weapon grid, so nothing it wears can swing",
                t.name()
            );
            assert!(t.slots().len() <= 3, "{} fills {} grids", t.name(), t.slots().len());
        }
    }

    /// Every grid a theme fills can be packed, and it says something in most.
    ///
    /// Two questions, and M15 separated them because they stopped having the
    /// same answer. *Can it be packed* is about anything wearable and it has
    /// to hold in every grid, or the packer reports a board it could not
    /// finish rather than a table that is wrong. *Does it say something* is
    /// about pieces with an identity, and it holds in the two grids a theme
    /// was authored around.
    ///
    /// The weapon grid is exempt from the second for the five themes that
    /// gained it at M15: a Drainer has three weapons in its own vocabulary and
    /// forty-five it can wear. Widening the vocabulary to reach four would
    /// mean loosening what "a drainer's weapon" means to satisfy a lint, which
    /// is the tail wagging the dog - the milestone asked for something that
    /// swings, not for a themed arsenal.
    #[test]
    fn every_theme_can_find_something_to_wear_in_every_grid_it_fills() {
        for t in MonsterTheme::ALL {
            for &s in t.slots() {
                let wearable =
                    crate::piece::CATALOG.iter().filter(|d| d.fits(s) && t.allows(d)).count();
                assert!(
                    wearable >= 4,
                    "{} has {} pieces it can wear in the {}, so it cannot be packed",
                    t.name(),
                    wearable,
                    s.name()
                );
            }
            let expressive = t
                .slots()
                .iter()
                .filter(|&&s| {
                    crate::piece::CATALOG
                        .iter()
                        .filter(|d| d.fits(s) && t.allows(d) && !plain(d))
                        .count()
                        >= 4
                })
                .count();
            assert!(
                expressive >= 2,
                "{} says something in only {} of its grids",
                t.name(),
                expressive
            );
        }
    }

    #[test]
    fn the_ladder_is_clustered_the_way_the_document_says() {
        assert_eq!(theme_for(0), Some(MonsterTheme::Striker));
        assert_eq!(theme_for(6), Some(MonsterTheme::Wall));
        assert_eq!(theme_for(13), Some(MonsterTheme::Burner));
        assert_eq!(theme_for(20), Some(MonsterTheme::Slower));
        assert_eq!(theme_for(28), Some(MonsterTheme::Caster));
        assert_eq!(theme_for(36), Some(MonsterTheme::Drainer));
        assert_eq!(theme_for(44), None, "the run-in is unthemed on purpose");
    }

    #[test]
    fn the_four_new_themes_stand_beside_the_road_and_not_on_it() {
        let on_the_road: Vec<MonsterTheme> =
            (0..LADDER.len()).filter_map(theme_for).collect();
        for t in [
            MonsterTheme::Hollow,
            MonsterTheme::Swarm,
            MonsterTheme::Beast,
            MonsterTheme::Warden,
        ] {
            assert!(!on_the_road.contains(&t), "{} was put on the ladder", t.name());
        }
    }

    #[test]
    fn a_mini_boss_is_a_hybrid_of_its_cluster_and_the_next() {
        // Rung 7 opens the Wall stretch; a named creature there also speaks
        // Burner, which is what rung 14 will introduce.
        let mini = themes_of(6, false);
        assert_eq!(mini, vec![MonsterTheme::Wall, MonsterTheme::Burner]);
        assert_eq!(themes_of(6, true), vec![MonsterTheme::Wall]);
    }

    /// The frame lint, shipped as a **ratchet**.
    ///
    /// The rule is "no frame ships without a board" and it is red for the
    /// whole of Phases 2 and 3 on purpose - that is the phase discipline, and
    /// E6.8 asks for exactly it. A suite that stays red for eleven milestones
    /// is not a safety net though; it is a light nobody looks at, and every
    /// milestone in between needs a green suite to notice what it breaks.
    ///
    /// So it is the shape `catalog_shape.rs` already uses: a budget that is
    /// today's distance and can only go down, and an `#[ignore]`d target that
    /// asserts zero. Lower the budget in the commit that dresses a creature;
    /// never raise it.
    ///
    /// **Zero again, since the Switchyard's M9.** Nine between its M6 and M9.
    ///
    /// The number went 14 -> 15 in M15, when THE UNWOUND turned out to be a
    /// label on the route map with no creature under it, and 15 -> 13 when the
    /// owner packed two by hand. M17 packed the remaining thirteen with the
    /// generator and it sat at its target through three missions.
    ///
    /// It is nine again because THE SWITCHYARD's nine floors landed as frames,
    /// which is the phase discipline working rather than failing: Phase 2
    /// ships creatures as a name, a band, a theme and the stats of the ladder
    /// creature at that band, and Phase 4 packs the boards. **This is the one
    /// budget in the repository that is allowed to go up**, and only here,
    /// and only because the mechanism exists to make "nobody has dressed these
    /// yet" a number somebody has to look at rather than a thing to forget.
    ///
    /// `design/the-switchyard.md` Part D M9 took it back to zero, one creature
    /// at a time. It is an equality, not a bound, so packing a creature
    /// without lowering it fails just as loudly as adding one without raising
    /// it - which is how this went red the moment the ninth board landed.
    /// **Back to zero at THE HUNDRED's F12.** It held five for four
    /// milestones - THE SURVEYOR, THE DROVER, THE DRIVEN, THE COMMISSIONER
    /// and THE PARISH - which is the phase discipline working a third time.
    ///
    /// The boards those five wear are **borrowed** rather than packed: each
    /// one is a ladder creature's board at or near its own band, spliced in
    /// whole. That is a deliberate half-measure and the mission says so - the
    /// hand-packing is a job with the owner in the loop and it comes after the
    /// deploy, not before it. What borrowing buys is that the county's five
    /// fights are real fights at roughly the right weight on the day it ships,
    /// rather than five creatures standing there in nothing.
    const UNDRESSED: usize = 0;

    #[test]
    fn the_frames_are_no_more_undressed_than_they_were() {
        let naked = unpacked();
        assert!(
            naked.len() <= UNDRESSED,
            "{} creatures are standing on the road with nothing on, against a budget of {}: {:?}",
            naked.len(),
            UNDRESSED,
            naked.iter().map(|f| f.name).collect::<Vec<_>>()
        );
        assert_eq!(
            naked.len(),
            UNDRESSED,
            "somebody dressed one and did not lower the budget - it is {} now",
            naked.len()
        );
    }

    /// The one that is red until Phase 4 finishes.
    #[test]
    #[ignore]
    fn no_frame_ships_without_a_board() {
        let naked = unpacked();
        assert!(
            naked.is_empty(),
            "these creatures are standing on the road with nothing on: {:?}",
            naked.iter().map(|f| f.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn every_frame_names_a_band_on_or_beside_the_road() {
        for f in FRAMES {
            assert!(f.band >= 1, "{} packs to rung zero", f.name);
            assert!(!f.note.is_empty(), "{} has nothing said about what it is for", f.name);
        }
    }

    #[test]
    fn no_two_frames_share_a_name() {
        for (i, a) in FRAMES.iter().enumerate() {
            for b in &FRAMES[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }
}
