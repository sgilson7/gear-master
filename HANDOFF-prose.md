# Handoff — the prose pass, and three small fixes

Written for an agent picking this up cold. Read `CLAUDE.md` first, then this.
The Unwinding is finished, merged and live; this is what playing it turned up.

Everything below was established by reading the code, not by guessing. Line
numbers are from `18d1b85`/`8b7bed1` and will drift — the names will not.

---

## 1. What is wrong, in one paragraph

The base game's event prose is vague, and **it is not an accident of authoring —
it is a scar from a specific migration**. M15 found fourteen canonical scenes
written in the book's voice and moved the book's proper nouns into `theme.rs`
where they belong. What it left behind in the canonical column was **job
titles**. The turtle telling reads well because it names people; the canonical
telling names *roles*, and a role is not a person. Gear Master mode shows
canonical prose for every single event — `PLAIN.told` is empty, so every scene
falls through to it — which is exactly where the vagueness is felt.

There are three more items, all small, listed in §6.

---

## 2. The diagnosis, with evidence

### Fault one: a role standing where a person should be

| Canonical (what ships in Gear Master mode) | Turtle (what reads well) |
|---|---|
| "The crownwright works out of one room over a fish shop" | "The Kolok Hatter works out of one room over a fish shop" |
| "the man who runs the place has been carrying a story" | "a man called Songil has been carrying a story" |
| "A man from an underwriting house sets a document on a milestone" | (Treyway Underwriting) |
| "an old watchman prays on stone that cuts his knees" | "an old analyst named Boyetano who prays on a floor that cuts his knees" |
| "The buyer works out of a hired room" | (the Multicity buyer) |
| "A woman with a clipboard and a folding stool" | — |

The turtle column is **not** the fix — it is the thing that must not be
borrowed from. The base game needs **its own cast**, invented, plain-port, and
not book pastiche.

### Fault two: abstraction where the buttons are already concrete

THE BUYER's prose: *"he buys the things a run has that a run cannot put a price
on"*. His three choices, immediately below it: **a word**, **a title**, **a
hundred of your maximum health**. The prose gestures at what the interface
states plainly. This pattern repeats.

### Fault three: withholding as a tic

"He does not say what their side is." "She is not from anywhere." "which is the
whole of his trade." A little is atmosphere. Everywhere, it is fog.

### A tell worth knowing

Several scenes satisfy `prose.rs::every_scene_names_something` with **digits
bolted on for that purpose** — "rice for the trade board for 19 years", "the 3
chairs", "40 years", "6 demands". They were added in M14/M15 *because* the
scenes had no names in them. Real names satisfy that lint honestly and those
props should come out as you go. Their presence is a map of which scenes are
worst.

---

## 3. Constraints. Violating any of these is a silent break.

1. **Choice labels are keys.** `Requirement::Took(label)` matches on the label
   string; there are **7** such sites in `event.rs`. THE BIGGER SIGN waits on
   `"Plug your ears"`, and renaming that label unhooks the only route into
   EXTRA LARGE. Reword a label only by changing both ends in the same edit.
   `tests/completable.rs` catches an orphaned `Took`.
2. **`tests/two_voices.rs` is the wall.** It walks every canonical string
   against the book's proper nouns. Its budget is **5** and every one of them
   is a `CATALOG` piece name that cannot be renamed (append-only). **The budget
   must not rise.** New canonical names must not appear in its `BOOK` list.
3. **`tests/prose.rs` already has the lint you want.**
   `no_scene_withholds_the_noun` holds a `HEDGES` list of register tells and
   fails on any of them. Add the new tells there rather than writing a new
   file. `scenes()` at the top of that file decides what the lint can see.
4. **Do not touch the turtle column.** `theme.rs::told` holds 36 retellings and
   they are the good ones. All work is canonical-side.
5. **Names quoted in prose must stay real** — pieces, creatures, towns and
   classes are string keys elsewhere, and `assembly.rs` and `two_voices.rs`
   both walk them.
6. **`Trigger::Rung` events stand on exactly one rung**; `Trigger::from()`
   returns 0 for them. If you touch a window, read `tests/completable.rs`'s
   header first — that footgun cost three bugs.

---

## 4. Where things live

| What | Where |
|---|---|
| The 33 events: prose, labels, blurbs, `unmet` | `crates/engine/src/event.rs`, `EVENTS` |
| The turtle retellings (leave alone) | `crates/engine/src/theme.rs`, `TURTLE_DICK.told` |
| Dungeon blurb / entry / landings | `crates/engine/src/dungeon.rs` |
| Town blurbs, door blurbs | `crates/engine/src/town.rs` |
| Class blurbs and power text | `crates/engine/src/class.rs` |
| Rumour hints | `crates/engine/src/rumour.rs` |
| Mode + difficulty select screen | `crates/gui/src/main.rs`, ~7870–8030 |
| Rogue's life count | `crates/engine/src/run.rs`, `ROGUE_LIVES` |

