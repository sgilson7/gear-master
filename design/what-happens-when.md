# What happens when — the gear card, rewritten

Written against `020bc7c` (2026-08-28), suite at **1043 green, 51 ignored**.
Every count below was measured off that tip.

**T0 is a hard lock found in play and is not part of this rewrite.** It is
first in §4 because a run that walks a chain to its end cannot get out of it.

A card tells you what a piece is worth. It does not tell you **when**, and for
most of the catalogue that is the whole of the information.

---

## 1. The complaint, and how far down it goes

Rootbound Material's card says `+2 nature each time its item fires` and then
prints it in the stat colour, in the stat block, beside `+8 curse res`. A
reader takes it as *this piece gives 2 nature*. It does not: it gives 2 nature
**every 2.8 seconds for the length of the fight**, which over a thirty-second
bout is twenty-odd nature, and the two readings are not close.

The rule doing it is deliberate and written down (`main.rs:4270`):

> an unconditional pool gain reads as a stat, because that is what it is. Only
> the conditional triggers — spending a pool, answering a neighbour — earn a
> line of their own.

It is half right. A pool gain is not conditional, so it does not need trigger
*wording*. But it is not a stat either, and putting it in the stat block says
it is.

### It is not one rule, it is three faults stacked

**Fault one — an inconsistency inside the same trigger.** `Trigger::OnActivate`
carries **15** distinct actions in the shipped catalogue: `Accrue`, `Ballast`,
`Curse`, `Damage`, `Drain`, `Gain`, `GainArmor`, `GainDeflection`, `GainDread`,
`GainMana`, `GainShield`, `Grow`, `MindDamage`, `ReduceCooldown`, `Shunt`.
Thirteen of them get a trigger-coloured line of their own. Two — `Gain` and
`GainMana` — are folded into the stat block. Nothing about the engine
distinguishes them; a helmet that gains 22 armour on activation and one that
gains 2 nature on activation are the same shape of thing, and the cards say
otherwise. **42 pieces** are folded this way.

**Fault two — and this is the large one — `Stats` is not passive.** Eight of
its fields are handed over *on every activation*, by the same code path as an
`OnActivate` trigger (`combat.rs:5150`, inside the fire path):

| Damage, per activation | Everything else, per activation | Passive |
|---|---|---|
| `physical_damage`, `magic_damage`, `mind` | `armor`, `mana`, `rage`, `faith`, `nature` | `health`, `strength`, `regen`, `power`, `mind_resist`, `curse_resist`, `physical_resist`, `magic_resist`, `physical_pierce`, `magic_pierce`, `physical_harden`, `magic_harden` |

The first column is split out because damage is not one more figure in a list -
it is the figure a reader came for, it is **totalled** rather than itemised,
and it is the only group whose parts have to be multiplied through the item's
own power before they mean anything. §2 gives it a group of its own.

`Stats::parts()` prints all of them in one undifferentiated list, and the piece
card renders that list as one block. So **201 pieces** carry a per-activation
figure (armour, damage, mind) inside `Stats` and show it exactly where they
show `+175 hp`. Rootbound Material is the visible case because its wording
gives the game away; the other two hundred say nothing at all.

**Fault three — two spellings of one effect.** Because `Stats { nature: 2 }`
and `OnActivate(Gain { Nature, 2 })` are the same thing to the engine, the
catalogue says it both ways:

| Spelling | Pieces |
|---|---:|
| a `Stats` field | **158** |
| an `OnActivate(Gain)` | **18** |
| **both, on the same piece** | **20** |

The twenty are the interesting ones. Their card adds the two figures together
into one number — correctly, since the engine does — which means the card is
already doing the arithmetic that proves the two spellings are one thing, while
the piece tooltip shows them in different colours in different places.

### What the item card already knows

