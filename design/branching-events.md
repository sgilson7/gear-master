# Branching events

The living design document for events that remember what you did. One event
per section; each names its trigger, its branches, and what each branch hands
you. Code follows this document, not the other way round — when they disagree,
this is the bug report.

Status legend: **spec** (written, not built) · **built** (in the game) ·
**partial** (some branches live).

---

## The shape of the thing

The existing framework (`crates/engine/src/event.rs`) already does most of
this. An event stands in front of a rung, asks a question, and never resolves
itself. What it does *not* yet do:

| Needed | Have | Gap |
|---|---|---|
| Fire on a condition, not a fixed rung | `at: usize` | No conditional trigger |
| Require an item you are carrying | `LooseItemOfSize { w, h }` | Not by name |
| Hand you a component | `Claim` gives a class | No `Give` for gear |
| Open a curated shop | — | New outcome + screen |
| Fight two creatures at once | 1v1 only | See *Multi-enemy* below |

Those gaps are the milestones, not the events. The events themselves are
content once the gaps are closed.

---

## THE CASINO — built

**Trigger.** The first time you kill something in **under 2 seconds**, while
you are on **rungs 1–10**. Once per run. If it never happens, the casino never
happens — this is a reward for building something sharp early, and it should
be possible to miss it entirely.

Recorded as `Run::best_fight_ms`. Only a real win counts: a stalemate lasts the
full clock by definition, and a half-second defeat is not a fight you won
quickly.

**The scene.** You go in to play. At one of the tables a fight is already
happening, and nobody is stopping it.

### Branch A — step in

You fight **both** creatures at once: **Bone Archer and Frost Wisp**.

Who is at the table is calibrated, not chosen. A complete auto-built board
beats this pair and loses to the next one up, and a test pins it — because the
chip is the key to the whole VIP event, so a pair nobody can beat would
quietly delete a later event rather than making an early one exciting.

The door can open as early as rung one, where a starter board loses badly, and
that is the tension worth having: step in early and you will probably lose,
wait and your build is better, but the door shuts at rung nine.

- **Win** → thrown out of the casino, and you keep the **Platinum Chip**.
  The chip is a component, not a token: it sits in your tray taking up space,
  and it is the key to the VIP event at rung ~30.
- **Lose** → thrown out, **no life lost**, back on the normal chain exactly
  where you were. A branch that punishes you for taking the interesting option
  is a branch nobody takes twice.

### Branch B — stay out of it

You get the **Gold Chip**: spends fnorp to deal escalating damage per trigger.

**Settled: it spends run gold, capped per fight.** Five fnorp a swing, hitting
four harder every time it pays, stopping at forty a fight. Both the budget and
the escalation reset when the next fight starts, so the worst it can do to your
shopping is known before you put it on.

Built as `Trigger::SpendGold { cost, budget, on_success }`, where the payout is
scaled by how many times it has paid — first at full, second at double, third
at triple. `Action::scaled` touches outcomes and never costs, so the price
stays flat while the payout climbs, which is the shape of the thing.

The simulation never touches `Run::gold`. It carries a `purse`, spends out of
it, and reports `CombatLog::gold_spent`; the run deducts that when the fight
settles. Looking at a log again must not charge you twice, and there is a test
that says so.

---

## THE VIP AREA — spec

**Trigger.** Rung ~30. **Always shown.** You can only go in holding the
Platinum Chip; without it the door is described and closed, so a player who
skipped the casino learns the casino exists.

**The scene.** Sprocketmen, being made to do heinous things.

### Branch A — keep your cover

You get a **shop of five extraordinary pieces**, and the class **Immense
Guilt**: you cannot regenerate health, for the rest of the run.

Two notes for whoever builds it:

- Those five pieces must be **exempt from the slot ceiling**, the way
  `BOSS_ONLY` is. `slot_ceiling` is the best possible item in a slot and every
  rating is a fraction of it — drop five outliers into the ordinary catalogue
  and every price in those slots deflates. This has bitten before.
- Immense Guilt is the first class that is purely a cost. The class system
  takes it fine (it is one more `ClassPower` among 43), but the fountain
  ranking must never *offer* it.

### Branch B — get them out

Two hard creatures are guarding them. Win and you get **Sprocketman's
Gratitude**: **one more row in every slot**.

That is a 6×8 grid becoming 6×9 — 30 more cells across the five boards, which
is the single largest power swing any item in the game has ever granted. It
also means the grid stops being a constant, which is its own milestone.

---

## Multi-enemy fights — built

The expensive one, and the reason it gets a milestone of its own rather than
being folded into the casino.

The rules barely care: `Side` is a two-variant enum with `other()`, one
accessor `pick(p, e, side)`, and one `for side in [Player, Enemy]` loop —
thirteen uses across the whole engine. The *presentation* is where the cost
is. `Playback` is built entirely out of paired fields — `player_hp`/`enemy_hp`,
`player_schedule`/`enemy_schedule`, `enemy_reg`, `enemy_loadout`,
`enemy_reports`, `enemy_profiles` — and the battle screen draws exactly two
boards, two health bars and two cooldown columns. Forty uses of `Side` live
there.

**Merging the two creatures into one is not an option.** Two monsters' gear
does not fit in five 6×8 grids, and pooled health would not read as two
opponents anyway.

**Proposed design.** Keep `Side` binary for the rules; make the enemy a party.

