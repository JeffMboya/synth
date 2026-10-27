# Irrigation controller

A two-zone irrigation controller: 12 V in through a fuse, a P-channel reverse
blocker and a TVS, an MP2307 buck to 3.3 V, an STM32G030, two low-side solenoid
drivers, two switched resistive soil probes, a flow-pulse input, an I2C bus with
a temperature and humidity sensor, an ambient light sensor and an EEPROM, and an
SWD header. Two layers, JLCPCB.

**Status: an engineer-review draft, not a release.** The board is routed and
KiCad reports no error on it, but it has no Gerbers, its BOM is not orderable,
and the open items below are real. Nothing here was built or tested on hardware.

## How this board was made

Nobody edited the design. An agent produced it from one prompt, driving
`synth mcp`, and the run was logged.

| | |
|---|---|
| Prompt | [`agent-run/prompt.md`](agent-run/prompt.md), a 21-section specification with a layout-quality block |
| Model | `claude-sonnet-5-5` |
| Tools | the Synth MCP server, plus Read, Write and Edit; no shell, no web |
| Start | an empty workspace and an empty user parts registry |
| Run | 73 tool calls in four logs, about 55 minutes of logged time; the agent's session limit ran out once and it was resumed three times ([`agent-run/resume-messages.md`](agent-run/resume-messages.md)) |
| Calls | [`agent-run/tool-calls.md`](agent-run/tool-calls.md) lists every one |
| Its report | [`agent-run/report.md`](agent-run/report.md), verbatim |

The agent's output is the `.synth` source and the placement sidecar beside this
file. The eight registry parts it imported or authored are in `registry/parts/`,
with a `footprint_dimensions` table taken from each footprint's courtyard where
the tool wrote none. The run used Synth built from the pull requests that fix
what the earlier runs of this prompt exposed: #335, #337, #339, #341, #343,
#345, #348, #350 and #352. The board needs them, and #266 for `polyfuse_1r5a`,
`smaj15a_tvs` and `l_1210_10uh`, which are not in the registry on `main`.

The server was KiCad 10.0.7 on a 16-vCPU machine. The checks marked 10.0.6 were
rerun on a second machine with the KiCad that CI uses.

## What was measured

All figures come from the files the agent's last `synth_export` wrote.

| | |
|---|---|
| Board | 83.9 x 67.0 mm, two layers, 86 components, 41 nets |
| Routing | state `routed`, 41 of 41 nets connected, 576 segments, 78 router vias |
| Vias in the file | 142, all 0.6 / 0.3 mm; the 64 beyond the router's are ground vias Synth adds beside layer changes |
| KiCad DRC, 10.0.6, `--refill-zones --severity-all` | 0 errors, 0 unconnected, 41 warnings: 22 `silk_overlap`, 17 `silk_over_copper`, 2 `track_dangling` |
| KiCad DRC, 10.0.7, run by the export | 0 errors, 0 unconnected, 41 warnings |
| Router and release gate | `release_ready` true, no blocking reason, no via in a pad |
| Trace width, as routed | `VIN_FUSED`, `VALVE1_SW`, `VALVE2_SW` 0.8 mm; `+12V_VALVE` 0.8 mm trunk, 0.2 to 0.72 mm at pads; `+3V3` 0.2 to 0.4 mm |
| Return paths | 5 signal vias with no ground via nearby (`W-SYNTH-DRC-016`, advisory) |
| `allow_incomplete` | never set, in any of the four logs |
| BOM | 86 rows; 19 have an LCSC number and 22 a manufacturer number |

## Layout

The compaction tool (`synth_compact_layout`) scaled the agent's pinned layout
inward and re-exported each step through the release gates. A step is kept only
when every net routes, KiCad DRC has no error and no via sits in a pad.

