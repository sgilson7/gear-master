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

---

## P2 - the two epigrams, and the lint that stops the register returning

Both lines under the setup screen's headings were knowing, balanced epigrams
restating the cards below them. Neither had ever been checked, because
`prose.rs` is an engine test binary and both strings lived in the GUI.

**What landed:**

- `Mode::WHAT_THE_CHOICE_IS` (`run.rs`) and `Difficulty::WHAT_THE_CHOICE_IS`
  (`combat.rs`). The engine already owns `Action::blurb`, `Outcome::describe`
  and `Requirement::describe`, so screen copy in the engine is established
  practice - and it is the only way the lint can reach it. The CLI picks a mode
  too, so this was never only the window's text.
  - was: *"Losing pays either way. It just does not get you past the thing that
    beat you."* -> **"The two differ in one thing: what a loss takes off you."**
  - was: *"Bigger numbers mean tougher, meaner monsters. Medium is the fight
    the game was built around."* -> **"Set once, for the whole run. It steps the
    gear the opposition wears before it touches any of its numbers."**
    The old line was also wrong about the mechanism: most of a setting is
    `gear_step`, and the numbers are what is left over.
- `prose.rs::scenes()` now reaches both.
- Two `HEDGES` entries - `"the game"`, `"it just does not"`. Probed against the
  whole corpus first: they fire on nothing else.
- A new test, `a_subtitle_does_not_name_the_options_under_it`. A subtitle may
  not name any of the cards it sits above; the difficulty one named MEDIUM, in
  a card that already says "the intended fight" on its own face.

**Proved, not assumed.** Both lints were run against the *old* strings and both
failed - `no_scene_withholds_the_noun` on "it just does not",
`a_subtitle_does_not_name_the_options_under_it` on MEDIUM - then passed on the
replacements. A lint that has never seen the fault it was written for is a lint
nobody has tested.

**The probe that shaped P4 and P5.** Twelve more register tells were tested
against the corpus and every one of them fires on shipped scene text, so they
are held back until the rewrite that removes them. This is the map of the worst
scenes, in the order the lint found them:

| Tell | Where it fires |
|---|---|
| `the worst of it` | `the-vip-area / Walk on`, `the-threshold` landing |
| `of some kind` | `the-shrine-fork` prose |
| `sitting wrong` | `the-bigger-sign` prose |
| `not from anywhere` | `the-inspection` prose |
| `does not say what` | `the-contract` prose |
| `the whole of his` | `the-buyer` prose |
| `stops being strange` | `the-manse` blurb, `the-threshold` blurb (**written twice, verbatim**) |
| `somebody who should not` | `the-crevice` blurb |
| `worth thinking about` | `the-under-mine` blurb (**twice in one sentence**) |

`either way` was tried and dropped: it fires only on
`the-sealed-bid / Name a figure` - *"They read the reserve out either way"* -
which is a statement of fact and exactly what the rest of the file is asking
for.

**Suites:** engine **777 green, 38 ignored, 0 warnings**; gui **61 green**.

---

## P3 - the cast, and the ratchet that says when it has landed

No prose was rewritten here. Two things landed instead: the measurement that
turns the mission into a countdown, and the cast verified against every
namespace it could collide with.

### The lint that let the fault through

`every_scene_names_something` passes on **any digit**. That is the hole M15 went
through: a scene with no name in it satisfied the lint as cheaply with a number
as with a person, so the scenes that lost their proper nouns had numbers bolted
onto them instead - "rice for the trade board for 19 years", "the 3 chairs",
"40 years", "6 demands". Green lint, anonymous scenes.

The same question without the loophole is now a budget, shipped the way
`catalog_shape` and `two_voices` are:

- `no_more_scenes_lean_on_a_number_than_already_did` - **<= 18**
- `the_digit_budget_is_not_slack` - **== 18**, so it cannot silently drift
- `every_scene_names_a_person_or_a_place` - `#[ignore]`d, asserts zero

**`DIGIT_PROPS` is 18 at `a46294a`**, and this is the worklist for P4 and P5,
in table order:

```
what-to-do-with-henpeck   what-the-table-said   the-thrumbus-race   mole-town
the-inspection            the-sealed-bid        the-contract        the-payout
the-buyer                 the-picket-line       the-exhibition      town extra-large
dungeon the-threshold landings                  dungeon the-under-mine landings
dungeon the-undertow landings                   dungeon den-rivals landings
dungeon wumpus-world blurb                      dungeon wumpus-world landings
```

