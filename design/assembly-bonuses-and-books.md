# Assembly bonuses, and the book

Written against commit `1634614` (2026-08-27), and **re-verified against it**:
this document was first drafted while the session's view of the repo was a day
stale, and the Switchyard, the validity solver and the road past Francis had
all landed in between. Every finding below was re-read on the current tip and
all three still hold. Every figure was read off it. Code follows this document; where it records what the code *does
today*, it was read and cited, and there the code is the news.

Four pieces of work in one milestone chain. Two of them move balance - the
bonuses and the book - and neither should move it twice, which is what decides
the order. The other two are about the game explaining itself: the pools want
their symbols wherever a number is printed, and the glossary wants to show a
mechanic rather than describe it.

---

## 1. What is wrong

### 1.1 "Adjacency" is not adjacency

`piece.rs:303` declares `Adjacency { label, stats }` and `loadout.rs:534`
applies it:

```rust
if assembled[gi] {
    for &p in group {
        if let Some(adj) = reg.def(p).adjacency {
            item_stats += adj.stats.scaled(100 + self.adjacency_pct);
```

The only condition is **did this group satisfy a recipe**. There is no
neighbour test anywhere on that path. The struct's own doc says where the name
came from - *"Gear Master's version of a Backpack Battles adjacency bonus"* -
and in that game the bonus really is adjacency-based. Here the trigger was
changed to assembly and the name was not.

It collides with five places that mean *touching*: `Trigger::OnAdjacentActivate`,
`RunningItem::adjacent_items`, `aligned_items`, `diagonal_items`, and
`Slot::sets_touch_diagonally`. The interface mostly gets it right - the GUI
prints "when assembled:" and the CLI "on assembly:" - but the word escapes in
the one place least able to survive it, **Recycler**, whose power text
(`class.rs:562`, `:621`, and again hardcoded in `main.rs:4077`) says
*"+10% adjacency bonuses"*. A player reading that will go and try to make
their pieces touch.

### 1.2 The numbers are not on the screen

There are **37** assembly bonuses. (`piece.rs:302` said *"Exactly one piece per
slot carries one"*, which was true once and is seven times wrong now - the
first thing this milestone corrects.) They were written by two different hands:

**29 are specifications** - a name, a colon, and the figure. `Timeworn: +0.30x
weapon power`, `Stonewall: +25% physical resistance`, `Breaker: +6 strength`,
`Heartwood: +4 regen a second`.

**8 are atmosphere with no number in them**, and every one of them is greaves:

| Piece | Label | What it actually is |
|---|---|---|
| Pilgrim Sole | `the road knows you` | curse_resist 4, faith 1 |
| Worldstrider Sole | `one stride ahead` | curse_resist 5 |
| Deeprooted Sole | `planted` | curse_resist 10 |
| Ridge Runner | `downhill all the way` | curse_resist 4, strength 2 |
| Rimebound Mold | `the cold gets into the works` | curse_resist 6 |
| Coldstep Mold | `sure-footed on ice` | curse_resist 8 |
| Ambush Mold | `already moving` | strength 4 |
| Deadfall Mold | `set before they arrive` | armor 6 |

Both kinds go through one renderer (`main.rs:4182`) which prints
`"when assembled: {label}"` verbatim - so a Deeprooted Sole card reads **"when
assembled: planted"**, which is worth +10 curse resist and says so nowhere. The
figure is printed on no card, no tooltip and no CLI line; the only trace is the
keyword rail, which shows a curse-resist glyph and no quantity.

**Six of the eight are curse resist**, and greaves carries five *more* curse
resist bonuses that do state their numbers (`Reliquary: +12`, `Overflow: +10`).
So the eight are not merely vague - they are eight names for a bonus the same
slot already sells plainly twice over. That is the real fault.

### 1.3 The fusion pools are unreachable

`Resource` has eight members. Three are fusions - **DruidicMight**
(nature+rage), **Communion** (faith+nature), **Zealotry** (rage+faith) - and
`Combatant::held_bonus` (`combat.rs:3217`) pays each at **double both parents'
rates**, uncapped: a point of Zealotry is +2 physical damage *and* +4 to both
resistances.

`Action::Fuse` exists. `Resource::parents()` exists. **No entry in the
504-piece `CATALOG` uses either.** Every mention of a fusion in `piece.rs` is
in the enum and its impl blocks; there are none past `:1202`, where the
catalogue starts.

This is the same shape as `cursed_for_good` before the Unwinding found it, and
as the three counters `completable.rs` now records: machinery that works,
costs nothing to reach, and is reached by nothing.

### 1.4 The pools have symbols and the numbers do not use them

`draw_keyword` (`main.rs:1746`) already draws eleven glyphs, five of them
pools: **mana, rage, faith, nature, armor**, plus curse, speed, mind, magic,
physical and quest. They are used in exactly one place - the keyword rail down
the left edge of a card, which says *that* a piece touches nature and never how
much.

Every actual figure is text. `Stats::summary` (`stats.rs:324`) builds
`"+1 nature"`, `"+3 str"`, `"+0.20x its own power"`, and every card, tooltip
and log line prints that string. So a card shows a nature glyph in the rail
**and** the words "+1 nature" in the body, and the two never meet.

Four pools have no glyph at all: **Insight**, and the three fusions. If §1.3's
fusions become reachable they will arrive with nothing to draw them with.

