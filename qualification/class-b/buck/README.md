# 24 V in, 3.3 V out buck with input parts (PTC, reverse-block diode, TVS) (SYN-67)

Status: designed, placed, routed, DRC and ERC counted, Gerber and drill files generated. Not fabricated,
not powered, no hardware test. All 8 new parts are UNVERIFIED (imported, no reviewer). No reviewer has
signed off this board, so the fab package reports `release_ready: false` by design.
Intended for 24 V +10% in and 1 A out. The converter is rated 4 A and the estimate supports 2 A, but the
PTC and the input connector set the 1 A rating (see Limits). No surge rating: the TVS clamp is above the
converter's absolute maximum.

## Purpose

Ticket #67 under #38 (Class B qualification): a switching-regulator board with input and output filtering,
feedback, power-good and thermal copper, plus a negative variant that the feedback-safety rule
(E-SYNTH-POWER-011, PR #56) must flag.

## What

- `buck.synth`, no placement sidecar, negative `buck_neg_fb_sw.synth`, hashes in `SHA256SUMS`, matrix and `#38` traceability in `QUALIFICATION.md`, evidence in `kicad/`, `reports/`, `images/`.
- `kicad/`: the KiCad project, BOM, pick-and-place, `gerbers/` (22 Gerbers plus the job file, 4 copper layers) and `drill/` (PTH and NPTH). `reports/`: `release.json` (source/DRC gate) and `release-fab.json` (fab package, untrusted), DRC, ERC, routing and connectivity.
- Chain: J1 (JST XH) -> F1 PTC -> D1 SS36 Schottky (reverse block) -> VIN node.
  VIN node: D2 SMBJ30A TVS, C1 47 uF bulk, C2 10 uF, C3 220 nF (on the U1 VIN pin), U1 LMR33640, UVLO divider.
- U1 -> L1 6.8 uH -> C5, C6, C9 (3x 22 uF, 25 V X7R) -> J2. C4 100 nF boot, C8 1 uF on VCC.
- Feedback: R_FB_T 100k from the output node (L1 pad 2 side), R_FB_B 43.2k, C7 56 pF across R_FB_T.
  Vout = 1.0 x (1 + 100k/43.2k) = 3.315 V typical, 3.26 to 3.36 V over Vref 0.985 to 1.015 V (ideal resistors).
- Power-good: U1 pin 4 is open drain, pulled up to +3V3 through R_PG 100k and brought out on J2 pin 3. D4 (green, "PG FAULT") and R_PGLED 470 R run from +3V3 to PG, so the LED lights while PG is low and stays dark while power is good.
- Thermal: U1 pin 9 is a 2.41 x 3.1 mm exposed pad soldered to the F.Cu GND pour (see Thermal).
- Negative file: identical except R_FB_T and C7 hang from SW. Besides E-SYNTH-POWER-011 it emits the same warnings as the good board except 1 fewer E-SYNTH-SCHEM-004 and 1 fewer E-SYNTH-SCHEM-005 (20 against 22).
- Board: 60.85 x 31.09 mm (1892 mm2), 4 layers, GND pours on all layers. Power tracks 0.5 mm, signal
  0.127 mm. Placement is Synth's own (`--place-route-iterate`, upstream `main` `2054046`) with no sidecar. The
  only constraints are hard edge hints: J1 on the left edge and J2 on the right edge, so a cable plugs in at
  the board edge and nothing sits in front of it. The mounting holes are placed by Synth, in the top corners.
  The previous revision was hand-placed at 71.9 x 31.125 mm (2238 mm2) with shorter decoupling loops; this one
  is 15% smaller and its loops are longer (see Layout under "Not true yet", B5, B12).
- Two M2.5 non-plated mounting holes (H1, H2) in opposite corners.

## UVLO (LMR33640 SNVSB99C section 9.2.2.9, Eq 10)

Datasheet values: VEN-H 1.2 / 1.231 / 1.26 V (min/typ/max), hysteresis 100 mV. RENB = R_UV_B = 100k, RENT = R_UV_T = 1.37M.
- Start = VEN-H x (1 + RENT/RENB) = 1.231 x (1 + 13.7) = 18.09 V.
- Stop = Start x (1 - VEN_HYS/VEN-H) = 18.09 x (1 - 0.1/1.231) = 16.62 V.
- 1% resistors, typical threshold: start 17.8 to 18.4 V, stop 16.3 to 16.9 V.
- EN at 28 V in is 28 x 0.1/1.47 = 1.90 V; the EN absolute maximum is VIN + 0.3 V.

## Power-good

| Item | Value | Source |
| --- | --- | --- |
| Pin | U1 pin 4, open-drain NMOS | SNVSB99C 8.3.1 |
| Pull-up | R_PG 100k to +3V3 | 8.3.1 suggests 100k to VCC or VOUT |
| Sink current | 33 uA through 100k, limit 5 mA | 8.3.1 |
| Asserted low | during start-up, current limit and thermal shutdown | 8.3.1 |
| Released high | Vout inside 95/105% (typ), 93/107% over limits | 7.5, Figure 8-1 |
| Start-up delay | about 4 ms from EN asserted | 8.3.1 |
| Glitch filter | output excursions shorter than tPG do not trip it | 8.3.1 |
| Presented on | J2 pin 3 (3.3 V logic) | this board |

The netlist proves the connection (A15 in `QUALIFICATION.md`). Nothing proves the thresholds, the delay or
the behaviour on a fault: Synth has no power-good rule (`synth capability list` marks
`power_sequencing_and_power_good` unsupported), and this board was not powered.

## Thermal

- U1's exposed pad (pin 9, 2.41 x 3.1 mm) is soldered to the F.Cu GND pour. The pour is stitched to the
  inner ground planes by 16 GND vias; 2 of them are within 4 mm of the pad edge (0.55 and 3.00 mm).
- There are no vias **in** the pad. The board's fabrication floor is a 0.3 mm drill (the exported
  `.kicad_pro` says `min_through_hole_diameter 0.3`), and every KiCad HSOP-8 thermal-via footprint ships
  0.2 mm drills, so that footprint was not used.
