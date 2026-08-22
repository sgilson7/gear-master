#!/usr/bin/env bash
# Which catalogue entries have no Turtle Dick name yet?
#
# The theme falls through to the canonical name when a key is missing, so an
# untranslated piece is invisible in play - it just shows up in plain English in
# a shop full of Fnorp and Sneel. This makes "am I done" a question with an
# answer. Run it before and after touching CATALOG.
#
# Usage: .claude/skills/gearmaster-gear/scripts/check-parity.sh
# Exits 1 if anything is unthemed beyond the nine deliberate exemptions.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
piece="$root/crates/engine/src/piece.rs"
theme="$root/crates/engine/src/theme.rs"

for f in "$piece" "$theme"; do
    [ -f "$f" ] || { echo "not where I expected: $f" >&2; exit 2; }
done

# Kept in plain words on purpose. Mirrors KEPT in
# the_turtle_theme_covers_the_catalogue; if you change one, change both.
kept='Ratchet Cog|Flywheel Cog|Anvil Frame|Hollow Weave|Witch.s Hat|Green Crown|Wandering Root|Worldeye Orb|The Money Jacket'

# Catalogue names: the `name:` field of every PieceDef.
awk -F'"' '/^ *name: "/ { print $2 }' "$piece" | sort -u > /tmp/gm-catalog.$$

# Themed keys: the left side of every ("canonical", "themed") pair in the
# pieces table, which runs from `pieces: &[` to its closing `],`.
awk '/^    pieces: &\[/ { inside = 1; next }
     inside && /^    \],/  { inside = 0 }
     inside' "$theme" \
    | awk -F'"' '/^ *\("/ { print $2 }' | sort -u > /tmp/gm-themed.$$

missing=$(comm -23 /tmp/gm-catalog.$$ /tmp/gm-themed.$$ | grep -Ev "^($kept)$" || true)
stale=$(comm -13 /tmp/gm-catalog.$$ /tmp/gm-themed.$$ || true)

catalog_n=$(wc -l < /tmp/gm-catalog.$$ | tr -d ' ')
themed_n=$(wc -l < /tmp/gm-themed.$$ | tr -d ' ')
rm -f /tmp/gm-catalog.$$ /tmp/gm-themed.$$

echo "$catalog_n components, $themed_n themed."

status=0

if [ -n "$stale" ]; then
    echo
    echo "Themed names pointing at components that no longer exist:"
    echo "$stale" | sed 's/^/  /'
    echo "  (a rename that only touched piece.rs - fix the key in theme.rs)"
    status=1
fi

if [ -n "$missing" ]; then
    echo
    echo "No Turtle Dick name yet:"
    echo "$missing" | sed 's/^/  /'
    echo
    echo "Read reference/turtle-dick.md, then add each to TURTLE_DICK.pieces."
    status=1
fi

[ $status -eq 0 ] && echo "In parity."
exit $status