### 1.5 The glossary describes what it could show

`render_glossary` (`main.rs:8874`) is three tabs of prose - WORDS, CLASSES,
WHAT DECIDES - and `GLOSSARY` is 40-odd `(term, definition)` pairs of text.
FAITH's entry reads *"Banked slowly. Every point adds resistance of both types
while held, up to 40%."* That is a sentence about an arrow.

The precedent for doing better is already in the file: `draw_tile_legend`
(`main.rs:8112`) draws the five slot motifs, in colour, at their real sizes,
under the heading READING A TILE. It is the one part of the glossary that shows
instead of telling, and it is the model.

### 1.6 The book is the worse ball

```
Book:  Book 1 + Ink 1 + Spell 1 + Accessory 0-1     (piece.rs:1044)
Orb:   Orb 1  + Spell 2-3 + Alignment 0-1
```

The book spends a whole mandatory piece - inks run 4 to 5 cells - on a flat
multiplier for **one** spell. The orb spends nothing on a core tax and gets
two or three payloads.

| Build | Cells | Casts |
|---|---|---|
| Grand Grimoire + Kingsblood Ink + Kingsbane | 6+5+4 = **15** | **1** |
| Worldeye Orb + Kingsbane + Cometfall | 6+4+4 = **14** | **2** |

Fewer cells, more payloads, and an alignment that colours all of them. There is
no board on which the book is the right answer.

**The machinery is already general.** `loadout.rs:819` filters
`PieceKind::Alignment` out of `item.pieces` and folds its stats and triggers
into **every** `Cast`; it never asks whether the core is an orb. `casts` is
built from every `Spell` in the item. `power_bonus` is summed over every piece,
so zero inks is no bonus and two is double. Cast cycling (`combat.rs:4509`) is
`cast_index % casts.len()` and is not orb-gated either.

**So the book is weak because of the recipe table and nothing else.**

---

## 2. What it becomes

### 2.1 The type

```rust
pub struct AssemblyBonus {
    pub label: &'static str,
    pub stats: Stats,
    /// Triggers that exist only while the item is assembled.
    pub triggers: &'static [Trigger],
}
```

One field added. It reuses the entire `Trigger`/`Action` vocabulary, so most of
§2.3 costs no new combat code at all.

### 2.2 The book

```
Book:  Book 1 + Spell 1-2 + Ink 0-2 + Alignment 0-1
Orb:   Orb 1  + Spell 2-3 + Alignment 0-1            (unchanged)
```

Every bound is **relaxed**, never tightened, which is why this cannot break an
existing board: anything that assembled before still assembles. Four creature
boards wear book cores - Chained Codex, Leaden Tome, Apprentice's Primer, Grand
Grimoire - and all four keep working.

The identities separate properly:

- **The book is the focused caster.** One or two spells, and up to two inks
  multiplying them. Ink stacking is the book's whole argument and it is what
  makes a single big spell worth building around.
- **The orb is the broad one.** Two or three spells, no ink, one alignment
  colouring all of them. Breadth, not depth.

They overlap at two spells and that is fine - at two spells the book is paying
cells for multipliers and the orb is paying them for a third payload.

### 2.3 The eight greaves bonuses

Greaves own **tempo and what can stop you**. Each design is built to its own
name; the cost column is honest about what exists.

| Label | Becomes | Cost |
|---|---|---|
| **the road knows you** | Start holding 6 faith; every 12 faith banked becomes 1 **Communion** | free - `OnBattleStart(Gain)` + `Consume{Faith, 12, Gain{Communion}}` |
| **planted** | It never moves and never fires: every 8 nature it holds becomes 1 **Communion** | free - `Consume{Nature, 8, ..}` |
| **the cold gets into the works** | Every second curse you land **derails** an enemy item 400 ms | free - `Watch{CurseApplied, 2, Derail, repeats}` |
| **set before they arrive** | At the bell, armour equal to 3x the empty cells touching it | small - the battle-start scan (`combat.rs:4016`) matches the top-level trigger, so it must unwrap `PerAdjacentEmpty` |
| **sure-footed on ice** | This item cannot misfire and cannot be stunned | small - `RunningItem::steady` exists; stun immunity is one new flag |
| **downhill all the way** | Starts at half cooldown and adds 200 ms every time it fires | **new** - persistent cadence drift |
| **already moving** | Every item on the board starts at 40% of its cooldown | **new** - board-wide priming. `ReduceCooldown` is deliberately clamped to `cooldown_ms - 1` so it *cannot* do this |
| **one stride ahead** | When an **enemy** item activates, push this one forward 150 ms | **new** - `Trigger::OnEnemyActivate`; every relational trigger today looks only at your own board |

The 29 specification labels keep their bonuses and lose the `Name: number`
half of their text once the renderer prints the stat block, or the card says
the number twice. `Stonewall`, `Breaker`, `Timeworn`.

Two of them are the natural homes for the remaining fusions: **Breaker**
(gloves, strength) becomes **Zealotry**, and **Heartwood** (chest, regen)
becomes **DruidicMight**.

---

## 3. Milestones

The two inert milestones come first on purpose. Both halves of this document
move balance, and the house rule from `design/HANDOFF.md` §5 is **land
primitives inert, arm them separately** - it is why every Phase-1 milestone of
the Unwinding shipped with the ladder byte-identical.

