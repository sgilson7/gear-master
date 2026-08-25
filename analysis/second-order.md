# Second-order effects, for after the sweep

Things the catalogue rewrite has turned up that are *not* the catalogue's
problem, and that would be a mistake to fix in the middle of it. Each one is
real, each one has evidence, and none of them blocks the sweep.

Written down as they appear rather than at the end, because the useful detail is
what was happening when they surfaced.

---

## 1. Rating decides monster difficulty, and rating is a shop model

`stepped_component` (`combat.rs:252`) picks a creature's gear above Medium by
walking its footprint family in `piece_rating` order. So **every edit to a piece
or to `rating.rs` re-gears every creature on three of the four settings.**

Rating is a model of what an item is worth *in a shop*. What a monster needs is
what wins a fight. The two are not the same thing and the gap is measurable:

- Halving what `Grow` is worth - a defensible model with no fault behind it -
  moved Grow-carrying pieces down their families, Francis's Insane step swapped
  a damage crest for **Tithe Collector**, a drain, against a board that banks
  nothing, and the best board in the project then lost to him on Hard and beat
  him on **Insane**. A final boss who gets easier as the setting rises.
- Giving `reflect` a weight - correcting a stat that had none - took two
  unrelated tests red the same way.
- `francis.rs` has now asked for a defeat, a victory, and a defeat again across
  three separate corrections, none of which were about Francis.

**Addressed, partly, because it blocked the sweep.** Two catalogue commits in,
the ladder inverted again and the guard caught it - Francis beaten on Insane and
holding on Hard. Chasing that every commit was not workable, so
`stepped_component` now orders families by `rating::monster_value`: the ordinary
rating with the mechanics discounted whose worth depends on what the *other*
side happens to be carrying - drains, pool spending, mind damage. Monotonicity
came back and Francis has held through the commits since.

It is a coarse correction and should be read as one. `monster_value` is
`piece_points` minus three categories, not a model of lethality; the real answer
is a scoring function written for creatures from the start, and this is a patch
that makes the sweep possible. What remains true is the underlying point: **two
different questions were being answered by one number**, and anything else that
reads `piece_rating` to make a decision about a monster is making the same
mistake.

Three tests also had to stop pinning exact outcomes against named creatures
above Medium - the two Francis tables and the sudden-death escalation - because
those *are* coin-flips while the catalogue moves. Each now states its claim
("he is not walked through", "the escalation goes 1, 2, 3") rather than one side
of the coin.

---

## 2. A sweep can empty a rule of carriers

Taking `DoubleAdjacentItemStat` off Cursed Handle left the mechanic with **no
carrier anywhere**, which makes its exclusivity rule vacuous - a rule naming
something the catalogue no longer contains can never fail again.

`every_rule_names_a_mechanic_that_exists` catches it, and it will keep catching
it: the sweep's whole job is moving mechanics out of slots, and a mechanic with
one carrier in the wrong slot has nowhere to go but a new piece in the right
one. That is authoring, not sweeping, and it should be recognised as such when
it comes up rather than treated as a blocked move.

---

## 3. Reflection cannot reach the criterion it was built for

Reflection pays a share of what your **armour** absorbed. Armour resets to zero
every fight, so a board that kills a rung-25 creature in twelve seconds is never
carrying much of it: `absorbed_total` binds and the percentage does not.

Arming six chest pieces the finished boards wear, at five to nine percent,
returned a time-to-kill table that was **byte-identical**. That is the
measurement that says the mechanic, not the tuning, is what limits it.

The criterion was restated instead - the body is read in health, where it is
worth 28-48% - and that is the right answer for the criterion. It is not an
answer for the mechanic. If reflection is ever meant to be a damage channel
rather than a punishment for hitting a wall, the trigger has to change: a share
of damage *taken* rather than of damage *absorbed*.

---

## 4. Tests that pass for reasons unrelated to their subject

Two found so far, both only exposed by moving the ladder underneath them:

- `curses_in_combat::frost_slows_everything...` searched the whole ladder for a
  fight lasting six seconds, and its three-piece fixture died before the window
  closed on every rung. It was not measuring frost; it was measuring whether any
  rung was slow enough to keep the fixture alive.
- `slash_and_burn::it_reaches_a_real_fight` is documented as "a board wearing
  the spell, with enough nature banking to feed it" and its fixture was
  `apply_preset()`, whose twenty-one pieces carry **no searing at all**. It had
  never once watched the spell it is named after.

