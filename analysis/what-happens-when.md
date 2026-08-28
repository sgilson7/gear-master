# What happens when — every piece in the catalogue

Written by `what_happens_when::audit`. Do not hand-edit.

## The shape of it

| | Pieces |
|---|---:|
| catalogue | 523 |
| carrying a per-activation figure in `Stats` | 266 |
| carrying a damage figure | 106 |
| **carrying damage the fight cannot land** | **23** |
| granting a pool as a trigger, not a stat | 0 |
| spelling one pool grant both ways | 0 |

## Damage the fight cannot land

Only a weapon swings (`loadout.rs`, `hit_for` returns 0 elsewhere), and
`rating.rs` prices every point of this. Mind is exempt: it lands from any
slot, which is why the helmets are not here.

- Emberplate               Chestpiece +7 magic dmg
- Voidsilk Base            Chestpiece +150 hp, +2 mana, +6 magic dmg
- Starlit Mantle           Chestpiece +9 magic dmg, +20% magic pierce
- Leyline Cuirass          Chestpiece +200 hp, +3 mana, +5 magic dmg
- Spiked Vambrace          Gloves   +8 phys dmg, +25% phys pierce
- Breaker's Fist           Gloves   +14 phys dmg, +35% phys pierce
- Thornweald Grip          Gloves   +5 phys dmg, +2 nature
- Emberloop                Gloves   +5 magic dmg
- Plaguewalkers            Greaves  +4 magic dmg
- Bulwark Base             Chestpiece +225 hp, +6 phys dmg, +22% phys res
- Riveted Layer            Chestpiece +5 phys dmg, +16% phys res
- Crown of Nails           Helmet   +11 armor, +6 phys dmg
- Thorn Layer              Chestpiece +9 armor, +7 phys dmg
- Ashwoven Material        Greaves  +5 phys dmg, +2 rage
- Rending Mold             Gloves   +11 phys dmg, +18% phys pierce
- Wrathful Mold            Gloves   +8 phys dmg, +3 rage
- Signet of Iron           Gloves   +6 phys dmg
- Signet of Ash            Gloves   +6 magic dmg
- Unshod Signet            Gloves   +1 mana, +7 magic dmg
- Blightfinger             Gloves   +5 magic dmg
- Witherroot               Greaves  +14% curse res, +6 magic dmg
- Siphon Ring              Gloves   +4 magic dmg
- Flaying Mold             Gloves   +5 magic dmg

## One effect, two spellings

`Stats { nature: 2 }` and `OnActivate(Gain { Nature, 2 })` are the same
thing to the fight. These say it as a trigger:


And these say it both ways at once, so their card adds the two together:


## Every piece


### Helm of Blades — Helmet frame
  PASSIVE: +7% mind res
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): gain 6 armor

### Blade of Helms — Weapon damaging
  EVERY TIME IT FIRES (triggered): gain 22 armor

### Apprentice's Primer — Weapon book
  TRIGGERS: on activation, spend 2 mana: if it works, deal 9 magic damage to the enemy; if not, gain 2 mana

### Archmage's Primer — Weapon book
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 5 mana: if it works, gain 1 spell forking; if not, gain 3 mana

### Cracked Pauldron — Chestpiece layer
  PASSIVE: +50 hp
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Warlord's Pauldron — Chestpiece layer
  PASSIVE: +240 hp

### Hexer's Tally — Gloves mold
  TRIGGERS: when a touching item activates, drain 2 nature from the enemy and deal 1 magic for each point

### Hexer's Reckoning — Gloves mold
  TRIGGERS: when a touching item activates, drain 2 faith from the enemy and deal 2 magic for each point

### Wayfarer's Sole — Greaves mold
  PASSIVE: +4% curse res
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Sevenleague Sole — Greaves mold
  PASSIVE: +12% curse res
  EVERY TIME IT FIRES: +2 mana

### Emberplate — Chestpiece layer
  DAMAGE: +7 magic dmg
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Runic Weave — Chestpiece layer
  PASSIVE: +18% magic res, +15% magic harden
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Voidsilk Base — Chestpiece base
  DAMAGE: +6 magic dmg
  PASSIVE: +150 hp
  EVERY TIME IT FIRES: +2 mana

### Starlit Mantle — Chestpiece layer
  DAMAGE: +9 magic dmg
  PASSIVE: +20% magic pierce
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Leyline Cuirass — Chestpiece base
  DAMAGE: +5 magic dmg
  PASSIVE: +200 hp
  EVERY TIME IT FIRES: +3 mana

### Spiked Vambrace — Gloves mold
  DAMAGE: +8 phys dmg
  PASSIVE: +25% phys pierce
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Ironhide Wrap — Gloves material
  PASSIVE: +34% phys res
  EVERY TIME IT FIRES: +12 armor

### Breaker's Fist — Gloves material
  DAMAGE: +14 phys dmg
  PASSIVE: +35% phys pierce

### Tempered Sole — Greaves mold
  PASSIVE: +10% curse res, +16% phys res
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Warplate Greave — Greaves material
  PASSIVE: +22% phys res
  EVERY TIME IT FIRES: +12 armor
  EVERY TIME IT FIRES (triggered): gain 14 armor

### Bloodrage Grip — Weapon handle
  DAMAGE: +4 phys dmg
  EVERY TIME IT FIRES: +2 rage

### Fury Sigil — Weapon accessory
  EVERY TIME IT FIRES: +1 rage

### Berserker's Plate — Chestpiece layer
  EVERY TIME IT FIRES: +8 armor
  EVERY TIME IT FIRES (triggered): gain 4 armor

### Wrathful Talons — Gloves mold
  TRIGGERS: on activation, spend 4 rage: if it works, deal 22 physical damage to the enemy; if not, gain 2 rage

### Cull — Weapon damaging
  DAMAGE: +16 phys dmg
  EVERY TIME IT FIRES: +1 rage

### Votive Crest — Helmet crest
  EVERY TIME IT FIRES: +3 faith

### Reliquary Frame — Helmet frame
  PASSIVE: +12% mind res
  EVERY TIME IT FIRES: +2 mana, +2 faith

### Consecrated Plating — Helmet plating
  PASSIVE: +15% phys res, +15% magic res

### Absolution — Weapon spell
  DAMAGE: +6 magic dmg
  TRIGGERS: on activation, spend 3 faith: if it works, gain 30 armor; if not, gain 1 faith

### Pilgrim's Sole — Greaves mold
  PASSIVE: +6% curse res
  EVERY TIME IT FIRES: +1 faith
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Rootbound Material — Greaves material
  PASSIVE: +8% curse res
  EVERY TIME IT FIRES: +2 nature

### Verdant Weave — Chestpiece layer
  PASSIVE: +1 regen
  EVERY TIME IT FIRES: +1 nature
  EVERY TIME IT FIRES (triggered): gain 3 maximum health for the rest of the fight

### Bloomcap — Helmet plating
  EVERY TIME IT FIRES: +1 nature

### Wildgrowth — Weapon spell
  DAMAGE: +4 magic dmg
  TRIGGERS: on activation, spend 3 nature: if it works, gain 6 mana; if not, gain 2 nature

### Thornweald Grip — Gloves material
  DAMAGE: +5 phys dmg
  EVERY TIME IT FIRES: +2 nature

### Astrolabe — Weapon crystal ball
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Obsidian Orb — Weapon crystal ball
  DAMAGE: +3 magic dmg
  TRIGGERS: when another spell in this item is cast, deal 7 magic damage to the enemy

### Prismatic Ink — Weapon ink
  EVERY TIME IT FIRES: +1 mana

### Shatterbolt — Weapon spell
  DAMAGE: +13 magic dmg
  PASSIVE: +40% magic pierce

### Hoarfrost — Weapon spell
  DAMAGE: +8 magic dmg

### Timeworn Orb — Weapon crystal ball
  DAMAGE: +2 magic dmg
  EVERY TIME IT FIRES: +2 mana
  EVERY TIME IT FIRES (triggered): cut 0.3s off its own cooldown

### Multi-Handle — Weapon handle

### Reliquary Frame of Nine — Helmet frame
  PASSIVE: +8% mind res
  EVERY TIME IT FIRES: +1 mana

### Layered Core — Chestpiece base
  PASSIVE: +125 hp

### Knuckleduster — Gloves mold

### Grimoire Rack — Weapon accessory

### Studded Sole — Greaves mold

### Signet of Vigour — Gloves ring
  EVERY TIME IT FIRES: +11 armor
  TRIGGERS: when a touching item activates, deal 5 physical damage to the enemy

### Iron Band — Gloves ring
  PASSIVE: +4 str
  TRIGGERS: when a touching item activates, cut 0.2s off its own cooldown

### Ring of Tides — Gloves ring
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when an item in another slot on the same rows activates, drain 2 mana from the enemy

### Emberloop — Gloves ring
  DAMAGE: +5 magic dmg
  TRIGGERS: when a touching item activates, deal 5 magic damage to the enemy

### Bloodring — Gloves ring
  EVERY TIME IT FIRES: +1 rage
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Warding Ring — Gloves ring
  PASSIVE: +14% curse res, +10% phys res, +10% magic res
  TRIGGERS: when a touching item activates, gain 2 armor

### Ring of Hours — Gloves ring
  TRIGGERS: when an item in another slot on the same rows activates, cut 0.4s off its own cooldown

