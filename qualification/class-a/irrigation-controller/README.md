# Irrigation controller

A two-zone irrigation controller: 12 V in through a polyfuse, an AO3401A
reverse blocker with a gate clamp and an SMAJ13A TVS, an MP2307 buck to 3.3 V,
an STM32G030, two low-side solenoid drivers, two switched resistive soil probes,
a flow-pulse input, an I2C header, an ambient light sensor and an EEPROM, an SWD
header, and rail decoupling, bleeders, snubbers, indicator LEDs and test points.
Two layers, JLCPCB, 75.1 x 65.4 mm, 126 components. There is no humidity sensor
on the board: a breakout attaches to the I2C header at address 0x44.

**Status:** the board is routed and released (every net routed, KiCad reports no
error, three exports are identical), and a fabrication package is generated from
it. The JLCPCB DFM check (2026-10-29, PCB DFM, `fab/` Gerbers) reports no
routing danger but silkscreen dangers and one through-hole-to-SMD danger: see
the DFM section. Nothing here was built or tested on hardware.

## How this board was made

An agent produced the design; Synth's own tools and scripts then packed and
tidied it, and a reviewer's list of faults was worked through. Every step is
logged under [`agent-run/`](agent-run/).

| Step | What ran | Result |
|---|---|---|
| 1. The design | An agent from one prompt ([`agent-run/prompt.md`](agent-run/prompt.md)), driving `synth mcp` with Read, Write and Edit only, from an empty workspace ([`report.md`](agent-run/report.md), [`tool-calls.md`](agent-run/tool-calls.md)) | 86 components |
| 2. Compaction | `synth_compact_layout` in `gap` mode | 75.1 x 65.4 mm after the later steps, released |
| 3. Fill | Two agent runs ([`fill-pass-prompt.md`](agent-run/fill-pass-prompt.md), reports [1](agent-run/fill-pass-report-1.md), [2](agent-run/fill-pass-report-2.md)): parts in the voids, each wired to a real function | 26 parts added |
| 4. Placement advice | [`tools/apply_advice_par.py`](agent-run/tools/apply_advice_par.py) applies Synth's placement advice in released steps | two passes, 15 and 18 moves kept |
| 5. Review fixes | An agent from [`prompt-1-review-fixes.md`](agent-run/review-fixes/prompt-1-review-fixes.md), report [`report-1-review-fixes.md`](agent-run/review-fixes/report-1-review-fixes.md): the clamps, the input TVS, the SHT30, the crystal load capacitors, the valve current, the schematic groups | source fixed; the board did not release |
| 6. Release search | Scripts ([`release_search.py`](agent-run/tools/release_search.py), [`fix_one.py`](agent-run/tools/fix_one.py), [`fix_pairs.py`](agent-run/tools/fix_pairs.py), [`verify_gap.py`](agent-run/tools/verify_gap.py)) that move one or two parts near a blocking finding and keep a move only if the export releases | released |
| 7. Input MOSFET | An agent from [`prompt-2-input-mosfet.md`](agent-run/review-fixes/prompt-2-input-mosfet.md), report [`report-2-input-mosfet.md`](agent-run/review-fixes/report-2-input-mosfet.md): `Q1` becomes an AO3401A (30 V) | released |
| 8. Fill again | An agent from [`prompt-3-fill.md`](agent-run/review-fixes/prompt-3-fill.md), report [`report-3-fill.md`](agent-run/review-fixes/report-3-fill.md): 18 parts, from values that already have a sourced part | 126 parts, 31.1 % fill |
| 9. Edits by the orchestrator | see below | final source and sidecar |

### Edits applied by the orchestrator

The droplet's agent login hit its weekly usage limit, so these small edits were
made by script and checked by export. Each one changes position or group
membership only.

- Sidecar positions: `R_D1` to x 45.0, `R_FBB` to (34.9, 25.1), `C_BS` to (27.1, 15.5) and `L1` to (25.0, 20.3), each found by the release search above because it cleared a blocking finding; `J_SOIL1` restored to (42.5, 63.0), where the other two 3-pin connectors sit, after an earlier slide moved it 2 mm off its edge, and `R_N5` moved to (50.0, 60.5) so it no longer overlaps that connector's courtyard; `C_OUT2` to (41.25, 23.0), 11.6 mm from the regulator instead of 29 mm.
- Groups: the review and fill runs left 18 schematic groups; [`merge_groups_remove_part.py`](agent-run/tools/merge_groups_remove_part.py) (with `KEEP_TP_V1=1`) folds them into 13. No part, value or net changed.

