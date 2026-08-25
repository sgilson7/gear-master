# CLAUDE.md — Gear Master, for a fresh agent

You are working on **Gear Master**: a deterministic, browser-playable
puzzle-autobattler written in Rust. Five gear grids, polyomino pieces, a
fifty-rung ladder of creatures, and a final boss named Francis. The player's
job is packing boards; the engine's job is making every fight a pure function
of what was packed.

Read this file top to bottom once. Then read
`design/gear-slot-basis-rewrite.md` — that is **the mission** (§Mission,
bottom of this file). It is **partly executed**: the engine work is done, the
catalogue work is not. `HANDOFF.md` says what is left.

---

## 1. Orientation in five minutes

```
cargo test -p gearmaster-engine          # the whole safety net (~27 suites)
cargo run  -p gearmaster-cli             # headless REPL: play the real game in a terminal
cargo run  -p gearmaster-gui             # macroquad GUI (native window)
make pack                                # board packer: dress creatures by hand
# docs/ holds the published wasm web build (index.html + gearmaster.wasm)
```

CLI REPL verbs (the same engine the GUI drives): `help`, `show [slot]`, `inv`,
`stats`, `equip <name> <slot> <x> <y>`, `unequip <name>`, `rotate <name>`,
`preset`, `clear`, `sandbox`, `shop`, `buy <n>`, `sell <name>`, `ladder`,
`items`, `fight`, `quit`. A scripted run piped into stdin replays identically —
that is not a convenience, it is the design contract.

**Workspace:** `crates/engine` (all rules, no graphics), `crates/cli`,
`crates/gui`. `make pack` runs the GUI in packing mode (`gui/src/pack.rs`):
the same screen, editing a creature's board instead of yours, with a free
shop over the whole catalogue and a save that writes back into `combat.rs`. `design/` holds living design documents — and the repo's stated
rule is *"code follows this document, not the other way round — when they
disagree, this is the bug report"* (`design/branching-events.md`).

---

## 2. The four doctrines (violate none of them)

1. **Determinism is load-bearing.** Combat consults no RNG anywhere —
   `combat.rs` is a pure function of the two boards. The engine owns one tiny
   seeded PRNG (`rng.rs`) for out-of-combat things like shop stock, seeded per
   run so tests replay. Even "50% chance to miss" (the Ticket to Ride class)
   is implemented deterministically. Share codes, the balance story, and half
   the test suite depend on this.
2. **Canonical names are string keys.** Piece and monster names key the theme
   layer (`theme.rs`), monster gear boards (`combat.rs`), quest `becomes`
   targets, event/rumour conditions, and dozens of tests. Renaming a piece
   without propagating is the classic repo mistake; the assembly test exists
   to catch it. Grep before and after.
3. **Tests pin behavior on purpose.** Distribution tests pin the rarity curve
   "so a batch of new components cannot quietly make everything legendary";
   progression tests pin fight outcomes. When your change moves a pinned
   number, re-pin it *with a one-line justification in the commit* — never
   loosen a test to make it pass.
4. **A theme cannot break the game.** `theme.rs` is display-only lookup;
   missing entries fall through to canonical names. Never route game logic
   through themed strings.

---

## 3. Engine map (`crates/engine/src/`)

| Module | Lines | Owns |
|---|---:|---|
| `piece.rs` | ~9,600 | Everything a piece is: `PieceDef`, `PieceKind` (including `Enchantment`, the layer under the grid), `Trigger` (`Watch` among them), `Action` (`Fuse` among them), `EffectKind`, `Adjacency`, `Resource` (7 — three of them fused), recipes (:810), per-slot default cooldowns (:860), and the **473**-entry `CATALOG` (:960) |
| `combat.rs` | ~5,350 | The fight: tick loop, hit math, typed damage, reflection, curses in effect, `MonsterSpec` + all creature boards (**54**: `LADDER` 50, `ALTERNATES` 4 for dungeon floors and event fights, `CREVICE` empty. The file holds 57 `MonsterSpec` tokens; two are the struct and its `impl`), `Difficulty {Easy, Medium, Hard, Insane}` |
| `run.rs` | ~2,060 | A run: `Mode {Grinder, Rogue}` (knock-back farming vs three lives), gold, rung, fountains, lives, `best_fight_ms`, scenes seen, towns visited, the theme in use, and `apply_preset` |
| `theme.rs` | ~1,400 | The turtle theme: names, story, cutscenes, vocabulary, glossary — all display-only |
| `class.rs` | ~1,200 | `ClassDef { name, blurb, requires: &[(Axis, i32)], power }`; fountains score your build on axes and hand out classes; stacking classes (Piety → Ticket to Ride) |
| `rating.rs` | ~900 | Item worth: effectiveness scale, price, and rarity — `RARE_AT=90, EPIC_AT=130, LEGENDARY_AT=170` (:203) |
| `loadout.rs` | ~920 | Boards, placement, assembly, `lock_assembled_in`. **No auto-builder** — the nearest thing is `Run::apply_preset`, twenty-two hard-coded placements (twenty-one pieces and one bonded enchantment), which is also a reference build the baseline is measured against |
| `event.rs` | ~760 | Events: stand in front of a rung, ask a question, never resolve themselves; adding one = adding to `EVENTS` |
| `naming.rs` | ~700 | Generated item names: earned qualifier + hash-stable base + suffix; **names grow with rarity** — Common 3 words, Rare 4, Epic 5, Legendary 6 |
| `stats.rs` | ~480 | `Stats`; note `power` is a multiplier in **hundredths** (`power: 250` = 2.50x) |
| `slot.rs` | ~450 | Grids: `SLOT_W`×`SLOT_H` = 6×8 **base** — boards can be *granted extra rows* as rewards, and resizing must never move a placed piece (`tests/taller_boards.rs`) |
| `curse.rs` | ~400 | Searing (damage over time), Frost (slows gear, capped), Stun (one item), Misfire (every Nth activation fizzles) — all deterministic |
| `shop.rs` | ~370 | Shelves dealt a slot at a time (`SHELF_TILT`), reroll, and a repair that guarantees a buildable weapon. **No milestone pricing** — the mission asks for it and it does not exist. Town stock and enchantments are excluded from the road's shelves and sold only in towns |
| `share.rs` | ~300 | Build share codes: base-32, a *record* of a board, not a save file |
| `rumour.rs` | ~240 | Rumours: 1-cell components that are *conditions*, not gear — they sit in the tray and unlock events |
| `town.rs` | ~215 | Towns: rungs with nothing to fight — three pinned (after rungs 6, 17, 31), one action per visit, or walk on for the bounty again |
| `dungeon.rs` | ~140 | Side fight-chains ending in classes you cannot get elsewhere; exiting puts you back where you entered |
| `shape.rs`, `rng.rs`, `glossary` etc. | small | Polyomino math; the seeded PRNG; words |

