# Irrigation controller

A two-zone irrigation controller: 12 V in through a fuse, a P-channel reverse
blocker and a TVS, an MP2307 buck to 3.3 V, an STM32G030, two low-side solenoid
drivers, two switched resistive soil probes, a flow-pulse input, an I2C bus with
a temperature and humidity sensor, an ambient light sensor and an EEPROM, an
SWD header, and rail decoupling, bleeders, indicator LEDs and test points added
to fill the board. Two layers, JLCPCB, 75.1 x 65.4 mm, 112 components.

**Status: an engineer-review draft, not a release.** The board is routed and
KiCad reports no error on it, but it has no Gerbers, its BOM is not orderable,
and the open items below are real. Nothing here was built or tested on hardware.

## How this board was made

An agent produced the design, and Synth's own tools and scripts then packed and
tidied it. Every step is logged, and the board was checked on the exported files
after each one.

| Step | What ran | Result |
|---|---|---|
| 1. The design | An agent from one prompt ([`agent-run/prompt.md`](agent-run/prompt.md)), driving `synth mcp` with Read, Write and Edit only, from an empty workspace. 73 tool calls; the session limit ran out once and it was resumed three times ([`resume-messages.md`](agent-run/resume-messages.md), [`tool-calls.md`](agent-run/tool-calls.md), [`report.md`](agent-run/report.md)). | 86 components; the first version of this PR compacted it by scale to 83.9 x 67.0 mm |
| 2. Compaction | `synth_compact_layout` in `gap` mode, 2.5 mm between courtyards and 6 mm next to a part with many pins | 84.1 x 67.4 mm as pinned, then 75.1 x 65.4 mm, both released |
| 3. Fill | Two agent runs from the compacted board ([`fill-pass-prompt.md`](agent-run/fill-pass-prompt.md), reports [1](agent-run/fill-pass-report-1.md) and [2](agent-run/fill-pass-report-2.md)): add parts in the voids, each wired to a real function, without growing the outline | 26 parts added, outline repaired to 75.1 x 65.4 mm |
| 4. Placement advice | A script ([`tools/apply_advice_par.py`](agent-run/tools/apply_advice_par.py)) applies the advice of Synth's placement review in small steps and keeps a step only if the export releases, the outline stays at or below 75.1 x 65.4 mm and every flush connector stays within 0.35 mm | two passes, 15 and 18 moves kept |
| 5. By hand on the sidecar | One resistor beside its rail ([`tools/move_one.py`](agent-run/tools/move_one.py)); at review the top-right mounting hole was asked to sit in the corner like the other three, so it moved to (72.0, 7.0) and `J_I2C`, `J_SWD` and their two test points moved 6.0 mm down ([`tools/corner_par.py`](agent-run/tools/corner_par.py)) | released at the same outline |

