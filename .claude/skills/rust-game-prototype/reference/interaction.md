# Interaction, layout, and animation

Patterns for turning a pure engine into something a person can play. The
click-select, panel, modal, and animation patterns are transcribed from
`~/Documents/ChessGame/crates/gui/src/main.rs`; the drag-and-drop section
extends the same approach to grid inventories.

## Layout: one struct, both directions, rebuilt every frame

Layout is derived state, not stored state. Build it from the engine state and
the current screen size at the top of each frame.

```rust
struct GridView {
    id: SlotId,
    origin: (f32, f32),   // top-left in pixels
    cols: u8,
    rows: u8,
    cell: f32,            // pixel size of one cell
}

impl GridView {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.origin.0 && x < self.origin.0 + self.cols as f32 * self.cell
            && y >= self.origin.1 && y < self.origin.1 + self.rows as f32 * self.cell
    }

    /// pixels -> grid coords
    fn hit(&self, mx: f32, my: f32) -> Option<(u8, u8)> {
        let gx = ((mx - self.origin.0) / self.cell).floor() as i32;
        let gy = ((my - self.origin.1) / self.cell).floor() as i32;
        if !(0..self.cols as i32).contains(&gx) || !(0..self.rows as i32).contains(&gy) {
            return None;
        }
        Some((gx as u8, gy as u8))
    }

    /// grid coords -> pixels (top-left of that cell)
    fn cell_origin(&self, gx: u8, gy: u8) -> (f32, f32) {
        (self.origin.0 + gx as f32 * self.cell, self.origin.1 + gy as f32 * self.cell)
    }

    fn cell_center(&self, gx: u8, gy: u8) -> (f32, f32) {
        let (x, y) = self.cell_origin(gx, gy);
        (x + self.cell / 2.0, y + self.cell / 2.0)
    }
}

struct Layout { views: Vec<GridView> }

impl Layout {
    fn from(state: &Run) -> Self { /* compute origins from screen_width()/height() */ }
    fn view_for(&self, id: SlotId) -> Option<&GridView> { self.views.iter().find(|v| v.id == id) }
    fn hit_test(&self, mx: f32, my: f32) -> Option<(SlotId, u8, u8)> {
        self.views.iter().find(|v| v.contains(mx, my))
            .and_then(|v| v.hit(mx, my).map(|(x, y)| (v.id, x, y)))
    }
}
```

**Both conversions live on the same struct.** That is the point — a change to
`origin` or `cell` updates drawing and hit-testing together, and they cannot
disagree.

If your game's y-axis runs bottom-up (chess ranks, for instance), do the flip
inside `hit`/`cell_origin` and nowhere else:

```rust
fn hit(&self, mx: f32, my: f32) -> Option<(u8, u8)> {
    /* ... */ Some((gx as u8, ROWS - 1 - gy as u8))
}
fn cell_origin(&self, x: u8, y: u8) -> (f32, f32) {
    (self.origin.0 + x as f32 * self.cell,
     self.origin.1 + (ROWS - 1 - y) as f32 * self.cell)
}
```

## Click-to-select, click-to-act

For turn-based games. GUI state is one `Option`:

```rust
let mut selected: Option<Position> = None;
```

Per frame, ask the engine what is legal from the selection and render that:

```rust
let dests: Vec<Move> = selected
    .map(|s| legal_moves(&engine_state, s))
    .unwrap_or_default();

for m in &dests {
    let Some(v) = layout.view_for(m.to.slot) else { continue };
    let (cx, cy) = v.cell_center(m.to.x, m.to.y);
    if m.is_capture { draw_circle_lines(cx, cy, v.cell * 0.42, 3.0, col_capture()); }
    else            { draw_circle(cx, cy, v.cell * 0.16, col_legal()); }
}
```

**The GUI never decides what is legal.** It renders whatever the engine
returns and matches the click against that same list:

