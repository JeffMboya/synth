#!/usr/bin/env python3
"""Run FreeRouting on a Synth/KiCad board and import its SES result.

The board arrives un-routed, because routing is the only path: clean-netlist
mode therefore always applies, so the SES geometry is produced against the
same board state on both sides of the external router.

The `--report` this writes is the router's own account of what it did. It is
recorded, and the independent checks downstream compare it against the copper
that was actually written, rather than taking it as the answer.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import pcbnew


def _top_level_netclass_blocks(text: str) -> list[str]:
    """Extract KiCad's top-level net_class forms without a full S-expression parser."""
    blocks = []
    start = 0
    while True:
        start = text.find("\n\t(net_class", start)
        if start < 0:
            return blocks
        depth = 0
        in_string = False
        escaped = False
        end = len(text)
        for index in range(start + 1, len(text)):
            char = text[index]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
            elif char == '"':
                in_string = True
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    end = index + 1
                    break
        blocks.append(text[start:end])
        start = end


def _restore_netclasses(source_path: str, output_path: str) -> None:
    source_text = Path(source_path).read_text()
    output_file = Path(output_path)
    output_text = output_file.read_text()
    if "\n\t(net_class" in output_text:
        return
    blocks = _top_level_netclass_blocks(source_text)
    if not blocks:
        return
    insertion = output_text.find("\n\t(gr_")
    if insertion < 0:
        insertion = output_text.find("\n\t(footprint")
    if insertion < 0:
        raise RuntimeError("could not find a safe insertion point for KiCad netclasses")
    output_file.write_text(
        output_text[:insertion] + "\n".join(blocks) + output_text[insertion:]
    )


def all_tracks(board) -> list:
    """Every track and via on the board, as owned Python objects.

    `GetTracks()` is the documented accessor but it is not usable on every
    KiCad build: its SWIG iterator calls `it.next()`, a name Python 3 removed,
    so on KiCad 10 it raises `AttributeError` before yielding anything.
    Indexing `Tracks()` directly avoids the iterator entirely and works on
    both, so it is tried first and the accessor is only a fallback.
    """
    container = board.Tracks()
    tracks = []
    index = 0
    while True:
        try:
            tracks.append(container[index])
        except IndexError:
            return tracks
        except (TypeError, AttributeError):
            break
        index += 1
    try:
        return list(board.GetTracks())
    except (AttributeError, TypeError):
        return tracks


