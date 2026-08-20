# Engine architecture

Everything here is lifted from `~/Documents/ChessGame`, a working Rust +
macroquad game. File references point at that repo so you can read the
original.

## Workspace

`Cargo.toml` at the root:

```toml
[workspace]
resolver = "2"
members = ["crates/engine", "crates/cli", "crates/gui"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
```

Each crate inherits with `edition.workspace = true`.

| crate    | deps                              | contains                          |
|----------|-----------------------------------|-----------------------------------|
| `engine` | `smallvec`, `thiserror` (+ `proptest` dev) | all rules, all state, all tests |
| `gui`    | `engine`, `macroquad`             | rendering, input, animation       |
| `cli`    | `engine`, `rustyline`, `anyhow`   | headless repl driver              |

`engine/src/lib.rs` is just the module list — no logic:

```rust
pub mod ability;
pub mod board;
pub mod coord;
pub mod moves;
pub mod piece;
pub mod run;
```

The `cli` crate is optional but cheap, and it is the only way an agent can
drive real gameplay end to end without a window. Build it once the engine has
a mutation entry point. See `crates/cli/src/main.rs` — a `rustyline` loop
dispatching `help / show / moves / move <i> / restart / quit`, where `moves`
prints a numbered list and `move <i>` plays index `i`. That indirection (list,
then pick by index) is what makes it scriptable.

## The data model

### IDs are newtypes over `u32`, allocated by a registry

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PieceId(pub u32);

impl std::fmt::Display for PieceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.0)
    }
}
```

Derive `Copy, Clone, Eq, PartialEq, Hash, Debug` on every ID type — you will
want all six. Add `Display` early; every log line and every test failure
message gets better.

### The registry is the single source of truth

```rust
pub struct PieceRegistry {
    pub pieces: HashMap<PieceId, PieceMeta>,
    pub alive: HashSet<PieceId>,
    next_id: u32,
}

impl PieceRegistry {
    pub fn alloc(&mut self, side: Side, /* ... */) -> PieceId {
        let id = PieceId(self.next_id);
        self.next_id += 1;
        self.pieces.insert(id, PieceMeta { id, side, /* ... */ });
        self.alive.insert(id);
        id
    }

    pub fn meta(&self, id: PieceId) -> &PieceMeta {
        self.pieces.get(&id).expect("missing piece meta")
    }

    pub fn is_alive(&self, id: PieceId) -> bool { self.alive.contains(&id) }
    pub fn mark_dead(&mut self, id: PieceId) { self.alive.remove(&id); }
}
```

`meta()` panicking on a missing ID is correct for a prototype: a dangling ID
is a bug in your code, not a runtime condition to handle. The panic message
tells you immediately.

Note that `mark_dead` only touches the alive-set — the caller is responsible
for clearing the ID out of containers. Document that split; it is a real
source of bugs otherwise.

### Containers store `Option<Id>`, flat, indexed by hand

```rust
pub const BOARD_SIZE: u8 = 8;
pub type Square = Option<PieceId>;

pub struct Board {
    pub squares: [Square; (BOARD_SIZE as usize) * (BOARD_SIZE as usize)],
    pub terrain: [TerrainKind; (BOARD_SIZE as usize) * (BOARD_SIZE as usize)],
}