**Volume:** 33 events, **91 prose paragraphs**, **148** choice `blurb`/`unmet`
lines, 40 dungeon lines, 6 town blurbs, 31 class blurbs, 8 rumour hints.

---

## 5. The approach the owner chose

Asked directly, the owner picked:

- **Give the base game plain-port names.** Not "keep roles and add detail" —
  invent a cast. The crownwright becomes a named person with a named street.
- **Full scope**: events *and* the things event text points at — dungeons,
  towns, classes, rumour hints.

Rumour hints are the one exception to "be concrete": they are **vague on
purpose**, and the module doc says why. Vague about the *condition* is the
design; vague about *who is talking* is the bug.

---

## 6. The three small fixes that ride along

### Rogue gets a fourth life

`ROGUE_LIVES: 3 -> 4` in `run.rs:61`. Balancing around **five** is the eventual
intent; four is what the owner asked for now, so leave a note saying so.

Two things break quietly when it changes:

- `Mode::blurb` (`run.rs:52`) says **"Three losses and it is over"**.
- The mode card's life pips (`main.rs`, in the mode-select renderer) loop over
  `ROGUE_LIVES` correctly **but space themselves with hardcoded arithmetic for
  three** — `rect.x + rect.w/2.0 - 60.0 + life * 60.0` — so a fourth pip lands
  off-centre. The caption `"three lives, then you start over"` is a literal.
  Centre the row on the count and format the caption. This is what the owner
  meant by "regenerate the SVGs" — they are macroquad primitives, not files;
  there are **no `.svg` assets in this repo**.

### Two lines of interface text in the wrong register

Both are knowing, balanced epigrams that restate the options below them:

- `main.rs:7877` — *"Losing pays either way. It just does not get you past the thing that beat you."*
- `main.rs:8020` — *"Bigger numbers mean tougher, meaner monsters. Medium is the fight the game was built around."*

Replace with text that says what the screen is **for**. Then widen
`prose.rs::scenes()` so the lint reaches interface subtitles and not only event
prose, and add this register's tells to `HEDGES`.

### Re-pins

Life-count changes may move pins in `progression.rs` and `two_runs.rs`. Re-pin
with a one-line reason **in the assertion**, which is house style — a reason in
a commit message is a reason nobody reads twice.

---

## 7. Suggested milestones, and where to deploy

Each ends green on `cargo test -p gearmaster-engine` **and**
`cargo test -p gearmaster-gui`, with no warnings. ▲ marks a sensible deploy.

1. **P1 ▲** Rogue's fourth life, the pips, the caption, the re-pins.
2. **P2 ▲** The two epigrams, plus the lint that stops the register returning.
3. **P3** The cast: name every canonical character a scene calls by role
   (~14 scenes). Structural, and it makes P4 possible.
4. **P4 ▲ per batch** The event pass, four batches by rung — 1–13, 14–24,
   25–36, 37–51 — so a playthrough sees it improve front to back. Include
   `blurb` and `unmet`; `unmet` is read at the moment of refusal and is some of
   the most-read text in the game.
5. **P5 ▲** Dungeons, towns, classes, rumour hints.
6. **P6 ▲** Full verification, the two printers, two CLI replays diffed, and
   the record written into `HANDOFF.md` and `analysis/baseline.md`.

---

## 8. How to run things here

- **Iterate with `cargo test -p gearmaster-engine --lib`** (0.13s) or one
  `--test <name>`. The full suite is ~30s but **46 test binaries relink on
  every engine edit**, so run it once per milestone, not per change.
- `[profile.test] debug = "line-tables-only"` is deliberate — see the comment
  in `Cargo.toml`. For a full backtrace on one run:
  `CARGO_PROFILE_TEST_DEBUG=2 cargo test ... --test <name>`.
- **Never start a second cargo while one is running.** They share an exclusive
  lock on `target/` and the second simply waits.
- **Deploy:** `make web && cp dist/web/* docs/ && touch docs/.nojekyll` then
  commit and push. `index.html` now carries the wasm's own hash as a query
  string, so a deploy busts its own cache — but a browser already holding the
  old page needs one hard reload to learn the new URL.
- **Nobody has played most of this.** Every claim in the mission handoffs comes
  from the suite. If you can look at a screen, look at it.

---

## 9. The habit that matters most here

This session fixed four bugs that had all survived a fully green suite, and
every one of them survived for the same reason: **a test asked whether a thing
existed, and none asked whether it worked in the order a player meets it.**

- A sign that revealed a town twenty-seven rungs behind you.
- A counter that asked for two of something the road offers once.
- A watcher counter read off the fighter as it was *before* the fight.
- A hover that knew two of the five relations the catalogue speaks.

Prose has the same failure mode. A scene can pass every lint in the file and
still be unreadable. **Read it aloud.** If you cannot picture who is talking,
the fix is a name, not another adjective.
