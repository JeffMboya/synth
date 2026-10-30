#!/usr/bin/env python3
"""Run: python3 tools/test_kicad_routing_tools_route.py

The adapter routes at a high via cost first and again at the engine's default
when the first attempt leaves anything open. A controlled stub stands in for
KiCadRoutingTools: it logs each invocation and answers with the next summary
from a script.
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ADAPTER = Path(__file__).parent / "kicad_routing_tools_route.py"

STUB = """\\
import json, sys
from pathlib import Path

here = Path(__file__).parent
calls = here / "calls.log"
count = len(calls.read_text().splitlines()) if calls.exists() else 0
with calls.open("a") as log:
    log.write(" ".join(sys.argv[1:]) + "\\n")
summaries = json.loads((here / "summaries.json").read_text())
summary = summaries[min(count, len(summaries) - 1)]
Path(sys.argv[sys.argv.index("--json-out") + 1]).write_text(json.dumps(summary))
Path(sys.argv[2]).write_text("(kicad_pcb)")
"""


def run_adapter(summaries):
    """Run the adapter against the stub; return (exit code, via costs requested)."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp) / "krt"
        (repo / "py_router").mkdir(parents=True)
        (repo / "py_router" / "route.py").write_text(STUB)
        (repo / "py_router" / "summaries.json").write_text(json.dumps(summaries))
        board = Path(tmp) / "in.kicad_pcb"
        board.write_text("(kicad_pcb)")
        done = subprocess.run(
            [
                sys.executable,
                str(ADAPTER),
                str(board),
                str(Path(tmp) / "out.kicad_pcb"),
                "--repo",
                str(repo),
                "--python",
                sys.executable,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        calls = (repo / "py_router" / "calls.log").read_text().splitlines()
        costs = [
            line.split("--via-cost ")[1].split()[0] if "--via-cost" in line else "default"
            for line in calls
        ]
        return done.returncode, costs


CLEAN = {"successful": 2, "failed": 0}


class RetryTest(unittest.TestCase):
    def test_a_clean_high_via_cost_attempt_is_not_retried(self):
        code, costs = run_adapter([CLEAN])
        self.assertEqual(costs, ["1000"])
        self.assertEqual(code, 0)

    def test_a_listed_open_group_retries_at_the_default_cost(self):
        code, costs = run_adapter(
            [{"successful": 1, "failed": 1, "failed_single": ["net_3"]}, CLEAN]
        )
        self.assertEqual(costs, ["1000", "default"])
        self.assertEqual(code, 0)

    def test_a_count_only_failure_retries_at_the_default_cost(self):
        code, costs = run_adapter([{"successful": 0, "failed": 1}, CLEAN])
        self.assertEqual(costs, ["1000", "default"])
        self.assertEqual(code, 0)

    def test_a_count_only_failure_that_persists_is_rejected(self):
        code, costs = run_adapter([{"successful": 0, "failed": 1}])
        self.assertEqual(costs, ["1000", "default"])
        self.assertNotEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