- `simulate` takes `&[MonsterSpec]` rather than `&MonsterSpec`.
- Internally `player: Combatant`, `foes: Vec<Combatant>`.
- **The aim rotates: every blow moves it along.** This replaced an earlier
  front-to-back rule, and the reason matters. Focusing the front one down
  makes a brawl a *queue* — kill the first thing and the incoming damage
  halves, so the back half of the fight is easier than the front. Spreading
  means both of them are hitting you until nearly the end, which is what makes
  two of something worse than one of something twice the size. There is a test
  that asserts exactly that.
- Every foe acts independently against the player.
- Victory when every foe is down.
- Events that name a combatant gain a `who: u8`, defaulting to 0, so every
  existing log reader keeps working for 1v1.
- The battle screen splits the enemy half into two narrower boards when
  `foes.len() > 1`.

The `who: u8` addition is mechanical but wide — it touches every event
construction site. Worth doing once, properly, because every future "fight two
things" event is free afterwards.

---

## Milestones

In dependency order. Each one ends with something playable.

| # | Milestone | Unblocks | Size |
|---|---|---|---|
| 1 | **This document** | everything | done |
| 2 | **Event triggers and rewards** — `Run::best_fight_ms`, conditional events, `Requirement::Holding`, `Outcome::Give` | 3, 7 | **built** |
| 3 | **The casino, walk-away branch** — event fires, scene, Gold Chip | — | **built** |
| 4 | **Multi-enemy fights** — engine party, then the battle screen | 5, 7 | **built** |
| 5 | **The casino, step-in branch** — 2-at-once, Platinum Chip, loss costs no life | 7 | **built** |
| 6 | **Variable slot height** — `SLOT_H` const becomes a per-run figure | 7 | medium |
| 7 | **The VIP area** — both branches, the five-piece shop, Immense Guilt, Sprocketman's Gratitude | — | medium |

Milestones 2 and 3 are in. The casino exists, opens on a sub-two-second kill
anywhere in rungs 1–9, and hands over the Gold Chip if you keep out of the
fight at the third table. Stepping in is written into the prose but is not yet
a choice you can take: it needs two creatures at once, which is milestone 4.

Both chips are `EVENT_ONLY` — a new exclusion beside `BOSS_ONLY` and the quest
rewards. A Platinum Chip bought off a shelf is a door key with no door behind
it.

One thing the build turned up: `at` has to stay unique even though an earned
event roams, because `event::at` returns the first match and a collision means
one of the two silently never fires. The casino's deadline moved to rung 9
(Whisperling) for that reason — rung 10 already has the shrine fork.

Milestone 6 is better than it sounds. `Slot::cells` is already a `Vec` rather
than a fixed array, so the height is a field waiting to happen — eight sites
in `slot.rs`, two in `run.rs`, the board layout in the interface, and the
packing tools. The share-code format stores `y` as a value and does not care.

---

## Content charter

Unchanged, and it constrains the VIP area specifically. Crude names are in
(Big Yomp, PoopFart). Still out: sexual and anatomical content, drugs, alcohol
and smoking, slur-adjacent coinages, and every real public figure. Violence
stays cartoon-grade.

"Sprocketmen being made to do heinous things" is written at the level the rest
of the game is written at: the Crimper crunches, and nobody is described being
crunched. The horror is that they are *made to work*, endlessly, for someone
else's amusement — which is the book's own joke about the Great Gear Cave, and
lands harder than anything explicit would.


---

## What the multi-enemy refactor actually cost

Recorded because the estimate was right about the shape and wrong about the
proportions.

`LogEntry` gained **one** `who` field rather than each of the twenty-odd
`Event` variants that name a side gaining one. That works because the player is
always singular: an entry is about a foe either way — the one acting, or the
one being acted upon — and there is never a third party. That single decision
is most of why the engine half came in small.

The trap was the targeting hook. The first attempt rotated the aim inside
`Action::Damage`, which looked complete and was not: a weapon's own swing never
goes through there, it lands `item.physical_damage` directly in `activate`. Two
identical toads came out of a "spread" fight on 240 and 97. Each repetition of
a swing now aims afresh and takes its own line in the log, which is also more
honest than folding an echo and its original into one number was.

The screen went the same way: `Playback`'s paired `enemy_*` fields became a
`Vec<FoeView>` indexed by `LogEntry::who`. That indexing is the whole reason
the log carries a foe index at all — without it both creatures log their
activations as `Side::Enemy` and nothing can tell their cooldown bars apart.

Layout, for whoever adds a three-creature fight later:

- Two board-sets only fit across the screen at a **15px cell** against the
  duel's 32, so `render_mini_board_at` takes the cell size and the gap.
- Each foe's health bar hangs off **its own** board, not off a fixed
  `enemy_bar_y` — a brawl's boards are 120px tall rather than 256, and the
  captions the board prints under itself need more clearance at a smaller
  cell, not less.
- The cooldown column becomes one section per creature, headed by its name:
  "THEIR COOLDOWNS" is no answer when there are two of them.
- The portrait is a duel thing. In a brawl the space under the cooldowns is
  the second creature's column.

`GEARMASTER_BRAWL=<n>` starts a fight against n creatures, which is the only
way to see one until milestone 5.


---

## A note on measuring difficulty

Recorded because it has now cost two milestones' worth of wrong answers.

Hand-seating a list of piece names to make a test build **does not produce the
build you meant**. Pieces land at the first free cell scanning row-major, join
their nearest core, and come out as items that assemble into something else
entirely. Calibrating the casino table against a hand-picked "sharp" list said
nothing on the ladder could be beaten — the list had assembled into a six-dps
weapon, while a two-piece "starter" list managed thirty.

`Run::apply_preset` is the auto-builder the game itself uses and produces a
board that actually assembles. Anything measuring difficulty should start
there, or from `pack_dense` in the packing tests, and never from a hand-written
list of names.
