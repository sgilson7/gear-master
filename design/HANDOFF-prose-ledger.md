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