### M1 - The name, and the numbers. No rules change. **▲**

- `Adjacency` -> `AssemblyBonus`, `adjacency_pct` -> `assembly_pct`. Internal
  type; no string key, no `CATALOG` index, no save format.
- Recycler stops saying "adjacency" in all three places, including the
  hardcoded copy in `main.rs:4077`.
- The card, the tooltip and the CLI print the bonus's **stat block** beside its
  label, from `Stats::summary`, the way every other stat block is printed.
- The 29 specification labels lose their now-duplicated numbers.
- `piece.rs:302`'s "exactly one piece per slot carries one" is corrected to
  what the table actually holds.
- **Deliverable:** no assembly bonus can ship without its numbers on screen,
  because the numbers come from the stat block and not from whoever wrote the
  label.
- **Gate:** both suites green; the ladder byte-identical; a test that every
  bonus's rendered text contains its own figures.

### M2 - The bonus can act. Inert.

- `AssemblyBonus.triggers`, wired in `loadout.rs` so they are live only while
  assembled and dead the moment the item comes apart.
- Every existing bonus ships `triggers: &[]`.
- **Deliverable:** the field and its wiring, arming nothing.
- **Gate:** the ladder is **byte-identical** to M1's. If it is not, the wiring
  is wrong and nothing above this is meaningful.

### M3 - The book, rebuilt. **▲**

- The recipe of §2.2. Nothing else.
- Re-pin the five `assembly.rs` tests that encode the old rule - including
  `a_book_will_not_take_a_second_spell_but_an_orb_wants_one`, whose *name* is
  the old rule, and the recipe listing at `:1023` that reads
  `vec!["1 book", "1 ink", "1 spell"]`. Re-pinned with the reason in the
  assertion.
- **The risk to watch:** relaxing a recipe cannot stop a board assembling, but
  it can make a loose pile *start* assembling - which changes fights, and moves
  `progression` pins. Every moved pin is inspected rather than re-blessed.
- **Deliverable:** a book build that is worth packing, and a measured statement
  of what it is worth.
- **Gate:** both suites green; `baseline` and `catalog_shape` printers re-run
  and their diffs read; a book board fought against the ladder and its rungs
  cleared written into `analysis/`.

### M4 - The four free bonuses, and two fusions. **▲**

- `the road knows you`, `planted`, `the cold gets into the works`,
  `set before they arrive`.
- **Communion becomes reachable** - the first fusion pool any board can make.
- `Watched::CurseApplied` gets something to pay for.
- **Gate:** a test that fights a board holding each new bonus and asserts the
  pool actually arrives, because a bonus that grants a fusion nothing reads is
  the fault this whole document is about.

### M5 - Three new primitives. **▲**

- `sure-footed on ice` - one flag on `RunningItem`.
- `downhill all the way` - persistent cadence drift. Nothing in the game
  changes an item's cadence permanently; this is the dial.
- `already moving` - board-wide priming at the bell.
- `one stride ahead` - `Trigger::OnEnemyActivate`, the reactive relation the
  trigger list has never had.
- **Re-gearing:** these change what a greaves item is worth, so
  `stepped_component` re-gears every creature on Easy, Hard and Insane. Weights
  settle **before** anything is measured against them.
- **Gate:** each primitive has a test that fails without it; the ladder's
  movement is measured and written down rather than discovered later.

### M6 - The pools get their symbols. **▲**

Interface only; no rule moves.

- **Glyphs for the four pools that have none:** Insight, and DruidicMight,
  Communion and Zealotry. A fusion's glyph is built from its parents' - it is
  the one thing that makes `parents()` legible at a glance - so the shape says
  what the pool is made of before any text does.
- **`Stats::summary` gains a symbolic sibling.** The engine keeps returning
  text, because the CLI has no glyphs and the log is a string; the GUI gains a
  renderer that walks the same fields and draws `1` + the nature glyph where
  the string says `+1 nature`. One traversal, two outputs, so a stat cannot
  appear in one and not the other.
- **Everywhere a figure is printed**: the piece card, the item card, the
  tooltip, the shop shelf, and the assembly-bonus line M1 added.
- **Gate:** a test that every field `Stats::summary` can emit has a glyph or a
  deliberate fallback, so a new stat cannot ship invisible. The fallback is the
  word, never nothing.

### M7 - The visual battle glossary. **▲**

A fourth tab, **HOW A FIGHT WORKS**, built the way `draw_tile_legend` is built:
drawn, not written.

Each entry is a small diagram in one row - a subject, a relation, an outcome -
using the glyphs M6 finished:

- **devotion** -> arrow -> **physical resist** + **magic resist**
- **fury** -> arrow -> **physical damage**
- **harvest** -> arrow -> **regen**
- **jokes** -> spent -> **a cast**, with the weak branch drawn beside the paid one
- **nature + rage** -> **druidic might**, and its two siblings: the fusion
  triangle, which is the diagram `parents()` has always implied
- **cork** absorbs before **health**, and empties at the bell
- the **three lanes** and their three answers, side by side, since that is the
  one thing a player has to hold in their head to build anything
- **sudden death**: the clock at 30 s and the share both sides lose after it

**Why a tab and not a rewrite.** The existing WORDS tab is right for words - a
player looking up MISFIRE wants the sentence. This is for the half a sentence
cannot carry, which is *what turns into what*.

