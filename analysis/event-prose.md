# The plain voice: every measurement, one block a milestone

The mission is rewriting all fifty-three event scenes and every choice on them
out of the withholding register and into one that says what a button does
before it is pressed. Ten milestones, E0 to E9, one chain a commit.

The brief is the artifact the owner approved, which holds the before and after
for all fifty-three doors:
<https://claude.ai/code/artifact/f01a3538-2c0e-4cfd-96ce-8d1e300e440a>

Each block below is headed by the commit its numbers were read off. Nothing
here is a design document; `design/` has that job and this one is the evidence.

---

## E0 - groundwork and three ratchets (read off 8b85b29)

Nothing player-facing changed. What landed is the machinery that will say, at
every later milestone, whether the rewrite is doing what it claims.

### What the road looks like today

| | |
|---|---:|
| Scenes a player reads | **53** (44 road doors, 9 county tiles) |
| Choices on them | **120** |
| Prose words across all 53 | **5,259** |
| Scenes running to three paragraphs or more | **35** of 53 |
| Road closers under forty characters | **4** of 44, ceiling 22 |

### The three ratchets

Each is a budget read off this commit that may only go down, in the shape
`catalog_shape` uses. Each was tightened by one and re-run, to check it fires
and names the offender rather than passing vacuously.

| Ratchet | Where | Budget at E0 | Fires on |
|---|---|---:|---|
| `a_scene_is_two_paragraphs` | `tests/prose.rs` | 35 scenes | `the-theodolite` and 34 others |
| `no_label_is_wider_than_the_widest_one_that_ships` | `gui/src/main.rs` | 102.81 ch/1000px | `the-county-surveyed`, 44 chars in a 428px cell |
| `no_blurb_is_longer_than_the_longest_one_that_ships` | `gui/src/main.rs` | 81.79 ch/1000px | `the-turntable`, 140 chars in a 428x4 box |

**The two geometry ones cannot assert pixels and do not pretend to.**
`measure_text` panics without a graphics context - probed, and it does, which
is trap 32 - and the bundled face's metrics are not readable from a test.
Guessing an advance width would be a lint measuring its own guess. So they
assert a *ratio*: characters per pixel of the cell the string is actually drawn
in, against the worst one in the shipped game, which is known to render because
it ships. Nothing about the font is assumed.

They read both columns. A theme is allowed to return a longer string than the
one it replaces, and the screen draws whichever `retell` gives it - so
`words::retell` and `words::retell_naming` are measured alongside the canonical
strings. **No themed string is worse than the canonical worst**, which is a
thing nobody had checked.

### Two gaps closed on the way

**The printer could not see nine of the fifty-three scenes.**
`read_the_road_aloud` walked `EVENTS` and not `COUNTY_EVENTS` for its whole
life, so THE HUNDRED's tiles - drawn on the same screen, by the same code, held
to the same lints bar two - were never read aloud. They are now, sorted after
the road because a tile stands on no rung.

**The event screen's geometry was written into `render_event` and nowhere
else.** `EVENT_PAD`, `EVENT_CHOICE_GAP`, `EVENT_CHOICE_INSET`,
`EVENT_CHOICE_H`, `EVENT_CHOICE_LABEL_TOP` and `EVENT_CHOICE_STEP` are named
now, and `event_choice_cell_w` is split out the way `wrap_measured` is, for the
same reason: geometry computed between `draw_*` calls can only be checked by
looking at it. The renderer and the lint now share one arithmetic, so a cell
width cannot drift from the one being measured.

### The before-state

`analysis/replays/road-aloud-8b85b29.txt` - all fifty-three scenes with their
choices under them, wrapped the way the screen wraps them, in the order a
player meets them. **Not a fixture**; nothing asserts it. It is what E9 diffs
against, because the one thing no lint in `prose.rs` can do is tell you whether
a voice landed evenly.

### Three bugs found while checking the mechanics, none fixed yet

Each is a shipped blurb that describes something that is not true. All three
are fixed in copy at the milestone that owns the door - E1, E4 and E5 - and
recorded here because a blurb is a string and no test can see any of them.

1. **THE TURNTABLE promises a fork that is not in the graph.** *"the line is
   your choice at the first points."* `dungeon.rs:501` gives floor 0 a single
   exit, `Exit::on(1)`, and its own comment says A7 cut the yard into two
   islands with no track between them. Only the down line leaves the pit, and
   the orb at its buffer stop is the only crossing.
2. **THE ASTRONOMER advertises a bounty that pays zero.** "Turn him in" is
   `BuyOff { times: 0 }`, and `run.rs:2175` computes `bounty * times`. The
   blurb says *"The bounty again"*. The rung goes behind you unfought and no
   gold moves.
3. **THE GLOW OVER THE RIDGE is the same fault.** "Ignore it" is the other
   `BuyOff { times: 0 }` in the game and says *"The rung pays again for the
   trouble."* Two doors, one arithmetic, and nothing could have caught either.

### Three dependencies later milestones have to respect

1. **`analysis/replays/switchyard-full.txt` is asserted**, by `switchyard.rs`
   at :1144 through `include_str!`, and it contains four events' shipped prose
   verbatim. E1 breaks it. It must be regenerated with the reason written into
   that test's own doc comment, which is the repo's standing rule.
2. **`Requirement::Took` compares choice label strings.** Four labels are
   load-bearing across chains - "Ask how he does it", "Plug your ears",
   "Sign it", "TAKE THE DEAL" - and renaming one deletes a door silently. The
   approved draft changes five labels and none of them is one of the four.
3. **Six road doors carry themed prose** in `theme.rs`: THE LOCKED GATE, THE
   BOLTER RACE, THE CROWNWRIGHT, THE CASINO, GERALD and BACK IN A MINUTE. Left
   alone they would run long where the base game has gone short. E8 owns them.

### Suite at E0

Measured on this commit rather than quoted from `CLAUDE.md`, whose figures are
from an older tip and count differently.

| Crate | Passed | Failed | Ignored | Delta |
|---|---:|---:|---:|---|
| `gearmaster-engine` | 1,075 | 0 | 52 | +1, the paragraph ratchet |
| `gearmaster-gui` | 90 | 0 | 0 | +2, the two geometry ratchets |
| `gearmaster-cli` | 12 | 0 | 0 | - |
| `gearmaster-trades` | 36 | 0 | 0 | - |
| `gearmaster-lab --test quests` | 5 | 0 | 0 | - (trap 46: nothing runs these by habit) |

`cargo build --workspace` is warning-free under rustc 1.95.

### Where E0 actually landed

**All four of E0's files are inside commit `4996676`**, whose message is about
clipping rewards out of a Huber loss and has nothing to do with prose. Two
Claude sessions were working in this one checkout at the same time; the other
one staged the whole tree, so `tests/prose.rs`, `gui/src/main.rs`, the
before-state capture and this file all went in under its message, alongside its
own `crates/lab` work.

Left as it is. The commit was not pushed and could have been unpicked, but the
other session was mid-run and rewriting a branch under a live agent is worse
than a wrong commit message. **If you are looking for where the three ratchets
came from, it is `4996676`, not this commit.**

The lesson is the one `CLAUDE.md` already gives about cargo, in git form: one
working directory, one agent at a time, or stage by path and never `-A`.

### One thing that went wrong, worth writing down

Two `cargo` invocations ran at once - a timed-out one of mine that had gone to
the background, plus a fresh one - and the second read a **stale test binary**,
reporting a budget the source did not contain. `CLAUDE.md` already says never
to start a second cargo while one is running; this is what it looks like when
you do. A red ratchet whose message quotes a constant that is not in the file
is the tell.
