# Baseline — the numbers before the slot rewrite

Captured on `baseline-metrics` at 446 catalog pieces, 57 monster specs, suite
green at 29 suites / 492 tests. Every figure here is the denominator for one of
the acceptance criteria in `gear-slot-basis-rewrite.md` §7.

Regenerate with:

    cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1

Read `crates/engine/tests/baseline.rs` for what each column means. Three points
of method matter when comparing anything below to a later capture:

- **Damage is attributed by activation.** `Event::Activate` precedes its own
  item's effects and carries that item's index; `RunningItem` remembers the grid
  it came from. So a hit belongs to whichever item last activated. Strength and
  power granted by *other* slots therefore land under the slot that swung —
  which is the intended reading of "the weapon deals the damage".
- **Emptying a grid takes its stats with it.** A slot's loose pieces pay flat
  stats whether or not they assemble, so removing only the items would measure a
  much weaker claim. The tables below remove both.
- **Mind damage is counted separately.** It removes maximum health rather than
  dealing damage, so it never reaches `Event::Hit` and cannot appear in a damage
  share. It has its own table, and it changes how the helmet reads.

Rungs are spoken from one and indexed from zero: rung 15 is `LADDER[14]`, The
Hollow King.

## Catalog census

```
                                               helmet    chest   gloves  greaves   weapon    total
pieces                                             80       68       75       51      172      446
inert (no trigger, effect or adjacency)            45       36       44       27       45      197
positional (effect, adjacency or reaction)         11       14        9        7       31       72
- effect                                            4        6        4        3        5       22
- adjacency bonus                                   4        7        4        3        7       25
curse application                                   5        6        8        4       35       58
- searing                                           1        2        2        1       15       21
- frost                                             2        1        4        1        8       16
- stun                                              1        1        2        1       10       15
- misfire                                           1        2        1        1        5       10
reaction trigger                                    3        1        2        1       10       17
- OnAdjacentActivate                                1        1        0        0        6        8
- OnAlignedActivate                                 2        0        2        1        2        7
- PerAdjacentItem                                   0        0        0        0        2        2
OnBattleStart                                       3        2        2        1        3       11
Drain                                               2        2        1        2        2        9
StunStrongest                                       0        0        0        0        1        1
Grow                                                2        3        2        3        4       14
MindDamage                                          3        0        0        0        9       12
GainEmpowerment                                     0        0        2        0        5        7
GainShield                                          2        1        1        1        4        9
GainForking                                         1        1        2        1        2        7
ReduceCooldown                                      0        0        0        3        9       12
pool spend (SpendMana / Spend / Consume)           10        7       10        7       39       73
- Consume                                           4        2        1        2        4       13
power_bonus                                         0        0        0        0       49       49
speed_bonus                                         2        2        6       10       20       40
mind_resist                                         7        1        0        1        2       11
harden (physical or magic)                          4        5        2        2        0       13
health above 15                                    35       49       10       17        2      113
crosses grids (Material or Plating)                23        0       20       18        0       61
```

### As a share of each slot

```
                                               helmet    chest   gloves  greaves   weapon
inert (no trigger, effect or adjacency)         56.2%    52.9%    58.7%    52.9%    26.2%
positional (effect, adjacency or reaction)      13.8%    20.6%    12.0%    13.7%    18.0%
- effect                                         5.0%     8.8%     5.3%     5.9%     2.9%
- adjacency bonus                                5.0%    10.3%     5.3%     5.9%     4.1%
```

## Damage share and time-to-kill

```
## starter - the opening weapon and nothing else

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     3.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol            loss     9.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss     1.30s     0.0%     0.0%     0.0%     0.0%     0.0%    0.0%
40 The Rust Parliament    loss     2.20s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%

## preset - the auto-builder's five-slot board

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win    12.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss     5.05s     0.0%     0.0%     0.0%     0.0%   100.0%    7.6%
40 The Rust Parliament    loss     8.20s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%

## owner - a finished run - 75 pieces, Berserker and Chronomancer

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.60s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     3.20s     0.0%     0.0%     0.0%     0.0%   100.0%    0.2%
25 Cog Priest              win    25.60s     0.0%     0.3%     4.1%     0.0%    95.6%    3.5%
40 The Rust Parliament     win    39.00s     0.0%     0.3%     4.2%     0.0%    95.5%    0.0%

## friend - a finished run - 76 pieces, half of it deliberately loose

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     2.60s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     2.75s     0.0%     0.0%     0.0%     0.0%   100.0%    0.2%
25 Cog Priest              win     7.75s     0.0%     0.0%     0.0%     0.0%   100.0%    0.7%
40 The Rust Parliament     win     7.75s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%

## Weapon share across the whole ladder

build          cleared    weapon %  median ttk    burn %
starter           1/50      100.0%       3.00s      0.3%
preset           10/50      100.0%       9.00s      6.8%
owner            49/50       96.1%      16.00s      5.2%
friend           48/50      100.0%       7.75s      0.5%

## Board cadence - friendly activations a second

build              items activations/s    per item
starter                1          0.50       0.498
preset                 8          1.82       0.227
owner                 13          4.80       0.369
friend                12          2.27       0.189

## Mind damage across the whole ladder (max health removed, not in the shares above)

build          helmet    chest   gloves  greaves   weapon
starter             0        0        0        0        0
preset              0        0        0        0        0
owner            1712        0        0       57        0
friend              0        0        0        0        0
```

## Criterion 3 — with the weapon grid emptied

```
(rung 15 is The Hollow King)

build       rungs won  best rung     rung 15      ttk         what carried it
starter          0/50       none      Defeat     0.8s                 nothing
preset           4/50         13      Defeat     3.4s               burn 100%
owner           34/50         50     Victory     5.0s               burn 100%
friend           8/50         24     Victory     5.1s               burn 100%
```

## Criterion 2 — what one grid is worth

```
## preset - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol              12.00s        12%        12%        38%         0%      flips
25 Cog Priest                    -          -          -          -          -          -
40 The Rust Parliament           -          -          -          -          -          -

## owner - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               3.20s         0%         0%       150%         0%       550%
25 Cog Priest               25.60s      flips         0%      flips         6%        68%
40 The Rust Parliament      39.00s      flips         0%      flips         3%      flips

## friend - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               2.75s         0%         0%       196%         0%      flips
25 Cog Priest                7.75s         5%         0%      flips         5%      flips
40 The Rust Parliament       7.75s         5%         0%      flips         0%      flips
```

---

## What the numbers say

**1. The weapon's share is 96–100%, not the 75–85% the spec estimated.**
Across the whole ladder: starter 100.0%, preset 100.0%, owner 96.1%, friend
100.0%. The target of 55–65% is a longer move than §7 assumed — roughly 35
points, not 15.

**2. Of the four armour slots, only gloves land any blows — and only the helmet
is doing invisible work.** Chest reaches 0.3% of damage and greaves 0.0% in
every build at every rung; gloves reach 4.2% on the owner's board. The helmet
also reads 0.0%, but that is the metric, not the slot: on the owner's board it
removes **1,712 points of maximum health** as mind damage over the ladder, which
never touches `Event::Hit`. Greaves add 57. Chest and gloves add none.

So the honest ranking of who contributes to killing things today is: weapon,
then gloves (small, real), then helmet (substantial, but through a channel the
damage share cannot see), then chest and greaves (nothing).

**3. Criterion 2 is far off for two slots, mixed for one, already met for one.**
Emptying a grid at rungs 10/25/40 costs:

| Slot | Cost when emptied | Against the ≥25% bar |
|---|---|---|
| Greaves | 0–6% | far short |
| Chest | 0–12% | far short |
| Helmet | 0–12% on time, but flips two of the owner's fights | short on time, real on outcome |
| Gloves | 38–196%, flips three fights | already passes |

Helmet is the instructive case. It costs almost no time-to-kill and yet removing
it flips the owner's rung 25 and rung 40 outcomes — it is already an engine,
paying out through mind damage and stats rather than through the clock. That is
the Economy axis, present but illegible.

**4. Criterion 3 passes today, for the wrong reason.** Two boards clear rung 15
with an empty weapon grid — but `what carried it` is **burn 100%** in every
case. No armour slot's items land a blow without a weapon; the wins are a searing
curse ticking down the clock. The criterion wants restating as "clears rung 15
on damage the other four axes actually deal", or it is satisfied on the day the
sweep starts and tells us nothing.

**5. Criterion 4's denominators (rung 1 time-to-kill):** starter 3.00s, preset
1.50s, owner 1.60s, friend 2.60s. The ±20% band is measured against these.

**6. The catalog's real problem is inertness, not misplacement.** 197 of 446
pieces (44%) carry no trigger, no positional effect and no adjacency bonus.
Per slot: gloves 58.7%, helmet 56.2%, chest 52.9%, greaves 52.9%, weapon 26.2%.
This is the largest single cost in the rewrite and the spec does not name it.

**7. The reaction slot has no reactions.** Gloves carry zero
`OnAdjacentActivate` — all eight live on weapon (6), helmet (1) and chest (1).
Greaves carry zero as well. The slot the spec makes the exclusive home of the
reaction game starts from nothing, so PR 7 is authoring rather than migration.

**8. 61 pieces cross grids.** Every Material (37) floats between gloves and
greaves; every Plating (24) floats between helmet and greaves. Chest cannot
receive a Plating at all, so the spec's Greaves→Chest bleed link does not exist
in the recipes. Per the settled decision, those two kinds carry no identity
mechanics and become the designated bleed carriers.

**9. A note on the preset.** The engine's own auto-built board clears only
10 of 50 rungs, against 49 and 48 for the two human boards. It is a fair
reference for an early-to-mid build and should not be read as a strong one.

**10. Board cadence, and what it prices.** A built board does 1.8–4.8 friendly
activations a second (preset 1.8, friend 2.3, owner 4.8); the starter board,
which is one item, manages 0.5. The rewrite spec guessed "about one a second"
when setting the value of a `Watch` trigger — a board nobody plays.
`rating.rs` now assumes **two**, which is what a reasonable build gets and the
standard every other discount in that file is set by.

---

## Drift log

