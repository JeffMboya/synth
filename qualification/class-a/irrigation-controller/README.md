# Irrigation controller

A two-zone irrigation controller: 12 V in through a polyfuse, an AO3401A
reverse blocker with a gate clamp and an SMAJ13A TVS, an MP2307 buck to 3.3 V,
an STM32G030, two low-side solenoid drivers, two switched resistive soil probes,
a flow-pulse input, an I2C header, an ambient light sensor and an EEPROM, an SWD
header, and rail decoupling, bleeders, snubbers, indicator LEDs and test points.
Two layers, JLCPCB, 75.1 x 65.4 mm, 126 components. There is no humidity sensor
on the board: a breakout attaches to the I2C header at address 0x44.

**Status:** the board is routed and released (45 of 45 nets; KiCad 10.0.7 reports
0 errors and 0 unconnected pads with the library tables), three fresh exports are
identical, and a fabrication package is generated from it. Since SammyOina's review
of 2026-10-29 13:51: the input TVS sits ahead of the series MOSFET with the supply
range stated as 13.0 V at most; the green LEDs are a part that lights from 3.3 V;
silkscreen is 0.15 mm wide and cut back from vias and pads; vias are 0.6 mm over a
0.2 mm drill (a 0.20 mm ring); the schematic is one A0 sheet. The JLCPCB check has
not been run on this package (the last run, on the earlier one, is below). Not
fixed: five decouplers are still 4.8 to 8.4 mm from their pins (see "Open items").
Nothing here was built or tested on hardware.

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
| 10. Final blockers | An agent from [`final-blockers/prompt.md`](agent-run/final-blockers/prompt.md) moved `D_TVS` ahead of `Q1` and rewrote the notes at 13.0 V; stopped when a stray quote in a note broke the source's syntax | source edit kept, quote repaired (below) |
| 11. Searches | [`final-blockers/tools/`](agent-run/final-blockers/tools/): decoupler placement on a x1.4 layout, then `R_NRST`, `C_NRST` and `SW_RESET` spots and pairs, a release search | none reached a released board with the decouplers at their pins (see "Open items") |
| 12. Silkscreen and vias | Synth commits `8052ca52`, `a38802ad` (silkscreen widened and cut back) and `2ef37388` (0.2 mm drill, Synth PR #388) | in the exports below |

### Edits applied by the orchestrator

The droplet's agent login hit its weekly usage limit, so these small edits were
made by script and checked by export. Each one changes position or group
membership only.

- Sidecar positions: `R_D1` to x 45.0, `R_FBB` to (34.9, 25.1), `C_BS` to (27.1, 15.5) and `L1` to (25.0, 20.3), each found by the release search above because it cleared a blocking finding; `J_SOIL1` restored to (42.5, 63.0), where the other two 3-pin connectors sit, after an earlier slide moved it 2 mm off its edge, and `R_N5` moved to (50.0, 60.5) so it no longer overlaps that connector's courtyard; `C_OUT2` to (41.25, 23.0), 11.6 mm from the regulator instead of 29 mm.
- Groups: the review and fill runs left 18 schematic groups; [`merge_groups_remove_part.py`](agent-run/tools/merge_groups_remove_part.py) (with `KEEP_TP_V1=1`) folds them into 13. No part, value or net changed.

- Source: the agent's note edit left a stray `"` after "valve loads." in the `R_B1` note, which made Synth report three parse errors and drop that note; the one character was removed.
- LEDs: `D_PWR` and `D_N2` changed from `led_green_0603` to the registry part `ltst_c190kgkt` (same footprint and pins), by `sed`; the old part is a NationStar NCD0603G1 with a forward voltage of 2.6 to 3.6 V, which cannot light from 3.3 V through 680 ohm.
- Netclasses: `via_diameter 0.6mm via_drill 0.2mm` added to `PWR12` and `PWR33`, and a `Default` class with the same.
- Sidecar: only `D_TVS` moved, from (27.5, 7.0) to (14.5, 7.5) (the agent, to sit at the input).

The `.synth` source and the placement sidecar beside this file are the result.
The board needs Synth built from pull request #370 and the ones it stacks on
(#310, #345, #348, #352, #368), the silkscreen commits above (`SYN-silk-clear-of-vias`), the drill change of #388, the parts of #326, and #266 for `polyfuse_1r5a`
and `l_1210_10uh`, which are not in the registry on `main`. `smaj13a_tvs` and
`p_mosfet_ao3401a` were authored for this board and are in `registry/parts/`;
neither has a reviewer recorded yet.

The server was KiCad 10.0.7 on a 16-vCPU machine, Synth built from
branch `SYN-silk-clear-of-vias` at commit 2ef37388. `allow_incomplete` was never set, in any run log or
script.

## What was measured

All figures come from the files of the first of the three exports below
(KiCad 10.0.7, 16-vCPU server, Synth commit 2ef37388).

| | |
|---|---|
| Board | 75.1 x 65.4 mm, 4,912 mm2, two layers, 126 components (106 excluding holes and test points), 45 nets |
| Routing | state `routed`, 45 of 45 nets connected, 593 segments, 165 vias in the file, all 0.6 mm over a 0.2 mm drill (ring 0.20 mm); the project's `min_through_hole_diameter` is 0.2 and `min_via_diameter` 0.46 |
| KiCad DRC, 10.0.7, `--refill-zones --severity-all`, with the KiCad footprint library table | 0 errors, 0 unconnected, 18 warnings: 13 `lib_footprint_mismatch` and 5 `silk_overlap` (below) |
| Router and release gate | `release_ready` true, no blocking reason, no via in a pad, no unresolved return via |
| Trace width, as routed | `VIN_FUSED`, `VALVE1_SW`, `VALVE2_SW` 0.8 mm; `+12V_VALVE` 0.4 to 0.8 mm; `+3V3` 0.2 to 0.4 mm |
| Silkscreen | all 484 lines and 229 label texts at 0.15 mm; 0 lines touch a via (13 before); 20 outline lines cut back from vias and pads; 31 labels moved and 6 hidden (`C_NRST`, `TP_NRST`, `R_G1`, `R_PWR`, `C_N11`, `C_B4`) |
| BOM | 126 rows, every one with a manufacturer, MPN, distributor, order number and page (below) |
| Schematic | one A0 sheet, 13 named groups, no `OTHER` region; rendered from this export with `kicad-cli` |

The 13 `lib_footprint_mismatch` warnings are footprints whose silkscreen the
exporter cut back from a via or a pad to meet JLCPCB's silkscreen limits, so they
no longer equal the library copy: `U1`, `C_OUT2`, `U2`, `Y1`, `SW_RESET`, `J_SWD`,
`Q_V1`, `D_FLY1`, `Q_E1`, `C_F1`, `U4`, `R_SDA` and `R_SCL`. The 5 `silk_overlap`
warnings are reference text that grazes a neighbour: `C_BULK` against `C_B1`,
`TP_V1` against `C_SN1`, `D_FLY1` against the outline of `J_VALVE2`, `R_D1`
against the outline of `J_FLOW`, and `TP_SWCLK` against the outline of `J_SWD`.
A hidden label is still in the BOM and the placement file.

## Layout

Measured on the exported file:

| | |
|---|---|
| Courtyard fill | 31.1 % of the board; 1 of 42 10 x 10 mm cells is empty |
| Connectors | all eight courtyards 0.00 to 0.23 mm from the board edge; the pin header `J_SWD` sits 0.23 mm inboard |
| Mounting holes | one in each corner: from the left or right edge and from the top or bottom edge, `H1` 5.95 and 4.95 mm, `H2` 5.15 and 4.95, `H3` 5.45 and 4.95, `H4` 5.15 and 4.95 |
| Per-pin decoupling, pad to pin | `C_VDD1` 3.6 mm from `U2.4`; `C_U4` 4.8 mm from `U4.1`, `C_IN1` 5.5 and `C_IN2` 8.4 mm from `U1.2`, `C_U5` 6.3 mm from `U5.8`, `C_VDD2` 7.8 mm from `U2.4` |
| Regulator | `C_OUT2` 11.6 mm and `C_OUT1` 8.8 mm from `U1`, origin to origin |
| Crystal, reset button | 9.0 mm and 11.6 mm from the MCU (the reset button was 16.7 mm) |
| SWD header | 43.8 mm from the MCU, at the right edge |
| Through-hole to SMD pad | the nearest pairs: `J_SWD` to `TP_SWDIO` and `TP_SWCLK` 2.05 mm, `C_BULK` to `C_IN2` 2.4 mm (it was 1.9 mm) |
| Test points | 2.1 to 7.1 mm from the nearest part on their net, except `TP_SCL` 12.1 and `TP_3V3` 16.0 mm |
| Rail capacitors | the capacitors that sit far from an IC (mean 12.4 mm over 37) are rail bulk and filter parts, not per-pin decoupling |
| Isolated parts | `F1`, `J1` and `R_N6`, 4.1 to 4.8 mm from their neighbours |

The SWD header is 43.8 mm from the MCU on purpose: it is a pin header on the
board edge so a probe cable reaches it without crossing the board, and the MCU
sits mid-board. The reset button is 11.6 mm from the MCU: reset is a slow
input behind a pull-up and a capacitor, so the track length does not affect its
timing. The decoupling and reset distances are not claimed fixed: see "Open items".

## The review of this board

SammyOina's review of 2026-10-29 (13:51), then the earlier ones, point by point:

| Review point | State |
|---|---|
| DFM findings fixed, package regenerated and re-run | Fixed in the file: silkscreen 0.15 mm wide and 0 lines over a via; 20 outlines cut back from vias and pads; vias 0.6 over 0.2 mm (ring 0.20 mm); `C_BULK` to `C_IN2` 2.4 mm. Not run: JLCPCB has not seen this package, so no DFM result is claimed for it. The report of the earlier package is summarised below and is not committed (3.5 MB). |
| Stale schematic evidence | The schematic is one A0 sheet with 13 named groups and no `OTHER` region, rendered from the final export; renders, hashes, numbers and the fab package all come from the same export (below). The renders are not attached here. |
| Input protection rating | The supply range is 10.8 to 13.0 V. `D_TVS` is an SMAJ13A (Littelfuse SMAJ series datasheet: standoff 13.0 V; breakdown 14.40 to 15.90 V at 1 mA; 1 uA maximum leakage at the standoff; 21.5 V clamp at 18.6 A) on `VIN_FUSED`, ahead of `Q1`, so a transient is clamped before it reaches the MOSFET. Standoff equals the maximum input; the clamp is below the MP2307's 23 V recommended input and its 26 V absolute maximum (MPS datasheet) and below `Q1`'s 30 V drain-source rating. A reversed supply forward-biases `D_TVS` and `F1` opens. The 5.1 V gate-source zener `D_QG` is inside the 12 V gate rating. |
| `C_VDD2` at 11 mm | Not fixed: `C_VDD2` is 7.8 mm from its pin (pad to pin), `C_U4` 4.8, `C_U5` 6.3, `C_IN1` 5.5 and `C_IN2` 8.4. Every layout that brought them closer failed the release gate; see "Open items". |
| SMT DFM and quote | Not run. |
| LED, bleeder and crystal values | The green LEDs were a NationStar NCD0603G1 (Vf 2.6 to 3.6 V on its LCSC page), which cannot light from 3.3 V through 680 ohm; they are now the Lite-On LTST-C190KGKT (AlInGaP, Vf 1.9 to 2.4 V at 20 mA, 30 mA maximum DC, from Arrow and LCSC-based listings: the Lite-On PDF server fails certificate verification here, so the PDF was not opened). `D_PWR`, `D_N2`: (3.3 - 2.4) / 680 = 1.3 mA to (3.3 - 1.9) / 680 = 2.1 mA. `D_VALVE` (KENTO KT-0603Y, Vf 2.4 V on its page) from an MCU pin through 680 ohm: 1.3 mA, the pin's own drop not counted. `D_N1` from 13.0 V through 2.2 k: (13.0 - 2.4) / 2.2 k = 4.8 mA, 51 mW in a 100 mW resistor. Bleeders are at 13.0 V in the source notes (`R_B1` 100 k: 0.13 mA). The 5 pF crystal stray capacitance remains an assumption. |
| 5 V clamps on 3.3 V signals | `D_N3` and `D_N4` are removed. `FLOW_PULSE` is clamped by `D_FZ`, a 3.3 V zener, behind `R_FS` 2.2 k: (13.0 V - 3.3 V) / 2.2 k = 4.4 mA, 14 mW in a 500 mW part. |
| SHT30 footprint and escape | The SHT30 and its capacitor are removed; a breakout attaches to `J_I2C` at 0x44. No fine-pitch part is left on the board. |
| BOM and fabrication output | 126 of 126 lines sourced; Gerbers, drill and pick-and-place generated and hashed (below). |
| DRC with the library table | 0 errors, 0 unconnected, 13 `lib_footprint_mismatch` and 5 `silk_overlap` warnings, above. |
| Return-path advisories | none unresolved: Synth's release gate refuses a board with a signal via that has no ground return, and this board passes. |
| SWD and reset distance | see the layout table: the SWD header is justified above; the reset button is 11.6 mm from the MCU (it was 16.7 mm). |
| Crystal load | the NDK NX3225GD-8MHZ-STD-CRA-3 has an 8 pF load. With 5 pF assumed for stray capacitance, CL = C1 x C2 / (C1 + C2) + 5 pF; C1 = C2 = 2 x (8 - 5) = 6 pF; the nearest E24 value 6.2 pF gives 8.1 pF. `C_X1` and `C_X2` are 6.2 pF. |
| Valve current | stated in the source as a 12 V solenoid at most 0.5 A: the `PWR12` class is 0.8 mm; the AO3400A (5.7 A on its listing) and the 1N5819HW flyback diode (1 A) are above it. Calculated from listing figures, not measured. |
| Schematic references and groups | every part is in a named group; the `OTHER` region is gone. |
| Open | the MP2307 divider (VFB 0.925 V, 0.900 to 0.950 V, MPS datasheet) and compensation are calculated, not simulated; none of this was tested on hardware. |

## Bill of materials and fabrication package

[`fab/`](fab/) holds the files a quote needs:

- `irrigation_controller_gerbers.zip`: the nine Gerber layers, the Excellon drill file, a drill map and the job file, from KiCad 10.0.7;
- `irrigation_controller_cpl.csv`: 106 placements in JLCPCB's column order;
- `irrigation_controller_bom_orderable.csv`: the exported BOM joined with [`agent-run/tools/sourcing.csv`](agent-run/tools/sourcing.csv). Each of the 106 fitted lines carries refdes, value, KiCad footprint, package, manufacturer, manufacturer part number, rating, datasheet, distributor, order number and the product page; the other 20 lines are mounting holes and test points, which are board features with no part;
- `SHA256SUMS` and `GERBER_CONTENT_SHA256`: the Gerber headers carry the creation time, so the content is hashed with those lines masked; the three exports give the same hash, `466c93bc2d`.

Regenerate them with [`fab_package.sh`](agent-run/tools/fab_package.sh). Every part
identity was looked up by manufacturer part number and the page is in the table.
The design names generic parts for passives and LEDs, so the table picks one
orderable part per value and footprint. Where the exact part the design names
has no LCSC listing, the table gives its page on another distributor, as the
part itself: the MP2307DN, the AO3401A, the fuse, the 9.1 k resistor and the
reset switch (DigiKey or Mouser), the EEPROM and the 100 uF capacitor (RS), and
the APDS-9301 and the 1x5 header (Farnell). The amber LED is the only part
chosen as a stand-in: a KENTO KT-0603Y, listed as yellow at 584 to 594 nm, which is
the amber range. The green LED is the registry's `ltst_c190kgkt`, Lite-On
LTST-C190KGKT (LCSC C125094). The APDS-9301 and the through-hole parts are not stocked for
JLCPCB assembly and are fitted by hand. The registry's own `lcsc_pn` for `d_schottky` (C8224),
`led_green_0603` (C72043, no longer used here) and `mp2307dn_buck_ep` (C18921) could not be confirmed
by any search and are not used here.

The rating, package and datasheet columns come from the manufacturer datasheets
(Yageo, NIC, NDK, Broadcom, AOS) where they could be read, and otherwise from the
product page named in the `link` column; the figures are listed in
[`extend_sourcing.py`](agent-run/tools/extend_sourcing.py).

## Reproducibility

The source and sidecar in this directory were exported three times with
`synth export-kicad`, each into an empty directory, router included. Each cell
is the first ten characters of a SHA-256. All three exports are `routed`,
`fabrication_ready` and `release_ready`, with 0 DRC errors and 0 unconnected pads.

| file | export 1 | export 2 | export 3 | |
|---|---|---|---|---|
| `irrigation_controller.kicad_sch` (one sheet) | 8dcef91d18 | 8dcef91d18 | 8dcef91d18 | identical |
| `irrigation_controller.kicad_pro` | ee7cbc7b7c | ee7cbc7b7c | ee7cbc7b7c | identical |
| `irrigation_controller.kicad_sym` | 8104da92fc | 8104da92fc | 8104da92fc | identical |
| `bom.csv` | a649ff4b38 | a649ff4b38 | a649ff4b38 | identical |
| `pnp.csv` | ad2ac026d6 | ad2ac026d6 | ad2ac026d6 | identical |
| `irrigation_controller.synth.kicad_pcb` (un-routed) | 974ebf7229 | 974ebf7229 | 974ebf7229 | identical |
| `irrigation_controller.kicad_pcb` | 68039062f2 | 8d42ddeab1 | 3b7ae7f4c8 | identical after the router's and KiCad's random ids |
| Gerber and drill content | 466c93bc2d | 466c93bc2d | 466c93bc2d | identical |

The routed PCB differs between runs only in the random ids the router gives its
tracks and vias and KiCad gives its zone fills. With those masked, all three are
`eebba1d08f`. The Gerber hash masks the creation time KiCad writes into every
Gerber, the drill file and the job file, and leaves out the drill-map PDF. The
exports ran on the droplet with Synth commit 2ef37388 and the registry checkout
of that commit, which includes `ltst_c190kgkt`.

The command-line export ran with the corrected registry parts of #326 and the two
new parts, so its BOM text can differ from an export through the MCP tool, which
uses the registry checkout. Positions and routing are the same.

## JLCPCB DFM result (PCB DFM, 2026-10-29 16:13, the earlier package)

Run by Jeff on the previous `fab/` Gerbers (before the silkscreen cut-back, the 0.2 mm
drill and the moved `D_TVS`). **This package has not been run through JLCPCB.** Clear
then: sharp corners, BGA, via in pad, trace to edge, trace spacing, unconnected ends,
trace width (0.2 mm, 85 good), plated-hole to trace (0.39 mm), fiducial, pad to board
edge, drill checks and slots. Findings:

| Item | Danger | Warning |
|---|---|---|
| Silkscreen to hole (0 mm) | 18 | 7 |
| Through-hole to SMD clearance (1.9 mm) | 1 | 4 |
| Silkscreen to pad (0.14 mm) | 0 | 32 |
| Silkscreen line width (0.12 mm) | 0 | 50 |
| Annular ring (0.15 mm) | 0 | 100 |
| Pad spacing (0.15 mm) | 0 | 2 |
| Solder mask opening near a trace (0.15 mm) | 0 | 57 |
| Negative mask expansion (0.04 mm) | 0 | 50 |

What changed in the file since, measured on the export above, and what is still
unmeasured by JLCPCB:

| Finding | Now | State |
|---|---|---|
| Silkscreen to hole | seven vias were under outline lines; 0 silkscreen lines now touch a via; 20 outline lines were cut back | expected cleared; not run |
| Silkscreen line width | every silkscreen line and text is 0.15 mm, JLCPCB's listed minimum | expected cleared; not run |
| Silkscreen to pad | outlines cut back to 0.15 mm from pads (exact pad shapes) | expected cleared; not run |
| Annular ring | 165 vias, all 0.6 mm over 0.2 mm: a 0.20 mm ring, equal to the 0.20 mm the capabilities page lists | expected cleared; not run (#388) |
| Through-hole to SMD | `C_BULK` to `C_IN2` is 2.4 mm (it was 1.9 mm); `J_SWD` to `TP_SWDIO` and `TP_SWCLK` 2.05 mm | the danger pair moved apart; the 2.05 mm pairs remain; #380 |
| Pad spacing | 0.15 mm, equal to the page's 0.15 mm SMD pad clearance | at the limit |
| Mask opening near a trace, negative expansion | not traced | open; #378 |

### Open items

None of these is a routing or electrical fault; each is a fabrication-check or
placement finding. Limits are from JLCPCB's [capabilities page](https://jlcpcb.com/capabilities/pcb-capabilities), read 2026-10-29.

| Finding | What it is | Why it stays, or what tracks it |
|---|---|---|
| JLCPCB DFM and SMT DFM not run on this package | The numbers above are for the earlier package. | Needs the package uploaded (Jeff); no order is placed. |
| Decouplers 4.8 to 8.4 mm from their pins | Pad to pin: `C_U4` 4.8, `C_IN1` 5.5, `C_U5` 6.3, `C_VDD2` 7.8, `C_IN2` 8.4 mm; `C_VDD1` is at 3.6 mm. | Not fixed. On this layout the area round each pin is full: a release search (4 rounds of 42 trials) and a swap search never reached a released board with them closer. On a x1.4 layout a search put four of them within 3 mm of their pins and cleared every blocker but one: the `NRST` net at `U2.6`, which the moved decouplers sat in front of; 21 `R_NRST` spots, 21 `C_NRST` spots, `SW_RESET`, and six pairs of the two small reset parts could not clear it. Tracked in #377. |
| Through-hole to SMD, 4 warning | `J_SWD` to `TP_SWDIO` and `TP_SWCLK`, 2.05 mm. | The capabilities page has no through-hole-to-SMD row, so the threshold is not sourced. #380. |
| Pad spacing, 2 warning | 0.15 mm between pads. | Equal to the page's 0.15 mm SMD pad clearance. |
| Mask opening near a trace, 57 warning; negative mask expansion, 50 warning | Mask openings 0.15 mm from a trace; openings 0.04 mm smaller than their pads. | Not traced to pads. #378. |
| Library mismatch, 13 warning | Footprints whose silkscreen was cut back. | Intentional: the cut-back is what clears the silkscreen findings. |

## Not done

- JLCPCB PCB DFM, SMT DFM and quote on this package.
- Decouplers within 4 mm of their pins (#377).
- Values not checked against datasheets: the Lite-On LED datasheet (its server fails certificate verification here), the NDK crystal datasheet, and the 5 pF stray capacitance of the crystal.
- Nothing was built or run on hardware.