### Seal of the Grove — Gloves ring
  PASSIVE: +1 regen
  EVERY TIME IT FIRES: +1 nature
  TRIGGERS: when an item in another slot on the same rows activates, gain 1 mana

### Oathring — Gloves ring
  EVERY TIME IT FIRES: +1 faith
  TRIGGERS: when a touching item activates, cut 0.2s off its own cooldown

### Piercer's Band — Gloves ring
  PASSIVE: +20% phys pierce, +20% magic pierce
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Reckoning Plate — Helmet plating
  EVERY TIME IT FIRES: +20 armor
  TRIGGERS: every 6 activations by your other items, deal 34 magic damage to the enemy

### Lightweave — Chestpiece layer
  EVERY TIME IT FIRES (triggered): gain 2 armor

### Deft Mold — Gloves mold
  PASSIVE: +0.20x its own power, +12% curse res
  TRIGGERS: when a touching item activates, deal 5 physical damage to the enemy

### Quickstep Mold — Greaves mold
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Watchful Crest — Helmet crest
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: every 5 activations by your other items, deal 26 magic damage to the enemy

### Rimeguard Base — Chestpiece base
  PASSIVE: +200 hp, +15% magic res
  EVERY TIME IT FIRES (triggered): gain 30 armor

### Tarpit Sole — Greaves mold
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Stonewall Frame — Helmet frame
  PASSIVE: +20% mind res, +18% phys res, +25% magic res
  EVERY TIME IT FIRES: +4 mana
  EVERY TIME IT FIRES (triggered): gain 26 armor

### Anchor Material — Gloves material
  EVERY TIME IT FIRES: +21 armor

### Bulwark Vial — Weapon accessory
  EVERY TIME IT FIRES: +8 armor

### Hexbrand — Weapon damaging
  DAMAGE: +8 magic dmg
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Coven Mold — Gloves mold
  TRIGGERS: when a touching item activates, deal 10 magic damage to the enemy

### Blight Layer — Chestpiece layer
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Malefic Crest — Helmet crest
  EVERY TIME IT FIRES (triggered): deal 22 mind damage to the enemy

### Plaguewalkers — Greaves material
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES (triggered): deal 14 magic damage to the enemy

### Heartwood Base — Chestpiece base
  PASSIVE: +175 hp
  EVERY TIME IT FIRES: +2 nature

### Sapling Mold — Greaves mold
  EVERY TIME IT FIRES: +2 nature
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Bloomguard — Gloves material
  PASSIVE: +1 regen
  EVERY TIME IT FIRES: +4 nature

### Green Crown — Helmet frame
  PASSIVE: +8% mind res
  EVERY TIME IT FIRES: +1 mana, +3 nature

### Oathplate — Chestpiece layer
  PASSIVE: +10% phys res
  EVERY TIME IT FIRES: +2 faith
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Chapel Frame — Helmet frame
  PASSIVE: +10% mind res
  EVERY TIME IT FIRES: +2 mana, +3 faith

### Zealot's Sole — Greaves mold
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: at the start of the fight, gain 12 faith

### Bulwark Base — Chestpiece base
  DAMAGE: +6 phys dmg
  PASSIVE: +225 hp, +22% phys res

### Riveted Layer — Chestpiece layer
  DAMAGE: +5 phys dmg
  PASSIVE: +16% phys res
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Warcry Crest — Helmet crest
  EVERY TIME IT FIRES: +4 rage

### Ravener's Mold — Gloves mold
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: when a touching item activates, gain 1 spellblade

### Runebound Tome — Weapon book
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES: +3 mana

### Seer's Orb — Weapon crystal ball
  DAMAGE: +3 magic dmg
  EVERY TIME IT FIRES: +2 mana

### Starfall — Weapon spell
  DAMAGE: +16 magic dmg
  PASSIVE: +25% magic pierce
  TRIGGERS: when another spell in this item is cast, deal 9 magic damage to the enemy

### Godsteel Haft — Weapon handle
  PASSIVE: +0.70x its own power

### Sunderer — Weapon damaging
  DAMAGE: +34 phys dmg
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Aegis Crown — Helmet frame
  PASSIVE: +20% mind res
  EVERY TIME IT FIRES: +5 mana
  EVERY TIME IT FIRES (triggered): gain 16 armor

### Adamant Carapace — Chestpiece base
  PASSIVE: +450 hp
  EVERY TIME IT FIRES (triggered): gain 30 armor

### Titan's Grip — Gloves material
  PASSIVE: +14 str
  EVERY TIME IT FIRES: +3 mana

### Sevenleague Boots — Greaves material
  PASSIVE: +5 regen
  EVERY TIME IT FIRES (triggered): gain 12 armor

### Pocket Grimoire — Weapon book
  EVERY TIME IT FIRES: +1 mana

### Leaden Tome — Weapon book
  EVERY TIME IT FIRES: +12 armor

### Chained Codex — Weapon book

### Scrying Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when another spell in this item is cast, gain 1 mana

### Hollow Sphere — Weapon crystal ball
  TRIGGERS: per empty cell touching this item: on activation, deal 6 magic damage to the enemy

### Soot Ink — Weapon ink
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Quicksilver Ink — Weapon ink
  TRIGGERS: on activation, spend 3 mana: if it works, deal 14 magic damage to the enemy; if not, apply curse of searing to yourself

### Bloodletter's Ink — Weapon ink
  EVERY TIME IT FIRES (triggered): deal 14 physical damage to yourself

### Emberburst — Weapon spell
  DAMAGE: +14 magic dmg
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Rime Nova — Weapon spell
  DAMAGE: +7 magic dmg
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Siphon — Weapon spell
  DAMAGE: +4 mind
  EVERY TIME IT FIRES: +3 mana

### Warding Sigil — Weapon spell
  EVERY TIME IT FIRES: +9 armor
  TRIGGERS: when another spell in this item is cast, gain 7 armor

### Arc Lightning — Weapon spell
  DAMAGE: +9 magic dmg
  TRIGGERS: when another spell in this item is cast, deal 6 magic damage to the enemy

### Mirrorcast — Weapon spell
  TRIGGERS: when another spell in this item is cast, deal 7 magic damage to the enemy

### Oak Handle — Weapon handle
  PASSIVE: +0.20x its own power

### Balanced Grip — Weapon handle
  PASSIVE: +0.10x its own power

### Iron Blade — Weapon damaging
  DAMAGE: +8 phys dmg
  PASSIVE: +2 str, +0.80x its own power

### Serrated Edge — Weapon damaging
  DAMAGE: +6 phys dmg
  PASSIVE: +4 str, +0.60x its own power

### Ruby Inlay — Weapon accessory
  PASSIVE: +3 str

### Balance Weight — Weapon accessory
  PASSIVE: +0.25x its own power

### Steel Frame — Helmet frame
  PASSIVE: +4% mind res
  EVERY TIME IT FIRES: +12 armor, +3 mana

### Iron Plating — Helmet plating
  EVERY TIME IT FIRES: +15 armor, +1 mana

### Visor of Focus — Helmet plating
  EVERY TIME IT FIRES: +2 armor, +1 mana

### Crest of Vigor — Helmet crest
  PASSIVE: +1 regen
  EVERY TIME IT FIRES: +4 mana

### Padded Base — Chestpiece base
  PASSIVE: +125 hp
  EVERY TIME IT FIRES: +16 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Chain Layer — Chestpiece layer
  PASSIVE: +60 hp
  EVERY TIME IT FIRES: +7 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Plate Layer — Chestpiece layer
  PASSIVE: +90 hp
  EVERY TIME IT FIRES: +10 armor
  EVERY TIME IT FIRES (triggered): gain 4 armor

### Woven Underlayer — Chestpiece layer
  PASSIVE: +30 hp

### Leather Material — Gloves material
  PASSIVE: +2 str
  EVERY TIME IT FIRES: +6 armor

### Steel Material — Gloves material
  PASSIVE: +5 hp, +4 str
  EVERY TIME IT FIRES: +9 armor

### Gauntlet Mold — Gloves mold
  PASSIVE: +1 str

### Gripping Mold — Gloves mold
  PASSIVE: +0.15x its own power, +10% curse res
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when a touching item activates, deal 5 physical damage to the enemy

### Runed Material — Greaves material
  EVERY TIME IT FIRES: +12 armor

### Boiled Leather — Greaves material
  EVERY TIME IT FIRES: +17 armor

### Greave Mold — Greaves mold
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown; apply curse of frost to the enemy

### Runner's Mold — Greaves mold
  EVERY TIME IT FIRES: +2 mana
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Runed Edge — Weapon damaging
  DAMAGE: +5 phys dmg
  PASSIVE: +1 str, +0.45x its own power

### Hollow Weave — Chestpiece layer
  PASSIVE: +20 hp

### Unbound Core — Chestpiece layer
  PASSIVE: +40 hp

### Cursed Handle — Weapon handle
  PASSIVE: +0.30x its own power
  TRIGGERS: on activation, spend 5 mana: if it works, apply curse of searing to the enemy; if not, apply curse of searing to yourself

### Cursed Blade — Weapon damaging
  DAMAGE: +10 phys dmg
  EVERY TIME IT FIRES (triggered): apply curse of searing to yourself

### Bone Frame — Helmet frame
  PASSIVE: +6 hp, +1 regen
  EVERY TIME IT FIRES: +10 armor, +1 mana, +2 rage

