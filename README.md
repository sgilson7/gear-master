# Gear Master — prototype

### ▶ [Play it in your browser](https://sgilson7.github.io/gear-master/)

No download, nothing to install. Works on any desktop browser.

A Diablo-shaped take on Backpack Battles: instead of one shared backpack, the
character has **five equipment slots, each its own 6x8 grid**, and gear is not
bought whole — it is **assembled** out of component pieces you drag in.

```
make              # play it
make help         # every command
```

| command | what it does |
|---|---|
| `make` / `make gearmaster` | build if needed, then play |
| `make geared` | play with every slot pre-filled |
| `make release` | optimised build — slower to compile, smoother to run |
| `make cli` | play headlessly in the terminal |
| `make test` | the whole suite, 59 tests, no window needed |
| `make check` | fast type-check |
| `make install` | put a standalone `gearmaster` command in `~/.local/bin` |
| `make uninstall` | remove it again |
| `make package` | build a shareable macOS `.app` (1.3 MB zip) |
| `make web` | build the browser version (244 KB zip) |
| `make serve` | build the browser version and open it locally |

After `make install` you can just type `gearmaster` from anywhere. Plain
`cargo run -p gearmaster-gui` still works too.

## How it plays

1. Drag components from the inventory tray into a slot. Each component only
   goes in its own slot; the drop preview turns green when it fits, red when
   it doesn't.
2. Components that orthogonally touch form a **group**. A group becomes a
   finished **item** when it matches the slot's recipe. Each finished item gets
   a gold outline; an incomplete group gets a red one.
3. **A slot holds as many items as you can fit** — as long as they don't touch
   each other. Two separate gloves in the glove slot is legal and counts twice;
   let them touch and they merge into one illegal lump.
4. Placed pieces always contribute their base stats. **Adjacency bonuses fire
   only once that piece's item assembles** — a gold dot marks one, hollow while
   dormant, filled once live.
5. Some pieces have **positional effects** (blue dot) that change what their
   neighbours are worth. See below.
6. Press **BEGIN FIGHT** and watch the auto-battle play out.

Controls: left-drag to move a piece, right-click to rotate (held or in place),
drag back to the tray to remove, `Esc` to cancel a drag, `F12` to screenshot.
**AUTO-BUILD** fills every slot with a legal loadout if you want to see the
end state immediately.

## Rules as implemented

| Slot | Recipe |
|---|---|
| Helmet | 1 frame + 1–2 plating + up to 1 crest |
| Chestpiece | 1 base + 1–3 layers |
| Gloves | 1 material + 1 mold |
| Greaves | 1 material + 1 mold |
| Weapon | 1 handle + 1–2 damaging + up to 2 accessories |

**Stats**: health, strength, regen (healing per turn), and weapon power (a
multiplier, stored in hundredths so combat stays exactly reproducible).

**Damage per attack = strength × weapon power.** Base character: 100 health,
5 strength, 0 regen, 1.00x power.

**The enemy** — Rust Golem: 400 health, 10 damage per turn, no regen.

**Combat**, each turn in order: you attack → golem dies? → golem attacks →
you die? → both regenerate (capped at max). Stalemate at 60 turns. No RNG —
the same loadout always produces the same fight.

**One adjacency bonus per slot:**

| Slot | Piece | Bonus when assembled |
|---|---|---|
| Helmet | Visor of Focus | +3 strength |
| Chestpiece | Woven Underlayer | +2 regen |
| Gloves | Gauntlet Mold | +2 strength |
| Greaves | Runed Material | +15 health |
| Weapon | Balanced Grip | +0.50x weapon power |

**Positional effects** — these read their surroundings rather than paying a
flat bonus, and each carries a condition:

| Piece | Effect | Condition |
|---|---|---|
| **Runed Edge** (weapon, damaging) | every accessory *touching it* gives double strength | while its weapon is assembled |
| **Hollow Weave** (chest, layer) | +1 strength per empty cell touching it | always |
| **Unbound Core** (chest, layer) | adjacent layers give double health | while its chestpiece is **not** assembled |