**Deliverable:** every relation in `Combatant::held_bonus` drawn, because that
function is the actual rulebook for what a banked pool is worth and it has
never been shown to anybody.

**Gate:** a layout test in the house style - every diagram lands inside its
row and none overlaps, at every width the panel can take, the same shape as
`the_log_keeps_its_column_at_every_width`.

### M8 - Zealotry, DruidicMight, and the record. **▲**

- Breaker -> Zealotry, Heartwood -> DruidicMight. All three fusions reachable,
  and M6's glyphs stop being art nothing produces.
- The printers, `analysis/`, the ledger, and `CLAUDE.md`'s counts and traps.
- **Gate:** a test asserting every `Resource` variant can be created by some
  board - the lint that would have caught this in the first place - and that
  every one of them can be drawn.

---

## 4. What could go wrong

1. **M2's ladder is not byte-identical.** Then the trigger wiring is firing
   when it should not, and every measurement above it is against the wrong
   baseline. Stop and fix before M3.
2. **M3 makes piles assemble that were never meant to.** Relaxing bounds is
   backward-compatible for assembly but not for *intent*: a player's loose
   spells may now bind into a weak item they did not ask for. Watch the preset
   and the three share codes specifically.
3. **The book overshoots.** Two inks on one spell is a large multiplier and the
   dial is the ink bound, not the spell count.
4. **M6's fusion glyphs land before M8 makes fusions reachable.** That is
   deliberate - a pool arriving on screen with nothing to draw it is how the
   first one would be found - but it means M6 ships four glyphs that nothing
   yet produces. They are drawn in the glossary from the first day so they are
   not dead art.
5. **M7 becomes a second rulebook.** A diagram that disagrees with
   `held_bonus` is worse than no diagram. Every relation drawn is read off that
   function, and the gate is that the numbers in it come from the engine rather
   than from the drawing code.
6. **M5's primitives are stronger than the ratings know.** `rating.rs` prices
   stats, and none of cadence drift, priming or enemy-reaction is a stat. If
   the ratings cannot see them, the shop misprices them and
   `stepped_component` puts them on creatures at the wrong rung. This is the
   milestone most likely to need a `rating.rs` weight, which is the one thing
   that must be settled before anything is authored against it.

### M9 - The pedestal gets a screen. **▲**

**The bug.** Clicking the pedestal in High Wick does nothing, for ever. Three
facts make the loop, and each one is correct on its own:

- `town::Action::Pedestal.costs_the_visit()` returns **false** (`town.rs:114`),
  deliberately - the pedestal "is not a door; it stands in the entryway and
  takes its own key", so it must not spend the town's one action.
- `run.rs:4160` answers `Action::Pedestal => {}`, also deliberately: the door
  is answered by `feed_pedestal` with an orb in hand, not by being walked
  through.
- **Nothing ever calls `feed_pedestal`.** Not the GUI, not the CLI. The only
  thing the GUI knows about `Action::Pedestal` is its blurb
  (`main.rs:7704`), which counts the orbs in the bag and says so.

So the click resolves to nothing, the visit is not spent, the town re-renders
unchanged, and the player clicks again. `feed_pedestal` is complete, tested,
guarded against duplicates and against a destination firing twice, and reached
by no interface - which is trap 30 exactly, one milestone after that trap was
written down.

**What it becomes.** A screen of its own, because feeding the pedestal is the
one action in the game that takes an *item* as its argument, and no existing
screen has anywhere to put one.

- **A slot you drag an orb into.** The pedestal is a thing you bring a key to,
  so the interaction is carrying rather than choosing from a list.
- **It reads the orb back.** Dropped, the screen says whether it is an Orb of
  Travel (`pedestal::is_orb_of_travel`) and, if it is, where it goes
  (`by_orb(..).name`). A duplicate says so and stays a weapon - the engine
  already refuses it, and the screen has to say *why* rather than doing
  nothing, which is the fault being fixed.
- **The inventory is reachable from it**, or there is nothing to drag.
- **A way out.** A LEAVE button that is always live, so an orbless run is not
  trapped by the thing that was supposed to be furniture.
- **INVOKE RITUAL**, enabled only with a valid orb seated, which calls
  `feed_pedestal` and lands on whatever the orb's destination is - an event,
  a dungeon, or a siding inside one.

Six orbs and six destinations exist (`pedestal.rs:53`): Wayfarer's -> THE
BOLTER RACE, Pilgrim's -> den-rivals, Ferry -> mole-town, Stray ->
wumpus-world, Shunter's -> the-switchyard floor 5, Signalman's -> floor 1.
Two events, two dungeons, two sidings. All six are unreachable today.

**Deliverable:** every destination in `DESTINATIONS` can be arrived at by
playing, which is the thing that has never been true.

**Gate:** two tests. One in the house style asking, of every `Destination`,
that some interface path reaches it - the `which_pools_a_board_can_actually_make`
shape, aimed at the road instead of the pools. One that a town visit
containing a pedestal click leaves the run in a state that differs from
before it, so "the click resolves to nothing" cannot come back.

**Worth checking while in there:** the CLI has no pedestal verb either, so the
scripted-replay contract cannot exercise any of this. A `pedestal <n>` verb is
cheap and makes the destinations testable from a script.

### M10 - The second voice stops advertising itself. **▲**