### Hide Base — Chestpiece base
  PASSIVE: +70 hp
  EVERY TIME IT FIRES: +12 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Mage's Rod — Weapon handle
  PASSIVE: +0.10x its own power
  EVERY TIME IT FIRES: +3 mana

### Arcane Splinter — Weapon damaging
  DAMAGE: +3 magic dmg
  PASSIVE: +0.20x its own power
  TRIGGERS: on activation, spend 4 mana: if it works, deal 18 magic damage to the enemy; if not, gain 1 mana

### Mana Loom — Chestpiece base
  PASSIVE: +90 hp
  EVERY TIME IT FIRES: +10 armor, +6 mana

### Mage's Circlet — Helmet frame
  PASSIVE: +3% mind res
  EVERY TIME IT FIRES: +8 mana
  TRIGGERS: on activation, spend 6 mana: if it works, gain 1 mana empowerment; if not, gain 2 mana

### Runed Lining — Chestpiece layer
  PASSIVE: +30 hp
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: on activation, spend 3 mana: if it works, apply curse of misfire to the enemy; if not, gain 2 mana

### Mage's Wrapping — Gloves material
  EVERY TIME IT FIRES: +3 mana

### Mage's Sandals — Greaves material
  EVERY TIME IT FIRES: +3 armor, +3 mana

### Scrying Lens — Helmet plating
  DAMAGE: +3 mind
  EVERY TIME IT FIRES: +10 armor

### Overflow Vial — Weapon accessory
  EVERY TIME IT FIRES: +2 mana

### Witch's Crook — Weapon handle
  PASSIVE: +0.20x its own power, +10% curse res
  TRIGGERS: on activation, spend 3 mana: if it works, apply curse of searing to the enemy; if not, apply curse of searing to yourself

### Hexbolt — Weapon damaging
  DAMAGE: +2 mind, +7 magic dmg
  PASSIVE: +0.40x its own power

### Witch's Hat — Helmet frame
  PASSIVE: +4% mind res, +15% curse res
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): deal 12 mind damage to the enemy

### Hexweave Shroud — Chestpiece base
  PASSIVE: +80 hp, +20% curse res
  EVERY TIME IT FIRES: +10 armor
  TRIGGERS: on activation, spend 4 mana: if it works, apply curse of searing to the enemy; if not, gain 4 armor

### Witch's Claw — Gloves material
  PASSIVE: +2 str, +5% curse res
  EVERY TIME IT FIRES (triggered): deal 16 physical damage to the enemy

### Hexer's Mold — Gloves mold
  TRIGGERS: on activation, spend 3 mana: if it works, apply curse of searing to the enemy; if not, gain 1 mana

### Witch's Stilts — Greaves material
  PASSIVE: +22% curse res

### Bileglass Vial — Weapon accessory
  DAMAGE: +1 mind

### Coven Crest — Helmet crest
  PASSIVE: +10% curse res
  TRIGGERS: when an item in another slot on the same rows activates, apply curse of searing to the enemy

### Quickening Charm — Weapon accessory
  TRIGGERS: when another spell in this item is cast, cut 1.0s off its own cooldown

### Chain Coil — Weapon accessory
  TRIGGERS: when another spell in this item is cast, deal 5 physical damage to the enemy

### Channeling Mold — Gloves mold
  TRIGGERS: when an item in another slot on the same rows activates, gain 1 mana

### Striding Mold — Greaves mold
  TRIGGERS: when an item in another slot on the same rows activates, cut 0.5s off its own cooldown

### Thornmail Layer — Chestpiece layer
  PASSIVE: +40 hp
  EVERY TIME IT FIRES: +9 armor

### Third Eye — Helmet crest
  DAMAGE: +2 mind
  TRIGGERS: when an item touching only a corner of this one acts, gain 1 mana

### Ember Crest — Helmet crest
  TRIGGERS: when an item in another slot on the same rows activates, deal 8 magic damage to the enemy

### Grave-Iron Mold — Greaves mold
  EVERY TIME IT FIRES: +11 armor
  EVERY TIME IT FIRES (triggered): apply curse of misfire to the enemy

### Featherweight Mold — Gloves mold
  TRIGGERS: when an item in another slot on the same rows activates, cut 0.4s off its own cooldown

### Warding Plate — Helmet plating
  PASSIVE: +10% curse res
  EVERY TIME IT FIRES: +17 armor, +1 mana

### Mirrored Visor — Helmet plating
  EVERY TIME IT FIRES: +27 armor, +1 mana

### Ironbark Layer — Chestpiece layer
  PASSIVE: +50 hp
  EVERY TIME IT FIRES: +16 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Duelist's Grip — Weapon handle
  PASSIVE: +0.15x its own power

### Executioner's Haft — Weapon handle
  PASSIVE: +0.90x its own power

### Bonesaw — Weapon damaging
  DAMAGE: +9 phys dmg
  PASSIVE: +3 str, +0.30x its own power

### Whetstone — Weapon accessory
  PASSIVE: +4 str

### Pathfinder Material — Greaves material
  PASSIVE: +2 regen
  EVERY TIME IT FIRES: +7 armor

### Bulwark Material — Gloves material
  PASSIVE: +3 str
  EVERY TIME IT FIRES: +14 armor

### Vast Tapestry — Chestpiece layer
  PASSIVE: +30 hp

### Colossus Ring — Chestpiece layer
  PASSIVE: +40 hp

### Sprawling Handwrap — Gloves material
  PASSIVE: +2 str

### Wandering Root — Greaves material
  PASSIVE: +5% curse res

### Broken Crown — Helmet plating
  EVERY TIME IT FIRES: +2 armor, +1 mana

### Empowering Focus — Weapon accessory
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: on activation, spend 4 mana: if it works, gain 1 spell forking; if not, gain 2 mana

### Empowering Mold — Gloves mold
  EVERY TIME IT FIRES: +4 armor, +1 mana
  TRIGGERS: on activation, spend 3 mana: if it works, cut 0.4s off its own cooldown; if not, gain 2 mana

### Mana Ward — Helmet plating
  EVERY TIME IT FIRES: +10 armor, +2 mana
  TRIGGERS: on activation, spend 3 mana: if it works, deal 30 magic damage to the enemy; if not, gain 8 armor

### Aegis Weave — Chestpiece layer
  PASSIVE: +50 hp
  EVERY TIME IT FIRES: +12 armor, +2 mana
  EVERY TIME IT FIRES (triggered): gain 18 armor

### Warded Sabatons — Greaves mold
  PASSIVE: +14% curse res
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: on activation, spend 3 mana: if it works, cut 0.5s off its own cooldown; if not, cut 0.1s off its own cooldown

### Ashfall Ink — Weapon ink
  TRIGGERS: when another spell in this item is cast, apply curse of searing to the enemy

### Tidewrack Ink — Weapon ink
  EVERY TIME IT FIRES: +2 mana

### Wrathwrit Ink — Weapon ink
  EVERY TIME IT FIRES: +2 rage

### Gravebloom Ink — Weapon ink
  EVERY TIME IT FIRES: +2 nature

### Oathbound Ink — Weapon ink
  EVERY TIME IT FIRES: +2 faith

### Mercurial Ink — Weapon ink
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Runewash Ink — Weapon ink
  EVERY TIME IT FIRES: +2 mana

### Cinderscript Ink — Weapon ink
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Glacier Ink — Weapon ink

### Hollow Ink — Weapon ink
  TRIGGERS: per empty cell touching this item: on activation, deal 2 magic damage to the enemy

### Deepwater Ink — Weapon ink
  EVERY TIME IT FIRES: +3 mana

### Starlit Ink — Weapon ink
  TRIGGERS: when an item in another slot on the same rows activates, deal 26 magic damage to the enemy

### Emberdust Ink — Weapon ink
  DAMAGE: +3 magic dmg
  TRIGGERS: on activation, spend 6 rage: if it works, deal 16 magic damage to the enemy; if not, gain 2 rage

### Voidwritten Ink — Weapon ink
  PASSIVE: +20% magic pierce
  EVERY TIME IT FIRES (triggered): deal 4 magic damage to the enemy

### Kingsblood Ink — Weapon ink
  TRIGGERS: on activation, spend 6 mana: if it works, deal 42 magic damage to the enemy; if not, apply curse of searing to yourself

### Echo Sigil — Weapon spell
  DAMAGE: +5 magic dmg
  TRIGGERS: when another spell in this item is cast, gain 3 mana

### Resonant Chord — Weapon spell
  DAMAGE: +7 magic dmg
  TRIGGERS: when another spell in this item is cast, deal 6 magic damage to the enemy

### Attendant Flame — Weapon spell
  DAMAGE: +6 magic dmg
  TRIGGERS: when another spell in this item is cast, apply curse of searing to the enemy

### Mirror Ward — Weapon spell
  EVERY TIME IT FIRES: +8 armor
  TRIGGERS: when another spell in this item is cast, gain 9 armor

### Sympathetic Bloom — Weapon spell
  DAMAGE: +4 magic dmg
  PASSIVE: +1 regen
  TRIGGERS: when another spell in this item is cast, gain 2 nature

### Choir of Ash — Weapon spell
  DAMAGE: +8 magic dmg
  TRIGGERS: when another spell in this item is cast, deal 2 magic damage to the enemy

