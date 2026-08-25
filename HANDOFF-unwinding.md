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
| M2 | Insight and Dread, dark (A3) | next |
| M3 | The road stack, receipts, tooltips (A7/A9/A6) | |
| M4 | Road machinery, landed inert (A4) | |
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
