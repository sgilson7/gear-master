---
name: gearmaster-gear
description: Author new gear for Gear Master and give it its Turtle Dick name in the same change. Use whenever adding, renaming, or rebalancing a PieceDef in crates/engine/src/piece.rs, or when a component exists in the base game but not in the themed catalogue. Covers which slot may carry which mechanic under the gear-slot basis rewrite, the PieceDef fields, the traps that fail silently, the theme table, and the tests that hold both halves together.
---

# Adding gear to Gear Master

A piece of gear is **two pieces of work**: the component, and the name it wears
in the *Tales from the Crypt* theme. Ship them together. A component with no
themed name is not broken — the theme falls through to the canonical name — which
is exactly why it is easy to forget and hard to notice. One untranslated word in
a shop full of Fnorp and Sneel is the kind of thing nobody reports and everybody
sees.

The rule this skill exists to enforce: **`piece.rs` and the theme's `pieces`
table change in the same commit, or the change is not finished.**

## The two files

| | |
|---|---|
| `crates/engine/src/piece.rs` | `CATALOG` — the component itself |
| `crates/engine/src/theme.rs` | `TURTLE_DICK.pieces` — `("canonical", "themed")` |

Nothing else needs to know. Names are **keys**, not labels: recipes, monster
loadouts, quest targets and most of the test suite are string-keyed on the
canonical name, and the theme is a lookup applied at draw time.

---

## Part 0 — which slot, and what it is allowed to do

**Read this before writing a `PieceDef`.** Since the gear-slot rewrite each slot
has a *basis vector*, and `crates/engine/tests/catalog_shape.rs` enforces it. A
piece written without this in mind fails the ratchet the moment you run the
suite, and the failure names a rule four hundred lines from your piece.

| Slot | Basis | Answers |
|---|---|---|
| **Weapon** | Conversion | turns time and banked pools into typed damage |
| **Gloves** | Reaction | acts when *other things* act |
| **Greaves** | Tempo | who moves, how often, and first |
| **Chest** | Reserve | how long you last |
| **Helmet** | Economy | income, what the pools are *for*, the mind |

### The exclusivity table

"Exclusive" means only that slot's pieces may carry it at all. "Majority" means
at least 70% of the catalogue's instances live there — **the minority is legal
and load bearing**: taking a helmet's only misfire away failed
`the_time_curses_are_reachable_from_every_slot`, which says every slot must be
able to land each time curse.

| Mechanic | Home | Level |
|---|---|---|
| `power_bonus`, Ink/Spell/Alignment/Book/Orb kinds, `GainForking`, `OnOtherCast`, `PerAdjacentEmpty` | Weapon | exclusive |
| searing | Weapon | majority (greaves share it) |
| `Consume`, `GainEmpowerment`, `GainShield`, `MindDamage`, `mind_resist` | Helmet | exclusive |
| `Grow`, `physical_harden`/`magic_harden`, `reflect` | Chest | exclusive |
| health above 15 per piece | Chest | majority |
| `OnAdjacentActivate`, `PerAdjacentItem`, `Drain`, `StunStrongest`, `DoubleAdjacentItemStat` | Gloves | exclusive |
| `OnAlignedActivate` | Gloves | majority |
| `OnBattleStart`, `speed_bonus` outside the weapon | Greaves | exclusive |
| `ReduceCooldown` outside the weapon | Greaves | exclusive, shared with Gloves |
| frost / stun / misfire | Greaves | majority |
| terrain (`PieceKind::Terrain`) | Chest and Greaves | exclusive |

Positional *stat* effects — `Adjacency`, `DoubleNeighbor`, `SoleIf`,
`SelfPerNeighborKind`, `SelfPerEmptyCell`, `Flat` — are pan-slot texture and
belong to nobody. Reach for those when a piece needs an interaction and its
slot's own verbs do not fit.

### Materials and Platings may carry no identity mechanic at all

`PieceDef::fits` lets a **Material** sit in gloves or greaves and a **Plating**
in helmet or greaves, so a rule keyed on `def.slot` cannot promise where they
end up. They are the deliberate bleed carriers: stats, adjacency, and nothing
from the table above. Three separate pieces in the sweep were caught carrying
two or three at once.

### Quotas, if you are filling a gap

