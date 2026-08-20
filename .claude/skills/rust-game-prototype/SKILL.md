---
name: rust-game-prototype
description: Build or extend a playable Rust game prototype with a macroquad GUI — workspace layout, a pure testable rules engine, immediate-mode rendering, click/drag input, and animation. Use when starting a new game prototype in Rust, adding a graphical front-end to game logic, or working in a repo that already follows the engine/gui crate split.
---

# Rust game prototypes with macroquad

Distilled from `~/Documents/ChessGame` — a working 8k-line Rust + macroquad
game, verified building and passing its ~50-test suite on rustc 1.95 /
macroquad 0.4.15. Every pattern below is taken from code that runs, and the
scaffold script here was built and run end to end.

## The one decision that matters

**Split the rules out of the renderer, into their own crate.**

```
GameName/
├── Cargo.toml                  # [workspace] members = engine, gui, cli
└── crates/
    ├── engine/                 # pure rules. ZERO graphics deps.
    │   ├── src/
    │   └── tests/              # integration tests, one file per mechanic
    ├── gui/                    # macroquad. ZERO rules.
    └── cli/                    # optional headless driver (repl)
```

This is not architectural taste — it is the difference between a prototype
you can iterate on and one you can't:

- `cargo test -p gamename-engine` runs the whole rule set headlessly in under
  a second. No window, no GPU, no human. That is your feedback loop.
- A GUI cannot be tested by an agent. Anything that lives only in the GUI is
  effectively unverified. So put nothing there that could be wrong about the
  game.
- When a rule misbehaves there is exactly one crate it can be in.

The engine crate's `Cargo.toml` must not depend on `macroquad`. If you find
yourself wanting `screen_width()` in the engine, you are computing layout in
the wrong crate.

## Get to a moving picture fast

Order matters. Do not build the whole rule set before you can see anything.

1. **Scaffold + first frame.** Run this skill's
   `scripts/scaffold.sh <ProjectDir> [crate-prefix]` (pass `.` to scaffold into
   the current directory; it refuses to overwrite an existing `Cargo.toml`).
   It emits the workspace, all three crates, 7 passing engine tests, and a GUI
   that opens a window with a working click-to-move interaction. Verify with
   `cargo test -p <name>-engine`, then `cargo run -p <name>-gui`.
   The macroquad build is the slow part (~38 transitive crates — its image and
   font stack); warm it before you need it.
2. **The core data model in the engine**, with 2–3 tests. IDs and a registry
   (see below).
3. **Render the state read-only.** No input yet. Look at it.
4. **One interaction end to end** — click or drag → engine mutation → redraw.
5. *Then* the rest of the rules, each with a test, each rendered as you go.

## Five patterns, in the order you will need them

### 1. Opaque IDs + a registry; containers hold only IDs

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PieceId(pub u32);

pub struct Board { pub squares: [Option<PieceId>; 64] }

pub struct PieceRegistry {
    pub pieces: HashMap<PieceId, PieceMeta>,
    pub alive: HashSet<PieceId>,
    next_id: u32,
}
```

The grid stores `Option<PieceId>`, never `Piece`. Consequences you get for
free: an entity can appear in several containers without cloning; "destroy it
everywhere" is one `alive.remove(&id)`; IDs are `Copy` so they pass through
functions without a single borrow-checker argument. `registry.meta(id)` is the
one place data lives.

### 2. One mutation entry point that returns an effect record

```rust
pub fn play(&mut self, m: &Move) -> RunStatus  // sets self.last_effect
```

`MoveEffect` records everything that visibly happened this step — what moved
from where to where, what died, what spawned. The GUI reads it to drive
animation instead of diffing two snapshots or re-deriving intent. Rules stay
ignorant of pixels; the renderer stays ignorant of rules.

Write the ordering of that function down as a doc comment listing the numbered
phases, and keep the code in that order. In ChessGame that comment is nine
steps long and it is the single most useful comment in the codebase.

### 3. Layout is a per-frame value that owns hit-testing both ways

Recompute layout every frame from `screen_width()` / `screen_height()`. Put
*both* coordinate directions on the same struct so they cannot drift:

```rust
impl LayoutEntry {
    fn square_origin(&self, x: u8, y: u8) -> (f32, f32)     // game -> pixels
    fn hit(&self, mx: f32, my: f32) -> Option<(u8, u8)>      // pixels -> game
}
```

Never store pixel coordinates in engine state.

### 4. Animation is a phase machine over wall-clock time

```rust
enum AnimPhase { None, PlayerSlide { start: f64 }, EnemyMoves { start: f64, idx: usize }, ... }

anim_phase = advance_phase(anim_phase, run.last_effect.as_ref());  // once per frame
let overlay = compute_overlay(&anim_phase, &run, &layout);          // what to hide / draw moved
```

`advance_phase` compares `get_time()` against each phase's `start` and returns
the next phase. `compute_overlay` returns a set of cells whose normal
rendering is *suppressed* plus a list of `(id, screen_pos)` to draw at
interpolated positions. Rendering itself stays stateless — it just consults
the overlay. Block input while `anim_phase.is_active()`.

### 5. Immediate-mode UI is a cursor and a shared rect function

No widget library. A panel is `let mut y = 28.0;` then `draw_text(...); y += 22.0;`
repeated. For clickable things, the same function that computes the rect for
drawing computes it for hit-testing:

```rust
fn card_rect(i: usize) -> (f32, f32, f32, f32) { ... }   // used by BOTH render and hit-test
```

## Reference material

Read these when you get to the matching work — they are dense and specific.

- `reference/architecture.md` — crate layout, the ID/registry model, effect
  records, content authoring, the test conventions, and a worked mapping onto
  a grid-inventory auto-battler.
- `reference/macroquad.md` — the verified 0.4.15 API surface (drawing, input,
  text, time, color), the frame-loop skeleton, `Conf`, and the screenshot
  hotkey that lets an agent actually see the game.
- `reference/interaction.md` — click-select, drag-and-drop for grid
  inventories, hover/tooltips, modal screens, and the animation overlay in
  full.

## Rules that keep the prototype workable

- **The engine never imports macroquad.** Enforced by not adding the dep.
- **Every rule gets an integration test** in `crates/engine/tests/`, one file
  per mechanic, asserting on state after a mutation. This is the only part of
  the game you can verify without a human.
- **The GUI holds only presentation state**: selection, hover, drag, animation
  phase, camera. If a field would change the outcome of a game, it belongs in
  the engine.
- **Content is plain Rust constructor functions** (`fn level_1(...) -> Run`)
  until it stops fitting. Do not build a data-file loader for a prototype.
- **`cargo build` warns are signal.** Keep the tree warning-clean; in a
  fast-moving prototype an unused import is usually a half-finished thought.
- **You cannot see the window.** Verify through `cargo test`, a CLI driver, or
  the screenshot hotkey. Never report a visual change as working on the
  strength of the code compiling — say what you actually checked.
