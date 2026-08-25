# Gear Master — Slot Basis-Vector Rewrite
## Execution spec for Claude Code (Opus)

**What this is.** A data-grounded redesign of the five gear slots so that each slot is oriented around one mechanical basis vector, with deliberate ~20–25% bleed into exactly one neighboring slot. The weapon keeps its identity (damage conversion) but *loses its monopolies*; the four armor slots stop being interchangeable stat piles. All judgment calls are made in this document — what remains is execution, verification, and number-tuning inside the stated bounds.

**Prime directives (read before touching anything):**
1. **Canonical names are keys.** `theme.rs` lookups, `combat.rs` monster `gear:` lists, `Quest.becomes` targets, and the test suite are all string-keyed on piece names. Prefer rewriting a piece's *stats and triggers under its existing name*. Rename only when the name would lie about the new identity, and then run the propagation checklist in §8.
2. **Determinism is sacred.** No randomness anywhere. Every mechanic you write must be expressible in the existing deterministic vocabulary (`piece.rs:418` Actions, `piece.rs:535` Triggers, `piece.rs:283` EffectKinds, `curse.rs` curses).
3. **No engine semantics change.** This is a catalog rewrite (`piece.rs:960` `CATALOG`, 444 entries) plus re-pinning of dependent systems. Every axis below is already expressible with shipped mechanics. The only new code is one new test file and optional constant tuning (§6).
4. **Measure before you move.** Capture the baseline metrics in §7 first; they are the acceptance criteria's denominators.

---

# 1. Current-state analysis (the evidence)