`item_summary_lines` splits into **OUT OF COMBAT** and **IN COMBAT — every
2.80s**, and puts armour, damage and the pools under the second. It is most of
the way to the answer already. The piece card has no sections at all, and the
two disagree about the same piece.

---

## 2. What a card becomes

Four groups, in the order a player needs them:

```
DAMAGE           what it hits for, totalled - and what answers it
PASSIVE          what it is worth standing still
WHEN IT FIRES    what else one activation hands over
TRIGGERS         what makes it do something else
```

**Damage is its own category and it is not the weapon's.** It belongs to any
item that deals any, which today means a weapon that swings *or* anything at
all that carries mind - the mind lane is the helmet's, and `item.mind` is
handled outside the weapon branch (`combat.rs:5094`) precisely so a helmet can
reach you. A helmet doing 40 mind gets the same top line a sword doing 61
physical gets, because to the reader they are the same question.

It stays **totalled and at the top**, the way `item_summary_lines` already
prints `hits for 61 (21.7 a second)` with its lane breakdown beneath. That
line is the best thing on any card in the game and none of this touches it -
the change is that it becomes a labelled group, that mind joins it, and that
it appears on the non-weapons that deal some.

### The thing this group will expose

`hit_for` returns **0 for anything that is not a weapon** (`loadout.rs:104`),
and that is not a display shortcut - it is the fight. `second-order.md` §10
recorded the consequence two missions ago and nothing has drawn it since:

> Twenty-three components carry raw damage they can never land - twelve
> gloves, seven chest, three greaves, one helmet - and `rating.rs` prices every
> point of it.

So a glove with `physical_damage: 8` has a figure that does nothing, and its
card has always shown it as though it did. Once damage is a group with a
total, that piece has a group whose total is **zero** — and the card either
says so or omits the group, and either way the lie stops. Mind is the
exception that proves it: same field family, different branch, and it lands
from any slot.

This makes the group an audit as well as a heading, and it is the reason to
build it from `hit_for` and the mind lane rather than from the `Stats` fields
directly. **The card must not total a figure the fight will not use.**

Rootbound Material, today and after:

```
  today                          after
  ---------------------------    ---------------------------
  Rootbound Material             Rootbound Material
  greaves material 2x2           greaves material 2x2
  +2 nature each time its        PASSIVE
    item fires                     +8% curse res  (curse)
  +8 curse res                   EVERY 2.8s
  18 gold                          +2 nature      (leaf)
                                 18 gold
```

The cooldown is in the heading because it is what turns a figure into a rate,
and it is the number the reader is missing.

A helmet that deals mind damage, which today has no total anywhere:

```
  Foreboding Crest              Foreboding Crest
  helmet crest 2x2              helmet crest 2x2
  ...                           DAMAGE
  +11 mind                        11 mind  (3.9 a second)
  +4 insight                    EVERY 2.8s
  22 gold                         +4 insight   (eye)
                                22 gold
```

### Chips, because four headings is usually three too many

Damage cross-cuts the other three - a weapon has damage *and* passives - so it
is counted separately. **201** pieces carry a damage figure of some kind, and
how many of those the fight will actually use is what T1 has to find out.

The other three, measured over the 518 pieces:

| Shape | Pieces |
|---|---:|
| passive only | 296 |
| passive + activation | 135 |
| passive + triggers | 162 |
| activation + triggers | 0 |
| **all three** | **1** |
| none of the three | 29 |

**One piece in the catalogue has all three of those groups**, and most have
one. Four full headings on every card would be three empty labels almost every
time, so headings are drawn only for groups that have something in them — and
where a group holds a single line, it collapses to a **chip**: the glyph, the
figure, and a small when-mark, on one row. The same compaction the class strip
and the `G` glossary already use.

Damage is the exception that keeps its heading whenever it is present, because
it is a total rather than a list and a total wants saying out loud.

