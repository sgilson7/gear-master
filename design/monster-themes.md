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
| **Wall** | Chest, Helmet, **Weapon** (one item) | armour, health, harden, **reflect** | slow; heavy; hits back harder when hit |
| **Burner** | Weapon, Greaves | searing, damage | kills on the clock, not the swing |
| **Slower** | Greaves, Gloves | frost, stun, misfire, `OnBattleStart` | denies tempo; deals little itself |
| **Drainer** | Gloves, Helmet | `Drain`, `Consume`, mind damage | starves a build that banks pools |
| **Caster** | Weapon, Helmet | spells, mana economy, forking | bursty and mana-gated |

Every slot appears in exactly two themes, so no grid goes unrepresented and no
theme is a superset of another.

*Amended, 2026-08-25 - four more, and the table moves into the engine.*

`the-unwinding.md` H4 adds four themes for the things that stand **beside** the
road: dungeon floors, the four destinations, and the thing after Francis. §4's
existing rule already exempts those from the curve and the clusters both, and
these four are exempt from the rung table for the same reason.

| Theme | Slots | Vocabulary | Reads as |
|---|---|---|---|
| **Hollow** | Helmet, Chest | mind damage, `Drain`, Dread, Insight, curse resist | takes your maximum away, and none of it comes back |
| **Swarm** | Gloves, Greaves | speed, `ReduceCooldown`, reactions, small blows | everywhere at once, and nowhere for long |
| **Beast** | Weapon, Chest | strength, rage, health, physical damage | no trick at all, and enough of everything else |
| **Warden** | Chest, Greaves | armour, harden, `GainDeflection`, frost/stun/misfire | out-waits you rather than out-hitting you |

**"Every slot appears in exactly two themes" was a property of six, not a
rule.** Ten themes cannot have it, and two of them - Swarm and Slower - fill
the same pair of grids. They are not remotely the same creature, because a
theme is a pair of grids *and* a vocabulary: one is quick and small, the other
lands curses and deals almost nothing. Where the six overlap, the vocabulary is
the whole of the difference, and `every_theme_can_find_something_to_wear_in_
every_grid_it_fills` is what keeps a vocabulary from being empty in a grid it
claims.

**Hollow has no weapon and does not need one.** That is the difference between
it and the Wall, which had to be given one: mind damage is the helmet's, so a
Hollow can already reach you through a grid it fills. What it cannot do is
appear in a damage share - mind damage removes maximum health and never touches
`Event::Hit` - so anything measuring whether a Hollow can hurt you has to read
the mind table.

**The table lives in `crates/engine/src/bestiary.rs` now.** It was a test-local
enum in `tests/pack_francis.rs` for as long as the only thing that needed it was
the search that authors boards. A `MonsterFrame` carries a theme and a frame is
engine data - a creature that exists before its board does - so the table came
home, and the packer and the interface read it from there.

**Wall is where reflection lives.** It is the only theme where paying back
absorbed damage earns its keep, and confining it there is what lets reflection
spread across the chest catalogue without arming every creature on the ladder —
which is precisely what went wrong the first three times chest gear was touched.

*Amended, 2026-08-25 — why the wall has a weapon.* Wall was the only theme of
the six whose slots deal no damage, and a creature in this game fights
**entirely through its gear**: exactly one creature on the ladder has an innate
attack, and it is the Cave Rat's bite. So a chest-and-helmet wall lands nothing,
ever. Packed that way The Iron Warden could not hurt anybody, two of them were
no harder to fight than one, and nine tests failed in nine different vocabularies
describing the same hole.

Reflection was supposed to be the answer and cannot be. It needs the player to
swing first and the wall's armour to soak the blow; it is reported as
`Reflected` rather than `Hit`, so nothing that measures whether a creature can
hurt you is able to see it; and it can never threaten somebody who out-damages
it. It is a way of *punishing* an attack, not a way of making one.