Every batch that lands lowers the number, and the number reaching zero is the
mission being finished rather than the mission feeling finished.

One incidental tightening: `names_something` no longer counts a bare **"I"**.
It is a capital in the middle of a sentence and it is nobody. It changed nothing
today - THE CROWNWRIGHT slips through on a capital after a closing quote mark
instead - but it was a hole of the same kind.

### The cast

Five scenes needed no invention. The name was already in the game and M15
simply did not reach for it:

| Scene | Was | Is |
|---|---|---|
| `what-to-do-with-henpeck` | "The overseer" | **the Hollow King** - `LADDER[14]`, the rung you just cleared |
| `the-under-mine` | boards stamped `HENPECK` | stamped **HOLLOW KING** |
| `the-fork`, `the-slagworks`, the Foreman door | Ossery / "He has been down there" | Ossery, in all three |
| `the-astronomer`, `through-the-cracked-lens` | Halloway | Halloway, said consistently |
| `the-vip-area`, `the-wizards-thirst` | Merrik, Sam the Wise | left alone |

Twenty-three invented, plain-port, no book pastiche:

| Scene(s) | Was | Is |
|---|---|---|
| `back-in-a-minute` | "A man on the road" | Wint |
| `the-casino` | "A woman with a clipboard" | Marlow |
| `the-long-way`, `where-it-was-going` | "a man at the roadside" | Rowe (Gerald is the tortoise) |
| `the-shrine-fork`, `the-crevice` | "an old watchman" / "one old analyst" | Wenlock |
| `the-teller`, `the-bigger-sign` | "the man who runs the place" | Ollam |
| `the-crownwright` | "The crownwright" | Padgett |
| `the-inspection` | "A woman with a clipboard and a folding stool" | Nance Twiss |
| `the-green-ledger` | "The tally man" | Creel |
| `what-the-table-said` | "The landlord" | Salter |
| `the-contract`, `the-payout` | "A man from an underwriting house" | Braddock |
| `the-bird-problem` | "A courier" | Pether |
| `the-buyer` | "The buyer" | Vell |
| `the-exhibition` | "The two finest players" | Dorn and Ilder |
| `the-sealed-bid` | nobody in it at all | Sarn |
| `the-picket-line` | "somebody on this line" | Nettle |
| `the-passenger` | "A courier", the second one | Larkin |
| `the-glow-over-the-ridge` | "A carter coming the other way" | Gull |
| `the-foundry-remembers` | "a man in Slagworks overalls" | Rusk, sent by Ossery |
| `the-thrumbus-race` | "a steward" | Cobb |
| `mole-town` | "an older mole with a case of tools" | Tibb |
| `the-threshold` | "The man behind the cellar door" | Corvin |
| `the-undertow` | "The old man fished here for 60 years" | Fenn |
| `Action::Manager` | "He will confirm" | Mawes |

Plus the four replacements for the shouted book words: gate and Manse plate
**HOLLIS**, boat transom **PATIENCE**, boards **HOLLOW KING**, thrumbus ->
**bolter**.

**Verified, not assumed.** All twenty-nine were checked as whole words against
every double-quoted string literal in `piece.rs`, `combat.rs`, `class.rs`,
`town.rs`, `dungeon.rs`, `rumour.rs`, `theme.rs` and `naming.rs`, against the
`BOOK` list, and against the turtle `vocabulary` keys - the last of which
matters because a character called Frost would be retold as "nut-freeze". All
clear.

**Suites:** engine **779 green, 39 ignored, 0 warnings**.

---

## P4 batch 1 - rungs 1 to 13

Seven scenes in the batch. Six rewritten, one read and left alone.

