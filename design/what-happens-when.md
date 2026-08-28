# What happens when — the gear card, rewritten

Written against `020bc7c` (2026-08-28), suite at **1043 green, 51 ignored**.
Every count below was measured off that tip.

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

| Per activation | Passive |
|---|---|
| `armor`, `mana`, `mind`, `physical_damage`, `magic_damage`, `rage`, `faith`, `nature` | `health`, `strength`, `regen`, `power`, `mind_resist`, `curse_resist`, `physical_resist`, `magic_resist`, `physical_pierce`, `magic_pierce`, `physical_harden`, `magic_harden` |

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

Three groups, in the order a player needs them:

```
PASSIVE          what it is worth standing still
WHEN IT FIRES    what one activation hands over
TRIGGERS         what makes it do something else
```

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

### Chips, because three headings is usually two too many

Measured over the 518 pieces:

| Shape | Pieces |
|---|---:|
| passive only | 296 |
| passive + activation | 135 |
| passive + triggers | 162 |
| activation + triggers | 0 |
| **all three** | **1** |
| none of the three | 29 |

**One piece in the catalogue has all three groups.** Three full headings on
every card would be two empty labels almost every time, so the headings are
drawn only for groups that have something in them — and where a group has a
single line, it collapses to a **chip**: the glyph, the figure, and a small
when-mark, on one row. The same compaction the class strip and the `G`
glossary already use.

The when-mark is three symbols and no words: a **dot** for standing still, a
**spark** for one activation, a **link** for answering something else. Drawn
once in the fourth glossary shelf (M7's HOW A FIGHT WORKS), which is already
the page that draws relations rather than describing them.

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

Each ends green on both suites with no warnings. ▲ marks a deploy.

### T1 — The audit, and the printer that finds the next one

- An ignored printer that walks `CATALOG` and prints, per piece, every figure
  it grants and when it happens — the three groups, resolved.
- Its output committed to `analysis/what-happens-when.md`, which is the list
  the owner asked for: every piece like Rootbound Material, named.
- **Gate:** a lint that every field `Stats::parts()` can print has a `when`,
  so a field added later cannot slip through unclassified. Red until T3 lands
  the classification, so it ships `#[ignore]`d with its reason, or T3 comes
  first — see the note at the end.
- **Deliverable:** the disagreements are a list rather than a suspicion.

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

### T4 — The piece card in three groups ▲

- The card renders `PASSIVE` / `EVERY n.ns` / `TRIGGERS`, headings only where
  a group is non-empty, single-line groups collapsed to a chip.
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
- The CLI's `inv` and `items` print the same three groups in text.
- **Gate:** a test that a piece's card and its item's card agree about which
  group every figure is in — the disagreement that started all of this.

### T6 — The when-marks, the shelf, and the record ▲

- The three when-marks drawn, and a row on the fourth glossary shelf that
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
3. **Three headings on a two-line card.** 296 pieces are passive-only and 29
   have nothing at all. If the rewrite makes those cards *taller*, it has made
   the common case worse to fix the rare one. The chip collapse is not a nicety.
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
