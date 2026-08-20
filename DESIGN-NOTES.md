# Gear Master — design notes

Working notes for the prototype. Two sections: what Backpack Battles actually
does (researched, with sources), and how Gear Master diverges.

---

## Part 1 — Backpack Battles, as researched

Sources: [official wiki](https://backpackbattles.wiki.gg/wiki/Game_Mechanics),
[cooldown page](https://backpackbattles.wiki.gg/wiki/Cooldown),
[demo mechanics](https://backpackbattles.wiki.gg/wiki/Game_Mechanics_(demo)),
[class overview](https://www.gamepressure.com/newsroom/classes-in-backpack-battles-bpb-explained/z56a77),
[recipe rules](https://www.thegamer.com/backpack-battles-item-combination-guide/).
Numbers vary by patch — treat them as proportions to aim at, not gospel.

### The loop

Round = **shop phase** (all the decisions) → **battle phase** (resolves
itself, you watch). Repeat. A run ends at **10 wins or 5 losses**, up to 18
rounds. You never fight live: your bag is matched against another player's
**recorded loadout** at a similar rating. Ranks: Bronze → Silver → Gold →
Platinum → Diamond → Master → Grandmaster → Grandma, tracked per character.

The whole game is therefore an **offline optimization puzzle with a stochastic
grader**. Nothing in the battle is interactive — which is exactly what makes
the placement decisions carry all the weight.

### Shop and economy

- Start: **13 gold, 25 health**. Both scale by round — round 8 gives ~22 gold
  and 115 health; by round 18 health is ~350.
- Extra gold per **Piggybank** held.
- Every item has a **10% chance to appear at 50% off**.
- **Reroll costs 1 gold**, then 2 for subsequent rolls in the same shop.
- Rarity by round: round 1 is 90% Common / 10% Rare. Legendary and Godly
  start appearing at round 4. Rounds 12–18 flatten to ~20% each of Common /
  Rare / Epic / Legendary / Godly.

### The grid

- Backpack starts at **12–14 usable tiles out of a possible 63**.
- Tiles are unlocked by *buying bag items* — Leather Bag, Fanny Pack, Stamina
  Sack, Potion Belt, Protective Purse. Inventory space is itself a purchase
  competing with weapons for gold.
- Items are **polyominoes**; placement is the core puzzle.
- Many items buff *neighbors* — Food, Steel Goobert, Flute. Some bags grant
  their contents a bonus (Fanny Pack: +10% speed).

### Crafting

Recipe ingredients placed **orthogonally touching** merge into the crafted
item — but **not immediately: after the next battle**. Holding an item draws
blue lines to anything it can combine with. Items can be **locked against
combining** (right-click). Some recipes use a **catalyst** that survives while
the other ingredients are consumed.

The delayed merge is a real design choice: you commit to a craft, then fight
the round with the ingredients still in their un-merged state.

### Combat

Continuous-time, fully automatic. Each item has a **cooldown in seconds per
activation**; it fills a highlight bar and fires when full, then resets.
One-shot items (most gemstones, playing cards, Vampiric Gloves) fire once and
do not reset — but their cooldown can still be modified.

**Effective cooldown** (all modifiers additive, capped ±1000%):

```
speedup > slowdown:   cd = base / (1 + faster - slower)
slowdown > speedup:   cd = base × (1 + slower - faster)
```

So a 4s base with +100% speed fires every 2s. Note the asymmetry — the two
branches are not the same function, which keeps slows from going degenerate.

Stats: Health, Block (absorbs 1 damage per stack), Accuracy (hit/miss), Crit
(double damage), attack damage, Mana, Stamina.

- **Buffs**: Empower (+1 weapon damage), Heat (+2% trigger speed/stack), Luck
  (+5% accuracy), Regeneration (+1 HP / 2s), Spikes, Vampirism.
- **Debuffs**: Blind (−5% accuracy), Cold (−2% trigger speed/stack), Poison
  (1 damage / 2s).
- **Status**: Stun (pauses *all* cooldowns), Invulnerability, Reflect, Resist,
  Reincarnate, Nullify, Fatigue, Battle Rage.
- **Mana** fuels magic items; some items trigger on mana *generated* rather
  than consumed (Mana Thirst Blade).

### Classes

Four, differing mainly in their unique item pools:

| Class | Identity |
|---|---|
| **Ranger** | crits, bows, nature pets; clover-stacking builds |
| **Reaper** | damage-over-time, poison, potions, playing cards, scythes |
| **Berserker** | heavy armor, huge weapons, armored pets, one Super Mode per fight |
| **Pyromancer** | fire/ice; generates Flames each round to speed up all equipment |

---

## Part 2 — Gear Master's divergence

> **Status:** a playable prototype of this now exists — see `README.md`.
> Slot sizes, recipes, adjacency bonuses and combat are settled and tested;
> the shop, rarity, rounds and crafting from Part 1 are not built.

**The change:** replace one free-form backpack grid with **5 equipment slots,
each its own grid of a different size.** Diablo-style: helm, chest, weapon,
gloves, boots (working names).

### What that changes about the puzzle

Backpack Battles gives you one large shared space where every item competes
with every other item and adjacency is unconstrained. Gear Master gives you
five small spaces with hard walls between them. Consequences worth designing
around:

- **Adjacency becomes scarce and local.** A slot only has so many neighbor
  pairs. Synergy items get more valuable and more positional.
- **Slot size becomes an upgrade axis** in place of BB's "buy more bag." A
  bigger chest grid is a different purchase from a bigger boot grid, and each
  changes which shapes you can accept.
- **Slot type restrictions** give the shop a second dimension — an item isn't
  just "does it fit," it's "does it fit *here*."
- **Cross-slot synergy is now a deliberate mechanic** rather than a default.
  Whether two items in different slots can interact at all is the single
  biggest design lever. Making it rare and expensive is what would keep the
  five grids feeling like five puzzles instead of one disconnected one.

### Open questions to settle before/while prototyping

1. **Slot sizes.** Concrete grids per slot — e.g. helm 2×2, chest 4×4,
   weapon 2×4, gloves 2×2, boots 3×2? Total tiles should land near BB's
   early-game 12–14 so the early rounds feel similarly tight.
2. **Cross-slot adjacency:** none, adjacent-slots-only, or via specific
   "linking" items?
3. **Slot upgrades:** does a slot grid grow, or do you replace the base gear
   piece with a larger one (which is more Diablo-ish and gives the shop
   better items to sell)?
4. **Crafting across slots?** BB requires touching. If slots are walled, does
   crafting work within a slot only?
5. **Classes at all** for the prototype, or one kit first?

### Prototype scope suggestion

Slice 1 is not the whole game. Aim for: five sized slots, a handful of
polyomino items, drag-and-drop placement with engine-side fit validation, and
a deterministic combat sim over a fixed timestep producing a replayable log.
Shop, economy, rarity and crafting come after placement feels good.

Build it with the `rust-game-prototype` skill in `.claude/skills/` — it has
the workspace scaffold, the engine/GUI split, and the drag-and-drop pattern
worked out.