Every entry here is a fight that moved and why. The baseline above is not
rewritten - it is what the game measured as before the rewrite started, and it
stays that.

### PR 5 — the weapon's reaction and denial monopolies leave

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 → 1/50 | 3.00s → 3.00s | 100.0% → 100.0% |
| preset | 10/50 → 10/50 | 9.00s → 9.00s | 100.0% → 100.0% |
| owner | 49/50 → **50/50** | 16.00s → 25.60s | 96.1% → 96.1% |
| friend | 48/50 → **49/50** | 7.75s → 7.75s | 100.0% → 100.0% |

**Time-to-kill at rungs 1, 10, 25 and 40 did not move at all**, on any of the
four boards. Criterion 4 - the early game feeling the same - holds exactly
rather than within its ±20% band.

The owner's median moved because the *set* of cleared rungs grew, not because
any fight got slower: clearing one more rung adds a long fight to the list the
median is taken over. No individual fight regressed.

Two boards each clear one rung more than they did. That is monsters getting
weaker, not players getting stronger: sixteen of the rewritten pieces are worn
by creatures on the ladder - Manaflay, Quickening Charm, Chain Coil, Cursed
Blade and Kingsbane among them - and every one of them gave up a monopoly this
pull request. The gloves pieces that received those monopolies are in the
catalogue but on nobody's board yet, so the exchange is currently one-sided.
It stops being one-sided when the shop starts selling the other half.

Weapon damage share did not move on any board, which is the expected result:
curses, drains and reactions are not damage, and the exodus was never going to
touch the number the acceptance criterion is about.

### PR 6 — the curse exodus, and the feet get an identity

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 3.00s | 100.0% |
| preset | 10/50 → **11/50** | 9.00s → 8.45s | 100.0% |
| owner | 50/50 | 25.60s | 96.1% |
| friend | 49/50 | 7.75s | 100.0% |

**Time-to-kill at rungs 1, 10, 25 and 40 has still not moved at all** on any
board, across both exoduses. Criterion 4 continues to hold exactly rather than
within its band.

Seventeen weapon pieces gave up frost, stun or misfire; six signature carriers
keep theirs. Fifteen greaves molds arrived to receive them. The preset clears
one more rung, which is again monsters getting weaker rather than players
getting stronger - the new molds are in the shop but on nobody's board.

Weapon damage share has not moved across five pull requests, and it will not:
curses, drains and reactions are not damage. Getting the weapon from 96% to the
target 55-65% is entirely a question of the other four slots learning to deal
damage of their own, which is the helmet, chest and gloves sweeps.

### PR 7 — the hands answer

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 3.00s | 100.0% |
| preset | 11/50 | 8.45s | 100.0% |
| owner | 50/50 | 25.60s → 24.00s | 96.1% → **92.9%** |
| friend | 49/50 | 7.75s → **5.55s** | 100.0% |

**The weapon's share moved for the first time in the whole rewrite.** Gloves
went from 4.1% of the owner's damage at rung 25 to 7.6%, and the weapon gave up
the difference. Three pull requests of moving curses, drains and reactions did
not shift this number by a tenth of a point, because none of those are damage.
Thirty-eight gloves pieces learning to answer their neighbours moved it three
points in one go.

Rung-1 time-to-kill is unchanged on all four boards, for the third pull request
running. Criterion 4 holds exactly.

The remaining distance is 92.9% against a target of 55-65%, and the shape of
the fix is now known rather than guessed: it is helmet, chest and greaves
learning to deal damage in their own vocabularies, the same way gloves just did.

### PR 8 — the helmet finds its economy

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 3.00s | 100.0% |
| preset | 11/50 | 8.45s | 100.0% |
| owner | 50/50 → 49/50 | 24.00s → 22.40s | 92.9% → 93.1% |
| friend | 49/50 | 5.55s | 100.0% |

Twenty-four inert helmet frames and crests gained pool income, shields,
watchers or mind damage. The owner's board gives a rung back: monsters wear
these frames, and a helmet that banks mana is a monster that casts more.

Rung-1 time-to-kill is unchanged for the fourth pull request running.

Weapon share is flat at 93%. Economy is not damage either - the helmet feeds
the weapon rather than replacing it, which is what the axis says it should do.
The two slots left that could move this number are chest and greaves.

### PR 9 — the body

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 3.00s | 100.0% |
| preset | 11/50 | 8.45s | 100.0% |
| owner | 49/50 | 22.40s → 23.10s | 93.1% |
| friend | 49/50 | 5.55s | 100.0% |

Thirty-two inert chest bases and layers gained armour on activation, and three
grow instead. Chest filler 16 → 0. Nothing else moved: armour is not damage,
and rung-1 time-to-kill is unchanged for the fifth pull request running.

**All four armour slots now sit at zero filler.** What is left of the shape
work is greaves' two axis quotas and the mechanics still sitting in the wrong
slot - `health above 15` (30), `Grow` (10), `harden` (8), `speed_bonus` (10),
`OnBattleStart` (9) - plus the 43 identity mechanics on floating kinds.

### Burn is attributed now, and it changes the reading

The damage share used to count burn apart and credit it to nobody. That was
defensible while a curse was only ever a weapon's; it is wrong the moment a
slot is meant to deal its damage *through* curses, which is the design. Burn is
now split across the slots that lit it, in proportion to how many searing
curses each applied - the burn event carries no source, so proportion is the
most the log can honestly support.

Re-reading the same boards with the same catalogue:

| Build | Weapon share, burn unattributed | Weapon share, burn attributed |
|---|---|---|
| starter | 100.0% | 100.0% |
| preset | 100.0% | **97.8%** |
| owner | 93.1% | **89.6%** |
| friend | 100.0% | **99.8%** |

Nothing about the game changed between those two columns. Three and a half
points of the owner's damage were already coming from somewhere other than the
weapon and the instrument could not see it. Every weapon-share figure recorded
in this file before this entry is measured the old way and reads about three
points high on a board that carries curses.

### PR 10 — the feet, and Francis repacked to meet them

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 3.00s | 100.0% |
| preset | 11/50 | 8.45s | 97.8% → 97.5% |
| owner | 49/50 | 23.10s → **19.40s** | 89.6% → **83.6%** |
| friend | 49/50 | 5.55s → 5.45s | 99.8% → 99.7% |

**Six points off the weapon in one pull request**, and the largest single move
of the rewrite. Greaves deal damage now - through searing, which is the only
curse that is damage - and the burn attribution added last time is what makes
it visible. Rung-1 time-to-kill is unchanged for the sixth pull request.

Francis was repacked at band 6 rather than band 0. Arming the greaves armed
*him* first, because he wears them: at the old band the best board any human
has built in this project could not beat him on Easy, which is a regression
rather than a rebalance. Forty-four pieces instead of fifty; the packer's two
dials are density and power, and this pull request needed the second one down.

Weapon share stands at 83.6% against a target of 55-65%.

### The band, re-derived

Criterion 1 asked for 55–65%, stated against an expected baseline of "~75–85%".
The measured baseline was **96.1%** — the estimate was twenty points low,
because it predated burn attribution, mind damage counting, and any replay of
the ladder at all.

The band was never an independent target. It was "take twenty to thirty points
off what we think the weapon is doing." Against the figure the game actually
has, that intent is **66–76%**, and the spec now says so.

Standing at 82.9%, that is seven points short rather than eighteen. It is still
short, which is why chest gets a way to deal damage.

### Reflection — the body gets an attack

`Stats::reflect` is a percentage of what your armour absorbs, turned back on
whoever swung. It is chest-exclusive, and it is the only offensive verb that
*is* outlasting: it needs the blow to land and be soaked first, so it pays
nothing to a board that dies quickly and everything to one built to be hit.
It cannot be reflected in turn - the return is dealt directly - so two
reflecting boards cannot bounce a blow between them for ever.

One carrier for now, Thorn Layer at ten percent. Four were armed and three
came back out: `Adamant Base` and `Bastion Base` are worn by Francis, and
reflection on his chest cost the friend's board the Medium setting. A fourth,
at eight percent, shifted a fight by a single activation and tripped the guard
in `debt_is_a_debt_and_takes_real_time_to_pay_off`, which compares two runs and
checks the same items fired in both.

Neither is an argument against the mechanic. Both are the same lesson every
sweep here has taught: the creatures wear the catalogue too, and arming a slot
arms them first. Spreading reflection across the chest is a sweep of its own,
and it wants the monster boards handled in the same change - which is exactly
what the greaves sweep needed and got.

---

## Re-baselined at Medium — one times

Every figure above this line was measured on **Easy**. A run opens there, which
is why it was picked, but opening difficulty and reference difficulty are not
the same question, and Medium is the setting with no multiplier on it. The
whole file above therefore reads one setting easier than the balance is meant
to sit.

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| starter | 1/50 | 4.50s | 100.0% |
| preset | 10/50 | 12.00s | 97.5% |
| owner | 46/50 | 14.40s | **79.5%** |
| friend | 46/50 | 5.45s | 99.8% |

Against the Easy figures: the owner drops from 49 rungs cleared to 46, and the
starter's first fight goes from 3.00s to 4.50s — half again as long on the rung
the game opens with, which is worth knowing before anything makes the early
ladder denser.

Weapon share reads **79.5%**, not 82.9%. Lower, because the fights are longer
and the slots that pay over time - reflection, burn, reaction damage - get more
of the fight to pay in. That is the honest number against the 66-76% band, and
it is three and a half points from the top of it rather than seven.

---

## The two owed criteria, measured at last

Criteria 2 and 3 had not been re-measured since the baseline. Both are read-only
questions the harness already knew how to ask; neither had been asked since the
catalogue changed under them.

### Criterion 3 — a build with no weapon clears rung 15

**Passes, and now for the right reason.**

| Build | Rung 15 | What carried it |
|---|---|---|
| owner | Victory in 6.6s | **greaves 66%** |
| friend | Victory in 7.3s | **gloves 76%** |
| preset | Defeat | burn only |

At the baseline this passed with `burn 100%` on every board - a searing curse
ticking down the clock while no armour slot landed a blow. Two slots now carry
a weaponless kill on their own gear. That is the existence proof the criterion
was written to demand, and it was a hollow pass before.

