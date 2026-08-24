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