Method: all 444 `PieceDef` entries were parsed into a feature matrix — 20 stat fields from `stats.rs`, 11 trigger variants, 13 action variants, 6 positional effect kinds, plus `speed_bonus`/`power_bonus` — then examined three ways: per-slot feature prevalence, cosine similarity between slot mechanical centroids, and PCA/k-means over the standardized matrix. (PCA stands in for ICA here; the conclusions below don't depend on the rotation.)

**Catalog counts:** Weapon 171 · Helmet 79 · Chest 68 · Gloves 75 · Greaves 51.

## 1a. The four armor slots are one slot wearing four shapes

Cosine similarity between slot mechanical profiles:

|         | Weapon | Helmet | Chest | Gloves | Greaves |
|---------|-------:|-------:|------:|-------:|--------:|
| Weapon  |  1.00  |  0.38  | 0.35  |  0.71  |  0.49   |
| Helmet  |        |  1.00  | **0.93** |  0.67  |  **0.85** |
| Chest   |        |        | 1.00  |  0.64  |  **0.84** |
| Gloves  |        |        |       |  1.00  |  0.73   |
| Greaves |        |        |       |        |  1.00   |

Helmet↔Chest at **0.93** is near-identity. k-means (k=5) confirms it from the other direction: one cluster absorbs **353 of 444 pieces across every slot** (122 W / 70 H / 60 C / 56 G / 45 Gr); the only clusters with real identity are weapon sub-archetypes (casting cores, melee cores, curse-carriers). Mechanical identity today is orthogonal to slot. The PCA spectrum is nearly flat (top five components explain only 21% of variance) — the space is *smeared*: outside the weapon there are no axes, only seasoning. PC1 (weapon −1.04 vs chest +1.07, helmet +0.85) is the one strong signal, and it just says "weapon vs everything."

The seasonings that do exist, per-slot prevalence: helmet leans mind_resist (11%) and faith (13%); chest leans health (38%), armor (40%), physical_resist (15%), nature (12%); gloves lean strength (19%) and physical_pierce (7%); greaves lean speed_bonus (20%) and regen (20%). These are the embryos of the axes in §2 — the design amplifies what's faintly there rather than inventing against the grain.

## 1b. The weapon isn't over-tuned — it's over-scoped

The weapon slot is the only slot with *decisions* (three recipes at `piece.rs:810`: melee handle/damaging, book/ink/spell, orb/spells/alignment), and on top of that it **hoards two other axes**:

- **Curse application** (the tempo-denial game): weapon **37** instances vs Helmet 5, Chest 6, Gloves 9, Greaves 4.
- **Reaction triggers** (`OnAdjacentActivate` + `OnAlignedActivate` + `PerAdjacentItem`): weapon **10** vs Helmet 3, Chest 1, Gloves 2, Greaves 1.
- Exclusive already: `power_bonus` (29% of weapon pieces, 0% elsewhere), `OnOtherCast` (15%, orb-internal), `PerAdjacentEmpty`, forking, inks/spells/alignments.

So a weapon answers "how do I deal damage," "how do I deny the enemy tempo," *and* "how do my items talk to each other," while a chest answers "how much armor." That is the real imbalance: the weapon buys choices, the armor slots buy stats. **The fix is redistribution, not nerfs** — ship the curse game to greaves and the reaction game to gloves, and the weapon's share of build agency falls without touching a single damage number.

## 1c. Named off-axis offenders (the sweep's starting points)

- **Helmet pieces that are pure armor/health piles** (belong to chest's axis): Reliquary Frame, Reliquary Frame of Nine, Stonewall Frame, Green Crown, Chapel Frame, Steel Frame, Iron Plating, Warding Plate — and more; ~28% of helmets carry health, 37% armor, while only 11% carry mana.
- **Chest pieces doing economy/casting work** (helmet's axis): Voidsilk Base, Mana Loom, Runed Lining, Hexweave Shroud, Aegis Weave, Wellspring Base, Aether Layer. (Some of these can stay — they become chest's *bleed* budget, §3.3.)
- **Gloves that are plain stat piles with no triggers or effects at all**: Spiked Vambrace, Ironhide Wrap, Breaker's Fist, Thornweald Grip, Signet of Vigour, Iron Band, Emberloop, Bloodring — a reaction slot where a third of the gear doesn't react to anything.
- **Greaves with zero tempo content** (no speed, no cooldown, no battle-start, no curse): Tempered Sole, Warplate Greave, Pilgrim's Sole, Rootbound Material, Studded Sole, Sapling Mold, Zealot's Sole, Runed Material.

---

# 2. The five basis vectors

| Slot | Basis vector | One-line identity | Bleeds into |
|------|--------------|-------------------|-------------|
| **Weapon** | **Conversion** | Turns time and banked resources into typed damage. | Gloves (a few accessories keep reaction triggers) |
| **Gloves** | **Reaction** | Acts when *other things* act: adjacency, alignment, answers. | Greaves (some reactions pay out in tempo) |
| **Greaves** | **Tempo** | Sets the clock: your speed, their curses, the opening move. | Chest (optional Plating armor — already in the recipe) |
| **Chest** | **Reserve** | How long you last: health, armor, regen, growth, hardening. | Helmet (bases that bank/hold pools) |
| **Helmet** | **Economy** | Income and conversion of the four pools; the mind game. | Weapon (mind/magic damage as cast support) |

The bleed relation is a directed 5-cycle — **W→G→Gr→C→H→W** — so every slot overlaps exactly one neighbor and no two non-adjacent slots share a secondary. Two links already exist in shipped code and are kept, not invented: greaves' recipe already admits a Plating (`piece.rs:844-848`), and helmet already leads mind/mind_resist.

**The "express it through your axis" principle** (this is the rule that keeps the sweep coherent — apply it everywhere): every slot may do defense, and every slot may do offense, but *only in its own vocabulary*:

- Chest defends with **armor/health/harden**; attacks not at all — it **Grows** (`Action::Grow` becomes chest-exclusive: outlasting *is* its offense).
- Helmet defends with **mana shield stacks and mind_resist**; attacks with **mind damage** (its bleed toward weapon).
- Gloves defend by **denial** — `Drain` and `StunStrongest`; attack with **reaction damage** (small typed hits that answer activations).
- Greaves defend with **curse_resist and Frost**; attack with **enemy curses** (Frost/Stun/Misfire — received from the weapon's exodus; **Searing stays weapon-majority**, it's damage wearing a curse costume).
- Weapon does raw conversion, as today.

**Exclusivity table** (enforced by the new test in §6.1 — "exclusive" means *only this slot's pieces may carry it*; "majority" means ≥70% of catalog instances live here):

| Mechanic | Home | Level |
|---|---|---|
| `power_bonus`, Ink/Spell/Alignment/Book/Orb kinds, `GainForking`, `OnOtherCast`, `PerAdjacentEmpty` | Weapon | exclusive (already true) |
| Searing application | Weapon | majority |
| `Consume`, `GainEmpowerment`, `GainShield`, `MindDamage` (player gear), `mind_resist` stat | Helmet | exclusive |
| `Grow`, `physical_harden`/`magic_harden` stats | Chest | exclusive |
| health above 15 per piece | Chest | majority |
| `OnAdjacentActivate`, `PerAdjacentItem`, `Drain`, `StunStrongest`, `DoubleAdjacentItemStat` | Gloves | exclusive |
| `OnAlignedActivate` | Gloves | majority (helmet/weapon minority allowed) |
| `OnBattleStart`, `speed_bonus` outside weapon, `ReduceCooldown` outside weapon | Greaves | exclusive |
| Frost/Stun/Misfire application | Greaves | majority |

**Quotas per non-weapon slot** (also test-enforced): ≥60% of the slot's pieces express the primary axis, 20–25% express the bleed axis, ≤15% are plain flat-stat fillers (cheap early gear is fine and the name generator even has Plain epithets for it).

**Interaction-density quotas (Part II, §10):** on top of the axis quotas, ≥35% of each slot's above-common pieces must carry an *interaction* (positional effect, adjacency bonus, reaction, watcher, diagonal, overlap, or fusion — Part II defines these), every Epic/Legendary non-weapon piece must carry at least one, and pool-spend texture (`SpendMana`/`Spend`/`Consume`) is capped at ≤15% of pieces per slot outside the helmet. Today the board barely talks to itself — 67 of 444 pieces (15%) have any positional content, only 36 of those outside the weapon — while 74 pieces are pool-spend texture. These quotas invert that.

---

# 3. Per-slot rewrite specs

Each spec gives: the identity, what the slot's ~budget looks like after the sweep, migration rules, and one concrete before→after example. Numbers in examples are sketches — tune them against §7, not against taste.

## 3.1 Weapon (171 pieces) — Conversion. *Narrow it, don't nerf it.*

Keep all three recipes and every damage number. The work is the **exodus**:

- **Curse exodus.** Of the 37 curse applications on weapon pieces, keep Searing (damage-flavored) and roughly a half-dozen signature Frost/Stun/Misfire carriers on high-rarity spells; move the *concept* of the remaining ~20 to new greaves pieces (§3.5). The weapon piece that loses its curse gains nothing — that's the point; its price and rating drop accordingly and `rating.rs` re-pins absorb it.
- **Reaction exodus.** Of the 10 reaction triggers, keep 2–3 on Accessories (this is the weapon's *bleed into gloves*, per the cycle); move the rest to gloves designs.
- `StunStrongest` and `Drain` instances on weapon pieces migrate to gloves (§3.4) — denial is the hands' vocabulary now.

## 3.2 Helmet (79 pieces) — Economy. *The mind: income, conversion, and the mind game.*

**Owns:** mana and pool income (`GainMana`, `Gain{}`), `Consume` (the whole-pool spender — currently smeared, becomes helmet-exclusive), `GainEmpowerment`, `GainShield`, `MindDamage`, `mind_resist`. Helmet is where a build decides *what its pools are for*.
**Bleed → Weapon:** crests carrying mind damage and small `magic_damage` — cast-support headwear.
**Loses:** armor/health piles. ~25 of 79 helmets currently carry armor and ~22 carry health; after the sweep, at most the plain-filler quota does.

**Migration rule:** a defensive helmet name doesn't move to chest — it gets re-expressed in helmet vocabulary (defense = Shield stacks + mind_resist, *bought with mana*, so it's economy all the way down).

**Example — `Stonewall Frame`** (today: an armor/health pile with no mana content):
```rust
// BEFORE (identity: chest's axis, wrong slot)
base: Stats { armor: 3, health: 25, ..Stats::ZERO },

// AFTER (same name — a wall of the mind, paid for in mana)
base: Stats { mana: 2, mind_resist: 15, ..Stats::ZERO },
triggers: &[Trigger::SpendMana {
    cost: 6,
    on_success: Action::GainShield(1),
    on_failure: Action::MindDamage { amount: 2, target: Target::Yourself },
}],
```

## 3.3 Chest (68 pieces) — Reserve. *The body: the only slot that answers "how long do I last."*

**Owns:** health (the majority home for any piece granting >15), armor per activation, regen, `physical_harden`/`magic_harden` (exclusive — hardening is a torso's job), and **`Grow` becomes chest-exclusive**: the fight-lengthening slot owns the only mechanic that rewards long fights. Today Grow is smeared 4/2/3/2/3 across slots; consolidate all of it here.
**Bleed → Helmet:** bases and layers that *hold* pools — the existing offenders `Mana Loom`, `Wellspring Base`, `Aether Layer` are not deleted, they're **reassigned to this bleed budget** (cap it at ~15 pieces); `Voidsilk Base` and `Hexweave Shroud`, whose content is casting rather than banking, get re-expressed or their identities traded to helmet designs.
**Loses:** nothing structurally — chest is closest to its axis already (40% armor, 38% health). The sweep here is mostly *amplification*: give it the harden monopoly, the Grow monopoly, and bigger reserve numbers to compensate for the armor that helmets/gloves give up.

**Example — new layer to absorb the Grow consolidation:**
```rust
PieceDef { name: "Patient Layer", slot: SlotKind::Chest, kind: PieceKind::Layer,
  base: Stats { health: 10, ..Stats::ZERO },
  triggers: &[Trigger::OnActivate(Action::Grow(3))], .. }
```

## 3.4 Gloves (75 pieces) — Reaction. *The hands: they act when other things act.*

**Owns (exclusive):** `OnAdjacentActivate`, `PerAdjacentItem`, `Drain`, `StunStrongest`, `DoubleAdjacentItemStat`; majority home of `OnAlignedActivate` and the `DoubleNeighbor`/`SelfPerNeighborKind` conversion effects. Keeps its faint strength/pierce lean — reaction *damage* is physical and small.
**Bleed → Greaves:** reactions whose payout is tempo — `OnAdjacentActivate(ReduceCooldown)`-style pieces.
**Loses:** plain stat piles. The eight named offenders (Spiked Vambrace, Ironhide Wrap, Breaker's Fist, Thornweald Grip, and the four plain rings) each either gain a reaction or shrink into the filler quota.
**Rings** are the natural reaction trinkets (0–2 per glove, `piece.rs:842`): every above-common ring should carry a reaction trigger or a conversion effect — a ring that just adds a stat is what commons are for.

**Example — `Spiked Vambrace`** (today: flat stats, no triggers, no effects — in the *reaction* slot):
```rust
// AFTER (same name — spikes answer contact)
base: Stats { strength: 2, ..Stats::ZERO },
triggers: &[Trigger::OnAdjacentActivate(
    Action::Damage { amount: 4, kind: DamageType::Physical, target: Target::Enemy },
)],
```

## 3.5 Greaves (51 pieces) — Tempo. *The feet: who moves, how often, and first.*

**Owns (exclusive):** `OnBattleStart` (the initiative mechanic — everything else starts a fight at zero; greaves are the gear that shows up already holding something), `speed_bonus` outside the weapon slot, `ReduceCooldown` outside the weapon slot. **Majority home of Frost/Stun/Misfire application** — the ~20 migrated weapon curse concepts land here as new Molds/Materials ("the slow is where you stand"). Keeps `curse_resist` lean — tempo defense.
**Bleed → Chest:** the recipe's optional Plating (`piece.rs:847`) is the armor bleed, already shipped; keep ~10–12 armor-flavored greaves inside this budget.
**Loses:** identityless health/regen padding (the eight named offenders each gain a tempo hook or shrink into filler).
**Smallest slot (51) and the one most under-built for its new axis** — expect this slot to need ~10–15 *new* pieces, which is where the migrated weapon curses go.

**Example — `Zealot's Sole`** (today: zero tempo content):
```rust
// AFTER (same name — arrives already devoted)
base: Stats { faith: 0, ..Stats::ZERO },
triggers: &[Trigger::OnBattleStart(Action::Gain { what: Resource::Faith, amount: 12 })],
```

---

# 4. Engine deltas (two tiers)

**Tier A — the catalog sweep** needs almost nothing from the engine (items 1–4 below). **Tier B — the Interaction Fabric (Part II)** adds four small, deterministic primitives (watchers, the diagonal relation, fusion pools, underlay) that the rewritten catalog then uses; those are specified in §11 and land *before* the slot sweeps so new pieces can be written against them. Item 4's "out of scope" list applies to Tier A only and is amended by Part II.

1. **Required: one new test file** `crates/engine/tests/catalog_shape.rs` that enforces §2's exclusivity table and quotas by iterating `CATALOG` — same philosophy as the existing rarity-distribution pinning in `rating.rs` ("so a batch of new components cannot quietly make everything legendary"; this test is "so a batch of new components cannot quietly dissolve a slot's identity"). **Write it first.** It will be red against today's catalog; the sweep is done when it's green.
2. **Optional, recommended: per-slot cadence tuning** in `default_cooldown_ms` (`piece.rs:860`) — cadence is a free identity lever: gloves tick fast (they react anyway), chest ticks slow and heavy, helmet middling, greaves middling-fast. Touch only after the sweep, measured against §7.
3. **Optional, only if baseline capture needs it:** per-slot damage attribution in the combat log (display/CLI only, no behavior change) so §7's "weapon damage share" is measurable. Check whether the log already attributes sources before writing anything.
4. **Explicitly out of scope:** new Trigger/Action variants, recipe changes (`recipes()` at `piece.rs:810` stays byte-identical), grid changes (all slots stay 6×8, `slot.rs:5-8`), curse constants (`curse.rs`).

---

# 5. What this does to balance (the actual fix for "weapon too strong")

The weapon's dominance is agency-share, not number-share. After the sweep: tempo denial lives on the feet, the combo game lives on the hands, survivability decisions live on the torso, and the resource engine that *feeds the weapon* lives on the head — so a weapon is only as good as the helmet paying for it, only as safe as the greaves slowing the enemy, and its raw numbers never changed. Four slots that used to answer "which stat pile" now each answer a question the fight actually asks.

---

# 6. File-by-file change map

| File | Work | Watch out for |
|---|---|---|
| `crates/engine/tests/catalog_shape.rs` (new) | §2 exclusivity + quotas, written first | Encode quotas as ranges, not exact counts, so tuning doesn't thrash the test |
| `crates/engine/src/piece.rs` | The sweep: ~444-entry audit; rewrite off-axis pieces in place; ~10–15 new greaves pieces; Grow/harden/Consume/etc. consolidation | Names are keys (directive 1). `cells:` shapes of any piece used in a packed named board must not change — the authoring tool re-pack is expensive |
| `crates/engine/src/rating.rs` | Effectiveness weights re-audit (`rating.rs:524` region): reaction triggers, OnBattleStart, curses, and Grow change worth when they change homes; re-pin the distribution tests | The rarity thresholds (`RARE_AT=90` etc.) should *not* move; re-pin the distribution by adjusting weights, not tiers, or item names (which grow with rarity) shift game-wide |
| `crates/engine/src/combat.rs` | No engine change. Re-audit all 50 `MonsterSpec.gear:` boards — monsters wear these pieces, so every rewritten piece silently rebalances a rung | After the sweep, replay the ladder in the CLI and compare per-rung TTK to baseline before touching `progression.rs` pins |
| `crates/engine/tests/progression.rs`, `effects.rs`, `reactions.rs`, `packing.rs`, `classes.rs`, `assembly.rs` | Re-pin with a one-line justification per changed constant | `packing.rs` covers the locked named boards; if it fails, a `cells:` shape changed — revert the shape, not the board |
| `crates/engine/src/shop.rs` | Pool routing audit: shelf pools and M4/M5 milestone pricing should surface each slot's new identity at the milestones where it matters | — |
| `crates/engine/src/loadout.rs` | Auto-build heuristic re-weight (it scores candidate boards; reaction/tempo pieces are worth more than their flat stats now) | — |
| `crates/engine/src/naming.rs` | Verify `action_word` coverage for the moved mechanics (Drain already has Bloodletting/Siphoning/Squandering at `naming.rs:178-180`; check `OnBattleStart` and `Consume` have words) | No structural change |
| `crates/engine/src/theme.rs` | Add turtle-theme entries for renamed/new canonical names; unchanged names need zero work (missing entries fall through by design) | — |
| `crates/engine/src/class.rs` | Audit `ClassDef.requires: &[(Axis, i32)]` (`class.rs:692`) and fountain scoring axes against the five vectors — if the existing `Axis` enum approximates them, adopt its vocabulary in the new test rather than inventing parallel names | — |
| GUI glossary / slot headers | One identity line per slot ("GREAVES — who moves, how often, and first") | Route through `theme.rs` vocabulary, not hardcoded strings |

---

# 7. Balance acceptance criteria (measure, don't vibe)

Capture **baseline first** with deterministic CLI replays (fixed seed, scripted reference builds at rungs 10 / 25 / 40 — build one melee, one caster, one hybrid reference):

1. **Weapon damage share** in reference builds falls to **66–76%**.

   *Re-derived, 2026-08-24.* This read 55–65% "expect baseline ~75–85%". The
   baseline was then measured and is **96.1%** — the estimate was twenty points
   low, because it was made before anything counted burn or mind damage and
   before anyone replayed the ladder. The band was never an independent target;
   it was "knock twenty to thirty points off what we think it is." Applied to
   the figure the game actually has, that same intent is 66–76%, and that is
   what this criterion asks for now. The old band is arithmetic on a premise
   that turned out to be wrong, and chasing it would have meant designing
   against a number nobody had ever measured.
2. **Slot-mattering test:** stripping any one non-weapon slot from a rung-25 reference build flips the outcome or regresses time-to-kill ≥25%. Today, stripping the helmet mostly costs stats; after, it should cost the build's engine.
3. **No-weapon viability:** a best-effort build with an empty weapon grid clears rung 15 (The Hollow King). This is the existence proof that the other four axes carry agency.
4. **Early game preserved:** rungs 1–10 TTK within ±20% of baseline (new players should feel nothing).
5. **Suite green** with every re-pinned constant documented in the commit message.

# 8. Rename propagation checklist (run per rename, and only rename when the name lies)

`grep -rn "OLD NAME"` across: `piece.rs` (quest `becomes` targets), `combat.rs` (monster `gear:` strings), `theme.rs` (canonical keys), `tests/` (assembly/packing/effects place pieces by name), `loadout.rs`/`shop.rs` (any curated lists), GUI strings. The assembly test exists to catch a missed one — trust it.

# 9. Suggested PR sequence (revised for Part II)

1. Baseline metrics harness + captured numbers (no gameplay change).
2. `catalog_shape.rs` written and red — §2 axis quotas **and** §10 interaction-density quotas encoded.
3. **Engine primitives, part 1** (§11): `Watch` counters + the diagonal relation + fusion pools, each with unit tests. No catalog changes yet.
4. **Engine primitives, part 2** (§11.4): underlay/overlap placement — isolated PR; it touches placement, packing, rating, loadout, and GUI, so it merges alone. (Deferral option in §11.4 if it balloons.)
5. Weapon exodus (curses → greaves concepts, reactions → gloves concepts) + the new greaves pieces.
6. Helmet + Chest sweep (§3.2, §3.3) — written against the new primitives.
7. Gloves + Greaves sweep (§3.4, §3.5) — `catalog_shape.rs` goes green here.
8. Monster boards replayed; `rating.rs` weights + shop pools re-pinned; progression re-pins.
9. Theme entries, naming coverage, glossary + tooltip templates; final §7 verification against baseline.

---
---

# PART II — THE INTERACTION FABRIC

# 10. Motivation, doctrine, and density quotas

**Measured problem.** Only 67 of 444 pieces (15%) carry *any* positional content (an `Adjacency` bonus, a positional `EffectKind`, or a reaction trigger), and 31 of those live in the weapon — the four armor slots hold just 36 interactive pieces between them (~13%). Meanwhile 74 pieces are pool-spend texture (`SpendMana`/`Spend`/`Consume`). The game's dominant verb is "pay a pool," when the interesting verb — the one the 6×8 grids exist for — is "stand in the right place next to the right thing."

**Doctrine — who owns which kind of interaction.** The five axes absorb the new fabric cleanly; each slot interacts *in its own tense*:

| Slot | Interaction tense | Vocabulary |
|---|---|---|
| Gloves | **Immediate** — answers events as they happen | `OnAdjacentActivate`, `PerAdjacentItem`, `OnAlignedActivate` (unchanged from Part I) |
| Helmet | **Accumulated** — counts, observes, converts | `Watch` counters (§11.1), the diagonal relation (§11.2), pool fusion (§11.3) |
| Chest | **Structural** — is the thing others rest on | Underlay/overlap (§11.4) |
| Greaves | **Sequential** — cares about order and cadence | `OnBattleStart` + `Watch{AlignedActivation}` cadence pieces |
| Weapon | **Cast-time** — as today | `OnOtherCast`, alignment, `PerAdjacentEmpty` |

*The hands answer; the mind counts; the body bears; the feet keep time.* The **fabric rule** stays from Part I: passive positional *stat effects* (`Adjacency{label, stats}`, `DoubleNeighbor`, `SoleIf`, `SelfPerNeighborKind`, `Flat(When)`) are pan-slot texture available everywhere — the trigger *families* above are what carry slot identity. Majority-home levels for the new primitives (added to §2's exclusivity table): `Watch` — majority helmet, minority anywhere; diagonal triggers — helmet and gloves only; fusion — **helmet-exclusive**; underlay — **chest and greaves only**.

**Density quotas** (already added to §2; encode in `catalog_shape.rs`):
1. ≥35% of each slot's above-common pieces carry an interaction.
2. Every Epic/Legendary non-weapon piece carries at least one — rarity buys interestingness, matching "names grow with what the item is worth."
3. Pool-spend triggers ≤15% of pieces per slot outside helmet. The helmet keeps the economy axis but expresses it increasingly through *fusion and watchers* rather than raw `Consume`.

# 11. The four new primitives (Tier B engine work)

All four are deterministic, integer-only, and small. Follow the existing same-tick resolution conventions in `combat.rs` (board order) for any payload that fires mid-tick.

## 11.1 `Watch` — event counters ("once N have been seen, do something")

```rust
// piece.rs — new Trigger variant
Trigger::Watch {
    what: Watched,       // which event stream this piece observes
    count: u32,          // fire the payload every `count` sightings
    then: Action,        // any existing Action
    repeats: bool,       // false = once per fight, true = every N
}

pub enum Watched {
    AnyActivation,        // any friendly item activates
    AdjacentActivation,   // an edge-neighbor activates
    DiagonalActivation,   // a corner-neighbor activates (§11.2)
    AlignedActivation,    // an item sharing this piece's rows activates
    CurseApplied,         // a curse lands on either side
}
```

State: one `u32` counter per watching piece instance, in combat state (reset per fight; counters tick *after* the observed event resolves; payload fires immediately after, in board order). Tooltip template: *"Every {count} {what}: {action}"* with a live *"7/10"* readout on the piece.

## 11.2 The diagonal relation ("non-adjacent, assembled item at distance 1")

Adjacency today is edge-sharing. Add the corner set: two placed cells at Chebyshev distance 1 that share **no edge**. Compute it exactly where edge-adjacency is computed today, cache it identically, and expose `Watched::DiagonalActivation` plus an immediate `Trigger::OnDiagonalActivate(Action)` (helmet/gloves only). Diagonals see *past* neighbors — perception is helmet flavor. Edge cases: board borders (fewer diagonals, fine), and the relation is between *items* (any cell-pair qualifies the item pair once — no double counting per shared corner).

## 11.3 Fusion pools ("druidic might")

The three passive pools have exact per-point rates in `combat.rs:3029-3037`: rage → +1 physical_damage, faith → +2 physical_resist and +2 magic_resist, nature → +1 regen. A **fused pool** is a new `Resource` variant whose passive is *both parents at double rate*:

| Fusion | Parents | Passive per point |
|---|---|---|
| **Druidic Might** | Nature + Rage | +2 physical_damage, +2 regen |
| **Communion** | Faith + Nature | +4 physical_resist, +4 magic_resist, +2 regen |
| **Zealotry** | Rage + Faith | +2 physical_damage, +4 physical_resist, +4 magic_resist |

```rust
// piece.rs — extend Resource; add the converting Action (helmet-exclusive)
pub enum Resource { Mana, Rage, Faith, Nature, DruidicMight, Communion, Zealotry }

Action::Fuse { a: Resource, b: Resource, into: Resource }
// consumes 1 of `a` and 1 of `b` if both held; gains 1 of `into`; else does nothing
```

Rules: mana is fuel, not a passive — it does not fuse in v1 (stretch: mana-fusions that double empowerment/shield efficiency). Fused pools are products, not fuel: not valid for `Spend`/`Consume`/`SpendMana`, but they **are** valid `Drain` targets (counterplay: gloves can drink someone's Druidic Might). Caps mirror whatever the parent pools do today. `Resource::ALL` grows to 7 — audit every exhaustive match (`naming.rs` resource words, GUI chips, rating).

## 11.4 Underlay — overlap as terrain ("for each core overlapping this piece")

The one big-ticket item, merged as an isolated PR:

```rust
// PieceDef gains:
pub underlay: bool,   // default false
```

Semantics kept deliberately narrow: an underlay piece is **always loose** (never part of an assembled item — it is terrain, not a component); its cells **may be shared** by normal pieces; underlays never overlap each other; one layer deep. Payloads read what covers them:

```rust
EffectKind::PerOverlappingItem { stat: StatKind, amount: i32 }  // per distinct item covering ≥1 cell
EffectKind::PerOverlappingCore { stat: StatKind, amount: i32 }  // count only recipe cores
```

Touch list (why it merges alone): placement validation, `loadout.rs` auto-build (place underlays first), `packing.rs` + the authoring tool (named boards must not cover underlays unless authored to), `rating.rs` (value by expected coverage), GUI (draw beneath, hatch covered cells, tooltip "*per core overlapping: …*"). **Deferral option:** if this balloons, approximate the three underlay pieces with `SelfPerNeighborKind`-style adjacency instead and revisit — the doctrine survives without it, but the user asked for overlap, so treat deferral as a fallback, not the plan.

# 12. The asked-for pieces, spec'd

The three examples from the design brief, as real catalog entries (canonical names; turtle-theme display names in parentheses go in `theme.rs`):

```rust
// 1 — the accumulator (helmet crest; "Tallykeeper's Crest" / theme: "Sherman's Tally")
PieceDef { name: "Tallykeeper's Crest", slot: Helmet, kind: Crest,
  base: Stats::ZERO,
  triggers: &[Trigger::Watch { what: Watched::AnyActivation, count: 10,
      then: Action::GainEmpowerment(2), repeats: true }], .. }

// 2 — the fusion frame (helmet core; "Communing Frame" / theme: "FrogDog's Frame" —
//     an amphibian-canine is nature+rage in one body; the book hands us the name)
PieceDef { name: "Communing Frame", slot: Helmet, kind: Frame,
  base: Stats { mana: 1, ..Stats::ZERO },
  triggers: &[Trigger::Watch { what: Watched::DiagonalActivation, count: 1,
      then: Action::Fuse { a: Resource::Nature, b: Resource::Rage,
                           into: Resource::DruidicMight }, repeats: true }], .. }
// exactly the brief: every time a non-adjacent assembled item at distance 1 goes off,
// 1 nature + 1 rage become 1 Druidic Might (+2 phys damage, +2 regen — both parents, doubled)

// 3 — the overlap keystone (chest base, underlay; "Keystone Base" / theme: "The Unmovable Rock")
PieceDef { name: "Keystone Base", slot: Chest, kind: Base, underlay: true,
  base: Stats { health: 10, ..Stats::ZERO },
  effect: Some(Effect { kind: EffectKind::PerOverlappingCore {
      stat: StatKind::Power, amount: 10 },   // power is hundredths (stats.rs:6): +10 = +0.10x
      when: When::Always,
      // payload scope: items sharing rows with this piece (the aligned set)
  }), .. }
```

One more per slot to seed the sweep's range: greaves **"Cadence Mold"** — `Watch{AlignedActivation, 4, ReduceCooldown(slowest aligned item, 800ms), repeats}` (the feet keep time); gloves **"Answering Ring"** — `OnAdjacentActivate(Damage{3, Physical, Enemy})` (the hands answer); weapon accessory **"Patient Fuse"** — `Watch{AnyActivation, 15, GainForking(1), once}` (a cast-time payoff for a long fight, and the weapon's legal minority use of Watch).

# 13. Rating, GUI, and test impact of the primitives

- **`rating.rs`:** value `Watch` as `payload_value × expected_fires`, with expected_fires derived from a stated board-cadence assumption (document the constant; something near "one friendly activation per second" — calibrate against the baseline replays, not intuition). Fusion pieces are worth roughly the fused passive delta over parents, times expected conversions. Underlay by expected coverage (assume 2 covering items unless quest-flagged). Re-pin the rarity distribution *by weights, not thresholds* (Part I rule).
- **GUI:** counter readouts on watching pieces, three new pool chips, underlay rendering, and tooltip templates per primitive — all strings routed through `theme.rs` vocabulary.
- **Tests:** unit tests per primitive (counter determinism across identical replays; diagonal set correctness at borders; fusion no-op when a parent is missing; underlay placement legality and that underlays never join items); `catalog_shape.rs` gains the §10 density assertions; one integration replay proving a Watch-heavy board produces identical logs across two runs of the same seed.