### Criterion 2 — stripping a slot costs 25% or flips the fight

| Slot | Best showing | Verdict |
|---|---|---|
| Gloves | 63-295%, flips three fights | **passes** |
| Helmet | 42-181%, flips rung 40 | **passes** |
| Greaves | 19-44%, flips rung 40 | **passes** |
| Chest | **0% everywhere** | **fails** |

Three of four slots now cost a build real time or the fight itself. Chest costs
nothing measurable anywhere, on any board, at any rung - which is exactly what
the spec set out to fix and the one place it is not yet fixed.

The cause is known rather than mysterious: reflection is chest's only way to
hurt anything and it has **one carrier**, Thorn Layer at ten percent. Three more
were armed and came back out - two are worn by Francis and cost the friend's
board a setting, and a third shifted a fight by one activation and tripped a
guard in `towns`. Spreading it is Phase 2's next job, and it wants the monster
boards in the same change.

### Reflection spread, and where it stopped

Seventeen chest pieces carry reflection now, up from one: bases at five to ten
percent, layers at four to seven. Suite green, and **nothing on the ladder
moved** - because none of the seventeen is worn by any creature.

That is also why chest still measures **0%** on criterion 2. The sixteen new
carriers were safe to arm precisely because nothing uses them, and "nothing"
includes the three reference boards. The owner's chest is Adamant Base, Bulwark
Layer, Deep Roots Base, Emberplate, Riveted Layer, Runed Lining, Runic Weave,
Scale Layer, Seedbed Layer, Thornmail Layer and Wellspring Base - and only Scale
Layer and Thornmail Layer are armed.

Arming six of those was tried and pulled back out. At five or six percent it
cost the owner's board its one setting against Francis; halved, it still tripped
`debt_is_a_debt_and_takes_real_time_to_pay_off`, which compares two runs and
requires the same items to fire in both, so a single shifted activation breaks
it.

So the remaining work is exactly located: **the chest gear that matters is the
gear creatures and finished boards share**, and arming it needs the monster
repack and the `towns` fixture in the same change. `debt_is_a_debt` is not in
`fixtures.rs` - it does not name a piece, it depends on activation counts being
identical, which is a different and more brittle kind of coupling worth adding
to that manifest.

---

## The reference boards were being rebuilt wrong

Every figure above this line is measured against boards that did not assemble
the way the players built them.

A finished board packs to within a cell or two of full, so nearly everything on
it touches nearly everything else. `Shared::loadout` seated every piece
correctly - 62 of 62, 75 of 75, 76 of 76, nothing dropped - and then derived
items from that in a single pass at the end, which asks which pieces are
connected. On a board that dense the answer is "most of them". The owner's
nineteen weapon pieces came back as **one** item; the perfect run's eleven came
back as **none**.

Locking each item the moment it assembles is what the player was doing while
building it: a locked item is finished, nothing may join it, and the next piece
packs flush against it rather than into it.

| Board | Items assembled, before → after |
|---|---|
| perfect, weapon | **0 → 3** |
| perfect, whole board | 12 → 17 |
| owner, weapon | 1 → 2 |
| owner, whole board | 13 → **19** |
| friend, whole board | 12 → **17** |

And the measurements move with it:

| Build | Cleared | Median TTK | Weapon share |
|---|---|---|---|
| owner | 46/50 | 14.40s → **9.00s** | 79.5% → **86.0%** |
| friend | 46/50 → **50/50** | 5.45s → 5.15s | 99.4% → 99.3% |

The friend's board clears the whole ladder now, which is what a run that cleared
the whole ladder ought to do — the clearest sign the reconstruction is right.

**Weapon share at one times is 86.0%, not 79.5%.** The weapon was
under-assembling more than the armour was, so repairing it raised the weapon's
share rather than lowering it. Against the re-derived 66–76% band that is ten
points out, not three and a half. Every drift entry above understates the
weapon, and the direction of each sweep still holds — the same fault ran before
and after all of them — but the absolute figures did not.

---

## Everything, re-measured on boards that assemble

Every entry between the re-baseline at Medium and this one recorded three
figures — cleared, median time-to-kill, weapon share — because those were the
three the sweep was moving. The per-slot table underneath them had not been
retaken since the very first capture, which was on Easy, against a catalogue
that has since had eleven sweeps through it, and on reference boards that were
being rebuilt wrong. So the shares below the weapon's were quotes from a
different game.

This is the whole harness run again, at Medium, on boards that assemble the way
their owners built them. Nothing was changed to produce it.

```

## Catalog census - 469 pieces

                                               helmet    chest   gloves  greaves   weapon    total
pieces                                             80       69       82       66      172      469
inert (no trigger, effect or adjacency)            21        4       11        8       54       98
positional (effect, adjacency or reaction)         11       15       53       15       23      117
- effect                                            4        7        4        3        5       23
- adjacency bonus                                   4        7        4       11        7       33
curse application                                   5        5        4       26       20       60
- searing                                           1        1        1        6       16       25
- frost                                             2        1        1        9        2       15
- stun                                              1        1        1        6        3       12
- misfire                                           1        2        1        5        1       10
reaction trigger                                    3        1       47        1        2       54
- OnAdjacentActivate                                1        1       34        0        0       36
- OnAlignedActivate                                 2        0       11        1        2       16
- PerAdjacentItem                                   0        0        2        0        0        2
OnBattleStart                                       3        2        2        6        2       15
Drain                                               2        2        9        2        0       15
StunStrongest                                       0        0        1        0        0        1
Grow                                                2        6        1        3        4       16
MindDamage                                          6        0        0        0        9       15
GainEmpowerment                                     1        0        2        0        5        8
GainShield                                          5        1        1        1        4       12
GainForking                                         1        1        2        1        2        7
ReduceCooldown                                      0        0        8       11        8       27
pool spend (SpendMana / Spend / Consume)           10        7       10        7       30       64
- Consume                                           4        2        1        2        4       13
power_bonus                                         0        0        0        0       49       49
speed_bonus                                         2        2        6       12       20       42
mind_resist                                         7        1        0        1        2       11
harden (physical or magic)                          4        5        2        2        0       13
health above 15                                    35       49       10       17        2      113
crosses grids (Material or Plating)                23        0       20       18        0       61

### As a share of each slot

                                               helmet    chest   gloves  greaves   weapon
inert (no trigger, effect or adjacency)         26.2%     5.8%    13.4%    12.1%    31.4%
positional (effect, adjacency or reaction)      13.8%    21.7%    64.6%    22.7%    13.4%
- effect                                         5.0%    10.1%     4.9%     4.5%     2.9%
- adjacency bonus                                5.0%    10.1%     4.9%    16.7%     4.1%
## starter - the opening weapon and nothing else

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     4.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol            loss     9.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss     1.40s     0.0%     0.0%     0.0%     0.0%     0.0%    0.0%
40 The Rust Parliament    loss     2.20s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%

## preset - the auto-builder's five-slot board

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win    13.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss     6.75s     9.8%     6.5%     0.0%     0.0%    83.7%    3.0%
40 The Rust Parliament    loss     4.35s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%

## owner - a finished run - 75 pieces, Berserker and Chronomancer

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     2.00s     0.0%     0.0%     0.0%     0.7%    99.3%    0.0%
25 Cog Priest              win    10.50s     1.9%     1.4%    16.6%     7.9%    72.2%    0.3%
40 The Rust Parliament     win    42.00s     0.0%     0.0%     5.3%     0.0%    94.7%    0.0%

## friend - a finished run - 76 pieces, half of it deliberately loose

rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     2.60s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     2.60s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest              win     5.15s     0.1%     0.8%     0.5%     0.1%    98.5%    0.2%
40 The Rust Parliament     win     7.75s     0.0%     0.5%     0.1%     0.0%    99.4%    0.0%

## Weapon share across the whole ladder

build          cleared    weapon %  median ttk    burn %
starter           1/50      100.0%       4.50s      0.8%
preset           10/50       97.5%      12.00s      0.9%
owner            46/50       86.0%       9.00s      0.1%
friend           50/50       99.3%       5.15s      0.0%

## Board cadence - friendly activations a second

build              items activations/s    per item
starter                1          0.49       0.492
preset                 8          2.00       0.250
owner                 19          6.79       0.358
friend                17          2.99       0.176

## Mind damage across the whole ladder (max health removed, not in the shares above)

build          helmet    chest   gloves  greaves   weapon
starter             0        0        0        0        0
preset              0        0        0        0        0
owner               0        0        0       63        0
friend            476        0        0        0        0
## With the weapon grid emptied

(rung 15 is The Hollow King)

build       rungs won  best rung     rung 15      ttk         what carried it
starter          0/50       none      Defeat     0.7s                 nothing
preset           2/50          8      Defeat     2.5s               burn 100%
owner           43/50         50     Victory     6.8s             greaves 45%
friend          27/50         41     Victory     6.9s              gloves 49%
## preset - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol              13.50s       181%        89%       152%        44%      flips
25 Cog Priest                    -          -          -          -          -          -
40 The Rust Parliament           -          -          -          -          -          -

## owner - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               2.00s         0%         0%       125%         0%       350%
25 Cog Priest               10.50s         0%         0%        81%        14%        83%
40 The Rust Parliament      42.00s      flips         0%      flips      flips         5%

## friend - time-to-kill with one grid emptied

rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               2.60s         0%         0%       296%         0%       338%
25 Cog Priest                5.15s         0%         0%       163%         0%       342%
40 The Rust Parliament       7.75s         0%         0%      flips         0%      flips

```

### What the numbers say now

**1. The catalogue is no longer inert, and the weapon is the inert slot.**
Pieces with no trigger, no effect and no adjacency have gone from 197 of 446
(44%) to **98 of 469 (21%)**. Per slot the ordering has inverted: chest 5.8%,
gloves 13.4%, greaves 12.1%, helmet 26.2% — and **weapon 31.4%**, which is now
the dullest grid in the game. That is the sweep working. It is also a thing to
watch: the weapon's identity is conversion, and a conversion piece can be a flat
damage number honestly, so this is not automatically a fault. It is a fault if
`the dearest third interacts` starts failing there, and today it does not (37.5%
against a 35% floor).