### Rite of Answer — Weapon spell
  DAMAGE: +6 magic dmg
  TRIGGERS: when another spell in this item is cast, gain 3 faith

### Sunder — Weapon spell
  DAMAGE: +15 magic dmg
  PASSIVE: +35% magic pierce
  EVERY TIME IT FIRES: +2 mana

### Frostbind — Weapon spell
  DAMAGE: +5 magic dmg

### Hollow Lance — Weapon spell
  DAMAGE: +21 magic dmg
  TRIGGERS: per empty cell touching this item: on activation, deal 4 magic damage to the enemy

### Verdant Surge — Weapon spell
  DAMAGE: +5 magic dmg
  TRIGGERS: on activation, spend 4 nature: if it works, gain 8 mana; if not, gain 3 nature

### Blood Rite — Weapon spell
  DAMAGE: +7 magic dmg
  TRIGGERS: on activation, spend 5 rage: if it works, deal 22 magic damage to the enemy; if not, gain 3 rage

### Sanctuary — Weapon spell
  EVERY TIME IT FIRES: +6 armor
  TRIGGERS: on activation, spend 4 faith: if it works, gain 20 armor; if not, gain 12 armor

### Cometfall — Weapon spell
  DAMAGE: +26 magic dmg
  TRIGGERS: on activation, spend 5 mana: if it works, apply curse of stun to the enemy; if not, apply curse of searing to the enemy

### Unmaking — Weapon spell
  DAMAGE: +8 magic dmg
  TRIGGERS: when another spell in this item is cast, deal 3 magic damage to the enemy

### Azure Alignment — Weapon alignment
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: per empty cell touching this item: on activation, gain 1 mana

### Crimson Alignment — Weapon alignment
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: when another spell in this item is cast, gain 3 rage

### Golden Alignment — Weapon alignment
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: per empty cell touching this item: on activation, spend 4 faith: if it works, gain 1 spell forking; if not, gain 2 faith

### Verdant Alignment — Weapon alignment
  EVERY TIME IT FIRES: +2 nature
  TRIGGERS: when another spell in this item is cast, gain 3 nature

### Tidal Alignment — Weapon alignment
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: on activation, spend 4 mana: if it works, gain 1 spell forking; if not, gain 2 mana

### Ember Alignment — Weapon alignment
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: on activation, spend 8 rage: if it works, apply curse of searing to the enemy; if not, gain 3 rage

### Pilgrim Alignment — Weapon alignment
  EVERY TIME IT FIRES: +5 armor, +2 faith
  TRIGGERS: on activation, spend 12 faith: if it works, gain 22 armor; if not, gain 3 faith

### Rootwork Alignment — Weapon alignment
  PASSIVE: +1 regen
  EVERY TIME IT FIRES: +3 nature
  EVERY TIME IT FIRES (triggered): deal 24 magic damage to the enemy

### Prism Alignment — Weapon alignment
  EVERY TIME IT FIRES: +1 mana, +1 rage, +1 faith, +1 nature
  TRIGGERS: per empty cell touching this item: on activation, gain 1 mana

### Void Alignment — Weapon alignment
  DAMAGE: +2 magic dmg
  PASSIVE: +25% magic pierce
  EVERY TIME IT FIRES (triggered): deal 3 magic damage to the enemy

### Ash Haft — Weapon handle
  PASSIVE: +2 str

### Corded Grip — Weapon handle
  PASSIVE: +4 str

### Ironbound Haft — Weapon handle
  PASSIVE: +7 str

### Duelist's Hilt — Weapon handle
  PASSIVE: +3 str

### Whipcord Hilt — Weapon handle
  PASSIVE: +5 str

### Warden's Haft — Weapon handle
  PASSIVE: +100 hp, +9 str

### Sunder Haft — Weapon handle
  PASSIVE: +13 str

### Twinned Grip — Weapon handle
  PASSIVE: +6 str

### Gravebound Haft — Weapon handle
  PASSIVE: +10 str, +10% magic res

### Kingmaker Hilt — Weapon handle
  PASSIVE: +16 str, +0.25x its own power

### Chipped Edge — Weapon damaging
  DAMAGE: +6 phys dmg

### Hooked Edge — Weapon damaging
  DAMAGE: +10 phys dmg

### Sawtooth Edge — Weapon damaging
  DAMAGE: +15 phys dmg

### Bronze Fang — Weapon damaging
  DAMAGE: +9 phys dmg

### Iron Fang — Weapon damaging
  DAMAGE: +15 phys dmg

### Adamant Fang — Weapon damaging
  DAMAGE: +23 phys dmg
  PASSIVE: +25% phys pierce

### Witchglass Shard — Weapon damaging
  DAMAGE: +12 magic dmg

### Voidglass Shard — Weapon damaging
  DAMAGE: +20 magic dmg
  PASSIVE: +30% magic pierce

### Reaver's Bill — Weapon damaging
  DAMAGE: +18 phys dmg
  PASSIVE: +15% phys pierce

### Worldsplitter — Weapon damaging
  DAMAGE: +30 phys dmg
  PASSIVE: +35% phys pierce

### Bone Charm — Weapon accessory
  PASSIVE: +2 str
  TRIGGERS: every 10 activations by your other items, gain 1 spellblade

### Silver Charm — Weapon accessory
  PASSIVE: +4 str

### Loaded Fob — Weapon accessory
  PASSIVE: +0.20x its own power

### Duelist's Fob — Weapon accessory
  PASSIVE: +0.35x its own power

### Windup Key — Weapon accessory
  PASSIVE: +0.15x its own power

### Clockwork Key — Weapon accessory
  PASSIVE: +0.25x its own power

### Ratchet Cog — Weapon accessory
  TRIGGERS: every 8 activations by your other items, gain 1 spellblade

### Flywheel Cog — Weapon accessory

### Bloodstone Bead — Weapon accessory
  DAMAGE: +4 phys dmg
  EVERY TIME IT FIRES: +2 rage

### Oathstone Bead — Weapon accessory
  PASSIVE: +6% magic res
  EVERY TIME IT FIRES: +2 faith

### Tin Frame — Helmet frame
  PASSIVE: +2% mind res
  EVERY TIME IT FIRES: +6 armor, +2 mana

### Bronze Frame — Helmet frame
  PASSIVE: +5% mind res
  EVERY TIME IT FIRES: +10 armor, +3 mana

### Warded Frame — Helmet frame
  PASSIVE: +9% mind res, +8% magic res
  EVERY TIME IT FIRES: +16 armor, +1 mana
  EVERY TIME IT FIRES (triggered): gain 1 mana shield

### Ridged Frame — Helmet frame
  PASSIVE: +7% mind res
  EVERY TIME IT FIRES: +14 armor, +1 mana, +2 faith

### Buttressed Frame — Helmet frame
  PASSIVE: +12% mind res, +10% phys res
  EVERY TIME IT FIRES: +22 armor, +2 mana
  EVERY TIME IT FIRES (triggered): gain 1 mana shield

### Hollowbone Frame — Helmet frame
  EVERY TIME IT FIRES: +8 armor, +2 mana
  TRIGGERS: every 8 activations by your other items, gain 4 mana

### Ossuary Frame — Helmet frame
  EVERY TIME IT FIRES: +12 armor, +2 faith
  TRIGGERS: every 3 curses landing on either side, gain 3 rage

### Stormcaught Frame — Helmet frame
  PASSIVE: +14% mind res, +14% magic res
  EVERY TIME IT FIRES: +26 armor, +2 mana
  TRIGGERS: every 2 activations by items meeting it at a corner, gain 2 mana

### Anvil Frame — Helmet frame
  PASSIVE: +18% mind res, +20% phys res
  EVERY TIME IT FIRES: +34 armor, +3 mana

### Crown of Nails — Helmet frame
  DAMAGE: +6 phys dmg
  EVERY TIME IT FIRES: +11 armor
  TRIGGERS: every 10 activations by your other items, gain 1 mana empowerment

### Tin Plating — Helmet plating
  EVERY TIME IT FIRES: +5 armor

### Bronze Plating — Helmet plating
  EVERY TIME IT FIRES: +9 armor

### Layered Plating — Helmet plating
  EVERY TIME IT FIRES: +14 armor

### Scaled Plating — Helmet plating
  PASSIVE: +8% phys res
  EVERY TIME IT FIRES: +11 armor

### Runed Plating — Helmet plating
  PASSIVE: +8% magic res
  EVERY TIME IT FIRES: +11 armor

### Warded Plating — Helmet plating
  PASSIVE: +12% magic res
  EVERY TIME IT FIRES: +17 armor

### Bulwark Plating — Helmet plating
  PASSIVE: +18% phys res
  EVERY TIME IT FIRES: +24 armor

### Mirrorbright Plating — Helmet plating
  PASSIVE: +20% magic res
  EVERY TIME IT FIRES: +13 armor

### Deadweight Plating — Helmet plating
  EVERY TIME IT FIRES: +27 armor, +1 mana

### Godsteel Plating — Helmet plating
  PASSIVE: +14% phys res, +14% magic res
  EVERY TIME IT FIRES: +30 armor

### Feather Crest — Helmet crest
  PASSIVE: +1 regen
  EVERY TIME IT FIRES (triggered): deal 2 mind damage to the enemy

### Gilded Crest — Helmet crest
  PASSIVE: +2 regen
  EVERY TIME IT FIRES: +2 mana