The `.synth` source and the placement sidecar beside this file are the result.
The board needs Synth built from pull request #370 and the ones it stacks on
(#310, #345, #348, #352, #368), the parts of #326, and #266 for `polyfuse_1r5a`
and `l_1210_10uh`, which are not in the registry on `main`. `smaj13a_tvs` and
`p_mosfet_ao3401a` were authored for this board and are in `registry/parts/`;
neither has a reviewer recorded yet.

The server was KiCad 10.0.7 on a 16-vCPU machine, Synth built from the layout
branch at commit 82e7a8ce. `allow_incomplete` was never set, in any run log or
script.

## What was measured

All figures come from the files of the first of the three exports below.

| | |
|---|---|
| Board | 75.1 x 65.4 mm, 4,912 mm2, two layers, 126 components (106 excluding holes and test points), 45 nets |
| Routing | state `routed`, 45 of 45 nets connected, 589 segments, 89 router vias, 165 vias in the file, all 0.6 / 0.3 mm |
| KiCad DRC, 10.0.7, `--refill-zones --severity-all`, with the KiCad footprint library table | 0 errors, 0 unconnected, 1 warning: `silk_overlap` (below). Without a library table the same file shows 126 more `lib_footprint_issues` warnings. |
| Router and release gate | `release_ready` true, no blocking reason, no via in a pad, no unresolved return via |
| Trace width, as routed | `VIN_FUSED`, `VALVE1_SW`, `VALVE2_SW` 0.8 mm; `+12V_VALVE` 0.4 to 0.8 mm; `+3V3` 0.2 to 0.4 mm |
| BOM | 126 rows, every one with a manufacturer, MPN, distributor, order number and page (below) |
| Schematic | 13 sheets, one per functional group, no `OTHER` region (below) |

The one DRC warning pairs the reference text of test point `TP_V1`, at (42.3,
18.5), with a silkscreen line of `D_FZ`, at (4.1, 41.0), 45 mm away. It does not
describe an overlap: deleting the `TP_V1` footprint from a scratch copy of the
board clears it, but hiding or renaming its label does not, and removing
`TP_V1` from the design breaks the routing, so it stays. It is treated as a
KiCad report fault.

The schematic is 13 sheets because 125 parts no longer fit one A0 page, which
is the largest single page Synth uses; each sheet is one functional group.

## Layout

Measured on the exported file:

| | |
|---|---|
| Courtyard fill | 31.1 % of the board; 1 of 42 10 x 10 mm cells is empty |
| Connectors | all eight courtyards 0.00 to 0.23 mm from the board edge; the pin header `J_SWD` sits 0.23 mm inboard |
| Mounting holes | one in each corner: from the left or right edge and from the top or bottom edge, `H1` 5.95 and 4.95 mm, `H2` 5.15 and 4.95, `H3` 5.45 and 4.95, `H4` 5.15 and 4.95 |
| Per-pin decoupling | `C_VDD1` 7.0 and `C_VDD2` 11.0 mm from the MCU, `C_U4` 4.5 and `C_U5` 3.5 mm from their ICs, footprint origin to origin |
| Regulator | `C_OUT2` 11.6 mm and `C_OUT1` 8.8 mm from `U1`, `C_IN1` 6.0 and `C_IN2` 8.9 mm |
| Crystal, reset button | 9.0 mm and 11.6 mm from the MCU (the reset button was 16.7 mm) |
| SWD header | 43.8 mm from the MCU, at the right edge |
| Test points | 2.1 to 7.1 mm from the nearest part on their net, except `TP_SCL` 12.1 and `TP_3V3` 16.0 mm |
| Rail capacitors | the capacitors that sit far from an IC (mean 12.4 mm over 37) are rail bulk and filter parts, not per-pin decoupling |
| Isolated parts | `F1`, `J1` and `R_N6`, 4.4 to 4.9 mm from their neighbours |

The SWD header is 43.8 mm from the MCU on purpose: it is a pin header on the
board edge so a probe cable reaches it without crossing the board, and the MCU
sits mid-board. The reset button is 11.6 mm from the MCU: reset is a slow
input behind a pull-up and a capacitor, so the track length does not affect its
timing, and pulling it closer broke routing in an earlier layout. The per-pin
decoupling distances above are measured from footprint origin to origin; the
nearest pad of the 20-pin MCU is closer than that, but `C_VDD2` at 11.0 mm is
the weakest point of the layout and is not justified electrically. `TP_3V3` is a rail probe on the 3.3 V pour; the one position
tried beside the new `C_OUT2` released but added a silkscreen warning, so it
stayed.

## The review of this board

SammyOina's review of 2026-10-28, point by point:

| Review point | State |
|---|---|
| 5 V clamps on 3.3 V signals | `D_N3` (5 V TVS on `FLOW_PULSE`) and `D_N4` (5 V TVS across the 3.3 V rail) are removed. `FLOW_PULSE` is clamped by `D_FZ`, a 3.3 V zener, behind `R_FS` 2.2 k: (13.2 V - 3.3 V) / 2.2 k = 4.5 mA, about 15 mW, in a 500 mW part. Nothing replaces `D_N4`. |
| Input TVS against the buck | `D_TVS` is an SMAJ13A (13 V standoff, 21.5 V clamp at 18.6 A), under the MP2307's 23 V input limit by 1.5 V. The supply maximum of 13.2 V is 0.2 V above the standoff, so it may leak a little; the leakage figure needs the datasheet. `Q1` is now an AO3401A (30 V drain-source) instead of a 20 V Si2301. `D_TVS` sits after `Q1`, so a surge at the connector reaches `Q1` unclamped and its 30 V rating is the limit. The 5.1 V gate-source zener `D_QG` is inside its 12 V gate rating. |
| SHT30 footprint and escape | The SHT30 and its capacitor are removed; a breakout attaches to `J_I2C` at 0x44. The router reported the 0.5 mm DFN unescapable; no fine-pitch part is left on the board. |
| BOM and fabrication output | 126 of 126 lines sourced; Gerbers, drill and pick-and-place generated and hashed (below). |
| DRC with the library table | 0 errors, 0 unconnected, 1 warning, above. |
| JLCPCB DFM | run 2026-10-29 on the `fab/` Gerbers (PCB DFM only, 2 layers, 75.1 x 65.4 mm): 7 danger items, below. |
| Return-path advisories | none unresolved: Synth's release gate now refuses a board with a signal via that has no ground return, and this board passes. |
| SWD and reset distance, decouplers | see the layout table: the reset button moved from 16.7 to 11.6 mm, the regulator's output capacitor from 29 to 11.6 mm; the SWD header is justified above. |
| Crystal load | the NDK NX3225GD-8MHZ-STD-CRA-3 has an 8 pF load. With 5 pF assumed for stray capacitance, CL = C1 x C2 / (C1 + C2) + 5 pF; C1 = C2 = 2 x (8 - 5) = 6 pF, the nearest E24 value 6.2 pF gives 8.1 pF. `C_X1` and `C_X2` are 6.2 pF. The 5 pF stray figure is an assumption. |
| Valve current | stated in the source as a 12 V solenoid at most 0.5 A: the `PWR12` class is 0.8 mm; the AO3400A (5.7 A on its listing) and the 1N5819HW flyback diode (1 A) are above it. This is calculated from listing figures, not measured. |
| Schematic references and groups | every part is in a named group; the `OTHER` region is gone. |
| Open | the MP2307 divider and compensation are calculated, not simulated; none of this was tested on hardware. |

## Bill of materials and fabrication package

[`fab/`](fab/) holds the files a quote needs:

- `irrigation_controller_gerbers.zip`: the nine Gerber layers, the Excellon drill file, a drill map and the job file, from KiCad 10.0.7;
- `irrigation_controller_cpl.csv`: 106 placements in JLCPCB's column order;
- `irrigation_controller_bom_orderable.csv`: the exported BOM joined with [`agent-run/tools/sourcing.csv`](agent-run/tools/sourcing.csv). Each of the 106 fitted lines carries refdes, value, KiCad footprint, package, manufacturer, manufacturer part number, rating, datasheet, distributor, order number and the product page; the other 20 lines are mounting holes and test points, which are board features with no part;
- `SHA256SUMS` and `GERBER_CONTENT_SHA256`: the Gerber headers carry the creation time, so the content is hashed with those lines masked; the three exports give the same hash, `719038ef3d`.

Regenerate them with [`fab_package.sh`](agent-run/tools/fab_package.sh). Every part
identity was looked up by manufacturer part number and the page is in the table.
The design names generic parts for passives and LEDs, so the table picks one
orderable part per value and footprint. Where the exact part the design names
has no LCSC listing, the table gives its page on another distributor, as the
part itself: the MP2307DN, the AO3401A, the fuse, the 9.1 k resistor and the
reset switch (DigiKey or Mouser), the EEPROM and the 100 uF capacitor (RS), and
the APDS-9301 and the 1x5 header (Farnell). The two LEDs are the only parts
chosen as stand-ins: the green is a NationStar NCD0603G1 and the amber a KENTO
KT-0603Y, listed as yellow at 584 to 594 nm, which is the amber range, so it is
used as the amber indicator. The APDS-9301 and the through-hole parts are not stocked for
JLCPCB assembly and are fitted by hand. The registry's own `lcsc_pn` for `d_schottky` (C8224),
`led_green_0603` (C72043) and `mp2307dn_buck_ep` (C18921) could not be confirmed
by any search and are not used here.

The rating, package and datasheet columns come from the manufacturer datasheets
(Yageo, NIC, NDK, Broadcom, AOS) where they could be read, and otherwise from the
product page named in the `link` column; the figures are listed in
[`extend_sourcing.py`](agent-run/tools/extend_sourcing.py).

## Reproducibility

The source and sidecar in this directory were exported three times with
`synth export-kicad`, each into an empty directory, router included. Each cell
is the first ten characters of a SHA-256. All three exports are `routed` and
`fabrication_ready`, with 0 DRC errors and 0 unconnected pads.

| file | export 1 | export 2 | export 3 | |
|---|---|---|---|---|
| `irrigation_controller.kicad_sch` | 4e594aca65 | 4e594aca65 | 4e594aca65 | identical |
| the 13 sub-sheets `irrigation_controller_*.kicad_sch` | | | | identical |
| `irrigation_controller.kicad_pro` | dfd26c417f | dfd26c417f | dfd26c417f | identical |
| `irrigation_controller.kicad_sym` | 0c12650ef7 | 0c12650ef7 | 0c12650ef7 | identical |
| `bom.csv` | 93f2a1e278 | 93f2a1e278 | 93f2a1e278 | identical |
| `pnp.csv` | 247cc82e12 | 247cc82e12 | 247cc82e12 | identical |
| `irrigation_controller.synth.kicad_pcb` (un-routed) | ebc4d0245d | ebc4d0245d | ebc4d0245d | identical |
| `irrigation_controller.kicad_pcb` | a4c8c34203 | b0c7950a27 | a117b1134c | identical after zone-fill ids |
| Gerber and drill content | 719038ef3d | 719038ef3d | 719038ef3d | identical |

The routed PCB differs between runs only in the ids KiCad gives its zone fills.
With those masked, all three are `c473cf6ca0`.

The command-line export ran with the corrected registry parts of #326 and the two
new parts, so its BOM text can differ from an export through the MCP tool, which
uses the registry checkout. Positions and routing are the same.

## JLCPCB DFM result (PCB DFM, 2026-10-29)

Run by Jeff on `irrigation_controller_gerbers.zip`. Clear: sharp corners, BGA,
via in pad, trace to edge, trace spacing, unconnected ends, trace width (0.2 mm,
85 good), plated-hole to trace (0.39 mm), drill checks, slots. Findings:

| Item | Danger | Warning |
|---|---|---|
| Silkscreen to hole (0 mm) | 50 | 0 |
| Silkscreen to pad (0 mm) | 5 | 33 |
| Through-hole to SMD clearance (1.9 mm) | 1 | 4 |
| Silkscreen line width (0.12 mm) | 0 | 50 |
| Annular ring (0.15 mm) | 0 | 100 |
| Pad spacing (0.15 mm) | 0 | 2 |
| Solder mask opening near a trace (0.15 mm) | 0 | 57 |
| Negative mask expansion (0.04 mm) | 0 | 50 |

The silkscreen items are reference text drawn over holes and pads; fixing them
is the silkscreen tidy, which Synth does not do yet. The report is not
committed (3.5 MB).

## Not done

- The JLCPCB quote and the SMT DFM (the PCB DFM ran, below).
- The DFM dangers below are not fixed yet.
- Values not checked against datasheets: the bleeder resistors and the LED currents; the 5 pF stray capacitance of the crystal.
- Nothing was built or run on hardware.
