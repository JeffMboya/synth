# Qualification matrix: 24 V to 3.3 V buck board (SYN-67)

**Status: requires expert review. This is not a completed qualification result.**
Nothing here was fabricated or powered. Section A lists what Synth or KiCad checked and the
evidence. Section B lists what no tool here proves and an SI/PI/RF or power expert has to sign
off. No expert has reviewed this board.

This is one board in Class B (#38) and it carries **case 1 only**, the switching regulator:
24 V to 3.3 V with input and output filtering, feedback, power-good and thermal copper. The analog
front end, the USB/Ethernet peripheral and the other negative cases belong to other boards.
Section C maps every `#38` checklist item to a row here, including the ones that do not apply.

## Reproduce

Run from the repository root on Synth `main` at `1eac8a1` or later (it includes the filled-pour
delivery #114, the label placement #144 and the 1.5 mm edge rim #272). Use an empty
`XDG_DATA_HOME` so a user registry cannot shadow parts.

```
export XDG_DATA_HOME=$(mktemp -d)
scripts/setup-routing-engines.sh --freerouting-only
B=qualification/class-b/buck
synth validate $B/buck.synth
synth validate $B/buck_neg_fb_sw.synth                     # must exit 1 with E-SYNTH-POWER-011
synth export-kicad $B/buck.synth --out out/buck \
  --allow-unverified-parts --validate-erc --router free-routing --place-route-iterate \
  --pnp --gerbers --drill                                  # the fab package, reports/release-fab.json
synth export-kicad $B/buck.synth --out out/buck-nofab \
  --allow-unverified-parts --validate-erc --router free-routing --place-route-iterate   # source/DRC gate, reports/release.json
kicad-cli pcb drc --severity-all --refill-zones --format json \
  -o $B/reports/buck.kicad-drc.json $B/kicad/buck.kicad_pcb
kicad-cli sch erc --severity-all --format json \
  -o $B/reports/buck.kicad-erc.json $B/kicad/buck.kicad_sch
kicad-cli sch export netlist --format kicadsexpr -o /tmp/buck.net $B/kicad/buck.kicad_sch
(cd $B && sha256sum -c SHA256SUMS)
```

Tool versions used: KiCad 10.0.7, FreeRouting 2.4.1 on OpenJDK 25.0.4, Synth 0.1.1 built from
upstream `main` `c5caf90` merged into this branch, on 2026-10-28. The placement is Synth's own
(`--place-route-iterate`); there is no placement sidecar, and J1 and J2 carry hard `edge: left` and
`edge: right` hints in `buck.synth`. Both exports (with and without fab flags) produced the same
`.kicad_pcb`. Run from the repository root, with `B` relative as above, the JSON reports hold
repository-relative paths; `out/` is a scratch directory and is not committed. The reports do not
record a source commit (`source_revision` is null in `buck.routing.json`): the source is identified
by the `buck.synth` hash below, the Synth commit above and the registry part files in this branch. A
different placement or router version can give different copper.

## Artifact hashes

`SHA256SUMS` lists every design artifact (source, KiCad project, Gerbers, drill files,
BOM, pick-and-place, pictures, routing, DRC and ERC reports). It leaves out `README.md`,
`QUALIFICATION.md` and itself. The JSON reports had the generating machine's directory prefix removed (paths are repository-relative); they do not record a source commit, so the source is identified by the `buck.synth` hash below and the Synth `main` commit in the tool versions above. The key ones:

| Artifact | SHA-256 |
| --- | --- |
| `buck.synth` | `acc80b36b762bb2d1fa85d806199841be003603154bda8aed2473e0eb12244fc` |
| `buck_neg_fb_sw.synth` | `e6a9e94ce4d01a3b5a58b941da073648a32c1aa9c48bb83ed2bddbec0cd17299` |
| `kicad/buck.kicad_pcb` | `d0ce5a48a2c64fc86a0d9169e9c5ce880005d497fb0d99108e1fa935d32b79ad` |
| `kicad/buck.kicad_sch` | `f2121709833139093865113a3728f8553f914c877cc2a193257deaa8fc7eec95` |
| `kicad/bom.csv` | `48fbaee99074ac762fbab81f55a615b280a0a318b45ee858469622617910239e` |
| `reports/release.json` | `24a1543bf3390c6d6dc85a9bab2fd8655ca4ad83eb4f26633078c55543324f56` |
| `reports/release-fab.json` | `8abe1200612d3c29f1c81bfe860afd62289fe67cdfb5166751114f811de976a9` |
| `reports/buck.kicad-drc.json` | `79fde56e30f7a47a6ba8b2ab2e6dc1bb63b43fa20e554d355116a6a37ae48057` |
| `reports/buck.kicad-erc.json` | `15789b5f6448f24711bdec684fe67f11906e04f25eb2dc08d7830d1210085110` |

Datasheet values used for the part values are in `README.md` ("Parts and datasheets"). The
datasheets are not committed, so the figures taken from them cannot be checked from this
repository.

## A. Checked by tools

| # | Claim | Evidence | Result |
| --- | --- | --- | --- |
| A1 | The source parses, resolves and lowers | `synth validate buck.synth` | Exit 0, 0 errors, 22 warnings: 9 `W-SYNTH-PART-UNVERIFIED`, 3 `E-SYNTH-SCHEM-003`, 2 `E-SYNTH-SCHEM-004`, 4 `E-SYNTH-SCHEM-005`, 1 `W-SYNTH-ANOMALY-001`, 3 `W-SYNTH-SUPPLY-002` |
| A2 | Regulator feedback is not taken from the switch node | `E-SYNTH-POWER-011` on `buck_neg_fb_sw.synth` | The twin exits 1 with the error and 20 warnings; the good board does not raise it. The rule follows one series resistor only, so a longer feedback path is not checked |
| A3 | Every net is connected end to end | `reports/buck.connectivity.json`, `reports/buck.routing.json` | 13 nets, 0 open, 0 unconnected pads, 103 segments, 24 vias in the routing statistics (30 vias in the PCB file: 22 on GND), 341.22 mm of copper, no via-in-pad |
| A4 | No KiCad DRC rule violation on the delivered board | `kicad-cli pcb drc --refill-zones`, `reports/buck.kicad-drc.json` | 0 errors, 0 unconnected, 0 warnings |
| A5 | The route passes Synth's source/DRC gate | `reports/release.json` (`synth export-kicad`, no fab flags) | `release_ready` true, `production_status` production, state `routed`, 0 blocking signal-integrity findings |
| A6 | The package is not a fabrication release | `reports/release-fab.json` (`--gerbers --drill`) | `release_ready` **false**, `production_status` `untrusted`, override `unverified_parts` naming 1 part (`ltst_c190kgkt`, the LED, whose datasheet could not be reached). This is the correct answer until that part has a reviewer |
| A7 | Schematic electrical rules | `kicad-cli sch erc`, `reports/buck.kicad-erc.json` | 0 errors, 1 warning: 1 `lib_symbol_mismatch` for SS36. KiCad's `Diode:SS36` is declared with `(extends "SB120")`; Synth's exporter writes the symbol with the parent's graphics copied in, so KiCad sees a different copy. The pins and graphics are the same. It is an exporter behaviour that applies to any library symbol that extends another, not a property of this board, and it is not changed in this PR |
| A8 | The schematic netlist matches the source | `kicad-cli sch export netlist` | 13 nets, node for node the source's 8 `net` blocks plus the 5 names its 9 `connect` statements create: GND 16 nodes, `+3V3` 10, VIN 7, FB 4, PG 4, EN 3, SW 3, BOOT 2, U1_VCC 2, VIN_FUSED 2, VIN_RAW 2, LED_PWR 2, LED_PG 2 |
| A9 | A layer change without a nearby ground return via is reported | `W-SYNTH-DRC-016` in both release files | 8 findings, all non-blocking warnings, in `release.json` and `release-fab.json`: layer changes on nets 0, 8 and 11 with no ground via within 0.64 mm. They are reported, not fixed. `synth capability list` still declares return-path analysis unsupported, so this is a proximity test, not a return-path analysis |
| A10 | Manufacturing exports exist and are complete | `kicad/gerbers/` (22 Gerbers plus the `.gbrjob`), `kicad/drill/` | 4 copper layers (F, In1, In2, B), mask, paste, silkscreen and Edge.Cuts present, Edge.Cuts traces the 60.75 x 31.855 mm outline; PTH drill 36 holes (30 x 0.3 mm vias, 6 connector pins at 0.95/1.0 mm), NPTH 2 x 2.5 mm mounting holes. Not fabricated |
| A11 | The exported project is readable by KiCad | KiCad 10.0.7 headless | `pcb drc`, `sch erc`, `sch export netlist`, `sch export pdf`, `pcb render` and `pcb export pos` all load the committed files and succeed. Not opened in the KiCad GUI |
| A12 | Ground pours are stored filled | `grep -c filled_polygon kicad/buck.kicad_pcb` | 6 filled polygons for the 4 GND zones: 3 on F.Cu (the pour is cut into 3 islands by the routing) and 1 each on In1, In2 and B.Cu; the other 2 zones are rule areas and carry no fill. `kicad-cli pcb drc` reports 0 unconnected items, so every F.Cu island reaches the rest of GND through vias or the other layers. KiCad opens showing the connectivity the gate validated |
| A13 | The placement is Synth's own and the edge hints held | `kicad/buck.kicad_pcb` | No sidecar: all 27 parts were placed by Synth's placer, chosen by `--place-route-iterate` among 7 routed candidates (candidate 0, 103 segments, 24 vias). J1 sits flush with the left edge and J2 with the right edge, as their hard `edge` hints in `buck.synth` ask; the MCP placement review (A14) reports no connector warning |
| A14 | The placement passes Synth's structural review | MCP `synth_describe_placement` on `buck.synth`, run on the final 27-part source with the `synth mcp` server built from the same commit | score 100, `requires_revision` false, no findings, no functional warnings, no dense regions. The review covers all 27 parts including D4 and R_PGLED. Its reported board size (60.75 x 36.855 mm) includes the review's own margin and is not the exported outline; the exported outline is 60.75 x 31.855 mm. This is a structural review, not a judgement of decoupling distance (B5, B12) |
| A15 | Power-good reaches the connector | `kicad-cli sch export netlist` (A8), `E-SYNTH-POWER-011` rule set | Net `PG` has exactly four nodes: `U1.pg` (pin 4, open drain), `R_PG.p1` (100k to `+3V3`), `J2.p3` and `D4.cathode` (A17). The connection and the pull-up are proven; the behaviour is B3 |
| A16 | Thermal copper exists under the converter | `kicad/buck.kicad_pcb`, `reports/buck.connectivity.json` | U1 pin 9 is a 2.41 x 3.1 mm exposed pad in the `GND` net, soldered to the F.Cu pour; the board has 22 GND vias, 3 within 4 mm of the pad edge (0.55, 3.00 and 3.55 mm). The copper is proven; the temperature is B2 |
| A17 | The power-good fault LED is wired as designed | `kicad-cli sch export netlist` (A8), `kicad/bom.csv`, `kicad/buck.kicad_pcb` | `D4` (`ltst_c190kgkt`, value "PG FAULT") has its anode on net `LED_PG` and its cathode on `PG`; `R_PGLED` (470 R) joins `+3V3` to `LED_PG`. Both are in the BOM; D4 is at (39.5, 15.0) mm and R_PGLED at (39.5, 13.5) mm. The LED current is a hand figure: (3.3 - 2.1) / 470 = 2.55 mA, taking Vf 2.1 V from the part entry's 1.9 to 2.4 V range (the LED datasheet was not read, B8), with `PG` held low by 76 to 150 ohm RDSON (TI SNVSB99C, Electrical Characteristics), so about 0.4 V. "PG FAULT" is the value field on F.Fab and is not printed on the silkscreen; only the reference D4 is. That the LED lights on a fault is B3 |

## B. Not proven: needs expert review

| # | Claim | Why it is not proven | Status |
| --- | --- | --- | --- |
| B1 | Survives a rated surge or a sustained 28 to 32 V rail | The converter is rated 3.8-36 V recommended and 38 V absolute maximum, so a sustained 28-32 V rail is inside its rating. A surge is not covered: SMBJ30A clamps at 48.4 V (12.4 A), above the 38 V absolute maximum, so the TVS is an ESD/transient clamp and not an OVP. A rated surge needs a 60 V front end or a series surge stopper, not designed here. Datasheet numbers only | Sustained rail in spec; surge unprotected |
| B2 | Thermal copper, thermal vias, thermal relief on pours and junction temperature | `synth capability list` declares `thermal_relief_analysis` unsupported: zone relief gap and spoke width are fixed export defaults and are not analysed. There are no vias in the pad, because the board's fabrication floor is a 0.3 mm drill and the KiCad HSOP-8 thermal-via footprint ships 0.2 mm drills. Estimate from TI Figure 9-3 (about 38 C/W at 20-24 cm2 of copper per layer on a four-layer board) and the assumed 85% efficiency: junction about 92 C at 1 A and 70 C ambient, about 114 C at 2 A. Not a simulation, not a measurement | Copper present; temperature unverified |
| B3 | Power-good behaviour | The connection and the LED wiring are proven (A15, A17); the LED is an indicator, not a check on the thresholds. The part's datasheet gives 93/95/105/107% thresholds, a glitch filter and a 4 ms start-up delay, but nothing declares those limits and `synth capability list` marks `power_sequencing_and_power_good` unsupported, so no stage checks them. Nothing was measured | Unsupported by Synth; unverified by an expert |
| B4 | Input and output filtering, loop stability, capacitor derating | Table 9-2 values are used (10 uF + 220 nF in, 3x 22 uF out, 6.8 uH, internal compensation). DC-bias derating of the input and output ceramics is unknown (no vendor data). C1 electrolytic ESR is not given in the datasheet text read; its ripple rating is 120 mA at 120 Hz. Crossover, phase margin and load-transient response were not computed or measured | Requires expert review |
| B5 | Switch-node loop area, return paths, EMI and ripple | Only pad-edge gaps were measured (README). No loop area, inductance, ripple or EMI result. In Synth's own placement the input cap C3 is 19.80 mm from the VIN pad and 21.22 mm of ground path from the GND pad, and L1 pad 1 is 3.33 mm from the SW pad; the input loop is large and a reviewer is likely to reject it. Not analysed further | Requires expert review |
| B6 | Analog placement separation and grounding | `synth capability list` declares `split_ground_planes` unsupported: each ground net gets one unsplit zone. The FB divider R_FB_T is 3.39 mm from U1 pin 5 and R_FB_B 13.77 mm; R_FB_B's ground is 5.86 mm from U1 pin 1 and reaches it through the plane. R_FB_T's FB pad is 8.98 mm from L1 pad 1 (SW). Whether that is acceptable is for a reviewer | Requires expert review |
| B7 | Reverse-polarity block, PTC margin, UVLO thresholds, output voltage | All by hand calculation from datasheet numbers (README): PTC 0.162 A at 24 V and 0.248 A at the UVLO stop voltage against Ihold 0.63 A at 70 C; UVLO 18.09 / 16.62 V; Vout 3.315 V. No bench measurement | Requires expert review |
| B8 | Part pins and footprints against the manufacturer drawings | `synth registry qualify --part <id>` (output in `reports/registry-qualify.txt`): all 8 new parts are reviewed and qualify (`lmr33640`, `ss36`, `smbj30a`, `l_bourns_srn6045ta_6r8m`, `mf_msmf110_33x`, `c_electrolytic_smd_8x10`, `jst_b2b_xh_a_2pin`, `jst_b4b_xh_a_4pin`; reviewer Steve Munene, 2026-10-28), each compared with its manufacturer datasheet. Five were compared for pin function, polarity and land pattern against the stock KiCad footprint; the KiCad pads differ from the datasheet land patterns in places (the SS36 and SMBJ30A pads sit wider apart than Vishay's reference, the SRN6045TA pads are 0.55 mm longer inward, the MF-MSMF110/33X pads are 1.125 mm long against Bourns' 1.68 mm). The electrolytic (Honor Elec RVT1H470M0810, JLCPCB C2895739) was checked for value, voltage, 8 x 10.2 mm size and part number; its datasheet text does not state the polarity marking or a land pattern, so pin 1 as + follows the KiCad `CP_Elec_8x10` convention and is not verified. `mf_msmf110_33x` also reports `E-SYNTH-QUAL-016` as a review finding (its footprint names a 3D model that is not installed). The two JST headers (JST eXH catalogue, through-hole top-entry type) were checked for pitch (2.5 mm), hole diameter (2 circuits: 1.0 mm, 3 or more: 0.9 to 1.0 mm; KiCad drills 1.0 and 0.95 mm), pad diameter (1.7 mm), and the 3 A / 250 V rating; JST gives hole sizes only, so the pad shape is the stock KiCad one and is not compared. Pin-1 polarity is not stated in the catalogue. LED datasheet unreachable; the generic passives have no MPN | Requires expert review |
| B9 | Compactness | Outline 60.75 x 31.855 mm (1935 mm2), against 70.99 x 31.125 mm (2210 mm2) for the earlier hand layout and 58.0 x 32.6 mm (1891 mm2) for the SOT-23-6 version. The hand layout is no longer reproducible: on current main the placer moves parts the sidecar marks hard, and that export fails DRC with courtyard overlaps and 2 open nets. Synth's own placement is used instead; it is smaller, routes clean and keeps J1 and J2 on the edges, but puts the input capacitors far from U1 (B5, B12). Earlier experiments are not repeated here: `near` hints (78.85 x 34.09 mm) and a sidecar that pins L1 and the output caps (67.5 mm wide) | Not a defect; not compared |
| B10 | Keepouts and shielding | The language has keepouts; none was declared under the switch node or the inductor in this design, and none was checked | Unsupported here |
| B11 | Power sequencing, soft-start and inrush | The converter has reference-based internal soft start (no SS pin) and a 4 ms power-good delay. Inrush into C1 and the output capacitors, and the sequencing of the PG signal against the output, were not analysed or measured | Unverified |
| B12 | Decoupling placement | Pad-edge gaps in the table in `README.md` (`pad_gaps.py`): C4 6.49 mm from BOOT, C8 4.73 mm from VCC, C7 8.80 mm from R_FB_T, C3 19.80 mm from VIN, C5/C6/C9 14.35 / 12.85 / 19.33 mm from L1 pad 2. These are poor and are not defended. The placer has no distance constraint for them (`near` hints are a soft 8 mm preference). Loop inductance was not computed | Unverified |
| B13 | Stackup and impedance assumptions | The board uses the default 4-layer stackup with inner ground planes. No stackup block was declared, no controlled-impedance net exists, and the capability descriptor marks inner-layer impedance unsupported | Not applicable to a single-ended power board |
| B14 | What the Gerbers, drill files and BOM say about orderability | The files in `kicad/` were generated and hashed (A10), but nothing was fabricated, no fab accepted them, no part carries an LCSC number, and every generic passive has no MPN. Availability not checked | Not done |
| B15 | Schematic correctness and readability | ERC has 0 errors (A7) and the netlist matches the source (A8). The sheet was rendered and read: `tools/schematic_quality.py` reports 1 overlapping text pair, 10 texts with a wire through them, 0 wire crossings, 1371.6 mm of wire, and a mean passive-to-pin distance of 62.6 mm (max 102.9). The power-good and thermal groups read correctly | Readable, not tidy |
| B16 | The other negative cases from #38 (missing pair constraints, invalid stackup assumptions) | This board has no differential pair and no stackup claim, so those cases do not apply here; the USB board (#103) carries them. Only the unsafe-feedback case is built here, and the rule follows one series resistor only | Not applicable here |
| B17 | Comparison with a human reference design | Only TI's copper-area curve and efficiency curves were used. No reference layout was compared (#68) | Not done |
| B18 | Function on real hardware | Nothing was powered, programmed or measured. Two M2.5 mounting holes exist, so the board can be fixed down | Not done |
| B19 | Independent mixed-signal / RF sign-off (#38) | No reviewer | Not done |

Rows A2, A9 and B1 to B8 are the ones that decide whether this board meets a Class B bar (#38
treats missing SI/PI/RF evidence as a failure). They are open or bounded, not closed.

## C. #38 traceability for case 1

Every line of `#38`'s "Check exactly" list, against this board.

| `#38` item | Verdict here | Where |
| --- | --- | --- |
| Differential-pair membership and geometry, length constraints | Not applicable: no differential pair on this board | B16 |
| Return paths and layer transitions | One advisory proximity finding; no return-current analysis, and the capability descriptor says so | A9 |
| Impedance assumptions | Not applicable: no controlled-impedance net, no stackup block | B13 |
| Keepouts | Unsupported here: the language has them, none was declared or checked | B10 |
| Analog placement separation | R_FB_T 3.39 mm and R_FB_B 13.77 mm from the FB pin; divider ground 5.86 mm from U1 GND; not judged | B6 |
| Grounding | One unsplit GND zone per layer; split planes unsupported; see the layer-change finding | B6, A9 |
| Decoupling | Boot and decoupling capacitors 4.7 to 19.8 mm from the pin they serve; loop inductance not computed | B12 |
| Thermal relief | Fixed export defaults, not analysed; the pad's copper is proven, the temperature is not | A16, B2 |
| Connector ESD paths | D2 SMBJ30A on VIN is the ESD/transient clamp; J1 and J2 are 2- and 4-pin power connectors with no signal pins | B1, README |
| Power sequencing | The power-good signal exists and is pulled up (A15); its thresholds, delay and fault behaviour are not checked or measured | A15, A17, B3, B11 |
| ERC and DRC | 0 errors on both, 0 DRC warnings and 1 ERC warning (library symbol mismatch) | A4, A7 |
| Schematic correctness | Netlist equals the source net for net; readability measured, not tidy | A8, B15 |
| BOM correctness | 27 components, one BOM row each, no MPN or LCSC on the generic parts | A6, B14 |
| Footprint correctness | Pins and packages not compared with the manufacturer drawings; none of the 8 new parts has a reviewer | B8 |
| Manufacturing exports | Gerbers, drill, BOM and pick-and-place generated and hashed; not fabricated | A10, B14 |
| KiCad reopenability | Loaded headlessly by KiCad 10.0.7 for every export used here; not opened in the GUI | A11 |
| Which claims Synth proves | Section A, and only section A | A1-A16 |
| Which claims need SI/PI/RF expert analysis | Section B | B1-B19 |
| Comparison against a human reference | Only TI's curves; no layout comparison | B17 |

All four features #38's first case names are now carried by the board: input and output filtering
(A3, B4, B12), feedback (A2, A8), power-good (A15, A17, B3) and thermal copper (A16, B2). The last two
are present and connected but unverified, where the previous revision declared them unsupported.

## D. The row this board declares

`qualification/matrix.toml` (the corpus #38 is measured on) is not on `main` yet, so this board
cannot be added to it in this branch. When it lands, this is the row, with the values measured
above and no value guessed:

```toml
[[board]]
id = "buck_24v_3v3"
path = "qualification/class-b/buck/buck.synth"
class = "B"
status = "experimental"
rationale = "Class B case 1: a 24 V to 3.3 V LMR33640 buck with input protection, UVLO, power-good \
on J2 pin 3, an exposed thermal pad and a switch-node-feedback negative twin. The board is its own \
case: the feedback-safety rule, the power-good net, the unreviewed-part fab block and the \
layer-change finding are measured on this source, not carried over from another board."
```

`expect` and `expect_fab` are deliberately missing. `synth check` in this revision always
selects KiCadRoutingTools and has no flag to select FreeRouting, and no KiCadRoutingTools
checkout was installed here, so the `synth check` verdict for this board was not measured. The
measured facts that would feed it are `validate` exit 0 (A1), `routing.state` routed (A3) and
`kicad-cli pcb drc` 0 errors (A4) for source/DRC, and `release_ready` false with
`production_status` untrusted (A6) for manufacturing.

**Issue #67 asks for the matrix row. It is not delivered in this PR**, because `qualification/matrix.toml` does not exist on `main` (#75 and #247 are open). The row above is the content for a follow-up that lands with #247; until then this PR does not claim #67 is fully satisfied.