### Seer's Crest — Helmet crest
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 3 mana: if it works, apply curse of misfire to the enemy; if not, gain 2 mana

### Zealot's Crest — Helmet crest
  EVERY TIME IT FIRES: +6 faith

### Berserker's Crest — Helmet crest
  EVERY TIME IT FIRES: +6 rage

### Bloomed Crest — Helmet crest
  EVERY TIME IT FIRES: +5 nature

### Warlord's Crest — Helmet crest
  PASSIVE: +6 str
  EVERY TIME IT FIRES: +2 rage
  EVERY TIME IT FIRES (triggered): deal 3 mind damage to the enemy

### Archon's Crest — Helmet crest
  PASSIVE: +0.30x its own power
  TRIGGERS: every 6 activations by your other items, deal 4 mind damage to the enemy

### Martyr's Crest — Helmet crest
  PASSIVE: +2 regen, +16% mind res
  EVERY TIME IT FIRES: +3 mana
  EVERY TIME IT FIRES (triggered): gain 1 mana shield

### Crown of the Deep — Helmet crest
  PASSIVE: +0.25x its own power, +20% magic pierce
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: on activation, spend 4 mana: if it works, apply curse of stun to the enemy; if not, deal 3 mind damage to the enemy

### Sackcloth Base — Chestpiece base
  PASSIVE: +90 hp
  EVERY TIME IT FIRES (triggered): gain 2 armor

### Quilted Base — Chestpiece base
  PASSIVE: +150 hp
  EVERY TIME IT FIRES (triggered): gain 4 armor

### Brigandine Base — Chestpiece base
  PASSIVE: +220 hp
  EVERY TIME IT FIRES: +8 armor
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Ribbed Base — Chestpiece base
  PASSIVE: +260 hp
  EVERY TIME IT FIRES: +12 armor
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Bastion Base — Chestpiece base
  PASSIVE: +350 hp, +10% phys res
  EVERY TIME IT FIRES: +20 armor
  EVERY TIME IT FIRES (triggered): gain 6 armor

### Cinder Base — Chestpiece base
  PASSIVE: +170 hp
  EVERY TIME IT FIRES: +2 rage
  EVERY TIME IT FIRES (triggered): gain 4 armor

### Grove Base — Chestpiece base
  PASSIVE: +170 hp, +2 regen
  EVERY TIME IT FIRES: +2 nature
  EVERY TIME IT FIRES (triggered): gain 4 maximum health for the rest of the fight

### Chapel Base — Chestpiece base
  PASSIVE: +150 hp
  EVERY TIME IT FIRES: +2 faith
  EVERY TIME IT FIRES (triggered): gain 5 armor

### Wellspring Base — Chestpiece base
  PASSIVE: +130 hp
  EVERY TIME IT FIRES: +3 mana

### Adamant Base — Chestpiece base
  PASSIVE: +440 hp, +12% magic res
  EVERY TIME IT FIRES: +26 armor
  EVERY TIME IT FIRES (triggered): gain 7 armor

### Rag Layer — Chestpiece layer
  EVERY TIME IT FIRES: +6 armor
  EVERY TIME IT FIRES (triggered): gain 1 armor

### Felt Layer — Chestpiece layer
  EVERY TIME IT FIRES: +11 armor
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Mail Layer — Chestpiece layer
  EVERY TIME IT FIRES: +17 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Scale Layer — Chestpiece layer
  PASSIVE: +8% phys res
  EVERY TIME IT FIRES: +15 armor
  EVERY TIME IT FIRES (triggered): gain 3 armor

### Sigil Layer — Chestpiece layer
  PASSIVE: +8% magic res
  EVERY TIME IT FIRES: +15 armor
  EVERY TIME IT FIRES (triggered): gain 2 armor

### Thorn Layer — Chestpiece layer
  DAMAGE: +7 phys dmg
  EVERY TIME IT FIRES: +9 armor
  EVERY TIME IT FIRES (triggered): gain 2 armor

### Mending Layer — Chestpiece layer
  PASSIVE: +2 regen
  EVERY TIME IT FIRES: +9 armor
  EVERY TIME IT FIRES (triggered): gain 2 maximum health for the rest of the fight

### Bulwark Layer — Chestpiece layer
  PASSIVE: +16% phys harden
  EVERY TIME IT FIRES: +24 armor
  EVERY TIME IT FIRES (triggered): gain 5 armor

### Aether Layer — Chestpiece layer
  PASSIVE: +16% magic harden
  EVERY TIME IT FIRES: +20 armor, +2 mana
  TRIGGERS: on activation, spend 4 mana: if it works, apply curse of stun to the enemy; if not, gain 10 armor

### Godsheet Layer — Chestpiece layer
  PASSIVE: +150 hp, +12% phys res, +12% magic res
  EVERY TIME IT FIRES: +34 armor
  EVERY TIME IT FIRES (triggered): gain 6 armor

### Hide Material — Gloves material
  PASSIVE: +1 str, +4 regen
  EVERY TIME IT FIRES: +10 armor

### Waxed Material — Gloves material
  PASSIVE: +3 str, +7 regen
  EVERY TIME IT FIRES: +18 armor

### Scaled Material — Gloves material
  PASSIVE: +5 str, +10 regen, +8% phys res
  EVERY TIME IT FIRES: +26 armor

### Spun Material — Gloves material
  PASSIVE: +0.12x its own power
  EVERY TIME IT FIRES: +2 mana

### Sanctified Material — Gloves material
  PASSIVE: +8% magic res
  EVERY TIME IT FIRES: +2 faith

### Ashwoven Material — Greaves material
  DAMAGE: +5 phys dmg
  EVERY TIME IT FIRES: +2 rage

### Rootwoven Material — Greaves material
  PASSIVE: +8% curse res
  EVERY TIME IT FIRES: +2 nature

### Ironthread Material — Greaves material
  PASSIVE: +7 regen
  EVERY TIME IT FIRES: +14 armor

### Duskweave Material — Greaves material
  PASSIVE: +22% magic pierce
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 3 mana: if it works, deal 24 magic damage to the enemy; if not, gain 1 mana

### Worldweave Material — Greaves material
  PASSIVE: +8 str, +10 regen
  EVERY TIME IT FIRES: +20 armor

### Padded Mold — Gloves mold
  PASSIVE: +2 str
  TRIGGERS: when a touching item activates, gain 2 armor

### Braced Mold — Gloves mold
  PASSIVE: +4 str
  TRIGGERS: when a touching item activates, gain 3 armor

### Vicegrip Mold — Gloves mold
  PASSIVE: +7 str
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Nimble Mold — Gloves mold
  PASSIVE: +0.25x its own power
  TRIGGERS: when an item in another slot on the same rows activates, cut 0.3s off its own cooldown

### Quickfinger Mold — Gloves mold
  PASSIVE: +0.35x its own power
  TRIGGERS: when a touching item activates, cut 0.4s off its own cooldown

### Warding Mold — Gloves mold
  PASSIVE: +8% magic res
  EVERY TIME IT FIRES: +14 armor
  TRIGGERS: when a touching item activates, gain 1 spellblade

### Rending Mold — Gloves mold
  DAMAGE: +11 phys dmg
  PASSIVE: +18% phys pierce
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Oathkeeper Mold — Gloves mold
  EVERY TIME IT FIRES: +10 armor, +3 faith
  TRIGGERS: when an item in another slot on the same rows activates, deal 4 physical damage to the enemy

### Wrathful Mold — Gloves mold
  DAMAGE: +8 phys dmg
  EVERY TIME IT FIRES: +3 rage
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Sovereign Mold — Gloves mold
  PASSIVE: +11 str, +0.30x its own power
  EVERY TIME IT FIRES: +12 armor
  TRIGGERS: on activation, per adjacent assembled item, deal 4 physical damage to the enemy

### Plain Sole — Greaves mold
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Sprung Sole — Greaves mold
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown; apply curse of stun to the enemy

### Racing Sole — Greaves mold
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Anchored Sole — Greaves mold
  PASSIVE: +130 hp
  EVERY TIME IT FIRES: +8 armor
  TRIGGERS: every 5 activations by your other items, apply curse of searing to the enemy

### Trailworn Sole — Greaves mold
  PASSIVE: +8% curse res
  EVERY TIME IT FIRES: +2 nature
  EVERY TIME IT FIRES (triggered): apply curse of misfire to the enemy

### Pilgrim Sole — Greaves mold
  PASSIVE: +10% magic res
  EVERY TIME IT FIRES: +3 faith

### Ironshod Sole — Greaves mold
  EVERY TIME IT FIRES: +34 armor
  TRIGGERS: every 3 activations by items sharing its rows, apply curse of searing to the enemy

### Stormstep Mold — Greaves mold
  PASSIVE: +0.18x its own power
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 3 mana: if it works, apply curse of stun to the enemy; if not, cut 0.2s off its own cooldown

### Gravewalker Mold — Greaves mold
  PASSIVE: +25% curse res
  EVERY TIME IT FIRES (triggered): apply curse of searing to the enemy

### Worldstrider Sole — Greaves mold
  PASSIVE: +200 hp, +4 regen
  EVERY TIME IT FIRES: +18 armor

### Tin Band — Gloves ring
  PASSIVE: +2 str
  TRIGGERS: when a touching item activates, cut 0.2s off its own cooldown

