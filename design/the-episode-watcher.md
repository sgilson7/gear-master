# The Episode Watcher — architecture

Written against `c2241cd` plus the working tree of the collapse triage. No code
yet; this is the shape and the decisions, with what was rejected and why.

The curve window (`crates/gui/src/curve.rs`) draws what a trainer *says*. This
is the other half: watching an episode **play**, in the game window, while the
trainer is still running. The mission's own record says why it is worth
building - the single cheapest diagnostic in two missions of this work turned
out to be a key histogram that no instrument produced and a person reading a
proof found in a minute.

---

## 1. What already exists, and it is most of it

**A proof is `(seed, mode, difficulty, [verb])` and nothing else.** Plain text,
a `#` header, one key a line. `Verb::line()` writes it and `Verb::parse` reads
it; pieces are addressed by `#id` rather than by name, precisely so a replay
cannot pick the wrong one out of a tray.

`GEARMASTER_WATCH=<file> cargo run -p gearmaster-gui` plays one **through the
same `Console` the agent uses**, which is the property that makes the window
worth trusting: there is no second implementation of what a verb does. Fights
are handed to `begin_next_fight` instead of being applied, so the battle screen
plays out as it does for a person.

`qproof` already writes one from a learned run, and - the part worth copying -
**it replays the transcript into a fresh `Console` and counts the refusals
before it writes the file.** A proof that does not replay is not a proof.

`packers::learned_recording` already records the packer's presses into a
`&mut Vec<String>`.

## 2. The three gaps

**A. The trainer has no transcript.** `row::run` makes presses from three
places and reports none of them:

| what presses | where it lives | visible to the trainer |
|---|---|---|
| the road - answers, gates, fountains, levers | `row::walk_on` | no |
| the packing | the trainer's own closure, through `row::pack_with` | the *index*, not the verb |
| the fight | `row::run` itself | no |

Two of the three are inside `row.rs`, and the third is a number.

**B. `Watcher::load` reads its file once**, at startup, with
`read_to_string`. There is no way to move on to another.

**C. The GUI starts a run once.** `run = Run::start(h.seed, h.mode,
h.difficulty)` happens inside the `and_then` that loads the watcher, and a
directory of episodes means restarting a run mid-session.

## 3. Decisions

### 3.1 The tape is verbs, not strings, and the closure hands its own back

`Verb` is `Copy`. A tape of ~200 `Verb`s an episode costs one `Vec` and no
formatting; a tape of `String`s costs two hundred `format!`s an episode, four
thousand times, to serve one episode in twenty-five. **Format only when a proof
is written.**

`Pressed` - `row.rs`'s existing per-press record - **gains its `Verb`**. That is
free, it is what the M1 key histogram wanted anyway, and it turns "what did the
packer press" from a question into a field.

The pack closure then **returns what it pressed**:

```
pack: &mut dyn FnMut(&mut Console) -> Vec<Verb>
```

*Rejected: `row::run` taking `Option<&mut Tape>` that the closure also writes
to.* The borrow checker refuses it and `RefCell` to get around a borrow error is
a design smell rather than a design. *Rejected: `Ran` recording road and fight
presses with a `Packing` marker for the caller to splice its own presses into.*
It preserves the order without changing the closure, and "splice at the markers"
is the kind of thing that goes wrong quietly. **The thing that presses should
say what it pressed.**

Blast radius is four `row::run` call sites and two `pack_with` sites, all in
`crates/lab/src/bin/{qrow,qhand}.rs`.

### 3.2 The watcher is a sampler and cannot be a mirror

An episode is about **1.8 s** of training and about **200 presses**. The window
plays a press every 90 ms, so replaying one episode takes about **18 seconds** -
ten times slower than producing it. There is no arrangement in which the window
keeps up.

So: `QROW_WATCH=<dir>` writes a proof every `QROW_WATCH_EVERY` episodes,
default **25**, and keeps the last `QROW_WATCH_KEEP`, default **20**. Four
thousand episodes leaves twenty files rather than four thousand.

And **when an episode ends, the watcher takes the newest proof on disk**, not
the next one in sequence. Skipping the backlog is the point: what is wanted is
"what does it do *now*", and the backlog is by definition stale.

*Rejected: jumping to a newer proof mid-episode.* Watching means watching a run,
and a window that cuts away every few seconds shows a montage of openings.
**Finish the episode, then jump.**

### 3.3 Every written proof is verified, and a failure is a number

