# Towns

A town is an **extra rung**, inserted after certain ladder rungs. It is the
first thing in the game that lengthens the road: an event stands *in front of*
a rung and hands it back to you afterwards, and a dungeon stands *beside* one.
A town sits *between* two rungs and is a rung of its own, with no creature on
it.

You arrive at the gate and answer one question: **go in, or walk on**. Walking
on pays a purse. Going in buys you exactly one of four actions, and then you
are back on the road.

The one-action rule is the whole design. Four doors and one key means a town is
a decision rather than a shopping trip, and it means the four can be tuned
against each other instead of against nothing.

---

## Where they stand

Towns are pinned, not earned. Three of them:

| After rung | Town | Why there |
|---|---|---|
| 6 | **Sump Bottom** | Early enough that a Piety stack has time to reach five. |
| 17 | **Kettleworks** | The factory is worth most when the bounty is worth something. |
| 31 | **High Wick** | Past the VIP area, so the two do not compete for the same run. |

The ladder is still fifty creatures long. A run that enters every town fights
fifty times and stands on fifty-three rungs.

### Walking on

Skipping pays **the rung's bounty again**, straight into your purse, and costs
nothing. That is deliberately a real offer: a build that is one component short
of an item wants gold more than it wants a class, and the town should lose that
argument sometimes.

---

## The four actions

### 1. The chapel — *Piety*, then *Ticket to Ride*

Praying grants a stack of **Piety**. Piety is the game's first stacking class:
holding three of them is holding the class three times.

- **Piety** — start every fight with **1 devotion per stack**.
- At **5 stacks**, every Piety is taken away and replaced with **Ticket to
  Ride**: *half of everything they swing at you misses entirely.*

Five stacks needs five prayers and there are three towns, so Ticket to Ride is
not reachable by praying alone — see *Rumours*, below, and the shrine at rung 9,
which counts.

#### "50% chance to miss" in a game with no dice

Combat is deterministic end to end. Nothing in `combat.rs` consults an RNG, and
that is load-bearing: a share code reproduces a fight exactly, the balance
solver can measure a build, and two identical boards always come out the same.

So Ticket to Ride counts rather than rolls: **every second enemy attack misses**,
tallied per attacker across the whole fight. That is exactly one half, it cannot
streak, and it reads the same way from the other side of the screen. The same
trick the misfire curse already uses.

### 2. The pub — rumours

A shop that does not take money. The shelves hold **rumours**, and a rumour is
paid for by **bartering**: hand over a loose component of the kind it asks for,
or another rumour.

A rumour is a component like any other — it takes up room in the tray, it can be
sold, it can be handed over — but it has no cells and never goes on a board. What
it does is stand as the condition on an event that would otherwise never fire.

Hovering one gives a **vague** description of how to set it off. Vague on
purpose: the fun of a rumour is working out what it means, and a rumour that
reads "trigger: helmet_empty_cells < 10" is a quest marker.

Two to start with, both firing around rung 20:

| Rumour | Bartered for | Hover says | Actually needs |
|---|---|---|---|
| **A Word About the Crownwright** | any loose *Frame* | "They only see people whose heads are already full." | Fewer than 10 empty cells in the helmet slot |
| **A Word About the Green Ledger** | any loose *Material* | "It is a long tally, and it wants to be finished." | 100 nature banked across the whole run |

Both are checked when you arrive at their rung. Fail the condition and the
event does not stand there — the rumour stays in the tray and does nothing,
which is the risk you took.

The running nature total is a new field on `Run`: nothing counted it before,
because nothing had ever asked a question about a whole playthrough.

### 3. The factory — money now, *Tired* later

Work a shift. You are paid **double the bounty of the fight you just won**, and
you pick up a stack of **Tired**.

- **Tired** — start every fight **3 mana in debt per stack**.

Mana debt is mana below zero. Nothing that spends mana can pay until income has
carried the pool back above the cost, so three stacks is nine mana of dead air
at the start of every fight for the rest of the run. A mana engine feels it
immediately; a board that never spends mana does not feel it at all, which is
the trade the class is for.

Tired stacks, and unlike Piety it never converts into anything.

### 4. The shop — town gear

A five-shelf shop of components that appear nowhere else. Priced normally and
bought with gold, so this is the door for a run that is simply short of a piece.

Town gear is **not** off-the-scale the way the VIP shelves are. The VIP shop is
a reward for a locked branch and its five pieces sit outside the rating ceiling;
a town is on the way to everywhere, and five outliers three times a run would
flatten the whole curve. Town pieces are ordinary-to-strong, and what makes them
worth the trip is that they are *shapes and effects the normal shop does not
stock*.

---

## Milestones

All eight built.

1. ~~**The rung that is not a fight.**~~ `town.rs`, three towns, `Run::town` /
   `towns_seen` / `last_bounty`, `skip_town`, `visit_town`.
2. ~~**Stacking classes.**~~ `ClassPower::{Piety, Tired, Ticket}`, the two
   accumulating arms in `simulate_party`, `class::stacks`, `Run::stacks_of`.
3. ~~**Chapel and factory.**~~ Both actions, and the five-stack conversion.
4. ~~**Ticket to Ride.**~~ `Combatant::warded_every` / `warded_count`, eaten at
   the activation beside the misfire check, `Event::Warded`.
5. ~~**Rumours.**~~ `rumour.rs`, `Trigger::Whispered`, `Run::banked_all_run`,
   `Run::barter` / `payment_for`, the two rung-19 and rung-22 doors.
6. ~~**The town shop.**~~ `piece::TOWN_ONLY`, five themed names.
7. ~~**The screen.**~~ `render_town`, the pub's two-click barter, the rumour
   hover.
8. ~~**Glossary sweep.**~~ Eleven terms, and the tests that found them.

### What the numbers came out at

Measured rather than guessed, on the two boards the project has:

| | auto-built | the winning board |
|---|---|---|
| empty helmet cells | 32 | 2 |
| nature banked by rung 22 | 0 | 453 |
| starting mana | 6 | 46 |

So "fewer than 10 empty helmet cells" and "100 nature all run" are both
conditions the endgame board meets and the auto-builder does not, which is
what a rumour should be: a bet on the board you will have.

The same table is why Tired is measured on the auto-built board and Ticket to
Ride on the winning one. Three stacks of Tired against 46 opening mana is a
discount, not a debt.

---

## Lessons this document already knows

Recorded here rather than learned again:

- **Do not hand-seat a test build.** Three separate times a "sharp" list of
  component names assembled a fraction of the damage the auto-builder manages.
  Use `apply_preset()` or `share::A_WINNING_RUN`.
- **`log.player` and `log.enemy` are the starting snapshots.** Anything
  measured from them is a constant, not an outcome. Read the events.
- **Do not measure anything that depends on how long a fight ran.** A class
  that slows the enemy makes the fight longer, so totals come out equal and the
  class looks like it does nothing. Use fixed windows.
- **Filter log events by side.** Counting `Cast` events to measure Tired
  counted the *enemy's* casts and reported that ninety-six mana of debt changed
  nothing. Neither board in the project casts spells at all.
- **A resource may have more than one event.** Mana arrives through `GainMana`,
  not `GainResource`, so the run-total mana counter read zero for ever while
  the other three worked.
- **`force_win` writes no log.** A loop built on it banks nothing, and a test
  measuring what a run accumulated will say the condition is unreachable when
  it is only unfought.