### Silver Band — Gloves ring
  PASSIVE: +4 str
  TRIGGERS: when a touching item activates, deal 5 magic damage to the enemy

### Signet of Iron — Gloves ring
  DAMAGE: +6 phys dmg
  TRIGGERS: when a touching item activates, gain 3 armor

### Signet of Ash — Gloves ring
  DAMAGE: +6 magic dmg
  TRIGGERS: when a touching item activates, deal 5 magic damage to the enemy

### Ring of Wells — Gloves ring
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when an item in another slot on the same rows activates, gain 1 spellblade

### Ring of Embers — Gloves ring
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: when a touching item activates, deal 10 magic damage to the enemy

### Ring of Vigils — Gloves ring
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: when an item in another slot on the same rows activates, deal 4 physical damage to the enemy

### Ring of Roots — Gloves ring
  EVERY TIME IT FIRES: +2 nature
  TRIGGERS: when a touching item activates, cut 0.2s off its own cooldown

### Seal of Power — Gloves ring
  PASSIVE: +0.30x its own power
  TRIGGERS: when a touching item activates, deal 10 physical damage to the enemy

### Seal of the Deep — Gloves ring
  PASSIVE: +0.20x its own power, +20% magic pierce
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: when a touching item activates, drain 2 mana from the enemy

### Chapbook — Weapon book
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): cut 0.2s off its own cooldown

### Traveller's Codex — Weapon book
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when an item in another slot on the same rows activates, gain 1 mana

### Scholar's Codex — Weapon book
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, gain 2 mana

### Hymnal — Weapon book
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: on activation, spend 10 faith: if it works, gain 18 armor; if not, gain 3 faith

### War Ledger — Weapon book
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: on activation, spend 10 rage: if it works, deal 24 physical damage to the enemy; if not, gain 3 rage

### Herbal — Weapon book
  EVERY TIME IT FIRES: +2 nature
  TRIGGERS: every 10 activations by your other items, gain 1 spell forking

### Quickread Folio — Weapon book
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): cut 0.3s off its own cooldown

### Whisperbound Tome — Weapon book
  PASSIVE: +12% magic res
  EVERY TIME IT FIRES: +2 mana
  EVERY TIME IT FIRES (triggered): deal 3 magic damage to the enemy

### Grand Grimoire — Weapon book
  PASSIVE: +0.20x its own power
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: when another spell in this item is cast, deal 8 magic damage to the enemy

### Codex Interminable — Weapon book
  PASSIVE: +0.35x its own power
  EVERY TIME IT FIRES: +4 mana
  TRIGGERS: per empty cell touching this item: on activation, deal 5 magic damage to the enemy

### Clouded Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when another spell in this item is cast, deal 2 magic damage to the enemy

### Polished Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, deal 5 magic damage to the enemy

### Fateglass Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: when another spell in this item is cast, apply curse of misfire to the enemy

### Tidecaller Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: when another spell in this item is cast, gain 2 mana

### Emberheart Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +3 rage
  TRIGGERS: on activation, spend 9 rage: if it works, deal 26 magic damage to the enemy; if not, gain 3 rage

### Grovemind Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +3 nature
  EVERY TIME IT FIRES (triggered): deal 34 magic damage to the enemy

### Reliquary Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +3 faith
  TRIGGERS: on activation, spend 9 faith: if it works, gain 1 spell forking; if not, gain 3 faith

### Spinning Orb — Weapon crystal ball
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, cut 0.2s off its own cooldown

### Orb of the Nine — Weapon crystal ball
  PASSIVE: +0.25x its own power
  EVERY TIME IT FIRES: +4 mana
  TRIGGERS: per empty cell touching this item: on activation, spend 2 mana: if it works, deal 11 magic damage to the enemy; if not, gain 1 mana

### Worldeye Orb — Weapon crystal ball
  PASSIVE: +0.40x its own power, +20% magic pierce
  EVERY TIME IT FIRES: +5 mana
  TRIGGERS: when another spell in this item is cast, apply curse of stun to the enemy

### The Money Jacket — Chestpiece base
  PASSIVE: +2100 hp, +26 str, +9 regen, +40% curse res, +40% phys res, +30% phys harden, +40% magic res, +30% magic harden
  EVERY TIME IT FIRES: +90 armor
  EVERY TIME IT FIRES (triggered): gain 70 armor; deal 40 physical damage to the enemy

### Heartwood Crest — Helmet crest
  PASSIVE: +5% mind res
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): gain 1 mana shield

### The Growing Weight — Chestpiece layer
  PASSIVE: +90 hp
  EVERY TIME IT FIRES: +10 armor
  EVERY TIME IT FIRES (triggered): gain 60 maximum health for the rest of the fight

### Grasping Ring — Gloves ring
  PASSIVE: +40 hp
  TRIGGERS: when a touching item activates, drain 3 mana from the enemy and deal 1 magic for each point

### Deeprooted Sole — Greaves mold
  PASSIVE: +12% curse res
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Gluttonous Fang — Weapon damaging
  DAMAGE: +9 phys dmg
  EVERY TIME IT FIRES (triggered): deal 30 physical damage to the enemy

### Hermit's Band — Gloves ring
  PASSIVE: +40 hp, +3 str
  TRIGGERS: when a touching item activates, gain 1 spellblade

### The Empty Crown — Helmet crest
  PASSIVE: +4% mind res
  EVERY TIME IT FIRES: +12 armor, +1 mana

### Lonely Plating — Helmet plating
  EVERY TIME IT FIRES: +14 armor

### Widow's Sole — Greaves mold
  PASSIVE: +18% curse res

### Bare-Headed Fang — Weapon damaging
  DAMAGE: +11 phys dmg

### Ungloved Layer — Chestpiece layer
  PASSIVE: +50 hp
  EVERY TIME IT FIRES: +16 armor

### Unshod Signet — Gloves ring
  DAMAGE: +7 magic dmg
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when an item in another slot on the same rows activates, apply curse of misfire to the enemy

### Reckoning Crest — Helmet crest
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: on activation, spend all your faith: per 6 spent, deal 11 magic damage to the enemy

### Zealot's Haft — Weapon handle
  PASSIVE: +2 str
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: on activation, spend 7 faith: if it works, deal 19 physical damage to the enemy; if not, gain 3 faith

### Bramble Mold — Gloves mold
  EVERY TIME IT FIRES: +2 nature
  TRIGGERS: on activation, spend 5 nature: if it works, deal 21 physical damage to the enemy; if not, gain 3 nature

### Wildfire Layer — Chestpiece layer
  PASSIVE: +40 hp
  EVERY TIME IT FIRES (triggered): deal 16 magic damage to the enemy

### Scarred Plating — Helmet plating
  EVERY TIME IT FIRES: +6 armor, +2 rage
  TRIGGERS: on activation, spend 6 rage: if it works, gain 30 armor; if not, gain 3 rage

### Bloodbank Base — Chestpiece base
  PASSIVE: +60 hp
  EVERY TIME IT FIRES (triggered): gain 18 armor

### Wellspring Sole — Greaves mold
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 4 mana: if it works, cut 0.4s off its own cooldown; if not, gain 2 mana

### Deepdraught Ring — Gloves ring
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: when a touching item activates, drain 4 mana from the enemy and deal 5 magic for each point

### Tithe Ring — Gloves ring
  EVERY TIME IT FIRES: +1 faith
  TRIGGERS: on activation, spend 5 faith: if it works, gain 8 rage; if not, gain 2 faith

### Ashen Material — Gloves material
  PASSIVE: +3 regen
  EVERY TIME IT FIRES: +7 armor, +2 rage
  TRIGGERS: on activation, spend 5 rage: if it works, gain 8 nature; if not, gain 2 rage

### Covenant Frame — Helmet frame
  PASSIVE: +3% mind res
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: on activation, spend 4 mana: if it works, gain 7 faith; if not, gain 2 mana

### Reliquary Sole — Greaves material
  PASSIVE: +12% curse res
  EVERY TIME IT FIRES: +2 faith

### Grudge Bead — Weapon accessory
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: every 3 curses landing on either side, deal 30 physical damage to the enemy

### Harvest Crest — Helmet crest
  EVERY TIME IT FIRES: +2 nature
  TRIGGERS: on activation, spend all your nature: per 6 spent, gain 4 mana

### Overflow Plate — Greaves plating
  PASSIVE: +14% curse res
  EVERY TIME IT FIRES: +3 faith

### Last Rite — Weapon spell
  DAMAGE: +6 magic dmg
  EVERY TIME IT FIRES (triggered): deal 26 magic damage to the enemy

### Asker's Monocle — Helmet crest
  DAMAGE: +26 mind
  PASSIVE: +45% mind res, +30% magic pierce
  EVERY TIME IT FIRES: +7 mana
  EVERY TIME IT FIRES (triggered): deal 18 mind damage to the enemy

### Toolwright's Grip — Weapon handle
  PASSIVE: +30 str, +30% phys pierce
  EVERY TIME IT FIRES (triggered): cut 0.4s off its own cooldown

### Kaklon's Patent — Weapon accessory
  PASSIVE: +0.90x its own power
  EVERY TIME IT FIRES: +5 mana
  TRIGGERS: when another spell in this item is cast, gain 1 spell forking

### Eighth Ray Crown — Helmet frame
  PASSIVE: +900 hp, +34% magic res
  EVERY TIME IT FIRES: +40 armor, +6 faith
  TRIGGERS: when an item touching only a corner of this one acts, deal 4 mind damage to the enemy; on activation, spend all your faith: per 5 spent, gain 1 mana shield