`fixtures.rs` exists for this and covers eleven rows. **A sweep of the whole
suite asking "does this fixture actually carry what its test is named for" is
worth doing**, and the sweep is the moment to do it, because a fixture that was
passing by accident fails loudly the first time its accident stops.

---

## 5. A static fixture models a build, not a run

The road's two tests wanted one board to grind rungs 2-9 *and* reach rung 21.
After the burner cluster no board is both: the weapon has to come off to grind,
and a weaponless board is stopped at 19. A player was never stuck with that
choice - they ground the shallow end and then bought a weapon.

The fixture now grinds, arms up, and walks. Every other test that walks many
rungs with a fixed board has the same latent problem, and it only shows when the
ladder moves.

---

## 6. Quota progress is not independent evidence

Giving the crown and the gift something to do satisfied helmet's
`the dearest third interacts` quota as a side effect - two of the helmet's
dearest pieces are those two trophies. The quota went from 2 to 0 without
anybody thinking about helmet density.

That is the ratchet working, but it means a quota reaching zero is not proof the
slot was designed at. Worth re-reading the density quotas at the end and asking
whether they say what they were meant to say.

---

## 7. Event tests were hanging on combat balance

`the_winning_board_can_walk_the_road_and_collect_on_it` and
`trundle_no_longer_costs_the_road` are about the road: that asking rather than
taking leaves a note, and that the note is honoured twelve rungs later. Both
were *fighting* those twelve rungs, so every catalogue edit that moved a
mid-ladder creature took them red for reasons that had nothing to do with roads.

The fixture behind them had already been re-blunted three times - weapon off,
then gloves off as well, then the setting dropped from Insane to Hard - each
time to keep one board simultaneously slow enough to open the road and strong
enough to finish the walk. Those are two requirements and they were being asked
of one board.

They walk the intervening rungs now (`play_or_walk`) rather than fighting them.
Whether a given board beats rung 20 on a given setting is a combat question and
`progression` answers it.

**Worth a sweep at the end:** any other test that walks many rungs to reach the
thing it is actually about. They are cheap to spot - a fixture, a `play` to a
deep rung, and an assertion about an event - and each one is a test that will
fail for an unrelated reason sooner or later.

---

## 8. Re-expressing a mechanic is not licence to drop the piece's gate

`Warded Sabatons` bought a mana shield. Warding off the clock instead of the
blow is the right translation into the feet's vocabulary - but it was written
as `OnActivate(ReduceCooldown)`, unconditional, where the original had paid
three mana for it.

Every creature wearing the piece got that tempo for free, and the ladder felt it
at once: a board that had cleared to rung 22 on the hardest setting stopped at
20. It keeps its gate now.

**The rule to hold for the rest of the sweep:** a piece has a shape as well as a
mechanic - a cost, a cooldown, a condition - and the shape is usually load
bearing even when the mechanic is in the wrong slot. Translate the verb, keep
the sentence.

---

## 10. Damage stats on a piece that is not a weapon do nothing

`combat.rs` decides in one line who swings: *"Weapons swing; everything else
just does its job."* A helmet, a chest, a pair of gloves or greaves activates,
runs its triggers, and never takes a blow. Its `physical_damage` and
`magic_damage` are read only inside the `is_weapon` branch, so on any other
slot they are decoration.

**Twenty-three components carry raw damage they can never land** - twelve
gloves, seven chest, three greaves, one helmet - and `rating.rs` prices every
point of it. That is a mispricing in the direction that matters: it flatters
exactly the slots the rewrite is trying to give a real job to, so a piece can
look like it earns its place and contribute nothing.

It also has a sharper consequence on the ladder. A themed creature holds two
slots, and four of the six themes hold no weapon at all. Such a creature's
entire offence is its triggers. Seven of them had none that dealt damage, and
so they stood on the road at Medium - the setting every figure in this project
is measured at - and did nothing whatsoever. `every_monster_can_actually_hurt_you`
did not notice because it read `simulate`, which is Easy, and Easy is the only
setting that steps a board *down*.

Both halves are fixed as far as the red went: the test now sweeps all four
settings, and six creatures each traded one draining ring for one that answers
a neighbour. Neither the twenty-three components nor the rating that flatters
them has been touched.


---

## 11. A watcher that answers what it watches for does not return

