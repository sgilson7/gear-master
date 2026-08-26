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