### Assassin's Hemline — Chestpiece layer
  PASSIVE: +22 str, +45% phys pierce, +45% magic pierce
  EVERY TIME IT FIRES (triggered): apply curse of misfire to the enemy

### Handman's Peel — Weapon damaging
  DAMAGE: +88 magic dmg
  PASSIVE: +55% magic pierce

### Gilded Offcuts — Greaves material
  PASSIVE: +80 regen, +34% phys res, +34% magic res
  EVERY TIME IT FIRES: +60 armor
  EVERY TIME IT FIRES (triggered): gain 48 armor

### Henpeck's Cell Keys — Gloves ring
  PASSIVE: +52 str, +60% curse res, +45% phys pierce
  EVERY TIME IT FIRES: +12 mana
  TRIGGERS: on activation, spend 4 mana: if it works, apply curse of stun to the enemy; if not, gain 3 mana

### The Seeker's Tears — Weapon crystal ball
  DAMAGE: +38 magic dmg
  PASSIVE: +45% magic pierce
  EVERY TIME IT FIRES: +14 mana
  TRIGGERS: when another spell in this item is cast, deal 40 magic damage to the enemy

### Tetrahedron Shard — Weapon alignment
  DAMAGE: +10 mind
  EVERY TIME IT FIRES: +7 mana, +4 rage, +4 faith, +4 nature
  TRIGGERS: per empty cell touching this item: on activation, deal 14 magic damage to the enemy

### Braced Plating — Helmet plating
  EVERY TIME IT FIRES: +40 armor, +1 mana
  EVERY TIME IT FIRES (triggered): gain 20 armor

### Standing Start — Greaves mold
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: at the start of the fight, gain 9 mana

### Opening Grudge — Gloves ring
  EVERY TIME IT FIRES: +1 rage
  TRIGGERS: every 1 activation by another of your items, gain 14 rage (once a fight); when a touching item activates, deal 25 physical damage to the enemy

### Vigil Crest — Helmet crest
  EVERY TIME IT FIRES: +1 faith
  TRIGGERS: every 1 activation by another of your items, gain 14 faith (once a fight); on activation, spend all your faith: per 7 spent, gain 1 mana shield

### Seedbed Layer — Chestpiece layer
  PASSIVE: +45 hp
  EVERY TIME IT FIRES: +1 nature
  TRIGGERS: on activation, spend 5 nature: if it works, gain 30 maximum health for the rest of the fight; if not, gain 4 nature

### First Word — Weapon spell
  DAMAGE: +5 magic dmg
  TRIGGERS: every 1 activation by another of your items, deal 34 magic damage to the enemy (once a fight)

### Ambusher's Grip — Weapon handle
  PASSIVE: +5 str

### Bulwark Bead — Weapon accessory

### Warmed Material — Gloves material
  PASSIVE: +4 str
  EVERY TIME IT FIRES: +6 armor

### Deep Roots Base — Chestpiece base
  PASSIVE: +180 hp
  EVERY TIME IT FIRES: +2 nature
  EVERY TIME IT FIRES (triggered): gain 20 maximum health for the rest of the fight

### The Idiot's Gift — Helmet crest
  DAMAGE: +30 mind
  PASSIVE: +8 regen, +55% mind res
  EVERY TIME IT FIRES: +6 nature
  TRIGGERS: every 1 activation by another of your items, gain 140 armor (once a fight); every 8 activations by your other items, deal 6 mind damage to the enemy; on activation, spend all your nature: per 6 spent, deal 9 mind damage to the enemy

### Forked Crest — Helmet crest
  EVERY TIME IT FIRES: +2 faith
  TRIGGERS: on activation, spend 14 faith: if it works, gain 2 mana empowerment; if not, gain 4 faith

### Split Weave — Chestpiece layer
  PASSIVE: +40 hp

### Twinning Mold — Gloves mold
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: on activation, spend 8 mana: if it works, drain 4 mana from the enemy and deal 4 magic for each point; if not, gain 3 mana

### Echo Sole — Greaves mold
  EVERY TIME IT FIRES: +2 rage
  TRIGGERS: on activation, spend 14 rage: if it works, cut 0.4s off its own cooldown; if not, gain 4 rage

### Forking Bead — Weapon accessory
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: every 8 activations by your other items, gain 1 spell forking

### The Split Wisdom — Weapon accessory
  PASSIVE: +0.90x its own power
  EVERY TIME IT FIRES: +6 mana
  TRIGGERS: when another spell in this item is cast, gain 1 spell forking

### Kingsbane — Weapon spell
  DAMAGE: +18 magic dmg
  EVERY TIME IT FIRES (triggered): apply curse of stun to the enemy

### Leech Bead — Weapon accessory
  DAMAGE: +4 magic dmg

### Doubter's Crest — Helmet crest
  PASSIVE: +2% mind res, +8% curse res
  EVERY TIME IT FIRES: +1 mana
  EVERY TIME IT FIRES (triggered): deal 3 mind damage to the enemy

### Becalming Layer — Chestpiece layer
  PASSIVE: +55 hp, +6% phys res, +12% phys harden

### Blightfinger — Gloves ring
  DAMAGE: +5 magic dmg
  EVERY TIME IT FIRES (triggered): drain 3 nature from the enemy

### Sump Sole — Greaves mold
  PASSIVE: +10% curse res
  EVERY TIME IT FIRES: +3 mana
  EVERY TIME IT FIRES (triggered): apply curse of misfire to the enemy

### Tithe Collector — Helmet crest
  PASSIVE: +3% mind res, +8% magic res
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: on activation, spend all your faith: per 3 spent, deal 4 mind damage to the enemy

### Wrathbreaker — Chestpiece layer
  PASSIVE: +62 hp, +7% phys res

### Witherroot — Greaves mold
  DAMAGE: +6 magic dmg
  PASSIVE: +14% curse res
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Manaflay — Weapon accessory
  DAMAGE: +5 magic dmg

### Gold Chip — Weapon accessory
  DAMAGE: +3 magic dmg
  TRIGGERS: on activation, spend 5 fnorp to deal 4 magic damage to the enemy - and again harder each time, up to 40 fnorp a fight

### Platinum Chip — Weapon quest

### Overseer's Circlet — Helmet frame
  PASSIVE: +210 hp, +30% mind res, +26% phys res, +26% magic res
  EVERY TIME IT FIRES (triggered): gain 40 armor

### Foreman's Harness — Chestpiece base
  PASSIVE: +420 hp, +20% phys res, +30% phys harden
  EVERY TIME IT FIRES (triggered): gain 18 maximum health for the rest of the fight

### Tallykeeper's Weave — Gloves material
  PASSIVE: +10 regen, +12% curse res
  EVERY TIME IT FIRES: +24 armor, +10 mana
  TRIGGERS: every 6 activations by your other items, deal 40 physical damage to the enemy

### Treadmill Sole — Greaves mold
  PASSIVE: +150 hp
  EVERY TIME IT FIRES (triggered): cut 0.4s off its own cooldown

### Quota Edge — Weapon damaging
  DAMAGE: +88 phys dmg
  PASSIVE: +20 str, +45% phys pierce
  EVERY TIME IT FIRES (triggered): deal 6 magic damage to the enemy

### Lamplighter's Cage — Helmet frame
  PASSIVE: +165 hp, +8% mind res
  EVERY TIME IT FIRES: +3 faith
  TRIGGERS: every 4 activations by items sharing its rows, gain 3 mana

### Wickstub — Chestpiece layer
  PASSIVE: +55 hp
  EVERY TIME IT FIRES: +9 armor

### Toll-Taker's Mitt — Gloves ring
  PASSIVE: +70 hp, +12% curse res
  TRIGGERS: when a touching item activates, gain 1 spellblade

### Ridge Runner — Greaves mold
  PASSIVE: +90 hp
  EVERY TIME IT FIRES: +12 armor, +4 nature
  EVERY TIME IT FIRES (triggered): gain 1 deflection

### Kettleworks Pin — Weapon damaging
  DAMAGE: +26 phys dmg
  PASSIVE: +6 str

### Crownwright's Measure — Helmet crest
  PASSIVE: +120 hp, +14% mind res
  EVERY TIME IT FIRES: +9 faith

### The Green Ledger — Chestpiece layer
  PASSIVE: +240 hp, +10% curse res
  EVERY TIME IT FIRES: +17 nature

### A Word About the Crownwright — Helmet quest

### A Word About the Green Ledger — Helmet quest

### Sprocketman's Gratitude — Chestpiece layer
  PASSIVE: +60 hp, +10% curse res

### Slash and Burn — Weapon spell
  DAMAGE: +8 magic dmg
  EVERY TIME IT FIRES: +4 nature
  TRIGGERS: on activation, spend 8 nature: if it works, apply curse of searing to the enemy; if not, gain 3 nature

### Scrap Ticket — Helmet quest

### Hoarfrost Mold — Greaves mold
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Rimebound Mold — Greaves mold
  PASSIVE: +4% curse res
  EVERY TIME IT FIRES (triggered): apply curse of frost to the enemy

### Glacier Mold — Greaves mold
  TRIGGERS: every 3 activations by items sharing its rows, apply curse of frost to the enemy

