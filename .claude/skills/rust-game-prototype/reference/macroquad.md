# macroquad 0.4.x reference

API surface read off macroquad 0.4.15 / miniquad 0.4.10 source and compiled
against 0.4.16. Everything listed here exists; anything not listed, check the
crate source before using it — `find ~/.cargo/registry/src -maxdepth 1 -type d
-name 'index.crates.io-*'` finds the vendored copy.

```toml
[dependencies]
macroquad = "0.4"
```

`use macroquad::prelude::*;` re-exports `input`, `math` (glam), `shapes`,
`text`, `texture`, `time`, `window`, `color::colors::*`, `Color`, and
`miniquad::conf::Conf`. `macroquad::rand` is the `quad-rand` crate.

## The frame loop

```rust
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Gear Master".to_string(),
        window_width: 1400,
        window_height: 900,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    macroquad::rand::srand(miniquad::date::now() as u64);

    let mut state = State::new();

    loop {
        clear_background(Color::from_rgba(15, 15, 25, 255));

        // 1. advance time-based state (animations)
        // 2. compute layout from screen_width()/screen_height()
        // 3. render
        // 4. handle input -> mutate the engine

        next_frame().await;
    }
}
```

Notes that will bite you otherwise:

- The `#[macroquad::main]` attribute takes either a `&str` title or a
  `fn() -> Conf`. It handles the macOS main-thread requirement for you.
- `main` is `async` and every frame ends with `next_frame().await`. Any early
  `continue` in the loop **must** `next_frame().await` first, or you spin.
  ChessGame does this in every ability-dispatch branch — copy that discipline.
- Do input *after* render in the frame, so what the player clicked is what
  they saw. State mutated by input shows up next frame.
- Rendering is immediate — nothing persists between frames. Draw order is
  painter's algorithm: last call wins.
- Seed the RNG once at startup or every run is identical.

`Conf` fields: `window_title: String`, `window_width: i32`,
`window_height: i32`, `high_dpi: bool`, `fullscreen: bool`,
`sample_count: i32`, `window_resizable: bool`, `icon`, `platform`. Use
`..Default::default()`.

## Drawing

```rust
draw_rectangle(x, y, w, h, color);
draw_rectangle_lines(x, y, w, h, thickness, color);
draw_rectangle_ex(x, y, w, h, DrawRectangleParams { offset, rotation, color });
draw_circle(cx, cy, r, color);
draw_circle_lines(cx, cy, r, thickness, color);
draw_line(x1, y1, x2, y2, thickness, color);
draw_triangle(Vec2, Vec2, Vec2, color);
draw_triangle_lines(Vec2, Vec2, Vec2, thickness, color);
draw_poly(cx, cy, sides: u8, radius, rotation_deg, color);
draw_poly_lines(cx, cy, sides, radius, rotation_deg, thickness, color);
draw_ellipse(cx, cy, rx, ry, rotation, color);
draw_arc(cx, cy, sides, radius, rotation, thickness, arc_deg);
draw_hexagon(cx, cy, size, border, vertical, border_color, fill_color);
```

Rectangles, circles, lines and triangles cover a whole prototype. ChessGame
draws every piece sprite — eight distinct silhouettes — out of exactly those
four primitives and no art assets. Do that first; textures can come later.

### Borders without a border API

Draw a slightly larger filled rect behind the real one:

```rust
draw_rectangle(x - 3.0, y - 3.0, w + 6.0, h + 6.0, border_color);
draw_rectangle(x, y, w, h, fill_color);
```

## Color

Constants: `LIGHTGRAY GRAY DARKGRAY YELLOW GOLD ORANGE PINK RED MAROON GREEN
LIME DARKGREEN SKYBLUE BLUE DARKBLUE PURPLE VIOLET DARKPURPLE BEIGE BROWN
DARKBROWN WHITE BLACK BLANK MAGENTA`.

```rust
Color::from_rgba(232, 217, 184, 255)   // u8 channels — what you want
Color::new(0.9, 0.85, 0.72, 1.0)       // f32 0..1
```

Wrap your palette in named functions rather than scattering literals:

```rust
fn col_slot_empty() -> Color { Color::from_rgba(40, 40, 55, 255) }
fn col_legal()      -> Color { Color::from_rgba(80, 180, 255, 220) }
fn col_invalid()    -> Color { Color::from_rgba(255, 90, 90, 230) }
```

Alpha as a parameter is how you get fades:

```rust
fn col_trail(alpha: f32) -> Color {
    Color::from_rgba(255, 145, 40, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}
```

For generated palettes (rarity tiers, per-entity hues) there is
`hsl_to_rgb(h, s, l) -> Color` and `rgb_to_hsl(Color) -> (f32, f32, f32)` —
but **they are not in the prelude**, which re-exports only `Color` and the
named constants. Call them by path:

```rust
fn col_for(id: ItemId) -> Color {
    macroquad::color::hsl_to_rgb((id.0 as f32 * 0.37) % 1.0, 0.55, 0.60)
}
```

## Text

```rust
draw_text(text, x, y, font_size: f32, color) -> TextDimensions;
draw_text_ex(text, x, y, TextParams { font, font_size: u16, font_scale, rotation, color, .. });
draw_multiline_text(text, x, y, font_size, line_distance_factor: Option<f32>, color);
measure_text(text, font: Option<&Font>, font_size: u16, font_scale: f32) -> TextDimensions;
get_text_center(text, font, font_size, font_scale, rotation) -> Vec2;
load_ttf_font_from_bytes(&[u8]) -> Result<Font, Error>;
set_default_font(font);
```

**`y` is the text baseline, not the top.** A 16px line drawn at `y = 0` is
invisible.

Centering — measure, then offset:

```rust
let d = measure_text(&name, None, 26, 1.0);
draw_text(&name, x + (w - d.width) / 2.0, y + 36.0, 26.0, WHITE);
```

Right-aligning: `x + PANEL_W - 16.0 - d.width`.

Note the type mismatch in the API: `draw_text` takes `font_size: f32`,
`measure_text` takes `u16`. Pass the same number to both.