The turtle telling is on the mode-select screen as a card the size of the
mode cards, captioned TALES FROM THE CRYPT and blurbed *"The same game, told
in the language of the book. It's about a turtle."* It is the first thing a
new player is asked to choose between, under a heading that says IN WHOSE
WORDS?, and it is a little raunchy. It should not be the front door.

**What it becomes.** The row offers one voice. The second is behind a hotspot
in the **top right** of that screen that is drawn as nothing at all -
background, no border, no hover tint, no cursor change. Clicking it turns the
turtle on.

**How it is built, and the three things that will bite:**

- `render_mode_select` (`main.rs:8758`) builds the row by iterating
  `theme::THEMES` and laying `n` cards centred on `LOGICAL_W`. With `n = 1`
  that centres a single card, which is the right look for free - the row keeps
  its IN WHOSE WORDS? heading only while there is a choice to make.
- **The hotspot rect must come out of a pure function**, not be computed
  between draw calls - trap 32, and `render_mode_select` already returns its
  rects rather than handling its own clicks, so it is the shape the file
  already uses.
- **`GEARMASTER_THEME` must keep working.** It is how screenshots and any
  scripted opening reach the turtle telling, and it is read at
  `main.rs:10656` before any of this. An env-set theme should arrive already
  unlocked, or the picker will hide the theme the run is actually in.

**Where the state lives.** Beside `chosen_theme` in `main`, session-only.
There is no settings file in this repo and this is not the milestone that
introduces one; a player who wants it every time can set the env var.

**Once it is on**, the card appears and is selected, because a hidden control
with no feedback is indistinguishable from a broken one. Clicking the hotspot
again puts it away.

**Deliverable:** a player who does not know it is there cannot find it by
reading the screen, and a player who does can turn it on in one click.

**Gate:** three tests, all in `gui`'s existing style.

1. The row offers exactly one theme by default, and both after the hotspot.
2. The hotspot overlaps **no** other control on that screen - the mode cards,
   the difficulty cards, the theme card. `no_two_buttons_share_a_pixel` is
   the test already in the file that does this shape, and the failure it
   guards against is a player finding the thing by accident, which is the
   whole point.
3. `GEARMASTER_THEME=td` arrives unlocked.

**Not in scope:** the turtle text itself, which is untouched, and
`two_voices`'s budget of 5, which this cannot move - the engine's themes are
unchanged and only the picker's iteration is.

### M11 - The road past Francis, for a run that earned it. **▲**

**The bug.** Reported from play: held the item for the unwinding, beat Francis,
and the road past him never appeared.

`run.rs:2658` is the door, and it asks for **two** things:

```rust
if self.rung == LADDER.len()
    && self.flags.contains(&"looked-through-the-lens")
    && !self.answered.contains(&"the-unwound")
```

The mainspring is not in that condition at all. The only thing that sets
`looked-through-the-lens` is one choice in **THROUGH THE CRACKED LENS**
(`event.rs:1866`), which stands on `at: 47` - displayed rung 48 - and whose
choice `requires: Requirement::Holding("The Cracked Lens")`. Miss the lens,
or reach rung 48 without it, and the flag can never be set: the event is a
`Trigger::Rung` door standing on exactly one rung, so the window shuts and
does not reopen.

So a run can do the entire chain, hold `An Unwound Mainspring`, put Francis
down, and be told nothing. That is trap 8 - a key that arrives after its
door's window shuts - and `completable.rs` did not catch it because the flag
*is* set by something, and the shape it knows is a flag set by nothing.

The design comment says this was deliberate: *"having looked is what makes the
door appear, and holding the mainspring is what opens it... you cannot miss
what you never saw."* It is a good line and it is wrong in practice, because
the thing being missed is not a hint - it is the entire ending, gated behind
a second collectible on a one-rung window. **The owner's call is that the item
is the key.**

**What it becomes.** `holds(MAINSPRING)` opens the road on its own. The lens
keeps what it is actually good at - `Outcome::Scout`, seeing the boards ahead
- and the flag stops being load-bearing.

The prose has to move with it: THE UNWOUND's opening should read for a player
who never looked through anything, and `past_the_top` already asks only for
the mainspring, so the two agree afterwards where today they do not.

**Gate:** a test that a run holding the mainspring and no flag at all is
offered the road past the top, and a row added to `completable.rs` for the
shape that got past it - **a flag whose only setter sits behind a second
requirement in a one-rung window**. That row is the deliverable that stops
the next one, and it is the fifth shape that file knows.

### M12 - Francis doubles. **▲**

`Run::monster` ends with `&LADDER[self.rung.min(LADDER.len() - 1)]`. Past the
ladder that clamp refights **plain, unscaled Francis, for ever**. The road
does not end; it just stops meaning anything. This milestone gives the clamp
a purpose: rung `50 + n` is `2^n` Francis.

**The finding that has to shape it.** "Twice as difficult" cannot be twice the
health. `SUDDEN_DEATH_MS` is 30,000 and nothing runs past ~44 s (CLAUDE.md
trap 5): at 9,400 health Francis is already a long fight, and at 18,800 he is
mostly a clock fight, at 37,600 he is *entirely* one - both sides bleed a
growing share of max health each second and the winner is whoever was ahead at
thirty seconds. Doubling health past `n = 1` does not make the fight harder,
it makes it shorter to describe and impossible to lose *or* win on the boards.