| Scene | What changed |
|---|---|
| `the-toads-offer` | **nothing.** The Bog Toad is a character, it is concrete, and "it will not say what it is for" is the toad refusing rather than the narrator withholding. One of the good ones |
| `back-in-a-minute` | "A man on the road" -> **Wint**, who says his name and then does not come back, which is worse. The chart's ring is round "a pub two towns up - the one that trades in words rather than money" where it was "the one with the odd words on the back shelf" |
| `the-casino` | "The card room" -> **the Parlour**. "A woman with a clipboard" -> **Marlow**, which also breaks the duplicate prop she shared with THE INSPECTION. The dealer stays the dealer: at a card table that is a job and not a withheld noun |
| `the-long-way` | Rowe is named in the paragraph he is introduced in, so the second one is "Rowe's cart" rather than "His cart". Gerald is the tortoise and always was |
| `the-shrine-fork` | "a works of some kind" -> **a seed line, three floors of it**, which is the dungeon behind the hole and now says so. "an old watchman" -> **an old analyst called Wenlock**, the same man THE CREVICE has at the bottom |
| `the-teller` | "the man who runs the place" -> **Ollam**. "carrying a story for 11 years" - the digit prop - is gone. The abstraction goes with it: *"what it costs to take is exactly what it costs him to keep"* was a riddle over three buttons that say a hundred of your maximum, half of that, and nothing. It says maximum health now, because that is what it is |
| `the-bigger-sign` | "something about it has been sitting wrong" - a `HEDGES` tell - becomes "The thing that has been wrong about it the whole way is the sign", which is the same beat without the fog. "The teller's story" -> "Ollam's story" |

### The printer

`read_the_road_aloud`, an `#[ignore]`d printer in `prose.rs`, in the style of
the ones in `baseline`:

```
cargo test -p gearmaster-engine --test prose -- --ignored --nocapture read
```

Every scene, every town gate and door, every dungeon and its landings, sorted
into the order a player meets them, wrapped the way the screen wraps them, with
the choices under each. It asserts nothing.

It exists because §9 of the handoff is right: every lint in that file is a cheap
mechanical proxy and says so at the top, and the only way to find out whether a
scene reads is to read it. Four fixes in this batch came out of reading the
printer's output rather than out of a failing test - a tangled first sentence in
BACK IN A MINUTE, an awkward name reveal in GERALD, a comma splice in THE
TELLER, and "the figure" twice in one sentence.

**Suites:** engine **779 green, 40 ignored, 0 warnings**; gui **61 green**.
`DIGIT_PROPS` unchanged at 18 - none of this batch's scenes were on that list,
which is itself the point: the eighteen are the *worst*, and these seven were
merely vague.

---

## P4 batch 2 - rungs 14 to 24

Seven scenes. Six rewritten, one left alone. **`DIGIT_PROPS` 18 -> 15.**

| Scene | What changed |
|---|---|
| `what-to-do-with-henpeck` | "The overseer" -> **the Hollow King**, which is `LADDER[14]` and the rung you cleared to get here. No invention needed: M15 stripped a name the base game already owned and put a job title in its place. "All 3 copies" -> "All three copies", the digit no longer load-bearing |
| `the-dispenser` | **nothing.** It is a scene about a machine with nobody at it, and "there is a bottle wedged sideways where somebody's last coin went" says the person is absent rather than withholding who they were. "WORTH IT, which is either a claim or a price" is the best line in the batch |
| `the-crownwright` | "The crownwright" -> **Padgett the crownwright**, once at the top and once at the bottom where the ledger is |
| `the-inspection` | "A woman with a clipboard and a folding stool ... without introducing herself" -> **Nance Twiss**, who gives her name and nothing else. "She is not from anywhere" - a `HEDGES` tell, and the withholding written out loud - is gone, and so is "rice for the trade board for 19 years". She still grades rice; the number was the prop, not the rice. Her second choice also stopped mentioning a cart she does not have |
| `where-it-was-going` | "The man has been paid" -> **Rowe** has been paid. Gerald was always the tortoise, and now both halves of that pair have names in both scenes |
| `the-green-ledger` | "The tally man" -> **Creel**. "He will not say who asked him to" stays: that is Creel refusing, which is the hook, not the narrator hedging |
| `what-the-table-said` | "The inn at this crossroads ... for 40 years. The landlord" -> **Salter's inn**, and nobody has sat at the middle of the table for as long as Salter has held the licence. The forty years were a prop; the licence is a fact about a man |

The ratchet caught the improvement rather than the other way round: the suite
failed with *"the list shrank to 15 - lower DIGIT_PROPS in the commit that
earned it"*, which is exactly what a budget that cannot be slack is for.

**Suites:** engine **779 green, 40 ignored, 0 warnings**; gui **61 green**.

---

## P4 batch 3 - rungs 25 to 36

Nine scenes. **`DIGIT_PROPS` 15 -> 10.**

### A real bug, found by reading

