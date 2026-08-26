# Handoff — The Solver

Written for an agent picking this up cold, at commit `118fbef` (2026-08-26).
Read `CLAUDE.md` first, then this, then `design/rl-agent-plan.md`, which is the
mission and is a full execution spec: eleven sections, seven milestones, a test
inventory, an evaluation protocol, a compute budget, and ten decisions with
defaults. **This document does not restate it.** It is the difference between
what that plan says and what the code says today, established by reading the
code rather than by guessing.

The prose pass shipped between the plan being written and you arriving. It
changed no rule and one number, and it moved about sixty lines of `run.rs`.

---

## 1. Read the plan's line numbers as names, not addresses

`design/rl-agent-plan.md` was written against `18d1b85` and cites `run.rs` and
`combat.rs` by line throughout §0, §3, §4 and §7. **Every `run.rs` citation in
it is now short by 46 to 57 lines**, because `ROGUE_LIVES` gained a doc block,
`lives_in_words()` and `capitalised()` were added above it, and `Mode::blurb`
became a `String`.

| The plan says | It is at |
|---|---|
| `run.rs:77` `BoardSnapshot` derive | **`:134`** |
| `run.rs:408` `pub struct Run` | **`:465`** |
| `run.rs:524, :631` the two `HashMap`s | **`:581, :688`** |
| `run.rs:626` the private `rng` | **`:683`** |
| `run.rs:644` `Run::new`'s seed | **`:700`** |
| `run.rs:730` `Run::start` | **`:787`** |
| `run.rs:791` `reroll_cost` | **`:848`** |
| `run.rs:1040` `choice_open` | **`:1097`** |
| `run.rs:1788` `holds` | **`:1845`** |
| `run.rs:1802, :1833` `price` / `payment_for` | **`:1859, :1890`** |
| `run.rs:2377, :2383` `force_win` / `skip_to` | **`:2434, :2440`** |
| `run.rs:2409` the Rogue wipe's next seed | **`:2466`** (`wipe` at `:2462`) |
| `run.rs:3070` `can_equip` | **`:3127`** |
| `run.rs:3167` `apply_preset` | **`:3224`** |
| `run.rs:3360` `Run::fight` | **`:3417`** |
| `combat.rs:3850` `simulate_party` | **`:3862`** |

**Correct as written**, checked one by one: `combat.rs:40` (`SUDDEN_DEATH_MS`),
`slot.rs:289` (`legal_anchors`), `loadout.rs:466` (the `HashSet<StatKind>`),
`piece.rs:1039` (`recipes`), `share.rs:159-180` (the three codes),
`pack_francis.rs:54, :197, :662`, `chain.rs:275-308`.

**Wrong before the prose pass, and now fixed in `CLAUDE.md`:** the placement
packing is `share.rs:218`, not `:182`. Both documents carried `:182`; `:182` is
`pub fn export`. Worth knowing because it is the one pin that proves the plan's
line numbers were not all read off its own tip.

**Also moved:** `post-unwinding.md` is at `design/`, not `analysis/`. The plan
cites `analysis/post-unwinding.md` in M0 and §8; that path resolves to nothing.

Every API name the plan uses exists and is spelled right. That is the part
worth trusting: `can_equip`, `legal_anchors`, `choice_open`, `take_choice_with`,
`toggle_lock_item`, `drink_choosing`, `pending_town`, `pending_event`,
`pending_brawl`, `at_fountain`, `fountain_offer`, `holds`, `price`,
`payment_for`, `reroll_cost`, `skip_town`, `visit_town`, `enter_dungeon`,
`fight_party`, `with_all_pieces`, `force_win`, `skip_to` - all present, all
`pub`.

---

## 2. Four corrections of substance

### 2.1 Rogue has **four** lives, not three

`ROGUE_LIVES` is 4 (`run.rs:88`). §5's M3 says *"Rogue has three lives; the beam
carries `lives_left`"*. The beam is still right; the number is not. Balancing
around **five** is the eventual intent and the constant is the only thing that
has to move for it - everything that quotes the count reads it now, and
`progression.rs::a_rogue_run_dies_when_it_runs_out_of_lives` plus a GUI guard
hold that.

This matters to §7 more than it looks: SCR in Rogue is measured against a
budget of losses, and that budget just went up by a third.

### 2.2 §7's seed set names a seed that does not exist

§7 says to build S from *"the three the repo already uses (`0x5EED_1234_ABCD_0001`
from `Run::new`, plus the two that `acceptance.rs::a_run` and
`two_runs.rs::a_run` are called with - read them off at M0)"*.

Read off:

- `Run::new` → `0x5EED_1234_ABCD_0001` (`run.rs:701`). Correct.
- `acceptance.rs::a_run(seed: u64)` is called with **three**: `0x60_60`,
  `0x11_11`, `0x12_12`, plus a closure over `seed` inside
  `e6_1_two_replays_of_a_seed_agree_about_everything_that_rolls`.
- **`two_runs.rs::a_run` does not take a seed at all.** Its parameter is a
  `Difficulty`; it calls `Run::new()`, so its seed is the default one already
  counted. There is no second seed to read off.

So the repo uses **four** distinct seeds, not three, and one of the two the plan
points at is the one it already had. Write the real four into
`analysis/rl-agent.md` at M0 and draw 60 rather than 61.

### 2.3 D1 (the toolchain) has evidence now

`Cargo.toml` still declares `rust-version = "1.75"` and the code needs 1.83.
What is new: **this repo builds and tests warning-free under rustc 1.95**, whole
workspace, `cargo build --workspace` and both suites. The two warnings
`CLAUDE.md` §5 used to record (`packing.rs:1141`, `primitives.rs:440`) are a
1.75 artefact and are not there.