There is no automatic wrapping in `draw_text`. Roll a word-wrapper (~15 lines,
see `wrap_text` in ChessGame's `gui/src/main.rs`) and loop over the result:

```rust
for line in wrap_text(msg, 30) {
    draw_text(&line, x + 16.0, y, 15.0, WHITE);
    y += 18.0;
}
```

## Input

```rust
mouse_position() -> (f32, f32)
mouse_delta_position() -> Vec2
mouse_wheel() -> (f32, f32)
is_mouse_button_pressed(MouseButton::Left)    // this frame only — the edge
is_mouse_button_down(MouseButton::Left)       // held — the level
is_mouse_button_released(MouseButton::Left)
is_key_pressed(KeyCode::A)                    // edge
is_key_down(KeyCode::A)                       // level
is_key_released(KeyCode::A)
get_last_key_pressed() -> Option<KeyCode>
get_char_pressed() -> Option<char>            // for text entry
get_keys_down() / get_keys_pressed() -> HashSet<KeyCode>
show_mouse(bool), set_cursor_grab(bool)
is_quit_requested(), prevent_quit()
```

`pressed` vs `down` is the whole of click-vs-drag: use `pressed` to start a
drag, `down` to continue it, `released` to commit it.

Number-row keys are `KeyCode::Key1 ..= KeyCode::Key0`, then `KeyCode::Minus`,
`KeyCode::Equal`. Drive hotkeys off a table so the panel and the handler can
share it:

```rust
let pairs: [(KeyCode, LevelChoice); 3] = [(KeyCode::Key1, One), (KeyCode::Key2, Two), (KeyCode::Key3, Three)];
for (k, choice) in pairs {
    if is_key_pressed(k) { level = choice; break; }
}
```

## Time and animation

```rust
get_time() -> f64          // seconds since start — use for animation clocks
get_frame_time() -> f32    // delta since last frame
get_fps() -> i32
draw_fps()
```

Store `start: f64` from `get_time()` when an animation begins, then each frame
compute `let t = ((get_time() - start) / DURATION).clamp(0.0, 1.0) as f32;`.
This is more robust than accumulating `get_frame_time()` — no drift, and a
stalled frame doesn't desync anything.

Easing and interpolation, in full:

```rust
fn ease_out_cubic(t: f32) -> f32 { 1.0 - (1.0 - t).powi(3) }
fn lerp_pt(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}
```

A pulse for "this is ready / selected", used everywhere in ChessGame's panel:

```rust
let pulse = ((get_time() * 4.5).sin() * 0.5 + 0.5) as f32;   // 0..1
let g = (170.0 + pulse * 85.0) as u8;
draw_circle(x, y, 6.0, Color::from_rgba(80, g, 110, 255));
```

## Screen and geometry

```rust
screen_width() -> f32
screen_height() -> f32
request_new_screen_size(w, h)
set_fullscreen(bool)
screen_dpi_scale() -> f32
```

Read `screen_width()/screen_height()` every frame and derive layout from them;
never cache them. Center by computing the content size first:

```rust
let origin_x = (screen_width() - PANEL_WIDTH - grid_w) / 2.0;
let origin_y = ((screen_height() - grid_h) / 2.0).max(PAD);
```

`Vec2` (glam) and `Rect` come from the prelude:

```rust
Rect::new(x, y, w, h)
rect.contains(Vec2::new(mx, my)) -> bool
rect.overlaps(&other), rect.intersect(other), rect.center(), rect.point(), rect.size()
rect.left() / right() / top() / bottom()
```

`Rect::contains` is the cleanest hit-test you have — prefer it over hand-rolled
comparisons once you have more than a couple of clickable regions.

## RNG

```rust
use macroquad::rand::{self, ChooseRandom};

rand::srand(miniquad::date::now() as u64);
let n = rand::gen_range(0, 10);        // i32 in [0, 10)
let f = rand::gen_range(0.0, 1.0);     // works for floats too
let mut pool: Vec<AbilityId> = POOL.to_vec();
pool.shuffle();                        // ChooseRandom, in place
let pick = pool.choose();              // Option<&T>
```

This is a global-state RNG — fine for shop rolls and UI flourish. For anything
you want to *reproduce* (combat simulation, seeded runs), do not use it: put a
small deterministic PRNG in the engine crate, seeded explicitly, so tests can
pin the seed and the engine stays graphics-free.

## Textures (later, if at all)

```rust
let tex: Texture2D = load_texture("assets/item.png").await.unwrap();
tex.set_filter(FilterMode::Nearest);        // for pixel art
build_textures_atlas();                     // after loading, batches draw calls
draw_texture(&tex, x, y, WHITE);
draw_texture_ex(&tex, x, y, WHITE, DrawTextureParams {
    dest_size: Some(vec2(64.0, 64.0)),
    source: Some(Rect::new(0.0, 0.0, 16.0, 16.0)),   // sprite-sheet cell
    rotation, flip_x, flip_y, pivot,
});
```

`load_texture` is `async` and paths are relative to the working directory —
run with `cargo run -p <name>-gui` from the workspace root, or resolve via
`env!("CARGO_MANIFEST_DIR")`.

## Screenshot hotkey — how an agent sees the game

You cannot look at the window. Wire this up on day one:

```rust
if is_key_pressed(KeyCode::F12) {
    let path = format!("/tmp/gearmaster-{}.png", (get_time() * 1000.0) as u64);
    get_screen_data().export_png(&path);
    println!("screenshot: {}", path);
}
```

`get_screen_data() -> Image` and `Image::export_png(&self, path: &str)` are
both real. Ask the user to press F12 and paste the path, then `Read` the PNG —
the Read tool renders images. It is the only honest way to verify a visual
change short of the user describing it.

## Gotchas

- **A missing `next_frame().await` on an early `continue` hangs the app.**
- **Text `y` is the baseline.** Add roughly the font size to your cursor before
  the first line.
- **First build is slow** (~80 transitive crates). Warm it before you need it.
- **No layout engine, no widgets, no z-index.** Draw order is your z-index.
- **Nothing is retained** — if you don't draw it this frame it isn't there.
- **Screen coordinates are y-down.** Board/grid coordinates usually aren't; do
  the flip in exactly one place (`square_origin` / `hit`) and nowhere else.
