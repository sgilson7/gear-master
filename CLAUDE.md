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
| `piece.rs` | ~9,600 | Everything a piece is: `PieceDef`, `PieceKind` (including `Enchantment`, the layer under the grid), `Trigger` (`Watch` among them), `Action` (`Fuse` among them), `EffectKind`, `Adjacency`, `Resource` (8 — three of them fused, and Insight), recipes (:810), per-slot default cooldowns (:860), and the **504**-entry `CATALOG` (:960) |
| `combat.rs` | ~5,350 | The fight: tick loop, hit math, typed damage, reflection, curses in effect, `MonsterSpec` + all creature boards (**69**: `LADDER` 50, `ALTERNATES` 19 for dungeon floors, event fights and rung 51, `CREVICE` empty), `Difficulty {Easy, Medium, Hard, Insane}` |
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
downstream). **764 tests, green, no warnings.** When one fails after your
change, it is telling you which doctrine you brushed.

---

## 6. WHERE THINGS STAND

**The Unwinding is finished and merged.** Twenty milestones, M0 to M19: the
event chain across the back half of the ladder, the super boss at rung 51,
three hidden towns, six dungeons, the four Orbs of Travel, the third combat
lane, the reward vocabulary, the road stack, the route map. `HANDOFF.md` is the
summary and `HANDOFF-unwinding.md` is the milestone-by-milestone record;
`design/the-unwinding.md` carries three reconciliation blocks and amendments
numbered to 23, and those win over its body wherever they disagree.

**The suite is 764 green, no warnings**, and the frame lint is at zero: every
creature in the game has a board.

**What is open, in the order it matters:**

1. **The fifteen new creatures wear generated boards.** They were packed by
   `tests/pack_francis.rs` at the rung each is met on - correctly sized, shaped
   by theme, and samples rather than authored fights. The owner is rebuilding
   them by hand in `make pack`. Nothing depends on that happening; the boards
   are legal and measured.
2. **The fourth reference build does not beat THE UNWOUND.** Two of the three
   shipped boards lose to it and the third wins at 28 seconds, which is the
   acceptance criterion. What is missing is a board that wins *because* of
   Deflection and Insight - the demonstration that the mind lane answers the
   thing at the top.
3. **Engraving and the Brain Farm slipped**, on measured cost, at the Phase-1
   gate. Amendment #20 says what would unblock Engraving: it is the only thing
   in the mission that reopens `share.rs`'s index-keyed format.
4. **Nobody has played this.** Every claim in either handoff comes from the
   test suite and from two CLI replays that diff clean.

**The traps, still true, in the order they will find you:**

1. **`CATALOG` is index-keyed by `share.rs`. Append-only for ever.**
2. **`stepped_component` re-gears every monster on Easy, Hard and Insane
   whenever a `rating.rs` weight moves** - 33 boards on Easy, last time. Settle
   the weights before authoring anything measured against them.
3. **The reconstruction fault.** A dense board does not come back as the items
   its owner built unless each is locked as it assembles. Every rebuild goes
   through `common::board_from`, and a hand-seated name list is not a board -
   this repo has learned that four times, most recently in M18.
4. **Sudden death owns everything past 30s.** THE UNWOUND is authored to 28.0s
   at Medium and there is no room above it.
5. **Enchantments are town stock.** Never on the road.
6. **`LadderEvent::at` and `Town::after` are zero-based; the displayed rung is
   `at + 1`.** `LADDER` is fifty because `Rust Golem` is spliced in by name.
7. **Names are string keys** across `theme.rs`, monster boards, quests,
   `event.rs`, `rumour.rs`, `town.rs`, `dungeon.rs` and the tests.
8. **The base game does not speak turtle.** A proper noun out of the book
   belongs in `theme.rs`; the canonical column names the role.
   `tests/two_voices.rs` is the ratchet and its budget is five, all of them
   piece names that cannot be changed.
9. **A guard that refuses your change is usually right** - and the packer's
   refusals are gradients. "wanted 11.8s, best was 8.0s" is a ratio to scale by.

**Running the suite.** 46 test binaries, and every engine edit relinks all of
them. Iterate with `cargo test -p gearmaster-engine --lib` (0.13s) or one
`--test <name>`; run the whole thing once, at the end. `[profile.test]` carries
line tables only - for a full backtrace on one run,
`CARGO_PROFILE_TEST_DEBUG=2 cargo test ... --test <name>`. Never start a second
cargo while one is running.

## 7. Etiquette

Match the module doc-comment voice (deadpan, first principles, one idea per
paragraph) — the codebase reads like it was written by one careful person,
and it should stay that way. Keep the engine free of graphics dependencies.
Never let a themed string reach game logic. And when a design document and
the code disagree, the document is right and the code has a bug report.