That makes D1's default - raise the declaration to `"1.83"` at M0 - a one-line
change with nothing behind it. The alternative (rewriting seven `is_none_or`
sites and six statics) is work nobody needs.

### 2.4 Three doors the road does not have

`completable.rs` gained `COUNTERS_NOBODY_READS = 3`. `Outcome::Count` is the
watcher pattern - a choice arms a silent counter and a door forty rungs later
reads the tally - and **three of the game's four counters are written and read
by nothing**: `shook-the-machine` (THE DISPENSER), `moles-paid` (MOLE TOWN) and
`crossed` (THE PICKET LINE). Only `crucible-melts` has a door.

For M3 and M4 this is a small, precise thing: the agent enumerates doors from
`choice_open`, so it will never see a payoff for those three, and a beam that
scores a node by "what this door leads to" should not be taught to value them.
It is recorded as a budget rather than fixed because closing it means authoring
three doors, which is content and not this mission.

---

## 3. What the ground actually is

| | The plan's §1 table (`18d1b85`) | At `118fbef` |
|---|---|---|
| engine suite | 776 green, 38 ignored | **781 green, 40 ignored**, 49 binaries |
| gui suite | 60 | **61** |
| warnings, workspace | two, under 1.75 | **none**, under 1.95 |
| `crates/agent` | does not exist | still does not exist |
| `analysis/rl-agent.md` | does not exist | still does not exist |
| workspace members | engine, gui, cli | unchanged - §2 wants a fourth, not in `default-members` |
| `Run: Clone` | no | **still no**; D5 is still open |

Everything in the plan's §1 SCR table is still **0 by construction**. Nothing in
this repo plays a run. That has not changed and is the whole point.

One stale comment you will meet on the way in: `Cargo.toml`'s `[profile.test]`
block says *"There are forty-six of them and every engine edit relinks all
forty-six."* There are **49**. The reasoning in that comment is still exactly
right and is worth reading before you add a test binary.

---

## 4. Four traps added since the plan was written

`CLAUDE.md` §6 now runs to 21. The four new ones came out of the prose pass and
three of them bear directly on how you will write tests here.

- **18. A ratchet can be blind to the shape of its own data.**
  `two_voices::leaks()` compared book words case-sensitively for its whole life,
  and this game puts proper nouns on signs in capitals - so four leaks shipped
  behind a green budget. It compares case-insensitively now.
- **19. Half a lint is not a lint.** `no_flag_is_waited_on_forever` caught a
  flag waited on and never set. Nothing caught the mirror until `completable.rs`
  gained the counter budget. **If you write a reachability check for the action
  space, write both directions.**
- **20. `LadderEvent::at` is zero-based and prose is not.** THE CONTRACT
  promised "rung 28" for a payout standing on rung 29. Trap 9's fourth bug. Your
  proofs will print rungs; print `at + 1` and say which you mean.
- **21. A lint can be satisfied by the wrong thing.**
  `every_scene_names_something` accepted any digit as evidence a scene was about
  something, so eighteen anonymous scenes grew numbers instead of names and the
  lint stayed green for two milestones. **This is the trap most likely to bite
  this mission**, because SCR is a metric an agent optimises: ask what the
  cheapest way to satisfy each gate is before you ship it. §7's "board clear vs
  game clear" split is the plan already doing exactly that, and it is the model
  to copy.

---

## 5. Two tools that did not exist when the plan was written

- **`cargo test -p gearmaster-engine --test prose -- --ignored --nocapture read`**
  prints the whole road in the order it is walked - every event, town gate,
  door and dungeon landing, with the choices under each. For M3's door
  enumeration and M4's four paths this is faster than reading `event.rs`, and it
  is what a player actually sees.
- **`design/HANDOFF-prose-ledger.md`** records what the prose pass changed and,
  more usefully here, what it *found* and did not fix: the three counters, the
  `pedestal.rs` titles duplicated as second literals, two event ids that still
  name book characters, and the MSRV.

---

## 6. Ten decisions still want a yes or no

§11 lists D1 to D10, each with a default so M0 can start without an answer.
Two of them have moved:

- **D1 (toolchain)** - the default is now clearly right. See §2.3 above.
- **D5 (`#[derive(Clone)]` on `Run`)** - still open, still one line, still every
  field already `Clone`. `Run` is at `run.rs:465`. M1 is written for either
  answer and the plan tells you to measure both if in doubt.

The other eight are genuinely the owner's: what machine and how many overnights
(D2), what "solved" means (D3), how far determinism may be relaxed in search
(D4), whether proofs are replayed by a normal test (D6), whether LibTorch is
acceptable if a measurement justifies it (D7), whether Grinder farming is capped
(D8), which boards the agent may replace (D9), and whether 64 seeds is right
(D10). **Ask before M0 rather than after M3.**

---

## 7. The habit that matters here

The prose pass fixed four bugs that had all survived a fully green suite, and
the previous mission fixed five, and every one of them survived for the same
reason: **a test asked whether a thing existed, and none asked whether it worked
in the order a player meets it.**

This mission is the repo's first attempt to ask that question directly - a run
played forward, from a seed, with only the actions a player has, and a proof at
the end that replays. That is exactly the missing test, generalised.

So the failure mode to watch for is the one it inherits: an agent that reaches a
target by a route no player could take, behind a green suite. §7's forbidden
list (`skip_to`, `force_win`, `with_all_pieces`, `apply_preset`, `Undo` in a
proof, fight-and-rewind) is the guard, and it is only as good as the test that
enforces it. **Write that test at M1, before the first strategy exists**, so
there is never a proof nobody checked.

And when a milestone's number comes out zero: the plan's §10 says which zeros
are findings and which are bugs. Read it before you conclude anything about the
economy.
