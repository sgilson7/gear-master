# Ledger — The Unwinding

The running record of the mission in `design/the-unwinding.md`. Written for an
agent starting with no context: read `CLAUDE.md` first, then this, then the
spec's two reconciliation blocks. `HANDOFF.md` is the previous mission's record
and is finished; §5 and §7 of it are still house law.

**Rewritten into `HANDOFF.md` at M19.** Until then this file says which
milestone is open and what the last one moved.

## Where the code is

- **`main`** is published and live at `390e425 Publish web build`. GitHub Pages
  serves `docs/` from it.
- **`unwinding`** is the working branch, cut from `main` at `390e425`. `main`
  is touched **once, at the end**.
- Suite: **548 tests, green, no warnings**, 33 suites.

## The denominator

`analysis/baseline.md` § *Before the Unwinding*, captured 2026-08-25:

| | |
|---|---|
| Weapon damage share, owner at Medium | **75.2%** (band 66-76%) |
| Owner's board | 50/50, median TTK 10.50s, cadence 6.69/s |
| Friend / preset / starter | 48/50 @ 97.6% · 9/50 @ 100% · 2/50 @ 100% |
| Catalogue | 473 pieces, 104 inert (22.0%) |
| `the_catalog_keeps_every_rule` | green, 0 rules unmet |
| Casino corridor | sharp 1,600ms · plain 6,000ms · plain worst 39,000ms |

## The four decisions, settled before the first commit

- **The stretch (Engraving + the Brain Farm) is decided at the Phase-1 gate,
  M8**, with the cost of reopening `share.rs`'s format measured rather than
  guessed. They go together or slip together and nothing depends on them.
- **Gold is denominated in bounties**, 1x/3x/10x, resolved at resolution time -
  the idiom `Outcome::BuyOff { times }` already uses. Spec table at
  RECONCILIATION II #16.
- **THE UNWOUND is authored to 16-29s at Medium**: the curve's band at rung 51
  with its top edge clipped clear of sudden death. RECONCILIATION II #17.
- **Mind damage is answered by `mind_resist` alone.** A1 takes the shield off
  physical *and* mind; the three lanes get one answer each. RECONCILIATION II
  #18.

## The milestones

Twenty, in four phases plus a phase zero. Each ends green, with its numbers in
`analysis/baseline.md`. The two ordering rules are absolute: **all engine work
before any content**, and **no authored board before Phase 4, with rating
re-pinned first**.

| | Milestone | State |
|---|---|---|
| M0 | The ground, written down | **done** |
| M1 | Typed lanes and the twins (A1+A2) - merges alone | **done** |
| M2 | Insight and Dread, dark (A3) | **done** |
| M3 | The road stack, receipts, tooltips (A7/A9/A6) | **done** |
| M4 | Road machinery, landed inert (A4) | **done** |
| M5 | Frames and the four new themes (H4) | **done** |
| M6 | Dungeon presentation, pedestals, route map (A8/A10/G6) | **done** |
| M7 | Relics, crushables and consignment (H1) | **done** |
| M8 | Phase-1 gate, and the stretch decision | **done** |
| M9 | The catalogue lands once | **done** |
| M10 | The chain (Part B) | **done** |
| M11 | The dungeons and what they pay | next |
| M12 | Extra Large and the Orbs of Travel (Part G) | |
| M13 | The five unconditional events (Part F) | |
| M14 | The nine structures and the three pairs (H2/D) | |
| M15 | The words (Phase 3) | |
| M16 | Rating re-pinned first (Phase 4) | |
| M17 | Every frame gets its board, in `make pack` | |
| M18 | Reference builds and the acceptance sweep | |
| M19 | Final verification, then one merge | |

---

## M0 - The ground, written down

No code. What it found, and what it wrote down.

**The claims checked out, with drift.** The suite is **548** green rather than
the 538 `CLAUDE.md` claimed; the ratchet is green with zero rules unmet; the
weapon's share is 75.2% rather than 74.9%. All three moved in the two commits
after `HANDOFF.md` was written, which is drift rather than error, but it means
the numbers in that file are quotes and not measurements.

