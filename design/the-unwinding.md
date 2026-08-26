# The Unwinding — event chain, hidden roads, and the thing after Francis
## Execution spec for Claude Code (Opus)

Companion to `gear-slot-basis-rewrite.md` (largely executed) and successor to
it in sequence. Read `design/towns.md`, `design/branching-events.md`, and
`design/monster-themes.md` first — this document is written in their language
and extends their tables. Like them: **code follows this document; when they
disagree, this is the bug report.**

**What the repo already has that this spec builds on** (verified): the events
framework (`event.rs` — events stand in front of rungs, ask questions, never
resolve themselves; the casino is built), two rumours (`rumour.rs`, both named
"A Word About the …"), one dungeon (`dungeon.rs`, "THE CREVICE IN THE ROCK"),
three pinned towns with the one-action rule but **no per-town action list in
the `Town` struct yet**, `simulate_party` (multi-enemy fights, duel-preserving
— `tests/brawl.rs`), monster themes + the density curve + the theme-locked
packer, typed physical/magic damage with resist/pierce/harden, `reflect` on
the Wall theme, underlay as a piece kind, Druidic Might fusion, and
`catalog_shape.rs` enforcing slot identity. The gaps table in
`design/branching-events.md` (conditional triggers, item conditions, `Give`,
curated shops) is closed *by this spec* — those gaps are milestones here.

**Structure of this document.** Part A: mechanics (typed empowerment, the
twins, the new pool). Part B: the chain, in the base game's own voice — **this
is canon; every engine string comes from Part B.** Part C: the same chain as
the turtle theme tells it, with book citations — `theme.rs` entries only.
Part D: standalone rumour/event pairs. Part E: the execution plan.

---

# RECONCILIATION — 2026-08-25 (wins over the body where they disagree)

Written after the gear-slot rewrite finished and deployed. The body below
predates it; these deltas are the current truth.

