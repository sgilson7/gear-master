# Handoff — The Unwinding

Written for an agent starting with no context. Read `CLAUDE.md` first, then
this. The spec is `design/the-unwinding.md` and it carries **three** dated
reconciliation blocks plus amendments numbered to 23 — read them where they
sit rather than trusting the body above each one. The milestone-by-milestone
record is `HANDOFF-unwinding.md`; this is the summary of it.

## 1. Where the code is

- **`main`** is published and live. GitHub Pages serves `docs/` from `main`.
- **`unwinding`** was the working branch. Twenty milestones, M0 to M19, merged
  once at the end.
- Suite: **764 tests, green, no warnings.** `cargo test -p gearmaster-engine`.

## 2. What the mission was for

The gear-slot rewrite gave five slots five identities and left the game with
nothing to walk the road *for*: fifty rungs, nine events, three towns, one
dungeon, and an ending at Francis that nothing pointed towards.

The Unwinding is the content answer. An event chain across the back half of the
ladder ending in a super boss at rung 51 that only a finished chain and a beaten
Francis open; three hidden towns; five more dungeons; four Orbs of Travel and
the places they go; a third combat lane; a reward vocabulary the road can pay
out in; and a route map drawn from run state.

## 3. What shipped

| Part | What |
|---|---|
| **A1-A2** | Empowerment and mana shield are magic-only. **Spellblade** and **Deflection** are their physical twins - flat, unscaled by mana, reset per fight |
| **A3** | **Insight**, an eighth pool, locked behind a dungeon, and **Dread**, the stack it multiplies. Mind is the third lane and `mind_resist` answers it alone |
| **A4-A10** | The road stack, receipts that describe themselves from the engine, two tooltips, dungeon presentation, the route map, pedestals |
| **B** | The chain: four stations, three words, two hidden towns, a mini-boss party, and rung 51 |
| **C, F6, G5, H5** | The turtle theme's own telling, in `theme.rs` and nowhere else |
| **D** | Three standalone rumour/event pairs and the classes they pay |
| **F** | Five events that always happen, so the road is never bare |
| **G** | Extra Large, its five doors, and the four destinations |
| **H** | Twelve payables, nine structures, fifteen frames, four new monster themes |

## 4. The five things that cost the most

**Every lint over `EVENTS` stopped at the top of an outcome.** Half this
mission's bargains are an `Outcome::All`, and `class::is_earned`,
`event::set_by` and the reachability lint all matched on `c.outcome` directly -
so a class claimed inside one read as a class no door hands out, and a fountain
could have poured it. `event::every_outcome` unpacks `All` and `Gamble` and
everything asks through it now.

**`take_choice` compared choices by address.** `EVENTS` is a static holding
promoted arrays and a caller in another crate holds a reference to a *copy*, so
the ownership guard passed in the engine, passed in the GUI, and silently
refused every choice made from a test binary. Not a wrong answer - a silent no.
It compares by value.

**The canonical game had been speaking turtle for eleven milestones.** Fourteen
scenes named people who exist only in a theme, and one leak was in `combat.rs`'s
own log line. The rule now: a common noun is fixed in place and `vocabulary`
puts the themed word back; a proper noun moves, canonical keeps the *role*, and
`theme.rs` gets the scene. `tests/two_voices.rs` is the ratchet.

**A creature that did not exist.** THE UNWOUND - rung 51's boss, the thing the
whole mission points at - was a label on the route map, a theme entry and a
`past_the_top()` that could never return true, through four content milestones.
Nothing caught it because nothing asks a route label to name a creature.

**`cursed_for_good` was a list nothing read.** Documented since M12 as pieces
carrying a curse for the rest of the run; the library set it, `Uncurse` popped
it, and no fight was any different for either.

## 5. Habits that paid, and are house law

- **Land primitives inert, arm them separately.** Every Phase-1 milestone
  shipped with the ladder byte-identical. It is why they shipped at all.
- **Pin the weights before authoring anything against them.**
  `stepped_component` re-gears every monster on three settings when a weight
  moves - 33 boards on Easy this time.
- **A guard that refuses your change is usually right, and its refusal is a
  gradient.** The packer says "wanted 11.8s, best was 8.0s", which is a number
  to scale by rather than a wall. Four of the six creatures it refused landed
  in one step from that ratio; the other two needed the *other* dial.
- **Re-pin with the reason in the assertion**, not in a commit nobody will read
  again.
- **Iterate with `--lib` or one `--test`.** Full suite once per milestone. The
  engine has 46 test binaries and every edit relinks all of them.

## 6. What is not done

- **The boards are the generator's, not a person's.** All fifteen frames were
  packed by `tests/pack_francis.rs` at the rung they are met on. They are
  samples, sized correctly and shaped by theme, and the owner is rebuilding
  them by hand.
- **Engraving and the Brain Farm slipped**, at the Phase-1 gate, on measured
  cost. Amendment #20 records what would unblock Engraving: it is the only
  thing in the mission that reopens `share.rs`'s index-keyed format.
- **The fourth reference build does not beat THE UNWOUND.** It assembles into
  five items and is written at the mind lane; two of the three shipped boards
  lose to the boss and the third wins at 28s, which is the criterion. A board
  that wins *because* of Deflection and Insight is still to be built.
- **Nobody has played this.** Every claim here comes from the suite and from
  two CLI replays that diff clean.
