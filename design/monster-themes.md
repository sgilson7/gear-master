# Monster themes

Every creature on the ladder is currently packed the same way: fill all five
grids with the highest-rated gear inside a band. Forty-five of fifty-two boards
use all five slots, nothing is built around any mechanic, and the shapes that do
emerge — a wall here, a curse-lean there — are accidents of which pieces happened
to rate highly rather than decisions.

That is why the boards feel alike, and it is also why the packer keeps producing
forty-three-second stalemates: it optimises density with no idea what the
creature is supposed to *do*.

A theme fixes three things at once. It makes a creature legible in the first
three seconds of a fight, it cuts the packer's candidate pool by roughly sixty
percent, and it makes difficulty predictable — because once the vocabulary is
fixed, density is the only dial left.

---

## 1. The six themes

Each names the slots it fills and the vocabulary it draws from. **The packer
considers nothing outside them.** Two slots, not five: two is what a player can
read at a glance, and five is what made every creature the same creature.

| Theme | Slots | Vocabulary | Reads as |
|---|---|---|---|
| **Striker** | Weapon, Gloves | damage, strength, reaction damage | fast and fragile; punishes a slow board |
| **Wall** | Chest, Helmet | armour, health, harden, **reflect** | slow; hits back only when hit |
| **Burner** | Weapon, Greaves | searing, damage | kills on the clock, not the swing |
| **Slower** | Greaves, Gloves | frost, stun, misfire, `OnBattleStart` | denies tempo; deals little itself |
| **Drainer** | Gloves, Helmet | `Drain`, `Consume`, mind damage | starves a build that banks pools |
| **Caster** | Weapon, Helmet | spells, mana economy, forking | bursty and mana-gated |

Every slot appears in exactly two themes, so no grid goes unrepresented and no
theme is a superset of another.

**Wall is where reflection lives.** It is the only theme where paying back
absorbed damage earns its keep, and confining it there is what lets reflection
spread across the chest catalogue without arming every creature on the ladder —
which is precisely what went wrong the first three times chest gear was touched.

## 2. Clustering

Themes run in stretches rather than rotating, so a player has time to work out
what is in front of them and build against it. A stretch is long enough to
learn and short enough not to bore: **five to eight rungs**.

The order matters. It opens on the two themes that punish nothing — a striker
teaches you to pack damage, a wall teaches you that damage alone is not enough —
and holds the drainer back until pools are something a build actually has.

| Rungs | Theme | Why here |
|---|---|---|
| 1–6 | Striker | the first thing you learn is to hit back |
| 7–13 | Wall | and the second is that hitting is not enough |
| 14–20 | Burner | introduces the clock, before curses are counterable |
| 21–28 | Slower | tempo denial, once cadence is a thing you own |
| 29–36 | Caster | bursty, and answerable by then |
| 37–44 | Drainer | punishes hoarding, when hoarding is possible |
| 45–50 | mixed | the run-in: whatever each creature already is |

The last stretch is deliberately unthemed. By rung forty-five a build has
answers to everything, and the interest is in the specific creature rather than
the category.

## 3. Mini-bosses are hybrids

A creature whose `rank` is not `Ordinary` draws from **two** themes: its
cluster's, and the one the *next* cluster will introduce. That makes a
mini-boss both a harder version of what you have learned and the first sight of
what is coming — which is what a mini-boss is for.

`MonsterSpec.rank` already carries this, so no new data is needed.

Francis keeps his hand-authored board. He is the one creature whose gear is a
character note rather than a category — a gambler in a coat with a sword — and
the two nudges in `pack_francis` that say so are already gated to him.

## 4. Density is the curve; theme is the character

Piece count comes from the rung and nothing else:

```
pieces(rung) ≈ 3 + rung
```

One more piece a rung, from a base of three. Rung 1 lands at four, rung 25 at
twenty-eight, rung 50 at fifty-three. Francis at forty-four sits just under the
line, which is about right for a boss whose board was authored by hand.

Two rules keep this honest:

- **The theme decides *where* the pieces go, never how many.** A wall at rung 30
  and a striker at rung 30 have the same piece count and completely different
  boards.
- **Events and dungeon fights are exempt** from both the curve and the themes.
  They are authored set-pieces standing beside the ladder, not furniture on it.

## 5. What the packer needs

Small changes, all of them constraints rather than new machinery:

1. `MonsterTheme` as data — slots and allowed vocabulary per theme, plus the
   rung→theme table above.
2. The pool filter in `pack_francis` narrows to the theme's slots and
   vocabulary before rating order is consulted.
3. The seating loop's target is `pieces(rung)` rather than "as many as fit".
4. The acceptance gate stays exactly as it is: same outcome and time-to-kill
   within a quarter, measured against the three reference boards **at Medium**,
   which is one times.

That last point is the one to hold on to. Forty boards were attempted with the
gate and all forty were skipped, because a denser board could not reproduce the
fight the creature already gave. A themed board is a *different* fight by
design, so the gate must be re-aimed at the curve — "is this the right
difficulty for this rung" — rather than at the board being replaced. That is the
one place this design asks for a judgement the current tooling cannot make on
its own.