`qproof`'s contract, in the trainer: replay the tape into a fresh `Console`,
count refusals, compare the rung, and **do not write a proof that does not
replay** - count it instead and print the count at the end.

The cost is one replay per watched episode, which is verbs applied with no
network and no packing decisions, on one episode in twenty-five. The thing it
buys is that the window cannot silently show a different run from the one that
was trained, which is exactly the class of fault this mission keeps finding.

*Rejected: verifying behind a flag, off by default.* A check that is off is a
check that was not run, and the failure it catches is invisible by construction.

### 3.4 `GEARMASTER_WATCH` takes a directory

No second environment variable. If the path is a directory, follow it; if it is
a file, play it once, exactly as today. The header of each proof carries the
seed and mode, which is what a restart needs.

### 3.5 The restart is the risky part, and it is a list

`run` is not the only per-run state in `main.rs`: `drag`, `pb`, `settled`,
`at_pedestal`, `pedestal_seated`, `pedestal_held` and the watcher itself all
belong to the run standing in front of them. Advancing to a new episode has to
reset the same set startup builds, so the design is **one function that starts a
watched run**, called at startup and again on every advance, with the list of
what it clears written out and commented rather than remembered.

This is where a bug would hide - a leftover `Playback` from the previous
episode's fight is the obvious one - so it wants a test that advances twice and
asserts the second run's rung and phase are the new proof's and not the old
one's.

## 4. Milestones

**W0 - the tape.** `Pressed` carries its `Verb`; the pack closure returns what
it pressed; `Ran` carries the tape. Deliverable: a test in `crates/lab` that a
taped row run replays into the same rung with zero refusals - `qproof`'s
contract, in the loop that will be producing them.

**W1 - the trainer writes proofs.** `QROW_WATCH`, `QROW_WATCH_EVERY`,
`QROW_WATCH_KEEP`; each one verified before it is written; the header carries
the episode, the exploration rate and the block mean so far, because a proof
with no epsilon in it cannot be read - a random press is a random press and not
a policy's opinion. Deliverable: a directory of proofs from a short run, each
replaying, and a count of any that did not.

**W2 - the watcher follows a directory.** Newest-wins selection as a plain
function over `(entries, current)` so it is testable without a filesystem;
`Watcher` gains the advance. Deliverable: tests for selection, and the
`proofs.rs` fix in §5.

**W3 - the window restarts.** The start-a-watched-run function, the reset list,
and a test that advancing twice leaves nothing of the first episode behind.

Ordering: W0 blocks W1; W2 is independent of both and can go first if the
trainer is busy; W3 needs W2.

## 5. A bug found on the way, and it is in the way

`crates/lab/tests/proofs.rs` replays **every** proof as `Mode::Grinder`:

```
let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
```

It parses `# seed` and `# reached` out of the header and never parses `# mode`,
and `analysis/proofs/` holds six Rogue proofs. The test is `#[ignore]`d and runs
only under `make eval`, which is how it has stayed that way.

**It is green, and it is green vacuously.** Run under `--ignored` it reports
`11 proofs replayed` in seventeen minutes. All six Rogue proofs claim
**rung 1**, and a rung-1 run replays to rung 1 in any mode having pressed almost
nothing; the three that go deep - rungs 52, 53 and 26 - are all Grinder, which
is what the test assumes. So the guard has never once been asked the question it
exists to ask.

This is in the way because **`qrow` trains Rogue**, so every proof the episode
watcher produces will be a Rogue proof that reaches a real rung. The failure
would not be silent: it would be a red test saying `claims rung 9 and replays to
4`, which reads as a determinism bug in the engine and is really a mode that was
never parsed. Somebody would spend a day on it. W2 fixes it. `watch.rs` parses
the mode correctly, so the window is fine - only its guard is not.

W0 answered this in its own suite, which is why `tests/row.rs` asserts that the
run it replays **reached at least rung 2**: a test whose subject is trivial is a
test that cannot fail.

## 6. What this will not do

* **Keep up.** Ten times slower than training, by construction. It samples.
* **Show a policy.** It shows the *behaviour* policy, exploration included,
  which is what training actually does - and is why the epsilon goes in the
  header.
* **Attach to another machine.** It follows a file, so a shared disk or an
  rsync loop works and a socket is not involved. Genuine remote viewing means
  the wasm build in `docs/` and serving the log over HTTP, which is a different
  job.