```rust
if is_mouse_button_pressed(MouseButton::Left) && !anim_phase.is_active() {
    let (mx, my) = mouse_position();
    if let Some((slot, x, y)) = layout.hit_test(mx, my) {
        let clicked = Position::new(slot, x, y);
        match selected {
            Some(from) if from == clicked => selected = None,           // click again = deselect
            Some(_) => {
                if let Some(m) = dests.iter().find(|m| m.to == clicked).cloned() {
                    engine.play(&m);                                     // legal: act
                    selected = None;
                } else if clicked_is_mine(&engine, clicked) {
                    selected = Some(clicked);                            // reselect
                } else {
                    message = format!("no legal move to {}", clicked);   // say why
                }
            }
            None => if clicked_is_mine(&engine, clicked) { selected = Some(clicked) },
        }
    }
}
```

That "say why" branch matters more than it looks: an illegal click that
silently does nothing reads as a broken game.

### Modal ability flows

When an action needs more than one click, model it as an explicit enum rather
than a pile of `Option`s:

```rust
enum AbilityMode {
    None,
    AwaitingActor,
    AwaitingTarget { actor: PieceId },
}
```

A hotkey toggles into the mode (and toggles back out — always give the player
an escape), each click advances one state, and `Escape` resets to `None`.
Dispatch these *before* the normal click handler and `continue` the frame so
the two flows can't both fire. Every branch needs `next_frame().await` before
its `continue`.

## Drag and drop for grid inventories

This is the interaction a slot/backpack game lives or dies by.

```rust
enum Drag {
    None,
    Held {
        item: ItemId,
        /// Where it came from, so a cancelled drag can restore it.
        origin: Option<(SlotId, u8, u8)>,
        /// Mouse position minus the item's top-left at grab time — keeps the
        /// item from snapping its corner to the cursor.
        grab_offset: (f32, f32),
    },
}
```

Three edges, three behaviors:

```rust
let (mx, my) = mouse_position();

// --- pick up ---
if is_mouse_button_pressed(MouseButton::Left) {
    if let Some((slot, x, y)) = layout.hit_test(mx, my) {
        if let Some(item) = engine.item_at(slot, x, y) {
            let anchor = engine.anchor_of(slot, item);          // top-left cell of the item
            let v = layout.view_for(slot).unwrap();
            let (ox, oy) = v.cell_origin(anchor.0, anchor.1);
            engine.lift(slot, item);                            // remove from the grid NOW
            drag = Drag::Held { item, origin: Some((slot, anchor.0, anchor.1)),
                                grab_offset: (mx - ox, my - oy) };
        }
    } else if let Some(item) = shop_hit_test(mx, my) {
        drag = Drag::Held { item, origin: None, grab_offset: (CELL / 2.0, CELL / 2.0) };
    }
}

// --- carry: render preview + validity, every frame while held ---
if let Drag::Held { item, grab_offset, .. } = &drag {
    let shape = engine.shape_of(*item);
    let ghost_x = mx - grab_offset.0;
    let ghost_y = my - grab_offset.1;

    // Snap the ghost to whichever cell the item's top-left is over, and ask
    // the ENGINE whether it fits there.
    if let Some((slot, ax, ay)) = layout.hit_test(ghost_x + CELL * 0.5, ghost_y + CELL * 0.5) {
        let ok = engine.can_place(*item, slot, ax, ay);
        let v = layout.view_for(slot).unwrap();
        let tint = if ok { col_legal() } else { col_invalid() };
        for (dx, dy) in shape.cells() {
            let (cx, cy) = v.cell_origin((ax as i8 + dx) as u8, (ay as i8 + dy) as u8);
            draw_rectangle(cx, cy, v.cell, v.cell, tint);
        }
    }
    draw_item(*item, ghost_x, ghost_y, CELL);                   // follows the cursor, on top
}

// --- drop ---
if is_mouse_button_released(MouseButton::Left) {
    if let Drag::Held { item, origin, grab_offset } = drag {
        let gx = mx - grab_offset.0 + CELL * 0.5;
        let gy = my - grab_offset.1 + CELL * 0.5;
        let placed = layout.hit_test(gx, gy)
            .map(|(slot, ax, ay)| engine.try_place(item, slot, ax, ay).is_ok())
            .unwrap_or(false);
        if !placed {
            match origin {
                Some((slot, ax, ay)) => { engine.try_place(item, slot, ax, ay).ok(); }  // snap back
                None => engine.return_to_shop(item),
            }
        }
        drag = Drag::None;
    }
}
```

