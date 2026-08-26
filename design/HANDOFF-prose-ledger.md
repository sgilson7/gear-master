# Ledger - the prose pass

Milestone by milestone, against `628185a`. Every number here was measured, not
quoted. The mission is `HANDOFF-prose.md`; the plan it was executed from is in
the approval record.

**Ground at the start** (re-measured, because the handoff's figures had drifted):

| Claim | Measured at `628185a` |
|---|---|
| engine suite "774 green" | **776 green, 38 ignored, 51 binaries, 0 warnings** |
| gui suite "60 green" | **60 green** |
| toolchain | rustc **1.95.0**; `Cargo.toml` still declares `rust-version = "1.75"`. The two warnings CLAUDE.md §5 records are gone under 1.95 |
| "**7** `Requirement::Took` sites" (§3.1) | **4** in the table - `event.rs:693, 975, 1429, 1661`. The other three grep hits are the enum's own match arms |

---

## P1 - Rogue's fourth life

`ROGUE_LIVES: 3 -> 4`, and the four places that quoted the number.

**The shape of the fault.** Three sentences across two crates spelled the count
out as a word and not one of them was reading the constant, so raising it left
the game telling the player a number that was no longer true - on the mode
card, under the pips, and in the glossary. The handoff named two of them. There
was a third literal nobody had found:

- `crates/gui/src/main.rs:6286`, the ROGUE glossary entry - *"Losing costs one
  of three lives; the third ends the run"*.

**What landed:**

- `run.rs` - `ROGUE_LIVES = 4`, with the note that balancing around **five** is
  the eventual intent and this is the one number to raise when that happens.
- `run.rs` - `lives_in_words()`, so the three lines that say the count in words
  all say it from one place. `Mode::blurb` is a `String` now and builds the
  Rogue card from it; it had been a literal saying "Three losses".
- `main.rs` - the pip row is centred on the count
  (`w/2 - (n-1)*step/2`) rather than spaced with the arithmetic for exactly
  three (`w/2 - 60 + life*60`), which put the fourth pip off the middle of its
  own card. The caption is formatted from the constant.
- `main.rs` - the glossary entry, plus a guard:
  `the_rogue_entry_counts_the_lives_the_engine_grants`. It is a `const` table
  and cannot format itself, so the guard is the thing that keeps it honest, and
  it names `ROGUE_LIVES` in the failure so the message says what to edit.
- `progression.rs` - `a_rogue_run_dies_after_three_losses` renamed to
  `..._when_it_runs_out_of_lives`. The body always read the constant; the
  *name* and the closing assertion said three, which is the half a constant
  cannot keep honest. Re-pinned with the reason in the assertion, and the same
  test now checks the mode card agrees with the engine.

**Suites:** engine **776 green, 38 ignored, 0 warnings**; gui **61 green**
(60 + the new guard). `cargo build --workspace` clean.

---

## P2 - the two epigrams, and the lint that stops the register returning

Both lines under the setup screen's headings were knowing, balanced epigrams
restating the cards below them. Neither had ever been checked, because
`prose.rs` is an engine test binary and both strings lived in the GUI.

**What landed:**

- `Mode::WHAT_THE_CHOICE_IS` (`run.rs`) and `Difficulty::WHAT_THE_CHOICE_IS`
  (`combat.rs`). The engine already owns `Action::blurb`, `Outcome::describe`
  and `Requirement::describe`, so screen copy in the engine is established
  practice - and it is the only way the lint can reach it. The CLI picks a mode
  too, so this was never only the window's text.
  - was: *"Losing pays either way. It just does not get you past the thing that
    beat you."* -> **"The two differ in one thing: what a loss takes off you."**
  - was: *"Bigger numbers mean tougher, meaner monsters. Medium is the fight
    the game was built around."* -> **"Set once, for the whole run. It steps the
    gear the opposition wears before it touches any of its numbers."**
    The old line was also wrong about the mechanism: most of a setting is
    `gear_step`, and the numbers are what is left over.
- `prose.rs::scenes()` now reaches both.
- Two `HEDGES` entries - `"the game"`, `"it just does not"`. Probed against the
  whole corpus first: they fire on nothing else.
- A new test, `a_subtitle_does_not_name_the_options_under_it`. A subtitle may
  not name any of the cards it sits above; the difficulty one named MEDIUM, in
  a card that already says "the intended fight" on its own face.

**Proved, not assumed.** Both lints were run against the *old* strings and both
failed - `no_scene_withholds_the_noun` on "it just does not",
`a_subtitle_does_not_name_the_options_under_it` on MEDIUM - then passed on the
replacements. A lint that has never seen the fault it was written for is a lint
nobody has tested.

**The probe that shaped P4 and P5.** Twelve more register tells were tested
against the corpus and every one of them fires on shipped scene text, so they
are held back until the rewrite that removes them. This is the map of the worst
scenes, in the order the lint found them:

| Tell | Where it fires |
|---|---|
| `the worst of it` | `the-vip-area / Walk on`, `the-threshold` landing |
| `of some kind` | `the-shrine-fork` prose |
| `sitting wrong` | `the-bigger-sign` prose |
| `not from anywhere` | `the-inspection` prose |
| `does not say what` | `the-contract` prose |
| `the whole of his` | `the-buyer` prose |
| `stops being strange` | `the-manse` blurb, `the-threshold` blurb (**written twice, verbatim**) |
| `somebody who should not` | `the-crevice` blurb |
| `worth thinking about` | `the-under-mine` blurb (**twice in one sentence**) |

`either way` was tried and dropped: it fires only on
`the-sealed-bid / Name a figure` - *"They read the reserve out either way"* -
which is a statement of fact and exactly what the rest of the file is asking
for.

**Suites:** engine **777 green, 38 ignored, 0 warnings**; gui **61 green**.

---

## P3 - the cast, and the ratchet that says when it has landed

No prose was rewritten here. Two things landed instead: the measurement that
turns the mission into a countdown, and the cast verified against every
namespace it could collide with.

### The lint that let the fault through

`every_scene_names_something` passes on **any digit**. That is the hole M15 went
through: a scene with no name in it satisfied the lint as cheaply with a number
as with a person, so the scenes that lost their proper nouns had numbers bolted
onto them instead - "rice for the trade board for 19 years", "the 3 chairs",
"40 years", "6 demands". Green lint, anonymous scenes.

The same question without the loophole is now a budget, shipped the way
`catalog_shape` and `two_voices` are:

- `no_more_scenes_lean_on_a_number_than_already_did` - **<= 18**
- `the_digit_budget_is_not_slack` - **== 18**, so it cannot silently drift
- `every_scene_names_a_person_or_a_place` - `#[ignore]`d, asserts zero

**`DIGIT_PROPS` is 18 at `a46294a`**, and this is the worklist for P4 and P5,
in table order:

```
what-to-do-with-henpeck   what-the-table-said   the-thrumbus-race   mole-town
the-inspection            the-sealed-bid        the-contract        the-payout
the-buyer                 the-picket-line       the-exhibition      town extra-large
dungeon the-threshold landings                  dungeon the-under-mine landings
dungeon the-undertow landings                   dungeon den-rivals landings
dungeon wumpus-world blurb                      dungeon wumpus-world landings
```

Every batch that lands lowers the number, and the number reaching zero is the
mission being finished rather than the mission feeling finished.

One incidental tightening: `names_something` no longer counts a bare **"I"**.
It is a capital in the middle of a sentence and it is nobody. It changed nothing
today - THE CROWNWRIGHT slips through on a capital after a closing quote mark
instead - but it was a hole of the same kind.

### The cast

Five scenes needed no invention. The name was already in the game and M15
simply did not reach for it:

| Scene | Was | Is |
|---|---|---|
| `what-to-do-with-henpeck` | "The overseer" | **the Hollow King** - `LADDER[14]`, the rung you just cleared |
| `the-under-mine` | boards stamped `HENPECK` | stamped **HOLLOW KING** |
| `the-fork`, `the-slagworks`, the Foreman door | Ossery / "He has been down there" | Ossery, in all three |
| `the-astronomer`, `through-the-cracked-lens` | Halloway | Halloway, said consistently |
| `the-vip-area`, `the-wizards-thirst` | Merrik, Sam the Wise | left alone |

Twenty-three invented, plain-port, no book pastiche:

| Scene(s) | Was | Is |
|---|---|---|
| `back-in-a-minute` | "A man on the road" | Wint |
| `the-casino` | "A woman with a clipboard" | Marlow |
| `the-long-way`, `where-it-was-going` | "a man at the roadside" | Rowe (Gerald is the tortoise) |
| `the-shrine-fork`, `the-crevice` | "an old watchman" / "one old analyst" | Wenlock |
| `the-teller`, `the-bigger-sign` | "the man who runs the place" | Ollam |
| `the-crownwright` | "The crownwright" | Padgett |
| `the-inspection` | "A woman with a clipboard and a folding stool" | Nance Twiss |
| `the-green-ledger` | "The tally man" | Creel |
| `what-the-table-said` | "The landlord" | Salter |
| `the-contract`, `the-payout` | "A man from an underwriting house" | Braddock |
| `the-bird-problem` | "A courier" | Pether |
| `the-buyer` | "The buyer" | Vell |
| `the-exhibition` | "The two finest players" | Dorn and Ilder |
| `the-sealed-bid` | nobody in it at all | Sarn |
| `the-picket-line` | "somebody on this line" | Nettle |
| `the-passenger` | "A courier", the second one | Larkin |
| `the-glow-over-the-ridge` | "A carter coming the other way" | Gull |
| `the-foundry-remembers` | "a man in Slagworks overalls" | Rusk, sent by Ossery |
| `the-thrumbus-race` | "a steward" | Cobb |
| `mole-town` | "an older mole with a case of tools" | Tibb |
| `the-threshold` | "The man behind the cellar door" | Corvin |
| `the-undertow` | "The old man fished here for 60 years" | Fenn |
| `Action::Manager` | "He will confirm" | Mawes |

Plus the four replacements for the shouted book words: gate and Manse plate
**HOLLIS**, boat transom **PATIENCE**, boards **HOLLOW KING**, thrumbus ->
**bolter**.

**Verified, not assumed.** All twenty-nine were checked as whole words against
every double-quoted string literal in `piece.rs`, `combat.rs`, `class.rs`,
`town.rs`, `dungeon.rs`, `rumour.rs`, `theme.rs` and `naming.rs`, against the
`BOOK` list, and against the turtle `vocabulary` keys - the last of which
matters because a character called Frost would be retold as "nut-freeze". All
clear.

**Suites:** engine **779 green, 39 ignored, 0 warnings**.