The when-mark is four symbols and no words: a **blade** for damage, a **dot**
for standing still, a **spark** for one activation, a **link** for answering
something else. Drawn once in the fourth glossary shelf (M7's HOW A FIGHT
WORKS), which is already the page that draws relations rather than describing
them.

---

## 3. Where the truth has to live

**Not in the interface.** The GUI, the CLI and the item card each carry their
own copy of the fold-a-pool-gain rule today, and they already disagree. The
classification belongs beside the figures, in `stats.rs`, so that a field added
later cannot be printed by anything before somebody has said when it happens.

`Stats::parts()` already returns `(text, glyph_key)` and exists precisely so
`summary()` and the tooltip cannot disagree — the same argument, one step
short. It returns `(text, glyph_key, when)`.

And `when` is **checked against the fight, not hand-written**: the activation
path reads a known set of fields, and a test walks a probe item through one
activation and asserts that exactly the fields marked `WhenItFires` moved. A
hand-maintained table would be wrong within two missions; this is the same
trick `Combatant::pool_pays` plays for the glossary diagrams.

---

## 4. Milestones

Each ends green on both suites with no warnings. ▲ marks a deploy. T0 is a
bug found in play and is not part of the card rewrite; it is first because it
is a hard lock.

### T0 — The county freeze, which comes first ▲

Reported from play: *found The Drover, then the game froze on the hundred map,
and I couldn't move or click on anything.* It is a hard lock with no keyboard
or mouse escape, and it is reachable by any run that walks a chain to its end,
so it goes before everything else in this document.

**The mechanism, end to end.**

1. `county_walk` onto a `TileKind::Pinnacle` calls `begin_county_fight()`
   (`run.rs:2883`), which calls `fight_party` and sets `phase =
   Phase::Fighting` with a log waiting to be settled.
2. The GUI's `run.county_at.is_some()` branch (`main.rs:12406`) draws
   `render_county` and **`continue`s**. The battle screen is two hundred lines
   below it and is never reached.
3. `county_walk` opens with `if self.phase != Phase::Loadout { return false }`
   (`run.rs:2734`), so every further step is refused.
4. `leave_county` opens with **the same guard** (`run.rs:3063`), so the way out
   is refused too.

The map redraws for ever, every control is dead, and nothing on screen says
why. Three of the four facts are individually correct: the phase guard is right,
the branch order is right, and `begin_county_fight` is right.

**The comment directly above that branch describes this exact bug**, for
events:

> The event screen is two hundred lines below this branch and this branch
> `continue`s, so a county event set by walking onto a tile was drawn by
> nothing at all - and then appeared the moment the trip ended and `county_at`
> went to `None`.

That was found in play, fixed for events, and the same question was never
asked about fights. **A branch that `continue`s owns every screen the thing
inside it can ask for**, and this one asks for two.

**What it becomes.** The county yields to the fight and takes you back
afterwards, which is what the owner asked for in as many words: break off the
map to fight, then return to it.

- The county branch checks for a pending fight before it draws the map, and
  falls through to the battle screen rather than `continue`ing past it. The
  event check already there is the shape to copy - it is the same fix, one
  screen along.
- **After the fight, you come back.** `settle` currently clears `county_at` and
  `county_moves_left` on a pinnacle **either way** (`run.rs:3535`), so the trip
  ends win or lose. Winning should put you back on the map with the moves you
  had left; the tile is marked cleared and the chain is done, so there is
  nothing to walk into twice.
- **Losing keeps ending the trip.** That is A7 and it is deliberate - a loss
  costs what a road loss costs - so only the victory path changes. Said out
  loud because it is the half a reader will assume was an oversight.

**This is a design change as well as a fix**, and worth naming: the pinnacle
currently ejects you from the county, and after this it does not. A run that
banked ten moves and spent one on a chain keeps nine of them.

**Gate.**
- A test that walks a run onto a pinnacle tile and asserts the run is in a
  state some screen will draw - the general form of the fault, not the
  specific one. `phase == Fighting` with `county_at` set is exactly the state
  nothing drew, and it is checkable without a graphics context.