def java_version(java: str) -> str | None:
    """First line of the interpreter's version banner, if it answers."""
    try:
        out = subprocess.run(
            [java, "-version"], capture_output=True, text=True, timeout=30
        )
    except (OSError, subprocess.SubprocessError):
        return None
    banner = (out.stderr or out.stdout).strip()
    return banner.splitlines()[0] if banner else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_board")
    parser.add_argument("output_board")
    parser.add_argument("--jar", required=True)
    parser.add_argument("--java", default="java")
    # Dense RP2350 boards often improve through the mid-30s before the
    # router's stagnation guard stops.  Keep the default bounded while
    # allowing explicit overrides for more difficult boards.
    parser.add_argument("--passes", type=int, default=40)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument(
        "--ses-output",
        help="copy the FreeRouting SES result to this path before KiCad import",
    )
    # Accepted and ignored: every route is now a clean-netlist route, so the
    # flag no longer selects anything. The adapter still passes it.
    parser.add_argument(
        "--clean-netlist",
        action="store_true",
        help="accepted for compatibility; always applied",
    )
    parser.add_argument(
        "--no-import",
        action="store_true",
        help="stop after routing and SES export",
    )
    parser.add_argument(
        "--report",
        help="write a JSON record of what the engine reported here",
    )
    parser.add_argument(
        "--no-retain-session",
        dest="retain_session",
        action="store_false",
        default=True,
        help="delete FreeRouting's scratch data directory when finished",
    )
    args = parser.parse_args()

    def write_report(**fields: object) -> None:
        """Record the engine's own account of the run.

        Written even on the failure paths below, because "the router ran and
        then failed" and "the router never ran" are different diagnoses and
        the run record has to tell them apart.
        """
        if not args.report:
            return
        Path(args.report).write_text(json.dumps(fields, indent=2, sort_keys=True))

    with tempfile.TemporaryDirectory(prefix="synth-freerouting-") as work:
        dsn = os.path.join(work, "board.dsn")
        ses = os.path.join(work, "board.ses")
        data = os.path.join(work, "freerouting-data")
        os.makedirs(data)

        # The board arrives un-routed, but strip any copper anyway: a stale
        # track would be routed around by the engine and then merged over,
        # and the independent check would be looking at geometry that was
        # never validated as a route.
        #
        # `Tracks()` exposes a SWIG vector whose indexed values can be
        # borrowed wrappers on newer KiCad builds.  Remove owned Python
        # objects from `GetTracks()` instead; this works with KiCad 9/10
        # and avoids the `SwigPyObject.thisown` failure.
        export_board = pcbnew.LoadBoard(args.input_board)
        ground_net_codes = {zone.GetNetCode() for zone in export_board.Zones()}
        for track in all_tracks(export_board):
            # Ground stitching vias are part of the board's plane topology,
            # not FreeRouting's candidate geometry. Keep them in the clean
            # netlist so the planes remain electrically joined after import.
            if track.GetClass() == "PCB_VIA" and track.GetNetCode() in ground_net_codes:
                continue
            export_board.Remove(track)
        print("routing a clean netlist", flush=True)
        # KiCad's SES importer can discard named netclasses. Keep the source
        # settings alive and restore them after import so the routed board is
        # checked with the same Power/RF constraints as the exported board.
        if not pcbnew.ExportSpecctraDSN(export_board, dsn):
            write_report(router="freerouting", error="dsn_export_failed")
            raise SystemExit("KiCad Specctra DSN export failed")

        completed = subprocess.run(
            [
                args.java,
                "-Djava.awt.headless=true",
                "-jar",
                args.jar,
                "-de",
                dsn,
                "-do",
                ses,
                "-mp",
                str(args.passes),
                "-mt",
                str(args.threads),
                "--user_data_path=" + data,
            ],
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            write_report(
                router="freerouting",
                jar=str(args.jar),
                java_version=completed.stderr.strip()[:200] or None,
                exit_code=completed.returncode,
                notes=["freerouting exited non-zero"],
            )
            sys.stderr.write(completed.stdout)
            sys.stderr.write(completed.stderr)
            raise SystemExit(completed.returncode)

        if args.ses_output:
            shutil.copyfile(ses, args.ses_output)
            print(f"saved SES result to {args.ses_output}", flush=True)

        # The engine's own account goes to stdout so it lands in the run's
        # session log. A partial route is the case where it matters most: the
        # SES shows what was produced, and this says why the rest was not.
        sys.stdout.write(completed.stdout)
        sys.stderr.write(completed.stderr)

        if not args.retain_session:
            shutil.rmtree(data, ignore_errors=True)

        if args.no_import:
            write_report(
                router="freerouting",
                jar=str(args.jar),
                java_version=java_version(args.java),
                passes=args.passes,
                threads=args.threads,
                ses_path=ses,
                imported_segments=None,
                imported_vias=None,
                notes=["ses exported without import"],
            )
            return 0

        # KiCad 10's Python ImportSpecctraSES can segfault on otherwise valid
        # dense SES geometry. Merge the external router's records textually
        # instead; KiCad CLI performs the authoritative refill and DRC after
        # this step. The merger removes all existing top-level segment/via
        # records before adding the SES records, so the original source file
        # remains a suitable merge base even when the DSN was exported from a
        # copper-free duplicate.
        merge_input = args.input_board
        merger = os.path.join(
            os.path.dirname(__file__), "import_freerouting_ses_text.py"
        )
        subprocess.run(
            [sys.executable, merger, merge_input, ses, args.output_board],
            check=True,
        )

        # Counted from the merged board rather than from the engine's log, so
        # the claim can be checked against the file that was written.
        written = pcbnew.LoadBoard(args.output_board)
        tracks = all_tracks(written)
        # KiCad names a routed track `PCB_TRACK`; the constant is matched by
        # suffix so a build that spells it differently still counts, and an
        # unmatched name is reported rather than quietly folded into a zero.
        segments = sum(1 for t in tracks if t.GetClass().endswith("TRACK"))
        vias = sum(1 for t in tracks if t.GetClass().endswith("VIA"))
        write_report(
            router="freerouting",
            jar=str(args.jar),
            java_version=java_version(args.java),
            passes=args.passes,
            threads=args.threads,
            ses_path=ses,
            imported_segments=segments,
            imported_vias=vias,
            notes=[],
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