So the dial is his **output and his defences**: strength (215), the three
resists (78/76/96) and `curse_resist`, with health scaled far more gently or
not at all. What exactly doubles is the one thing in this milestone worth
measuring before choosing, and the oracle is right there - the printer already
fights the reference boards against anything.

**The other four things that will bite:**

- **Overflow.** `2^n` on an `i32` leaves the rails around `n = 18`. Saturating
  arithmetic and a stated ceiling, and the ceiling should be a named constant
  with the reason on it rather than a clamp somebody finds later.
- **Which `n`.** "Every time you beat him" and "rung minus 50" are the same
  number on every run except one that took the THE UNWOUND detour, where rung
  51 is not a Francis at all. **This needs your answer** - the milestone
  assumes a counter of Francises beaten, because that is what the first
  sentence of the request says and it survives the detour.
- **Determinism.** The multiplier must be a pure function of that counter and
  live on `Run`, not come from the RNG - `simulate_party` consults no RNG
  anywhere and this must not be the thing that changes that.
- **`stepped_component` is not the tool.** It steps gear along the catalogue
  and the catalogue runs out; doubling is a scalar on the spec. A scaled
  **copy** of the `MonsterSpec` at fight time is the shape - it is already
  `Copy`, and `fight_next` already dereferences it into one.

Bounty should double with him, or the fight is worth less every time it gets
harder.

**Gate:** the ladder is byte-identical up to rung 50 - this may not touch a
single fight anybody currently plays - plus a test that `n = 0` is exactly the
Francis in `LADDER`, one that the multiplier is a pure function of the counter,
and one that the ceiling holds rather than wrapping.

### M13 - The second-order sweep. **▲**

Every milestone from M9 to M12 changes something a player meets, and three of
them change engine behaviour. This milestone is the pass that asks what *else*
moved, run after the rest have landed rather than during them, because the
useful detail is what was happening when a thing surfaced.

`analysis/second-order.md` is where it goes - it already exists, it is already
this exact document for the catalogue sweep, and its numbering continues from
12. Each entry recorded as it appears, with the evidence.

**Three outcomes and each gets a different answer:**

- **A bug** - fix it in this milestone and say so in the entry.
- **A design outcome** - record it and surface it to the owner. Not fixed
  silently: a consequence somebody chose to accept is worth more written down
  than quietly patched.
- **Something that needs its own milestone** - file it as one, numbered after
  this, with the same shape as every other entry here.

**Deliverable:** nothing in M9-M12 has a consequence that only shows up in a
later mission.

**Gate:** the suite, both printers diffed against the tip, and a walk of the
road far enough to meet what changed - the pedestal in High Wick, the mode
screen, the door past Francis. `prose`'s ignored printer reads the road aloud
and is the cheapest way to meet all three.

### M14 - What the book recipe costs, and who decides. **▲**

M3 was attempted at `cab0364` and the recipe itself is not the problem. This
milestone is the decision it runs into, which is the owner's rather than the
implementation's.

**What was measured.** With the §2.2 recipe applied - `Book 1 + Spell 1-2 +
Ink 0-2 + Alignment 0-1`, every bound relaxed - the engine goes to **909 green,
6 failed**. Four are the re-pins M3 already named. Two are not:

```
e6_5_the_unwound_is_harder_than_francis
  ["friend: won in 10.0s", "owner: won in 28.5s", "perfect: lost in 3.8s"]
  wanted at least two losses, got one

the_unwound_finishes_inside_the_measurable_region
  THE UNWOUND falls in 10.0s, which is not a boss (wanted 16s-30s)
```

**Both are the same fact:** the friend's weapon grid re-partitions into a
fully-loaded book weapon and its time against THE UNWOUND goes from a loss at
8.6s to a win at 10.0s.

**The narrower reading does not help.** Holding `Spell` at exactly one and
relaxing only the ink gives byte-identical results. The optional ink *is* the
change - a book and a spell with no ink now assemble, and the friend's run is
described in `share.rs` as "half of it deliberately loose". Those pieces were
loose because the old recipe would not take them. There is no version of
"books buildable with just books plus spells" that leaves them loose.

**And THE UNWOUND cannot be tuned to absorb it.** Four sweeps, all at Medium:

| Dial | Range swept | Effect |
|---|---|---|
| `magic_resist` | 30 -> 80 | **perfectly flat** - the boards pierce straight through |
| `strength` | 100% -> 25% | no change to who wins |
| `health` | 15,000 -> 26,000 | no change at all; the boards die, so its health is never reached |
| `gear_offset` | 0 -> -6 | still a loss for every board |

Resistance is inert because piercing answers it and only *hardening* answers
piercing - and `MonsterSpec` has no hardening field. Hardening comes from gear
alone. The owner's own attempt at that is on branch **`unwound-gear`**: nine
placements, taking it from 43 to 52, which beats all four boards, the best in
5.30s. Trimmed to five placements it is 12.65s, to three 13.55s - still losses.
Gear grants offence and defence together, and its offence is what the boards
cannot survive.

So the ordering cannot be restored by any scalar: friend kills it in 10.0s and
owner in 28.5s, and every uniform buff moves both together. Friend will always
get there first.

**The three ways out, and they are all decisions:**

1. **Re-author the friend share code** so it means what it meant - the loose
   half stays loose under the new recipe. It is a published constant players
   can paste, and it moves `baseline`, `progression`, `francis` and
   `reference_builds` together.
