# Handoff — the episode watcher shows a run that did not happen

Written against `4a50536`. Read `CLAUDE.md` first, then
`design/the-episode-watcher.md` for what the watcher is and
`analysis/the-collapse.md` for the mission it was built for.

One open bug, and it is close to solved: the root cause is identified and
measured, and the fix is not written.

**Reproduce it:**

```
GEARMASTER_WATCH_DEBUG=1 \
GEARMASTER_WATCH=runs/show/deep-rung13-239A554D7F922603.proof \
GEARMASTER_WATCH_MS=1 cargo run -p gearmaster-gui
```

It panics at `piece.rs:1389`, `missing piece instance`, from
`render_inventory` -> `PieceRegistry::def` -> `instance`, at press 249 of 665.

---

## 1. What the watcher is supposed to guarantee

A proof is `(seed, mode, difficulty, [verb])`, and `lab::proof::write` **refuses
to write one that does not replay**: it feeds the tape to a fresh `Console`,
requires zero refusals and requires the rung it claims. So every proof on disk
is known-good *through a `Console`*.

The window is a second reader of that file, and it does not agree with the
first. That is the whole bug: **the artefact is sound and the viewer is not.**

## 2. The root cause, measured

`Run::undo` restores `registry`, `owned`, `gold` and `loadout` wholesale from a
`BoardSnapshot`, so an undo can make the registry **smaller**. `forget_undo`
exists to stop that being dangerous and its own doc says when it must run:

> Drop the history. Used when the board stops being the one the history
> describes - **a fight ending**, or a run being wiped.

The window carries the stack through a fight anyway. Measured, with the debug
flag above:

```
  frame 5040  press 249/665  pb PLAYING  phase Fighting  undo HELD  next "buy 3"
```

`undo HELD` while `phase Fighting` is the fault in one line. The tape's next
`undo` after that fight then restores a **pre-fight registry**, while the
loadout and inventory still name pieces bought after it - and the first thing
that draws one panics.

It matters here and not in ordinary play because this packer presses `undo` for
**24.7% of every press** (`analysis/the-collapse.md` M4), and the tape at
242-255 is `undo, place, undo, place, undo, place, fight, buy, place, buy,
place, buy, undo, undo`.

## 3. Where to fix it

`crates/gui/src/main.rs`, the watcher's fight arm - the one place that does not
go through the console:

```rust
Verb::Fight | Verb::FightParty => {
    pb = begin_next_fight(&mut run, playback_speed);
    ...
}
```

`Console::apply(Verb::Fight)` reaches `forget_undo` and this does not. The
narrow fix is to call `run.forget_undo()` wherever the window ends a fight -
`begin_next_fight`, and the settle in the `pb.done && !settled` branch - so the
window obeys the same contract the engine states.

**The wide fix is better and was half-tried:** route fights through the console
like every other verb and animate from the log it produces
(`run.log`), so there is one implementation of what a verb does. That was
attempted at `73237eb` and reverted: it did not fix the divergence *on its own*
(the undo stack was still held) and it cost the fight animation, because
`pb` came back `None`. Knowing the cause now, it is worth trying again -
`Playback::new(run.log.as_ref()?, &profiles, speed)` after the console applies
the verb.

## 4. What is already fixed, so it is not re-found

* **An orphaned `Playback` deadlocked the watcher** (`73237eb`). `pb` is cleared
  inside the fight screen, which draws only while the phase is `Fighting`, so
  anything returning the run to the loadout with a live playback froze it - at
  press 167 for twenty-seven thousand frames. Cleared now when the phase is not
  `Fighting`.
* **And that guard skipped `Run::settle`** (`4a50536`), which dropped each
  fight's gold, rung and knock-back - a visible deadlock turned into an
  invisible divergence. `settle` is idempotent; it is called before the playback
  is dropped.
* **A scene had no hand to dismiss it** (`30d6f27`). `pending_scene` is prose
  with a button and only a click clears it; the watcher now spends one beat on
  it at the viewer's pace.
* **A verb is checked against the menu before it is pressed** (`73237eb`), so a
  divergence says where it happened instead of panicking inside `apply`. It does
  not prevent this bug, because the state is already wrong before the press.

## 5. Things that will waste your time

* **Blaming slowness.** Fights animate in real time with a 2.5 s linger and
  there is no fast-forward, so a stalled watcher and a slow one look identical
  from the outside. Two screenshots could not tell them apart; the debug flag
  told them apart in one run. Print the state, do not infer it.
* **Blaming the proof.** It replays in a `Console` - that is checked before the
  file is written.
* **`Phase`.** There are two, `Loadout` and `Fighting`, and neither is a third
  state the watcher is stuck in.
* **`Console::standing_in(run, 0)`.** The `0` is the console's own seed, not the
  engine's RNG, which lives in `Run`.
* **The Rogue wipe.** It was the first suspect for a shrinking registry and it
  is not this: `undo` restores a snapshot, and the run had lives left.

## 6. How to know it is fixed

The reproduction above must reach press 665 of 665 and show `rung 13` in the
banner, which is what the proof claims and what a `Console` replay of it
reaches. Then widen it: `runs/r18-deep` holds 177 proofs from a single training
run, and `analysis/proofs` holds eleven committed ones.

**And it wants a test, because nothing catches this class today.** The natural
one is the window's own version of `lab/tests/proofs.rs`: drive the watcher's
verb-application path headlessly over a committed proof and assert the rung it
reaches. It cannot be the GUI's render loop, but the fight arm and the console
arm are separable from it, and that separation is the fix in §3 anyway.