- A test that a won pinnacle leaves `county_at` set and `county_moves_left`
  unchanged, and that a lost one still ends the trip.
- A walker in the house style that bounds its steps - trap 24, because a
  county walk that runs until it runs out is a hang the day a tile refuses.

**Deliverable:** no state a county trip can reach is a state without a screen.

### T1 — The audit, and the printer that finds the next one

- An ignored printer that walks `CATALOG` and prints, per piece, every figure
  it grants and when it happens — the four groups, resolved — and for each
  damage figure, whether the fight will use it.
- Its output committed to `analysis/what-happens-when.md`, which is the list
  the owner asked for: every piece like Rootbound Material, named.
- **Gate:** a lint that every field `Stats::parts()` can print has a `when`,
  so a field added later cannot slip through unclassified. Red until T3 lands
  the classification, so it ships `#[ignore]`d with its reason, or T3 comes
  first — see the note at the end.
- **Deliverable:** the disagreements are a list rather than a suspicion,
  including the twenty-three components carrying damage the fight cannot land.

### T2 — One spelling ▲

Normalise the pool grants. `Stats { nature: 2 }` and `OnActivate(Gain {
Nature, 2 })` mean the same thing, so the catalogue picks one: the **`Stats`
field**, because 158 pieces already use it, the fight's own activation path
reads it directly, and it is the spelling that survives an item merging.

- Convert the **18** trigger-only pieces and fold the **20** that say it both
  ways.
- `Action::Gain` stays in the language — it is still how a *conditional*
  trigger grants a pool (`Consume`, `Watch`, `SpendMana`), and those are
  untouched.
- **Gate:** the ladder is **byte-identical**, printer diffed before and after.
  It must be: the engine already treats the two spellings identically, so any
  movement at all means the conversion changed a number. Plus a lint that no
  piece spells it both ways, budget zero.
- **Risk:** `assembly.rs` and `primitives.rs` pin trigger lists by piece name.
  Expect re-pins, each with the reason in the assertion.

### T3 — The engine says when ▲

- `Stats::parts()` returns the `when` beside each figure. **No display
  changes.** The house rule from `HANDOFF.md` §5 — land primitives inert, arm
  them separately — and the reason the ladder can be proved untouched.
- The classification is derived from, or checked against, the activation path,
  not written twice.
- **Gate:** a test that fires a probe item once and asserts exactly the fields
  marked `WhenItFires` moved on the combatant, and no other. That is the test
  that makes the rest of this safe.

### T4 — The piece card in four groups ▲

- The card renders `DAMAGE` / `PASSIVE` / `EVERY n.ns` / `TRIGGERS`, headings
  only where a group is non-empty, single-line groups collapsed to a chip, and
  damage keeping its heading whenever it is there.
- **Damage is built from what the fight uses**, not from the `Stats` fields:
  `hit_for` for the swing, the mind lane for everything that carries it. A
  figure the fight will not use does not get totalled - see §2.
- The `+2 nature each time its item fires` sentence goes: the heading says
  when, so the line does not have to, and the figure gets its glyph back.
- **Gate:** the layout test in the house style — every line and chip inside
  the card, nothing overlapping, at every width the card can take, off a
  **pure layout function** handed a measure. Trap 32, and the same shape as
  `fight_diagram_layout` and `pedestal_rects`.

### T5 — The item card and the CLI catch up ▲

- `item_summary_lines` keeps its OUT OF COMBAT / IN COMBAT split and gains the
  third group, so a piece and the item it becomes describe themselves the same
  way.
- The CLI's `inv` and `items` print the same four groups in text.
- **Gate:** a test that a piece's card and its item's card agree about which
  group every figure is in — the disagreement that started all of this — and
  that the damage total on both is the number `hit_for` and the mind lane
  actually produce, for a weapon and for a mind-dealing helmet.