The load-bearing decisions:

- **Lift on pickup.** Remove the item from the grid the moment it is grabbed,
  so `can_place` doesn't collide with the item's own old cells. Then a
  cancelled drag is just a re-place at the origin.
- **`can_place` is an engine function**, called once per frame for the hovered
  anchor. The GUI renders red or blue; it never reasons about overlap, slot
  kind, or shape.
- **Anchor on the item's top-left, not the cursor.** Hit-test at the ghost's
  first cell center, not the raw mouse — otherwise a large item placed by its
  middle lands one cell off and it feels wrong in a way players report as
  "clunky" without knowing why.
- **The dragged item draws last**, after every grid, so it is never occluded.
- Right-click while held is a good "rotate": `shape = shape.rotated()` on the
  held item, re-checked on the next frame automatically.

## Immediate-mode panels

A side panel is a cursor and a stack of draw calls:

```rust
fn render_panel(state: &Run, msg: &str) {
    let x = screen_width() - PANEL_WIDTH;
    draw_rectangle(x, 0.0, PANEL_WIDTH, screen_height(), Color::from_rgba(20, 20, 30, 255));

    let mut y = 28.0;
    draw_text("GEAR MASTER", x + 16.0, y, 22.0, WHITE);       y += 28.0;
    draw_text(&format!("Round: {}", state.round), x + 16.0, y, 18.0, LIGHTGRAY); y += 22.0;
    draw_text(&format!("Gold: {}", state.gold), x + 16.0, y, 18.0, GOLD);        y += 32.0;

    draw_text("Controls", x + 16.0, y, 18.0, WHITE);          y += 22.0;
    for line in ["  drag    move item", "  R-click rotate", "  F       fight"] {
        draw_text(line, x + 16.0, y, 15.0, LIGHTGRAY);        y += 18.0;
    }
    y += 14.0;

    for line in wrap_text(msg, 30) {
        draw_text(&line, x + 16.0, y, 15.0, WHITE);           y += 18.0;
    }
}
```

Constant vertical steps (`22.0` for body, `18.0` for list items, `32.0`
between sections) are what make it look designed. A status legend listing what
each color means costs eight lines and saves the player guessing.

Status dots pulse when actionable and go flat grey when not:

```rust
let (dot, text_col) = if usable {
    let pulse = ((get_time() * 4.5).sin() * 0.5 + 0.5) as f32;
    (Color::from_rgba(80, (170.0 + pulse * 85.0) as u8, 110, 255), WHITE)
} else {
    (Color::from_rgba(70, 70, 80, 255), Color::from_rgba(110, 110, 120, 255))
};
draw_circle(x + 22.0, y - 5.0, 6.0, dot);
draw_text(&label, x + 36.0, y, 16.0, text_col);
```

Derive "usable" from the engine (`legal_moves(...)` non-empty, `gold >= price`)
rather than tracking it in GUI state.

## Full-screen modal phases

A shop screen, a class picker, a draft — a modal is an `Option` on the main
state plus an early `continue`:

```rust
let mut draft: Option<Vec<AbilityId>> = Some(draw_pool());

loop {
    clear_background(BG);

    if let Some(cards) = &draft {
        let (mx, my) = mouse_position();
        let hover = card_hit_test(mx, my, cards);
        render_cards(cards, hover);
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(i) = hover { run = build_run(cards[i]); draft = None; }
        }
        next_frame().await;      // REQUIRED before continue
        continue;
    }

    // ... normal gameplay ...
    next_frame().await;
}
```

Render and hit-test share one rect function — this is the rule that keeps
clickable regions honest:

