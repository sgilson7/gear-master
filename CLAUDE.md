# CLAUDE.md — Gear Master, for a fresh agent

Written against commit `41e2180` (2026-08-27), at the end of the Switchyard's
M10 - the mission is finished on branch `switchyard` and **not yet merged**.
Every count below was read off that tip; if `git log --oneline -1` says
something else, the numbers are quotes and the printers in §5 are the
measurements.

You are working on **Gear Master**: a deterministic, browser-playable
puzzle-autobattler in Rust. Five gear grids, polyomino pieces, a fifty-rung
ladder of creatures, a final boss named Francis, and a fifty-first rung behind
him. The player's job is packing boards; the engine's job is making every
fight a pure function of what was packed.

Read this file top to bottom once. Then read `design/rl-agent-plan.md`, which
is **the mission** (§6), and `HANDOFF-solver.md`, which is the brief for it.

Four missions are finished. Three are deployed; the fourth is on a branch:

- **the gear-slot rewrite** - `design/gear-slot-basis-rewrite.md`, and
  `design/HANDOFF.md`'s predecessor as its record.
- **the Unwinding** - `design/the-unwinding.md`, recorded in
  `design/HANDOFF.md` and `design/HANDOFF-unwinding.md`.
  `design/post-unwinding.md` audits it against the code and is more recent
  than either handoff.
- **the prose pass** - `HANDOFF-prose.md` is the brief and
  `design/HANDOFF-prose-ledger.md` is what it did.
- **the Switchyard** - `design/the-switchyard.md` is the spec,
  `design/HANDOFF-switchyard.md` the decisions and
  `analysis/switchyard.md` every measurement, one block a milestone headed by
  the commit it was read off. **On branch `switchyard`, eleven commits, not
  merged and not published.**

---

## 1. Orientation in five minutes

```
cargo test -p gearmaster-engine          # the safety net: 866 tests, 51 binaries + lib, ~35s
cargo test -p gearmaster-gui             # 65 more; cargo build does NOT compile them
cargo test -p gearmaster-cli             # 5 more: a scripted run, piped in twice
cargo run  -p gearmaster-cli             # headless REPL: play the real game in a terminal
cargo run  -p gearmaster-gui             # macroquad GUI (native window)
make pack                                # board packer: dress creatures by hand, saves into combat.rs
# docs/ holds the published wasm web build (index.html + gearmaster.wasm)
```

**Toolchain.** `Cargo.toml` says `rust-version = "1.83"` and that is now the
truth: the code needs 1.83 for `Option::is_none_or` and for const references to
statics in `event.rs`. It said 1.75 for three missions and the Switchyard's M0
fixed the declaration. This machine builds it under **1.95**, warning-free.

CLI REPL verbs (the same engine the GUI drives): `help`, `show [slot]`, `inv`,
`stats`, `equip <n> <slot> <x> <y>`, `unequip <n>`, `rotate <n>`, `preset`,
`clear`, `sandbox`, `shop`, `buy <n>`, `sell <n>`, `ladder`, `items`,
`fight`, `road`, `map`, `answer <n>`, `town`, `town on`, `town <door>`,
`drink`, `throw <n>`, `leave`, `quit`. A scripted run piped into stdin
replays identically - that is the design contract, and since the Switchyard's
M3 it is also a test (`crates/cli/tests/replay.rs`), which it had never been.

**Workspace:** `crates/engine` (all rules, **zero dependencies** -
`crates/engine/Cargo.toml` has an empty `[dependencies]` and a comment saying
why), `crates/cli` (595 lines), `crates/gui` (macroquad; `main.rs` 12,903
lines, `pack.rs` 627). `design/` holds the living design documents and the
repo's rule is *"code follows this document, not the other way round - when
they disagree, this is the bug report"*. `analysis/` holds measurements.
`.claude/skills/gearmaster-gear` is the checklist for adding a piece.

---

## 2. The four doctrines (violate none of them)