**2. The two monopolies are gone.** Reaction triggers: gloves **47**, weapon
**2**, where the census that opened this file read gloves 2 and weapon 10. Curse
applications: greaves **26**, weapon **20**, from greaves 4 and weapon 35. Both
of those were named as the rewrite's actual subject, and both have moved further
than the spec asked.

**3. The armour slots deal damage on the owner's board, and the measurement can
see it.** At rung 25 the owner reads gloves **16.6%**, greaves **7.9%**, helmet
1.9%, chest 1.4%. Every one of those was 0.0% or a rounding error at the
baseline. At rung 40 it collapses back to gloves 5.3% and weapon 94.7%, which is
the honest shape of the problem: the armour contributes where the fight is long
enough for cadence to matter and vanishes where the weapon one-shots the rung.

**4. Criterion 1 stands at 86.0%** on the owner's board against a band of
66–76%. The friend reads 99.3% and the preset 97.5%, which are not failures of
those boards so much as a statement about what they are: the friend's is half
loose pieces and the preset is twenty-one hard-coded placements that predate the
rewrite.

**5. Criterion 2 fails for chest and only for chest.** Emptying a grid at rungs
10/25/40 costs the owner gloves 125%/81%/flips, greaves 0%/14%/flips, helmet
0%/0%/flips; the friend, gloves 296%/163%/flips. Chest reads **0% on every
board at every rung**. Seventeen chest pieces reflect and no creature wears one,
so the slot's only attack has never been in a fight.

**6. Criterion 3 passes and the reason improved again.** With the weapon grid
emptied the owner takes **43 of 50 rungs** and reaches rung 50; rung 15 is a
victory in 6.8s carried by **greaves 45%**. The friend takes 27 and rung 15 in
6.9s on **gloves 49%**. At the baseline every weaponless win was `burn 100%`.

**7. Board cadence is 6.79 activations a second on the owner's board**, against
the 2.0 that `rating.rs` assumes when it prices a `Watch`. The assumption was
calibrated when the owner's board was mis-assembling into 13 items; it holds 19
now. Anything that watches is under-priced on a good board by roughly three
times, and that is a `rating.rs` correction, not a cadence problem.

**8. Mind damage moved slots entirely** — owner 1,712 on the helmet before, 63
on the greaves now; friend 0 before, 476 on the helmet now. Both boards are the
same boards. This is the reconstruction fix showing up in a channel nobody was
watching, and it is the clearest single illustration that figures taken before
it are not comparable to figures taken after.

---

## One way to rebuild a board, and what the other three were hiding

`Shared::loadout` locks each item the moment it assembles. Three tests
hand-rolled the same placement loop without that step — `towns.rs` (seventeen
tests stand on it), `francis.rs`, and the packer's own reference boards — so the
fault that was fixed in `share.rs` went on running everywhere it was measured
from. The engine never locks on its own; locking is something the player does,
with a button. Replaying placements without replaying the locks replays half of
what was done.

All three go through `common::board_from` now. What that changed:

### Francis, against the two finished boards

| board | setting | rebuilt wrong | rebuilt right |
|---|---|---|---|
| owner | Easy | Victory 43.00s | Victory 43.00s |
| owner | Medium | Defeat 43.00s | Defeat 43.00s |
| owner | Hard | Defeat 40.00s | Defeat **32.00s** |
| owner | Insane | Defeat 39.00s | Defeat **27.75s** |
| friend | Easy | Victory 14.00s | Victory **9.50s** |
| friend | Medium | Victory 11.40s | Victory **9.50s** |
| friend | Hard | **Defeat 8.35s** | **Victory 17.10s** |
| friend | Insane | Defeat 6.95s | Defeat 8.70s |

Two things worth reading twice. **The friend's board beats Francis on Hard**,
and `francis.rs` pinned that as a defeat — a pin taken against a board holding
twelve items instead of the seventeen its owner built. And the repack did work:
Hard went from the nine and a half seconds the module doc complains about to
**seventeen**. The pin has been re-aimed at that, by the clock rather than by an
outcome, because the clock is what actually moved. Whether the final boss ought
to stop the best board in the project at Hard rather than at Insane is a design
question, and it is recorded rather than answered here.

The owner's board **dies faster** properly assembled — 40.0s to 32.0s on Hard,
39.0s to 27.75s on Insane. A board that merges into a handful of over-full items
acts less often and holds more, which is a different fight, not a weaker one.

### The debt guard was standing on the same board

`towns::debt_is_a_debt_and_takes_real_time_to_pay_off` required both runs to
have the same number of income events and then asserted the gap between their
mana curves was exactly the debt. Both halves held while the board came back
holding thirteen items and neither holds now that it holds nineteen.

The reason is worth keeping. The curve records income, not spending, so what
sits between two income events is everything the board paid for in between — and
a board in debt cannot always pay. A spend that fails leaves the pool *higher*
than it would otherwise have been, which closes the gap without a point of the
debt being repaid. A test demanding a constant offset was demanding a board too
poor to spend. It asks three things now: the fight opens exactly the debt short,
the indebted pool is never above the free one at any shared moment, and climbing
back to zero takes real time.

### And the boards are pinned by name now

`decode_build::the_boards_come_back_holding_exactly_these_items` writes out all
fifty-one items across the three shared boards, by member name. Counts and ladder
results agreed for the whole rewrite while the reconstruction was wrong —
nineteen weapon pieces coming back as one item is still one item, and one item
still fights. Checked against the fault rather than assumed: reverting the
incremental lock fails it with `owner came back holding a different item / left:
(Helmet, "Aegis Crown + Asker's Monocle + Bulwark Plating + Overflow Plate") /
right: (Helmet, "Aegis Crown + Warding Plate")`.

**The packer's reference boards are correct from here on**, which is what the
monster repack was waiting for: its acceptance curve is read off the owner's
board, and it was reading a board nobody built.

---

## The curve the repack packs against, and what it is really pinned to

The gate in `design/monster-themes.md` §6 asks "is this the right difficulty for
this rung", against `target(rung) = 2.8s + 0.4s × rung`, ±30%, read off the
owner's board at Medium. It justified itself by saying the line runs through
where the game already sits, citing a median of 14.4s.

Both halves of that were wrong and the line is right anyway.

**The median is 9.00s**, on a board that assembles. **And the line does not run
through the game**: the owner's board settles 37 of its 45 wins on its own, and
of those, **13** land inside the band.

```
rung  1 Cave Rat            1.50s   want  2.80s      rung 23 The Gearwright      4.55s   want 11.60s
rung 10 Warded Idol         2.00s   want  6.40s      rung 26 Mire Behemoth      24.00s   want 12.80s
rung 20 Bone Cantor         8.05s   want 10.40s      rung 33 Iron Abbot         27.00s   want 15.60s
```

The ladder is a scatter, not a ramp, which is the thing the repack exists to
fix — so a target the ladder does not follow is exactly what a target should be.
It was never a description and should not have been written as one.

**The other eight wins are not measurements at all.** Sudden death begins at 30s
(`combat.rs:40`), so rungs 40, 42, 43, 44, 45, 46, 48 and 49 — all between 37s
and 43s, with 43.00s appearing four times — are being finished by escalation
rather than by anybody's gear. A curve fitted through those is a curve fitted
through the clock. (A least-squares line through everything reads 911ms a rung;
through the fights the gear actually settles, 622ms.)

Which gives the slope the justification it should have had. **The band's top
edge has to clear sudden death.** At 0.4s a rung the line reaches 22.4s at rung
50, and +30% of that is 29.1s — just inside the 30s where the clock takes over.
Any steeper and the packer would be authoring the top of the ladder into a
region it cannot measure. So the line is unchanged, for a reason that holds.

### The casino corridor, measured

Both doors key off rungs 1–10, so the repack is inside them from its first
board.

| | now | needs | room |
|---|---|---|---|
| sharp run's best early win | 1600ms | < 3000ms | **1.4s of slack** |
| plain board's best early win | 4500ms | ≥ 3000ms | 1.5s of slack |
| plain board's worst early win | 44000ms | > 10000ms | not close |

Stronger early creatures slow both boards, weaker ones speed both, so the
binding constraint is the 1.4s: that is how much the early ladder may be
hardened before the casino door shuts on the build it was written for.

---

## The wall could not fight, and three other things a slot list was deciding

Rungs 7-13 are Walls. Getting them there took four attempts and turned up the
one place where the theme design and the engine genuinely disagreed.

**A creature fights entirely through its gear.** Exactly one creature on the
ladder has an innate attack, and it is the Cave Rat's bite; every other one of
the fifty-three boards deals damage only through what it is wearing. Wall is
Chest and Helmet, and neither slot deals damage. So a themed wall lands nothing,
ever: The Iron Warden packed into one slow chest item and could not hurt
anybody, two of them were no harder to fight than one, no rung on the early
ladder offered two busy enemy items, nothing burned in twelve fights. Nine tests
failed in nine vocabularies describing one hole.

Reflection was meant to be the answer and structurally cannot be. It needs the
player to swing first and the wall's armour to soak the blow; it is reported as
`Reflected` rather than `Hit`, so nothing measuring whether a creature can hurt
you is able to see it; and it never threatens somebody who out-damages it. It is
a way of punishing an attack, not a way of making one. **A wall carries one
weapon item now** - the most any creature may carry - and is otherwise unchanged.

Then the same fault three times over, each time from the slot list doing a
second job nobody had asked it to do:

1. The theme *allowed* the weapon and the packer filled chest and helmet first,
   so every wall came back weaponless - the armour had spent the board before
   the search reached the weapon. **A list of permitted slots was acting as a
   priority order.**
2. Seating the weapon first produced a wall wearing a sword and no armour. One
   weapon item is a handle, two damaging pieces and two accessories: five, which
   was the whole budget. **A cap on items is not a cap on pieces.**
3. Giving every slot an equal share of the budget left the thinnest walls unable
   to fit a chest item at all, and left a mini-boss unable to meet its rank
   inside a share split four ways. **A share is a ceiling and must never be a
   reason a slot cannot hold one item.**