### Frostbite Mold — Greaves mold
  TRIGGERS: at the start of the fight, apply curse of frost to the enemy

### Coldstep Mold — Greaves mold
  PASSIVE: +6% curse res
  TRIGGERS: every 5 activations by your other items, apply curse of frost to the enemy

### Deepwinter Mold — Greaves mold
  TRIGGERS: every 3 activations by items sharing its rows, apply curse of frost to the enemy

### Stumblefoot Mold — Greaves mold
  EVERY TIME IT FIRES (triggered): apply curse of stun to the enemy

### Ambush Mold — Greaves mold
  TRIGGERS: at the start of the fight, apply curse of stun to the enemy

### Tripwire Mold — Greaves mold
  TRIGGERS: every 5 activations by items sharing its rows, apply curse of stun to the enemy

### Deadfall Mold — Greaves mold
  PASSIVE: +3% curse res
  TRIGGERS: at the start of the fight, apply curse of stun to the enemy

### Hobbling Mold — Greaves mold
  TRIGGERS: every 6 activations by your other items, apply curse of stun to the enemy

### Fumbler's Mold — Greaves mold
  EVERY TIME IT FIRES (triggered): apply curse of misfire to the enemy

### Loose-Sole Mold — Greaves mold
  TRIGGERS: every 7 activations by your other items, apply curse of misfire to the enemy

### Stutterstep Mold — Greaves mold
  TRIGGERS: at the start of the fight, apply curse of misfire to the enemy

### Cadence Mold — Greaves mold
  TRIGGERS: every 4 activations by items sharing its rows, cut 0.8s off its own cooldown

### Answering Ring — Gloves ring
  PASSIVE: +1 str
  TRIGGERS: when a touching item activates, deal 15 physical damage to the enemy

### Mirrorplate Ring — Gloves ring
  TRIGGERS: when a touching item activates, deal 26 magic damage to the enemy

### Chainlink Mold — Gloves mold
  PASSIVE: +2 str
  TRIGGERS: when a touching item activates, deal 25 physical damage to the enemy

### Storm Signet — Gloves ring
  TRIGGERS: on activation, per adjacent assembled item, deal 18 magic damage to the enemy

### Siphon Ring — Gloves ring
  DAMAGE: +4 magic dmg
  TRIGGERS: when a touching item activates, drain 4 mana from the enemy

### Flaying Mold — Gloves mold
  DAMAGE: +5 magic dmg
  TRIGGERS: when an item in another slot on the same rows activates, drain all of the enemy's mana and deal 2 magic for each point

### Throttling Mold — Gloves mold
  PASSIVE: +3 str
  TRIGGERS: on activation, spend 9 mana: if it works, stun the strongest item the enemy has; if not, drain all of the enemy's faith and deal 3 magic for each point

### Keystone Base — Chestpiece enchantment
  PASSIVE: +10 hp

### Chalked Circle — Weapon enchantment
  PASSIVE: +0.15x its own power
  EVERY TIME IT FIRES (triggered): deal 22 magic damage to the enemy

### Open Palm — Gloves enchantment
  PASSIVE: +8% curse res
  TRIGGERS: when a touching item activates, deal 14 physical damage to the enemy

### Sprung Board — Greaves enchantment
  PASSIVE: +12% curse res
  EVERY TIME IT FIRES (triggered): cut 0.3s off its own cooldown

### Quiet Room — Helmet enchantment
  EVERY TIME IT FIRES: +8 mana

### Wayfarer's Orb — Weapon crystal ball
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: every 1 activation by another of your items, gain 3 mana (once a fight)

### Pilgrim's Orb — Weapon crystal ball
  DAMAGE: +5 magic dmg
  EVERY TIME IT FIRES: +2 mana

### Ferry Orb — Weapon crystal ball
  DAMAGE: +5 magic dmg
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, cut 1.0s off its own cooldown

### Stray Orb — Weapon crystal ball
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES: +2 mana

### The Cracked Lens — Weapon accessory
  DAMAGE: +12 mind

### The Stranger's Parcel — Weapon quest

### An Unwound Mainspring — Weapon quest

### The Tally — Weapon accessory

### The Odometer — Weapon accessory

### The Ledger — Weapon accessory

### the Second Key — Weapon accessory

### the Appeal — Weapon accessory

### the Skip Stone — Weapon accessory

### Bearhide — Chestpiece base
  PASSIVE: +260 hp, +12 str
  EVERY TIME IT FIRES: +8 armor
  EVERY TIME IT FIRES (triggered): gain 6 armor

### the Lightning Rod — Chestpiece enchantment
  PASSIVE: +6% curse res

### Thin Veil — Helmet frame
  PASSIVE: +6% mind res
  EVERY TIME IT FIRES (triggered): gain 2 insight

### Doorward Frame — Helmet frame
  PASSIVE: +12% mind res
  EVERY TIME IT FIRES: +2 mana
  EVERY TIME IT FIRES (triggered): gain 1 insight

### Sightless Crown — Helmet frame
  PASSIVE: +18% mind res
  TRIGGERS: on activation, spend 4 mana: if it works, gain 4 insight; if not, gain 2 mana

### Listening Frame — Helmet frame
  PASSIVE: +8% mind res
  EVERY TIME IT FIRES: +1 mana
  TRIGGERS: every 6 activations by your other items, gain 1 dread

### Antechamber Crown — Helmet frame
  DAMAGE: +4 mind
  PASSIVE: +10% mind res
  EVERY TIME IT FIRES (triggered): gain 2 insight

### Foreboding Crest — Helmet crest
  PASSIVE: +4% mind res
  EVERY TIME IT FIRES (triggered): gain 1 dread

### Second Sight — Helmet crest
  PASSIVE: +6% mind res
  TRIGGERS: on activation, spend 4 mana: if it works, gain 1 dread; if not, gain 2 insight

### The Quiet Ear — Helmet crest
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: every 3 activations by items sharing its rows, gain 2 insight

### The Eyeless Stare — Helmet crest
  PASSIVE: +5% mind res
  EVERY TIME IT FIRES (triggered): deal 6 mind damage to the enemy; gain 1 insight

### Doorway Primer — Weapon book
  TRIGGERS: on activation, spend 3 mana: if it works, gain 3 insight; if not, gain 1 mana

### A Word About the Wrong Stars — Helmet quest

### A Word About the Cellar — Helmet quest

### A Word About the Glow — Helmet quest

### A Word About the Thirsty Wizard — Helmet quest

### A Word About the Picket — Helmet quest

### A Word About the Exhibition — Helmet quest

### Ballast Bed — Chestpiece enchantment
  EVERY TIME IT FIRES: +8 armor
  EVERY TIME IT FIRES (triggered): turn up to 30 armour into 30 maximum health, for the rest of the fight

### Points Rodding — Greaves enchantment
  PASSIVE: +10% curse res
  EVERY TIME IT FIRES (triggered): hand 0.4s of this item's next cooldown to its slowest neighbour

### Booking Hall — Helmet enchantment
  EVERY TIME IT FIRES: +4 mana
  EVERY TIME IT FIRES (triggered): gain 10% of the mana you are holding

### Signal Wire — Gloves enchantment
  PASSIVE: +6% curse res
  TRIGGERS: when a touching item activates, if the enemy's best item is within 1.0s of firing, set it back 0.6s

### Shunter's Orb — Weapon crystal ball
  DAMAGE: +5 magic dmg
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, hand 0.5s of this item's next cooldown to its slowest neighbour

### Signalman's Orb — Weapon crystal ball
  DAMAGE: +4 magic dmg
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: when another spell in this item is cast, if the enemy's best item is within 1.0s of firing, set it back 0.4s

### A Word About the Sidings — Helmet quest

### A Word About the Points — Helmet quest

### Trig Pillar — Greaves enchantment
  PASSIVE: +3% curse res
  EVERY TIME IT FIRES: +5 armor

### Drove Way — Gloves enchantment
  PASSIVE: +6 str, +9% curse res
  TRIGGERS: when a touching item activates, gain 3 armor

### The Common Ground — Chestpiece enchantment
  PASSIVE: +26 hp

### Surveyor's Orb — Weapon crystal ball
  DAMAGE: +6 magic dmg
  EVERY TIME IT FIRES: +3 mana
  TRIGGERS: on activation, spend 3 mana: if it works, gain 1 spell forking; if not, gain 2 mana

### Drover's Orb — Weapon crystal ball
  DAMAGE: +7 magic dmg
  EVERY TIME IT FIRES: +2 mana
  TRIGGERS: when another spell in this item is cast, hand 0.4s of this item's next cooldown to its slowest neighbour

### A Word About the Hundred — Helmet quest

### Listener's Frame — Helmet frame
  DAMAGE: +9 mind
  PASSIVE: +60 hp
  EVERY TIME IT FIRES (triggered): gain 2 insight

### Countingstair Plating — Helmet plating
  PASSIVE: +6% curse res
  EVERY TIME IT FIRES: +18 armor

### Four Hundred and Second Step — Helmet crest
  PASSIVE: +12% mind res
  EVERY TIME IT FIRES (triggered): gain 3 insight

### Watcher's Crest — Helmet crest
  DAMAGE: +7 mind
  EVERY TIME IT FIRES (triggered): gain 2 dread

### The Wrong Sense — Helmet crest
  DAMAGE: +12 mind
  EVERY TIME IT FIRES (triggered): gain 3 insight