**THE CONTRACT promised a rung nobody stands on.** Its prose said *"you will
believe it at rung 28"*. THE PAYOUT is `at: 28`, and `LadderEvent::at` is
zero-based, so the payout stands on **rung 29**. A player who signed and walked
to rung 28 to collect would have found an empty stretch of road. Trap nine in
CLAUDE.md, and this is the fourth bug it has caused.

Nothing caught it because nothing reads prose for numbers. There is now a pin -
`structures.rs::the_contract_names_the_rung_the_payout_actually_stands_on` -
which builds the string from `event("the-payout").at + 1` and asserts the
contract's prose contains it. Pinned rather than generalised on purpose: a lint
over every figure in every scene cannot tell which of them is meant to be a
rung. This one is.

### The scenes

| Scene | What changed |
|---|---|
| `the-contract` | "A man from an underwriting house" -> **Braddock**, who underwrites and says the word the way another man would say farriery. *"He does not say what their side is"* - a `HEDGES` tell, and fault two in its purest form - now says their side: four rungs' worth and one loss underwritten, which is exactly what THE PAYOUT's own button pays. A contract whose terms you cannot read is not a decision, it is a shrug |
| `the-bird-problem` | "A courier" -> **Pether**, which also breaks the duplicate he shared with THE PASSENGER fifteen rungs later |
| `the-payout` | "the same man behind it" -> **Braddock** again, and "the 4th one down" gone |
| `the-astronomer` | Halloway was already named. What changed is the close: it ended *"He says it as though it settles something"* and AHEAD OF SCHEDULE ends *"as though that settles it"*. Same beat, two scenes, seven rungs apart. Halloway "offers the crack as the proof" instead |
| `the-vip-area` | Merrik was already named. The "Walk on" blurb ended on *"which is the worst of it"*, which THE THRESHOLD's last landing uses better; Merrik holds the door instead, which stings more |
| `the-wizards-thirst` | **nothing.** Sam the Wise is named, the hoard is concrete, and "entirely unwilling to say why" is the wizard refusing rather than the narrator hedging |
| `the-buyer` | The type specimen for fault two. *"he buys the things a run has that a run cannot put a price on, and he can, which is the whole of his trade"* sat directly above three buttons reading a word, a title, and a hundred of your maximum health. It says those three things now, and the paragraph's interest moves to why the room is hard to be in. "The buyer" -> **Vell**; "the 3 chairs" gone, in a scene whose first line mentions one chair |
| `the-exhibition` | "The two finest players" -> **Dorn and Ilder**, and "6 years each" gone. The decline blurb was "One of them says that is fair. The other does not say that is fair" and is now those two by name, which is the same joke with people in it |
| `the-sealed-bid` | Had nobody in it at all. **Sarn** writes the reserve, takes the figures, and reads your losing bid out to the room in the voice he reads the winning one in - which is the sting the scene was gesturing at |

Also: the casino's Marlow works a **book**, not a clipboard. Merrik has a
clipboard, and two of them twenty rungs apart is the same duplicate-prop fault
this pass keeps finding.

### A limitation of the proxy, worth writing down

`names_something` cannot see a name that only ever **opens** a sentence: at a
sentence start it cannot tell "Vell" from "The". THE BUYER named its man twice
and failed the lint anyway. The fix was to write him into the middle of a
sentence, which is better prose regardless - not to widen the proxy, which
would mean keeping the cast list in a test file and fitting the test to its
data, which is the fault this file's own comments warn about.

**Suites:** engine **780 green, 40 ignored, 0 warnings**; gui **61 green**.

---

## P4 batch 4 - rungs 37 to 51, and the two off the road

Ten scenes. **`DIGIT_PROPS` 10 -> 7.** Everything left on that list is a town
gate or a dungeon, which is P5.

