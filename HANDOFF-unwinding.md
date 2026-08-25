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
| M4 | Road machinery, landed inert (A4) | next |
| M5 | Frames and the four new themes (H4) | |
| M6 | Dungeon presentation, pedestals, route map (A8/A10/G6) | |
| M7 | Relics, crushables and consignment (H1) | |
| M8 | Phase-1 gate, and the stretch decision | |
| M9 | The catalogue lands once | |
| M10 | The chain (Part B) | |
| M11 | The dungeons and what they pay | |
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