**Seven places where the code and the documents disagree**, all now dated
amendments in the spec (RECONCILIATION II #9-#15). The two that will actually
bite:

- **`LADDER` is fifty and `Rust Golem` is rung 4.** It is spliced in by name
  rather than written inline, so counting the table by eye - or with a script -
  comes back one short and every rung above three reads one low. Two separate
  passes at the bounty table got this wrong before the CLI settled it.
- **`at` is an index and the displayed rung is `at + 1`.** Every rung number in
  Parts B, D, F, G and H is a displayed number and must be converted once, on
  the way in.

**And one the spec does not name at all**, written up as #19: **appending to
`CATALOG` re-gears every monster on three settings exactly as a `rating.rs`
weight change does.** `stepped_component` sorts a footprint family by
`monster_value`, so a new piece sharing a kind, slot and shape with an existing
one inserts itself into that family and moves every stepped board wearing a
sibling. That is why every new component in the mission lands in one milestone
(M9) rather than in five content PRs.

**Numbers written.** The whole *Before the Unwinding* section of
`analysis/baseline.md` - the four-board table, the catalog census, criterion 2
and 3, the mind-damage table, the ratchet in full, and the casino corridor,
which is the constraint M1 is about to push on.


---

## M1 - Typed lanes and the twins

Two commits. The first is the only one in the mission allowed to move the
ladder; the second moved nothing at all.

**A1.** Empowerment multiplies magic-typed hits only; the mana shield reduces
magic-typed damage only; physical skips both, and so does mind. Mind is
answered by `mind_resist` and nothing else (RECONCILIATION II #18) - the shield
used to blunt it too, which made mana the answer to three lanes out of three.

**A2.** `GainSpellblade` (+0.50x power a stack, flat, physical) and
`GainDeflection` (-10 a hit, flat, physical, ahead of armour). Both constants
are set where a twin stack equals its mana cousin at ten mana, which is where
the two bargains cross.

**What it cost.** The shallow ladder is byte-identical - rungs 1-14, all four
boards - and the casino corridor is unmoved. The whole cost is two rungs on the
owner's board (50/50 → 48/50, losing Nine of Ashes and Francis), which was
taking a magic multiplier onto iron. Weapon share 75.2% → 75.5%. The friend's
mind damage 595 → 707.

**The three things that will bite the next sweep**, all learned by tripping
over them here and all written up in `analysis/baseline.md`:

1. **Never convert a reaction that deals a blow.** Four themes of six hold no
   weapon, so a glove's answer is the whole of some creatures' offence, and a
   Spellblade stack multiplies a swing that is not there.
2. **A carrier must be free on four boards, not one**: every monster's `gear`,
   all three share codes, *and* `apply_preset`. Missing the second cost a
   rumour its door; missing the third moved the preset three rungs.
3. **Deflection's greaves share sits on a mold, not a plating.** A Plating
   floats into the helmet's grid and a floating kind carries no identity
   mechanic. The spec is amended.

**New in the harness.** `tests/typed_lanes.rs` (8 tests), `report_early_ladder`
in `baseline.rs`, and census rows for both twins. `Combatant::take_typed` and
`take_mind` are public now, because the lanes are a rule rather than an
implementation detail and there is no other way to put one number in and read
what each lane did to it.

Suite: **556 green**, 0 warnings.


---

## M2 - Insight and Dread, dark

One commit, and the ladder did not move by a byte.

`Resource::Insight` is the eighth, and it is **fuel rather than a holding**.
That is the part of A3 worth reading twice: "what mana empowerment is to magic"
means Insight pays nothing at all while held, exactly as mana does, because
what it is worth is decided by the Dread standing on it. `held_bonus` therefore
gets no Insight arm, and `insight.rs` pins that so a later passive rate has to
come through the test and argue for itself.

`Action::GainDread(u32)`; mind damage gains `dread x insight / DREAD_DIVISOR`,
picked up off the swinger on the way out the way empowerment is, and wired into
**both** routes to mind damage - a piece's `mind` stat and `Action::MindDamage`
- because a bonus that reached one of them would be a lane with a hole in it.

`Run::banked_all_run` was `[i32; 4]` against an index that already ran to six.
No live panic, because a fusion has an event of its own; widened to eight
before something wrote past the end rather than after.

The gate is `Shop::insight_open`, set by `Run::unlock_insight` in the same
call as the run's flag. `piece::touches_insight` is the predicate.
`insight.rs::the_catalogue_carries_no_insight_yet` is a lint on the *phase*: it
fails in the commit that authors the family (M9), and the fix is to delete it.

Suite: **569 green**, 0 warnings.


---

## M3 - The road stack, receipts and tooltips

Two commits, and nothing in `combat.rs`. The harness is byte-identical to M2.

**A7, derived rather than stored.** `Run::road_stack()` is a function over run
state, not a `Vec` field pushed and popped. Every entry is already decided by
something that exists - `dungeon`, `town`, `at_fountain`, `answered`, `brawl` -
and a stored copy would be a second source of truth for a question five fields
already answer. Two of this project's bugs were exactly that shape. Derived,
"resolving an interrupt may push more" and "a dungeon exit resumes the pop
where it left off" both need no code at all.

**Amended: the pop order is the gate, then the fountain, then the events.** The
spec asks for fountain first. The two genuinely collide - `FOUNTAINS` is 7 and
14 and Sump Bottom's gate stands at rung 7 - and the shipped road reads the
gate first in both places it decides. E6 criterion 2 settles it.

**A6 and A9.** `Requirement::describe`, `Outcome::describe`,
`TownVisit::receipt` and `rumour::conditions_line` all live in the engine and
return canonical prose, so the CLI prints the sentences the interface draws and
the theme layer swaps the nouns in both. `unmet` did not go anywhere: it is
flavour for the moment *after* a door refuses you, and `describe` is the plain
statement before you try one. Both ship.

`Run::last_receipt` / `take_receipt` is read once and dismissed, and the panel
that shows it blocks the next pop of the stack.

**The CLI can now walk the road.** It could equip and fight and nothing else,
so a scripted run went straight past every event, town and fountain in the
game - which makes "two replays produce identical logs" (E6.1) a claim about a
road nobody was on. `road`, `answer <n>`, `town`, `town on`, `town <door>` and
`drink`.

**And a test that was passing for the wrong reason.**
`a_town_gate_blocks_the_road_even_mid_replay` was satisfied by the fountain
that shares rung seven with the gate. It names what it is looking for now.

Suite: **589 green**, 0 warnings. New: `road_stack.rs` (8), `tooltips.rs` (11).


---

## M4 - Road machinery, landed inert

Two commits, and the road is byte-identical to the one before them. Everything
here is reachable by nothing: Phase 2's job is to name it.

**Towns carry their own doors** and know whether they are on the map.
`Unlock::{Pinned, Hidden}` and `actions: &[Action]`; the three shipped ones are
pinned with all four and unchanged. `apply_outcome` is split out of
`take_choice`, because a town door hands over an outcome too.

**Five new conditions.** `Flag` (a chain station), `Counter` (the watcher
pattern), `AssembledOfRarity`, `AlignedItems` (the inspector reads the live
board), and `Figure` - a door that wants a number, which `take_choice` refuses
and `take_choice_with` answers, because a default bid is a bid nobody made.

Flags are strings rather than a field per station, and that is the one place
this milestone argues with the spec. Named booleans are checked by the
compiler; a string is not. What a string buys is `event::set_by` - the reverse
index that makes "a chain with a station nothing reaches" one assertion. The
fault worth guarding against is not a typo.

**Eleven new outcomes**, all inert: Flag, Count, RevealTown, OpenShop,
StartDungeon, GrantRow, GrantQuest, ClaimTicket, StandingOrder, Underwrite,
Scout.

**A board can be taller than the board beside it now.** The Depth grants one
row on a slot of your choice, so `Loadout::grow_one` exists, `rows()` means
"the tallest" rather than "the height", `equip_locked_at` asks the slot instead
of the loadout - the third time that question has been asked of the wrong thing
- and the **share code goes to version 3** carrying five row counts. Version 2
codes still read.

**The crucible melts.** `Run::melt` trades a piece for a same-slot piece within
fifteen rating, out of the run's own PRNG, never combat. Quest pieces and
rumours refuse the pot. Every melt is counted whether or not it worked, because
the foundry is counting visits.

**Rung 51 is plumbed and shut.** `past_the_top` wants the Mainspring and a
cleared ladder; `ladder_complete` steps aside for it; share codes already
accept the rung.

Suite: **621 green**, 0 warnings. New: `hidden_towns.rs` (6),
`road_machinery.rs` (23).


---

## M5 - Frames, and the four new themes

One commit. Ladder byte-identical.

**`crates/engine/src/bestiary.rs`** is the theme table, moved out of
`tests/pack_francis.rs`. It was test-local for as long as the only thing that
needed it was the search that authors boards; a `MonsterFrame` carries a theme
and a frame is engine data, so it came home, and the packer and the interface
read one of it now.

**Hollow, Swarm, Beast and Warden**, all four standing beside the road rather
than on it - `theme_for` is unchanged and a test says the four are not on it.
Two amendments to `design/monster-themes.md`:

- "Every slot appears in exactly two themes" was a property of six and not a
  rule. Swarm and Slower fill the same pair of grids and are not the same
  creature, because a theme is a pair of grids **and** a vocabulary.
- **Hollow needs no weapon**, which is the difference between it and the Wall.
  Mind damage is the helmet's, so it can already reach you through a grid it
  fills. What it cannot do is appear in a damage share.

**`MonsterFrame` and the frame lint.** `FRAMES` is empty, so
`no_frame_ships_without_a_board` is green today; it goes red on the first frame
Phase 2 declares and green again only when the last board is authored in Phase
4. Debug builds shout UNPACKED over a creature standing on the road with
nothing on.

One thing worth knowing: `pack_francis::pack` - the `#[ignore]`d generator, not
a test - now refuses Francis, because M1 took the reference board's magic
multiplier off its iron and it no longer beats him at Medium. The generator
refusing is the generator working; Francis keeps his hand-authored board either
way.

Suite: **630 green**, 0 warnings. New: 9 tests in `bestiary.rs`.


---

## M6 - Presentation, pedestals and the route map

One commit. Ladder byte-identical.

**A8: you always know you are inside one.** A dungeon carries `entry` lines
now, played on the machinery the bosses use the moment you step through - a
different thing from `blurb`, which is read at the door while it is still a
decision. Inside, the screen is edged in violet, and the boards say
"THE CREVICE IN THE ROCK - FLOOR 2 OF 3" with pips, in place of a rung line
that would otherwise have said the same number for three fights in a row.

**A10: `route::route(run)` is a pure function of the tables plus the run.**
Nodes come from `LADDER`, `TOWNS`, `EVENTS` and `DUNGEONS`; fill comes from the
run; names are canonical. Nine tests, one per grammar rule and two on the
shape. `route::ascii` is the same map for the headless driver, on `map` - which
is the point of putting it in the engine, because two renderings of one
function cannot disagree about which road the game has.

**G6: the pedestal**, with an empty destination table and all three rules
already true - an orb is a piece first, a destination fires once a run across
both pedestals, and an orbless run sees furniture rather than an error.

One mechanism worth naming: **`Run::forced_event`**. Every other event is found
by rung; this is for the ones pushed onto the stack from somewhere that is not
a rung at all - a pedestal, and later THE FORK.

Suite: **647 green**, 0 warnings. New: `route.rs` (9), `pedestal.rs` (5), one
in `dungeon.rs`.


---

## M7 - Relics, crushables and consignment

One commit. Ladder byte-identical. `relic.rs` is the reward vocabulary that is
not gear.

**A run-relic** is worth what the run has done - the only piece in the game
whose card is different at rung forty from what it was at rung four. It pays
from a **board** rather than from the tray, because a reward that pays from a
pocket has no decision in it.

One thing had to give. `Relic::pays` returns a `Payout` rather than a `Stats`,
because **speed is not a `Stats` field** and never has been: every speed in
this game is a percentage on an item's cooldown. So the Odometer's payout is
carried separately and applied to the profiles in `Run::combat_items`, which is
where the other speeds already live.

**A crushable** is spent, which nothing else in this game is. The Second Key is
the only legal breach of the one-action rule, and it is legal in exactly one
place - `visit_town` reads `second_key_ready` once, which is what keeps the
exception to one. All three refuse before destroying themselves if they cannot
do their one thing.

**The Lightning Rod** redirects any curse that picks a target on your board.
Only one does - a stun - and the other three land on the fighter and always
have; the spec is amended to say so. It asks for *covering* rather than the
bond's *burying*, so a rod half under something still has a wire running into
it. That makes it a decision: lay it under something you do not mind losing the
use of.

**Consignment** and `Run::restock`. Nine call sites turned the shelves over;
they turn them over in one place now, because a shelf has two jobs at a restock
and the second one is exactly what gets added to eight of nine.

Suite: **666 green**, 0 warnings. New: `relics.rs` (16), 3 in `relic.rs`.


---

## M8 - Phase 1 closed, and the stretch slips

No code. Four printers re-run, one decision taken.

**The ladder has not moved since M1.** Byte-identical across M2 to M7: four
boards, fifty rungs, every census row. The ratchet is green, the casino
corridor is where M0 left it, and a twenty-one line scripted CLI run replays to
1,032 identical lines - E6.1 in the form the road can currently take.

**666 green, 42 suites, 0 warnings.**

**The stretch slips**, and the reason is one measurement.

Engraving needs a piece *instance* to differ from its *definition*.
`PieceRegistry` could carry that cheaply - it already carries a rotation and a
`transform` - and combat would be correct for free. Nothing else would be:
`rating::piece_rating` is `fn(&PieceDef) -> i32`, and the shop's price,
`Rarity::of`, the naming layer, `stepped_component` and all twenty-six of
`catalog_shape`'s rules are built on that signature. An engraved piece would
fight correctly and be priced, named and rated as the piece it used to be.

Fixing *that* is a rating over instances rather than definitions, threaded
through five modules and a ratchet - which is `second-order.md` §1's "two
questions answered by one number" wearing different clothes, and a mission
rather than a milestone. The Brain Farm's only prize is Engraving, so the tie
E1.8 draws holds and both slip. Written into the spec as amendment #20,
including what would unblock it, so the decision does not have to be retaken
from scratch.

**One correction to the plan.** It said the frame lint would be red at the
Phase-1 gate. `FRAMES` is empty until Phase 2 declares one, and a lint over an
empty list cannot fail. It goes red on the first frame and green at M17, which
is what E6.8 actually asks for.


---

## M9 - The catalogue lands once

Thirty-one components in one commit, and the four-board table at Medium does
not move by a figure. 473 pieces to **504**.

Four Orbs of Travel, nine one-cell things the road hands over, ten pieces of
mind-lane gear, six more words, a chest base and an enchantment. Every reward
is `EVENT_ONLY`; the rod is an enchantment, so `is_town_stock` keeps it off the
road's shelves without anybody listing it.

**The trap the spec does not name, walked into and measured.** Appending to
`CATALOG` re-gears creatures on Easy, Hard and Insane exactly as a `rating.rs`
weight does, and it moved **29 of 162** stepped boards - into the astronomer's
lens, the stranger's parcel and two pieces of gear that bank a pool the run has
not been given.

`stepped_component` already filtered boss gear and quest rewards, and both
filters were added after something went wrong. The list should always have been
four long. With `is_event_only` and `touches_insight` in it, the 29 falls to
**11** - and all eleven are the *old* leak closing: `Gold Chip` and
`Crownwright's Measure` have been on monster boards at Easy and Hard since
before this mission, and are ordinary gear now.

**Two pieces the ratchet argued with and won.** The Cracked Lens at 20 mind
out-rated boss gear; it is 12. Bearhide's "Gain Fury on battle start" is two
other slots' words - `OnBattleStart` is the feet's, banking rage is the head's
- and put chest's bleed a third of a point over its band; the fury is strength
and the verb is armour.

**`GainDread` is conversion**, beside `GainSpellblade`: a stack that doubles a
word counts as the word. That is what brought helmet's bleed back into band
after fifteen new helmet pieces.

And the promise that every component can be met moved rather than lapsed, the
way it did for town gear: `avail.rs` has a second test now, and the mind lane
is reachable the moment `Shop::insight_open` is.

Suite: **668 green**, 0 warnings, ratchet green.


---

## M10 - The chain

One commit, and the ladder does not move. Four stations, two hidden towns, a
dungeon, three words and five frames.

**The chain's state is the words you are carrying.** A5 lists three flags that
the run already knew - a word in the tray, and `towns_revealed` - and a second
copy of a fact is a second thing to keep true. Only `threshold-cleared` is a
flag, because a dungeon walked is the one station that leaves nothing to look
at.

**Every station fails forward** and there is a test that says so from the only
angle that proves it: after refusing each door, the thing that opens the next
one is still gettable. Turning the astronomer in ends *that* road to the cellar
word and the Slagworks foreman is the other, which is why the chain has two
roads to its own middle.

Four amendments, all written into the spec as #21, and all corrections rather
than changes of mind. Rumour doors stand in **windows** now, because a door
priced in a rumour is a door you might arrive at holding nothing.
`Outcome::Defer` is the only outcome that does not close the door it was
offered at - "walk on, and the gate finds you again" needed one. The Wrong
Stars is sold at the **pub**, because a chain whose first step is luck is a
chain most runs never see the shape of. And the Slagworks stands after rung
**33**: the body says "one clear of High Wick at 31" and then says 32, which is
next to it.

**One real bug.** `take_choice` never checked that the choice belonged to the
door in front of you. One door on one rung made that safe; two open at once
does not, and the first fixture holding every word in the game answered a
locked gate with the VIP area's rescue button.

**`ClassPower::WrongSense`** is new, and it exists because the class the
antechamber hands out was only a stat bundle and a test refuses those. The
third lane had an amplifier, a pool and an answer, and no way through the
answer - which the other two have had since typed damage landed. It has
piercing now.

The **frame lint is red**, as a ratchet: five undressed creatures, a budget
that can only go down, and an `#[ignore]`d target at zero.

Suite: **683 green**, 0 warnings.