- Estimate, not a measurement: TI Figure 9-3 gives about 38 C/W for the DDA package on a four-layer board
  with 20 to 24 cm2 of copper per layer, and this board has 22.4 cm2. At 1 A out with the assumed 85%
  efficiency the loss is 3.3/0.85 - 3.3 = 0.58 W, so the junction is about 70 + 22 = 92 C. At 2 A out the
  loss is 1.17 W and the junction about 114 C. Both are under the 125 C limit, but the efficiency is an
  assumption and the copper-area curve is not a board-level simulation.
- TI's `RthetaJA` table value is deliberately not used: the datasheet says it is not valid for design.

## Parts and datasheets (read with curl + pdftotext, 2026-10-24)

| Part | Datasheet | Revision | Values taken | Conflict or gap |
| --- | --- | --- | --- | --- |
| U1 TI LMR33640ADDA (HSOIC-8 DDA) | https://www.ti.com/lit/ds/symlink/lmr33640.pdf | SNVSB99C, Nov 2020 | 3.8-36 V recommended, 38 V abs max; 4 A; 400 kHz; internal compensation; VFB 0.985/1.0/1.015 V; pins 1 GND, 2 VIN, 3 EN, 4 PG, 5 FB, 6 VCC, 7 BOOT, 8 SW, 9 EP; Table 9-2 at 3.3 V/400 kHz: L 6.8 uH, Cout 4x22 uF typical (3x22 minimum), Cin 10 uF + 220 nF, Rfbt 100k, Rfbb 4.32k (Table 9-3 says 43.3k), Cboot 100 nF, Cvcc 1 uF; PG open drain, 4 ms start-up delay, 93/95/105/107% thresholds; thermal pad on pin 9 | Table 9-2's Rfbb of 4.32k contradicts Eq 3 and Table 9-3; 43.2k is used here. KiCad symbol LMR33640ADDA has the pin order of the physical package, not the symbol drawing |
| L1 Bourns SRN6045TA-6R8M | https://www.bourns.com/docs/Product-Datasheets/SRN6045TA.pdf | REV. 06/24 (footer) | 6.8 uH +-20%, DCR 34 mOhm typ, Irms 3.6 A, Isat 5.7 A (L -30%), 6.0 x 6.0 mm | Registry has no Isat field; Isat is in the description |
| D1 Vishay SS36 | https://www.vishay.com/docs/88751/ss32.pdf | 23-Apr-2020 (doc 88751) | VRRM 60 V, 3 A, IFSM 100 A, VF 0.75 V max at 3 A, IR 0.5 mA at 25 C and 10 mA at 100 C, SMC | KiCad `Diode:SS36` defaults to SMA, this sheet is SMC (SMC used). VF at low current not read |
| D2 Vishay SMBJ30A | https://www.vishay.com/docs/88392/smbj.pdf | 09-Jan-2024 (doc 88392) | 600 W 10/1000 us, VWM 30 V, VBR 33.3-36.8 V at 1 mA, VC 48.4 V at 12.4 A, VBR temperature coefficient 0.099 %/C, SMB | No KiCad SMBJ30A symbol; Device:D_Zener used. Registry `max_voltage_v` 30 is VWM. The clamp is above U1's 38 V absolute maximum, so this part is not an OVP |
| F1 Bourns MF-MSMF110/33X | https://www.bourns.com/docs/Product-Datasheets/mfmsmf.pdf | REV. BE, 09/26 (footer) | Vmax 33 V, Imax 20 A, Ihold 1.1 A at 23 C, 0.93 A at 40 C, 0.73 A at 60 C, 0.63 A at 70 C, 0.50 A at 85 C, Itrip 2.2 A, R 0.06-0.20 Ohm | none found. Registry `max_current_ma` 1100 is Ihold at 23 C |
| J1, J2 JST B2B-XH-A / B4B-XH-A | https://www.jst-mfg.com/product/pdf/eng/eXH.pdf | none printed | 2.5 mm pitch, 3 A, 250 V | Existing registry `jst_xh_2pin`/`jst_xh_4pin` say 5 V / 100 mA; that conflicts with this sheet, so new parts were added |
| D3 Lite-On LTST-C190KGKT | https://optoelectronics.liteon.com/upload/download/DS22-2000-074/LTST-C190KGKT.PDF | not checked | none (existing registry part) | Site unreachable (curl code 000) |
| C1 47 uF | none | | generic 8 x 10 mm SMD electrolytic, 50 V class | no MPN, ESR and ripple rating unknown |
| C2, C5, C6, C9 (1206), C3, C4, C7, C8 (0603), resistors | none | | generic registry parts; voltage and dielectric are set per instance in `buck.synth` | no MPN. Murata and TDK pages returned 404/403/500: no DC-bias data |