Per non-weapon slot: **≥60%** of its pieces express its own axis, **20-25%**
express its bleed axis, **≤30%** are plain filler (15% after the rewrite), and
**≥35% of the dearest third** carry an interaction. Every epic-or-better
non-weapon piece must carry one. Pool-spend texture (`SpendMana`/`Spend`/
`Consume`) is capped at 15% per slot outside the helmet.

The bleed cycle is **W→G→Gr→C→H→W**: a piece expressing its slot's *next*
neighbour's axis is filling the bleed quota, not breaking a rule.

### Two habits the sweep paid for

**Translate the verb, keep the sentence.** A piece has a shape as well as a
mechanic — a cost, a cooldown, a condition — and the shape is usually load
bearing even when the mechanic is in the wrong slot. Warded Sabatons bought a
mana shield for three mana; rewriting it as an *unconditional* cooldown
reduction was the right verb with the gate dropped, and every creature wearing
the piece got it free. A board that had cleared to rung 22 on the hardest
setting stopped at 20.

**Do not empty a mechanic of carriers.** Taking the last `DoubleAdjacentItemStat`
off a weapon left the rule naming something the catalogue no longer contained,
which is a rule that can never fail again. If a mechanic's only carrier is in
the wrong slot, the move is to *author one in the right slot*, not to delete it.
`every_rule_names_a_mechanic_that_exists` will tell you.

### The ratchet, and how to leave it

