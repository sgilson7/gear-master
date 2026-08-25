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