## Limits and honesty notes

- Overvoltage: the converter is rated to 36 V recommended / 38 V absolute maximum, so a **sustained**
  28 to 32 V rail is inside its rating and the board survives it. A **surge** is not covered: SMBJ30A
  clamps at 48.4 V (12.4 A), above the 38 V absolute maximum, so the TVS is an ESD/transient clamp on the
  connector and not an OVP. A rated surge needs a 60 V front end or a series surge stopper; neither is
  designed here. The TVS standoff (30 V) is above the 26.4 V maximum input, so it does not conduct on a
  healthy rail.
- PTC and rating: output current is rated 1 A. At 1 A out the input current is 3.3 / 0.85 / 24 = 0.162 A
  at 24 V and 0.248 A at the UVLO stop voltage, against F1 Ihold 0.63 A at 70 C (3.9x and 2.5x). At 2 A
  out the input current is 0.50 A, no margin at 85 C and 1.26x at 70 C, so 2 A is PTC-limited even though
  the converter and the thermal estimate allow it.
- Output capacitors: Table 9-2 asks for 3x 22 uF minimum at 3.3 V/400 kHz; C5, C6 and C9 are 22 uF 25 V
  X7R 1206. DC-bias derating at 3.3 V is unknown. The internal compensation is specified for this
  inductance and capacitance, but stability is unverified.
