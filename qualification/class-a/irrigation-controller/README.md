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
silkscreen is 0.18 mm wide, cut back 0.25 mm from vias and pads, with the test-point rings dropped; vias are 0.6 mm over a
0.2 mm drill (a 0.20 mm ring); the schematic is one A0 sheet. JLCPCB's PCB DFM ran on the
previous package (2026-10-30 10:41, Gerber hash `3fccbb18d3`): 0 Danger items; silkscreen to hole and to pad are clear. This
package moves `C_U5` and `C_IN1` and has not been run through JLCPCB. Not fixed: `C_IN1` (5.3 mm) and `C_IN2` (8.4 mm)
are still beyond the 4 mm the placement review allows for the regulator's input pin (see "Open items").
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
| 12. Silkscreen and vias | Synth commits `8052ca52`, `a38802ad` (silkscreen widened and cut back), `2ef37388` (0.2 mm drill, Synth PR #388), then on `SYN-case1-binary` the 0.25 mm clearance with 0.18 mm strokes and `7972c911` (test-point rings dropped) and `be19e90a` (pin-1 triangles dropped too) | in the exports below |

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
- Sidecar: `C_U4` moved from (55.0, 44.5) to (51.62, 41.78) and the test points `TP_SWDIO` from (71.5, 26.5) to (70.34, 26.81) and `TP_SWCLK` from (71.5, 31.5) to (70.3, 31.5), each found by a release search ([`tht_search.py`](agent-run/tools/tht_search.py) for the test points, which keeps a position only when the export still releases) to clear a JLCPCB finding.
- Sidecar: `C_U5` moved from (54.0, 56.0) to (54.64, 48.76), `C_IN1` from (31.0, 11.5) to (27.54, 13.19), `C_BS` from (27.1, 15.5) to (25.1, 15.5) and the fill capacitors `C_N13` from (54.5, 48.0) to (52.5, 39.0) and `C_N9` from (26.5, 13.0) to (23.0, 6.0), each by [`make_room_search.py`](agent-run/tools/make_room_search.py): it moves a decoupling capacitor to a spot near its supply pin and moves the parts that do not serve that IC out of the way (the named input and supply capacitors, the IC's own support parts and connectors never move), and keeps a move only when the export still releases and no through-hole to SMD gap falls under 2.4 mm.

The `.synth` source and the placement sidecar beside this file are the result.
The board needs Synth built from pull request #370 and the ones it stacks on
(#310, #345, #348, #352, #368), the silkscreen commits above (`SYN-silk-clear-of-vias`), the drill change of #388, the parts of #326, and #266 for `polyfuse_1r5a`
and `l_1210_10uh`, which are not in the registry on `main`. `smaj13a_tvs` and
`p_mosfet_ao3401a` were authored for this board and are in `registry/parts/`;
neither has a reviewer recorded yet.

The server was KiCad 10.0.7 on a 16-vCPU machine, Synth built from
branch `SYN-case1-binary` (stacked on `SYN-silk-clear-of-vias`) at commit be19e90a. `allow_incomplete` was never set, in any run log or
script.

## What was measured

All figures come from the files of the first of the three exports below
(KiCad 10.0.7, 16-vCPU server, Synth commit be19e90a).

| | |
|---|---|
| Board | 75.1 x 65.4 mm, 4,912 mm2, two layers, 126 components (106 excluding holes and test points), 45 nets |
| Routing | state `routed`, 45 of 45 nets connected, 563 segments, 163 vias in the file, all 0.6 mm over a 0.2 mm drill (ring 0.20 mm); the project's `min_through_hole_diameter` is 0.2 and `min_via_diameter` 0.46 |
| KiCad DRC, 10.0.7, `--refill-zones --severity-all`, with the KiCad footprint library table | 0 errors, 0 unconnected, 125 warnings: 120 `lib_footprint_mismatch` and 5 `silk_overlap` (below) |
| Router and release gate | `release_ready` true, no blocking reason, no via in a pad, no unresolved return via |
| Trace width, as routed | `VIN_FUSED`, `VALVE1_SW`, `VALVE2_SW` 0.8 mm; `+12V_VALVE` 0.4 to 0.8 mm; `+3V3` 0.2 to 0.4 mm |
| Silkscreen | all 327 lines and 229 label texts at 0.18 mm; 0 lines touch a via; 300 outline lines cut back 0.25 mm from vias and pads; 25 silkscreen circles and polygons dropped (16 test-point rings, 9 pin-1 triangles); 28 labels moved and 13 hidden |
| BOM | 126 rows, every one with a manufacturer, MPN, distributor, order number and page (below) |
| Schematic | one A0 sheet, 13 named groups, no `OTHER` region; rendered from this export with `kicad-cli` |

The 120 `lib_footprint_mismatch` warnings are footprints whose silkscreen the
exporter cut back from a via or a pad (0.25 mm clearance, above JLCPCB's 0.15 mm
minimum), so they no longer equal the library copy. The 5 `silk_overlap`
warnings are reference text that grazes a neighbour or an outline: `C_BULK`
against `C_B1`, `R_COMP` against `R_EN`, `D_FLY1` against the outline of
`J_VALVE2`, `TP_V1` against `C_SN1`, and `TP_SOIL1` against the outline of `SW_RESET`. JLCPCB's DFM does not check silkscreen against silkscreen.
A hidden label is still in the BOM and the placement file.

## Layout

Measured on the exported file:

| | |
|---|---|
| Courtyard fill | 31.1 % of the board; 1 of 42 10 x 10 mm cells is empty |
| Connectors | all eight courtyards 0.00 to 0.23 mm from the board edge; the pin header `J_SWD` sits 0.23 mm inboard |
| Mounting holes | one in each corner: from the left or right edge and from the top or bottom edge, `H1` 5.95 and 4.95 mm, `H2` 5.15 and 4.95, `H3` 5.45 and 4.95, `H4` 5.15 and 4.95 |
| Per-pin decoupling, pad to pin | `C_VDD1` 3.6 mm from `U2.4`; `C_U4` 2.8 mm from `U4.1`, `C_IN1` 5.3 and `C_IN2` 8.4 mm from `U1.2`, `C_U5` 3.2 mm from `U5.8` (was 6.3), `C_VDD2` 7.8 mm from `U2.4` |
| Regulator | `C_OUT2` 11.6 mm and `C_OUT1` 8.8 mm from `U1`, origin to origin |
| Crystal, reset button | 9.0 mm and 11.6 mm from the MCU (the reset button was 16.7 mm) |
| SWD header | 43.8 mm from the MCU, at the right edge |
| Through-hole to SMD pad | the nearest pairs: the nearest pair is `C_BULK` to `C_IN2` at 2.4 mm (it was 1.9 mm); `TP_SWDIO` and `TP_SWCLK` were moved (below) from 2.05 mm to at least 2.9 mm of `J_SWD` |
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
| DFM findings fixed, package regenerated and re-run | The PCB DFM of 2026-10-30 07:21 (previous package) found 32 silkscreen-to-pad and 3 silkscreen-to-hole Danger items; the cause was the ring every test-point footprint draws 0.11 mm off its own pad, which the outline cut-back did not touch. Fixed in the file: those 16 circles are dropped, silkscreen is 0.18 mm wide and 0.25 mm from pads and vias, 0 lines touch a via, 301 outline lines are cut back. JLCPCB's PCB DFM of the previous package (2026-10-30 10:41) reports 0 Danger items, silkscreen to pad 0.25 mm and line width 0.18 mm both good, no silkscreen to hole finding; warnings remain for through-hole to SMD (1), pad spacing (2) and the solder-mask checks (107), listed below. The reports are summarised below and not committed (3.5 MB). |
| Stale schematic evidence | The schematic is one A0 sheet with 13 named groups and no `OTHER` region, rendered from the final export; renders, hashes, numbers and the fab package all come from the same export (below). The renders are not attached here. |
| Input protection rating | The supply range is 10.8 to 13.0 V. `D_TVS` is an SMAJ13A (Littelfuse SMAJ series datasheet: standoff 13.0 V; breakdown 14.40 to 15.90 V at 1 mA; 1 uA maximum leakage at the standoff; 21.5 V clamp at 18.6 A) on `VIN_FUSED`, ahead of `Q1`, so a transient is clamped before it reaches the MOSFET. Standoff equals the maximum input; the clamp is below the MP2307's 23 V recommended input and its 26 V absolute maximum (MPS datasheet) and below `Q1`'s 30 V drain-source rating. A reversed supply forward-biases `D_TVS` and `F1` opens. The 5.1 V gate-source zener `D_QG` is inside the 12 V gate rating. |
| Decouplers at their pins | Fixed for `C_U5` (3.2 mm from `U5.8`, was 6.3) and `C_U4` (2.8 mm, was 4.8); `C_VDD1` is 3.6 mm. Not fixed: `C_IN1` 5.3 mm and `C_IN2` 8.4 mm from the regulator's input pin `U1.2`, and `C_VDD2` 7.8 mm (the 4.7 uF bulk capacitor on `U2.4`, which Synth's placement review does not flag). The placement review flags `C_IN1` (5 mm) and `C_IN2` (9 mm) against its 4 mm limit and no other decoupling capacitor; see "Open items". |
| SMT DFM and quote | Not run. |
| LED, bleeder and crystal values | Datasheets opened (2026-10-30). **LED:** Lite-On LTST-C190KGKT, Spec DS22-2000-074 Rev N, via LCSC's copy (the Lite-On server's certificate still fails): AlInGaP green, VF 1.9 to 2.4 V at 20 mA (no typical; bins 4 to 8 in 0.1 V steps, each +/-0.1 V), 30 mA DC, 80 mA peak, 75 mW, 5 V reverse, peak 574 nm, 18 to 71 mcd at 20 mA. The green LEDs were a NationStar NCD0603G1 (Vf 2.6 to 3.6 V on its LCSC page), which cannot light from 3.3 V through 680 ohm. `D_PWR`, `D_N2`: (3.3 - 2.4) / 680 = 1.3 mA to (3.3 - 1.9) / 680 = 2.1 mA, from the 20 mA limits (the datasheet gives VF only at 20 mA); 14 times under the 30 mA maximum. The datasheet specifies brightness only at 20 mA; Fig. 4 puts 1 to 2 mA at roughly a tenth of it (graph read, approximate), about 1 to 11 mcd. **Crystal:** NDK NX3225GD-8MHZ-STD-CRA-3, Spec STD-CRA-3, Document EXS10B-26983 (28 Feb 2017), via an LCSC-hosted copy: 8.000 MHz fundamental, CL 8 pF, +/-50 ppm at 25 C, +/-150 ppm over -40 to +150 C, ESR 500 ohm maximum, drive 10 uW typical and 200 uW maximum. The DigiKey datasheet the BOM cited before (EXS11B-10834) is the automotive variant `EXS00A-CG04874` (+/-20 ppm, +/-50 ppm), not this part; the BOM link is replaced. The spec gives no shunt capacitance and no stray capacitance. **Stray capacitance:** the STM32G030 datasheet (DS12991 Rev 4, Table 36 note, p. 52) says PCB and MCU pin capacitance must be included and that 10 pF can be used as a rough estimate of the combined pin and board capacitance; its I/O pin capacitance is 5 pF typical. Read as 10 pF at each oscillator pin, 6.2 pF load capacitors give CL = (6.2 + 10) / 2 = 8.1 pF, the same 8.1 pF as the source notes' 6.2 / 2 + 5 (5 pF in the series formula is 10 pF per pin). The datasheet does not say which reading it means; ST's application note AN2867, which would, could not be downloaded (st.com refuses it), so the 5 pF is consistent with ST's figure but not confirmed. **Start-up margin:** ST's maximum critical crystal transconductance is 1.5 mA/V (Table 36). With the crystal's maximum ESR of 500 ohm, CL 8 pF and 8 MHz, the standard formula gm_crit = 4 ESR (2 pi f)^2 (C0 + CL)^2 (from AN2867, not opened) stays under 1.5 mA/V for any shunt capacitance C0 up to 9.2 pF (0.61 mA/V at 3 pF, 0.85 mA/V at 5 pF); C0 is not in the NDK spec, so the margin is bounded, not computed. `D_VALVE` (KENTO KT-0603Y, Vf 2.4 V on its page) from an MCU pin through 680 ohm: 1.3 mA, the pin's own drop not counted. `D_N1` from 13.0 V through 2.2 k: (13.0 - 2.4) / 2.2 k = 4.8 mA, 51 mW in a 100 mW resistor. Bleeders are at 13.0 V in the source notes (`R_B1` 100 k: 0.13 mA). |
| 5 V clamps on 3.3 V signals | `D_N3` and `D_N4` are removed. `FLOW_PULSE` is clamped by `D_FZ`, a 3.3 V zener, behind `R_FS` 2.2 k: (13.0 V - 3.3 V) / 2.2 k = 4.4 mA, 14 mW in a 500 mW part. |
| SHT30 footprint and escape | The SHT30 and its capacitor are removed; a breakout attaches to `J_I2C` at 0x44. No fine-pitch part is left on the board. |
| BOM and fabrication output | 126 of 126 lines sourced; Gerbers, drill and pick-and-place generated and hashed (below). |
| DRC with the library table | 0 errors, 0 unconnected, 121 `lib_footprint_mismatch` and 8 `silk_overlap` warnings, above. |
| Return-path advisories | none unresolved: Synth's release gate refuses a board with a signal via that has no ground return, and this board passes. |
| SWD and reset distance | see the layout table: the SWD header is justified above; the reset button is 11.6 mm from the MCU (it was 16.7 mm). |
| Crystal load | the NDK NX3225GD-8MHZ-STD-CRA-3 has an 8 pF load. With 5 pF assumed for stray capacitance, CL = C1 x C2 / (C1 + C2) + 5 pF; C1 = C2 = 2 x (8 - 5) = 6 pF; the nearest E24 value 6.2 pF gives 8.1 pF. `C_X1` and `C_X2` are 6.2 pF. The 8 pF is from NDK's spec (EXS10B-26983); the 5 pF is the series-formula equivalent of the 10 pF per pin ST's STM32G030 datasheet gives as a rough estimate, not confirmed (see "LED, bleeder and crystal values" above). |
| Valve current | stated in the source as a 12 V solenoid at most 0.5 A: the `PWR12` class is 0.8 mm; the AO3400A (5.7 A on its listing) and the 1N5819HW flyback diode (1 A) are above it. Calculated from listing figures, not measured. |
| Schematic references and groups | every part is in a named group; the `OTHER` region is gone. |
| Open | the MP2307 divider (VFB 0.925 V, 0.900 to 0.950 V, MPS datasheet) and compensation are calculated, not simulated; none of this was tested on hardware. |

## Bill of materials and fabrication package

[`fab/`](fab/) holds the files a quote needs:

- `irrigation_controller_gerbers.zip`: the nine Gerber layers, the Excellon drill file, a drill map and the job file, from KiCad 10.0.7;
- `irrigation_controller_cpl.csv`: 106 placements in JLCPCB's column order;
- `irrigation_controller_bom_orderable.csv`: the exported BOM joined with [`agent-run/tools/sourcing.csv`](agent-run/tools/sourcing.csv). Each of the 106 fitted lines carries refdes, value, KiCad footprint, package, manufacturer, manufacturer part number, rating, datasheet, distributor, order number and the product page; the other 20 lines are mounting holes and test points, which are board features with no part;
- `SHA256SUMS` and `GERBER_CONTENT_SHA256`: the Gerber headers carry the creation time, so the content is hashed with those lines masked; the three exports give the same hash, `1dbe5f798b`.

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
| `pnp.csv` | eb924ecd84 | eb924ecd84 | eb924ecd84 | identical |
| `irrigation_controller.synth.kicad_pcb` (un-routed) | 5ff05df104 | 5ff05df104 | 5ff05df104 | identical |
| `irrigation_controller.kicad_pcb` | 55a589256d | 29c5d2ddf8 | 21354dd5e1 | identical after the router's and KiCad's random ids |
| Gerber and drill content | 1dbe5f798b | 1dbe5f798b | 1dbe5f798b | identical |

The routed PCB differs between runs only in the random ids the router gives its
tracks and vias and KiCad gives its zone fills. With those masked, all three are
`3168fb45cd`. The Gerber hash masks the creation time KiCad writes into every
Gerber, the drill file and the job file, and leaves out the drill-map PDF. The
exports ran on the droplet with Synth commit be19e90a and the registry checkout
of that commit, which includes `ltst_c190kgkt`.

The command-line export ran with the corrected registry parts of #326 and the two
new parts, so its BOM text can differ from an export through the MCP tool, which
uses the registry checkout. Positions and routing are the same.

## JLCPCB DFM results

Four PCB DFM reports (PDF, 3.5 MB each, not committed). The fourth ran on the previous package (Gerber
content hash `3fccbb18d3`, JLCDFM report of 2026-10-30 10:41, 2 layers, 75.1 x 65.4 mm, PCB DFM only; the SMT DFM was
not run). This package (`1dbe5f798b`) moves `C_U5`, `C_IN1`, `C_BS` and two fill capacitors; the nearest through-hole to SMD gap is the same 2.4 mm and the
silkscreen counts are the same or lower, but it has not been run. In all four, clear: sharp corners, BGA, via in pad, trace to edge, trace spacing, unconnected ends,
trace width (0.2 mm), plated-hole to trace, fiducial, pad to board edge, drill checks and slots.

| Item (worst value) | 10-29 16:13 | 10-30 07:21 | 10-30 08:48 | 10-30 10:41, previous package |
|---|---|---|---|---|
| Silkscreen to pad | 0.14 mm: 0 D / 32 W | 0.12 mm: 32 D / 17 W | 0.21 mm: 0 / 0 (45 good) | 0.25 mm: 0 / 0 (26 good) |
| Silkscreen to hole | 0 mm: 18 D / 7 W | 0.08 mm: 3 D / 3 W | 0.14 mm: 0 / 2 | none found |
| Silkscreen line width | 0.12 mm: 0 / 50 | 0.15 mm: 0 / 50 | 0.18 mm: 0 / 0 (50 good) | 0.18 mm: 0 / 0 (50 good) |
| Annular ring | 0.15 mm: 0 / 100 | 0.2 mm: 0 / 0 | 0.2 mm: 0 / 0 (100 good) | 0.2 mm: 0 / 0 (100 good) |
| Through-hole to SMD | 1.9 mm: 1 D / 4 W | 2.06 mm: 0 / 5 | 2.06 mm: 0 / 5 | 2.4 mm: 0 / 1 (14 good) |
| Pad spacing | 0.15 mm: 0 / 2 | 0.15 mm: 0 / 2 | 0.15 mm: 0 / 2 | 0.15 mm: 0 / 2 (48 good) |
| Solder mask opening near a trace | 0.15 mm: 0 / 57 | 0.13 mm: 0 / 57 | 0.13 mm: 0 / 57 | 0.13 mm: 0 / 57 (1 good) |
| Negative mask expansion | 0.04 mm: 0 / 50 | 0.04 mm: 0 / 50 | 0.04 mm: 0 / 50 | 0.04 mm: 0 / 50 |

(D = Danger, W = Warning.) The previous package has no Danger item. What cleared, and why:

- Silkscreen to pad Danger (07:21 report): the ring every test-point footprint draws 0.11 mm
  off its own pad. Those circles are dropped, and silkscreen is kept 0.25 mm from pads and vias.
- Silkscreen to hole (08:48 report, 2 warnings): a pin-1 triangle (a silkscreen polygon) over a via
  pad. Polygons on a pad or via are dropped now; the 10:41 report finds none.
- Through-hole to SMD (5 warnings, 2.06 mm): `J_SWD` to `TP_SWDIO` and `TP_SWCLK`. A release search
  moved `TP_SWDIO` from (71.5, 26.5) to (70.34, 26.81) and `TP_SWCLK` from (71.5, 31.5) to
  (70.3, 31.5); one pair remains, `C_BULK` to `C_IN2` at 2.4 mm.

Remaining warnings:

| Finding | State |
|---|---|
| Through-hole to SMD, 1 warning (2.4 mm) | `C_BULK` to `C_IN2`. The capabilities page has no through-hole-to-SMD row, so the threshold is not sourced; #380. |
| Pad spacing, 2 warning (0.15 mm) | Equal to the page's 0.15 mm SMD pad clearance. |
| Mask opening near a trace, 57 warning; negative mask expansion, 50 warning | The export writes a solder-mask expansion of 0 and a minimum mask width of 0. JLCPCB's capabilities page lists 0.05 mm of growth around a pad for misregistration. A variant package with expansion 0.05 mm and minimum width 0.1 mm was built for an upload (`irrigation-controller-final-4-mask-experiment`, not committed); no result yet. #378. |

### Open items

None of these is a routing or electrical fault; each is a fabrication-check or
placement finding. Limits are from JLCPCB's [capabilities page](https://jlcpcb.com/capabilities/pcb-capabilities), read 2026-10-29.

| Finding | What it is | Why it stays, or what tracks it |
|---|---|---|
| PCB DFM re-run, SMT DFM and quote not run on this package | The PCB DFM ran on the previous package (above). | Needs the package uploaded (Jeff); no order is placed. |
| `C_IN1` 5.3 mm and `C_IN2` 8.4 mm from the regulator input `U1.2` | Pad to pin. Synth's placement review limit is 4 mm; it flags these two (5 and 9 mm) and nothing else. `C_VDD2` (4.7 uF bulk on `U2.4`, 7.8 mm) is not flagged. | Not fixed. The pin sits on the IC's left edge between `L1`, the bootstrap capacitor `C_BS` and the test point `TP_12V`; every spot within 4 mm is the IC itself or one of those support parts. A make-room search (`agent-run/tools/make_room_search.py`) moved the fill capacitors `C_N13`, `C_N9` and the bootstrap capacitor `C_BS`, put `C_U5` at its pin and `C_IN1` 0.2 mm closer, and found no released layout for `C_IN2` (moving `C_BS` leaves return-via blockers). Closing the rest needs a smaller input capacitor (an 0805 10 uF 25 V instead of the 1206, a derating decision for a reviewer) or a different regulator orientation. Tracked in #377. |
| Through-hole to SMD, 1 warning | `C_BULK` to `C_IN2`, 2.4 mm (the `J_SWD` pairs were moved clear). | The capabilities page has no through-hole-to-SMD row, so the threshold is not sourced. #380. |
| Pad spacing, 2 warning | 0.15 mm between pads. | Equal to the page's 0.15 mm SMD pad clearance. |
| Mask opening near a trace, 57 warning; negative mask expansion, 50 warning | Mask openings 0.15 mm from a trace; openings 0.04 mm smaller than their pads. | Not traced to pads. #378. |
| Library mismatch, 121 warning | Footprints whose silkscreen was cut back. | Intentional: the cut-back is what clears the silkscreen findings. |

## Not done

- JLCPCB PCB DFM re-run, SMT DFM and quote on this package (the PCB DFM of the previous package ran: 0 Danger).
- Decouplers within 4 mm of their pins (#377).
- Values not checked against datasheets: the crystal's shunt capacitance (not in the NDK spec), ST's AN2867 (the download is refused), the KENTO KT-0603Y LED datasheet, and the power ratings of the resistors.
- Nothing was built or run on hardware.