#[inline]
fn idx(x: u8, y: u8) -> usize {
    debug_assert!(x < BOARD_SIZE && y < BOARD_SIZE);
    y as usize * BOARD_SIZE as usize + x as usize
}
```

A fixed-size array with a private `idx` helper and a `debug_assert!` beats
`Vec<Vec<T>>` on every axis: it is `Copy`-friendly, cache-friendly, and the
assert catches your off-by-ones in dev builds while release stays branch-free.
Build it with `std::array::from_fn(|_| None)`.

Give the container small, total operations and let the rules compose them:

```rust
pub fn get(&self, x: u8, y: u8) -> Square
pub fn take(&mut self, x: u8, y: u8) -> Square
pub fn place(&mut self, x: u8, y: u8, id: PieceId) -> Square  // returns displaced occupant
pub fn find(&self, id: PieceId) -> Option<(u8, u8)>
pub fn remove_piece(&mut self, id: PieceId)
```

`place` returning the previous occupant is the whole capture mechanic in one
line. Look for that shape — an operation whose return value *is* the rule.

### Parallel-array metadata beats an enum with payloads

`PieceMeta` carries `archetype: PieceArchetype` (a plain behavior tag) plus
optional numeric fields — `swap_resistance: f32`, `suppression_radius: u8`,
`replay_delay: u8`, `trajectory_index: Option<u32>` — all defaulted to
inert values by `alloc`, and set afterwards by `set_*` methods. Adding a new
enemy behavior means adding a field and a default, not restructuring an enum
and fixing every match arm. For a prototype that churns, this wins.

## The mutation entry point

One function mutates the world. It is documented with its ordering, and the
code follows the comment:

```rust
/// Resolve a player move. Strict ordering:
///   1. snapshot parent board + pursuer positions
///   2. pick K alternate candidates
///   3. apply chosen move -> new active id + maybe-captured
///   4. teleport pursuers
///   5. enemy step
///   6. demote prior frontiers
///   7. build alternates from the snapshot
///   8. apply ALL captures via `kill_globally`
///   9. record `last_effect`, bump turn
pub fn play(&mut self, m: &Move) -> RunStatus { ... }
```

Deferring *all* deaths to step 8 is the kind of ordering decision that only
shows up as a bug much later — resolve simultaneously, then apply.

### The effect record

```rust
pub struct MoveEffect {
    pub player_move: Move,
    pub new_active: BoardId,
    pub pursuer_jumps: Vec<PursuerJump>,
    pub enemy_moves: Vec<EnemyMoveRecord>,   // in execution order
    pub spawned_alternates: Vec<BoardId>,
    pub splitter_splits: Vec<SplitterSplit>,
}
```

Stored as `run.last_effect: Option<MoveEffect>`, overwritten each `play`. The
GUI animates purely from this. Rule of thumb: **if the player should see it
happen, it goes in the effect record** — including ordering, because the
renderer replays enemy moves one at a time in `enemy_moves` order.

### Read-only prediction via clone

```rust
pub fn peek_enemy_step(&self) -> Vec<EnemyMoveRecord> {
    let mut graph_clone = self.graph.clone();
    let registry_clone = self.registry.clone();
    enemy_step(&mut graph_clone, &registry_clone, &HashSet::new(), &tape).moves
}
```

Deriving `Clone` on the whole world state gives you preview/foresight/undo
almost free. Keep state cloneable.

## Content authoring

Levels are constructor functions returning a fully-built world:

```rust
pub fn level_1(extra_ability: AbilityId) -> Run {
    let mut registry = PieceRegistry::new();
    let mut initial = Board::empty();

    let player = registry.alloc(Side::Player, smallvec![PAWN, KNIGHT, extra_ability], true, PieceArchetype::Player);
    initial.place(3, 3, player);

    for (x, y) in [(4_u8, 4_u8), (2, 4), (4, 2)] {
        let id = registry.alloc(Side::Enemy, smallvec![PAWN], false, PieceArchetype::BasicEnemy);
        initial.place(x, y, id);
    }

    Run { graph: BoardGraph::new_with_initial(initial), registry, turn: 0, /* ... */ }
}
```

Factor a `fresh_run(registry, board, ...)` helper once the tail of `Run { .. }`
repeats. Do not reach for RON/JSON/serde in a prototype — a constructor
function is greppable, type-checked, and refactorable, and content that lives
in a data file is content the compiler cannot check.

Static data — names, descriptions, hotkeys, pools — lives in `match`
functions over ID constants:

```rust
pub const ALTERNATE: AbilityId = AbilityId(5);
pub const ABILITY_DRAW_POOL: [AbilityId; 10] = [TIME_SNIPE, ALTERNATE, SWAP, /* ... */];

pub fn ability_name(id: AbilityId) -> &'static str { match id { ALTERNATE => "alternate", _ => "?" } }
pub fn ability_hotkey(id: AbilityId) -> Option<char> { match id { ALTERNATE => Some('A'), _ => None } }
pub fn ability_description(id: AbilityId, level: u8) -> String { ... }
```

`ability_description` taking `level` — so text reflects the current numbers
rather than restating them — is worth copying. Tooltips that lie are worse
than no tooltips.

## Tests

Integration tests in `crates/engine/tests/`, **one file per mechanic**:
`level_1.rs`, `abilities.rs`, `snipe.rs`, `dead_pieces.rs`,
`copy_semantics.rs`, `swap_alternate.rs`. ~50 tests, whole suite under a
second.

Shape of a test — build a known world, act, assert on observable state:

```rust
const PLAYER_START: (u8, u8) = (3, 3);
const PAWN_E5: (u8, u8) = (4, 4);