1. **Determinism is load-bearing.** Combat consults no RNG anywhere -
   `combat.rs` is a pure function of the two boards (`simulate_party`,
   `combat.rs:3862`). The engine owns one seeded xorshift64* (`rng.rs`), held
   privately by `Run` (`run.rs:683`), for out-of-combat things: shop stock,
   the crucible's melt, the sealed bid's reserve, the dispenser's gamble, a
   Rogue wipe's next seed. `Run::seeded(seed)` / `Run::start(seed, mode,
   difficulty)` (`run.rs:705, :787`) are the only ways in. Share codes, the
   balance story, `acceptance::e6_1` and half the suite depend on this.
2. **Canonical names are string keys.** Piece and monster names key the theme
   layer (`theme.rs`), monster boards (`combat.rs`), quest `becomes` targets,
   event and rumour conditions, town and dungeon tables, and dozens of tests.
   Renaming without propagating is the classic repo mistake; `assembly` and
   `two_voices` exist to catch it. Grep before and after.
3. **Tests pin behavior on purpose.** Distribution tests pin the rarity curve;
   progression tests pin fight outcomes; `catalog_shape` is a ratchet whose
   budgets only go down. When your change moves a pinned number, re-pin it
   *with the reason in the assertion* - never loosen a test to make it pass.
4. **A theme cannot break the game.** `theme.rs` is display-only lookup;
   missing entries fall through to canonical names. Never route game logic
   through a themed string. `tests/two_voices.rs` is the ratchet (budget 5).

---

## 3. Engine map (`crates/engine/src/`, 37,892 lines)

| Module | Lines | Owns |
|---|---:|---|
| `piece.rs` | 10,986 | Everything a piece is: `SlotKind` (5), `PieceKind` (17, `Enchantment` and `Quest` among them), `EffectKind`, `Resource` (8: Mana, Rage, Faith, Nature, three fusions, Insight), `Action`, `Trigger` (`Watch` among them), `Quest`, recipes (`:1039`), per-slot default cooldowns (`:1089`: weapon 1500 ms, gloves 3000, greaves 3500, helmet 4000, chest 5000), `PieceRegistry` (instance = def index + rotation, `:1120`), and the **512**-entry `CATALOG` (`:1202`; helmet 99, chest 72, gloves 84, greaves 68, weapon 189). `BOSS_ONLY`, `EVENT_ONLY`, `VIP_ONLY`, `TOWN_ONLY` lists at the bottom |
| `combat.rs` | 7,174 | The fight: 50 ms ticks (`curse::TICK_MS`), `SUDDEN_DEATH_MS = 30_000` (`:40`), `MAX_DURATION_MS = 60_000`, `Difficulty {Easy, Medium, Hard, Insane}` (`:527`; Medium is gear-as-written, others step gear via `stepped_component`, `:292`), `MonsterSpec` (`:234`), **78 creatures**: `LADDER` 50 (`RUST_GOLEM` spliced in by name), `ALTERNATES` 28, `CREVICE` 0. `CombatLog { outcome, duration_ms, entries, .. }` (`:3513`); `tally_items` - what one item did in a fight, attributed by the last `Activate` |
| `run.rs` | 4,053 | A run: `Mode {Grinder, Rogue}`, `Phase`, gold, rung, lives, the shop, classes, every road flag and counter, `road_stack()` (derived, `:912`), `take_choice`/`apply_outcome`, `visit_town`, `enter_dungeon`, `melt`, `crush`, `fight_next` (`:3437`), `settle` (`:2043`), `apply_preset` (`:3224`), `skip_to`/`force_win` (`:2440`, `:2434` - test and picker helpers that win without fighting) |
| `event.rs` | 2,659 | `EVENTS` - **37**, a `pub const` (`:590`), each `LadderEvent { id, at (zero-based), trigger, choices }`; `Requirement` (12 variants), `Outcome` (35), `Trigger` (Rung, QuickKill, SlowKill, Whispered, WhenFlagged - the last three carry `from`, a window); six `Brawl`s; reverse indexes `set_by`, `every_outcome`, `opened_by_taking` |
| `theme.rs` | 2,189 | The turtle theme: names, story, `vocabulary`, `told: &[Retold]` keyed by road id - all display-only |
| `class.rs` | 1,370 | `CLASSES` - **31**; `ClassPower` (SlowTime, Overflowing, Leeching, WrongSense, FirstBlood, ...); fountains score axes; Piety and Unionized stack |
| `rating.rs` | 1,305 | Worth: `piece_rating`, `monster_value` (`:815`, the creature-side correction), `RARE_AT=90, EPIC_AT=130, LEGENDARY_AT=170` (`:230`), `ACTIVATIONS_PER_S = 5.0` (`:497`) |
| `loadout.rs` | 1,014 | Five `Slot`s, `ItemProfile`, assembly, `lock_assembled_in` (`:243`), `combat_items` (`:658`). **No player-build solver anywhere** - the nearest things are `Run::apply_preset` (twenty-two hard-coded placements) and the three share codes in `share.rs` |
| `naming.rs` | 746 | Generated item names; names grow with rarity (3/4/5/6 words) |
| `bestiary.rs` | 756 | `MonsterTheme` (**10**: Striker, Wall, Burner, Slower, Drainer, Caster, Hollow, Swarm, Beast, Warden; slots per theme at `:119-134`), `theme_for(rung)` (`:343`), `MonsterFrame` and `FRAMES` (**24**, all dressed, `:400`), the frame lint |
| `slot.rs` | 603 | One grid: `SLOT_W = 6`, `SLOT_H = 8` base (`:5, :8`), growable by rows; `can_place` (`:221`), `legal_anchors` (`:289`), items, neighbours, groups, `sets_touch_diagonally` |
| `town.rs` | 577 | `TOWNS` - **6** (3 pinned after rung indices 6, 17, 31; 3 hidden), `Action` (17 doors), one action a visit |
| `shop.rs` | 524 | `SHOP_SIZE = 6`, `STARTING_GOLD = 28`, `REROLL_COST = 1`, `SHELF_TILT`, `insight_open`, standing orders; town stock and enchantments never dealt on the road |
| `route.rs` | 583 | `route(run)` (`:124`) and `ascii(run)` (`:265`): the road drawn from the tables plus the run |
| `stats.rs` | 491 | `Stats`; `power` is a multiplier in hundredths |
| `rumour.rs` | 476 | **10** rumours; 1-cell conditions that sit in the tray and open doors |
| `curse.rs` | 396 | Searing, Frost, Stun, Misfire; `TICK_MS = 50` (`:12`) |
| `share.rs` | 356 | Share codes, version 3, base-32; a placement is `def<<12 \| slot<<9 \| x<<6 \| y<<2 \| rot` (`:218`); `A_FRIENDS_RUN`, `A_WINNING_RUN`, `A_PERFECT_RUN` (`:159-180`). **Index-keyed into `CATALOG`** |
| `dungeon.rs` | 993 | **7** dungeons. `Floor` and `Exit`: floors are a **graph**, so a floor with one exit is the next room, none is a buffer stop and two are a set of points. `fights_ahead` is what a banner counts; seven graph lints |
| `relic.rs` | 188 | **4** run-relics (pay from a board, off run counters), crushables |
| `pedestal.rs` | 223 | **6** destinations, once a run; `Where::Siding` puts you down *inside* a dungeon |
| `shape.rs`, `rng.rs`, `lib.rs` | 100, 94, 36 | Polyomino math; the PRNG; exports |

---

## 4. The game, mechanically

**The road.** Fifty creature rungs and a fifty-first (THE UNWOUND, opened by a
finished chain and a beaten Francis), three bosses, seven mini-bosses. Towns
stand *between* rungs (six, three hidden until revealed), events stand *in
front of* rungs (thirty-three), dungeons stand *beside* them (six), pedestals
send you to four destinations. Nothing on the road gets walked past
(`tests/the_road.rs`); the road stack pops gate, then fountain, then events
(`the-unwinding.md` #12). Fountains before each boss score the build on axes
and grant classes; the third can double one. Named creatures leave gear
behind. Enchantments are town stock only. A kill under 2 s in rungs 1-10 opens
the casino, once.

**A fight.** Both boards tick in 50 ms steps; items fire on cooldown. A hit is
`(flat + strength) × power`, typed **physical**, **magic** or **mind**; the
defender answers with `*_resist`, punched through by `*_pierce`, shored up by
`*_harden`. Three lanes, three answers: the mana shield blunts magic,
Deflection blunts physical, `mind_resist` alone answers mind. Empowerment
multiplies magic hits; Spellblade multiplies physical. Armour absorbs first
and resets to zero every fight. Mind damage lowers max health and never heals.
Curses stack by kind with caps. **Sudden death from 30 s**: both sides bleed a
growing share of max health each second; nothing runs past ~44 s. A fight past
30 s was decided by the clock, not the boards.

**Pools.** Mana is fuel. Rage → +1 physical damage a point, faith → +2 both
resists, nature → +1 regen. Insight is fuel for Dread (mind damage gains
`dread × insight / 2`), locked until THE THRESHOLD is cleared. `Drain` steals.

**A piece.** `PieceDef`: name, slot, kind, polyomino `cells`, base `Stats`,
optional `Adjacency`, optional positional `Effect`, triggers, cooldown, price,
`power_bonus`/`speed_bonus`, sometimes a `Quest`.

**Assembly.** Loose pieces contribute passive stats; pieces connected into a
recipe become an *item* that acts. Recipes (`piece.rs:1039`):

| Slot | Recipes |
|---|---|
| Weapon | Handle + 1-2 Damaging + 0-2 Accessory · Book + Ink + Spell + 0-1 Accessory · Orb + 2-3 Spell + 0-1 Alignment |
| Helmet | Frame + 1-2 Plating + 0-1 Crest |
| Chest | Base + 1-3 Layer |
| Gloves | Material + Mold + 0-2 Ring |
| Greaves | Material + Mold + 0-1 Plating |

Only weapons swing; every other slot acts through triggers
(`analysis/second-order.md` §10). A dense board comes back as the items its
owner built **only if each is locked as it assembles** - `common::board_from`
does this and hand-seated name lists do not (`design/HANDOFF.md` §5).

**Worth.** `rating.rs` prices a piece; price sets rarity; rarity sets name
length. Adjust by weights, never thresholds. `stepped_component` re-gears
every creature on Easy, Hard and Insane when a weight moves **or when
`CATALOG` grows a footprint sibling** (`the-unwinding.md` #19).

---

## 5. The test suite is the map of what matters

51 integration binaries in `crates/engine/tests/` plus the lib's 173.
**866 green, 44 ignored** in the engine, **65** in the GUI and **5** in the
CLI - 936 in the workspace - and it builds with **no warnings** under rustc
1.95.

| Group | Binaries (tests) |
|---|---|
| Catalogue and assembly | `assembly` (66), `catalog_shape` (3 + 2 ignored, the ratchet), `fixtures` (3), `prices` (1 ignored), `enchantment` (21), `primitives` (17) |
| Combat | `fight` (22), `effects` (26), `reactions` (12), `drains` (3), `curses_in_combat` (4), `sudden_death` (6), `brawl` (7), `typed_lanes` (8), `insight` (13), `slash_and_burn` (4), `class_reaches_combat` (2), `classes` (20) |
| Whole runs | `progression` (82, 12.8 s), `the_long_way` (9), `two_runs` (13 + 1 ignored), `taller_boards` (7), `decode_build` (6 + 4 ignored) |
| The road | `the_road` (6), `towns` (32), `casino` (15), `vip` (10), `earned_events` (6), `hidden_towns` (8), `road_stack` (11), `road_machinery` (23), `unconditional_events` (12), `structures` (24), `chain` (13), `dungeons` (14), `pedestal` (9), `relics` (17), `tooltips` (13), `phase_two` (7), **`completable` (4)** - can every key exist before its door shuts |
| Bosses and references | `francis` (6), `reference_builds` (4), `acceptance` (10; E6 criterion by criterion), `packing` (19 ignored generators), `pack_francis` (2 ignored: the board generator and `probe_the_curve`) |
| Analysis | `tallies` (7) - per-item attribution over a log |
| Words | `prose` (8 + 1 ignored printer), `two_voices` (6 + 1 ignored), `avail` (5, **43 s** - 400 seeded runs) |
| Measurement | `baseline` (4 + 6 ignored printers), `prices` (2 + 3 ignored) |
| The yard | **`switchyard` (30 + 1 ignored)** - the floor graph, the points, the chain and the balance |
| The driver | **`crates/cli/tests/replay.rs` (5)** - a scripted run piped in twice and byte-compared |

The printers, which write `analysis/`:

```
cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1
cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture
cargo test -p gearmaster-engine --test prose -- --ignored --nocapture read
PACK_MONSTER="Cog Priest" cargo test --release -p gearmaster-engine --test pack_francis pack -- --ignored --nocapture --exact
```

At the tip: owner 48/50, 75.5% weapon share, median 9.00 s; friend 48/50,
97.4%, 8.15 s; preset 9/50; starter 2/50. **Byte-identical through all eleven
Switchyard milestones**, which is what `analysis/switchyard.md` keeps proving.
Ratchet green, 0 away. Oracle cost in release: 0.03-1.4 ms a fight, whole
ladder 31 ms.

Two fixtures are diffed rather than asserted, and both name the command that
re-baselines them: `tests/fixtures/gear_at.txt` (every creature's gear at every
difficulty, 5,568 placements) and `analysis/replays/dungeons.txt` (all six
straight-line dungeons walked from the top). Neither may be re-baselined
without saying in the commit which creature or which dungeon started saying
something different.

The third one is not a measurement, it is the **road read aloud** - every
scene, town gate and dungeon landing in the order a player meets them, wrapped
the way the screen wraps them, choices underneath. It asserts nothing. It is
there because every lint in `prose.rs` is a cheap mechanical proxy and says so
at the top, and four fixes a batch came out of reading its output rather than
out of a failing test.

**Running the suite.** Every engine edit relinks all 50 binaries. Iterate with
`cargo test -p gearmaster-engine --lib` or one `--test <n>`; run the whole
thing once at the end. `[profile.test]` carries line tables only; for a full
backtrace on one run, `CARGO_PROFILE_TEST_DEBUG=2 cargo test ... --test <n>`.
Never start a second cargo while one is running.

---

## 6. WHERE THINGS STAND

**The Unwinding is merged and published.** `design/post-unwinding.md` is
the audit: what landed (nearly all of it), what changed shape (fifteen things,
listed), what slipped (Engraving, the Brain Farm), what is not met (a fourth
reference build that beats THE UNWOUND *because of* the mind lane), and the
eight post-merge commits no ledger records - two of which found doors whose
keys could not exist in time behind a green suite.

**The prose pass is merged and published.** The base game had been calling its
people by their jobs since M15 - "the crownwright", "the man who runs the
place", "a woman with a clipboard" - because that milestone correctly moved the
book's proper nouns into `theme.rs` and left the *roles* behind in the
canonical column. It has its own cast now, invented and plain-port, and the
scenes say what the buttons under them say.
`design/HANDOFF-prose-ledger.md` is the record; `HANDOFF-prose.md` is the brief
it was executed from. Rogue also gets a fourth life (`ROGUE_LIVES`, and five is
the eventual intent).

**The Switchyard is finished and NOT merged.** Eleven milestones on branch
`switchyard`, M0 to M10 plus this record. A four-door chain across rungs 21-34,
a nine-floor dungeon that is a **graph** rather than a list, four new combat
verbs, six components and two Orbs of Travel that are tickets back into a yard
you have already half-walked. `design/HANDOFF-switchyard.md` is the decisions
and what is not done; `analysis/switchyard.md` is every measurement, one block
a milestone, each headed by the commit it was read off. All twelve of the
spec's acceptance criteria are met. **Merging and publishing are the owner's
to do.**

**The mission is `design/rl-agent-plan.md`**: make the game playable by a
reinforcement-learning agent, with no generative AI anywhere in the loop, so
that a trained agent becomes a better validity solver than the repo has. Read
`design/rl-research.md` first for the stack recommendation. Its milestone 1 is
a non-learning search baseline, and every later milestone reports against it.

**Read `HANDOFF-solver.md` beside it.** The plan is a complete execution spec
and this file does not duplicate it; the handoff is the difference between what
the plan says and what the code says today. It was written against `18d1b85`
and the prose pass moved sixteen of its `run.rs` and `combat.rs` line
citations, changed one number it depends on (Rogue has **four** lives now, and
§5's M3 says three), and named a seed in §7 that does not exist. Every API name
in it is correct; the addresses are not.

**What the repo uses today to say a build is valid or a rung is clearable**,
which is what the mission has to beat:

1. `tests/pack_francis.rs::pack` - a seeded stochastic sampler over themed
   recipes, 300 trials, scored by the combat oracle against four reference
   boards at four settings and a TTK curve (`target_ms`, floor 2,000 ms, +490
   ms a rung past 10, ±30%). It authors **monster** boards. 39.5 s a creature
   in release here.
2. Three share codes (`share.rs:159-180`) and `apply_preset` - **player**
   boards somebody built by hand, replayed through the oracle by `francis`,
   `reference_builds`, `baseline`, `progression`.
3. `force_win` and `skip_to` - the road walked with fights won by fiat
   (`tests/chain.rs:275-308` proves the chain "completable" this way; 25
   `skip_to` call sites in `progression.rs`).

Nothing demonstrates that a build a *seed's own shop economy* can produce
fights its way to any given door. That is the gap.

**The traps, re-derived from this tip, in the order they will find you:**

1. **The MSRV is honest now.** 1.83 declared and 1.83 required, since the
   Switchyard's M0. It said 1.75 for three missions.
2. **`CATALOG` is index-keyed by `share.rs`. Append-only for ever.**
3. **`stepped_component` re-gears every creature on Easy, Hard and Insane
   whenever a `rating.rs` weight moves or a footprint sibling is appended.**
   Settle weights before authoring anything measured against them.
4. **The reconstruction fault.** Every rebuild goes through
   `common::board_from`; a name list is not a board. Learned four times, the
   last at M17 (`THE_FOURTH` came back as zero items).
5. **Sudden death owns everything past 30 s.** THE UNWOUND is authored to
   28.0 s at Medium. Both no-weapon "clears" of rung 15 are the clock's.
6. **"Completable" today means `force_win`.** See above.
7. **`EVENTS` is a `const`.** Every `&EVENTS[i].choices[j]` in another crate
   is a reference to a copy. Compare by value. `ptr::eq` on it refused every
   choice from a test binary, silently, for a milestone.
8. **A key that arrives after its door's window shuts survives the suite.**
   `completable.rs` is the audit; it knows four shapes. `Trigger::from`
   returns 0 for `Rung`, which is not the earliest a door can be met. Add a
   row when you add a requirement kind.
9. **`LadderEvent::at` and `Town::after` are zero-based; displayed rung is
   `at + 1`.** `LADDER` is fifty because `RUST_GOLEM` is spliced in by name;
   every grep of the table comes back one short.
10. **Names are string keys** across `theme.rs`, monster boards, quests,
    `event.rs`, `rumour.rs`, `town.rs`, `dungeon.rs`, `class.rs`, the tests.
11. **Enchantments are town stock.** Never on the road (`is_town_stock`).
12. **The base game does not speak turtle.** Proper nouns from the book go
    in `theme.rs`; `two_voices` is the ratchet at budget 5.
13. **`Run::begin_fight` fights Rust Golem** whatever rung you are on. The
    road's fight is `fight_next`; a brawl's is `fight_party`.
14. **`cargo build` does not compile the GUI's `cfg(test)` module.** It
    rotted for eight milestones. `cargo test -p gearmaster-gui`.
15. **`make pack`'s save rewrites `combat.rs` in place** and once rewrote a
    creature nobody was editing (The Iron Warden, M15). Read the diff.
16. **A `Watch` trigger whose payload can produce the event it counts
    recurses.** Guarded for curses (`analysis/second-order.md` §11); any new
    `Watched` variant needs the same question asked.
17. **A guard that refuses your change is usually right, and its refusal is
    a gradient.** "wanted 11.8s, best was 8.0s" is a ratio to scale by.
18. **The book-word ratchet was blind to capitals for its whole life.**
    `two_voices::leaks()` compared exact case, and this game puts its proper
    nouns on signs and brass plates - so EGGBERT on a gate post, BUNKO on a
    boat transom, HENPECK stamped on the Under-Mine's boards and THRUMBUS in a
    whole event's title all shipped in the canonical column behind a green
    budget of 5. It compares case-insensitively now, and `pedestal.rs` is
    walked too, which it never was. Add a `BOOK` word in the case the book uses
    and trust the lint.
19. **A silent counter with no door is dead content, and only half of that was
    linted.** `no_flag_is_waited_on_forever` catches a flag waited on and never
    set; nothing caught the mirror until `completable.rs` gained
    `COUNTERS_NOBODY_READS`, which is **3** - `shook-the-machine`, `moles-paid`
    and `crossed` are written by a choice and read by no door at all.
20. **`LadderEvent::at` is zero-based and prose is not.** THE CONTRACT promised
    "rung 28" for a payout standing on rung 29, and a player who signed and
    walked there would have found empty road. That is trap 9's fourth bug.
    `structures.rs::the_contract_names_the_rung_the_payout_actually_stands_on`
    pins that one; nothing pins the general case, because nothing can tell
    which figure in a scene is meant to be a rung.
21. **`Run::take_choice` returns the component handed over, not success.**
    Most choices hand over nothing and return `None` on the happy path, so
    `take_choice(c).is_some()` is not "the door was answered" - `run.answered`
    is. Two tests written that way passed for the wrong reason.
22. **A dungeon's floors are a graph, and half a suite assumed a list.** Five
    lints were right about a list and wrong about a graph, all found in one
    afternoon: bands had to rise along `floors` rather than along a *road out*;
    `floors.last()` was treated as "the ending" when a graph has one per buffer
    stop; `every_dungeon_pays_something` had no idea a floor could pay; a map
    fixture compared lengths; and a banner walk assumed room count equalled
    fight count. Ask of any `d.floors` code whether it means "the list" or "the
    walk".
23. **A road-walking helper has to know how to throw a lever.** `two_runs::play`
    answered every door on the road and then stood at a set of points for forty
    iterations, which reads as "the board stalled at rung 26" and is really
    "nobody could decide". Any walker that can reach a dungeon needs an
    `at_points` arm.
24. **A test that walks `DUNGEONS` until it runs out is a hang** the day a
    dungeon has points in it. Six minutes of suite before anybody noticed it
    was not merely slow. Bound every walk.
25. **The shop's shelf tilt counted the catalogue, not the pool.** A slot got
    tickets in proportion to how much of it *exists* rather than how much is
    *for sale*, so appending eight unsellable components moved every shelf in
    the game. It had been wrong since the Unwinding appended thirty-one
    event-only rewards. Fixed at M5; if you append content and a distribution
    test moves, this is the shape to look for.
26. **`PieceKind::Orb` is twenty-three pieces over eight footprints**, not the
    four Orbs of Travel. Any claim about "the orbs" wants checking against the
    kind.
27. **A town gate and an event may not share a rung**, and the free-rung lists
    in design documents count events only. Cross-check `TOWNS[i].after + 1`
    before placing a `Trigger::Rung` door.
28. **`Combatant::player` starts every pool and the wall at zero** whatever
    `Stats` says - and a player built from `Stats::ZERO` has no maximum health
    and is **dead on the first tick**. Every measurement off that fight reads
    as "the mechanic does nothing". `effects.rs` and `reactions.rs` carry an
    `ALIVE` constant with a comment saying so.
29. **A lint can be satisfied by the wrong thing.** `every_scene_names_something`
    passed on any digit, so the scenes M15 left anonymous grew numbers - "the 3
    chairs", "19 years", "40 years" - and stayed anonymous. Eighteen of them.
    The loophole is closed and the budget retired at zero; if you write a new
    lint, ask what the cheapest way to satisfy it is before you ship it.

One blind spot in `prose.rs` worth knowing before it finds you: a name that
only ever **opens** a sentence is invisible to `names_something`, because at a
sentence start it cannot tell "Vell" from "The". Four scenes named their people
and failed the lint anyway. Write the name into the middle of a sentence; do
not widen the proxy, which would mean keeping the cast list in a test file.

Retired since the last version of this file: "the MSRV is a lie" (fixed at
M0), "there is no milestone pricing"
(the mission that wanted it is over and priced in bounties instead), "the
frame lint is red" (it is green; `FRAMES` are all dressed), "`MonsterTheme`
does not exist in the engine" (`bestiary.rs`).

## 7. Etiquette

Match the module doc-comment voice (deadpan, first principles, one idea per
paragraph) - the codebase reads like it was written by one careful person and
should stay that way. Keep the engine free of every dependency, not only
graphics ones: `crates/engine/Cargo.toml` is empty on purpose and the RL work
lives in its own crate. Never let a themed string reach game logic. When a
design document and the code disagree, the document is right and the code has
a bug report - unless the document is a *record* of what shipped, in which
case the code is the news and the document does. Write down which commit a
number came from.