| Scene | What changed |
|---|---|
| `the-fork` | **nothing.** Ossery is named, the seam is concrete, and the third paragraph states the choice plainly. One of the best in the game |
| `the-picket-line` | "somebody on this line has been raking up after people like you" -> **Nettle**, who put demand four on the board herself. "6 demands" -> "six demands"; the count is load-bearing because demand four is quoted, but the digit was the prop |
| `the-locked-gate` | The plate said **EGGBERT**, which is the book's. It says **HOLLIS** |
| `the-thrumbus-race` | A thrumbus is the book's animal. The canonical race is run by **bolters**, the title is **THE BOLTER RACE**, and the steward is **Cobb**. `pedestal.rs::Destination::name` carries the title as a second literal and was changed with it |
| `mole-town` | "an older mole with a case of tools" -> **Tibb**; "3 storeys" spelled out |
| `the-passenger` | "A courier" -> **Larkin**, the second of the two couriers this pass has had to tell apart |
| `the-glow-over-the-ridge` | "A carter coming the other way" -> **a carter called Gull** |
| `the-foundry-remembers` | "a man at the roadside in Slagworks overalls" -> **Rusk**, and *Ossery sent him*, which is what the scene was always implying and never said |
| `through-the-cracked-lens` | Halloway "was thrown out of six observatories"; THE ASTRONOMER says "every observatory on this road". The same man, two counts, thirty rungs apart. It says every observatory now |
| `the-second-shadow` | **nothing.** It is carrying your build, the same pieces in the same corners with the same one crooked, and then "It has stopped waiting." Leave it alone |

### The turtle keeps its own words

Two `Retold.prose` entries added - `the-locked-gate` and `the-thrumbus-race`.
Nothing already in `told` was touched. They exist because those two scenes had
been reaching the turtle column by **fall-through**, so moving the canonical
text off EGGBERT and off thrumbus would have left EGGBERT'S GATE standing over
a gate that says HOLLIS. The new entries are the new canonical text with the
book's word put back, so Cobb is at the paddock rail in both voices and the
plate says what each column's plate should say.

### A second thing found by reading: three counters nobody reads

THE PICKET LINE's "Cross it" blurb promised *"This rung's shelf, cheap. The
next three arrive better dressed."* Its outcome is `Pay { times: 2 }` and
`Count("crossed")`. There is no shelf and no discount, and **nothing anywhere
reads `crossed`**.

`Outcome::Count`'s own doc says what the mechanic is for: *"Nothing says a word;
a door forty rungs later reads the tally and says what it noticed."* Three of
the four counters in the game have no such door:

| Counter | Written by | Read by |
|---|---|---|
| `crucible-melts` | the Slagworks' crucible | THE FOUNDRY REMEMBERS |
| `shook-the-machine` | THE DISPENSER, losing its gamble | **nothing** |
| `moles-paid` | paying Tibb in MOLE TOWN | **nothing** |
| `crossed` | crossing THE PICKET LINE | **nothing** |

This is the mirror of `no_flag_is_waited_on_forever`, which catches a flag
waited on and never set. Nothing was catching a counter set and never waited on.
`completable.rs` now carries `COUNTERS_NOBODY_READS = 3` as a budget, with an
`#[ignore]`d target at zero and a third test asserting the other direction.

**Shipped as a budget rather than a fix on purpose.** Closing it means authoring
three doors, which is a content mission and not a prose one. What was in scope
was the blurb, which promised a discount that does not exist and now says what
crossing actually pays.

**Suites:** engine **783 green, 41 ignored, 0 warnings**; gui **61 green**.

---

## P5 - everything the events point at

Dungeons, town gates, the seventeen door blurbs, class blurbs and rumour hints.
**`DIGIT_PROPS` 7 -> 0, and the budget came out of the file.**

### Dungeons

| Dungeon | What changed |
|---|---|
| `the-crevice` | "somebody who should not have it" -> **a line foreman who cannot possibly have paid for it**. "one old analyst left on the line. He still prays..." -> **Wenlock**, four times through the landings, which is the same man the shrine's hole leads down to. The hamlet has canonical names now: it was **Fenmouth** before the company renamed it. "The old man is 71" -> "Wenlock is seventy-one", which is a better line for being a name |
| `the-threshold` | "The man behind the cellar door" -> **Corvin**, who is the only one in the Manse who will say the number of steps out loud - which sets up the stair that has counted 402 of them. The verbatim duplicate of THE MANSE's "the sort of thing that stops being strange about four minutes after you notice it" is gone; the original stays where it was written |
| `the-under-mine` | The boards were stamped **HENPECK** and are stamped **HOLLOW KING**. "which is worth thinking about for a moment and then worth thinking about again" - the tic twice inside one sentence - says what there is to think about. The fourteen diggers are fourteen because **Ossery** said so, and Ossery has never been down there |
| `the-undertow` | "The old man fished here for 60 years" -> **Fenn**, in the blurb and both landings. The transom said **BUNKO**, which is the book's, and says **PATIENCE** |
| `den-rivals` | "The exhibit at the Emporium" - the Emporium is the turtle's - is **the exhibit called THE FURY OF A THOUSAND BEARS**. The placard now says A THOUSAND BEARS and does not say to what scale, which is the joke the landing was reaching for |
| `wumpus-world` | A card nailed at the mouth says **ROOMS 20, HAZARDS SOME, WUMPUS ONE, and somebody has crossed out ONE and written it again**. Signs are this game's motif - LARGE, DAYS SINCE, WORTH IT, ONE (1) HANDFUL - and wumpus-world had none |