Each slot takes a rolling share now, floored at one item - or at the items its
rank owes - and the weapon goes down first wherever a theme has one.

### The long way asks for fifteen seconds

The door that sends an ordinary run down the road wanted a **20s** win in rungs
2-9. Nothing can produce one any more: a board blunted until it grinds - the
winning build with its weapon off, at 27x - takes **18.0s** at its slowest down
there, and a board blunted further cannot reach the pay-off twelve rungs later.

The threshold had been raised from ten to twenty, and the reason was recorded:
"a creature carries a piece a rung now". That is not true where this door is
asked. The density curve is deliberately **flat** across rungs 1-10, four or
five pieces, so the casino stays reachable; a piece a rung only begins above
that. So the shallow end got lighter again and the line came back down with it.
Fifteen separates the two boards that matter - sharp 8.0s, grinding 18.0s - and
twenty separates nothing, because nothing reaches it. The door's own prose has
always said "that last one took eleven seconds".

### Where the ladder stands after two clusters

| | |
|---|---|
| Rungs repacked | 2-13, less Rust Colossus |
| Skipped | Rust Colossus - the weakest wall buildable at rung 12 still takes 4.5s against a 3.0s target |
| Casino corridor | sharp 1600ms, plain 4500ms - unmoved from before the repack began |
| Owner's board | clears 45 of 50, median 9.00s |

---

## The body was never doing nothing; it was being asked the wrong question

Criterion 2 strips one grid and reads time-to-kill. Chest has read **0-3%** on
that for the whole rewrite, and the conclusion drawn - written into the handoff,
the spec and this file - was that chest does nothing.

It is doing more than any other slot.

Time-to-kill is the right instrument for four slots and the wrong one for the
fifth. A slot that deals damage, denies tempo or pays for casting all show up on
the clock. The body does not: strip it and the fight takes the same time, you
simply arrive at the end of it with less left. Measured that way:

```
## owner - health left at the end, one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol                2941        44%        29%        -3%        23%         0%
25 Cog Priest                 3141        49%        29%        88%        15%        -5%
40 The Rust Parliament        3261        49%        28%        25%        14%        -4%

## friend - health left at the end, one grid emptied
10 Warded Idol                2865         9%        48%        19%        21%         0%
25 Cog Priest                 2865         9%        48%        24%        21%        85%
40 The Rust Parliament        2862         9%        48%        63%        21%        84%

## preset - health left at the end, one grid emptied
10 Warded Idol                 710        21%        43%         2%        21%        56%
25 Cog Priest                  143        18%        37%         1%        35%         0%
40 The Rust Parliament         149        17%        36%       -54%        34%         0%
```

Chest costs **28-48%** of the health a build walks away with, on every board at
every rung - the most consistent contribution of any of the five. The criterion
was reading a defensive slot in an offensive currency.

### And the remedy that did not work

The plan for chest was to arm reflection more widely on the gear the finished
boards wear. Six pieces were armed at five to nine percent - Adamant Base, Plate
Layer, Riveted Layer, Runic Weave, Rimeguard Base, Becalming Layer, all of them
on the owner's or the friend's chest.

**It moved the time-to-kill figures by nothing at all.** Not a little: the table
came back byte-identical.

The reason is in the mechanic. Reflection pays a share of what your *armour*
ate, armour resets to zero every fight, and a board that kills a rung-25
creature in twelve seconds is never carrying much of it - so `absorbed_total`
is the binding constraint and the percentage is not. Arming more of it is
pushing on the wrong end.

The six were reverted. What they did do was re-gear every creature on three of
the four difficulty settings, through `stepped_component`, and take two tests
red - which is a lot of movement to buy a column of zeros.

`report_what_a_slot_is_worth_in_health` is the reading for this slot now, and
the spec says so at criterion 2.

---

## Drift — the greaves sweep

Twenty-three greaves pieces stopped carrying the body's padding. Greaves'
bleed axis was 57.6% against a 20-25% band, the largest single gap in the
ratchet; it is 22.7% now, and every greaves quota is in band:

| | own axis | bleed | filler | dearest third | pool-spend |
|---|---:|---:|---:|---:|---:|
| Greaves | 75.8% | 22.7% | 12.1% | 47.6% | 7.6% |

Health, armour and regeneration came off; curse resistance, `speed_bonus` and
`ReduceCooldown` went on, which is the slot's own vocabulary. Six rules fell
with it - `Grow` 10 to 7, `harden` 7 to 5, `Consume` 9 to 7, `mind_resist` 4 to
3, `health above 15` to zero - and six floating carriers went with them, 23 to
17. Twenty-eight rules unmet, from thirty-six.

Two of the sweep's own edits had to be walked back before the suite agreed.
`Ironthread Material` and `Worldweave Material` traded 170 and 240 base health
for armour, and armour is not a smaller version of health: it absorbs before
health and resets each fight, so 60 armour on a rung-4 Bog Toad made the
creature immune to a six-piece fixture rather than merely tougher. They carry
modest armour and regeneration now.

| build | cleared | weapon share | median ttk |
|---|---|---|---|
| owner | 49/50 | **81.4%** | 9.00s |
| friend | 49/50 | 99.3% | 5.45s |

Criterion 1 moved the wrong way by a point, which is what taking stats off a
slot does to a share measured in damage. It is M7's problem and the levers for
it are still untouched.

---

## Drift — the floating carriers, and criterion 1 lands

Forty-three pieces of a floating kind carried an identity mechanic when the
ratchet was written. It is **zero**. `PieceDef::fits` lets a Material into the
gloves and greaves grids and a Plating into the helmet and greaves grids, so
until this held, every "greaves-exclusive" line in the table was a claim about
where a piece was *written*, not where it can sit.

Fifteen pieces changed. Five gloves Materials traded base health for armour and
regeneration; three Platings traded hardening for resistance; every curse came
off a floating kind, because all four curses belong to a slot; and three pieces
that had nothing left took a `Watch` instead, which belongs to nobody.

One rename. `Swiftplate` was forty-five percent haste and an empty stat line,
and haste outside the weapon is the feet's - a promise a Plating cannot keep. It
is **Reckoning Plate** now: it counts the board and settles every sixth
activation. Propagated through `combat.rs` (7 boards), `theme.rs`,
`decode_build.rs` (2 membership rows).

Eleven rules unmet, from twenty-eight.

### Criterion 1

| build | cleared | weapon share | median ttk |
|---|---|---|---|
| starter | 3/50 | 100.0% | 45.00s |
| preset | 9/50 | 100.0% | 7.50s |
| **owner** | **50/50** | **75.1%** | 10.15s |
| friend | 49/50 | 98.0% | 7.75s |

**75.1% is inside the 66-76% band.** It was 86.0% when this pass began and
81.4% one commit ago. The move came from arming the floating kinds: a piece
that answers the board deals damage the attribution can see, where the base
`physical_damage` and `magic_damage` those pieces used to carry never landed at
all outside a weapon (see second-order 10). The owner's board also clears the
whole ladder for the first time.

Criterion 4 holds. Criterion 2 is unchanged and unchanged in character: helmet,
gloves and weapon cost 19-100% of TTK on the owner's board, chest and greaves
cost 0-12% of TTK and 18-44% of the health left, which is the reading that
matches what those two slots do.

---

## Drift — the mechanics come home, and the ratchet goes green

`the_catalog_keeps_every_rule` **passes**. It was sixty-nine rules unmet when
this pass began.

Forty-three pieces moved in this commit. `GainForking` (4), `Consume` (7),
`mind_resist` (3), `Grow` (7), `harden` (2), `OnBattleStart` (6), `speed_bonus`
(8) and three time-curses came home to their slots, and each one was translated
rather than deleted: a herbal doubles the dose instead of growing you, a
gluttonous fang bites harder instead of swelling, a split weave splits the blow.

One rename: `Hastening Crest` is **Watchful Crest**, because a crest cannot
promise haste - haste is the feet's, and a Plating floats into their grid.

Every quota is in band on every slot:

| slot | own axis | bleed | filler | dearest third | pool-spend |
|---|---:|---:|---:|---:|---:|
| Helmet | 77.5% | 21.2% | 25.0% | 42.3% | — |
| Chest | 98.6% | 24.6% | 11.6% | 36.4% | 5.8% |
| Gloves | 63.4% | 20.7% | 15.9% | 63.0% | 11.0% |
| Greaves | 75.8% | 22.7% | 12.1% | 47.6% | 7.6% |
| Weapon | — | — | — | 37.5% | 14.0% |

Identity mechanics on floating kinds: **0**. Dull epic/legendary non-weapons: **0**.

### The four criteria

| build | cleared | weapon share | median ttk |
|---|---|---|---|
| starter | 2/50 | 100.0% | 45.00s |
| preset | 9/50 | 100.0% | 9.00s |
| **owner** | **50/50** | **74.9%** | 10.50s |
| friend | 48/50 | 97.6% | 7.75s |

Criterion 1 holds at 74.9% against 66-76%. Criterion 3 holds. Criterion 4 holds.
Criterion 2 is unchanged: helmet, gloves and weapon cost 19-100% of TTK on the
owner's board; chest and greaves cost almost no TTK and a third of the health
left, which is the reading that matches what those two slots are for.

### What the sweep cost, and what it bought

`rating.rs` gained one correction. A creature's holding pools are re-priced at
what `held_bonus` converts them to rather than at what a player would pay for
the choice - a point of nature is a point of regeneration, and a creature never
spends any of it. Without that, stepping *down* walked Francis into three crowns
carrying nature between them; his regeneration on Easy came out four times what
it was on Medium, and the best board in the project lost to him on the easiest
setting and beat him on the next two.

`fixtures.rs` is down to **one row** from eleven. Four of its predictions came
true in this commit, each naming the test it was about to break before it broke
it. That is the manifest working exactly as designed.

Four tests had their sample widened or their target searched rather than named -
`an_oracle_stops_their_gear` now finds a creature its fixture survives four
turns against instead of trusting rung 31, and `growth_is_kept_after_a_loss_too`
finds the deepest rung that beats its fixture *slowly* instead of assuming the
last one does.


