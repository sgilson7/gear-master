---
name: gearmaster-gear
description: Author new gear for Gear Master and give it its Turtle Dick name in the same change. Use whenever adding, renaming, or rebalancing a PieceDef in crates/engine/src/piece.rs, or when a component exists in the base game but not in the themed catalogue. Covers the PieceDef fields, the traps that fail silently, the theme table, and the tests that hold both halves together.
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

1. Write the `PieceDef`. Reuse a footprint if the piece belongs to a family.
2. `cargo test -p gearmaster-engine` — before naming anything. A rating outside
   the band or a shifted ceiling is easier to fix now.
3. Pick the themed name from the book; place it on the right rung of its
   substance ladder. Cite the page in your message, not in the code.
4. Add `("Canonical Name", "Themed Name")` to `TURTLE_DICK.pieces`, sorted.
   The table opens with a bare `    pieces: &[` — `PLAIN`'s is `pieces: &[],`
   and matching on the prefix will drop your entry into the story array.
5. `cargo test` — the whole suite.
6. `scripts/check-parity.sh` — expect "In parity."
7. Look at it. `GEARMASTER_THEME=td GEARMASTER_SKIP_INTRO=1` with
   `GEARMASTER_SHOT`: a name that satisfies every test can still be two words
   too long for a shop card.

## When the piece is a rename, not an addition

Update every `LADDER` gear list that references the old name **and** the theme's
key. The assembly test catches the first; `every_themed_piece_names_a_real_one`
catches the second. Both only run if you run them.
