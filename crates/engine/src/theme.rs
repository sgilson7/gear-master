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
    words: &[],
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
        "A Sprocketman's whole craft is making working gear out of loose \
         pieces. That is what the five frames are. Piece by scavenged piece, \
         you build yourself out of the hole.",
        "Above the pit are the planes, and above those is Mount Dobira, and at \
         the top of it is a gambler in a coat made of money who flattens worlds \
         when he loses.",
        "Climb anyway.",
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
        ("Absolution", "Remark of Renewal"),
        ("Adamant Base", "Ypytryktrium Base"),
        ("Adamant Carapace", "Ypytryktrium Carapace"),
        ("Adamant Fang", "Megalodon Tooth"),
        ("Aegis Crown", "Thirty-Foot Hat"),
        ("Aegis Weave", "Cork Aegis"),
        ("Aether Layer", "Wimple Layer"),
        ("Anchor Material", "Ice-Anchor Material"),
        ("Anchored Sole", "Ice-Anchor Sole"),
        ("Anvil Frame", "Anvil Frame"),
        ("Apprentice's Primer", "Extra Funny Jokebook"),
        ("Arc Lightning", "Spooky Action"),
        ("Arcane Splinter", "Tetrahedron Splinter"),
        ("Archmage's Primer", "Comedian's Jokebook"),
        ("Archon's Crest", "Comptroller's Crest"),
        ("Ash Haft", "Banana-Peel Grip"),
        ("Ashfall Ink", "Shelf-Drink Fluid"),
        ("Ashwoven Material", "Ash-Field Material"),
        ("Astrolabe", "Dodecathlon Wheel"),
        ("Attendant Flame", "Blingarian Flare"),
        ("Azure Alignment", "Basseterian Long Grain"),
        ("Balance Weight", "Trillion-Pound Plate"),
        ("Balanced Grip", "Betting Stick"),
        ("Bare-Headed Fang", "Bare-Handed Goof"),
        ("Bastion Base", "Sneel Base"),
        ("Berserker's Crest", "Gorillathon Crest"),
        ("Berserker's Plate", "Gladiator Plate"),
        ("Bileglass Vial", "Bileglass Phial"),
        ("Blade of Helms", "Katana Glint"),
        ("Blight Layer", "Rot Layer"),
        ("Blood Rite", "Grand Calculation"),
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
        ("Breaker's Fist", "Gorilla Knuckle"),
        ("Brigandine Base", "Cork Vest"),
        ("Broken Crown", "The Teetering Crown"),
        ("Bronze Fang", "Frong Tooth"),
        ("Bronze Frame", "Vinyl Frame"),
        ("Bronze Plating", "Vinyl Plating"),
        ("Bulwark Base", "Quarry Base"),
        ("Bulwark Layer", "Sneel Layer"),
        ("Bulwark Material", "Cork Material"),
        ("Bulwark Plating", "Time-Tempered Plating"),
        ("Bulwark Vial", "Spindrift Can"),
        ("Buttressed Frame", "Quarry Frame"),
        ("Chain Coil", "Fishing Reel"),
        ("Chain Layer", "Fishing-Line Layer"),
        ("Chained Codex", "The Chained Archive"),
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
        ("Colossus Ring", "Wheel of the Thrumbus Race"),
        ("Consecrated Plating", "Francian Plating"),
        ("Corded Grip", "Fishing-Line Grip"),
        ("Coven Crest", "Order Crest"),
        ("Coven Mold", "Order Mold"),
        ("Cracked Pauldron", "Cracked Gear-Plate"),
        ("Crest of Vigor", "Soft Drink Cap"),
        ("Crimson Alignment", "Striped Boskner"),
        ("Crown of Nails", "Crown of Darts"),
        ("Crown of the Deep", "Crown of Cleveland"),
        ("Cull", "The Rubber"),
        ("Cursed Blade", "Martyr's Anvil-Blade"),
        ("Cursed Handle", "Confetti Trigger"),
        ("Deadweight Plating", "Trillion-Pound Plating"),
        ("Deeprooted Sole", "Grungo-Rooted Sole"),
        ("Deepwater Ink", "Cleveland Water"),
        ("Deft Mold", "Origami Mold"),
        ("Duelist's Fob", "Fit Watch"),
        ("Duelist's Grip", "Dart Grip"),
        ("Duelist's Hilt", "Dart Blade Hilt"),
        ("Duskweave Material", "Dark-Matter Material"),
        ("Echo Sigil", "Radio Broadcast"),
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
        ("Frostbind", "Nut Freeze"),
        ("Fury Sigil", "Union Grievance"),
        ("Gauntlet Mold", "Panini-Press Mold"),
        ("Gilded Crest", "Bedazzle Crest"),
        ("Glacier Ink", "Dobira Meltwater"),
        ("Gluttonous Fang", "Ench Skewer"),
        ("Godsheet Layer", "Wimpler-Fur Layer"),
        ("Godsteel Haft", "Ypytryktrium Haft"),
        ("Godsteel Plating", "Ypytryktrium Plating"),
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
        ("Hastening Crest", "Rooster Crest"),
        ("Heartwood Base", "Grungo-Wood Base"),
        ("Heartwood Crest", "Nautilus Crest"),
        ("Helm of Blades", "Helm of Darts"),
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
        ("Kingmaker Hilt", "Treyway Hilt"),
        ("Kingsblood Ink", "Time Sap"),
        ("Knuckleduster", "Tennis-Racquet Mold"),
        ("Layered Core", "Rice-Bale Core"),
        ("Layered Plating", "Onigiri Plating"),
        ("Leaden Tome", "Yodregar Disk"),
        ("Leather Material", "Boiled Gooster Hide"),
        ("Leyline Cuirass", "Plug-Energy Harness"),
        ("Lightweave", "Featherlight Weave"),
        ("Loaded Fob", "Radio Watch"),
        ("Lonely Plating", "Hermit's Cork"),
        ("Mage's Circlet", "Owl Circlet"),
        ("Mage's Rod", "Kappa Wand"),
        ("Mage's Sandals", "Velcro Tabs"),
        ("Mage's Wrapping", "Funnel Wrapping"),
        ("Mail Layer", "Wallspider Mail"),
        ("Malefic Crest", "Rot Crest"),
        ("Mana Loom", "Funnel Loom"),
        ("Mana Ward", "Funny Damper"),
        ("Martyr's Crest", "Jester's Crest"),
        ("Mending Layer", "Healing-Pod Layer"),
        ("Mercurial Ink", "Nut Bar Slurry"),
        ("Mirror Ward", "But Wait, There's Less"),
        ("Mirrorbright Plating", "Museum-Glass Plating"),
        ("Mirrorcast", "Copy Paste Race"),
        ("Mirrored Visor", "Kaleidoscope Visor"),
        ("Multi-Handle", "Crank Assembly"),
        ("Nimble Mold", "Fast Roller Mold"),
        ("Oak Handle", "Nut Bar Handle"),
        ("Oathbound Ink", "Petal Elixir"),
        ("Oathkeeper Mold", "Union Mold"),
        ("Oathplate", "Union Plate"),
        ("Oathring", "Onion Ring"),
        ("Oathstone Bead", "Drambus Seed"),
        ("Obsidian Orb", "Academy Steel Ball"),
        ("Orb of the Nine", "Orb of the Eighth Ray"),
        ("Overflow Vial", "Soda Labyrinth Phial"),
        ("Padded Base", "Cardboard Base"),
        ("Padded Mold", "Oven-Mitt Mold"),
        ("Pathfinder Material", "Trail-of-Holes Material"),
        ("Piercer's Band", "Dart Band"),
        ("Pilgrim Alignment", "Old Man's Beard"),
        ("Pilgrim Sole", "Monastery Sole"),
        ("Pilgrim's Sole", "Dobira Pilgrim Sole"),
        ("Plaguewalkers", "Rot-Walkers"),
        ("Plain Sole", "Slow Trundler Sole"),
        ("Plate Layer", "Cork Plate"),
        ("Pocket Grimoire", "Pocket Koans"),
        ("Polished Orb", "Teetering Marble"),
        ("Prism Alignment", "Super Strain R-B-G-O"),
        ("Prismatic Ink", "Chromatic Rice-Water"),
        ("Quickening Charm", "Time-Sap Drop"),
        ("Quickfinger Mold", "Dart-Thrower's Mold"),
        ("Quickread Folio", "Tactical Haiku"),
        ("Quicksilver Ink", "Exotic Juice"),
        ("Quickstep Mold", "Skip-to-the-Slurpee Sole"),
        ("Quilted Base", "Onigiri Base"),
        ("Racing Sole", "Fast Roller Sole"),
        ("Rag Layer", "Robe Scrap"),
        ("Ravener's Mold", "Megalodon Mold"),
        ("Reaver's Bill", "Crimper Jaw"),
        ("Reliquary Frame", "Acolyte's Hood"),
        ("Reliquary Frame of Nine", "The Master's Hood"),
        ("Reliquary Orb", "Rock-Core Shard"),
        ("Rending Mold", "Wallspider Mold"),
        ("Resonant Chord", "Very Fast This Time"),
        ("Ribbed Base", "Vinyl Base"),
        ("Ridged Frame", "Corrugated Frame"),
        ("Rime Nova", "Minus One Degrees"),
        ("Rimeguard Base", "Blizzard Base"),
        ("Ring of Embers", "LSB Ring"),
        ("Ring of Hours", "Radio-Watch Ring"),
        ("Ring of Roots", "Grungo Ring"),
        ("Ring of Tides", "Brie-Sea Ring"),
        ("Ring of Vigils", "Night-Worker's Ring"),
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
        ("Scale Layer", "Toad-Skin Layer"),
        ("Scaled Material", "Skink Scale"),
        ("Scaled Plating", "Megalodon Scale"),
        ("Scholar's Codex", "Rick Richard's Notebook"),
        ("Scrying Lens", "Cork Glasses"),
        ("Scrying Orb", "Mog Watcher"),
        ("Seal of Power", "Treyway Seal"),
        ("Seal of the Deep", "Seal of Cleveland"),
        ("Seal of the Grove", "Drambus Seal"),
        ("Seer's Crest", "Quadruple-Eclipse Crest"),
        ("Seer's Orb", "Foreston Glass"),
        ("Serrated Edge", "Multiplication Rim"),
        ("Sevenleague Boots", "Thrumbus Boots"),
        ("Sevenleague Sole", "Thrumbus Sole"),
        ("Shatterbolt", "Steel-Ball Volley"),
        ("Sigil Layer", "Squeal Layer"),
        ("Signet of Ash", "Ash-Field Signet"),
        ("Signet of Iron", "Gear Signet"),
        ("Signet of Vigour", "Soft-Drink Cap Ring"),
        ("Silver Band", "Fnorp Piece"),
        ("Silver Charm", "Forever Stamp"),
        ("Siphon", "Semuta Strain"),
        ("Soot Ink", "Slime Cola"),
        ("Sovereign Mold", "Treyway Mold"),
        ("Spiked Vambrace", "Dart-Board Vambrace"),
        ("Spinning Orb", "Multiplication Wheel"),
        ("Sprawling Handwrap", "Gappy's Spare Hand"),
        ("Sprung Sole", "Wallspider Spring"),
        ("Spun Material", "Spun Rice-Silk"),
        ("Starfall", "Moonfall"),
        ("Starlit Ink", "Skink Brink's Soft Drink"),
        ("Starlit Mantle", "Katalungan Mantle"),
        ("Steel Frame", "Cork Helm"),
        ("Steel Material", "Sneel Material"),
        ("Stonewall Frame", "Unmovable Frame"),
        ("Stormcaught Frame", "Blizzard Hood"),
        ("Stormstep Mold", "Blizzard Step"),
        ("Striding Mold", "Mile-in-Months Mold"),
        ("Studded Sole", "Dart Sole"),
        ("Sunder", "Steel Ball"),
        ("Sunder Haft", "Steel-Ball Haft"),
        ("Sunderer", "Moon Fragment"),
        ("Swiftplate", "Thrumbus Plate"),
        ("Sympathetic Bloom", "Wextreen Bloom"),
        ("Tarpit Sole", "Brie-Cliff Sole"),
        ("Tempered Sole", "Kiln-Fired Sole"),
        ("The Empty Crown", "The Empty Throne"),
        ("The Growing Weight", "The Growing Stone"),
        ("The Money Jacket", "The Money Jacket"),
        ("Third Eye", "Foreston Monocle"),
        ("Thorn Layer", "Wallspider Thorn"),
        ("Thornmail Layer", "Dart-Board Layer"),
        ("Thornweald Grip", "Wallspider Silk"),
        ("Tidal Alignment", "Senndrier Vertigo Straw"),
        ("Tidecaller Orb", "Cleveland Tide Glass"),
        ("Tidewrack Ink", "Eleven-Fourteen Brew"),
        ("Timeworn Orb", "Time-Sap Amber"),
        ("Tin Band", "Spindrift Tab"),
        ("Tin Frame", "Spindrift-Can Frame"),
        ("Tin Plating", "Cork Plating"),
        ("Titan's Grip", "Megalodon Grip"),
        ("Trailworn Sole", "Pilgrim of Dobira Sole"),
        ("Traveller's Codex", "Mrs. Freya's Syllabus"),
        ("Twinned Grip", "Screw-Twister Grip"),
        ("Unbound Core", "Loose Sprocket Core"),
        ("Ungloved Layer", "Bare-Frame Layer"),
        ("Unmaking", "The Flattening"),
        ("Unshod Signet", "Bare-Sole Signet"),
        ("Vast Tapestry", "The Nut Tapestry"),
        ("Verdant Alignment", "Ocharpa Glass Stalk"),
        ("Verdant Surge", "Rice Harvest"),
        ("Verdant Weave", "Rice-Straw Weave"),
        ("Vicegrip Mold", "Crimper Mold"),
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
        ("Warplate Greave", "Gladiator Greave"),
        ("Waxed Material", "Brie-Cliff Wax"),
        ("Wayfarer's Sole", "Wanderer's Nut Bar Sole"),
        ("Wellspring Base", "Soda-Fountain Base"),
        ("Whetstone", "Quarry Granite"),
        ("Whipcord Hilt", "Grungo-Elastic Hilt"),
        ("Whisperbound Tome", "The Words of Angelo"),
        ("Widow's Sole", "Stone-Keeper's Sole"),
        ("Wildgrowth", "Bumper Crop"),
        ("Windup Key", "Great Brass Key"),
        ("Witch's Claw", "Frong Claw"),
        ("Witch's Crook", "Ladle of Dobira"),
        ("Witch's Hat", "Witch's Hat"),
        ("Witch's Stilts", "Baguette Stilts"),
        ("Witchglass Shard", "Petal of Wextreen"),
        ("Worldeye Orb", "Worldeye Orb"),
        ("Worldsplitter", "The Flattener's Edge"),
        ("Worldstrider Sole", "Planeswalker Sole"),
        ("Worldweave Material", "Planeswoven Material"),
        ("Woven Underlayer", "Silk-Cloth Underlayer"),
        ("Wrathful Mold", "Gorillathon Mold"),
        ("Wrathful Talons", "Frong Talons"),
        ("Wrathwrit Ink", "Power Serenade"),
        ("Zealot's Crest", "Rice Crier Crest"),
        ("Zealot's Sole", "Rice Crier Sole"),
    ],
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

    #[test]
    fn an_unknown_id_falls_back_to_the_default() {
        assert_eq!(by_id("nonsense").id, THEMES[0].id);
        assert_eq!(by_id("td").id, "td");
    }
}