2. **Re-author THE UNWOUND** against boards that have book weapons, which is
   the "re-authoring, not tuning" §6 has been saying, now with the sweeps that
   prove tuning cannot do it.
3. **Revisit E6.5 itself.** "At least two of three lose to it" was written
   before books were worth building. A criterion measured against three
   historical boards ages with the catalogue, and this is it ageing.

**Not done, deliberately.** Re-pinning `lost >= 2` to `lost >= 1`, or the
16-second floor down to 10, is loosening a test to make a change pass -
doctrine 3 forbids it, and the reason it forbids it is exactly this case: the
number is the criterion, not the obstacle.

**Deliverable:** whichever of the three the owner picks, plus M3's own gate -
`baseline` and `catalog_shape` diffs read, and a book board fought against the
ladder with its rungs cleared written into `analysis/`.

### M15 - Difficulty is packing, and every theme can hit you. **▲**

The largest content milestone in this document. Difficulty stops being a
multiplier and becomes a board.

**Where it lives today** (`combat.rs`, `Difficulty`):

| Piece | Now | After |
|---|---|---|
| `factor()` | Easy 0.5, Medium 1.0, **Hard 3.0, Insane 9.0** | the two labels lose their meaning - see below |
| `each_way()` | `factor^0.25` on health and damage | **gone** for Hard and Insane |
| `passives()` | Medium `Hardened`; Hard `+Warded`; Insane `+Relentless` | Medium keeps `Hardened`; the other two **gone** |
| `gear_step()` | Easy -1, Medium 0, Hard +1, Insane +2 | **a decision - see the open question** |

**The new rule.**

1. **Hard is Medium until the Hollow King.** `LADDER[14]` is The Hollow King -
   rung 15 spoken, and the boundary. Every creature at or before it fights on
   Hard exactly as it does on Medium: same board, same stats, same fixture
   lines. The run-in stops being a difficulty selection at all.
2. **After him, Hard is Medium plus one more assembled item**, per creature.
   Not a better component - an additional item, which is a different axis from
   `gear_step` and the reason this milestone is packing rather than tuning.
3. **Insane is Hard plus one more on top.** Two more than Medium, after the
   Hollow King; identical to Medium before him.
4. **Every theme gets at least one assembled weapon or spell.** Five of the ten
   have no weapon slot today: Slower, Drainer, Hollow, Swarm and Warden
   (`bestiary.rs:119`). Hollow's comment explains itself - *"No weapon: mind
   damage is the helmet's"* - so this overrules a deliberate decision and the
   comment has to change with it, not be left contradicting the table.
5. **Monster boards get denser generally**, which is the thread running through
   all of it: Francis packs 95% of his cells in nineteen items and most of the
   ladder is nowhere near that.

**What this costs, named up front.**

- **`gear_at.txt` is re-baselined wholesale.** 6,216 placements today, and
  effectively every line after `LADDER[14]` moves at two settings. The rule
  that it may not be re-baselined without naming what changed cannot be met
  creature by creature here, so this milestone owns a single statement of the
  new rule instead - which is the honest version and should be written as such.
- **`stepped_component` stops being the difficulty story.** Trap 3 says a
  rating weight moving re-gears every creature on three settings; after this it
  re-gears them on fewer, and the trap text needs updating rather than leaving
  a stale warning in `CLAUDE.md`.
- **Every creature after rung 15 needs two authored items.** Sixty-odd
  creatures, two boards each. `make pack` is the tool that exists and the owner
  has recorded it as too slow, preferring a local build tool; whichever authors
  them, `tests/packing.rs` and `pack_francis.rs` are the harness and the TTK
  curve is what says a board is right.
- **`baseline` moves everywhere except Medium.** Its Easy/Hard/Insane columns
  are the measurement, and the printer diff is the deliverable rather than an
  afterthought.

**The open question, and it wants an answer before authoring starts.**

`gear_step` and "one more item" are two different ways to make a board harder,
and the request says *all* scaling should be packing. Two readings:

- **`gear_step` goes to 0 everywhere.** Hard and Insane differ from Medium
  only by item count. Cleanest statement of the rule; also the biggest change,
  because Easy's `-1` is what makes the early ladder gentle.
- **`gear_step` stays as it is.** Better components *and* more items after the
  Hollow King. Less pure, but it keeps Easy working and keeps two dials for
  authoring.

The milestone assumes the second unless told otherwise, because removing Easy's
step is not something the request asks for and doing it silently would change
the setting most new runs use.

**And the labels.** `Difficulty::label()` prints `factor()` as "3x" and "9x",
and those are shown on the mode screen. Once nothing is multiplied by three,
the label is a lie. Hard and Insane want naming by what they now are - a board
with one more item, and with two - which is a prose change on the difficulty
cards rather than a number.

**What landed, and what did not.** The difficulty rework is in at `da1f1a9`:
the multipliers and passives are gone, the run-in is flat through The Hollow
King, Hard adds one assembled item after him and Insane two, and every theme
has a weapon. Measured, owner's board over the whole ladder: Hard 47/50 ->
46/50, Insane 45/50 -> 44/50, Easy and Medium untouched.

