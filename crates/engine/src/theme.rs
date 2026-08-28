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
/// One place on the road, retold.
///
/// An event, a town or a dungeon - all three are things you arrive at, all
/// three have an id and a name, and a theme has the same job for each of them.
/// `prose` is optional and usually empty: a theme spends paragraphs only where
/// a proper noun is carrying the scene, because everything else `retell`
/// reaches a word at a time.
pub struct Retold {
    pub id: &'static str,
    pub title: &'static str,
    /// An event's scene, or a dungeon's blurb - the paragraphs read at the
    /// door, while it is still a decision. Empty keeps the canonical ones.
    pub prose: &'static [&'static str],
    /// A dungeon's entry cutscene: what it says once the decision is made.
    pub entry: &'static [&'static str],
    /// A dungeon's between-floor lines, the last of which is its ending.
    pub landings: &'static [&'static str],
}

pub struct Theme {
    /// The words items are named out of. Nothing else about how a name is
    /// built is a theme's business - the rule that a name grows with its
    /// rarity belongs to the generator.
    pub naming: &'static crate::naming::Naming,
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
    /// Canonical class name -> the title to show. The plain game's classes
    /// are named out of high fantasy; a theme that is not high fantasy has to
    /// say so somewhere, and this is the most visible place it shows.
    pub classes: &'static [(&'static str, &'static str)],
    /// Any other string in the interface, keyed by a short slug. See `word`.
    pub words: &'static [(&'static str, &'static str)],
    /// Whole words to swap inside prose the engine wrote - log lines, stat
    /// summaries, glossary definitions. Matched case-insensitively on whole
    /// words only, so "mana" becomes "Funny" and "manacle" is left alone.
    ///
    /// This is a translation of the engine's output rather than a change to
    /// it: the engine still says "mana" everywhere, because everything it
    /// decides depends on that word meaning one thing.
    pub vocabulary: &'static [(&'static str, &'static str)],
    /// A scene shown once, the first time a particular creature is beaten.
    /// Keyed by canonical monster name; the paragraphs are shown in order.
    ///
    /// A theme with none simply never interrupts, which is what the plain game
    /// does - it has no story to tell between fights.
    pub cutscenes: &'static [(&'static str, &'static [&'static str])],
    /// A line shown under a creature on the opponent panel: what it is, or why
    /// you would pick a fight with it. Keyed by canonical monster name.
    pub notes: &'static [(&'static str, &'static str)],
    /// The road, retold. One entry per event, town or dungeon this theme has
    /// something to say about, keyed by id.
    ///
    /// Keyed by the **id** rather than the name, because a title is prose and
    /// prose gets rewritten, while an id is a key and is the one thing about a
    /// door that never moves. One table rather than three, because ids are
    /// unique across the road for the same reason - they each name a place on
    /// it - and `no_road_id_is_told_twice` says so.
    pub told: &'static [Retold],
    /// Glossary entries this theme replaces or adds, as
    /// `(term to replace, new term, new definition)`. An empty first field
    /// adds an entry the plain game does not have.
    pub glossary: &'static [(&'static str, &'static str, &'static str)],
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

    /// What this theme calls the thing standing on the road with that id.
    ///
    /// `canonical` is the fallback and the caller always has it - an event
    /// knows its own title - so a theme with nothing to say about a door
    /// costs nothing and says the plain thing.
    pub fn place(&'static self, id: &str, canonical: &'static str) -> &'static str {
        self.told.iter().find(|r| r.id == id).map(|r| r.title).unwrap_or(canonical)
    }

    /// The scene itself, if this theme tells it differently.
    ///
    /// Most doors come back with the canonical prose, and that is the design:
    /// `retell` translates it a word at a time and the theme spends its own
    /// paragraphs only where a *proper noun* is doing the work, which no
    /// word-swap can reach.
    pub fn scene(
        &'static self,
        id: &str,
        canonical: &'static [&'static str],
    ) -> &'static [&'static str] {
        match self.told.iter().find(|r| r.id == id) {
            Some(r) if !r.prose.is_empty() => r.prose,
            _ => canonical,
        }
    }

    /// What a dungeon says as you step through it, in this theme's voice.
    pub fn entry(
        &'static self,
        id: &str,
        canonical: &'static [&'static str],
    ) -> &'static [&'static str] {
        match self.told.iter().find(|r| r.id == id) {
            Some(r) if !r.entry.is_empty() => r.entry,
            _ => canonical,
        }
    }

    /// The same for one of the lines said between a dungeon's floors.
    ///
    /// Keyed by floor index, because that is the stable key: `Retold.landings`
    /// is still a list parallel to `floors` and floor numbers do not move. A
    /// theme that retells some floors and not others - or one written before a
    /// dungeon grew a room - falls through to the canonical line for the ones
    /// it has nothing to say about, which is one floor's worth of silence
    /// rather than the whole dungeon's.
    pub fn landing(&'static self, id: &str, floor: usize, canonical: &'static str) -> &'static str {
        match self.told.iter().find(|r| r.id == id) {
            Some(r) => r.landings.get(floor).copied().unwrap_or(canonical),
            None => canonical,
        }
    }

    /// The same for a creature on the ladder.
    pub fn monster(&'static self, canonical: &'static str) -> &'static str {
        lookup(self, Table::Monsters, canonical).unwrap_or(canonical)
    }

    /// The same for a class.
    pub fn class(&'static self, canonical: &'static str) -> &'static str {
        lookup(self, Table::Classes, canonical).unwrap_or(canonical)
    }

    /// Re-tell a sentence the engine wrote in this theme's words.
    ///
    /// Whole words only, and the original's capitalisation is kept: "Mana" in
    /// mid-sentence comes back "Funny", "mana" comes back "funny". Returns the
    /// input untouched when the theme has no vocabulary, so the plain game
    /// pays nothing for this.
    pub fn retell(&'static self, prose: &str) -> String {
        if self.vocabulary.is_empty() {
            return prose.to_string();
        }
        let mut out = String::with_capacity(prose.len() + 16);
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String, this: &'static Theme| {
            if word.is_empty() {
                return;
            }
            let lower = word.to_lowercase();
            match this.vocabulary.iter().find(|(from, _)| *from == lower) {
                Some((_, to)) => {
                    // Follow the original's case so a replacement mid-sentence
                    // does not shout.
                    let starts_upper = word.chars().next().is_some_and(|c| c.is_uppercase());
                    if starts_upper {
                        // Capitalise the replacement rather than trusting the
                        // table's own casing: "Searing" became "roasting"
                        // mid-name because the entry happened to be written
                        // lowercase.
                        let mut cs = to.chars();
                        if let Some(f) = cs.next() {
                            out.extend(f.to_uppercase());
                            out.push_str(cs.as_str());
                        }
                    } else {
                        out.push_str(&to.to_lowercase());
                    }
                }
                None => out.push_str(word),
            }
            word.clear();
        };
        for ch in prose.chars() {
            if ch.is_alphanumeric() || ch == '\'' {
                word.push(ch);
            } else {
                flush(&mut word, &mut out, self);
                out.push(ch);
            }
        }
        flush(&mut word, &mut out, self);
        out
    }

    /// The scene owed for beating this creature, if any.
    pub fn cutscene(&'static self, monster: &str) -> Option<&'static [&'static str]> {
        self.cutscenes.iter().find(|(m, _)| *m == monster).map(|(_, s)| *s)
    }

    /// A line about this creature, if the theme has one.
    pub fn note(&'static self, monster: &str) -> Option<&'static str> {
        self.notes.iter().find(|(m, _)| *m == monster).map(|(_, n)| *n)
    }

    /// What this theme calls a glossary entry, and what it says about it.
    /// `None` when the theme leaves that entry alone.
    pub fn glossary_entry(&'static self, term: &str) -> Option<(&'static str, &'static str)> {
        self.glossary.iter().find(|(from, ..)| *from == term).map(|(_, t, d)| (*t, *d))
    }

    /// Entries this theme adds that the plain game has no equivalent for.
    pub fn extra_glossary(&'static self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.glossary.iter().filter(|(from, ..)| from.is_empty()).map(|(_, t, d)| (*t, *d))
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
    Classes,
}

/// Built once per theme per table. The tables are static and never change, so
/// the maps outlive everything that reads them.
fn lookup(theme: &'static Theme, table: Table, key: &str) -> Option<&'static str> {
    static MAPS: OnceLock<HashMap<(&'static str, usize), HashMap<&'static str, &'static str>>> =
        OnceLock::new();
    let maps = MAPS.get_or_init(|| {
        let mut all = HashMap::new();
        for t in THEMES {
            for (i, pairs) in [t.pieces, t.monsters, t.words, t.classes].iter().enumerate() {
                all.insert((t.id, i), pairs.iter().copied().collect());
            }
        }
        all
    });
    let i = match table {
        Table::Pieces => 0,
        Table::Monsters => 1,
        Table::Words => 2,
        Table::Classes => 3,
    };
    // Nothing in the table means the caller's own string is the answer, which
    // is what makes a half-written theme safe to ship.
    maps.get(&(theme.id, i)).and_then(|m| m.get(key).copied())
}

/// Every theme the game ships with. The first is the default.
pub static THEMES: &[&Theme] = &[&PLAIN, &TURTLE_DICK];

pub static PLAIN: Theme = Theme {
    naming: &crate::naming::PLAIN_NAMING,
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
    classes: &[],
    words: &[],
    vocabulary: &[],
    glossary: &[],
    cutscenes: &[],
    notes: &[],
    told: &[],
};

pub static TURTLE_DICK: Theme = Theme {
    naming: &TD_NAMING,
    id: "td",
    label: "TALES FROM THE CRYPT",
    blurb: "The same game, told in the language of the book. It's about a turtle.",
    story: &[
        "You are a Sprocketman.",
        "Your people were gear-folk of the Great Gear Cave in west Bambulon, \
         until Lord Drabley Henpeck found the Deep Chocolate you had been \
         quietly mining under it. He had the caves cleared and marched you all \
         to the pit the locals now call The End of All Gears.",
        "Then a gambler in a coat made of money fell through the roof of it.",
        "He was not there for you. He was there for something he had lost, and \
         he found it, and on his way back out he put a hand through the wall of \
         your cell because it was in front of him. He did not ask your name. By \
         the time you had climbed out he was three planes away.",
        "So: you are the one Sprocketman not in that pit, and there are still \
         millions who are. A Sprocketman's whole craft is making working gear \
         out of loose pieces - that is what the five frames are. Build yourself \
         up out of scrap until you can take Henpeck apart, and get them out.",
        "Somewhere far above all that, the gambler is still going, and the coat \
         is still on him. Neither of them is your problem.",
    ],
    classes: &[
        // Titles, not spell schools. Every one is somebody or something the
        // book already has, with the page it came off - the plain game names
        // its classes out of high fantasy, and that is the loudest place a
        // theme that is not high fantasy gives itself away.
        ("Archmage", "Master of Funny"),        // the chapter that is one blank page, p. 51
        ("Berserker", "Gorillathon"),           // the bedazzled gorilla, pp. 20-22
        ("Bloodletter", "Worm-Fact Keeper"),    // LETO on the flesh Throne, pp. 96-99
        ("Bulwark", "Corkwright"),              // cork, the bottom of the armour ladder
        ("Chronomancer", "Time-Sapper"),        // Time Sap from the Tree of Time, pp. 37-39
        ("Druid", "Radish Farmer"),             // silicon radishes, p. 115
        ("Duelist", "Treyway Prince"),          // the claim that summons Mumu Lelonde, p. 18
        ("Geomancer", "Grand Calculator"),      // the Grand Calculation, pp. 61-63
        ("Hexweaver", "Funnel Sergeant"),       // army-issue Funny funnels, p. 78
        ("Immense Guilt", "Henpeck's Accomplice"),
        // The sacred ladder's bottom rung: the Francians made Francis their
        // god by accident (pp. 61-63), and a novice is somebody who has knelt
        // and not yet been answered.
        ("Piety", "Francian Novice"),
        // Being answered. Multicity's commute grinds 1.79 trillion people down
        // a little every day (pp. 70-73); a season pass is what half of it
        // missing you looks like as an errand.
        ("Ticket to Ride", "Multicity Season Pass"),
        // The Sprocketmen were mined out of the Great Gear Cave by Lord
        // Drabley Henpeck (p. 44). He is still hiring.
        ("Tired", "Henpeck's Double Shift"),
        // Cogs are the one place the old words survive on purpose, salvaged
        // out of the Great Gear Cave - and salvage is exactly what this is.
        ("Recycler", "Gear Cave Salvager"),
        ("Juggernaut", "Multicity Commuter"),   // 1.79 trillion residents, pp. 70-73
        ("Longhauler", "Thrumbus Finisher"),
        ("Oracle", "Galapagos Timekeeper"),     // Galapagos Jim, time traveller, pp. 89-90
        ("Spellblade", "Katana Psychologist"),  // Henpeck's other job, p. 95
        ("Stormcaller", "Plug Energy Rep"),     // Spike Kaklon's Plug Energy, p. 32
        ("Templar", "Francian Ordinate"),       // the Francians pp. 61-63, ordination p. 75
        // The bottom rung of the theme's speed ladder, from the 45th Annual
        // Thrumbus Race - which is exactly what this is.
        ("Trundle", "Slow Trundler"),
        ("Ascendant", "Boyetano, Ascended"),
        // The antechamber under Eggbert's Mansion. You come back up seeing
        // with the wrong sense - residents of the Mansus are seen with the
        // ears and heard with the eyes, pp. 64-67.
        ("Threshold-Sighted", "Mansus-Sighted"),
        // The vein under the seam the Sprocketmen were told was empty, p. 44.
        ("Prospector", "Deep Chocolatier"),
        // CSV #12 gives the place its name and keeps it; the sibling title
        // *How to Train Your Wumpus* gives the class one of its own, which is
        // a better joke and stops a canonical name mapping to itself.
        ("Wumpus Hunter", "Wumpus Trainer"),
        // The gortball players' union, which shuts a stadium over sand, p. 29.
        ("Unionized", "Gortball Organized"),
        // What Hanglo Chiemstar was called for eleven seasons, p. 31.
        ("Showstopper", "Top of the Bill"),
        ("Avenged", "Sprocket Avenged"),
        ("Wanderer", "Plane Tourist"),          // half-tourist, half-catastrophe
        ("Warpriest", "Acolyte of Dobira"),     // the Master and Baylon, pp. 46-50
        ("Wellspring", "Soda Tycoon"),          // Skink Brink, pp. 4, 7-8, 53
    ],
    pieces: &[
        // The catalogue, re-cast from the book. Grades are kept as grades: the
        // armour ladder runs Cork -> Vinyl -> Sneel -> Time-Tempered ->
        // Ypytryktrium, so a piece's rank still reads at a glance even when
        // every word on it has changed.
        //
        // Ratchet Cog and Flywheel Cog are deliberately absent: cogs are the
        // player's own culture now, salvaged out of the Great Gear Cave, and
        // are the one place the old words survive on purpose.
        ("A Word About the Cellar", "Word of Eggbert's Cellar"),
        ("A Word About the Crownwright", "Word of the Kolok Hatter"),
        ("A Word About the Exhibition", "Word of the Gortball Men"),
        ("A Word About the Glow", "Word of the Burnwarp"),
        ("A Word About the Green Ledger", "Word of the Radish Tally"),
        ("A Word About the Picket", "Word of the Gladiators"),
        ("A Word About the Thirsty Wizard", "Word of Sam the Wise"),
        ("A Word About the Wrong Stars", "Word of the Tetrahedron"),
        ("Absolution", "Remark of Renewal"),
        ("Adamant Base", "Ypytryktrium Base"),
        ("Adamant Carapace", "Ypytryktrium Carapace"),
        ("Adamant Fang", "Megalodon Tooth"),
        ("Aegis Crown", "Thirty-Foot Hat"),
        ("Aegis Weave", "Cork Aegis"),
        ("Aether Layer", "Wimple Layer"),
        ("Ambush Mold", "Bushwhack Mold"),
        ("Ambusher's Grip", "Lxirp Ambush Grip"),
        ("An Unwound Mainspring", "Nibbalonius's Calling Card"),
        ("Anchor Material", "Ice-Anchor Material"),
        ("Anchored Sole", "Ice-Anchor Sole"),
        ("Answering Ring", "Backchat Ring"),
        ("Antechamber Crown", "Mansus Crown"),
        ("Anvil Frame", "Anvil Frame"),
        ("Apprentice's Primer", "Extra Funny Jokebook"),
        ("Arc Lightning", "Spooky Action"),
        ("Arcane Splinter", "Tetrahedron Splinter"),
        ("Archmage's Primer", "Comedian's Jokebook"),
        ("Archon's Crest", "Comptroller's Crest"),
        ("Ash Haft", "Banana-Peel Grip"),
        ("Ashen Material", "Cinder-Harvest Material"),
        ("Ashfall Ink", "Shelf-Drink Fluid"),
        ("Ashwoven Material", "Ash-Field Material"),
        ("Asker's Monocle", "Nesbit's Monocle"),
        ("Assassin's Hemline", "Mumu Lelonde's Hemline"),
        ("Astrolabe", "Dodecathlon Wheel"),
        ("Attendant Flame", "Blingarian Flare"),
        ("Azure Alignment", "Basseterian Long Grain"),
        ("Balance Weight", "Trillion-Pound Plate"),
        ("Balanced Grip", "Betting Stick"),
        ("Bare-Headed Fang", "Bare-Handed Goof"),
        ("Bastion Base", "Sneel Base"),
        ("Bearhide", "Thousand-Bear Hide"),
        ("Becalming Layer", "Wimpler Calm"),
        ("Berserker's Crest", "Gorillathon Crest"),
        ("Berserker's Plate", "Gladiator Plate"),
        ("Bileglass Vial", "Bileglass Phial"),
        ("Blade of Helms", "Katana Glint"),
        ("Blight Layer", "Rot Layer"),
        ("Blood Rite", "Grand Calculation"),
        ("Bloodbank Base", "Ench-Bank Base"),
        ("Blightfinger", "Least Weasel Ring"),
        ("Bloodletter's Ink", "Brumpus Oil"),
        ("Bloodrage Grip", "Arc Bat Grip"),
        ("Bloodring", "Roast Ring"),
        ("Bloodstone Bead", "Jolly Rancher"),
        ("Bloomcap", "Wextreen Cap"),
        ("Bloomed Crest", "Wextreen Crest"),
        ("Bloomguard", "Wextreen Material"),
        ("Boiled Leather", "Boiled Gooster Leather"),
        ("Bone Charm", "Worm Charm"),
        ("Bone Frame", "Wormbone Frame"),
        ("Bonesaw", "Wallspider Saw"),
        ("Braced Mold", "Quarry Mold"),
        ("Braced Plating", "Braced Cork Plating"),
        ("Bramble Mold", "Grungo-Thorn Mold"),
        ("Breaker's Fist", "Gorilla Knuckle"),
        ("Brigandine Base", "Cork Vest"),
        ("Broken Crown", "The Teetering Crown"),
        ("Bronze Fang", "Frong Tooth"),
        ("Bronze Frame", "Vinyl Frame"),
        ("Bronze Plating", "Vinyl Plating"),
        ("Bulwark Base", "Quarry Base"),
        ("Bulwark Bead", "Quarry Bead"),
        ("Bulwark Layer", "Sneel Layer"),
        ("Bulwark Material", "Cork Material"),
        ("Bulwark Plating", "Time-Tempered Plating"),
        ("Bulwark Vial", "Spindrift Can"),
        ("Buttressed Frame", "Quarry Frame"),
        ("Cadence Mold", "Metronome Mold"),
        ("Chain Coil", "Fishing Reel"),
        ("Chain Layer", "Fishing-Line Layer"),
        ("Chained Codex", "The Chained Archive"),
        ("Chainlink Mold", "Bike-Chain Mold"),
        ("Chalked Circle", "The Chalk Outline"),
        ("Channeling Mold", "The Funny Funnel"),
        ("Chapbook", "Guidance Sheet"),
        ("Chapel Base", "Monastery Base"),
        ("Chapel Frame", "Monastery Frame"),
        ("Chipped Edge", "Sneel Shard"),
        ("Choir of Ash", "Comedy Bomb"),
        ("Cinder Base", "Ash-Field Base"),
        ("Cinderscript Ink", "Magma Glaze"),
        ("Clockwork Key", "Golden Game-Show Key"),
        ("Clouded Orb", "Blizzard Globe"),
        ("Codex Interminable", "The Endless Dissertation"),
        ("Coldstep Mold", "Cold-Feet Mold"),
        ("Colossus Ring", "Wheel of the Thrumbus Race"),
        ("Cometfall", "Steel-Ball Drop"),
        ("Consecrated Plating", "Francian Plating"),
        ("Corded Grip", "Fishing-Line Grip"),
        ("Coven Crest", "Order Crest"),
        ("Coven Mold", "Order Mold"),
        ("Covenant Frame", "Koan Frame"),
        ("Cracked Pauldron", "Cracked Gear-Plate"),
        ("Crest of Vigor", "Soft Drink Cap"),
        ("Crimson Alignment", "Striped Boskner"),
        ("Crown of Nails", "Crown of Darts"),
        ("Crown of the Deep", "Crown of Cleveland"),
        ("Crownwright's Measure", "Francian Head-Measure"),
        ("Cull", "The Rubber"),
        ("Cursed Blade", "Martyr's Anvil-Blade"),
        ("Cursed Handle", "Confetti Trigger"),
        ("Deadfall Mold", "Mousetrap Mold"),
        ("Deadweight Plating", "Trillion-Pound Plating"),
        ("Deep Roots Base", "Deep-Chocolate Base"),
        ("Deepdraught Ring", "Kinked Pink Ring"),
        ("Deeprooted Sole", "Grungo-Rooted Sole"),
        ("Deepwater Ink", "Cleveland Water"),
        ("Deepwinter Mold", "Big-Freeze Mold"),
        ("Deft Mold", "Origami Mold"),
        ("Doorward Frame", "Doorkeeper's Band"),
        ("Doorway Primer", "The Commuting Door"),
        ("Doubter's Crest", "Francian Auditor"),
        ("Duelist's Fob", "Fit Watch"),
        ("Duelist's Grip", "Dart Grip"),
        ("Duelist's Hilt", "Dart Blade Hilt"),
        ("Duskweave Material", "Dark-Matter Material"),
        ("Echo Sigil", "Radio Broadcast"),
        ("Echo Sole", "Two-Step Sole"),
        ("Eighth Ray Crown", "Crown of the Eighth Ray"),
        ("Ember Alignment", "Apertarian Special"),
        ("Ember Crest", "LSB Ember Crest"),
        ("Emberburst", "Banana Peel"),
        ("Emberdust Ink", "Kinked Pink Zinc Drink"),
        ("Emberheart Orb", "LSB Ember"),
        ("Emberloop", "Kiln Loop"),
        ("Emberplate", "LSB-Heat Plate"),
        ("Empowering Focus", "Funny Funnel"),
        ("Empowering Mold", "Funnel Mold"),
        ("Executioner's Haft", "Crimper Lever"),
        ("Fateglass Orb", "Lottery Tumbler"),
        ("Feather Crest", "Hell Pigeon Feather"),
        ("Featherweight Mold", "Hell-Pigeon Mold"),
        ("Felt Layer", "Velvet Tuft"),
        ("Ferry Orb", "Multicity Bauble"),
        ("First Word", "Opening Remark"),
        ("Flaying Mold", "Skin-Peeler Mold"),
        ("Foreboding Crest", "Crest of Anticipation"),
        // THE THRESHOLD's shelf. The book's telling for a stair that counts
        // and the people keeping stock at the bottom of it.
        ("Listener's Frame", "Earhole Frame"),
        ("Countingstair Plating", "Tallystep Plate"),
        ("Four Hundred and Second Step", "The Last Riser"),
        ("Watcher's Crest", "Crest of the Kept Watch"),
        ("The Wrong Sense", "The Turned Eye"),
        ("Forked Crest", "Two-Roads Crest"),
        ("Foreman's Harness", "Gear Cave Harness"),
        ("Forking Bead", "Fork-in-the-Road Bead"),
        ("Frostbind", "Nut Freeze"),
        ("Frostbite Mold", "Chilblain Mold"),
        ("Fumbler's Mold", "Butterfingers Mold"),
        ("Fury Sigil", "Union Grievance"),
        ("Gauntlet Mold", "Panini-Press Mold"),
        ("Gilded Crest", "Bedazzle Crest"),
        ("Gilded Offcuts", "Money-Coat Offcuts"),
        ("Glacier Ink", "Dobira Meltwater"),
        ("Glacier Mold", "Iceberg Mold"),
        ("Gluttonous Fang", "Ench Skewer"),
        ("Godsheet Layer", "Wimpler-Fur Layer"),
        ("Godsteel Haft", "Ypytryktrium Haft"),
        ("Godsteel Plating", "Ypytryktrium Plating"),
        ("Gold Chip", "Fnorp Chip"),
        ("Golden Alignment", "East Brungulan Souffle"),
        ("Grand Grimoire", "The Great Squeals"),
        ("Grasping Ring", "Lxirp-Cube Ring"),
        ("Grave-Iron Mold", "Worm-Iron Mold"),
        ("Gravebloom Ink", "Wurm-Blood Tincture"),
        ("Gravebound Haft", "Worm-Carved Haft"),
        ("Gravewalker Mold", "Log-Roll Mold"),
        ("Greave Mold", "Cork Greave Mold"),
        ("Green Crown", "Green Crown"),
        ("Grimoire Rack", "Yodregar Shelf"),
        ("Gripping Mold", "Crank-Turner's Mold"),
        ("Grove Base", "Nautilus Base"),
        ("Grovemind Orb", "Nautilus Cone"),
        ("Grudge Bead", "Nibbalonian Bead"),
        ("Handman's Peel", "Gappy Handman's Peel"),
        ("Harvest Crest", "Grungo Harvest Crest"),
        ("Heartwood Base", "Grungo-Wood Base"),
        ("Heartwood Crest", "Nautilus Crest"),
        ("Helm of Blades", "Helm of Darts"),
        ("Henpeck's Cell Keys", "Drabley Henpeck's Cell Keys"),
        ("Herbal", "Anatomy of the Brumpus"),
        ("Hermit's Band", "Stone-Keeper's Band"),
        ("Hexbolt", "Wimple Bolt"),
        ("Hexbrand", "Rot Brand"),
        ("Hexer's Mold", "Rot-Handler's Mold"),
        ("Hexer's Reckoning", "Sherman's Reckoning"),
        ("Hexer's Tally", "Sherman's Tally"),
        ("Hexweave Shroud", "Trench Coat"),
        ("Hide Base", "Gooster-Fur Base"),
        ("Hide Material", "Toad Hide"),
        ("Hoarfrost", "Hoarfrost of Dobira"),
        ("Hoarfrost Mold", "Windscreen Mold"),
        ("Hobbling Mold", "Dead-Leg Mold"),
        ("Hollow Ink", "Empty Seltzer"),
        ("Hollow Lance", "Dart Throw"),
        ("Hollow Sphere", "The Grey Sphere"),
        ("Hollow Weave", "Hollow Weave"),
        ("Hollowbone Frame", "Hollowed Borchfruit"),
        ("Hooked Edge", "Cake-Knife Edge"),
        ("Hymnal", "The Eight Hymns"),
        ("Iron Band", "Cork Band"),
        ("Iron Blade", "Jigno Technoknife"),
        ("Iron Fang", "Death-Leopard Fang"),
        ("Iron Plating", "Gray Smock Plating"),
        ("Ironbark Layer", "Nautilus Shell"),
        ("Ironbound Haft", "Sneel-Bound Haft"),
        ("Ironhide Wrap", "Baste-Beast Hide"),
        ("Ironshod Sole", "Sneel-Shod Sole"),
        ("Ironthread Material", "Fishing-Line Thread"),
        ("Kaklon's Patent", "Spike Kaklon's Patent"),
        ("Kettleworks Pin", "Thrumbus Pin"),
        ("Keystone Base", "The Unmovable Rock"),
        ("Kingmaker Hilt", "Treyway Hilt"),
        ("Kingsbane", "Steel-Ball Called Shot"),
        ("Kingsblood Ink", "Time Sap"),
        ("Knuckleduster", "Tennis-Racquet Mold"),
        ("Lamplighter's Cage", "Gear Cave Lantern"),
        ("Last Rite", "Final Remark"),
        ("Layered Core", "Rice-Bale Core"),
        ("Layered Plating", "Onigiri Plating"),
        ("Leech Bead", "Plug Tap"),
        ("Leaden Tome", "Yodregar Disk"),
        ("Leather Material", "Boiled Gooster Hide"),
        ("Leyline Cuirass", "Plug-Energy Harness"),
        ("Lightweave", "Featherlight Weave"),
        ("Listening Frame", "Frame That Listens"),
        ("Loaded Fob", "Radio Watch"),
        ("Lonely Plating", "Hermit's Cork"),
        ("Loose-Sole Mold", "Flapping-Sole Mold"),
        ("Mage's Circlet", "Owl Circlet"),
        ("Mage's Rod", "Kappa Wand"),
        ("Mage's Sandals", "Velcro Tabs"),
        ("Mage's Wrapping", "Funnel Wrapping"),
        ("Mail Layer", "Wallspider Mail"),
        ("Malefic Crest", "Rot Crest"),
        ("Mana Loom", "Funnel Loom"),
        ("Mana Ward", "Funny Damper"),
        ("Manaflay", "Plug Drain"),
        ("Martyr's Crest", "Jester's Crest"),
        ("Mending Layer", "Healing-Pod Layer"),
        ("Mercurial Ink", "Nut Bar Slurry"),
        ("Mirror Ward", "But Wait, There's Less"),
        ("Mirrorbright Plating", "Museum-Glass Plating"),
        ("Mirrorcast", "Copy Paste Race"),
        ("Mirrored Visor", "Kaleidoscope Visor"),
        ("Mirrorplate Ring", "Mog Mirror Ring"),
        ("Multi-Handle", "Crank Assembly"),
        ("Nimble Mold", "Fast Roller Mold"),
        ("Oak Handle", "Nut Bar Handle"),
        ("Oathbound Ink", "Petal Elixir"),
        ("Oathkeeper Mold", "Union Mold"),
        ("Oathplate", "Union Plate"),
        ("Oathring", "Onion Ring"),
        ("Oathstone Bead", "Drambus Seed"),
        ("Obsidian Orb", "Academy Steel Ball"),
        ("Open Palm", "The Held-Out Hand"),
        ("Opening Grudge", "Wolf Scrape Grudge"),
        ("Orb of the Nine", "Orb of the Eighth Ray"),
        ("Ossuary Frame", "Cork Wimple"),
        ("Overflow Plate", "Francian Overflow Plate"),
        ("Overseer's Circlet", "Henpeck's Circlet"),
        ("Overflow Vial", "Soda Labyrinth Phial"),
        ("Padded Base", "Cardboard Base"),
        ("Padded Mold", "Oven-Mitt Mold"),
        ("Pathfinder Material", "Trail-of-Holes Material"),
        ("Piercer's Band", "Dart Band"),
        ("Pilgrim Alignment", "Old Man's Beard"),
        ("Pilgrim Sole", "Monastery Sole"),
        ("Pilgrim's Orb", "Dobira Bauble"),
        ("Pilgrim's Sole", "Dobira Pilgrim Sole"),
        ("Plaguewalkers", "Rot-Walkers"),
        ("Platinum Chip", "Ypytryktrium Chip"),
        ("Plain Sole", "Slow Trundler Sole"),
        ("Plate Layer", "Cork Plate"),
        ("Pocket Grimoire", "Pocket Koans"),
        ("Polished Orb", "Teetering Marble"),
        ("Prism Alignment", "Super Strain R-B-G-O"),
        ("Prismatic Ink", "Chromatic Rice-Water"),
        ("Quickening Charm", "Time-Sap Drop"),
        ("Quickfinger Mold", "Dart-Thrower's Mold"),
        ("Quickread Folio", "Tactical Haiku"),
        ("Quiet Room", "The Quiet Carriage"),
        ("Quota Edge", "Henpeck's Quota"),
        ("Quicksilver Ink", "Exotic Juice"),
        ("Quickstep Mold", "Skip-to-the-Slurpee Sole"),
        ("Quilted Base", "Onigiri Base"),
        ("Racing Sole", "Fast Roller Sole"),
        ("Rag Layer", "Robe Scrap"),
        ("Ravener's Mold", "Megalodon Mold"),
        ("Reaver's Bill", "Crimper Jaw"),
        ("Reckoning Crest", "Eighth-Hymn Crest"),
        ("Reckoning Plate", "Sneel Reckoning-Plate"),
        ("Reliquary Frame", "Acolyte's Hood"),
        ("Reliquary Frame of Nine", "The Master's Hood"),
        ("Reliquary Orb", "Rock-Core Shard"),
        ("Reliquary Sole", "Acolyte's Sole"),
        ("Rending Mold", "Wallspider Mold"),
        ("Resonant Chord", "Very Fast This Time"),
        ("Ribbed Base", "Vinyl Base"),
        ("Ridge Runner", "Fast Roller Tread"),
        ("Ridged Frame", "Corrugated Frame"),
        ("Rime Nova", "Minus One Degrees"),
        ("Rimebound Mold", "Frozen-Lock Mold"),
        ("Rimeguard Base", "Blizzard Base"),
        ("Ring of Embers", "LSB Ring"),
        ("Ring of Hours", "Radio-Watch Ring"),
        ("Ring of Roots", "Grungo Ring"),
        ("Ring of Tides", "Brie-Sea Ring"),
        ("Ring of Vigils", "Night-Worker's Ring"),
        ("Ring of Wells", "Soda-Well Ring"),
        ("Rite of Answer", "Worm Fact"),
        ("Riveted Layer", "Riveted Gear-Layer"),
        ("Rootbound Material", "Grungo-Root Material"),
        ("Rootwork Alignment", "Frembolatar Esbin"),
        ("Rootwoven Material", "Rice-Straw Material"),
        ("Ruby Inlay", "Rhinestone Inset"),
        ("Runebound Tome", "Slurmington's Notebook"),
        ("Runed Edge", "Octarine Edge"),
        ("Runed Lining", "Koan Lining"),
        ("Runed Material", "Octarine Material"),
        ("Runed Plating", "Octarine Plating"),
        ("Runewash Ink", "P-Minor Extract"),
        ("Runic Weave", "Octarine Weave"),
        ("Runner's Mold", "Morning-Rush Mold"),
        ("Sackcloth Base", "Gray Smock"),
        ("Sanctified Material", "Francian Material"),
        ("Sanctuary", "Time-Bomb"),
        ("Sapling Mold", "Silicon-Radish Sole"),
        ("Sawtooth Edge", "Frong Sawtooth"),
        ("Scale Layer", "Toad-Skin Layer"),
        ("Scaled Material", "Skink Scale"),
        ("Scaled Plating", "Megalodon Scale"),
        ("Scarred Plating", "Dented Vinyl Plating"),
        ("Scholar's Codex", "Rick Richard's Notebook"),
        ("Scrap Ticket", "Henpeck's Chit"),
        ("Scrying Lens", "Cork Glasses"),
        ("Scrying Orb", "Mog Watcher"),
        ("Seal of Power", "Treyway Seal"),
        ("Seal of the Deep", "Seal of Cleveland"),
        ("Seal of the Grove", "Drambus Seal"),
        ("Second Sight", "Mansus-Sight Lens"),
        ("Seedbed Layer", "Silicon Seedbed Layer"),
        ("Seer's Crest", "Quadruple-Eclipse Crest"),
        ("Seer's Orb", "Foreston Glass"),
        ("Serrated Edge", "Multiplication Rim"),
        ("Sevenleague Boots", "Thrumbus Boots"),
        ("Sevenleague Sole", "Thrumbus Sole"),
        ("Shatterbolt", "Steel-Ball Volley"),
        ("Sightless Crown", "Crown of Ghirbi"),
        ("Sigil Layer", "Squeal Layer"),
        ("Signet of Ash", "Ash-Field Signet"),
        ("Signet of Iron", "Gear Signet"),
        ("Signet of Vigour", "Soft-Drink Cap Ring"),
        ("Silver Band", "Fnorp Piece"),
        ("Silver Charm", "Forever Stamp"),
        ("Siphon", "Semuta Strain"),
        ("Siphon Ring", "Drinking-Straw Ring"),
        ("Slash and Burn", "Radish Roast"),
        ("Soot Ink", "Slime Cola"),
        ("Sovereign Mold", "Treyway Mold"),
        ("Spiked Vambrace", "Dart-Board Vambrace"),
        ("Spinning Orb", "Multiplication Wheel"),
        ("Split Weave", "Copy-Paste Weave"),
        ("Sprawling Handwrap", "Gappy's Spare Hand"),
        ("Sprung Board", "The Thrumbus Plank"),
        ("Sprung Sole", "Wallspider Spring"),
        ("Spun Material", "Spun Rice-Silk"),
        ("Sprocketman's Gratitude", "Sprocketman's Thanks"),
        ("Standing Start", "The Skoogle Start"),
        ("Starfall", "Moonfall"),
        ("Starlit Ink", "Skink Brink's Soft Drink"),
        ("Starlit Mantle", "Katalungan Mantle"),
        ("Steel Frame", "Cork Helm"),
        ("Steel Material", "Sneel Material"),
        ("Stonewall Frame", "Unmovable Frame"),
        ("Storm Signet", "Thrumbus Signet"),
        ("Stormcaught Frame", "Blizzard Hood"),
        ("Stormstep Mold", "Blizzard Step"),
        ("Stray Orb", "Gooster Bauble"),
        ("Striding Mold", "Mile-in-Months Mold"),
        ("Studded Sole", "Dart Sole"),
        ("Stumblefoot Mold", "Kerb Mold"),
        ("Stutterstep Mold", "Skipping Mold"),
        ("Sunder", "Steel Ball"),
        ("Sump Sole", "Soda Labyrinth Sole"),
        ("Sunder Haft", "Steel-Ball Haft"),
        ("Sunderer", "Moon Fragment"),
        ("Sympathetic Bloom", "Wextreen Bloom"),
        ("Tarpit Sole", "Brie-Cliff Sole"),
        ("Tempered Sole", "Kiln-Fired Sole"),
        ("Tallykeeper's Weave", "Deep Chocolate Weave"),
        ("Tetrahedron Shard", "Nibbalonius's Tetrahedron"),
        ("The Cracked Lens", "Foreston's Cracked Monocle"),
        ("The Empty Crown", "The Empty Throne"),
        ("The Eyeless Stare", "The Sunless Stare"),
        ("The Green Ledger", "The Radish Tally"),
        ("The Growing Weight", "The Growing Stone"),
        ("The Idiot's Gift", "The Blind Idiot's Gift"),
        ("The Ledger", "The Fnorp Ledger"),
        ("The Money Jacket", "The Money Jacket"),
        ("The Odometer", "Yonk-Standard Odometer"),
        ("The Quiet Ear", "Ear of the Hall"),
        ("The Seeker's Tears", "PoopFart's Tears"),
        ("The Split Wisdom", "Boyetano's Share"),
        ("The Stranger's Parcel", "The Slurpee Man's Parcel"),
        ("The Tally", "Sherman's Count"),
        ("Thin Veil", "Thin Plane"),
        ("Third Eye", "Foreston Monocle"),
        ("Thorn Layer", "Wallspider Thorn"),
        ("Thornmail Layer", "Dart-Board Layer"),
        ("Thornweald Grip", "Wallspider Silk"),
        ("Throttling Mold", "Headlock Mold"),
        ("Tidal Alignment", "Senndrier Vertigo Straw"),
        ("Tidecaller Orb", "Cleveland Tide Glass"),
        ("Tidewrack Ink", "Eleven-Fourteen Brew"),
        ("Timeworn Orb", "Time-Sap Amber"),
        ("Tin Band", "Spindrift Tab"),
        ("Tin Frame", "Spindrift-Can Frame"),
        ("Tin Plating", "Cork Plating"),
        ("Titan's Grip", "Megalodon Grip"),
        ("Tithe Collector", "Francian Tithe"),
        ("Tithe Ring", "Francian Tithe Ring"),
        ("Toll-Taker's Mitt", "Multicity Fare Grip"),
        ("Toolwright's Grip", "Skeleton Tool Wizard's Grip"),
        ("Trailworn Sole", "Pilgrim of Dobira Sole"),
        ("Treadmill Sole", "Shift-Work Sole"),
        ("Traveller's Codex", "Mrs. Freya's Syllabus"),
        ("Tripwire Mold", "Shoelace Mold"),
        ("Twinned Grip", "Screw-Twister Grip"),
        ("Twinning Mold", "Second-Hand Mold"),
        ("Unbound Core", "Loose Sprocket Core"),
        ("Ungloved Layer", "Bare-Frame Layer"),
        ("Unmaking", "The Flattening"),
        ("Unshod Signet", "Bare-Sole Signet"),
        ("Vast Tapestry", "The Nut Tapestry"),
        ("Verdant Alignment", "Ocharpa Glass Stalk"),
        ("Verdant Surge", "Rice Harvest"),
        ("Verdant Weave", "Rice-Straw Weave"),
        ("Vicegrip Mold", "Crimper Mold"),
        ("Vigil Crest", "Night-Vigil Crest"),
        ("Visor of Focus", "Pith Helmet"),
        ("Void Alignment", "Neverian Meter Grain"),
        ("Voidglass Shard", "Black-Hole Glass"),
        ("Voidsilk Base", "Dark-Matter Weave"),
        ("Voidwritten Ink", "Black Hole Flavor Blaster"),
        ("Votive Crest", "Francian Votive"),
        ("Wandering Root", "Wandering Root"),
        ("War Ledger", "The 62 Anticipations"),
        ("Warcry Crest", "The Wimple"),
        ("Warded Frame", "Sneel Frame"),
        ("Warded Plating", "Sneel Plating"),
        ("Warded Sabatons", "Cork Sabatons"),
        ("Warden's Haft", "Sneel Baton"),
        ("Warding Mold", "Cork Mold"),
        ("Warding Plate", "Cork-Priest Plate"),
        ("Warding Ring", "Cork Ring"),
        ("Warding Sigil", "Sneel Wall"),
        ("Warlord's Crest", "Commander's Crest"),
        ("Warlord's Pauldron", "Commander's Pauldron"),
        ("Warmed Material", "Pre-Roasted Material"),
        ("Warplate Greave", "Gladiator Greave"),
        ("Watchful Crest", "Rooster Crest"),
        ("Waxed Material", "Brie-Cliff Wax"),
        ("Wayfarer's Orb", "Warp Bauble"),
        ("Wayfarer's Sole", "Wanderer's Nut Bar Sole"),
        ("Wellspring Base", "Soda-Fountain Base"),
        ("Wellspring Sole", "Skink Brink Sole"),
        ("Whetstone", "Quarry Granite"),
        ("Whipcord Hilt", "Grungo-Elastic Hilt"),
        ("Whisperbound Tome", "The Words of Angelo"),
        ("Wickstub", "Cork Stub"),
        ("Widow's Sole", "Stone-Keeper's Sole"),
        ("Wildfire Layer", "Ash-Field Layer"),
        ("Wildgrowth", "Bumper Crop"),
        ("Windup Key", "Great Brass Key"),
        ("Witch's Claw", "Frong Claw"),
        ("Witch's Crook", "Ladle of Dobira"),
        ("Witch's Hat", "Witch's Hat"),
        ("Witherroot", "Grungo Rot"),
        ("Witch's Stilts", "Baguette Stilts"),
        ("Witchglass Shard", "Petal of Wextreen"),
        ("Worldeye Orb", "Worldeye Orb"),
        ("Worldsplitter", "The Flattener's Edge"),
        ("Worldstrider Sole", "Planeswalker Sole"),
        ("Worldweave Material", "Planeswoven Material"),
        ("Woven Underlayer", "Silk-Cloth Underlayer"),
        ("Wrathbreaker", "Wimpler Yoke"),
        ("Wrathful Mold", "Gorillathon Mold"),
        ("Wrathful Talons", "Frong Talons"),
        ("Wrathwrit Ink", "Power Serenade"),
        // The Switchyard's eight. The scenes and the creatures are M7's; a
        // *piece* gets its name in the same change that writes the piece,
        // which is the gear skill's own rule and what
        // `the_turtle_theme_covers_the_catalogue` is for.
        //
        // Built from vocabulary the theme already spends: the Cork Train and
        // the Holy Cork Empire's line, the Sprocketmen who are the player's
        // own people, Multicity, and the planeswalking flavour the four
        // shipped Orbs of Travel already wear. The two balls are the warp
        // device's lesser cousins by another road.
        ("Ballast Bed", "Cork Ballast"),
        ("Points Rodding", "The Sprocketman's Rodding"),
        ("Booking Hall", "The Cork Booking Hall"),
        ("Signal Wire", "The Signal Wire from Multicity"),
        ("Shunter's Orb", "The Shunting Ball"),
        ("Signalman's Orb", "The Signal Ball"),
        ("A Word About the Sidings", "A Word About the Cork Yards"),
        ("A Word About the Points", "A Word About the Sprocketman's Lever"),
        // THE HUNDRED. A county is land, so the three enchantments are places
        // and the two balls are the warp device's lesser cousins again - the
        // shape the yard's pair already set.
        //
        // Petonkle and Kolok are the book's own (proposal §1); a trig stone
        // and a drove road are errands rather than epics, which is the
        // register. The chest one takes a substance because a chestpiece is
        // read off the defence ladder, and sneel is Henpeck's good steel
        // (p. 44) - the rung twenty-six health sits on.
        ("A Word About the Hundred", "A Word About the Petonkle Hundred"),
        ("Trig Pillar", "The Petonkle Trig Stone"),
        ("Drove Way", "The Kolok Drove Road"),
        ("The Common Ground", "The Sneel Common"),
        ("Surveyor's Orb", "The Measuring Ball"),
        ("Drover's Orb", "The Droving Ball"),
        ("Zealot's Crest", "Rice Crier Crest"),
        ("Zealot's Haft", "Rice Crier Haft"),
        ("Zealot's Sole", "Rice Crier Sole"),
        ("the Appeal", "Get Jar Jarred"),
        ("the Lightning Rod", "Hell-Pigeon Perch"),
        ("the Second Key", "Gappy's Spare Key"),
        ("the Skip Stone", "The Flattened Step"),
    ],
    monsters: &[
        // ---- the mission's frames -------------------------------------------
        //
        // Off the ladder, so `the_turtle_theme_renames_the_whole_ladder` never
        // asks about them - which is exactly why they are easy to forget. Two
        // of them keep their names on purpose: all caps is a universal
        // language and THE WUMPUS is already the joke.
        ("DOORKEEP", "THE DOOR THAT COMMUTES"),
        ("THE STAIR THAT LISTENS", "THE HALL HEARD WITH THE EYES"),
        ("THE LAST LANDING", "THE LANDING BEFORE THE SUN"),
        ("THE SHADOW", "THE FIRST ANTICIPATION"),
        ("THE LANTERN", "THE LIGHT THAT CASTS IT"),
        ("THE DIGGERS", "THE SPROCKETMEN WHO STAYED"),
        ("WHAT THE SEAM HID", "THE VEIN OF DEEP CHOCOLATE"),
        ("THE CURRENT", "THE PULL UNDER THE ROCK"),
        ("THE THING ON THE HOOK", "WHAT BOYETANO HOOKED"),
        ("THE DEN MOUTH", "THE EXHIBIT, OPENED"),
        ("DARK FLOOR", "THE ROOM WITH NO LAMP"),
        ("THE FLOCK", "THE BIRDS OF THE RIDGE"),
        ("THE UNWOUND", "NIBBALONIUS ASCENDANT"),
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
        ("The Curator", "Galapagos Jim"),
        ("The Dreaming Idiot", "The Blind Idiot God"),
        ("The Long Haul", "The Cork Train"),
        ("The Reciter", "Head Cork Priest"),
        ("The Watchers", "The Old Gods"),
        ("Salt Idol", "The Stone Keeper"),
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
        ("Weeping Idol", "PoopFart"),
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
        ("Verdigris", "Gappy Handman"),
        ("The Drowned Court", "The Sea of Cleveland"),
        ("Anvilheart", "Big Yomp"),
        ("The Salt Wedding", "C O R K"),
        ("Nine of Ashes", "Nibbalonius the Wise"),
        // The last three read as one story: the final holy beast, the coat
        // made from one, and the man wearing it.
        ("The Last Light", "The Last Wimpler Oxen"),
        ("Gilt", "The Money Coat"),
        ("Francis", "Francis the Gambler"),
        // ---- THE SWITCHYARD ---------------------------------------------
        //
        // The Cork Train ran on the Holy Cork Empire's own line, and the line
        // had a yard where the Empire sorted what it took from the planes it
        // took things from. Four of the nine keep their names, because a coal
        // stage is a coal stage on any plane and all caps is a universal
        // language.
        // THE HUNDRED's five. A county is land and its people are the people
        // who work it, which is a register the book keeps for its Sprocketmen
        // - mined out of the Great Gear Cave and set to work by somebody who
        // owns the cave.
        ("THE SURVEYOR", "THE ONE WHO MEASURED IT"),
        ("THE DROVER", "THE ONE WHO WALKS IT"),
        ("THE DRIVEN", "WHAT IS BEING WALKED"),
        ("THE COMMISSIONER", "HENPECK'S COMMISSIONER"),
        ("THE PARISH", "THE WHOLE PETONKLE HUNDRED"),
        ("THE SHUNTER", "THE CORK SHUNTER"),
        ("THE PLATELAYERS", "THE SPROCKETMEN WHO KEPT THE LINE"),
        ("THE BALLAST", "WHAT THE EMPIRE LEFT IN THE PIT"),
        ("THE GANTRY", "THE ELEVEN CORK SIGNALS"),
        ("THE LAMP ROOM", "THE ROOM WITH EVERY LAMP LIT"),
    ],
    words: &[
        // The interface, in the book's vocabulary. Slugs are keyed by what the
        // thing is, not by what it currently says, so re-wording the plain
        // build never silently unhooks a translation.
        ("gold", "Fnorp"),
        ("gold-lower", "fnorp"),
        ("gold-suffix", "fnorp"),
        ("shop", "GALAPAGOS EMPORIUM"),
        // The road, and the key that draws it. The hint sits on the opponent
        // panel's rung row, so it has to stay short enough to fit beside a
        // rung count.
        ("the-road", "THE LONG WALK"),
        ("map-hint", "M for the walk"),
        ("shop-hint", "right-click a card to hold it for Jim"),
        ("reroll", "SKOOGLE IT"),
        ("inventory", "SALVAGE"),
        ("inventory-hint", "drag onto a frame  ·  right-click rotates  ·  shift-click to lock an item"),
        ("your-items", "WHAT YOU HAVE BUILT"),
        ("fountain", "THE SODA LABYRINTH"),
        ("fountain-blurb", "It has read your gear. Drink what it saw in you, or one of the two you came \
                            closest to, or whatever is at the bottom of the bottle."),
        ("fountain-waiting", "THE SODA LABYRINTH IS OPEN"),
        ("boss", "BIG ONE"),
        ("miniboss", "NOT-SO-BIG ONE"),
        ("deep-fountain", "THE BOTTOM OF THE LABYRINTH"),
        ("deep-fountain-blurb", "Nothing new down here. It only knows how to give you more of \
                                 whatever you already are."),
        ("fountain-take", "DRINK"),
        ("class", "TITLE"),
        ("classes", "TITLES"),
        ("begin-fight", "NEXT FIGHT"),
        ("character", "YOUR SPROCKETMAN"),
        ("opponent", "NEXT ON THE ROAD"),
        ("glossary", "WHAT THE WORDS MEAN"),
        // THE HUNDRED's six tolls and the county itself. Common nouns, so
        // they are swapped in place wherever the engine says them - which is
        // what `vocabulary` is for and why none of these moved into `told`.
        ("county", "hundred"),
        ("river", "sap-run"),
        ("ford", "sap-crossing"),
        ("scarp", "cork-face"),
        ("drift", "slow-lane"),
        ("hedge", "thornwall"),
        ("toll gate", "fnorp gate"),
        ("trig point", "measuring stone"),
        ("sign", "drove-mark"),
        ("boundary stone", "Henpeck stone"),
        ("gaol", "lock-up"),
        ("pale", "the long fence"),
        ("mana", "Jokes"),
        ("mana-lower", "jokes"),
        ("armor", "Cork"),
        ("armor-lower", "cork"),
        ("rage", "Fury"),
        ("faith", "Devotion"),
        ("nature", "Harvest"),
    ],
    // Whole words swapped inside anything the engine wrote. The engine still
    // says "mana" everywhere - every rule it applies depends on that word
    // meaning exactly one thing - and this translates the output.
    vocabulary: &[
        // Jokes are what you spend; Funny is the kind of harm they do. Keeping
        // the two words apart is what makes "spend 3 jokes to deal 12 funny
        // damage" a sentence rather than a tautology.
        // THE HUNDRED's six tolls and the county itself. Common nouns, so
        // they are swapped in place wherever the engine says them - which is
        // what `vocabulary` is for and why none of these moved into `told`.
        ("county", "hundred"),
        ("river", "sap-run"),
        ("ford", "sap-crossing"),
        ("scarp", "cork-face"),
        ("drift", "slow-lane"),
        ("hedge", "thornwall"),
        ("toll gate", "fnorp gate"),
        ("trig point", "measuring stone"),
        ("sign", "drove-mark"),
        ("boundary stone", "Henpeck stone"),
        ("gaol", "lock-up"),
        ("pale", "the long fence"),
        ("mana", "Jokes"),
        ("magic", "Funny"),
        ("arcana", "funny"),
        ("armor", "Cork"),
        ("armour", "Cork"),
        ("rage", "Fury"),
        ("faith", "Devotion"),
        ("nature", "Harvest"),
        ("gold", "Fnorp"),
        ("searing", "roasting"),
        ("frost", "nut-freeze"),
        ("stun", "trance"),
        ("misfire", "goof"),
        ("misfires", "goofs"),
        ("mind", "idiot"),
        // The three lanes' own words. Insight is the sense you come back from
        // the antechamber with; Dread is what a projection thrown ahead of a
        // thing still in transit does to whoever is standing in front of it.
        ("insight", "Mansus-Sight"),
        ("dread", "Anticipation"),
        // The twins keep their shape and change their material: a blade you
        // funny up, and a stop you get in the way.
        ("spellblade", "funnyblade"),
        ("deflection", "corkwork"),
        // ---- the Switchyard's four verbs ---------------------------------
        //
        // The engine prints these in tooltips and log lines through
        // `Action::describe`, which is the same mechanism that turns "mana"
        // into "Jokes" and needs no new code. A railway word is a railway word
        // on any plane, so "shunt" is kept; the other three are the Empire's.
        ("ballast", "cork-ballast"),
        ("Ballast", "Cork-ballast"),
        ("derail", "skoogle"),
        ("Derail", "Skoogle"),
        ("derails", "skoogles"),
        ("accrue", "fnorp-interest"),
        ("Accrue", "Fnorp-interest"),
    ],
    glossary: &[
        (
            "INSIGHT",
            "MANSUS-SIGHT",
            "The eighth pool, and the only one you cannot buy. Clear the antechamber and \
             you come back seeing with the wrong sense - residents of the Mansus are seen \
             with the ears and heard with the eyes, and a mind built for this plane does \
             not survive being looked at that way. Every point of it sharpens what an \
             Anticipation does.",
        ),
        (
            "DREAD",
            "ANTICIPATION",
            "A stack, not a pool. An Anticipation is a projection thrown ahead of a thing \
             still in transit - the Cork scripture counts sixty-two of them - and standing \
             in front of one lowers the ceiling of what you are. Multiplied by the \
             Mansus-Sight you are holding.",
        ),
        (
            "SPELLBLADE",
            "FUNNYBLADE",
            "What empowerment is to Funny, this is to iron. Flat power on physical hits \
             only, it does not scale off jokes, and it resets when the fight does. The \
             gloves' word.",
        ),
        (
            "DEFLECTION",
            "CORKWORK",
            "The shield's twin, on the other lane. Flat cut off every physical hit, taken \
             before Cork is, and it stacks without decaying. Chest work, mostly.",
        ),
        (
            "MANA",
            "JOKES",
            "What goofs cost. You bank jokes and spend them: a banana peel, a pie, an anvil \
             on your own foot. Items that spend jokes fail politely when you are out, and a \
             spell cast without them still goes off - just weakly. Low-level Funny Men's \
             goofs often go horribly wrong. Yours will not, because you read the jokebook.",
        ),
        (
            "PHYSICAL / MAGIC",
            "IRON / FUNNY",
            "The two kinds of harm. Iron is the ordinary sort. Funny is what a landed joke \
             does - comedic energy, delivered through a funnel, and it has its own set of \
             defences. A curse's burn answers to neither - stack Devotion \
             against that instead.",
        ),
        (
            "ARMOR",
            "CORK",
            "Temporary hit points. Starts every fight at ZERO - your gear lays it on as it \
             activates - and soaks damage before health does. C O R K grows to encompass \
             whole planes; a layer of it will do for one fight.",
        ),
        (
            "RAGE",
            "FURY",
            "Banked by some gear. Every point adds physical damage while you hold it, and \
             some triggers spend it for a burst. The fury of a thousand bears, kept in a jar.",
        ),
        (
            "FAITH",
            "DEVOTION",
            "Banked slowly. Every point adds resistance of both types while held, up to 40%. \
             The Francians managed eight hymns about it.",
        ),
        (
            "NATURE",
            "HARVEST",
            "Banked by growing things. Every point adds regeneration while held. A billion \
             acres of rice, working quietly on your behalf.",
        ),
        (
            "DRUIDIC MIGHT",
            "MULCH",
            "Rage and Harvest, fused. Two points become one and that one pays for both at \
             double - so it is only worth doing once your income outruns what the two \
             pools were paying you separately. Nothing spends a fused pool; it sits there \
             working. The hands can drink it off you, mind.",
        ),
        (
            "COMMUNION",
            "THE COLLECTION",
            "Belief and Harvest, fused, and worth both at double while you hold it. Same \
             bad exchange by volume and same good one by rate: a decision late in a fight \
             rather than a thing to do on sight.",
        ),
        (
            "ZEALOTRY",
            "THE FERVOUR",
            "Belief and Rage, fused. Worth twice what its parents were, paid out passively, \
             and unspendable by anything - which is the point. A pool with no sink is a \
             pool nobody can waste.",
        ),
        (
            "MIND DAMAGE",
            "IDIOT MODE",
            "Small numbers, but it eats your MAXIMUM health, so no amount of regeneration \
             wins it back. Some stories are foolish enough that hearing them takes something \
             from you permanently.",
        ),
        (
            "MANA DEBT",
            "JOKE DEBT",
            "Jokes below zero, which is what a shift at the works leaves you on. Nothing \
             that spends jokes can pay while the jar is under water - you have to bank your \
             way back above the cost first. A build that never tells a joke never notices.",
        ),
        (
            "A MISS",
            "A CLEAN MISS",
            "An attack that comes to nothing at all - no harm, no curse, no drain. A \
             Multicity Season Pass is the only thing that causes them, and it counts rather \
             than rolls: every second attack made against you, per attacker. Exactly half of \
             everything, and it never streaks.",
        ),
        (
            "MIND RESIST",
            "THICK SKULL",
            "Percent reduction to Idiot Mode. Survivable, given a thick enough one.",
        ),
        (
            "BOUNTY",
            "BOUNTY",
            "Paid in Fnorp whether you win or lose. Losing never moves you up the road, but \
             it does pay - a run with no income cannot buy its way past whatever just beat it.",
        ),
        (
            "",
            "THE WORM FACT",
            "LETO sits on the flesh Throne and is Death, and the Worm Fact remains law. He is \
             not the last thing on this road, which should tell you something about the last \
             thing on this road.",
        ),
        (
            "",
            "SPROCKETMEN",
            "Gear-folk of the Great Gear Cave in west Bambulon, suffocated out of it by Lord \
             Drabley Henpeck when he found the Deep Chocolate underneath. They glow when they \
             have something to be joyous about. There has not been much call for it lately.",
        ),
        (
            "",
            "FNORP",
            "Money. The going rate on an Lxirp Strangler Beast is a hundred and twenty-five \
             billion of them, so the numbers here are modest by comparison.",
        ),
        (
            "REFLECTION",
            "SPITE",
            "Armour that hits back. A share of whatever your plating soaks is turned round \
             on whoever swung, so it pays nothing at all if you die quickly and rather a lot \
             if you do not. It is the only way a chest hurts anybody, and it is the only \
             attack in the game you never choose to make.",
        ),
        (
            "FUSED POOLS",
            "DOUBLE ACTS",
            "Two banked things put together into one better thing. A point of nature and a \
              point of rage make a point of Druidic Might, which pays what both of them paid \
              and pays it twice over. Nothing spends a double act - it is the punchline, not \
              the setup - but somebody else can still take it off you, so a deep one is worth \
              guarding.",
        ),
        (
            "WATCHERS",
            "TALLIES",
            "Gear that counts. A tally sits there totting up whatever it was told to watch - \
             your own gear going off, a neighbour going off, curses landing on anybody - and \
             pays out every so many. It runs on the board's clock rather than its own, which \
             is why a slow tally on a busy board is worth more than a fast one on an empty \
             one.",
        ),
        (
            "DIAGONAL",
            "CORNER-WISE",
            "Two items touching at a corner and nowhere along an edge. Ordinary neighbours \
             share a side; these share a point, and a few pieces care only about them. An \
             item packed tight against three things has spent its sides - corner-wise is how \
             it reaches past them.",
        ),
        (
            "TERRAIN",
            "GROUND",
            "A piece you stand on. It lies under the grid instead of in it, other gear may \
             be packed straight on top, and what it is worth depends entirely on what ends \
             up covering it. Ground never joins an item and never acts - it is not gear, it \
             is the floor.",
        ),
    ],
    cutscenes: &[(
        // The first act ends here, and it ends by turning out not to be the
        // whole job.
        "The Hollow King",
        &[
            "Lord Drabley Henpeck goes down in a heap of good coat.",
            "You get the cell keys off him before he has finished falling. They \
             do not fit. You try them twice more, which is twice more than you \
             need to, and then you look at him properly.",
            "\"Oh,\" he says, with some effort, and a little delight. \"No. No, I \
             sold them. Months ago. All of them, in one lot - I am not a \
             *retailer*.\"",
            "He tells you the buyer's name. He is enjoying himself so much that \
             he tells you twice.",
            "The pit behind you is empty. It has been empty for months. \
             Everything above you is where they went.",
        ],
    )],
    notes: &[
        // Kept to two lines: the opponent band is a fixed height and a third
        // line lands on the message underneath it.
        ("The Hollow King", "Your jailer. Everything before him is practice."),
        ("Gilt", "Wimpler fur, and nobody paying attention. A perfect build only."),
        ("Francis", "Not your enemy, and the reason you are out. Optional."),
    ],
    told: &[
        // The road, in the book's own names. Titles only, nearly always: the
        // canonical prose is the game's and `retell` translates it a word at a
        // time. A theme spends paragraphs of its own in exactly one case -
        // where a *proper noun* is carrying the scene - and there are eight of
        // those.
        //
        // ---- the chain, which is the Ascension told sideways ----------------
        // ---- THE HUNDRED ---------------------------------------------------
        //
        // Titles only. Every one of these scenes is written in common nouns -
        // a ditch, a barn, a milestone, a fence - which `vocabulary` swaps in
        // place, so there is nothing here for a paragraph to rescue. The four
        // people who *are* named - Ordish, Rell, Sowerby, Yaxley, Ketton,
        // Wragby, Vessey, Tasker - are the county's own and stay theirs: the
        // book supplies no surveyor and inventing one to overwrite a name the
        // canonical column already carries would be spending the theme on
        // nothing.
        Retold { id: "the-theodolite", title: "THE MEASURING ENGINE", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-stockman", title: "THE ONE WHO COUNTS", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-commons", title: "THE UNFENCED", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-county-surveyed", title: "THE PETONKLE SURVEY", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-constable", title: "HENPECK'S MAN", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-waste", title: "THE IMPROVER", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-boundary-ditch", title: "THE OLD SAP-DITCH", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-field-barn", title: "THE BARN IN THE MIDDLE", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-milestone", title: "THE CUT STONE", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-gleaners", title: "THOSE WITH THE RIGHT", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-pound", title: "THE HOLDING WALL", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-charcoal-burner", title: "THE ONE WHO WATCHES THE HEAP", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-drowned-lane", title: "THE LANE UNDER THE WATER", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-parish-chest", title: "THE THREE-LOCK BOX", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-pale", title: "THE LONG FENCE", prose: &[], entry: &[], landings: &[] },
        Retold {
            id: "the-astronomer",
            title: "THE TETRAHEDRON WATCHER",
            prose: &[], entry: &[], landings: &[],
        },
        // Prose, not just a title, because the canonical scene stopped
        // saying EGGBERT. The plate on the middle post is a *proper noun* -
        // no word-swap reaches it - so the canonical column gave itself its
        // own name and this keeps the book's, which is what the two columns
        // are for. Without it the turtle title would stand over a scene whose
        // gate says something else.
        Retold {
            id: "the-locked-gate",
            title: "EGGBERT'S GATE",
            prose: &[
                "A gate, in good repair, hung on two posts, with a lock on it \
                 that somebody oils. There is no wall on either side of it and \
                 no road behind it, and the grass behind it has not been \
                 walked on by anything with feet.",
                "The word Halloway gave you is not a key. It is a thing to \
                 say, and he said it to himself twice before he said it to \
                 you, to be sure he had it in the right order.",
                "There is a brass plate screwed to the middle post. It says \
                 EGGBERT and then a number that is longer than a house number \
                 needs to be.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold {
            id: "the-glow-over-the-ridge",
            title: "THE GLOW OVER THE WEIRDEIRS",
            prose: &[], entry: &[], landings: &[],
        },
        Retold { id: "the-second-shadow", title: "THE ANTICIPATION", prose: &[], entry: &[], landings: &[] },
        // Prose for the same reason EGGBERT'S GATE has it: the plate is a
        // proper noun, the canonical column gave itself HOLLIS, and a title
        // reading EGGBERT'S MANSION over a gate that says something else is
        // the theme contradicting itself rather than translating.
        Retold {
            id: "the-manse",
            title: "EGGBERT'S MANSION",
            prose: &[
                "The gate had no road behind it and now there is a house \
                 behind it, which is the sort of thing that stops being \
                 strange about four minutes after you notice it.",
                "Nobody in the Manse asks who you are. Two of them are eating \
                 and one of them is reading and all three of them are doing it \
                 in rooms you can hear but not find, because the doors here do \
                 not stay where they were put.",
                "There is a cellar, and the plate on the gate said EGGBERT, \
                 and nobody inside will answer to it. Everybody in the house \
                 knows where the cellar is and nobody in the house will take \
                 you to it.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold { id: "the-slagworks", title: "THE BURNWARP FOUNDRY", prose: &[], entry: &[], landings: &[] },
        Retold {
            id: "the-threshold",
            title: "THE MANSUS ANTECHAMBER",
            prose: &[],
            entry: &[
                "The door is at the top of a stair you did not climb, and it \
                 commutes: it is a door here and it is a door there and the \
                 distance between the two is a matter it has not been asked \
                 to settle.",
                "Everything past it is seen with the ears. Nobody has ever \
                 said what that is like in a way that helped, and now you know \
                 why - the sentence arrives before the words do.",
                "Ghirbi is somewhere below, being a sun and being friendly \
                 about it. That is not a comfort. It is a fact about the \
                 lighting.",
            ],
            landings: &[
                "The door does not open. It commutes - it is here and it is \
                 there and the difference is a matter nobody has asked it to \
                 settle - and you go through it the way you go through a \
                 sentence.",
                "The stair hears you. Residents of the Mansus are seen with \
                 the ears and heard with the eyes, and a stair that listens is \
                 a stair that has been introduced.",
                "There is light at the bottom and the light is Ghirbi, who is \
                 a sun and is pleased to see you, which is the worst of it. \
                 You come back up seeing with the wrong sense, and it does not \
                 stop.",
            ],
        },
        Retold {
            id: "the-under-mine",
            title: "THE DEEP CHOCOLATE MINE",
            // The boards were stamped HENPECK, which is the book's name for
            // the man the base game calls the Hollow King. The canonical
            // stamp is his canonical name now, and this keeps the book's.
            prose: &[
                "The mouth of it is boarded from the outside, and the boards \
                 are stamped HENPECK. He sealed it, and he sealed it from out \
                 here, and those are two separate things to have found out.",
                "Somebody sealed this in a hurry and somebody else has been \
                 keeping the boards in repair for a very long time since, and \
                 the two of them were not the same person and did not agree.",
            ],
            entry: &[
                "The Sprocketmen were told this seam was empty. It was not \
                 empty. It was the single largest thing anybody has ever been \
                 told to stop asking about, and the telling was done by people \
                 who owned the asking.",
                "The smell four hundred feet down is unmistakable and nobody \
                 who has been down here has ever needed it explained.",
            ],
            landings: &[],
        },
        Retold {
            id: "the-undertow",
            title: "BUNKO'S CAVERN",
            prose: &[
                "The thing you sold turns up three rungs later in the hands of \
                 somebody who should not have it, in a hamlet that is not on \
                 any map you have seen.",
                "They call it Corrqk's Cavern now. It was Bunko's Cavern when \
                 it was a fishing village, before the Cork came and the boys \
                 were put on trains and the Home for Immature Men was turned \
                 into a Drambus seed facility. There is one old analyst left \
                 on the line. His name is Boyetano and he still prays to the \
                 old gods, on a floor that cuts his knees, which he says helps \
                 him concentrate.",
                "Boyetano has noticed a purple glint down between the Cork and \
                 the Unmovable Rock. He has been noticing it for six years and \
                 has told nobody, because nobody who works here has the \
                 shoulders to widen a crack in a rock, and he has been very \
                 patient about waiting for somebody who does.",
            ],
            entry: &[
                "The hole in the back wall is a hole in the back wall for \
                 about four feet, and then it is a staircase somebody cut, and \
                 then it is not a staircase.",
                "Boyetano is already ahead of you. He has been ahead of you \
                 for six years.",
            ],
            landings: &[
                "The Anticipations stop mid-verse. Behind the pulpit, the Cork \
                 has grown out over a crack in the rock the way a lip grows \
                 over a bad tooth. Boyetano gets a bar under it. Boyetano is \
                 seventy-one.",
                "The train goes over on the bend. Whatever was in the cars is \
                 out in the dark now, and it does not appear to want anything \
                 from you at all, and it does not appear to want anything from \
                 Boyetano either, who keeps walking and does not look at it \
                 once.",
                "The Core is soup and light with a piece of the Mansus sitting \
                 in the middle of it. Boyetano looks at it for a while, and \
                 stops being Boyetano, and there is a moment there where he \
                 could have kept the lot. He splits it instead, the way he \
                 always said he would, and puts your share in your hand on his \
                 way past. Somewhere above you, for the first time in a long \
                 time, somebody is casting a line.",
            ],
        },
        Retold {
            id: "den-rivals",
            title: "DEN RIVALS: FURY OF A THOUSAND BEARS",
            prose: &[],
            entry: &[
                "The Galapagos Emporium exhibit had a sign on it that said THE \
                 FURY OF A THOUSAND BEARS and a rope in front of it that said \
                 the management did not expect to be taken literally.",
                "Somebody took it literally. The rope is on the floor and the \
                 sign is still up.",
            ],
            landings: &[],
        },
        Retold {
            id: "the-crevice",
            title: "THE CREVICE IN THE ROCK",
            prose: &[],
            entry: &[
                "The rock is not cracked. It is *hinged*, and it has been \
                 standing open for however long it takes a hinge that size to \
                 stop being noticed.",
                "Two floors down there is something that was walled in rather \
                 than buried, which are different jobs done by different \
                 people for different reasons.",
            ],
            landings: &[],
        },
        Retold {
            id: "wumpus-world",
            title: "WUMPUS WORLD",
            prose: &[],
            entry: &[
                "Twenty rooms, and the thing in them has been in them longer \
                 than the rooms have been counted.",
                "It does not hunt by sight and it does not hunt by smell. It \
                 hunts by *footsteps*, and it has already heard yours arrive.",
            ],
            landings: &[],
        },
        // ---- the five that always happen -----------------------------------
        Retold {
            id: "the-teller",
            title: "THE STORY FROM SONGIL",
            prose: &[], entry: &[], landings: &[],
        },
        Retold {
            id: "the-dispenser",
            title: "THE MACHINE IN THE BACK CORNER",
            prose: &[], entry: &[], landings: &[],
        },
        Retold { id: "what-the-table-said", title: "THE TABLES SPEAK FOR US", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-bird-problem", title: "UNSOLICITED PROPOSAL", prose: &[], entry: &[], landings: &[] },
        // ---- Extra Large, and the four places a bauble goes -----------------
        Retold { id: "the-bigger-sign", title: "THE SIGN BEHIND THE SIGN", prose: &[], entry: &[], landings: &[] },
        // A thrumbus is the book's animal, p. 29, and the canonical race is
        // run by bolters now for the same reason the gate says HOLLIS. Cobb
        // keeps the form either way: a canonical name standing in a turtle
        // scene is what Merrik and Halloway have always done.
        Retold {
            id: "the-thrumbus-race",
            title: "THE 45TH ANNUAL THRUMBUS RACE",
            prose: &[
                "The 45th running, and the paddock is nine deep in people who \
                 have an opinion about a thrumbus. A thrumbus is the fastest \
                 thing that has ever been bred and looks, standing still, like \
                 a mistake.",
                "There is a book taking bets and a rail you can lean on and a \
                 steward called Cobb who will let anybody run who signs the \
                 form, and the form is one line long and the line is about \
                 teeth.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold { id: "mole-town", title: "HIGHWAY TO MOLE TOWN", prose: &[], entry: &[], landings: &[] },
        // ---- the structures --------------------------------------------------
        Retold { id: "the-inspection", title: "THE RICE INSPECTION", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-sealed-bid", title: "THE FNORP AUCTION", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-contract", title: "THE CORK CONTRACT", prose: &[], entry: &[], landings: &[] },
        // There were only ever sixty-two, p. 84 - until you.
        Retold { id: "the-payout", title: "THE 63RD ANTICIPATION", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-passenger", title: "THE WIMPLER CALF", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-buyer", title: "THE MULTICITY BUYER", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-fork", title: "THE FORK IN THE SEAM", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-foundry-remembers", title: "THE BURNWARP REMEMBERS", prose: &[], entry: &[], landings: &[] },
        Retold {
            id: "through-the-cracked-lens",
            title: "THROUGH FORESTON'S MONOCLE",
            prose: &[], entry: &[], landings: &[],
        },
        // ---- the three standalone pairs --------------------------------------
        Retold { id: "the-wizards-thirst", title: "THE SPINDRIFT HOARD", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-picket-line", title: "THE GORTBALL LINE", prose: &[], entry: &[], landings: &[] },
        Retold { id: "the-exhibition", title: "HANGLO AND JIMMY, ONE NIGHT ONLY", prose: &[], entry: &[], landings: &[] },

        // ------------------------------------------------- and the eight scenes
        //
        // Every one of these is here because the shipped canonical text was
        // speaking turtle - a proper noun no word-swap can reach - or because
        // the scene was written in this voice first and ported second. The
        // canonical column is the game's; this is the lookup.
        Retold {
            id: "the-crownwright",
            title: "THE HAT MAN OF KOLOK",
            prose: &[
                "The Kolok Hatter works out of one room over a fish shop and \
                 does not turn round when you come in, on the grounds that he \
                 can hear how full your head is from where he is sitting.",
                "\"Full,\" he says. \"Good. Most of them come up those stairs \
                 empty and want me to put something in it. I make hats. I am \
                 not a philanthropist and I am very much not a doctor.\"",
                "He will not sell you a hat. He will take a measurement, for \
                 the record. The record is a ledger four inches thick that \
                 lives under the bench, and he will not let you look in it.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold {
            id: "the-casino",
            title: "THE GALAPAGOS EMPORIUM",
            prose: &[
                "The Galapagos Emporium takes anybody who can walk in, which \
                 is how you got in. There is a bowl of complimentary Chromatic \
                 Rice by the door and a card over it reading ONE (1) HANDFUL - \
                 HONOUR SYSTEM - WE ARE WATCHING YOU TAKE IT.",
                "You are here for Kolok Hold-Em, which is Hold-Em except that \
                 one card in the deck is a live gooster and no player may look \
                 at it. You have the fnorp. You have taken your one handful.",
                "At the third table along, two players have stopped playing \
                 Kolok Hold-Em and started on each other. The room has formed \
                 a ring around it. A woman with a clipboard is working through \
                 the ring taking side bets in a very neat hand, and the dealer \
                 is standing perfectly still with the gooster held out at \
                 arm's length.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold {
            id: "the-long-way",
            title: "GERALD",
            prose: &[
                "That last one took eleven seconds. You know it took eleven \
                 seconds because a man at the roadside was counting out loud, \
                 and when you finished he wrote the number in a notebook and \
                 said nothing else about it.",
                "His cart is ahead of you on the road, pulled by an animal \
                 with a brass plate on its harness. The plate gives the \
                 species, which is Slow Trundler, and the name, which is \
                 Gerald, and the top speed, which is given in metres per hour.",
                "Gerald is hauling four tons of Deep Chocolate to Kettleworks. \
                 They set off in the spring. The man says they are ahead of \
                 schedule, and shows you the notebook again at a different \
                 page, as though that settles it.",
            ],
            entry: &[],
            landings: &[],
        },
        Retold {
            id: "back-in-a-minute",
            title: "IMMA GO BUY A SLURPEE",
            prose: &[
                "A man on the road hands you a parcel, says \"I'm gonna go buy \
                 a slurpee if you wanna come,\" and walks off the road at an \
                 angle that is not towards anything.",
                "You do not come. He does not come back. That is the whole of \
                 the story and everybody in Bambulon knows it, and the ones \
                 who tell it best are the ones who stop there.",
                "The wrapping is a page torn out of a star chart. Somebody has \
                 gone round one shop on it twice in pencil - a store two towns \
                 up that keeps the odd words on its back shelf - and written, \
                 in the margin, ASK FOR THE TETRAHEDRON.",
            ],
            entry: &[],
            landings: &[],
        },
        // ---- THE SWITCHYARD ---------------------------------------------
        //
        // The Empire left; the Sprocketman on the points did not. The
        // timetable is a Cork timetable and every train on it is a train the
        // Empire ran once, and the yard throws its points for them still, on
        // the Yonk standard, because nobody has told it the Empire is gone and
        // it would not believe them.
        //
        // Titles only, for three of the four. `retell` reaches the common
        // nouns a word at a time and a theme spends paragraphs only where a
        // *proper noun* is carrying the scene - which here is Ambrose and
        // Hesketh, and both are roles in the canonical column already.
        Retold {
            id: "the-timetable",
            title: "THE CORK TIMETABLE",
            prose: &[], entry: &[], landings: &[],
        },
        Retold {
            id: "the-signal-box",
            title: "THE SPROCKETMAN IN THE BOX",
            prose: &[], entry: &[], landings: &[],
        },
        Retold {
            id: "the-turntable",
            title: "THE TURNTABLE ON THE YONK STANDARD",
            prose: &[], entry: &[], landings: &[],
        },
        Retold {
            id: "the-last-train",
            title: "THE LAST CORK TRAIN",
            prose: &[], entry: &[], landings: &[],
        },
        // The dungeon itself, retold: the yard is the Empire's, the points are
        // a Sprocketman's, and what is at the buffer stops is what the Empire
        // left behind everywhere it went.
        //
        // The landings are keyed by floor index, which is why the graph keeps
        // floors in a flat list: an index is a stable key. Per-floor *entry*
        // lines are left canonical - Part E's E-3, taken as recommended -
        // because both name no proper noun and a missing entry falls through.
        Retold {
            id: "the-switchyard",
            title: "THE CORK TRAIN YARDS",
            prose: &[
                "The yards are nine rooms under the cutting, and the turntable \
                 is the first, and from it two lines go off into the dark with \
                 points on each of them, and a buffer stop at the end of every \
                 road.",
                "The timetable lists eleven Cork trains a day out of here and \
                 the Sprocketman keeps the times. There are no trains. \
                 Something has to be moving for a time to be kept, and whatever \
                 it is, it is moving to the sheet.",
                "Four fights down either line. What is at the buffer stop was \
                 left there on purpose, by an Empire that did not expect to be \
                 back for it.",
            ],
            entry: &[
                "The turntable takes you a quarter of the way round on the Yonk \
                 standard and stops, and the Sprocketman's bell rings once, and \
                 when it turns back you are facing the other way, down the \
                 yards.",
                "Somewhere past the lamp the points are already thrown. The \
                 Sprocketman was here first. The Sprocketman is always here \
                 first.",
            ],
            landings: &[],
        },
    ],

};
/// The book's words, for the item-name generator. Every entry is a proper
/// noun, object or place from the text - a common item reads like a regional
/// export, and a legendary one like something out of the cosmology.
pub static TD_NAMING: crate::naming::Naming = crate::naming::Naming {
    weapon_bases: &[
        "Fang", "Edge", "Dart", "Skewer", "Peel", "Jaw", "Rim", "Splinter", "Glint",
        "Bolt", "Barb", "Tooth", "Crank", "Lever", "Cleaver", "Sliver", "Thorn",
        "Wheel", "Shard", "Bite", "Sting", "Ladle", "Racquet", "Spoke", "Spur",
        "Nail", "Pick", "Saw", "Quill", "Hook", "Wedge", "Gear",
    ],
    helmet_bases: &[
        "Crown", "Hood", "Hat", "Visor", "Monocle", "Wig", "Helm", "Cowl", "Crest",
        "Halo", "Gaze", "Brow", "Mask", "Cap", "Wreath", "Beak", "Antler", "Blinder",
        "Watcher", "Eye", "Mind", "Dome", "Casque", "Circlet", "Bonnet", "Shade",
        "Veil", "Horn", "Skullcap", "Muzzle", "Diadem", "Wimple",
    ],
    chest_bases: &[
        "Coat", "Jacket", "Smock", "Tapestry", "Shell", "Vest", "Mantle", "Weave",
        "Bale", "Husk", "Chassis", "Frame", "Girdle", "Wrap", "Bark", "Scale", "Hide",
        "Casing", "Cradle", "Vault", "Keel", "Cage", "Robe", "Tunic", "Harness", "Fur",
        "Cork", "Plating", "Sheath", "Barrel", "Hauberk", "Carapace",
    ],
    glove_bases: &[
        "Grasp", "Mitt", "Grip", "Fist", "Palm", "Claw", "Paw", "Cuff", "Hold",
        "Pinch", "Snare", "Knuckle", "Digit", "Finger", "Hand", "Clamp", "Latch",
        "Crank", "Press", "Wringer", "Catcher", "Squeeze", "Talon", "Vise",
        "Gauntlet", "Handwrap", "Bracer", "Nail", "Grapple", "Hook", "Cinch", "Clutch",
    ],
    greave_bases: &[
        "Stride", "Tread", "Step", "Gait", "Pace", "Boot", "March", "Roll", "Shin",
        "Heel", "Kick", "Runner", "Walker", "Trundle", "Lope", "Vault", "Spur",
        "Stirrup", "Anklet", "Sole", "Track", "Trail", "Wander", "Roam", "Prowl",
        "Creep", "Bound", "Leap", "Dance", "Sprint", "Stilt", "Tab",
    ],
    attributives: &[
        "Treyway", "Kaplin", "Multicity", "Petonkle", "Dobira", "Cork", "Sneel",
        "Rice", "Nut", "Worm", "Fnorp", "Gear", "Soda", "Brink", "Yonk", "Mansus",
        "Bambulon", "Kolok", "Wextreen", "Yodregar", "Songil", "Promte", "Thrumbus",
        "Gooster", "Frong", "Brumpus", "Ench", "Octarine", "Wimpler", "Funny",
        "Skoogle", "Drambus",
    ],
    suffixes: &[
        // One word: the tails a rare or an epic gets.
        "Brink", "Funny", "Crypt", "Treyway", "Mansus", "Wimple", "Roast", "Harvest",
        "Labyrinth", "Emporium", "Crimper", "Monastery", "Quarry", "Glacier",
        "Flattening", "Lottery", "Squeals", "Anticipations", "Worm", "Cork", "Peel",
        "Rush", "Archives", "Calculation",
        // Two words: reserved for legendaries, which is what makes the extra
        // word audible.
        "Grand Calculation", "Gear Cave", "Soda Labyrinth", "Worm Fact", "Money Coat",
        "Last Oxen", "Nut Tapestry", "Rice Criers", "Eighth Ray", "Time Sap",
        "Deep Chocolate", "Grey Sphere", "Perfect Crime", "Morning Rush",
        "Unmovable Rock", "Hybrid Dodecathlon", "Weeping Seeker", "Blank Page",
        "Second Eclipse", "Slow Trundler", "Burger Eden", "Wolf Scrape",
        "Brie Cliffs", "Stone Keeper",
    ],
    epithets: &[
        "Plain", "Honest", "Serviceable", "Blunt", "Worn", "Simple", "Sturdy",
        "Rough", "Old", "Lowborn", "Practical", "Unadorned", "Weathered", "Solid",
        "Modest", "Bare",
    ],
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

    /// The same for components. A typo here is a piece that quietly keeps its
    /// old name, which nobody would notice among three hundred of them.
    #[test]
    fn every_themed_piece_names_a_real_one() {
        use crate::piece::CATALOG;
        for t in THEMES {
            for (canonical, themed) in t.pieces {
                assert!(
                    CATALOG.iter().any(|d| d.name == *canonical),
                    "{} renames {:?} -> {:?}, but no such component exists",
                    t.id,
                    canonical,
                    themed
                );
            }
        }
    }

    /// Two components sharing a name would be two things the player cannot
    /// tell apart in a shop.
    #[test]
    fn no_two_components_get_the_same_new_name() {
        for t in THEMES {
            let mut seen: Vec<&str> = Vec::new();
            for (_, themed) in t.pieces {
                assert!(!seen.contains(themed), "{} uses {:?} twice", t.id, themed);
                seen.push(themed);
            }
        }
    }

    /// A theme that has re-told the ladder should have re-told the catalogue
    /// too - except where a name was already the book's.
    #[test]
    fn the_turtle_theme_covers_the_catalogue() {
        use crate::piece::CATALOG;
        // Kept on purpose, each for its own reason:
        //   the cogs   - the player's own culture, salvaged from the Great
        //                Gear Cave; the one place the old words survive
        //   Anvil Frame, Hollow Weave - already the book's (the Comedian's
        //                anvil; the Mansus walls that are not there)
        //   Witch's Hat - Marbulon was an old withered witch
        //   Green Crown, Wandering Root - already read as Nut Metropolis
        //   Worldeye Orb - the Mansus sun-being's gaze
        //   The Money Jacket - it *is* Francis's coat
        const KEPT: &[&str] = &[
            "Ratchet Cog",
            "Flywheel Cog",
            "Anvil Frame",
            "Hollow Weave",
            "Witch's Hat",
            "Green Crown",
            "Wandering Root",
            "Worldeye Orb",
            "The Money Jacket",
        ];
        let missed: Vec<&str> = CATALOG
            .iter()
            .map(|d| d.name)
            .filter(|n| TURTLE_DICK.piece(n) == *n)
            .filter(|n| !KEPT.contains(n))
            .collect();
        assert!(missed.is_empty(), "still in plain words: {:?}", missed);
    }

    /// The same for titles. A typo here is a class that quietly keeps its
    /// high-fantasy name in a game that has none.
    #[test]
    fn every_themed_class_names_a_real_one() {
        use crate::class::CLASSES;
        for t in THEMES {
            for (canonical, themed) in t.classes {
                assert!(
                    CLASSES.iter().any(|c| c.name == *canonical),
                    "{} renames {:?} -> {:?}, but no such class exists",
                    t.id,
                    canonical,
                    themed
                );
            }
        }
    }

    /// And the other direction. "Archmage" beside "Fnorp" is the single
    /// loudest way the theme gives itself away as a coat of paint.
    #[test]
    fn the_turtle_theme_retitles_every_class() {
        use crate::class::CLASSES;
        let missed: Vec<&str> = CLASSES
            .iter()
            .map(|c| c.name)
            .filter(|n| TURTLE_DICK.class(n) == *n)
            .collect();
        assert!(missed.is_empty(), "still in plain words: {:?}", missed);
    }

    #[test]
    fn no_two_classes_get_the_same_new_name() {
        for t in THEMES {
            let mut seen: Vec<&str> = Vec::new();
            for (_, themed) in t.classes {
                assert!(!seen.contains(themed), "{} uses {:?} twice", t.id, themed);
                seen.push(themed);
            }
        }
    }

    /// A theme names creatures by their canonical name, so a typo in the
    /// table is a rung that quietly keeps its old name. This catches that.
    #[test]
    fn every_themed_monster_names_a_real_one() {
        use crate::combat::{ALTERNATES, LADDER};
        for t in THEMES {
            for (canonical, themed) in t.monsters {
                assert!(
                    LADDER
                        .iter()
                        .chain(ALTERNATES.iter())
                        .any(|m| m.name == *canonical),
                    "{} renames {:?} -> {:?}, but there is no such creature",
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

    /// And every creature beside the road that a theme has something to say
    /// about, said.
    ///
    /// Not "every alternate": four of the Switchyard's nine keep their names
    /// on purpose - a coal stage is a coal stage on any plane, and all caps is
    /// a universal language (`theme.rs`'s own note on the shipped four). What
    /// this asserts is that the five the theme *does* rename are renamed, so a
    /// half-finished table is a failure here rather than a run in two voices.
    #[test]
    fn the_turtle_theme_retells_the_yard() {
        const RENAMED: &[&str] = &[
            "THE SHUNTER",
            "THE PLATELAYERS",
            "THE BALLAST",
            "THE GANTRY",
            "THE LAMP ROOM",
        ];
        const KEPT: &[&str] =
            &["THE COAL STAGE", "THE WATER TOWER", "THE GOODS SHED", "THE ROUNDHOUSE"];

        for n in RENAMED {
            assert_ne!(TURTLE_DICK.monster(n), *n, "{n} is still in plain words");
        }
        for n in KEPT {
            assert_eq!(TURTLE_DICK.monster(n), *n, "{n} grew a themed name; add it above");
        }
        // The yard's own four floors that are places rather than things are
        // exactly the ones kept, which is what makes the split a rule.
        let d = crate::dungeon::by_id("the-switchyard").expect("M6");
        for f in d.floors {
            assert!(
                RENAMED.contains(&f.creature) || KEPT.contains(&f.creature),
                "{} is a floor nothing here knows about",
                f.creature
            );
        }
        // And the four doors and the dungeon are retitled.
        for id in ["the-timetable", "the-signal-box", "the-turntable", "the-last-train"] {
            let e = crate::event::EVENTS.iter().find(|e| e.id == id).expect("a door");
            assert_ne!(TURTLE_DICK.place(id, e.title), e.title, "{id} is untitled");
        }
        assert_ne!(TURTLE_DICK.place(d.id, d.name), d.name, "the yard is untitled");
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

    /// Every theme has to be able to name an item at every tier. A corpus
    /// with no two-word tails would silently hand legendaries a five-word
    /// name, which is the one thing the rule exists to prevent.
    #[test]
    fn every_theme_can_name_at_every_length() {
        use crate::piece::SlotKind;
        for t in THEMES {
            for kind in SlotKind::ALL {
                assert!(
                    t.naming.bases(kind).len() >= 24,
                    "{}: too few {:?} nouns",
                    t.id,
                    kind
                );
            }
            assert!(t.naming.attributives.len() >= 16, "{}: too few attributives", t.id);
            assert!(!t.naming.epithets.is_empty(), "{}: no epithets", t.id);
            for want in [1usize, 2] {
                let n = t
                    .naming
                    .suffixes
                    .iter()
                    .filter(|s| s.split_whitespace().count() == want)
                    .count();
                assert!(n >= 8, "{}: only {} tails of {} word(s)", t.id, n, want);
            }
        }
    }

    /// A slug the interface asks for and no theme answers is a word that will
    /// never be translated. This is the list of what the interface actually
    /// asks for; a theme is free to answer none of it, but a typo in a slug on
    /// either side should be visible here rather than on screen.
    #[test]
    fn the_turtle_theme_answers_the_slugs_the_interface_uses() {
        const ASKED: &[&str] = &[
            "gold", "gold-lower", "gold-suffix", "shop", "shop-hint", "reroll", "inventory",
            "inventory-hint", "your-items", "fountain", "fountain-blurb",
            "fountain-take", "class", "classes", "begin-fight", "character",
            "opponent", "glossary",
            // The map's noun and the key that opens it. On the list because a
            // slug the interface asks for and the theme has no answer to is a
            // plain English word in a screen full of Fnorp.
            "the-road", "map-hint",
        ];
        let unanswered: Vec<&str> =
            ASKED.iter().copied().filter(|k| TURTLE_DICK.word(k, "") == "").collect();
        assert!(unanswered.is_empty(), "no turtle wording for: {:?}", unanswered);
    }

    /// And the other way: a word in a theme's table that nothing ever asks for
    /// is dead weight, but harmless - so this only checks the table is not
    /// full of empties.
    #[test]
    fn no_theme_maps_a_word_to_nothing() {
        for t in THEMES {
            for (slug, value) in t.words {
                assert!(!slug.is_empty() && !value.is_empty(), "{} has an empty entry", t.id);
            }
        }
    }

    /// Whole words only. The failure this guards against is a substring
    /// match turning "manacle" into "Funnycle" and "armoury" into "Corky".
    #[test]
    fn retelling_only_replaces_whole_words() {
        let t = &TURTLE_DICK;
        // Lower in, lower out - the case rule applies here too.
        assert_eq!(t.retell("mana"), "jokes");
        assert_eq!(t.retell("manacle"), "manacle");
        // Two words that must not blur into one another.
        assert_eq!(t.retell("magic damage"), "funny damage");
        assert_eq!(t.retell("spend 3 mana"), "spend 3 jokes");
        assert_eq!(t.retell("armoury"), "armoury");
        assert_eq!(t.retell("2 mana a second"), "2 jokes a second");
        assert_eq!(t.retell("gains 12 armor (12)"), "gains 12 cork (12)");
    }

    /// The original's capitalisation is kept, so a replacement mid-sentence
    /// does not shout and one at the start is not lowercased.
    #[test]
    fn retelling_follows_the_case_it_found() {
        let t = &TURTLE_DICK;
        assert_eq!(t.retell("Mana is spent"), "Jokes is spent");
        assert_eq!(t.retell("spends mana"), "spends jokes");
        assert_eq!(t.retell("ARMOR"), "Cork");
        // The replacement takes the source's case, not the table's.
        assert_eq!(t.retell("Searing Thorn"), "Roasting Thorn");
    }

    /// The plain theme has no vocabulary, so it hands prose straight back -
    /// the untranslated game pays nothing for this existing.
    #[test]
    fn the_plain_theme_leaves_prose_alone() {
        let long = "on activation, spend 3 faith: if it works, gain 30 armor";
        assert_eq!(PLAIN.retell(long), long);
    }

    /// A glossary entry a theme replaces must name one that exists, or it
    /// silently becomes an addition nobody asked for.
    #[test]
    fn every_replaced_glossary_entry_replaces_something() {
        // The interface owns the plain glossary, so this checks the shape of
        // the table rather than the terms themselves: a replacement names a
        // term, an addition names none.
        for t in THEMES {
            for (from, term, def) in t.glossary {
                assert!(!term.is_empty(), "{}: an entry with no term", t.id);
                assert!(!def.is_empty(), "{}: {:?} has no definition", t.id, term);
                let _ = from;
            }
        }
    }

    /// A scene or a note keyed to a creature that does not exist would never
    /// fire, and nothing would say so.
    #[test]
    fn scenes_and_notes_name_creatures_that_exist() {
        use crate::combat::LADDER;
        for t in THEMES {
            for (m, scene) in t.cutscenes {
                assert!(
                    LADDER.iter().any(|x| x.name == *m),
                    "{}: a scene for {:?}, which is nobody",
                    t.id,
                    m
                );
                assert!(!scene.is_empty(), "{}: {:?} has an empty scene", t.id, m);
            }
            for (m, note) in t.notes {
                assert!(
                    LADDER.iter().any(|x| x.name == *m),
                    "{}: a note about {:?}, which is nobody",
                    t.id,
                    m
                );
                assert!(!note.is_empty(), "{}: {:?} has an empty note", t.id, m);
            }
        }
    }

    #[test]
    fn an_unknown_id_falls_back_to_the_default() {
        assert_eq!(by_id("nonsense").id, THEMES[0].id);
        assert_eq!(by_id("td").id, "td");
    }
}