`catalog_shape.rs` carries a `budget` (today's distance) and a `target` (zero)
for every rule. `the_catalog_stays_within_its_budgets` fails if you make
something worse; `no_budget_is_slack` fails if you make it *better* and do not
record it. So **every catalogue commit is a two-file commit** — the piece and
the budget it moved. Lower a budget in the commit that earns it; never raise
one.

Run `cargo test -p gearmaster-engine --test catalog_shape -- --ignored` to see
the whole distance, and `report_shape` for the per-slot tables.

---

## Part 1 — the component

### The shape of one

```rust
PieceDef {
    name: "Ash Haft",              // unique across CATALOG - enforced
    slot: SlotKind::Weapon,
    kind: PieceKind::Handle,
    cells: &[(0, 0), (0, 1), (0, 2)],
    base: Stats { strength: 4, ..Stats::ZERO },
    adjacency: None,               // flat bonus, only once the item assembles
    effect: None,                  // positional; reads/writes neighbouring cells
    cooldown_ms: 2300,             // meaningful on a core; 0 = use the slot default
    speed_bonus: 0,                // percent, summed across the item
    triggers: &[],                 // fire each time the item activates
    quest: None,
    power_bonus: 0,                // hundredths of weapon power, this item only; ink
    price: 14,                     // vestigial - see below
}
```

### Six things that fail quietly

1. **Append inside `CATALOG`, not after it.** Anchoring on the last `];` in the
   file finds `BOSS_ONLY`'s terminator, not the catalogue's. Insert before the
   `];` that precedes the `/// Gear that exists only on a boss.` comment.

2. **`price` is vestigial.** `shop_price()` derives cost from the rating. Set it
   to something sane for readability, but changing it does nothing. To make a
   piece expensive, make it *strong* — the curve will price it.

3. **Labels quote their own numbers.** `adjacency.label` and `effect.label` are
   prose: `"Runed: +15 health"`. Change the stat and the label lies. Scaling all
   health fivefold once left eleven pieces saying one thing and doing another.
   No test can catch this; grep the label for digits when you touch a stat.

4. **A strong piece raises its slot's ceiling.** `slot_ceiling` is the best
   possible item in a slot and every rating is a fraction of it, so one outlier
   deflates the rarity mark *and price* of everything else in that slot. If the
   piece is meant to be off the scale it belongs in `BOSS_ONLY`, which is
   exempt from the ceiling, the shop and the absurdity check.

5. **Renaming an existing piece breaks monster loadouts.** `LADDER` gear lists
   reference pieces by string. `every_monster_actually_assembles_its_gear`
   catches it — that is its job — but only if you run it.

6. **Cells may be hollow, but know what you are doing.** A piece is atomic:
   `Slot::groups` reaches every cell of a piece from any one of them, so a ring
   shape like the Hollow Sphere stays one item. What a hole *does* change is
   packing — the Hollow Sphere's centre is where a spell goes. Use a hole
   deliberately or not at all.

### Footprint families are a feature

`stepped_component` swaps a monster's gear for another piece of the **same kind
and the same footprint**, one rung up or down. Reusing a shape across a power
range is what gives difficulty stepping somewhere to step. A shape used once is
a dead end for the whole system.

### Tests that will tell you

`cargo test -p gearmaster-engine`. Most of these are unit tests inside `src/`,
not files under `tests/` — grep the whole crate if you go looking for one.

- `no_two_components_share_a_name` — every lookup is `find(|d| d.name == n)`,
  so a repeated name makes the second definition unreachable while the shop
  still stocks both
- `no_piece_is_larger_than_a_slot` — at every rotation
- `a_hollow_piece_placed_alone_is_still_one_item`
- `every_component_has_a_rating_and_none_of_them_is_absurd` — rating in `-40..=200`
- `a_slots_ceiling_is_full_marks` — every slot tops out in the same place
- `every_slot_can_reach_every_tier`
- `every_monster_actually_assembles_its_gear`
- `boss_gear_does_not_move_the_scale_for_anything_else`
- `boss_gear_never_appears_in_the_shop`, `quest_rewards_never_appear_in_the_shop`

---

## Part 2 — the Turtle Dick name

**Read `reference/turtle-dick.md`** before naming anything. It has the source
material with page numbers, the substance ladders, the content charter, and the
method. The three things you cannot skip:

**Grade must survive the rename.** The theme spends substances as ranks, so a
player can still sort a shelf by reading the first word:

- Defence: Cork → Vinyl → Sneel → Time-Tempered → **Ypytryktrium**
- Speed: Slow Trundler → Fast Roller → **Thrumbus**
- Sacred: Francian → **Wimpler**

A new bottom-tier plating is a Cork something; a new top-tier one is
Ypytryktrium. Do not invent a sixth metal.

**Every name comes from the book,** with a page you could cite. When nothing
fits, write a plain descriptive name in the book's register rather than
fabricating a proper noun.

**The charter:** crude character names are allowed - the owner asked for Big
Yomp and PoopFart by name. Still out: sexual or anatomical content, drugs,
alcohol and smoking, slur-adjacent coinages, and every real public figure.
Violence stays cartoon-grade. The build is publicly hosted.

Nine canonical names stay in English on purpose; they are listed with their
reasons in `the_turtle_theme_covers_the_catalogue`.

### Tests that will tell you

- `every_themed_piece_names_a_real_one` — a typo in a canonical key is a piece
  that quietly keeps its plain name; among 370 nobody would spot it
- `no_two_components_get_the_same_new_name`
- `the_turtle_theme_covers_the_catalogue` — the exempt list lives here

---

## The workflow

Run `scripts/check-parity.sh` at the start and the end. It lists every catalogue
entry with no themed name and every themed key pointing at a component that no
longer exists, so "am I done" is a question with an answer.

0. Decide the slot from Part 0, and check the mechanic you have in mind is
   allowed there. If it is not, the question is what this piece does *in its
   own slot's vocabulary* - that is the sweep's whole method and it is almost
   always a better piece than the one you started with.
1. Write the `PieceDef`. Reuse a footprint if the piece belongs to a family.
2. `cargo test -p gearmaster-engine` — before naming anything. A rating outside
   the band or a shifted ceiling is easier to fix now.
3. Pick the themed name from the book; place it on the right rung of its
   substance ladder. Cite the page in your message, not in the code.
4. Add `("Canonical Name", "Themed Name")` to `TURTLE_DICK.pieces`, sorted.
   The table opens with a bare `    pieces: &[` — `PLAIN`'s is `pieces: &[],`
   and matching on the prefix will drop your entry into the story array.
5. `cargo test` — the whole suite, and `--test catalog_shape -- --ignored` for
   the distance left. If a budget moved, lower it in this commit.
6. `scripts/check-parity.sh` — expect "In parity."
7. Look at it. `GEARMASTER_THEME=td GEARMASTER_SKIP_INTRO=1` with
   `GEARMASTER_SHOT`: a name that satisfies every test can still be two words
   too long for a shop card.

## When the piece is a rename, not an addition

Update every `LADDER` gear list that references the old name **and** the theme's
key. The assembly test catches the first; `every_themed_piece_names_a_real_one`
catches the second. Both only run if you run them.