- Input capacitors: Table 9-2 asks for 10 uF + 220 nF. C2 is 10 uF X7R 50 V; its effective value at 24 V
  is unknown and may be well under 10 uF. C1 47 uF electrolytic adds bulk with unknown ESR.
- Reverse polarity: Schottky chosen over a P-FET. SS36 VRRM 60 V against 26 V. Drop 0.75 V max at 3 A,
  so under 0.13 W at 0.162 A (bound; VF at low current not read). A P-FET at 24 V needs a gate clamp;
  no P-FET datasheet was read.
- Inductor: ripple at 24 V is 1.05 A pp; at 1 A out the peak is 1.52 A against Isat 5.7 A. At 2 A out the
  peak is 2.52 A.

## Commands

```
export XDG_DATA_HOME=$(mktemp -d)   # empty dir: a user overlay in ~/.local/share/synth shadows registry parts
export KICAD_SYMBOL_DIR=/usr/share/kicad/symbols KICAD_FOOTPRINT_DIR=/usr/share/kicad/footprints
scripts/setup-routing-engines.sh --freerouting-only          # FreeRouting jar + Java, or symlink tools/freerouting and tools/jre25
B=$(pwd)/qualification/class-b/buck   # absolute path, set from the repo root before any cd
synth validate $B/buck.synth
synth route $B/buck.synth --router free-routing
synth export-kicad $B/buck.synth --out out/buck --allow-unverified-parts --validate-erc \
  --router free-routing --pnp --gerbers --drill             # fab package: reports/release-fab.json
synth export-kicad $B/buck.synth --out out/buck-nofab --allow-unverified-parts --validate-erc \
  --router free-routing                                     # source/DRC gate: reports/release.json
kicad-cli pcb drc --severity-all --refill-zones --format json -o $B/reports/buck.kicad-drc.json $B/kicad/buck.kicad_pcb
kicad-cli sch erc --severity-all --format json -o $B/reports/buck.kicad-erc.json $B/kicad/buck.kicad_sch
synth validate $B/buck_neg_fb_sw.synth
python3 -I $B/pad_gaps.py $B/kicad/buck.kicad_pcb
```

`out/` is gitignored scratch. The two `export-kicad` runs produce a byte-identical `.kicad_pcb` and
`.kicad_sch`; they differ only in whether the fab gate is asked, which is why there are two release
reports.
Registry import: `synth part import-kicad Regulator_Switching:LMR33640ADDA --id lmr33640`;
likewise `Device:L`, `Diode:SS36` (Diode_SMD:D_SMC), `Device:D_Zener` (SMBJ30A, Diode_SMD:D_SMB),
`Device:Polyfuse`, `Device:C_Polarized`, `Connector_Generic:Conn_01x02`, `Connector_Generic:Conn_01x04`,
`Mechanical:MountingHole`.
The files were then edited by hand (names, MPN, dimensions, datasheet URL, `switch_node` on SW and `feedback` on FB as for `mp2307_buck`).

## Measured results

The export routes with FreeRouting 2.4.1; the routed board is written as `out/buck/buck.kicad_pcb` when Synth's gate passes (state `routed`). Numbers below are from that file, on Synth `main` `1eac8a1`, 2026-10-26.