### T6 — The when-marks, the shelf, and the record ▲

- The four when-marks drawn, and a row on the fourth glossary shelf that
  shows what each one means by drawing it.
- `analysis/what-happens-when.md` re-run and diffed; `CLAUDE.md`'s counts and
  a new trap — *a stat block is not a stat block; eight of its fields are a
  rate* — which is the thing that will bite the next reader.

---

## 5. What could go wrong

1. **The ladder moves at T2.** It must not. If it does, the two spellings were
   not identical after all and the difference is a bug worth more than this
   mission.
2. **`when` gets hand-written.** Then it is right for one mission. The gate at
   T3 is the whole safeguard and it should be built before the table, not
   after.
3. **Four headings on a two-line card.** 296 pieces are passive-only and 29
   have nothing at all. If the rewrite makes those cards *taller*, it has made
   the common case worse to fix the rare one. The chip collapse is not a nicety.
6. **The damage group turns a known mispricing into a visible one.** Twenty-three
   components will show a damage group that totals nothing, or no group where
   the old card showed a figure. That is correct and it is `second-order.md`
   §10 arriving on screen, but it will read as a regression to anybody who has
   not read it - so T1's audit should name those twenty-three by piece, and T6
   should say plainly in the record that the figures did not change, only the
   claim about them.
4. **`regen` reads as a per-activation figure and is not.** It is per second,
   passive, and it wears the leaf glyph — the one field most likely to be
   filed in the wrong group by eye. It is a good test case.
5. **`Stats::parts()` has other callers.** `summary()` and the CLI both walk
   it. Adding a field to the tuple touches all of them; adding a *method* that
   returns the grouping leaves them alone. Prefer the second.

---

## 6. The one open question

**T1's gate cannot pass before T3 exists** — it asserts every printable field
is classified, and nothing is classified until T3. Two readings:

- **T1 first, gate ignored**, with its reason, and T3 arms it. Keeps the
  audit's output available while the conversion in T2 is authored against it.
- **T3 first**, then T1's audit is a printer over a classification that
  already exists and its gate is green on arrival.

The order above assumes the first, because the owner asked to *identify the
pieces first and fix them*, and the audit is the identification. Say so if the
other way round is preferred; nothing else in the plan moves.


---

## 7. What shipped

Written at the end of the mission. All seven landed.

| | Milestone | State |
|---|---|---|
| T0 | The county freeze | **in** — and the trip returns you to the map |
| T1 | The audit and its printer | **in** — `analysis/what-happens-when.md` |
| T2 | One spelling | **in** — 38 pieces, and the gate caught two real faults |
| T3 | The engine says when | **in** — `Stats::parts_when`, checked against the fight |
| T4 | The card in four groups | **in** |
| T5 | The item card and the CLI | **in** |
| T6 | The marks and the record | **in** |

Three things came out differently from how they were planned.

**T2's byte-identical gate did not hold, and that was the point.** The premise
was that the two spellings are identical to the engine. They are identical in
*amount* and not in *order* - a `Stats` pool is banked sixty lines before
`OnActivate` triggers run - and `Figures::of` reads `stats.mana` without ever
walking a trigger, so eighteen pieces' worth of mana a second was invisible to
every toll in the county. Both are recorded at `second-order.md` 29. The gate
turned a premise into a question, which is what a gate is for.

**Three predicates were measuring spelling rather than behaviour.** The
census, `bestiary::plain` and `catalog_shape::inert` all asked "does this piece
have triggers". All three were already wrong before T2 - 158 pieces banked a
pool as a stat and were counted as filler - and T2 made them visibly wrong.
Corrected, the true inert count is **54, not the 124** the census had been
printing for two missions.

**T6 did not fit.** M7's layout gate fired on the first build, exactly as its
own commit predicted it would, and then its horizontal half fired too. The
section is in the shelf's second column now, which was empty the whole time.
