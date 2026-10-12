#!/usr/bin/env bash
# Plot a schematic and a board image for every qualification PCB, so a review
# can look at what Synth produced instead of reading a hash.
#
# Needs `kicad-cli` (KiCad 10) for the plots and `rsvg-convert` for the PNGs.
# Copper needs a router: set `KICAD_ROUTING_TOOLS_REPO`. A board that does not
# route is still plotted, from the un-routed baseline Synth preserves beside
# the export, and the image is named `pcb-unrouted` so nobody mistakes it for
# a finished board.
#
# Usage: scripts/render_boards.sh [board.synth ...]
# With no arguments every positive qualification board is plotted.
set -uo pipefail

SYNTH=${SYNTH:-target/release/synth}
KICAD_CLI=${KICAD_CLI:-kicad-cli}
OUT=${OUT:-target/renders}
WIDTH=${WIDTH:-2400}
HEIGHT=${HEIGHT:-1600}

boards=("$@")
if [ ${#boards[@]} -eq 0 ]; then
  mapfile -t boards < <(find qualification -name '*.synth' ! -name 'negative_*' | sort)
fi

mkdir -p "$OUT"
failed=()

# Record the plotter's own surface: a flag this script guesses wrong is the
# first thing to check when an image is missing.
{
  "$KICAD_CLI" version
  "$KICAD_CLI" pcb export svg --help
  "$KICAD_CLI" pcb render --help
  "$KICAD_CLI" sch export svg --help
} > "$OUT/kicad-cli.txt" 2>&1

svg_to_png() { # svg png
  rsvg-convert --width "$WIDTH" --keep-aspect-ratio --background-color white \
    --output "$2" "$1"
}

for board in "${boards[@]}"; do
  id=$(basename "$board" .synth)
  work="$OUT/$id"
  log="$work/export.log"
  rm -rf "$work"
  mkdir -p "$work"

  args=(export-kicad "$board" --out "$work/project" --allow-incomplete)
  if [ -n "${KICAD_ROUTING_TOOLS_REPO:-}" ]; then
    args+=(--router kicad-routing-tools
           --kicad-routing-tools-repo "$KICAD_ROUTING_TOOLS_REPO")
  fi
  echo "== $id"
  "$SYNTH" "${args[@]}" > "$log" 2>&1 \
    || echo "   export returned non-zero; see $log"

  sch="$work/project/$id.kicad_sch"
  pcb="$work/project/$id.kicad_pcb"
  pcb_label=pcb
  if [ ! -f "$pcb" ]; then
    pcb="$work/project/$id.synth.kicad_pcb"
    pcb_label=pcb-unrouted
  fi

  if [ -f "$sch" ]; then
    if "$KICAD_CLI" sch export svg --output "$work/sheets" "$sch" >> "$log" 2>&1; then
      n=0
      for svg in "$work"/sheets/*.svg; do
        [ -e "$svg" ] || continue
        n=$((n + 1))
        if [ "$n" -eq 1 ]; then name=schematic; else name="schematic-$n"; fi
        svg_to_png "$svg" "$work/$id.$name.png" >> "$log" 2>&1
      done
      rm -rf "$work/sheets"
    else
      failed+=("$id schematic")
    fi
  else
    failed+=("$id schematic (not exported)")
  fi

  if [ -f "$pcb" ]; then
    # One combined sheet, cropped to the board, with the layers a reviewer
    # reads: copper both sides, silkscreen, the outline and the drills.
    if "$KICAD_CLI" pcb export svg --output "$work/$id.$pcb_label.svg" \
        --layers "F.Cu,B.Cu,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts" \
        --page-size-mode 2 --exclude-drawing-sheet "$pcb" >> "$log" 2>&1; then
      svg_to_png "$work/$id.$pcb_label.svg" "$work/$id.$pcb_label.png" >> "$log" 2>&1
    else
      failed+=("$id $pcb_label")
    fi

    # The 3D view is the one picture a non-engineer reads. Component bodies
    # need `kicad-packages3d`; without it this is still the real board.
    render=("$KICAD_CLI" pcb render --output "$work/$id.$pcb_label-3d.png"
            --quality basic --side top --width "$WIDTH" --height "$HEIGHT" "$pcb")
    if command -v xvfb-run > /dev/null 2>&1; then
      xvfb-run -a "${render[@]}" >> "$log" 2>&1 || failed+=("$id $pcb_label-3d")
    else
      "${render[@]}" >> "$log" 2>&1 || failed+=("$id $pcb_label-3d")
    fi
  else
    failed+=("$id pcb (not exported)")
  fi

  ls "$work"/*.png 2>/dev/null | sed 's/^/   /'
done

if [ ${#failed[@]} -gt 0 ]; then
  printf 'not plotted: %s\n' "${failed[@]}"
  exit 1
fi