| Number | Value | Command |
| --- | --- | --- |
| validate, good board | exit 0, 0 errors, 22 warnings (9 part-unverified, 3 E-SYNTH-SCHEM-003 (schematic-sheet distance of a cap to U1, not PCB distance), 2 E-SYNTH-SCHEM-004 long explicit net, 4 E-SYNTH-SCHEM-005 junction fan-out, 1 anomaly, 3 supply) | `synth validate $B/buck.synth 2>&1 \| grep -c '^warning'` and `grep -c '^error'` |
| validate, negative | exit 1, 1 error (E-SYNTH-POWER-011), 20 warnings | same on `buck_neg_fb_sw.synth` |
| routing | `routed`, 13 nets connected, 0 open, 119 segments, 26 vias, 330.69 mm | `synth route $B/buck.synth` (JSON `state`, `statistics`); cross-check `grep -c '^\t(segment' out/buck/buck.kicad_pcb`, `grep -c '^\t(via' ...` |
| Synth routing check | kicad_drc 0 errors, 0 unconnected, 27 warnings; `fabrication_ready` true (routing check only) | `synth route` JSON `validation` |
| Release gate | `release_ready` true, `production_status` production, 0 blocking signal-integrity findings; with `--gerbers --drill` the same board reports `release_ready` false and `untrusted` because 8 distinct parts have no reviewer (9 components use them: D3 and D4 share one part) | `reports/release.json`, `reports/release-fab.json` |
| DRC | 0 errors, 0 unconnected, 27 warnings, all lib_footprint_issues | `kicad-cli pcb drc ... -o reports/buck.kicad-drc.json`, then `python3 -I -c "import json,collections as c;d=json.load(open('$B/reports/buck.kicad-drc.json'));print(c.Counter((v['severity'],v['type']) for v in d['violations']),len(d['unconnected_items']))"` |
| ERC | 0 errors, 72 warnings: 45 lib_symbol_issues, 27 footprint_link_issues (libraries not in the KiCad config) | `kicad-cli sch erc ... -o reports/buck.kicad-erc.json`, same one-liner over `sheets[].violations` |
| Schematic netlist | 13 nets: the source's 8 `net` blocks plus the 5 distinct names its 9 `connect` statements create, node for node | `kicad-cli sch export netlist --format kicadsexpr -o /tmp/buck.net $B/kicad/buck.kicad_sch` |
| Outline | 60.85 x 31.09 mm (x -0.05 to 60.8, y 2.45 to 33.54) | `grep -A3 gr_line $B/kicad/buck.kicad_pcb \| grep start` |
| Manufacturing exports | 22 Gerbers plus the job file (4 copper layers), PTH 32 holes (26 x 0.3 mm vias, 6 connector pins), NPTH 2 x 2.5 mm mounting holes | `ls $B/kicad/gerbers $B/kicad/drill`; `grep -c '^X' $B/kicad/drill/buck-PTH.drl` |
| Hashes | pcb 919ba36aa69f..., sch 4cfbaf5c6522..., two exports identical | `cd $B && sha256sum -c SHA256SUMS` (full values in `SHA256SUMS` and `QUALIFICATION.md`) |

Pad-edge gaps in the placed, routed board, from `pad_gaps.py` (pcbnew pad bounding boxes, edge to edge):

| Pair | Gap mm |
| --- | --- |
| C2 (10 uF) / C3 (220 nF) to U1 VIN | 10.86 / 19.80 |
| C2 / C3 ground pad to U1 GND | 13.75 / 21.22 |
| C4 to U1 BOOT / SW | 6.49 / 4.63 |
| C8 to U1 VCC | 4.73 |
| L1 pad 1 to U1 SW | 3.33 |
| C5, C6, C9 to L1 pad 2 | 14.35 / 12.85 / 19.33 |
| D2 TVS to U1 VIN | 10.00 |
| R_UV_T to U1 EN | 2.33 |
| R_PG to U1 PG | 24.13 |
| R_FB_T / R_FB_B to U1 FB | 3.39 / 13.77 |
| C7 to R_FB_T (FB side) | 8.80 |
| R_FB_B ground pad to U1 GND | 5.86 |
| R_FB_T FB pad to L1 pad 1 (SW) | 8.98 |