`design/towns.md` and `design/branching-events.md` are the intent documents
for the newest systems — read them before touching towns, events, rumours, or
dungeons.

---

## 4. The game, mechanically

**The road.** Fifty creature rungs, three bosses, seven mini-bosses; three
towns *between* rungs (a run that enters all three stands on 53 rungs); events
stand *in front of* rungs; dungeons stand *beside* them; nothing on the road
gets walked past (`tests/the_road.rs`). Fountains appear before each boss and
score the build on axes to grant classes; the third can double a class.
Named creatures leave their gear behind. A town sells five curated components
and every **enchantment** — the layer under a grid is bought where somebody has one to sell,
never off the road.
A sharp early build (a kill under 2s, rungs 1–10) opens the casino, once.

**A fight.** Both sides' boards tick in 50ms steps. Items activate on
cooldown (piece `cooldown_ms`, else the slot default at `piece.rs:860`). A
hit is `(flat damage + strength) × power`, typed **physical** or **magic**;
the defender answers with the matching `*_resist`, punched through by
`*_pierce`, shored up by `*_harden`. Armor absorbs first and **resets to zero
every fight**. Regen heals per second; `Grow` raises max health mid-fight;
mind damage *lowers* max health and cannot be healed. Curses stack by kind
with caps and floors. Stalemates go to the full clock.

**Pools.** Mana is fuel (spent by `SpendMana`/`Spend`/`Consume` triggers;
empowerment and shield scale off it). The other three are passive holdings
with exact per-point rates (`combat.rs`): rage → +1 physical damage, faith →
+2 physical *and* +2 magic resist, nature → +1 regen. `Drain` steals pools.

**A piece** (`PieceDef`): name, slot, kind, polyomino `cells`, base `Stats`,
optional `Adjacency { label, stats }`, optional positional `Effect`
(`DoubleNeighbor`, `SoleIf`, `SelfPerEmptyCell`, `SelfPerNeighborKind`,
`DoubleAdjacentItemStat`, `Flat(When)` — `When::NotAssembled` powers
deliberately-loose gear), triggers, cooldown, price, `power_bonus`/
`speed_bonus`, and sometimes a `Quest` (the piece *becomes* another piece
when its condition is met).

**Assembly.** Loose pieces contribute passive stats; pieces connected into a
**recipe** become an *item* that acts in combat. Recipes (`piece.rs:810`):

| Slot | Recipes |
|---|---|
| Weapon | Handle + 1–2 Damaging + 0–2 Accessory · Book + Ink + Spell + 0–1 Accessory · Orb + 2–3 Spells + 0–1 Alignment |
| Helmet | Frame + 1–2 Plating + 0–1 Crest |
| Chest | Base + 1–3 Layers |
| Gloves | Material + Mold + 0–2 Rings |
| Greaves | Material + Mold + 0–1 Plating |

**Worth.** `rating.rs` scores a board; rating sets price and rarity; rarity
sets the generated name's length (3/4/5/6 words). Adjust worth by weights,
never by moving the rarity thresholds — every item name in the game shifts if
you touch those.

---

## 5. The test suite is the map of what matters

