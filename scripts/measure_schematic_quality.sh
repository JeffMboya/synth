#!/usr/bin/env bash
# Measure schematic layout quality across a broad corpus, using the
# CLI's `layout --score` output. Prints one TSV row per design that
# lowers successfully, so two runs can be diffed numerically.
#
#   scripts/measure_schematic_quality.sh > after.tsv
#   git stash && cargo build --release -p synth-cli
#   scripts/measure_schematic_quality.sh > before.tsv
#   diff -u before.tsv after.tsv
set -uo pipefail
cd "$(dirname "$0")/.."
BIN=./target/release/synth
printf "%s\t%s\t%s\t%s\n" design crossings wire_len_mm label_stubs
for f in fixtures/designs/*.synth examples/*.synth fixtures/kicad-reference/*.synth; do
  [ -e "$f" ] || continue
  out=$("$BIN" layout "$f" --score 2>/dev/null) || continue
  row=$(printf '%s' "$out" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(1)
s = d.get("score")
if not s:
    sys.exit(1)
print("%d\t%.2f\t%d" % (s["crossing_count"], s["total_wire_length_mm"], s["label_stub_count"]))
') || continue
  printf "%s\t%s\n" "$(basename "$f" .synth)" "$row"
done
