# Reference board qualification

## What the matrix is

`qualification/matrix.toml` is a checked-in corpus of boards, each declared
with a capability class (A–D), a status, and the result the deterministic
release command must produce. CI runs the same command a human would —
`synth check <board> --json` — and fails when any board stops matching its
declaration.

| Status | Meaning |
| --- | --- |
| `qualified` | The declared envelope is proven for this board by automation and a recorded human review. |
| `experimental` | The release command passes, but no human has signed off on the artifacts. Not a launch claim. |
| `unsupported` | A negative test. The board is expected to fail, and the diagnostics it must report are named. |

The gate fails in **both** directions. A board that was passing and starts
failing is a regression. A board declared `unsupported` that starts passing is
a launch claim that grew without anyone declaring it, which is the failure this
matrix exists to catch. Either way the fix is to understand the change first
and edit `matrix.toml` second.

## Running it

```bash
# The release-blocking gate. Use a release build: a debug DRC on a real board
# takes about ten times as long and overruns the stage budget.
cargo test --release -p synth-cli --test qualification_matrix -- --ignored --nocapture

# Also exercise the manufacturing stage. Needs KiCad installed.
SYNTH_QUALIFY_FAB=1 cargo test --release -p synth-cli \
  --test qualification_matrix -- --ignored --nocapture
```

Results land in `target/qualification/results.json`, and CI uploads them as an
artifact on every run, passing or failing. The file records the KiCad version,
a SHA-256 of the registry manifest, every stage status per board, the
error-level diagnostics each board reported, and hashes for the deterministic
artifacts — so a result that changes is attributable to a compiler change, a
registry edit, or a different KiCad, rather than being a mystery.

Stage budgets are configurable when a board needs more time:
`SYNTH_CHECK_SOURCE_TIMEOUT_SECS`, `SYNTH_CHECK_DRC_TIMEOUT_SECS`,
`SYNTH_CHECK_FAB_TIMEOUT_SECS`.

### Why Gerber hashes are not pinned

`kicad-cli` stamps a per-run timestamp into Gerber headers, so those files are
not byte-reproducible (see the determinism note in `crates/synth-kicad/src/fab.rs`).
The matrix records every artifact hash `synth check` reports but compares only
the `.kicad_sch`, `.kicad_pcb` and `bom.csv` hashes, which are IR-driven and
stable. Pinning the rest would make the gate fail at random, which trains
people to ignore it.

## What automation cannot prove

A board that passes this matrix has passed the compiler, the rule checks and
the exported-artifact checks. That is **not** the same as being fabricable.
The following require a human with PCB experience looking at the artifacts,
and no amount of green CI substitutes for them:

| Area | What a reviewer has to judge |
| --- | --- |
| Visual / schematic | Readability, sheet organisation, net naming, reference/value consistency, whether the drawing communicates the design to the next engineer. |
| Assembly | Courtyard clearances, pick-and-place access, polarity and orientation markings, silkscreen legibility after fabrication, reflow shadowing, panelisation. |
| Signal integrity | Impedance against the real stackup, pair spacing and length matching, skew, return-path continuity, via stubs, neck-downs, plane splits under signals. |
| Power integrity | Rail droop, decoupling placement and effectiveness, plane resonance, inrush and sequencing, star-point and ground-return strategy. |
| Thermal | Copper area and via count for dissipating parts, exposed-pad attachment, derating at the intended ambient, airflow assumptions. |
| Fabrication | Fab-house capability against the chosen profile, drill-to-copper, annular ring, solder-mask slivers, impedance-control feasibility, first-pass yield. |

## The handoff

Promoting a board from `experimental` to `qualified` takes a review, not a
code change:

1. Run the matrix with `SYNTH_QUALIFY_FAB=1` and attach
   `target/qualification/results.json` to the review.
2. Open the exported project in KiCad and run its own ERC and DRC.
3. Have a reviewer with PCB experience work the table above, recording a
   verdict per area rather than one overall opinion.
4. File every defect found as its own issue, linked to the board id and the
   exact artifact, and classified as launch blocker, must-fix, should-fix or
   follow-up.
5. Only once no launch blocker remains, change that board's `status` to
   `qualified` in `matrix.toml`, in a commit whose message names the reviewer
   and links the review.

A board whose review found blockers stays `experimental`. A board whose review
found the envelope is not supported at all becomes `unsupported`, with the
diagnostics that prove it named in `expect_source_codes`.

## Capability classes

The classes match the launch-qualification issues, so a matrix row and a
qualification exercise refer to the same envelope:

- **Class A** — low-speed embedded: MCU, power, sensors, connectors.
- **Class B** — mixed-signal and RF, and USB/Ethernet-class interfaces.
- **Class C** — dense or high-speed: differential pairs, length matching, constrained placement.
- **Class D** — complex systems where the obligation is an auditable result and a stated limitation, not a claim of full support.

## A note on branch protection

The workflow is path-filtered, so it does not run on pull requests that touch
only documentation. GitHub reports a filtered-out workflow as missing rather
than as passing, so if `Qualification matrix` is made a required status check,
configure it in a way that tolerates the filter — otherwise documentation-only
pull requests will wait forever for a check that was never going to run.
