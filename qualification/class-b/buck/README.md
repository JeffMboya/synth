# 24 V in, 3.3 V out buck with input parts (PTC, reverse-block diode, TVS) (SYN-67)

Status: designed, placed, routed, DRC and ERC counted, Gerber and drill files generated. Not fabricated,
not powered, no hardware test. All new parts are UNVERIFIED (imported, no reviewer). No reviewer has
signed off this board, so the fab package reports `release_ready: false` by design.
Intended for 24 V +10% in and 1 A out. 1 A at 70 C is not shown (see Limits, thermal). Not an overvoltage-safe or 2 A design.

## Purpose

Ticket #67 under #38 (Class B qualification): a switching-regulator board with input parts, feedback, UVLO
and a power LED, plus a negative variant that the feedback-safety rule (E-SYNTH-POWER-011, PR #56) must flag.

## What

- `buck.synth`, board-mm sidecar `buck.placement.layout.toml`, negative `buck_neg_fb_sw.synth`, hashes in `SHA256SUMS`, matrix and `#38` traceability in `QUALIFICATION.md`, evidence in `kicad/`, `reports/`, `images/`.
- `kicad/`: the KiCad project, BOM, pick-and-place, `gerbers/` (22 Gerbers plus the job file, 4 copper layers) and `drill/` (PTH and NPTH). `reports/`: `release.json` (source/DRC gate) and `release-fab.json` (fab package, untrusted), DRC, ERC, routing and connectivity.
- Chain: J1 (JST XH) -> F1 PTC -> D1 SS36 Schottky (reverse block) -> VIN node.
  VIN node: D2 SMBJ26A TVS, C1 47 uF bulk, C2 10 uF, C3 100 nF (on the U1 VIN pin), U1 TPS54202, UVLO divider.
- U1 -> L1 10 uH -> C5, C6 (2x 22 uF, 25 V X7R) -> J2. C4 100 nF boot. D3 green LED + R_LED 470.
- Feedback: R_FB_T 100k from the output node (L1 pad 2 side), R_FB_B 22.1k, C7 56 pF across R_FB_T.
  Vout = 0.596 x (1 + 100k/22.1k) = 3.293 V typical, 3.21 to 3.376 V over Vref 0.581 to 0.611 V (ideal resistors).
- Negative file: identical except R_FB_T and C7 hang from SW. Besides E-SYNTH-POWER-011 it emits 13 of the good board's 14 warnings.
- Board: 58.0 x 32.605 mm (1891 mm2) with the 1.5 mm edge rim from #272. The part positions come from the
  sidecar, so the placer's margin no longer sets the outline; the SMC diode, the 8 x 10 mm electrolytic and
  the six-part input chain set it instead. 4 layers, GND pours on all layers. Power tracks 0.5 mm, signal
  0.127 mm. J1 sits 1.5 mm from the top and left edges and J2 1.6 mm from the right edge, which is the rim,
  not a chosen connector position.

## UVLO (TPS54202 SLVSD26C section 6.3.5, Eq 1 and 2, Figure 6-1)

Datasheet values: Ip 0.7 uA, Ih 1.55 uA, VENrising 1.21 V (max 1.28), VENfalling 1.19 V (min 1.1). R4 = R_UV_T = 1.5M, R5 = R_UV_B = 100k.
- Start = VENr + R4 x (VENr/R5 - Ip) = 1.21 + 1.5M x (12.10 uA - 0.70 uA) = 18.31 V.
- Stop = VENf + R4 x (VENf/R5 - Ip - Ih) = 1.19 + 1.5M x (11.90 - 0.70 - 1.55 uA) = 15.67 V.
- Check: Eq 1 with these thresholds returns R4 = 1.5M and Eq 2 returns R5 = 100k.
- 1% resistors, typical thresholds: start 17.96 to 18.67 V, stop 15.35 to 15.99 V.
- 1% resistors plus the datasheet EN threshold limits (VENr up to 1.28, VENf down to 1.1; Ip and Ih typical, no tolerance given):
  start 17.96 to 19.81 V, stop 13.93 to 15.99 V.
- EN at 28 V in is 1.75 V (EN recommended max 5.5 V).

## Parts and datasheets (read with curl + pdftotext, 2026-10-11)

| Part | Datasheet | Revision | Values taken | Conflict or gap |
| --- | --- | --- | --- | --- |
| U1 TI TPS54202 (SOT-23-6 DDC) | https://www.ti.com/lit/ds/symlink/tps54202.pdf | SLVSD26C, Feb 2026 | VIN 4.5-28 V recommended, abs max 30 V; SW abs max 30 V; EN abs 7 V, rec 5.5 V; Vref 0.581/0.596/0.611 V; 500 kHz (390-630); 2 A; HS limit 2.5/3.2/3.9 A; pins 1 GND, 2 SW, 3 VIN, 4 FB, 5 EN, 6 BOOT; Table 7-2 at 3.3 V: L 10 uH, Cout 44 uF, R2 100k, R3 22.1k, C6 56 pF (based on VIN 28 V); CIN over 10 uF ceramic; boot 0.1 uF; RthetaJA 118.6 C/W (57.2 on the TI EVM); no power-good pin | KiCad symbol is TPS54202DDC. Pin numbers equal the TI table. R2/R3 in TI text are R_FB_T/R_FB_B here |
| L1 Bourns SRN6045TA-100M | https://www.bourns.com/docs/Product-Datasheets/SRN6045TA.pdf | REV. 06/24 (footer) | 10 uH +-20%, DCR 52 mOhm typ (+-20%), Irms 3.20 A typ, Isat 4.60 A typ (L -30%), 6.0 x 6.0 mm, land 6.5 x 5.1 mm | Registry has no Isat field; Isat is in the description |
| D1 Vishay SS36 | https://www.vishay.com/docs/88751/ss32.pdf | 23-Apr-2020 (doc 88751) | VRRM 60 V, 3 A, IFSM 100 A, VF 0.75 V max at 3 A, IR 0.5 mA at 25 C and 10 mA at 100 C, SMC | KiCad `Diode:SS36` defaults to SMA, this sheet is SMC (SMC used). VF at low current not read |
| D2 Vishay SMBJ26A | https://www.vishay.com/docs/88392/smbj.pdf | 09-Jan-2024 (doc 88392) | 600 W 10/1000 us, VWM 26 V, VBR 28.9-31.9 V at 1 mA, VC 42.1 V at 14.3 A, VBR temperature coefficient 0.097 %/C, SMB | No KiCad SMBJ26A symbol; Device:D_Zener used. Registry `max_voltage_v` 26 is VWM |
| F1 Bourns MF-MSMF110/33X | https://www.bourns.com/docs/Product-Datasheets/mfmsmf.pdf | REV. BE, 09/26 (footer) | Vmax 33 V, Imax 20 A, Ihold 1.1 A at 23 C, 0.93 A at 40 C, 0.73 A at 60 C, 0.63 A at 70 C, 0.50 A at 85 C, Itrip 2.2 A, R 0.06-0.20 Ohm | none found. Registry `max_current_ma` 1100 is Ihold at 23 C |
| J1, J2 JST B2B-XH-A | https://www.jst-mfg.com/product/pdf/eng/eXH.pdf | none printed | 2.5 mm pitch, 3 A, 250 V | Existing registry `jst_xh_2pin` says 5 V / 100 mA; conflicts with this sheet, left untouched, new part added |
| D3 Lite-On LTST-C190KGKT | https://optoelectronics.liteon.com/upload/download/DS22-2000-074/LTST-C190KGKT.PDF | not checked | none (existing registry part) | Site unreachable (curl code 000) |
| C1 47 uF | none | | generic 8 x 10 mm SMD electrolytic, 50 V class | no MPN, ESR and ripple rating unknown |
| C2, C5, C6 (1206), C3, C4, C7 (0603), resistors | none | | generic registry parts; voltage and dielectric are set per instance in `buck.synth` (C2 50 V X7R, C3 50 V X7R, C4 16 V X7R, C5/C6 25 V X7R, C7 50 V C0G, resistors 1%) | no MPN. Murata and TDK pages returned 404/403/500: no DC-bias data |

## Limits and honesty notes

- Overvoltage: NEEDS EXPERT. TPS54202 abs max VIN is 30 V (recommended 28 V). SMBJ26A VBR is 28.9 to 31.9 V and rises 0.097 %/C
  (about 30.2 V minimum at 70 C, by calculation from the datasheet coefficient), VC is 42.1 V at 14.3 A. The TVS therefore clamps surge energy only; it gives no protection against a
  sustained 28 to 32 V rail, and a rated surge reaches 42 V at the TVS. SMBJ24A (VC 38.9 V) is also above 30 V. The board tolerates 24 V +10% (26.4 V, 0.4 V above the TVS standoff). A real
  24 V industrial rail needs a series OVP stage or surge stopper in front of U1, or a 40 to 60 V rated buck. Not designed here. The surge waveform of an application is not defined.
- PTC and rating: output current is rated 1 A, not 2 A. Input current = Pout / 0.85 / Vin. The 85% efficiency is an assumption, not a datasheet value.
  At 1 A out and the UVLO stop voltage 15.67 V: 3.3 / 0.85 / 15.67 = 0.248 A. F1 Ihold is 0.63 A at 70 C (2.5x margin) and 0.50 A at 85 C (2.0x).
  At the worst-case UVLO stop of 13.93 V: 3.3 / 0.85 / 13.93 = 0.279 A, margin 2.26x at 70 C and 1.79x at 85 C. SS36 drop and PTC self-heating are not included.
  At 2 A out the same input current is 0.50 A, no margin at 85 C. Ambient ceiling from the PTC alone: 70 C.
- Thermal: UNVERIFIED, and this estimate does not show 1 A at 70 C. Total loss at 1 A with the assumed 85% is 3.3 / 0.85 - 3.3 = 0.582 W (estimate). L1 DCR loss is 1 A^2 x 0.052 Ohm = 0.052 W,
  leaving 0.53 W in U1 if all of it is dissipated there. At RthetaJA 118.6 C/W that is +62.9 C: junction about 133 C at 70 C ambient, above the 125 C recommended limit; 125 C is reached at about 62 C ambient,
  below the PTC's 70 C ceiling. With the EVM figure 57.2 C/W it is +30.3 C, about 100 C at 70 C. Which figure applies depends on copper, which Synth does not model.
  The 70 C ceiling is the PTC ceiling only, not a thermal clearance for U1.
- Output capacitors: Table 7-2 asks for 44 uF at 3.3 V (2 x 22 uF, 25 V X7R 1206 on a generic registry part, voltage set per instance). Crossover fo = 3.95 / (Vout x COUT) = 27.3 kHz at 44 uF, staying under 40 kHz needs
  at least 30 uF effective (68% of 44 uF). DC-bias derating at 3.3 V is unknown. Crossover and stability are UNVERIFIED.
- Input capacitor: TI asks for over 10 uF ceramic. C2 is 10 uF X7R 50 V; its effective value at 24 V is unknown and may be well under 10 uF. C1 47 uF electrolytic adds bulk with unknown ESR. UNVERIFIED.
- Reverse polarity: Schottky chosen over a P-FET. SS36 VRRM 60 V against 26 V. Drop 0.75 V max at 3 A, so under 0.19 W at 0.248 A (bound; VF at low current not read). A P-FET at 24 V needs a gate clamp (typical Vgs max is +-20 V); no P-FET datasheet was read.
- Inductor: ripple at 24 V is 0.57 A pp; at 1 A out the peak is 1.29 A against Isat 4.6 A.

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
Registry import: `synth part import-kicad Regulator_Switching:TPS54202DDC --id tps54202 --footprint Package_TO_SOT_SMD:SOT-23-6`;
likewise `Diode:SS36` (Diode_SMD:D_SMC), `Device:D_Zener` (SMBJ26A, Diode_SMD:D_SMB), `Device:Polyfuse`, `Device:L`, `Device:C_Polarized`, `Connector_Generic:Conn_01x02`.
The files were then edited by hand (names, MPN, dimensions, datasheet URL, `switch_node` on SW and `feedback` on FB as for `mp2307_buck`).

## Measured results

The export routes with FreeRouting 2.4.1; the routed board is written as `out/buck/buck.kicad_pcb` when Synth's gate passes (state `routed`). Numbers below are from that file, on Synth `main` `1eac8a1`, 2026-10-24.

| Number | Value | Command |
| --- | --- | --- |
| validate, good board | exit 0, 0 errors, 14 warnings (8 part-unverified, 3 E-SYNTH-SCHEM-003 (schematic-sheet distance of a cap to U1, not PCB distance), 1 E-SYNTH-SCHEM-005 junction fan-out, 1 anomaly, 1 supply) | `synth validate $B/buck.synth 2>&1 \| grep -c '^warning'` and `grep -c '^error'` |
| validate, negative | exit 1, 1 error (E-SYNTH-POWER-011), 13 warnings | same on `buck_neg_fb_sw.synth` |
| routing | `routed`, 10 nets connected, 0 open, 51 segments, 15 vias, 155.16 mm | `synth route $B/buck.synth` (JSON `state`, `statistics`); cross-check `grep -c '^\t(segment' out/buck/buck.kicad_pcb`, `grep -c '^\t(via' ...` |
| Synth routing check | kicad_drc 0 errors, 0 unconnected, 20 warnings; `fabrication_ready` true (routing check only) | `synth route` JSON `validation` |
| Release gate | `release_ready` true, `production_status` production, 0 blocking signal-integrity findings; with `--gerbers --drill` the same board reports `release_ready` false and `untrusted` because 7 parts have no reviewer | `reports/release.json`, `reports/release-fab.json` |
| DRC | 0 errors, 0 unconnected, 20 warnings, all lib_footprint_issues | `kicad-cli pcb drc ... -o reports/buck.kicad-drc.json`, then `python3 -I -c "import json,collections as c;d=json.load(open('$B/reports/buck.kicad-drc.json'));print(c.Counter((v['severity'],v['type']) for v in d['violations']),len(d['unconnected_items']))"` |
| ERC | 0 errors, 54 warnings: 34 lib_symbol_issues, 20 footprint_link_issues (libraries not in the KiCad config) | `kicad-cli sch erc ... -o reports/buck.kicad-erc.json`, same one-liner over `sheets[].violations` |
| Schematic netlist | 10 nets, equal to the source's 5 nets and 5 `connect`s node for node | `kicad-cli sch export netlist --format kicadsexpr -o /tmp/buck.net $B/kicad/buck.kicad_sch` |
| Outline | 58.0 x 32.605 mm (x 0.3 to 58.3, y 1.125 to 33.73) | `grep -A3 gr_line $B/kicad/buck.kicad_pcb \| grep start` |
| Manufacturing exports | 22 Gerbers plus the job file (4 copper layers), PTH 19 holes (15 x 0.3 mm vias, 4 x 1.0 mm pins), NPTH none | `ls $B/kicad/gerbers $B/kicad/drill`; `grep -c '^X' $B/kicad/drill/buck-PTH.drl` |
| Hashes | pcb eda864e33ca1..., sch 19a2f15154fa..., three exports identical | `cd $B && sha256sum -c SHA256SUMS` (full values in `SHA256SUMS` and `QUALIFICATION.md`) |

Pad-edge gaps in the placed, routed board, from `pad_gaps.py` (pcbnew pad bounding boxes, edge to edge):

| Pair | Gap mm |
| --- | --- |
| C3 (100 nF) to U1 VIN | 1.18 |
| C2 (10 uF) to U1 VIN | 4.34 |
| C3 / C2 ground pad to U1 GND | 3.08 / 5.15 |
| C4 to U1 BOOT / SW | 2.77 / 3.75 |
| L1 pad 1 to U1 SW | 1.95 |
| C5, C6 to L1 pad 2 | 1.20 |
| D2 TVS to U1 VIN | 9.43 |
| R_FB_T / R_FB_B to U1 FB | 1.98 / 3.73 |
| C7 to R_FB_T | 0.75 |
| R_FB_B ground pad to U1 GND | 4.25 |
| R_FB_T FB pad to L1 pad 1 (SW) | 7.53 |

The sidecar holds the placed positions. All 20 parts in the routed file match it within 0.01 mm; only the JST connectors differ, by their fixed anchor offset of (-1.25, -0.525) mm (QUALIFICATION.md A13).
`pad_gaps.py` on the final routed file is the reference for gaps, not the sidecar coordinates.

## Negative case: feedback from the switch node

`synth validate $B/buck_neg_fb_sw.synth` exits 1 (JSON, schema 1.3):
```
error: [E-SYNTH-POWER-011] regulator feedback is taken from the switching node
  found:    feedback pin `U1.fb` (group "BUCK") is joined through resistor `R_FB_T` to switch-node pin `U1.sw` (group "BUCK")
  expected: the feedback pin to sense the regulated output, not the switching node
```
`synth export-kicad` stops with "Synth ERC validation failed; use --force to export anyway" (exit 2). The good board does not raise it. The rule follows one resistor only.

## Qualification matrix

The matrix (A: checked by tools, B: needs an expert) is in [`QUALIFICATION.md`](QUALIFICATION.md),
together with the `#38` checklist traceability for this board and the row it declares for
`qualification/matrix.toml` when that corpus lands on `main`. Evidence files: `kicad/`,
`reports/`, `images/`.

## Not true yet

- Not fabricable on this evidence: parts UNVERIFIED, overvoltage case open, nothing built. The fab
  package says so itself (`release_ready: false`, `production_status: untrusted`).
- Layout: `+3V3` is 50.2 mm of copper split over F.Cu and B.Cu and crosses under the buck core on
  B.Cu; FB is 8.29 mm on F.Cu only. C3 sits on the VIN pin and C7 against R_FB_T, but R_FB_B's ground
  is 4.25 mm from U1 pin 1, so it reaches it through a via to the plane. The one layer change with no
  ground return via within 0.64 mm is `+3V3` at (35.97, 14.43) mm, reported as `W-SYNTH-DRC-016`
  (the report calls it "net 3", which is Synth's numbering; in the `.kicad_pcb` net 3 is FB).
- Schematic (rendered and read): readable, not tidy. `tools/schematic_quality.py` counts 1
  overlapping text pair, 3 texts with a wire through them, 5 wire crossings and 803.9 mm of wire;
  mean passive-to-pin distance is 83.8 mm (max 148.3). The `+3V3` wire to C5 runs past the BOOT pin
  stub without a junction and at print size reads like a boot capacitor. The netlist is correct:
  BOOT is U1 pin 6 and C4 pin 1, and both 22 uF output capacitors are on `+3V3`. No schematic sidecar
  is used (default layout).
- There are no mounting holes, so the board cannot be fixed down as it stands.
- Routing used FreeRouting defaults, no length or coupling constraints.

## Not verified

- Nothing powered, measured or thermally tested. Efficiency (85%) and the 0.58 W loss are assumptions.
- The MCP placement review (`synth_describe_placement` with this sidecar) returns score 100,
  `requires_revision` false and no findings, but it reports a 63.0 x 37.6 mm outline because it does
  not include #272's 1.5 mm rim. The verdict is used, the outline figure is not.
- No reviewer. Availability, LCSC numbers and assembly not checked.
- LED datasheet unreachable. J1/J2, C1, and all generic passives have no MPN or manufacturer sheet.
- PTC behaviour with the TVS in a surge, and inrush into C1 and the output caps, are not analysed.
- The Gerbers and drill files were not opened in a fab's DFM tool and no fab accepted them.
- Hash reproducibility on another machine depends on FreeRouting 2.4.1, the Java build and KiCad
  10.0.7. Three exports at this revision gave byte-identical `.kicad_pcb` and `.kicad_sch` here.