| Scale | Outline (mm) | Result |
|---|---|---|
| 100.0 % | 89.9 x 72.5 | released |
| 97.0 % | 87.9 x 70.1 | released |
| 94.0 % | 84.9 x 68.6 | released |
| 91.0 % | 82.9 x 67.1 | failed: a GND zone connection |
| 92.5 % | 83.9 x 68.1 | released |
| 91.7 % | 83.4 x 67.1 | failed: `U3` GND pads 9 and 2 unconnected |
| 92.1 % | 83.9 x 67.1 | released, kept |

That is 14 % less board area than the agent's own layout. Measured on the
exported file:

| | |
|---|---|
| Connectors | all eight courtyards 0.00 mm from the board edge; the pin header `J_SWD` sits 0.23 mm inboard |
| Valve drivers | `Q_V1` and `Q_V2` 0.6 mm and their flyback diodes 1.1 mm from their connectors |
| Soil and flow circuits | their resistors 2.0 mm from `J_SOIL1`, `J_SOIL2` and `J_FLOW`; the filter resistors 5.5 mm |
| Test points | 3.0 to 9.8 mm from the nearest part on their net; three are over 5 mm: `TP_NRST` 9.8, `TP_SWCLK` 7.1, `TP_VIN` 5.7 |
| Courtyard fill | 24.3 % of the board; 8 of 48 10 x 10 mm cells are empty |
| Reset button, crystal | 10.8 mm and 9.0 mm from the MCU, centre to centre |
| SWD header | 23.8 mm from the MCU, at the right edge |
| Silkscreen | 39 warnings, silkscreen over copper or over other silkscreen; Synth cannot place text yet |

## The review of this board

| Review point | State |
|---|---|
| MP2307 divider and compensation | `R_FBT` 26.1 k over `R_FBB` 10 k at 0.925 V gives 3.34 V, as the source's note says; `R_COMP` 9.1 k and `C_COMP` 2.2 nF compensate. Calculated, not simulated. |
| Input protection | `D_TVS`, an SMAJ15A, sits after the reverse blocker `Q1`, with a gate-source clamp. Its 24.4 V clamp against the buck's absolute maximum is **not checked** and is open. |
| Crystal | The 15 pF load capacitors assume a 12 pF crystal load and 5 pF stray; the source says to confirm the crystal. |
| Footprints | `U3`, a 0.5 mm pitch DFN, is a footprint name the agent supplied and did not verify against a datasheet. The router reports that it cannot escape it on two layers and routed it anyway. |
| Placement review | Three warnings were left by the agent as misreadings: `C_U5` read as a reset network, `C_NRST` and `C_VDD2` read as crystal load capacitors. |
| Net classes | The widths above are as routed. All vias are 0.6 / 0.3 mm. |
| Schematic and BOM | The BOM cannot be quoted: generic passives carry no manufacturer or LCSC number. |

## Reproducibility

The source and sidecar in this directory were exported three more times with
`synth export-kicad`, each into an empty directory, router included, and compared
with the files the agent's own `synth_export` wrote. Each cell is the first ten
characters of a SHA-256. All three exports passed validation.

| file | agent (MCP) | export 1 | export 2 | export 3 | |
|---|---|---|---|---|---|
| `irrigation_controller.kicad_sch` | b3f29ac973 | b3f29ac973 | b3f29ac973 | b3f29ac973 | identical |
| `irrigation_controller.kicad_pro` | d2ff554807 | d2ff554807 | d2ff554807 | d2ff554807 | identical |
| `irrigation_controller.kicad_sym` | 4010d27acd | 4010d27acd | 4010d27acd | 4010d27acd | identical |
| `bom.csv` | d240fdde7e | d240fdde7e | d240fdde7e | d240fdde7e | identical |
| `pnp.csv` | bdb7a8625c | bdb7a8625c | bdb7a8625c | bdb7a8625c | identical |
| `irrigation_controller.kicad_pcb` | 18a3e24c8c | 816acb3a0a | 375a57484a | 097f915120 | identical after zone-fill ids |

The PCB differs between runs only in the ids KiCad gives its zone fills. With
those masked, all four are `73577b7c53`.
