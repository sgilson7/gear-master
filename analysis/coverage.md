# Coverage — aimed at, rather than met by accident

`cargo run --release -p gearmaster-lab --bin solve`. 40 runs: 20 seeds in each of two modes, coverage at maximum, one shared memory across all of them.

| | reached | of | |
|---|---:|---:|---|
| doors | **42** | 53 | 79% |
| **dungeons** | **5** | 7 | 71% |
| towns | **5** | 6 | 83% |
| branches | **82** | 120 | 68% |

## The dungeons, one at a time

| dungeon | floors | stood on |
|---|---:|---:|
| `the-crevice` | 3 | 1 |
| `the-threshold` | 4 | 3 |
| `the-under-mine` | 2 | 0 |
| `the-undertow` | 2 | 0 |
| `den-rivals` | 2 | 2 |
| `wumpus-world` | 2 | 2 |
| `the-switchyard` | 9 | 8 |

## Not reached

  - `the-constable` (door)
  - `the-county-surveyed` (door)
  - `the-crownwright` (door)
  - `the-green-ledger` (door)
  - `the-glow-over-the-ridge` (door)
  - `the-sealed-bid` (door)
  - `the-fork` (door)
  - `the-foundry-remembers` (door)
  - `the-unwound` (door)
  - `the-boundary-ditch` (door)
  - `the-charcoal-burner` (door)
  - `the-under-mine` (dungeon)
  - `the-undertow` (dungeon)
  - `THE SLAGWORKS` (town)

## What the memory learned

2 door-choices are known to lead into a dungeon:

  - `the-shrine-fork` choice 1 -> `the-crevice`
  - `the-turntable` choice 0 -> `the-switchyard`

4 choice labels were asked for by a shut door somewhere:

  - "Ask how he does it"
  - "Plug your ears"
  - "Sign it"
  - "TAKE THE DEAL"

wrote analysis/coverage.md