**Denser Medium boards did not land, and should be their own milestone.** It is
the one part of the request that is authoring rather than machinery - sixty
creatures repacked by hand or by a tool - and it pulls *against* the extra-item
half: twenty-two of fifty boards already had no room for another item and had
to grow a row to take one. Packing Medium tighter makes that worse everywhere.
The order that works is to settle what a dense board is worth first, then
re-derive how much room the settings need on top.

**Gate.**
- A test that **Hard equals Medium at or before `LADDER[14]`**, creature by
  creature, on gear and on stats. That is the rule's sharpest edge and the
  cheapest thing to get wrong.
- A test that every creature after it has exactly one more assembled item on
  Hard than Medium, and one more again on Insane - counted through
  `combat_items`, not by reading the `items` partition, because a declared
  item is not an assembled one.
- A lint that **every `MonsterTheme` has a weapon or a spell** in its slots,
  the same shape as the pool and destination lints: walk the table, collect
  what can act, assert the enum defines nothing that cannot.
- `baseline` and `catalog_shape` printers re-run, both diffs read, and the new
  Easy/Hard/Insane columns written into `analysis/baseline.md`.

---

## 5. What shipped

Written at `cab0364`, updated through M15. Twelve of fifteen are in. M3 is
blocked and M14 is the measured statement of what blocks it. M15's difficulty
rework landed; the half of it that asks for denser Medium boards did not, and
is the note at the end of its section.

| | Milestone | Commits | State |
|---|---|---|---|
| M1 | The name, and the numbers | `84552b7`, `e8abe09` | **in** |
| M2 | The bonus can act, inert | `d22b34a` | **in**, ladder byte-identical |
| M3 | The book, rebuilt | - | **blocked** - see §6 |
| M4 | Four free bonuses, and Communion | `cee6459` | **in** |
| M5 | Three new primitives | `de6648a` | **in** |
| M6 | The pools get their symbols | `c406ed1` | **in** |
| M7 | The visual battle glossary | `66507c0`, `6b7e275` | **in** |
| M8 | Zealotry, DruidicMight, the record | `120db15`, `b40e793` | **in** |
| M9 | The pedestal gets a screen | - | **open** - a bug report, filed 2026-08-27 |
| M10 | The second voice stops advertising itself | - | **open** - filed 2026-08-27 |
| M11 | The road past Francis, for a run that earned it | - | **open** - a bug report, filed 2026-08-27 |
| M12 | Francis doubles | - | **open** - filed 2026-08-27; n counts Francises beaten |
| M13 | The second-order sweep | - | **open** - filed 2026-08-27 |
| M14 | What the book recipe costs, and who decides | - | **open** - three options, owner's call |
| M15 | Difficulty is packing, and every theme can hit you | `da1f1a9` | **in** - density half still open |

Suite at the tip: **907 engine green**, 48 ignored; **75 GUI green**; 0
warnings across the workspace. The ladder's movement from M8 is measured in
`analysis/baseline.md` under "The three fusions, and what arming the last two
cost".

Two things came out differently from how they were planned.

**M7's gate could not be written as specified.** "Every diagram lands inside
its row and none overlaps, at every width the panel can take" cannot be asked
of a page laid out inside `draw_*` calls, because `measure_text` needs a
graphics context and a test has none. So the geometry was split out:
`fight_diagram_layout(w, measure)` returns where every mark goes, the way
`wrap_measured` takes a measure, and `draw_fight_diagrams` paints that list
and chooses nothing. The test hands it a stand-in font deliberately *wider*
than the real one, so a row that fits under it fits in the face - a one-sided
guarantee rather than a guess. Worth knowing: the page ends 22px above the
panel floor, so one more section overflows and now something says so.

**M8's Heartwood is larger than the plan's one line.** The plan said only
"Heartwood -> DruidicMight". Built as described in the brief it came from, it
is also the first assembly bonus that pays for what you put *around* it: every
item beside it banks nature when it fires. That needed no new primitive -
`OnAdjacentActivate(Gain)` was already there - and it is what makes the
fusion self-sustaining rather than a one-shot at the bell.

---

## 6. Why M3 is blocked

Relaxing the book recipe to Book + Spells (inks and alignments optional) does
what it was meant to: the friend's weapon grid re-partitions into a
fully-loaded book weapon, 17 items becoming 18. It then beats THE UNWOUND in
10.0s, and `acceptance::e6_5` is the test that says it must not.

Repacking the reference boards does not help, and it is worth writing down
why: a packer maximises assembly, so it makes the problem larger.

The root cause is not the recipe. **THE UNWOUND lives in `ALTERNATES`, not
`LADDER`**, so `loadout_at`'s depth lookup returns `None` for it and it is
geared with no piercing and no hardening at all. Its 30/30/40/60 resist block
is decorative against reference boards that pierce 75-110. Two sweeps confirm
it: health is a cliff rather than a dial (15,000 and both boards win; 20,000
and both lose), and a magic-resist sweep from 30 to 70 is perfectly flat.

The fix is on branch `unwound-depth`: an `OFF_LADDER_DEPTH` table beside
`ALTERNATES`, because `combat.rs` references no other module by design, linted
from `the_road.rs` so a creature added off the ladder cannot skip it. It
breaks exactly one test, and honestly: restoring the defences makes THE
UNWOUND unbeatable by the boards that exist - the owner's 28.5s win becomes a
12.6s death - and no single scalar restores the ordering. It wants
re-authoring, not tuning, and that is the owner's call.