`assembly` (names place correctly — catches renames), `packing` +
`pack_francis` (the authoring tool's locked named boards still pack),
`progression` + `the_long_way` + `two_runs` (whole runs played end to end),
`effects`/`reactions`/`drains`/`curses_in_combat` (per-mechanic), `fight` /
`sudden_death` / `brawl` (combat edges), `francis` (the man himself),
`classes` + `class_reaches_combat`, `prices`, `towns` / `casino` / `vip` /
`earned_events` / `the_road` (road furniture), `taller_boards` (resize moves
nothing), `decode_build` (share codes), `prose` (the words), `avail`,
`slash_and_burn`, `two_runs`. When one fails after your change, it is telling
you which doctrine you brushed.

---

## 6. THE MISSION — the gear-slot rewrite (part done)

Read `design/gear-slot-basis-rewrite.md` in full, amendments and all. Short form:

The weapon was the only slot with mechanical identity — it owned damage *and*
hoarded the curse game and the reaction game, while Helmet/Chest/Gloves/Greaves
were one stat-pile slot wearing four shapes (0.93 cosine similarity between
helmet and chest). The rewrite gives each slot a basis vector — Weapon
**Conversion**, Gloves **Reaction**, Greaves **Tempo**, Chest **Reserve**,
Helmet **Economy** — with a directed bleed cycle, an exclusivity table, per-slot
quotas, and an **Interaction Fabric** (Part II): `Watch` counters, the diagonal
relation, fusion pools (Druidic Might / Communion / Zealotry), and
and the enchantment layer under every grid.

**What is done.** The engine half. All five primitives ship, reflection among
them, and the catalogue has moved a long way: gloves hold 47 reaction triggers
against the weapon's 2, greaves hold 26 curse applications against the weapon's
20, and the share of the catalogue that is inert has gone from 44% to 21%.

**What is not, and what since became so.** The catalogue half was the larger
half and it is done: `cargo test -p gearmaster-engine --test catalog_shape --
--ignored` was **69 rules unmet** and is **green**. Every mechanic the
exclusivity table names is in its slot, every axis quota is in band on all five
slots, no floating kind carries an identity mechanic, and the weapon deals
**74.9%** of a finished board's damage against a 66–76% band. `analysis/baseline.md`
is every number and how it moved.

What is left is on the creatures rather than in the catalogue. **Seventeen of
fifty rungs sit outside the difficulty band**, and the last six are the ones that
matter: rungs 45–50 all finish past the 30s where sudden death takes the fight
over, because their *stat blocks* put them there and `pack_francis` authors a
board rather than a creature. The repack was halted as too slow — see
`analysis/second-order.md` — and creature boards are to be hand-authored with a
build tool instead. `HANDOFF.md` is the ledger.

The spec now carries its own dated amendments inline — read them where they
sit rather than trusting the line above each one. What follows is the short list
of things a fresh agent gets wrong first.

**Corrections to the spec, discovered since it was written** (the spec is
older than the repo; these amendments win):

1. Spec §4 says "grids stay 6×8" — grids are 6×8 **base** and can gain rows
   (`taller_boards.rs`). `catalog_shape.rs` and the enchantment layer must
   tolerate resized boards; placement legality runs against the board's
   *current* dims, not the constants.
2. The rename-propagation checklist (spec §8) now also includes `event.rs`,
   `rumour.rs`, `town.rs`, and `dungeon.rs` — events can require carried
   items and rumours are catalog-adjacent components with names.
3. **Share-code stability:** `share.rs` encodes a piece as its **`CATALOG`
   index** (`share.rs:87`). Checked, not assumed. So the catalogue is
   append-only for ever: nothing is reordered, nothing is deleted, and a sweep
   rewrites a piece in place under its own name. Every code ever pasted into a
   chat depends on it and `decode_build.rs` will say so.
4. `Difficulty` lives in `combat.rs`, not `run.rs`. Monster count is **54**
   creatures (`LADDER` 50 + `ALTERNATES` 4) — the board re-audit in spec §6
   covers all of them, not just the ladder. Note `stepped_component`
   (`combat.rs:252`) picks a creature's gear on Easy/Hard/Insane by walking a
   footprint family sorted by `piece_rating`, so **any change to `rating.rs`
   weights silently re-gears every monster on three of the four settings**.
5. The Recycler spell ("spends a harvest") post-dates the census — pool-spend
   texture counts in spec §10 should be re-measured, not trusted, before the
   quotas are pinned into `catalog_shape.rs`.

**Your first three moves:** (1) run the suite green — it should be, and if it is
not, that is the news; (2) capture the current numbers, because they move under
you (`--test baseline -- --ignored --nocapture --test-threads=1`, and
`--test catalog_shape -- --ignored` for the distance left); (3) read `HANDOFF.md`
§5, which is the fault that ran unnoticed for the whole rewrite and is the shape
of the next one. Then work the milestones in `HANDOFF.md`.

---

## 7. Etiquette

Match the module doc-comment voice (deadpan, first principles, one idea per
paragraph) — the codebase reads like it was written by one careful person,
and it should stay that way. Keep the engine free of graphics dependencies.
Never let a themed string reach game logic. And when a design document and
the code disagree, the document is right and the code has a bug report.