1. **"Underlay" shipped as `PieceKind::Enchantment`** — a second layer in
   `Slot`, laid under the grid. Every "underlay" below reads *enchantment*.
   Enchantments are **town stock only** ("ground is bought where somebody
   has a floor to sell"), which lands this spec's two enchantment rewards —
   the Lightning Rod at the Slagworks mold line, Aisle 9's shelf — exactly
   where the law already puts them.
2. **`Resource` is 7** — all three fusions (Druidic Might, Communion,
   Zealotry) are live. **Insight is the 8th**, settling A3's verify-first
   note. Audit every exhaustive match, `Resource::ALL`, naming words, GUI
   chips, `Drain`, and `held_bonus` (creature pools are priced at what
   `held_bonus` converts them to — give Insight a rate there or monsters
   holding it are free).
3. **Phase 4's instrument is `make pack`**, the hand-authoring board tool —
   the automated repack was halted (`analysis/second-order.md`); frames
   stand as specified, and the empty `CREVICE` plus `ALTERNATES` are the
   in-repo precedent for creatures awaiting boards. Phase-2 scaffold boards
   for tests remain generator-made and lint-marked.
4. **Phase 4's order flips: rating re-pins come first, boards second.**
   `stepped_component` (`combat.rs:252`) re-gears every monster on Easy,
   Hard and Insane whenever a `rating.rs` weight changes — and this spec
   adds several weights. Pin the weights, then author boards against the
   settled curve. (E4 below is amended in place.)
5. **Sudden death bounds rung 51.** The clock takes over at 30s and the
   band's top edge at rung 50 is 29.1s. THE UNWOUND must be authored to
   finish inside the measurable region — its target is a TTK band, not a
   feeling. "Harder than Francis" is measured at **Medium**; the open
   Francis-on-Hard question (`HANDOFF.md`) stays uncoupled from this spec.
6. **There is no milestone pricing.** E6's number anchoring reads against
   real `SHELF_TILT` shelves and rung bounties. `StandingOrder` outcomes
   must define their behavior against slot-at-a-time shelf dealing.
7. **The reconstruction fault applies here** (`HANDOFF.md` §5): any code
   that rebuilds a board — the Claim Ticket's whole-board drop, the
   pedestal's returns, shared boards — goes through `common::board_from`,
   locking items as they assemble, from the first commit.
8. **Reference builds are authored presets** in the `apply_preset` mould —
   there is no auto-builder. The three Francis-beating builds and the
   Deflection-and-Insight build in E6.5 are presets to write, and they
   double as baseline fixtures.

---

# RECONCILIATION II — 2026-08-25, execution (wins over the body and over the block above)

Written at the start of execution, after running the suite and reading the
modules this spec names. The block above was written from the gear-slot
rewrite's finish line; these are the places where the *code* and this document
disagree, plus the four decisions the reconciliation left open. Numbered from
9 so the two blocks read as one list.

9. **`LADDER` is fifty and `Rust Golem` is rung 4.** It is spliced into the
   table by name (`pub const RUST_GOLEM`, `combat.rs:646`) rather than written
   inline, so every text search of the ladder comes back one short and every
   rung above three reads one low. Bounties by displayed rung: 4 -> 10g,
   11 -> 34g, 16 -> 93g, 27 -> 188g, 33 -> 233g, 50 -> 500g (Francis).

10. **`LadderEvent::at` and `Town::after` are zero-based indices. The displayed
    rung is `at + 1`.** `the-casino` is `at: 8` and its own comment calls it
    rung 9. Every rung number in Parts B, D, F, G and H is written in displayed
    numbers and must be converted once, deliberately, on the way into the
    tables. `every_event_stands_where_it_thinks_it_does` is the guard.

11. **Two of `branching-events.md`'s five gaps are already closed.**
    `Outcome::Give` and `Requirement::Holding` both ship and are in use.
    `Outcome::Stock` is also a curated shelf, though it *empties* the shop and
    restocks it rather than opening a one-visit one, so A4's `OpenShop` is a
    new outcome beside it and not a replacement for it.

12. **A7's pop order is amended to the order the game already resolves in:
    town gate, then fountain, then events by `EVENTS` order.** The body says
    "fountain, then gate, then events". The code puts the gate first in both
    places it decides (`run.rs::road_is_blocked`, and the GUI's screen
    dispatch), and `pending_event` returns `None` at a fountain. The two
    genuinely collide - `Run::FOUNTAINS` is `&[7, 14]` and Sump Bottom stands
    at rung index 7 - so this is not a hypothetical. E6 criterion 2 requires
    the three shipped towns' tests to pass unmodified, which settles it: the
    stack is a data structure for the order the road already has, not a new
    order.

13. **`MonsterTheme` does not exist in the engine.** The six themes are a
    test-local `enum Theme` in `tests/pack_francis.rs`. `MonsterFrame` is
    specified as engine data carrying a theme, so the table is promoted into
    the engine and the packer and the GUI read it from there. The four new
    themes (Hollow, Swarm, Beast, Warden) arrive in the promoted table.

14. **`Run::banked_all_run` is `[i32; 4]` and `Resource::index()` already
    returns up to 6.** No live panic today - a fusion emits `Event::Fused`
    rather than `Event::GainResource`, so the out-of-range indices are never
    written - but Insight is index 7 and the array is grown before the pool is
    added, not after.

15. **`Nine of Ashes` is rung 47, not 46** (Part C's note on the theme already
    naming a creature "Nibbalonius the Wise"), and **the packer can address
    `ALTERNATES`**: `gui/src/pack.rs::everyone()` chains them onto the ladder.
    `HANDOFF.md` says it cannot; that is stale.

## 16. Gold: every figure is a multiple of the standing rung's bounty

There is no milestone table and the body's absolute figures were written
against one. Measured against the real economy they are not merely wrong,
they are wrong by an order of magnitude at one end and correct at the other:
starting gold is 28g, and a run has earned about 61g by rung 4, 223g by rung
11, 604g by rung 16 and 2,177g by rung 27. So F1's 150g at rung 4 is two and a
half times everything the run has ever seen, while the Slagworks foreman's
250g at rung 33 is one bounty and needs no change at all.

**Every gold constant in this document is replaced by a multiple of the bounty
of the rung the thing stands on**, resolved at the moment it resolves - which
is the idiom `Outcome::BuyOff { times }` already uses (`run.rs:622`,
`LADDER[rung].bounty * times`). Three tiers:

| tier | multiple | what it is |
|---|---:|---|
| small | 1x | a toll, a bribe, a consolation |
| medium | 3x | a real purchase, a real payout |
| large | 10x | the big-ticket item, the jackpot |

| Where | Body | Tier | At its earliest rung |
|---|---|---|---|
| F1, leave the parcel on the milestone | 150g | medium | ~30g at rung 4 |
| F2 THE TELLER, the short version | 750g | large | ~340g at rung 11 |
| F3 THE DISPENSER, one coin | 100g | small | ~90g at rung 16 |
| F3 THE DISPENSER, the red one | 1,000g | large | ~930g at rung 16 |
| THE ASTRONOMER, buy the lens | 400g | medium | ~350g at rung 18 |
| G1 THE BIGGER SIGN, forget you saw it | 200g | medium | ~225g at rung 13 |
| F5 THE BIRD PROBLEM, pay the toll | 300g | small | ~190g at rung 27 |
| The Slagworks foreman, if the Manse is found | 250g | small | ~250g at rung 33 |
| MOLE TOWN, trade a curse off a piece | 400g | medium | by where you entered |
| THE THRUMBUS RACE, back a runner | 300g | medium | by where you entered |
| THE PAYOUT | 400g | medium | by where you entered |

The last three stand at destinations a run can reach at two very different
depths - Extra Large opens after rung 13 and High Wick after rung 31, and both
hold a pedestal - so their figures are the multiple and nothing else. That is
the rule's real payoff: **a price expressed in bounties is worth the same thing
wherever the road is when you meet it.**

Untouched: the Slagworks tempering ("half a rung's bounty") and the Manse
gallery ("sell at double") were already relative; THE BUYER's prices are
seeded; the casino and the VIP area belong to the last mission.

## 17. THE UNWOUND's target is 16-29s at Medium

The density curve gives `target(51) = 2.8 + 0.4 x 51 = 23.2s` and a +/-30% band
of 16.2-30.2s. Sudden death takes the fight over at 30s, so **the band's top
edge is clipped at 29s** - the same rule the whole ladder is packed under, and
the reason the curve's slope is 0.4s a rung in the first place. Nothing about
rung 51 may sit where escalation decides it.

"Harder than Francis" is E6.5's replay test and not a number: Francis himself is
pinned only by "not walked through" (a victory must cost 15s or more), so there
is no Francis TTK to be a multiple of.

## 18. Mind damage is answered by `mind_resist` alone

A1 takes empowerment and the shield out of the physical lane. The shield also
blunts **mind** damage today (`combat.rs:3198`, "whatever the damage type"),
and A3 then arms mind damage with Insight and Dread - so as written this spec
removes the mind lane's only flat mitigation in the same change that amplifies
it, and never says what replaces it.

It replaces it with nothing, on purpose. Three lanes, three answers: the mana
shield answers magic, Deflection answers physical, and **`mind_resist` answers
mind**. It already exists, it is helmet-exclusive with 28 carriers, and it makes
the helmet the mind lane's defence as well as its offence - which is the same
shape the other two lanes have. A1 therefore reads: the shield reduces magic
only; physical and mind both skip it.

## 19. The catalogue lands once, and it lands like a rating change

Reconciliation #4 says a `rating.rs` weight change re-gears every monster on
three settings through `stepped_component`. **So does appending to `CATALOG`**,
and the body never says so. `stepped_component` sorts a piece's *footprint
family* - same kind, same slot, same cells - by `monster_value`, so a new piece
that shares a footprint with an existing one inserts itself into that family and
shifts every stepped board wearing a sibling.

Every new component in this spec therefore lands in **one** Phase-2 milestone
with one measured re-pin, rather than arriving in five content PRs with five
uncontrolled re-gearings. `CATALOG` is closed from that milestone until the
Phase-4 rating pin.

## 20. The stretch slips: Engraving and the Brain Farm, decided at the gate

E1.8 makes them one decision - "they land together or slip together" - and the
tie is real, because the Brain Farm's only prize *is* Engraving. Taken at the
Phase-1 gate with M1 to M7 green and the cost measured rather than guessed.

**They slip.** Not cancelled, and not because either is a bad idea. The
measurement:

1. **Engraving requires a piece instance to differ from its definition**, and
   nothing in this codebase is built for that. `PieceRegistry` could carry a
   per-instance trigger list cheaply - it already carries a rotation and a
   `transform` - and combat would be correct for free. Everything else would
   not be. `rating::piece_rating` is `fn(&PieceDef) -> i32`, and the shop's
   price, `Rarity::of`, the naming layer, `stepped_component` and every one of
   `catalog_shape`'s twenty-six rules are built on that signature. An engraved
   piece would fight correctly and be **priced, named and rated as the piece it
   used to be**.
2. Fixing that means a rating function over instances rather than definitions,
   threaded through five modules and a ratchet. That is the same problem
   `analysis/second-order.md` §1 records about `monster_value` - two questions
   answered by one number - and it is a mission rather than a milestone.
3. `share.rs` would take its **second** format bump of this mission, for a
   feature that moves one trigger once.
4. Nothing in Phases 2, 3 or 4 depends on either, which is what E1.8 says and
   what makes slipping them cost nothing but themselves.

**What would unblock it**, written down so the decision does not have to be
retaken from scratch: a rating that takes an instance, not a definition. Do
that first, for its own reasons, and Engraving becomes a small feature
afterwards.

The Brain Farm is cheap on its own - a deterministic three-by-three with one
seeded flaw square is perhaps two hundred lines - and it is held here only by
the tie. If it is ever wanted without Engraving it needs a prize, and that is a
design decision rather than an engineering one.

## 21. What the chain turned out to be, and five places the body was one out

Written while building Part B. Everything here is a correction to the body
rather than a change of mind about it.

1. **The chain's state is the words you are carrying**, not a record of flags.
   A5 lists `heard_the_astronomer`, `slagworks_revealed` and `manse_revealed`;
   all three are already visible in the run - the first as a word in your tray,
   the other two as `towns_revealed` - and a second copy of a fact is a second
   thing to keep true. Only `threshold_cleared` is a flag, because a dungeon
   walked is the one station that leaves nothing behind to look at.
2. **A Word About the Wrong Stars is sold at the pub.** The body puts it in the
   shop's rare pool or behind the casino's second door, and both are luck: a
   chain whose first step is luck is a chain most runs never see the shape of.
   It goes where every other word in this game is come by, and the two after it
   are handed over by the chain itself.
3. **Rumour doors stand in windows now.** `Trigger::Whispered` gained a `from`,
   because a door priced in a rumour is a door you might arrive at holding
   nothing, and one standing on exactly one rung is a door a run walks past for
   reasons that have nothing to do with the bet it made. The two shipped ones
   set `from` to their own rung and behave exactly as they did.
4. **"Walk on, and the gate finds you again" needed an outcome.**
   `Outcome::Defer { rungs }` is the only one in the game that does not close
   the door it was offered at: it takes the event back off `answered` and puts
   it on a list of things that will find you again. THE LOCKED GATE and THE
   SECOND SHADOW are the two that use it, and they are the two the body
   describes that way.
5. **The Slagworks stands after rung 33, not 32.** The body says "after rung 32
   ... one clear of High Wick at 31, so the two never share a stretch of road",
   and thirty-two is not one clear of thirty-one - the two gates would stand on
   consecutive rungs. The sentence is right and the number was one out.

And two smaller ones. **THE ASTRONOMER's window ends at rung 29** rather than
30, because the VIP area stands on 30 and a rung with two doors on it is a rung
where one of them is a surprise. **THE SECOND SHADOW asks only for the
antechamber**, not for both towns - requiring the Slagworks would mean a run
that ignored the ridge could never meet the Herald, which is a chain that fails
*backward*.

**One real bug it turned up.** `Run::take_choice` never checked that the choice
belonged to the door standing in front of you. It did not have to while one
door stood on a rung and the interface only ever offered that door's choices;
the chain's windows are wide enough that two can be open at once, and answering
one with the other's choice marked the wrong event answered.

---

## 22. What the structures turned out to be, and what a composite hides

Written while building H2 and Part D, which closes Phase 2. Corrections, in
the order they were found.

1. **THE BUYER's menu is gated rather than generated.** H2 describes a menu
   built from what the run is holding. `Choice` is static data and an event is
   a table; generating choices would mean the table stopped being a table. So
   the menu is three doors that open on what you hold -
   `Requirement::HoldingRumour`, `Requirement::Classes(1)`, and a hundred of
   your maximum, which anybody can sell - and the shut ones say why. The
   effect is the one the body wanted and the mechanism is the one the file
   already has.
2. **The Contract is frost you asked for.** It is applied in `combat_items`,
   where every other speed in this game is applied, rather than by teaching
   `simulate` about a piece of paper. `CONTRACT_SLOWER = 50` on every item's
   cooldown, three rungs, no early exit, and THE PAYOUT reads
   `contract_honoured` rather than asking anybody.
3. **The passenger rides as The Stranger's Parcel.** The rent is cells, and
   cells are what components cost - so the calf arrives as the component M9
   already appended for it, goes in the tray like anything else, and the
   player has to find it a seat. `passenger_is_seated` is what checks they
   did. The prose says so: it travels wrapped, and everybody who sees it will
   call it a parcel.
4. **Showstopper is claimed when you agree to headline, not when you win.** A
   `Brawl`'s `win` field is a *component*, and nothing in the game grants a
   class for winning a fight. Requiring a Rare assembled item is what makes
   the billing mean something; the bout is what you do with it.
5. **Unionized is the second stacking class**, after Piety - a picket line
   honoured twice is two picket lines. It is also the only thing in the game
   that hands out armour before a blow is struck, which is worth more than it
   reads: armour resets to zero every fight and soaks before health does.
6. **A Word About the Picket comes from THE INSPECTION, refused.** The bar
   draws exactly `SHOP_SIZE` shelves and it now has six things on it, so the
   third of Part D's words had to be come by somewhere else. The woman with
   the clipboard is a labour professional being refused, and on her way out
   she says where else that is happening this month - which makes THE
   INSPECTION the second door in the game where **declining is what pays**,
   after the Teller's third choice.
7. **The sealed bid is capped at 5,000.** `Requirement::Figure` carries its
   own range and the reserve is drawn as one to six times the standing rung's
   bounty, so the ceiling is roughly three times the largest reserve the door
   can name. A door that will take any number is a door with no shape.

**Where they stand** (indices, displayed rung is one more): THE INSPECTION 19,
THE CONTRACT 24, THE PAYOUT 28, THE BUYER 31, THE SEALED BID 35, THE FORK 36,
THE PASSENGER 41, THE FOUNDRY REMEMBERS 46, THROUGH THE CRACKED LENS 47. The
three pairs: THE WIZARD'S THIRST 30, THE EXHIBITION 33, THE PICKET LINE 38.
The foundry's three wait on `slagworks-known`, which the glow now sets
alongside its town reveal.

**Two real bugs, and they are the same bug twice.** Both are about a check
that reads less than it thinks it does.

- **`Run::take_choice` compared choices by address.** The ownership check that
  #21 added used `std::ptr::eq`. `EVENTS` is a static holding promoted arrays,
  and a caller in another crate can hold a reference to a *copy* of the same
  choice - so the test passed inside the engine, passed in the interface, and
  refused every choice in a test binary, which is the worst of the three
  places to find out. It compares by value now, which is what "belongs to this
  door" actually means.
- **`Run::cursed_for_good` was a list nothing read.** Documented since M12 as
  pieces carrying a curse for the rest of the run; the library set it,
  `Outcome::Uncurse` popped it, and no fight was any different for either.
  `CURSED_SLOWER = 25` closes it, in `combat_items` beside the contract, and
  the chill picks something that *acts* - a loose component has no cooldown to
  slow.
- **Every lint over `EVENTS` stopped at the top of an outcome.** Half this
  mission's bargains are an `Outcome::All`, and `class::is_earned`,
  `event::set_by` and the reachability lint all matched on `c.outcome`
  directly - so a class claimed inside an `All` read as a class no door hands
  out, and a fountain could have poured it. `event::every_outcome` unpacks
  `All` and `Gamble`, and everything that asks "does any door do X" asks it
  through there.


## 23. The base game had been speaking turtle for eleven milestones

Written while doing Phase 3. Part F's audit note flagged three canonical events
whose prose was in the book's voice - THE HAT MAN OF KOLOK, GERALD's *Deep
Chocolate*, THE GALAPAGOS EMPORIUM - and said the clean fix was to move the
turtle nouns into `theme.rs`. The note undercounted. **There were fourteen**,
and every milestone of this mission had added more, because the book's voice is
the fun one to write in.

The rule, stated properly, and now a lint:

- **A common noun that leaks is fixed in place.** The canonical column says
  *gold* and `vocabulary` puts *Fnorp* back. Five of these, including one in a
  **combat log line** - `"{} spends {} fnorp"` - which is the engine itself
  speaking turtle to a plain-theme player.
- **A proper noun that leaks moves.** The canonical column gets the *role* -
  the crownwright, the old watchman, the man who runs the store, an
  underwriting house - and `theme.rs` gets the scene, verbatim, with the name
  in it.

**`theme.rs` gained one table for that**, `told`, a list of `Retold { id,
title, prose, entry, landings }`. Keyed by id, because a title is prose and
prose gets rewritten while an id never moves; one table for events, towns and
dungeons, because ids are unique across the road and all three are things you
arrive at. A dungeon uses all four columns (its name, its blurb, its entry
cutscene, its landings) and an event uses two. Empty means "say the canonical
thing", which is what makes a half-written theme safe.

A8's entry cutscenes are in it, including the CREVICE retrofit the spec asked
for. Landings and entries are themed **at the source**, in `Run`, rather than
by each interface: the run already holds the theme, and a translation the two
interfaces have to remember separately is a translation one of them will
forget.

`tests/two_voices.rs` is the lint and it is a ratchet. Its budget is **five**,
and all five are the same thing: a component in `CATALOG` named after somebody
in the book - Sprocketman's Gratitude, Henpeck's Cell Keys, Kaklon's Patent,
Tetrahedron Shard - plus the two creatures that drop them. `CATALOG` is
index-keyed by `share.rs` and append-only for ever, so those cannot be renamed,
only recorded and translated. The `#[ignore]`d target asserts zero for the day
that stops being true, which is never.

**And a creature that did not exist.** THE UNWOUND was a label on the route
map, a theme entry and a `past_the_top` that could never be true - rung 51's
boss had no `MonsterSpec` and no frame at all, through four content
milestones. It is a frame now, band 51, Hollow, and the frame lint counts it.

---

# PART A — MECHANICS

## A1. Empowerment and shield become magic-only

Today (`combat.rs:3115`, `:3118-3123`): empowerment adds `stacks × 5 × mana`
to weapon **power** on *every* hit, and mana shield flat-reduces **any**
incoming damage, ahead of armor. Change both to the magic lane:

- **Empowerment**: its power contribution applies only to **magic-typed**
  hits. Physical hits are computed as if the stacks weren't there.
- **Mana shield**: reduces **magic** damage only. Absorb order for magic
  becomes shield → armor → health; physical skips shield entirely.

Glossary, tooltips, and the affected tests re-pin. Expect caster monsters
(Caster theme) to get slightly safer against physical and squishier against
mixed — re-run the ladder replays before re-pinning `progression.rs`.

## A2. The twins: Spellblade and Deflection (physical, not mana-scaled)

Two new stack counters, exact mirrors in the physical lane, **not** scaled by
mana (that scaling is what makes the mana pair "mana"):

```rust
Action::GainSpellblade(u32)   // +50 power-hundredths (0.50x, flat) per stack, physical hits only
Action::GainDeflection(u32)   // flat -10 per stack against physical damage,
                              // absorbed before armor (the physical lane's shield)
```

Numbers are starting points; tune against A5. Neither decays; both reset per
fight like every other counter. `reflect` is unchanged and distinct —
Deflection reduces, reflect pays back.

**Name collision, resolved:** `class.rs:819` already has a class named
**Spellblade** ("Half sword, half spellbook, and unwilling to choose",
`Transmute(50)`). Keep both — the overlap is harmonious, since the class
already lives on the physical/magic boundary. Audit `Transmute` after A1 (it
likely converts across the lanes this spec just separated), and consider (not
required) re-wiring the class power to grant Spellblade stacks.

**Catalog homes** (extend `catalog_shape.rs`): Spellblade grants live on
gloves (majority) and weapon accessories (minority) — reaction-flavored
amplification; Deflection grants live on chest (majority) and greaves plating
(minority). Empowerment/Shield remain helmet-exclusive.

## A3. Insight — the third lane's pool (locked until earned)

A new pool that is to **mind damage** what mana empowerment is to magic:

```rust
Resource::Insight            // the 8th — all three fusions shipped (see
                             // Reconciliation #2); audit every match,
                             // Resource::ALL, naming words, GUI chips, Drain
Action::GainDread(u32)       // the stack; helmet-exclusive grants
// mind damage dealt gains: dread_stacks × insight_held / 2   (tune divisor)
```

Rules: **locked until THE THRESHOLD is cleared** (`Run::insight_unlocked`);
until then Insight-granting pieces don't appear in shops (pool routing) and
the pool renders nothing. Insight is a valid `Drain` target (the counterplay
doctrine from the fusion pools). Income pieces are helmet-exclusive; a small
Insight/Dread gear family (~8–10 pieces, mostly helmet + a weapon book or
two) enters the catalog and the post-unlock shop pools. Not fusion-eligible
in v1.

## A4. Road machinery (the engine gaps this spec closes)

1. **Hidden towns.** `Town` gains an unlock: pinned (`after: usize`, as
   today) or **hidden** — revealed by an event outcome, then standing on the
   road ahead at its own `after` rung like any town. `Town` also gains its
   own action list (today the struct is `id/after/name/blurb` only):
   `actions: &'static [TownAction]` — the three shipped towns keep their
   current four doors, hidden towns bring unique ones (Part B).
2. **Event conditions** (the design doc's own gap table): conditional
   triggers on run state (chain flags, `best_fight_ms`-style records), and
   item conditions by name and by rarity, alongside the existing
   `LooseItemOfSize`.
3. **Event outcomes:** `RevealTown(id)`, `Give(component)` (hands a specific
   piece or rumour), `OpenShop(pool)` (curated shelf, one visit), and
   `StartDungeon(id)` from a town action.
4. **The seeded reroll** (for the crucible action): re-rolls a player piece
   into a random same-slot piece within ±15 rating, drawn from the run's own
   PRNG (`rng.rs`) — never combat, so determinism holds; quest pieces and
   rumours refuse the pot. Share codes are unaffected (they record the final
   board).
5. **Rung 51.** `ladder_complete()` currently ends the road at 50. When the
   chain is complete (A5 flags) and Francis falls, one more rung appears.
   Grinder: losable and retryable like any rung. Rogue: it costs lives like
   any rung. Share codes and scoring must accept rung 51.

## A5. Chain state

`Run` carries a small chain record: `heard_the_astronomer`,
`threshold_cleared` (= `insight_unlocked`), `slagworks_revealed`,
`manse_revealed`, `herald_beaten`, plus possession of the relic component.
The chain is **completable in a single run in either mode**, and every
station fails forward: a refused choice costs the reward, never the chain —
except the Herald, who can be fought again two rungs later if lost to
(Grinder) or costs a life like anything else (Rogue).

## A6. Two hover tooltips (small GUI change, engine-backed)

1. **Locked event paths explain themselves.** A choice whose `requires` is
   unmet still renders, grayed, and hovering it shows what would unlock it.
   Implement as `Requirement::describe(&self) -> String` in `event.rs` — the
   engine owns the words so the CLI can print the same thing and the theme
   layer can swap the nouns — e.g. *"Requires: A Word About the Cellar"*,
   *"Requires: having asked him on the road"* (the `Took` case), *"Requires:
   a loose Legendary"*. The existing per-choice `unmet` prose stays for
   flavor after an attempt; the tooltip is the plain statement before one.
2. **Rumours say what they're for.** Hovering a rumour (or the relic) in the
   tray shows the event it conditions and where that event fires: built from
   a reverse index over `EVENTS` at startup — *"Conditions: THE ASTRONOMER —
   rungs 18–30"*, *"Conditions: THE LOCKED GATE — after rung 22"*, and for
   the Mainspring: *"Opens the road past rung 50, once Francis falls."* No
   hand-written table to drift: if the event moves, the tooltip moves.

Free lint this buys: a test that every rumour is conditioned by at least one
event — an orphan rumour is dead content, and the reverse index makes the
check one assertion.


## A7. The road stack (interrupt resolution)

Everything that stands on a rung besides the fight — events, town gates,
fountains, and dungeons opened mid-rung — resolves through one explicit
**stack** in `run.rs`:

```rust
enum Interrupt { Event(EventId), TownGate(TownId), Fountain(usize), Dungeon(DungeonId) }
// Run gains: road_stack: Vec<Interrupt>
```

Arriving at a rung pushes that rung's furniture in reverse table order, so
pops come out in canonical order (fountain, then gate, then events by
`EVENTS` order). Resolving an interrupt may **push more**: an event whose
outcome is `StartDungeon` pushes the dungeon, which runs to completion
before the next pop. So two events on one rung with a dungeon between them
resolve exactly as: event one pops → dungeon pushes → floors run → dungeon
pops → event two pops → the rung's fight begins. The fight never starts
until the stack is empty — `the_road.rs`'s "nothing gets walked past"
doctrine, made into a data structure instead of a discipline.

Rules: push order is fixed by the tables, so replays are identical; consumed
interrupts do not return on a Grinder knockback (events stay once-per-run);
a dungeon exit (cleared or fled) resumes the pop where it left off.

**Seeing the stack.** Whenever two or more interrupts are queued — or the
player is inside any interrupt — the GUI shows a **stack strip**: the
current interrupt highlighted, the rest listed in pop order beneath it, and
the rung's fight always drawn last as the strip's floor, so the queue
visibly ends somewhere. Mid-dungeon, the strip is how you know what's under
you: *THE THRESHOLD (floor 2 of 3) → THE GLOW OVER THE RIDGE → the fight.*
Each pending entry hovers to an A6-style tooltip (name and kind), names
route through the theme layer, and a single-interrupt rung shows no strip —
the road only explains itself when there's something to explain.

## A8. Dungeon presentation (you always know you're inside one)

Three rules for every mini dungeon, the shipped THE CREVICE IN THE ROCK
included:

1. **Entry plays a cutscene**, on the same `pending_scene` machinery the
   bosses use — one or two lines, both voices, skippable like the rest.
2. **A persistent indicator while inside:** a distinct dungeon glyph and
   board-edge tint, and the road UI swaps for floor pips.
3. **The name, clearly:** a banner in the house all-caps — *THE THRESHOLD —
   FLOOR 2 OF 3* — routed through the theme layer like every string.

Entry lines (canonical / turtle; the CREVICE retrofit lines are Opus's to
write in that dungeon's existing register):

| Dungeon | Canonical entry line | Turtle entry line |
|---|---|---|
| THE THRESHOLD | "The door was not locked. Doors like this never are." | "Mind the doors. The doors here commute." *(p. 65)* |
| THE UNDER-MINE | "The seam was sealed from the outside, which is worth thinking about." | "The Sprocketmen sealed nothing. Someone sealed it *for* them." *(p. 44)* |
| THE UNDERTOW | "The water goes down and does not come back up. Neither does the light." | "Boyetano fished here sixty years. Ask what he was fishing *for*." *(p. 84)* |
| DEN RIVALS | "You counted the eyes. You stopped at forty." | "The exhibit promised the fury of a thousand bears. The museum never lied." *(pp. 89–90)* |
| WUMPUS WORLD | "Something in the dark already knows your footsteps." | "You smell it. Worse: that is how it finds you." *(CSV #12)* |

A cleared dungeon may add a one-line stinger before the stack pops; optional
per dungeon, never required.

## A9. The receipt (every resolution says what it did)

When an event choice, a dungeon, or a town door finishes resolving, a
**receipt** appears before the road moves on: a small panel listing the
concrete mechanical deltas of what just happened, one line each — *"Gained:
The Stranger's Parcel"*, *"+150g"*, *"Revealed: the Manse (after rung 24)"*,
*"−100 max health this run"*, *"Insight unlocked"*, *"Curse of Misfire →
Oathring"*. Like A6, it is engine-backed: outcomes describe themselves
(`Outcome::describe()` alongside `Requirement::describe()`), so the CLI
prints the same receipt and the theme layer swaps the nouns. Multi-outcome
choices list every line; seeded gambles reveal their *result* here, not
their odds — the dispenser's receipt is where you learn *"It wedged.
Nothing."* The receipt sits between a resolution and the next stack pop
(A7): dismiss it, and the strip advances. Flavor prose stays in the event;
the receipt is the plain accounting underneath it.

## A10. The route map (the road, drawn)

A Star Fox-style branching map of the whole run, engine-backed so it can
never drift from the road it depicts:

```rust
// engine: pure function of the tables + run state; the GUI only draws
pub fn route(run: &Run) -> RouteMap   // nodes, edges, fill states
```

**The grammar** (one rule per visual element):
1. **The spine** is the ladder: all fifty rungs visible from rung one, with
   pinned towns and bosses already marked ahead. Cleared rungs render
   filled; the road ahead is hollow and dashed; the current rung is ringed.
2. **Loops are events** — an out-and-back branch off the rung, which is
   literally a rendering of the road stack (A7): a dungeon opened mid-event
   extends the loop deeper (floor pips along the branch) before it returns
   to the rung it left.
3. **Exceptions draw as exceptions.** A branch that doesn't return home —
   the flock merging into the next rung's fight, a Skip Stone passing a
   rung — renders as a merge-ahead edge to wherever it actually lands.
4. **Hidden towns sit off-spine as diamonds**, because they were never on
   the road until an event put them there; their dungeons and orb trips
   hang off them as pip rows. Pinned towns are diamonds *on* the spine.
5. **Rung 51 appears only once the Mainspring is held** — the map growing a
   red node past Francis *is* the reveal; no other announcement needed.
6. Hover follows the A6 family: a future town shows its blurb, a branch its
   event's title, a locked branch its `Requirement::describe()`.

Nothing is hand-authored: nodes come from `LADDER`, `TOWNS`, `EVENTS`, and
`DUNGEONS`; fill and branch history come from run state; names route
through the theme layer. Because `route()` lives in the engine, the CLI
prints the same map in ASCII for free, and the map test is one assertion
per grammar rule rather than a screenshot diff.

---

# PART B — THE UNWINDING (base game; this text is canon)

## The story, in the game's own voice

The road is a winding. Every rung climbed turns the world's spring one tooth
tighter, and the man at the top is not the end of the road but its tension —
Francis's standing game is the pawl that keeps the whole mechanism from
running backward. Far under the road, where the first gear was cut, something
waits for the winding to stop. The astronomer reads it in stars that fall
against their arcs. The man in the Manse's cellar hears it through walls that
are not there. The Slagworks melts down what it keeps sending up. Gather the
words, take the mainspring they point to, and understand: beating Francis
releases the tension he held. Rung fifty-one is what the tension was holding
out. The Unwound does not swing at you. It simply stops things — and it has
been stopping things since before the first turn.

## New rumours (three, plus a relic)

| Component | How you get it | What it conditions |
|---|---|---|
| **A Word About the Wrong Stars** | shop shelf (rare pool) or the casino's second door | THE ASTRONOMER |
| **A Word About the Cellar** | THE ASTRONOMER, heard out | THE LOCKED GATE → the Manse |
| **A Word About the Glow** | the Manse's gallery, or the Herald's drop | THE GLOW OVER THE RIDGE → the Slagworks |
| **An Unwound Mainspring** *(relic, 1-cell)* | THE SECOND SHADOW, won | rung 51 |

## New events (in the `design/branching-events.md` format; all **spec**)

**THE ASTRONOMER.** *Trigger:* holding A Word About the Wrong Stars, rungs
18–30, once per run. A man with a cracked lens has been thrown out of every
observatory on the road for saying the same sentence. *Branches:* **Hear him
out** — lose the rumour, gain A Word About the Cellar and the chain's first
flag. **Turn him in** — the bounty pays double this rung; the chain does not
open this run. **Buy the lens** (requires 400g) — gain a 1-cell accessory,
"The Cracked Lens" (+20 mind, unique), *and* hear him out anyway. (The
lens matters again near the summit — H2.)

**THE LOCKED GATE.** *Trigger:* holding A Word About the Cellar, after rung
22. A gate with no road behind it. *Branches:* **Use the word** — lose the
rumour; **the Manse** stands after the next rung. **Walk on** — keep the
rumour; the gate finds you again three rungs later.

**THE GLOW OVER THE RIDGE.** *Trigger:* holding A Word About the Glow, after
rung 30. *Branches:* **Follow it** — **the Slagworks** stands after the next
rung. **Ignore it** — the rung's bounty again (the towns doc's "walking on"
rule, honored even for a town you haven't met).

**THE SECOND SHADOW.** *Trigger:* both towns revealed + THE THRESHOLD
cleared, after rung 42. Your shadow arrives before you do, and it is
carrying your build. *Branches:* **Face it** — fight **THE HERALD** (below).
Win: **An Unwound Mainspring** and, if not already held, A Word About the
Glow. Lose (Grinder): it waits two rungs up. **Refuse** — it follows; the
event re-offers every three rungs, last chance in front of rung 49. (A
quieter route to the Mainspring exists: THE PASSENGER, H2.)

## Hidden towns (unique doors; one action, then the road)

**THE SLAGWORKS** — stands after rung 32 when revealed (one clear of High
Wick at 31, so the two never share a stretch of road). Four doors:
1. **The crucible** — throw one piece into the melt: it comes back as a
   random same-slot piece within ±15 rating (run PRNG; quest pieces refuse).
2. **The mold line** — a curated shelf: enchantments and platings only, one
   visit (`OpenShop`); **the Lightning Rod** (H1) is always in stock.
3. **The tempering** — pay half a rung's bounty: one piece gains +10 rating
   (its name may grow a word; that is the point and the tooltip says so).
4. **The foreman** — he has heard something below: gain A Word About the
   Cellar if the Manse is still unfound, else 250g.

**THE MANSE** — stands after rung 24 when revealed. Four doors:
1. **Listen at the cellar door** — the man inside sounds insane and is
   right: **THE THRESHOLD** opens (`StartDungeon`).
2. **The gallery** — sell any one piece at double; if it was Epic or better,
   gain A Word About the Glow.
3. **The long table** — eat: +100 max health this run (once per run).
4. **The library** — one piece gains `Curse of Misfire` permanently and +25
   rating. The book was worth it. Probably.

## Mini dungeons (all-caps, per house convention; floors fought in order)

Floors are packed on the density curve at their *entry* bands, not their
unlock events: THE THRESHOLD at the mid-20s band, THE UNDER-MINE and THE
UNDERTOW at the mid-30s — these are meant to be hard fights met by formed
builds, which is why the chain now reaches them late.

**THE THRESHOLD** (via the Manse cellar) — three floors, Drainer-themed
wardens: **DOORKEEP**, **THE STAIR THAT LISTENS**, **THE LAST LANDING** —
each leans mind damage and `Drain`, each drops its gear per the named-drop
rule. Clearing it: **Insight unlocks** (A3), plus the first Insight helmet
so the pool means something immediately.

**THE UNDER-MINE** (via **THE FORK**, H2 — the event that stacks the mine
and a curated shop and makes you choose the order; this also repairs a
review catch: towns are one visit, so the earlier "second time at the
foreman" could never happen) — two floors of
Wall-theme diggers ending at **WHAT THE SEAM HID**. Reward: exclusive class
**Prospector** — *named creatures drop one extra piece of their gear.*

**THE UNDERTOW** (via the gallery: sell a Legendary and the buyer mentions
where it was fished up) — two floors, Slower-themed, ending at **THE THING
ON THE HOOK**. Reward: **the Depth** (H1) — one board slot of
your choice gains **+1 row** for the rest of the run (`taller_boards.rs`
already proves resize preserves placements).

## THE HERALD (mini-boss) and THE UNWOUND (rung 51)

**THE HERALD** — a Drainer/Striker party of two (`simulate_party`): your
shadow and its lantern. First multi-enemy fight in the game; tuned to be
beatable at-level with a coherent build.

**THE UNWOUND** — rung 51, appears only with the Mainspring held after
Francis falls. Hand-packed named board (authoring tool, locked), not
theme-packed: Wall + Drainer vocabulary, dense past the curve, `reflect`
high, heavy mind damage fed by its own Dread, `Drain` on every glove, and
curse resists near cap — the anti-build boss: it starves pools, erodes max
health, and pays melee back. Difficulty target in Part E. Defeat text (base):
*"The spring is cut. The road holds its own tension now."*

---

# PART C — THE ASCENSION OF NIBBALONIUS (turtle theme; `theme.rs` entries only)

## The same story, as the book tells it

Nibbalonius the Wise drifts between planes on a tetrahedron, ancient enough
to have **consumed the blind idiot god** (p. 64), and there is exactly one
plane he has never entered: the one he cannot enter **while Francis lives**
(p. 114). Every glimpse of him before the summit — the rung-46 encounter the
theme already names "Nibbalonius the Wise," the Tetrahedron Shard on the
board — is canonized by this chain as an *Anticipation*: a projection thrown
ahead of a thing still in transit, in the Cork scripture's sense (the 62
Anticipations, p. 84). The chain is the multiverse noticing his approach: a
watcher counts stars falling against their arcs as the tetrahedron occludes
them; **Eggbert's Mansion** (an unused title finally spent) stands over a
door of **the Mansus**, where doors commute near light speed and residents
are *seen with the ears and heard with the eyes* (pp. 64–67); the **Burnwarp
Foundry** in Kolok, between the Weirdeir Mountains and the Burnwarp Ocean
(p. 15), keeps melting down things that climb out of the melt. Clear the
Mansus antechamber and you come back with **Mansus-Sight** — the wrong sense
for this plane, and devastating to minds built on the right ones. Beat
Francis, and the last lock opens. He crossed a hundred dead planes to reach
the one Francis kept. He should have asked why Francis kept it.

## Theme entry table (canonical → turtle; the complete set for this spec)

| Canonical (Part B, ships in engine) | Turtle theme (display only) | Source |
|---|---|---|
| The Unwinding *(chain name, glossary)* | The Ascension of Nibbalonius | pp. 64, 114 |
| THE UNWOUND | Nibbalonius Ascendant | p. 64 ("consumed the blind idiot god") |
| THE HERALD | THE FIRST ANTICIPATION | p. 84 (the 62 Anticipations) |
| Insight *(pool)* | Mansus-Sight | pp. 64–67 (seen with the ears) |
| Dread *(stack)* | Anticipation | p. 84 |
| THE ASTRONOMER | THE TETRAHEDRON WATCHER | p. 64 |
| THE LOCKED GATE | EGGBERT'S GATE | CSV #45 (*Eggbert's Mansion*, unused) |
| THE GLOW OVER THE RIDGE | THE GLOW OVER THE WEIRDEIRS | p. 15 |
| THE SECOND SHADOW | THE ANTICIPATION | p. 84 |
| The Manse *(town)* | Eggbert's Mansion | CSV #45 |
| The Slagworks *(town)* | The Burnwarp Foundry | p. 15; volcano iguanas p. 6 |
| THE THRESHOLD | THE MANSUS ANTECHAMBER | pp. 64–67 |
| DOORKEEP / THE STAIR THAT LISTENS / THE LAST LANDING | THE DOOR THAT COMMUTES / THE HALL HEARD WITH THE EYES / THE LANDING BEFORE THE SUN | p. 65 (doors at light speed); p. 66 (Ghirbi, the sun-being, waits below — friendly, which is the joke) |
| THE UNDER-MINE | THE DEEP CHOCOLATE MINE | p. 44 (the Sprocketmen's secret); pp. 13–14 |
| WHAT THE SEAM HID | THE VEIN OF DEEP CHOCOLATE | p. 44 |
| THE UNDERTOW | BUNKO'S CAVERN | p. 84 (Boyetano; the Unmovable Rock) |
| THE THING ON THE HOOK | WHAT BOYETANO HOOKED | pp. 84–85 |
| Prospector *(class)* | Deep Chocolatier | p. 44 |
| the Depth *(reward)* | Boyetano's Patience | p. 84 |
| A Word About the Wrong Stars | A Word About the Tetrahedron | p. 64 |
| A Word About the Cellar | A Word About Eggbert's Cellar | CSV #45 |
| A Word About the Glow | A Word About the Burnwarp | p. 15 |
| An Unwound Mainspring *(relic)* | Nibbalonius's Calling Card | p. 114 |
| The Cracked Lens *(accessory)* | Foreston's Cracked Monocle | p. 2 |
| *Manse library flavor* | a reprint of the Words of Angelo — the banned text; of course it curses the piece with Misfire | p. 33 |
| *Manse long-table flavor* | the pudding, which is a universal constant | pp. 66–67 |
| *Unwound defeat text* | "He crossed a hundred dead planes to reach the one Francis kept. He should have asked why Francis kept it." | pp. 64, 114 |

Doctrine reminder: nothing in this column reaches game logic. The base game
never says "Nibbalonius" — except that its final boss was named Francis all
along, which is the one bleed-through the design keeps on purpose.

---

# PART D — Standalone rumour/event pairs (not chain-gated; all **spec**)

**A Word About the Thirsty Wizard → THE WIZARD'S THIRST.** *Trigger:*
holding the rumour, rungs 8–30. A wizard wants vial- and can-kind
accessories with an urgency he will not explain. *Branches:* **Trade one** —
triple its sell price, plus **the Appeal** (H1) — he keeps his seltzer and
has no use for second chances. **Refuse** — he Frost-curses a random equipped piece
(run PRNG) and leaves. *(Turtle theme: Sam the Wise and the Spindrift hoard,
pp. 108–109 — the one story in the book by Sarah.)*

**A Word About the Picket → THE PICKET LINE.** *Trigger:* holding the
rumour, standing before any mini-boss. Arena workers have downed tools over
six demands. *Branches:* **Honor the line** — the next town's shop door is
closed to you, gain stacking class **Unionized** (*start every fight with +20
armor per stack*). **Cross it** — this rung's shop shelf at 20% off; the
next three named creatures arrive with +1 gear_offset. *(Theme: the
gladiators' union and its six granted demands, pp. 105–107.)*

**A Word About the Exhibition → THE EXHIBITION.** *Trigger:* holding the
rumour, after the first boss; requires an assembled Epic. Two showfighters
want a demonstration bout — a 2-v-1 `simulate_party` fight at exhibition
stakes: losing costs this rung's bounty, never a life or a rung. *Win:*
class **Showstopper** (*+50% bounty on fights won in under 10 seconds* —
built to rhyme with the casino's trigger). *(Theme: Hanglo "Air Genius"
Chiemstar and Jimmy Chonga, gortball's finest, pp. 40–43.)*

---

# PART E — EXECUTION PLAN (phased: engine first, packing last)

## E0. The two ordering rules

1. **All engine work lands before any content.** Phase 1 is every mechanic,
   outcome, condition, and UI system in this spec, shipped dark or gated,
   each with unit tests. Phase 2 content PRs are then pure data.
2. **No creature gets an authored board until the end.** In Phase 2 every
   enemy in this spec exists only as a **MonsterFrame** — name, band, theme,
   one-line note (H4). Authored packing happens in Phase 4, all together,
   against the density curve, when the full content picture exists. A
   **frame lint** keeps everyone honest: the suite fails if a frame ships
   without an authored board, and debug builds render unpacked creatures
   with a loud UNPACKED tag. (Phase-2 behavioral tests fight against
   *scaffold boards* — packer-generated throwaways the lint marks; the
   packer already exists, so scaffolding is free. Phase 4's job is authored
   packing and pinning, not the first boards.)

## E1. Phase 1 — engine (no content strings beyond tests)

1. **Lanes and twins** (A1 + A2): empowerment/shield to the magic lane,
   Spellblade/Deflection; `typed_lanes.rs`; ladder re-pins measured alone —
   this is a rebalance and merges by itself.
2. **Insight + Dread** (A3), fully gated behind `insight_unlocked`.
3. **Stack, receipts, tooltips, map** (A7, A9, A6, A10): `Interrupt` +
   `road_stack`, `Requirement::describe()`, `Outcome::describe()`, the
   reverse rumour index, the stack strip and receipt panels, and `route()`
   with its GUI renderer and ASCII CLI printer; `road_stack.rs`,
   `tooltips.rs`, `route_map.rs`.
4. **Road machinery** (A4): `Town` migration (behavior-identical for the
   three shipped towns — their tests must not change), hidden towns,
   per-town doors; event conditions — run-state flags, item by name and
   rarity, **board inspection** (THE INSPECTION), **numeric answers** (THE
   SEALED BID), **behavioral counters** (the watcher pattern); outcomes —
   `RevealTown`, `Give`, `OpenShop`, `StartDungeon`, `GrantQuest`,
   **`GrantRow`** (rides the `taller_boards` resize), **`ClaimTicket`**,
   **`StandingOrder`** (guaranteed-kind shelf / free first reroll /
   consignment), **`Underwriter`**, the **scouting flag**; the seeded
   crucible/dispenser reroll; dark rung-51 plumbing.
5. **MonsterFrame system** (H4): the struct, party frames, the frame lint,
   and the four new themes registered in `design/monster-themes.md`
   (Hollow, Swarm, Beast, Warden).
6. **Dungeon presentation** (A8), the pedestal fixture, the shared
   per-destination visited-set.
7. **Relic and consumable plumbing** (H1): crushable 1-cell uniques (the
   Second Key — the only legal breach of the one-action rule, ever; the
   Appeal; the Skip Stone), the run-referencing stat functions (Tally,
   Odometer, Ledger — pure functions over counters `Run` already keeps),
   the Lightning Rod's curse-attraction rule, passenger occupancy.
8. **Stretch, last in the phase:** the Engraving representation (piece
   surgery) with its modified-piece share encoding, and the Brain Farm
   minigame widget. They land together or slip together; nothing in
   Phases 2–4 depends on them.

## E2. Phase 2 — content, creatures as frames only

PRs in any order once Phase 1 is green:
1. Chain rumours + events (Part B), hidden towns and their doors.
2. Dungeons as floor-frame lists + rewards; THE THRESHOLD unlock loop and
   the Insight gear family entering the pools.
3. Extra Large, the four Orbs of Travel, the four destinations (Part G).
4. The five unconditional events (Part F).
5. Part H structures: THE INSPECTION, THE SEALED BID, THE CONTRACT and THE
   PAYOUT, THE PASSENGER, THE BUYER, THE FORK, THE FOUNDRY REMEMBERS,
   THROUGH THE CRACKED LENS — and THE BRAIN FARM if the stretch shipped.
6. Part D standalone pairs; the classes (Prospector, Unionized,
   Showstopper, Wumpus Hunter); run-relics into their homes.

Phase-2 exit: a scripted run can reach every event, town, dungeon, and
reward in this spec; the diff contains **zero authored `gear:` boards**.

## E3. Phase 3 — theme

The Part C table plus the F6, G5, and H5 additions into `theme.rs`; A8
cutscenes in both voices; the existing canonical events' turtle nouns
(KOLOK, Deep Chocolate, GALAPAGOS EMPORIUM) migrated into theme entries per
the Part F audit note; glossary; docs updated — `branching-events.md` gains
every event at its true status, `towns.md` a hidden-town section,
`monster-themes.md` the four new themes and THE UNWOUND's hand-packed
exception.

## E4. Phase 4 — pinning, then packing, then balance (the last step)

1. `rating.rs` weights for every new mechanic re-pinned **first** — weights,
   never thresholds; shop pools finalized. (`stepped_component` re-gears
   every monster on three settings when a weight moves — settle the curve
   before authoring a single board; Reconciliation #4.)
2. Every frame receives its authored board, **by hand, in `make pack`**;
   THE UNWOUND likewise, locked, targeted inside the sudden-death ceiling
   (Reconciliation #3, #5).
3. `progression.rs` and difficulty pins re-baselined, one-line
   justification per constant.
4. The full E6 acceptance sweep.

## E5. Consolidated test inventory

`typed_lanes` · `insight` · `chain` (flag flow, fail-forward, completable
both modes, played end to end) · `hidden_towns` (reveal; old towns
byte-identical; crucible seed-stable) · `road_stack` · `tooltips` (both
`describe()`s; orphan-rumour lint; gamble receipts show results, never
odds) · `dungeon_presentation` · `pedestal` · `unconditional_events` ·
`threshold` / `undermine` / `undertow` · `herald` · `unwound` (appears only
with the relic after Francis; share codes round-trip rung 51) · the **frame
lint** · `sealed_bid` (reserve is seed-derived; receipt reveals it) ·
`inspection` (tiers read the live board) · `contract` (flags verify the
handicap was honored) · `passenger` (occupies real cells; lost on a loss;
delivery pays) · `buyer` (menu generated from holdings, prices seeded) ·
`fork` (both orders legal and materially different) · `watchers` (silent
arming sets flags; the later event reads them) · `claim_ticket` (whole
board drops, once, from the chosen creature) · `standing_orders` ·
`underwriter` (absorbs exactly one loss) · `grantrow` (no placed piece
moves) · `lightning_rod` (all incoming curses route to what covers it) ·
`consignment` (returns three shops later at +30) · `scouting` (grants zero
stats) · `route_map` (one assertion per A10 grammar rule: fill matches
cleared rungs, every consumed interrupt has a branch, merge-ahead edges
land where their outcomes say, hidden towns render off-spine only after
reveal, rung 51 absent until the Mainspring is held) · extensions to `catalog_shape` and `decode_build`.

## E6. Acceptance criteria

1. **Determinism:** two CLI replays of a chain-complete seed produce
   identical logs — crucible rerolls, sealed-bid reserves, and dispenser
   results included.
2. **No regression:** the three shipped towns' tests pass unmodified;
   rungs 1–14 TTK within ±10% of the pre-A1 baseline.
3. **A1 is a rebalance, not a nerf:** caster reference-build margins at
   rungs 20–35 within ±20%; tune the empowerment constant, never revert
   the lanes.
4. **The chain is completable** at Medium in both modes, proven by a
   scripted end-to-end run — via the Herald *and* via the Passenger.
5. **THE UNWOUND is harder than Francis:** at least two of the three
   Francis-beating reference builds lose to it unadapted; a fourth build
   written with Deflection and Insight in mind wins. Measured by replay.
6. **Stack determinism:** a same-rung event pair with a dungeon between
   them replays identically.
7. **Number anchoring:** every gold figure re-anchored against the live
   milestone table and bounties before pinning; the dispenser's 1:10 ratio
   and the Teller's price-of-listening preserved.
8. **Phase discipline is auditable:** no authored board in any Phase-2
   diff; the frame lint is red before Phase 4 and green after; every
   scaffold board is gone from the shipped binary.
9. **The Second Key breaks the one-action rule exactly once per crush**,
   and nothing else ever does.
10. **GrantRow moves no placed piece**, and the Depth's receipt names the
    slot it grew.
11. **The Underwriter absorbs exactly one loss**, and its receipt says
    which fight it ate.
12. **Scouting grants zero stats** — the board view is the entire reward.
13. Suite green; every re-pin justified in its commit message.

---

# PART F — Five unconditional events (no requirements, no overlaps)

All five: `trigger: Trigger::Rung` (they simply happen when you reach the
rung), every choice `Requirement::None`, once per run, `blocked_by: &[]`
except where noted. Suggested rungs — **4, 11, 16, 23, 27** — chosen against
the current table (casino window 1–10, GERALD at 8, AHEAD OF SCHEDULE at 21,
the chain's windows 18–30); Opus verifies against the final `EVENTS` table
and fills each `expects:` from the live ladder. Authored in the turtle voice
first, from stories not yet spent, then ported; the base port is canon and
the turtle text is a `theme.rs` entry.

**A note discovered while auditing:** the shipped canonical events already
speak turtle — THE HAT MAN OF KOLOK, GERALD hauling *Deep Chocolate*, THE
GALAPAGOS EMPORIUM. Since these five ship with both texts anyway, the clean
fix is to add event prose to the theme lookup tables and move the turtle
nouns of the *existing* events into theme entries at the same time. Flagged
as a task in E1, not relitigated here.

---

## F1 — the on-ramp *(rung 4; the required one)*

**Turtle: "IMMA GO BUY A SLURPEE"** *(the entire source story is one line,
p. 121 — the event honors that)*. A man hands you a parcel, says *"I'm gonna
go buy a slurpee if you wanna come,"* and walks off the road. You don't
come. He doesn't come back. The wrapping paper is a page torn from a star
chart, and someone has circled the shop two towns up that keeps the odd
words on its back shelf.

**Base port: "BACK IN A MINUTE."** A stranger hands you something to hold
and never returns. *Choices:* **Keep it** — gain **The Stranger's Parcel**
(1-cell accessory, modest, unique) and the prose points at where **A Word
About the Wrong Stars** is sold (it sits in the rare shop pool; this event
is how a player learns the chain exists). **Leave it on the milestone** — 150g
from whoever takes it; no pointer. *(Outcome: `Give`, already spec'd in A4.)*

## F2 — the trade you feel *(rung 11)*

**Turtle: "THE STORY FROM SONGIL"** *(Idiot Mode, pp. 15–17)*. In the Kolok
shopping district stands a windowless store the size of a county, whose
entire sign says **Large**. A man inside has carried a story out of Songil so
potent that hearing it drops IQ to decimals, and he wants to be rid of it —
he'll pay handsomely to say it to someone who'll still be standing after.

**Base port: "THE TELLER."** A man pays to be listened to, and the listening
costs what listening costs. *Choices:* **Hear it all** — permanent **−100 max
health this run**, and he gives you the best piece off his back (seeded,
Epic band). **Hear the short version** — −50 max health, 750g. **Plug your
ears** — he trudges on — and keeps what a head can hold, which
turns out to matter (Part G). *(Fresh mechanical space: nothing else trades max
health away; the Manse's long table is its mirror.)*

## F3 — the dispenser *(rung 16)*

**Turtle: "THE MACHINE IN THE BACK CORNER"** *(My Short Journey…, pp. 68–69
— the vending machine that ate Francis's dime: everything lit up but the
cherry bamblesnap, which costs $100 in dimes, and the water bottle that
wedged itself between the sides; "hopefully his $1 sacrifice would let him
win the war")*.

**Base port: "THE DISPENSER."** A machine at the roadside, humming.
*Choices:* **One coin** (100g) — run-PRNG: a common piece drops, *or it
wedges* and you get nothing but the memory. **The red one behind the glass**
(1,000g) — a guaranteed Epic-band accessory; the machine dispenses it with
ceremony. **Shake it** (free) — run-PRNG: two commons fall at once, or the
next rung's creature arrives with **+1 gear_offset** ("someone heard").
*(Distinct from the crucible: that rerolls what you own; this gambles what
you don't.)*

## F4 — the whisperer *(rung 23)*

**Turtle: "THE TABLES SPEAK FOR US"** *(p. 122, title verbatim — the package
set at the table's center, and the table that spoke and spoke until what was
on it breathed)*. Set one loose piece on the table at the roadside inn, and
the table will tell it what it is trying to become.

**Base port: "WHAT THE TABLE SAID."** *Choices:* **Set a quest piece on it** —
its condition is spoken aloud (revealed in full) and **halved**. **Set an
ordinary piece on it** — it gains a small quest (*becomes* a +15-rating
variant of itself after 30 activations). **Keep your gear to yourself** —
the table says nothing, pointedly. *(Fresh space: the only content anywhere
that touches the quest system. New outcome `GrantQuest` — add to A4's list.)*

## F5 — the flock *(rung 27; `blocked_by: ["the-vip-area"]` if rungs collide)*

**Turtle: "UNSOLICITED PROPOSAL"** *(pp. 123–124 — the memo, in full campus-
bureaucratic register, on the coming territorial war with the birds and its
"three potential outcomes," and the armament that story reaches for:
racquets)*. A courier hands you a memo about the birds massing past the next
ridge. It is CC'd to several governing bodies. It is not wrong.

**Base port: "THE BIRD PROBLEM."** *Choices:* **Arm up** — gain the racquet-
style glove mold (Opus: use the shipped canonical glove piece nearest to a
racquet; the theme table already maps one). **Pay the toll** (300g) — the
flock parts; nothing follows. **Ignore the memo** — the next rung is a
**party fight**: the creature plus **THE FLOCK** (a new weak swarm spec,
Striker-themed, more annoying than deadly). *(Fresh space: the only event
that changes the shape of the next fight, and the first adversarial use of
`simulate_party` outside the Herald.)*

---

## F6 — additions to earlier parts

**Theme table (Part C) gains:** BACK IN A MINUTE → IMMA GO BUY A SLURPEE
(p. 121) · The Stranger's Parcel → The Slurpee Man's Parcel · THE TELLER →
THE STORY FROM SONGIL (pp. 15–17; the store called Large earns a mention in
prose) · THE DISPENSER → THE MACHINE IN THE BACK CORNER (pp. 68–69) · the
Epic behind the glass → the cherry bamblesnap · WHAT THE TABLE SAID → THE
TABLES SPEAK FOR US (p. 122) · THE BIRD PROBLEM → UNSOLICITED PROPOSAL
(pp. 123–124) · THE FLOCK → THE BIRDS OF THE RIDGE.

Execution and acceptance for these five are folded into Part E (Phases 2
and 4). The guarantee stands: a run that touches no rumour, no town, and no
chain still meets all five — the road is never bare, and F1 is how a blind
run finds the chain at all.

---

# PART G — EXTRA LARGE AND THE ORBS OF TRAVEL

## G1. The unlock: an event only intact heads can have

**THE BIGGER SIGN** — *Trigger:* `Requirement::Took("Plug your ears")` from
THE TELLER (the cross-event `Took` pattern GERALD → AHEAD OF SCHEDULE already
ships), rungs 13+, once per run. Because you kept your head whole, you notice
what nobody else on the road can: behind the warehouse's sign that says
**Large**, a second sign, further back and taller — **Extra Large**.
*Branches:* **Follow the sign** — the hidden town **Extra Large** stands
after the next rung. **Forget you saw it** — 200g; some knowledge is for
selling. *(This retroactively makes F2's "nothing" choice the secret best
one: the reward for refusing the story is the ability to find the sequel.
"Large" and "Extra Large" are adopted as canonical names — plain-English
absurdism, not turtle vocabulary; the turtle theme adds the Songil flavor.)*

## G2. The town, and the pedestals

**EXTRA LARGE** — stands after rung 13 when revealed. A store the size of a
weather system, all ground floor, no windows. Its four doors follow the
one-action rule like every town; **the pedestal does not** — it stands in
the entryway and takes its own key: an **Orb of Travel**. Feeding it an orb
you own consumes the orb and pushes the orb's destination onto the road
stack (A7): the trip runs, pops, and returns you to the entryway with the
road exactly where you left it. Any number of held orbs can be fed in one
visit; each destination fires **once per run**. The four doors, briefly:
1. **Aisle 9** — a curated shelf of Orb-kind pieces (`OpenShop`), the only
   guaranteed place to meet the Orbs of Travel; two of H1's run-relics — the
   Odometer and the Ledger — restock here (the Tally must be earned, H2).
2. **The returns desk** — sell any piece at full price (no sell penalty), or
   **consign** it (H1): it returns to a shop shelf three shops later at
   +30 rating.
3. **The sample counter** — a free common piece, seeded.
4. **The manager** — he confirms the store is the only one, on any plane,
   and gives you A Word About the Wrong Stars if unheld.

**The second pedestal** stands in **High Wick** (after rung 31, the ladder's
final pinned town) — same rules, no door consumed. It exists because the
orbs are random shop finds: a player whose orbs arrived late still gets to
spend them, and a chain-runner passing High Wick at rung 31+ meets the
destinations at the difficulty band they were packed for.

## G3. The four Orbs of Travel (shop pieces first, tickets second)

All four are Orb-kind weapon cores in the regular shop's orb pool at low
weight — real pieces with **unique effects on the spells slotted into
them**, worth buying even if the pedestal is never found. Duplicates are
legal and simply remain weapons; the pedestal refuses a destination already
visited.

| Orb (canonical) | Effect on slotted spells | Pedestal destination |
|---|---|---|
| **Wayfarer's Orb** | Each slotted spell's first cast per fight costs no mana | THE THRUMBUS RACE |
| **Pilgrim's Orb** | Slotted spells: +25% power, +25% cooldown | DEN RIVALS |
| **Ferry Orb** | When a slotted spell casts, the orb's other spells' cooldowns drop 1s (orb-internal, `OnOtherCast` machinery) | MOLE TOWN |
| **Stray Orb** | Slotted spells ignore Misfire | WUMPUS WORLD |

## G4. The four destinations (each once per run, then back to the road)

- **THE THRUMBUS RACE** *(event)* — the 45th running. *Choices:* back a
  runner (300g stake, seeded: **the Claim Ticket** (H1) or nothing — the
  jackpot ticket: the next named creature of your choice drops its *entire
  board*), or ride (no stake;
  finish and take a Thrumbus-band greaves piece off the paddock rail).
- **DEN RIVALS** *(mini dungeon, two floors)* — the den, then **THE
  THOUSANDTH BEAR**. Packed Striker-heavy at the traveler's band. Reward:
  **Bearhide**, a unique chest base (big health, `Gain` Fury on battle
  start).
- **MOLE TOWN** *(event)* — the highway ends at a town built entirely at
  ankle height. A tiny curated shop at a permanent discount, and a mole who
  will trade any one curse off a piece for 400g.
- **WUMPUS WORLD** *(mini dungeon, two floors)* — dark floors, then **THE
  WUMPUS**. Slower-themed; the classic hunt, deterministic. Reward: class
  **Wumpus Hunter** — *your first hit each fight cannot miss and cannot be
  deflected.*

## G5. Theme table additions (Part C gains)

THE BIGGER SIGN → THE SIGN BEHIND THE SIGN (Songil; the store from Idiot
Mode, pp. 15–17) · Extra Large → Extra Large (the turtle theme keeps the
name and adds the Songil provenance in prose — the joke needs no
translation) · Wayfarer's/Pilgrim's/Ferry/Stray Orb → planeswalking flavor
(the warp device's lesser cousins, p. 11) · THE THRUMBUS RACE → THE 45TH
ANNUAL THRUMBUS RACE (CSV #11, unused title, spent at last) · DEN RIVALS →
DEN RIVALS: FURY OF A THOUSAND BEARS (CSV #61; the Galapagos Emporium
exhibit made real, pp. 89–90) · MOLE TOWN → HIGHWAY TO MOLE TOWN (CSV #96,
unused) · WUMPUS WORLD → WUMPUS WORLD (CSV #12, unused; sibling title *How
to Train Your Wumpus* free for a floor name) · THE THOUSANDTH BEAR / THE
WUMPUS → themselves, all caps being a universal language.

## G6. Execution

Folded into Part E: pedestal plumbing in Phase 1, Extra Large and the four
destinations in Phase 2 (creatures as frames), boards in Phase 4.
`pedestal.rs` keeps its brief: an orb is consumed, a destination fires once,
the stack returns to the exact position, a duplicate orb is a weapon rather
than a ticket, both pedestals share one visited-set, and an orbless run sees
dormant pedestals and never an error.

---

# PART H — THE REWARD VOCABULARY AND THE NEW STRUCTURES

## H1. Twelve new payables (rewards the road can hand out)

| Reward | What it does | Home |
|---|---|---|
| **the Depth** | `GrantRow`: one slot of your choice gains +1 row for the run | THE UNDERTOW, cleared |
| **the haunted row** | +1 row whose cells give pieces +30% stats and a time curse | a SEALED BID lot |
| **the Claim Ticket** | The next named creature of your choice drops its **entire board** | THE THRUMBUS RACE, bet won |
| **The Tally** *(run-relic)* | +2 strength per event resolved this run | THE INSPECTION, tier 3 |
| **The Odometer** *(run-relic)* | +1 speed per ten rungs climbed | Aisle 9 |
| **The Ledger** *(run-relic)* | power scales with unspent gold | Aisle 9 |
| **the Second Key** *(crush)* | take a second town action, once — the only legal breach of the one-action rule | Mole Town's shop |
| **the Appeal** *(crush)* | re-offer one once-per-run event you declined | the Wizard's trade |
| **the Skip Stone** *(crush)* | pass a rung, forfeiting its bounty | a SEALED BID lot |
| **standing orders** | guaranteed-kind shelf, or first reroll always free | SEALED BID lots |
| **consignment** | sell a piece; it returns three shops later at +30 rating | Extra Large's returns desk |
| **the Underwriter** | your next loss within five rungs doesn't count (one fight, once) | THE PAYOUT |
| **the Lightning Rod** *(enchantment)* | every curse applied to your board lands on whatever covers it | the Slagworks mold line |
| **scouting** | see an upcoming boss's packed board from the loadout screen | THROUGH THE CRACKED LENS |
| **Engraving** *(stretch)* | move one trigger from piece A to piece B, once — requires the modified-piece share encoding | THE BRAIN FARM, won |

Run-relics are 1-cell uniques whose stats are pure functions over counters
`Run` already tracks; crushables are 1-cell uniques consumed on use, receipt
included.

## H2. Nine new structures (events that aren't doors)

**THE INSPECTION** *(rungs 20+, unconditional)* — an inspector reads your
live board: assembled items sharing an alignment, tiered 0/1/2/3 → nothing /
300g / a piece off the cart / **The Tally**. Building *for* events becomes a
strategy.

**THE SEALED BID** *(rungs 33+, requires the Slagworks revealed)* — the
foundry auctions one lot against a hidden reserve derived from the run seed.
You name a figure: over pays, under loses the lot; the A9 receipt reveals
the reserve either way, so losing teaches. Lots rotate seeded from: the
haunted row, both standing orders, the Skip Stone, a Legendary-band piece.

**THE CONTRACT → THE PAYOUT** *(rungs 25+, a `Took` pair)* — accept Frost on
all your gear for three rungs now; THE PAYOUT verifies via run flags that
you honored it and pays **the Underwriter** plus 400g. Player-priced
difficulty, local and transactional.

**THE PASSENGER** *(after rung 40; same prereqs as THE SECOND SHADOW)* — a
fragile 1-cell passenger must occupy a board of your choice for five rungs —
rent paid in dead cells. Lose any fight and the passenger is lost; deliver,
and the courier hands you **An Unwound Mainspring**. The quiet route: the
Shadow fights for it, the Passenger pays floor space for it.

**THE BUYER** *(rungs 30+)* — the first event whose menu is generated from
what you hold: sell a rumour (the chain, sabotaged for cash), sell a class
stack, sell 100 max health. Seeded prices; the receipt states exactly what
left you.

**THE FORK** *(rungs 33+, requires the Slagworks revealed)* — pushes **two**
futures onto the road stack — THE UNDER-MINE and a curated shop — and the
only choice is the order. Shop-then-mine is not mine-then-shop when the
shelf can feed the fight; the event teaches A7 by making order the decision.

**THE FOUNDRY REMEMBERS** *(rung 45+; the documented watcher)* — the silent
pattern: revealing the Slagworks arms a counter nobody mentions (crucible
melts). At 45+ the foundry speaks: two or more melts and it returns your
best-melted piece at Legendary band ("we kept your best"); zero melts and
prices run +10% ahead — it noticed the snub. Receipt-only arming, payoff
later: the closest this game gets to being haunted.

**THROUGH THE CRACKED LENS** *(rungs 46+, requires holding The Cracked
Lens)* — the astronomer's lens finally focuses: **scouting** for the rest
of the run, THE UNWOUND included. Seeing that board before fighting it is a
story beat disguised as a feature.

**THE BRAIN FARM** *(rungs 35+, stretch-gated with Engraving)* — an actual
playable game of tic-tac-toe against a farm of perfect brains with one
seeded flaw square. Draw: 200g and their respect. Find the flaw and win:
**Engraving**. The first interactive minigame structure; the casino already
proved performance-gated rewards belong here.

## H3. Integration changelog (what moved elsewhere)

THE UNDERTOW's class reward is cut in favor of **the Depth** (the class
roster is now Prospector, Unionized, Showstopper, Wumpus Hunter) · the
Thrumbus Race's triple-or-nothing is now **Claim Ticket**-or-nothing ·
Aisle 9 stocks the Odometer and Ledger · the returns desk gained
consignment · the mold line stocks the Lightning Rod · **THE FORK replaces
the foreman's impossible "second visit"** (a review catch: towns are one
visit) · the Wizard's sweetener is now the Appeal · THE SECOND SHADOW and
the Cracked Lens gained forward pointers · F6 and G6 fold their execution
notes into Part E.

## H4. MonsterFrames and the four new themes

```rust
pub struct MonsterFrame {
    pub name: &'static str,
    pub band: usize,              // the rung whose difficulty it packs to
    pub theme: MonsterTheme,      // the packer draws from exactly one
    pub note: &'static str,       // one line for the Phase-4 packer-author
}
```

No `gear:` board is authored before Phase 4 (E0 rule 2). New themes, in the
house "what each creature is for" register:

- **Hollow** — kills by shrinking you: mind damage, Drain, high curse
  resist. The eldritch lane's face.
- **Swarm** — arrives everywhere and dies fast: many small, quick
  activations, thin health.
- **Beast** — the honest fight: strength, rage, health, nothing clever.
- **Warden** — makes you pay for time: armor, harden, and curses applied.

| Frame | Where | Band | Theme | Note |
|---|---|---:|---|---|
| DOORKEEP | THE THRESHOLD f1 | 24 | Hollow | teaches Drain before it hurts |
| THE STAIR THAT LISTENS | THE THRESHOLD f2 | 25 | Hollow | mind pressure, little else |
| THE LAST LANDING | THE THRESHOLD f3 | 26 | Hollow | the gate before the light |
| THE DIGGERS | THE UNDER-MINE f1 | 33 | Warden | armor that digs in |
| WHAT THE SEAM HID | THE UNDER-MINE f2 | 34 | Warden | sealed for a reason |
| THE CURRENT | THE UNDERTOW f1 | 33 | Slower | the water sets the pace |
| THE THING ON THE HOOK | THE UNDERTOW f2 | 35 | Slower | patient, like its fisherman |
| THE DEN MOUTH | DEN RIVALS f1 | 30 | Beast | the first hundred bears |
| THE THOUSANDTH BEAR | DEN RIVALS f2 | 32 | Beast | the exhibit's promise, kept |
| DARK FLOOR | WUMPUS WORLD f1 | 30 | Swarm | what lives near a wumpus |
| THE WUMPUS | WUMPUS WORLD f2 | 32 | Beast | it already knows your footsteps |
| THE FLOCK | F5's party adjunct | matches rung | Swarm | annoying before deadly |
| THE SHADOW | THE HERALD (party) | 43 | Hollow | your build, hollowed |
| THE LANTERN | THE HERALD (party) | 43 | Striker | what the shadow carries |
| THE UNWOUND | rung 51 | 51 | Hollow | hand-packed in Phase 4; the exception |

## H5. Theme additions (Part C gains)

THE INSPECTION → THE RICE INSPECTION (Petonkle grades everything, pp.
61–63) · THE SEALED BID → THE FNORP AUCTION (p. 28) · THE CONTRACT / THE
PAYOUT → THE CORK CONTRACT / **THE 63RD ANTICIPATION** (there were only
ever 62, p. 84 — until you) · THE PASSENGER → **THE WIMPLER CALF** (deliver
the calf before the road reaches the Last Oxen at 47, pp. 91–93 — the
timing is the point) · THE BUYER → THE MULTICITY BUYER (pp. 70–73) · THE
FORK → THE FORK IN THE SEAM · THE FOUNDRY REMEMBERS → THE BURNWARP
REMEMBERS (p. 15) · THROUGH THE CRACKED LENS → THROUGH FORESTON'S MONOCLE
(p. 2) · THE BRAIN FARM → OMBREDOR-5 (Brain Power Incorporated, at last) ·
the Claim Ticket → the Galapagos Claim Chit (pp. 89–90) · The Tally →
Sherman's Count (p. 114) · The Odometer → the Yonk-Standard Odometer · The
Ledger → the Fnorp Ledger · the Second Key → Gappy's Spare Key (p. 2) · the
Appeal → Get Jar Jarred (the book's attorney, gladly) · the Skip Stone →
the Flattened Step · the Underwriter → Treyway Underwriting (p. 18) · the
Lightning Rod → the Hell-Pigeon Perch (curses roost, p. 29) · scouting →
Foreston's Long Look · Engraving → Kaklon Engraving Services (pp. 32–35).