```rust
const CARD_W: f32 = 280.0;
const CARD_H: f32 = 360.0;
const CARD_GAP: f32 = 24.0;

fn card_rect(i: usize) -> (f32, f32, f32, f32) {
    let total = 3.0 * CARD_W + 2.0 * CARD_GAP;
    let x0 = (screen_width() - total) / 2.0;
    (x0 + i as f32 * (CARD_W + CARD_GAP), (screen_height() - CARD_H) / 2.0, CARD_W, CARD_H)
}

fn card_hit_test(mx: f32, my: f32, cards: &[AbilityId]) -> Option<usize> {
    (0..cards.len()).find(|&i| {
        let (x, y, w, h) = card_rect(i);
        mx >= x && mx < x + w && my >= y && my < y + h
    })
}
```

Hover feedback is a brighter border and background — cheap, and without it
cards read as decoration rather than buttons.

## The animation overlay in full

Three pieces: a phase enum, an advance function, and an overlay computation.

```rust
#[derive(Clone, Debug)]
enum AnimPhase {
    None,
    PlayerSlide { start: f64 },
    EnemyMoves { start: f64, idx: usize },   // one enemy at a time
    Settle { start: f64 },
}

impl AnimPhase {
    fn is_active(&self) -> bool { !matches!(self, AnimPhase::None) }
}
```

**Advance** — pure, driven by elapsed time and the effect record:

```rust
fn advance_phase(phase: AnimPhase, effect: Option<&MoveEffect>) -> AnimPhase {
    let now = get_time();
    let Some(effect) = effect else { return AnimPhase::None };
    match phase {
        AnimPhase::None => AnimPhase::None,
        AnimPhase::PlayerSlide { start } if now - start >= PLAYER_SLIDE_SECS =>
            if effect.enemy_moves.is_empty() { AnimPhase::Settle { start: now } }
            else { AnimPhase::EnemyMoves { start: now, idx: 0 } },
        AnimPhase::EnemyMoves { start, idx } if now - start >= ENEMY_SLIDE_SECS => {
            if idx + 1 < effect.enemy_moves.len() { AnimPhase::EnemyMoves { start: now, idx: idx + 1 } }
            else { AnimPhase::Settle { start: now } }
        }
        AnimPhase::Settle { start } if now - start >= SETTLE_SECS => AnimPhase::None,
        other => other,
    }
}
```

**Overlay** — what the renderer must do differently this frame:

```rust
struct AnimOverlay {
    /// Cells whose normal contents must NOT be drawn (the thing is in flight).
    suppress: HashSet<(SlotId, u8, u8)>,
    /// Things to draw at explicit screen positions instead.
    floating: Vec<(ItemId, (f32, f32), f32)>,
}
```

Then rendering is: draw the grids normally but skip suppressed cells, then
draw everything in `floating` on top. The renderer stays a pure function of
`(state, layout, overlay)` — no animation bookkeeping leaks into it.

Frame order that works:

```rust
anim_phase = advance_phase(anim_phase, engine.last_effect.as_ref());
let layout = Layout::from(&engine);
let overlay = compute_overlay(&anim_phase, &engine, &layout);

for view in &layout.views { render_grid_background(view); }
for view in &layout.views { render_grid_contents(view, &engine, &overlay.suppress); }
render_highlights(&dests, &layout);
for (id, pos, size) in &overlay.floating { draw_item(*id, pos.0, pos.1, *size); }
render_panel(&engine, &message);

if !anim_phase.is_active() { handle_input(/* ... */); }
next_frame().await;
```

Background layer first, then contents, then highlights, then floating things,
then UI chrome. Gate *all* input on `!anim_phase.is_active()` so a fast
double-click can't queue a move against a state the player can't see yet.

### Playing back a combat log

For an auto-battler the engine hands you an ordered log with timestamps rather
than a phase-shaped effect. Same idea, simpler:

```rust
struct Playback { log: CombatLog, start: f64, cursor: usize, speed: f32 }

let elapsed = ((get_time() - playback.start) as f32) * playback.speed;
while playback.cursor < playback.log.events.len()
    && playback.log.events[playback.cursor].at <= elapsed {
    apply_visual_event(&playback.log.events[playback.cursor]);   // HP bars, floating numbers
    playback.cursor += 1;
}
```

The fight is already fully decided in the engine — playback is presentation
only. That is what makes speed controls, skip, and replay trivial, and it is
why combat belongs in the engine crate where it can be tested.