#[test]
fn global_kill_removes_piece_from_all_frontiers() {
    let mut run = level_1(DUMMY_A);
    let initial = run.graph.active;
    let moves = legal_moves(&run.graph, &run.registry, Position::new(initial, 3, 3), run.level);
    let chosen = moves.iter().find(|m| m.is_capture && m.to.x == PAWN_E5.0).unwrap().clone();
    let captured = run.graph.get(initial).board.get(PAWN_E5.0, PAWN_E5.1).unwrap();

    run.play(&chosen);

    assert!(!run.registry.is_alive(captured));
    for id in run.graph.frontier_ids() {
        assert!(run.graph.get(id).board.find(captured).is_none(),
                "frontier board {:?} still contains the dead pawn", id);
    }
}
```

Conventions worth keeping:

- Name coordinates as `const`s at the top of the file. `PAWN_E5` reads;
  `(4, 4)` does not.
- Small local helpers (`fn enemy_ids(run: &Run) -> Vec<PieceId>`) per file.
- Failure messages that name the entity: `"frontier board {:?} still contains
  the dead pawn"`.
- Test names are sentences: `past_is_read_only_after_kill`,
  `swap_resistance_at_1_keeps_pieces_in_place`.
- Unit tests for pure helpers go inline in `#[cfg(test)] mod tests` next to
  the code (see `coord.rs`); anything touching world state goes in `tests/`.

## Worked mapping: a grid-inventory auto-battler

Applying the same skeleton to a Backpack-Battles-shaped game (shop phase,
polyomino items placed in grids, combat that resolves itself on a timer):

```
engine/src/
  item.rs      ItemId, ItemDef (shape, rarity, cooldown, effects), ItemRegistry
  shape.rs     Shape { cells: Vec<(i8, i8)> }, normalize, rotate, fits_in
  slot.rs      SlotKind, Slot { w, h, cells: Vec<Option<ItemId>> }, Loadout [Slot; N]
  shop.rs      roll(round, rng) -> Vec<ItemId>, prices, reroll cost
  combat.rs    simulate(&Loadout, &Loadout, seed) -> CombatLog
  run.rs       Run { loadout, gold, round, wins, losses }, Run::buy/place/sell/fight
```

The mappings, one to one:

- **`PieceId`/`PieceRegistry` → `ItemId`/`ItemRegistry`.** Slot cells hold
  `Option<ItemId>`; item stats live in the registry. A multi-cell item is the
  same `ItemId` in several cells — which is exactly why cells must not hold
  item structs.
- **`Board.place` returning the displaced occupant → `Slot::place` returning
  `Result<(), PlacementError>`** after a `shape.fits_in(slot, anchor)` check.
  Placement legality is engine-side and unit-testable; the GUI only asks.
- **`legal_moves(...) -> Vec<Move>` → `legal_anchors(&Slot, &Shape) -> Vec<(u8, u8)>`.**
  The GUI highlights whatever the engine returns and never computes fit
  itself. This is the single most important line to hold.
- **`MoveEffect` → `CombatLog`**: an ordered `Vec<CombatEvent>` with
  timestamps. Combat is simulated to completion in the engine, deterministically,
  from a seed; the GUI then *plays the log back* against wall-clock time. The
  same phase machine, with `t` indexing into the log instead of a phase enum.
  This also makes combat testable: assert on the log, not on pixels.
- **`level_N()` constructors → `fn starting_run() -> Run`** plus a static item
  table (`item_def(ItemId) -> &'static ItemDef`) in the `match`-function style
  above.

Two things to decide in the engine before any of it renders, because both are
pure functions that want tests first: how a tick resolves (fixed timestep —
e.g. 100ms — with each item accumulating toward its cooldown, ties broken by a
documented order), and how effective cooldown is computed from modifiers.
Backpack Battles uses additive modifiers with an asymmetric formula
(`base / (1 + faster - slower)` when speedups dominate, `base * (1 + slower -
faster)` when slowdowns do); pick your own, but write it as one function with
a test table.