---

# Before the Unwinding — captured 2026-08-25

The denominator for `design/the-unwinding.md`. Everything above this line
belongs to the gear-slot rewrite, which is finished; nothing above is rewritten.

Retaken because the last entry in this file was written two commits before the
rewrite's final ones and the numbers under it moved. Retake with:

    cargo test -p gearmaster-engine
    cargo test -p gearmaster-engine --test baseline -- --ignored --nocapture --test-threads=1
    cargo test -p gearmaster-engine --test catalog_shape -- --ignored
    cargo test -p gearmaster-engine --test two_runs -- --ignored --nocapture

| | last recorded | today |
|---|---|---|
| Suite | 538 green | **548 green**, 33 suites, 35 ignored, 0 warnings |
| `the_catalog_keeps_every_rule` | 69 rules unmet, then green | **green**, 0 unmet, 0 identity mechanics on floating kinds |
| Weapon damage share, owner at Medium | 74.9% | **75.2%** |
| Owner's board | 50/50, median 10.50s | 50/50, median **10.50s** |
| Board cadence, owner | 6.79/s | **6.69/s** |
| Catalogue | 469, then 473 | **473**, inert 104 (22.0%) |

## The casino corridor

Both shallow-end doors key off rungs 1-10, so anything that touches the early
ladder is inside them.

| | now | needs | room |
|---|---|---|---|
| sharp run's best early win | 1,600ms | < 3,000ms | **1.4s of slack** |
| plain run's best early win | 6,000ms | >= 3,000ms | 3.0s of slack |
| plain run's worst early win | 39,000ms | > 10,000ms | not close |

The binding constraint is still the sharp run's 1.4s: that is how much the early
ladder may be hardened before the casino door shuts on the build it was written
for. The plain run's best has moved from 4,500ms to 6,000ms since the last
capture, which widens the other side.

## The harness, in full

```
## Catalog census - 473 pieces
                                               helmet    chest   gloves  greaves   weapon    total
pieces                                             81       69       83       67      173      473
inert (no trigger, effect or adjacency)            20        8       13        8       55      104
positional (effect, adjacency or reaction)         11       15       56       19       24      125
- effect                                            5        7        7        4        6       29
- adjacency bonus                                   4        8        4       14        7       37
curse application                                   3        4        3       30       19       59
- searing                                           1        1        1        5       15       23
- frost                                             0        0        0       12        1       13
- stun                                              1        1        1        7        3       13
- misfire                                           1        2        1        6        1       11
reaction trigger                                    2        0       49        1        2       54
- OnAdjacentActivate                                0        0       36        0        0       36
- OnAlignedActivate                                 2        0       11        1        2       16
- PerAdjacentItem                                   0        0        2        0        0        2
OnBattleStart                                       0        0        0        6        0        6
Drain                                               0        0       11        0        0       11
StunStrongest                                       0        0        1        0        0        1
Grow                                                0        7        0        0        0        7
MindDamage                                         11        0        0        0        0       11
GainEmpowerment                                     3        0        0        0        0        3
GainShield                                          6        0        0        0        0        6
GainForking                                         0        0        0        0        9        9
ReduceCooldown                                      0        0        9       14        8       31
pool spend (SpendMana / Spend / Consume)           13        4        9        5       24       55
- Consume                                           6        0        0        0        0        6
power_bonus                                         0        0        0        0       50       50
speed_bonus                                         0        0        0       16       20       36
mind_resist                                        28        0        0        0        0       28
harden (physical or magic)                          0        6        0        0        0        6
health above 15                                     4       49        3        4        1       61
crosses grids (Material or Plating)                23        0       20       18        0       61
### As a share of each slot
                                               helmet    chest   gloves  greaves   weapon
inert (no trigger, effect or adjacency)         24.7%    11.6%    15.7%    11.9%    31.8%
positional (effect, adjacency or reaction)      13.6%    21.7%    67.5%    28.4%    13.9%
- effect                                         6.2%    10.1%     8.4%     6.0%     3.5%
- adjacency bonus                                4.9%    11.6%     4.8%    20.9%     4.0%
.
## starter - the opening weapon and nothing else
rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     4.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol            loss     9.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss     5.90s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
40 The Rust Parliament    loss     7.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
## preset - the auto-builder's five-slot board
rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win    19.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
25 Cog Priest             loss    39.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
40 The Rust Parliament    loss    44.00s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
## owner - a finished run - 75 pieces, Berserker and Chronomancer
rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     1.50s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     2.80s     0.5%     0.0%    24.8%     0.3%    74.4%    0.0%
25 Cog Priest              win    12.00s     1.0%     1.7%    18.0%     3.4%    75.8%    0.0%
40 The Rust Parliament     win    22.50s     0.8%     0.8%    18.2%     3.4%    76.8%    0.0%
## friend - a finished run - 76 pieces, half of it deliberately loose
rung                    result       ttk   helmet    chest   gloves  greaves   weapon    burn
1 Cave Rat                 win     2.60s     0.0%     0.0%     0.0%     0.0%   100.0%    0.0%
10 Warded Idol             win     4.75s    13.4%     0.9%     9.8%     0.9%    75.0%    0.0%
25 Cog Priest              win     7.75s     0.8%     0.5%     0.9%     0.0%    97.9%    0.0%
40 The Rust Parliament     win    13.55s     0.3%     0.3%     1.1%     0.5%    97.9%    0.0%
## Weapon share across the whole ladder
build          cleared    weapon %  median ttk    burn %
starter           2/50      100.0%      45.00s      0.0%
preset            9/50      100.0%       9.00s      0.0%
owner            50/50       75.2%      10.50s      0.0%
friend           48/50       97.6%       7.75s      0.0%
## Board cadence - friendly activations a second
build              items activations/s    per item
starter                1          0.50       0.502
preset                 8          2.05       0.257
owner                 19          6.69       0.352
friend                17          3.21       0.189
## Mind damage across the whole ladder (max health removed, not in the shares above)
build          helmet    chest   gloves  greaves   weapon
starter             0        0        0        0        0
preset              0        0        0        0        0
owner              62        0        0       59        0
friend            595        0        0        0        0
.
## With the weapon grid emptied
(rung 15 is The Hollow King)
build       rungs won  best rung     rung 15      ttk         what carried it
starter          1/50         42      Defeat     2.8s                 nothing
preset           0/50       none      Defeat     5.6s                 nothing
owner           45/50         50      Defeat    47.0s the clock, not the gear
friend          35/50         46     Victory    44.6s the clock, not the gear
.
## preset - time-to-kill with one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol              19.50s        15%        23%        62%         0%      flips
25 Cog Priest                    -          -          -          -          -          -
40 The Rust Parliament           -          -          -          -          -          -
## owner - time-to-kill with one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               2.80s         7%         0%        61%         0%       168%
25 Cog Priest               12.00s        25%         0%      flips         0%       275%
40 The Rust Parliament      22.50s        27%         7%        96%         7%       100%
## friend - time-to-kill with one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol               4.75s         8%         8%        15%         8%       141%
25 Cog Priest                7.75s        66%         0%       199%         0%      flips
40 The Rust Parliament      13.55s        40%         0%       151%         0%       232%
.
## preset - health left at the end, one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol                 495         0%        64%         1%        15%        95%
25 Cog Priest                   28       196%       121%       -32%        25%        61%
40 The Rust Parliament          91        12%        60%         1%        10%         0%
## owner - health left at the end, one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol                2346        39%        37%         0%        20%         0%
25 Cog Priest                 2446        37%        39%        63%        19%        61%
40 The Rust Parliament        2546        35%        42%        70%        19%        63%
## friend - health left at the end, one grid emptied
rung                        intact     helmet      chest     gloves    greaves     weapon
10 Warded Idol                1755         0%        78%         0%        16%        -2%
25 Cog Priest                 1785        -2%        79%         0%        16%        83%
40 The Rust Parliament        1811         1%        79%        -4%        15%        83%
.
```

## The ratchet, in full

```
## Exclusivity - pieces out of place

mechanic                                         home     away   budget
power_bonus                                   50/50          0        0
the casting kinds (Ink/Spell/Alignment/Book/Orb)  93/93          0        0
GainForking                                    9/9           0        0
OnOtherCast                                   30/30          0        0
PerAdjacentEmpty                               9/9           0        0
searing                                       15/23          0        0
Consume                                        6/6           0        0
GainEmpowerment                                3/3           0        0
GainShield                                     6/6           0        0
MindDamage                                    11/11          0        0
mind_resist                                   28/28          0        0
Grow                                           7/7           0        0
reflect                                       20/20          0        0
harden                                         6/6           0        0
health above 15                               49/61          0        0
OnAdjacentActivate                            36/36          0        0
PerAdjacentItem                                2/2           0        0
Drain                                         11/11          0        0
StunStrongest                                  1/1           0        0
DoubleAdjacentItemStat                         2/2           0        0
OnAlignedActivate                             11/16          0        0
OnBattleStart                                  6/6           0        0
speed_bonus outside the weapon                16/36          0        0
ReduceCooldown outside the weapon             14/31          0        0
enchantment                                    1/5           0        0
frost, stun and misfire                       25/37          0        0

## Rarity of the catalogue, per slot

slot           common     rare     epic   legend    total
Helmet             79        0        2        0       81
Chest              68        0        0        1       69
Gloves             81        2        0        0       83
Greaves            66        0        0        1       67
Weapon            172        0        1        0      173

## Quotas  (filler is held at 30% for this rewrite, 15% after it)

slot        quota                                  of    share     wanted   away
Helmet      expresses its own axis                 81    77.8%   60-100%      0
Helmet      expresses its bleed axis               81    21.0%    20-25%      0
Helmet      plain flat-stat filler                 81    24.7%     0-30%      0
Helmet      the dearest third interacts            26    42.3%   35-100%      0
Chest       expresses its own axis                 69    98.6%   60-100%      0
Chest       expresses its bleed axis               69    24.6%    20-25%      0
Chest       plain flat-stat filler                 69    11.6%     0-30%      0
Chest       the dearest third interacts            22    40.9%   35-100%      0
Chest       pool-spend texture                     69     5.8%     0-15%      0
Gloves      expresses its own axis                 83    63.9%   60-100%      0
Gloves      expresses its bleed axis               83    21.7%    20-25%      0
Gloves      plain flat-stat filler                 83    15.7%     0-30%      0
Gloves      the dearest third interacts            27    63.0%   35-100%      0
Gloves      pool-spend texture                     83    10.8%     0-15%      0
Greaves     expresses its own axis                 67    76.1%   60-100%      0
Greaves     expresses its bleed axis               67    22.4%    20-25%      0
Greaves     plain flat-stat filler                 67    11.9%     0-30%      0
Greaves     the dearest third interacts            22    50.0%   35-100%      0
Greaves     pool-spend texture                     67     7.5%     0-15%      0
Weapon      the dearest third interacts            57    38.6%   35-100%      0
Weapon      pool-spend texture                    173    13.9%     0-15%      0

## Identity mechanics on floating kinds: 0
```