There is no placement sidecar; every position comes from Synth. D4 is at (39.5, 15.0) mm and R_PGLED at
(39.5, 13.5) mm in the exported board, J1 at (2.85, 26.91) mm on the left edge and J2 at (56.85, 29.91) mm on
the right edge. `pad_gaps.py` on the final routed file is the reference for gaps.

## Negative case: feedback from the switch node

`synth validate $B/buck_neg_fb_sw.synth` exits 1 (JSON, schema 1.3):
```
error: [E-SYNTH-POWER-011] regulator feedback is taken from the switching node
  found:    feedback pin `U1.fb` (group "FEEDBACK") is joined through resistor `R_FB_T` to switch-node pin `U1.sw` (group "BUCK")
  expected: the feedback pin to sense the regulated output, not the switching node
```
`synth export-kicad` stops with "Synth ERC validation failed; use --force to export anyway" (exit 2). The good board does not raise it. The rule follows one resistor only.

## Qualification matrix

The matrix (A: checked by tools, B: needs an expert) is in [`QUALIFICATION.md`](QUALIFICATION.md),
together with the `#38` checklist traceability for this board and the row it declares for
`qualification/matrix.toml` when that corpus lands on `main`. Evidence files: `kicad/`,
`reports/`, `images/`.

## Not true yet

- Not fabricable on this evidence: parts UNVERIFIED, no surge rating, nothing built. The fab package says
  so itself (`release_ready: false`, `production_status: untrusted`).
- Layout: the placement is Synth's own and the decoupling loops are longer than on the hand-placed revision.
  C3 (the 220 nF high-frequency cap) is 19.8 mm from U1's VIN pad, C5, C6 and C9 are 14.4, 12.9 and 19.3 mm
  from L1 pad 2, and the C2/C3 ground returns are 13.8 and 21.2 mm from U1's GND pin. The hand-placed
  revision had these at 2.88 mm and 1.20 to 1.73 mm. A hint such as `near: "U1"` on the input and output
  capacitors would pull them in; none was added, so this is what the placer does on its own. The board
  carries 330.69 mm of copper (`+3V3` 94.05 mm, `PG` 74.42 mm, FB 23.77 mm) and `W-SYNTH-DRC-016` reports
  10 layer changes with no ground return via within 0.64 mm.
- Silkscreen: the "PG FAULT" and "PWR" LED values are on the F.Fab layer only, so the board prints just the references D4 and D3. Synth has no silk text for a value here; a printed label needs a Synth change, not done in this branch. Adding D4 re-ran the router over the whole board, so other copper moved too (see A9 and A16 in `QUALIFICATION.md`).
- Silkscreen: DRC reports no silkscreen warnings on this placement (the hand-placed revision had 5).
- Schematic (rendered and read): readable, not tidy. `tools/schematic_quality.py` counts 1 overlapping
  text pair, 10 texts with a wire through them, 0 wire crossings and 1371.6 mm of wire; mean
  passive-to-pin distance is 62.6 mm (max 102.9). No schematic sidecar is used (default layout).
- Routing used FreeRouting defaults, no length or coupling constraints.

## Not verified

- Nothing powered, measured or thermally tested. Efficiency (85%) and the 0.58 W loss are assumptions.
- The MCP placement review (`synth_describe_placement`) was not run on this revision: the Synth MCP server was unavailable.
- No reviewer. Availability, LCSC numbers and assembly not checked.
- LED datasheet unreachable. J1/J2, C1, and all generic passives have no MPN or manufacturer sheet.
- PTC behaviour with the TVS in a surge, and inrush into C1 and the output caps, are not analysed.
- The Gerbers and drill files were not opened in a fab's DFM tool and no fab accepted them.
- Hash reproducibility on another machine depends on FreeRouting 2.4.1, the Java build and KiCad
  10.0.7. Two exports at this revision gave byte-identical `.kicad_pcb` and `.kicad_sch` here.