### Towns, doors, classes, rumours

- `the-manse` gate: the plate said **EGGBERT**, says **HOLLIS**.
- `extra-large` gate: the manager is **Mawes**, and nobody who works there will
  discuss the pedestal, Mawes least of all.
- Doors: the Foreman is **Ossery** (the same man THE FORK and the Slagworks gate
  name), the Cellar Door is **Corvin**, the Manager is **Mawes**. The Shop's
  "The man with the cart" became "**Whoever** has the cart this week", which is
  the true thing - it is a different man in each of three towns - and the
  Sample Counter's "the woman behind the counter" the same way. A role that
  really is a role is allowed to stay one.
- Rumour hints name **who**, and stay vague about **what**, which is the
  module doc's design and `the_hint_does_not_give_it_away` still holds:
  Padgett, Creel, Halloway, Corvin, Sam the Wise, Dorn and Ilder, Nettle.
- Three class blurbs. **"You carried a dead lord's coat into a bar"** was the
  only "lord" left in the canonical column - a leftover from Lord Drabley
  Henpeck - and the trophy the pub actually takes is a **Scrap Ticket** (whose
  turtle name is, tellingly, "Henpeck's Chit"). Piety knelt in **Sump Bottom**;
  Unionized did not cross **Nettle's** line. Class blurbs are also in
  `prose.rs::scenes()` now, which they had never been.

### The turtle keeps EGGBERT and HENPECK

Two more `Retold.prose` entries - `the-manse` and `the-under-mine` - for the
same reason as P4's two. Nothing already in `told` was touched; four entries
that had been title-only now carry prose, because their titles name people the
canonical column no longer does.

### The seven hedges, and the three left off

`HEDGES` gains the prose pass's own tells, each re-probed against the whole
corpus after the rewrite: `of some kind`, `sitting wrong`, `not from anywhere`,
`does not say what`, `the whole of his`, `somebody who should not`,
`worth thinking about`.

Three were deliberately left off, and the reason is written into the file so
nobody adds them back:

- **`either way`** fires only on THE SEALED BID - *"Sarn reads the reserve out
  either way"* - which is a statement of fact.
- **`the worst of it`** fires only on THE THRESHOLD's last landing, where the
  noun is said: the light is a person and it is pleased to see you. The clause
  is a judgement, not a withheld thing. Its twenty-rungs-earlier copy is gone.
- **`stops being strange`** fires only on THE MANSE, which is the sentence the
  tic was copied *from*. The copy is gone.

### The ratchet the digit budget became

`DIGIT_PROPS` went 18 -> 15 -> 10 -> 7 -> 0, and at zero the budget scaffolding
came out: `names_something` simply no longer counts a digit, and
`every_scene_names_something` is one test again. Numbers are welcome in these
scenes. They are no longer *evidence* of anything.

### And the ratchet that was blind to capitals

`two_voices::leaks()` compares **case-insensitively** now. It compared exact
case for its whole life, and this game puts proper nouns on signs and brass
plates in capitals, so four of them walked past it and shipped. The budget of 5
is untouched by the change - **proved by putting EGGBERT and THRUMBUS back and
watching it report 7 over a budget of 5**, where before the fix it reported 5
and passed.

`pedestal.rs::DESTINATIONS` is walked now too. It had never been, and it
carries each destination's title as a second literal, so THE THRUMBUS RACE was
written down twice and this lint could see neither copy.

**Suites:** engine **781 green, 40 ignored, 0 warnings**; gui **61 green**.

---

## P6 - verification, and what is left

### The numbers