## What this capture says

**1. Criterion 1 holds with a point and a half to spare.** 75.2% against
66-76%. The rewrite's last recorded figure was 74.9%; the drift is the two
commits since, not an error in either reading.

**2. The owner's board clears the ladder and the friend's does not.** 50/50
against 48/50. That is a reversal from the capture before it and it is the
honest state: the friend's board is half deliberately loose, so it feels a
catalogue that has learned to reward adjacency less than a packed one does.

**3. Mind damage is small and it is about to matter.** Owner 62 on the helmet
and 59 on the greaves; friend 595 on the helmet. Insight and Dread multiply
exactly this channel, and it is currently a rounding error on two boards out of
four. Anything the third lane does will be visible against these figures rather
than lost in them.

**4. Cadence is 6.69 activations a second against the 2.0 `rating.rs`
assumes.** Unchanged in character from the rewrite's finding and still
unaddressed: anything that watches is under-priced on a good board by roughly
three times. It is a Phase-4 correction, listed at M16.

---

## Drift — M1, the lanes separate and the twins arrive

Two commits. The first moved the ladder; the second did not move it at all, and
that was the harder half.

### What A1 cost, and where

| build | cleared | weapon share | median ttk |
|---|---|---|---|
| starter | 2/50 | 100.0% | 45.00s |
| preset | 9/50 | 100.0% | 9.00s |
| **owner** | 50/50 → **48/50** | 75.2% → **75.5%** | 10.50s → 9.00s |
| friend | 48/50 | 97.6% → 97.4% | 7.75s → 8.15s |

**The shallow ladder is byte-identical.** Rungs 1 to 14, all four boards, every
figure unchanged - not within ten percent, unchanged - and the casino corridor
sits where it did at 1,600ms and 6,000ms. `report_early_ladder` is a new
printer and exists because the four sampled rungs could not answer that
question: two of them are past rung 14, and a change that left rung 1 and rung
10 alone while moving the eleven rungs between them would have read as
"unmoved".

**The whole cost is in the deep ladder**, and it is two rungs on one board. The
owner's board stops clearing Nine of Ashes and Francis. It was taking a
magic-lane multiplier onto physical swings - its helmet banks empowerment and
its weapon deals iron - and that is the exact arrangement A1 exists to end. The
friend's board slows at rungs 25 and 40 (7.75s → 10.30s, 13.55s → 15.45s) for
the same reason and clears the same 48.

**The mind lane moved most.** The friend's mind damage over the ladder goes
**595 → 707**, because the mana shield used to blunt mind damage as well and
now does not. That is decision #18 arriving: three lanes, three answers, and
`mind_resist` is the only thing standing in front of this one.

Not chased. The compensation for a physical board losing its multiplier is
Spellblade, which is a piece a player buys; the three shared boards are records
and cannot buy anything. Tuning the empowerment constant would hand more to
casters, which is not what came off here.

### What the twins cost: nothing, twice measured

Fourteen pieces carry the two new actions - Spellblade on five gloves and two
weapon accessories, Deflection on six chest layers and one greaves mold - and
**every fight in the harness is byte-identical to the commit before them.** The
only lines that move in the whole capture are census counts: weapon inert 55 →
53, gloves reaction triggers 49 → 52.

That took three attempts and each failure is worth keeping.

1. **Taking a blow away from a glove can leave a creature with no offence at
   all.** Six of the first carriers were reactions that dealt small flat
   physical damage, and converting them to a Spellblade grant is a translation
   that reads well and is wrong: four themes out of six hold no weapon, and a
   Spellblade stack multiplies a swing that is not there. `Cog Priest` stepped
   down into one of them on Easy and stopped being able to land anything -
   `every_monster_can_actually_hurt_you`, which sweeps all four settings for
   exactly this. The rule that came out of it: **arm what answers with armour,
   a pool or nothing; never what answers with a blow.**
2. **"Worn by no monster" is not "worn by nobody".** The second set was chosen
   against the monster boards alone, and two of them - `Padded Mold` and
   `Silver Charm` - are on the owner's board. Its fights got shorter, so it
   banked 89 nature by rung 22 instead of a hundred, and the Green Ledger's
   door stopped opening. Two tests in `towns.rs` said so.
3. **`apply_preset` is a board too.** The third set cleared the monsters and
   the three share codes and still moved the preset from 9/50 to 12/50, because
   it wears `Chain Layer` and `Ruby Inlay`. Swapped for `Blight Layer` and
   `Ratchet Cog`, which nothing wears at all.

So the carrier test is: **not in any monster's `gear`, not in any of the three
share codes, and not in `apply_preset`.** Four boards, not one.

### Where the twins live

```
GainSpellblade    gloves 5   weapon 2     (Mostly(70), home gloves)
GainDeflection    chest  6   greaves 1    (Mostly(70), home chest)
```

`catalog_shape` carries both rules and the ratchet is still green at zero rules
unmet. One amendment to the spec: A2 asks for Deflection's minority share on
greaves **plating**, and a Plating floats into the helmet's grid, so a floating
kind may carry no identity mechanic - `identity_carriers` holds that at zero.
It sits on a greaves **mold** instead, which is the feet's and nobody else's.

### Still open

`ClassPower::Transmute(50)` converts part of a physical swing into magic, and
after A1 that conversion happens *after* Spellblade and *before* nothing -
the transmuted half no longer picks up empowerment on the way across. That is
the honest reading (a conversion, not a second amplifier) and it is written
into the swing math as a comment, but the **Spellblade class** and the
Spellblade *stack* now sit either side of the same line without either knowing
about the other. A2 says re-wiring the class to grant stacks is optional. It is
still optional, and it is still open.

## Drift — M2, Insight and Dread

**None.** The whole harness is byte-identical to M1: every board, every rung,
every share, every census row. That is the milestone's exit criterion and it is
what "land primitives inert" is supposed to look like when it works.

`Resource` is eight. The eighth is **fuel, on mana's terms** rather than a
holding on rage's: `held_bonus` pays nothing for a point of Insight, exactly as
it pays nothing for a point of mana, because what both are worth is decided by
the stacks standing on them. A3 asks for "what mana empowerment is to magic",
and that is the half of the comparison that is easy to miss.

`Run::banked_all_run` was `[i32; 4]` against a `Resource::index()` that already
returned six. Nothing wrote past the end - a fusion emits `Event::Fused` rather
than `Event::GainResource` - so this was a fact about today's actions and not
about the array. It is eight now, and `insight.rs` writes to every index.

The gate is a field on the **shop** (`Shop::insight_open`) set by
`Run::unlock_insight` at the same moment as the run's own flag, because a flag
the shop has to be reminded of separately is a flag that will one day be set
without the reminder. `piece::touches_insight` is the predicate; it matches
nothing in the catalogue today, and `insight.rs` has a lint that says so and
asks to be deleted on the day the family lands.

The glossary was carrying a wrong sentence after M1 - "MANA SHIELD ... damage
of any kind" - and now says magic. SPELLBLADE, DEFLECTION, INSIGHT, DREAD and
THE THREE LANES are new entries beside it.

## Drift — M3, the road stack, receipts and tooltips

**None.** Byte-identical to M2 across the whole harness. Nothing in this
milestone is in `combat.rs`.

What it is worth recording instead is a test that was passing for the wrong
reason. `the_road::a_town_gate_blocks_the_road_even_mid_replay` asserts that a
gate still stops the next fight while a replay is up, and it checks
`road_is_blocked().is_some()`. Sump Bottom's gate stands at rung index 7 and so
does the first fountain, so "something is blocking the road" was answerable by
the wrong one of the two - and was, the first time `road_stack` read the
phase-gated `pending_town` there. It names what it is looking for now.

That is the third entry in `second-order.md` §4's list, and the first one found
by a change rather than by reading.

## Drift — M4, the road machinery

**None.** Byte-identical again. Every mechanic lands dark: no event names any
of them, so the road a player walks is the road they walked before.

One structural change worth writing down because it touches a format. **A board
can now be taller than the board beside it.** `branching-events.md` says a run
where one slot outgrows the others "would be a different game and a much more
confusing one", and that was right while the only thing handing out room was
Sprocketman's Gratitude, which hands out five. The Depth hands out **one**, on a
board of your choice, and the choice is the reward - so the rule is amended
rather than worked around.

Three things had to move with it, and all three are the same fault at different
depths:

- `Loadout::rows()` meant "every slot is the same height" and now means "the
  tallest", which is the right number for laying out a row of boards and the
  wrong one for asking whether a placement fits.
- `equip_locked_at` compared against it. That is the third time this exact
  question has been asked of the wrong thing: it was `SLOT_H` once, then the
  loadout's height, and now the slot's own.
- **The share code goes to version 3** and carries five row counts. One number
  was the whole answer only while rows arrived five at a time; a code that
  averaged an uneven board would put pieces in a row the sharer did not have,
  or drop the ones in the row they did. Version 2 codes still read, and read as
  what they are: boards where nothing had outgrown anything.

## Drift — M5 and M6

**None from either.** `bestiary.rs` moved a table between files; `route.rs`,
`pedestal.rs` and the dungeon's entry lines add reading rather than fighting.