`Trigger::Watch { what: CurseApplied, then: Action::Curse { .. } }` counts a
curse landing and lands a curse. The second curse is a curse landing, so it
counts, so it lands another. One accessory was written that way during the
mechanic sweep and the whole test binary died with

    fatal runtime error: stack overflow, aborting

in two files that had nothing to do with the piece. No catalogue test could see
it: the shape rules read a `PieceDef`, and this is a property of two triggers
meeting at runtime.

`notify_curse_watchers` no longer re-enters. The piece was fixed as well - a
grudge pays in a blow now - but the guard is the part that matters, because the
next author to write that trigger pair should get a piece that works rather
than a crash three files away.

The general shape is worth holding on to: **`Watch` is the one trigger whose
payload can produce the event it counts.** Activations cannot, because a
reaction never emits one; curses can, and so could any future `Watched` variant
whose event an `Action` is able to cause.


---

## 12. Over half of every shelf is the weapon

Measured across 400 seeded runs and six restocks each - 14,400 shelf slots:

| slot | share of shelves | share of catalogue |
|---|---:|---:|
| Weapon | **54.8%** | 36.7% |
| Gloves | 13.2% | 17.5% |
| Helmet | 11.6% | 17.1% |
| Chest | 10.2% | 14.7% |
| Greaves | 10.2% | 14.1% |

**Corrected, and the correction is most of the finding.** That measurement
forced `ensure_weapon` on every restock, and the real shop only asks for it when
the player has no assembled weapon at all - true at the start of a run and false
after. Measured the way a run actually restocks, the old shop was **36.6%**
weapon: exactly its share of the catalogue, because the pool was uniform over
the catalogue. The 54.8% was the repair, and the repair is rare.

So the shop was not over-representing the weapon. **The catalogue is.** The
weapon is two fifths of the pieces, and a uniform draw faithfully reproduces
that on every shelf a player ever sees.

This is the damage-share problem wearing different clothes. The shop is the one
surface where a player meets the catalogue, and on it the weapon is not 37% of
the game, it is 55%. A rewrite that gives four other slots a job has to put
their parts in front of somebody.

The shelves are dealt a slot at a time now, weighted by `SHELF_TILT` - the
weapon at 33.9% and the four armour slots at 14-18% each, where before they were
10-13%. It is a small move and it is bounded by a real tension: at a tilt of 1.0
each slot is dealt in proportion to its catalogue, every component is equally
likely, and the weapon takes the shelf; at 0.5 the shelf is even and a chest
piece is 2.5x as likely as a weapon piece, which
`avail::the_shelves_are_not_the_same_six_things_every_time` refuses at 3.7x.
0.9 is where both hold.

The larger version of this is not a shop change at all. If the weapon should be
a fifth of what a player meets rather than two fifths, that is 100 pieces the
other four slots do not have, and no amount of dealing fixes it.


---

## 9. What is worth checking at the end

Running list, so the last mile is not guesswork.

- Every test that walks many rungs to reach something that is not about combat
  (see 7). Cheap to spot: a fixture, a `play` to a deep rung, an assertion about
  an event.
- Every fixture that might be passing for a reason unrelated to its subject
  (see 4). `fixtures.rs` covers eleven; the suite is larger than that.
- **`fixtures.rs`'s own rows.** Two of them bound pieces to
  `curses_in_combat::a_stunning_caster` as banking empowerment, and that test
  does not mention empowerment anywhere - the fixture happened to carry some and
  somebody recorded it. When the sweep took it away the manifest failed and the
  test did not. The manifest is a good idea that can be wrong in the same way
  the tests it guards can be.
- The density quotas, re-read as design rather than as arithmetic (see 6).
- `monster_value` (see 1), which is a patch and should be replaced by a scoring
  function written for creatures rather than borrowed from the shop.
- Reflection's trigger (see 3), if the body is ever meant to matter on the clock
  as well as in health.
- **Damage stats outside the weapon (see 10).** Twenty-three components price
  damage they cannot deal. Either the stat should reach the slot or the rating
  should stop paying for it, and the first is a combat change.
- **Watchers that can feed themselves (see 11).** Guarded for curses; any new
  `Watched` variant needs the same question asked of it before it ships.
- **The catalogue's slot mix (see 12).** The shop is dealt fairly now; the weapon is still two fifths of the pieces that exist.