The `.synth` source and the placement sidecar beside this file are the result.
The eight registry parts the agent imported or authored are in `registry/parts/`.
The board needs Synth built from pull request #370 and the ones it stacks on
(#310, #345, #348, #352, #368), the parts of #326, and #266 for `polyfuse_1r5a`,
`smaj15a_tvs` and `l_1210_10uh`, which are not in the registry on `main`.

The server was KiCad 10.0.7 on a 16-vCPU machine. `allow_incomplete` was never
set, in any run log or script.

## What was measured

All figures come from the files of the last export, read back from disk.

| | |
|---|---|
| Board | 75.1 x 65.4 mm, 4,912 mm2, two layers, 112 components, 43 nets |
| Routing | state `routed`, 43 of 43 nets connected, 549 segments, 73 router vias |
| Vias in the file | 129, all 0.6 / 0.3 mm; the rest are ground vias Synth adds beside layer changes |
| KiCad DRC, 10.0.7, `--refill-zones --severity-all` | 0 errors, 0 unconnected, 113 warnings: 112 `lib_footprint_issues` (the server's KiCad has no footprint library table) and 1 `track_dangling`; no silkscreen finding |
| Router and release gate | `release_ready` true, no blocking reason, no via in a pad |
| Trace width, as routed | `VIN_FUSED`, `VALVE1_SW`, `VALVE2_SW` 0.8 mm; `+12V_VALVE` 0.4 to 0.8 mm; `+3V3` 0.2 to 0.4 mm |
| Return paths | 6 signal vias with no ground via nearby (`W-SYNTH-DRC-016`, advisory); none blocking |
| BOM | 112 rows; 7 have an LCSC number and 10 a manufacturer number, from the command-line export |
| Schematic | one A1 page; every group is a region, the added parts are in `OTHER` |

## Layout

Measured on the exported file:

| | |
|---|---|
| Courtyard fill | 30.3 % of the board; 1 of 42 10 x 10 mm cells is empty |
| Connectors | all eight courtyards 0.00 to 0.23 mm from the board edge; the pin header `J_SWD` sits 0.23 mm inboard |
| Mounting holes | one in each corner, about 5 mm from two edges: from the left or right edge and from the top or bottom edge, `H1` 5.95 and 4.95 mm, `H2` 5.15 and 4.95, `H3` 5.45 and 4.95, `H4` 5.15 and 4.95 |
| Valve drivers | `Q_V1` and `Q_V2` 2.1 mm and their flyback diodes 2.6 mm from their connectors, courtyard to courtyard |
| Soil and flow circuits | their resistors 2.0 mm from `J_SOIL1`, `J_SOIL2` and `J_FLOW` |
| Test points | 2.1 to 7.1 mm from the nearest part on their net; over 5 mm: `TP_SWCLK` 7.1, `TP_NRST` 5.2 |
| Decoupling | mean 10.2 mm from a capacitor to the nearest IC on its rail; eight are 14.6 to 23.0 mm away |
| Silkscreen | no finding; the reference font is 0.8 mm |
| Isolated parts | `F1` and `J1`, 4.9 and 4.8 mm from their neighbours |
| Reset button, crystal | 16.7 mm and 9.0 mm from the MCU, centre to centre |
| SWD header | 43.8 mm from the MCU, at the right edge |

The reset button and the SWD header are farther from the MCU than they were in
the first layout of this board (10.8 mm and 23.8 mm). The placement advice asked
for the reset parts to move closer; every try broke the `NRST` net's routing, so
they stayed. That is an open item.

## The review of this board

| Review point | State |
|---|---|
| MP2307 divider and compensation | `R_FBT` 26.1 k over `R_FBB` 10 k at 0.925 V gives 3.34 V, as the source's note says; `R_COMP` 9.1 k and `C_COMP` 2.2 nF compensate. Calculated, not simulated. |
| Input protection | `D_TVS`, an SMAJ15A, sits after the reverse blocker `Q1`, with a gate-source clamp. Its 24.4 V clamp against the buck's absolute maximum is **not checked** and is open. |
| Crystal | The 15 pF load capacitors assume a 12 pF crystal load and 5 pF stray; the source says to confirm the crystal. |
| Footprints | `U3`, a 0.5 mm pitch DFN, is a footprint name the agent supplied and did not verify against a datasheet. The router reports that it cannot escape it on two layers and routed it anyway. |
| The 26 added parts | Rail decoupling and bulk capacitors, bleeders across the 3.3 V and 12 V rails, two rail LEDs with series resistors, two 5 V TVS diodes and test points. **Values are not checked against datasheets**: the 5 V TVS on the 3.3 V rail, the bleeder values and the LED currents are open. |
| Net classes | The widths above are as routed. All vias are 0.6 / 0.3 mm. |
| Schematic and BOM | The BOM cannot be quoted: generic passives carry no manufacturer or LCSC number. |

## Reproducibility

The source and sidecar in this directory were exported three times with
`synth export-kicad`, each into an empty directory, router included. Each cell
is the first ten characters of a SHA-256. All three exports are `release_ready`,
with 0 DRC errors and 0 unconnected pads.

| file | export 1 | export 2 | export 3 | |
|---|---|---|---|---|
| `irrigation_controller.kicad_sch` | f58eae78af | f58eae78af | f58eae78af | identical |
| `irrigation_controller.kicad_pro` | 2f3ab66b31 | 2f3ab66b31 | 2f3ab66b31 | identical |
| `irrigation_controller.kicad_sym` | 199557778a | 199557778a | 199557778a | identical |
| `bom.csv` | 609ebc347f | 609ebc347f | 609ebc347f | identical |
| `pnp.csv` | fb88389536 | fb88389536 | fb88389536 | identical |
| `irrigation_controller.synth.kicad_pcb` (un-routed) | e6523a246a | e6523a246a | e6523a246a | identical |
| `irrigation_controller.kicad_pcb` | 23d2302904 | 43a69960af | a58aaf1975 | identical after zone-fill ids |

The routed PCB differs between runs only in the ids KiCad gives its zone fills.
With those masked, all three are `75e6a9a7ad`.

The export through the MCP tool, which produced the files in the review
folder, is not byte-identical to these. It differs in the text of some part
values: the BOM manufacturer and LCSC columns, a few footprint values and the
package names in `pnp.csv`. The command-line export ran outside a checkout and
so used the embedded seed registry, which lacks those fields; I did not rerun it
with the checkout's registry. Positions and routing are the same.