Two rules worth knowing: the doubling only reaches pieces that are genuinely
orthogonally adjacent (being in the same item isn't enough), and cells beyond
the grid edge don't count as empty — so the Hollow Weave is worth more out in
open space than shoved into a corner.

Bare-handed you deal 5 damage and lose on turn 10. Fully geared you hit for
71 and win on turn 6. The AUTO-BUILD loadout totals 226 health, 29 strength,
6 regen, 2.45x power — and deliberately shows the mechanics off rather than
maxing the numbers: chest, gloves and greaves each carry two separate items.

## Sharing it

Three ways out, in order of how little friction your friends hit.

### A link (best)

    make web        # -> dist/web/  and  dist/GearMaster-Web.zip
    make serve      # try it locally at localhost:8080 first

macroquad compiles to WebAssembly, so the whole game runs in a browser — 244 KB
zipped. Host it and paste the link in Discord: no download, no security
warnings, works on Windows and Linux and on a phone.

This repo is already published that way: `docs/` holds the built page and
GitHub Pages serves it at
**https://sgilson7.github.io/gear-master/**. To ship a change:

    make publish    # rebuilds docs/, commits, pushes; live in about a minute

* **itch.io** — an alternative host. New project, Kind: HTML, upload the zip,
  tick "This file will be played in the browser", viewport 1600x980.

The interface is authored at a fixed 1600x980 and letterboxes itself into
whatever window it gets, so it stays correct at any size.

### A macOS app

    make package    # -> dist/GearMaster-macOS.zip  (1.3 MB)

A universal `.app` (Apple Silicon + Intel), ad-hoc signed, small enough for
Discord's 10 MB free limit. **Send `dist/READ-ME-FIRST.txt` with it** — macOS
blocks unsigned apps on first launch and that file walks them through the
two-click override. Getting rid of the warning entirely means an Apple
Developer ID at $99/year.

### A Windows .exe

    make package-windows

Needs a one-time `brew install mingw-w64` and
`rustup target add x86_64-pc-windows-gnu`; the script tells you if either is
missing. A Windows friend can also just run `cargo build --release` directly.
SmartScreen will warn on first launch for the same reason macOS does.

## Layout

```
crates/engine   all rules and all state. No graphics dependency, so the whole
                rule set is testable with `cargo test` in under a second.
  shape.rs      polyominoes: normalize, rotate
  piece.rs      component catalog (27 pieces) + the instance registry
  slot.rs       one 6x8 grid: fit checking, anchors, connected groups
  loadout.rs    five slots, per-item recipes, effect resolution, stat totals
  combat.rs     deterministic simulation -> a replayable CombatLog
  run.rs        equip / unequip / rotate / begin_fight

crates/gui      macroquad. Rendering and input only — it asks the engine
                whether a placement is legal and never decides for itself.

crates/cli      headless driver for the same game.

packaging/      package-macos.sh, package-web.sh, package-windows.sh,
                make-icon.py (draws the app icon from scratch, no
                dependencies), and the READ-ME-FIRST files that ship
                alongside the downloads.
```

Built with the `rust-game-prototype` skill in `.claude/skills/`.

## Assumptions made

These weren't specified; they're all one-line changes if you want them
different:

- **"You start with 100 damage" was read as 100 _health_** — the only reading
  consistent with damage being strength × weapon power. `stats.rs:BASE_HEALTH`.
- **Helmet components** weren't specified, so they mirror the others:
  frame + plating + crest.
- **Golem health = 400** (unspecified). Tuned so an ungeared character loses
  and a full build wins in a watchable six turns. `combat.rs:ENEMY_HEALTH`.
- **Base strength = 5** so an unequipped character isn't literally harmless.
- **Effect conditions.** The Runed Edge and Hollow Weave were described without
  saying whether they need their item assembled, so: the Runed Edge requires it
  (a finished weapon focuses its gems) and the Hollow Weave doesn't (its bonus
  already keys off open space). **Unbound Core** exists purely to demonstrate
  the *while-not-assembled* case. Each condition is one field —
  `When::Always | Assembled | NotAssembled` in `piece.rs`.
- **Two spare components** — Bone Frame and Hide Base — were added so the
  helmet and chest slots can host two items at once like the other three
  already could.

## Not built yet

Shop and gold economy, rarity tiers, multiple rounds, crafting/merging,
cross-slot synergies, and any opponent other than the golem.