Worth recording: `pack_francis::pack` - the `#[ignore]`d generator, not a test
- now **refuses Francis**. M1 took the reference board's magic multiplier off
its iron, the board no longer beats him at Medium, and the search reports "best
was a loss. Leaving it alone." That is the generator working: it refuses rather
than writing a board it cannot measure, and Francis keeps his hand-authored one
either way. The CURVE printer beside it agrees with M1's capture at 48 of 50
and a 9.00s median.

---

# Phase 1 closed — the engine, and the ladder where M1 left it

| | M0, before anything | Phase 1 closed |
|---|---|---|
| Suite | 548 green, 33 suites | **666 green**, 42 suites, 0 warnings |
| Owner at Medium | 50/50, 75.2%, median 10.50s | 48/50, **75.5%**, median 9.00s |
| Friend | 48/50, 97.6%, median 7.75s | 48/50, 97.4%, median 8.15s |
| preset / starter | 9/50 · 2/50 | 9/50 · 2/50 |
| Catalogue | 473 pieces, 104 inert | 473, 102 inert |
| `the_catalog_keeps_every_rule` | green | **green** |
| Casino corridor | sharp 1,600ms · plain 6,000ms | **unmoved** |

**The ladder has not moved since M1.** Every capture from M2 to M7 is
byte-identical to M1's - four boards, fifty rungs, every share, every census
row - which is the phase's own exit criterion and the whole of what "land
primitives inert, arm them separately" is for. M1 is the only milestone in this
mission licensed to move a fight, and it moved two: the owner's board stops
clearing Nine of Ashes and Francis, because it was taking a magic-lane
multiplier onto physical swings.

**A scripted CLI run replays identically.** Twenty-one lines of input covering
an event, a town, a fountain, a shop, four fights and two maps; two runs, 1,032
lines each, diffed to nothing. E6.1 in the form the road can currently take.

**The frame lint is green, and the plan said it would be red.** That was wrong
in the plan rather than in the code: `FRAMES` is empty until Phase 2 declares
one, and a lint over an empty list cannot fail. It goes red on the first frame
and green again at M17, which is what E6.8 asks for.

## What Phase 1 built

Eight milestones, seven of them with the fights standing perfectly still.

- **The three lanes** separate, with `mind_resist` as the mind lane's only
  answer, and the two physical twins carried by fourteen pieces nothing wears.
- **Insight**, the eighth resource, fuel rather than a holding, locked.
- **The road stack**, derived rather than stored, and the receipts and
  tooltips that let the engine own every sentence the road says.
- **Five conditions and eleven outcomes**, none of them named by any event.
- **A board that can be taller than the board beside it**, and a share code at
  version 3 that can describe one.
- **`bestiary.rs`**, four new themes, and a frame that is a creature before its
  board is.
- **`route.rs`**, a map that is a pure function of the tables, drawn twice from
  one function.
- **The reward vocabulary that is not gear** - relics that read the run,
  crushables that are spent, a rod that curses would rather land on.

## Drift — M9, the catalogue lands once

Thirty-one components, and **the four-board table at Medium does not move by a
figure**: owner 48/50 at 75.5%, friend 48/50 at 97.4%, preset 9/50, starter
2/50. The census is the only thing in the whole capture that changes, which is
what a catalogue landing correctly looks like.

| | before | after |
|---|---:|---:|
| Catalogue | 473 | **504** |
| Helmet | 81 | 96 |
| Weapon | 173 | 187 |
| Chest | 69 | 71 |
| `mind_resist` | 28 | 36 |
| inert | 102 (21.6%) | 120 (23.8%) |

Medium steps nothing, so Medium was never the question. **The question was the
other three settings, and the answer was twenty-nine boards.**

### The event-gear leak, found by walking into it

`stepped_component` filters boss gear and quest rewards out of a footprint
family before it sorts one, and both filters were added after something went
wrong: a trophy handed to the fourth creature on the ladder, and a quest reward
stepped into rather than earned. The list should always have been four entries
long.

Measured across every creature at Easy, Hard and Insane, the thirty-one new
components moved **29 of 162** stepped boards - and what they moved into was
`The Stranger's Parcel`, `The Cracked Lens`, `Doorward Frame` and
`Foreboding Crest`: three things the road hands over and one that banks a pool
the run has not been given yet.

With `is_event_only` and `touches_insight` added to the filter, that falls to
**11 of 162** - and every one of the eleven is the *old* leak being closed.
`Gold Chip` and `Crownwright's Measure` were already on monster boards at Easy
and Hard before this mission started; they are `Fury Sigil`, `Zealot's Crest`
and `Grudge Bead` now.

So M9's net effect on the ladder is: **nothing new reaches a creature, and a
quiet wrongness that predates the mission is closed on eleven boards.** The
thirty-one new components were what made it loud enough to see - a creature
being handed the astronomer's lens is harder to miss than a creature being
handed a casino chip.

### Two pieces the ratchet argued with, and won

- **The Cracked Lens** at `mind: 20` out-rated `The Split Wisdom`, which is
  boss gear and is supposed to be the best accessory a player can meet. Twelve,
  and the spec is amended: twenty points of mind on a one-cell piece is four
  times what any `MindDamage` action in the game pays.
- **Bearhide** wanted "Gain Fury on battle start", and both halves belong to
  somebody else - `OnBattleStart` is the feet's and banking rage on a chest is
  the helmet's axis wearing a coat, which put chest's bleed at 25.4% against a
  band that stops at 25. The fury is **strength**, which reaches every weapon
  and belongs to nobody, and what the piece *does* is armour.

`GainDread` counts as conversion now, beside `GainSpellblade` and for the same
reason: a stack that doubles a word counts as the word. That is what brought
helmet's bleed back into band after fifteen new helmet pieces landed.

## Drift — M10, the chain

**None.** Byte-identical to M9. The chain is data: four events, two towns, a
dungeon, three words and five frames, and not one of them is on the ladder.

The frame lint has gone **red**, which is the phase discipline working. It is
shipped as a ratchet rather than a failing test, the way `catalog_shape` is: a
green budget at today's count that can only go down, and an `#[ignore]`d target
that asserts zero. Five undressed creatures - the three floors of THE THRESHOLD
and the Herald's two.

One thing worth recording because it is a real bug rather than a design
choice. **`Run::take_choice` never checked that the choice belonged to the door
standing in front of you.** It did not have to: one door stood on one rung, and
the interface only ever offered that door's choices. The chain's windows are
wide enough for two doors to be open at once, and the first fixture holding all
five words answered a locked gate with the VIP area's rescue button. Two fixes,
because it was two faults: `take_choice` verifies ownership now, and
`Run::with_all_pieces` hands out every piece of *gear* rather than every entry
in the catalogue - a rumour is a key, and a fixture holding all of them opens
every rumour door in the game at once.

## Drift — M11 through M14, and the Phase-2 close

**None, from M9 to here.** Four content milestones — the dungeons, Extra Large
and the orbs, the five unconditional events, and the nine structures with Part
D's three pairs — and the ladder is byte-identical to the capture M9 left. That
is the phase working rather than luck: Phase 2's diffs are events, towns,
dungeons, words and classes, and not one of them is a creature, a weight or a
board.

The frame budget is unchanged at **14**. The frame lint is still red, which
E6.8 requires it to be: it goes green in M17, by hand, in `make pack`, and only
after M16 has re-pinned the rating that decides what `stepped_component` hands
every monster on three of the four settings.

### What the structures cost, measured

| | |
|---|---|
| Events in the table | 33 (nine structures, three pairs, and the twenty-one that were there) |
| Classes | 31 — Unionized and Showstopper appended, never inserted |
| Rumours | 8, of which 5 are on the bar and the bar draws exactly `SHOP_SIZE` |
| New engine surface | `every_outcome`, `Choice: PartialEq`, `Requirement::{HoldingRumour, Classes}`, `Outcome::{SealedBid, ShopAfter, Markup, Passenger, Contract, SellWord, SellTitle, Chill}` |
| Combat | one constant, `CONTRACT_SLOWER = 50`, applied where every other speed is |

### Two lints that were reading half of what they thought

Worth recording in this file rather than only in the ledger, because both are
measurement faults rather than design faults, and this file is where the
measurements live.

**A pointer test is not a portable test.** `take_choice`'s ownership check
compared choices with `std::ptr::eq`. `EVENTS` is a static holding promoted
arrays, and a caller in another crate can hold a reference to a copy — so the
check passed in the engine, passed in the GUI, and refused every choice made
from a test binary. The failure mode is the bad one: not a wrong answer, a
silent no. It compares by value now.

**A composite outcome hid everything inside it.** `class::is_earned`,
`event::set_by` and the event reachability lint all matched on `c.outcome`
directly, and half this mission's bargains are an `Outcome::All`. A class
claimed inside one read as a class no door hands out — which is exactly the
condition `every_class_but_the_floor_asks_for_something` exists to catch, and
it caught it. `event::every_outcome` unpacks `All` and `Gamble`, and the three
callers go through it.

### Phase 2, closed

`tests/phase_two.rs` is E6.8 said as six assertions: every door can be arrived
at, every hidden town and every dungeon has a mouth, every reward the mission
promises is in somebody's gift, both routes to the Mainspring are walkable,
every creature the mission added is still a frame, and a run that answers
everything meets every door that stands on a rung. The last one is a sweep
rather than a replay — it walks all fifty rungs holding every word, with the
flags set and a packed board, and reports what it never met. It reports
nothing.

### One field that was doing nothing

`Run::cursed_for_good` has been documented since M12 as "pieces carrying a
curse for the rest of the run" and **nothing read it**. The Manse library set
it, `Outcome::Uncurse` popped it, and no fight was any different for either.
The thirsty wizard's refusal would have inherited the same hole, so it is
closed here: `CURSED_SLOWER = 25` on any item holding a cursed piece, applied
in `combat_items` beside the contract's own frost, which is where every speed
in this game is applied.

It chills something that **acts**, not something that merely sits there —
drawn from `combat_items`, because a loose component has no cooldown to slow
and freezing one would be a receipt line and nothing else. Nothing pinned
moves: neither the library nor the wizard is on the ladder.