| | At `628185a` | At the end |
|---|---|---|
| engine suite | 776 green, 38 ignored | **781 green, 40 ignored** |
| gui suite | 60 green | **61 green** |
| warnings, whole workspace | 0 (under rustc 1.95) | **0** |
| `DIGIT_PROPS` | 18 | **0, and the budget retired** |
| `two_voices` budget | 5, compared case-sensitively | **5, compared case-insensitively** |
| `COUNTERS_NOBODY_READS` | not measured | **3, recorded** |

### The printers moved nothing

```
cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1
cargo test -p gearmaster-engine --test catalog_shape -- --ignored --nocapture
```

Both ran clean and `git status analysis/` came back empty. A prose pass that
moved a balance number would be a prose pass that touched something it should
not have.

### Two CLI replays, diffed

Two scripted runs - one walking the road, shopping and visiting a town, one
fighting three rungs and answering an event - each piped through
`gearmaster-cli` twice and diffed. 547 lines and 336 lines, byte-identical
both times. A scripted run replaying identically is the design contract and
not a convenience.

The replays also confirm the cast reaches the terminal: Marlow is working the
ring with a book in the CLI's own rendering of THE CASINO, and the route map
draws THE HOLLOW KING at rung 15 one line above HE IS STILL TALKING at 16 -
which is the scene that now names him.

### The road, read

`read_the_road_aloud` over all 1,118 lines, then a scan for the fault this pass
existed to remove. Every surviving "a man" / "the man" in shipped prose is
followed by a name in the same sentence - "A man called Braddock", "The man who
owns it is Ollam", "a steward called Cobb", "The foreman is called Ossery".
The three that are not are the three that should not be:

- Sump Bottom's gate has "a man selling out of a cart", and the Shop door says
  "Whoever has the cart this week". It is a different man in three towns.
- THE CASINO has "the dealer", which at a card table is a job.
- THE TELLER has "a man who has done this three times", which is Ollam,
  already named twice above it.

### What this pass did not do

- **Three counters are still read by nothing** - `shook-the-machine`,
  `moles-paid`, `crossed`. Recorded as `COUNTERS_NOBODY_READS = 3` in
  `completable.rs`, with an `#[ignore]`d target at zero. Closing it means
  authoring three doors, which is content.
- **The MSRV is still a lie.** `Cargo.toml` declares 1.75; this builds under
  1.95 and the code needs 1.83. Untouched because it is not prose.
- **`pedestal.rs::Destination::name` still duplicates each title** as a second
  literal. It is walked by `two_voices` now, so a *book word* in one and not
  the other would be caught, but two different titles still would not be.
- **`the-thrumbus-race` is still the event's id**, and `what-to-do-with-henpeck`
  is still an id naming a book character. Ids are keys and were left alone on
  purpose; nothing shows them to a player.

### CLAUDE.md

**Not edited, deliberately.** The working tree carried an uncommitted rewrite of
it from the previous session - the one that wrote `HANDOFF-prose.md` - along
with an unfinished move of the handoffs into `design/`, and one reference in it
(`analysis/post-unwinding.md`) points at a file that is at `design/` instead.
Committing that would have meant finishing and signing somebody else's
in-flight reorganisation inside a prose commit.

Three things belong in its trap list when that lands:

> **The book-word ratchet used to be blind to capitals.** `two_voices::leaks()`
> compared exact case, and this game puts proper nouns on signs and brass
> plates: EGGBERT, BUNKO, HENPECK and THRUMBUS all shipped in the canonical
> column with a green budget of 5. It compares case-insensitively now. If you
> add a `BOOK` word, add it in the case the book uses and trust the lint.
>
> **A silent counter with no door is dead content.** `no_flag_is_waited_on_forever`
> catches a flag waited on and never set; nothing caught the mirror until
> `completable.rs` gained `COUNTERS_NOBODY_READS`, which is 3.
>
> **`LadderEvent::at` is zero-based and prose is not.** THE CONTRACT promised
> "rung 28" for a payout standing on rung 29. That is trap nine's fourth bug.
> `structures.rs::the_contract_names_the_rung_the_payout_actually_stands_on`
> pins that one; nothing pins the general case, because nothing can tell which
> figure in a scene is meant to be a rung.

And the counts, when they are re-read: engine **781 green, 40 ignored, 51
binaries**; gui **61**.

### `analysis/baseline.md`

Not edited either, and that is the record: it is generated by a printer, the
printer was re-run, and it came back byte-identical. The prose pass is visible
in nothing it measures, which is the correct outcome.