So a wall carries a weapon — **one item** of it, which is the most any creature
may carry. It is still slow, still mostly armour, and it still hits back harder
than it hits. What it is not any more is a thing that cannot fight.

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
5. *Added 2026-08-25.* **A creature off the ladder has to be told its rung.**
   The four in `ALTERNATES` - The Dreaming Idiot, and the three floors of the
   crevice - stand beside the road rather than on it, so nothing in the game
   says how hard they are meant to be, and the curve, the density target and
   the theme are all functions of a rung. The packer takes `PACK_RUNG` and
   refuses to guess. All four are met at the shrine fork on **rung 10**; whether
   a dungeon floor should be packed for the rung it is entered from, or for
   something deeper because it is optional and pays a class, is a question this
   document does not answer yet. Their current boards - 27 to 40 pieces against
   the five a rung-10 creature carries - say somebody already thought they
   should be much harder.

That last point is the one to hold on to. Forty boards were attempted with the
gate and all forty were skipped, because a denser board could not reproduce the
fight the creature already gave. A themed board is a *different* fight by
design, so the gate must be re-aimed at the curve — "is this the right
difficulty for this rung" — rather than at the board being replaced. That is the
one place this design asks for a judgement the current tooling cannot make on
its own.

---

## 6. The curve, and the gate re-aimed

Section 5 left one question open: the acceptance gate asked *"is this the same
fight the creature already gave"*, which cannot accept a themed board, because a
theme changes what a creature is on purpose.

The gate asks about difficulty now:

```
target(rung) = 2.8s + 0.4s × rung
```

Read off the **owner's board at Medium**, within **±30%**. Medium is one times,
and the owner's is the only reference board that clears far enough up the ladder
to give a reading at every rung. Rung 1 is 3.2s, rung 25 is 12.4s, rung 50 is
22.4s; its median across the 46 rungs it currently clears is 14.4s, so the line
runs through roughly where the game already sits rather than moving it.

*Amended, 2026-08-25.* **That last sentence was wrong twice over, and the line
it was defending is right for a better reason.**

The median was 14.4s on a board that was being rebuilt without locking each item
as it assembled. Rebuilt correctly the owner's board clears **45 of 50** and its
median is **9.00s**. So the line does not run through where the game sits: of
the 37 rungs whose fights are decided by the gear, only **13** land within ±30%
of it. Rung 23 takes 4.55s against a target of 11.6s; rung 26 takes 24.0s
against 12.8s. The ladder is not a ramp, it is a scatter — which is the thing
the repack exists to fix, so a target the ladder does not follow yet is exactly
what a target should be. It was never a description and should not have been
written as one.

The eight remaining wins are not measurements at all. **Sudden death begins at
30s** (`combat.rs:40`), so every fight past that point is being finished by the
clock rather than by anybody's gear: rungs 40, 42, 43, 44, 45, 46, 48 and 49 all
land between 37s and 43s, and 43.00s appears four times because that is where
the escalation happens to reach these boards. A curve fitted through those is a
curve fitted through the clock.

Which gives the slope its real justification. **The band's top edge must clear
sudden death.** At 0.4s a rung the line reaches 22.4s at rung 50 and +30% of
that is 29.1s — just inside the 30s where the gear stops deciding. Any steeper
and the top of the ladder is authored into a region where the fight is settled
by escalation, and the packer would be tuning boards it cannot measure. That is
a constraint the line satisfies, rather than a coincidence it was justified by,
and it is why the line is unchanged here despite the numbers under it moving.

The floor was two seconds for about ten minutes. Rung 2 wanted 2.4s and the
best themed board any search could find took 3.2s — a striker at rung 2 cannot
be built weaker than that out of gear that assembles. A curve whose bottom end
nothing can reach rejects the entire early ladder, so the floor is measured
rather than assumed.

With it, Bog Toad packs: **seven** pieces, weapon and gloves only, on the curve.

It was fifteen for a while, and that was a bug worth recording. §5.3 - the
seating loop targeting `pieces(rung)` - had not been built, so the loop was
still bounded by the old ceiling of "twice what the board has, or eight more",
which is a bound relative to the board being *replaced*. That was the right
guard while the job was densifying existing boards and the wrong one the moment
the job became authoring them to a curve. Rung two asks for five.

Seven rather than five because the loop checks the bound before seating an item
and an item is two to four pieces, so it overshoots by up to one item. That is
tolerable - a board is built of items, not pieces, and the alternative is
refusing the last item and landing short - but the curve should be read as
"about `3 + rung`" rather than exactly.
