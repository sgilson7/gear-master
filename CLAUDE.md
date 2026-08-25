# CLAUDE.md — Gear Master, for a fresh agent

You are working on **Gear Master**: a deterministic, browser-playable
puzzle-autobattler written in Rust. Five gear grids, polyomino pieces, a
fifty-rung ladder of creatures, and a final boss named Francis. The player's
job is packing boards; the engine's job is making every fight a pure function
of what was packed.

Read this file top to bottom once. Then read
`design/the-unwinding.md` — that is **the mission** (§6, bottom of this
file). Nothing in it has been executed. The previous mission — the gear-slot
rewrite, `design/gear-slot-basis-rewrite.md` — is **finished and deployed**;
`HANDOFF.md` is its record and its habits section is worth your time.

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
`slash_and_burn`, `baseline` (the measurement harness — `#[ignore]`d
printers report damage share by slot), `catalog_shape` (the slot-identity
ratchet: budgets only go down), `fixtures` (the manifest of tests that name a
piece as their example of a mechanic, so a sweep fails there rather than
downstream). **548 tests, green, no warnings.** When one fails after your
change, it is telling you which doctrine you brushed.

---

## 6. THE MISSION — The Unwinding

**What came before, in one paragraph.** The gear-slot rewrite is done and
live: each slot owns a basis vector (Weapon **Conversion**, Gloves
**Reaction**, Greaves **Tempo**, Chest **Reserve**, Helmet **Economy**), the
weapon's side-monopolies moved out (gloves hold 47 reaction triggers to its
2), the interaction primitives ship (`Watch`, the diagonal relation, three
fused pools, the **Enchantment** layer, reflection), `catalog_shape`'s
ratchet closed from 69 rules unmet to green, and the weapon's damage share
sits at **75.2%** inside the 66–76% band. `analysis/baseline.md` holds every
number; `HANDOFF.md` §5 and §7 hold the lessons.

**The mission now** is `design/the-unwinding.md` — read it in full,
including its **two** dated reconciliation blocks, which win wherever they and
the body disagree; the second is dated at the start of execution and wins over
the first. Short form: an overarching event chain across the back half
of the ladder that ends with a super boss at **rung 51**, unlocked only by
finishing the chain and beating Francis; two hidden towns and five mini
dungeons; four Orbs of Travel and their destinations; typed combat lanes
(empowerment/shield become magic-only, with **Spellblade** and
**Deflection** as their physical twins); **Insight**, an eighth pool that is
to mind damage what mana empowerment is to magic, locked behind a dungeon;
five unconditional road events; a reward vocabulary (row grants, claim
tickets, run-relics, crushable rule-benders, standing orders); the **road
stack** that resolves everything standing on a rung; receipts and tooltips
that describe themselves from the engine; and a Star Fox-style **route
map** rendered purely from run state. Execution is phased and the phasing
is the point: **all engine work first** (Part E, Phase 1), **all creatures
authored as frames** — name, band, theme, note — through Phase 2, theme in
Phase 3, and **no board authored until Phase 4**, by hand, in `make pack`.

**The traps, in the order they will find you:**

1. **`CATALOG` is index-keyed by `share.rs:87`. Append-only for ever.** New
   pieces go on the end; nothing moves, nothing is deleted.
2. **`stepped_component` (`combat.rs:252`) re-gears every monster on Easy,
   Hard and Insane whenever a `rating.rs` weight changes** — and the mission
   adds weights (Spellblade, Deflection, Dread, Insight income, new
   outcomes). Consequence, already folded into the spec: Phase 4 re-pins
   rating **before** any board is authored, never after.
3. **The reconstruction fault** (`HANDOFF.md` §5): a dense board does not
   come back as the items its owner built unless each item is locked as it
   assembles — every reconstruction goes through `common::board_from`. The
   Claim Ticket's whole-board drop and the pedestal's returns are exactly
   this fault's shape; build them on `board_from` from the first commit.
4. **Sudden death owns everything past 30s**, and the difficulty band's top
   edge at rung 50 is 29.1s. THE UNWOUND at rung 51 must be authored to
   finish inside the measurable region, or the fight is decided by the
   clock rather than the board. "Harder than Francis" is measured at
   **Medium** — the open question of Francis-on-Hard (`HANDOFF.md` §4, M1)
   stays open and uncoupled.
5. **Enchantments are town stock** — ground is bought where somebody has a
   floor to sell, never off the road. The mission's two enchantment rewards
   (the Lightning Rod, Aisle 9's stock) already live in town shelves; keep
   it that way.
6. **There is no milestone pricing and no auto-builder.** Gold figures in
   the spec anchor against real `SHELF_TILT` shelves and rung bounties;
   reference builds for acceptance replays are authored presets in the
   `apply_preset` mould.
7. **Names are string keys** across `theme.rs`, monster boards, quests,
   `event.rs`, `rumour.rs`, `town.rs`, `dungeon.rs`, and the tests. Grep
   before and after. Grids are 6×8 **base** and can gain rows; legality
   runs against current dims.
8. **`ALTERNATES` and the empty `CREVICE` are the frame precedent** —
   creatures without authored boards already exist in the repo. The
   mission's frames extend that pattern rather than inventing one.
9. **`LadderEvent::at` and `Town::after` are zero-based indices; the
   displayed rung is `at + 1`.** And `LADDER` is fifty because `Rust Golem`
   is spliced in by name at rung 4 rather than written inline, so counting
   the table by eye comes back one short.

**The mission is under way.** It is being executed on the branch
**`unwinding`**, milestone by milestone, and `HANDOFF-unwinding.md` is the
running ledger — read it before anything else, because it says which milestone
is open and what the last one moved. `analysis/baseline.md`'s *Before the
Unwinding* section is the denominator every figure since is measured against.

**Your first three moves:** (1) run the suite — 548 green, and if not, that
is the news; (2) run the two printers to capture today's numbers
(`--test baseline -- --ignored --nocapture --test-threads=1` and
`--test catalog_shape -- --ignored`), because they move under you; (3) read
`design/the-unwinding.md` Part E, then **both** reconciliation blocks — the
second one is dated at the start of execution and wins over the first — then
pick up whichever milestone `HANDOFF-unwinding.md` says is open.

---

## 7. Etiquette

Match the module doc-comment voice (deadpan, first principles, one idea per
paragraph) — the codebase reads like it was written by one careful person,
and it should stay that way. Keep the engine free of graphics dependencies.
Never let a themed string reach game logic. And when a design document and
the code disagree, the document is right and the code has a bug report.
